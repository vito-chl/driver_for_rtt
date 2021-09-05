use crate::alloc::sync::Arc;
use crate::device::base::DynCycleQueue;
use core::cell::UnsafeCell;
use rtt_rs::semaphore::Semaphore;

pub struct SerialSimpleHelper {
    pub r_buffer: UnsafeCell<DynCycleQueue<u8>>,
    pub w_buffer: UnsafeCell<DynCycleQueue<u8>>,
    pub rx_sem: Arc<Semaphore>,
}
