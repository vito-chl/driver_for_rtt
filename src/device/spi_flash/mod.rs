//! 后期需要把spi-flash库的代码直接移动到这里来
//! 目前参数无法传递

use crate::alloc::boxed::Box;
use crate::alloc::prelude::v1::Vec;
use crate::device::spi_device::DeviceSpi;
use crate::device::DeviceOps;
use crate::third_party::spi_flash::{self, Flash, FlashAccess};
use crate::{IOError, OpenFlag, StdData, ToMakeStdData};
use core::any::Any;
use core::cell::UnsafeCell;
use core::time::Duration;

pub struct SpiFlashAccess<T: DeviceSpi> {
    dev: T,
}

impl<T: DeviceSpi> SpiFlashAccess<T> {
    pub fn new(dev: T) -> SpiFlashAccess<T> {
        SpiFlashAccess{dev}
    }
}

#[derive(Debug)]
pub enum SpiFlashError {
    SomeError,
}

#[derive(Copy, Clone)]
pub struct GetFlashInfo;

impl ToMakeStdData for GetFlashInfo {
    fn make_data(&self) -> StdData {
        StdData::Type(Box::new(GetFlashInfo))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct EraseFlash(u32);

impl ToMakeStdData for EraseFlash {
    fn make_data(&self) -> StdData {
        StdData::Type(Box::new(EraseFlash))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct SyncFlash;

impl ToMakeStdData for SyncFlash {
    fn make_data(&self) -> StdData {
        StdData::Type(Box::new(SyncFlash))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct FlashInfo {
    pub sector_count: u32,
    pub bytes_per_sector: u32,
    pub block_size: u32,
}

impl ToMakeStdData for FlashInfo {
    fn make_data(&self) -> StdData {
        StdData::Type(Box::new(self.clone()))
    }
}

impl From<SpiFlashError> for spi_flash::Error {
    fn from(err: SpiFlashError) -> spi_flash::Error {
        spi_flash::Error::Access
    }
}

impl<T: DeviceSpi> FlashAccess for SpiFlashAccess<T> {
    type Error = SpiFlashError;

    fn access_init(&self, _param: Box<dyn Any>) -> Result<(), Self::Error> {
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
        // TODO
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
            (*flash).access.access_init(Box::new(flag.clone())).unwrap();
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

    fn control(&self, data: &dyn ToMakeStdData) -> Result<StdData, IOError> {
        let data = data.make_data().take_type().unwrap();
        let mut ret = StdData::Null;

        if data.is::<GetFlashInfo>() {
            let cmd = data.downcast::<GetFlashInfo>().unwrap();
            unsafe {
                let flash = &(*self.flash.get());
                let p = flash.get_params().unwrap();

                let info = FlashInfo {
                    sector_count: (flash.capacity.unwrap() / flash.erase_size.unwrap()) as u32,
                    bytes_per_sector: flash.erase_size.unwrap() as u32,
                    // TODO
                    block_size: (flash.erase_size.unwrap() * 16) as u32,
                };
                ret = StdData::Type(Box::new(info));
            }
        }

        Ok(ret)
    }

    fn is_block_dev(&self) -> bool {
        true
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
