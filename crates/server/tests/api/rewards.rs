//! A finished ladder game pays Jlore and XP (SPEC §14.8, #1302): through the one reward hook,
//! in the same transaction as its results row, onto every active set track.
//!
//! Won and lost games pay exactly `JLORE_LADDER_WIN`/`JLORE_LADDER_LOSS`; XP counts the log's
//! actions plus the win bonus up to `XP_GAME_CAP`; the second track unlocks at
//! `SET_TRACK_UNLOCK_LEVEL`; and draws, concedes, reaper-resolved games, the moves that count and
//! a track's last level pay per R1302000–R1302005, whose proving tests live here. Series games,
//! room games, voided matches and bot sides pay nothing. Every level position is read through the
//! constants and the level curve, so the tests hold whatever values #1333 sets.
//!
//! The engine is driven for real where the outcome matters (a win, a mutual-lethal draw, a
//! concede); the action log the hook counts is appended as rows, which need no engine legality.

use std::sync::Arc;

use jackioh_engine::wire::{SHIPPED_SETS, SetName, newest_shipped_set};
use jackioh_engine::{Action, GameOverReason, PlayerId, Winner};
use serde::Serialize;
use serde_json::{Value, json};

use jackioh_server::actor::clock::initial_clocks;
use jackioh_server::actor::contracts::{RecordResultInput, TerminalOutcome};
use jackioh_server::api::results::{ceiling_draw_input, record_ceiling_draw, record_result};
use jackioh_server::app::{App, now_ms};
use jackioh_server::config::{
    JLORE_LADDER_LOSS, JLORE_LADDER_WIN, MATCH_CEILING_MINUTES, SET_TRACK_FIRST_LEVEL, SET_TRACK_LEVELS,
    SET_TRACK_UNLOCK_LEVEL, XP_GAME_CAP,
};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{Db, MatchActionRow, MatchRow, MatchSeat, ResultRow, StoreError};
use jackioh_server::economy::math::total_xp_for_level;
use jackioh_server::economy::rewards::{GameMode, track_level};
use jackioh_server::economy::store::{
    EconomyLedger, EconomyReason, InventoryEntry, InventoryItem, JloreReason, SetTrack, TrackGrant,
};
use jackioh_server::economy::track_rewards::reward_for_level;

use crate::support::deps::{add_user, call, test_app as built_app};
use crate::support::engine::{fake_deck, install_test_cards};

const MINUTE: i64 = 60 * 1000;
/// A minute past the ceiling (R79, R389), whatever it is.
const PAST_CEILING_MS: i64 = (MATCH_CEILING_MINUTES + 1) * MINUTE;
const MATCH_ID: &str = "match-1";
const MATCH_2: &str = "match-2";
const MATCH_3: &str = "match-3";
const A: &str = "profile-a";
const B: &str = "profile-b";

/// A ladder game's XP, the hook's own math.
fn game_xp(actions: i64, won: bool) -> i64 {
    jackioh_server::economy::rewards::game_xp(GameMode::Ladder, actions, won)
}

/// The test app with the economy on (§14.12): `ECONOMY_LIVE` is off in the build, so every test
/// of a payout turns it on for its own app.
async fn test_app() -> Arc<App> {
    let mut app = built_app().await;
    Arc::get_mut(&mut app)
        .expect("the test app has no second handle yet")
        .economy_live = true;
    app
}

/// Every store call in this file: one transaction, committed when the call answers.
macro_rules! store {
    ($app:expr, |$t:ident| $call:expr) => {{
        let mut $t = $app.db.begin(None).await.expect("a store transaction begins");
        let out = $call
            .await
            .expect(concat!("the store answers ", stringify!($call)));
        $t.commit().await.expect("the store transaction commits");
        out
    }};
}

/// The JSON a row or any port value serialises to.
fn j<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("the value serialises")
}

/// The fake store behind `test_app()`.
fn fake(app: &App) -> &Arc<tokio::sync::Mutex<FakeData>> {
    match &app.db {
        Db::Fake(data) => data,
        _ => panic!("support::deps::test_app() runs on the fake store"),
    }
}

/// `test_app()` holds the E2E fixtures (R144), so they go first and every row a test reads is
/// one it wrote.
async fn fresh_store(app: &App) {
    fake(app).lock().await.reset();
}

/// A profile written without the API (status `active` unless set).
async fn seed_profile(app: &App, input: Value) {
    let _ = fake(app).lock().await.seed_profile(input);
}

/// One read of the fake's raw tables, as JSON.
async fn table(app: &App, read: impl Fn(&FakeData) -> Value) -> Value {
    let data = fake(app).lock().await;
    read(&data)
}

/// The catalog version every match here is created at.
fn catalog_version() -> &'static str {
    jackioh_cards::catalog_version()
}

fn seat_of(profile_id: Value, player: &str, deck: Value) -> MatchSeat {
    serde_json::from_value(json!({ "profileId": profile_id, "player": player, "deck": deck }))
        .expect("a MatchSeat")
}

/// The two seats every scenario plays, on filler decks: the log the hook counts is appended as
/// rows, and outcomes the engine must really emit go through `play`, so the decks never decide.
fn seats() -> (MatchSeat, MatchSeat) {
    (
        seat_of(json!(A), "p1", json!(fake_deck(&[]))),
        seat_of(json!(B), "p2", json!(fake_deck(&[]))),
    )
}

/// A ranked match between A and B, unless `ranked: false` makes it a room's (R604): the row
/// alone, as a crashed actor leaves it, since `record_result` never needs a live actor.
async fn scenario_with(match_id: &str, ranked: bool, past_ceiling: bool) -> Arc<App> {
    let app = test_app().await;
    fresh_store(&app).await;
    seed_profile(&app, json!({ "id": A, "rating": 1000.0, "inMatchId": match_id })).await;
    seed_profile(&app, json!({ "id": B, "rating": 1000.0, "inMatchId": match_id })).await;
    create_match(&app, match_id, ranked, past_ceiling).await;
    app
}

