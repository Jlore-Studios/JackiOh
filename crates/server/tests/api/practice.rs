//! Practice earning (SPEC §14.8, #1303): a practice game pays half rates once the server has
//! checked it, on a seed it issued (`POST /api/practice/seeds`, `POST /api/practice/claims`).
//!
//! An honest claim pays `JLORE_PRACTICE_WIN` or `JLORE_PRACTICE_LOSS` and the practice XP; a
//! forged result, a forged AI move, another seed's log, an unfinished log, a sandbox tool, a seed
//! claimed before, a lapsed seed and a pruned one pay nothing and say why (R1303000–R1303003, whose
//! proving tests live here). A claim past `PRACTICE_VERIFY_CONCURRENCY` waits and still pays, and
//! so does one whose caller stops waiting. A paid game moves the account's quests by what it did,
//! as a ladder game does (R1304001). The day's Jlore cap holds a game's Jlore back and says so,
//! and an account holds only `PRACTICE_SEEDS_OPEN_MAX` open seeds (R1483000).
//!
//! The games are played for real, on the shipped catalog (no `install_test_cards`), through
//! `economy::practice`'s own `opening` and `ai_action`, so each AI move in a log is the one the
//! server re-runs; the fixture test plays one the browser's worker recorded instead. Every amount
//! is read through its constant and the hook's own math, so the tests hold whatever #1333 sets.

use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::{Value, json};

use jackioh_engine::wire::newest_shipped_set;
use jackioh_engine::{Action, ActionBody, Difficulty, GameState, LastBoardEntry, PlayerId, Rng, SandboxOp};
use jackioh_server::actor::engine::{self, Pinned};
use jackioh_server::app::{App, now_ms};
use jackioh_server::config::{
    JLORE_GAMES_DAILY_CAP, JLORE_PRACTICE_LOSS, JLORE_PRACTICE_WIN, PRACTICE_SEED_TTL_SECONDS,
    PRACTICE_SEEDS_OPEN_MAX, PRACTICE_VERIFY_CONCURRENCY, SET_TRACK_FIRST_LEVEL, SET_TRACK_LEVELS,
    STARTER_CARDS,
};
use jackioh_server::db::fake::{E2eStoreOptions, FakeCatalog, create_e2e_store};
use jackioh_server::db::store::{LastBoardKind, ProfileStatus};
use jackioh_server::economy::math::{level_span_xp, xp_to_next_level};
use jackioh_server::economy::practice::{self, ClaimedResult};
use jackioh_server::economy::quests::{QuestDef, QuestGoal, pool, quest_interval_ms};
use jackioh_server::economy::rewards::{GameMode, game_xp, is_player_action, track_level};
use jackioh_server::economy::store::{
    EconomyLedger, EconomyReason, JloreGrant, JloreReason, PracticeDeck, PracticeSeed, PracticeSetup,
    QuestChange, QuestGrant,
};
use jackioh_server::economy::track_rewards::reward_for_level;

use crate::support::deps::{add_user, call, test_app as built_app};
use crate::support::store::{fake_ref, store};

const A: &str = "profile-a";
const USER_A: &str = "user-a";
/// Most actions a test game takes before it is called stuck.
const MOVE_CAP: usize = 4000;
/// How many seeds `honest_win` tries for a game the human wins.
const WIN_SEEDS: usize = 32;
/// A seed's lifetime in milliseconds, through its constant.
const TTL_MS: i64 = PRACTICE_SEED_TTL_SECONDS * 1000;

/// The test app with the economy on (§14.12, R1302006), as `ECONOMY_LIVE` builds it since the
/// economy went live (R1310000); set here, so a payout test holds whatever the build's switch says.
async fn test_app() -> Arc<App> {
    economy_app(true).await
}

/// The test app with the economy switched to `live`.
async fn economy_app(live: bool) -> Arc<App> {
    let mut app = built_app().await;
    Arc::get_mut(&mut app)
        .expect("the test app has no second handle yet")
        .economy_live = live;
    app
}

/// An app with A as a signed-in active account and nothing else in its store, and A's token.
async fn authed_with(app: Arc<App>) -> (Arc<App>, String) {
    fake_ref(&app).lock().await.reset();
    let _ = fake_ref(&app)
        .lock()
        .await
        .seed_profile(json!({ "id": A, "userId": USER_A, "status": "active" }));
    let token = add_user(&app, USER_A, "a@example.test", true);
    (app, token)
}

async fn authed() -> (Arc<App>, String) {
    authed_with(test_app().await).await
}

fn catalog_version() -> &'static str {
    jackioh_cards::catalog_version()
}

fn pinned() -> Pinned {
    Pinned::of(catalog_version()).expect("the build folds its own catalog")
}

/// A random-deck game at `difficulty`, the human second.
fn setup(difficulty: Difficulty, last_board: Vec<LastBoardEntry>) -> PracticeSetup {
    PracticeSetup {
        difficulty,
        human_seat: PlayerId::P2,
        deck: PracticeDeck::Random { lean: None },
        last_board,
        catalog_version: catalog_version().to_string(),
    }
}

/// `POST /api/practice/seeds` for an Easy random-deck game, the human second: the status and the
/// answer, whatever they are.
async fn request_seed(app: &Arc<App>, token: &str) -> (u16, Value) {
    let (status, _, body) = call(
        app,
        "POST",
        "/api/practice/seeds",
        Some(token),
        json!({
            "difficulty": "easy", "humanSeat": "p2", "deck": { "kind": "random", "lean": null },
            "catalog": catalog_version(),
        }),
    )
    .await;
    (status, body)
}

/// A seed issued: its name and the whole answer.
async fn issue(app: &Arc<App>, token: &str) -> (String, Value) {
    let (status, body) = request_seed(app, token).await;
    assert_eq!(status, 201, "{body}");
    let seed = body["seed"].as_str().expect("a seed").to_string();
    (seed, body)
}

