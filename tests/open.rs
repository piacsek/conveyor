use conveyor::open::{Opener, SystemOpener, web_url};

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

#[test]
fn only_web_urls_reach_the_platform_opener() {
    for url in ["https://ci/test", "HTTPS://ci/test", "http://ci/test"] {
        assert_eq!(web_url(url).expect(url), url);
    }
    for url in [
        "file:///etc/passwd",
        "/Applications/Calculator.app",
        "-a Calculator",
        "vscode://file/etc/passwd",
        "https://",
        "",
        "ĥttps://x",
    ] {
        let err = web_url(url).expect_err(url).to_string();
        assert!(err.contains("not an http(s) URL"), "{url}: {err}");
        let err = SystemOpener
            .open(url)
            .expect_err("the system opener refuses before it spawns")
            .to_string();
        assert!(err.starts_with("refusing to open"), "{url}: {err}");
    }
}
