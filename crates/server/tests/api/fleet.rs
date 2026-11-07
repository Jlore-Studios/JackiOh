//! The Fleet track (`jackioh_shared::economy::fleet`, `src/economy/rewards.rs`, SPEC §14.15, #3335): what a
//! recorded game pays it, each source once and under the daily cap, the reward each level pays once with the
//! reason `track`, and `GET /api/tracks`'s `fleet` (R3335001).

use std::sync::Arc;

use jackioh_engine::wire::{CreatedUse, SeatSummary};
use jackioh_engine::{PlayerId, SetName};
use serde_json::{Value, json};

use jackioh_server::app::{App, now_ms};
use jackioh_server::config::{
    FLEET_TRACK_LEVELS, FLEET_TRACK_XP_DAILY_MAX, FLEET_TRACK_XP_PER_LAUNCH, FLEET_TRACK_XP_PER_LEVEL,
    FLEET_TRACK_XP_PER_PIECE_FUSED, FLEET_TRACK_XP_PER_STINKYSCAPE_PLAY, FLEET_TRACK_XP_STINKYSCAPE_WIN,
    MS_PER_DAY,
};
use jackioh_server::cosmetics::fleet_reward;
use jackioh_server::economy::quests::GameTally;
use jackioh_server::economy::rewards::{GameMode, GameSummary, pay_game_rewards};
use jackioh_server::economy::store::{EconomyReason, FleetGrant, FleetTrack};
use jackioh_shared::economy::fleet::{FleetPlay, fleet_play};
use jackioh_shared::economy::rewards::utc_day_bounds;

use crate::support::deps::{call, test_app as built_app};
use crate::support::store::store;

/// The caller's bearer token and managed-auth user id (`e2e/support/config.ts`'s `e2e-p1`).
const TOKEN: &str = "e2e-token-p1";
const USER: &str = "e2e-p1";

