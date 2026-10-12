use crate::protocol::{ExtendPacketData as _, PacketData, PartiallyExtendablePacketData as _};

#[derive(Debug)]
pub struct AvailableFile {
    pub name: String,
    pub hash: String,
}

impl AvailableFile {
    pub fn new(name: String, hash: String) -> Self {
        Self { name, hash }
    }
}
impl PacketData for AvailableFile {
    fn to_bytes(&self) -> Option<Vec<u8>> {
        let mut data = Vec::<u8>::init();
        data.extend_with_data(&self.name).ok()?;
        data.extend_with_data(&self.hash).ok()?;
        Some(data)
    }

    fn from_bytes(bytes: &[u8]) -> Option<(Self, usize)> {
        let mut offset = 0;
        let name = bytes.load_data::<String>(&mut offset).ok()?;
        let hash = bytes.load_data::<String>(&mut offset).ok()?;
        Some((Self { name, hash }, offset + <&[u8]>::EMPTY_SIZE))
    }
}
