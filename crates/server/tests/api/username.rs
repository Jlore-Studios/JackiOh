//! Usernames (SPEC §9.4, R1432–R1435): the form a name must have, the light filter, and the
//! routes a player picks one through (`src/username/`, `src/api/username.rs`).
//!
//! The form and the filter are pure functions and are asserted on them directly. The routes go
//! through the whole app (`support::deps::test_app`) as a client calls them, each caller a fresh
//! active account seeded in the fake store with the default a new account gets. Uniqueness, tags
//! and concurrency are the store's and are asserted against both stores in
//! `tests/store/contract.rs` (`r1434_usernames`); the database's half is
//! `tests/sql/15_usernames.sql`.

use std::sync::Arc;

use jackioh_server::app::{App, now_ms};
use jackioh_server::config::{API_REQUESTS_PER_MINUTE, USERNAME_CHANGE_COOLDOWN_MS, USERNAME_MAX_LENGTH};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::Db;
use jackioh_server::username::{UsernameRefusal, normalize_username};
use serde_json::{Value, json};

use crate::support::deps::{add_user, call, test_app};

const PREVIEW: &str = "/api/username/preview";
const SAVE: &str = "/api/username";
const SKIP: &str = "/api/username/skip";
const ME: &str = "/api/auth/me";

async fn fake(app: &App) -> tokio::sync::MutexGuard<'_, FakeData> {
    match &app.db {
        Db::Fake(data) => data.lock().await,
        Db::Pg(_) => panic!("the API tests run on the fake store"),
    }
}

/// A fresh account of `status`, with the default username a new account gets and the prompt not
/// yet answered (R1434); answers its bearer token.
async fn account(app: &App, id: &str, status: &str) -> String {
    let user_id = format!("user-{id}");
    let token = add_user(app, &user_id, &format!("{id}@example.test"), true);
    fake(app)
        .await
        .seed_profile(json!({ "id": id, "userId": user_id, "status": status }));
    token
}

/// `GET /api/username/preview?name=…`, the name percent-encoded as a browser's `URLSearchParams`
/// sends it.
async fn preview(app: &Arc<App>, token: &str, name: &str) -> (u16, Value) {
    let encoded: String = name
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' | b'.' | b'~' => (byte as char).to_string(),
            _ => format!("%{byte:02X}"),
        })
        .collect();
    let (status, _headers, body) = call(
        app,
        "GET",
        &format!("{PREVIEW}?name={encoded}"),
        Some(token),
        Value::Null,
    )
    .await;
    (status, body)
}

async fn save(app: &Arc<App>, token: &str, username: &str) -> (u16, Value) {
    let (status, _headers, body) = call(app, "PUT", SAVE, Some(token), json!({ "username": username })).await;
    (status, body)
}

async fn me(app: &Arc<App>, token: &str) -> Value {
    let (status, _headers, body) = call(app, "GET", ME, Some(token), Value::Null).await;
    assert_eq!(status, 200, "{body}");
    body["username"].clone()
}

fn refusal_of(raw: &str) -> Option<UsernameRefusal> {
    normalize_username(raw).err()
}

// ---------------------------------------------------------------------------
// R1432: the form
// ---------------------------------------------------------------------------

mod the_form_r1432 {
    use super::*;

    #[test]
    fn r1432_accepts_every_listed_script_with_digits_and_underscores() {
        for name in [
            "Zoë",
            "Ångström",
            "Дмитрий",
            "Мах",
            "Νίκος",
            "محمد",
            "محمد٣",
            "דוד",
            "देवनागरी",
            "สมศักดิ์",
            "李明",
            "ヤマダ太郎",
            "やまだ太郎",
            "김민수",
            "김太",
            "Max_99",
            "Дмитрий_7",
            "Player",
            "admin",
        ] {
            assert_eq!(normalize_username(name), Ok(name.to_string()), "{name}");
        }
    }

    #[test]
    fn r1432_refuses_emoji_anything_outside_the_bmp_spaces_punctuation_and_symbols() {
        for name in [
            "Max😀",
            "𠀀𠀀",
            "𝐌𝐚𝐱",
            "Max Power",
            "Max!",
            "Max-1",
            "Max#1",
            "Max$",
            "Max♥",
            "M@x",
        ] {
            assert_eq!(refusal_of(name), Some(UsernameRefusal::Characters), "{name}");
        }
    }

