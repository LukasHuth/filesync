use std::collections::VecDeque;

use smoltcp::{
    phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken},
    time::Instant,
};

use crate::vpn::consts::MTU;

/// A virtual "NIC" that just queues raw IP packets. We shuttle them
/// between smoltcp and boringtun ourselves.
#[derive(Default)]
pub struct QueueDevice {
    pub(super) rx: VecDeque<Vec<u8>>, // decrypted packets coming from the tunnel
    pub(super) tx: VecDeque<Vec<u8>>, // plaintext packets to be encrypted and sent
}
pub struct QRx(Vec<u8>);
pub struct QTx<'a>(&'a mut VecDeque<Vec<u8>>);

impl RxToken for QRx {
    fn consume<R, F: FnOnce(&[u8]) -> R>(mut self, f: F) -> R {
        f(&mut self.0)
    }
}
impl<'a> TxToken for QTx<'a> {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, len: usize, f: F) -> R {
        let mut buf = vec![0u8; len];
        let r = f(&mut buf);
        self.0.push_back(buf);
        r
    }
}
impl Device for QueueDevice {
    type RxToken<'a> = QRx;
    type TxToken<'a> = QTx<'a>;

    fn receive(&mut self, _t: Instant) -> Option<(QRx, QTx<'_>)> {
        let pkt = self.rx.pop_front()?;
        Some((QRx(pkt), QTx(&mut self.tx)))
    }
    fn transmit(&mut self, _t: Instant) -> Option<QTx<'_>> {
        Some(QTx(&mut self.tx))
    }
    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ip;
        caps.max_transmission_unit = MTU;
        caps
    }
}
