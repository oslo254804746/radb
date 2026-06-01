
# radb

[![crates.io](https://img.shields.io/crates/v/radb)](https://crates.io/crates/radb)
[![Docs](https://docs.rs/radb/badge.svg)](https://docs.rs/radb)

A Rust implementation of [openatx/adbutils](https://github.com/openatx/adbutils) —— a library for interacting with Android devices using ADB (Android Debug Bridge).

## 📌 项目简介

`radb` 是一个基于 Rust 实现的 ADB 工具库，灵感来源于 Python 的 [adbutils](https://github.com/openatx/adbutils)，旨在为开发者提供简洁、高效且类型安全的方式来与 Android 设备进行交互。

该库支持同步和异步两种模式，并封装了常用的 ADB 操作命令，如设备管理、文件传输、shell 命令执行、日志抓取等。

## 🚀 快速开始

### 添加依赖

在你的 [Cargo.toml](file://C:\Users\wangbaofeng\RustroverProjects\radb\Cargo.toml) 文件中添加以下内容以引入 `radb`：

```toml
[dependencies]
radb = "0.1.7"
```


### 示例代码

#### 获取设备列表（blocking，默认）

```rust
use radb::{AdbClient, AdbResult};

fn main() -> AdbResult<()> {
    let mut client = AdbClient::connect("127.0.0.1:5037")?;
    let mut devices = client.iter_devices()?;

    if let Some(mut device) = devices.next() {
        println!("{:#?}", &device.serial);
        let output = device.shell(["echo", "Hello Android"])?;
        println!("{}", output);
    }

    Ok(())
}
```

#### 获取设备列表（Tokio async）

```rust
use radb::{AdbClient, AdbResult};

#[tokio::main]
async fn main() -> AdbResult<()> {
    let mut client = AdbClient::connect("127.0.0.1:5037").await?;
    let devices = client.list_devices().await?;

    for mut device in devices {
        println!("{:#?}", &device.serial);
        let output = device.shell(["echo", "Hello Android"]).await?;
        println!("{}", output);
    }

    Ok(())
}
```


## ✅ 支持的功能

| 功能 | 描述 |
|------|------|
| ✅ 设备管理 | 列出连接的设备、获取设备状态、通过序列号或 transport ID 获取设备 |
| ✅ ADB Server 控制 | 获取版本、启动/关闭 server、连接/断开设备 |
| ✅ Shell 执行 | 在设备上运行 shell 命令 |
| ✅ 文件操作 | 推送文件到设备、从设备拉取文件、列出目录内容 |
| ✅ 网络控制 | 设置 TCP/IP 模式、转发端口 |
| ✅ UI 自动化 | 模拟点击、滑动、按键事件 |
| ✅ 应用管理 | 安装、卸载应用 |
| ✅ 日志抓取 | 实时获取设备日志（logcat） |
| ✅ 截图 | 抓取设备屏幕截图 |
| ✅ 设备控制 | 开关屏幕、切换飞行模式、Wi-Fi 等 |

## 🔧 特性开关（Features）

`radb` 提供以下特性开关供选择使用：

- `blocking`：启用同步 API（默认）
- `tokio_async`：启用异步 API 支持（需要配合 `tokio` 运行时）

你可以根据需求选择启用不同的特性：

```toml
[dependencies.radb]
version = "0.1.7"
features = ["tokio_async"]
```

`blocking` 和 `tokio_async` 不能同时启用。两套 public API 共享同一套 ADB 命令构造、响应解析和 sync 文件传输核心；区别只在连接、读写和 `await` 适配层。`AdbClient::default()` 和 `AdbClient::new(...)` 仍保留兼容，但连接失败会 panic；新代码建议使用 `AdbClient::connect(...)`。

## 🧪 测试

默认测试不需要真机：

```powershell
cargo test --lib
cargo test --test protocol_regressions
cargo test --no-run
```

真机集成测试需要显式设置 `RADB_TEST_SERIAL`：

```powershell
$env:RADB_TEST_SERIAL="emulator-5554"
cargo test --test blocking_integration
cargo test --no-default-features --features tokio_async --test async_integration
```

未设置 `RADB_TEST_SERIAL` 时，真机测试会直接跳过。


## 📦 文档

完整的 API 文档请查看 [docs.rs/radb](https://docs.rs/radb)。

## 📦 Crates 链接

- [Crates.io](https://crates.io/crates/radb)

## 🛠 贡献

欢迎提交 PR 或 issue！如果你有新功能建议或发现了 bug，请随时提出。

## 📄 许可证

该项目采用 MIT 协议开源。详情请参阅 [LICENSE](./LICENSE) 文件。

