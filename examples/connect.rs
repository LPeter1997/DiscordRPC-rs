use dc_rpc_rs::*;

fn main() {
    let mut conn = DiscordRPC::new("683696447815811121");
    conn.start();

    let mut rp = RichPresence::default();
    rp.state = "Doing stuff".into();
    rp.details = "More stuff...".into();
    conn.set_rich_presence(Some(rp));

    loop {}
}