use anyhow::{bail, Result};
use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer, ServerName},
    ClientConfig, ClientConnection, RootCertStore,
};
use rustls_pemfile::{certs, private_key};
use std::{
    fs::File,
    io::{BufReader, Read as _, Write as _},
    path::Path,
    pin::Pin,
    sync::Arc,
    task::Poll,
    time::{Duration, Instant},
};
use tokio::io::AsyncWrite;

use crate::vpn::{io_err, SockIo, VPN};

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
    eof: bool,
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
            eof: false,
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
    async fn flush(&mut self) -> Result<()> {
        let mut out = Vec::new();
        while self.conn.wants_write() {
            self.conn.write_tls(&mut out)?;
        }
        if !out.is_empty() {
            self.vpn.write(&out).await?; // make sure this writes everything (write_all semantics)
        }
        Ok(())
    }

    /// Move bytes between rustls and the TCP socket once.
    async fn drive(&mut self) -> Result<()> {
        self.flush().await?;
        if self.conn.wants_read() {
            let mut tmp = [0u8; 4096];
            match self.vpn.recv_some(&mut tmp).await? {
                Some(0) => {
                    if self.conn.is_handshaking() {
                        bail!("peer closed during TLS handshake");
                    }
                    // EOF after the handshake: let the caller decide what it means
                    self.eof = true; // add a bool field
                }
                Some(n) => {
                    let mut rd = &tmp[..n];
                    while !rd.is_empty() {
                        self.conn.read_tls(&mut rd)?;
                        self.conn.process_new_packets()?;
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
            self.flush().await?;
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
            self.flush().await?;
        }
        self.vpn.close().await
    }
    fn write_tls_to_tcp(&mut self) -> std::io::Result<()> {
        while self.conn.wants_write() {
            match self.conn.write_tls(&mut SockIo(self.vpn.sock())) {
                Ok(_) => (),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}
impl tokio::io::AsyncRead for TlsStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        loop {
            match this.conn.reader().read(buf.initialize_unfilled()) {
                Ok(n) => {
                    buf.advance(n);
                    return Poll::Ready(Ok(()));
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => (),
                Err(e) => return Poll::Ready(Err(e)),
            }

            if let Err(e) = this.vpn.poll_pump(cx) {
                return Poll::Ready(Err(e));
            }
            if let Err(e) = this.write_tls_to_tcp() {
                return Poll::Ready(Err(e));
            }

            let mut progressed = false;
            if this.conn.wants_read() {
                match this.conn.read_tls(&mut SockIo(this.vpn.sock())) {
                    Ok(_) => {
                        if let Err(e) = this.conn.process_new_packets() {
                            let _ = this.write_tls_to_tcp();
                            this.vpn.poll_and_flush_sync();
                            return Poll::Ready(Err(io_err(e)));
                        }
                        progressed = true;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => (),
                    Err(e) => return Poll::Ready(Err(e)),
                }
            }
            let _ = this.write_tls_to_tcp();
            this.vpn.poll_and_flush_sync();

            if !progressed {
                return Poll::Pending;
            }
        }
    }
}

impl AsyncWrite for TlsStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        this.vpn.poll_pump(cx)?;
        this.write_tls_to_tcp()?;

        let n = this.conn.writer().write(buf)?;
        if n == 0 && !buf.is_empty() {
            return Poll::Pending;
        }
        this.write_tls_to_tcp()?;
        this.vpn.poll_and_flush_sync();
        Poll::Ready(Ok(n))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        this.vpn.poll_pump(cx)?;
        this.write_tls_to_tcp()?;
        this.vpn.poll_and_flush_sync();
        if this.conn.wants_write() || this.vpn.sock().send_queue() > 0 {
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = &mut *self;
        this.conn.send_close_notify();
        if let Poll::Pending = Pin::new(&mut *this).poll_flush(cx) {
            return Poll::Pending;
        }
        this.vpn.sock().close();
        this.vpn.poll_pump(cx)?;
        if this.vpn.sock().is_active() {
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }
}