/// A seed row written without the route, issued at `issued_at`.
async fn insert(app: &Arc<App>, seed: &str, setup: &PracticeSetup, issued_at: i64) {
    store!(app, |t| t.economy_practice_seed_insert(&PracticeSeed {
        seed: seed.to_string(),
        profile_id: A.to_string(),
        setup: setup.clone(),
        issued_at,
        claimed_at: None,
    }));
}

/// The setup the server kept with A's seed.
async fn kept_setup(app: &Arc<App>, seed: &str) -> PracticeSetup {
    store!(app, |t| t.economy_practice_seed_get(A, seed))
        .expect("the seed is held")
        .setup
}

/// `POST /api/practice/claims`: the status-200 answer.
async fn claim(app: &Arc<App>, token: &str, seed: &str, result: &str, log: &[Action]) -> Value {
    let (status, _, body) = call(
        app,
        "POST",
        "/api/practice/claims",
        Some(token),
        json!({ "seed": seed, "result": result, "log": log }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    body
}

fn result_name(result: ClaimedResult) -> &'static str {
    match result {
        ClaimedResult::Win => "win",
        ClaimedResult::Loss => "loss",
        ClaimedResult::Draw => "draw",
    }
}

/// How the human plays a test game.
#[derive(Clone, Copy)]
enum Human {
    /// Concedes at its first move.
    Concede,
    /// Plays as the practice AI does, on its own stream `"{seed}:human"`.
    Ai,
}

/// A practice game played out from `setup` on `seed` as the worker plays it: the AI moves whenever
/// it owes one, through `practice::ai_action`, and the human otherwise, each under its own nonces
/// from the seat it plays then (R677). The log, the human's result and the final state.
fn play(seed: &str, setup: &PracticeSetup, human: Human) -> (Vec<Action>, ClaimedResult, GameState) {
    let pinned = pinned();
    let mut state = practice::opening(pinned, seed, setup).expect("the setup begins a game");
    let (human_home, ai_home) = (setup.human_seat, setup.human_seat.opponent());
    let mut ai_rng = Rng::new(&format!("{seed}:ai"), 0);
    let mut human_rng = Rng::new(&format!("{seed}:human"), 0);
    let (mut humans, mut ais) = (0, 0);
    let mut log = Vec::new();
    while state.result.is_none() {
        assert!(
            log.len() < MOVE_CAP,
            "the game on {seed} ends within {MOVE_CAP} actions"
        );
        let ai_seat = engine::seat_played_by(&state, ai_home);
        let ai_nonce = format!("a{ais}");
        let action = match practice::ai_action(pinned, &state, ai_seat, &ai_nonce, &mut ai_rng) {
            Some(body) => {
                ais += 1;
                Action::new(body, ai_seat, ai_nonce)
            }
            None => {
                let seat = engine::seat_played_by(&state, human_home);
                let nonce = format!("h{humans}");
                let body = match human {
                    Human::Concede => ActionBody::Concede,
                    Human::Ai => practice::ai_action(pinned, &state, seat, &nonce, &mut human_rng)
                        .expect("the human owes an action whenever the AI owes none"),
                };
                humans += 1;
                Action::new(body, seat, nonce)
            }
        };
        let next = engine::reduce(pinned, &state, &action);
        assert!(next.error.is_none(), "{action:?} is refused: {:?}", next.error);
        state = next.state;
        log.push(action);
    }
    let result = state.result.as_ref().expect("the game ended");
    let human_now = engine::seat_played_by(&state, human_home);
    let claimed = match result.winner {
        jackioh_engine::Winner::Draw => ClaimedResult::Draw,
        winner if winner == jackioh_engine::Winner::from(human_now) => ClaimedResult::Win,
        _ => ClaimedResult::Loss,
    };
    (log, claimed, state)
}

/// A won game: its seed, setup, log and final state.
type Won = (String, PracticeSetup, Vec<Action>, GameState);

/// The first `practice-win-{n}` Easy game the human, playing as the AI does, wins; played once for
/// every test that needs it, since each of its AI moves is a full search.
fn honest_win() -> &'static Won {
    static WON: OnceLock<Won> = OnceLock::new();
    WON.get_or_init(|| {
        let setup = setup(Difficulty::Easy, Vec::new());
        for n in 0..WIN_SEEDS {
            let seed = format!("practice-win-{n}");
            let (log, result, state) = play(&seed, &setup, Human::Ai);
            if result == ClaimedResult::Win {
                return (seed, setup, log, state);
            }
        }
        panic!("the human wins none of {WIN_SEEDS} Easy games");
    })
}

/// The human's actions that earn XP (R1302004): the `h…` rows the hook counts.
fn human_actions(log: &[Action]) -> i64 {
    log.iter()
        .filter(|action| action.nonce.starts_with('h') && is_player_action(&action.body))
        .count() as i64
}

async fn ledger(app: &Arc<App>) -> EconomyLedger {
    store!(app, |t| t.economy_ledger_get(A))
}

/// A's ledger holds exactly one paid practice game on `seed`: its Jlore, as a game's, and its XP
/// on the newest set's track.
async fn assert_paid(app: &Arc<App>, seed: &str, jlore: i32, xp: i64) {
    let ledger = ledger(app).await;
    let reference = Some(format!("practice:{seed}"));
    let games: Vec<(i64, Option<String>)> = ledger
        .jlore
        .iter()
        .filter(|grant| grant.reason == JloreReason::Game)
        .map(|grant| (grant.delta, grant.reference.clone()))
        .collect();
    assert_eq!(games, vec![(i64::from(jlore), reference.clone())]);
    assert!(!ledger.tracks.is_empty(), "the game's XP lands on a track");
    for track in &ledger.tracks {
        assert_eq!((track.xp, &track.reference), (xp, &reference));
    }
}

/// A's ledger holds nothing: no Jlore, no XP.
async fn assert_unpaid(app: &Arc<App>) {
    let ledger = ledger(app).await;
    assert!(ledger.jlore.is_empty(), "{:?}", ledger.jlore);
    assert!(ledger.tracks.is_empty(), "{:?}", ledger.tracks);
}

