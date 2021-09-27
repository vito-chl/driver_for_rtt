//! 提供的总线类型，驱动开发者需要构建此类型的对象
//! 并且将总线使用 Arc<Mutex<Box<T>>>类型封装
//! 该总线将被注册到 Bus 链表中
//! 供 SpiDevice 使用

#![allow(dead_code)]

pub mod bsp;
use crate::alloc::boxed::Box;
use crate::alloc::prelude::v1::Vec;
use crate::device::spi_bus::bsp::BspBusSpi;
use crate::device::spi_device::{SpiConfig, SpiError};
use crate::OpenFlag;
use core::sync::atomic::{AtomicBool, Ordering};

// SPI总线框架支持4种操作方式
// BUF + 中断
// BUF + DMA
// 阻塞访问
// 不使用 BUF
// 这4中访问模式由init指定
// 总线的初始化应在设备之前
// 由系统进行初始化
pub trait BusSpi {
    // 驱动开发者需要提供初始化函数用来初始化 SPI 总线设备
    fn init(&self, f: &OpenFlag, cfg: &SpiConfig) -> Result<(), SpiError>;
    // 无参数初始化
    fn np_init(&self) -> Result<(), SpiError>;
    // 默认的CS引脚
    // fn cs(&self, f: bool) {
    //     unimplemented!()
    // }
    // 用来关闭总线设备
    fn uninit(&self) -> Result<(), SpiError>;
    // 使用总线发送数据
    fn trans_bit(&self, data: u8) -> Result<u8, SpiError>;
    fn trans_bits_dma(&self, ptr: *const u8, len: usize);
    // 获取辅助器
    // TODO：总线的辅助器还没有被定义
    fn get_helper(&self) -> &BspBusSpi;
}

// 提供给设备使用的API
pub trait BusSpiOps {
    fn trans_bit(&self, data: u8) -> Result<u8, ()>;
    // 会产生休眠
    fn trans_bits_dam(&self, data: Vec<u8>) -> Result<Vec<u8>, ()>;
    fn sync(&self) -> Result<(), ()>;
}

pub struct BusSpiHandler<T: BusSpi> {
    pub(crate) dev: T,
    init: AtomicBool,
}

impl<T: BusSpi + Send + 'static> BusSpiHandler<T> {
    pub fn new(dev: T) -> Box<dyn BusSpiOps + Send + 'static> {
        let b = BusSpiHandler {
            dev,
            init: AtomicBool::new(false),
        };
        Box::new(b)
    }
}

// 整合一些传输算法到该实现里面
impl<T: BusSpi> BusSpiOps for BusSpiHandler<T> {
    fn trans_bit(&self, data: u8) -> Result<u8, ()> {
        if !self.init.load(Ordering::Acquire) {
            self.dev.np_init().unwrap();
            rtt_rs::thread::Thread::delay(1);
            self.init.store(true, Ordering::Release);
        }
        Ok(self.dev.trans_bit(data).unwrap())
    }

    fn trans_bits_dam(&self, data: Vec<u8>) -> Result<Vec<u8>, ()> {
        todo!()
    }

    fn sync(&self) -> Result<(), ()> {
        Ok(())
    }
}
