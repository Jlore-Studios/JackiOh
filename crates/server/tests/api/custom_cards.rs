//! Custom cards on the account (`crates/server/src/api/custom_cards.rs`, SPEC §12.9, R915008).
//!
//! Not a test of §12.2's limits: those are the engine's validator's (`custom::validate_custom_card`,
//! R915001) and are tested there. Here the questions are the routes' own: is the id the server's,
//! does the version count and refuse a stale save, are the validator's issues passed through
//! untouched, what does each store outcome answer, is another profile's card indistinguishable from
//! a missing one, and does a card at every limit fit the request the router reads.

use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use serde_json::{Value, json};

use jackioh_engine::config::{
    CUSTOM_BUILDER_MAX_BYTES, CUSTOM_NAME_MAX_LENGTH, CUSTOM_PARAM_KEY_MAX_LENGTH, CUSTOM_PARAMS_MAX,
    CUSTOM_SOURCE_MAX_BYTES, CUSTOM_TEXT_MAX_LENGTH,
};
use jackioh_engine::custom::validate_custom_card;
use jackioh_engine::{CustomCard, CustomFrame};
use jackioh_server::api::http::AuthLevel;
use jackioh_server::app::{self, App, now_ms};
use jackioh_server::config::{API_MAX_BODY_BYTES, CUSTOM_CARDS_PER_ACCOUNT};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{CustomCardWrite, Db};

use crate::support::deps::{add_user, call, test_app};

const OWNER: &str = "owner";
const OTHER: &str = "other";

// Harness (a private copy per file, SURFACE rule 5)

fn from<T: serde::de::DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_else(|error| panic!("{error}: {value}"))
}

fn json_of<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// `{ ...base, ...extra }` on JSON objects.
fn merged(mut base: Value, extra: Value) -> Value {
    if let (Some(into), Value::Object(extra)) = (base.as_object_mut(), extra) {
        for (key, value) in extra {
            into.insert(key, value);
        }
    }
    base
}

async fn fake(app: &App) -> tokio::sync::MutexGuard<'_, FakeData> {
    match &app.db {
        Db::Fake(data) => data.lock().await,
        Db::Pg(_) => panic!("the API tests run on the fake store"),
    }
}

async fn profile(app: &App, id: &str, status: &str) -> String {
    let user_id = format!("user-{id}");
    fake(app)
        .await
        .seed_profile(json!({ "id": id, "userId": user_id, "status": status }));
    add_user(app, &user_id, &format!("{id}@example.test"), true)
}

struct Ctx {
    app: Arc<App>,
    token: String,
    other_token: String,
}

async fn setup() -> Ctx {
    let app = test_app().await;
    let token = profile(&app, OWNER, "active").await;
    let other_token = profile(&app, OTHER, "active").await;
    Ctx {
        app,
        token,
        other_token,
    }
}

impl Ctx {
    async fn request(&self, method: &str, path: &str, body: Option<Value>, bearer: &str) -> (u16, Value) {
        let (status, _headers, answer) =
            call(&self.app, method, path, Some(bearer), body.unwrap_or(Value::Null)).await;
        (status, answer)
    }

    async fn create(&self, body: Value, bearer: &str) -> (u16, Value) {
        self.request("POST", "/api/custom-cards", Some(body), bearer)
            .await
    }

    async fn update(&self, id: &str, body: Value, bearer: &str) -> (u16, Value) {
        self.request("PUT", &format!("/api/custom-cards/{id}"), Some(body), bearer)
            .await
    }

    async fn get(&self, id: &str, bearer: &str) -> (u16, Value) {
        self.request("GET", &format!("/api/custom-cards/{id}"), None, bearer)
            .await
    }

    async fn delete(&self, id: &str, bearer: &str) -> (u16, Value) {
        self.request("DELETE", &format!("/api/custom-cards/{id}"), None, bearer)
            .await
    }

    async fn list(&self, bearer: &str) -> Value {
        let (status, body) = self.request("GET", "/api/custom-cards", None, bearer).await;
        assert_eq!(status, 200, "{body}");
        body
    }