/// The row `scenario_with` writes, for a second or third game on the same app.
async fn create_match(app: &Arc<App>, match_id: &str, ranked: bool, past_ceiling: bool) {
    let (first, second) = seats();
    let started_at = now_ms() - if past_ceiling { PAST_CEILING_MS } else { 0 };
    let row: MatchRow = serde_json::from_value(json!({
        "id": match_id,
        "seed": "seed-1",
        "players": [A, B],
        "decks": [j(&first)["deck"], j(&second)["deck"]],
        "catalogVersion": catalog_version(),
        "ranked": ranked,
        "status": "live",
        "createdAt": started_at,
        "finishedAt": null,
        "clocks": j(&initial_clocks(started_at)),
    }))
    .expect("a MatchRow");
    store!(app, |t| t.matches_create(&row));
}

async fn scenario() -> Arc<App> {
    scenario_with(MATCH_ID, true, false).await
}

/// Records `outcome` for `match_id` with the scenario's seats.
async fn record_on(app: &Arc<App>, match_id: &str, outcome: TerminalOutcome) -> ResultRow {
    record_result(
        app,
        RecordResultInput {
            match_id: match_id.to_owned(),
            seats: seats(),
            outcome,
            turns: 3,
            at: now_ms(),
            last_boards: None,
            final_hash: None,
        },
    )
    .await
    .expect("the result is written")
}

/// Runs the engine with the scripted cards to a terminal state and hands back the outcome the
/// actor would pass to `record_result`, so the reasons under test are the ones `reduce` really
/// emits. A scenario on turn 1 with both scripted cards in p1's hand; both seats switch off
/// R345's automatic end of turn first, so a turn ends only when an input ends it.
fn play(inputs: &[Value]) -> TerminalOutcome {
    let s = jackioh_engine::testkit::scenario(json!({
        "seed": "seed-1",
        "turn": 1,
        "p1": { "hand": ["test-lethal", "test-mutual-lethal"], "mana": 10 },
        "p2": {},
    }));
    let mut state = s.state().clone();
    let mut actions = vec![
        json!({ "type": "setAutoEndTurn", "enabled": false, "playerId": "p1" }),
        json!({ "type": "setAutoEndTurn", "enabled": false, "playerId": "p2" }),
    ];
    actions.extend(inputs.iter().cloned());
    for (n, mut input) in actions.into_iter().enumerate() {
        if let Some(card) = input.get("card").and_then(Value::as_str).map(str::to_owned) {
            let instance_id = s.card(&card).id.clone();
            if let Some(fields) = input.as_object_mut() {
                fields.remove("card");
                fields.insert("instanceId".to_owned(), json!(instance_id));
            }
        }
        input["nonce"] = json!(format!("nonce-{}", n + 1));
        let kind = input["type"].clone();
        let action: Action = serde_json::from_value(input).expect("an Action");
        let result = jackioh_engine::reduce(&state, &action);
        if let Some(error) = result.error {
            panic!("{kind}: {error}");
        }
        state = result.state;
    }
    let Some(result) = state.result else {
        panic!("the scripted game did not end");
    };
    TerminalOutcome {
        winner: result.winner,
        reason: result.reason,
    }
}

/// The log rows the hook counts: `inputs` as the rows the actor writes, in order. Every input
/// carries its `playerId`; each gets the nonce `log-<seq>`.
async fn append_log_to(app: &App, match_id: &str, inputs: &[Value]) {
    let rows: Vec<MatchActionRow> = inputs
        .iter()
        .enumerate()
        .map(|(n, input)| {
            let seq = i64::try_from(n).expect("a count") + 1;
            let mut input = input.clone();
            input["nonce"] = json!(format!("log-{seq}"));
            MatchActionRow {
                match_id: match_id.to_owned(),
                seq,
                action: serde_json::from_value(input).expect("an Action"),
                at: now_ms(),
            }
        })
        .collect();
    store!(app, |t| t.matches_append_actions(&rows));
}

/// `n` log rows for `player`, of the cheapest kind: an ended turn each.
fn actions_of(player: &str, n: usize) -> Vec<Value> {
    (0..n)
        .map(|_| json!({ "type": "endTurn", "playerId": player }))
        .collect()
}

async fn jlore_of(app: &App, profile: &str) -> i64 {
    store!(app, |t| t.economy_jlore_get(profile))
}

async fn tracks_of(app: &App, profile: &str) -> Vec<SetTrack> {
    store!(app, |t| t.economy_tracks_get(profile))
}

async fn ledger_of(app: &App, profile: &str) -> EconomyLedger {
    store!(app, |t| t.economy_ledger_get(profile))
}

async fn inventory_of(app: &App, profile: &str) -> Vec<InventoryEntry> {
    store!(app, |t| t.economy_inventory_get(profile))
}

/// What the finished games paid this profile for the games themselves (reason `game`), apart from
/// the track levels those games also reached.
async fn game_jlore_paid(app: &App, profile: &str) -> i64 {
    ledger_of(app, profile)
        .await
        .jlore
        .iter()
        .filter(|grant| grant.reason == JloreReason::Game)
        .map(|grant| grant.delta)
        .sum()
}

fn newest() -> SetName {
    newest_shipped_set()
}

/// The set that unlocks first: the nearest older shipped set to the newest.
fn second_newest() -> SetName {
    SHIPPED_SETS[SHIPPED_SETS.len() - 2]
}

fn track_of(tracks: &[SetTrack], set: SetName) -> Option<SetTrack> {
    tracks.iter().find(|track| track.set_id == set.as_str()).cloned()
}

/// A track row written without the hook (a `dev` setup grant, §14.10): the hook's tests start
/// the newest track where they need it without paying anything.
async fn grant_track(app: &App, profile: &str, set: SetName, xp: i64, level: i64, claimed: i64) {
    let grant = TrackGrant {
        profile_id: profile.to_string(),
        set_id: set.as_str().to_string(),
        xp,
        levels: level - SET_TRACK_FIRST_LEVEL,
        rewards: claimed,
        reason: EconomyReason::Dev,
        reference: None,
        dev: true,
        at: now_ms(),
    };
    store!(app, |t| t.economy_tracks_grant(&grant));
}

// A won and a lost ladder game pay exactly the configured Jlore; XP counts the log's actions
// plus the win bonus, up to the cap.

mod ladder_payout {
    use super::*;

