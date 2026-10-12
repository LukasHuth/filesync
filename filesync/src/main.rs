use filesync_lib::protocol::{
    ClientBoundPacket, PacketUtilsIn as _, PacketUtilsOut as _, ServerBoundPacket,
};
use filesync_lib::vpn::{VPN, VPNConfig};
use tokio::io::BufReader;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    dotenvy::dotenv().unwrap();
    let config = VPNConfig::from_env().unwrap();
    let stream = VPN::connect_tls(config).await.unwrap();
    let mut buf_reader = BufReader::new(stream);
    buf_reader
        .write_packet(ServerBoundPacket::DebugMessage {
            message: "test".to_string(),
        })
        .await
        .unwrap();
    let Some(res): Option<ClientBoundPacket> = buf_reader.read_packet().await.unwrap() else {
        panic!("Expected an response")
    };
    let ClientBoundPacket::DebugMessage { message } = res else {
        panic!("Expected debug message, got: {res:?}");
    };
    println!("Debug Message: {message}");
}
