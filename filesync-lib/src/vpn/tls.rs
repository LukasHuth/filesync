use anyhow::{Result, bail};
use rustls::{
    ClientConfig, ClientConnection, RootCertStore,
    pki_types::{CertificateDer, PrivateKeyDer, ServerName},
};
use rustls_pemfile::{certs, private_key};
use std::{
    fs::File,
    io::{BufReader, Read as _, Write as _},
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::vpn::VPN;

pub fn client_config(dir: &Path) -> Result<Arc<ClientConfig>> {
    let ca_file = File::open(dir.join("ca.pem"))?;
    let mut ca_reader = BufReader::new(ca_file);

    let mut roots = RootCertStore::empty();

    for cert in certs(&mut ca_reader) {
        roots.add(cert?)?;
    }

    let client_file = File::open(dir.join("client.pem"))?;
    let mut client_reader = BufReader::new(client_file);

    let chain: Vec<CertificateDer<'static>> =
        certs(&mut client_reader).collect::<std::result::Result<Vec<_>, _>>()?;

    let key_file = File::open(dir.join("client.key"))?;
    let mut key_reader = BufReader::new(key_file);

    let key: PrivateKeyDer<'static> = private_key(&mut key_reader)?
        .ok_or_else(|| anyhow::anyhow!("no private key found in client.key"))?;

    Ok(Arc::new(
        ClientConfig::builder()
            .with_root_certificates(roots)
            .with_client_auth_cert(chain, key)?,
    ))
}

pub struct TlsStream {
    vpn: VPN,
    conn: ClientConnection,
}
impl TlsStream {
    pub async fn connect(
        vpn: VPN,
        cfg: Arc<ClientConfig>,
        name: ServerName<'static>,
    ) -> Result<Self> {
        let mut s = Self {
            vpn,
            conn: ClientConnection::new(cfg, name)?,
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        while s.conn.is_handshaking() {
            if Instant::now() > deadline {
                bail!("TLS handshake timed out");
            }
            s.drive().await?;
        }
        Ok(s)
    }
    /// Move bytes between rustls and the TCP socket once.
    async fn drive(&mut self) -> Result<()> {
        let mut out = Vec::new();
        while self.conn.wants_write() {
            self.conn.write_tls(&mut out)?;
        }
        if !out.is_empty() {
            self.vpn.write(&out).await?;
        }
        if self.conn.wants_read() {
            let mut tmp = [0u8; 4096];
            match self.vpn.recv_some(&mut tmp).await? {
                Some(0) => bail!("peer closed during TLS"),
                Some(n) => {
                    let mut rd = &tmp[..n];
                    while !rd.is_empty() {
                        self.conn.read_tls(&mut rd)?;
                        self.conn.process_new_packets()?; // cert errors surface here
                    }
                }
                None => {}
            }
        }
        Ok(())
    }
    pub async fn send(&mut self, data: &[u8]) -> Result<()> {
        self.conn.writer().write_all(data)?;
        while self.conn.wants_write() {
            self.drive().await?;
        }
        self.vpn.flush().await
    }
    /// Returns Ok(0) after a clean TLS close_notify.
    pub async fn recv(&mut self, buf: &mut [u8]) -> Result<usize> {
        loop {
            match self.conn.reader().read(buf) {
                Ok(n) => return Ok(n),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e.into()),
            }
            self.drive().await?;
        }
    }
    pub async fn close(&mut self) -> Result<()> {
        self.conn.send_close_notify();
        while self.conn.wants_write() {
            self.drive().await?;
        }
        self.vpn.close().await
    }
}
