#![allow(dead_code)]

use crate::alloc::boxed::Box;
use crate::alloc::sync::Arc;
use crate::device::i2c_bus::SBusI2CTrans;
use rtt_rs::mutex::Mutex;

pub type SBusInDev = Arc<Mutex<Box<dyn SBusI2CTrans>>>;

pub struct BspI2CDev {
    pub(crate) bus: SBusInDev,
}
