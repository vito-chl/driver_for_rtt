#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_must_use)]

//! 对接 rt-thread 上层的 API
//! 主要是 uart:网络 i2c_bus:传感器 spi_device:文件系统
use crate::alloc::boxed::Box;
use crate::api::find;
use crate::BTreeMap;
use crate::Driver;
use crate::{Arc, OpenFlag, StdData};
use alloc::string::String;
use lazy_static::lazy_static;
use rtt_rs::base::{CStr, CVoid};
use rtt_rs::mutex::Mutex;

// 通过设备指针寻找guard指针
lazy_static! {
    pub static ref DEVMAP: Mutex<BTreeMap<usize, usize>> = Mutex::new(BTreeMap::new()).unwrap();
}

// 返回查找设备标号
pub extern "C" fn rt_device_find(name: *const u8) -> usize {
    let name = CStr::new(name);
    let name: String = name.into();

    let dev = find(name.as_str());
    return match dev {
        Ok(dev) => {
            let dev = Box::new(dev);
            Box::into_raw(dev) as usize
        }
        Err(_) => 0,
    };
}

use crate::api::DevOpen;
use crate::driver::DriverOps;
use crate::guard::DriverGuard;
use core::mem;

pub extern "C" fn rt_device_open(device: *mut CVoid, _flag: u16) -> usize {
    let dev;
    unsafe {
        dev = Box::from_raw(device as *mut Arc<Mutex<Driver>>);
    }
    let dev_ref = Box::leak(dev);
    let dev_guard = dev_ref.open(&OpenFlag::zero());
    let dev_guard = Box::new(dev_guard);
    Box::into_raw(dev_guard) as usize
}

pub extern "C" fn rt_device_read(device: *mut CVoid) -> usize {
    let dev;
    unsafe {
        dev = Box::from_raw(device as *mut DriverGuard);
    }
    dev.read(0, 0);
    mem::forget(dev);
    0
}

pub extern "C" fn rt_device_write(device: *mut CVoid) -> usize {
    let dev;
    unsafe {
        dev = Box::from_raw(device as *mut DriverGuard);
    }
    //dev.write(0, &StdData::U32(0));
    mem::forget(dev);
    0
}
