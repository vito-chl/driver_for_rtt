use crate::device::spi_device::bsp::BspSpiDev;
use crate::device::spi_device::{DeviceSpi, SpiError, SpiConfig};
use crate::{OpenFlag, Mutex};
use super::spi_gpio;
use crate::alloc::sync::Arc;
use crate::alloc::boxed::Box;
use crate::device::spi_bus::BusSpiOps;

pub struct BspFlashDev {
    hp: BspSpiDev,
}

impl BspFlashDev {
    pub fn new(bus: Arc<Mutex<Box<dyn BusSpiOps + Send>>>) -> Self{
        BspFlashDev {
            hp: BspSpiDev::new(bus)
        }
    }
}

impl DeviceSpi for BspFlashDev {
    fn cs(&self, f: bool) {
        if f {
            spi_gpio::gpio_1_cs_on();
        } else {
            spi_gpio::gpio_1_cs_off();
        }
    }

    fn np_init(&self) -> Result<(), SpiError> {
        Ok(())
    }

    fn init(&self, _f: &OpenFlag, _cfg: &SpiConfig) -> Result<(), SpiError> {
        Ok(())
    }

    fn uninit(&self) -> Result<(), SpiError> {
        Ok(())
    }

    fn get_helper(&self) -> &BspSpiDev {
        &self.hp
    }
}