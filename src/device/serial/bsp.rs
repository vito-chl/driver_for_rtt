use crate::alloc::collections::LinkedList;
use crate::device::base::DynCycleQueue;
use crate::device::serial::DeviceSerial;
use core::cell::UnsafeCell;
use core::task::Waker;

// 需要具体的设备包含
pub struct BspAsyncSerial {
    pub(crate) async_wakers: LinkedList<Waker>,
    pub(crate) async_notify: fn(Waker),
}

pub struct BspSerial {
    pub(crate) read_async_helper: UnsafeCell<Option<BspAsyncSerial>>,
    pub(crate) r_buffer: UnsafeCell<DynCycleQueue<u8>>,
    pub(crate) w_buffer: UnsafeCell<DynCycleQueue<u8>>,

    // c形式的
    // 接受中断执行中断函数
    // 将内容读取到buf
    // pub(crate) tx_complete: fn()
    pub(crate) rx_indicate: UnsafeCell<Option<fn()>>,
    // TODO
    pub(crate) ch: u8,
}

// 应该在中断中被调用，接受一个字符
fn irq_receive_char(dev: *mut BspSerial, ch: u8) {
    unsafe {
        let chs = (*dev).r_buffer.get();
        (*chs).force_push(ch).unwrap();
    }
}

// 用来异步通知的
fn notify_form_irq(dev: *mut BspSerial) {
    unsafe {
        let helper = (*dev).read_async_helper.get();
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

// 用来在中断中发送数据的
fn irq_send_char<T: DeviceSerial>(sdev: *mut BspSerial, dev: *mut T) {
    unsafe {
        let wb = (*sdev).w_buffer.get();
        let ch = (*wb).pop();
        match ch {
            None => {}
            Some(a) => {
                (*dev).write_char(a as u8).unwrap();
            }
        }
    }
}

// 用来使dam发送数据的, 该函数在DMA完成中段里面调用
fn dma_write_data<T: DeviceSerial>(sdev: *mut BspSerial, dev: *mut T) {
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

// 一般注册来的函数都是去释放一个信号量
fn call_rx_indicate(dev: *mut BspSerial) {
    unsafe {
        let f = (*dev).rx_indicate.get();
        if let Some(f) = *f {
            f()
        }
    }
}
