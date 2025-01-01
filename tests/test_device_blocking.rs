#[cfg(feature = "blocking")]
mod test_device {
    use std::fmt::Debug;
    use std::fs;
    use std::io::Write;
    use std::net::ToSocketAddrs;
    use std::sync::{Arc, RwLock};
    use std::thread::sleep;
    use std::time::Duration;

    use radb::client::adb_device::AdbDevice;
    use radb::client::AdbClient;

    fn get_android_emulator_device() -> Option<AdbDevice<impl ToSocketAddrs + Clone + Debug>> {
        let mut adb = AdbClient::default();
        for i in adb.list_devices().unwrap() {
            println!("{:#?}", i.serial);
            if i.serial.as_ref().unwrap().eq("emulator-5554") {
                return Some(i);
            }
        }
        None
    }

    #[test]
    fn test_device_ls_blocking() {
        let mut device = AdbDevice::new("emulator-5554", "127.0.0.1:5037");
        let resp = device.shell(&["ls", "/data/local/tmp", "-allh"]);
        assert_eq!("total 12K\ndrwxrwx--x 3 shell shell 4.0K 2023-12-31 11:17:43.777000000 +0000 .\ndrwxr-x--x 4 root  root  4.0K 2023-02-04 10:13:17.564000000 +0000 ..\ndrwxr-xr-x 5 shell shell 4.0K 2024-03-16 02:33:13.684000000 +0000 .studio\n", resp.unwrap())
    }

    #[test]
    fn test_device_logcat() {
        let mut device = get_android_emulator_device().unwrap();
        println!("{:#?}", device.addr);
        let mutex = Arc::new(RwLock::new(true));
        let mutex_arc = Arc::clone(&mutex);
        let handle = std::thread::spawn(move || {
            for i in device.logcat(false, None, mutex_arc).unwrap() {
                println!("{:#?}", i)
            }
        });
        sleep(Duration::from_secs(10));
        let mut target = mutex.write().unwrap();
        *target = false;
    }

    #[test]
    fn test_launch_app() {
        let mut device = get_android_emulator_device().unwrap();
        let pkg = "com.tencent.android.qqdownloader";
        device.shell(&["am", "start", pkg]).unwrap();
        let output = device.shell(&["ps", "-ef"]).unwrap();
        assert!(output.contains(pkg))
    }
}
