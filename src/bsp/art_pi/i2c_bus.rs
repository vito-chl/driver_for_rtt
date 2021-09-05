#![allow(dead_code)]

use super::DP;
use crate::device::i2c_bus::{I2CBusError, SBusI2CBase};
use crate::OpenFlag;
use core::sync::atomic::{AtomicBool, AtomicU32};
use rtt_rs::print;

pub struct Stm32f746I2CBus {
    init: AtomicBool,
    num: AtomicU32,
}

impl Stm32f746I2CBus {
    pub fn new() -> Stm32f746I2CBus {
        Stm32f746I2CBus {
            init: AtomicBool::new(false),
            num: AtomicU32::new(0),
        }
    }
}

fn inner_delay() -> i32 {
    let mut u = 0;
    for _ in 0..1000000 {
        u += 1;
    }
    u
}

fn inner_delay_ms() {
    rtt_rs::thread::Thread::delay(1);
}

impl SBusI2CBase for Stm32f746I2CBus {
    fn init(&self, _f: &OpenFlag) -> Result<(), I2CBusError> {
        // 使能 GPIO_D 时钟
        let rcc = &DP.0.RCC;
        rcc.ahb1enr.modify(|_, w| w.gpiocen().set_bit());
        rcc.ahb1enr.modify(|_, w| w.gpioben().set_bit());

        // 设置 GPIO_D
        let rc = &DP.0.GPIOC;
        let rb = &DP.0.GPIOB;
        // 设置 8.9 号引脚为输出模式, 默认推挽
        rc.moder.modify(|_, w| w.moder6().output());
        rb.moder.modify(|_, w| w.moder15().output());
        rtt_rs::thread::Thread::delay(1);
        // 设置输出模式
        // TODO: 由于没有真正的I2C总线，目前使用的是逻辑分析仪采集信号，所以设置为输出上拉
        rc.ospeedr.modify(|_, w| w.ospeedr6().low_speed());
        rb.ospeedr.modify(|_, w| w.ospeedr15().low_speed());
        rtt_rs::thread::Thread::delay(1);
        // 设置为上拉
        rc.pupdr.modify(|_, w| w.pupdr6().pull_up());
        rb.pupdr.modify(|_, w| w.pupdr15().pull_up());
        rtt_rs::thread::Thread::delay(1);

        rc.bsrr.write(|w| w.bs6().set_bit());
        rb.bsrr.write(|w| w.bs15().set_bit());
        print!("i2c io init finish!\n");
        Ok(())
    }

    fn uninit(&self) -> Result<(), I2CBusError> {
        // TODO
        self.sda_set(true);
        self.scl_set(true);
        Ok(())
    }

    fn get_atomic_init_flag(&self) -> &AtomicBool {
        &self.init
    }

    fn get_atomic_num(&self) -> &AtomicU32 {
        &self.num
    }

    fn scl_set(&self, flag: bool) {
        let rc = &DP.0.GPIOC;
        //rb.moder.modify(|_, w| w.moder14().output());
        if flag {
            rc.bsrr.write(|w| w.bs6().set_bit());
        } else {
            rc.bsrr.write(|w| w.br6().set_bit());
        }
    }

    fn sda_set(&self, flag: bool) {
        let rb = &DP.0.GPIOB;
        //rb.moder.modify(|_, w| w.moder15().output());
        //inner_delay();
        if flag {
            rb.bsrr.write(|w| w.bs15().set_bit());
        } else {
            rb.bsrr.write(|w| w.br15().set_bit());
        }
    }

    fn scl_get(&self) -> bool {
        let rb = &DP.0.GPIOD;
        rb.moder.modify(|_, w| w.moder14().input());
        inner_delay();
        print!("c");
        if rb.idr.read().idr14().bit_is_set() {
            true
        } else {
            false
        }
    }

    fn sda_get(&self) -> bool {
        let rb = &DP.0.GPIOD;
        rb.moder.modify(|_, w| w.moder15().input());
        inner_delay();
        print!("d");
        if rb.idr.read().idr15().bit_is_set() {
            true
        } else {
            false
        }
    }

    fn get_delay_time_us(&self) -> u32 {
        2
    }

    fn delay_us(&self, _us: u32) {
        // TODO : 由于rust进行了优化
        // TODO: 暂时使用ms延时函数，此时 i2c速率很慢，建议使用硬件定时
        inner_delay_ms();
    }
}