    #[tokio::test]
    async fn won_game_pays_win_jlore_and_actions_plus_bonus_xp() {
        install_test_cards();
        let app = scenario().await;
        let mut log = actions_of("p1", 30);
        log.extend(actions_of("p2", 5));
        append_log_to(&app, MATCH_ID, &log).await;
        // The scripted engine really kills p1's enemy hero: A wins by hero-death.
        let outcome = play(&[json!({ "type": "play", "card": "test-lethal", "playerId": "p1" })]);
        assert_eq!(outcome.winner, Winner::P1);
        let row = j(&record_on(&app, MATCH_ID, outcome).await);
        assert_eq!(row["winnerProfileId"], json!(A));

        // The games pay exactly the configured Jlore; the wallet also holds level 1's track
        // Jlore, which the level tests pin down.
        assert_eq!(game_jlore_paid(&app, A).await, i64::from(JLORE_LADDER_WIN));
        assert_eq!(game_jlore_paid(&app, B).await, i64::from(JLORE_LADDER_LOSS));
        assert_eq!(
            track_of(&tracks_of(&app, A).await, newest())
                .expect("A's newest track")
                .xp,
            game_xp(30, true)
        );
        assert_eq!(
            track_of(&tracks_of(&app, B).await, newest())
                .expect("B's newest track")
                .xp,
            game_xp(5, false)
        );
    }

    #[tokio::test]
    async fn lost_game_pays_loss_jlore_without_bonus() {
        install_test_cards();
        let app = scenario().await;
        append_log_to(&app, MATCH_ID, &actions_of("p1", 12)).await;
        // p1 concedes: B wins, A loses.
        let outcome = play(&[json!({ "type": "concede", "playerId": "p1" })]);
        assert_eq!(outcome.winner, Winner::P2);
        record_on(&app, MATCH_ID, outcome).await;

        assert_eq!(game_jlore_paid(&app, A).await, i64::from(JLORE_LADDER_LOSS));
        assert_eq!(game_jlore_paid(&app, B).await, i64::from(JLORE_LADDER_WIN));
        assert_eq!(
            track_of(&tracks_of(&app, A).await, newest())
                .expect("A's newest track")
                .xp,
            game_xp(12, false)
        );
    }

    #[tokio::test]
    async fn xp_stops_at_the_cap() {
        install_test_cards();
        let app = scenario().await;
        append_log_to(&app, MATCH_ID, &actions_of("p1", 500)).await;
        let outcome = play(&[json!({ "type": "play", "card": "test-lethal", "playerId": "p1" })]);
        record_on(&app, MATCH_ID, outcome).await;

        assert_eq!(game_xp(500, true), i64::from(XP_GAME_CAP));
        assert_eq!(
            track_of(&tracks_of(&app, A).await, newest())
                .expect("A's newest track")
                .xp,
            i64::from(XP_GAME_CAP)
        );
    }

    #[tokio::test]
    async fn xp_reaches_every_active_track_in_full() {
        install_test_cards();
        let app = scenario().await;
        // An unlocked older track: a row with some XP, below the unlock level so nothing new
        // unlocks.
        grant_track(&app, A, second_newest(), 10, 1, 0).await;
        append_log_to(&app, MATCH_ID, &actions_of("p1", 30)).await;
        let outcome = play(&[json!({ "type": "concede", "playerId": "p2" })]);
        record_on(&app, MATCH_ID, outcome).await;

        let tracks = tracks_of(&app, A).await;
        assert_eq!(
            track_of(&tracks, newest()).expect("the newest track").xp,
            game_xp(30, true)
        );
        assert_eq!(
            track_of(&tracks, second_newest()).expect("the older track").xp,
            10 + game_xp(30, true)
        );
        assert_eq!(tracks.len(), 2, "no third track unlocks: {tracks:?}");
    }
}

// The second track unlocks at the configured level of the newest; below it, nothing unlocks.

mod track_unlock {
    use super::*;

    fn unlock_level() -> u32 {
        SET_TRACK_UNLOCK_LEVEL
    }

    #[tokio::test]
    async fn newest_at_unlock_level_unlocks_the_second_track() {
        install_test_cards();
        let app = scenario().await;
        grant_track(
            &app,
            A,
            newest(),
            total_xp_for_level(unlock_level()),
            i64::from(unlock_level()),
            i64::from(unlock_level()),
        )
        .await;
        append_log_to(&app, MATCH_ID, &actions_of("p1", 1)).await;
        let outcome = play(&[json!({ "type": "concede", "playerId": "p2" })]);
        record_on(&app, MATCH_ID, outcome).await;

        let tracks = tracks_of(&app, A).await;
        let unlocked = track_of(&tracks, second_newest()).expect("the second track unlocks");
        assert_eq!(unlocked.xp, game_xp(1, true));
        assert_eq!(unlocked.level, track_level(unlocked.xp));
        assert_eq!(
            unlocked.rewards_claimed, unlocked.level,
            "its levels reached have paid"
        );
    }

    #[tokio::test]
    async fn below_unlock_level_unlocks_nothing() {
        install_test_cards();
        let app = scenario().await;
        let below = unlock_level() - 1;
        // Premise: one small loss cannot cross the last step to the unlock level.
        let one_game = game_xp(1, false);
        assert!(
            total_xp_for_level(unlock_level()) - total_xp_for_level(below) > one_game,
            "one action's XP must not cross into the unlock level"
        );
        grant_track(
            &app,
            A,
            newest(),
            total_xp_for_level(below),
            i64::from(below),
            i64::from(below),
        )
        .await;
        append_log_to(&app, MATCH_ID, &actions_of("p1", 1)).await;
        let outcome = play(&[json!({ "type": "concede", "playerId": "p2" })]);
        record_on(&app, MATCH_ID, outcome).await;

        let tracks = tracks_of(&app, A).await;
        assert_eq!(tracks.len(), 1, "no older track unlocks: {tracks:?}");
    }
}

// R1302000–R1302005: what a draw, a concede, a reaper-resolved game, an unlocking backlog, the
// server's own rows and a track's last level pay.

mod rulings {
    use super::*;

