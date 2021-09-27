#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_must_use)]

use crate::alloc::boxed::Box;
use crate::alloc::vec::Vec;
use crate::api::find;
use crate::api::DevOpen;
use crate::driver::DriverOps;
use crate::guard::DriverGuard;
use crate::BTreeMap;
use crate::Driver;
use crate::{Arc, OpenFlag, StdData};
use alloc::collections::btree_set::Union;
use alloc::string::String;
use core::cmp::min;
use core::mem;
use lazy_static::lazy_static;
use rtt_rs::base::{CStr, CVoid};
use rtt_rs::mutex::Mutex;

#[repr(C)]
union DataTrans {
    val: u32,
    bytes: [u8; 4],
}

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

pub extern "C" fn rt_device_open(device: *mut CVoid, _flag: u16) -> usize {
    let dev;
    unsafe {
        dev = Box::from_raw(device as *mut Arc<Mutex<Driver>>);
    }
    let dev_ref: &'static mut Arc<Mutex<Driver>> = Box::leak(dev);
    let dev_guard = dev_ref.open(&OpenFlag::zero());
    let dev_guard = Box::new(dev_guard);
    let dev_open_id = Box::into_raw(dev_guard) as usize;

    // 将设备插入到链表中
    let mut map = DEVMAP.lock().unwrap();
    map.insert(device as _, dev_open_id);
    dev_open_id
}

pub extern "C" fn rt_device_read(
    device: *mut CVoid,
    pos: usize,
    buffer: *mut CVoid,
    len: usize,
) -> usize {
    let dev;
    {
        let map = DEVMAP.lock().unwrap();
        unsafe {
            let dev_ptr = map.get(&(device as usize)).unwrap();
            dev = Box::from_raw(*dev_ptr as *mut DriverGuard);
        }
    }
    let data = dev.read(pos, len as u32).unwrap();
    mem::forget(dev);

    let mut r_len = 0;
    let mut buf = buffer as *mut u8;
    match data {
        StdData::Bytes(a) => {
            r_len = min(a.len(), len);
            for i in a {
                unsafe {
                    buf.write(i);
                    buf = buf.add(1);
                }
            }
        }
        StdData::U32(a) => {
            r_len = 4;
            let data = DataTrans { val: a };
            unsafe {
                for i in data.bytes {
                    buf.write(i);
                    buf.add(1);
                }
            }
        }
        StdData::U8(a) => {
            r_len = 1;
            unsafe {
                buf.write(a);
            }
        }
        _ => {}
    }
    r_len
}

pub extern "C" fn rt_device_write(
    device: *mut CVoid,
    pos: usize,
    buffer: *const CVoid,
    len: usize,
) -> usize {
    let dev;
    {
        let map = DEVMAP.lock().unwrap();
        unsafe {
            let dev_ptr = map.get(&(device as usize)).unwrap();
            dev = Box::from_raw(*dev_ptr as *mut DriverGuard);
        }
    }
    let mut c_buf = buffer as *const u8;
    let mut r_buf = Vec::new();

    for _ in 0..len {
        let d;
        unsafe {
            d = c_buf.read();
            c_buf = c_buf.add(1);
        }
        r_buf.push(d);
    }

    dev.write(pos, &r_buf);
    mem::forget(dev);
    len
}

/**
 * general device commands
 */
const RT_DEVICE_CTRL_RESUME: u8 = 0x01;
/**< resume device */
const RT_DEVICE_CTRL_SUSPEND: u8 = 0x02;
/**< suspend device */
const RT_DEVICE_CTRL_CONFIG: u8 = 0x03;
/**< configure device */
const RT_DEVICE_CTRL_CLOSE: u8 = 0x04;
/**< close device */

const RT_DEVICE_CTRL_SET_INT: u8 = 0x10;
/**< set interrupt */
const RT_DEVICE_CTRL_CLR_INT: u8 = 0x11;
/**< clear interrupt */
const RT_DEVICE_CTRL_GET_INT: u8 = 0x12;
/**< get interrupt status */