    #[test]
    fn r1432_refuses_zero_width_and_direction_control_characters() {
        for name in [
            "Ma\u{200B}x",
            "Ma\u{200D}x",
            "Ma\u{202E}x",
            "Ma\u{2066}x",
            "Ma\u{FEFF}x",
        ] {
            assert_eq!(refusal_of(name), Some(UsernameRefusal::Characters), "{name:?}");
        }
    }

    #[test]
    fn r1432_refuses_invisible_characters_whose_category_is_a_letter_or_a_mark() {
        // Default_Ignorable_Code_Point: the combining grapheme joiner and the variation selectors are
        // `Mn`, the Hangul fillers `Lo` (the halfwidth one becomes U+1160 under NFKC). Each would
        // render as `Max`, or as nothing, beside a real name.
        for name in [
            "Max\u{034F}",
            "Max\u{FE0F}",
            "Max\u{FE00}",
            "Ma\u{180B}x",
            "\u{115F}\u{1160}\u{115F}\u{1160}",
            "\u{3164}\u{3164}",
            "\u{FFA0}\u{FFA0}",
            "민\u{3164}수",
        ] {
            assert_eq!(refusal_of(name), Some(UsernameRefusal::Characters), "{name:?}");
        }
    }

    #[test]
    fn r1432_allows_three_combining_marks_after_a_letter_and_refuses_a_fourth() {
        // `q` has no precomposed form with these, so NFKC leaves every mark in place.
        assert_eq!(
            normalize_username("Mq\u{301}\u{302}\u{303}x"),
            Ok("Mq\u{301}\u{302}\u{303}x".to_string())
        );
        assert_eq!(
            refusal_of("Mq\u{301}\u{302}\u{303}\u{304}x"),
            Some(UsernameRefusal::Characters)
        );
        // A mark that follows no letter.
        assert_eq!(refusal_of("\u{301}Max"), Some(UsernameRefusal::Characters));
        assert_eq!(refusal_of("M1\u{301}x"), Some(UsernameRefusal::Characters));
        assert_eq!(refusal_of("Max_\u{301}"), Some(UsernameRefusal::Characters));
    }

    #[test]
    fn r1432_refuses_mixed_scripts_but_not_the_mixes_a_language_needs() {
        // `Mаx` with a Cyrillic `а`.
        assert_eq!(refusal_of("M\u{0430}x"), Some(UsernameRefusal::MixedScripts));
        assert_eq!(refusal_of("Max太郎"), Some(UsernameRefusal::MixedScripts));
        assert_eq!(refusal_of("Νίκοςx"), Some(UsernameRefusal::MixedScripts));
        assert_eq!(refusal_of("김太ヤ"), Some(UsernameRefusal::MixedScripts));
        assert_eq!(refusal_of("ヤ김"), Some(UsernameRefusal::MixedScripts));
        assert!(normalize_username("ヤマダ太郎").is_ok());
        assert!(normalize_username("김太").is_ok());
    }

    #[test]
    fn r1432_counts_length_in_grapheme_clusters_from_2_to_16() {
        assert_eq!(refusal_of("M"), Some(UsernameRefusal::TooShort));
        assert_eq!(refusal_of("李"), Some(UsernameRefusal::TooShort));
        assert_eq!(refusal_of(""), Some(UsernameRefusal::TooShort));
        assert_eq!(refusal_of("   "), Some(UsernameRefusal::TooShort));
        let longest = "a".repeat(USERNAME_MAX_LENGTH);
        assert_eq!(normalize_username(&longest), Ok(longest.clone()));
        assert_eq!(refusal_of(&format!("{longest}b")), Some(UsernameRefusal::TooLong));
        // A letter with its marks counts once: sixteen of them is sixteen characters.
        let marked = "q\u{301}\u{302}".repeat(USERNAME_MAX_LENGTH);
        assert!(normalize_username(&marked).is_ok());
        assert_eq!(refusal_of(&format!("{marked}q")), Some(UsernameRefusal::TooLong));
        // Sixteen wide characters fit.
        assert!(normalize_username(&"李".repeat(USERNAME_MAX_LENGTH)).is_ok());
        // Input far past any name is refused unread.
        assert_eq!(refusal_of(&"a".repeat(1000)), Some(UsernameRefusal::TooLong));
    }