    async fn table(&self) -> Vec<Value> {
        fake(&self.app)
            .await
            .tables
            .custom_cards
            .iter()
            .map(json_of)
            .collect()
    }
}

/// A Unit's frame within §12.2, with a keyword and a number.
fn frame(name: &str) -> Value {
    json!({
        "name": name,
        "type": "Unit",
        "cost": 2,
        "rarity": "Rare",
        "attack": 2,
        "health": 3,
        "keywords": ["Taunt"],
        "text": "Cry: Deal {dmg}.",
        "params": [{ "key": "dmg", "base": 1, "radiant": 2 }],
    })
}

const SOURCE: &str = "return Card {\n  cry = function(c, n)\n    c:damage(\"an enemy\", n.dmg)\n  end,\n}\n";

fn card_body(extra: Value) -> Value {
    merged(
        json!({
            "frame": frame("Spark"),
            "source": SOURCE,
            "builder": { "trigger": "cry", "steps": [{ "verb": "damage", "n": "dmg" }] },
        }),
        extra,
    )
}

/// The engine's verdict on the card a body makes, as the route must pass it on.
fn issues_for(id: &str, version: i32, body: &Value) -> Value {
    let card = CustomCard {
        id: id.to_string(),
        version,
        frame: from::<CustomFrame>(body["frame"].clone()),
        source: body["source"].as_str().unwrap_or_default().to_string(),
        builder: body.get("builder").filter(|value| !value.is_null()).cloned(),
    };
    json_of(&validate_custom_card(&card))
}

/// Creates the body's card for `bearer`, answering it.
async fn created(ctx: &Ctx, body: Value, bearer: &str) -> Value {
    let (status, answer) = ctx.create(body, bearer).await;
    assert_eq!(status, 201, "{answer}");
    answer["card"].clone()
}

fn id_of(card: &Value) -> String {
    card["id"].as_str().expect("a card id").to_string()
}

mod custom_cards_on_the_account_r915008 {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r915008_creates_a_card_minting_its_id_at_version_1_and_reads_it_back() {
        let ctx = setup().await;
        let body = card_body(json!({}));
        let card = created(&ctx, body.clone(), &ctx.token).await;

        // The id is the server's: `custom-` and 32 lowercase hex digits (R915000).
        let id = id_of(&card);
        let tail = id.strip_prefix("custom-").expect("the custom prefix");
        assert_eq!(tail.len(), 32, "{id}");
        assert!(
            tail.bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        );
        // The engine's `CustomCard`, exactly: version 1, and the frame, source and builder sent.
        assert_eq!(
            card,
            json!({
                "id": id,
                "version": 1,
                "frame": json_of(&from::<CustomFrame>(frame("Spark"))),
                "source": SOURCE,
                "builder": body["builder"],
            })
        );
        assert!(card.get("profileId").is_none());

        let (status, read) = ctx.get(&id, &ctx.token).await;
        assert_eq!(status, 200);
        assert_eq!(read, json!({ "card": card }));
        assert_eq!(
            ctx.list(&ctx.token).await,
            json!({ "cards": [card], "limits": { "cards": CUSTOM_CARDS_PER_ACCOUNT } })
        );

        // A second card is a second id, listed after the first.
        tokio::time::advance(Duration::from_millis(10)).await;
        let vanilla = created(&ctx, json!({ "frame": frame("Plain"), "source": "" }), &ctx.token).await;
        assert_ne!(id_of(&vanilla), id);
        assert!(vanilla.get("builder").is_none(), "{vanilla}");
        assert_eq!(vanilla["source"], "");
        assert_eq!(ctx.list(&ctx.token).await["cards"], json!([card, vanilla]));
    }

