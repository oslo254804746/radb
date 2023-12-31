
mod test_device{
    use radb::client::adb::AdbClient;
    use radb::client::device::BaseDevice;

    fn get_the_only_device() -> anyhow::Result<BaseDevice> {
        let mut adb = AdbClient::default();
        adb.get_device_by_serial(None)
    }
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

    #[test]
    fn device_output() {
        let mut device = get_the_only_device().unwrap();
        let output = device.adb_output(&["forward", "--list"]);
        println!("{:#?}", output);
    }

    #[test]
    fn device_logcat() {
        let mut device = get_the_only_device().unwrap();
        let logcat = device.logcat(false);
        if logcat.is_ok(){
            let mut n = 0;
            for line in logcat.unwrap() {
                if n <=100{
                    println!("{:#?}", line);
                    n+=1;
                    continue;
                }
                break;
            }
        }
    }

    #[test]
    fn device_push() {
        let mut device = get_the_only_device().unwrap();
        let push_result = device.push(
            "C:\\Users\\oslo\\Desktop\\compony-extra.yml",
            "/sdcard/compony-extra.yml",
        );
        if push_result.is_ok(){
            let stat = device.stat("/sdcard/compony-extra.yml");
            println!("{:#?}", stat);
        }
    }

    #[test]
    fn device_file_stat(){
        let mut device = get_the_only_device().unwrap();
        let stat = device.stat("/sdcard/compony-extra.yml");
        println!("{:#?}", stat);
    }

    #[test]
    fn device_iter_directory(){
        let mut device = get_the_only_device().unwrap();
        if let Ok(stat) = device.iter_directory("/sdcard"){
            for x in stat {
                println!("{:#?}", x);
            }
        }
    }

    #[test]
    fn device_iter_content(){
        let mut device = get_the_only_device().unwrap();
        if let Ok(stat) = device.iter_content("/sdcard/compony-extra.yml"){
            for x in stat {
                println!("{:#?}", x);
            }
        }
    }

    #[test]
    fn device_exists(){
        let mut device = get_the_only_device().unwrap();
        if let Ok(stat) = device.exists("/sdcard/compony-extra2.yml"){
            println!("file exists {}",stat)
        };
    }

    #[test]
    fn device_screenshot(){
        let mut device = get_the_only_device().unwrap();
        let data = device.screenshot();
        println!("image {:#?}",data);
    }

    #[test]
    fn device_install(){
        let mut device = get_the_only_device().unwrap();
        let data = device.install("C:\\Users\\oslo\\Downloads\\PCAPdroid_v1.6.9.apk");

        println!("image {:#?}",data);
    }

    #[test]
    fn device_uninstall(){
        let mut device = get_the_only_device().unwrap();
        let data = device.uninstall("com.emanuelef.remote_capture");
        println!("image {:#?}",data);
    }
}