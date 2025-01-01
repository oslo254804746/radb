#[derive(Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum AdbCommand<'a> {
    Slice(&'a [&'a str]),
    String(&'a str),
}

impl<'b, 'a> Into<AdbCommand<'a>> for &'b [&str]
where
    'b: 'a,
{
    fn into(self) -> AdbCommand<'a> {
        AdbCommand::Slice(self.as_ref())
    }
}
impl<'a> AdbCommand<'a> {
    pub fn get_command(&self) -> String {
        match self {
            AdbCommand::Slice(s) => s.join(" "),
            AdbCommand::String(s) => s.to_string(),
        }
    }
}

impl<'a> Into<AdbCommand<'a>> for &'a str {
    fn into(self) -> AdbCommand<'a> {
        AdbCommand::String(self)
    }
}

impl<'a, const N: usize> Into<AdbCommand<'a>> for &'a [&'a str; N] {
    fn into(self) -> AdbCommand<'a> {
        AdbCommand::Slice(self)
    }
}
impl<'a> Into<AdbCommand<'a>> for &'a Vec<&'a str> {
    fn into(self) -> AdbCommand<'a> {
        AdbCommand::Slice(self)
    }
}

#[test]
fn test_into() {
    let a = "a";
    let b = ["a", "b", "c"];
    let c = vec![a, "b", "c"];
    let d = vec!["a".to_string()];
    assert_eq!(AdbCommand::String(a), a.into());
    assert_eq!(AdbCommand::Slice(&b), (&b).into());
    assert_eq!(AdbCommand::Slice(&c), (&c).into());
}
