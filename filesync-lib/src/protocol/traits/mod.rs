use tokio::io::{
    AsyncBufRead, AsyncBufReadExt as _, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _,
};

use crate::protocol::{internal_traits::PacketData, ClientBoundPacket, ServerBoundPacket};

pub trait PacketUtilsIn<T: PacketData> {
    fn read_packet(&mut self) -> impl Future<Output = std::io::Result<Option<T>>>;
}

pub trait PacketUtilsOut<T: PacketData> {
    fn write_packet(&mut self, packet: T) -> impl Future<Output = Result<(), Box<dyn std::error::Error>>>;
}

pub trait ServerSidePacketUtils:
    PacketUtilsIn<ServerBoundPacket> + PacketUtilsOut<ClientBoundPacket>
{
}

pub trait ClientSidePacketUtils:
    PacketUtilsIn<ClientBoundPacket> + PacketUtilsOut<ServerBoundPacket>
{
}

impl<T> ServerSidePacketUtils for T where T: AsyncBufRead + AsyncWrite + Unpin {}
impl<T> ClientSidePacketUtils for T where T: AsyncBufRead + AsyncWrite + Unpin {}

const HDR: usize = size_of::<u64>();
const MAX_PACKET: u64 = 16 * 1024 * 1024;

impl<T, P> PacketUtilsIn<P> for T
where
    T: AsyncBufRead + Unpin,
    P: PacketData,
{
    /// Ok(None) = clean EOF at a packet boundary.
    async fn read_packet(&mut self) -> std::io::Result<Option<P>> {
        // Fast path: the whole packet is already in the BufReader's buffer.
        let buf = self.fill_buf().await?;
        if buf.is_empty() {
            return Ok(None); // clean EOF
        }

        let mut len_bytes = [0u8; HDR];
        let mut header_known = false;
        if buf.len() >= HDR {
            len_bytes.copy_from_slice(&buf[..HDR]);
            header_known = true;
            let len = u64::from_le_bytes(len_bytes);
            if len > MAX_PACKET {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "packet too large",
                ));
            }
            let total = HDR + len as usize;
            if buf.len() >= total {
                let parsed = P::from_bytes(&buf[..total]);
                return match parsed {
                    Some((packet, used)) => {
                        self.consume(used); // `buf` borrow has ended
                        Ok(Some(packet))
                    }
                    None => Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "malformed packet",
                    )),
                };
            }
        }

        // Slow path: packet is split or larger than the buffer; read it exactly.
        // EOF from here on is mid-packet, so it's a real error.
        if !header_known {
            self.read_exact(&mut len_bytes).await?;
        } else {
            self.consume(HDR);
        }
        let len = u64::from_le_bytes(len_bytes);
        if len > MAX_PACKET {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "packet too large",
            ));
        }

        let mut full = vec![0u8; HDR + len as usize];
        full[..HDR].copy_from_slice(&len_bytes);
        self.read_exact(&mut full[HDR..]).await?;

        P::from_bytes(&full)
            .map(|(packet, _)| Some(packet))
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "malformed packet"))
    }
}

pub enum PacketUtilsOutError {
    FailedToConvertToBytes,
}
impl std::fmt::Debug for PacketUtilsOutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as std::fmt::Display>::fmt(self, f)
    }
}
impl std::fmt::Display for PacketUtilsOutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FailedToConvertToBytes => f.write_str("Failed to convert to bytes"),
        }
    }
}
impl std::error::Error for PacketUtilsOutError {}
impl<T, T0> PacketUtilsOut<T0> for T
where
    T: AsyncWrite + Unpin,
    T0: PacketData,
{
    async fn write_packet(&mut self, packet: T0) -> Result<(), Box<dyn std::error::Error>> {
        let data = packet
            .to_bytes()
            .ok_or(PacketUtilsOutError::FailedToConvertToBytes)?;
        self.write_all(&data).await?;
        self.flush().await?;
        Ok(())
    }
}
