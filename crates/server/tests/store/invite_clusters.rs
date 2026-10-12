//! `cli/invite_clusters.rs` (invite-clusters), SPEC §9.8's watch for clusters of accounts sharing an
//! IP hash, against the fake store: what the options read, that one hash behind six accounts is
//! listed and six hashes behind one account each are not, that the window is respected, and that
//! the printed report holds counts and a hash's prefix and nothing else. The store query's own
//! agreement between the fake and Postgres is `tests/store/contract.rs`'s.

use serde_json::json;

use jackioh_server::app::now_ms;
use jackioh_server::cli::invite_clusters::{
    ClusterRow, InviteClustersOptions, invite_clusters_report, parse_invite_clusters_args,
    render_invite_clusters,
};
use jackioh_server::config::{
    INVITE_CLUSTER_HASH_PREFIX_LEN, INVITE_CLUSTER_MIN_ACCOUNTS, INVITE_CLUSTER_WINDOW_SECONDS,
};
use jackioh_server::db::store::{CodeAttempt, Db};

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|arg| (*arg).to_owned()).collect()
}

mod parse_invite_clusters_args_cases {
    use super::*;

    #[test]
    fn defaults_to_the_configs_window_and_threshold() {
        let options = parse_invite_clusters_args(&args(&[])).expect("no arguments parse");
        assert_eq!(options.window_seconds, INVITE_CLUSTER_WINDOW_SECONDS);
        assert_eq!(options.min_accounts, INVITE_CLUSTER_MIN_ACCOUNTS);
    }

    #[test]
    fn reads_both_flags() {
        let options = parse_invite_clusters_args(&args(&["--window=7200", "--min-accounts=3"]))
            .expect("both flags parse");
        assert_eq!(
            options,
            InviteClustersOptions {
                window_seconds: 7200,
                min_accounts: 3
            }
        );
    }

    #[test]
    fn refuses_a_window_or_a_threshold_that_is_not_a_whole_number_in_range() {
        for bad in [
            "--window=0",
            "--window=-5",
            "--window=x",
            "--window=1.5",
            "--window=99999999999",
            "--min-accounts=0",
            "--min-accounts=many",
        ] {
            let error = parse_invite_clusters_args(&args(&[bad]))
                .expect_err(bad)
                .to_string();
            assert!(error.contains("must be a whole number"), "{bad}: {error}");
            assert!(
                error.contains("Usage: jackioh-server invite-clusters"),
                "{bad}: {error}"
            );
        }
    }

    #[test]
    fn refuses_an_option_it_does_not_know_and_an_argument_that_is_not_a_flag() {
        let error = parse_invite_clusters_args(&args(&["--bogus=1"]))
            .expect_err("unknown")
            .to_string();
        assert!(error.contains("Unrecognised option --bogus"), "{error}");
        let error = parse_invite_clusters_args(&args(&["window"]))
            .expect_err("bare")
            .to_string();
        assert!(error.contains("Unrecognised argument \"window\""), "{error}");
        let error = parse_invite_clusters_args(&args(&["--window="]))
            .expect_err("empty")
            .to_string();
        assert!(error.contains("Unrecognised argument"), "{error}");
    }
}

fn attempt(profile: Option<&str>, ip_hash: &str, at: i64) -> CodeAttempt {
    serde_json::from_value(json!({
        "profileId": profile,
        "ipHash": ip_hash,
        "result": "rejected",
        "reason": "missing",
        "at": at,
    }))
    .expect("an attempt")
}

async fn log_all(db: &Db, attempts: &[CodeAttempt]) {
    let mut tx = db.begin(None).await.expect("a transaction");
    for attempt in attempts {
        tx.codes_log_attempt(attempt).await.expect("logged");
    }
    tx.commit().await.expect("committed");
}

fn hash(n: usize) -> String {
    format!("{n:02}{}", "ab".repeat(31))
}

mod invite_clusters_report_cases {
    use super::*;

    fn defaults() -> InviteClustersOptions {
        parse_invite_clusters_args(&args(&[])).expect("defaults")
    }

    #[tokio::test]
    async fn lists_one_hash_behind_six_accounts() {
        let db = Db::fake();
        let at = now_ms();
        let farm = hash(1);
        let attempts: Vec<CodeAttempt> = (0..6)
            .map(|n| attempt(Some(&format!("profile-{n}")), &farm, at))
            .collect();
        log_all(&db, &attempts).await;
        let rows = invite_clusters_report(&db, &defaults())
            .await
            .expect("the report");
        assert_eq!(
            rows,
            vec![ClusterRow {
                ip_hash_prefix: farm.chars().take(INVITE_CLUSTER_HASH_PREFIX_LEN).collect(),
                accounts: 6,
                attempts: 6,
            }]
        );
    }

