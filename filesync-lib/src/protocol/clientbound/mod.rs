use crate::protocol::{ExtendPacketData as _, PacketData, PartiallyExtendablePacketData as _};

#[derive(Debug)]
pub enum ClientBoundPacket {
    AvailableFiles {
        revision: u64,
        files: Vec<AvailableFile>,
    },
    DebugMessage {
        message: String,
    },
}
impl ClientBoundPacket {
    fn get_id(&self) -> u8 {
        match self {
            Self::DebugMessage { .. } => 0x00,
            Self::AvailableFiles { .. } => 0x01,
        }
    }
}

mod available_file;
pub use available_file::AvailableFile;

impl PacketData for ClientBoundPacket {
    fn to_bytes(&self) -> Option<Vec<u8>> {
        let mut result = Vec::<u8>::init();
        result.extend_with_data(&self.get_id()).ok()?;
        match self {
            ClientBoundPacket::AvailableFiles { revision, files } => {
                result.extend_with_data(revision).ok()?;
                result.extend_with_data(files).ok()?;
            }
            ClientBoundPacket::DebugMessage { message } => {
                result.extend_with_data(message).ok()?;
            }
        }
        Some(result)
    }

    fn from_bytes(bytes: &[u8]) -> Option<(Self, usize)> {
        let mut offset = 0;
        let id = bytes.load_data(&mut offset).ok()?;
        match id {
            0x00 => {
                let message = bytes.load_data(&mut offset).ok()?;
                Some((Self::DebugMessage { message }, offset))
            }
            0x01 => {
                let revision = bytes.load_data(&mut offset).ok()?;
                let files = bytes.load_data(&mut offset).ok()?;
                Some((Self::AvailableFiles { revision, files }, offset))
            }
            _ => None,
        }
    }
}
