use conveyor::tmux::links_notice;

const ADD: &str = "tmux drops the #N links: add set -as terminal-features ',xterm-ghostty:hyperlinks' to tmux.conf and reattach";

#[test]
fn a_detached_server_has_no_client_to_tell() {
    assert_eq!(links_notice("\t3.4\t\n"), None);
}

#[test]
fn a_tmux_older_than_3_4_cannot_pass_links_at_all() {
    assert_eq!(
        links_notice("xterm-ghostty\t3.3a\tclipboard,title\n").as_deref(),
        Some("tmux 3.3a drops the #N links: tmux 3.4 or later passes them through")
    );
}

#[test]
fn versions_compare_as_numbers_not_text() {
    assert_eq!(
        links_notice("xterm-ghostty\t3.10\ttitle\n").as_deref(),
        Some(ADD)
    );
}

#[test]
fn a_build_from_master_reads_as_the_release_it_leads_to() {
    assert_eq!(
        links_notice("xterm-ghostty\tnext-3.4\ttitle\n").as_deref(),
        Some(ADD)
    );
}

#[test]
fn a_version_that_does_not_parse_gets_the_line_to_add() {
    assert_eq!(
        links_notice("xterm-ghostty\topenbsd-7.6\ttitle\n").as_deref(),
        Some(ADD)
    );
}

#[test]
fn a_term_holding_a_control_character_is_never_echoed() {
    assert_eq!(links_notice("xterm\u{1b}]8;;x\t3.4\ttitle\n"), None);
}
