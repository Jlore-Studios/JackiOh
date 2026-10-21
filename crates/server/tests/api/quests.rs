//! Daily quests (SPEC §14.9, #1304): their rhythm on the server's clock, what a finished game
//! moves, the payout of one that completes, `GET /api/quests` and the dev tools.
//!
//! The clock is injected: `top_up_quests` and the reward hook take the time they run at, so a test
//! walks an account through days in milliseconds. Every count and payout is read through the
//! constants (`QUEST_INTERVAL_HOURS`, `QUEST_MAX_HELD`, `QUEST_XP_PER_JLORE`) and the pool, so the
//! tests hold whatever values #1333 sets. R1304000–R1304002's proving tests live here, beside
//! `rewards.rs`'s ladder game.
//!
//! Requests go through the whole app (`support::deps::test_app`, the E2E fixtures): the caller is
//! `e2e-p1`, the other account `e2e-p2`. The dev tools' tests list `e2e-p1` as a dev account.

use std::collections::BTreeSet;
use std::sync::Arc;

use jackioh_cards::{CATALOG, register_all};
use jackioh_engine::wire::{CardType, SHIPPED_SETS};
use jackioh_engine::{
    Action, ActionBody, CreateGameOptions, DECK_SIZE, FoldArgs, PlayerId, Tag, begin_game, create_game,
    create_rng, deckable, reduce, seat_to_act, subsystems, summarize_game,
};
use serde_json::{Value, json};

use jackioh_server::app::{App, now_ms};
use jackioh_server::config::{JLORE_GAMES_DAILY_CAP, QUEST_MAX_HELD, QUEST_XP_PER_JLORE};
use jackioh_server::economy::quests::{
    GameTally, PlayedCard, QuestDef, QuestGoal, pool, progress_of, quest_def, quest_interval_ms, quest_xp,
    tally_seat, top_up_quests,
};
use jackioh_server::economy::rewards::{GameMode, GameSummary, pay_game_rewards};
use jackioh_server::economy::store::{
    EconomyLedger, EconomyReason, JloreGrant, JloreReason, QuestChange, QuestGrant,
};

use crate::support::deps::{call, test_app as built_app};

/// The caller's bearer token and managed-auth user id (`e2e/support/config.ts`'s `e2e-p1`).
const TOKEN: &str = "e2e-token-p1";
const USER: &str = "e2e-p1";
/// The other active account (`e2e-p2`).
const OTHER_TOKEN: &str = "e2e-token-p2";
const OTHER_USER: &str = "e2e-p2";
/// When each clock test's account first shows up.
const T0: i64 = 1_800_000_000_000;

fn interval() -> i64 {
    quest_interval_ms()
}

/// The test app with the economy on (§14.12), as `rewards.rs`'s.
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

async fn profile_id_of(app: &App, user_id: &str) -> String {
    let mut tx = app.db.begin(None).await.expect("a store transaction");
    let profile = tx
        .profiles_get_by_user_id(user_id)
        .await
        .expect("profiles.getByUserId")
        .unwrap_or_else(|| panic!("the fixture account {user_id} has a profile"));
    tx.commit().await.expect("commit");
    profile.id
}

/// The test app with `e2e-p1` listed in `DEV_PROFILE_IDS`, so the quest tools answer it as they
/// answer a dev account (R1300000); and its profile id.
async fn dev_app() -> (Arc<App>, String) {
    let mut app = built_app().await;
    let p1 = profile_id_of(&app, USER).await;
    Arc::get_mut(&mut app)
        .expect("the test app has no second handle yet")
        .env
        .dev_profile_ids = vec![p1.clone()];
    (app, p1)
}

async fn ledger_of(app: &App, profile: &str) -> EconomyLedger {
    store!(app, |t| t.economy_ledger_get(profile))
}

/// The ids `profile` holds, oldest first.
async fn held_ids(app: &App, profile: &str) -> Vec<String> {
    store!(app, |t| t.economy_quests_get(profile))
        .held
        .into_iter()
        .map(|quest| quest.quest_id)
        .collect()
}

/// `profile` holds `def` at `progress`, written as a dev grant without the clock.
async fn hold(app: &App, profile: &str, def: &QuestDef, progress: i64, at: i64) {
    let grant = |change: QuestChange, progress: i64| QuestGrant {
        profile_id: profile.to_owned(),
        quest_id: def.id.clone(),
        change,
        progress,
        reason: EconomyReason::Dev,
        reference: None,
        dev: true,
        at,
    };
    assert!(store!(app, |t| t.economy_quests_change(&grant(QuestChange::Arrived, 0))));
    if progress > 0 {
        assert!(store!(app, |t| t
            .economy_quests_change(&grant(QuestChange::Progressed, progress))));
    }
}