    #[tokio::test]
    async fn r1302000_draw_pays_loss_without_bonus() {
        install_test_cards();
        let app = scenario().await;
        let mut log = actions_of("p1", 20);
        log.extend(actions_of("p2", 20));
        append_log_to(&app, MATCH_ID, &log).await;
        // Both heroes die in the same check: a draw by mutual lethal.
        let outcome = play(&[json!({ "type": "play", "card": "test-mutual-lethal", "playerId": "p1" })]);
        assert_eq!(outcome.winner, Winner::Draw);
        let row = j(&record_on(&app, MATCH_ID, outcome).await);
        assert!(row["winnerProfileId"].is_null());

        for profile in [A, B] {
            assert_eq!(
                game_jlore_paid(&app, profile).await,
                i64::from(JLORE_LADDER_LOSS),
                "{profile} draws, so both earn the loss rate"
            );
            assert_eq!(
                track_of(&tracks_of(&app, profile).await, newest())
                    .expect("the newest track")
                    .xp,
                game_xp(20, false),
                "{profile}'s XP counts actions with no win bonus"
            );
        }
    }

    #[tokio::test]
    async fn r1302001_concede_pays_recorded_result() {
        install_test_cards();
        let app = scenario().await;
        let mut log = actions_of("p1", 7);
        log.extend(actions_of("p2", 9));
        append_log_to(&app, MATCH_ID, &log).await;
        let outcome = play(&[json!({ "type": "concede", "playerId": "p2" })]);
        assert_eq!(outcome.winner, Winner::P1);
        record_on(&app, MATCH_ID, outcome).await;

        // The winner earns the win rate with the bonus; the conceder the loss rate without it.
        assert_eq!(game_jlore_paid(&app, A).await, i64::from(JLORE_LADDER_WIN));
        assert_eq!(game_jlore_paid(&app, B).await, i64::from(JLORE_LADDER_LOSS));
        assert_eq!(
            track_of(&tracks_of(&app, A).await, newest())
                .expect("A's newest track")
                .xp,
            game_xp(7, true)
        );
        assert_eq!(
            track_of(&tracks_of(&app, B).await, newest())
                .expect("B's newest track")
                .xp,
            game_xp(9, false)
        );
    }

    #[tokio::test]
    async fn r1302002_reaper_ceiling_draw_pays_loss() {
        install_test_cards();
        let app = scenario_with(MATCH_ID, true, true).await;
        let mut log = actions_of("p1", 11);
        log.extend(actions_of("p2", 4));
        append_log_to(&app, MATCH_ID, &log).await;
        let match_row: MatchRow = store!(app, |t| t.matches_get(MATCH_ID)).expect("the past-ceiling match");
        let row = j(
            &record_ceiling_draw(&app, ceiling_draw_input(&match_row, now_ms()))
                .await
                .expect("the ceiling draw is written"),
        );
        assert_eq!(row["reason"], "match-ceiling");
        assert_eq!(row["turns"], json!(0));

        // Unrated, yet paid: each human side earns the loss rate over the log's actions.
        for (profile, actions) in [(A, 11), (B, 4)] {
            assert_eq!(game_jlore_paid(&app, profile).await, i64::from(JLORE_LADDER_LOSS));
            assert_eq!(
                track_of(&tracks_of(&app, profile).await, newest())
                    .expect("the newest track")
                    .xp,
                game_xp(actions, false)
            );
        }
    }

    #[tokio::test]
    async fn r1302002_reaper_recovered_real_result_pays_as_that_result() {
        install_test_cards();
        // What the reaper files after folding a log that already ends (R833000): the actor's own
        // write, rated, through `record_result`.
        let app = scenario_with(MATCH_ID, true, true).await;
        append_log_to(&app, MATCH_ID, &actions_of("p1", 6)).await;
        let outcome = play(&[json!({ "type": "play", "card": "test-lethal", "playerId": "p1" })]);
        record_on(&app, MATCH_ID, outcome).await;

        assert_eq!(game_jlore_paid(&app, A).await, i64::from(JLORE_LADDER_WIN));
        assert_eq!(
            track_of(&tracks_of(&app, A).await, newest())
                .expect("A's newest track")
                .xp,
            game_xp(6, true)
        );
    }

    #[tokio::test]
    async fn r1302003_nearest_older_track_unlocks_first() {
        install_test_cards();
        let app = scenario().await;
        grant_track(
            &app,
            A,
            newest(),
            total_xp_for_level(SET_TRACK_UNLOCK_LEVEL),
            i64::from(SET_TRACK_UNLOCK_LEVEL),
            i64::from(SET_TRACK_UNLOCK_LEVEL),
        )
        .await;

        // Game one unlocks at most one track: the nearest older one.
        append_log_to(&app, MATCH_ID, &actions_of("p1", 3)).await;
        let outcome = play(&[json!({ "type": "concede", "playerId": "p2" })]);
        record_on(&app, MATCH_ID, outcome).await;
        let tracks = tracks_of(&app, A).await;
        let unlocked = track_of(&tracks, second_newest()).expect("the nearest older track");
        assert_eq!(unlocked.xp, game_xp(3, true));
        assert!(
            track_of(&tracks, SHIPPED_SETS[SHIPPED_SETS.len() - 3]).is_none(),
            "only one track unlocks per payout: {tracks:?}"
        );

        // Game two unlocks the next nearest older one.
        create_match(&app, MATCH_2, true, false).await;
        append_log_to(&app, MATCH_2, &actions_of("p1", 3)).await;
        let outcome = play(&[json!({ "type": "concede", "playerId": "p2" })]);
        record_on(&app, MATCH_2, outcome).await;
        let tracks = tracks_of(&app, A).await;
        let next = track_of(&tracks, SHIPPED_SETS[SHIPPED_SETS.len() - 3])
            .expect("the next older track unlocks second");
        assert_eq!(next.xp, game_xp(3, true));
    }

