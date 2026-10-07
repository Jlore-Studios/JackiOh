//! `cli/seed_accounts.rs`'s gate. The accounts it makes skip the invite gate, so it must refuse
//! unless the caller names the one project it is meant for, and its password must come from the
//! environment: a password written in this public repo let anyone sign in to the seeded accounts.
//! (← `apps/server/test/db/seed-accounts.test.ts`)

use indexmap::IndexMap;

use jackioh_server::cli::seed_accounts::{seed_accounts_settings, SEED_PASSWORD_VAR, SEED_PROJECT_VAR};
use jackioh_server::config::AUTH_PASSWORD_MIN_LENGTH;

/// TS's `ENV`: the two server variables the gate reads (`Pick<ServerEnv, "SUPABASE_URL" | "NODE_ENV">`).
const SUPABASE_URL: &str = "https://dev-project.supabase.co";
const NODE_ENV: &str = "development";

fn password() -> String {
    "p".repeat(AUTH_PASSWORD_MIN_LENGTH as usize)
}

fn source(pairs: &[(&str, &str)]) -> IndexMap<String, String> {
    pairs.iter().map(|(key, value)| ((*key).to_owned(), (*value).to_owned())).collect()
}

fn refusal(source: &IndexMap<String, String>, node_env: &str) -> String {
    seed_accounts_settings(source, SUPABASE_URL, node_env).err().expect("the gate refuses").to_string()
}

#[test]
fn reads_the_password_from_the_environment_once_the_named_project_matches() {
    let password = password();
    let source = source(&[(SEED_PROJECT_VAR, "dev-project.supabase.co"), (SEED_PASSWORD_VAR, password.as_str())]);
    let settings = seed_accounts_settings(&source, SUPABASE_URL, NODE_ENV).ok().expect("the gate opens");
    assert_eq!(settings.password, password);
}

#[test]
fn refuses_without_the_opt_in_and_names_the_host_it_expects() {
    let password = password();
    let message = refusal(&source(&[(SEED_PASSWORD_VAR, password.as_str())]), NODE_ENV);
    assert!(message.contains("SEED_ACCOUNTS_PROJECT must equal SUPABASE_URL's host (dev-project.supabase.co)"));
}

#[test]
fn refuses_when_the_opt_in_names_a_different_project_than_supabase_url() {
    let password = password();
    let source = source(&[(SEED_PROJECT_VAR, "prod-project.supabase.co"), (SEED_PASSWORD_VAR, password.as_str())]);
    assert!(refusal(&source, NODE_ENV).contains("it is \"prod-project.supabase.co\""));
}

#[test]
fn refuses_a_missing_or_short_password() {
    let missing = refusal(&source(&[(SEED_PROJECT_VAR, "dev-project.supabase.co")]), NODE_ENV);
    // TS: /SEED_ACCOUNTS_PASSWORD .*it is not set/
    assert!(missing
        .lines()
        .any(|line| line.find("SEED_ACCOUNTS_PASSWORD ").is_some_and(|at| line[at..].contains("it is not set"))));

    let short = "p".repeat(AUTH_PASSWORD_MIN_LENGTH as usize - 1);
    let source = source(&[(SEED_PROJECT_VAR, "dev-project.supabase.co"), (SEED_PASSWORD_VAR, short.as_str())]);
    assert!(refusal(&source, NODE_ENV).contains("at least 12 characters"));
}

#[test]
fn refuses_under_node_env_production_even_with_the_opt_in() {
    let password = password();
    let source = source(&[(SEED_PROJECT_VAR, "dev-project.supabase.co"), (SEED_PASSWORD_VAR, password.as_str())]);
    assert!(refusal(&source, "production").contains("NODE_ENV is production"));
}

#[test]
fn carries_no_password_literal_in_the_script() {
    let script = include_str!("../../src/cli/seed_accounts.rs");
    assert!(!script.contains("const PASSWORD = \""));
    assert!(!script.contains("const PASSWORD: &str = \""));
    assert!(!script.contains("password: \""));
}
