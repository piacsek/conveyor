use crate::app::{Action, App, Stage};
use crate::model::builds::BuildStatus;

use super::{Choice, Offer, offer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rerun {
    pub repo: String,
    pub run_ids: Vec<u64>,
    pub label: String,
}

impl App {
    pub(super) fn rerun_offer(&self) -> Offer {
        let target = self.rerun_target();
        let label = match &target {
            Ok(rerun) if rerun.run_ids.len() > 1 => {
                format!("rerun failed jobs ({} runs)", rerun.run_ids.len())
            }
            _ => "rerun failed jobs".to_string(),
        };
        offer('f', &label, target.map(Choice::Rerun))
    }

    pub(super) fn send_rerun(&mut self, rerun: Rerun) -> Action {
        self.notice = Some(format!("rerunning {}…", rerun.label));
        self.rerunning = Some((self.focus, rerun.label));
        Action::Rerun {
            repo: rerun.repo,
            run_ids: rerun.run_ids,
        }
    }

    pub fn receive_rerun(&mut self, results: Vec<(u64, Result<(), String>)>) -> Option<Stage> {
        let (stage, label) = self.rerunning.take()?;
        let errors: Vec<String> = results
            .iter()
            .filter_map(|(run_id, result)| {
                result
                    .as_ref()
                    .err()
                    .map(|message| format!("run {run_id}: {message}"))
            })
            .collect();
        let reran = results.len() - errors.len();
        self.notice = Some(match (reran, errors.is_empty()) {
            (_, true) => format!("rerun requested: {label}"),
            (0, false) => format!("rerun failed: {}", errors.join("; ")),
            _ => format!(
                "rerun requested for {reran} of {} runs; {}",
                results.len(),
                errors.join("; ")
            ),
        });
        (reran > 0).then_some(stage)
    }

    fn rerun_target(&self) -> Result<Rerun, String> {
        if matches!(self.focus, Stage::Builds | Stage::Prs) && self.focused_error().is_some() {
            return Err("the last refresh failed; press r and try again".to_string());
        }
        match self.focus {
            Stage::Builds => {
                let build = self.builds.column.selected().ok_or("nothing selected")?;
                let label = format!("run {}", build.run_number);
                let newest = self.builds.column.all().iter().map(|b| b.run_number).max();
                if newest != Some(build.run_number) {
                    return Err("only the newest main build can be rerun".to_string());
                }
                if !build.is_settled() {
                    return Err(format!("{label} is still running"));
                }
                if !matches!(build.status, BuildStatus::Failure | BuildStatus::Cancelled) {
                    return Err("nothing failed to rerun".to_string());
                }
                let repo = self.builds.repo.clone().ok_or("no repository known")?;
                Ok(Rerun {
                    repo,
                    run_ids: vec![build.id],
                    label,
                })
            }
            Stage::Prs => {
                let pr = self.prs.column.selected().ok_or("nothing selected")?;
                let run_ids = pr.rerunnable_runs();
                let label = match run_ids.len() {
                    0 => return Err("no finished failing run to rerun".to_string()),
                    1 => format!("#{}", pr.number),
                    runs => format!("#{} ({runs} runs)", pr.number),
                };
                Ok(Rerun {
                    repo: pr.repo.clone(),
                    run_ids,
                    label,
                })
            }
            Stage::Queue => Err("nothing to rerun in the merge queue".to_string()),
            Stage::Deployed => Err("nothing to rerun here".to_string()),
        }
    }
}
