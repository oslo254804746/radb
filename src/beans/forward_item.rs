#[derive(Debug)]
pub struct ForwardIterm {
    pub(crate) serial: String,
    pub(crate) local: String,
    pub(crate) remote: String,
}

impl ForwardIterm {
    pub fn new(serial: &str, local: &str, remote: &str) -> ForwardIterm {
        ForwardIterm {
            serial: serial.to_string(),
            local: local.to_string(),
            remote: remote.to_string(),
        }
    }
}
