use std::time::{Duration, SystemTime};

use conveyor::config::{Config, MergeMethod, Prs, Repo, Ui, UpdateMethod};
use conveyor::model::deployed::Health;
use conveyor::model::prs::StatusChange;
use conveyor::sources::demo::{self, Demo};
use conveyor::sources::fetch::{
    BuildsSource, DeploySource, fetch_jobs, fetch_log, fetch_prs, fetch_queue,
};
use conveyor::sources::github::Github;

#[test]
fn the_demo_watches_its_own_repositories_instead_of_the_users() {
    let user = Config {
        repo: vec![Repo {
            name: "someone/else".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let config = demo::config(user);

    assert_eq!(config.repo.len(), 3);
    assert_eq!(config.repo[0].name, "acme/webapp");
    assert_eq!(config.repo[1].name, "acme/infra");
    assert_eq!(config.repo[2].name, "acme/docs");
}

#[test]
fn the_demo_deploys_acme_webapp_to_four_environments() {
    let config = demo::config(Config::default());

    let envs = &config.repo[0].deploy[0].env;
    assert_eq!(envs.len(), 4);
    assert_eq!(envs[0].name, "dev");
    assert_eq!(envs[3].name, "uat");
}

#[test]
fn the_demo_has_its_own_my_prs_tabs() {
    let user = Config {
        prs: Prs {
            limit: 3,
            ..Default::default()
        },
        ..Default::default()
    };

    let config = demo::config(user);

    assert_eq!(config.prs.tab.len(), 2);
    assert_eq!(config.prs.tab[0].name, "mine");
    assert_eq!(config.prs.tab[1].name, "review");
    assert_eq!(config.prs.limit, Config::default().prs.limit);
}

#[test]
fn the_demo_review_tab_holds_other_pull_requests_than_mine() {
    let config = demo::config(Config::default());

    let review = fetch_prs(&Demo, &config, 1).unwrap();

    assert_eq!(review.len(), 2);
    assert_eq!(review[0].number, 4844);
    assert_eq!(review[1].number, 212);
    assert_eq!(review[1].repo, "acme/infra");
}

#[test]
fn the_demo_mine_tab_holds_pull_requests_the_viewer_opened() {
    let config = demo::config(Config::default());

    let mine = fetch_prs(&Demo, &config, 0).unwrap();

    assert!(mine[0].viewer_did_author);
    assert!(mine[4].viewer_did_author);
}

#[test]
fn the_demo_review_tab_holds_pull_requests_the_viewer_can_approve() {
    let config = demo::config(Config::default());

    let review = fetch_prs(&Demo, &config, 1).unwrap();

    assert!(!review[0].viewer_did_author);
    assert_eq!(review[0].viewer_approved_oid, None);
}

#[test]
fn the_demo_keeps_the_users_ui_settings() {
    let user = Config {
        ui: Ui {
            min_column_width: 33,
            ..Default::default()
        },
        ..Default::default()
    };

    let config = demo::config(user);

    assert_eq!(config.ui.min_column_width, 33);
}

#[test]
fn the_demo_queue_of_acme_webapp_holds_two_pull_requests_with_their_runs() {
    let config = demo::config(Config::default());

    let queue = fetch_queue(&Demo, &config.repo[0]).unwrap();

    assert_eq!(queue.entries.len(), 2);
    assert_eq!(queue.entries[0].number, 4836);
    assert_eq!(queue.entries[0].author, "erin");
    assert_eq!(queue.entries[0].run.as_ref().unwrap().id, 313);
    assert_eq!(queue.entries[1].number, 4827);
    assert_eq!(queue.entries[1].author, "me");
    assert_eq!(queue.entries[1].run.as_ref().unwrap().id, 314);
}

#[test]
fn my_demo_pull_request_in_the_queue_is_queued_in_my_prs_at_the_same_position() {
    let config = demo::config(Config::default());
    let queue = fetch_queue(&Demo, &config.repo[0]).unwrap();

    let mine = fetch_prs(&Demo, &config, 0).unwrap();

    let queued = mine.iter().find(|pr| pr.number == 4827).unwrap();
    assert_eq!(queued.queue_position, Some(queue.entries[1].position));
}

#[test]
fn my_demo_pull_requests_share_a_number_across_two_repositories() {
    let config = demo::config(Config::default());

    let mine = fetch_prs(&Demo, &config, 0).unwrap();

    let infra = mine.iter().find(|pr| pr.repo == "acme/infra").unwrap();
    assert_eq!(infra.number, 4821);
}

#[test]
fn only_my_queued_demo_pull_request_is_queued_in_my_prs() {
    let config = demo::config(Config::default());

    let mine = fetch_prs(&Demo, &config, 0).unwrap();

    let queued: Vec<u64> = mine
        .iter()
        .filter(|pr| pr.queue_position.is_some())
        .map(|pr| pr.number)
        .collect();
    assert_eq!(queued, vec![4827]);
}

#[test]
fn no_open_demo_pull_request_of_mine_was_already_built_on_main() {
    let config = demo::config(Config::default());
    let mine = fetch_prs(&Demo, &config, 0).unwrap();
    let mut source = BuildsSource::new(config.repo[0].clone());

    let builds = source.fetch(&Demo, SystemTime::now()).unwrap();

    let built_and_open: Vec<u64> = builds
        .iter()
        .filter_map(|build| build.pull.as_ref().map(|pull| pull.number))
        .filter(|number| mine.iter().any(|pr| pr.number == *number))
        .collect();
    assert!(built_and_open.is_empty(), "{built_and_open:?}");
}

#[test]
fn the_demo_queue_of_acme_infra_is_empty() {
    let config = demo::config(Config::default());

    let queue = fetch_queue(&Demo, &config.repo[1]).unwrap();

    assert!(queue.entries.is_empty());
}

#[test]
fn acme_docs_has_no_merge_queue_in_the_demo() {
    let config = demo::config(Config::default());

    let queue = fetch_queue(&Demo, &config.repo[2]).unwrap();

    assert!(queue.absent);
}

#[test]
fn the_demo_main_builds_of_acme_docs_name_their_own_pull_requests() {
    let config = demo::config(Config::default());
    let mut source = BuildsSource::new(config.repo[2].clone());

    let builds = source.fetch(&Demo, SystemTime::now()).unwrap();

    assert_eq!(builds.len(), 2);
    assert_eq!(builds[0].repo().as_deref(), Some("acme/docs"));
    assert_eq!(builds[0].pull.as_ref().unwrap().number, 4819);
    assert_eq!(builds[1].pull.as_ref().unwrap().number, 4806);
}

#[test]
fn the_demo_main_builds_are_four_runs_newest_first_with_their_pull_requests() {
    let config = demo::config(Config::default());
    let mut source = BuildsSource::new(config.repo[0].clone());

    let builds = source.fetch(&Demo, SystemTime::now()).unwrap();

    assert_eq!(builds.len(), 4);
    assert_eq!(builds[0].run_number, 312);
    assert_eq!(builds[0].pull.as_ref().unwrap().number, 4840);
    assert_eq!(builds[0].pull.as_ref().unwrap().author, "bob");
    assert_eq!(builds[1].pull.as_ref().unwrap().number, 4815);
    assert_eq!(builds[1].pull.as_ref().unwrap().author, "alice");
    assert_eq!(builds[3].pull.as_ref().unwrap().number, 4799);
}

#[test]
fn the_demo_main_builds_of_acme_infra_name_their_own_pull_requests() {
    let config = demo::config(Config::default());
    let mut source = BuildsSource::new(config.repo[1].clone());

    let builds = source.fetch(&Demo, SystemTime::now()).unwrap();

    assert_eq!(builds.len(), 2);
    assert_eq!(builds[0].repo().as_deref(), Some("acme/infra"));
    assert_eq!(builds[0].pull.as_ref().unwrap().number, 4817);
    assert_eq!(builds[1].pull.as_ref().unwrap().number, 4810);
}

#[test]
fn a_failed_demo_run_names_its_failed_step() {
    let jobs = fetch_jobs(&Demo, "acme/webapp", 311).unwrap();

    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].failed_step.as_deref(), Some("Run cargo test"));
}

#[test]
fn every_demo_run_has_jobs() {
    let jobs = fetch_jobs(&Demo, "acme/webapp", 1).unwrap();

    assert_eq!(jobs.len(), 2);
}

#[test]
fn the_demo_dev_runs_the_newest_main_build() {
    let deployed = demo_deployed();

    assert_eq!(deployed.rows[0].env, "dev");
    assert_eq!(
        deployed.rows[0].sha.as_deref(),
        Some("8c1f2a7d94e3b06f5a2c9d18e7b4f0a3c6d5e291")
    );
    assert_eq!(
        deployed.rows[0].pull.as_ref().map(|pull| pull.number),
        Some(4840)
    );
    assert_eq!(deployed.rows[0].health, Health::Healthy);
}

#[test]
fn the_demo_staging_rolls_out_a_crash_looping_pod() {
    let deployed = demo_deployed();

    assert_eq!(deployed.rows[1].env, "staging");
    assert_eq!(deployed.rows[1].pods.len(), 3);
    assert_eq!(
        deployed.rows[1].pods[0].stuck.as_deref(),
        Some("CrashLoopBackOff")
    );
    assert_eq!(
        deployed.rows[1].health,
        Health::Warning("1 of 3 pods CrashLoopBackOff".to_string())
    );
}

#[test]
fn the_demo_prod_runs_an_older_build() {
    let deployed = demo_deployed();

    assert_eq!(deployed.rows[2].env, "prod");
    assert_eq!(
        deployed.rows[2].pull.as_ref().map(|pull| pull.number),
        Some(4799)
    );
}

#[test]
fn the_demo_uat_cannot_be_read() {
    let deployed = demo_deployed();

    assert_eq!(deployed.rows[3].env, "uat");
    assert_eq!(
        deployed.rows[3].error.as_ref().map(|err| err.message()),
        Some("ERROR: Active profile expired.")
    );
}

#[test]
fn a_demo_merge_changes_nothing() {
    let merged = Demo.merge("acme/webapp", 4821, MergeMethod::Squash, "b7e4d2c9");

    assert_eq!(
        nothing_changed(merged.map(|_| ())),
        "demo: nothing was changed"
    );
}

#[test]
fn a_demo_branch_update_changes_nothing() {
    let updated = Demo.update_branch("acme/webapp", 4821, UpdateMethod::Merge);

    assert_eq!(
        nothing_changed(updated.map(|_| ())),
        "demo: nothing was changed"
    );
}

#[test]
fn a_demo_status_change_changes_nothing() {
    let changed = Demo.change_status("acme/webapp", 4790, &StatusChange::Ready);

    assert_eq!(nothing_changed(changed), "demo: nothing was changed");
}

#[test]
fn a_demo_approve_changes_nothing() {
    let approve = StatusChange::Approve {
        node_id: "PR_kwacme4844".to_string(),
        head_oid: "8c1f2a7d".to_string(),
    };

    let changed = Demo.change_status("acme/webapp", 4844, &approve);

    assert_eq!(nothing_changed(changed), "demo: nothing was changed");
}

#[test]
fn a_demo_checkout_changes_nothing() {
    let checked_out = Demo.checkout("acme/webapp", 4821, None);

    assert_eq!(nothing_changed(checked_out), "demo: nothing was changed");
}

#[test]
fn a_demo_rerun_changes_nothing() {
    let rerun = Demo.rerun_failed("acme/webapp", 311);

    assert_eq!(nothing_changed(rerun), "demo: nothing was changed");
}

#[test]
fn the_failed_main_build_log_belongs_to_its_pull_request() {
    let log = fetch_log(&Demo, "acme/webapp", 311, 31101).unwrap();

    let text = log.join("\n");
    assert!(text.contains("session"), "{text}");
    assert!(!text.contains("webhook"), "{text}");
}

#[test]
fn the_failed_my_prs_run_log_belongs_to_its_pull_request() {
    let log = fetch_log(&Demo, "acme/webapp", 1, 101).unwrap();

    let text = log.join("\n");
    assert!(text.contains("webhook"), "{text}");
}

#[test]
fn every_demo_main_build_and_merge_group_run_holds_its_jobs_in_time() {
    let config = demo::config(Config::default());
    let mut source = BuildsSource::new(config.repo[0].clone());
    let builds = source.fetch(&Demo, SystemTime::now()).unwrap();
    let mut infra = BuildsSource::new(config.repo[1].clone());
    let infra_builds = infra.fetch(&Demo, SystemTime::now()).unwrap();
    let mut docs = BuildsSource::new(config.repo[2].clone());
    let docs_builds = docs.fetch(&Demo, SystemTime::now()).unwrap();
    let queue = fetch_queue(&Demo, &config.repo[0]).unwrap();
    let runs = builds
        .iter()
        .chain(infra_builds.iter())
        .chain(docs_builds.iter())
        .chain(queue.entries.iter().filter_map(|entry| entry.run.as_ref()));

    let outside: Vec<u64> = runs
        .filter(|run| {
            fetch_jobs(&Demo, "acme/webapp", run.id)
                .unwrap()
                .iter()
                .any(|job| !within(job.started_at, run) || !within(job.completed_at, run))
        })
        .map(|run| run.id)
        .collect();
    assert!(outside.is_empty(), "{outside:?}");
}

#[test]
fn the_failed_demo_step_has_a_log() {
    let log = fetch_log(&Demo, "acme/webapp", 311, 31101).unwrap();

    assert_eq!(
        log.last().map(String::as_str),
        Some("error: test failed, to rerun pass `--lib`")
    );
}

fn demo_deployed() -> conveyor::model::deployed::Deployed {
    let config = demo::config(Config::default());
    let repo = &config.repo[0];
    let mut source = DeploySource::new(repo.name.clone(), repo.deploy[0].clone());
    source.fetch(&Demo, &Demo, SystemTime::now())
}

fn within(at: Option<SystemTime>, run: &conveyor::model::builds::Build) -> bool {
    let slack = Duration::from_secs(1);
    let Some(at) = at else {
        return true;
    };
    let after_start = run.started_at.is_some_and(|start| at + slack >= start);
    let before_end = !run.is_settled() || run.finished_at.is_none_or(|end| at <= end + slack);
    after_start && before_end
}

fn nothing_changed(result: Result<(), conveyor::sources::failure::Failure>) -> String {
    result.unwrap_err().message().to_string()
}
