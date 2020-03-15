use dc_rpc_rs::*;

fn main() {
    if let Ok(mut c) = Client::build_ipc_connection() {
        println!("Ok");
    }
    else {
        println!("Err");
    }
}