    #[tokio::test]
    async fn r1302004_server_rows_and_the_auto_end_setting_earn_no_xp() {
        install_test_cards();
        let app = scenario().await;
        // A takes four moves; around them the log holds A's automatic-end setting and the rows
        // the server writes itself. B's every row is the server's: a clock that ran out each turn.
        let mut log = actions_of("p1", 4);
        log.extend([
            json!({ "type": "setAutoEndTurn", "enabled": false, "playerId": "p1" }),
            json!({ "type": "disconnectExpired", "player": "p1", "playerId": "p1" }),
            json!({ "type": "ceilingReached", "playerId": "p1" }),
        ]);
        log.extend((0..5).map(|_| json!({ "type": "timeout", "playerId": "p2" })));
        append_log_to(&app, MATCH_ID, &log).await;
        let outcome = play(&[json!({ "type": "concede", "playerId": "p2" })]);
        record_on(&app, MATCH_ID, outcome).await;

        assert_eq!(
            track_of(&tracks_of(&app, A).await, newest())
                .expect("A's newest track")
                .xp,
            game_xp(4, true),
            "only A's four moves count"
        );
        // B took no move and lost: its game pays the loss rate and no XP at all.
        assert_eq!(game_xp(0, false), 0);
        assert_eq!(game_jlore_paid(&app, B).await, i64::from(JLORE_LADDER_LOSS));
        assert!(tracks_of(&app, B).await.is_empty(), "B earns no XP");
    }

    #[tokio::test]
    async fn r1302005_a_track_pays_no_level_past_its_last() {
        install_test_cards();
        let app = scenario().await;
        let last = i64::from(SET_TRACK_LEVELS);
        let newest_packs = |inventory: Vec<InventoryEntry>| -> i64 {
            inventory
                .iter()
                .filter(|entry| entry.item == InventoryItem::SetPack && entry.key == newest().as_str())
                .map(|entry| entry.count)
                .sum()
        };
        // A's newest track one XP short of its last level, every level before it paid.
        let start = total_xp_for_level(SET_TRACK_LEVELS) - 1;
        grant_track(&app, A, newest(), start, last - 1, last - 1).await;

        // Game one reaches the last level, which pays once.
        append_log_to(&app, MATCH_ID, &actions_of("p1", 5)).await;
        let outcome = play(&[json!({ "type": "concede", "playerId": "p2" })]);
        record_on(&app, MATCH_ID, outcome).await;
        let track = track_of(&tracks_of(&app, A).await, newest()).expect("A's newest track");
        assert_eq!((track.level, track.rewards_claimed), (last, last));
        let packs = newest_packs(inventory_of(&app, A).await);
        assert_eq!(packs, reward_for_level(last).set_packs);

        // A setup grant carries the track one XP short of where a level past the last would be.
        let short = total_xp_for_level(SET_TRACK_LEVELS + 1) - 1 - track.xp;
        assert!(short > 0, "game one stays short of a level past the last");
        grant_track(&app, A, newest(), short, SET_TRACK_FIRST_LEVEL, 0).await;
        let before = track.xp + short;

        // Game two's XP lands in full, yet reaches no level and pays nothing.
        create_match(&app, MATCH_2, true, false).await;
        append_log_to(&app, MATCH_2, &actions_of("p1", 5)).await;
        let outcome = play(&[json!({ "type": "concede", "playerId": "p2" })]);
        record_on(&app, MATCH_2, outcome).await;
        let track = track_of(&tracks_of(&app, A).await, newest()).expect("A's newest track");
        assert_eq!(track.xp, before + game_xp(5, true));
        assert_eq!((track.level, track.rewards_claimed), (last, last));
        assert_eq!(newest_packs(inventory_of(&app, A).await), packs);
        let grants: Vec<_> = ledger_of(&app, A)
            .await
            .tracks
            .into_iter()
            .filter(|grant| grant.set_id == newest().as_str() && grant.reference.as_deref() == Some(MATCH_2))
            .map(|grant| (grant.xp, grant.levels, grant.rewards))
            .collect();
        assert_eq!(grants, vec![(game_xp(5, true), 0, 0)]);
    }
}

// A level's rewards are granted once: crossing one level over two games pays it a single time,
// and a repeated write pays nothing twice.

mod level_rewards_once {
    use super::*;

    fn track_jlore_total(ledger: &EconomyLedger) -> i64 {
        ledger
            .jlore
            .iter()
            .filter(|grant| grant.reason == JloreReason::Track)
            .map(|grant| grant.delta)
            .sum()
    }

    /// The inventory's count of `item`, summed over its keys.
    fn held(inventory: &[InventoryEntry], item: InventoryItem) -> i64 {
        inventory
            .iter()
            .filter(|entry| entry.item == item)
            .map(|entry| entry.count)
            .sum()
    }

    /// What the track rewards paid so far: Jlore, set packs and card tokens.
    async fn track_paid(app: &App, profile: &str) -> (i64, i64, i64) {
        let inventory = inventory_of(app, profile).await;
        (
            track_jlore_total(&ledger_of(app, profile).await),
            held(&inventory, InventoryItem::SetPack),
            held(&inventory, InventoryItem::CardToken),
        )
    }

    /// One small ladder loss for A on `match_id`: one counted action, conceded.
    async fn small_loss(app: &Arc<App>, match_id: &str) {
        append_log_to(app, match_id, &actions_of("p1", 1)).await;
        let outcome = play(&[json!({ "type": "concede", "playerId": "p1" })]);
        record_on(app, match_id, outcome).await;
    }

    #[tokio::test]
    async fn each_level_pays_once_and_a_repeated_write_pays_nothing() {
        install_test_cards();
        let app = scenario().await;
        // A's newest track sits two small losses below its next level, every level it holds paid,
        // and short of the unlock level, so no older track's rewards join the ledger. Every
        // position is read through the constants, so the premises hold whatever #1333 sets.
        let one_game = game_xp(1, false);
        let level = i64::from(SET_TRACK_UNLOCK_LEVEL) - 2;
        let next = u32::try_from(level + 1).expect("a level");
        let start = total_xp_for_level(next) - 2 * one_game;
        assert!(
            one_game > 0 && level >= SET_TRACK_FIRST_LEVEL,
            "a level under the unlock level"
        );
        assert_eq!(track_level(start), level, "the start stands on the level below");
        assert!(
            total_xp_for_level(next + 1) > start + 3 * one_game,
            "three small losses stay short of the level after next"
        );
        let reward = reward_for_level(i64::from(next));
        let paid = (reward.jlore, reward.set_packs, reward.card_tokens);
        assert_ne!(paid, (0, 0, 0), "the level crossed pays something");
        grant_track(&app, A, newest(), start, level, level).await;

        // Game one stays on the level below: nothing pays.
        small_loss(&app, MATCH_ID).await;
        let track = track_of(&tracks_of(&app, A).await, newest()).expect("A's newest track");
        assert_eq!((track.level, track.rewards_claimed), (level, level));
        assert_eq!(track_paid(&app, A).await, (0, 0, 0));

        // Game two crosses into the next level: it pays, once.
        create_match(&app, MATCH_2, true, false).await;
        small_loss(&app, MATCH_2).await;
        let track = track_of(&tracks_of(&app, A).await, newest()).expect("A's newest track");
        assert_eq!((track.level, track.rewards_claimed), (level + 1, level + 1));
        assert_eq!(track_paid(&app, A).await, paid);

        // Game three stays on it: no level pays again.
        create_match(&app, MATCH_3, true, false).await;
        small_loss(&app, MATCH_3).await;
        let track = track_of(&tracks_of(&app, A).await, newest()).expect("A's newest track");
        assert_eq!(track.xp, start + 3 * one_game);
        assert_eq!((track.level, track.rewards_claimed), (level + 1, level + 1));
        assert_eq!(track_paid(&app, A).await, paid);

        // A repeated write of the same match pays nothing twice.
        let before = j(&ledger_of(&app, A).await);
        let outcome = play(&[json!({ "type": "concede", "playerId": "p1" })]);
        record_on(&app, MATCH_3, outcome).await;
        assert_eq!(j(&ledger_of(&app, A).await), before);
        assert_eq!(game_jlore_paid(&app, A).await, 3 * i64::from(JLORE_LADDER_LOSS));
    }
}

