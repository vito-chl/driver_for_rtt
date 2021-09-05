#![allow(dead_code)]
#![allow(unused_variables)]
use super::DP;
use crate::device::base::DynCycleQueue;
use crate::device::serial::bsp::BspSerial;
use crate::device::serial::DeviceSerial;
use crate::device::serial::{bsp, SerialBaudRate, SerialError};
use crate::OpenFlag;
use core::cell::UnsafeCell;

// 保存了设备的指针，由于都是被Pin住的设备，没有风险
static mut UART_DEV_PTR: [usize; 8] = [0 as _; 8];

pub struct Stm32f746Uart {
    num: u32,
    hp: bsp::BspSerial,
}

fn cal_brr(pclk2: u32, bound: u32) -> u32 {
    (pclk2 * 1000000 + bound / 2) / bound
}

impl Stm32f746Uart {
    pub fn new(num: u32) -> Self {
        Stm32f746Uart {
            num,
            hp: BspSerial {
                read_async_helper: UnsafeCell::new(None),
                r_buffer: UnsafeCell::new(DynCycleQueue::new(32)),
                w_buffer: UnsafeCell::new(DynCycleQueue::new(32)),
                rx_indicate: UnsafeCell::new(None),
                ch: 0,
            },
        }
    }
}

// NOTE： 框架目标先做一个支持阻塞发送，中断接收的设备实现
impl DeviceSerial for Stm32f746Uart {
    fn init(&self, f: &OpenFlag) -> Result<(), SerialError> {
        if self.num >= 8 {
            return Err(SerialError::InitError);
        } else {
            // TODO
            // 将地址传递到全局变量供中断使用
            unsafe {
                UART_DEV_PTR[self.num as usize] = self as *const _ as usize;
            }
        }

        // TX: PG14
        // RX: PG9

        // 时钟的配置
        let rcc = &DP.0.RCC;
        // 使能IO引脚的时钟
        rcc.ahb1enr.modify(|_, w| w.gpiogen().set_bit());
        // 使能串口6的时钟
        rcc.apb2enr.modify(|_, w| w.usart6en().set_bit());

        // 设置串口引脚
        let rg = &DP.0.GPIOG;
        rg.moder.modify(|_, w| w.moder14().alternate());
        rg.moder.modify(|_, w| w.moder9().alternate());
        rg.otyper.modify(|_, w| w.ot14().push_pull());
        rg.otyper.modify(|_, w| w.ot9().push_pull());
        rg.pupdr.modify(|_, w| w.pupdr14().floating());
        rg.pupdr.modify(|_, w| w.pupdr9().floating());
        rg.ospeedr.modify(|_, w| w.ospeedr14().high_speed());
        rg.ospeedr.modify(|_, w| w.ospeedr9().high_speed());
        rg.afrh.modify(|_, w| w.afrh14().af8());
        rg.afrh.modify(|_, w| w.afrh9().af8());

        // 串口复位
        rcc.apb2rstr.modify(|_, w| w.usart6rst().set_bit());
        rcc.apb2rstr.modify(|_, w| w.usart6rst().clear_bit());

        let ut = &DP.0.USART6;
        // 串口的配置
        ut.cr1.modify(|_, w| w.ue().clear_bit());
        // TODO: 波特率
        let baud = cal_brr(54, 115200);
        ut.cr1.write(|w| unsafe { w.bits(0) });
        ut.brr.write(|w| unsafe { w.bits(baud) });
        ut.cr1.write(|w| unsafe { w.bits(0x000C) });
        // 开启接收缓冲区非空中断
        ut.cr1.modify(|_, w| w.rxneie().set_bit());
        // 启动串口
        ut.cr1.modify(|_, w| w.ue().set_bit());
        // 串口使能
        Ok(())
    }

    fn uninit(&self) -> Result<(), SerialError> {
        let ut = &DP.0.USART6;
        ut.cr1.modify(|_, w| w.ue().clear_bit());
        Ok(())
    }

    fn get_helper(&self) -> &BspSerial {
        &self.hp
    }

    fn read_char(&self) -> Result<u8, SerialError> {
        // 从寄存器读取一个值
        let r = Ok(self.hp.ch);
        r
    }

    fn read_able(&self) -> bool {
        // TODO: 判断当前是否有数据可读
        true
    }

    fn write_char(&self, val: u8) -> Result<(), SerialError> {
        // 写一个数据
        let ut = &DP.0.USART6;
        ut.tdr.write(|w| unsafe { w.bits(val as u32) });
        Ok(())
    }

    fn write_able(&self) -> bool {
        // 判断当前是否有数据可写
        let ut = &DP.0.USART6;
        let isr = ut.isr.read().bits();
        if (isr & 0x40) == 0 {
            false
        } else {
            true
        }
    }

    fn write_finish(&self) -> bool {
        // TODO: 判断当前是否写完成
        true
    }

    fn dma_write(&self, ptr: *const u8, len: usize) {
        // TODO: 调用dma来写数据
    }

    fn config_baud(&self, val: SerialBaudRate) {
        // TODO: 控制波特率
    }
}

#[no_mangle]
pub extern "C" fn USART6_IRQHandler() {
    unsafe {
        crate::rt_interrupt_enter();
    }
    let ut = &DP.0.USART6;
    let dev;
    unsafe {
        dev = UART_DEV_PTR[6] as *const Stm32f746Uart as *mut Stm32f746Uart;
        if ut.isr.read().rxne().bit_is_set() {
            let data = ut.rdr.read().bits();
            (*dev).hp.ch = data as u8;
            match *((*dev).hp.rx_indicate.get_mut()) {
                None => {}
                Some(func) => {
                    func();
                }
            }
        }
    }
    unsafe {
        crate::rt_interrupt_leave();
    }
}
