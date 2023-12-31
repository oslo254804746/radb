
## 前言

这个项目是基于`adbutils`库， 项目链接[openatx/adbutils](https://github.com/openatx/adbutils) 用rust重写的

主要是为了方便在`rust`中进行`adb`命令的操作。

这个项目是我的`rust`练手项目, 所以可能有的地方洗得很奇怪，不过没事，又不是不能用

## 项目介绍

`radb`的主要实现了`adbutils`中的部分功能：

|      | adb                          | device                 |
| ---- | ---------------------------- | ---------------------- |
|      | `list_device`                | `open_transport`       |
|      | `iter_device`                | `get_state`            |
|      | `get_device_by_serial`       | `shell`                |
|      | `get_device_by_transport_id` | `forward`              |
|      | `server_version`             | `forward_list`         |
|      | `server_kill`                | `reverse`              |
|      | `connect`                    | `adb_output`           |
|      | `disconnect`                 | `push`                 |
|      |                              | `create_connection`    |
|      |                              | `tcpip`                |
|      |                              | `screenshot`           |
|      |                              | `switch_screen`        |
|      |                              | `switch_airplane_mode` |
|      |                              | `keyevent`             |
|      |                              | `switch_wifi`          |
|      |                              | `click`                |
|      |                              | `swipe`                |
|      |                              | `send_keys`            |
|      |                              | `wlan_ip`              |
|      |                              | `uninstall`            |
|      |                              | `install`              |
|      |                              | ...                    |

部分功能没有经过测试，可能会存在`bug`



## 使用示例



### `adb`

#### 获取设备列表

```rust
    #[test]
    fn list_device() {
        let mut adb = AdbClient::default();
        if let Ok(devices) = adb.iter_devices() {
            for device in devices {
                println!("{:?}", device);
            }
        }
    }
```



#### 连接/断开设备

```rust
    #[test]
    fn connect_and_disconnect_device() {
        let mut adb = AdbClient::default();
        if let Ok(device) = adb.get_device_by_serial(None) {
            println!("{:?}", &device);
            if device.serial.is_some(){
                let serial = device.serial.unwrap();
                adb.disconnect(&serial).unwrap();
                let current_device_list = adb.list_devices().unwrap();
                println!("{:?}", current_device_list);
                adb.connect(&serial, None).unwrap();
                let current_device_list = adb.list_devices().unwrap();
                println!("{:?}", current_device_list);
            }
        }
    }
```



#### 获取设备实例

```rust
    #[test]
    fn get_device_without_any_condition() {
        let mut adb = AdbClient::default();
        if let Ok(device) = adb.get_device_by_serial(None) {
            println!("{:?}", device)
        }
        if let Ok(device) = adb.get_device_by_transport_id(None) {
            println!("{:?}", device)
        }
    }
```

### 设备实例

#### 执行命令

**注意: 包含管道符`|`的命令通过socket执行会有问题，有管道符的建议通过`adb_output`**

```rust
    #[test]
    fn device_shell() {
        let mut adb = AdbClient::default();
        if let Ok(mut device) = adb.get_device_by_serial(None) {
            let resp = device.shell(&["ps", "-ef"]);
            if let Ok(output) = resp {
                println!("{}", output);
            }
        }
    }
```

#### forward端口

```rust
    #[test]
    fn device_forward() {
        let mut device = get_the_only_device().unwrap();
        if let Ok(_) = device.forward("tcp:8888", "tcp:10086", false){
            let forward_list = device.forward_list().unwrap();
            for forward in forward_list {
                println!("{:?}", forward);
            }
        }
    }
```



具体请查看`tests`目录下的示例，这里不再一一列举

## 依赖

```toml
anyhow = "1.0.77"
chrono = "0.4.31"
which = "5.0.0"
tempfile = "3.2.0"
image = "0.24.7"
regex = "1.10.2"
reqwest = { version = "0.11.23", features = ["blocking"] }
md5 = "0.7.0"
tracing-subscriber = "0.3.18"
log = "0.4.20"
tracing = "0.1.40"
```

