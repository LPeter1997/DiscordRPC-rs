use dc_rpc_rs::*;

#[tokio::main]
async fn main() {
    if let Ok(mut c) = Client::build_ipc_connection(None).await {
        println!("Ok");
        if let Ok(resp) = c.authorize(None).await {
            println!("Auth response: {:?}", resp);
        }
        else {
            println!("Failed to auth");
        }
    }
    else {
        println!("Err");
    }
}