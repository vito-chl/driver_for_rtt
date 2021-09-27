pub(crate) mod bsp;

use crate::device::DeviceOps;
use crate::{IOError, IOError::WriteFull};
use crate::{OpenFlag, StdData, ToMakeStdData};
use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use bsp::BspSerial;
use core::cell::Cell;
use core::task::Waker;
use rtt_rs::raw_api::no_irq;

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

#[derive(Copy, Clone, PartialOrd, PartialEq)]
pub enum SerialBaudRate {
    B2400 = 2400,
    B4800 = 4800,
    B9600 = 9600,
    B19200 = 19200,
    B38400 = 38400,
    B57600 = 57600,
    B115200 = 115200,
    B230400 = 230400,
    B460800 = 460800,
    B921600 = 921600,
    B2000000 = 2000000,
    B3000000 = 3000000,
}

#[derive(Copy, Clone, PartialOrd, PartialEq)]
pub enum SerialDataBits {
    B5 = 5,
    B6,
    B7,
    B8,
    B9,
}

#[derive(Copy, Clone, PartialOrd, PartialEq)]
pub enum SerialStopBits {
    B1 = 1,
    B2,
    B3,
    B4,
}

#[derive(Copy, Clone, PartialEq)]
pub enum SerialParity {
    NONE,
    ODD,
    EVEN,
}

#[derive(Copy, Clone, PartialEq)]
pub enum SerialBitOrder {
    MSB,
    LSB,
}

#[derive(Copy, Clone)]
pub enum SerialConfig {
    Baud(SerialBaudRate),
    DataBits(SerialDataBits),
    StopBits(SerialStopBits),
    Parity(SerialParity),
    BitOrder(SerialBitOrder),
    WBufSize(u32),
    RBufSize(u32),
}

impl ToMakeStdData for SerialConfig {
    fn make_data(&self) -> StdData {
        StdData::Type(Box::new(self.clone()))
    }
}

pub trait DeviceSerial {
    fn init(&self, f: &OpenFlag) -> Result<(), SerialError>;
    fn uninit(&self) -> Result<(), SerialError>;
    fn get_helper(&self) -> &BspSerial;
    fn read_char(&self) -> Result<u8, SerialError>;
    fn read_able(&self) -> bool;
    fn write_char(&self, val: u8) -> Result<(), SerialError>;
    fn write_able(&self) -> bool;
    fn write_finish(&self) -> bool;
    fn rx_irq_en(&self, f: bool);
    fn tx_irq_en(&self, f: bool);
    fn dma_write(&self, ptr: *const u8, len: usize);
    fn config_baud(&self, val: SerialBaudRate);
    fn stop_bots(&self, val: SerialStopBits);
    fn data_bits(&self, val: SerialDataBits);
    fn parity(&self, val: SerialParity);
    fn bit_order(&self, val: SerialBitOrder);
    fn update_flags(&self, f: OpenFlag) -> OpenFlag;
}

pub struct Serial<T: DeviceSerial> {
    dev: T,
    flag: Cell<Option<OpenFlag>>,
}

#[allow(dead_code)]
impl<T: DeviceSerial> Serial<T> {
    pub(crate) fn new(dev: T) -> Serial<T> {
        Serial {
            dev,
            flag: Cell::new(None),
        }
    }
}

