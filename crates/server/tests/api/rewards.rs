//! A finished ladder game pays Jlore and XP (SPEC §14.8, #1302): through the one reward hook,
//! in the same transaction as its results row, onto every active set track.
//!
//! Won and lost games pay exactly `JLORE_LADDER_WIN`/`JLORE_LADDER_LOSS`; XP counts the log's
//! actions plus the win bonus up to `XP_GAME_CAP`; the second track unlocks at
//! `SET_TRACK_UNLOCK_LEVEL`; and draws, concedes and reaper-resolved games pay per R1302000–
//! R1302003, whose proving tests live here. Series games, room games and bot sides pay nothing.
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
    JLORE_LADDER_LOSS, JLORE_LADDER_WIN, MATCH_CEILING_MINUTES, SET_TRACK_FIRST_LEVEL,
    SET_TRACK_UNLOCK_LEVEL, XP_GAME_CAP,
};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{Db, MatchActionRow, MatchRow, MatchSeat, ResultRow, StoreError};
use jackioh_server::economy::math::{level_for_xp, total_xp_for_level};
use jackioh_server::economy::rewards::game_xp;
use jackioh_server::economy::store::{
    EconomyLedger, EconomyReason, InventoryEntry, InventoryItem, JloreReason, SetTrack, TrackGrant,
};

use crate::support::deps::{add_user, call, test_app};
use crate::support::engine::{fake_deck, install_test_cards};

const MINUTE: i64 = 60 * 1000;
/// A minute past the ceiling (R79, R389), whatever it is.
const PAST_CEILING_MS: i64 = (MATCH_CEILING_MINUTES + 1) * MINUTE;
const MATCH_ID: &str = "match-1";
const MATCH_2: &str = "match-2";
const MATCH_3: &str = "match-3";
const A: &str = "profile-a";
const B: &str = "profile-b";

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
        assert_eq!(level_for_xp(unlocked.xp), 1);
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

// R1302000–R1302003: what a draw, a concede, a reaper-resolved game and an unlocking backlog pay.

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

    fn track_packs(inventory: &[InventoryEntry]) -> i64 {
        inventory
            .iter()
            .filter(|entry| entry.item == InventoryItem::SetPack)
            .map(|entry| entry.count)
            .sum()
    }

    #[tokio::test]
    async fn each_level_pays_once_and_a_repeated_write_pays_nothing() {
        install_test_cards();
        let app = scenario().await;
        // Game one: 30 counted actions won. Level 1 reached, so level 1 pays.
        append_log_to(&app, MATCH_ID, &actions_of("p1", 30)).await;
        let outcome = play(&[json!({ "type": "concede", "playerId": "p2" })]);
        record_on(&app, MATCH_ID, outcome).await;
        let one_game = game_xp(30, true);
        assert!(
            level_for_xp(one_game) == 1,
            "one game stays on level 1: {one_game}"
        );
        assert_eq!(track_jlore_total(&ledger_of(&app, A).await), 100);

        // Game two crosses into level 2: only level 2 pays now.
        create_match(&app, MATCH_2, true, false).await;
        append_log_to(&app, MATCH_2, &actions_of("p1", 30)).await;
        let outcome = play(&[json!({ "type": "concede", "playerId": "p2" })]);
        record_on(&app, MATCH_2, outcome).await;
        assert_eq!(level_for_xp(2 * one_game), 2, "two games reach level 2");
        assert_eq!(track_jlore_total(&ledger_of(&app, A).await), 100);
        assert_eq!(track_packs(&inventory_of(&app, A).await), 1);

        // Game three stays on level 2: no level pays again.
        create_match(&app, MATCH_3, true, false).await;
        append_log_to(&app, MATCH_3, &actions_of("p1", 1)).await;
        let outcome = play(&[json!({ "type": "concede", "playerId": "p1" })]);
        record_on(&app, MATCH_3, outcome).await;
        assert_eq!(level_for_xp(2 * one_game + game_xp(1, false)), 2);
        assert_eq!(track_jlore_total(&ledger_of(&app, A).await), 100);
        assert_eq!(track_packs(&inventory_of(&app, A).await), 1);

        // A repeated write of the same match pays nothing twice.
        let before = j(&ledger_of(&app, A).await);
        record_on(&app, MATCH_3, outcome).await;
        assert_eq!(j(&ledger_of(&app, A).await), before);
        assert_eq!(
            game_jlore_paid(&app, A).await,
            2 * i64::from(JLORE_LADDER_WIN) + i64::from(JLORE_LADDER_LOSS)
        );
    }
}

// Nothing is paid for series games, room challenges, voided matches or bot sides.

mod no_payout {
    use super::*;
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
    use jackioh_server::economy::track_rewards::reward_for_level;

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
        // The game's Jlore plus level 1's track Jlore.
        assert_eq!(
            body,
            json!({ "jlore": i64::from(JLORE_LADDER_WIN) + reward_for_level(1).jlore })
        );
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
        let tracks = body["tracks"].as_array().expect("five tracks");
        assert_eq!(tracks.len(), SHIPPED_SETS.len());
        assert_eq!(
            tracks[SHIPPED_SETS.len() - 1],
            json!({
                "setId": newest().as_str(),
                "level": i64::from(level_for_xp(one_game)),
                "xp": one_game,
                "xpToNext": xp_to_next_level(one_game),
                "nextRewards": j(&reward_for_level(2)),
                "rewardsClaimed": 1,
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
}
