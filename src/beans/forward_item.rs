#[derive(Debug)]
pub struct ForwardItem {
    pub(crate) serial: String,
    pub(crate) local: String,
    pub(crate) remote: String,
}

impl ForwardItem {
    pub fn new<T>(serial: T, local: T, remote: T) -> ForwardItem
    where
        T: ToString,
    {
        ForwardItem {
            serial: serial.to_string(),
            local: local.to_string(),
            remote: remote.to_string(),
        }
    }
}
