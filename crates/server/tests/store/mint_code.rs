//! `cli/mint_code.rs`'s argument parsing, and the one property that makes the script worth having:
//! a code it mints hashes to what redemption will look up.
//!
//! The hash is the whole risk here. Nothing reports a pepper that is merely *different* from the
//! server's — the insert succeeds, the code prints, and it is simply never redeemable. So the
//! second module mints through the fake store with the same `{CODE_PEPPER}:code` derivation
//! `app.rs` and `cli/mint_code.rs` both use, then looks the printed plaintext up again.
//! (← `apps/server/test/db/mint-code.test.ts`)

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|arg| (*arg).to_owned()).collect()
}

mod parse_mint_args {
    use super::args;
    use jackioh_server::cli::mint_code::parse_mint_args;

    #[test]
    fn r161_defaults_to_one_use_and_no_expiry() {
        let options = parse_mint_args(&args(&[])).expect("no arguments parse");
        assert_eq!(options.max_uses, None);
        assert_eq!(options.expires_in_days, None);
    }

    #[test]
    fn reads_max_uses_and_expires_in_days() {
        let max_uses = parse_mint_args(&args(&["--max-uses=5"])).expect("--max-uses parses");
        assert_eq!(max_uses.max_uses, Some(5));
        assert_eq!(max_uses.expires_in_days, None);

        let expiry = parse_mint_args(&args(&["--expires-in-days=30"])).expect("--expires-in-days parses");
        assert_eq!(expiry.max_uses, None);
        assert_eq!(expiry.expires_in_days, Some(30));

        let both = parse_mint_args(&args(&["--max-uses=2", "--expires-in-days=7"])).expect("both parse");
        assert_eq!(both.max_uses, Some(2));
        assert_eq!(both.expires_in_days, Some(7));
    }

    fn refusal(argv: &[&str]) -> String {
        parse_mint_args(&args(argv)).expect_err("the arguments are refused").to_string()
    }

    #[test]
    fn refuses_an_unknown_option_rather_than_silently_minting_a_default_code() {
        assert!(refusal(&["--label=bring-up"]).contains("Unrecognised option --label"));
        assert!(refusal(&["--max-uses"]).contains("Unrecognised argument"));
        assert!(refusal(&["5"]).contains("Unrecognised argument"));
    }

    /// A zero or negative `max_uses` violates `invite_codes_max_uses_positive` (migration 0001).
    #[test]
    fn refuses_a_non_positive_or_fractional_count() {
        assert!(refusal(&["--max-uses=0"]).contains("positive integer"));
        assert!(refusal(&["--max-uses=-1"]).contains("positive integer"));
        assert!(refusal(&["--max-uses=1.5"]).contains("positive integer"));
        assert!(refusal(&["--expires-in-days=x"]).contains("positive integer"));
    }
}

mod the_pepper_derivation_the_script_shares_with_app_rs {
    use jackioh_server::api::codes::{mint_invite_code, MintDeps, MintInput};
    use jackioh_server::api::crypto::{create_hashes, Hashes};
    use jackioh_server::db::store::{Db, InviteCode};

    const CODE_PEPPER: &str = "a-pepper-of-at-least-thirty-two-characters";

    /// Exactly what `cli/mint_code.rs` and `app.rs` both build: one pepper, two domains.
    fn mint_hashes() -> Hashes {
        create_hashes(&format!("{CODE_PEPPER}:code"), &format!("{CODE_PEPPER}:ip"))
    }

    async fn find(db: &Db, code_hash: &str) -> Option<InviteCode> {
        let mut tx = db.begin(None).await.expect("a transaction");
        let found = tx.codes_find_by_hash(code_hash).await.expect("the lookup runs");
        tx.commit().await.expect("the lookup commits");
        found
    }

    #[tokio::test]
    async fn stores_a_hash_that_the_same_pepper_finds_again_by_the_printed_plaintext() {
        let db = Db::fake();
        let hashes = mint_hashes();
        let minted = mint_invite_code(MintDeps { db: &db, code_pepper: CODE_PEPPER }, MintInput { max_uses: Some(2), ..Default::default() })
            .await
            .expect("a code is minted");

        let found = find(&db, &hashes.code(&minted.formatted)).await.expect("the code is found by its hash");
        assert_eq!(found.id, minted.id);
        assert_eq!(found.max_uses, 2);
    }

    /// The failure the script's full environment load exists to prevent.
    #[tokio::test]
    async fn stores_a_hash_a_different_pepper_cannot_find() {
        let db = Db::fake();
        let minted = mint_invite_code(MintDeps { db: &db, code_pepper: CODE_PEPPER }, MintInput::default()).await.expect("a code is minted");

        let other = create_hashes("a-different-pepper-of-thirty-two-plus", "x");
        assert_eq!(find(&db, &other.code(&minted.formatted)).await, None);
    }

    /// The two domains must not collide (§9.4, §9.8).
    #[test]
    fn derives_an_ip_hash_that_differs_from_the_code_hash_for_the_same_input() {
        let hashes = mint_hashes();
        let sample = "ABCD-EFGH-JKLM-NPQR";
        assert_ne!(hashes.ip(sample), hashes.code(sample));
    }
}
