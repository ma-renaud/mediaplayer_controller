use dbus::blocking::Connection;
use std::time::Duration;
use regex::{Regex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // First open up a connection to the session bus.
    let conn = Connection::new_session()?;

    // Second, create a wrapper struct around the connection that makes it easy
    // to send method calls to a specific destination and path.
    let proxy = conn.with_proxy("org.freedesktop.DBus", "/", Duration::from_millis(5000));

    // Now make the method call. The ListNames method call takes zero input parameters and
    // one output parameter which is an array of strings.
    // Therefore the input is a zero tuple "()", and the output is a single tuple "(names,)".
    let (names,): (Vec<String>,) = proxy.method_call("org.freedesktop.DBus", "ListNames", ())?;

    let re = Regex::new(r"org.mpris.MediaPlayer2.(.+)").unwrap();

    // Let's print all the names to stdout.
    for name in names {
        if re.is_match(&name) {
            println!("{}", name);
        }
    }

    Ok(())
}