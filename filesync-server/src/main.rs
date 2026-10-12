use std::{fs::File, io::BufReader as StdBufReader, path::Path, sync::Arc};

use filesync_lib::{
    auth::CA,
    protocol::{
        ClientBoundPacket, PacketUtilsIn, PacketUtilsOut, ServerBoundPacket, ServerSidePacketUtils,
    },
    vpn::VPNConfig,
};
use rustls::{
    RootCertStore, ServerConfig,
    pki_types::{CertificateDer, PrivateKeyDer},
    server::WebPkiClientVerifier,
};
use rustls_pemfile::{certs, private_key};
use tokio::{
    io::{AsyncBufReadExt as _, BufReader},
    net::{TcpListener, TcpStream},
};
use tokio_rustls::TlsAcceptor;

use crate::functions::HandlePacket as _;

type BoxErr = Box<dyn std::error::Error>;
#[tokio::main]
async fn main() -> Result<(), BoxErr> {
    let acceptor = tls_acceptor(Path::new("pki"))?;
    let listener = TcpListener::bind("10.9.0.2:8088").await?;
    dotenvy::dotenv()?;
    let config = VPNConfig::from_env()?;
    CA::setup_with_config(config)?;
    loop {
        let (stream, addr) = listener.accept().await?;
        println!("Client connected: {addr}");

        let acceptor = acceptor.clone(); // cheap, it's an Arc inside
        tokio::spawn(async move {
            if let Err(e) = client_handling(acceptor, stream).await {
                eprintln!("{addr}: {e}");
            }
        });
    }
}
fn tls_acceptor(dir: &Path) -> Result<TlsAcceptor, BoxErr> {
    // CA certificate(s)
    let ca_file = File::open(dir.join("ca.pem"))?;
    let mut ca_reader = StdBufReader::new(ca_file);

    let mut roots = RootCertStore::empty();

    for cert in certs(&mut ca_reader) {
        roots.add(cert?)?;
    }

    // Require clients to present a certificate signed by our CA.
    let verifier = WebPkiClientVerifier::builder(Arc::new(roots)).build()?;

    // Server certificate chain.
    let server_file = File::open(dir.join("server.pem"))?;
    let mut server_reader = StdBufReader::new(server_file);

    let chain: Vec<CertificateDer<'static>> =
        certs(&mut server_reader).collect::<std::result::Result<Vec<_>, _>>()?;

    // Server private key.
    let key_file = File::open(dir.join("server.key"))?;
    let mut key_reader = StdBufReader::new(key_file);

    let key: PrivateKeyDer<'static> =
        private_key(&mut key_reader)?.ok_or_else(|| "no private key found in server.key")?;

    let cfg = ServerConfig::builder()
        .with_client_cert_verifier(verifier)
        .with_single_cert(chain, key)?;

    Ok(TlsAcceptor::from(Arc::new(cfg)))
}

type ExpBufReader = BufReader<tokio::io::ReadHalf<tokio_rustls::server::TlsStream<TcpStream>>>;
type ExpWriter = tokio::io::WriteHalf<tokio_rustls::server::TlsStream<TcpStream>>;
async fn client_handling(acceptor: TlsAcceptor, stream: TcpStream) -> Result<(), BoxErr> {
    // TLS handshake; fails here if the client cert is missing or untrusted
    let tls = acceptor.accept(stream).await?;

    let (read_half, mut write_half) = tokio::io::split(tls);

    let mut reader: ExpBufReader = BufReader::new(read_half);
    while let Some(packet) =
        <ExpBufReader as PacketUtilsIn<ServerBoundPacket>>::read_packet(&mut reader).await?
    {
        PacketHandler::handle_packet(packet, &mut reader, &mut write_half).await?;
    }
    Ok(())
}
struct PacketHandler;
impl functions::HandlePacket for PacketHandler {
    async fn handle_packet(
        packet: ServerBoundPacket,
        reader: &mut ExpBufReader,
        writer: &mut ExpWriter,
    ) -> Result<(), BoxErr> {
        match packet {
            ServerBoundPacket::DebugMessage { .. } => {
                functions::debug_message::DebugMessage::handle_packet(packet, reader, writer).await
            }
            ServerBoundPacket::GetAvailableFiles => {
                functions::get_available_files::GetAvailableFiles::handle_packet(
                    packet, reader, writer,
                )
                .await
            }
        }
    }
}
mod functions;
