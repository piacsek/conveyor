use std::io;

use conveyor::config::Config;

use crate::fakes::FakeGithub;

#[test]
fn repos_come_from_the_config_or_else_from_the_current_directory() {
    use conveyor::config::Repo;
    use conveyor::sources::fetch::repos;
    let gh = FakeGithub::with_response(Ok(serde_json::Value::Null));

    let mut config = Config::default();
    config.repo.push(Repo {
        name: "acme/webapp".to_string(),
        ..Repo::default()
    });
    assert_eq!(
        repos(&gh, &config)
            .unwrap()
            .iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>(),
        vec!["acme/webapp"]
    );

    let fallback = repos(&gh, &Config::default()).unwrap();
    assert_eq!(fallback.len(), 1);
    assert_eq!(fallback[0].name, "acme/from-cwd");
    assert_eq!(fallback[0].main_workflow, "CI/CD");

    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.cwd_repo = Err(io::Error::other("not a git repository"));
    let err = repos(&gh, &Config::default()).unwrap_err();
    assert!(err.contains("not a git repository"), "{err}");
    assert!(err.contains("[[repo]]"), "hints at the config: {err}");
}
