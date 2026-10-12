use crate::protocol::internal_traits::{
    ExtendPacketData as _, PacketData, PartiallyExtendablePacketData as _,
};

#[derive(Debug)]
pub enum ServerBoundPacket {
    DebugMessage { message: String },
    GetAvailableFiles,
}
impl ServerBoundPacket {
    fn get_id(&self) -> u8 {
        match self {
            Self::DebugMessage { .. } => 0x00,
            Self::GetAvailableFiles => 0x01,
        }
    }
}

impl PacketData for ServerBoundPacket {
    fn to_bytes(&self) -> Option<Vec<u8>> {
        let mut result = Vec::init();
        result.extend_with_data(&self.get_id()).ok()?;
        match self {
            ServerBoundPacket::DebugMessage { message } => {
                result.extend_with_data(message).ok()?;
            }
            ServerBoundPacket::GetAvailableFiles => (),
        }
        Some(result)
    }

    fn from_bytes(bytes: &[u8]) -> Option<(Self, usize)> {
        let mut offset = 0;
        let id: u8 = bytes.load_data(&mut offset).ok()?;
        match id {
            0x00 => {
                let message = bytes.load_data(&mut offset).ok()?;
                Some((Self::DebugMessage { message }, offset))
            }
            0x01 => Some((Self::GetAvailableFiles, offset)),
            _ => None,
        }
    }
}
