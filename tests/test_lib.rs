use radb::client::AdbClient;
use radb::utils::start_adb_server;

#[cfg(test)]
mod tests {
    use super::*;
    use radb::client::{AdbClient, AdbDevice};
    use radb::utils::start_adb_server;
    use std::io::Write;
    use tempfile::NamedTempFile;

    const TEST_DEVICE_SERIAL: &str = "3508719615000K5";
    const TEST_PACKAGE: &str = "com.android.chrome";
    const TEST_DIR: &str = "/data/local/tmp";
    const DEFAULT_ADB_ADDR: &str = "127.0.0.1:5037";

    // 测试辅助函数
    fn setup_test_environment() {
        // 启动 ADB 服务器
        start_adb_server();

        // 等待服务器启动
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    // 创建临时测试文件
    fn create_test_file(content: &str) -> std::io::Result<NamedTempFile> {
        let mut file = NamedTempFile::new()?;
        file.write_all(content.as_bytes())?;
        file.flush()?;
        Ok(file)
    }

    // ========== AdbClient 测试 ==========

    #[cfg(feature = "blocking")]
    mod blocking_tests {
        use super::*;

        #[test]
        fn test_client_creation() {
            setup_test_environment();
            let client = AdbClient::default();
            assert!(client.stream.peer_addr().is_ok());
        }

        #[test]
        fn test_client_with_custom_addr() {
            setup_test_environment();
            let client = AdbClient::new(DEFAULT_ADB_ADDR);
            assert!(client.stream.peer_addr().is_ok());
        }

        #[test]
        fn test_server_version() {
            setup_test_environment();
            let mut client = AdbClient::default();
            let version = client.server_version();
            assert!(version.is_ok());
            println!("ADB Server Version: {}", version.unwrap());
        }

        #[test]
        fn test_list_devices() {
            setup_test_environment();
            let mut client = AdbClient::default();
            let devices = client.list_devices();
            assert!(devices.is_ok());

            let devices = devices.unwrap();
            println!("Found {} devices", devices.len());
            for device in &devices {
                println!("Device: {:?}", device.serial);
            }
        }

        #[test]
        fn test_iter_devices() {
            setup_test_environment();
            let mut client = AdbClient::default();
            let devices_iter = client.iter_devices();
            assert!(devices_iter.is_ok());

            let mut count = 0;
            for device in devices_iter.unwrap() {
                count += 1;
                println!("Device {}: {:?}", count, device.serial);
            }
            println!("Total devices: {}", count);
        }

        // #[test]
        // fn test_connect_device() {
        //     setup_test_environment();
        //     let mut client = AdbClient::default();
        //
        //     // 尝试连接到已连接的设备（应该返回 "already connected" 或类似消息）
        //     let result = client.connect_device(TEST_DEVICE_SERIAL);
        //     println!("Connect result: {:?}", result);
        //     // 不做断言，因为设备可能已经连接
        // }

        // #[test]
        // fn test_disconnect_device() {
        //     setup_test_environment();
        //     let mut client = AdbClient::default();
        //
        //     // 尝试断开连接（可能会失败，因为设备可能通过USB连接）
        //     let result = client.disconnect_device(TEST_DEVICE_SERIAL);
        //     println!("Disconnect result: {:?}", result);
        // }

        // #[test]
        // fn test_server_kill() {
        //     setup_test_environment();
        //     let mut client = AdbClient::default();
        //
        //     // 注意：这会杀死 ADB 服务器，可能影响其他测试
        //     // 仅在必要时运行
        //     // let result = client.server_kill();
        //     // assert!(result.is_ok());
        // }
    }

    #[cfg(feature = "tokio_async")]
    mod async_tests {
        use super::*;
        use futures_util::StreamExt;
        use radb::client::AdbClient;
        use tokio;

        #[tokio::test]
        async fn test_async_client_creation() {
            setup_test_environment();
            let client = AdbClient::default().await;
            assert!(client.stream.peer_addr().is_ok());
        }

        #[tokio::test]
        async fn test_async_server_version() {
            setup_test_environment();
            let mut client = AdbClient::default().await;
            let version = client.server_version().await;
            assert!(version.is_ok());
            println!("ADB Server Version: {}", version.unwrap());
        }

        #[tokio::test]
        async fn test_async_list_devices() {
            setup_test_environment();
            let mut client = AdbClient::default().await;
            let devices = client.list_devices().await;
            assert!(devices.is_ok());

            let devices = devices.unwrap();
            println!("Found {} devices", devices.len());
            for device in &devices {
                println!("Device: {:?}", device.serial);
            }
        }

        #[tokio::test]
        async fn test_async_iter_devices() {
            setup_test_environment();
            let mut client = AdbClient::default().await;
            let mut devices_stream = client.iter_devices().await;

            let mut count = 0;
            while let Some(device) = devices_stream.next().await {
                count += 1;
                println!("Device {}: {:?}", count, device.serial);
            }
            println!("Total devices: {}", count);
        }
    }

    // ========== AdbDevice 测试 ==========

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

        #[test]
        fn test_device_state() {
            setup_test_environment();
            let mut device = create_test_device();
            let state = device.get_state();
            assert!(state.is_ok());
            println!("Device state: {}", state.unwrap());
        }

        #[test]
        fn test_device_serial() {
            setup_test_environment();
            let mut device = create_test_device();
            let serial = device.get_serialno();
            assert!(serial.is_ok());
            assert_eq!(serial.unwrap().trim(), TEST_DEVICE_SERIAL);
        }

        #[test]
        fn test_device_features() {
            setup_test_environment();
            let mut device = create_test_device();
            let features = device.get_features();
            assert!(features.is_ok());
            println!("Device features: {}", features.unwrap());
        }

        #[test]
        fn test_shell_command() {
            setup_test_environment();
            let mut device = create_test_device();
            let result = device.shell(["echo", "hello"]);
            assert!(result.is_ok());
            assert_eq!(result.unwrap().trim(), "hello");
        }

        #[test]
        fn test_shell_trim() {
            setup_test_environment();
            let mut device = create_test_device();
            let result = device.shell_trim(["echo", "hello world"]);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "hello world");
        }

