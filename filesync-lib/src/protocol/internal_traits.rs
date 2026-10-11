pub(crate) trait PacketData: Sized {
    fn to_bytes(&self) -> Option<Vec<u8>>;
    fn from_bytes(bytes: &[u8]) -> Option<(Self, usize)>;
}
pub(crate) trait ExtendPacketData: PartiallyExtendablePacketData {
    /// Append data to an extendable data struct
    fn extend_with_data<T>(&mut self, data: &T) -> Result<(), ()>
    where
        T: PacketData;
    /// creates an basic empty instance that can be extended with data
    fn init() -> Self;
}
pub(crate) trait PartiallyExtendablePacketData {
    const EMPTY_SIZE: usize;
    /// loads data from an extendable data struct
    fn load_data<T>(&self, offset: &mut usize) -> Result<T, ()>
    where
        T: PacketData;
}
