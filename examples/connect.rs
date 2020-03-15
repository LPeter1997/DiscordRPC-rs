use dc_rpc_rs::*;

#[tokio::main]
async fn main() {
    let c = IpcConnection::connect(0, None).await;
    if let Ok(c) = c {
        println!("Ok!");
    }
    else {
        println!("Err!");
    }
}