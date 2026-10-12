use filesync_lib::protocol::{ClientBoundPacket, PacketUtilsOut as _, ServerBoundPacket};

use crate::functions::HandlePacket;

pub struct DebugMessage;
impl HandlePacket for DebugMessage {
    async fn handle_packet(
        packet: filesync_lib::protocol::ServerBoundPacket,
        _reader: &mut crate::ExpBufReader,
        writer: &mut crate::ExpWriter,
    ) -> Result<(), crate::BoxErr> {
        let ServerBoundPacket::DebugMessage { message } = packet else {
            unreachable!(
                "This branch should never be hit, expected 'DebugMessage' got '{packet:?}'"
            );
        };
        println!("Debug Message: {message}");
        Self::send("Pong!".to_string(), writer).await
    }
}
impl DebugMessage {
    pub(crate) async fn send(
        message: String,
        writer: &mut crate::ExpWriter,
    ) -> Result<(), crate::BoxErr> {
        writer
            .write_packet(ClientBoundPacket::DebugMessage { message })
            .await
    }
}
