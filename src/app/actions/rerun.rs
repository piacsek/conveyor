use crate::app::{Action, App, Stage};

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
        let first = results.iter().find_map(|(_, result)| result.clone().err());
        match (reran, first) {
            (_, None) => {}
            (0, Some(first)) => self.fail(Some(write), &first.reworded(errors.join("; "))),
            (_, Some(first)) => self.fail(
                Some(write),
                &first.reworded(format!(
                    "{} of {} runs failed: {}",
                    errors.len(),
                    results.len(),
                    errors.join("; ")
                )),
            ),
        }
        let timed_out = results.iter().any(|(_, result)| timed_out(result));
        self.notice = Some(match (reran, errors.is_empty()) {
            (_, true) => format!("rerun requested: {label}"),
            (0, false) if timed_out => errors.join("; "),
            (0, false) => format!("rerun failed: {}", errors.join("; ")),
            _ => format!(
                "rerun requested for {reran} of {} runs; {}",
                results.len(),
                errors.join("; ")
            ),
        });
        (reran > 0 || timed_out).then_some(stage)
    }
}