        #[test]
        fn test_device_properties() {
            setup_test_environment();
            let mut device = create_test_device();

            // 测试各种设备属性
            let sdk_version = device.get_sdk_version();
            assert!(sdk_version.is_ok());
            println!("SDK Version: {}", sdk_version.unwrap());

            let android_version = device.get_android_version();
            assert!(android_version.is_ok());
            println!("Android Version: {}", android_version.unwrap());

            let model = device.get_device_model();
            assert!(model.is_ok());
            println!("Device Model: {}", model.unwrap());

            let brand = device.get_device_brand();
            assert!(brand.is_ok());
            println!("Device Brand: {}", brand.unwrap());

            let abi = device.get_device_abi();
            assert!(abi.is_ok());
            println!("Device ABI: {}", abi.unwrap());
        }

        #[test]
        fn test_file_operations() {
            setup_test_environment();
            let mut device = create_test_device();

            // 测试目录是否存在
            let exists = device.exists(TEST_DIR);
            assert!(exists.is_ok());
            if exists.unwrap() {
                println!("Test directory {} exists", TEST_DIR);

                // 列出目录内容
                let files = device.list(TEST_DIR);
                assert!(files.is_ok());
                println!("Files in {}: {:?}", TEST_DIR, files.unwrap().len());
            }
        }

        #[test]
        fn test_stat_file() {
            setup_test_environment();
            let mut device = create_test_device();

            // 测试 stat 一个已知存在的文件/目录
            let stat = device.stat(TEST_DIR);
            assert!(stat.is_ok());

            let file_info = stat.unwrap();
            println!(
                "File info for {}: size={}, mtime={}",
                file_info.path, file_info.size, file_info.mtime
            );
        }

        #[test]
        fn test_push_pull_file() {
            setup_test_environment();
            let mut device = create_test_device();

            // 创建临时文件
            let test_content = "Hello, ADB test!";
            let temp_file = create_test_file(test_content).unwrap();
            let temp_path = temp_file.path().to_str().unwrap();

            // 推送文件到设备
            let remote_path = format!("{}/test_file.txt", TEST_DIR);
            let push_result = device.push(temp_path, &remote_path);
            assert!(push_result.is_ok());

            // 验证文件是否存在
            let exists = device.exists(&remote_path);
            assert!(exists.is_ok());
            assert!(exists.unwrap());

            // 拉取文件
            let pull_dest = PathBuf::from("/tmp/pulled_test_file.txt");
            let pull_result = device.pull(&remote_path, &pull_dest);
            assert!(pull_result.is_ok());

            // 清理
            let _ = device.remove(&remote_path);
            let _ = std::fs::remove_file(&pull_dest);
        }

