use std::fmt::Display;

pub enum NetworkType {
    Tcp,
    Unix,
    Dev,
    Local,
    LocalReserverd,
    LocalFileSystem,
    LocalAbstrcat,
}

impl Display for NetworkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            NetworkType::Tcp => "tcp:".to_string(),
            NetworkType::Unix | NetworkType::LocalAbstrcat => "localabstract:".to_string(),
            NetworkType::Dev => "dev".to_string(),
            NetworkType::Local => "local".to_string(),
            NetworkType::LocalReserverd => "localreserved".to_string(),
            NetworkType::LocalFileSystem => "localfilesystem".to_string(),
        };
        write!(f, "{}", str)
    }
}
