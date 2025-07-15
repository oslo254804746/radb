use std::borrow::Cow;

/// ADB命令表示，支持单个字符串或多个参数
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdbCommand {
    /// 单个命令字符串
    Single(String),
    /// 多个命令参数
    Multiple(Vec<String>),
}

impl AdbCommand {
    /// 创建单个命令
    pub fn single<S: Into<String>>(cmd: S) -> Self {
        AdbCommand::Single(cmd.into())
    }

    /// 创建多参数命令
    pub fn multiple<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        AdbCommand::Multiple(args.into_iter().map(|s| s.into()).collect())
    }

    /// 获取完整的命令字符串
    pub fn get_command(&self) -> String {
        match self {
            AdbCommand::Single(s) => s.clone(),
            AdbCommand::Multiple(parts) => shell_escape_args(parts),
        }
    }

    /// 获取命令字符串的借用版本（减少分配）
    pub fn get_command_cow(&self) -> Cow<str> {
        match self {
            AdbCommand::Single(s) => Cow::Borrowed(s),
            AdbCommand::Multiple(parts) => Cow::Owned(shell_escape_args(parts)),
        }
    }
}

/// 将参数数组转换为安全的shell命令行字符串
fn shell_escape_args(args: &[String]) -> String {
    args.iter()
        .map(|arg| shell_escape_arg(arg))
        .collect::<Vec<_>>()
        .join(" ")
}

/// 转义单个参数以确保shell安全性
fn shell_escape_arg(arg: &str) -> String {
    if arg.is_empty() {
        return "\"\"".to_string();
    }

    // 如果参数不包含特殊字符，直接返回
    if !arg
        .chars()
        .any(|c| matches!(c, ' ' | '"' | '\'' | '\\' | '\t' | '\n' | '\r'))
    {
        return arg.to_string();
    }

    // 需要转义的情况
    let mut escaped = String::with_capacity(arg.len() + 10);
    escaped.push('"');

    for c in arg.chars() {
        match c {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            _ => escaped.push(c),
        }
    }

    escaped.push('"');
    escaped
}

// From trait 实现
impl From<String> for AdbCommand {
    fn from(s: String) -> Self {
        AdbCommand::Single(s)
    }
}

impl From<&str> for AdbCommand {
    fn from(s: &str) -> Self {
        AdbCommand::Single(s.to_string())
    }
}

impl From<Vec<String>> for AdbCommand {
    fn from(args: Vec<String>) -> Self {
        AdbCommand::Multiple(args)
    }
}

impl From<Vec<&str>> for AdbCommand {
    fn from(args: Vec<&str>) -> Self {
        AdbCommand::Multiple(args.into_iter().map(String::from).collect())
    }
}

impl<const N: usize> From<[&str; N]> for AdbCommand {
    fn from(args: [&str; N]) -> Self {
        AdbCommand::Multiple(args.into_iter().map(String::from).collect())
    }
}

impl<const N: usize> From<&[&str; N]> for AdbCommand {
    fn from(args: &[&str; N]) -> Self {
        AdbCommand::Multiple(args.iter().map(|&s| String::from(s)).collect())
    }
}

impl From<&[&str]> for AdbCommand {
    fn from(args: &[&str]) -> Self {
        AdbCommand::Multiple(args.iter().map(|&s| String::from(s)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_command() {
        let cmd = AdbCommand::from("test");
        assert_eq!(cmd.get_command(), "test");
    }

    #[test]
    fn test_multiple_command() {
        let cmd = AdbCommand::from(vec!["adb", "shell", "ls"]);
        assert_eq!(cmd.get_command(), "adb shell ls");
    }

    #[test]
    fn test_command_with_spaces() {
        let cmd = AdbCommand::from(vec!["echo", "hello world"]);
        assert_eq!(cmd.get_command(), "echo \"hello world\"");
    }

    #[test]
    fn test_command_with_quotes() {
        let cmd = AdbCommand::from(vec!["echo", "say \"hello\""]);
        assert_eq!(cmd.get_command(), "echo \"say \\\"hello\\\"\"");
    }

    #[test]
    fn test_empty_argument() {
        let cmd = AdbCommand::from(vec!["test", ""]);
        assert_eq!(cmd.get_command(), "test \"\"");
    }

    #[test]
    fn test_shell_escape_simple() {
        assert_eq!(shell_escape_arg("simple"), "simple");
        assert_eq!(shell_escape_arg(""), "\"\"");
        assert_eq!(shell_escape_arg("hello world"), "\"hello world\"");
        assert_eq!(shell_escape_arg("test\"quote"), "\"test\\\"quote\"");
    }

    #[test]
    fn test_array_conversion() {
        let cmd = AdbCommand::from(["echo", "test"]);
        assert_eq!(cmd.get_command(), "echo test");

        let arr = ["echo", "hello world"];
        let cmd = AdbCommand::from(&arr);
        assert_eq!(cmd.get_command(), "echo \"hello world\"");
    }

    #[test]
    fn test_cow_optimization() {
        let cmd = AdbCommand::single("test");
        match cmd.get_command_cow() {
            Cow::Borrowed(s) => assert_eq!(s, "test"),
            Cow::Owned(_) => panic!("Should be borrowed"),
        }

        let cmd = AdbCommand::multiple(vec!["echo", "test"]);
        match cmd.get_command_cow() {
            Cow::Owned(s) => assert_eq!(s, "echo test"),
            Cow::Borrowed(_) => panic!("Should be owned"),
        }
    }
}
