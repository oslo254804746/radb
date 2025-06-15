const DEFAULT_ADB_ADDR: &'static str = "127.0.0.1:5037";

#[cfg(feature = "tokio_async")]
mod test_adb_s2 {
    use crate::DEFAULT_ADB_ADDR;
    use radb::client::adb_client::AdbClient;

    #[tokio::test]
    async fn test_adb_list_devices() {
        let mut adb = AdbClient::new("127.0.0.1:5037").await;
        let devices = adb.list_devices().await.unwrap();
        assert_eq!(devices[0].serial, Some("emulator-5554".to_string()));
    }

    #[tokio::test]
    async fn test_adb_server_version() {
        let mut adb = AdbClient::new("127.0.0.1:5037").await;
        let version = adb.server_version().await.unwrap();
        // adb --version
        // Android Debug Bridge version 1.0.41
        assert_eq!("41", version)
    }

    #[tokio::test]
    async fn test_adb_disconnect_device() {
        let mut adb = AdbClient::new(DEFAULT_ADB_ADDR).await;
        let result = adb.disconnect_device("emulator-5554").await.unwrap();
        assert_eq!("disconnected emulator-5554", result)
    }

    #[tokio::test]
    async fn test_adb_connect_device_v2() {
        let mut adb = AdbClient::new(DEFAULT_ADB_ADDR).await;
        let result = adb.connect_device("127.0.0.1:5555").await.unwrap();
        assert_eq!("connected to 127.0.0.1:5555", result)
    }
}
