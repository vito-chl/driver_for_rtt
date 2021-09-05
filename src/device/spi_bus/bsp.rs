#![allow(dead_code)]
use crate::device::base::DynCycleQueue;
use core::cell::UnsafeCell;

pub struct BspBusSpi {
    pub(crate) r_buffer: UnsafeCell<DynCycleQueue<u8>>,
    pub(crate) w_buffer: UnsafeCell<DynCycleQueue<u8>>,
}
