use crate::alloc::sync::Arc;
use crate::api::OpenType;
use crate::async_rw::{AsyncReadFuture, AsyncWriteFuture};
use crate::data::{StdData, ToMakeStdData};
use crate::driver::{Driver, DriverAsyncHelper, DriverOps};
use crate::error::IOError;
use crate::Mutex;
use core::task::Waker;

pub struct DriverGuard<'a> {
    pub(crate) raw: &'a Arc<Mutex<Driver>>,
    pub(crate) o_type: OpenType,
}

impl<'c> DriverOps for DriverGuard<'c> {
    fn control(&self, data: &dyn ToMakeStdData) -> Result<StdData, IOError> {
        let dev = self.raw.lock().unwrap();
        dev.ops.control(data)
    }

    fn read(&self, address: usize, len: u32) -> Result<StdData, IOError> {
        let dev = self.raw.lock().unwrap();
        if dev.ops.is_block_dev() {
            dev.ops.b_read(address, len as usize)
        } else {
            dev.ops.read(len)
        }
    }

    fn write(&self, address: usize, data: &dyn ToMakeStdData) -> Result<(), IOError> {
        let dev = self.raw.lock().unwrap();
        if dev.ops.is_block_dev() {
            dev.ops.b_write(address, data)
        } else {
            dev.ops.write(data)
        }
    }

    fn sync(&self) {
        todo!()
    }

    fn chf(&self, _data: StdData) -> Result<StdData, IOError> {
        todo!()
    }

    fn is_only(&self) -> bool {
        match self.o_type {
            OpenType::Only => true,
            _ => false,
        }
    }

    fn is_master(&self) -> bool {
        match self.o_type {
            OpenType::Master => true,
            _ => false,
        }
    }

    fn is_user(&self) -> bool {
        match self.o_type {
            OpenType::User => true,
            _ => false,
        }
    }

    fn async_read(&self, address: usize, len: u32) -> Result<AsyncReadFuture, IOError> {
        let dev = self.raw.lock().unwrap();
        if !dev.open_able {
            Err(IOError::ReadError)
        } else {
            Ok(AsyncReadFuture {
                0: self,
                1: address,
                2: len,
            })
        }
    }

    fn async_write<'a, 'b>(
        &'a self,
        address: usize,
        data: &'b dyn ToMakeStdData,
    ) -> Result<AsyncWriteFuture<'a, 'b, 'c>, IOError> {
        let dev = self.raw.lock().unwrap();
        if !dev.open_able {
            Err(IOError::ReadError)
        } else {
            Ok(AsyncWriteFuture {
                0: self,
                1: address,
                2: data,
            })
        }
    }

    fn register_rx_indicate(&self, func: fn()) {
        let dev = self.raw.lock().unwrap();
        dev.ops.register_rx_indicate(func)
    }
}

impl DriverAsyncHelper for Arc<Mutex<Driver>> {
    fn register_read_callback(&self, func: fn(Waker), cx: Waker) -> Result<(), IOError> {
        let dev = self.lock().unwrap();
        dev.ops.register_read_callback(func, cx)
    }

    fn register_write_callback(&self, func: fn(Waker), cx: Waker) -> Result<(), IOError> {
        let dev = self.lock().unwrap();
        dev.ops.register_write_callback(func, cx)
    }
}

impl Drop for DriverGuard<'_> {
    fn drop(&mut self) {
        let mut inner_dev = self.raw.lock().unwrap();
        if inner_dev.open_able == false {
            // 独占的打开了设备
            inner_dev.open_able = true;
            inner_dev.ops.close().unwrap();
        } else if inner_dev.open_num > 1 {
            // 非独占的打开了设备
            inner_dev.open_num -= 1;
        } else if inner_dev.open_num == 1 {
            inner_dev.open_num = 0;
            inner_dev.ops.close().unwrap();
        } else {
            panic!()
        }
    }
}

#[macro_export]
macro_rules! write {
    ($dev:tt, $($data:expr),*) => {
        $($dev.write(0, &($data)).unwrap();)*
    }
}
