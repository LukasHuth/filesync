use crate::functions::HandlePacket;

pub struct GetAvailableFiles;
impl HandlePacket for GetAvailableFiles {
    async fn handle_packet(
        _packet: filesync_lib::protocol::ServerBoundPacket,
        _reader: &mut crate::ExpBufReader,
        _writer: &mut crate::ExpWriter,
    ) -> Result<(), crate::BoxErr> {
        todo!()
    }
}
