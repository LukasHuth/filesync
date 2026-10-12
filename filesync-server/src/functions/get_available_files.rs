use crate::functions::HandlePacket;

pub struct GetAvailableFiles;
impl HandlePacket for GetAvailableFiles {
    async fn handle_packet(
        packet: filesync_lib::protocol::ServerBoundPacket,
        reader: &mut crate::ExpBufReader,
        writer: &mut crate::ExpWriter,
    ) -> Result<(), crate::BoxErr> {
        todo!()
    }
}
