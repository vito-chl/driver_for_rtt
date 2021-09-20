use crate::alloc::vec::Vec;

// 可以使用长数据类型和值数据类型
#[derive(Debug)]
pub enum StdData {
    Bytes(Vec<u8>),
    U32(u32),
    U8(u8),
    OpenFlag(OpenFlag),
    Type(Box<dyn Any>),
    Null,
}

impl StdData {
    pub fn take_bytes(self) -> Result<Vec<u8>, StdData> {
        if let StdData::Bytes(a) = self {
            Ok(a)
        } else {
            Err(self)
        }
    }

    pub fn take_u32(self) -> Result<u32, StdData> {
        if let StdData::U32(a) = self {
            Ok(a)
        } else {
            Err(self)
        }
    }

    pub fn take_u8(self) -> Result<u8, StdData> {
        if let StdData::U8(a) = self {
            Ok(a)
        } else {
            Err(self)
        }
    }

    pub fn take_type(self) -> Result<Box<dyn Any>, StdData> {
        if let StdData::Type(a) = self {
            Ok(a)
        } else {
            Err(self)
        }
    }

    pub fn is_null(&self) -> bool {
        if let StdData::Null = self {
            true
        } else {
            false
        }
    }
}

pub trait ToMakeStdData {
    fn make_data(&self) -> StdData;
}

impl ToMakeStdData for u32 {
    fn make_data(&self) -> StdData {
        StdData::U32(self.clone())
    }
}

impl ToMakeStdData for u8 {
    fn make_data(&self) -> StdData {
        StdData::U8(self.clone())
    }
}

impl ToMakeStdData for OpenFlag {
    fn make_data(&self) -> StdData {
        StdData::OpenFlag(self.clone())
    }
}

impl ToMakeStdData for &str {
    fn make_data(&self) -> StdData {
        let mut a = Vec::new();
        for i in self.bytes() {
            a.push(i)
        }
        StdData::Bytes(a)
    }
}

use crate::alloc::boxed::Box;
use core::any::Any;
use paste::paste;
use alloc::fmt::{Formatter, Display, Debug};

#[derive(Copy, Clone)]
pub struct OpenFlag(u32);

impl Debug for OpenFlag {
    fn fmt(&self, _f: &mut Formatter<'_>) -> alloc::fmt::Result {
        todo!()
    }
}

impl Display for OpenFlag {
    fn fmt(&self, _f: &mut Formatter<'_>) -> alloc::fmt::Result {
        todo!()
    }
}

macro_rules! flag {
    ($flag: expr, $name: ident) => {
        paste! {
            // flag
            pub fn [<set_ $name>](&mut self, f: bool) -> &mut Self {
                if f {
                    self.0 |= 1 << $flag;
                } else {
                    self.0 &= !(1 << $flag);
                }
                self
            }
            pub fn [<get_ $name>](&self) -> bool {
                self.0 & (1 << $flag) != 0
            }
        }
    };
}

//  打开标志的设置函数
impl OpenFlag {
    pub const fn zero() -> Self {
        OpenFlag(0)
    }

    // 普通的读操作是尝试读，可能返回读失败

    // 普通的写操作是写到buf里面
    // 提供清空写buf操作
    // 但是当buf写满时，会发生错误

    flag!(0, only);
    // flag!(1, read_dma);
    // flag!(2, write_dma);

    // 中断操作，配合异步使用的
    flag!(3, read_int);
    // 一般中断写操作比较不常见
    flag!(4, write_int);

    // 阻塞读规范：
    // 尝试读取数据，读取不到的时候
    // yield 调用其他线程
    flag!(5, read_block);

    // 写数据，检查是否写完成
    // 没完成则yield
    flag!(6, write_block);

    flag!(7, read_c_type);

    flag!(8, read_async);
    flag!(9, write_async);
}