async fn set_due(app: &App, profile: &str, at: i64) {
    store!(app, |t| t.economy_quests_set_next_due(profile, Some(at)));
}

/// The pool's entry of `goal` with the smallest target.
fn easiest(goal: &QuestGoal) -> &'static QuestDef {
    pool()
        .iter()
        .filter(|quest| quest.goal == *goal)
        .min_by_key(|quest| quest.target)
        .unwrap_or_else(|| panic!("the pool has a {goal:?} quest"))
}

/// A won ladder game of `profile`'s, as the reward hook takes it.
fn won_game(profile: &str, match_id: &str, tally: GameTally, at: i64) -> GameSummary {
    GameSummary {
        mode: GameMode::Ladder,
        profile_id: profile.to_owned(),
        player: PlayerId::P1,
        won: Some(true),
        actions: 0,
        match_id: match_id.to_owned(),
        at,
        bot: false,
        tally,
    }
}

/// A game that moves every goal kind the pool has: a Unit, a Spell and a Trap of every shipped
/// set, each Radiant, a Trap that fired and an attack.
fn tally_of_everything() -> GameTally {
    let played = SHIPPED_SETS
        .iter()
        .flat_map(|set| {
            [CardType::Unit, CardType::Spell, CardType::Trap].map(|card_type| PlayedCard {
                card_type,
                set: *set,
                radiant: true,
            })
        })
        .collect();
    GameTally {
        played,
        traps_fired: 1,
        attacks: 1,
    }
}

async fn get_quests(app: &Arc<App>, token: &str) -> (u16, Value) {
    let (status, _, body) = call(app, "GET", "/api/quests", Some(token), Value::Null).await;
    (status, body)
}

#[tokio::test]
async fn r1304000_quests_arrive_by_the_injected_clock() {
    let app = test_app().await;
    let p1 = profile_id_of(&app, USER).await;
    let i = interval();
    for (now, holds) in [(T0, 0), (T0 + i - 1, 0), (T0 + i, 1)] {
        let book = store!(app, |t| top_up_quests(&mut t, &p1, now));
        assert_eq!(book.held.len(), holds, "at T0 + {}", now - T0);
    }
    let book = store!(app, |t| top_up_quests(&mut t, &p1, T0 + i));
    assert_eq!(book.held[0].arrived_at, T0 + i, "dated when it fell due");
    assert_eq!(book.next_due_at, Some(T0 + 2 * i));

    let rows = ledger_of(&app, &p1).await.quests;
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].change, QuestChange::Arrived);
    assert_eq!(rows[0].reason, EconomyReason::Quest);
    assert!(!rows[0].dev);
    assert!(
        rows[0]
            .reference
            .as_deref()
            .is_some_and(|reference| reference.starts_with("seed:")),
        "the draw's seed is logged: {rows:?}"
    );
    assert!(quest_def(&rows[0].quest_id).is_some());
}

#[tokio::test]
async fn r1304000_a_long_absence_returns_exactly_the_cap() {
    let app = test_app().await;
    let i = interval();
    // By the injected clock: a first visit, then a hundred intervals away.
    let p2 = profile_id_of(&app, OTHER_USER).await;
    store!(app, |t| top_up_quests(&mut t, &p2, T0));
    let book = store!(app, |t| top_up_quests(&mut t, &p2, T0 + 100 * i));
    let ids: BTreeSet<&str> = book.held.iter().map(|quest| quest.quest_id.as_str()).collect();
    assert_eq!(book.held.len(), QUEST_MAX_HELD);
    assert_eq!(ids.len(), QUEST_MAX_HELD, "no quest is held twice");
    assert_eq!(book.next_due_at, Some(T0 + 101 * i));

    // And through the route, by the server's own clock: the caller shows up after the same absence.
    let p1 = profile_id_of(&app, USER).await;
    set_due(&app, &p1, now_ms() - 100 * i).await;
    let (status, body) = get_quests(&app, TOKEN).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["quests"].as_array().map(Vec::len), Some(QUEST_MAX_HELD));
    assert_eq!(body["nextDueAt"], Value::Null, "none is due at the cap");
}

