use crate::alloc::boxed::Box;
use alloc::string::String;
use core::ops::DerefMut;
use rtt_rs::Deref;

pub struct FastDev<T> {
    pub(crate) dev: Box<T>,
    pub(crate) name: String,
}

impl<T> Deref for FastDev<T> {
    type Target = Box<T>;

    fn deref(&self) -> &Self::Target {
        &self.dev
    }
}

impl<T> DerefMut for FastDev<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.dev
    }
}
