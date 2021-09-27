#![allow(dead_code)]
#![allow(unused_imports)]

mod led;
mod spi_bus;
mod spi_gpio;
mod uart;
mod spi_flash_dev;

use crate::bsp::art_pi::spi_bus::BspSpiBus;
use crate::dev_init;
use crate::device::register_device;
use crate::device::spi_bus::BusSpiHandler;
use crate::device::spi_bus::BusSpiOps;
use crate::Box;
use crate::Mutex;
use alloc::sync::Arc;
use lazy_static::lazy_static;
use rtt_rs::println;
pub(crate) use stm32h7::stm32h743v as hal;

lazy_static! {
    static ref DP: PH = PH {
        0: hal::Peripherals::take().unwrap()
    };
}

unsafe impl Send for PH {}
unsafe impl Sync for PH {}
struct PH(hal::Peripherals);

lazy_static! {
    pub static ref SPI_BUS1: Arc<Mutex<Box<dyn BusSpiOps + Send + 'static>>> = {
        let bsp_bus = BspSpiBus::new();
        let bus = BusSpiHandler::new(bsp_bus);
        let box_bus = Arc::new(Mutex::new(bus).unwrap());
        box_bus
    };
}

dev_init!(init);
pub fn init() {
    println!("Device Init start!!!");
    use crate::device::led::Led;
    register_device(Led::new(led::BspLed::new()), "led0").unwrap();
    println!("Register LED finished");
    use crate::device::serial::Serial;
    register_device(Serial::new(uart::BspUart::new(1)), "uart1").unwrap();
    println!("Register UART1 finished");
    use crate::device::spi_flash::SpiFlash;
    use crate::device::spi_flash::SpiFlashAccess;
    use spi_flash_dev::BspFlashDev;
    let spi_dev = BspFlashDev::new(SPI_BUS1.clone());
    register_device(SpiFlash::new(spi_dev), "flash0").unwrap();
}
