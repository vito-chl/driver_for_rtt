#![allow(dead_code)]
use super::DP;
use crate::device::led::{DeviceLed, LedError};
use core::cell::RefCell;

pub(crate) struct BspLed(pub(crate) RefCell<RawBspLed>);

// 记录当前LED的状态
pub enum BspLedState {
    Uninit,
    On,
    Off,
}

pub struct RawBspLed {
    pub(crate) state: BspLedState,
}

impl BspLed {
    pub fn new() -> Self {
        let r = RawBspLed {
            state: BspLedState::Uninit,
        };
        BspLed { 0: RefCell::new(r) }
    }
}

impl DeviceLed for BspLed {
    fn init(&self) -> Result<(), LedError> {
        // 使能 GPIO_C 时钟
        let rcc = &DP.0.RCC;
        rcc.ahb1enr.modify(|_, w| w.gpiocen().set_bit());

        // 设置 GPIO_C
        let rb = &DP.0.GPIOC;
        // 设置 6 号引脚为输出模式, 默认推挽
        rb.moder.modify(|_, w| w.moder6().output());
        // 设置为低速输出模式
        rb.ospeedr.modify(|_, w| w.ospeedr6().low_speed());
        // 设置为下拉
        rb.pupdr.modify(|_, w| w.pupdr6().pull_down());
        Ok(())
    }

    fn on(&self) -> Result<(), LedError> {
        let rb = &DP.0.GPIOC;
        // 写入高电平
        rb.bsrr.write(|w| w.bs6().set_bit());
        self.0.borrow_mut().state = BspLedState::On;
        Ok(())
    }

    fn off(&self) -> Result<(), LedError> {
        let rb = &DP.0.GPIOC;
        // 写入高电平
        rb.bsrr.write(|w| w.br6().set_bit());
        self.0.borrow_mut().state = BspLedState::Off;
        Ok(())
    }

    fn is_on(&self) -> bool {
        if let BspLedState::On = self.0.borrow().state {
            true
        } else {
            false
        }
    }

    fn uninit(&self) -> Result<(), LedError> {
        // 设置引脚为默认的输入模式，无上下拉
        // 设置 GPIO_C
        let rb = &DP.0.GPIOC;
        // 设置 6 号引脚为输出模式, 默认推挽
        rb.moder.modify(|_, w| w.moder6().input());
        // 设置为低速输出模式
        rb.ospeedr.modify(|_, w| w.ospeedr6().low_speed());
        // 设置为下拉
        rb.pupdr.modify(|_, w| w.pupdr6().floating());
        Ok(())
    }
}
