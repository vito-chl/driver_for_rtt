#![allow(dead_code)]

pub mod bsp;

use crate::alloc::prelude::v1::Vec;
use crate::device::spi_device::bsp::BspSpiDev;
use crate::OpenFlag;

#[allow(dead_code)]
#[derive(Debug)]
pub enum SpiError {
    InitError,
    WriteError,
    ReadError,
    UninitError,
    BufferNull,
}

#[derive(Copy, Clone)]
pub enum SpiType {
    TypeA,
    TypeB,
    TypeC,
    TypeD,
}

#[derive(Copy, Clone)]
pub struct SpiConfig {
    pub(crate) s_type: SpiType,
}

pub type SpiBits = Vec<u8>;

/* 支持使用缓冲区进行传输 */
/* 支持异步传输 */
pub trait DeviceSpi {
    fn cs(&self, f: bool);
    // 无参数初始化
    fn np_init(&self) -> Result<(), SpiError>;
    fn init(&self, f: &OpenFlag, cfg: &SpiConfig) -> Result<(), SpiError>;
    fn uninit(&self) -> Result<(), SpiError>;
    // fn trans_bits(&self, data: SpiBits) -> Result<SpiBits, SpiError>;
    // fn trans_able(&self) -> bool;
    // fn dma_trans_bits(&self, data: SpiBits) -> Result<(), SpiError>;

    fn get_helper(&self) -> &BspSpiDev;
}

// 需要实现 DeviceOps
