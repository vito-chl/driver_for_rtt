use crate::StdData;

#[derive(Debug)]
pub enum IOError {
    OpenError,
    ReadError,
    ReadEmpty,
    WriteError,

    // 在有缓冲的时候读
    // 会发生缓冲满的情况
    // 此时返回此错误
    // 没有被发送出去的数据被传回到这里
    WriteFull(StdData),
    CloseError,
    ControlError,
    WriteBusy,
    FindError,
    RegisterError,
    DataError,
    DeviceOpsError,
}