fn refusal(reason: &str) -> Value {
    json!({ "paid": false, "reason": reason })
}

/// A paid claim's answer for A's first paid game (#1657), its Jlore `jlore` and its XP `xp`: the
/// newest set's track moved from its first level with no XP, each level that XP reached paid, the
/// first among them, and no quest completed, since none has arrived yet (R1304000).
fn first_paid(jlore: impl Into<i64>, xp: i64, capped: bool) -> Value {
    let point = |level: i64, xp: i64| {
        let next = level < i64::from(SET_TRACK_LEVELS);
        json!({
            "level": level,
            "xp": xp,
            "levelXp": next.then(|| level_span_xp(xp)),
            "xpToNext": next.then(|| xp_to_next_level(xp)),
        })
    };
    let level = track_level(xp);
    let levels: Vec<Value> = (SET_TRACK_FIRST_LEVEL..=level)
        .map(|level| json!({ "level": level, "rewards": reward_for_level(level) }))
        .collect();
    json!({
        "paid": true,
        "jlore": jlore.into(),
        "xp": xp,
        "jloreCapped": capped,
        "tracks": [{
            "setId": newest_shipped_set().as_str(),
            "before": point(SET_TRACK_FIRST_LEVEL, 0),
            "after": point(level, xp),
            "levels": levels,
        }],
        "quests": [],
    })
}

// ---------------------------------------------------------------------------
// Honest games pay practice rates.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn an_honest_loss_pays_practice_rates() {
    let (app, token) = authed().await;
    let (seed, issued) = issue(&app, &token).await;
    assert_eq!(issued["lastBoard"], json!([]));
    assert!(issued["expiresAt"].as_i64().expect("an expiry") > now_ms());
    let setup = kept_setup(&app, &seed).await;
    assert_eq!(setup, self::setup(Difficulty::Easy, Vec::new()));

    let (log, result, _) = play(&seed, &setup, Human::Concede);
    assert_eq!(result, ClaimedResult::Loss);
    let xp = game_xp(GameMode::Practice, human_actions(&log), false);
    assert_eq!(
        claim(&app, &token, &seed, "loss", &log).await,
        first_paid(JLORE_PRACTICE_LOSS, xp, false)
    );
    assert_paid(&app, &seed, JLORE_PRACTICE_LOSS, xp).await;
}

#[tokio::test]
async fn an_honest_win_pays_practice_rates() {
    let (app, token) = authed().await;
    let (seed, setup, log, _) = honest_win();
    insert(&app, seed, setup, now_ms()).await;
    let xp = game_xp(GameMode::Practice, human_actions(log), true);
    assert_eq!(
        claim(&app, &token, seed, "win", log).await,
        first_paid(JLORE_PRACTICE_WIN, xp, false)
    );
    assert_paid(&app, seed, JLORE_PRACTICE_WIN, xp).await;
}

#[tokio::test]
async fn a_game_on_a_list_of_cards_pays_too() {
    let (app, token) = authed().await;
    let cards = engine::deal_random_deck("practice-cards-deck", None);
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/practice/seeds",
        Some(&token),
        json!({
            "difficulty": "hard", "humanSeat": "p1", "deck": { "kind": "cards", "cards": cards },
            "catalog": catalog_version(),
        }),
    )
    .await;
    assert_eq!(status, 201, "{body}");
    let seed = body["seed"].as_str().expect("a seed").to_string();
    let setup = kept_setup(&app, &seed).await;
    assert_eq!(setup.deck, PracticeDeck::Cards { cards });
    let (log, _, _) = play(&seed, &setup, Human::Concede);
    let xp = game_xp(GameMode::Practice, human_actions(&log), false);
    assert_eq!(
        claim(&app, &token, &seed, "loss", &log).await,
        first_paid(JLORE_PRACTICE_LOSS, xp, false)
    );
}

/// The test app over a store that carries R111's activation grant over the real catalog, as the
/// end-to-end server builds its store (`app::build`).
async fn granting_app() -> Arc<App> {
    let app = Arc::try_unwrap(test_app().await)
        .unwrap_or_else(|_| panic!("test_app() hands back its only reference"));
    let (tokens, bans) = (app.catalog.clone(), app.catalog.clone());
    let db = create_e2e_store(E2eStoreOptions {
        catalog: FakeCatalog {
            card_ids: app.catalog.card_ids.clone(),
            is_token: Arc::new(move |card_id: &str| tokens.is_token(card_id)),
            is_banned: Arc::new(move |card_id: &str| bans.is_banned(card_id)),
        },
        now: Arc::new(now_ms),
        redemption: None,
    });
    Arc::new(App { db, ..app })
}

#[tokio::test]
async fn r1310004_practice_takes_cards_the_account_does_not_own() {
    let (app, token) = authed_with(granting_app().await).await;
    // A becomes active again as a new account does, owning the starter cards alone (R1310000).
    store!(app, |t| t.profiles_set_status(A, ProfileStatus::Pending));
    store!(app, |t| t.profiles_set_status(A, ProfileStatus::Active));
    let owned: Vec<String> = store!(app, |t| t.collection_get(A))
        .into_iter()
        .filter(|entry| entry.quantity > 0)
        .map(|entry| entry.card_id)
        .collect();
    assert!(!owned.is_empty() && owned.iter().all(|id| STARTER_CARDS.contains(&id.as_str())));
    // A deck of collectable cards A does not own, none of them a starter: practice reads no
    // collection (§14.12), so the seed is issued and the game pays.
    let size = usize::try_from(jackioh_engine::config::DECK_SIZE).expect("a deck size");
    let cards: Vec<String> = app
        .catalog
        .card_ids
        .iter()
        .filter(|id| {
            !app.catalog.is_token(id)
                && !app.catalog.is_banned(id)
                && app
                    .catalog
                    .defs
                    .get(*id)
                    .is_some_and(|def| jackioh_engine::set_ships(def.set))
                && !owned.contains(id)
        })
        .take(size)
        .cloned()
        .collect();
    assert_eq!(cards.len(), size);
    let (status, _, body) = call(
        &app,
        "POST",
        "/api/practice/seeds",
        Some(&token),
        json!({
            "difficulty": "easy", "humanSeat": "p1", "deck": { "kind": "cards", "cards": cards },
            "catalog": catalog_version(),
        }),
    )
    .await;
    assert_eq!(status, 201, "{body}");
    let seed = body["seed"].as_str().expect("a seed").to_string();
    let setup = kept_setup(&app, &seed).await;
    let (log, _, _) = play(&seed, &setup, Human::Concede);
    let xp = game_xp(GameMode::Practice, human_actions(&log), false);
    assert_eq!(
        claim(&app, &token, &seed, "loss", &log).await,
        first_paid(JLORE_PRACTICE_LOSS, xp, false)
    );
}

