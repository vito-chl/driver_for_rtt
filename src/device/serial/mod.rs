#![allow(dead_code)]

pub(crate) mod bsp;

use crate::alloc::boxed::Box;
use crate::device::serial::bsp::{BspAsyncSerial, BspSerial};
use crate::device::DeviceOps;
use crate::IOError::WriteFull;
use crate::{IOError, OpenFlag, StdData, ToMakeStdData};
use alloc::collections::{LinkedList, VecDeque};
use alloc::vec::Vec;
use core::cell::Cell;
use core::ops::Deref;
use core::task::Waker;
use rtt_rs::raw_api::{no_irq, rt_hw_interrupt_disable, rt_hw_interrupt_enable};

#[allow(dead_code)]
#[derive(Debug)]
pub enum SerialError {
    InitError,
    WriteError,
    ReadError,
    UninitError,
    BufferNull,
}

impl From<SerialError> for IOError {
    fn from(_: SerialError) -> Self {
        IOError::DeviceOpsError
    }
}

#[derive(Copy, Clone)]
pub enum SerialBaudRate {
    B9600 = 9066,
    B115200 = 115200,
}

#[derive(Copy, Clone)]
pub enum SerialConfig {
    Baud(SerialBaudRate),
    WBufSize(u32),
    RBufSize(u32),
}

impl ToMakeStdData for SerialConfig {
    fn make_data(&self) -> StdData {
        StdData::Type(Box::new(self.clone()))
    }
}

impl ToMakeStdData for u32 {
    fn make_data(&self) -> StdData {
        StdData::U32(self.clone())
    }
}

pub trait DeviceSerial {
    fn init(&self, f: &OpenFlag) -> Result<(), SerialError>;
    fn uninit(&self) -> Result<(), SerialError>;

    // helper是串口通用层提供的数据和函数，
    // 具体的串口设备需要包含helper相关的结构
    // 此函数的目的即返回所包含的helper的引用
    fn get_helper(&self) -> &BspSerial;

    // 读一个字节
    fn read_char(&self) -> Result<u8, SerialError>;
    // 判断是否可读
    fn read_able(&self) -> bool;

    // 尝试写一个字节
    fn write_char(&self, val: u8) -> Result<(), SerialError>;
    // 判断当前是否可写
    fn write_able(&self) -> bool;
    // 等待写完成
    fn write_finish(&self) -> bool;

    fn dma_write(&self, ptr: *const u8, len: usize);

    fn config_baud(&self, val: SerialBaudRate);
}

pub struct Serial<T: DeviceSerial> {
    dev: T,
    flag: Cell<Option<OpenFlag>>,
}

impl<T: DeviceSerial> Serial<T> {
    pub(crate) fn new(dev: T) -> Serial<T> {
        Serial {
            dev,
            flag: Cell::new(None),
        }
    }
}

impl<T> Deref for Serial<T>
where
    T: DeviceSerial,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.dev
    }
}

