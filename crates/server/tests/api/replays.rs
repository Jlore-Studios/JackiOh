//! Online replays on the server (SPEC §9.3, R768, issue #510): `GET /api/replays` and
//! `GET /api/replays/:matchId` (`src/api/replays.rs`), over the fake store and the real engine.
//!
//!  - Both seats' steps are `view_for` after a fresh fold of the first k accepted actions, for every
//!    k, and follow the seat the account played after a Glitch's swap (R677).
//!  - An account that held no seat gets what an unknown id gets (404); a match still open or live is
//!    409; a purged log is 410 `gone`; a fold that misses its recorded hash, or its results row when
//!    it has none, is 422 with part 1's reason.
//!  - No body holds the seed or the log; a page holds at most `REPLAY_PAGE_STEPS` steps; the list
//!    pages by `REPLAY_LIST_PAGE`; the cache keeps at most `REPLAY_CACHE_MATCHES` replays for at most
//!    `REPLAY_CACHE_TTL_SECONDS`; the routes answer 429 past `REPLAY_REQUESTS_PER_MINUTE`.
//!
//! Each match is written straight to the store as the actor and `api/results.rs` leave a finished
//! one: its row, its log, its results row, then the final hash.

use std::sync::Arc;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use jackioh_engine::config::REPLAY_PAGE_STEPS;
use jackioh_engine::{
    Action, ActionBody, GameState, PlayerId, ReplayOpen, ReplayRefusal, Winner, legal_actions, reduce,
    seat_to_act, view_for,
};
use jackioh_server::actor::clock::initial_clocks;
use jackioh_server::actor::engine::{fold, fold_args, hash_state, seat_played_by};
use jackioh_server::api::crypto::player_tag;
use jackioh_server::api::replays::ReplayCache;
use jackioh_server::api::retention::purge_expired;
use jackioh_server::app::{App, now_ms};
use jackioh_server::config::{
    MATCH_ACTION_RETENTION_DAYS, REPLAY_CACHE_MATCHES, REPLAY_CACHE_TTL_SECONDS, REPLAY_LIST_PAGE,
    REPLAY_REQUESTS_PER_MINUTE,
};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{Db, MatchActionRow, MatchRow, ResultRow, Room};

use crate::support::deps::{add_user, call, empty_test_app};
use crate::support::engine::{
    TEST_GLITCH_SWAP, begun_game, decks_that_open_on_the_mulligans, fake_deck, hand_card, install_test_cards,
    past_the_mulligans, real_pool,
};

const A: &str = "profile-a";
const B: &str = "profile-b";
const C: &str = "profile-c";
const DAY_MS: i64 = 86_400_000;
/// How many log actions `played_game` lets a game run before its seat to act concedes.
const PLAYED_ACTIONS: usize = 36;
/// A catalog version no build has: a match played on it is refused before anything is folded.
const OLD_CATALOG: &str = "v0.0.1-old";

// ---------------------------------------------------------------------------------------------
// Plumbing (private copies: each test file of this binary keeps its own)
// ---------------------------------------------------------------------------------------------

fn from<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_else(|error| panic!("{error}: {value}"))
}

fn to_json<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// One store call in its own transaction.
macro_rules! q {
    ($app:expr, $method:ident($($arg:expr),* $(,)?)) => {{
        let mut tx = $app.db.begin(None).await.expect("begin");
        let out = tx.$method($($arg),*).await.expect(stringify!($method));
        tx.commit().await.expect("commit");
        out
    }};
}

async fn fake(app: &App) -> tokio::sync::MutexGuard<'_, FakeData> {
    match &app.db {
        Db::Fake(data) => data.lock().await,
        Db::Pg(_) => panic!("the API tests run on the fake store"),
    }
}

/// The three accounts' tokens: A and B play, C never holds a seat.
struct Tokens {
    a: String,
    b: String,
    c: String,
}

/// The test app over an empty store, with A, B and C active and signed in.
async fn app_with_players() -> (Arc<App>, Tokens) {
    let app = empty_test_app().await;
    let mut tokens = Vec::new();
    for (id, user) in [(A, "user-a"), (B, "user-b"), (C, "user-c")] {
        fake(&app)
            .await
            .seed_profile(json!({ "id": id, "userId": user, "status": "active" }));
        tokens.push(add_user(&app, user, &format!("{user}@example.test"), true));
    }
    let [a, b, c] = <[String; 3]>::try_from(tokens).expect("three tokens");
    (app, Tokens { a, b, c })
}