/// The pool's entry of `goal` with the largest target.
fn hardest(goal: &QuestGoal) -> &'static QuestDef {
    pool()
        .iter()
        .filter(|quest| quest.goal == *goal)
        .max_by_key(|quest| quest.target)
        .unwrap_or_else(|| panic!("the pool has a {goal:?} quest"))
}

#[tokio::test]
async fn r1304001_a_paid_practice_game_moves_the_quests_by_what_it_did() {
    let (app, token) = authed().await;
    let (seed, setup, log, _) = honest_win();
    insert(&app, seed, setup, now_ms()).await;
    // No quest falls due during the claim, and A holds a quest of cards played and one of attacks,
    // which only what the game did moves: a summary with no plays and no attacks moves neither.
    store!(app, |t| t.economy_quests_set_next_due(
        A,
        Some(now_ms() + 10 * quest_interval_ms())
    ));
    let (cards, attack) = (
        hardest(&QuestGoal::PlayCards),
        hardest(&QuestGoal::AttackWithUnits),
    );
    for def in [cards, attack] {
        assert!(store!(app, |t| t.economy_quests_change(&QuestGrant {
            profile_id: A.to_owned(),
            quest_id: def.id.clone(),
            change: QuestChange::Arrived,
            progress: 0,
            reason: EconomyReason::Dev,
            reference: None,
            dev: true,
            at: now_ms(),
        })));
    }
    let attacks = log
        .iter()
        .filter(|action| action.nonce.starts_with('h') && matches!(action.body, ActionBody::Attack { .. }))
        .count() as i64;
    assert!(attacks > 0, "the human attacks on its way to the win");
    assert_eq!(claim(&app, &token, seed, "win", log).await["paid"], json!(true));

    let reference = format!("practice:{seed}");
    let moved = |id: &str, quests: &[QuestGrant]| {
        quests
            .iter()
            .filter(|row| row.quest_id == id && row.change == QuestChange::Progressed)
            .map(|row| (row.progress, row.reference.clone()))
            .collect::<Vec<_>>()
    };
    let quests = ledger(&app).await.quests;
    assert_eq!(
        moved(&attack.id, &quests),
        vec![(attacks.min(attack.target), Some(reference.clone()))],
        "each of the human's attacks counts"
    );
    let played = moved(&cards.id, &quests);
    assert_eq!(played.len(), 1, "{quests:?}");
    assert!(
        (1..=cards.target).contains(&played[0].0) && played[0].1 == Some(reference),
        "the cards the human played count: {played:?}"
    );
}

/// The newest set's track as `GET /api/tracks` serves it to A: its `level`, `xp`, `levelXp` and
/// `xpToNext`, and how many levels have paid.
async fn newest_track(app: &Arc<App>, token: &str) -> (Value, i64) {
    let (status, _, body) = call(app, "GET", "/api/tracks", Some(token), Value::Null).await;
    assert_eq!(status, 200, "{body}");
    let track = body["tracks"]
        .as_array()
        .expect("the tracks")
        .iter()
        .find(|track| track["setId"] == newest_shipped_set().as_str())
        .cloned()
        .expect("the newest track");
    let point = json!({
        "level": track["level"],
        "xp": track["xp"],
        "levelXp": track["levelXp"],
        "xpToNext": track["xpToNext"],
    });
    (point, track["rewardsClaimed"].as_i64().expect("a count"))
}

#[tokio::test]
async fn a_paid_claim_answers_the_tracks_it_moved_and_the_quests_it_completed() {
    let (app, token) = authed().await;
    let (seed, setup, log, _) = honest_win();
    insert(&app, seed, setup, now_ms()).await;
    // A holds the pool's one-win quest, which this game completes, and none falls due meanwhile.
    store!(app, |t| t.economy_quests_set_next_due(
        A,
        Some(now_ms() + 10 * quest_interval_ms())
    ));
    let win = pool()
        .iter()
        .filter(|quest| quest.goal == QuestGoal::WinGames)
        .min_by_key(|quest| quest.target)
        .expect("the pool has a win quest");
    assert_eq!(win.target, 1, "one win completes it");
    assert!(store!(app, |t| t.economy_quests_change(&QuestGrant {
        profile_id: A.to_owned(),
        quest_id: win.id.clone(),
        change: QuestChange::Arrived,
        progress: 0,
        reason: EconomyReason::Dev,
        reference: None,
        dev: true,
        at: now_ms(),
    })));
    let (status, _, held) = call(&app, "GET", "/api/quests", Some(&token), Value::Null).await;
    assert_eq!(status, 200, "{held}");
    let named = &held["quests"][0];
    assert_eq!(named["questId"], json!(win.id));
    let (was, claimed) = newest_track(&app, &token).await;

    let answer = claim(&app, &token, seed, "win", log).await;
    let (now, now_claimed) = newest_track(&app, &token).await;
    let levels: Vec<Value> = (claimed + 1..=now_claimed)
        .map(|level| json!({ "level": level, "rewards": reward_for_level(level) }))
        .collect();
    assert!(!levels.is_empty(), "the game and the quest reach a level");
    assert_eq!(
        answer,
        json!({
            "paid": true,
            "jlore": JLORE_PRACTICE_WIN,
            "xp": game_xp(GameMode::Practice, human_actions(log), true),
            "jloreCapped": false,
            "tracks": [{
                "setId": newest_shipped_set().as_str(),
                "before": was,
                "after": now,
                "levels": levels,
            }],
            "quests": [{
                "questId": named["questId"],
                "title": named["title"],
                "jlore": named["jlore"],
                "xp": named["xp"],
            }],
        })
    );
}