    #[test]
    fn r1432_stores_fullwidth_input_in_its_nfkc_form_and_trims_it() {
        assert_eq!(normalize_username("ＭＡＸ"), Ok("MAX".to_string()));
        assert_eq!(normalize_username("  Max  "), Ok("Max".to_string()));
        // NFKC composes a letter and its mark.
        assert_eq!(normalize_username("Zoe\u{308}"), Ok("Zoë".to_string()));
    }

    #[test]
    fn r1432_each_refusal_is_one_sentence_that_names_its_kind() {
        for refusal in [
            UsernameRefusal::Characters,
            UsernameRefusal::MixedScripts,
            UsernameRefusal::TooShort,
            UsernameRefusal::TooLong,
            UsernameRefusal::NotAllowed,
        ] {
            assert!(!refusal.code().is_empty());
            assert!(refusal.message().ends_with('.'), "{}", refusal.message());
        }
        assert!(
            UsernameRefusal::TooLong
                .message()
                .contains(&USERNAME_MAX_LENGTH.to_string())
        );
    }
}

// ---------------------------------------------------------------------------
// R1433: the light filter
// ---------------------------------------------------------------------------

mod the_filter_r1433 {
    use super::*;

    #[test]
    fn r1433_refuses_blocklisted_words_and_their_fullwidth_accented_digit_and_camel_case_forms() {
        for name in [
            "fuck",
            "xXfuckXx",
            "ＦＵＣＫ",
            "fück",
            "f_u_c_k",
            "b1tch",
            "B1TCH_99",
            "BigAss",
            "my_ass",
            "Ass99",
            "ASS",
            "Shit",
            "LordShit",
            "5h1t",
            "sh1t",
            "a55",
            "my_a55",
            "BigA55",
            "d1ck",
            "C0CK_7",
            "5H1T",
            "a55Max",
        ] {
            assert_eq!(refusal_of(name), Some(UsernameRefusal::NotAllowed), "{name}");
        }
    }

    #[test]
    fn r1433_lets_innocent_names_that_hold_a_token_word_through() {
        for name in [
            "Scunthorpe",
            "Cassandra",
            "Assassin",
            "Yoshitaka",
            "Hancock",
            "Dickens",
            "Bob455",
            "Bob_455",
            "Tit4n",
            "H4ck3r",
            "Grape",
            "Player",
            "admin",
        ] {
            assert_eq!(normalize_username(name), Ok(name.to_string()), "{name}");
        }
    }