        #[test]
        fn test_screen_operations() {
            setup_test_environment();
            let mut device = create_test_device();

            // 测试屏幕状态
            let screen_on = device.if_screen_on();
            assert!(screen_on.is_ok());
            println!("Screen on: {}", screen_on.unwrap());

            // 测试按键事件
            let keyevent_result = device.keyevent("4"); // Back key
            assert!(keyevent_result.is_ok());
        }

        #[test]
        fn test_click_and_swipe() {
            setup_test_environment();
            let mut device = create_test_device();

            // 测试点击
            let click_result = device.click(100, 100);
            assert!(click_result.is_ok());

            // 测试滑动
            let swipe_result = device.swipe(100, 100, 200, 200, 500);
            assert!(swipe_result.is_ok());
        }

        #[test]
        fn test_app_operations() {
            setup_test_environment();
            let mut device = create_test_device();

            // 测试应用信息
            let app_info = device.app_info(TEST_PACKAGE);
            if app_info.is_some() {
                let info = app_info.unwrap();
                println!(
                    "App info for {}: version={:?}",
                    info.package_name, info.version_name
                );
            }

            // 测试应用停止
            let stop_result = device.app_stop(TEST_PACKAGE);
            assert!(stop_result.is_ok());

            // 测试应用启动
            let start_result = device.app_start(TEST_PACKAGE);
            // 注意：启动可能需要完整的 activity 名称，这里可能会失败
            println!("App start result: {:?}", start_result);
        }

        #[test]
        fn test_network_operations() {
            setup_test_environment();
            let mut device = create_test_device();

            // 测试获取 WLAN IP
            let wlan_ip = device.wlan_ip();
            if wlan_ip.is_ok() {
                println!("WLAN IP: {}", wlan_ip.unwrap());
            }

            // 测试端口转发
            let forward_result = device.forward("tcp:8080", "tcp:8080", false);
            assert!(forward_result.is_ok());

            // 测试转发列表
            let forward_list = device.forward_list();
            assert!(forward_list.is_ok());
            println!("Forward list: {:?}", forward_list.unwrap());
        }

        #[test]
        fn test_screenshot() {
            setup_test_environment();
            let mut device = create_test_device();

            // 测试截图
            let screenshot_result = device.screenshot();
            if screenshot_result.is_ok() {
                let image = screenshot_result.unwrap();
                println!("Screenshot taken: {}x{}", image.width(), image.height());
            }
        }

        #[test]
        fn test_logcat() {
            setup_test_environment();
            let mut device = create_test_device();

            // 测试 logcat
            let logcat_result = device.logcat(true, None);
            assert!(logcat_result.is_ok());

            let mut logcat_iter = logcat_result.unwrap();
            let mut count = 0;
            for line in logcat_iter {
                if line.is_ok() {
                    count += 1;
                    if count >= 5 {
                        // 只读取前5行
                        break;
                    }
                }
            }
            println!("Read {} logcat lines", count);
        }

        #[test]
        fn test_wifi_operations() {
            setup_test_environment();
            let mut device = create_test_device();

            // 测试 WiFi 开关（需要 root 权限）
            let wifi_result = device.switch_wifi(true);
            println!("WiFi switch result: {:?}", wifi_result);
        }

        #[test]
        fn test_command_types() {
            setup_test_environment();
            let mut device = create_test_device();

            // 测试不同类型的命令
            let cmd1 = AdbCommand::single("echo hello");
            let result1 = device.shell(cmd1);
            println!("Shell command result: {:?}", result1);
            assert!(result1.is_ok());

            let cmd2 = AdbCommand::multiple(vec!["echo", "world"]);
            let result2 = device.shell(cmd2);
            assert!(result2.is_ok());

            let cmd3 = AdbCommand::from(["echo", "test"]);
            let result3 = device.shell(cmd3);
            assert!(result3.is_ok());
        }

