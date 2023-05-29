use clap::{Arg, ArgAction, Command};
use confy;
use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;
use dbus::{arg, blocking::Connection};
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone)]
struct PlayerPriority {
    player_name: String,
    priority: i8,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Config {
    priorities: Vec<PlayerPriority>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            priorities: vec![
                PlayerPriority {
                    player_name: String::from("spotify"),
                    priority: 25,
                },
                PlayerPriority {
                    player_name: String::from("Lollypop"),
                    priority: 20,
                },
                PlayerPriority {
                    player_name: String::from("rhythmbox"),
                    priority: 15,
                },
                PlayerPriority {
                    player_name: String::from("io.github.GnomeMpv"),
                    priority: 10,
                },
                PlayerPriority {
                    player_name: String::from("brave"),
                    priority: 5,
                },
            ],
        }
    }
}

fn find_media_players(conn: &Connection) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let proxy = conn.with_proxy("org.freedesktop.DBus", "/", Duration::from_millis(5000));
    let (names,): (Vec<String>,) = proxy.method_call("org.freedesktop.DBus", "ListNames", ())?;

    let re = Regex::new(r"org.mpris.MediaPlayer2.(.+)").unwrap();
    let detected_players: Vec<String> = names.iter().filter(|name| re.is_match(&name)).cloned().collect();

    Ok(detected_players)
}

fn sort_players(players: &Vec<String>, cfg: &Config) -> Vec<PlayerPriority> {
    let mut ordered_players: Vec<PlayerPriority> = Vec::new();
    for name in players {
        let player = get_player_name_from_bus(&name);
        match cfg.priorities.iter().find(|x| x.player_name == player) {
            Some(player_priority) => ordered_players.push(PlayerPriority {
                player_name: String::from(name),
                priority: player_priority.priority,
            }),
            None => (),
        }
    }

    ordered_players.sort_by_key(|x| x.priority);
    ordered_players.reverse();

    return ordered_players;
}

fn get_player_name_from_bus(interface: &str) -> String {
    let strings: Vec<&str> = interface.split('.').collect();
    let player;

    if strings.len() == 4 {
        player = strings[3].to_string();
    } else {
        player = strings[3..(strings.len() - 1)].join(".");
    }

    return player;
}

fn dbus_call(conn: &Connection, bus: &str, method: &str, arg: &str) {
    let p = conn.with_proxy(bus, "/org/mpris/MediaPlayer2", Duration::from_millis(5000));

    if !arg.is_empty() {
        if method == "Volume" {
            let action = arg.parse::<String>().unwrap_or(String::from(""));
            let volume: &dyn arg::RefArg = &(p.get("org.mpris.MediaPlayer2.Player", "Volume")
                as Result<Box<dyn arg::RefArg + 'static>, dbus::Error>)
                .unwrap();

            if let Some(volume) = volume.as_f64() {
                if action == "+" {
                    p.set("org.mpris.MediaPlayer2.Player", "Volume", volume + 0.05)
                        .unwrap_or({
                            println!("Problem increasing volume");
                        });
                } else if action == "-" {
                    p.set("org.mpris.MediaPlayer2.Player", "Volume", volume - 0.05)
                        .unwrap_or({
                            println!("Problem lowering volume");
                        });
                }
            }
        } else {
            let offset = arg.parse::<i64>().unwrap_or_else(|error| {
                println!("Problem converting seek offset: {:?}", error);
                0
            });

            p.method_call("org.mpris.MediaPlayer2.Player", method, (offset,))
                .unwrap_or_else(|error| {
                    eprintln!("Problem calling the dbus method: {:?}", error);
                });
        }
    } else {
        if method == "Shuffle" {
            let shuffle_state: bool = p.get("org.mpris.MediaPlayer2.Player", "Shuffle").unwrap();
            p.set("org.mpris.MediaPlayer2.Player", "Shuffle", !shuffle_state)
                .unwrap_or_else(|error| {
                    eprintln!("Problem setting the dbus property: {:?}", error);
                });
        } else {
            p.method_call("org.mpris.MediaPlayer2.Player", method, ())
                .unwrap_or_else(|error| {
                    eprintln!("Problem calling the dbus method: {:?}", error);
                });
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("Mediaplayer Controller")
        .version("0.2.0")
        .author("Marc-André Renaud <ma.renaud@slashvoid.com>")
        .about("Call various actions of active media players")
        .subcommand(Command::new("list").about("List discovered players."))
        .subcommand(
            Command::new("call")
                //.setting(AppSettings::AllowLeadingHyphen)
                .about("Call a dbus method")
                .arg(Arg::new("method").help("Method to call").index(1).required(true))
                .arg(Arg::new("arg").help("Method argument").index(2).requires("method"))
                .arg(
                    Arg::new("all")
                        .help("Apply action to all discovered media players")
                        .long("all")
                        .action(ArgAction::SetTrue)
                        .requires("method"),
                ),
        )
        .get_matches();

    let cfg = confy::load("mediaplayer-controller", None).unwrap_or_default();

    match matches.subcommand() {
        Some(("list", _sub_matches)) => {
            let conn = Connection::new_session()?;
            let detected_players = find_media_players(&conn).unwrap();
            for name in detected_players {
                println!("{}", get_player_name_from_bus(&name));
            }
        }
        Some(("call", sub_matches)) => {
            let requested_action = sub_matches
                .get_one::<String>("method")
                .map(|s| s.as_str())
                .unwrap_or("");
            let arg = sub_matches.get_one::<String>("arg").map(|s| s.as_str()).unwrap_or("");

            if !requested_action.is_empty() {
                let conn = Connection::new_session()?;
                let detected_players = find_media_players(&conn).unwrap();

                if sub_matches.get_flag("all") {
                    for player in detected_players {
                        dbus_call(&conn, &player, &requested_action, &arg);
                    }
                } else {
                    let sorted_players = sort_players(&detected_players, &cfg);

                    match sorted_players.first() {
                        Some(player) => {
                            dbus_call(&conn, &(player.player_name), &requested_action, &arg);
                        }
                        None => (),
                    }
                }
            }

            confy::store("mediaplayer-controller", None, cfg)?;
        }
        _ => (),
    }

    Ok(())
}