impl<T> DeviceOps for Serial<T>
where
    T: DeviceSerial,
{
    fn open(&self, flag: &OpenFlag) -> Result<(), IOError> {
        self.dev.init(flag)?;
        if flag.get_read_int() || flag.get_read_async() {
            self.dev.rx_irq_en(true);
        }
        self.flag.set(Some((*flag).clone()));
        Ok(())
    }

    fn read(&self, len: u32) -> Result<StdData, IOError> {
        if self.flag.get().unwrap().get_read_int() || self.flag.get().unwrap().get_read_async() {
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

    // support irq send, block send
    fn write(&self, data: &dyn ToMakeStdData) -> Result<(), IOError> {
        if self.flag.get().unwrap().get_write_block() {
            match data.make_data() {
                StdData::Bytes(a) => {
                    let mut ok = true;
                    for ch in a {
                        loop {
                            if self.dev.write_able() {
                                break;
                            }
                            rtt_rs::thread::Thread::_yield();
                        }
                        if let Err(_) = self.dev.write_char(ch) {
                            ok = false;
                        }
                    }
                    if ok {
                        Ok(())
                    } else {
                        Err(IOError::WriteError)
                    }
                }
                StdData::U32(b) => {
                    loop {
                        if self.dev.write_able() {
                            break;
                        }
                        rtt_rs::thread::Thread::_yield();
                    }
                    self.dev.write_char(b as u8).map_err(|e| e.into())
                }
                _ => return Err(IOError::WriteError),
            }
        } else {
            return match data.make_data() {
                StdData::Bytes(a) => {
                    let ret;
                    if a.is_empty() {
                        ret = Ok(())
                    } else {
                        let mut dq = VecDeque::from(a);
                        no_irq(|| unsafe {
                            let wb = self.dev.get_helper().w_buffer.get();
                            for ch in 0..(*wb).free_len() {
                                if let Some(ch) = dq.pop_front() {
                                    (*wb).force_push(ch);
                                } else {
                                    break;
                                }
                            }
                            self.dev.tx_irq_en(true);
                        });
                        if !dq.is_empty() {
                            ret = Err(WriteFull(StdData::Bytes(Vec::from(dq))));
                        } else {
                            ret = Ok(())
                        }
                    }
                    ret
                }
                StdData::U32(b) => no_irq(|| {
                    while !self.dev.read_able() {}
                    self.dev.write_char(b as _).map_err(|e| e.into())
                }),
                StdData::U8(b) => no_irq(|| {
                    while !self.dev.read_able() {}
                    self.dev.write_char(b).map_err(|e| e.into())
                }),
                _ => Err(IOError::WriteError),
            };
        }
    }

    fn close(&self) -> Result<(), IOError> {
        self.dev.uninit()?;
        self.flag.set(None);
        no_irq(|| unsafe {
            (*self.dev.get_helper().r_buffer.get()).clean();
            (*self.dev.get_helper().w_buffer.get()).clean();
            self.dev.get_helper().read_async_helper.destroy();
        });
        Ok(())
    }

    fn control(&self, data: &dyn ToMakeStdData) -> Result<StdData, IOError> {
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
                            no_irq(|| unsafe {
                                (*wb).resize(a as usize);
                            });
                        }
                        SerialConfig::RBufSize(a) => {
                            let hp = &self.dev.get_helper().r_buffer;
                            let wb = hp.get();
                            no_irq(|| unsafe {
                                (*wb).resize(a as usize);
                            });
                        }
                        _ => {
                            // TODO
                        }
                    }
                    Ok(StdData::Null)
                }
            }
            _ => Err(IOError::ControlError),
        }
    }

    fn sync(&self) {
        loop {
            let mut end = false;
            no_irq(|| unsafe {
                let bf = self.dev.get_helper().w_buffer.get();
                if (*bf).empty() {
                    end = true;
                }
            });
            if end {
                break;
            } else {
                rtt_rs::thread::Thread::delay(1);
            }
        }
    }

    fn register_read_callback(&self, _func: fn(Waker), cx: Waker) -> Result<(), IOError> {
        no_irq(|| {
            let helper = self.dev.get_helper().read_async_helper.register(&cx);
        });
        Ok(())
    }

    fn register_rx_indicate(&self, func: fn()) {
        if !self.flag.get().unwrap().get_read_c_type() {
            return;
        }
        no_irq(|| unsafe {
            let f = self.dev.get_helper().rx_indicate.get();
            (*f) = Some(func);
        });
    }
}