async fn get(app: &Arc<App>, path: &str, token: &str) -> (u16, Value) {
    let (status, _headers, body) = call(app, "GET", path, Some(token), Value::Null).await;
    (status, body)
}

// ---------------------------------------------------------------------------------------------
// Games
// ---------------------------------------------------------------------------------------------

/// A finished game: what made it and the state it ended on.
#[derive(Clone)]
struct Played {
    seed: String,
    decks: (Vec<String>, Vec<String>),
    log: Vec<Action>,
    last: GameState,
}

impl Played {
    /// The state after the first `k` log actions, folded afresh as the actor's rebuild folds it.
    fn state_at(&self, k: usize) -> GameState {
        let folded = fold(&fold_args(
            &self.seed,
            &self.decks,
            self.log[..k].to_vec(),
            None,
            None,
            None,
        ));
        assert!(folded.errors.is_empty(), "step {k}: {:?}", folded.errors);
        folded.state
    }

    fn steps(&self) -> usize {
        self.log.len() + 1
    }
}

/// One action by the seat to act, logged under the test's nonce.
fn act(state: &GameState, log: &mut Vec<Action>, seat: PlayerId, body: ActionBody) -> GameState {
    let action = Action::new(body, seat, format!("logged-action-{}", log.len() + 1));
    let result = reduce(state, &action);
    if let Some(error) = result.error {
        panic!("{action:?} was refused: {error}");
    }
    log.push(action);
    result.state
}

/// Plays `state` on: the seat to act ends its turn when it may, and otherwise takes its first legal
/// action, until the game ends or the log holds `max` actions; then the seat to act concedes.
fn play_out(mut state: GameState, mut log: Vec<Action>, max: usize) -> (GameState, Vec<Action>) {
    while state.result.is_none() && log.len() < max {
        let seat = seat_to_act(&state).expect("a seat to act");
        let legal = legal_actions(&state, seat);
        let body = if legal.iter().any(|body| matches!(body, ActionBody::EndTurn)) {
            ActionBody::EndTurn
        } else {
            legal.first().cloned().expect("a legal action")
        };
        state = act(&state, &mut log, seat, body);
    }
    if state.result.is_none() {
        let seat = seat_to_act(&state).expect("a seat to act");
        state = act(&state, &mut log, seat, ActionBody::Concede);
    }
    (state, log)
}

/// A game of real cards under `seed`, played to its end (`play_out`).
fn played_game(seed: &str, max: usize) -> Played {
    install_test_cards();
    let decks = decks_that_open_on_the_mulligans(&real_pool(), seed);
    let (last, log) = play_out(begun_game(seed, decks.clone()), Vec::new(), max);
    Played {
        seed: seed.to_string(),
        decks,
        log,
        last,
    }
}

/// What a finished match is written with besides its game.
struct Finish<'a> {
    id: &'a str,
    /// The seats as the match began (A first unless a test says otherwise).
    players: (&'a str, &'a str),
    /// The final hash `api/results.rs` records; `None` for a match from before migration 0027.
    hash: Option<String>,
    ended_at: i64,
    /// Fields laid over the match row's JSON (`catalogVersion`, `mode`, `portraits`).
    row: Value,
    /// Fields laid over the results row's JSON (`turns`).
    result: Value,
}