// Nothing is paid for series games, room challenges, voided matches or bot sides.

mod no_payout {
    use super::*;
    use jackioh_server::actor::contracts::VoidMatchInput;
    use jackioh_server::api::results::void_match;
    use jackioh_server::api::series::{ensure_series_game, start_series};
    use jackioh_server::api::series_rules::{NewSeriesInput, pick_deck};
    use jackioh_server::db::store::SeriesRow;

    /// A slot in whatever integer type the series rules take.
    fn int<T: TryFrom<i64>>(n: i64) -> T
    where
        T::Error: std::fmt::Debug,
    {
        T::try_from(n).expect("a slot the rules' signature can hold")
    }

    fn trio(owner: &str) -> Value {
        let deck = |slot: i64| json!({ "name": format!("{owner} {slot}"), "cards": fake_deck(&[]) });
        json!({ "name": owner, "decks": [deck(0), deck(1), deck(2)] })
    }

    async fn match_row_of(app: &App, id: &str) -> Value {
        j(&store!(app, |t| t.matches_get(id)))
    }

    /// The seats of a match as its row records them: index 0 is p1.
    async fn started_seats(app: &App, match_id: &str) -> (MatchSeat, MatchSeat) {
        let row = match_row_of(app, match_id).await;
        (
            seat_of(row["players"][0].clone(), "p1", row["decks"][0].clone()),
            seat_of(row["players"][1].clone(), "p2", row["decks"][1].clone()),
        )
    }

    /// A ranked Conquest series whose first game (`MATCH_ID`) is being played.
    async fn series_game() -> Arc<App> {
        let app = test_app().await;
        fresh_store(&app).await;
        seed_profile(&app, json!({ "id": A, "rating": 1200.0 })).await;
        seed_profile(&app, json!({ "id": B, "rating": 1000.0 })).await;
        let input: NewSeriesInput = serde_json::from_value(json!({
            "seriesId": "series-1",
            "firstMatchId": MATCH_ID,
            "sides": [
                { "profileId": A, "trio": trio("a") },
                { "profileId": B, "trio": trio("b") },
            ],
            "seedBase": "seed",
            "catalogVersion": catalog_version(),
            "ranked": true,
        }))
        .expect("a NewSeriesInput");
        let mut tx = app.db.begin(None).await.expect("store.tx");
        start_series(&app, input, &mut tx)
            .await
            .expect("the series starts");
        tx.commit().await.expect("the series commits");
        let mut row: SeriesRow = store!(app, |t| t.series_get("series-1")).expect("the series row");
        for (index, seat) in [PlayerId::P1, PlayerId::P2].into_iter().enumerate() {
            let next = pick_deck(&row, seat, int(index as i64), now_ms(), None).expect("the pick");
            let _ = store!(app, |t| t.series_update(&next));
            row = next;
        }
        ensure_series_game(&app, &row).await.expect("the game starts");
        app
    }

    async fn assert_paid_nothing(app: &App) {
        for profile in [A, B] {
            assert_eq!(jlore_of(app, profile).await, 0, "{profile} earns no Jlore");
            assert!(tracks_of(app, profile).await.is_empty(), "{profile} earns no XP");
            assert_eq!(
                j(&ledger_of(app, profile).await),
                json!({
                    "jlore": [], "inventory": [], "counters": [],
                    "tracks": [], "quests": [], "cosmetics": [],
                }),
                "{profile} holds no ledger row"
            );
        }
    }

    #[tokio::test]
    async fn series_game_pays_nothing() {
        install_test_cards();
        let app = series_game().await;
        let seats = started_seats(&app, MATCH_ID).await;
        record_result(
            &app,
            RecordResultInput {
                match_id: MATCH_ID.to_owned(),
                seats,
                outcome: TerminalOutcome {
                    winner: Winner::P1,
                    reason: GameOverReason::HeroDeath,
                },
                turns: 3,
                at: now_ms(),
                last_boards: None,
                final_hash: None,
            },
        )
        .await
        .expect("the result is written");
        assert_paid_nothing(&app).await;
        assert_eq!(
            table(&app, |data| j(&data.tables.results))
                .await
                .as_array()
                .map(Vec::len),
            Some(1)
        );
    }

    #[tokio::test]
    async fn room_game_pays_nothing() {
        install_test_cards();
        let app = scenario_with(MATCH_ID, false, false).await;
        append_log_to(&app, MATCH_ID, &actions_of("p1", 30)).await;
        let outcome = play(&[json!({ "type": "play", "card": "test-lethal", "playerId": "p1" })]);
        record_on(&app, MATCH_ID, outcome).await;
        assert_paid_nothing(&app).await;
        assert_eq!(
            table(&app, |data| j(&data.tables.results))
                .await
                .as_array()
                .map(Vec::len),
            Some(1)
        );
    }

