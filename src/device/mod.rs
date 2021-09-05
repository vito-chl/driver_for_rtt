#![allow(unused_variables)]

use crate::alloc::boxed::Box;
use crate::alloc::string::String;
use crate::alloc::sync::Arc;
use crate::data::{OpenFlag, StdData, ToMakeStdData};
use crate::driver::Driver;
use crate::error::IOError;
use crate::Mutex;
use crate::DEVICE_LIST;
use crate::FAST_DEVICE_LIST;
use core::any::Any;
use core::task::Waker;

pub mod i2c_bus;
pub mod i2c_device;
pub mod led;
pub mod serial;
pub mod serial_simple;
pub mod spi_bus;
pub mod spi_device;
pub mod spi_flash;
// 提供基础的数据结构
pub(crate) mod base;

// 该组操作函数用来被具体的驱动继承
pub trait DeviceOps {
    // 必须实现
    fn open(&self, flag: &OpenFlag) -> Result<(), IOError>;
    fn read(&self, len: u32) -> Result<StdData, IOError> {
        if self.is_block_dev() {
            panic!()
        } else {
            Err(IOError::ReadError)
        }
    }
    fn write(&self, data: &dyn ToMakeStdData) -> Result<(), IOError> {
        if self.is_block_dev() {
            panic!()
        } else {
            Err(IOError::ReadError)
        }
    }
    fn close(&self) -> Result<(), IOError>;
    fn control(&self, data: &dyn ToMakeStdData) -> Result<(), IOError>;

    fn is_block_dev(&self) -> bool {
        false
    }

    fn b_read(&self, address: usize, len: usize) -> Result<StdData, IOError> {
        unimplemented!()
    }
    fn b_write(&self, address: usize, data: &dyn ToMakeStdData) -> Result<(), IOError> {
        unimplemented!()
    }
    // 进行同步操作
    fn sync(&self) {}

    // 还有一些支持C接口注册的函数待完成

    // for async, 非必须实现
    // NOTE: device 实现规范
    // waker 在通知的时候必须转移所有权
    // "注册一次，通知一次"
    // 下次无法再通知了
    // Waker通知之前会先检查线程是否中止
    // 只要线程没被中止，异步运行时即存在，
    // 且对应协程正在被阻塞，即可进行异步的通知
    fn register_read_callback(&self, func: fn(Waker), cx: Waker) -> Result<(), IOError> {
        Ok(())
    }
    fn register_write_callback(&self, func: fn(Waker), cx: Waker) -> Result<(), IOError> {
        Ok(())
    }

    // for c-type
    fn register_rx_indicate(&self, func: fn()) {}
}

pub fn register_device<T: DeviceOps + Send + 'static>(
    raw_dev: T,
    name: &str,
) -> Result<(), IOError> {
    let mut list = DEVICE_LIST.lock().unwrap();

    let dev = Arc::new(
        Mutex::new(Driver {
            open_able: true,
            open_num: 0,
            ops: Box::pin(raw_dev),
        })
        .unwrap(),
    );

    return if let None = list.get(name) {
        list.insert(String::from(name), dev);
        Ok(())
    } else {
        Err(IOError::RegisterError)
    };
}

pub fn register_fast_device(dev: Box<dyn Any + Send>, name: &str) -> Result<(), IOError> {
    let mut list = FAST_DEVICE_LIST.lock().unwrap();

    return if let None = list.get(name) {
        list.insert(String::from(name), dev);
        Ok(())
    } else {
        Err(IOError::RegisterError)
    };
}
