use anyhow::{Result, anyhow, bail};
use boringtun::{
    noise::{Tunn, TunnResult},
    x25519::{PublicKey, StaticSecret},
};
use smoltcp::{
    iface::{Config, Interface, SocketHandle, SocketSet},
    socket::tcp,
    wire::{HardwareAddress, IpAddress, IpCidr, Ipv4Address},
};
use std::time::{Duration, Instant as StdInstant};
use tokio::net::UdpSocket;

use crate::vpn::{
    helpers::{key32, smol_now},
    queue_device::QueueDevice,
};

mod consts;
mod helpers;
mod queue_device;

// async fn flush_tx(
//     dev: &mut QueueDevice,
//     tunn: &mut Tunn,
//     udp: &UdpSocket,
//     buf: &mut [u8],
// ) -> Result<()> {
//     while let Some(pkt) = dev.tx.pop_front() {
//         match tunn.encapsulate(&pkt, buf) {
//             TunnResult::WriteToNetwork(enc) => {
//                 udp.send(enc).await?;
//             }
//             TunnResult::Err(e) => eprintln!("encapsulate error: {e:?}"),
//             _ => {}
//         }
//     }
//     Ok(())
// }
// async fn handle_incoming(
//     datagram: &[u8],
//     dev: &mut QueueDevice,
//     tunn: &mut Tunn,
//     udp: &UdpSocket,
//     buf: &mut [u8],
// ) -> Result<()> {
//     let mut res = tunn.decapsulate(None, datagram, buf);
//     loop {
//         match res {
//             TunnResult::WriteToNetwork(reply) => {
//                 // handshake response / keepalive; then drain queued packets
//                 udp.send(reply).await?;
//                 res = tunn.decapsulate(None, &[], buf);
//             }
//             TunnResult::WriteToTunnelV4(plain, _) | TunnResult::WriteToTunnelV6(plain, _) => {
//                 dev.rx.push_back(plain.to_vec());
//                 break;
//             }
//             TunnResult::Done => break,
//             TunnResult::Err(e) => {
//                 eprintln!("decapsulate error: {e:?}");
//                 break;
//             }
//         }
//     }
//     Ok(())
// }

pub struct VPNConfig {
    client_key: String,
    client_ip: Ipv4Address,
    server_key: String,
    server_ip: String,
    server_port: u32,
    target_ip: Ipv4Address,
    target_port: u16,
}
impl VPNConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            client_key: std::env::var("CLIENT_KEY")?,
            server_key: std::env::var("SERVER_KEY")?,
            server_ip: std::env::var("SERVER_IP")?,
            server_port: std::env::var("SERVER_PORT")?.parse()?,
            client_ip: std::env::var("CLIENT_IP")?.parse()?,
            target_ip: std::env::var("TARGET_IP")?.parse()?,
            target_port: std::env::var("TARGET_PORT")?.parse()?,
        })
    }
}
pub struct VPN {
    tunn: Tunn,
    udp: UdpSocket,
    dev: QueueDevice,
    iface: Interface,
    sockets: SocketSet<'static>,
    handle: SocketHandle,
    start: StdInstant,
    timeout: Duration,
    buf: Vec<u8>,
}
impl VPN {
    pub async fn new(config: VPNConfig) -> Result<Self> {
        let private = StaticSecret::from(key32(&config.client_key)?);
        let server_pub = PublicKey::from(key32(&config.server_key)?);
        let tunn = Tunn::new(private, server_pub, None, Some(25), 0, None);
        let udp = UdpSocket::bind("0.0.0.0:0").await?;
        // Android: call VpnService.protect() on this socket's fd if you ever
        // run this alongside a system VPN. Not needed for the in-process mode.
        udp.connect(format!("{}:{}", config.server_ip, config.server_port))
            .await?;

        let (dev, iface, sockets, handle, start) = Self::setup_ip_stack(&config).await?;

        let mut this = Self {
            tunn,
            udp,
            dev,
            iface,
            sockets,
            handle,
            start,
            timeout: Duration::from_secs(5),
            buf: vec![0u8; 2048],
        };

        this.initial_handshake().await?;

        Ok(this)
    }
    async fn initial_handshake(&mut self) -> Result<()> {
        let deadline = StdInstant::now() + self.timeout;
        loop {
            match self.sock().state() {
                tcp::State::Established => break Ok(()),
                tcp::State::Closed => bail!("connection refused or reset"),
                _ => {}
            }
            if StdInstant::now() > deadline {
                let (hs_age, tx, rx, _loss, _rtt) = self.tunn.stats();
                bail!(
                    "timed out: tcp={:?}, last_handshake={:?}, wg_tx={}B, wg_rx={}B",
                    self.sock().state(),
                    hs_age,
                    tx,
                    rx
                );
            }
            self.pump().await?;
        }
    }
    async fn setup_ip_stack(
        vpn_config: &VPNConfig,
    ) -> Result<(
        QueueDevice,
        Interface,
        SocketSet<'static>,
        SocketHandle,
        StdInstant,
    )> {
        let mut dev = QueueDevice::default();
        let start = StdInstant::now();

        let mut config = Config::new(HardwareAddress::Ip);
        config.random_seed = start.elapsed().as_nanos() as u64 ^ 0x9E37_79B9_7F4A_7C15;
        let mut iface = Interface::new(config, &mut dev, smol_now(start));
        iface.update_ip_addrs(|addrs| {
            addrs
                .push(IpCidr::new(IpAddress::Ipv4(vpn_config.client_ip), 24))
                .ok();
        });
        iface
            .routes_mut()
            .add_default_ipv4_route(vpn_config.target_ip)
            .ok();

        let mut sockets = SocketSet::new(vec![]);
        let tcp_handle = sockets.add(tcp::Socket::new(
            tcp::SocketBuffer::new(vec![0u8; 16 * 1024]),
            tcp::SocketBuffer::new(vec![0u8; 16 * 1024]),
        ));
        sockets.get_mut::<tcp::Socket>(tcp_handle).connect(
            iface.context(),
            (
                IpAddress::Ipv4(vpn_config.target_ip),
                vpn_config.target_port,
            ),
            49152,
        )?;
        Ok((dev, iface, sockets, tcp_handle, start))
    }

