//! GITHUB_ISSUES_TOKEN and GITHUB_ISSUES_REPO -> `Env.github_issues_token` and
//! `Env.github_issues_repo` (src/env.rs): where the server files a match its own log does not
//! reproduce (`bug_reports.rs`). Both optional: with no token a finding is only logged.

use indexmap::IndexMap;
use jackioh_server::config::BUG_REPORT_REPO_DEFAULT;
use jackioh_server::env::{SERVER_ONLY_ENV_VARS, load_env};

/// A complete, valid environment for the server, with `extra` over it.
fn env_with(extra: &[(&str, &str)]) -> IndexMap<String, String> {
    [
        ("SUPABASE_URL", "https://project.supabase.test"),
        (
            "SUPABASE_SECRET_KEY",
            "sb_secret_0123456789abcdefghijklmnopqrstuv",
        ),
        (
            "DATABASE_URL",
            "postgres://postgres:postgres@localhost:5432/jackioh",
        ),
        ("CODE_PEPPER", "a-pepper-of-at-least-thirty-two-characters"),
        ("CATALOG_VERSION", jackioh_cards::catalog_version()),
        ("PUBLIC_ORIGINS", "https://play.jackioh.test"),
        ("NODE_ENV", "test"),
    ]
    .into_iter()
    .chain(extra.iter().copied())
    .map(|(name, value)| (name.to_string(), value.to_string()))
    .collect()
}

#[test]
fn unset_files_nothing_and_names_the_default_repository() {
    let env = load_env(&env_with(&[])).expect("a valid environment");
    assert_eq!(env.github_issues_token, None);
    assert_eq!(env.github_issues_repo, BUG_REPORT_REPO_DEFAULT);
}

#[test]
fn a_blank_token_is_no_token() {
    let env = load_env(&env_with(&[("GITHUB_ISSUES_TOKEN", "  ")])).expect("a valid environment");
    assert_eq!(env.github_issues_token, None);
}

#[test]
fn keeps_the_token_out_of_the_debug_print() {
    let env =
        load_env(&env_with(&[("GITHUB_ISSUES_TOKEN", "github_pat_secret")])).expect("a valid environment");
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
    assert!(SERVER_ONLY_ENV_VARS.contains(&"GITHUB_ISSUES_TOKEN"));
    assert!(SERVER_ONLY_ENV_VARS.contains(&"GITHUB_ISSUES_REPO"));
}