/// The test app with the economy on (§14.12), as `rewards.rs`'s.
async fn test_app() -> Arc<App> {
    let mut app = built_app().await;
    Arc::get_mut(&mut app)
        .expect("the test app has no second handle yet")
        .economy_live = true;
    app
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

/// A Stinkyscape card that is no token, a Stinkyscape token and another set's card, of the server's catalog.
fn cards(app: &App) -> (String, String, String) {
    let find = |stinkyscape: bool, token: bool| {
        app.catalog
            .card_ids
            .iter()
            .find(|id| {
                app.catalog.is_token(id) == token
                    && app
                        .catalog
                        .defs
                        .get(id.as_str())
                        .is_some_and(|def| (def.set == SetName::Stinkyscape) == stinkyscape)
            })
            .cloned()
            .expect("the catalog ships one")
    };
    (find(true, false), find(true, true), find(false, false))
}

/// One finished game of `mode` for `profile_id` at `at`, doing `fleet`.
fn game(
    mode: GameMode,
    profile_id: &str,
    match_id: &str,
    at: i64,
    won: bool,
    fleet: FleetPlay,
) -> GameSummary {
    GameSummary {
        mode,
        profile_id: profile_id.to_owned(),
        player: PlayerId::P1,
        won: Some(won),
        actions: 4,
        match_id: match_id.to_owned(),
        at,
        bot: false,
        tally: GameTally::default(),
        proficiency: Vec::new(),
        fleet,
    }
}

/// `profile_id`'s Fleet track as a payout at `at` reads it.
async fn track_of(app: &App, profile_id: &str, at: i64) -> FleetTrack {
    let (from, until) = utc_day_bounds(at);
    store!(app, |t| t.economy_fleet_get(profile_id, from, until))
}

/// `profile_id`'s Fleet ledger rows and the cosmetic rows a Fleet level paid it (reason `track`).
async fn ledger_of(app: &App, profile_id: &str) -> (Vec<FleetGrant>, Vec<(String, Option<String>)>) {
    let ledger = store!(app, |t| t.economy_ledger_get(profile_id));
    let rewards = ledger
        .cosmetics
        .into_iter()
        .filter(|row| row.reason == EconomyReason::Track)
        .map(|row| (row.cosmetic_id, row.reference))
        .collect();
    (ledger.fleet, rewards)
}

/// Pays `summary` in its own transaction.
async fn pay(app: &App, summary: &GameSummary) {
    store!(app, |t| pay_game_rewards(&mut t, summary));
}

/// A game paying `launches` launches and nothing else.
fn launches(launches: i32) -> FleetPlay {
    FleetPlay {
        launched: launches,
        ..FleetPlay::default()
    }
}

#[tokio::test]
async fn r3335001_a_ladder_game_pays_each_fleet_source_once() {
    let app = test_app().await;
    let a = profile_id_of(&app, USER).await;
    let (stinky, token, other) = cards(&app);
    // The seat's fold: its deck's Stinkyscape card played twice, another set's card, a token and a Created copy
    // played too, which count nothing; three Pieces fused and one Starship launched; and a deck that is half
    // Stinkyscape, so its win pays the bonus.
    let seat = SeatSummary {
        deck: vec![stinky.clone(), stinky.clone(), other.clone(), other.clone()],
        played: vec![stinky.clone(), other.clone(), token.clone(), stinky.clone()],
        created_played: Some(vec![CreatedUse {
            card: stinky.clone(),
            turn: 2,
            radiant: false,
        }]),
        starship_pieces_fused: Some(3),
        starships_launched: Some(1),
        ..SeatSummary::default()
    };
    let fleet = fleet_play(Some(&seat), &app.catalog.defs);
    let at = now_ms();
    pay(&app, &game(GameMode::Ladder, &a, "match-3335", at, true, fleet)).await;
    let xp = 2 * i64::from(FLEET_TRACK_XP_PER_STINKYSCAPE_PLAY)
        + 3 * i64::from(FLEET_TRACK_XP_PER_PIECE_FUSED)
        + i64::from(FLEET_TRACK_XP_PER_LAUNCH)
        + i64::from(FLEET_TRACK_XP_STINKYSCAPE_WIN);
    assert!(
        xp < i64::from(FLEET_TRACK_XP_PER_LEVEL),
        "one game reaches no level here"
    );
    assert_eq!(
        track_of(&app, &a, at).await,
        FleetTrack {
            xp,
            level: 0,
            xp_today: xp,
        }
    );
    // One ledger row, of reason `game`, naming the match.
    let (rows, rewards) = ledger_of(&app, &a).await;
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(
        (
            rows[0].xp,
            rows[0].levels,
            rows[0].reason,
            rows[0].reference.as_deref(),
            rows[0].dev
        ),
        (xp, 0, EconomyReason::Game, Some("match-3335"), false)
    );
    assert!(rewards.is_empty());

    // Unranked and Conquest pay as the ladder does; a loss pays no win bonus; a bot's side writes nothing.
    for mode in [GameMode::Unranked, GameMode::Conquest] {
        pay(&app, &game(mode, &a, "match-3335-b", at, false, fleet)).await;
    }
    let loss = xp - i64::from(FLEET_TRACK_XP_STINKYSCAPE_WIN);
    assert_eq!(track_of(&app, &a, at).await.xp, xp + 2 * loss);
    let bot = GameSummary {
        bot: true,
        ..game(GameMode::Ladder, &a, "match-3335-c", at, true, fleet)
    };
    pay(&app, &bot).await;
    assert_eq!(track_of(&app, &a, at).await.xp, xp + 2 * loss);
}

#[tokio::test]
async fn r3335001_practice_and_coop_games_pay_no_fleet_xp() {
    let app = test_app().await;
    let a = profile_id_of(&app, USER).await;
    let at = now_ms();
    // Whatever a summary says the seat did, a game against the AI (the browser ran it) and a co-op game (its log
    // is not folded) pay the Fleet track nothing, while they still pay their XP. Hotseat and a dungeon run are
    // never recorded by the server, so nothing reaches the payout from them at all.
    let did = FleetPlay {
        stinkyscape_played: 5,
        pieces_fused: 3,
        launched: 1,
        deck_share_met: true,
    };
    for mode in [GameMode::Practice, GameMode::Coop] {
        let paid = store!(app, |t| pay_game_rewards(
            &mut t,
            &game(mode, &a, "match-3335", at, true, did)
        ));
        assert!(paid.xp > 0, "{mode:?} still pays");
    }
    assert_eq!(track_of(&app, &a, at).await, FleetTrack::default());
    assert_eq!(ledger_of(&app, &a).await, (Vec::new(), Vec::new()));
}

#[tokio::test]
async fn r3335001_the_daily_cap_holds_and_the_next_day_pays() {
    let app = test_app().await;
    let a = profile_id_of(&app, USER).await;
    let (from, _) = utc_day_bounds(now_ms());
    let at = from + 1;
    let per_game = i64::from(FLEET_TRACK_XP_PER_LAUNCH) * 3;
    // Enough games to pass the cap: the one that crosses it is paid the part that fits, the rest nothing.
    let games = FLEET_TRACK_XP_DAILY_MAX / per_game + 2;
    for n in 0..games {
        pay(
            &app,
            &game(
                GameMode::Ladder,
                &a,
                &format!("m-{n}"),
                at + n,
                false,
                launches(3),
            ),
        )
        .await;
    }
    let today = track_of(&app, &a, at).await;
    assert_eq!(today.xp_today, FLEET_TRACK_XP_DAILY_MAX);
    assert_eq!(today.xp, FLEET_TRACK_XP_DAILY_MAX);
    let (rows, _) = ledger_of(&app, &a).await;
    assert_eq!(
        rows.iter().map(|row| row.xp).sum::<i64>(),
        FLEET_TRACK_XP_DAILY_MAX
    );
    assert!(
        rows.iter().all(|row| row.xp > 0),
        "a capped game writes no row of nothing"
    );
    // The next UTC day starts the count again.
    let tomorrow = at + MS_PER_DAY;
    pay(
        &app,
        &game(GameMode::Ladder, &a, "m-next", tomorrow, false, launches(3)),
    )
    .await;
    let next = track_of(&app, &a, tomorrow).await;
    assert_eq!(
        (next.xp, next.xp_today),
        (FLEET_TRACK_XP_DAILY_MAX + per_game, per_game)
    );
}

#[tokio::test]
async fn r3335001_a_level_grants_its_reward_once_with_reason_track() {
    let app = test_app().await;
    let a = profile_id_of(&app, USER).await;
    let at = now_ms();
    let launch = i64::from(FLEET_TRACK_XP_PER_LAUNCH);
    // A dev grant puts the track one launch short of level 1, outside the day's game XP.
    assert!(store!(app, |t| t.economy_fleet_grant(&FleetGrant {
        profile_id: a.clone(),
        xp: i64::from(FLEET_TRACK_XP_PER_LEVEL) - launch,
        levels: 0,
        reason: EconomyReason::Dev,
        reference: None,
        dev: true,
        at,
    })));
    assert_eq!(track_of(&app, &a, at).await.xp_today, 0);
    pay(
        &app,
        &game(GameMode::Ladder, &a, "m-level", at, false, launches(1)),
    )
    .await;
    let level_1 = fleet_reward(1).expect("level 1 pays a cosmetic").id.clone();
    let track = track_of(&app, &a, at).await;
    assert_eq!((track.level, track.xp), (1, i64::from(FLEET_TRACK_XP_PER_LEVEL)));
    let (rows, rewards) = ledger_of(&app, &a).await;
    assert_eq!(rows.last().map(|row| (row.xp, row.levels)), Some((launch, 1)));
    assert_eq!(rewards, vec![(level_1.clone(), Some("m-level".to_owned()))]);
    let owned = store!(app, |t| t.economy_cosmetics_get(&a));
    assert!(owned.account.contains(&level_1));
    // The next game reaches no new level and pays no reward again.
    pay(
        &app,
        &game(GameMode::Ladder, &a, "m-after", at, false, launches(1)),
    )
    .await;
    let (_, rewards) = ledger_of(&app, &a).await;
    assert_eq!(rewards, vec![(level_1, Some("m-level".to_owned()))]);
    assert_eq!(track_of(&app, &a, at).await.level, 1);
}

#[tokio::test]
async fn r3335001_get_tracks_answers_the_fleet_track() {
    let app = test_app().await;
    let a = profile_id_of(&app, USER).await;
    let fresh = |body: &Value| body["fleet"].clone();
    let (status, _, body) = call(&app, "GET", "/api/tracks", Some(TOKEN), Value::Null).await;
    assert_eq!(status, 200, "{body}");
    let first: Vec<String> = (1..=3)
        .filter_map(fleet_reward)
        .map(|cosmetic| cosmetic.id.clone())
        .collect();
    assert_eq!(
        fresh(&body),
        json!({
            "level": 0,
            "xp": 0,
            "levelXp": FLEET_TRACK_XP_PER_LEVEL,
            "xpToNext": FLEET_TRACK_XP_PER_LEVEL,
            "xpToday": 0,
            "nextRewards": first,
        })
    );
    // A game today moves it, its XP today among it.
    pay(
        &app,
        &game(GameMode::Ladder, &a, "m-get", now_ms(), false, launches(1)),
    )
    .await;
    let (_, _, body) = call(&app, "GET", "/api/tracks", Some(TOKEN), Value::Null).await;
    let launch = i64::from(FLEET_TRACK_XP_PER_LAUNCH);
    assert_eq!(fresh(&body)["xp"], json!(launch));
    assert_eq!(fresh(&body)["xpToday"], json!(launch));
    assert_eq!(
        fresh(&body)["xpToNext"],
        json!(i64::from(FLEET_TRACK_XP_PER_LEVEL) - launch)
    );
    // At the last level there is no next: no bar's span, nothing to name.
    assert!(store!(app, |t| t.economy_fleet_grant(&FleetGrant {
        profile_id: a.clone(),
        xp: i64::from(FLEET_TRACK_LEVELS) * i64::from(FLEET_TRACK_XP_PER_LEVEL),
        levels: i64::from(FLEET_TRACK_LEVELS),
        reason: EconomyReason::Dev,
        reference: None,
        dev: true,
        at: now_ms(),
    })));
    let (_, _, body) = call(&app, "GET", "/api/tracks", Some(TOKEN), Value::Null).await;
    assert_eq!(fresh(&body)["level"], json!(FLEET_TRACK_LEVELS));
    assert_eq!(fresh(&body)["levelXp"], Value::Null);
    assert_eq!(fresh(&body)["xpToNext"], Value::Null);
    assert_eq!(fresh(&body)["nextRewards"], json!([]));
}
