use crate::app::{Action, App, Stage};
use crate::sources::failure::FailureKind;

use super::{Choice, Offer, WriteKind, offer, timed_out};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rerun {
    pub repo: String,
    pub run_ids: Vec<u64>,
    pub label: String,
}

pub(in crate::app) fn rerun_offer(target: Result<Rerun, String>) -> Offer {
    let label = match &target {
        Ok(rerun) if rerun.run_ids.len() > 1 => {
            format!("rerun failed jobs ({} runs)", rerun.run_ids.len())
        }
        _ => "rerun failed jobs".to_string(),
    };
    offer('f', &label, target.map(Choice::Rerun))
}

impl App {
    pub(super) fn send_rerun(&mut self, rerun: Rerun) -> Action {
        let subject = self.selected_key().unwrap_or_default();
        self.start_write(
            subject,
            WriteKind::Rerun {
                run_ids: rerun.run_ids.clone(),
                label: rerun.label,
            },
        );
        Action::Rerun {
            repo: rerun.repo,
            run_ids: rerun.run_ids,
        }
    }

    pub fn receive_rerun(
        &mut self,
        results: Vec<(u64, Result<(), crate::sources::failure::Failure>)>,
    ) -> Option<Stage> {
        let index = self.writes.iter().position(|write| {
            matches!(&write.kind, WriteKind::Rerun { run_ids, .. }
                if results.iter().any(|(run_id, _)| run_ids.contains(run_id)))
        })?;
        let write = self.writes.remove(index);
        let stage = write.stage;
        let WriteKind::Rerun { label, .. } = write.kind.clone() else {
            return None;
        };
        let mut ordered: Vec<_> = results.iter().collect();
        ordered.sort_by_key(|(_, result)| timed_out(result));
        let errors: Vec<String> = ordered
            .iter()
            .filter_map(|(run_id, result)| {
                result
                    .as_ref()
                    .err()
                    .map(|message| format!("run {run_id}: {message}"))
            })
            .collect();
        let unsure = results
            .iter()
            .filter(|(_, result)| timed_out(result))
            .count();
        let failed = errors.len() - unsure;
        let reran = results.len() - errors.len();
        let failures = results
            .iter()
            .filter_map(|(_, result)| result.as_ref().err());
        let cause = failures
            .clone()
            .find(|failure| failure.kind() != FailureKind::TimedOut)
            .or_else(|| failures.clone().next())
            .cloned();
        match cause {
            None => {}
            Some(cause) if failed == 0 => self.fail(Some(write), &cause),
            Some(cause) if reran == 0 && unsure == 0 => {
                self.fail(Some(write), &cause.reworded(errors.join("; ")))
            }
            Some(cause) => self.fail(
                Some(write),
                &cause.reworded(format!(
                    "{failed} of {} runs failed: {}",
                    results.len(),
                    errors.join("; ")
                )),
            ),
        }
        self.notice = Some(match (reran, failed) {
            _ if errors.is_empty() => format!("rerun requested: {label}"),
            (0, 0) => errors.join("; "),
            (0, _) => format!("rerun failed: {}", errors.join("; ")),
            _ => format!(
                "rerun requested for {reran} of {} runs; {}",
                results.len(),
                errors.join("; ")
            ),
        });
        (reran > 0 || unsure > 0).then_some(stage)
    }
}
