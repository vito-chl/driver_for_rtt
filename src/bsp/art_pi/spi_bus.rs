#![allow(dead_code)]

use super::DP;
use crate::device::spi_bus::bsp::BspBusSpi;
use crate::device::spi_bus::BusSpi;
use crate::device::spi_device::{SpiConfig, SpiError};
use crate::OpenFlag;

pub struct BspSpiBus;

impl BspSpiBus {
    pub fn new() -> Self {
        BspSpiBus
    }
}

impl BusSpi for BspSpiBus {
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
        spi.cr1.modify(|_, w| {
            w.ssi().set_bit();
            w
        });
        spi.cfg1.modify(|_, w| {
            w.dsize().bits(7);
            w.mbr().div256();
            w
        });
        spi.cfg2.modify(|_, w| {
            w.afcntr().set_bit();
            w.ssoe().clear_bit();
            w.ssm().set_bit();
            w.cpol().set_bit();
            w.cpha().set_bit();
            w.lsbfrst().clear_bit();
            w.master().set_bit();
            unsafe {
                w.sp().bits(0);
                w.comm().bits(0);
            }
            w
        });
        spi.cr1.modify(|_, w| w.spe().set_bit());
        spi.i2scfgr.modify(|_, w| w.i2smod().clear_bit());
        Ok(())
    }

    // fn cs(&self, f: bool) {
    //     use super::spi_gpio;
    //     if f {
    //         spi_gpio::gpio_1_cs_on()
    //     } else {
    //         spi_gpio::gpio_1_cs_off()
    //     }
    // }

    fn uninit(&self) -> Result<(), SpiError> {
        Ok(())
    }

    fn trans_bit(&self, data: u8) -> Result<u8, SpiError> {
        let spi = &DP.0.SPI1;
        spi.cr1.modify(|_, w| w.spe().set_bit());
        spi.cr1.modify(|_, w| w.cstart().set_bit());
        loop {
            if spi.sr.read().dxp().bit_is_set() {
                break;
            }
            rtt_rs::thread::Thread::_yield();
        }
        spi.txdr.write(|w| unsafe { w.bits(data as _) });
        loop {
            if spi.sr.read().rxp().bit_is_set() {
                break;
            }
            rtt_rs::thread::Thread::_yield();
        }
        let ret = spi.rxdr.read().bits() as u8;
        spi.ifcr.write(|w| {
            w.eotc().set_bit();
            w.txtfc().set_bit();
            w
        });
        spi.cr1.modify(|_, w| w.spe().clear_bit());
        Ok(ret)
    }

    fn trans_bits_dma(&self, _ptr: *const u8, _len: usize) {
        todo!()
    }

    fn get_helper(&self) -> &BspBusSpi {
        todo!()
    }
}
