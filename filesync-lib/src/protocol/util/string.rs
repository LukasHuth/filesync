use crate::protocol::PacketData;

impl PacketData for String {
    fn to_bytes(&self) -> Option<Vec<u8>> {
        let data = self.as_bytes();
        let mut res = Vec::with_capacity(data.len() + size_of::<u64>());
        res.extend_from_slice(&(data.len() as u64).to_le_bytes());
        res.extend_from_slice(data);
        Some(res)
    }

    fn from_bytes(bytes: &[u8]) -> Option<(Self, usize)> {
        if bytes.len() < size_of::<u64>() {
            return None;
        }
        let byte_len = u64::from_le_bytes(bytes[..size_of::<u64>()].try_into().ok()?) as usize;
        if bytes.len() < size_of::<u64>() + byte_len {
            return None;
        }
        let data = &bytes[size_of::<u64>()..size_of::<u64>() + byte_len];
        Some((
            str::from_utf8(data).ok()?.to_string(),
            size_of::<u64>() + byte_len,
        ))
    }
}
