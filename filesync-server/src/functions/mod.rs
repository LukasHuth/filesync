use filesync_lib::protocol::ServerBoundPacket;

use crate::{BoxErr, ExpBufReader, ExpWriter};

pub trait HandlePacket {
    async fn handle_packet(
        packet: ServerBoundPacket,
        reader: &mut ExpBufReader,
        writer: &mut ExpWriter,
    ) -> Result<(), BoxErr>;
}

pub mod get_available_files;
pub mod debug_message;