#[tokio::test]
async fn r1304002_no_quest_arrives_while_the_economy_is_off() {
    // The build's own app: `ECONOMY_LIVE` is off (§14.12).
    let app = built_app().await;
    assert!(!app.economy_live);
    let p1 = profile_id_of(&app, USER).await;
    set_due(&app, &p1, now_ms() - 10 * interval()).await;
    let (status, body) = get_quests(&app, TOKEN).await;
    assert_eq!(status, 200);
    assert_eq!(body, json!({ "quests": [], "nextDueAt": null }));
    assert!(ledger_of(&app, &p1).await.quests.is_empty());
}

#[tokio::test]
async fn get_api_quests_answers_only_the_callers_own() {
    let app = built_app().await;
    let p1 = profile_id_of(&app, USER).await;
    let win_3 = quest_def("win-3").expect("the pool has win-3");
    hold(&app, &p1, win_3, 1, T0).await;

    let (status, body) = get_quests(&app, TOKEN).await;
    assert_eq!(status, 200);
    assert_eq!(
        body,
        json!({
            "quests": [{
                "questId": "win-3",
                "title": win_3.title,
                "progress": 1,
                "target": win_3.target,
                "jlore": win_3.jlore,
                "xp": quest_xp(win_3),
                "arrivedAt": T0,
            }],
            "nextDueAt": null,
        })
    );
    let (status, body) = get_quests(&app, OTHER_TOKEN).await;
    assert_eq!(status, 200);
    assert_eq!(body, json!({ "quests": [], "nextDueAt": null }));
    let (status, _, _) = call(&app, "GET", "/api/quests", None, Value::Null).await;
    assert_eq!(status, 401);
}

#[tokio::test]
async fn r1304001_a_game_that_ends_as_a_quest_arrives_counts_toward_it() {
    let app = test_app().await;
    let p1 = profile_id_of(&app, USER).await;
    let i = interval();
    store!(app, |t| top_up_quests(&mut t, &p1, T0));

    // A game that ends a moment before the first quest falls due moves nothing: none is held.
    let before = won_game(&p1, "match-1", tally_of_everything(), T0 + i - 1);
    store!(app, |t| pay_game_rewards(&mut t, &before));
    assert!(held_ids(&app, &p1).await.is_empty());

    // One that ends as it falls due counts toward it.
    let at = won_game(&p1, "match-2", tally_of_everything(), T0 + i);
    store!(app, |t| pay_game_rewards(&mut t, &at));
    let rows = ledger_of(&app, &p1).await.quests;
    let arrived = rows
        .iter()
        .find(|row| row.change == QuestChange::Arrived)
        .expect("a quest arrived at T0 + I");
    assert_eq!(arrived.at, T0 + i);
    assert!(
        rows.iter().any(|row| {
            row.quest_id == arrived.quest_id
                && matches!(row.change, QuestChange::Progressed | QuestChange::Completed)
                && row.reference.as_deref() == Some("match-2")
        }),
        "the game that ended as it arrived moved it: {rows:?}"
    );
}

#[tokio::test]
async fn r1304001_a_completed_quest_pays_its_jlore_and_xp_exactly_once() {
    let app = test_app().await;
    let p1 = profile_id_of(&app, USER).await;
    let now = now_ms();
    set_due(&app, &p1, now + 10 * interval()).await;
    let win = easiest(&QuestGoal::WinGames);
    hold(&app, &p1, win, win.target - 1, now).await;

    let paid = |ledger: &EconomyLedger| {
        ledger
            .jlore
            .iter()
            .filter(|grant| grant.reason == JloreReason::Quest)
            .cloned()
            .collect::<Vec<_>>()
    };
    store!(app, |t| pay_game_rewards(
        &mut t,
        &won_game(&p1, "match-1", GameTally::default(), now)
    ));
    let ledger = ledger_of(&app, &p1).await;
    let jlore = paid(&ledger);
    assert_eq!(jlore.len(), 1, "{jlore:?}");
    assert_eq!(jlore[0].delta, i64::from(win.jlore));
    assert_eq!(jlore[0].reference.as_deref(), Some(win.id.as_str()));
    assert!(!jlore[0].dev);
    let xp: Vec<_> = ledger
        .tracks
        .iter()
        .filter(|grant| grant.reason == EconomyReason::Quest)
        .collect();
    assert!(!xp.is_empty(), "the XP lands on the active tracks");
    for grant in &xp {
        assert_eq!(grant.xp, i64::from(win.jlore) * i64::from(QUEST_XP_PER_JLORE));
        assert_eq!(grant.reference.as_deref(), Some(win.id.as_str()));
    }
    assert!(
        ledger
            .quests
            .iter()
            .any(|row| row.change == QuestChange::Completed && row.reference.as_deref() == Some("match-1"))
    );
    assert!(held_ids(&app, &p1).await.is_empty());

    // A second win pays it no more: it is no longer held.
    store!(app, |t| pay_game_rewards(
        &mut t,
        &won_game(&p1, "match-2", GameTally::default(), now + 1)
    ));
    assert_eq!(paid(&ledger_of(&app, &p1).await).len(), 1);
}