// ---------------------------------------------------------------------------
// A forged claim pays nothing, and says why.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_deck_the_engine_refuses_pays_nothing() {
    let (app, token) = authed().await;
    let setup = PracticeSetup {
        deck: PracticeDeck::Cards {
            cards: vec!["no-such-card".to_string()],
        },
        ..setup(Difficulty::Easy, Vec::new())
    };
    insert(&app, "practice-bad-deck", &setup, now_ms()).await;
    let log = [Action::new(ActionBody::Concede, PlayerId::P2, "h0")];
    assert_eq!(
        claim(&app, &token, "practice-bad-deck", "loss", &log).await,
        refusal("does_not_fold")
    );
    assert_unpaid(&app).await;
}

#[tokio::test]
async fn a_forged_result_pays_nothing() {
    let (app, token) = authed().await;
    let (seed, _) = issue(&app, &token).await;
    let (log, _, _) = play(&seed, &kept_setup(&app, &seed).await, Human::Concede);
    assert_eq!(
        claim(&app, &token, &seed, "win", &log).await,
        refusal("result_mismatch")
    );
    assert_unpaid(&app).await;
}

#[tokio::test]
async fn r1303002_a_forged_ai_move_pays_nothing() {
    let (app, token) = authed().await;
    let (seed, _) = issue(&app, &token).await;
    let (mut log, _, _) = play(&seed, &kept_setup(&app, &seed).await, Human::Concede);
    // The AI's first move swapped for one it never makes: an End turn it did not take, or (when it
    // did) a concede, which R188 forbids it.
    let first = log
        .iter_mut()
        .find(|action| action.nonce.starts_with('a'))
        .expect("the AI moved");
    first.body = match first.body {
        ActionBody::EndTurn => ActionBody::Concede,
        _ => ActionBody::EndTurn,
    };
    assert_eq!(
        claim(&app, &token, &seed, "loss", &log).await,
        refusal("ai_mismatch")
    );
    assert_unpaid(&app).await;
}

#[tokio::test]
async fn a_log_from_another_seed_pays_nothing() {
    let (app, token) = authed().await;
    let (seed, setup, log, _) = honest_win();
    let other = format!("{seed}-other");
    insert(&app, &other, setup, now_ms()).await;
    let answer = claim(&app, &token, &other, "win", log).await;
    assert!(
        answer == refusal("does_not_fold") || answer == refusal("ai_mismatch"),
        "{answer}"
    );
    assert_unpaid(&app).await;
}

#[tokio::test]
async fn an_unfinished_log_pays_nothing() {
    let (app, token) = authed().await;
    let (seed, _) = issue(&app, &token).await;
    let (mut log, _, _) = play(&seed, &kept_setup(&app, &seed).await, Human::Concede);
    log.pop();
    assert_eq!(
        claim(&app, &token, &seed, "loss", &log).await,
        refusal("not_finished")
    );
    assert_unpaid(&app).await;
}

#[tokio::test]
async fn r1303000_a_sandbox_tool_in_a_claim_pays_nothing() {
    let (app, token) = authed().await;
    let (seed, _) = issue(&app, &token).await;
    let (log, _, _) = play(&seed, &kept_setup(&app, &seed).await, Human::Concede);
    // The human's first action a tool that sets the AI's hero to 1 health, as a sandbox game would
    // take it, then the concede: the server folds every claim as a plain game, which refuses it.
    let mut forged: Vec<Action> = log
        .iter()
        .take_while(|action| action.nonce.starts_with('a'))
        .cloned()
        .collect();
    let human = PlayerId::P2;
    forged.push(Action::new(
        ActionBody::Sandbox {
            op: SandboxOp::SetHero {
                player: human.opponent(),
                health: Some(1),
                armor: None,
            },
        },
        human,
        "h0",
    ));
    forged.push(Action::new(ActionBody::Concede, human, "h1"));
    assert_eq!(
        claim(&app, &token, &seed, "loss", &forged).await,
        refusal("does_not_fold")
    );
    assert_unpaid(&app).await;
}

// ---------------------------------------------------------------------------
// A seed's lifetime (R1303001).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn r1303001_a_seed_pays_once() {
    let (app, token) = authed().await;
    let (seed, _) = issue(&app, &token).await;
    let (log, _, _) = play(&seed, &kept_setup(&app, &seed).await, Human::Concede);
    assert_eq!(
        claim(&app, &token, &seed, "loss", &log).await["paid"],
        json!(true)
    );
    assert_eq!(
        claim(&app, &token, &seed, "loss", &log).await,
        refusal("already_claimed")
    );
    let xp = game_xp(GameMode::Practice, human_actions(&log), false);
    assert_paid(&app, &seed, JLORE_PRACTICE_LOSS, xp).await;
}

#[tokio::test]
async fn r1303001_a_refused_claim_burns_its_seed() {
    let (app, token) = authed().await;
    let (seed, _) = issue(&app, &token).await;
    let (log, _, _) = play(&seed, &kept_setup(&app, &seed).await, Human::Concede);
    assert_eq!(
        claim(&app, &token, &seed, "win", &log).await,
        refusal("result_mismatch")
    );
    assert_eq!(
        claim(&app, &token, &seed, "loss", &log).await,
        refusal("already_claimed")
    );
    assert_unpaid(&app).await;
}

