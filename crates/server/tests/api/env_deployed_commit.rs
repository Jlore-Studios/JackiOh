//! RENDER_GIT_COMMIT -> `Env.deployed_commit` (src/env.rs): the commit `GET /api/catalog` reports
//! so deploy-watch.yml can compare it with the push. It is optional, never a problem, and only a
//! git SHA survives, because the value ends up in a response header.
//!
//! Port of `apps/server/test/env-deployed-commit.test.ts` (part 18).

use indexmap::IndexMap;
use jackioh_server::env::{SERVER_ONLY_ENV_VARS, load_env};
use serde_json::json;

/// A complete, valid environment for the server (apps/server/README.md's table).
///
/// `CATALOG_VERSION` is the compiled-in catalog's version, where TS wrote `"core-1"`: the Rust
/// server serves that one whatever the variable says (SURFACE §11.3).
fn valid_env() -> IndexMap<String, String> {
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
    .map(|(name, value)| (name.to_string(), value.to_string()))
    .collect()
}

/// `valid_env()` with `RENDER_GIT_COMMIT` set to `value`.
fn with_commit(value: &str) -> IndexMap<String, String> {
    let mut source = valid_env();
    source.insert("RENDER_GIT_COMMIT".to_string(), value.to_string());
    source
}

mod render_git_commit {
    use super::*;

    #[test]
    fn is_unset_not_a_problem_when_the_server_is_not_on_render() {
        assert_eq!(
            load_env(&valid_env())
                .expect("a valid environment")
                .deployed_commit,
            None
        );
    }

    #[test]
    fn is_kept_lower_cased_when_it_is_a_git_sha() {
        let sha = "71DFDB6CF14B66670D88D6561EC50B5783C4F998";
        let env = load_env(&with_commit(&format!(" {sha}\n"))).expect("a valid environment");
        assert_eq!(env.deployed_commit, Some(sha.to_lowercase()));
    }

    #[test]
    fn drops_anything_that_is_not_a_git_sha_instead_of_failing_the_boot() {
        let forty_gs = "g".repeat(40);
        for value in [
            "",
            "main",
            "not a sha",
            "abc123",
            "71dfdb6\r\nx-injected: 1",
            forty_gs.as_str(),
        ] {
            let env = load_env(&with_commit(value)).expect("a valid environment");
            assert_eq!(env.deployed_commit, None, "{}", json!(value));
        }
    }

    #[test]
    fn is_listed_with_the_server_only_variables_load_env_reads() {
        assert!(SERVER_ONLY_ENV_VARS.contains(&"RENDER_GIT_COMMIT"));
    }
}
