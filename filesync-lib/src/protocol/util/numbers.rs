use crate::protocol::PacketData;

macro_rules! number_packets {
    ($($ty:ty),*) => {
        $(impl PacketData for $ty {
            fn to_bytes(&self) -> Option<Vec<u8>> {
                Some(self.to_le_bytes().to_vec())
            }

            fn from_bytes(bytes: &[u8]) -> Option<(Self, usize)> {
                bytes
                    .get(0..size_of::<Self>())
                    .and_then(|arr| arr.try_into().ok())
                    .map(|arr| (Self::from_le_bytes(arr), size_of::<Self>()))
            }
        })*
    };
}
number_packets!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
