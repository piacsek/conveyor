use std::time::SystemTime;

use crate::app::actions::{conflicts, offer, rerun_offer};
use crate::app::{
    CheckoutTarget, Choice, Column, Listing, Menu, MergeTarget, Offer, Rerun, Row, StatusTarget,
    UpdateTarget,
};
use crate::config::UpdateMethod;
use crate::model::prs::{MergeState, PullRequest, StatusChange};

use super::{Context, Rows, StageView};

#[derive(Debug, Clone)]
pub struct PrsStage {
    pub views: Vec<Column<PullRequest>>,
    pub active: usize,
}

impl StageView for PrsStage {
    fn listing(&self) -> &dyn Listing {
        self.column()
    }

    fn listing_mut(&mut self) -> &mut dyn Listing {
        self.column_mut()
    }

    fn views(&self) -> usize {
        self.views.len()
    }

    fn active_view(&self) -> usize {
        self.active
    }

    fn view(&self, view: usize) -> &dyn Listing {
        &self.views[view]
    }

    fn view_mut(&mut self, view: usize) -> &mut dyn Listing {
        &mut self.views[view]
    }

    fn receive(&mut self, view: usize, rows: Rows, now: SystemTime) {
        if let Rows::Prs(prs) = rows {
            self.views[view].receive(prs, now);
        }
    }

    fn pull_target(&self) -> Result<(u64, String), String> {
        self.column()
            .selected()
            .map(|pr| (pr.key(), pr.url.clone()))
            .ok_or_else(|| "nothing selected".to_string())
    }

    fn build_url(&self, _: &Context) -> Result<String, String> {
        self.column()
            .selected()
            .and_then(|pr| {
                pr.checks_failures_first()
                    .into_iter()
                    .find(|check| !check.url.is_empty())
                    .map(|check| check.url.clone())
            })
            .ok_or_else(|| "no checks yet".to_string())
    }

    fn menu(&self, context: &Context) -> Result<Menu, String> {
        if self.column().error.is_some() {
            return Err("the last refresh failed; press r and try again".to_string());
        }
        let pr = self.column().selected().ok_or("nothing selected")?;
        Ok(Menu {
            title: format!("Manage #{}", pr.number),
            offers: self.offers(pr, context),
        })
    }
}

impl PrsStage {
    pub fn new(views: usize) -> Self {
        Self {
            views: vec![Column::default(); views],
            active: 0,
        }
    }

    pub fn column(&self) -> &Column<PullRequest> {
        &self.views[self.active]
    }

    pub fn column_mut(&mut self) -> &mut Column<PullRequest> {
        &mut self.views[self.active]
    }

    fn offers(&self, pr: &PullRequest, context: &Context) -> Vec<Offer> {
        let status = |change| {
            Ok(Choice::Status(StatusTarget {
                number: pr.number,
                repo: pr.repo.clone(),
                title: pr.title.clone(),
                head_oid: pr.head_oid.clone(),
                change,
            }))
        };
        let update = match context.config.prs.update_method {
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
        } else if context.landing(pr.number) {
            offer('t', "convert to draft", Err(waiting(pr.number)))
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
                    method: context.config.prs.merge_method,
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
            offer('u', update, self.update_target(context).map(Choice::Update)),
            rerun_offer(self.rerun_target()),
            draft,
            auto,
            offer('e', "leave the merge queue", dequeue),
            offer(
                'c',
                "check out locally",
                Ok(Choice::Checkout(checkout_target(pr, context))),
            ),
            offer('x', "close PR", status(StatusChange::Close)),
        ]
    }

    fn merge_target(&self) -> Result<MergeTarget, String> {
        let pr = self.column().selected().ok_or("nothing selected")?;
        if pr.is_draft {
            return Err(format!("#{} is a draft", pr.number));
        }
        if pr.queue_position.is_some() {
            return Err(format!("#{} is already queued", pr.number));
        }
        Ok(MergeTarget {
            number: pr.number,
            repo: pr.repo.clone(),
            title: pr.title.clone(),
            head_oid: pr.head_oid.clone(),
        })
    }

    fn update_target(&self, context: &Context) -> Result<UpdateTarget, String> {
        let pr = self.column().selected().ok_or("nothing selected")?;
        if context.landing(pr.number) {
            return Err(waiting(pr.number));
        }
        if pr.queue_position.is_some() {
            return Err(format!(
                "#{} is in the merge queue; updating it would drop it",
                pr.number
            ));
        }
        if pr.merge_state == MergeState::Dirty {
            return Err(conflicts(pr.number));
        }
        Ok(UpdateTarget {
            number: pr.number,
            repo: pr.repo.clone(),
            title: pr.title.clone(),
            head_oid: pr.head_oid.clone(),
            method: context.config.prs.update_method,
        })
    }

    fn rerun_target(&self) -> Result<Rerun, String> {
        let pr = self.column().selected().ok_or("nothing selected")?;
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
}

fn waiting(number: u64) -> String {
    format!("waiting for My PRs to catch up with #{number}")
}

fn checkout_target(pr: &PullRequest, context: &Context) -> CheckoutTarget {
    CheckoutTarget {
        number: pr.number,
        repo: pr.repo.clone(),
        dir: context
            .config
            .repo
            .iter()
            .filter(|repo| repo.name.eq_ignore_ascii_case(&pr.repo))
            .find_map(|repo| repo.path.clone()),
    }
}
