use std::collections::HashMap;

#[derive(Debug)]
pub struct AdbDeviceInfo {
    pub serial: String,
    pub state: String,
    pub properties: HashMap<String, String>,
}

impl AdbDeviceInfo {
    pub fn new(serial: String, state: String) -> AdbDeviceInfo {
        AdbDeviceInfo {
            serial,
            state,
            properties: HashMap::new(),
        }
    }
}
