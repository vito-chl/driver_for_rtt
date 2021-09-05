use super::DP;
use crate::device::led::{DeviceLed, LedError};
use core::cell::RefCell;

pub(crate) struct BspLed(pub(crate) RefCell<RawBspLed>);

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
        let rcc = &DP.0.RCC;
        rcc.ahb4enr.modify(|_, w| w.gpioien().set_bit());
        let rb = &DP.0.GPIOI;
        rb.moder.modify(|_, w| w.moder8().output());
        rb.otyper.modify(|_, w| w.ot8().push_pull());
        rb.ospeedr.modify(|_, w| w.ospeedr8().low_speed());
        rb.pupdr.modify(|_, w| w.pupdr8().pull_up());
        rb.bsrr.write(|w| w.bs8().set_bit());
        Ok(())
    }

    fn on(&self) -> Result<(), LedError> {
        let rb = &DP.0.GPIOI;
        rb.bsrr.write(|w| w.br8().set_bit());
        self.0.borrow_mut().state = BspLedState::On;
        Ok(())
    }

    fn off(&self) -> Result<(), LedError> {
        let rb = &DP.0.GPIOI;
        rb.bsrr.write(|w| w.bs8().set_bit());
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
        let rb = &DP.0.GPIOI;
        rb.moder.modify(|_, w| w.moder8().input());
        rb.ospeedr.modify(|_, w| w.ospeedr8().low_speed());
        rb.pupdr.modify(|_, w| w.pupdr8().floating());
        Ok(())
    }
}
