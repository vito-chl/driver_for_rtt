use crate::alloc::string::ToString;
use crate::alloc::sync::Arc;
use crate::data::OpenFlag;
use crate::device::register_fast_device;
use crate::driver::Driver;
use crate::error::IOError;
use crate::fast_dev::FastDev;
use crate::guard::DriverGuard;
use crate::Mutex;
use crate::DEVICE_LIST;
use crate::FAST_DEVICE_LIST;

pub fn find(name: &str) -> Result<Arc<Mutex<Driver>>, IOError> {
    let list = DEVICE_LIST.lock().unwrap();

    let dev = list.get(name);

    return match dev {
        Some(dev) => Ok(dev.clone()),
        None => Err(IOError::FindError),
    };
}

pub trait DevOpen {
    fn open(&self, f: &OpenFlag) -> Result<DriverGuard, IOError>;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OpenType {
    Master,
    Only,
    User,
}

impl DevOpen for Arc<Mutex<Driver>> {
    fn open(&self, f: &OpenFlag) -> Result<DriverGuard, IOError> {
        let ot = raw_open(self, f)?;
        Ok(DriverGuard {
            raw: self,
            o_type: ot,
        })
    }
}

// 快设备：没有任何框架
// 利用反射，从注册的链表中得到之前注册的设备
// 操作也仅能使用该类型提供的操作
pub fn take_fast_dev<T: Send + 'static>(name: &str) -> Result<FastDev<T>, IOError> {
    let mut list = FAST_DEVICE_LIST.lock().unwrap();
    let dev = list.remove(name);
    return match dev {
        Some(dev) => {
            if !dev.is::<T>() {
                list.insert(name.to_string(), dev);
                return Err(IOError::FindError);
            }
            match dev.downcast::<T>() {
                Ok(d) => {
                    let ret = FastDev {
                        dev: d,
                        name: name.to_string(),
                    };
                    Ok(ret)
                }
                Err(_) => Err(IOError::FindError),
            }
        }
        None => Err(IOError::FindError),
    };
}

// 释放快设备，将设备归还到全局链表中
pub fn release_fast_dev<T: Send + 'static>(dev: FastDev<T>) {
    let FastDev {
        dev: raw_dev,
        name: raw_name,
    } = dev;
    register_fast_device(raw_dev, &raw_name).unwrap();
}

pub fn raw_open(dev: &Arc<Mutex<Driver>>, f: &OpenFlag) -> Result<OpenType, IOError> {
    let mut inner_dev = dev.lock().unwrap();

    return if f.get_only() {
        if !(inner_dev.open_able) {
            // 当前已经被独占
            Err(IOError::OpenError)
        } else if inner_dev.open_num != 0 {
            // 已经被非独占打开
            Err(IOError::OpenError)
        } else {
            inner_dev.open_able = false;
            inner_dev.ops.open(f).map_err(|e| {
                inner_dev.open_able = true;
                e
            })?;
            Ok(OpenType::Only)
        }
    } else {
        if !(inner_dev.open_able) {
            // 当前已经被独占
            Err(IOError::OpenError)
        } else if inner_dev.open_num == 0 {
            // 第一次打开
            inner_dev.open_num = 1;
            inner_dev.ops.open(f).map_err(|e| {
                inner_dev.open_num = 0;
                e
            })?;
            Ok(OpenType::Master)
        } else {
            // 已经执行过打开函数，不需要再次执行
            inner_dev.open_num += 1;
            Ok(OpenType::User)
        }
    };
}
