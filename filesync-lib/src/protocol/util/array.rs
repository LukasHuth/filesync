use crate::protocol::{ExtendPacketData as _, PacketData, PartiallyExtendablePacketData as _};

impl<T> PacketData for Vec<T>
where
    T: PacketData,
{
    fn to_bytes(&self) -> Option<Vec<u8>> {
        let mut result = Vec::<u8>::init();
        result.extend_with_data(&(self.len() as u64)).ok()?;
        for entry in self {
            result.extend_with_data(entry).ok()?;
        }
        Some(result)
    }

    fn from_bytes(bytes: &[u8]) -> Option<(Self, usize)> {
        let mut offset = 0;
        let element_amount = bytes.load_data::<u64>(&mut offset).ok()?;
        let mut result = Vec::new();
        for _ in 0..element_amount {
            let element = bytes.load_data::<T>(&mut offset).ok()?;
            result.push(element);
        }
        Some((result, <&[u8]>::EMPTY_SIZE + offset))
    }
}
