use conveyor::tmux::links_fix;

fn add(term: &str) -> Vec<String> {
    vec![
        "tmux drops the #N links for this terminal. If it shows links, run".to_string(),
        format!("  tmux set -as terminal-features ',{term}:hyperlinks'"),
        "then detach and reattach; add that line to tmux.conf to keep it.".to_string(),
    ]
}

#[test]
fn a_detached_server_has_no_client_to_tell() {
    assert!(links_fix("\t3.4\t\n").is_empty());
}

#[test]
fn a_client_with_hyperlinks_needs_nothing() {
    assert!(links_fix("xterm-ghostty\t3.4\tclipboard,hyperlinks\n").is_empty());
}

#[test]
fn a_client_without_hyperlinks_gets_the_command_for_its_own_term() {
    assert_eq!(
        links_fix("xterm-ghostty\t3.4\tclipboard,title\n"),
        add("xterm-ghostty")
    );
}

#[test]
fn a_tmux_older_than_3_4_cannot_pass_links_at_all() {
    assert_eq!(
        links_fix("xterm-ghostty\t3.3a\tclipboard,title\n"),
        vec!["tmux 3.3a drops the #N links: tmux 3.4 or later passes them through"]
    );
}

#[test]
fn versions_compare_as_numbers_not_text() {
    assert_eq!(
        links_fix("xterm-ghostty\t3.10\ttitle\n"),
        add("xterm-ghostty")
    );
}

#[test]
fn a_build_from_master_reads_as_the_release_it_leads_to() {
    assert_eq!(
        links_fix("xterm-ghostty\tnext-3.4\ttitle\n"),
        add("xterm-ghostty")
    );
}

#[test]
fn a_version_that_does_not_parse_gets_the_command() {
    assert_eq!(
        links_fix("xterm-ghostty\topenbsd-7.6\ttitle\n"),
        add("xterm-ghostty")
    );
}

#[test]
fn a_term_holding_a_control_character_is_never_echoed() {
    assert!(links_fix("xterm\u{1b}]8;;x\t3.4\ttitle\n").is_empty());
}
