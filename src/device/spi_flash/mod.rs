//! 后期需要把spi-flash库的代码直接移动到这里来
//! 目前参数无法传递

use crate::alloc::prelude::v1::Vec;
use crate::device::spi_device::DeviceSpi;
use crate::device::DeviceOps;
use crate::{IOError, OpenFlag, StdData, ToMakeStdData};
use core::cell::UnsafeCell;
use core::time::Duration;
use spi_flash::{Flash, FlashAccess};

pub struct SpiFlashAccess<T: DeviceSpi> {
    dev: T,
}

#[derive(Debug)]
pub enum SpiFlashError {
    SomeError,
}

impl From<SpiFlashError> for spi_flash::Error {
    fn from(err: SpiFlashError) -> spi_flash::Error {
        spi_flash::Error::Access
    }
}

impl<T: DeviceSpi> FlashAccess for SpiFlashAccess<T> {
    type Error = SpiFlashError;

    fn access_init(&self) -> Result<(), Self::Error> {
        self.dev.np_init().unwrap();
        Ok(())
    }

    fn access_uninit(&self) -> Result<(), Self::Error> {
        self.dev.uninit().unwrap();
        Ok(())
    }

    fn exchange(&self, data: &[u8]) -> Result<Vec<u8>, Self::Error> {
        let hp = self.dev.get_helper();
        let mut ret = Vec::new();
        let mx = hp.bus.lock().unwrap();
        self.dev.cs(true);
        for i in data {
            ret.push(mx.trans_bit(*i).unwrap());
        }
        mx.sync().unwrap();
        self.dev.cs(false);
        Ok(ret)
    }

    fn delay(&self, duration: Duration) {
        let ms = duration.as_micros();
        rtt_rs::thread::Thread::delay(ms as _);
    }
}

pub struct SpiFlash<T: DeviceSpi> {
    flash: UnsafeCell<Flash<SpiFlashAccess<T>>>,
}

impl<T: DeviceSpi> SpiFlash<T> {
    pub fn new(dev: T) -> SpiFlash<T> {
        SpiFlash {
            flash: UnsafeCell::new(spi_flash::Flash::new(SpiFlashAccess { dev })),
        }
    }
}

impl<T: DeviceSpi> DeviceOps for SpiFlash<T> {
    fn open(&self, flag: &OpenFlag) -> Result<(), IOError> {
        let flash;
        unsafe {
            flash = self.flash.get();
            (*flash).access.access_init().unwrap();
        }
        Ok(())
    }

    fn close(&self) -> Result<(), IOError> {
        let flash;
        unsafe {
            flash = self.flash.get();
            (*flash).access.access_uninit().unwrap();
        };
        Ok(())
    }

    fn control(&self, data: &dyn ToMakeStdData) -> Result<(), IOError> {
        Ok(())
    }

    fn b_read(&self, address: usize, len: usize) -> Result<StdData, IOError> {
        let flash;
        let ret;
        unsafe {
            flash = self.flash.get();
            ret = (*flash).read(address as _, len as _).unwrap();
        }
        Ok(StdData::Bytes(ret))
    }

    fn b_write(&self, address: usize, data: &dyn ToMakeStdData) -> Result<(), IOError> {
        let flash;
        let d = data.make_data().take_bytes().unwrap();
        unsafe {
            flash = self.flash.get();
            (*flash).program(address as _, d.as_slice(), false).unwrap();
        }
        Ok(())
    }
}
