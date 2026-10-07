use filesync_lib::vpn::{VPN, VPNConfig};

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    dotenvy::dotenv().unwrap();
    let config = VPNConfig::from_env().unwrap();
    let mut stream = VPN::connect_tls(config).await.unwrap();
    stream.send("HELLO, WORLD!".as_bytes()).await.unwrap();
    stream.close().await.unwrap();
}
