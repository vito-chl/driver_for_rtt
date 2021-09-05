#![allow(dead_code)]
#![allow(unused_imports)]

mod i2c_bus;
mod led;
mod spi_bus;
mod spi_gpio;
mod uart;
mod virtual_dev;

use crate::api::{find, DevOpen};
// use crate::bsp::stm32f746zg_nucleo::i2c_bus::Stm32f746I2CBus;
use crate::bsp::stm32f746zg_nucleo::led::BspLed;
use crate::bsp::stm32f746zg_nucleo::spi_bus::Stm32f746SPIBus;
use crate::bsp::stm32f746zg_nucleo::uart::Stm32f746Uart;
use crate::bsp::stm32f746zg_nucleo::virtual_dev::VirtualBspLed;
use crate::dev_init;
use crate::device::i2c_bus::{I2CAddressType, SBusI2C};
use crate::device::i2c_bus::{I2CMsg, I2CRW};
use crate::device::led::{Led, LedState, ToLedState};
use crate::device::register_device;
use crate::device::serial::Serial;
use crate::device::spi_bus::{BusSpiHandler, BusSpiOps};
use crate::driver::DriverOps;
use crate::lazy_static;
use crate::OpenFlag;
use rtt_rs::dbg;
use rtt_rs::dlog;
use rtt_rs::log;
use rtt_rs::print;
use rtt_rs::println;
use rtt_rs::vec;
use stm32f7::stm32f7x6 as hal;

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
    log!("hello {}", 12);
    dlog!("hello {}", 123);

    register_device(Led::new(BspLed::new()), "led0").unwrap();

    dbg!("init device");
    register_device(Led::new(VirtualBspLed::new()), "vled").unwrap();

    dbg!("init uart");
    register_device(Serial::new(Stm32f746Uart::new(6)), "uart6").unwrap();

    let ut6 = find("uart6").unwrap();
    dbg!("find uart");
    let ch;
    {
        let ut = ut6
            .open(OpenFlag::zero().set_read_block(true).set_write_block(true))
            .unwrap();
        ut.write(0, &0x0Au32).unwrap();
        rtt_rs::thread::Thread::delay(10);
        let a = ut.read(0, 1).unwrap();
        ch = a.take_u32().unwrap();
    }
    dbg!("uart transmit finish : {}", ch);

    dbg!("test spi");
    let spi_ret = 0xFF;

    let spid = BusSpiHandler::new(Stm32f746SPIBus::new());
    use crate::device::spi_bus::BusSpi;
    // init
    spid.trans_bit(0x00).unwrap();
    rtt_rs::thread::Thread::delay(10);
    spid.dev.cs(true);
    spid.trans_bit(0x0A).unwrap();
    rtt_rs::thread::Thread::delay(1);
    spid.dev.cs(false);

    dbg!("test spi end: {}", spi_ret);

    // print!("i2c test!!!!\n");
    //
    // let i2cb = Stm32f746I2CBus::new();
    // let i2cb_dev = SBusI2C::new(i2cb);
    // rtt_rs::thread::Thread::delay(100);
    // let mut msg = [
    //     I2CMsg {
    //         address: 0x10 >> 1,
    //         address_type: I2CAddressType::Bits7,
    //         send_ack: false,
    //         len: 2,
    //         rw: I2CRW::Write,
    //         buf: vec![0x0A, 0x0B],
    //         ignore_ack: false
    //     },
    //     I2CMsg {
    //         address: 0x14 >> 1,
    //         address_type: I2CAddressType::Bits7,
    //         send_ack: false,
    //         len: 2,
    //         rw: I2CRW::Write,
    //         buf: vec![0x0C, 0x0D],
    //         ignore_ack: false
    //     },
    // ];
    // i2cb_dev.i2c_trans(&mut msg[..]);

    dbg!("On led0");
    {
        let dev = find("led0").unwrap();
        let open_dev = dev.open(OpenFlag::zero().set_only(true)).unwrap();
        {
            open_dev.write(0, &LedState::On).unwrap();
            rtt_rs::thread::Thread::delay(100);
            open_dev.write(0, &LedState::Off).unwrap();
            rtt_rs::thread::Thread::delay(100);
        }
    }

    dbg!("Write");
    {
        let dev = find("vled").unwrap();
        let open_dev = dev.open(OpenFlag::zero().set_only(true)).unwrap();
        {
            open_dev.write(0, &LedState::On).unwrap();
            dbg!("led Write on");
        }
    }
    dbg!("Read");
    {
        let dev = find("vled").unwrap();
        let open_dev = dev.open(OpenFlag::zero().set_only(true)).unwrap();
        {
            let t = open_dev.read(0, 1).unwrap().to_led_state();
            if let LedState::On = t {
                dbg!("led state is on");
            } else {
                dbg!("led state is off");
            }
        }
    }
}
