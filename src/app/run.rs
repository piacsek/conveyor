use std::io;

use ratatui::Terminal;
use ratatui::backend::Backend;

use crate::model::prs::StatusChange;
use crate::open::Opener;
use crate::ui;

use super::{Action, App, Input, Request, Stage};

fn attempt(app: &mut App, result: io::Result<()>) {
    if let Err(err) = result {
        app.notice = Some(err.to_string());
    }
}

pub fn run<B, O, R>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    inputs: impl Iterator<Item = io::Result<Input>>,
    opener: &O,
    mut request: R,
) -> io::Result<()>
where
    B: Backend,
    B::Error: Send + Sync + 'static,
    O: Opener,
    R: FnMut(Request),
{
    let mut inputs = inputs;
    loop {
        terminal
            .draw(|frame| ui::draw(frame, app))
            .map_err(io::Error::other)?;
        let Some(input) = inputs.next() else {
            return Ok(());
        };
        match input? {
            Input::Tick(now) => {
                app.now = now;
                if app.follow_up_due() {
                    request(Request::Refresh(Stage::Prs));
                }
            }
            Input::Fetching(stage) => app.fetching(stage),
            Input::Data(stage, data) => app.receive(stage, data),
            Input::Jobs(run_id, result) => app.receive_jobs(run_id, result),
            Input::Log(job_id, result) => app.receive_log(job_id, result),
            Input::BranchUpdated(number, result) => {
                if app.receive_branch_update(number, &result) {
                    request(Request::Refresh(Stage::Prs));
                    app.follow_up_prs();
                }
            }
            Input::Merged(number, result) => {
                app.receive_merged(number, &result);
                if result.is_ok() {
                    request(Request::Refresh(Stage::Prs));
                    request(Request::Refresh(Stage::Queue));
                }
            }
            Input::StatusChanged(number, change, result) => {
                if app.receive_status(number, &change, &result) {
                    request(Request::Refresh(Stage::Prs));
                    if change != StatusChange::Ready {
                        request(Request::Refresh(Stage::Queue));
                    }
                    app.follow_up_prs();
                }
            }
            Input::CheckedOut(target, result) => app.receive_checkout(&target, &result),
            Input::Rerun(result) => {
                if let Some(stage) = app.receive_rerun(result) {
                    request(Request::Refresh(stage));
                    if stage == Stage::Prs {
                        app.follow_up_prs();
                    }
                }
            }
            Input::Key(key) => match app.handle_key(key) {
                Action::Quit => return Ok(()),
                Action::Open(url) => attempt(app, opener.open(&url)),
                Action::Copy(text) => attempt(app, opener.copy(&text)),
                Action::Refresh(stage) => request(Request::Refresh(stage)),
                Action::RefreshAll => {
                    for stage in Stage::ALL {
                        request(Request::Refresh(stage));
                    }
                }
                Action::Log { run_id, job_id } => request(Request::Log { run_id, job_id }),
                Action::Merge(target) => request(Request::Merge(target)),
                Action::UpdateBranch(target) => request(Request::UpdateBranch(target)),
                Action::Rerun { repo, run_ids } => request(Request::Rerun { repo, run_ids }),
                Action::ChangeStatus(target) => request(Request::ChangeStatus(target)),
                Action::Checkout(target) => request(Request::Checkout(target)),
                Action::Continue => {}
            },
        }
        if let Some(run_id) = app.jobs_needed() {
            request(Request::Jobs { run_id });
        }
    }
}