impl<'a> Finish<'a> {
    fn new(id: &'a str, played: &Played, ended_at: i64) -> Finish<'a> {
        Finish {
            id,
            players: (A, B),
            hash: Some(hash_state(&played.last)),
            ended_at,
            row: json!({}),
            result: json!({}),
        }
    }
}

fn spread(mut base: Value, over: &Value) -> Value {
    if let (Some(fields), Some(over)) = (base.as_object_mut(), over.as_object()) {
        for (key, value) in over {
            fields.insert(key.clone(), value.clone());
        }
    }
    base
}

/// A match row as the registry writes one, live.
fn match_row(id: &str, players: (&str, &str), played: &Played, created_at: i64, over: &Value) -> MatchRow {
    from(spread(
        json!({
            "id": id,
            "seed": played.seed,
            "players": [players.0, players.1],
            "decks": [played.decks.0, played.decks.1],
            "catalogVersion": jackioh_cards::catalog_version(),
            "status": "live",
            "createdAt": created_at,
            "finishedAt": null,
            "clocks": to_json(&initial_clocks(created_at)),
        }),
        over,
    ))
}

/// The results row the writer leaves for `played`: its players are the seats as the game ended
/// (R677), so a Glitch's swap crosses them.
fn result_row(finish: &Finish<'_>, played: &Played) -> ResultRow {
    let (first, second) = finish.players;
    let players = if seat_played_by(&played.last, PlayerId::P1) == PlayerId::P1 {
        (first, second)
    } else {
        (second, first)
    };
    let ended = played.last.result.as_ref().expect("a finished game");
    let winner = match ended.winner {
        Winner::P1 => json!(players.0),
        Winner::P2 => json!(players.1),
        Winner::Draw => Value::Null,
    };
    from(spread(
        json!({
            "matchId": finish.id,
            "players": [players.0, players.1],
            "winnerProfileId": winner,
            "reason": to_json(&ended.reason),
            "turns": played.last.turn,
            "endedAt": finish.ended_at,
            "ratingBefore": [1000, 1000],
            "ratingAfter": [1000, 1000],
        }),
        &finish.result,
    ))
}

/// A finished match, written as the actor and the results writer leave one.
async fn seed_finished(app: &Arc<App>, finish: Finish<'_>, played: &Played) {
    let created_at = finish.ended_at - DAY_MS / 24;
    q!(
        app,
        matches_create(&match_row(
            finish.id,
            finish.players,
            played,
            created_at,
            &finish.row
        ))
    );
    let rows: Vec<MatchActionRow> = played
        .log
        .iter()
        .enumerate()
        .map(|(n, action)| MatchActionRow {
            match_id: finish.id.to_string(),
            seq: i64::try_from(n + 1).expect("a small seq"),
            action: action.clone(),
            at: created_at,
        })
        .collect();
    q!(app, matches_append_actions(&rows));
    q!(app, results_insert(&result_row(&finish, played)));
    q!(app, matches_finish(finish.id, finish.ended_at));
    if let Some(hash) = &finish.hash {
        q!(app, matches_record_final_hash(finish.id, hash));
    }
}

/// Every step of the caller's replay, a page at a time from step 0, each page's body checked for
/// its keys; the pages' `from` is the last page's end.
async fn all_steps(app: &Arc<App>, match_id: &str, token: &str) -> Vec<Value> {
    let mut steps: Vec<Value> = Vec::new();
    loop {
        let (status, body) = get(
            app,
            &format!("/api/replays/{match_id}?from={}", steps.len()),
            token,
        )
        .await;
        assert_eq!(status, 200, "{body}");
        let keys: Vec<&String> = body.as_object().expect("an object").keys().collect();
        assert_eq!(keys, vec!["from", "matchId", "steps", "total"]);
        assert_eq!(body["matchId"], match_id);
        assert_eq!(body["from"], steps.len());
        let page = body["steps"].as_array().expect("steps").clone();
        assert!(!page.is_empty(), "an empty page at {}", steps.len());
        assert!(page.len() <= REPLAY_PAGE_STEPS);
        let total = body["total"].as_u64().expect("a total") as usize;
        steps.extend(page);
        if steps.len() >= total {
            assert_eq!(steps.len(), total);
            return steps;
        }
    }
}

/// What step `k` of a seat's replay must be.
fn step_of(played: &Played, k: usize, seat: PlayerId) -> Value {
    let state = played.state_at(k);
    json!({ "step": k, "turn": state.turn, "view": to_json(&view_for(&state, seat)) })
}

fn error_code(body: &Value) -> &str {
    body["error"]["code"].as_str().unwrap_or("")
}

// ---------------------------------------------------------------------------------------------
// The steps
// ---------------------------------------------------------------------------------------------

mod r768_the_steps {
    use super::*;

