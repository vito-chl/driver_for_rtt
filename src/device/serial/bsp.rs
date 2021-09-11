use crate::alloc::collections::LinkedList;
use crate::device::base::DynCycleQueue;
use crate::device::serial::DeviceSerial;
use core::cell::UnsafeCell;
use core::task::Waker;

pub struct BspAsyncSerial {
    pub(crate) async_wakers: LinkedList<Waker>,
    pub(crate) async_notify: fn(Waker),
}

pub struct BspSerial {
    pub(crate) read_async_helper: UnsafeCell<Option<BspAsyncSerial>>,
    pub(crate) r_buffer: UnsafeCell<DynCycleQueue<u8>>,
    pub(crate) w_buffer: UnsafeCell<DynCycleQueue<u8>>,
    // for c type
    pub(crate) rx_indicate: UnsafeCell<Option<fn()>>,
}

pub(crate) fn irq_receive_char<T: DeviceSerial>(dev: *mut T, ch: u8) {
    unsafe {
        let hp = (*dev).get_helper();
        let chs = hp.r_buffer.get();
        (*chs).force_push(ch).unwrap();
    }
}

pub(crate) fn notify_form_irq<T: DeviceSerial>(dev: *mut T) {
    unsafe {
        let helper = (*dev).get_helper().read_async_helper.get();
        match *helper {
            None => {}
            Some(ref mut hp) => {
                let waker = hp.async_wakers.pop_front();
                match waker {
                    None => {}
                    Some(a) => (hp.async_notify)(a),
                }
            }
        }
    }
}

pub(crate) fn irq_send_char<T: DeviceSerial>(dev: *mut T) {
    unsafe {
        let wb = (*dev).get_helper().w_buffer.get();
        let ch = (*wb).pop();
        match ch {
            None => {
                (*dev).tx_irq_en(false);
            }
            Some(a) => {
                (*dev).write_char(a as u8).unwrap();
            }
        }
    }
}

#[allow(dead_code)]
pub(crate) fn dma_write_data<T: DeviceSerial>(sdev: *mut BspSerial, dev: *mut T) {
    // TODO： 需要DMA对齐
    // TODO： 需要配置MPU划分写通区域，并将缓冲区放在写通区域里面
    static mut DMA_BUF: [u8; 32] = [0; 32];
    unsafe {
        let wb = (*sdev).w_buffer.get();
        let mut len = 0;
        for i in 0..32 {
            match (*wb).pop() {
                None => {
                    len = i;
                    break;
                }
                Some(a) => {
                    DMA_BUF[i] = a;
                }
            }
        }
        (*dev).dma_write(DMA_BUF.as_ptr(), len);
    }
}

pub(crate) fn call_rx_indicate<T: DeviceSerial>(dev: *mut T) {
    unsafe {
        let f = (*dev).get_helper().rx_indicate.get();
        if let Some(f) = *f {
            f()
        }
    }
}
