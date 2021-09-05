use crate::OpenFlag;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

// TODO: 相关的错误处理还没有做
#[derive(Debug, Copy, Clone)]
pub enum I2CBusError {
    InitError,
}

pub trait SBusI2CBase {
    fn init(&self, f: &OpenFlag) -> Result<(), I2CBusError>;
    fn uninit(&self) -> Result<(), I2CBusError>;

    fn get_atomic_init_flag(&self) -> &AtomicBool;
    fn get_atomic_num(&self) -> &AtomicU32;

    fn scl_set(&self, flag: bool);
    fn sda_set(&self, flag: bool);

    fn scl_get(&self) -> bool;
    fn sda_get(&self) -> bool;

    fn get_delay_time_us(&self) -> u32;
    fn delay_us(&self, us: u32);
}

pub struct SBusI2C<T: SBusI2CBase> {
    pub(crate) bus_raw: T,
}

#[allow(dead_code)]
impl<T: SBusI2CBase> SBusI2C<T> {
    pub(crate) fn new(bus_raw: T) -> SBusI2C<T> {
        let ret = SBusI2C { bus_raw };
        ret
    }
}

#[derive(Debug, Copy, Clone)]
pub enum I2CRW {
    Read,
    Write,
}

#[derive(Debug, Copy, Clone)]
pub enum I2CAddressType {
    Bits10,
    Bits7,
    Uninit,
}

/// 目前没有支持 ack 相关的操作和 10BITS 的地址
#[allow(dead_code)]
pub struct I2CMsg {
    pub(crate) address: u16,
    pub(crate) address_type: I2CAddressType,
    pub(crate) send_ack: bool,
    pub(crate) ignore_ack: bool,
    pub(crate) rw: I2CRW,
    pub(crate) len: u16,
    pub(crate) buf: Vec<u8>,
}

/// 设备需要负责总线的初始化
pub trait SBusI2CTrans {
    fn i2c_trans(&self, msgs: &mut [I2CMsg]);

    /* 以下函数提供了自动初始化与反初始化操作，需要设备进行调用来完成 */
    fn is_i2c_init(&self) -> bool;
    fn set_uninit(&self);
    fn call_i2c_init(&self) -> Result<(), I2CBusError>;
    fn inc_open_num(&self);
    fn dec_open_num(&self) -> u32;
    fn call_i2c_uninit(&self) -> Result<(), I2CBusError>;
}

impl<T: SBusI2CBase> SBusI2CTrans for SBusI2C<T> {
    fn i2c_trans(&self, msgs: &mut [I2CMsg]) {
        let mut first = true;
        for msg in msgs {
            if first {
                first = false;
                self.i2c_start();
            } else {
                self.i2c_restart();
            }
            if let I2CRW::Read = msg.rw {
                let add = ((msg.address as u8) << 1) | 0x01;
                self.i2c_send_address(add, 1);
                self.i2c_rec_bytes(msg).unwrap();
            } else {
                let add = (msg.address as u8) << 1;
                self.i2c_send_address(add, 1);
                self.i2c_send_bytes(msg).unwrap();
            }
        }
        self.i2c_stop();
    }

    fn is_i2c_init(&self) -> bool {
        let a = self.bus_raw.get_atomic_init_flag();
        a.load(Ordering::Acquire)
    }

    fn set_uninit(&self) {
        let a = self.bus_raw.get_atomic_init_flag();
        a.store(false, Ordering::SeqCst)
    }

    fn call_i2c_init(&self) -> Result<(), I2CBusError> {
        let flag = OpenFlag::zero();
        if let Err(_) = self.bus_raw.init(&flag) {
            return Err(I2CBusError::InitError);
        }
        let a = self.bus_raw.get_atomic_init_flag();
        a.store(true, Ordering::Release);
        Ok(())
    }

    fn inc_open_num(&self) {
        let num = self.bus_raw.get_atomic_num();
        num.fetch_add(1, Ordering::SeqCst);
    }

