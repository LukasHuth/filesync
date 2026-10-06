use anyhow::{Context as _, Result, anyhow};
use base64::{Engine, engine::general_purpose::STANDARD as B64};
use smoltcp::time::Instant;
use std::time::Instant as StdInstant;

pub fn key32(b64: &str) -> Result<[u8; 32]> {
    let v = B64.decode(b64.trim()).context("invalid base64 key")?;
    v.try_into().map_err(|_| anyhow!("key must be 32 bytes"))
}

pub fn smol_now(start: StdInstant) -> Instant {
    Instant::from_millis(start.elapsed().as_millis() as i64)
}
