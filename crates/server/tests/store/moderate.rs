//! `cli/moderate.rs` (issue #887), the operator's abuse levers, against the fake store: what
//! each subcommand parses to, that ban/unban round-trip a profile and refuse unknown and pending
//! accounts, that revoke round-trips a minted code and refuses malformed and unknown ones, and
//! that the redemption switch round-trips with prior-state lines, and that `chat-reports` (#970)
//! lists every report with its message, oldest first, and writes nothing. The store methods' own
//! agreement between the fake and Postgres is `contract.rs`'s.

use jackioh_server::api::codes::{MintDeps, MintInput, mint_invite_code};
use jackioh_server::api::crypto::canonical_invite_code;
use jackioh_server::cli::moderate::{
    ModerateCommand, ban_profile, chat_reports, parse_moderate_args, revoke_code, set_redemption,
    unban_profile,
};
use jackioh_server::config::{CHAT_RETENTION_DAYS, RATING_START};
use jackioh_server::db::store::{ChatMessageRow, ChatReportRow, Db, ProfileCreateInput, ProfileStatus};

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|arg| (*arg).to_owned()).collect()
}

mod parse_moderate_args_cases {
    use super::*;

    #[test]
    fn routes_each_subcommand() {
        assert_eq!(
            parse_moderate_args(&args(&["ban", "profile-1"])).expect("ban parses"),
            ModerateCommand::Ban {
                profile_id: "profile-1".to_string(),
            }
        );
        assert_eq!(
            parse_moderate_args(&args(&["unban", "profile-1"])).expect("unban parses"),
            ModerateCommand::Unban {
                profile_id: "profile-1".to_string(),
            }
        );
        assert_eq!(
            parse_moderate_args(&args(&["revoke-code", "ABCD-EFGH-JKLM-NPQR"])).expect("revoke parses"),
            ModerateCommand::RevokeCode {
                code: "ABCD-EFGH-JKLM-NPQR".to_string(),
            }
        );
        assert_eq!(
            parse_moderate_args(&args(&["redemption", "on"])).expect("redemption on parses"),
            ModerateCommand::Redemption { enabled: true }
        );
        assert_eq!(
            parse_moderate_args(&args(&["redemption", "off"])).expect("redemption off parses"),
            ModerateCommand::Redemption { enabled: false }
        );
        assert_eq!(
            parse_moderate_args(&args(&["chat-reports"])).expect("chat-reports parses"),
            ModerateCommand::ChatReports
        );
    }

    #[test]
    fn refuses_unknown_verb_missing_arg_and_bad_on_off() {
        for argv in [
            vec![],
            vec!["suspend", "profile-1"],
            vec!["ban"],
            vec!["ban", "profile-1", "profile-2"],
            vec!["unban"],
            vec!["revoke-code"],
            vec!["redemption"],
            vec!["redemption", "maybe"],
            vec!["redemption", "on", "off"],
            vec!["chat-reports", "profile-1"],
        ] {
            let error = parse_moderate_args(&args(&argv))
                .expect_err(&format!("{argv:?} is refused"))
                .to_string();
            assert!(
                error.contains("Usage: jackioh-server moderate"),
                "{argv:?}: {error}"
            );
        }
    }
}

mod moderate_commands {
    use super::*;

    const PEPPER: &str = "moderate-test-pepper-of-at-least-thirty-two-characters";

    async fn create_profile(db: &Db, user_id: &str) -> String {
        let mut tx = db.begin(None).await.expect("a transaction opens");
        let profile = tx
            .profiles_create(&ProfileCreateInput {
                user_id: user_id.to_string(),
                email: format!("{user_id}@example.test"),
                rating: RATING_START,
                at: 0,
            })
            .await
            .expect("the profile is created");
        tx.commit().await.expect("the transaction commits");
        profile.id
    }

    async fn status_of(db: &Db, profile_id: &str) -> ProfileStatus {
        let mut tx = db.begin(None).await.expect("a transaction opens");
        let profile = tx
            .profiles_get_by_id(profile_id)
            .await
            .expect("the lookup runs")
            .expect("the profile");
        tx.commit().await.expect("the transaction commits");
        profile.status
    }

    #[tokio::test]
    async fn ban_is_idempotent_and_refuses_an_unknown_profile() {
        let db = Db::fake();
        let id = create_profile(&db, "user-ban").await;

        assert_eq!(
            ban_profile(&db, &id).await.expect("the ban lands"),
            format!("banned {id}")
        );
        assert_eq!(status_of(&db, &id).await, ProfileStatus::Banned);
        assert_eq!(
            ban_profile(&db, &id).await.expect("the second ban lands"),
            format!("already banned {id}")
        );

        let error = ban_profile(&db, "profile-999")
            .await
            .expect_err("an unknown profile is refused")
            .to_string();
        assert!(error.contains("no such profile"), "{error}");
    }

    #[tokio::test]
    async fn unban_restores_a_banned_account_and_refuses_a_pending_one() {
        let db = Db::fake();
        let id = create_profile(&db, "user-unban").await;

        let error = unban_profile(&db, &id)
            .await
            .expect_err("a pending account is refused")
            .to_string();
        assert!(error.contains("pending"), "{error}");
        assert_eq!(status_of(&db, &id).await, ProfileStatus::Pending);

        assert_eq!(
            ban_profile(&db, &id).await.expect("the ban lands"),
            format!("banned {id}")
        );
        assert_eq!(
            unban_profile(&db, &id).await.expect("the unban lands"),
            format!("unbanned {id}")
        );
        assert_eq!(status_of(&db, &id).await, ProfileStatus::Active);
        assert_eq!(
            unban_profile(&db, &id).await.expect("the second unban lands"),
            format!("already active {id}")
        );

        let error = unban_profile(&db, "profile-999")
            .await
            .expect_err("an unknown profile is refused")
            .to_string();
        assert!(error.contains("no such profile"), "{error}");
    }

