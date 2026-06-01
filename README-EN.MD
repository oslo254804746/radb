# radb

[![crates.io](https://img.shields.io/crates/v/radb)](https://crates.io/crates/radb)
[![Docs](https://docs.rs/radb/badge.svg)](https://docs.rs/radb)

A Rust implementation of [openatx/adbutils](https://github.com/openatx/adbutils) —— a library for interacting with Android devices using ADB (Android Debug Bridge).

## 📌 Project Overview

`radb` is an ADB utility library implemented in Rust, inspired by Python's [adbutils](https://github.com/openatx/adbutils), designed to provide developers with a simple, efficient, and type-safe way to interact with Android devices.

The library supports both synchronous and asynchronous modes, and encapsulates common ADB operation commands such as device management, file transfer, shell command execution, log capture, and more.

## 🚀 Quick Start

### Add Dependency

Add the following to your [Cargo.toml](file://C:\Users\wangbaofeng\RustroverProjects\radb\Cargo.toml) file to include `radb`:

```toml
[dependencies]
radb = "0.1.7"
```

### Example Code

#### Get Device List (blocking, default)

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

#### Get Device List (Tokio async)

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

## ✅ Supported Features

| Feature | Description |
|---------|-------------|
| ✅ Device Management | List connected devices, get device status, get device by serial number or transport ID |
| ✅ ADB Server Control | Get version, start/stop server, connect/disconnect devices |
| ✅ Shell Execution | Run shell commands on device |
| ✅ File Operations | Push files to device, pull files from device, list directory contents |
| ✅ Network Control | Set TCP/IP mode, port forwarding |
| ✅ UI Automation | Simulate click, swipe, key events |
| ✅ App Management | Install, uninstall applications |
| ✅ Log Capture | Real-time device log capture (logcat) |
| ✅ Screenshot | Capture device screen screenshots |
| ✅ Device Control | Turn screen on/off, toggle airplane mode, Wi-Fi, etc. |

## 🔧 Feature Flags

`radb` provides the following feature flags for selective use:

- `blocking`: Enable synchronous API (default)
- `tokio_async`: Enable asynchronous API support (requires `tokio` runtime)

You can choose to enable different features based on your needs:

```toml
[dependencies.radb]
version = "0.1.7"
features = ["tokio_async"]
```

`blocking` and `tokio_async` cannot be enabled at the same time. Both public APIs share the same ADB command construction, response parsing, and sync file-transfer core; only connection handling, IO, and `await` adaptation are runtime-specific. `AdbClient::default()` and `AdbClient::new(...)` remain available for compatibility, but they panic on connection failure; new code should prefer `AdbClient::connect(...)`.

## 🧪 Testing

Default tests do not require a physical device:

```powershell
cargo test --lib
cargo test --test protocol_regressions
cargo test --no-run
```

Hardware integration tests are opt-in with `RADB_TEST_SERIAL`:

```powershell
$env:RADB_TEST_SERIAL="emulator-5554"
cargo test --test blocking_integration
cargo test --no-default-features --features tokio_async --test async_integration
```

When `RADB_TEST_SERIAL` is not set, hardware tests return early.

## 📦 Documentation

For complete API documentation, please visit [docs.rs/radb](https://docs.rs/radb).

## 📦 Crates Link

- [Crates.io](https://crates.io/crates/radb)

## 🛠 Contributing

Pull requests and issues are welcome! If you have feature suggestions or find bugs, please feel free to submit them.

## 📄 License

This project is licensed under the MIT License. See the [LICENSE](./LICENSE) file for details.