#[tokio::test]
async fn r1483000_quest_jlore_pays_past_the_game_cap() {
    let app = test_app().await;
    let p1 = profile_id_of(&app, USER).await;
    let now = now_ms();
    set_due(&app, &p1, now + 10 * interval()).await;
    let win = easiest(&QuestGoal::WinGames);
    hold(&app, &p1, win, win.target - 1, now).await;
    // Today's games have already paid the cap.
    let earlier = JloreGrant {
        profile_id: p1.clone(),
        delta: JLORE_GAMES_DAILY_CAP,
        reason: JloreReason::Game,
        reference: Some("earlier-game".to_string()),
        dev: false,
        at: now,
    };
    store!(app, |t| t.economy_jlore_grant(&earlier));

    store!(app, |t| pay_game_rewards(
        &mut t,
        &won_game(&p1, "match-1", GameTally::default(), now)
    ));
    let jlore = ledger_of(&app, &p1).await.jlore;
    let by = |reason: JloreReason| -> Vec<JloreGrant> {
        jlore
            .iter()
            .filter(|grant| grant.reason == reason)
            .cloned()
            .collect()
    };
    assert_eq!(
        by(JloreReason::Game),
        vec![earlier],
        "the win's own Jlore was held back"
    );
    let quest = by(JloreReason::Quest);
    assert_eq!(quest.len(), 1, "{quest:?}");
    assert_eq!(
        quest[0].delta,
        i64::from(win.jlore),
        "the quest's Jlore is paid past the cap"
    );
}

#[tokio::test]
async fn r1304001_progress_stops_at_the_target() {
    let app = test_app().await;
    let p1 = profile_id_of(&app, USER).await;
    let now = now_ms();
    set_due(&app, &p1, now + 10 * interval()).await;
    let attack = easiest(&QuestGoal::AttackWithUnits);
    hold(&app, &p1, attack, 0, now).await;

    let tally = GameTally {
        attacks: attack.target + 5,
        ..GameTally::default()
    };
    store!(app, |t| pay_game_rewards(
        &mut t,
        &won_game(&p1, "match-1", tally, now)
    ));
    let rows: Vec<QuestGrant> = ledger_of(&app, &p1)
        .await
        .quests
        .into_iter()
        .filter(|row| row.quest_id == attack.id && row.reference.as_deref() == Some("match-1"))
        .collect();
    assert_eq!(
        rows.iter()
            .map(|row| (row.change, row.progress))
            .collect::<Vec<_>>(),
        vec![
            (QuestChange::Progressed, attack.target),
            (QuestChange::Completed, 0)
        ]
    );
}

#[tokio::test]
async fn dev_quest_tools_give_and_complete_once() {
    let (app, p1) = dev_app().await;
    for _ in 0..QUEST_MAX_HELD {
        let (status, _, body) = call(&app, "POST", "/api/dev/quests", Some(TOKEN), Value::Null).await;
        assert_eq!(status, 200, "{body}");
        assert!(quest_def(body["quest"]["questId"].as_str().unwrap_or_default()).is_some());
    }
    let (status, _, _) = call(&app, "POST", "/api/dev/quests", Some(TOKEN), Value::Null).await;
    assert_eq!(status, 409, "at the cap");
    let ledger = ledger_of(&app, &p1).await;
    assert_eq!(ledger.quests.len(), QUEST_MAX_HELD);
    assert!(
        ledger
            .quests
            .iter()
            .all(|row| row.change == QuestChange::Arrived && row.reason == EconomyReason::Dev && row.dev)
    );

    let held = held_ids(&app, &p1).await;
    let def = quest_def(&held[0]).expect("a pool entry");
    let path = format!("/api/dev/quests/{}/complete", def.id);
    let (status, _, body) = call(&app, "POST", &path, Some(TOKEN), Value::Null).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body,
        json!({ "completed": def.id, "jlore": def.jlore, "xp": quest_xp(def) })
    );
    let jlore: Vec<_> = ledger_of(&app, &p1)
        .await
        .jlore
        .into_iter()
        .filter(|grant| grant.reason == JloreReason::Quest)
        .collect();
    assert_eq!(jlore.len(), 1);
    assert!(jlore[0].dev, "a dev tool's payout is marked dev");
    assert_eq!(jlore[0].delta, i64::from(def.jlore));

    let (status, _, _) = call(&app, "POST", &path, Some(TOKEN), Value::Null).await;
    assert_eq!(status, 404, "no longer held");
    let (status, _, _) = call(
        &app,
        "POST",
        "/api/dev/quests/no-such-quest/complete",
        Some(TOKEN),
        Value::Null,
    )
    .await;
    assert_eq!(status, 404);
    assert_eq!(
        ledger_of(&app, &p1)
            .await
            .jlore
            .iter()
            .filter(|grant| grant.reason == JloreReason::Quest)
            .count(),
        1
    );
}

