use super::{hal, DP};
use crate::device::base::DynCycleQueue;
use crate::device::serial::bsp::BspSerial;
use crate::device::serial::{
    bsp, DeviceSerial, SerialBaudRate, SerialBitOrder, SerialDataBits, SerialError, SerialParity,
    SerialStopBits,
};
use crate::rtt_rs::raw_api::no_irq;
use crate::OpenFlag;
use core::cell::{Cell, UnsafeCell};
use core::task::Waker;

static mut UART_DEV_PTR: [usize; 8] = [0 as _; 8];
static mut UART_FLAG: [OpenFlag; 8] = [OpenFlag::zero(); 8];

pub struct BspUart {
    num: u32,
    hp: bsp::BspSerial,
    reg: Cell<usize>,
}

fn cal_brr(clk: u32, bound: u32) -> u32 {
    (clk * 1000000 + bound / 2) / bound
}

impl BspUart {
    pub fn new(num: u32) -> Self {
        BspUart {
            num,
            hp: BspSerial {
                read_async_helper: UnsafeCell::new(None),
                r_buffer: UnsafeCell::new(DynCycleQueue::new(256)),
                w_buffer: UnsafeCell::new(DynCycleQueue::new(256)),
                rx_indicate: UnsafeCell::new(None),
            },
            reg: Cell::new(0),
        }
    }
}

impl DeviceSerial for BspUart {
    fn init(&self, f: &OpenFlag) -> Result<(), SerialError> {
        if self.num >= 8 {
            return Err(SerialError::InitError);
        } else {
            unsafe {
                UART_DEV_PTR[self.num as usize] = self as *const _ as usize;
                UART_FLAG[self.num as usize] = f.clone();
            }
        }

        let rcc = &DP.0.RCC;

        match self.num {
            1 => {
                let r = &DP.0.GPIOA;
                let ut = &DP.0.USART1;
                let baud = cal_brr(100, 115200);
                self.reg.set(hal::USART1::ptr() as usize);

                rcc.ahb4enr.modify(|_, w| w.gpioaen().set_bit());
                rcc.apb2enr.modify(|_, w| w.usart1en().set_bit());

                r.moder.modify(|_, w| {
                    w.moder9().alternate();
                    w.moder10().alternate();
                    w
                });
                r.otyper.modify(|_, w| {
                    w.ot9().push_pull();
                    w.ot10().push_pull();
                    w
                });
                r.pupdr.modify(|_, w| {
                    w.pupdr9().floating();
                    w.pupdr10().floating();
                    w
                });
                r.ospeedr.modify(|_, w| {
                    w.ospeedr9().high_speed();
                    w.ospeedr10().high_speed();
                    w
                });
                r.afrh.modify(|_, w| {
                    w.afr9().af7();
                    w.afr10().af7();
                    w
                });
                rcc.apb2rstr.modify(|_, w| w.usart1rst().set_bit());
                rcc.apb2rstr.modify(|_, w| w.usart1rst().clear_bit());

                ut.cr1.write(|w| unsafe { w.bits(0) });
                ut.brr.write(|w| unsafe { w.bits(baud) });
                ut.cr1.modify(|_, w| {
                    // w.txeie().set_bit();
                    w.rxneie().set_bit();
                    w.te().set_bit();
                    w.re().set_bit();
                    w
                });
                ut.cr1.modify(|_, w| w.ue().set_bit())
            }
            _ => {}
        }
        Ok(())
    }

    fn uninit(&self) -> Result<(), SerialError> {
        let ut = unsafe {
            let reg = self.reg.get() as *mut hal::usart1::RegisterBlock;
            &(*reg)
        };
        ut.cr1.modify(|_, w| w.ue().clear_bit());
        Ok(())
    }

    fn get_helper(&self) -> &BspSerial {
        &self.hp
    }

    fn read_char(&self) -> Result<u8, SerialError> {
        let ut = unsafe {
            let reg = self.reg.get() as *mut hal::usart1::RegisterBlock;
            &(*reg)
        };
        Ok(ut.rdr.read().bits() as _)
    }

    fn read_able(&self) -> bool {
        let ut = unsafe {
            let reg = self.reg.get() as *mut hal::usart1::RegisterBlock;
            &(*reg)
        };
        ut.isr.read().rxne().bit_is_set()
    }

    fn write_char(&self, val: u8) -> Result<(), SerialError> {
        let ut = unsafe {
            let reg = self.reg.get() as *mut hal::usart1::RegisterBlock;
            &(*reg)
        };
        ut.tdr.write(|w| unsafe { w.bits(val as u32) });
        Ok(())
    }

    fn write_able(&self) -> bool {
        let ut = unsafe {
            let reg = self.reg.get() as *mut hal::usart1::RegisterBlock;
            &(*reg)
        };
        ut.isr.read().txe().bit_is_set()
    }

    fn write_finish(&self) -> bool {
        self.write_able()
    }

    fn rx_irq_en(&self, f: bool) {
        let ut = unsafe {
            let reg = self.reg.get() as *mut hal::usart1::RegisterBlock;
            &(*reg)
        };
        ut.cr1.modify(|_, w| w.rxneie().bit(f));
    }

    fn tx_irq_en(&self, f: bool) {
        let ut = unsafe {
            let reg = self.reg.get() as *mut hal::usart1::RegisterBlock;
            &(*reg)
        };
        ut.cr1.modify(|_, w| w.txeie().bit(f));
    }

    fn dma_write(&self, _ptr: *const u8, _len: usize) {
        unimplemented!()
    }

    fn config_baud(&self, _val: SerialBaudRate) {
        unimplemented!()
    }

    fn stop_bots(&self, _val: SerialStopBits) {
        todo!()
    }

    fn data_bits(&self, _val: SerialDataBits) {
        todo!()
    }

    fn parity(&self, _val: SerialParity) {
        todo!()
    }

    fn bit_order(&self, _val: SerialBitOrder) {
        todo!()
    }

    fn update_flags(&self, _f: OpenFlag) -> OpenFlag {
        todo!()
    }
}

#[no_mangle]
pub extern "C" fn USART1_IRQHandler() {
    unsafe {
        crate::rt_interrupt_enter();
        let ut = &DP.0.USART1;
        let dev = UART_DEV_PTR[1] as *const BspUart as *mut BspUart;
        let flag = UART_FLAG[1];
        if ut.isr.read().rxne().bit_is_set() {
            let data = ut.rdr.read().bits();
            bsp::irq_receive_char(dev, data as _);
            if flag.get_read_c_type() {
                bsp::call_rx_indicate(dev);
            } else if flag.get_read_async() {
                bsp::notify_form_irq(dev);
            }
        }
        if ut.isr.read().txe().bit_is_set() {
            if flag.get_write_int() {
                bsp::irq_send_char(dev);
            }
        }
        crate::rt_interrupt_leave();
    }
}
