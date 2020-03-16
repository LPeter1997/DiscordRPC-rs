use dc_rpc_rs::*;

fn main() {
    let mut conn = IpcConnection::new();
    if conn.open() {
        println!("Ok");
    }
    else {
        println!("Err");
    }
}