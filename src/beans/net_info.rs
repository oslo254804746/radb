use std::fmt::Display;

/// 网络连接类型枚举
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkType {
    Tcp,
    Unix,
    Dev,
    Local,
    LocalReserved,
    LocalFileSystem,
    LocalAbstract,
}

impl Display for NetworkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            NetworkType::Tcp => "tcp:",
            NetworkType::Unix | NetworkType::LocalAbstract => "localabstract:",
            NetworkType::Dev => "dev:",
            NetworkType::Local => "local:",
            NetworkType::LocalReserved => "localreserved:",
            NetworkType::LocalFileSystem => "localfilesystem:",
        };
        write!(f, "{}", str)
    }
}

impl NetworkType {
    /// 从字符串解析网络类型
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

impl std::str::FromStr for NetworkType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tcp" => Ok(NetworkType::Tcp),
            "unix" | "localabstract" => Ok(NetworkType::LocalAbstract),
            "dev" => Ok(NetworkType::Dev),
            "local" => Ok(NetworkType::Local),
            "localreserved" => Ok(NetworkType::LocalReserved),
            "localfilesystem" => Ok(NetworkType::LocalFileSystem),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_type_display() {
        assert_eq!(NetworkType::Tcp.to_string(), "tcp:");
        assert_eq!(NetworkType::LocalAbstract.to_string(), "localabstract:");
        assert_eq!(NetworkType::Dev.to_string(), "dev:");
    }

    #[test]
    fn test_network_type_from_str() {
        assert_eq!(NetworkType::from_str("tcp"), Some(NetworkType::Tcp));
        assert_eq!(
            NetworkType::from_str("localabstract"),
            Some(NetworkType::LocalAbstract)
        );
        assert_eq!(NetworkType::from_str("invalid"), None);
    }
}