/**
 * special device commands
 */
const RT_DEVICE_CTRL_CHAR_STREAM: u8 = 0x10;
/**< stream mode on char device */
const RT_DEVICE_CTRL_BLK_GETGEOME: u8 = 0x10;
/**< get geometry information   */
const RT_DEVICE_CTRL_BLK_SYNC: u8 = 0x11;
/**< flush data to block device */
const RT_DEVICE_CTRL_BLK_ERASE: u8 = 0x12;
/**< erase block on block device */
const RT_DEVICE_CTRL_BLK_AUTOREFRESH: u8 = 0x13;
/**< block device : enter/exit auto refresh mode */
const RT_DEVICE_CTRL_NETIF_GETMAC: u8 = 0x10;
/**< get mac address */
const RT_DEVICE_CTRL_MTD_FORMAT: u8 = 0x10;
/**< format a MTD device */
const RT_DEVICE_CTRL_RTC_GET_TIME: u8 = 0x10;
/**< get time */
const RT_DEVICE_CTRL_RTC_SET_TIME: u8 = 0x11;
/**< set time */
const RT_DEVICE_CTRL_RTC_GET_ALARM: u8 = 0x12;
/**< get alarm */
const RT_DEVICE_CTRL_RTC_SET_ALARM: u8 = 0x13;
/**< set alarm */

#[allow(non_camel_case_types)]
#[repr(C)]
struct rt_device_blk_geometry {
    sector_count: u32,     /* count of sectors */
    bytes_per_sector: u32, /* number of bytes per sector */
    block_size: u32,       /* number of bytes to erase one block */
}

pub extern "C" fn rt_device_control(device: *mut CVoid, cmd: u8, arg: *mut CVoid) -> usize {
    let dev;
    {
        let map = DEVMAP.lock().unwrap();
        unsafe {
            let dev_ptr = map.get(&(device as usize)).unwrap();
            dev = Box::from_raw(*dev_ptr as *mut DriverGuard);
        }
    }

    match cmd {
        // 获取块设备信息
        RT_DEVICE_CTRL_BLK_GETGEOME => {
            use crate::device::spi_flash;
            let ctl = spi_flash::GetFlashInfo;
            let info = dev.control(&ctl).unwrap().take_type().unwrap();
            if info.is::<spi_flash::FlashInfo>() {
                let r_info = info.downcast::<spi_flash::FlashInfo>().unwrap();
                let c_info = rt_device_blk_geometry {
                    sector_count: r_info.sector_count,
                    bytes_per_sector: r_info.bytes_per_sector,
                    block_size: r_info.block_size,
                };
                let w_arg = arg as *mut rt_device_blk_geometry;
                unsafe {
                    *w_arg = c_info;
                }
            }
        }
        // 块设备同步
        RT_DEVICE_CTRL_BLK_SYNC => {
            // noting to do
        }
        // 块设备擦除
        RT_DEVICE_CTRL_BLK_ERASE => {
            // noting to do
            // auto erase block
        }
        _ => {
            unimplemented!()
        }
    }

    0
}

pub extern "C" fn rt_device_close(device: *mut CVoid) -> usize {
    let dev;
    {
        let map = DEVMAP.lock().unwrap();
        unsafe {
            let dev_ptr = map.get(&(device as usize)).unwrap();
            dev = Box::from_raw(*dev_ptr as *mut DriverGuard);
        }
    }
    let raw_dev;
    unsafe { raw_dev = Box::from_raw(device as *mut Arc<Mutex<Driver>>) }
    core::mem::drop(dev);
    core::mem::drop(raw_dev);
    0
}

pub extern "C" fn rt_device_set_tx_complete(
    _device: *mut CVoid,
    _tx_done: extern "C" fn(dev: *mut CVoid, buffer: *mut CVoid) -> usize,
) -> usize {
    0
}

pub extern "C" fn rt_device_set_rx_indicate(
    _device: *mut CVoid,
    _rx_ind: extern "C" fn(dev: *mut CVoid, buffer: usize) -> usize,
) -> usize {
    0
}