        #[test]
        fn test_list2cmdline() {
            let args = ["echo", "hello world", "test"];
            let cmdline = AdbDevice::<&str>::list2cmdline(&args);
            assert!(cmdline.contains("\"hello world\""));
            println!("Command line: {}", cmdline);
        }
    }

    #[cfg(feature = "tokio_async")]
    mod device_async_tests {
        use super::*;
        use futures_util::{pin_mut, StreamExt};
        use radb::beans::NetworkType;
        use radb::client::AdbDevice;
        use tokio;

        async fn create_test_device() -> AdbDevice<&'static str> {
            AdbDevice::new(TEST_DEVICE_SERIAL, DEFAULT_ADB_ADDR)
        }

        #[tokio::test]
        async fn test_async_device_state() {
            setup_test_environment();
            let mut device = create_test_device().await;
            let state = device.get_state().await;
            assert!(state.is_ok());
            println!("Device state: {}", state.unwrap());
        }

        #[tokio::test]
        async fn test_async_shell_command() {
            setup_test_environment();
            let mut device = create_test_device().await;
            let result = device.shell(["echo", "hello"]).await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap().trim(), "hello");
        }

        #[tokio::test]
        async fn test_async_file_operations() {
            setup_test_environment();
            let mut device = create_test_device().await;

            // 测试目录是否存在
            let exists = device.exists(TEST_DIR).await;
            assert!(exists.is_ok());

            if exists.unwrap() {
                // 列出目录内容
                let files = device.list(TEST_DIR).await;
                assert!(files.is_ok());
                println!("Files in {}: {:?}", TEST_DIR, files.unwrap().len());
            }
        }

        #[tokio::test]
        async fn test_async_iter_content() {
            setup_test_environment();
            let mut device = create_test_device().await;

            // 创建一个测试文件
            let test_file = format!("{}/async_test.txt", TEST_DIR);
            let _ = device
                .shell(["echo", "test content", ">", &test_file])
                .await;

            // 测试内容迭代
            let content_stream = device.iter_content(&test_file).await;
            assert!(content_stream.is_ok());

            let mut content = String::new();
            let mut stream = content_stream.unwrap();
            pin_mut!(stream);
            while let Some(chunk) = stream.next().await {
                if let Ok(chunk) = chunk {
                    content.push_str(&String::from_utf8_lossy(&chunk));
                }
            }

            println!("File content: {}", content);

            // 清理
            let _ = device.remove(&test_file).await;
        }

        #[tokio::test]
        async fn test_async_screenshot() {
            setup_test_environment();
            let mut device = create_test_device().await;

            let screenshot_result = device.screenshot().await;
            if screenshot_result.is_ok() {
                let image = screenshot_result.unwrap();
                println!("Screenshot taken: {}x{}", image.width(), image.height());
            }
        }

        #[tokio::test]
        async fn test_async_logcat_stream() {
            setup_test_environment();
            let mut device = create_test_device().await;

            let logcat_result = device.logcat(true, None).await;
            assert!(logcat_result.is_ok());

            let mut logcat_stream = logcat_result.unwrap();
            let mut count = 0;
            pin_mut!(logcat_stream);
            while let Some(line) = logcat_stream.next().await {
                if line.is_ok() {
                    count += 1;
                    if count >= 3 {
                        // 只读取前3行
                        break;
                    }
                }
            }
            println!("Read {} logcat lines", count);
        }

        #[tokio::test]
        async fn test_async_forward_operations() {
            setup_test_environment();
            let mut device = create_test_device().await;

            // 测试端口转发
            let forward_result = device.forward("tcp:9090", "tcp:9090", false).await;
            assert!(forward_result.is_ok());

            // 测试转发列表
            let forward_list = device.forward_list().await;
            assert!(forward_list.is_ok());

            // 测试远程端口转发
            let remote_port_result = device.forward_remote_port(8080).await;
            if remote_port_result.is_ok() {
                println!(
                    "Remote port forwarded to local port: {}",
                    remote_port_result.unwrap()
                );
            }
        }

        #[tokio::test]
        async fn test_async_app_operations() {
            setup_test_environment();
            let mut device = create_test_device().await;

            // 测试应用信息
            let app_info = device.app_info(TEST_PACKAGE).await;
            if app_info.is_some() {
                let info = app_info.unwrap();
                println!(
                    "App info for {}: version={:?}",
                    info.package_name, info.version_name
                );
            }

            // 测试应用停止
            let stop_result = device.app_stop(TEST_PACKAGE).await;
            assert!(stop_result.is_ok());
        }

        #[tokio::test]
        async fn test_async_network_connection() {
            setup_test_environment();
            let mut device = create_test_device().await;

            // 测试创建网络连接
            let connection_result = device
                .create_connection(NetworkType::Tcp, "127.0.0.1:8080")
                .await;

            // 这可能会失败，因为端口可能没有监听
            println!("Connection result: {:?}", connection_result);
        }

        #[tokio::test]
        async fn test_async_tcpip_mode() {
            setup_test_environment();
            let mut device = create_test_device().await;

            // 测试切换到 TCP/IP 模式
            let tcpip_result = device.tcpip(5555).await;
            println!("TCPIP mode result: {:?}", tcpip_result);
        }
    }

    // ========== 集成测试 ==========

    #[cfg(feature = "blocking")]
    #[test]
    fn test_client_device_integration() {
        setup_test_environment();
        let mut client = AdbClient::default();

        // 获取设备列表
        let devices = client.list_devices().unwrap();
        assert!(!devices.is_empty(), "No devices found for testing");

        // 使用第一个设备进行测试
        let mut device = devices.into_iter().next().unwrap();

        // 测试基本功能
        let state = device.get_state().unwrap();
        assert_eq!(state.trim(), "device");

        let serial = device.get_serialno().unwrap();
        assert!(!serial.trim().is_empty());

        println!("Integration test passed with device: {}", serial);
    }

    #[cfg(feature = "tokio_async")]
    #[tokio::test]
    async fn test_async_client_device_integration() {
        setup_test_environment();
        let mut client = AdbClient::default().await;

        // 获取设备列表
        let devices = client.list_devices().await.unwrap();
        assert!(!devices.is_empty(), "No devices found for testing");

        // 使用第一个设备进行测试
        let mut device = devices.into_iter().next().unwrap();

        // 测试基本功能
        let state = device.get_state().await.unwrap();
        assert_eq!(state.trim(), "device");

        let serial = device.get_serialno().await.unwrap();
        assert!(!serial.trim().is_empty());

        println!("Async integration test passed with device: {}", serial);
    }

    // ========== 错误处理测试 ==========

    // #[test]
    // fn test_error_handling() {
    //     setup_test_environment();
    //     let mut device = AdbDevice::new("invalid_serial", DEFAULT_ADB_ADDR);
    //
    //     // 测试无效设备的错误处理
    //     let state = device.get_state();
    //     assert!(state.is_err());
    //
    //     let error = state.unwrap_err();
    //     println!("Expected error: {:?}", error);
    //     assert!(error.is_fatal() || error.is_retryable());
    // }
    //
    // #[test]
    // fn test_file_not_found() {
    //     setup_test_environment();
    //     let mut device = AdbDevice::new(TEST_DEVICE_SERIAL, DEFAULT_ADB_ADDR);
    //
    //     // 测试不存在的文件
    //     let exists = device.exists("/nonexistent/file.txt");
    //     assert!(exists.is_ok());
    //     assert!(!exists.unwrap());
    // }

    // ========== 性能测试 ==========

    #[cfg(feature = "blocking")]
    #[test]
    fn test_performance_multiple_commands() {
        setup_test_environment();
        let mut device = AdbDevice::new(TEST_DEVICE_SERIAL, DEFAULT_ADB_ADDR);

        let start = std::time::Instant::now();

        // 执行多个命令
        for i in 0..10 {
            let result = device.shell(["echo", &format!("test_{}", i)]);
            assert!(result.is_ok());
        }

        let duration = start.elapsed();
        println!("10 commands took: {:?}", duration);

        // 确保性能在合理范围内（10秒内）
        assert!(duration < std::time::Duration::from_secs(10));
    }
}

// 运行测试的帮助函数
#[cfg(test)]
pub fn run_basic_tests() {
    println!("Running basic ADB tests...");

    // 确保测试环境设置正确
    start_adb_server();

    // 运行一些基本测试
    #[cfg(feature = "blocking")]
    {
        let mut client = AdbClient::default();
        let devices = client.list_devices().expect("Failed to list devices");
        println!("Found {} devices", devices.len());

        if !devices.is_empty() {
            let mut device = devices.into_iter().next().unwrap();
            let state = device.get_state().expect("Failed to get device state");
            println!("Device state: {}", state);
        }
    }

    println!("Basic tests completed!");
}
