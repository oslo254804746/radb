use anyhow::{anyhow, Result};
use chrono::Utc;
use std::convert::TryInto;

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq)]
pub struct FileInfo {
    pub mode: u32,
    pub size: u32,
    pub mtime: u32,
    pub mdtime: Option<chrono::DateTime<Utc>>,
    pub path: String,
}

pub fn parse_file_info<T: ToString>(data: Vec<u8>, path: T) -> Result<FileInfo> {
    let mut mode = 0;
    let mut size = 0;
    let mut mtime = 0;
    let mut mdtime = None;
    let mode_bytes = &data[0..4];
    let size_bytes = &data[4..8];
    let mtime_bytes = &data[8..12];
    mode = u32::from_le_bytes(mode_bytes.try_into().unwrap());
    size = u32::from_le_bytes(size_bytes.try_into().unwrap());
    mtime = u32::from_le_bytes(mtime_bytes.try_into().unwrap());
    mdtime = Some(
        chrono::DateTime::<Utc>::from_timestamp(mtime as i64, 0)
            .ok_or(anyhow!("Parse Datetime Error"))?,
    );
    Ok(FileInfo::new(mode, size, mtime, mdtime, path.to_string()))
}

impl FileInfo {
    fn new(
        mode: u32,
        size: u32,
        mtime: u32,
        mdtime: Option<chrono::DateTime<Utc>>,
        path: String,
    ) -> FileInfo {
        FileInfo {
            mode,
            size,
            mtime,
            mdtime,
            path,
        }
    }
}
