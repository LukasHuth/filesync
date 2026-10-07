use tokio::{
    io::{AsyncBufReadExt as _, BufReader},
    net::TcpListener,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("10.9.0.1:8088").await?;
    loop {
        let (stream, addr) = listener.accept().await?;
        println!("Client connected: {addr}");

        tokio::spawn(async move {
            let reader = BufReader::new(stream);
            let mut lines = reader.lines();

            while let Some(line) = lines.next_line().await? {
                println!("Received: {line}");
            }

            Ok::<(), std::io::Error>(())
        });
    }
}
