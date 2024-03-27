
#[cfg(test)]
mod test_adb{
    use radb::client::AdbConnection;
    use radb::client::AdbDevice;

    #[test]
    fn test_adb_list_devices(){
        let mut adb = AdbConnection::default().unwrap();
        let mut devices = adb.list_devices().unwrap();
        assert_eq!(devices[0].serial, Some("emulator-5554".to_string()));
    }
    #[test]
    fn test_adb_device_function(){
        let mut adb = AdbConnection::default().unwrap();
        let  mut device = adb.device("emulator-5554");
        let data = device.shell(&["ls", "/data/local/tmp", "-all"]).unwrap();
        assert_eq!("total 24\ndrwxrwx--x 3 shell shell 4096 2023-12-31 11:17:43.777000000 +0000 .\ndrwxr-x--x 4 root  root  4096 2023-02-04 10:13:17.564000000 +0000 ..\ndrwxr-xr-x 5 shell shell 4096 2024-03-16 02:33:13.684000000 +0000 .studio\n", &data)
    }


}
