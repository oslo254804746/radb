use radb::client::AdbDevice;

#[test]
fn list2cmdline_quotes_shell_arguments_with_spaces() {
    let args = ["echo", "hello world", "plain"];
    let cmdline = AdbDevice::<&str>::list2cmdline(&args);

    assert_eq!(cmdline, "echo \"hello world\" plain");
}
