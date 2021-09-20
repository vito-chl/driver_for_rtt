use crate::data::{StdData, ToMakeStdData};
use crate::driver::{DriverAsyncHelper, DriverOps};
use crate::error::IOError;
use crate::guard::DriverGuard;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use rtt_rs::embassy_async::executor::device_wake;

pub struct AsyncReadFuture<'a, 'c>(
    pub(crate) &'a DriverGuard<'c>,
    pub(crate) usize,
    pub(crate) u32,
);
pub struct AsyncWriteFuture<'a, 'b, 'c>(
    pub(crate) &'a DriverGuard<'c>,
    pub(crate) usize, // address
    pub(crate) &'b dyn ToMakeStdData,
);

impl<'a, 'c> Future for AsyncReadFuture<'a, 'c> {
    type Output = Result<StdData, IOError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        return match self.0.read(self.1, self.2) {
            Ok(a) => match a {
                StdData::Null => {
                    self.0
                        .raw
                        .register_read_callback(device_wake, cx.waker().clone())
                        .unwrap();
                    Poll::Pending
                }
                _ => Poll::Ready(Ok(a)),
            },
            Err(_) => Poll::Ready(Err(IOError::ReadError)),
        };
    }
}

impl<'a, 'b, 'c> Future for AsyncWriteFuture<'a, 'b, 'c> {
    type Output = Result<(), IOError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        return match self.0.write(self.1, self.2) {
            Ok(_) => Poll::Ready(Ok(())),
            Err(b) => match b {
                IOError::WriteBusy => {
                    self.0
                        .raw
                        .register_write_callback(device_wake, cx.waker().clone())
                        .unwrap();
                    Poll::Pending
                }
                _ => Poll::Ready(Err(b)),
            },
        };
    }
}
