#![no_std]

use lazy_static::lazy_static;
#[cfg(feature = "r_core")]
pub(crate) use mlib::Mutex;
#[cfg(feature = "c_core")]
pub(crate) use rtt_rs::mutex::Mutex;

use crate::alloc::boxed::Box;
use crate::alloc::collections::BTreeMap;
use crate::alloc::string::String;
use crate::alloc::sync::Arc;
use crate::driver::Driver;
use core::any::Any;

pub mod api;
pub mod async_rw;
mod bsp;
pub mod c_api;
pub mod data;
pub mod device;
pub mod driver;
pub mod error;
mod fast_dev;
pub mod guard;

/* 导出的函数 */
pub use data::*;
pub use driver::DriverOps as DevOPS;
pub use error::*;
pub(crate) extern crate alloc;
pub(crate) extern crate rtt_rs;

lazy_static! {
    pub static ref DEVICE_LIST: Mutex<BTreeMap<String, Arc<Mutex<Driver>>>> =
        Mutex::new(BTreeMap::new()).unwrap();
}

lazy_static! {
    pub static ref FAST_DEVICE_LIST: Mutex<BTreeMap<String, Box<dyn Any + Send>>> =
        Mutex::new(BTreeMap::new()).unwrap();
}

#[macro_export]
macro_rules! dev_init {
    ($func: ident) => {
        #[no_mangle]
        pub extern "C" fn rust_device_init() -> usize {
            $func();
            0
        }
    };
}

#[allow(dead_code)]
extern "C" {
    /* enter interrupt */
    fn rt_interrupt_enter();
    /* leave interrupt */
    fn rt_interrupt_leave();
}
