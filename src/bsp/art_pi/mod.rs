#![allow(dead_code)]
#![allow(unused_imports)]

mod led;

use crate::dev_init;
use crate::device::register_device;
use lazy_static::lazy_static;
use rtt_rs::log;
use stm32f7::stm32f750 as hal;

lazy_static! {
    static ref DP: PH = PH {
        0: hal::Peripherals::take().unwrap()
    };
}

unsafe impl Send for PH {}
unsafe impl Sync for PH {}
struct PH(hal::Peripherals);

dev_init!(init);
pub fn init() {
    log!("Device Init start!!!");

    log!("Register LED");
    use crate::device::led::Led;
    register_device(Led::new(led::BspLed::new()), "led0").unwrap();
    log!("Register LED finished");
}
