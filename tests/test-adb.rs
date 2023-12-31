

#[cfg(test)]
mod test_adb {
    use radb::client::adb::AdbClient;
    use radb::client::device::BaseDevice;


    fn get_the_only_device() -> anyhow::Result<BaseDevice> {
        let mut adb = AdbClient::default();
        adb.get_device_by_serial(None)
    }

    #[test]
    fn list_device() {
        let mut adb = AdbClient::default();
        if let Ok(devices) = adb.iter_devices() {
            for device in devices {
                println!("{:?}", device);
            }
        }
    }

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


}