    #[tokio::test(start_paused = true)]
    async fn r915008_saves_a_card_whole_counting_its_version_and_refuses_a_stale_save_with_409() {
        let ctx = setup().await;
        let card = created(&ctx, card_body(json!({})), &ctx.token).await;
        let id = id_of(&card);
        let made_at = now_ms();

        tokio::time::advance(Duration::from_millis(5_000)).await;
        let edit = json!({ "version": 1, "frame": frame("Big Spark"), "source": "return Card {}" });
        let (status, saved) = ctx.update(&id, edit, &ctx.token).await;
        assert_eq!(status, 200, "{saved}");
        // Replaced whole: the builder the edit did not send is gone.
        assert_eq!(
            saved,
            json!({ "card": {
                "id": id,
                "version": 2,
                "frame": json_of(&from::<CustomFrame>(frame("Big Spark"))),
                "source": "return Card {}",
            } })
        );
        let row = ctx.table().await.remove(0);
        assert_eq!(row["createdAt"], json!(made_at));
        assert_eq!(row["updatedAt"], json!(now_ms()));

        // A tab that still holds version 1: 409, naming the version it should open again.
        let late = json!({ "version": 1, "frame": frame("Late"), "source": "" });
        let (status, refused) = ctx.update(&id, late, &ctx.token).await;
        assert_eq!(status, 409);
        assert_eq!(refused["error"]["code"], "conflict");
        assert_eq!(refused["error"]["details"], json!({ "version": 2 }));
        let (_, read) = ctx.get(&id, &ctx.token).await;
        assert_eq!(read, saved);

        // At the version it holds, it saves: version 3.
        let (status, again) = ctx
            .update(
                &id,
                json!({ "version": 2, "frame": frame("Late"), "source": "" }),
                &ctx.token,
            )
            .await;
        assert_eq!(status, 200);
        assert_eq!(again["card"]["version"], 3);
    }

    #[tokio::test(start_paused = true)]
    async fn r915008_refuses_a_card_the_validator_refuses_with_its_issues_and_writes_nothing() {
        let ctx = setup().await;
        let mut bad_frame = frame("Spark");
        bad_frame["cost"] = json!(99);
        bad_frame["keywords"] = json!(["Taunt", "Taunt"]);
        let body = card_body(json!({
            "frame": bad_frame,
            "source": "x".repeat(CUSTOM_SOURCE_MAX_BYTES + 1),
        }));
        // PREMISE: the validator refuses this card, for more than one reason.
        let issues = issues_for("custom-0", 1, &body);
        assert!(
            issues.as_array().is_some_and(|issues| issues.len() >= 3),
            "{issues}"
        );

        let (status, refused) = ctx.create(body.clone(), &ctx.token).await;
        assert_eq!(status, 400);
        assert_eq!(refused["error"]["code"], "bad_request");
        assert_eq!(refused["error"]["message"], issues[0]["message"]);
        assert_eq!(refused["error"]["details"], json!({ "issues": issues }));
        assert!(ctx.table().await.is_empty());

        // An update is judged the same way, and the stored card stays as it was.
        let card = created(&ctx, card_body(json!({})), &ctx.token).await;
        let id = id_of(&card);
        let (status, refused) = ctx
            .update(&id, merged(body.clone(), json!({ "version": 1 })), &ctx.token)
            .await;
        assert_eq!(status, 400);
        assert_eq!(
            refused["error"]["details"],
            json!({ "issues": issues_for(&id, 2, &body) })
        );
        assert_eq!(ctx.get(&id, &ctx.token).await.1["card"], card);
    }

    #[tokio::test(start_paused = true)]
    async fn r915008_refuses_a_malformed_body_or_id_with_400_before_touching_the_store() {
        let ctx = setup().await;
        for body in [
            json!({ "source": "" }),
            json!({ "frame": "a frame", "source": "" }),
            json!({ "frame": merged(frame("Spark"), json!({ "cost": "two" })), "source": "" }),
            json!({ "frame": frame("Spark"), "source": 42 }),
        ] {
            let (status, refused) = ctx.create(body.clone(), &ctx.token).await;
            assert_eq!(status, 400, "{body}");
            assert_eq!(refused["error"]["code"], "bad_request");
        }
        assert!(ctx.table().await.is_empty());

        let card = created(&ctx, card_body(json!({})), &ctx.token).await;
        let id = id_of(&card);
        for version in [json!(null), json!(0), json!(-1), json!(1.5), json!("1")] {
            let body = card_body(json!({ "version": version }));
            let (status, _) = ctx.update(&id, body, &ctx.token).await;
            assert_eq!(status, 400, "version {version}");
        }
        // Not a custom id: a catalog id, an upper-case tail, an empty tail.
        for bad in ["core-001", "custom-ABC", "custom-"] {
            assert_eq!(ctx.get(bad, &ctx.token).await.0, 400, "{bad}");
            assert_eq!(
                ctx.update(bad, card_body(json!({ "version": 1 })), &ctx.token)
                    .await
                    .0,
                400,
                "{bad}"
            );
            assert_eq!(ctx.delete(bad, &ctx.token).await.0, 400, "{bad}");
        }
        assert_eq!(ctx.get(&id, &ctx.token).await.1["card"], card);
    }

