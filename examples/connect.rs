use dc_rpc_rs::*;
use std::io;
use std::time;

fn main() {
    let mut conn = DiscordRPC::new("683696447815811121");
    conn.start();

    loop {
        let mut line = String::new();
        io::stdin().read_line(&mut line);
        if line.chars().next() == Some('c') {
            conn.set_rich_presence(None);
        }
        else {
            let mut rp = RichPresence::default();
            rp.state = "Doing stuff".into();
            rp.details = "More stuff...".into();
            rp.start_timestamp = Some(time::SystemTime::now());
            conn.set_rich_presence(Some(rp));
        }
    }
}