#[tokio::test]
async fn r1303001_an_expired_seed_pays_nothing() {
    let (app, token) = authed().await;
    let seed = "practice-expired";
    let setup = setup(Difficulty::Easy, Vec::new());
    insert(&app, seed, &setup, now_ms() - TTL_MS - 1).await;
    let (log, _, _) = play(seed, &setup, Human::Concede);
    assert_eq!(claim(&app, &token, seed, "loss", &log).await, refusal("expired"));
    assert_unpaid(&app).await;
}

#[tokio::test]
async fn r1303001_an_abandoned_seed_is_pruned_by_the_next_issue() {
    let (app, token) = authed().await;
    let abandoned = "practice-abandoned";
    let setup = setup(Difficulty::Easy, Vec::new());
    insert(&app, abandoned, &setup, now_ms() - TTL_MS - 1).await;
    let (fresh, _) = issue(&app, &token).await;
    assert!(store!(app, |t| t.economy_practice_seed_get(A, abandoned)).is_none());
    assert!(store!(app, |t| t.economy_practice_seed_get(A, &fresh)).is_some());
    let (log, _, _) = play(abandoned, &setup, Human::Concede);
    assert_eq!(
        claim(&app, &token, abandoned, "loss", &log).await,
        refusal("unknown_seed")
    );
    assert_unpaid(&app).await;
}

#[tokio::test]
async fn r1303001_another_accounts_seed_is_unknown() {
    let (app, token) = authed().await;
    let setup = setup(Difficulty::Easy, Vec::new());
    let _ = fake_ref(&app)
        .lock()
        .await
        .seed_profile(json!({ "id": "profile-b", "userId": "user-b", "status": "active" }));
    store!(app, |t| t.economy_practice_seed_insert(&PracticeSeed {
        seed: "practice-theirs".to_string(),
        profile_id: "profile-b".to_string(),
        setup: setup.clone(),
        issued_at: now_ms(),
        claimed_at: None,
    }));
    let (log, _, _) = play("practice-theirs", &setup, Human::Concede);
    assert_eq!(
        claim(&app, &token, "practice-theirs", "loss", &log).await,
        refusal("unknown_seed")
    );
    assert_unpaid(&app).await;
}

// ---------------------------------------------------------------------------
// The last board (R1303003).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn r1303003_a_paid_game_leaves_the_board_the_next_seed_brings() {
    let (app, token) = authed().await;
    let (seed, setup, log, state) = honest_win();
    insert(&app, seed, setup, now_ms()).await;
    let (p1, p2) = engine::last_boards(state);
    let board = match engine::seat_played_by(state, setup.human_seat) {
        PlayerId::P1 => p1,
        PlayerId::P2 => p2,
    };
    assert!(!board.is_empty(), "the won game leaves the human a board");
    assert_eq!(claim(&app, &token, seed, "win", log).await["paid"], json!(true));
    assert_eq!(
        store!(app, |t| t.last_boards_get(A, LastBoardKind::Practice)),
        Some(board.clone())
    );

    let (next, issued) = issue(&app, &token).await;
    assert_eq!(issued["lastBoard"], json!(board));
    let kept = kept_setup(&app, &next).await;
    assert_eq!(kept.last_board, board);
    // The next game folds with it, and pays.
    let (log, result, _) = play(&next, &kept, Human::Concede);
    assert_eq!(
        claim(&app, &token, &next, result_name(result), &log).await["paid"],
        json!(true)
    );
}

#[tokio::test]
async fn r1303003_a_refused_claim_leaves_no_board() {
    let (app, token) = authed().await;
    let (seed, _) = issue(&app, &token).await;
    let (log, _, _) = play(&seed, &kept_setup(&app, &seed).await, Human::Concede);
    assert_eq!(
        claim(&app, &token, &seed, "win", &log).await,
        refusal("result_mismatch")
    );
    assert_eq!(
        store!(app, |t| t.last_boards_get(A, LastBoardKind::Practice)),
        None
    );
}

// ---------------------------------------------------------------------------
// The switch, the cap, and the browser's own game.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn nothing_is_issued_or_paid_while_the_economy_is_off() {
    // The economy switched off (§14.12), as production ran until launch (R1310000).
    let (app, token) = authed_with(economy_app(false).await).await;
    let (status, _, _) = call(
        &app,
        "POST",
        "/api/practice/seeds",
        Some(&token),
        json!({
            "difficulty": "easy", "humanSeat": "p2", "deck": { "kind": "random", "lean": null },
            "catalog": catalog_version(),
        }),
    )
    .await;
    assert_eq!(status, 409);
    let setup = setup(Difficulty::Easy, Vec::new());
    insert(&app, "practice-off", &setup, now_ms()).await;
    let (log, _, _) = play("practice-off", &setup, Human::Concede);
    assert_eq!(
        claim(&app, &token, "practice-off", "loss", &log).await,
        refusal("economy_off")
    );
    assert_unpaid(&app).await;
}

#[tokio::test]
async fn a_seed_for_a_catalog_the_server_cannot_fold_is_refused() {
    let (app, token) = authed().await;
    let (status, _, _) = call(
        &app,
        "POST",
        "/api/practice/seeds",
        Some(&token),
        json!({
            "difficulty": "easy", "humanSeat": "p2", "deck": { "kind": "random", "lean": null },
            "catalog": "no-such-catalog",
        }),
    )
    .await;
    assert_eq!(status, 409);
    assert!(
        fake_ref(&app)
            .lock()
            .await
            .tables
            .economy
            .practice_seeds
            .is_empty()
    );
}

#[tokio::test]
async fn an_anonymous_caller_is_refused() {
    let (app, _) = authed().await;
    let (status, _, _) = call(&app, "POST", "/api/practice/seeds", None, json!({})).await;
    assert_eq!(status, 401);
    let (status, _, _) = call(&app, "POST", "/api/practice/claims", None, json!({})).await;
    assert_eq!(status, 401);
}

