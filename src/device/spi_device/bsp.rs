use crate::alloc::boxed::Box;
use crate::alloc::sync::Arc;
use crate::device::spi_bus::BusSpiOps;
use crate::Mutex;

pub struct BspSpiDev {
    pub(crate) bus: Arc<Mutex<Box<dyn BusSpiOps + Send>>>,
}

impl BspSpiDev {
    pub fn new(bus: Arc<Mutex<Box<dyn BusSpiOps + Send>>>) -> Self{
        BspSpiDev {
            bus
        }
    }
}