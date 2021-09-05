use crate::device::led::{DeviceLed, LedError};
use core::cell::RefCell;
use rtt_rs::print;

pub(crate) struct VirtualBspLed(pub(crate) RefCell<RawBspLed>);

// 记录当前LED的状态
pub enum VirtualLedState {
    Uninit,
    On,
    Off,
}

pub struct RawBspLed {
    pub(crate) state: VirtualLedState,
}

impl VirtualBspLed {
    pub fn new() -> Self {
        let r = RawBspLed {
            state: VirtualLedState::Uninit,
        };
        VirtualBspLed { 0: RefCell::new(r) }
    }
}

impl DeviceLed for VirtualBspLed {
    fn init(&self) -> Result<(), LedError> {
        print!("OPS: init\n");
        Ok(())
    }

    fn on(&self) -> Result<(), LedError> {
        print!("OPS: on\n");
        self.0.borrow_mut().state = VirtualLedState::On;
        Ok(())
    }

    fn off(&self) -> Result<(), LedError> {
        print!("OPS: off\n");
        self.0.borrow_mut().state = VirtualLedState::Off;
        Ok(())
    }

    fn is_on(&self) -> bool {
        if let VirtualLedState::On = self.0.borrow().state {
            true
        } else {
            false
        }
    }

    fn uninit(&self) -> Result<(), LedError> {
        print!("OPS: uninit\n");
        Ok(())
    }
}
