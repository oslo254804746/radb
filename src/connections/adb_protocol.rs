use anyhow::{anyhow, Context};
use log::info;
use std::fmt::{Display, Formatter};
use std::io::{Read, Write};
use std::net::TcpStream;

// AdbProtocolRespDataType 定义了ADB协议响应数据类型的枚举
#[derive(Debug)]
pub enum AdbProtocolRespDataType {
    OKAY,      // 操作成功
    FAIL,      // 操作失败
    DATA,      // 响应数据
    DONE,      // 操作完成
    UNKNOWN,    // 未知类型
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
            AdbProtocolRespDataType::OKAY => "OKAY",  // 操作成功
            AdbProtocolRespDataType::FAIL => "FAIL",  // 操作失败
            AdbProtocolRespDataType::DATA => "DATA",  // 包含数据的响应
            AdbProtocolRespDataType::DONE => "DONE",  // 操作完成
            _ => "",  // 未知或未定义的枚举值
        }
    }

}

// 单元测试，确保所有枚举值都被正确处理
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adb_protocol_resp_data_type_as_str() {
        assert_eq!(AdbProtocolRespDataType::OKAY.as_str(), "OKAY");
        assert_eq!(AdbProtocolRespDataType::FAIL.as_str(), "FAIL");
        assert_eq!(AdbProtocolRespDataType::DATA.as_str(), "DATA");
        assert_eq!(AdbProtocolRespDataType::DONE.as_str(), "DONE");
        assert_eq!(AdbProtocolRespDataType::UNKNOWN.as_str(), "");
    }
}


/// AdbProtocolStreamHandler trait定义了ADB协议流处理的接口。
pub trait AdbProtocolStreamHandler {
    /// 获取TcpStream的可变引用。
    fn stream(&mut self) -> &mut TcpStream;

    /// 发送数据到设备。
    ///
    /// # 参数
    /// - `data`: 要发送的数据切片。
    ///
    /// # 返回值
    /// - 成功返回发送的数据大小，失败返回错误。
    fn send(&mut self, data: &[u8]) -> anyhow::Result<usize> {
        info!(">>>>>>> Send Size: {:#?} >>>>>>>", data.len());
        let size = self.stream().write(data)?;
        Ok(size)
    }

    /// 将字节切片转换为字符串。
    ///
    /// # 参数
    /// - `data`: 要转换的字节切片。
    ///
    /// # 返回值
    /// - 成功返回转换后的字符串，失败返回错误。
    fn vec_to_string(data: &[u8]) -> anyhow::Result<String> {
        String::from_utf8(data.to_vec()).context("Failed to convert vec to string")
    }

    /// 从设备接收指定数量的数据。
    ///
    /// # 参数
    /// - `n`: 要接收的数据大小。
    ///
    /// # 返回值
    /// - 成功返回接收到的数据，失败返回错误。
    fn recv(&mut self, n: usize) -> anyhow::Result<Vec<u8>> {
        info!("<<<<<<< Try Recv Size: {:#?} <<<<<<<", n);
        let mut target = vec![0; n];
        let result = self.stream().read(&mut target)?;
        info!("<<<<<<< Recv Size: {:#?} <<<<<<<", result);
        Ok(target[..result].to_owned())
    }

    /// 发送命令到设备。
    ///
    /// # 参数
    /// - `command`: 要发送的命令字符串。
    ///
    /// # 返回值
    /// - 成功返回发送的数据大小，失败返回错误。
    fn send_command(&mut self, command: &str) -> anyhow::Result<usize> {
        info!("Send COMMAND: <{:#?}>", command);

        let cmd_bytes = command.as_bytes();
        let length = format!("{:04x}", cmd_bytes.len());
        let mut data = Vec::with_capacity(length.len() + cmd_bytes.len());
        data.extend_from_slice(length.as_bytes());
        data.extend_from_slice(cmd_bytes);
        let send_size = self.send(&data)?;
        Ok(send_size)
    }

    /// 读取指定大小的字符串。
    ///
    /// # 参数
    /// - `size`: 字符串的字节大小。
    ///
    /// # 返回值
    /// - 成功返回读取的字符串，失败返回错误。
    fn read_string(&mut self, size: usize) -> anyhow::Result<String> {
        let data = self.recv(size)?;
        Self::vec_to_string(&data)
    }


    /// 读取一个字符串块，以字符串长度开始。
    ///
    /// # 返回值
    /// - 成功返回读取的字符串，失败返回错误。
    fn read_string_block(&mut self) -> anyhow::Result<String> {
        let string_length = self.read_string(4)?;
        let string_size = usize::from_str_radix(&string_length, 16).context("Failed to parse string length")?;
        self.read_string(string_size)
    }

    /// 读取直到关闭的消息。
    ///
    /// # 返回值
    /// - 成功返回读取的全部内容，失败返回错误。
    fn read_until_close(&mut self) -> anyhow::Result<String> {
        let mut content = Vec::new();
        loop {
            let bytes_read = self.recv(4096)?;
            if bytes_read.is_empty() {
                break;
            }
            content.extend_from_slice(&bytes_read);
        }
        Self::vec_to_string(&content)
    }

    /// 检查设备返回是否为"OKAY"。
    ///
    /// # 返回值
    /// - 成功返回()`，表示检查通过，失败返回错误。
    fn check_okay(&mut self) -> anyhow::Result<()> {
        let data = self.read_string(4)?;
        info!("Check Okay Response >>> {:#?}", &data);
        if data.eq(AdbProtocolRespDataType::OKAY.as_str()) {
            return Ok(());
        }
        Err(anyhow!("Check Okay Failed"))
    }
}