    #[tokio::test]
    async fn revoke_round_trips_a_minted_code_and_is_idempotent() {
        let db = Db::fake();
        let minted = mint_invite_code(
            MintDeps {
                db: &db,
                code_pepper: PEPPER,
            },
            MintInput::default(),
        )
        .await
        .expect("a code is minted");
        let canonical = canonical_invite_code(&minted.formatted).expect("a minted code is canonical");

        assert_eq!(
            revoke_code(&db, PEPPER, &minted.formatted)
                .await
                .expect("the revoke lands"),
            format!("revoked {canonical}")
        );
        let mut tx = db.begin(None).await.expect("a transaction opens");
        assert!(
            tx.codes_find_by_hash(
                &jackioh_server::api::crypto::hashes_for_pepper(PEPPER).code(&minted.formatted)
            )
            .await
            .expect("the lookup runs")
            .expect("the code")
            .revoked
        );
        tx.commit().await.expect("the transaction commits");
        assert_eq!(
            revoke_code(&db, PEPPER, &minted.formatted)
                .await
                .expect("the second revoke lands"),
            format!("already revoked {canonical}")
        );
    }

    #[tokio::test]
    async fn revoke_refuses_malformed_and_unknown_codes() {
        let db = Db::fake();
        let error = revoke_code(&db, PEPPER, "not-a-code")
            .await
            .expect_err("a malformed code is refused")
            .to_string();
        assert!(error.contains("not an invite code"), "{error}");

        let error = revoke_code(&db, PEPPER, "ABCD-EFGH-JKLM-NPQR")
            .await
            .expect_err("an unminted code is refused")
            .to_string();
        assert!(error.contains("unknown code"), "{error}");
    }

    #[tokio::test]
    async fn redemption_round_trips_with_prior_state_lines() {
        let db = Db::fake();
        assert_eq!(
            set_redemption(&db, false).await.expect("the switch flips"),
            "redemption paused"
        );
        assert_eq!(
            set_redemption(&db, false).await.expect("the switch flips"),
            "redemption already paused"
        );
        assert_eq!(
            set_redemption(&db, true).await.expect("the switch flips"),
            "redemption resumed"
        );
        assert_eq!(
            set_redemption(&db, true).await.expect("the switch flips"),
            "redemption already resumed"
        );
    }

    #[tokio::test]
    async fn chat_reports_lists_each_report_with_its_message_oldest_first_and_writes_nothing() {
        let db = Db::fake();
        assert_eq!(
            chat_reports(&db).await.expect("the read runs"),
            format!("no chat reports (each is kept until its message is {CHAT_RETENTION_DAYS} days old)")
        );

        let ada = create_profile(&db, "user-ada").await;
        let bea = create_profile(&db, "user-bea").await;
        let mut tx = db.begin(None).await.expect("a transaction opens");
        for (id, from, to, body, at) in [
            ("message-1", &ada, &bea, "you ***\nloser", 1_000),
            ("message-2", &bea, &ada, "buy gold", 2_000),
        ] {
            tx.chat_insert(&ChatMessageRow {
                id: id.to_string(),
                from_profile: from.clone(),
                to_profile: to.clone(),
                body: body.to_string(),
                created_at: at,
                read_at: None,
            })
            .await
            .expect("the message is stored");
        }
        // Filed in the other order: the list is by when each report was made.
        for (id, message, reporter, reason, at) in [
            ("report-2", "message-2", &ada, "spam", 4_000),
            ("report-1", "message-1", &bea, "harassment", 3_000),
        ] {
            assert!(
                tx.chat_report(&ChatReportRow {
                    id: id.to_string(),
                    message_id: message.to_string(),
                    reporter: reporter.clone(),
                    reason: reason.to_string(),
                    created_at: at,
                })
                .await
                .expect("the report is stored")
            );
        }
        tx.commit().await.expect("the transaction commits");

        let printed = chat_reports(&db).await.expect("the read runs");
        assert_eq!(
            printed,
            format!(
                "2 chat report(s), oldest first (each kept until its message is {CHAT_RETENTION_DAYS} days old):\n\n\
                 1970-01-01T00:00:03.000Z  report report-1  reason \"harassment\"\n  reporter {bea}\n  \
                 sender   {ada}  (ban with: moderate ban {ada})\n  message  message-1 sent 1970-01-01T00:00:01.000Z: \
                 \"you ***\\nloser\"\n\n\
                 1970-01-01T00:00:04.000Z  report report-2  reason \"spam\"\n  reporter {ada}\n  \
                 sender   {bea}  (ban with: moderate ban {bea})\n  message  message-2 sent 1970-01-01T00:00:02.000Z: \
                 \"buy gold\""
            )
        );
        assert_eq!(
            status_of(&db, &ada).await,
            ProfileStatus::Pending,
            "nothing is banned"
        );
        assert_eq!(
            chat_reports(&db).await.expect("a second read"),
            printed,
            "nothing is written"
        );
    }
}