    #[tokio::test]
    async fn r768_both_seats_steps_equal_view_for_after_a_fresh_fold_of_every_prefix() {
        let (app, tokens) = app_with_players().await;
        let played = played_game("replay-route-1", PLAYED_ACTIONS);
        assert!(
            played.steps() > 2 * REPLAY_PAGE_STEPS,
            "the game spans three pages"
        );
        seed_finished(&app, Finish::new("m-1", &played, now_ms()), &played).await;

        for (token, seat) in [(&tokens.a, PlayerId::P1), (&tokens.b, PlayerId::P2)] {
            let steps = all_steps(&app, "m-1", token).await;
            assert_eq!(steps.len(), played.steps());
            for (k, step) in steps.iter().enumerate() {
                let keys: Vec<&String> = step.as_object().expect("a step").keys().collect();
                assert_eq!(keys, vec!["step", "turn", "view"]);
                assert_eq!(step, &step_of(&played, k, seat), "{seat} step {k}");
            }
            // No seed, no log: a nonce appears in no view.
            let text = Value::Array(steps).to_string();
            assert!(!text.contains(&played.seed), "the seed left the server");
            assert!(!text.contains("logged-action-"), "the log left the server");
        }
    }

    #[tokio::test]
    async fn r768_r677_steps_follow_the_seat_the_account_played_after_a_glitch_swap() {
        let (app, tokens) = app_with_players().await;
        install_test_cards();
        let decks = (fake_deck(&[TEST_GLITCH_SWAP]), fake_deck(&[]));
        let seed = (0..5_000)
            .map(|k| format!("replay-glitch-{k}"))
            .find(|seed| {
                hand_card(
                    &past_the_mulligans(&begun_game(seed, decks.clone())),
                    PlayerId::P1,
                    TEST_GLITCH_SWAP,
                )
                .is_some()
            })
            .expect("a seed that deals the swap to p1");
        // Both mulligans keeping the whole hand, the swap, two more actions, a concede.
        let mut state = begun_game(&seed, decks.clone());
        let mut log = Vec::new();
        for seat in [PlayerId::P1, PlayerId::P2] {
            let keep = state.players[seat]
                .hand
                .iter()
                .map(|card| card.id.clone())
                .collect();
            state = act(&state, &mut log, seat, ActionBody::Mulligan { keep });
        }
        let card = hand_card(&state, PlayerId::P1, TEST_GLITCH_SWAP).expect("the swap in hand");
        state = act(
            &state,
            &mut log,
            PlayerId::P1,
            from(json!({ "type": "play", "instanceId": card })),
        );
        let (last, log) = play_out(state, log, 5);
        let played = Played {
            seed,
            decks,
            log,
            last,
        };
        assert_eq!(seat_played_by(&played.last, PlayerId::P1), PlayerId::P2);
        seed_finished(&app, Finish::new("m-glitch", &played, now_ms()), &played).await;

        // A's first page ends at the swap: steps 0-2 are p1's view, every later one p2's.
        let (status, body) = get(&app, "/api/replays/m-glitch", &tokens.a).await;
        assert_eq!(status, 200, "{body}");
        let first: Vec<Value> = body["steps"].as_array().expect("steps").clone();
        assert_eq!(first.len(), 3);
        let steps = all_steps(&app, "m-glitch", &tokens.a).await;
        for (k, step) in steps.iter().enumerate() {
            let seat = if k < 3 { PlayerId::P1 } else { PlayerId::P2 };
            assert_eq!(step, &step_of(&played, k, seat), "A's step {k}");
        }
        // B, who began in p2, plays p1 after the swap.
        let steps = all_steps(&app, "m-glitch", &tokens.b).await;
        for (k, step) in steps.iter().enumerate() {
            let seat = if k < 3 { PlayerId::P2 } else { PlayerId::P1 };
            assert_eq!(step, &step_of(&played, k, seat), "B's step {k}");
        }
    }

    #[tokio::test]
    async fn r768_caps_a_page_at_replay_page_steps() {
        let (app, tokens) = app_with_players().await;
        let played = played_game("replay-route-2", PLAYED_ACTIONS);
        seed_finished(&app, Finish::new("m-cap", &played, now_ms()), &played).await;

        let (status, body) = get(&app, "/api/replays/m-cap?count=1000", &tokens.a).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["steps"].as_array().map(Vec::len), Some(REPLAY_PAGE_STEPS));
        let (status, body) = get(&app, "/api/replays/m-cap?from=3&count=2", &tokens.a).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(
            body["steps"],
            json!([
                step_of(&played, 3, PlayerId::P1),
                step_of(&played, 4, PlayerId::P1)
            ])
        );

