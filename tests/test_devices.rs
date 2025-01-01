#[cfg(feature = "blocking")]
mod test_device_blocking {

    #[test]
    fn test_device_blocking() {
        let mut device =
            radb::client::adb_device::AdbDevice::new("emulator-5554", "127.0.0.1:5037");
        let resp = device.shell(&["ls", "/data/local/tmp", "-allh"]);
        println!("{:#?}", &resp)
    }
}
