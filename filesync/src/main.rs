use filesync_lib::vpn::{VPN, VPNConfig};

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    dotenvy::dotenv().unwrap();
    let config = VPNConfig::from_env().unwrap();
    let mut vpn = VPN::new(config).await.unwrap();
    vpn.send("HELLO, WORLD!".as_bytes()).await.unwrap();
    vpn.close().await.unwrap();
}
