use std::fmt;
use thiserror::Error;

/// ADB操作中可能出现的错误类型
#[derive(Error, Debug)]
pub enum AdbError {
    /// 连接相关错误
    #[error("Connection failed: {message}")]
    ConnectionFailed { message: String },

    /// 设备未找到错误
    #[error("Device not found: {serial}")]
    DeviceNotFound { serial: String },

    /// 命令执行失败
    #[error("Command execution failed: {command}, reason: {reason}")]
    CommandFailed { command: String, reason: String },

    /// 协议错误
    #[error("Protocol error: {message}")]
    ProtocolError { message: String },

    /// 解析错误
    #[error("Parse error: {message}")]
    ParseError { message: String },

    /// 文件操作错误
    #[error("File operation failed: {operation} on {path}")]
    FileOperationFailed { operation: String, path: String },

    /// 网络错误
    #[error("Network error: {message}")]
    NetworkError { message: String },

    /// 超时错误
    #[error("Operation timed out after {seconds} seconds")]
    Timeout { seconds: u64 },

    /// 权限错误
    #[error("Permission denied: {message}")]
    PermissionDenied { message: String },

    /// 应用相关错误
    #[error("Application error: {package_name} - {message}")]
    ApplicationError {
        package_name: String,
        message: String,
    },

    /// IO错误的包装
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// 正则表达式错误
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    /// UTF-8编码错误
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    /// 数字解析错误
    #[error("Parse number error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),

    /// JSON解析错误
    #[cfg(feature = "serde")]
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// 时间相关错误
    #[error("Time error: {0}")]
    SystemTime(#[from] std::time::SystemTimeError),

    /// Anyhow错误的包装 - 新增
    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),

    /// 其他未分类错误
    #[error("Unknown error: {message}")]
    Unknown { message: String },
}

/// 专门用于结果类型的别名
pub type AdbResult<T> = Result<T, AdbError>;

impl AdbError {
    /// 从任何实现了Display的错误创建
    pub fn from_display<E: fmt::Display>(err: E) -> Self {
        AdbError::Unknown {
            message: err.to_string(),
        }
    }

    /// 创建连接失败错误
    pub fn connection_failed<S: Into<String>>(message: S) -> Self {
        AdbError::ConnectionFailed {
            message: message.into(),
        }
    }

    /// 创建设备未找到错误
    pub fn device_not_found<S: Into<String>>(serial: S) -> Self {
        AdbError::DeviceNotFound {
            serial: serial.into(),
        }
    }

    /// 创建命令执行失败错误
    pub fn command_failed<S1: Into<String>, S2: Into<String>>(command: S1, reason: S2) -> Self {
        AdbError::CommandFailed {
            command: command.into(),
            reason: reason.into(),
        }
    }

    /// 创建协议错误
    pub fn protocol_error<S: Into<String>>(message: S) -> Self {
        AdbError::ProtocolError {
            message: message.into(),
        }
    }

    /// 创建解析错误
    pub fn parse_error<S: Into<String>>(message: S) -> Self {
        AdbError::ParseError {
            message: message.into(),
        }
    }

    /// 创建文件操作错误
    pub fn file_operation_failed<S1: Into<String>, S2: Into<String>>(
        operation: S1,
        path: S2,
    ) -> Self {
        AdbError::FileOperationFailed {
            operation: operation.into(),
            path: path.into(),
        }
    }

    /// 创建网络错误
    pub fn network_error<S: Into<String>>(message: S) -> Self {
        AdbError::NetworkError {
            message: message.into(),
        }
    }

    /// 创建超时错误
    pub fn timeout(seconds: u64) -> Self {
        AdbError::Timeout { seconds }
    }

    /// 创建权限错误
    pub fn permission_denied<S: Into<String>>(message: S) -> Self {
        AdbError::PermissionDenied {
            message: message.into(),
        }
    }

    /// 创建应用错误
    pub fn application_error<S1: Into<String>, S2: Into<String>>(
        package_name: S1,
        message: S2,
    ) -> Self {
        AdbError::ApplicationError {
            package_name: package_name.into(),
            message: message.into(),
        }
    }

    /// 创建未知错误
    pub fn unknown<S: Into<String>>(message: S) -> Self {
        AdbError::Unknown {
            message: message.into(),
        }
    }

    /// 检查是否为可重试的错误
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            AdbError::ConnectionFailed { .. }
                | AdbError::NetworkError { .. }
                | AdbError::Timeout { .. }
                | AdbError::Io(_)
        )
    }

    /// 检查是否为致命错误（不应重试）
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            AdbError::DeviceNotFound { .. }
                | AdbError::PermissionDenied { .. }
                | AdbError::ParseError { .. }
        )
    }

    /// 获取错误的简短描述
    pub fn error_code(&self) -> &'static str {
        match self {
            AdbError::ConnectionFailed { .. } => "CONNECTION_FAILED",
            AdbError::DeviceNotFound { .. } => "DEVICE_NOT_FOUND",
            AdbError::CommandFailed { .. } => "COMMAND_FAILED",
            AdbError::ProtocolError { .. } => "PROTOCOL_ERROR",
            AdbError::ParseError { .. } => "PARSE_ERROR",
            AdbError::FileOperationFailed { .. } => "FILE_OPERATION_FAILED",
            AdbError::NetworkError { .. } => "NETWORK_ERROR",
            AdbError::Timeout { .. } => "TIMEOUT",
            AdbError::PermissionDenied { .. } => "PERMISSION_DENIED",
            AdbError::ApplicationError { .. } => "APPLICATION_ERROR",
            AdbError::Io(_) => "IO_ERROR",
            AdbError::Regex(_) => "REGEX_ERROR",
            AdbError::Utf8(_) => "UTF8_ERROR",
            AdbError::ParseInt(_) => "PARSE_INT_ERROR",
            #[cfg(feature = "serde")]
            AdbError::Json(_) => "JSON_ERROR",
            AdbError::SystemTime(_) => "SYSTEM_TIME_ERROR",
            AdbError::Anyhow(_) => "ANYHOW_ERROR",
            AdbError::Unknown { .. } => "UNKNOWN_ERROR",
        }
    }
}

