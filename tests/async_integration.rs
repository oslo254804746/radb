#![cfg(feature = "tokio_async")]

use radb::client::AdbDevice;
use std::io::Write;

const DEFAULT_ADB_ADDR: &str = "127.0.0.1:5037";

fn configured_device() -> Result<Option<AdbDevice<&'static str>>, Box<dyn std::error::Error>> {
    let Ok(serial) = std::env::var("RADB_TEST_SERIAL") else {
        return Ok(None);
    };
    radb::utils::start_adb_server_result()?;
    Ok(Some(AdbDevice::new(serial, DEFAULT_ADB_ADDR)))
}

#[tokio::test]
async fn push_pull_round_trip_skips_without_radb_test_serial(
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(mut device) = configured_device()? else {
        return Ok(());
    };

    let mut local = tempfile::NamedTempFile::new()?;
    let expected = b"radb\0async\xffroundtrip\n".to_vec();
    local.write_all(&expected)?;
    local.flush()?;

    let remote = format!(
        "/data/local/tmp/radb-async-roundtrip-{}.bin",
        std::process::id()
    );
    let pulled_dir = tempfile::tempdir()?;
    let pulled_path = pulled_dir.path().join("pulled.bin");

    device.push(local.path().to_str().unwrap(), &remote).await?;
    let pulled_size = device.pull(&remote, &pulled_path).await?;
    let actual = std::fs::read(&pulled_path)?;
    let _ = device.remove(&remote).await;

    assert_eq!(pulled_size, expected.len());
    assert_eq!(actual, expected);
    Ok(())
}

#[tokio::test]
async fn screenshot_skips_without_radb_test_serial() -> Result<(), Box<dyn std::error::Error>> {
    let Some(mut device) = configured_device()? else {
        return Ok(());
    };

    let image = device.screenshot().await?;
    assert!(image.width() > 0);
    assert!(image.height() > 0);
    Ok(())
}
