use dc_rpc_rs::*;

#[tokio::main]
async fn main() {
    let mut client = Client::<connection::IpcConnection>::connect(None)
        .expect("Could not connect!");
}