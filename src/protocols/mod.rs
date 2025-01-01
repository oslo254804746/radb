mod blocking;
mod tokio_async;

#[cfg(feature = "blocking")]
pub use blocking::AdbProtocol;

#[cfg(feature = "tokio_async")]
pub use tokio_async::AdbProtocol;

#[derive(Debug)]
pub enum AdbProtocolRespDataType {
    OKAY,    // 操作成功
    FAIL,    // 操作失败
    DATA,    // 响应数据
    DONE,    // 操作完成
    UNKNOWN, // 未知类型
}

impl AdbProtocolRespDataType {
    /// 将 AdbProtocolRespDataType 枚举值转换为对应的静态字符串。
    ///
    /// # 参数
    /// `self`：AdbProtocolRespDataType 枚举的一个实例。
    ///
    /// # 返回值
    /// 返回一个静态字符串，对应于枚举值的含义。如果枚举值未匹配到任何已知类型，则返回空字符串。
    pub fn as_str(&self) -> &'static str {
        match self {
            AdbProtocolRespDataType::OKAY => "OKAY", // 操作成功
            AdbProtocolRespDataType::FAIL => "FAIL", // 操作失败
            AdbProtocolRespDataType::DATA => "DATA", // 包含数据的响应
            AdbProtocolRespDataType::DONE => "DONE", // 操作完成
            _ => "",                                 // 未知或未定义的枚举值
        }
    }
}
