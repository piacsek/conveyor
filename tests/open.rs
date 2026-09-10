use conveyor::open::SystemOpener;

#[test]
fn open_and_copy_use_the_platform_tools() {
    let (open, copy) = (
        SystemOpener::open_args("https://x"),
        SystemOpener::copy_args(),
    );
    if cfg!(target_os = "macos") {
        assert_eq!(open, vec!["open", "https://x"]);
        assert_eq!(copy, vec!["pbcopy"]);
    } else {
        assert_eq!(open, vec!["xdg-open", "https://x"]);
        assert_eq!(copy, vec!["xclip", "-selection", "clipboard"]);
    }
}
