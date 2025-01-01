const DEFAULT_ADB_ADDR: &'static str = "127.0.0.1:5037";
#[cfg(feature = "blocking")]
mod test_adb {
    use crate::DEFAULT_ADB_ADDR;
    use radb::client::AdbClient;

    #[test]
    fn test_adb_list_devices() {
        let mut adb = AdbClient::new(DEFAULT_ADB_ADDR);
        let devices = adb.list_devices().unwrap();
        assert_eq!(devices[0].serial, Some("emulator-5554".to_string()));
    }

    #[test]
    fn test_adb_server_version() {
        let mut adb = AdbClient::new(DEFAULT_ADB_ADDR);
        let version = adb.server_version().unwrap();
        // adb --version
        // Android Debug Bridge version 1.0.41

        assert_eq!("41", version)
    }

    #[test]
    fn test_adb_disconnect_device() {
        let mut adb = AdbClient::new(DEFAULT_ADB_ADDR);
        let result = adb.disconnect_device("emulator-5554").unwrap();
        assert_eq!("disconnected emulator-5554", result)
    }

    #[test]
    fn test_adb_connect_device() {
        let mut adb = AdbClient::new(DEFAULT_ADB_ADDR);
        let result = adb.connect_device("emulator-5554").unwrap();
        assert_eq!("connected to emulator-5554", result)
    }
}