    #[tokio::test(start_paused = true)]
    async fn r915008_answers_another_profile_s_card_as_missing_and_leaves_it_alone() {
        let ctx = setup().await;
        let card = created(&ctx, card_body(json!({})), &ctx.token).await;
        let id = id_of(&card);

        let (status, missing) = ctx.get(&id, &ctx.other_token).await;
        assert_eq!(status, 404);
        assert_eq!(missing["error"]["code"], "not_found");
        // The same answer as for an id nobody holds.
        let (status, nobody) = ctx.get("custom-nobody", &ctx.other_token).await;
        assert_eq!(status, 404);
        assert_eq!(nobody, missing);

        let (status, _) = ctx
            .update(&id, card_body(json!({ "version": 1 })), &ctx.other_token)
            .await;
        assert_eq!(status, 404);
        assert_eq!(
            ctx.delete(&id, &ctx.other_token).await,
            (200, json!({ "deleted": false }))
        );
        assert_eq!(ctx.list(&ctx.other_token).await["cards"], json!([]));
        assert_eq!(ctx.get(&id, &ctx.token).await.1["card"], card);
    }

    #[tokio::test(start_paused = true)]
    async fn r915008_refuses_a_create_past_custom_cards_per_account_with_a_409_naming_the_limit() {
        let ctx = setup().await;
        // The account at its cap, straight into the store.
        for n in 0..CUSTOM_CARDS_PER_ACCOUNT {
            let write: CustomCardWrite = from(json!({
                "id": format!("custom-held-{n}"),
                "profileId": OWNER,
                "frame": frame("Held"),
                "source": "",
                "builder": null,
                "at": now_ms(),
            }));
            let mut t = ctx.app.db.begin(None).await.expect("begin");
            t.custom_cards_create(&write, CUSTOM_CARDS_PER_ACCOUNT as i64)
                .await
                .expect("customCards.create");
            t.commit().await.expect("commit");
        }

        let (status, refused) = ctx.create(card_body(json!({})), &ctx.token).await;
        assert_eq!(status, 409);
        assert_eq!(refused["error"]["code"], "conflict");
        assert_eq!(
            refused["error"]["details"],
            json!({ "limit": CUSTOM_CARDS_PER_ACCOUNT })
        );
        assert_eq!(ctx.table().await.len(), CUSTOM_CARDS_PER_ACCOUNT);

        // A save at the cap is not a create; another account has a cap of its own.
        let (status, _) = ctx
            .update("custom-held-0", card_body(json!({ "version": 1 })), &ctx.token)
            .await;
        assert_eq!(status, 200);
        created(&ctx, card_body(json!({})), &ctx.other_token).await;
    }