impl<T> DeviceOps for Serial<T>
where
    T: DeviceSerial,
{
    fn open(&self, flag: &OpenFlag) -> Result<(), IOError> {
        self.init(flag)?;
        self.flag.set(Some((*flag).clone()));
        Ok(())
    }

    fn read(&self, len: u32) -> Result<StdData, IOError> {
        if self.flag.get().unwrap().get_read_int() {
            Ok(no_irq(|| unsafe {
                let buf = self.dev.get_helper().r_buffer.get();
                match (*buf).pop() {
                    None => StdData::Null,
                    Some(a) => StdData::U32(a as u32),
                }
            }))
        } else if self.flag.get().unwrap().get_read_block() {
            loop {
                if self.dev.read_able() {
                    return self
                        .dev
                        .read_char()
                        .map(|a| StdData::U32(a as u32))
                        .map_err(|_| IOError::ReadError);
                }
                rtt_rs::thread::Thread::_yield();
            }
        } else {
            if !self.dev.read_able() {
                Err(IOError::ReadEmpty)
            } else {
                self.dev
                    .read_char()
                    .map(|a| StdData::U32(a as u32))
                    .map_err(|_| IOError::ReadError)
            }
        }
    }

    fn write(&self, data: &dyn ToMakeStdData) -> Result<(), IOError> {
        if self.flag.get().unwrap().get_write_block() {
            match data.make_data() {
                StdData::Bytes(a) => {
                    for ch in a {
                        loop {
                            if self.dev.write_able() {
                                break;
                            }
                            rtt_rs::thread::Thread::_yield();
                        }
                        self.dev.write_char(ch).unwrap();
                    }
                }
                StdData::U32(b) => {
                    loop {
                        if self.dev.write_able() {
                            break;
                        }
                        rtt_rs::thread::Thread::_yield();
                    }
                    self.dev.write_char(b as u8).unwrap();
                }
                _ => return Err(IOError::WriteError),
            }
            Ok(())
        } else {
            return match data.make_data() {
                StdData::Bytes(mut a) => {
                    let mut ret = Ok(());
                    if a.is_empty() {
                        ret = Ok(())
                    } else {
                        let mut dq = VecDeque::from(a);
                        no_irq(|| unsafe {
                            let wb = self.dev.get_helper().w_buffer.get();
                            if (*wb).empty() {
                                while !self.dev.read_able().unwrap() {}
                                self.dev.write_char(dq.pop_front().unwrap());
                            }
                            for ch in 0..(*wb).free_len() {
                                if let Some(ch) = a.pop_front() {
                                    (*wb).force_push(ch);
                                } else {
                                    break;
                                }
                            }
                        });
                        if !a.is_empty() {
                            ret = Err(WriteFull(StdData::Bytes(Vec::from(dq))));
                        } else {
                            ret = Ok(())
                        }
                    }
                    ret
                },
                StdData::U32(b) => no_irq(|| unsafe {
                    while !self.dev.read_able().unwrap() {}
                    self.dev.write_char(first_char);
                    Ok(())
                }),
                _ => Err(IOError::WriteError),
            };
        }
    }

    fn close(&self) -> Result<(), IOError> {
        self.dev.uninit().unwrap();
        self.flag.set(None);
        no_irq(|| unsafe {
            (*self.dev.get_helper().r_buffer.get()).clean();
            (*self.dev.get_helper().w_buffer.get()).clean();
            (*self.dev.get_helper().read_async_helper.get()) = None;
        });
        Ok(())
    }

    //  用来配置波特率等信息
    fn control(&self, data: &dyn ToMakeStdData) -> Result<(), IOError> {
        let cfg = data.make_data();
        match cfg {
            StdData::Type(a) => {
                if !a.is::<SerialConfig>() {
                    Err(IOError::ControlError)
                } else {
                    let b = a.downcast::<SerialConfig>().unwrap();
                    match *b {
                        SerialConfig::Baud(a) => {
                            self.dev.config_baud(a);
                        }
                        SerialConfig::WBufSize(a) => {
                            let hp = &self.dev.get_helper().w_buffer;
                            let wb = hp.get();
                            unsafe {
                                let level = rt_hw_interrupt_disable();
                                (*wb).resize(a as usize);
                                rt_hw_interrupt_enable(level);
                            }
                        }
                        SerialConfig::RBufSize(a) => {
                            let hp = &self.dev.get_helper().r_buffer;
                            let wb = hp.get();
                            unsafe {
                                let level = rt_hw_interrupt_disable();
                                (*wb).resize(a as usize);
                                rt_hw_interrupt_enable(level);
                            }
                        }
                    }
                    Ok(())
                }
            }
            _ => Err(IOError::ControlError),
        }
    }

    fn sync(&self) {
        loop {
            unsafe {
                let mut end = false;
                let level = rt_hw_interrupt_disable();
                let bf = self.dev.get_helper().w_buffer.get();
                if (*bf).empty() {
                    end = true;
                }
                rt_hw_interrupt_enable(level);

                if end {
                    break;
                } else {
                    rtt_rs::thread::Thread::delay(1);
                }
            }
        }
    }

    fn register_read_callback(&self, func: fn(Waker), cx: Waker) -> Result<(), IOError> {
        let level;
        unsafe {
            level = rt_hw_interrupt_disable();
        }
        // 防止中断意外响应
        let helper = self.get_helper().read_async_helper.get();
        unsafe {
            match *helper {
                None => {
                    *helper = Some(BspAsyncSerial {
                        async_wakers: {
                            let mut list = LinkedList::new();
                            list.push_back(cx);
                            list
                        },
                        async_notify: func,
                    });
                }
                Some(ref mut a) => a.async_wakers.push_back(cx),
            }
        }
        unsafe { rt_hw_interrupt_enable(level) };
        Ok(())
    }

    fn register_rx_indicate(&self, func: fn()) {
        let level;
        unsafe {
            level = rt_hw_interrupt_disable();
        }
        // 防止中断意外响应
        let f = self.get_helper().rx_indicate.get();
        unsafe {
            (*f) = Some(func);
        }
        unsafe { rt_hw_interrupt_enable(level) };
    }
}
