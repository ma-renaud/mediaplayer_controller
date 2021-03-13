use dbus::{blocking::Connection};
use std::time::Duration;
use regex::{Regex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::new_session()?;

    let proxy = conn.with_proxy("org.freedesktop.DBus", "/", Duration::from_millis(5000));
    let (names,): (Vec<String>,) = proxy.method_call("org.freedesktop.DBus", "ListNames", ())?;

    let re = Regex::new(r"org.mpris.MediaPlayer2.(.+)").unwrap();
    let players: Vec<String> = names.iter().filter(|name| re.is_match(&name)).cloned().collect();

    for name in players {
        let p = conn.with_proxy(&name, "/org/mpris/MediaPlayer2", Duration::from_millis(5000));
        p.method_call("org.mpris.MediaPlayer2.Player", "PlayPause", ())?
    }

    Ok(())
}