    fn dec_open_num(&self) -> u32 {
        let num = self.bus_raw.get_atomic_num();
        num.fetch_sub(1, Ordering::SeqCst) - 1
    }

    fn call_i2c_uninit(&self) -> Result<(), I2CBusError> {
        self.bus_raw.uninit()
    }
}

#[allow(dead_code)]
impl<T: SBusI2CBase> SBusI2C<T> {
    fn i2c_delay(&self) {
        self.bus_raw.delay_us(self.bus_raw.get_delay_time_us() >> 1);
    }

    fn i2c_delay2(&self) {
        self.bus_raw.delay_us(self.bus_raw.get_delay_time_us());
    }

    fn scl_high(&self) {
        self.bus_raw.scl_set(true);
        self.i2c_delay();
    }

    fn sda_high(&self) {
        self.bus_raw.sda_set(true);
        self.i2c_delay();
    }

    fn scl_low(&self) {
        self.bus_raw.scl_set(false);
        self.i2c_delay();
    }

    fn sda_low(&self) {
        self.bus_raw.sda_set(false);
        self.i2c_delay();
    }

    fn get_sda(&self) -> bool {
        self.bus_raw.sda_get()
    }

    fn get_scl(&self) -> bool {
        self.bus_raw.scl_get()
    }

    fn i2c_start(&self) {
        self.sda_low();
        self.i2c_delay();
        self.scl_low();
    }

    fn i2c_restart(&self) {
        self.sda_high();
        self.scl_high();
        self.i2c_delay();
        self.sda_low();
        self.i2c_delay();
        self.scl_low();
    }

    fn i2c_stop(&self) {
        self.sda_low();
        self.i2c_delay();
        self.scl_high();
        self.i2c_delay();
        self.sda_high();
        self.i2c_delay();
    }

    fn i2c_wait_ack(&self) -> bool {
        self.sda_high();
        self.i2c_delay();
        self.scl_high();
        let ack = !self.get_sda();
        self.scl_low();
        ack
    }

    fn i2c_write_byte(&self, data: u8) -> bool {
        let mut i = 7;
        while i >= 0 {
            self.scl_low();
            let bits = (data >> i) & 1;
            self.bus_raw.sda_set(bits == 1);
            self.i2c_delay();
            self.scl_high();
            i -= 1;
        }
        self.scl_low();
        self.i2c_delay();

        self.i2c_wait_ack()
    }

    fn i2c_read_byte(&self) -> u8 {
        let mut data = 0u8;
        let mut i = 0;
        self.sda_high();
        self.i2c_delay();

        while i < 8 {
            data <<= 1;
            if self.get_sda() {
                data |= 1;
            }
            self.scl_low();
            self.i2c_delay2();
            i += 1;
        }
        data
    }

    fn i2c_send_bytes(&self, msg: &I2CMsg) -> Result<i32, I2CBusError> {
        let mut cnt = msg.len;
        let mut bytes = 0;
        let mut index = 0;

        while cnt > 0 {
            let ret = self.i2c_write_byte(msg.buf[index]);
            cnt -= 1;
            index += 1;
            bytes += 1;
        }

        Ok(bytes)
    }

    fn i2c_rec_bytes(&self, msg: &mut I2CMsg) -> Result<i32, I2CBusError> {
        let mut cnt = msg.len;
        let mut bytes = 0;
        let mut index = 0;

        while cnt > 0 {
            let ret = self.i2c_read_byte();

            cnt -= 1;
            index += 1;
            bytes += 1;

            msg.buf[index] = ret;

            // TODO ACK
        }

        Ok(bytes)
    }

    fn i2c_send_address(&self, addr: u8, retries: u32) {
        for i in 0..retries {
            self.i2c_write_byte(addr);
            self.i2c_stop();
            self.i2c_delay2();
            self.i2c_start();
        }
    }
}