    fn sock(&mut self) -> &mut tcp::Socket<'static> {
        self.sockets.get_mut::<tcp::Socket>(self.handle)
    }

    /// One turn of the crank: run the TCP/IP stack, send what it produced,
    /// wait briefly for incoming packets, run WireGuard timers.
    async fn pump(&mut self) -> Result<()> {
        self.poll_and_flush().await?;

        let mut rx = [0u8; 2048];
        tokio::select! {
            r = self.udp.recv(&mut rx) => {
                let n = r?;
                self.handle_incoming(&rx[..n]).await?;
            }
            _ = tokio::time::sleep(Duration::from_millis(20)) => {}
        }

        // handshake initiation, rekeying, keepalives
        if let TunnResult::WriteToNetwork(p) = self.tunn.update_timers(&mut self.buf) {
            self.udp.send(p).await?;
        }
        self.poll_and_flush().await
    }

    async fn poll_and_flush(&mut self) -> Result<()> {
        self.iface
            .poll(smol_now(self.start), &mut self.dev, &mut self.sockets);
        while let Some(pkt) = self.dev.tx.pop_front() {
            match self.tunn.encapsulate(&pkt, &mut self.buf) {
                TunnResult::WriteToNetwork(enc) => {
                    self.udp.send(enc).await?;
                }
                TunnResult::Err(e) => eprintln!("encapsulate error: {e:?}"),
                _ => {}
            }
        }
        Ok(())
    }
    async fn handle_incoming(&mut self, datagram: &[u8]) -> Result<()> {
        let mut res = self.tunn.decapsulate(None, datagram, &mut self.buf);
        loop {
            match res {
                TunnResult::WriteToNetwork(reply) => {
                    self.udp.send(reply).await?;
                    res = self.tunn.decapsulate(None, &[], &mut self.buf);
                }
                TunnResult::WriteToTunnelV4(p, _) | TunnResult::WriteToTunnelV6(p, _) => {
                    self.dev.rx.push_back(p.to_vec());
                    return Ok(());
                }
                TunnResult::Done => return Ok(()),
                TunnResult::Err(e) => {
                    eprintln!("decapsulate error: {e:?}");
                    return Ok(());
                }
            }
        }
    }
    pub fn close_sync(&mut self) {
        self.sock().close();
    }
    /// Graceful TCP close.
    pub async fn close(&mut self) -> Result<()> {
        self.sock().close();
        let deadline = StdInstant::now() + Duration::from_secs(5);
        while self.sock().is_active() && StdInstant::now() < deadline {
            self.pump().await?;
        }
        Ok(())
    }
    /// Send all of `data`; returns once the peer has acknowledged it.
    pub async fn send(&mut self, mut data: &[u8]) -> Result<()> {
        let deadline = StdInstant::now() + self.timeout;
        while !data.is_empty() {
            let sock = self.sock();
            if !sock.may_send() {
                bail!("connection closed");
            }
            let n = sock.send_slice(data)?; // copies what fits in the tx buffer
            data = &data[n..];
            self.pump().await?;
            if StdInstant::now() > deadline {
                bail!("send timed out");
            }
        }
        while self.sock().send_queue() > 0 {
            if !self.sock().is_active() {
                bail!("connection closed before data was acked");
            }
            if StdInstant::now() > deadline {
                bail!("send timed out waiting for ack");
            }
            self.pump().await?;
        }
        Ok(())
    }

    /// Read up to `buf.len()` bytes. Returns `Ok(0)` when the peer closed.
    pub async fn recv(&mut self, buf: &mut [u8]) -> Result<usize> {
        let deadline = StdInstant::now() + self.timeout;
        loop {
            let sock = self.sock();
            if sock.can_recv() {
                return Ok(sock.recv_slice(buf)?);
            }
            if !sock.may_recv() {
                return Ok(0);
            }
            if StdInstant::now() > deadline {
                bail!("recv timed out");
            }
            self.pump().await?;
        }
    }
}