/// 扩展Result类型，添加ADB特定的便利方法
pub trait AdbResultExt<T> {
    /// 将anyhow::Error转换为AdbError
    fn to_adb_error(self) -> AdbResult<T>;

    /// 添加上下文信息（重命名以避免与anyhow::Context冲突）
    fn with_adb_context<F>(self, f: F) -> AdbResult<T>
    where
        F: FnOnce() -> String;
}

impl<T> AdbResultExt<T> for anyhow::Result<T> {
    fn to_adb_error(self) -> AdbResult<T> {
        self.map_err(AdbError::Anyhow)
    }

    fn with_adb_context<F>(self, f: F) -> AdbResult<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| AdbError::Anyhow(e.context(f())))
    }
}

impl<T> AdbResultExt<T> for Result<T, std::io::Error> {
    fn to_adb_error(self) -> AdbResult<T> {
        self.map_err(AdbError::Io)
    }

    fn with_adb_context<F>(self, f: F) -> AdbResult<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| AdbError::Io(std::io::Error::new(e.kind(), format!("{}: {}", f(), e))))
    }
}

/// 用于链式错误处理的宏
#[macro_export]
macro_rules! adb_bail {
    ($err:expr) => {
        return Err($err.into())
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err($crate::errors::AdbError::unknown(format!($fmt, $($arg)*)))
    };
}

/// 用于确保条件的宏
#[macro_export]
macro_rules! adb_ensure {
    ($cond:expr, $err:expr) => {
        if !$cond {
            return Err($err.into());
        }
    };
    ($cond:expr, $fmt:expr, $($arg:tt)*) => {
        if !$cond {
            return Err($crate::errors::AdbError::unknown(format!($fmt, $($arg)*)));
        }
    };
}

/// 便利宏：将anyhow::Result转换为AdbResult
#[macro_export]
macro_rules! adb_context {
    ($result:expr, $msg:expr) => {
        $result.context($msg).map_err(AdbError::Anyhow)
    };
    ($result:expr, $fmt:expr, $($arg:tt)*) => {
        $result.context(format!($fmt, $($arg)*)).map_err(AdbError::Anyhow)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = AdbError::connection_failed("Test connection failed");
        assert_eq!(err.error_code(), "CONNECTION_FAILED");
        assert!(err.is_retryable());
        assert!(!err.is_fatal());
    }

    #[test]
    fn test_device_not_found() {
        let err = AdbError::device_not_found("emulator-5554");
        assert_eq!(err.error_code(), "DEVICE_NOT_FOUND");
        assert!(!err.is_retryable());
        assert!(err.is_fatal());
    }

    #[test]
    fn test_command_failed() {
        let err = AdbError::command_failed("shell ls", "permission denied");
        assert_eq!(err.error_code(), "COMMAND_FAILED");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_timeout_error() {
        let err = AdbError::timeout(30);
        assert_eq!(err.error_code(), "TIMEOUT");
        assert!(err.is_retryable());
        assert!(!err.is_fatal());
    }

    #[test]
    fn test_error_display() {
        let err = AdbError::application_error("com.example.app", "App crashed");
        let display_str = format!("{}", err);
        assert!(display_str.contains("com.example.app"));
        assert!(display_str.contains("App crashed"));
    }

    #[test]
    fn test_anyhow_conversion() {
        let anyhow_err = anyhow::anyhow!("Some error");
        let adb_err: AdbResult<()> = Err(anyhow_err).to_adb_error();
        assert!(matches!(adb_err, Err(AdbError::Anyhow(_))));
    }

    #[test]
    fn test_anyhow_from_conversion() {
        let anyhow_err = anyhow::anyhow!("Some error");
        let adb_err: AdbError = anyhow_err.into();
        assert!(matches!(adb_err, AdbError::Anyhow(_)));
    }
}

// 使用示例
mod examples {
    use super::*;
    use anyhow::Context;

    // 示例函数，展示如何使用改进的错误处理
    async fn example_function() -> AdbResult<()> {
        // 方法1：使用 #[from] 自动转换（推荐）
        let _result = some_anyhow_function().context("Failed to create port forward")?;

        // 方法2：使用扩展trait方法
        let _result = some_anyhow_function()
            .with_adb_context(|| "Failed to create port forward".to_string())?;

        // 方法3：使用便利宏
        let _result = adb_context!(some_anyhow_function(), "Failed to create port forward")?;

        Ok(())
    }

    // 模拟返回anyhow::Result的函数
    fn some_anyhow_function() -> anyhow::Result<()> {
        Ok(())
    }
}
