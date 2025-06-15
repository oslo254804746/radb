#[cfg(feature = "tokio_async")]
mod test_device_tokio_async {
    use radb::client::adb_device::AdbDevice;

    #[tokio::test]
    async fn test_device_async() {
        let mut device = AdbDevice::new("emulator-5554", "127.0.0.1:5037");
        let resp = device.shell(&["ls", "/data/local/tmp", "-allh"]).await;
        println!("{:#?}", &resp)
    }
}
