use crate::protocol::{ExtendPacketData, PacketData, PartiallyExtendablePacketData};

impl ExtendPacketData for Vec<u8> {
    fn extend_with_data<T>(&mut self, data: &T) -> Result<(), ()>
    where
        T: PacketData,
    {
        let data = data.to_bytes().ok_or(())?;
        let mut size = u64::from_le_bytes(self[0..size_of::<u64>()].try_into().map_err(|_| ())?);
        size += data.len() as u64;
        self[0..size_of::<u64>()].copy_from_slice(&size.to_le_bytes());
        self.extend_from_slice(&data);
        Ok(())
    }

    fn init() -> Self {
        0u64.to_le_bytes().to_vec()
    }
}
impl PartiallyExtendablePacketData for Vec<u8> {
    const EMPTY_SIZE: usize = <&[u8] as PartiallyExtendablePacketData>::EMPTY_SIZE;

    fn load_data<T>(&self, offset: &mut usize) -> Result<T, ()>
    where
        T: PacketData,
    {
        (&self as &[u8]).load_data::<T>(offset)
    }
}
impl PartiallyExtendablePacketData for &[u8] {
    const EMPTY_SIZE: usize = size_of::<u64>();

    fn load_data<T>(&self, offset: &mut usize) -> Result<T, ()>
    where
        T: PacketData,
    {
        let size =
            u64::from_le_bytes(self[0..size_of::<u64>()].try_into().map_err(|_| ())?) as usize;
        if size >= *offset {
            return Err(());
        }
        T::from_bytes(&self[*offset + size_of::<u64>()..])
            .ok_or(())
            .map(|(data, size)| {
                *offset += size;
                data
            })
    }
}
