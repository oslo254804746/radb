use chrono::{DateTime, Utc};

#[derive(Debug, PartialEq, Ord, PartialOrd, Eq)]
pub struct AppInfo {
    pub package_name: String,
    pub version_name: Option<String>,
    pub version_code: Option<u32>,
    pub flags: Vec<String>,
    pub first_install_time: Option<DateTime<Utc>>,
    pub last_update_time: Option<DateTime<Utc>>,
    pub signature: Option<String>,
    pub path: String,
    pub sub_apk_paths: Vec<String>,
}

impl AppInfo {
    pub fn new(package_name: &str) -> AppInfo {
        Self {
            package_name: package_name.to_string(),
            version_name: None,
            version_code: None,
            flags: vec![],
            first_install_time: None,
            last_update_time: None,
            signature: None,
            path: "".to_string(),
            sub_apk_paths: vec![],
        }
    }
}