    #[tokio::test(start_paused = true)]
    async fn r915008_deletes_a_card_idempotently() {
        let ctx = setup().await;
        let card = created(&ctx, card_body(json!({})), &ctx.token).await;
        let id = id_of(&card);
        assert_eq!(
            ctx.delete(&id, &ctx.token).await,
            (200, json!({ "deleted": true }))
        );
        assert_eq!(
            ctx.delete(&id, &ctx.token).await,
            (200, json!({ "deleted": false }))
        );
        assert_eq!(ctx.get(&id, &ctx.token).await.0, 404);
        assert!(ctx.table().await.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn r915008_a_card_at_every_size_limit_fits_the_request_the_router_reads_and_saves() {
        let ctx = setup().await;
        // A source of exactly `CUSTOM_SOURCE_MAX_BYTES`, written as a script is: lines, quotes and all.
        let line = "  c:damage(\"each enemy unit\", n.p1) -- a comment\n";
        let mut source = String::from("return Card {\n");
        while source.len() + line.len() <= CUSTOM_SOURCE_MAX_BYTES {
            source.push_str(line);
        }
        source.push_str(&"-".repeat(CUSTOM_SOURCE_MAX_BYTES - source.len()));
        assert_eq!(source.len(), CUSTOM_SOURCE_MAX_BYTES);
        // A builder's model of exactly `CUSTOM_BUILDER_MAX_BYTES` of compact JSON.
        let shell = serde_json::to_string(&json!({ "notes": "" }))
            .expect("json")
            .len();
        let builder = json!({ "notes": "b".repeat(CUSTOM_BUILDER_MAX_BYTES - shell) });
        assert_eq!(
            serde_json::to_string(&builder).expect("json").len(),
            CUSTOM_BUILDER_MAX_BYTES
        );
        // A frame at its own limits: the longest name and text, every number.
        let params: Vec<Value> = (0..CUSTOM_PARAMS_MAX)
            .map(|n| {
                let key = format!("p{n}{}", "k".repeat(CUSTOM_PARAM_KEY_MAX_LENGTH - 2));
                json!({ "key": key, "base": 1, "radiant": 2 })
            })
            .collect();
        let text = "x".repeat(CUSTOM_TEXT_MAX_LENGTH);
        let big_frame = merged(
            frame(&"N".repeat(CUSTOM_NAME_MAX_LENGTH)),
            json!({ "text": text, "params": params }),
        );
        let body = json!({ "frame": big_frame, "source": source, "builder": builder });
        // PREMISE: the validator passes it, and it is the request the router reads in full.
        assert_eq!(issues_for("custom-0", 1, &body), json!([]));
        assert!(
            body.to_string().len() <= API_MAX_BODY_BYTES,
            "{}",
            body.to_string().len()
        );

        let card = created(&ctx, body.clone(), &ctx.token).await;
        assert_eq!(card["source"], body["source"]);
        assert_eq!(card["builder"], body["builder"]);
        let (status, saved) = ctx
            .update(&id_of(&card), merged(body, json!({ "version": 1 })), &ctx.token)
            .await;
        assert_eq!(status, 200, "{saved}");
    }
}

// §9.4's gate, and the script the server never runs

mod the_routes_a_pending_account_sees_no_custom_cards {
    use super::*;

    #[test]
    fn r915008_declares_every_custom_card_route_active() {
        let routes: Vec<(String, bool)> = app::ROUTES
            .iter()
            .filter(|(_, path, _, _)| path.starts_with("/api/custom-cards"))
            .map(|(method, path, auth, _)| (format!("{method} {path}"), matches!(auth, AuthLevel::Active)))
            .collect();
        assert_eq!(
            routes.iter().map(|(route, _)| route.as_str()).collect::<Vec<_>>(),
            vec![
                "GET /api/custom-cards",
                "POST /api/custom-cards",
                "GET /api/custom-cards/:id",
                "PUT /api/custom-cards/:id",
                "DELETE /api/custom-cards/:id",
            ]
        );
        assert!(routes.iter().all(|(_, active)| *active));
    }

    #[tokio::test(start_paused = true)]
    async fn r915008_a_pending_account_gets_403_and_nothing_is_saved() {
        let ctx = setup().await;
        let pending = profile(&ctx.app, "pending", "pending").await;
        let (status, refused) = ctx.create(card_body(json!({})), &pending).await;
        assert_eq!(status, 403);
        assert_eq!(refused["error"]["code"], "account_pending");
        let (status, _) = ctx.request("GET", "/api/custom-cards", None, &pending).await;
        assert_eq!(status, 403);
        assert!(ctx.table().await.is_empty());
    }

    #[test]
    fn r915008_the_server_links_no_luau_so_it_never_runs_a_script() {
        // #902 V4: no game crate takes `jackioh-luau`, and the server takes it neither; a source is
        // text the routes store.
        let manifest = include_str!("../../Cargo.toml");
        assert!(!manifest.contains("jackioh-luau"));
        assert!(!manifest.contains("mlua"));
    }
}