    #[tokio::test]
    async fn voided_match_pays_nothing() {
        install_test_cards();
        let app = scenario().await;
        append_log_to(&app, MATCH_ID, &actions_of("p1", 30)).await;
        void_match(
            &app,
            VoidMatchInput {
                match_id: MATCH_ID.to_owned(),
                players: (A.to_owned(), B.to_owned()),
                at: now_ms(),
            },
        )
        .await
        .expect("the match is voided");
        assert_paid_nothing(&app).await;
        assert_eq!(table(&app, |data| j(&data.tables.results)).await, json!([]));
    }

    #[tokio::test]
    async fn r1302006_nothing_is_paid_while_the_economy_is_off() {
        install_test_cards();
        // The build's own app: `ECONOMY_LIVE` is off (§14.12), as in production until launch.
        let app = built_app().await;
        assert!(!app.economy_live);
        fresh_store(&app).await;
        seed_profile(&app, json!({ "id": A, "rating": 1000.0, "inMatchId": MATCH_ID })).await;
        seed_profile(&app, json!({ "id": B, "rating": 1000.0, "inMatchId": MATCH_ID })).await;
        create_match(&app, MATCH_ID, true, false).await;
        append_log_to(&app, MATCH_ID, &actions_of("p1", 30)).await;
        let outcome = play(&[json!({ "type": "play", "card": "test-lethal", "playerId": "p1" })]);
        record_on(&app, MATCH_ID, outcome).await;

        assert_paid_nothing(&app).await;
        assert_eq!(
            table(&app, |data| j(&data.tables.results))
                .await
                .as_array()
                .map(Vec::len),
            Some(1)
        );
    }

    #[tokio::test]
    async fn bot_side_earns_nothing() {
        install_test_cards();
        let app = test_app().await;
        fresh_store(&app).await;
        seed_profile(&app, json!({ "id": A, "rating": 1000.0, "inMatchId": MATCH_ID })).await;
        seed_profile(
            &app,
            json!({ "id": B, "kind": "bot", "rating": 1000.0, "inMatchId": MATCH_ID }),
        )
        .await;
        create_match(&app, MATCH_ID, true, false).await;
        append_log_to(&app, MATCH_ID, &actions_of("p1", 30)).await;
        let outcome = play(&[json!({ "type": "play", "card": "test-lethal", "playerId": "p1" })]);
        record_on(&app, MATCH_ID, outcome).await;

        // The human side is paid; the bot's side earns nothing (R1454).
        assert_eq!(game_jlore_paid(&app, A).await, i64::from(JLORE_LADDER_WIN));
        assert_eq!(game_jlore_paid(&app, B).await, 0);
        assert!(tracks_of(&app, B).await.is_empty());
        assert_eq!(
            j(&ledger_of(&app, B).await),
            json!({
                "jlore": [], "inventory": [], "counters": [],
                "tracks": [], "quests": [], "cosmetics": [],
            })
        );
    }
}

// The payout commits in the results-row transaction: one write leaves the row, the Jlore grant
// and the track grants with the same match id, and a failure leaves none of them.

mod joint_commit {
    use super::*;

    #[tokio::test]
    async fn row_jlore_and_tracks_commit_together() {
        install_test_cards();
        let app = scenario().await;
        append_log_to(&app, MATCH_ID, &actions_of("p1", 10)).await;
        let outcome = play(&[json!({ "type": "play", "card": "test-lethal", "playerId": "p1" })]);
        record_on(&app, MATCH_ID, outcome).await;

        let results = table(&app, |data| j(&data.tables.results)).await;
        assert_eq!(results.as_array().map(Vec::len), Some(1));
        assert_eq!(results[0]["matchId"], json!(MATCH_ID));

        let ledger = ledger_of(&app, A).await;
        let game_jlore: Vec<_> = ledger
            .jlore
            .iter()
            .filter(|grant| grant.reason == JloreReason::Game)
            .collect();
        assert_eq!(game_jlore.len(), 1);
        assert_eq!(game_jlore[0].delta, i64::from(JLORE_LADDER_WIN));
        assert_eq!(game_jlore[0].reference.as_deref(), Some(MATCH_ID));
        assert!(!ledger.tracks.is_empty());
        assert!(
            ledger
                .tracks
                .iter()
                .all(|grant| grant.reference.as_deref() == Some(MATCH_ID)),
            "every track grant names its game: {:?}",
            ledger.tracks
        );
    }

    #[tokio::test]
    async fn payout_failure_writes_neither_row_nor_ledger() {
        install_test_cards();
        let app = scenario().await;
        append_log_to(&app, MATCH_ID, &actions_of("p1", 10)).await;
        fake(&app).lock().await.on_call = Some(Arc::new(|method: &str| {
            if method == "economy.wallets.grant" {
                Err(StoreError::Other("injected fault".to_string()))
            } else {
                Ok(())
            }
        }));
        let outcome = play(&[json!({ "type": "play", "card": "test-lethal", "playerId": "p1" })]);
        let error = record_result(
            &app,
            RecordResultInput {
                match_id: MATCH_ID.to_owned(),
                seats: seats(),
                outcome,
                turns: 3,
                at: now_ms(),
                last_boards: None,
                final_hash: None,
            },
        )
        .await
        .expect_err("the injected fault fails the write");
        assert!(format!("{error:?}").contains("injected fault"), "{error:?}");

        assert_eq!(table(&app, |data| j(&data.tables.results)).await, json!([]));
        assert_eq!(jlore_of(&app, A).await, 0);
        assert!(tracks_of(&app, A).await.is_empty());
    }
}

// `GET /api/wallet` and `GET /api/tracks` serve the balance, the levels, the XP and the next
// rewards, for the caller's account alone.

mod reads {
    use super::*;
    use jackioh_server::economy::math::xp_to_next_level;

    /// The scenario's profiles as signed-in accounts, with their tokens.
    async fn authed() -> (Arc<App>, String) {
        let app = test_app().await;
        fresh_store(&app).await;
        seed_profile(
            &app,
            json!({ "id": A, "userId": "user-a", "status": "active", "inMatchId": MATCH_ID }),
        )
        .await;
        seed_profile(
            &app,
            json!({ "id": B, "userId": "user-b", "status": "active", "inMatchId": MATCH_ID }),
        )
        .await;
        create_match(&app, MATCH_ID, true, false).await;
        let token = add_user(&app, "user-a", "a@example.test", true);
        (app, token)
    }

