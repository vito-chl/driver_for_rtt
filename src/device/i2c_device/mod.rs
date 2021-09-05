//! I2C 总线设备
//! 提供读写 I2C 的操作
//! I2C 地址由 control 使用
//! 该驱动并不驱动任何设备，仅仅作为总线的一个访问接口

#![allow(dead_code)]

use crate::device::i2c_bus::{I2CAddressType, I2CMsg, I2CRW};
use crate::device::DeviceOps;
use crate::IOError::ControlError;
use crate::{IOError, OpenFlag, StdData, ToMakeStdData};
use alloc::vec::Vec;
use bsp::SBusInDev;
use core::cell::Cell;

mod bsp;

#[derive(Debug, Copy, Clone)]
pub enum I2CDeviceError {
    InitError,
    UninitError,
}

impl From<I2CDeviceError> for IOError {
    fn from(e: I2CDeviceError) -> Self {
        match e {
            I2CDeviceError::InitError => IOError::OpenError,
            I2CDeviceError::UninitError => IOError::CloseError,
        }
    }
}

pub trait DeviceI2C {
    fn init(&self) -> Result<(), I2CDeviceError>;
    fn uninit(&self) -> Result<(), I2CDeviceError>;
    fn get_bus(&self) -> SBusInDev;
}

pub struct I2CDev<T: DeviceI2C> {
    dev: T,
    address: Cell<u32>,
    address_type: Cell<I2CAddressType>,
}

pub struct I2CDevConfig {
    pub address: u32,
    pub address_type: I2CAddressType,
}

impl<T: DeviceI2C> I2CDev<T> {
    pub fn new(dev: T) -> I2CDev<T> {
        I2CDev {
            dev,
            address: Cell::new(0),
            address_type: Cell::new(I2CAddressType::Uninit),
        }
    }
}

impl<T: DeviceI2C> DeviceOps for I2CDev<T> {
    fn open(&self, _flag: &OpenFlag) -> Result<(), IOError> {
        let bus = self.dev.get_bus();
        {
            let bus_lock = bus.lock().unwrap();
            if !bus_lock.is_i2c_init() {
                bus_lock.call_i2c_init().unwrap();
            }
            bus_lock.inc_open_num();
        }
        self.dev.init()?;
        Ok(())
    }

    fn read(&self, p_len: u32) -> Result<StdData, IOError> {
        if self.address.get() == 0 {
            return Err(IOError::ReadError);
        }
        if let I2CAddressType::Uninit = self.address_type.get() {
            return Err(IOError::ReadError);
        }

        let mut msgs = [I2CMsg {
            address: self.address.get() as u16,
            address_type: self.address_type.get(),
            send_ack: false,
            ignore_ack: false,
            rw: I2CRW::Read,
            len: p_len as _,
            buf: Vec::new(),
        }];

        {
            let bus = self.dev.get_bus();
            let locked_bus = bus.lock().unwrap();

            locked_bus.i2c_trans(&mut msgs[..]);
        }

        let mut only_msg = None;
        for i in msgs {
            only_msg = Some(i);
        }
        let I2CMsg { buf, .. } = only_msg.unwrap();
        Ok(StdData::Bytes(buf))
    }

    fn write(&self, data: &dyn ToMakeStdData) -> Result<(), IOError> {
        let mut msgs = [I2CMsg {
            address: self.address.get() as u16,
            address_type: I2CAddressType::Bits7,
            send_ack: false,
            ignore_ack: false,
            rw: I2CRW::Write,
            len: 0,
            buf: Vec::new(),
        }];
        match data.make_data() {
            StdData::Bytes(a) => {
                msgs[0].len = a.len() as _;
                msgs[0].buf = a;
            }
            StdData::U32(a) => {
                msgs[0].len = 1;
                msgs[0].buf.push(a as _);
            }
            StdData::U8(a) => {
                msgs[0].len = 1;
                msgs[0].buf.push(a as _);
            }
            _ => {
                return Err(IOError::DataError);
            }
        }

        {
            let bus = self.dev.get_bus();
            let locked_bus = bus.lock().unwrap();

            locked_bus.i2c_trans(&mut msgs[..]);
        }

        Ok(())
    }

    fn close(&self) -> Result<(), IOError> {
        let _ = self.dev.uninit();
        let bus = self.dev.get_bus();
        {
            let bus_lock = bus.lock().unwrap();
            if bus_lock.dec_open_num() == 0 {
                bus_lock.call_i2c_uninit().unwrap();
                bus_lock.set_uninit();
            }
        }
        Ok(())
    }

    // 设置地址和地址类型
    fn control(&self, data: &dyn ToMakeStdData) -> Result<(), IOError> {
        let a = data.make_data().take_type().unwrap();
        let b;
        if a.is::<I2CDevConfig>() {
            b = a.downcast::<I2CDevConfig>().unwrap();
        } else {
            return Err(ControlError);
        }

        self.address_type.set(b.address_type);
        self.address.set(b.address);
        Ok(())
    }
}
