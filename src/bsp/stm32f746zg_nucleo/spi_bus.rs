#![allow(dead_code)]

use super::DP;
use crate::device::spi_bus::bsp::BspBusSpi;
use crate::device::spi_bus::BusSpi;
use crate::device::spi_device::{SpiConfig, SpiError};
use crate::OpenFlag;

pub struct Stm32f746SPIBus {}

impl Stm32f746SPIBus {
    pub fn new() -> Self {
        Stm32f746SPIBus {}
    }
}

impl BusSpi for Stm32f746SPIBus {
    fn init(&self, _f: &OpenFlag, _cfg: &SpiConfig) -> Result<(), SpiError> {
        todo!()
    }

    /// 无参数初始化
    /// 默认SPI1
    fn np_init(&self) -> Result<(), SpiError> {
        use super::spi_gpio::gpio_config;
        use super::spi_gpio::SpiN;

        gpio_config(SpiN::Spi1);
        let rcc = &DP.0.RCC;
        rcc.apb2enr.modify(|_, w| w.spi1en().set_bit());
        rcc.apb2rstr.modify(|_, w| w.spi1rst().set_bit());
        rcc.apb2rstr.modify(|_, w| w.spi1rst().clear_bit());

        let spi = &DP.0.SPI1;
        spi.cr2.modify(|_, w| {
            w.ds().eight_bit();
            w.frxth().set_bit();
            w
        });

        spi.cr1.modify(|_, w| {
            w.br().div32();
            w.lsbfirst().clear_bit();
            w.ssi().set_bit();
            w.ssm().set_bit();
            w.rxonly().clear_bit();
            w.cpha().set_bit();
            w.cpol().set_bit();
            w.mstr().set_bit();
            w
        });
        spi.cr1.modify(|_, w| w.spe().set_bit());
        spi.i2scfgr.modify(|_, w| w.i2smod().clear_bit());
        Ok(())
    }

    fn cs(&self, f: bool) {
        use super::spi_gpio;
        if f {
            spi_gpio::gpio_1_cs_on()
        } else {
            spi_gpio::gpio_1_cs_off()
        }
    }

    fn uninit(&self) -> Result<(), SpiError> {
        todo!()
    }

    fn trans_bit(&self, data: u8) -> Result<u8, SpiError> {
        let spi = &DP.0.SPI1;
        loop {
            if spi.sr.read().txe().bit_is_set() {
                break;
            }
            rtt_rs::thread::Thread::_yield();
        }
        spi.dr.write(|w| unsafe { w.bits(data as _) });
        loop {
            if spi.sr.read().rxne().bit_is_set() {
                break;
            }
            rtt_rs::thread::Thread::_yield();
        }
        let ret = spi.dr.read().bits() as u8;
        Ok(ret)
    }

    fn trans_bits_dma(&self, _ptr: *const u8, _len: usize) {
        todo!()
    }

    fn get_helper(&self) -> &BspBusSpi {
        todo!()
    }
}
