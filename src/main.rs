use dbus::{blocking::Connection};
use std::time::Duration;
use regex::{Regex};
use clap::{Arg, App};
use confy;
use serde_derive::{Serialize, Deserialize};

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
            priorities: vec![PlayerPriority { player_name: String::from("rhythmbox"), priority: 20 },
                             PlayerPriority { player_name: String::from("spotify"), priority: 15 },
                             PlayerPriority { player_name: String::from("io.github.GnomeMpv"), priority: 10 },
                             PlayerPriority { player_name: String::from("chromium"), priority: 5 }]
        }
    }
}

fn find_media_players(conn: &Connection) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let proxy = conn.with_proxy("org.freedesktop.DBus", "/", Duration::from_millis(5000));
    let (names, ): (Vec<String>, ) = proxy.method_call("org.freedesktop.DBus", "ListNames", ())?;

    let re = Regex::new(r"org.mpris.MediaPlayer2.(.+)").unwrap();
    let detected_players: Vec<String> = names.iter().filter(|name| re.is_match(&name)).cloned().collect();

    Ok(detected_players)
}

fn sort_players(players: &Vec<String>, cfg: &Config) -> Vec<PlayerPriority> {
    let mut ordered_players: Vec<PlayerPriority> = Vec::new();
    for name in players {
        let player = get_player_name_from_bus(&name);
        match cfg.priorities.iter().find(|x| x.player_name == player) {
            Some(player_priority) => ordered_players.push(PlayerPriority { player_name: String::from(name), priority: player_priority.priority }),
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new("Mediaplayer Controller")
        .version("0.1.0")
        .author("Marc-André Renaud <ma.renaud@slashvoid.com>")
        .about("Call various actions of active media players")
        .arg(Arg::new("action")
            .about("Action to call") // Displayed when showing help info
            .short('a') // Trigger this arg with "-a"
            .long("action") // Trigger this arg with "--awesome"
            .takes_value(true)
            .conflicts_with("list"))
        .arg(Arg::new("list")
            .about("List discovered players") // Displayed when showing help info
            .short('l') // Trigger this arg with "-a"
            .long("list") // Trigger this arg with "--awesome"
            .conflicts_with("action"))
        .get_matches();

    let mut resquested_action: String = String::from("");

    let cfg = confy::load("mediaplayer-controller").unwrap_or_default();

    if matches.is_present("list") {
        let conn = Connection::new_session()?;
        let detected_players = find_media_players(&conn).unwrap();
        for name in detected_players {
            println!("{}", get_player_name_from_bus(&name));
        }
    }

    if matches.is_present("action") {
        resquested_action = matches.value_of_t("action").unwrap_or(String::from(""));
    }

    if !resquested_action.is_empty() {
        let conn = Connection::new_session()?;
        let detected_players = find_media_players(&conn).unwrap();

        let sorted_players = sort_players(&detected_players, &cfg);

        match sorted_players.first() {
            Some(player) => {
                let p = conn.with_proxy(&(player.player_name), "/org/mpris/MediaPlayer2", Duration::from_millis(5000));
                p.method_call("org.mpris.MediaPlayer2.Player", &resquested_action, ())?
            }
            None => (),
        }

        confy::store("mediaplayer-controller", cfg)?;
    }

    Ok(())
}