#[tokio::test]
async fn a_claim_over_the_cap_waits_and_still_pays() {
    let (app, token) = authed().await;
    let (seed, _) = issue(&app, &token).await;
    let (log, _, _) = play(&seed, &kept_setup(&app, &seed).await, Human::Concede);
    // How long this claim's check takes with a permit free, so the wait below outlasts it.
    let started = Instant::now();
    let free = practice::verify(
        pinned(),
        &seed,
        &kept_setup(&app, &seed).await,
        ClaimedResult::Loss,
        &log,
    );
    assert!(free.is_ok(), "{free:?}");
    let check = started.elapsed();

    let held = app
        .practice_verifications
        .acquire_many(PRACTICE_VERIFY_CONCURRENCY as u32)
        .await
        .expect("the permits are open");
    let mut waiting = {
        let (app, token, seed) = (app.clone(), token.clone(), seed.clone());
        tokio::spawn(async move { claim(&app, &token, &seed, "loss", &log).await })
    };
    // The claim burns its seed first, then waits for a permit, however long its check would take.
    while store!(app, |t| t.economy_practice_seed_get(A, &seed))
        .expect("the seed is held")
        .claimed_at
        .is_none()
    {
        tokio::task::yield_now().await;
    }
    let wait = (check * 4).max(Duration::from_millis(200));
    assert!(
        tokio::time::timeout(wait, &mut waiting).await.is_err(),
        "a claim past the cap waits for a permit"
    );
    assert_unpaid(&app).await;
    drop(held);
    let answer = waiting.await.expect("the claim answers");
    assert_eq!(answer["paid"], json!(true), "{answer}");
}

#[tokio::test]
async fn a_claim_whose_caller_stops_waiting_still_pays() {
    let (app, token) = authed().await;
    let (seed, _) = issue(&app, &token).await;
    let (log, _, _) = play(&seed, &kept_setup(&app, &seed).await, Human::Concede);
    let xp = game_xp(GameMode::Practice, human_actions(&log), false);
    let held = app
        .practice_verifications
        .acquire_many(PRACTICE_VERIFY_CONCURRENCY as u32)
        .await
        .expect("the permits are open");
    let waiting = {
        let (app, token, seed) = (app.clone(), token.clone(), seed.clone());
        tokio::spawn(async move { claim(&app, &token, &seed, "loss", &log).await })
    };
    while store!(app, |t| t.economy_practice_seed_get(A, &seed))
        .expect("the seed is held")
        .claimed_at
        .is_none()
    {
        tokio::task::yield_now().await;
    }
    // The page gives up while the claim waits in the queue, as a closed connection drops its request.
    waiting.abort();
    assert!(waiting.await.is_err(), "the request was dropped");
    drop(held);

    let mut waited = Duration::ZERO;
    while ledger(&app).await.jlore.is_empty() {
        assert!(
            waited < Duration::from_secs(20),
            "the burned seed's claim is paid"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
        waited += Duration::from_millis(10);
    }
    assert_paid(&app, &seed, JLORE_PRACTICE_LOSS, xp).await;
}

// ---------------------------------------------------------------------------
// The day's Jlore cap and the open seeds (R1483000).
// ---------------------------------------------------------------------------

/// A `game` Jlore row of `delta` that an earlier game wrote today, as the cap counts it.
async fn earlier_game_jlore(app: &Arc<App>, delta: i64) -> JloreGrant {
    let grant = JloreGrant {
        profile_id: A.to_string(),
        delta,
        reason: JloreReason::Game,
        reference: Some("earlier-game".to_string()),
        dev: false,
        at: now_ms(),
    };
    store!(app, |t| t.economy_jlore_grant(&grant));
    grant
}

/// A's `game` Jlore rows.
async fn game_rows(app: &Arc<App>) -> Vec<JloreGrant> {
    ledger(app)
        .await
        .jlore
        .into_iter()
        .filter(|grant| grant.reason == JloreReason::Game)
        .collect()
}

#[tokio::test]
async fn r1483000_a_capped_claim_pays_xp_reports_zero_jlore_and_writes_no_game_row() {
    let (app, token) = authed().await;
    let earlier = earlier_game_jlore(&app, JLORE_GAMES_DAILY_CAP).await;
    let (seed, _) = issue(&app, &token).await;
    let (log, _, _) = play(&seed, &kept_setup(&app, &seed).await, Human::Concede);
    let xp = game_xp(GameMode::Practice, human_actions(&log), false);
    assert!(xp > 0, "a conceded game still moved");
    assert_eq!(
        claim(&app, &token, &seed, "loss", &log).await,
        first_paid(0, xp, true)
    );
    assert_eq!(
        game_rows(&app).await,
        vec![earlier],
        "the capped game wrote no Jlore row"
    );
    let reference = Some(format!("practice:{seed}"));
    let tracks = ledger(&app).await.tracks;
    assert!(!tracks.is_empty(), "the game's XP lands on a track");
    for track in &tracks {
        assert_eq!((track.xp, &track.reference), (xp, &reference));
    }
}

#[tokio::test]
async fn r1483000_a_claim_that_crosses_the_cap_pays_the_remainder() {
    let (app, token) = authed().await;
    let left = i64::from(JLORE_PRACTICE_LOSS) - 2;
    earlier_game_jlore(&app, JLORE_GAMES_DAILY_CAP - left).await;
    let (seed, _) = issue(&app, &token).await;
    let (log, _, _) = play(&seed, &kept_setup(&app, &seed).await, Human::Concede);
    let xp = game_xp(GameMode::Practice, human_actions(&log), false);
    assert_eq!(
        claim(&app, &token, &seed, "loss", &log).await,
        first_paid(left, xp, true)
    );
    let paid: Vec<(i64, Option<String>)> = game_rows(&app)
        .await
        .into_iter()
        .skip(1)
        .map(|grant| (grant.delta, grant.reference))
        .collect();
    assert_eq!(paid, vec![(left, Some(format!("practice:{seed}")))]);
    assert_eq!(
        game_rows(&app).await.iter().map(|grant| grant.delta).sum::<i64>(),
        JLORE_GAMES_DAILY_CAP
    );
}

#[tokio::test]
async fn r1483000_the_open_seed_limit_refuses_without_insert_and_claim_or_expiry_frees_a_place() {
    let (app, token) = authed().await;
    let held = |app: Arc<App>| async move { fake_ref(&app).lock().await.tables.economy.practice_seeds.len() };
    let mut seeds = Vec::new();
    for _ in 0..PRACTICE_SEEDS_OPEN_MAX {
        seeds.push(issue(&app, &token).await.0);
    }
    let (status, body) = request_seed(&app, &token).await;
    assert_eq!(status, 409, "{body}");
    assert_eq!(
        held(app.clone()).await,
        PRACTICE_SEEDS_OPEN_MAX as usize,
        "nothing was issued"
    );

    // A claim, paid or refused, burns its seed and frees its place.
    let setup = kept_setup(&app, &seeds[0]).await;
    let (log, _, _) = play(&seeds[0], &setup, Human::Concede);
    assert_eq!(
        claim(&app, &token, &seeds[0], "loss", &log).await["paid"],
        json!(true)
    );
    let (after_claim, _) = issue(&app, &token).await;
    let (status, _) = request_seed(&app, &token).await;
    assert_eq!(status, 409, "the claimed seed's place is taken again");

    // A seed past its lifetime no longer counts, and the request prunes it.
    {
        let mut data = fake_ref(&app).lock().await;
        let lapsed = data
            .tables
            .economy
            .practice_seeds
            .iter_mut()
            .find(|held| held.seed == seeds[1])
            .expect("the seed is held");
        lapsed.issued_at = now_ms() - TTL_MS - 1;
    }
    let (after_expiry, _) = issue(&app, &token).await;
    assert!(store!(app, |t| t.economy_practice_seed_get(A, &seeds[1])).is_none());
    assert!(store!(app, |t| t.economy_practice_seed_get(A, &after_claim)).is_some());
    assert!(store!(app, |t| t.economy_practice_seed_get(A, &after_expiry)).is_some());
    let (status, _) = request_seed(&app, &token).await;
    assert_eq!(status, 409);
}

#[tokio::test]
async fn r1483000_another_accounts_open_seeds_do_not_count() {
    let (app, token) = authed().await;
    let _ = fake_ref(&app)
        .lock()
        .await
        .seed_profile(json!({ "id": "profile-b", "userId": "user-b", "status": "active" }));
    for n in 0..PRACTICE_SEEDS_OPEN_MAX {
        store!(app, |t| t.economy_practice_seed_insert(&PracticeSeed {
            seed: format!("practice-theirs-{n}"),
            profile_id: "profile-b".to_string(),
            setup: setup(Difficulty::Easy, Vec::new()),
            issued_at: now_ms(),
            claimed_at: None,
        }));
    }
    let (status, body) = request_seed(&app, &token).await;
    assert_eq!(status, 201, "{body}");
}

#[tokio::test]
async fn r1483000_concurrent_seed_requests_never_exceed_the_open_limit() {
    let (app, token) = authed().await;
    let requests: Vec<_> = (0..PRACTICE_SEEDS_OPEN_MAX * 2)
        .map(|_| {
            let (app, token) = (app.clone(), token.clone());
            tokio::spawn(async move { request_seed(&app, &token).await.0 })
        })
        .collect();
    let mut statuses = Vec::new();
    for request in requests {
        statuses.push(request.await.expect("the request answers"));
    }
    let count = |wanted: u16| statuses.iter().filter(|status| **status == wanted).count();
    assert_eq!(count(201), PRACTICE_SEEDS_OPEN_MAX as usize, "{statuses:?}");
    assert_eq!(count(409), PRACTICE_SEEDS_OPEN_MAX as usize, "{statuses:?}");
    assert_eq!(
        fake_ref(&app).lock().await.tables.economy.practice_seeds.len(),
        PRACTICE_SEEDS_OPEN_MAX as usize
    );
}

/// The browser's worker's game (`apps/web/src/practice/claim.test.ts` records it).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Fixture {
    seed: String,
    difficulty: Difficulty,
    human_seat: PlayerId,
    deck: PracticeDeck,
    result: ClaimedResult,
    log: Vec<Action>,
}