    #[tokio::test]
    async fn get_wallet_serves_the_balance() {
        install_test_cards();
        let (app, token) = authed().await;
        let (status, _, body) = call(&app, "GET", "/api/wallet", Some(&token), Value::Null).await;
        assert_eq!(status, 200);
        assert_eq!(body, json!({ "jlore": 0 }));

        append_log_to(&app, MATCH_ID, &actions_of("p1", 10)).await;
        let outcome = play(&[json!({ "type": "play", "card": "test-lethal", "playerId": "p1" })]);
        record_on(&app, MATCH_ID, outcome).await;

        let (status, _, body) = call(&app, "GET", "/api/wallet", Some(&token), Value::Null).await;
        assert_eq!(status, 200);
        // The game's Jlore plus the track Jlore of every level that game reached, each once.
        let levels: i64 = (1..=track_level(game_xp(10, true)))
            .map(|level| reward_for_level(level).jlore)
            .sum();
        assert_eq!(body, json!({ "jlore": i64::from(JLORE_LADDER_WIN) + levels }));
    }

    #[tokio::test]
    async fn get_tracks_serves_levels_xp_and_next_rewards() {
        install_test_cards();
        let (app, token) = authed().await;
        let (status, _, body) = call(&app, "GET", "/api/tracks", Some(&token), Value::Null).await;
        assert_eq!(status, 200);
        let virgin: Vec<Value> = SHIPPED_SETS
            .iter()
            .map(|set| {
                json!({
                    "setId": set.as_str(),
                    "level": SET_TRACK_FIRST_LEVEL,
                    "xp": 0,
                    "xpToNext": xp_to_next_level(0),
                    "nextRewards": j(&reward_for_level(1)),
                    "rewardsClaimed": 0,
                    "unlocked": *set == newest(),
                })
            })
            .collect();
        assert_eq!(body, json!({ "tracks": virgin }));

        append_log_to(&app, MATCH_ID, &actions_of("p1", 10)).await;
        let outcome = play(&[json!({ "type": "play", "card": "test-lethal", "playerId": "p1" })]);
        record_on(&app, MATCH_ID, outcome).await;

        let (status, _, body) = call(&app, "GET", "/api/tracks", Some(&token), Value::Null).await;
        assert_eq!(status, 200);
        let one_game = game_xp(10, true);
        let level = track_level(one_game);
        assert!(
            level < i64::from(SET_TRACK_LEVELS),
            "one game leaves a next level"
        );
        let tracks = body["tracks"].as_array().expect("five tracks");
        assert_eq!(tracks.len(), SHIPPED_SETS.len());
        assert_eq!(
            tracks[SHIPPED_SETS.len() - 1],
            json!({
                "setId": newest().as_str(),
                "level": level,
                "xp": one_game,
                "xpToNext": xp_to_next_level(one_game),
                "nextRewards": j(&reward_for_level(level + 1)),
                "rewardsClaimed": level,
                "unlocked": true,
            })
        );
        assert!(
            tracks[..SHIPPED_SETS.len() - 1]
                .iter()
                .all(|track| track["unlocked"] == json!(false)),
            "older tracks stay locked: {tracks:?}"
        );
    }

    #[tokio::test]
    async fn r1302005_a_track_on_its_last_level_reads_no_next_level() {
        install_test_cards();
        let (app, token) = authed().await;
        let last = i64::from(SET_TRACK_LEVELS);
        let xp = total_xp_for_level(SET_TRACK_LEVELS + 1);
        grant_track(&app, A, newest(), xp, last, last).await;

        let (status, _, body) = call(&app, "GET", "/api/tracks", Some(&token), Value::Null).await;
        assert_eq!(status, 200);
        assert_eq!(
            body["tracks"][SHIPPED_SETS.len() - 1],
            json!({
                "setId": newest().as_str(),
                "level": last,
                "xp": xp,
                "xpToNext": null,
                "nextRewards": null,
                "rewardsClaimed": last,
                "unlocked": true,
            })
        );
    }
}

// R1304001: the same hook moves the quests each side holds, by what the ladder game did.

mod quests {
    use super::*;
    use jackioh_server::economy::quests::{QuestGoal, pool, quest_interval_ms};
    use jackioh_server::economy::store::{QuestChange, QuestGrant};

    #[tokio::test]
    async fn r1304001_a_ladder_game_moves_its_players_quests() {
        install_test_cards();
        let app = scenario().await;
        // No quest falls due during the game, and A holds the pool's easiest attack quest.
        store!(app, |t| t.economy_quests_set_next_due(
            A,
            Some(now_ms() + 10 * quest_interval_ms())
        ));
        let attack = pool()
            .iter()
            .filter(|quest| quest.goal == QuestGoal::AttackWithUnits)
            .min_by_key(|quest| quest.target)
            .expect("the pool has an attack quest");
        assert!(attack.target > 3, "three attacks leave it held");
        let arrived = QuestGrant {
            profile_id: A.to_owned(),
            quest_id: attack.id.clone(),
            change: QuestChange::Arrived,
            progress: 0,
            reason: EconomyReason::Dev,
            reference: None,
            dev: true,
            at: now_ms(),
        };
        assert!(store!(app, |t| t.economy_quests_change(&arrived)));
        let attack_row = json!({ "type": "attack", "attackerId": "c1", "targetId": "c2", "playerId": "p1" });
        append_log_to(
            &app,
            MATCH_ID,
            &[attack_row.clone(), attack_row.clone(), attack_row],
        )
        .await;
        let outcome = play(&[json!({ "type": "play", "card": "test-lethal", "playerId": "p1" })]);
        record_on(&app, MATCH_ID, outcome).await;

        let book = store!(app, |t| t.economy_quests_get(A));
        assert_eq!(book.held.len(), 1);
        assert_eq!(book.held[0].quest_id, attack.id);
        assert_eq!(book.held[0].progress, 3, "each logged attack of A's seat counts");
        let moved = ledger_of(&app, A).await.quests;
        assert_eq!(
            moved.last().map(|row| (row.change, row.reference.as_deref())),
            Some((QuestChange::Progressed, Some(MATCH_ID)))
        );
    }
}
