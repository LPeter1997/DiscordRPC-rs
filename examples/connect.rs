use dc_rpc_rs::*;

#[tokio::main]
async fn main() {
    let c = Client::build_ipc_connection(None).await;
    if c.is_ok() {
        println!("Ok");
    }
    else {
        println!("Err");
    }
}