#[tokio::test]
async fn r1303002_the_ai_rerun_matches_the_game_the_worker_played() {
    let fixture: Fixture = serde_json::from_str(include_str!("../fixtures/practice-claim.json"))
        .expect("the fixture is a practice claim");
    let setup = PracticeSetup {
        difficulty: fixture.difficulty,
        human_seat: fixture.human_seat,
        deck: fixture.deck,
        last_board: Vec::new(),
        catalog_version: catalog_version().to_string(),
    };
    let rerecord = "the worker's game no longer re-runs here: after an AI, card or engine change, record \
                    it again with `JACKIOH_RECORD_PRACTICE_CLAIM=1 pnpm --dir apps/web exec vitest run \
                    src/practice/claim.test.ts`";

    // The measurement the issue asks for: one verification of the worker's game.
    let started = Instant::now();
    let verified = practice::verify(pinned(), &fixture.seed, &setup, fixture.result, &fixture.log);
    let ms = started.elapsed().as_millis();
    let verified = verified.unwrap_or_else(|reason| panic!("{reason:?}: {rerecord}"));
    eprintln!(
        "practice claim verified in {ms} ms: {} logged actions, {} of them the AI's",
        fixture.log.len(),
        verified.ai_moves
    );

    let (app, token) = authed().await;
    insert(&app, &fixture.seed, &setup, now_ms()).await;
    let won = fixture.result.won();
    let xp = game_xp(GameMode::Practice, human_actions(&fixture.log), won == Some(true));
    let jlore = jackioh_server::economy::rewards::game_jlore(GameMode::Practice, won);
    assert_eq!(
        claim(
            &app,
            &token,
            &fixture.seed,
            result_name(fixture.result),
            &fixture.log
        )
        .await,
        first_paid(jlore, xp, false),
        "{rerecord}"
    );
    assert_paid(&app, &fixture.seed, jlore, xp).await;
}
