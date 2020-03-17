use dc_rpc_rs::*;

fn main() {
    let mut conn = DiscordRPC::new("123784432336");
    conn.start();

    loop {}
}