        let total = played.steps();
        for query in [
            "count=0".to_string(),
            format!("from={total}"),
            "from=x".to_string(),
            "count=-1".to_string(),
        ] {
            let (status, body) = get(&app, &format!("/api/replays/m-cap?{query}"), &tokens.a).await;
            assert_eq!(status, 400, "{query}: {body}");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The refusals
// ---------------------------------------------------------------------------------------------

mod r768_the_refusals {
    use super::*;

    #[tokio::test]
    async fn r768_a_non_seat_gets_the_same_not_found_as_an_unknown_match() {
        let (app, tokens) = app_with_players().await;
        let played = played_game("replay-route-3", 4);
        seed_finished(&app, Finish::new("m-seat", &played, now_ms()), &played).await;

        let (status, held) = get(&app, "/api/replays/m-seat", &tokens.c).await;
        let (unknown_status, unknown) = get(&app, "/api/replays/no-such-match", &tokens.c).await;
        assert_eq!((status, unknown_status), (404, 404));
        assert_eq!(held, unknown);
        assert_eq!(error_code(&held), "not_found");
        // A seat reads it.
        assert_eq!(get(&app, "/api/replays/m-seat", &tokens.a).await.0, 200);
    }

    #[tokio::test]
    async fn r768_refuses_an_open_or_live_match() {
        let (app, tokens) = app_with_players().await;
        let played = played_game("replay-route-4", 4);
        q!(
            app,
            matches_create(&match_row("m-live", (A, B), &played, now_ms(), &json!({})))
        );
        let (status, body) = get(&app, "/api/replays/m-live", &tokens.a).await;
        assert_eq!(status, 409, "{body}");
        assert_eq!(error_code(&body), "conflict");

        // A claimed room: the match id is reserved and nothing plays it yet.
        let now = now_ms();
        let room: Room = from(json!({
            "code": "ABC234",
            "hostProfileId": A,
            "mode": "bo1",
            "hostDeck": played.decks.0,
            "hostTrio": null,
            "catalogVersion": jackioh_cards::catalog_version(),
            "createdAt": now,
            "expiresAt": now + 600_000,
            "guestProfileId": null,
            "matchId": null,
        }));
        assert!(q!(app, rooms_create(&room)));
        assert!(q!(app, rooms_claim("ABC234", B, "m-open", now)).is_some());
        for token in [&tokens.a, &tokens.b] {
            assert_eq!(get(&app, "/api/replays/m-open", token).await.0, 409);
        }
        assert_eq!(get(&app, "/api/replays/m-open", &tokens.c).await.0, 404);
    }

    #[tokio::test]
    async fn r768_a_purged_log_answers_gone_and_is_not_listed() {
        let (app, tokens) = app_with_players().await;
        let played = played_game("replay-route-5", 4);
        let long_ago = now_ms() - (MATCH_ACTION_RETENTION_DAYS + 1) * DAY_MS;
        seed_finished(&app, Finish::new("m-old", &played, long_ago), &played).await;
        let (_, before) = get(&app, "/api/replays", &tokens.a).await;
        assert_eq!(before["replays"][0]["matchId"], "m-old");

        let purged = purge_expired(&app).await.expect("the purge runs");
        assert_eq!(purged.match_actions, played.log.len() as i64);
        let (status, body) = get(&app, "/api/replays/m-old", &tokens.a).await;
        assert_eq!(status, 410, "{body}");
        assert_eq!(error_code(&body), "gone");
        assert_eq!(body["error"]["message"], "expired");
        let (status, list) = get(&app, "/api/replays", &tokens.a).await;
        assert_eq!(status, 200);
        assert_eq!(list["replays"], json!([]));
    }

    #[tokio::test]
    async fn r768_refuses_a_fold_that_misses_its_hash_or_its_result_row_with_the_reason() {
        let (app, tokens) = app_with_players().await;
        let played = played_game("replay-route-6", 8);
        let now = now_ms();
        let cases: Vec<(Finish<'_>, Option<&str>)> = vec![
            (
                Finish {
                    hash: Some("not-the-hash".to_string()),
                    ..Finish::new("m-bad-hash", &played, now)
                },
                Some("rules_changed"),
            ),
            (
                Finish {
                    hash: None,
                    result: json!({ "turns": played.last.turn + 1 }),
                    ..Finish::new("m-bad-turns", &played, now)
                },
                Some("rules_changed"),
            ),
            (
                Finish {
                    hash: None,
                    ..Finish::new("m-legacy", &played, now)
                },
                None,
            ),
            (
                Finish {
                    row: json!({ "catalogVersion": OLD_CATALOG }),
                    ..Finish::new("m-old-patch", &played, now)
                },
                Some("earlier_patch"),
            ),
        ];
        for (finish, refusal) in cases {
            let id = finish.id.to_string();
            seed_finished(&app, finish, &played).await;
            let (status, body) = get(&app, &format!("/api/replays/{id}"), &tokens.a).await;
            match refusal {
                Some(reason) => {
                    assert_eq!(status, 422, "{id}: {body}");
                    assert_eq!(error_code(&body), "replay_unavailable", "{id}");
                    assert_eq!(body["error"]["details"]["reason"], reason, "{id}");
                    assert!(body.get("steps").is_none(), "{id}: a refusal gives no step");
                }
                None => {
                    assert_eq!(status, 200, "{id}: {body}");
                    assert_eq!(body["steps"][0], step_of(&played, 0, PlayerId::P1));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The list
// ---------------------------------------------------------------------------------------------

mod r768_the_list {
    use super::*;

    #[tokio::test]
    async fn r768_lists_newest_first_with_seat_result_portraits_and_tag() {
        let (app, tokens) = app_with_players().await;
        let played = played_game("replay-route-7", 6);
        let now = now_ms();
        seed_finished(&app, Finish::new("m-plain", &played, now - 4_000), &played).await;
        seed_finished(
            &app,
            Finish {
                row: json!({ "mode": "bo1" }),
                ..Finish::new("m-bo1", &played, now - 3_000)
            },
            &played,
        )
        .await;
        seed_finished(
            &app,
            Finish {
                players: (B, A),
                row: json!({ "portraits": ["gary", "timmy"] }),
                ..Finish::new("m-portraits", &played, now - 2_000)
            },
            &played,
        )
        .await;
        seed_finished(
            &app,
            Finish {
                hash: Some("not-the-hash".to_string()),
                ..Finish::new("m-bad-hash", &played, now - 1_000)
            },
            &played,
        )
        .await;
        q!(
            app,
            matches_create(&match_row("m-live", (A, B), &played, now, &json!({})))
        );
        seed_finished(
            &app,
            Finish {
                players: (B, C),
                ..Finish::new("m-others", &played, now)
            },
            &played,
        )
        .await;

        let (status, body) = get(&app, "/api/replays", &tokens.a).await;
        assert_eq!(status, 200, "{body}");
        let ids: Vec<&str> = body["replays"]
            .as_array()
            .expect("replays")
            .iter()
            .map(|entry| entry["matchId"].as_str().unwrap_or(""))
            .collect();
        assert_eq!(ids, vec!["m-bad-hash", "m-portraits", "m-bo1", "m-plain"]);
        assert!(body["nextOffset"].is_null());

        let ended = played.last.result.as_ref().expect("a finished game");
        let result_for = |seat: &str| match (ended.winner, seat) {
            (Winner::Draw, _) => "draw",
            (Winner::P1, "p1") | (Winner::P2, "p2") => "win",
            _ => "loss",
        };
        let entry = |id: &str, ended_at: i64, seat: &str, mode: Value, portraits: [&str; 2]| {
            json!({
                "matchId": id,
                "mode": mode,
                "endedAt": ended_at,
                "seat": seat,
                "result": result_for(seat),
                "turns": played.last.turn,
                "steps": played.steps(),
                "portraits": { "p1": portraits[0], "p2": portraits[1] },
                "opponentTag": player_tag(B),
            })
        };
        let mut bad_hash = entry(
            "m-bad-hash",
            now - 1_000,
            "p1",
            Value::Null,
            ["vanilla", "vanilla"],
        );
        bad_hash["unavailable"] = json!("rules_changed");
        assert_eq!(
            body["replays"],
            json!([
                bad_hash,
                entry("m-portraits", now - 2_000, "p2", Value::Null, ["gary", "timmy"]),
                entry("m-bo1", now - 3_000, "p1", json!("bo1"), ["vanilla", "vanilla"]),
                entry("m-plain", now - 4_000, "p1", Value::Null, ["vanilla", "vanilla"]),
            ])
        );
        let text = body.to_string();
        assert!(!text.contains(&played.seed), "the seed left the server");
        assert!(!text.contains("logged-action-"), "the log left the server");

        // C held a seat in one of them.
        let (_, others) = get(&app, "/api/replays", &tokens.c).await;
        assert_eq!(others["replays"][0]["matchId"], "m-others");
        assert_eq!(others["replays"][0]["opponentTag"], json!(player_tag(B)));
    }

    #[tokio::test]
    async fn r768_pages_the_list_by_replay_list_page() {
        let (app, tokens) = app_with_players().await;
        let played = played_game("replay-route-8", 2);
        let now = now_ms();
        let count = REPLAY_LIST_PAGE + 1;
        for n in 0..count {
            let id = format!("m-page-{n:02}");
            seed_finished(
                &app,
                Finish {
                    row: json!({ "catalogVersion": OLD_CATALOG }),
                    ..Finish::new(&id, &played, now - n as i64)
                },
                &played,
            )
            .await;
        }
        let (status, first) = get(&app, "/api/replays", &tokens.a).await;
        assert_eq!(status, 200, "{first}");
        assert_eq!(first["replays"].as_array().map(Vec::len), Some(REPLAY_LIST_PAGE));
        assert_eq!(first["replays"][0]["matchId"], "m-page-00");
        assert_eq!(first["replays"][0]["unavailable"], "earlier_patch");
        assert_eq!(first["nextOffset"], json!(REPLAY_LIST_PAGE));

        let (status, second) = get(
            &app,
            &format!("/api/replays?offset={REPLAY_LIST_PAGE}"),
            &tokens.a,
        )
        .await;
        assert_eq!(status, 200, "{second}");
        assert_eq!(
            second["replays"].as_array().map(|entries| entries
                .iter()
                .map(|entry| entry["matchId"].clone())
                .collect::<Vec<_>>()),
            Some(vec![json!(format!("m-page-{REPLAY_LIST_PAGE:02}"))])
        );
        assert!(second["nextOffset"].is_null());
        assert_eq!(get(&app, "/api/replays?offset=x", &tokens.a).await.0, 400);
    }
}

// ---------------------------------------------------------------------------------------------
// Bounded cost
// ---------------------------------------------------------------------------------------------

mod r768_bounded_cost {
    use super::*;

    #[tokio::test]
    async fn r768_the_replay_routes_answer_429_past_their_allowance() {
        let (app, tokens) = app_with_players().await;
        for n in 0..REPLAY_REQUESTS_PER_MINUTE {
            let (status, body) = get(&app, "/api/replays", &tokens.a).await;
            assert_eq!(status, 200, "request {n}: {body}");
        }
        // Both routes draw on the one allowance.
        let (status, headers, body) = call(
            &app,
            "GET",
            "/api/replays/no-such-match",
            Some(&tokens.a),
            Value::Null,
        )
        .await;
        assert_eq!(status, 429, "{body}");
        assert_eq!(error_code(&body), "rate_limited");
        assert!(headers.contains_key("retry-after"));
        // Another account's allowance is its own.
        assert_eq!(get(&app, "/api/replays", &tokens.b).await.0, 200);
    }

    fn refused() -> Arc<ReplayOpen> {
        Arc::new(ReplayOpen::Refused {
            reason: ReplayRefusal::EarlierPatch,
        })
    }

    #[test]
    fn r768_the_cache_holds_at_most_replay_cache_matches_for_at_most_its_ttl() {
        let cache = ReplayCache::default();
        let extra = 3;
        for n in 0..REPLAY_CACHE_MATCHES + extra {
            cache.put(&format!("m-{n}"), refused(), 0);
            assert!(cache.held(0) <= REPLAY_CACHE_MATCHES);
        }
        assert_eq!(cache.held(0), REPLAY_CACHE_MATCHES);
        // The ones put first went first.
        for n in 0..extra {
            assert!(cache.get(&format!("m-{n}"), 0).is_none(), "m-{n}");
        }
        // A replay just read is kept past the next eviction; the one used longest ago goes.
        let oldest = format!("m-{extra}");
        assert!(cache.get(&oldest, 0).is_some());
        cache.put("m-new", refused(), 0);
        assert!(cache.get(&oldest, 0).is_some());
        assert!(cache.get(&format!("m-{}", extra + 1), 0).is_none());
        assert_eq!(cache.held(0), REPLAY_CACHE_MATCHES);

        // Each is kept for less than its TTL from when it was opened.
        let ttl_ms = REPLAY_CACHE_TTL_SECONDS * 1000;
        assert!(cache.get("m-new", ttl_ms - 1).is_some());
        assert_eq!(cache.held(ttl_ms), 0);
        assert!(cache.get("m-new", ttl_ms).is_none());
    }
}
