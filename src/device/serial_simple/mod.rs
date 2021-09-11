use crate::device::DeviceOps;
use crate::{IOError, OpenFlag, StdData, ToMakeStdData};
use rtt_rs::raw_api::no_irq;

mod bsp;

// 提供一个简单的串口抽象程序
// 具体怎么实现由驱动开发者来做
// 给驱动开发者更高的自由度
pub trait DeviceSerialSimple {
    fn init(&self);
    fn uninit(&self);
    // 用来通知设备去写
    fn info_write(&self);
    // 用来获取两个东西：
    // 接收的信号量，发送的缓冲区，接收的缓冲区
    fn get_helper(&self) -> &bsp::SerialSimpleHelper;
}

pub struct SerialSimple<T: DeviceSerialSimple> {
    dev: T,
}

impl<T: DeviceSerialSimple> DeviceOps for SerialSimple<T> {
    fn open(&self, _flag: &OpenFlag) -> Result<(), IOError> {
        self.dev.init();
        Ok(())
    }

    fn read(&self, len: u32) -> Result<StdData, IOError> {
        let hp = self.dev.get_helper();
        let sem = hp.rx_sem.clone();
        sem.take_wait_forever().unwrap();
        let ch = no_irq(|| unsafe { (*hp.w_buffer.get()).pop().unwrap() });
        Ok(StdData::U32(ch as _))
    }

    fn write(&self, data: &dyn ToMakeStdData) -> Result<(), IOError> {
        let data = data.make_data();
        let hp = self.dev.get_helper();
        unsafe {
            // 在无中断的上下文执行
            // buf可能在中断环境中被使用
            no_irq(|| match data {
                StdData::Bytes(a) => {
                    for i in a {
                        (*hp.w_buffer.get()).force_push(i);
                    }
                }
                StdData::U32(a) => {
                    (*hp.w_buffer.get()).force_push(a as _);
                }
                StdData::U8(a) => {
                    (*hp.w_buffer.get()).force_push(a as _);
                }
                _ => {}
            });
        }
        Ok(())
    }

    fn close(&self) -> Result<(), IOError> {
        self.dev.uninit();
        Ok(())
    }

    fn control(&self, data: &dyn ToMakeStdData) -> Result<(), IOError> {
        unimplemented!()
    }
}
