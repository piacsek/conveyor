use crate::app::App;
use crate::config::UpdateMethod;
use crate::model::prs::{PullRequest, StatusChange};

use super::{Choice, Menu, Offer, StatusTarget, offer};

impl App {
    pub(super) fn prs_menu(&self) -> Result<Menu, String> {
        let pr = self.prs.selected().ok_or("nothing selected")?;
        Ok(Menu {
            title: format!("Manage #{}", pr.number),
            offers: self.pr_offers(pr),
        })
    }

    fn pr_offers(&self, pr: &PullRequest) -> Vec<Offer> {
        let status = |change| {
            Ok(Choice::Status(StatusTarget {
                number: pr.number,
                repo: pr.repo.clone(),
                title: pr.title.clone(),
                head_oid: pr.head_oid.clone(),
                change,
            }))
        };
        let update = match self.config.prs.update_method {
            UpdateMethod::Merge => "update branch",
            UpdateMethod::Rebase => "rebase branch",
        };
        let queued = pr.queue_position.is_some();
        let draft = if pr.is_draft {
            offer('t', "mark ready", status(StatusChange::Ready))
        } else if queued {
            offer(
                't',
                "convert to draft",
                Err(format!("#{} is in the merge queue", pr.number)),
            )
        } else if let Some((number, _)) = self.merge_landing {
            offer(
                't',
                "convert to draft",
                Err(format!("waiting for My PRs to catch up with #{number}")),
            )
        } else {
            offer('t', "convert to draft", status(StatusChange::Draft))
        };
        let auto = if pr.can_disable_auto_merge {
            offer(
                'a',
                "disable auto-merge",
                status(StatusChange::DisableAutoMerge {
                    node_id: pr.node_id.clone(),
                }),
            )
        } else if pr.can_enable_auto_merge {
            offer(
                'a',
                "enable auto-merge",
                status(StatusChange::EnableAutoMerge {
                    node_id: pr.node_id.clone(),
                    method: self.config.prs.merge_method,
                    head_oid: pr.head_oid.clone(),
                }),
            )
        } else {
            offer(
                'a',
                "enable auto-merge",
                Err(format!("GitHub does not allow it for #{}", pr.number)),
            )
        };
        let dequeue = if queued {
            status(StatusChange::Dequeue {
                node_id: pr.node_id.clone(),
            })
        } else {
            Err(format!("#{} is not queued", pr.number))
        };
        let merge = if pr.merge_queue_enabled {
            offer(
                'm',
                "add to merge queue",
                self.merge_target().and_then(|_| {
                    status(StatusChange::Enqueue {
                        node_id: pr.node_id.clone(),
                        head_oid: pr.head_oid.clone(),
                    })
                }),
            )
        } else {
            offer('m', "merge", self.merge_target().map(Choice::Merge))
        };
        vec![
            merge,
            offer('u', update, self.update_target().map(Choice::Update)),
            self.rerun_offer(),
            draft,
            auto,
            offer('e', "leave the merge queue", dequeue),
            offer(
                'c',
                "check out locally",
                Ok(Choice::Checkout(self.checkout_target(pr))),
            ),
            offer('x', "close", status(StatusChange::Close)),
        ]
    }
}