    #[tokio::test]
    async fn is_silent_for_six_hashes_behind_one_account_each() {
        let db = Db::fake();
        let at = now_ms();
        let attempts: Vec<CodeAttempt> = (0..6)
            .map(|n| attempt(Some(&format!("profile-{n}")), &hash(n), at))
            .collect();
        log_all(&db, &attempts).await;
        let rows = invite_clusters_report(&db, &defaults())
            .await
            .expect("the report");
        assert_eq!(rows, Vec::new());
    }

    #[tokio::test]
    async fn one_account_trying_many_times_is_not_a_cluster() {
        let db = Db::fake();
        let at = now_ms();
        let attempts: Vec<CodeAttempt> = (0..20)
            .map(|_| attempt(Some("profile-0"), &hash(1), at))
            .collect();
        log_all(&db, &attempts).await;
        assert_eq!(
            invite_clusters_report(&db, &defaults())
                .await
                .expect("the report"),
            Vec::new()
        );
    }

    #[tokio::test]
    async fn respects_the_window_and_the_threshold() {
        let db = Db::fake();
        let at = now_ms();
        // Six accounts, three of them two hours ago: inside a day's window, outside an hour's.
        let attempts: Vec<CodeAttempt> = (0..6)
            .map(|n| {
                attempt(
                    Some(&format!("profile-{n}")),
                    &hash(1),
                    if n < 3 { at - 7_200_000 } else { at },
                )
            })
            .collect();
        log_all(&db, &attempts).await;

        let day = InviteClustersOptions {
            window_seconds: 86_400,
            min_accounts: 6,
        };
        assert_eq!(invite_clusters_report(&db, &day).await.expect("a day").len(), 1);
        let hour = InviteClustersOptions {
            window_seconds: 3600,
            min_accounts: 6,
        };
        assert_eq!(
            invite_clusters_report(&db, &hour).await.expect("an hour"),
            Vec::new()
        );
        let hour_of_three = InviteClustersOptions {
            window_seconds: 3600,
            min_accounts: 3,
        };
        let rows = invite_clusters_report(&db, &hour_of_three)
            .await
            .expect("an hour of three");
        assert_eq!(rows.len(), 1);
        assert_eq!((rows[0].accounts, rows[0].attempts), (3, 3));
    }

    #[tokio::test]
    async fn lists_the_biggest_cluster_first() {
        let db = Db::fake();
        let at = now_ms();
        let mut attempts: Vec<CodeAttempt> = (0..7)
            .map(|n| attempt(Some(&format!("big-{n}")), &hash(9), at))
            .collect();
        attempts.extend((0..6).map(|n| attempt(Some(&format!("small-{n}")), &hash(1), at)));
        log_all(&db, &attempts).await;
        let rows = invite_clusters_report(&db, &defaults())
            .await
            .expect("the report");
        assert_eq!(
            rows.iter().map(|row| row.accounts).collect::<Vec<_>>(),
            vec![7, 6]
        );
    }
}

mod render_invite_clusters_cases {
    use super::*;

    /// Nothing but counts and a hash's prefix reaches the page: the full hash, the profile ids and
    /// the emails the accounts signed up with are in no `ClusterRow`, and the report is built from
    /// `ClusterRow`s alone.
    #[tokio::test]
    async fn prints_a_hashs_prefix_and_counts_and_nothing_else() {
        let db = Db::fake();
        let at = now_ms();
        let farm = hash(1);
        let attempts: Vec<CodeAttempt> = (0..6)
            .map(|n| {
                attempt(
                    Some(&format!("11111111-0000-0000-0000-00000000000{n}")),
                    &farm,
                    at,
                )
            })
            .collect();
        log_all(&db, &attempts).await;
        let options = parse_invite_clusters_args(&args(&[])).expect("defaults");
        let rows = invite_clusters_report(&db, &options).await.expect("the report");
        let text = render_invite_clusters(&rows, &options);

        let prefix: String = farm.chars().take(INVITE_CLUSTER_HASH_PREFIX_LEN).collect();
        assert!(text.contains(&format!("| `{prefix}` | 6 | 6 |")), "{text}");
        assert!(!text.contains(&farm), "the full hash is printed: {text}");
        assert!(
            !text.contains(&farm[..INVITE_CLUSTER_HASH_PREFIX_LEN + 1]),
            "more than the prefix is printed: {text}"
        );
        assert!(!text.contains("11111111-0000"), "a profile id is printed: {text}");
        assert!(!text.contains('@'), "an email is printed: {text}");
        assert!(text.contains("Advisory only"), "{text}");
    }

    #[test]
    fn says_so_when_nothing_is_found() {
        let options = InviteClustersOptions {
            window_seconds: 3600,
            min_accounts: 6,
        };
        let text = render_invite_clusters(&[], &options);
        assert!(
            text.contains("No invite clusters at or above 6 accounts in the last 3600 seconds."),
            "{text}"
        );
        assert!(!text.contains('|'), "{text}");
    }
}
