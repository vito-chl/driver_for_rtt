use crate::data::{OpenFlag, StdData, ToMakeStdData};
use crate::device::DeviceOps;
use crate::error::IOError;
use core::ops::Deref;
use core::task::Waker;

#[allow(dead_code)]
#[derive(Debug)]
pub enum LedError {
    InitError,
    OnOFFError,
    UninitError,
}

pub enum LedState {
    On,
    Off,
}

impl ToMakeStdData for LedState {
    fn make_data(&self) -> StdData {
        match *self {
            LedState::On => StdData::U32(1),
            LedState::Off => StdData::U32(0),
        }
    }
}

pub trait ToLedState {
    fn to_led_state(&self) -> LedState;
}

impl ToLedState for StdData {
    fn to_led_state(&self) -> LedState {
        match *self {
            StdData::U32(a) => {
                if a == 1 {
                    LedState::On
                } else if a == 0 {
                    LedState::Off
                } else {
                    panic!()
                }
            }
            _ => {
                panic!()
            }
        }
    }
}

pub trait DeviceLed {
    fn init(&self) -> Result<(), LedError>;
    fn on(&self) -> Result<(), LedError>;
    fn off(&self) -> Result<(), LedError>;
    fn is_on(&self) -> bool;
    fn uninit(&self) -> Result<(), LedError>;
}

pub struct Led<T: DeviceLed> {
    pub dev: T,
}

impl<T: DeviceLed> Led<T> {
    pub fn new(dev: T) -> Led<T> {
        Led { dev }
    }
}

impl<T> Deref for Led<T>
where
    T: DeviceLed,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.dev
    }
}

impl<T> DeviceOps for Led<T>
where
    T: DeviceLed,
{
    #[allow(unused_variables)]
    fn open(&self, flag: &OpenFlag) -> Result<(), IOError> {
        self.init().or_else(|_| Err(IOError::OpenError))
    }

    #[allow(unused_variables)]
    fn read(&self, len: u32) -> Result<StdData, IOError> {
        if self.is_on() {
            Ok(StdData::U32(1))
        } else {
            Ok(StdData::U32(0))
        }
    }

    fn write(&self, data: &dyn ToMakeStdData) -> Result<(), IOError> {
        let d = data.make_data();
        match d {
            StdData::U32(a) => {
                if a == 0 {
                    self.off().unwrap();
                    Ok(())
                } else {
                    self.on().unwrap();
                    Ok(())
                }
            }
            _ => {
                panic!()
            }
        }
    }

    fn close(&self) -> Result<(), IOError> {
        self.uninit().map_err(|_| IOError::CloseError)
    }

    #[allow(unused_variables)]
    fn control(&self, data: &dyn ToMakeStdData) -> Result<StdData, IOError> {
        unimplemented!()
    }

    // LED设备不支持异步读取\写入
    #[allow(unused_variables)]
    fn register_read_callback(&self, func: fn(Waker), cx: Waker) -> Result<(), IOError> {
        unimplemented!()
    }

    #[allow(unused_variables)]
    fn register_write_callback(&self, func: fn(Waker), cx: Waker) -> Result<(), IOError> {
        unimplemented!()
    }
}
