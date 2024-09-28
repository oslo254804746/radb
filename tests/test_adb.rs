const DEFAULT_ADB_ADDR: &'static str = "127.0.0.1:5037";
#[cfg(feature = "blocking")]
mod test_adb {
    use crate::DEFAULT_ADB_ADDR;
    use radb::client::adb_client::AdbClient;

    #[test]
    fn test_adb_list_devices() {
        let mut adb = AdbClient::new(DEFAULT_ADB_ADDR);
        let devices = adb.list_devices().unwrap();
        assert_eq!(devices[0].serial, Some("f94ba50e".to_string()));
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
        let result = adb.disconnect_device("f94ba50e").unwrap();
        assert_eq!("disconnected f94ba50e", result)
    }

    #[test]
    fn test_adb_connect_device() {
        let mut adb = AdbClient::new(DEFAULT_ADB_ADDR);
        let result = adb.connect_device("f94ba50e").unwrap();
        assert_eq!("connected to f94ba50e", result)
    }

    // #[test]
    // fn test_adb_device_function() {
    //     let data = device.shell(&["ls", "/data/local/tmp", "-all"]).unwrap();
    //     assert_eq!("total 24\ndrwxrwx--x 3 shell shell 4096 2023-12-31 11:17:43.777000000 +0000 .\ndrwxr-x--x 4 root  root  4096 2023-02-04 10:13:17.564000000 +0000 ..\ndrwxr-xr-x 5 shell shell 4096 2024-03-16 02:33:13.684000000 +0000 .studio\n", &data)
    // }
}

#[cfg(feature = "tokio")]
mod test_adb_s2 {
    use crate::DEFAULT_ADB_ADDR;
    use radb::client::adb_client::AdbClient;

    #[tokio::test]
    async fn test_adb_list_devices() {
        let mut adb = AdbClient::new("127.0.0.1:5037").await;
        let devices = adb.list_devices().await.unwrap();
        assert_eq!(devices[0].serial, Some("f94ba50e".to_string()));
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
        let result = adb.disconnect_device("f94ba50e").await.unwrap();
        assert_eq!("disconnected f94ba50e", result)
    }

    #[tokio::test]
    async fn test_adb_connect_device_v2() {
        let mut adb = AdbClient::new(DEFAULT_ADB_ADDR).await;
        let result = adb.connect_device("127.0.0.1:5555").await.unwrap();
        assert_eq!("connected to 127.0.0.1:5555", result)
    }
}
