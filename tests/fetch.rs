use std::cell::RefCell;
use std::io;

use conveyor::config::Config;
use conveyor::fetch::fetch_prs;
use conveyor::github::{Github, Variable};

type Call = (String, Vec<(String, Variable)>);

struct FakeGithub {
    calls: RefCell<Vec<Call>>,
    response: io::Result<serde_json::Value>,
}

impl Github for FakeGithub {
    fn graphql(&self, query: &str, vars: &[(&str, Variable)]) -> io::Result<serde_json::Value> {
        self.calls.borrow_mut().push((
            query.to_string(),
            vars.iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        ));
        match &self.response {
            Ok(value) => Ok(value.clone()),
            Err(err) => Err(io::Error::new(err.kind(), err.to_string())),
        }
    }
}

#[test]
fn fetch_prs_runs_the_configured_search_and_parses_the_rows() {
    let gh = FakeGithub {
        calls: RefCell::default(),
        response: Ok(serde_json::from_str(include_str!("fixtures/prs.json")).unwrap()),
    };
    let mut config = Config::default();
    config.prs.query = "is:pr involves:@me".to_string();
    config.prs.limit = 7;

    let prs = fetch_prs(&gh, &config).unwrap();

    assert_eq!(prs.len(), 1);
    assert_eq!(prs[0].number, 1);
    let calls = gh.calls.borrow();
    assert!(
        calls[0]
            .0
            .contains("search(query: $q, type: ISSUE, first: $first)")
    );
    assert_eq!(
        calls[0].1,
        vec![
            ("q".to_string(), Variable::from("is:pr involves:@me")),
            ("first".to_string(), Variable::from(7usize)),
        ]
    );
}

#[test]
fn fetch_prs_turns_gh_errors_into_a_message() {
    let gh = FakeGithub {
        calls: RefCell::default(),
        response: Err(io::Error::other("gh: HTTP 401: Bad credentials")),
    };

    let err = fetch_prs(&gh, &Config::default()).unwrap_err();

    assert_eq!(err, "gh: HTTP 401: Bad credentials");
}
