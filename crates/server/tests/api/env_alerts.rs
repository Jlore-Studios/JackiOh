//! Environment mapping for ALERT_RESEND_API_KEY and ALERT_EMAILS (#3595, `alerts.rs`).

use indexmap::IndexMap;
use jackioh_server::alerts::Notifier;
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
fn unset_tells_nobody() {
    let env = load_env(&env_with(&[])).expect("a valid environment");
    assert_eq!(env.alert_email_key, None);
    assert!(env.alert_emails.is_empty());
    assert!(Notifier::from_env(&env).is_none());
}

#[test]
fn takes_the_addresses_comma_separated_and_leaves_out_what_is_not_one() {
    let env = load_env(&env_with(&[(
        "ALERT_EMAILS",
        " one@example.com,two@example.org , not-an-address, a b@example.com, @example.com, ,three@mail.example.com",
    )]))
    .expect("a malformed address never refuses the boot");
    assert_eq!(
        env.alert_emails,
        ["one@example.com", "two@example.org", "three@mail.example.com"]
    );
}

#[test]
fn keeps_the_key_and_the_addresses_out_of_the_debug_print() {
    let env = load_env(&env_with(&[
        ("ALERT_RESEND_API_KEY", "re_secret_key"),
        ("ALERT_EMAILS", "someone.private@example.com"),
    ]))
    .expect("a valid environment");
    assert_eq!(env.alert_email_key.as_deref(), Some("re_secret_key"));
    let printed = format!("{env:?}");
    assert!(!printed.contains("re_secret_key"), "{printed}");
    assert!(!printed.contains("someone.private"), "{printed}");
}

#[test]
fn either_way_alone_turns_alerts_on_and_says_which_without_the_addresses() {
    let email = load_env(&env_with(&[
        ("ALERT_RESEND_API_KEY", "re_key"),
        ("ALERT_EMAILS", "one@example.com,two@example.com"),
    ]))
    .expect("a valid environment");
    let ready = Notifier::from_env(&email).expect("email alone is enough").ready();
    assert_eq!(ready["issues"], false);
    assert_eq!(ready["emails"], 2);
    assert_eq!(ready["site"], "play.jackioh.test");
    assert!(!ready.to_string().contains("example.com"), "{ready}");

    let issues =
        load_env(&env_with(&[("CREATE_ISSUES_TOKEN", "github_pat_x")])).expect("a valid environment");
    let ready = Notifier::from_env(&issues)
        .expect("the token alone is enough")
        .ready();
    assert_eq!(ready["issues"], true);
    assert_eq!(ready["emails"], 0);
}

#[test]
fn a_key_without_addresses_or_addresses_without_a_key_email_nobody() {
    for half in [
        [("ALERT_RESEND_API_KEY", "re_key")],
        [("ALERT_EMAILS", "one@example.com")],
    ] {
        let env = load_env(&env_with(&half)).expect("half set never refuses the boot");
        assert!(Notifier::from_env(&env).is_none(), "{half:?}");
    }
}

#[test]
fn both_are_server_only() {
    assert!(SERVER_ONLY_ENV_VARS.contains(&"ALERT_RESEND_API_KEY"));
    assert!(SERVER_ONLY_ENV_VARS.contains(&"ALERT_EMAILS"));
}