    #[tokio::test]
    async fn r1433_a_refusal_never_echoes_the_word_it_matched() {
        let app = test_app().await;
        let token = account(&app, "filter", "active").await;
        for name in ["BigAss", "fück", "b1tch"] {
            let (status, body) = preview(&app, &token, name).await;
            assert_eq!(status, 200);
            assert_eq!(
                body,
                json!({ "ok": false, "reason": "not_allowed", "message": "That name isn't allowed." }),
                "{name}"
            );
            let text = body.to_string().to_lowercase();
            for word in ["ass", "fuck", "bitch", "fück", "b1tch"] {
                assert!(!text.contains(word), "{name}: {text}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// R1435: the routes
// ---------------------------------------------------------------------------

mod the_routes_r1435 {
    use super::*;

    #[tokio::test]
    async fn r1435_a_new_active_account_owes_the_prompt_and_may_change_at_once() {
        let app = test_app().await;
        let token = account(&app, "fresh", "active").await;
        let own = me(&app, &token).await;
        assert!(
            own["name"]
                .as_str()
                .is_some_and(|name| name.starts_with("Player#")),
            "{own}"
        );
        assert_eq!(own["nextChangeAt"], Value::Null);
        assert_eq!(own["promptOwed"], true);

        // A pending account owes no prompt (it sees the code screen) and may not use the routes.
        let pending = account(&app, "waiting", "pending").await;
        assert_eq!(me(&app, &pending).await["promptOwed"], false);
        assert_eq!(preview(&app, &pending, "Max").await.0, 403);
        assert_eq!(save(&app, &pending, "Max").await.0, 403);
        assert_eq!(call(&app, "POST", SKIP, Some(&pending), Value::Null).await.0, 403);
        assert_eq!(preview(&app, "no-such-token", "Max").await.0, 401);
    }

    #[tokio::test]
    async fn r1435_the_preview_names_the_exact_username_a_save_gives_tag_included() {
        let app = test_app().await;
        let first = account(&app, "first", "active").await;
        let second = account(&app, "second", "active").await;

        assert_eq!(
            preview(&app, &first, "Max").await,
            (
                200,
                json!({ "ok": true, "base": "Max", "username": "Max", "tagged": false })
            )
        );
        let (status, body) = save(&app, &first, "Max").await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["username"]["name"], "Max");

        // `max` clashes with `Max` (R1434), so the second player is shown the tag they would get.
        assert_eq!(
            preview(&app, &second, "max").await,
            (
                200,
                json!({ "ok": true, "base": "max", "username": "max#1", "tagged": true })
            )
        );
        // Fullwidth input is previewed in the form it is stored in.
        assert_eq!(
            preview(&app, &second, "ＭＡＸ").await,
            (
                200,
                json!({ "ok": true, "base": "MAX", "username": "MAX#1", "tagged": true })
            )
        );
        assert_eq!(save(&app, &second, "MAX#1").await.0, 200);
        assert_eq!(me(&app, &second).await["name"], "MAX#1");

        // `Player` is a base like any other: free bare, so a player who picks it gets it bare.
        let third = account(&app, "third", "active").await;
        assert_eq!(
            preview(&app, &third, "Player").await,
            (
                200,
                json!({ "ok": true, "base": "Player", "username": "Player", "tagged": false })
            )
        );

        // A refused name says why.
        let (status, body) = preview(&app, &third, "Mа x").await;
        assert_eq!(status, 200);
        assert_eq!(body["ok"], false);
        assert_eq!(body["reason"], "characters");
        assert_eq!(
            preview(&app, &third, "M\u{0430}x").await.1["reason"],
            "mixed_scripts"
        );
        assert_eq!(preview(&app, &third, "M").await.1["reason"], "too_short");
        assert_eq!(preview(&app, &third, "").await.1["reason"], "too_short");
        assert_eq!(
            preview(&app, &third, &"a".repeat(17)).await.1["reason"],
            "too_long"
        );
    }

    #[tokio::test]
    async fn r1435_a_save_whose_outcome_changed_gets_409_and_a_fresh_preview() {
        let app = test_app().await;
        let slow = account(&app, "slow", "active").await;
        let quick = account(&app, "quick", "active").await;
        let before = me(&app, &slow).await["name"].clone();

        // Both see `Lena` bare; the quick one saves first.
        assert_eq!(preview(&app, &slow, "Lena").await.1["username"], "Lena");
        assert_eq!(save(&app, &quick, "Lena").await.0, 200);

        let (status, body) = save(&app, &slow, "Lena").await;
        assert_eq!(status, 409, "{body}");
        assert_eq!(body["error"]["code"], "conflict");
        assert_eq!(
            body["error"]["details"],
            json!({ "ok": true, "base": "Lena", "username": "Lena#1", "tagged": true })
        );
        assert_eq!(me(&app, &slow).await["name"], before, "nothing was written");

        // The fresh preview's username is what the next save sends, and it lands.
        let (status, body) = save(&app, &slow, "Lena#1").await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["username"]["name"], "Lena#1");
    }

    #[tokio::test]
    async fn r1435_a_second_change_inside_the_cooldown_is_refused_with_the_time_it_is_allowed() {
        let app = test_app().await;
        let token = account(&app, "renamer", "active").await;
        let before = now_ms();
        let (status, body) = save(&app, &token, "Max").await;
        let after = now_ms();
        assert_eq!(status, 200, "{body}");
        let next = body["username"]["nextChangeAt"]
            .as_i64()
            .expect("the cooldown runs");
        assert!(
            (before + USERNAME_CHANGE_COOLDOWN_MS..=after + USERNAME_CHANGE_COOLDOWN_MS).contains(&next),
            "{next}"
        );
        assert_eq!(body["username"]["promptOwed"], false, "a pick answers the prompt");
        assert_eq!(me(&app, &token).await["nextChangeAt"], next);

        let (status, body) = preview(&app, &token, "Zed").await;
        assert_eq!(status, 200);
        assert_eq!(body["ok"], false);
        assert_eq!(body["reason"], "cooldown");
        assert_eq!(body["nextChangeAt"], next);
        assert!(
            body["message"]
                .as_str()
                .is_some_and(|message| message.contains("24 hours")),
            "{body}"
        );

        let (status, body) = save(&app, &token, "Zed").await;
        assert_eq!(status, 409, "{body}");
        assert_eq!(body["error"]["details"]["reason"], "cooldown");
        assert_eq!(body["error"]["details"]["nextChangeAt"], next);
        assert_eq!(me(&app, &token).await["name"], "Max");
    }

    #[tokio::test]
    async fn r1435_skip_answers_the_prompt_keeps_the_default_and_starts_no_clock() {
        let app = test_app().await;
        let token = account(&app, "skipper", "active").await;
        let before = me(&app, &token).await;
        let (status, _headers, body) = call(&app, "POST", SKIP, Some(&token), Value::Null).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(
            body["username"],
            json!({ "name": before["name"], "nextChangeAt": null, "promptOwed": false })
        );
        assert_eq!(me(&app, &token).await["promptOwed"], false, "never shown again");
        // A second skip changes nothing.
        assert_eq!(call(&app, "POST", SKIP, Some(&token), Value::Null).await.0, 200);
        // The first pick away from the default is still allowed at once.
        assert_eq!(save(&app, &token, "Ravi").await.0, 200);
        assert_eq!(me(&app, &token).await["promptOwed"], false);
    }

    #[tokio::test]
    async fn r1435_a_save_must_send_a_previewed_username() {
        let app = test_app().await;
        let token = account(&app, "careful", "active").await;
        let own = me(&app, &token).await["name"]
            .as_str()
            .expect("a name")
            .to_string();

        // Not a username at all.
        for body in [json!({}), json!({ "username": 7 })] {
            let (status, _headers, reply) = call(&app, "PUT", SAVE, Some(&token), body).await;
            assert_eq!(status, 400, "{reply}");
        }
        for sent in ["Max#0", "Max#", "Max#x"] {
            assert_eq!(save(&app, &token, sent).await.0, 400, "{sent}");
        }
        // A refused base: 400, with the refusal as a preview gives it.
        let (status, body) = save(&app, &token, "BigAss").await;
        assert_eq!(status, 400);
        assert_eq!(body["error"]["details"]["reason"], "not_allowed");
        // A base not in its stored form: 409 with the preview of what it would be.
        let (status, body) = save(&app, &token, "ＭＡＸ").await;
        assert_eq!(status, 409);
        assert_eq!(body["error"]["details"]["username"], "MAX");
        // The name the player has already: refused, and nothing is written.
        assert_eq!(save(&app, &token, &own).await.0, 409);
        assert_eq!(
            preview(&app, &token, &own).await.1["reason"],
            "characters",
            "a # is no base"
        );
        assert_eq!(me(&app, &token).await["name"], own.as_str());
        assert_eq!(me(&app, &token).await["nextChangeAt"], Value::Null);
    }

    #[tokio::test]
    async fn r1435_the_preview_says_when_a_name_is_already_the_players_own() {
        let app = test_app().await;
        let user_id = "user-holder";
        let token = add_user(&app, user_id, "holder@example.test", true);
        fake(&app).await.seed_profile(json!({
            "id": "holder",
            "userId": user_id,
            "status": "active",
            "username": "Max",
            "usernamePrompted": true,
            "usernameChangedAt": now_ms() - USERNAME_CHANGE_COOLDOWN_MS,
        }));
        assert_eq!(me(&app, &token).await["name"], "Max");
        assert_eq!(me(&app, &token).await["nextChangeAt"], Value::Null);
        let (status, body) = preview(&app, &token, "Max").await;
        assert_eq!(status, 200);
        assert_eq!(body["ok"], false);
        assert_eq!(body["reason"], "unchanged");
        let (status, body) = save(&app, &token, "Max").await;
        assert_eq!(status, 409);
        assert_eq!(body["error"]["details"]["reason"], "unchanged");
        // Their own name is not taken against them: another case of it is theirs bare.
        assert_eq!(
            preview(&app, &token, "MAX").await.1,
            json!({ "ok": true, "base": "MAX", "username": "MAX", "tagged": false })
        );
    }

    /// R109: the preview counts against the per-account limit like every other request.
    #[tokio::test(start_paused = true)]
    async fn r1435_r109_the_preview_is_rate_limited_per_account() {
        let app = test_app().await;
        let token = account(&app, "typist", "active").await;
        for n in 0..API_REQUESTS_PER_MINUTE {
            assert_eq!(preview(&app, &token, "Max").await.0, 200, "request {n}");
        }
        let (status, body) = preview(&app, &token, "Max").await;
        assert_eq!(status, 429);
        assert_eq!(body["error"]["code"], "rate_limited");
    }
}
