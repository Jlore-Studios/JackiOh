//! Environment mapping for CREATE_ISSUES_TOKEN and GITHUB_ISSUES_REPO.

use jackioh_server::config::BUG_REPORT_REPO_DEFAULT;
use jackioh_server::env::{SERVER_ONLY_ENV_VARS, load_env};

use crate::support::deps::env_with;

#[test]
fn unset_files_nothing_and_names_the_default_repository() {
    let env = load_env(&env_with(&[])).expect("a valid environment");
    assert_eq!(env.github_issues_token, None);
    assert_eq!(env.github_issues_repo, BUG_REPORT_REPO_DEFAULT);
}

#[test]
fn a_blank_token_is_no_token() {
    let env = load_env(&env_with(&[("CREATE_ISSUES_TOKEN", "  ")])).expect("a valid environment");
    assert_eq!(env.github_issues_token, None);
}

#[test]
fn keeps_the_token_out_of_the_debug_print() {
    let env =
        load_env(&env_with(&[("CREATE_ISSUES_TOKEN", "github_pat_secret")])).expect("a valid environment");
    assert_eq!(env.github_issues_token.as_deref(), Some("github_pat_secret"));
    assert!(!format!("{env:?}").contains("github_pat_secret"));
}

#[test]
fn takes_an_owner_and_name_and_refuses_anything_else() {
    let env = load_env(&env_with(&[("GITHUB_ISSUES_REPO", "someone/fork")])).expect("a valid environment");
    assert_eq!(env.github_issues_repo, "someone/fork");
    for bad in ["no-slash", "a/b/c", "a/", "a b/c"] {
        let refused = load_env(&env_with(&[("GITHUB_ISSUES_REPO", bad)])).expect_err("not an owner/name");
        assert!(refused.to_string().contains("GITHUB_ISSUES_REPO"), "{refused}");
    }
}

#[test]
fn both_are_server_only() {
    assert!(SERVER_ONLY_ENV_VARS.contains(&"CREATE_ISSUES_TOKEN"));
    assert!(SERVER_ONLY_ENV_VARS.contains(&"GITHUB_ISSUES_REPO"));
}
