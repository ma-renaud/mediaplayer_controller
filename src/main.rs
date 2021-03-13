use dbus::{blocking::Connection};
use std::time::Duration;
use regex::{Regex};
use clap::{Arg, App};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new("Mediaplayer Controller")
        .version("0.1.0")
        .author("Marc-André Renaud <ma.renaud@slashvoid.com>")
        .about("Call various actions of active media players")
        .arg(Arg::new("action")
            .about("Action to call") // Displayed when showing help info
            .short('a') // Trigger this arg with "-a"
            .long("action") // Trigger this arg with "--awesome"
            .takes_value(true))
        .get_matches();

    let mut resquested_action: String = String::from("");
    if matches.is_present("action") {
        resquested_action = matches.value_of_t("action").unwrap_or(String::from(""));
    }

    if !resquested_action.is_empty() {
        let conn = Connection::new_session()?;

        let proxy = conn.with_proxy("org.freedesktop.DBus", "/", Duration::from_millis(5000));
        let (names, ): (Vec<String>, ) = proxy.method_call("org.freedesktop.DBus", "ListNames", ())?;

        let re = Regex::new(r"org.mpris.MediaPlayer2.(.+)").unwrap();
        let players: Vec<String> = names.iter().filter(|name| re.is_match(&name)).cloned().collect();

        for name in players {
            //Show player name
            let strings: Vec<&str> = name.split('.').collect();
            println!("{}", strings[3]);

            let p = conn.with_proxy(&name, "/org/mpris/MediaPlayer2", Duration::from_millis(5000));
            p.method_call("org.mpris.MediaPlayer2.Player", &resquested_action, ())?
        }
    }

    Ok(())
}