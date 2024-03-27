
#[cfg(test)]
mod test_devices{
    use std::sync::{Arc, RwLock};
    use std::thread;
    use std::thread::sleep;
    use std::time::Duration;
    use chrono::{DateTime, TimeZone, Utc};
    use log::info;
    use radb::beans::AppInfo;
    use radb::beans::FileInfo;
    use radb::client::AdbDevice;
    use radb::utils::init_logger;



    fn setup() -> AdbDevice{
        init_logger();
        let serial = "emulator-5554";
        let device = AdbDevice::new_device_default(serial);
        device
    }

    #[test]
    fn test_logcat(){
        let mut device = setup();
        let mut mtx = Arc::new(RwLock::new(true));
        let logcat_lock = mtx.clone();
        thread::spawn(
            move ||{
                for i in device.logcat(true, None,logcat_lock).unwrap(){
                    info!("{}", i)
                }
            }
        );
        sleep(Duration::from_secs(10));
        let mut s= mtx.write().unwrap();
        *s = false;
        info!("stop log cat");
    }

    #[test]
    fn test_get_sdk_version(){
        let mut device = setup();
        let sdk_version = device.get_sdk_version().unwrap();
        assert_eq!(sdk_version, "30");
    }

    #[test]
    fn test_get_gpu(){
        let mut device = setup();
        let gpu = device.get_device_gpu().unwrap();
        assert_eq!(gpu, "GLES: Google (NVIDIA Corporation), Android Emulator OpenGL ES Translator (NVIDIA GeForce RTX 4070 Ti/PCIe/SSE2), OpenGL ES 2.0 (4.5.0 NVIDIA 551.76)".to_string());
    }

    #[test]
    fn test_app_info(){
        let mut device = setup();
        let app_info = AppInfo {
            package_name: "com.example.myapplication".to_string(),
            version_name: None,
            version_code: None,
            flags: vec![],
            first_install_time: None,
            last_update_time: None,
            signature: None,
            path: "".to_string(),
            sub_apk_paths: vec![],
        };
        let s = device.app_info("com.example.myapplication").unwrap();
        assert_eq!(app_info, s)
    }

    #[test]
    fn test_iter_directory(){

        fn string_to_datetime(ts: &str) -> chrono::DateTime<Utc>{

            let datetime = DateTime::parse_from_rfc3339(ts).unwrap();
            datetime.with_timezone(&Utc)
        }
        let mut device = setup();
        let file_info_1 = FileInfo{
            mode: 16873,
            size: 4096,
            mtime: 1675505597,
            mdtime: Some(string_to_datetime("2023-02-04T10:13:17Z")),
            path: String::from(".."),
        };
        let file_info_2 = FileInfo{
            mode: 16889,
            size: 4096,
            mtime: 1704021463,
            mdtime: Some(string_to_datetime("2023-12-31T11:17:43Z")),
            path: String::from("."),
        };
        let file_info_3 = FileInfo{
            mode: 16877,
            size: 4096,
            mtime: 1710556393,
            mdtime: Some(string_to_datetime("2024-03-16T02:33:13Z")),
            path: String::from(".studio"),
        };
        let mut preset_file_info = vec![file_info_1, file_info_2, file_info_3];
        let mut file_info = device.iter_directory("/data/local/tmp").unwrap().collect::<Vec<FileInfo>>();
        assert_eq!(preset_file_info.sort(), file_info.sort());
    }

    #[test]
    fn test_switch_screen(){
        let mut device = setup();
        device.switch_screen(false).unwrap();
        sleep(Duration::from_secs(1)); // maybe need wait
        assert_eq!(device.if_screen_on().unwrap(), false)
    }

}