#[tokio::test]
async fn dev_quest_tools_answer_an_account_the_dev_list_does_not_name_403() {
    let (app, _) = dev_app().await;
    let p2 = profile_id_of(&app, OTHER_USER).await;
    let def = &pool()[0];
    hold(&app, &p2, def, 0, now_ms()).await;
    let before = ledger_of(&app, &p2).await;
    let complete = format!("/api/dev/quests/{}/complete", def.id);
    for path in ["/api/dev/quests", complete.as_str()] {
        let (status, _, body) = call(&app, "POST", path, Some(OTHER_TOKEN), Value::Null).await;
        assert_eq!(status, 403, "{path}: {body}");
        assert_eq!(body["error"]["code"], json!("forbidden"), "{path}: {body}");
    }
    assert_eq!(ledger_of(&app, &p2).await, before, "nothing is written");
}

/// Real games, played by SPEC §10.7's policy over real decks as `crates/cards/tests/cross/
/// game_summary.rs` plays them, and read as the ladder's payout reads them: every entry of the pool
/// is moved by at least one seat of one of them.
#[test]
fn fixture_games_move_every_goal_kind_in_the_pool() {
    const GAMES: usize = 10;
    const MAX_ACTIONS: usize = 3000;
    register_all();
    let mut cards: Vec<String> = CATALOG
        .iter()
        .filter(|(_, def)| deckable(def) && !def.tags.contains(&Tag::Token))
        .map(|(id, _)| id.clone())
        .collect();
    cards.sort();
    let size = usize::try_from(DECK_SIZE).expect("a deck size");
    let mut moved: BTreeSet<String> = BTreeSet::new();
    for n in 1..=GAMES {
        let seed = format!("quests-{n}");
        let shuffled = create_rng(&format!("quests-decks-{n}"), 0).shuffle(&cards);
        let decks = (shuffled[..size].to_vec(), shuffled[size..size * 2].to_vec());
        let mut policy = create_rng(&format!("quests-policy-{n}"), 0);
        let mut state = begin_game(&create_game(&CreateGameOptions {
            seed: seed.clone(),
            decks: decks.clone(),
            ..CreateGameOptions::default()
        }))
        .state;
        let mut log: Vec<Action> = Vec::new();
        while state.result.is_none() && log.len() < MAX_ACTIONS {
            let player = seat_to_act(&state).unwrap_or_else(|| panic!("{seed}: no seat to act"));
            let chosen = subsystems::choose_action(&state, player, &mut policy)
                .unwrap_or_else(|| panic!("{seed}: no action for {}", player.as_str()));
            let action = Action::new(chosen, player, format!("g{}", log.len()));
            let result = reduce(&state, &action);
            if let Some(error) = &result.error {
                panic!("{seed}: {} refused: {error}", action.action_type());
            }
            state = result.state;
            log.push(action);
        }
        let summary = summarize_game(&FoldArgs {
            seed: seed.clone(),
            decks,
            log: log.clone(),
            ..FoldArgs::default()
        })
        .unwrap_or_else(|| panic!("{seed} has no summary"));
        for seat in [PlayerId::P1, PlayerId::P2] {
            let attacks = log
                .iter()
                .filter(|action| action.player_id == seat && matches!(action.body, ActionBody::Attack { .. }))
                .count();
            let tally = tally_seat(
                Some(&summary.seats[seat]),
                i64::try_from(attacks).expect("a count"),
                &CATALOG,
            );
            let won = summary.winner.player().map(|winner| winner == seat);
            for quest in pool() {
                if progress_of(&quest.goal, won, &tally) > 0 {
                    moved.insert(quest.id.clone());
                }
            }
        }
    }
    let unmoved: Vec<&str> = pool()
        .iter()
        .map(|quest| quest.id.as_str())
        .filter(|id| !moved.contains(*id))
        .collect();
    assert!(unmoved.is_empty(), "no fixture game moves {unmoved:?}");
}
