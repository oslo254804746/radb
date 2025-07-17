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

#### Get Device List

```rust
use radb::AdbDevice;
use radb::AdbClient;

fn main() {
    let mut a = AdbClient::default();
    let device = a.iter_devices().unwrap().next();
    if let Some(mut device) = device{
        println!("{:#?}",&device.serial);
        let output = device.shell("echo \"Hello Android\" ");
        println!("{}",output.unwrap());
    }
}
```

#### Using Log Feature (logcat)

```rust
#[cfg(test)]
mod tests {
    const TEST_DEVICE_SERIAL: &str = "3508719615000K5";
    const TEST_PACKAGE: &str = "com.android.chrome";
    const TEST_DIR: &str = "/data/local/tmp";
    const DEFAULT_ADB_ADDR: &str = "127.0.0.1:5037";

    #[cfg(feature = "blocking")]
    mod device_blocking_tests {
        use super::*;
        use radb::beans::command::AdbCommand;
        use std::path::PathBuf;

        fn create_test_device() -> AdbDevice<&'static str> {
            AdbDevice::new(TEST_DEVICE_SERIAL, DEFAULT_ADB_ADDR)
        }

        #[test]
        fn test_device_creation() {
            let device = create_test_device();
            assert_eq!(device.serial, Some(TEST_DEVICE_SERIAL.to_string()));
        }
    }

    #[test]
    fn test_logcat() {
        setup_test_environment();
        let mut device = create_test_device();

        // Test logcat
        let logcat_result = device.logcat(true, None);
        assert!(logcat_result.is_ok());

        let mut logcat_iter = logcat_result.unwrap();
        let mut count = 0;
        for line in logcat_iter {
            if line.is_ok() {
                count += 1;
                if count >= 5 {
                    // Only read first 5 lines
                    break;
                }
            }
        }
        println!("Read {} logcat lines", count);
    }

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

## 📦 Documentation

For complete API documentation, please visit [docs.rs/radb](https://docs.rs/radb).

## 📦 Crates Link

- [Crates.io](https://crates.io/crates/radb)

## 🛠 Contributing

Pull requests and issues are welcome! If you have feature suggestions or find bugs, please feel free to submit them.

## 📄 License

This project is licensed under the MIT License. See the [LICENSE](./LICENSE) file for details.

