#![allow(dead_code)]
#![allow(unused_variables)]

use crate::alloc::boxed::Box;
use crate::async_rw::{AsyncReadFuture, AsyncWriteFuture};
use crate::data::{StdData, ToMakeStdData};
use crate::device::DeviceOps;
use crate::error::IOError;
use core::pin::Pin;
use core::task::Waker;

// 设备抽象
pub struct Driver {
    pub(crate) open_able: bool,
    pub(crate) open_num: u32,
    pub(crate) ops: Pin<Box<dyn DeviceOps + Send>>,
}

pub trait DriverOps {
    fn control(&self, data: &dyn ToMakeStdData) -> Result<(), IOError>;
    fn read(&self, address: usize, len: u32) -> Result<StdData, IOError>;
    fn write(&self, address: usize, data: &dyn ToMakeStdData) -> Result<(), IOError>;
    // 进行同步，将没有写的写出去
    fn sync(&self);
    fn chf(&self, data: StdData) -> Result<StdData, IOError>;
    fn is_only(&self) -> bool;
    fn is_master(&self) -> bool;
    fn is_user(&self) -> bool;
    fn async_read(&self, address: usize, len: u32) -> Result<AsyncReadFuture, IOError>;
    fn async_write<'a, 'b>(
        &'a self,
        address: usize,
        data: &'b dyn ToMakeStdData,
    ) -> Result<AsyncWriteFuture<'a, 'b, '_>, IOError>;

    // 为了兼用 c api 做的接受完成函数
    fn register_rx_indicate(&self, func: fn());
}

pub(crate) trait DriverAsyncHelper {
    fn register_read_callback(&self, func: fn(Waker), cx: Waker) -> Result<(), IOError>;

    fn register_write_callback(&self, func: fn(Waker), cx: Waker) -> Result<(), IOError>;
}
