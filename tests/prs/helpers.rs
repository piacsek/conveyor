use crate::support::pr;

pub fn three() -> Vec<conveyor::model::prs::PullRequest> {
    vec![pr(1, "a"), pr(2, "b"), pr(3, "c")]
}

pub fn highlighted_row(screen: &str) -> usize {
    screen
        .lines()
        .position(|line| line.starts_with("│▌"))
        .expect("a highlighted row")
}
