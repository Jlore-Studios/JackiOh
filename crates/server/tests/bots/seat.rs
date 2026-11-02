//! R1452 (SPEC §9.12): bot seat rules. Think time limits, R188 guard, fallback move.
//! R639000: think time scaling, mulligan and draw responses.
//! R2660000: concedes only facing an unstoppable lethal; R2660001: offers a draw while well behind.
//! R747000: skill roll vs miss; R2214000: beginner misses vs §10.7 random policy.

use jackioh_ai::{
    AI_SKILL_MAX, AI_SKILL_MIN, AiOptions, AiStyle, DeterminizeOptions, GREEDY_EVAL, NextSwing,
    candidate_set, decide_styled, determinize, evaluate, one_ply_scores, plays_best, random_action, redact,
    skill_miss_action, unstoppable_lethal,
};
use jackioh_engine::testkit::{Value, json, scenario};
use jackioh_engine::wire::PLAYER_IDS;
use jackioh_engine::{
    Action, ActionBody, ActionType, CardType, GameState, PlayerId, Rng, Row, Zone, seat_to_act,
};
use jackioh_server::actor::engine::{
    Pinned, begin_game, create_game, create_game_args, deal_random_deck, legal_actions, reduce,
};
use jackioh_server::bots::seat::{
    BotJob, THINK_MODEL, ThinkFor, ThinkModel, decision_deadline_ms, fallback_action, never_played, play,
    r188_forbidden, think_ms,
};
use jackioh_server::bots::signals::{accepts_draw, concedes, offers_draw, standard_normal, standing};
use jackioh_server::config::{
    BOT_CLOCK_MARGIN_MS, BOT_CONCEDE_FIT, BOT_DECISION_DEADLINE_MS, BOT_DRAW_ANSWER_FIT, BOT_DRAW_OFFER,
    BOT_STRENGTHS, BOT_THINK_MAX_MS, BOT_THINK_MIN_MS, BOT_THINK_PRIOR_LN_LEGAL_MEAN, BOT_THINK_PRIORS,
    BotStrength, DrawAnswerFit, DrawOfferRule, SKILL_ELO_ANCHOR, SKILL_ELO_TABLE, THINK_COMPLEXITY_SLOPE,
    THINK_TIME_FITS, ThinkTimeFit, ThinkTimePrior, expected_elo_for_skill,
};

/// The catalog's playable ids (no tokens), sliced into two disjoint decks.
fn decks() -> (Vec<String>, Vec<String>) {
    jackioh_cards::register_all();
    let pool: Vec<String> = jackioh_cards::CATALOG
        .values()
        .filter(|def| !def.token)
        .map(|def| def.id.clone())
        .collect();
    let size = usize::try_from(jackioh_engine::config::DECK_SIZE).expect("a deck size");
    (pool[..size].to_vec(), pool[size..size * 2].to_vec())
}

/// A plain move's think time: not a lethal one, on a board of seven legal actions.
fn think(action_kind: ActionType, first_in_turn: bool) -> ThinkFor {
    ThinkFor {
        action_kind,
        first_in_turn,
        legal_count: 7,
        lethal: false,
    }
}

#[test]
fn r1452_a_think_time_stays_inside_its_bounds_and_the_clock() {
    let mut rng = Rng::new("think-bounds", 0);
    for at in 0..400 {
        let first = at % 2 == 0;
        let cursor = rng.cursor();
        let free = think_ms(&mut rng, &think(ActionType::Play, first), &THINK_MODEL, 1.0, None);
        assert!((BOT_THINK_MIN_MS..=BOT_THINK_MAX_MS).contains(&free), "{free}");
        // Two draws a think time (Box–Muller).
        assert_eq!(rng.cursor(), cursor + 2);
    }

    // The same draws at twice the speed multiple wait at least as long; at the clock's edge the
    // wait leaves BOT_CLOCK_MARGIN_MS.
    for at in 0..100u32 {
        let slow = think_ms(
            &mut Rng::new("think-speed", at * 2),
            &think(ActionType::EndTurn, true),
            &THINK_MODEL,
            2.0,
            None,
        );
        let plain = think_ms(
            &mut Rng::new("think-speed", at * 2),
            &think(ActionType::EndTurn, true),
            &THINK_MODEL,
            1.0,
            None,
        );
        assert!(slow >= plain, "{slow} < {plain}");
        let left = BOT_CLOCK_MARGIN_MS + 3_000;
        let tight = think_ms(
            &mut Rng::new("think-speed", at * 2),
            &think(ActionType::EndTurn, true),
            &THINK_MODEL,
            2.0,
            Some(left),
        );
        assert!((BOT_THINK_MIN_MS..=3_000).contains(&tight), "{tight}");
    }
    // With less than the margin and the minimum left, the bot moves at once rather than time out.
    let edge = BOT_CLOCK_MARGIN_MS + BOT_THINK_MIN_MS / 2;
    assert_eq!(
        think_ms(
            &mut Rng::new("think-edge", 0),
            &think(ActionType::EndTurn, false),
            &THINK_MODEL,
            1.0,
            Some(edge)
        ),
        BOT_THINK_MIN_MS / 2
    );
    assert_eq!(
        think_ms(
            &mut Rng::new("think-edge", 0),
            &think(ActionType::EndTurn, false),
            &THINK_MODEL,
            1.0,
            Some(0)
        ),
        0
    );
}

#[test]
fn r1452_the_decision_deadline_leaves_the_clock_margin() {
    assert_eq!(decision_deadline_ms(None), BOT_DECISION_DEADLINE_MS);
    assert_eq!(
        decision_deadline_ms(Some(BOT_CLOCK_MARGIN_MS + BOT_DECISION_DEADLINE_MS * 10)),
        BOT_DECISION_DEADLINE_MS
    );
    assert_eq!(
        decision_deadline_ms(Some(BOT_CLOCK_MARGIN_MS + BOT_DECISION_DEADLINE_MS / 2)),
        BOT_DECISION_DEADLINE_MS / 2
    );
    assert_eq!(decision_deadline_ms(Some(BOT_CLOCK_MARGIN_MS)), 0);
    assert_eq!(decision_deadline_ms(Some(0)), 0);
}

#[test]
fn r1452_r188_forbids_a_concession_a_draw_offer_an_accepted_draw_and_an_emote() {
    for body in [
        ActionBody::Concede,
        ActionBody::OfferDraw,
        ActionBody::AnswerDraw { accept: true },
        // R1127: a bot's emotes come from its persona, never from its moves.
        ActionBody::Emote {
            emote: jackioh_engine::wire::EmoteId::Greetings,
        },
    ] {
        assert!(r188_forbidden(&body), "{body:?}");
    }
    for body in [
        ActionBody::AnswerDraw { accept: false },
        ActionBody::EndTurn,
        ActionBody::SetAutoEndTurn { enabled: true },
    ] {
        assert!(!r188_forbidden(&body), "{body:?}");
    }
}

#[test]
fn r1452_the_fallback_move_answers_the_mulligan_ends_the_turn_and_never_concedes() {
    let decks = decks();
    let mut state = begin_game(
        Pinned::default(),
        &create_game(
            Pinned::default(),
            &create_game_args("seed-fallback", &decks, None, None, None),
        ),
    )
    .state;
    // Both mulligans open: each seat's fallback is its keep.
    for seat in PLAYER_IDS {
        let body = fallback_action(Pinned::default(), &state, seat).expect("a mulligan is owed");
        assert!(matches!(body, ActionBody::Mulligan { .. }), "{body:?}");
        let result = reduce(
            Pinned::default(),
            &state,
            &Action::new(body, seat, format!("fallback-{seat}")),
        );
        assert!(result.error.is_none(), "{:?}", result.error);
        state = result.state;
    }
    // The first turn: the active seat ends it; neither seat's fallback is ever one R188 forbids.
    assert_eq!(
        fallback_action(Pinned::default(), &state, state.active),
        Some(ActionBody::EndTurn)
    );
    for seat in PLAYER_IDS {
        if let Some(body) = fallback_action(Pinned::default(), &state, seat) {
            assert!(!r188_forbidden(&body), "{seat}: {body:?}");
        }
    }
}

/// A model with one later-move play fit of no spread, centered on seven legal actions, and a
/// concession prior centered on `prior_ln_legal_mean`, so each think time is its mean exactly.
const FIT: [ThinkTimeFit; 1] = [ThinkTimeFit {
    rank_bucket: None,
    action_kind: "play",
    first_in_turn: false,
    samples: 40,
    mu: 8.0,
    sigma: 0.0,
    ln_legal_mean: 1.945_910_149_055_313_3,
}];
const PRIOR: [ThinkTimePrior; 1] = [ThinkTimePrior {
    action_kind: "concede",
    mu: 8.5,
    sigma: 0.0,
}];
const MODEL: ThinkModel<'static> = ThinkModel {
    fits: &FIT,
    priors: &PRIOR,
    prior_ln_legal_mean: 1.5,
    complexity_slope: 0.5,
    lethal_shift: 0.7,
};

#[test]
fn r639000_think_time_grows_with_legal_actions_and_on_lethal() {
    let at = |action_kind, legal_count, lethal| {
        think_ms(
            &mut Rng::new("think-model", 0),
            &ThinkFor {
                action_kind,
                first_in_turn: false,
                legal_count,
                lethal,
            },
            &MODEL,
            1.0,
            None,
        )
    };
    let exact = |ln_ms: f64| ln_ms.exp().round() as i64;
    // At the fit's own complexity, the fit's mean; each unit of ln(legal) above it adds the slope.
    assert_eq!(at(ActionType::Play, 7, false), exact(8.0));
    assert_eq!(
        at(ActionType::Play, 20, false),
        exact(8.0 + 0.5 * (20f64.ln() - 7f64.ln()))
    );
    assert_eq!(at(ActionType::Play, 1, false), exact(8.0 - 0.5 * 7f64.ln()));
    // No legal action counts as one.
    assert_eq!(at(ActionType::Play, 0, false), at(ActionType::Play, 1, false));
    // A lethal decision takes the shift on top.
    assert_eq!(at(ActionType::Play, 7, true), exact(8.0 + 0.7));
    assert!(at(ActionType::Play, 20, false) > at(ActionType::Play, 7, false));
    assert!(at(ActionType::Play, 7, true) > at(ActionType::Play, 7, false));
    // A kind with no fit takes its own prior, centered on the priors' complexity.
    assert_eq!(at(ActionType::Concede, 1, false), exact(8.5 - 0.5 * 1.5));
    // The shipped model leans the same way.
    let shipped = |legal_count, lethal| {
        think_ms(
            &mut Rng::new("think-shipped", 0),
            &ThinkFor {
                action_kind: ActionType::Attack,
                first_in_turn: false,
                legal_count,
                lethal,
            },
            &THINK_MODEL,
            1.0,
            None,
        )
    };
    assert!(shipped(30, false) >= shipped(3, false));
    assert!(shipped(7, true) >= shipped(7, false));
}

#[test]
fn r639000_a_mulligan_draws_the_mulligan_prior() {
    let mulligan = |legal_count| ThinkFor {
        action_kind: ActionType::Mulligan,
        first_in_turn: true,
        legal_count,
        lethal: false,
    };
    // While no fit holds the mulligan, its prior is the one drawn: one standard normal, two draws.
    if THINK_TIME_FITS.iter().all(|fit| fit.action_kind != "mulligan") {
        let prior = BOT_THINK_PRIORS
            .iter()
            .find(|prior| prior.action_kind == "mulligan")
            .expect("a mulligan prior");
        for at in 0..200u32 {
            let legal_count = 2 + (at as usize % 30);
            let z = standard_normal(&mut Rng::new("think-mulligan", at * 2));
            let ln_ms = prior.mu
                + THINK_COMPLEXITY_SLOPE.value * ((legal_count as f64).ln() - BOT_THINK_PRIOR_LN_LEGAL_MEAN)
                + prior.sigma * z;
            let expected = (ln_ms.exp().round() as i64).clamp(BOT_THINK_MIN_MS, BOT_THINK_MAX_MS);
            let mut rng = Rng::new("think-mulligan", at * 2);
            assert_eq!(
                think_ms(&mut rng, &mulligan(legal_count), &THINK_MODEL, 1.0, None),
                expected
            );
            assert_eq!(rng.cursor(), at * 2 + 2);
        }
    }
    // A person's mulligan takes seconds, never the practice AI's instant answer (R266).
    let mut drawn: Vec<i64> = (0..400u32)
        .map(|at| {
            think_ms(
                &mut Rng::new("think-mulligan-median", at * 2),
                &mulligan(8),
                &THINK_MODEL,
                1.0,
                None,
            )
        })
        .collect();
    drawn.sort_unstable();
    let median = drawn[drawn.len() / 2];
    assert!((5_000..=15_000).contains(&median), "{median}");
}

#[test]
fn r2660001_never_played_is_only_an_emote() {
    assert!(never_played(&ActionBody::Emote {
        emote: jackioh_engine::wire::EmoteId::Greetings,
    }));
    // A ladder bot's concession, its draw answer either way (R639000) and its draw offer (R2660001) are
    // played.
    for body in [
        ActionBody::OfferDraw,
        ActionBody::Concede,
        ActionBody::AnswerDraw { accept: true },
        ActionBody::AnswerDraw { accept: false },
        ActionBody::EndTurn,
    ] {
        assert!(!never_played(&body), "{body:?}");
    }
}

/// A game on `seed` past both mulligans (each seat keeps its hand), ended turn by turn until it is
/// at least `turn` and the first seat's: the active seat's main phase.
fn opened_at(seed: &str, turn: i32) -> GameState {
    let decks = decks();
    let mut state = begin_game(
        Pinned::default(),
        &create_game(
            Pinned::default(),
            &create_game_args(seed, &decks, None, None, None),
        ),
    )
    .state;
    for seat in PLAYER_IDS {
        let body = fallback_action(Pinned::default(), &state, seat).expect("a mulligan is owed");
        let result = reduce(
            Pinned::default(),
            &state,
            &Action::new(body, seat, format!("keep-{seat}")),
        );
        assert!(result.error.is_none(), "{:?}", result.error);
        state = result.state;
    }
    let first = state.active;
    let mut at = 0;
    while state.turn < turn || state.active != first {
        let result = reduce(
            Pinned::default(),
            &state,
            &Action::new(ActionBody::EndTurn, state.active, format!("end-{at}")),
        );
        assert!(result.error.is_none(), "{:?}", result.error);
        state = result.state;
        at += 1;
    }
    state
}

/// The job the actor hands `seat::play` for `seat`'s move on `state`, its R639000 rolls on `seed`, at
/// the weakest strength's search and style with full skill: the search always runs.
fn job(state: &GameState, seat: PlayerId, seed: &str, own_turn_start: bool) -> BotJob {
    job_at(state, seat, seed, own_turn_start, AI_SKILL_MAX)
}

/// `job` at `skill` (R747000), the weakest strength's search and style otherwise.
fn job_at(state: &GameState, seat: PlayerId, seed: &str, own_turn_start: bool, skill: f64) -> BotJob {
    BotJob {
        pinned: Pinned::default(),
        state: state.clone(),
        seat,
        home: seat,
        seed: seed.to_string(),
        strength: BotStrength {
            skill,
            ..BOT_STRENGTHS[0]
        },
        speed: 1.0,
        ai_rng: Rng::new(&format!("{seed}:bot-ai:{seat}"), 0),
        timing_rng: Rng::new(&format!("{seed}:bot-timing:{seat}"), 0),
        first_in_turn: own_turn_start,
        own_turn_start,
        aim_lead_ms: 0,
        clock_left_ms: None,
    }
}

/// The first of the seeds `{prefix}-{k}` that `wanted` holds for.
fn seed_where(prefix: &str, wanted: impl Fn(&str) -> bool) -> String {
    (0..10_000)
        .map(|k| format!("{prefix}-{k}"))
        .find(|seed| wanted(seed))
        .unwrap_or_else(|| panic!("no {prefix} seed"))
}

/// A scenario on the real cards with the bot, p1, on its own turn as `p1`, facing `p2`.
fn facing(p1: Value, p2: Value) -> GameState {
    jackioh_cards::register_all();
    scenario(json!({ "seed": "facing", "active": "p1", "p1": p1, "p2": p2 }))
        .state()
        .clone()
}

#[tokio::test(start_paused = true)]
async fn r2660000_a_bot_facing_an_unstoppable_lethal_concedes_at_its_turn_start_and_only_then() {
    // A Tempo Timmy 3/3 and a Mr. Vanilla 4/4 swing for 7 at a bot on 6 with nothing to stop them.
    let bot = PlayerId::P1;
    let state = facing(
        json!({ "health": 6 }),
        json!({ "field": ["core-011", "core-008"] }),
    );
    assert!(
        unstoppable_lethal(&state, bot, None),
        "PREMISE: the lethal is unstoppable"
    );
    let gives_up = seed_where("concede", |seed| concedes(seed, bot, || true, &BOT_CONCEDE_FIT));
    let plays_on = seed_where("stay", |seed| !concedes(seed, bot, || true, &BOT_CONCEDE_FIT));

    // At its turn's start, the bot whose temper gives up concedes, without a search.
    let conceded = play(job(&state, bot, &gives_up, true)).await;
    assert_eq!(conceded.body, Some(ActionBody::Concede));
    assert_eq!(conceded.ai_cursor, 0, "no search ran");
    // Past its turn's start it plays on, and so does one whose temper never gives up.
    for (seed, own_turn_start) in [(&gives_up, false), (&plays_on, true)] {
        let played = play(job(&state, bot, seed, own_turn_start)).await;
        assert!(
            played.body.is_some() && played.body != Some(ActionBody::Concede),
            "{seed} at turn start {own_turn_start}: {:?}",
            played.body
        );
    }
}

#[tokio::test(start_paused = true)]
async fn r2660000_a_bot_far_behind_with_no_unstoppable_lethal_plays_on() {
    // A conceder by its temper, on its last points: before R2660000 it gave up on the deficit alone.
    let bot = PlayerId::P1;
    let gives_up = seed_where("concede", |seed| concedes(seed, bot, || true, &BOT_CONCEDE_FIT));
    for (p1, p2) in [
        // Nothing on the opponent's board to kill it.
        (json!({ "health": 1 }), json!({})),
        // A 4/4 that Midrange Menace's Taunt walls.
        (
            json!({ "health": 3, "field": ["core-019"] }),
            json!({ "field": ["core-008"] }),
        ),
    ] {
        let state = facing(p1.clone(), p2.clone());
        let behind = standing(&state, bot, &gives_up, bot).behind;
        assert!(behind > 0.0, "PREMISE: {p1} {p2} stands behind ({behind})");
        assert!(
            !unstoppable_lethal(&state, bot, None),
            "PREMISE: {p1} {p2} faces no unstoppable lethal"
        );
        let played = play(job(&state, bot, &gives_up, true)).await;
        assert!(
            played.body.is_some() && played.body != Some(ActionBody::Concede),
            "{p1} {p2}: {:?}",
            played.body
        );
    }
}

#[tokio::test(start_paused = true)]
async fn r2660001_a_bot_well_behind_offers_a_draw_at_its_turn_start_by_its_own_roll() {
    // A card to play keeps the turn open past the offer, which R82 would otherwise end (R36).
    let bot = PlayerId::P1;
    let state = facing(json!({ "health": 5, "hand": ["core-008"] }), json!({}));
    let behind = standing(&state, bot, "premise", bot).behind;
    assert!(
        behind >= BOT_DRAW_OFFER.min_behind,
        "PREMISE: the bot stands well behind ({behind})"
    );
    assert!(
        legal_actions(Pinned::default(), &state, bot).contains(&ActionBody::OfferDraw),
        "PREMISE: R36 lets the bot offer"
    );
    let offering = |seed: &str| {
        offers_draw(
            seed,
            bot,
            state.turn,
            &standing(&state, bot, seed, bot),
            &BOT_DRAW_OFFER,
        )
    };
    let offers = seed_where("offer", offering);
    let holds = seed_where("hold", |seed| !offering(seed));

    // At its turn's start, the bot whose roll offers offers, without a search.
    let offered = play(job(&state, bot, &offers, true)).await;
    assert_eq!(offered.body, Some(ActionBody::OfferDraw));
    assert_eq!(offered.ai_cursor, 0, "no search ran");
    // Past its turn's start it plays on, and so does one whose roll holds.
    for (seed, own_turn_start) in [(&offers, false), (&holds, true)] {
        let played = play(job(&state, bot, seed, own_turn_start)).await;
        assert!(
            played.body.is_some() && played.body != Some(ActionBody::OfferDraw),
            "{seed} at turn start {own_turn_start}: {:?}",
            played.body
        );
    }
    // R36: one offer a turn. Once the engine has the bot's offer, the same roll offers no second.
    let after = reduce(
        Pinned::default(),
        &state,
        &Action::new(ActionBody::OfferDraw, bot, "bot-offer".to_string()),
    );
    assert!(after.error.is_none(), "{:?}", after.error);
    let again = play(job(&after.state, bot, &offers, true)).await;
    assert!(
        again.body.is_some() && again.body != Some(ActionBody::OfferDraw),
        "{:?}",
        again.body
    );
}

#[tokio::test(start_paused = true)]
async fn r639000_a_bot_answers_a_draw_offer_by_its_own_roll_not_the_searchs_decline() {
    let mut state = opened_at("seed-play-draw", 1);
    let person = state.active;
    let bot = person.opponent();
    state.players[bot].hero.health = 1;
    let offered = reduce(
        Pinned::default(),
        &state,
        &Action::new(ActionBody::OfferDraw, person, "offer".to_string()),
    );
    assert!(offered.error.is_none(), "{:?}", offered.error);
    let state = offered.state;
    let behind = standing(&state, bot, "premise", bot).behind;
    assert!(behind > 0.0, "PREMISE: the bot stands behind ({behind})");
    // `decide` declines every offer (R188); the bot's answer is its roll's, either way.
    for accept in [true, false] {
        let seed = seed_where("draw", |seed| {
            let standing = standing(&state, bot, seed, bot);
            accepts_draw(seed, bot, state.turn, &standing, &BOT_DRAW_ANSWER_FIT) == accept
        });
        let body = play(job(&state, bot, &seed, false)).await.body;
        assert_eq!(body, Some(ActionBody::AnswerDraw { accept }), "{seed}");
    }
}

/// `state` with a Trap the opponent of `seat` controls set face-down in its first backrow lane: a
/// card of its library, made a Trap, which `redact` hides from `seat` as a placeholder.
fn with_face_down_trap(mut state: GameState, seat: PlayerId) -> GameState {
    let opp = seat.opponent();
    let trap = jackioh_cards::CATALOG
        .values()
        .find(|def| !def.token && def.type_ == CardType::Trap)
        .expect("a Trap in the catalog");
    let mut card = state.players[opp].library.pop().expect("a library card");
    card.def_id = trap.id.clone();
    card.zone = Zone::Field {
        player: opp,
        row: Row::Backrow,
        lane: 0,
    };
    card.face_up = None;
    state.players[opp].backrow[0] = Some(card);
    state
}

/// States at each turn start of game with random legal moves.
fn random_turn_starts(seed: &str) -> Vec<GameState> {
    jackioh_cards::register_all();
    let decks = (
        deal_random_deck(&format!("{seed}-a"), None),
        deal_random_deck(&format!("{seed}-b"), None),
    );
    let mut state = begin_game(
        Pinned::default(),
        &create_game(
            Pinned::default(),
            &create_game_args(seed, &decks, None, None, None),
        ),
    )
    .state;
    let mut rng = Rng::new(seed, 0);
    let mut starts: Vec<GameState> = Vec::new();
    for step in 0..2_000 {
        let Some(seat) = seat_to_act(&state).filter(|_| state.result.is_none()) else {
            break;
        };
        if state.mulligan.is_none()
            && state.pending.is_none()
            && starts.last().is_none_or(|last| last.turn != state.turn)
        {
            starts.push(state.clone());
        }
        let legal: Vec<ActionBody> = legal_actions(Pinned::default(), &state, seat)
            .into_iter()
            .filter(|body| !matches!(body, ActionBody::Concede | ActionBody::OfferDraw))
            .collect();
        let at = (rng.next() * legal.len() as f64) as usize;
        let Some(body) = legal.get(at) else { break };
        let result = reduce(
            Pinned::default(),
            &state,
            &Action::new(body.clone(), seat, format!("random-{step}")),
        );
        if result.error.is_none() {
            state = result.state;
        }
    }
    starts
}

#[tokio::test(start_paused = true)]
async fn r639000_a_face_down_enemy_card_stops_neither_the_standing_read_nor_the_bots_turn() {
    // Evaluates determinized world, reading at each turn start.
    let mut hidden = 0;
    for game in 0..12 {
        let seed = format!("hidden-game-{game}");
        for state in random_turn_starts(&seed) {
            let seat = state.active;
            hidden += usize::from(
                state.players[seat.opponent()]
                    .backrow
                    .iter()
                    .flatten()
                    .any(|card| card.face_up != Some(true)),
            );
            let read = standing(&state, seat, &seed, seat);
            assert!(read.deficit.is_finite() && read.behind.is_finite(), "{read:?}");
        }
    }
    assert!(
        hidden > 50,
        "PREMISE: turn starts facing a face-down card: {hidden}"
    );

    // With a Trap set against it, the bot's turn start plays a move.
    let state = opened_at("seed-play-hidden", BOT_CONCEDE_FIT.min_turn);
    let bot = state.active;
    let state = with_face_down_trap(state, bot);
    let played = play(job(&state, bot, "hidden-play", true)).await;
    assert!(played.body.is_some(), "the turn start played nothing");
    assert!(played.ai_cursor > 0, "the search ran");

    // And so does a draw answer with the Trap set.
    let person = bot.opponent();
    let mut state = with_face_down_trap(opened_at("seed-play-hidden-draw", 1), bot);
    if state.active != person {
        state = reduce(
            Pinned::default(),
            &state,
            &Action::new(ActionBody::EndTurn, bot, "pass".to_string()),
        )
        .state;
    }
    let offered = reduce(
        Pinned::default(),
        &state,
        &Action::new(ActionBody::OfferDraw, person, "offer".to_string()),
    );
    assert!(offered.error.is_none(), "{:?}", offered.error);
    let body = play(job(&offered.state, bot, "hidden-draw", false)).await.body;
    assert!(matches!(body, Some(ActionBody::AnswerDraw { .. })), "{body:?}");
}

#[test]
fn r639000_r2660001_an_even_board_reads_level_though_its_deficit_does_not() {
    // Both seats have only ended their turns: no board, both heroes whole. The seed is the first
    // whose seats cast nothing on draw by then (Core #27 Blood Ridden Glowy Jelly Bean costs its
    // hero health, R58), so nothing has hurt a hero or reached a graveyard.
    let seed = seed_where("seed-even", |seed| {
        let state = opened_at(seed, BOT_CONCEDE_FIT.min_turn);
        PLAYER_IDS.iter().all(|&seat| {
            state.players[seat].hero.health == jackioh_engine::config::HERO_HEALTH
                && state.players[seat].graveyard.is_empty()
        })
    });
    let state = opened_at(&seed, BOT_CONCEDE_FIT.min_turn);
    let bot = state.active;
    let read = standing(&state, bot, "even", bot);
    // R1442's deficit counts the enemy hero's health against the seat, so an even board reads
    // about that far behind on it; the standing's `behind` is centered on the even board.
    assert!(read.deficit > 20.0, "PREMISE: {read:?}");
    assert!(read.behind.abs() < 6.0, "{read:?}");

    // A bot that offers a draw at any lead the opponent holds offers only while it trails its
    // opponent: never with the opponent on its last point, and at once with its own hero there.
    let any_lead = DrawOfferRule {
        chance_per_turn: 1.0,
        min_behind: f64::MIN_POSITIVE,
    };
    let mut winning = state.clone();
    winning.players[bot.opponent()].hero.health = 1;
    let ahead = standing(&winning, bot, "even", bot);
    assert!(ahead.behind < 0.0, "{ahead:?}");
    assert!(!offers_draw("even", bot, state.turn, &ahead, &any_lead));
    assert!(!accepts_draw(
        "even",
        bot,
        state.turn,
        &ahead,
        &DrawAnswerFit {
            samples: 0,
            accept_share: 1.0
        }
    ));
    let mut losing = state;
    losing.players[bot].hero.health = 1;
    let trailing = standing(&losing, bot, "even", bot);
    assert!(trailing.behind > 0.0, "{trailing:?}");
    assert!(offers_draw("even", bot, losing.turn, &trailing, &any_lead));
}

/// The AI stream `job_at` gives the bot on `seed`.
fn ai_stream(seed: &str, seat: PlayerId) -> Rng {
    Rng::new(&format!("{seed}:bot-ai:{seat}"), 0)
}

/// AI move decision at weakest strength search and style.
fn ai_move(state: &GameState, seat: PlayerId, rng: Rng) -> (Option<ActionBody>, u32) {
    let strength = BOT_STRENGTHS[0];
    let mut options = AiOptions {
        rng,
        budget: strength.budget,
        should_stop: None,
    };
    let style = AiStyle {
        temperature: strength.temperature,
        missed_lethal_chance: strength.missed_lethal_chance,
    };
    let decision = decide_styled(state, seat, &mut options, &style);
    (decision.map(|decision| decision.action), options.rng.cursor())
}

#[tokio::test(start_paused = true)]
async fn r747000_bot_seat_applies_strength_skill() {
    let state = opened_at("seed-skill", 1);
    let bot = state.active;

    // Skill 0.0 plays the skill miss (R2214000) on the bot's own stream, with no search and no roll
    // drawn.
    let seed = "skill-zero";
    let mut stream = ai_stream(seed, bot);
    let missed = skill_miss_action(&state, bot, &mut stream);
    let played = play(job_at(&state, bot, seed, false, AI_SKILL_MIN)).await;
    assert!(missed.is_some(), "PREMISE: the miss has a move");
    assert_eq!(played.body, missed);
    assert_eq!(played.ai_cursor, stream.cursor());

    // Skill 1.0 decides exactly as the search does, on a stream no roll was drawn from, and the same
    // job reproduces.
    let seed = "skill-full";
    let (decided, cursor) = ai_move(&state, bot, ai_stream(seed, bot));
    let first = play(job_at(&state, bot, seed, false, AI_SKILL_MAX)).await;
    let second = play(job_at(&state, bot, seed, false, AI_SKILL_MAX)).await;
    assert!(decided.is_some() && cursor > 0, "PREMISE: the search ran");
    assert_eq!(first.body, decided);
    assert_eq!(first.ai_cursor, cursor);
    assert_eq!(second, first);

    // In between, one roll of the stream says which: a decision it keeps from the AI searches on
    // the stream after the roll, one it does not is the miss's on that stream.
    for keeps in [false, true] {
        let seed = seed_where("skill-roll", |seed| {
            plays_best(0.5, &mut ai_stream(seed, bot)) == keeps
        });
        let mut stream = ai_stream(&seed, bot);
        assert_eq!(plays_best(0.5, &mut stream), keeps);
        assert_eq!(stream.cursor(), 1, "one draw a roll");
        let (expected, cursor) = if keeps {
            ai_move(&state, bot, stream)
        } else {
            let action = skill_miss_action(&state, bot, &mut stream);
            (action, stream.cursor())
        };
        let played = play(job_at(&state, bot, &seed, false, 0.5)).await;
        assert_eq!(played.body, expected, "{seed} keeps the AI: {keeps}");
        assert_eq!(played.ai_cursor, cursor, "{seed} keeps the AI: {keeps}");
        assert_eq!(play(job_at(&state, bot, &seed, false, 0.5)).await, played);
    }
}

#[tokio::test(start_paused = true)]
async fn r747000_a_random_bot_still_answers_a_draw_offer_by_its_own_roll() {
    let mut state = opened_at("seed-skill-draw", 1);
    let person = state.active;
    let bot = person.opponent();
    state.players[bot].hero.health = 1;
    let offered = reduce(
        Pinned::default(),
        &state,
        &Action::new(ActionBody::OfferDraw, person, "offer".to_string()),
    );
    assert!(offered.error.is_none(), "{:?}", offered.error);
    let state = offered.state;
    // The skill miss declines an offer as the search does (R2214000), and the bot's roll turns it into
    // its answer (R639000), whatever the skill.
    for accept in [true, false] {
        let seed = seed_where("skill-draw", |seed| {
            let standing = standing(&state, bot, seed, bot);
            accepts_draw(seed, bot, state.turn, &standing, &BOT_DRAW_ANSWER_FIT) == accept
        });
        let played = play(job_at(&state, bot, &seed, false, AI_SKILL_MIN)).await;
        assert_eq!(played.body, Some(ActionBody::AnswerDraw { accept }), "{seed}");
        assert_eq!(played.ai_cursor, 0, "no search and no draw ran");
    }
}

#[tokio::test(start_paused = true)]
async fn r2214000_a_strength_zero_miss_is_legal_and_replays() {
    let skill = BOT_STRENGTHS[0].skill;
    assert_eq!(
        skill, AI_SKILL_MIN,
        "PREMISE: the weakest strength misses every decision"
    );
    let starts: Vec<GameState> = (0..3)
        .flat_map(|game| random_turn_starts(&format!("r2214000-legal-{game}")))
        .collect();
    assert!(starts.len() >= 20, "PREMISE: {} turn starts", starts.len());
    for (at, state) in starts.iter().enumerate() {
        let seat = state.active;
        let seed = format!("r2214000-legal-{at}");
        let played = play(job_at(state, seat, &seed, false, skill)).await;
        // The bot's move is the miss on its own stream, from cursor 0: no roll and no search.
        let mut stream = ai_stream(&seed, seat);
        assert_eq!(
            skill_miss_action(state, seat, &mut stream),
            played.body,
            "turn start {at}"
        );
        assert_eq!(stream.cursor(), played.ai_cursor, "turn start {at}");
        let body = played.body.clone().expect("a miss at a turn start");
        assert!(
            legal_actions(Pinned::default(), state, seat).contains(&body),
            "turn start {at}: {body:?} is legal"
        );
        assert_eq!(
            play(job_at(state, seat, &seed, false, skill)).await,
            played,
            "turn start {at}"
        );
    }
}

/// Tallies premature turn ends and suboptimal attacks across random vs skill miss pickers.
#[derive(Default)]
struct MissTally {
    idle: [usize; 2],
    idle_of: usize,
    bad_attacks: [usize; 2],
    bad_of: usize,
}

/// The `MissTally` of the random game `r2214000-rates-{game}`, two picks of each picker at each of its
/// turn starts.
fn miss_tally(game: usize) -> MissTally {
    const DRAWS: usize = 2;
    let mut tally = MissTally::default();
    for (at, state) in random_turn_starts(&format!("r2214000-rates-{game}"))
        .iter()
        .enumerate()
    {
        let seat = state.active;
        let position = format!("r2214000-rates-{game}-{at}");
        let has_play = legal_actions(Pinned::default(), state, seat)
            .iter()
            .any(|body| matches!(body, ActionBody::Play { .. }));
        let candidates = candidate_set(state, seat);
        let det = determinize(
            &redact(state, seat),
            seat,
            &mut Rng::new(&format!("{position}:det"), 0),
            DeterminizeOptions {
                match_shown_cost: Some(false),
            },
        );
        let standing = evaluate(&det, seat, NextSwing::Enemy, &GREEDY_EVAL);
        let bad: Vec<&ActionBody> = candidates
            .iter()
            .zip(one_ply_scores(&det, seat, &candidates))
            .filter(|(body, score)| matches!(body, ActionBody::Attack { .. }) && *score < standing)
            .map(|(body, _)| body)
            .collect();
        if has_play {
            tally.idle_of += DRAWS;
        }
        if !bad.is_empty() {
            tally.bad_of += DRAWS;
        }
        for draw in 0..DRAWS {
            let picks = [
                random_action(state, seat, &mut Rng::new(&format!("{position}:{draw}"), 0)),
                skill_miss_action(state, seat, &mut Rng::new(&format!("{position}:{draw}"), 0)),
            ];
            for (picker, pick) in picks.iter().enumerate() {
                if has_play && *pick == Some(ActionBody::EndTurn) {
                    tally.idle[picker] += 1;
                }
                if pick.as_ref().is_some_and(|pick| bad.contains(&pick)) {
                    tally.bad_attacks[picker] += 1;
                }
            }
        }
    }
    tally
}

#[test]
fn r2214000_a_strength_zero_miss_makes_far_fewer_beginners_mistakes_than_random() {
    const GAMES: usize = 20;
    // The games are played side by side, each on a thread of its own: one after another they took
    // most of a minute, the last test `rust (test server)` waited on (#2323). Each count is a sum over
    // the games, the same in any order.
    let tallies: Vec<MissTally> = std::thread::scope(|scope| {
        let games: Vec<_> = (0..GAMES)
            .map(|game| scope.spawn(move || miss_tally(game)))
            .collect();
        games
            .into_iter()
            .map(|game| {
                game.join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
            })
            .collect()
    });
    let mut total = MissTally::default();
    for tally in tallies {
        for (sum, count) in total.idle.iter_mut().zip(tally.idle) {
            *sum += count;
        }
        for (sum, count) in total.bad_attacks.iter_mut().zip(tally.bad_attacks) {
            *sum += count;
        }
        total.idle_of += tally.idle_of;
        total.bad_of += tally.bad_of;
    }
    let MissTally {
        idle,
        idle_of,
        bad_attacks,
        bad_of,
    } = total;
    assert!(idle_of >= 500, "PREMISE: {idle_of} picks with a card to play");
    assert!(bad_of >= 150, "PREMISE: {bad_of} picks with a bad attack to make");
    let rate = |count: usize, of: usize| count as f64 / of as f64;
    let (idle_random, idle_miss) = (rate(idle[0], idle_of), rate(idle[1], idle_of));
    let (bad_random, bad_miss) = (rate(bad_attacks[0], bad_of), rate(bad_attacks[1], bad_of));
    let report = format!(
        "ended the turn with a card to play: random {idle_random:.3}, miss {idle_miss:.3} ({} and {} of \
         {idle_of}); attacked below standing still: random {bad_random:.3}, miss {bad_miss:.3} ({} and {} of \
         {bad_of})",
        idle[0], idle[1], bad_attacks[0], bad_attacks[1]
    );
    println!("{report}");
    assert!(idle_miss * 2.0 < idle_random, "{report}");
    assert!(bad_miss * 2.0 < bad_random, "{report}");
}

#[test]
fn r747000_strength_seed_ratings_match_skill_curve() {
    let mut last = None;
    for (index, strength) in BOT_STRENGTHS.iter().enumerate() {
        assert!(
            (AI_SKILL_MIN..=AI_SKILL_MAX).contains(&strength.skill),
            "strength {index}"
        );
        assert_eq!(
            strength.seed_rating,
            expected_elo_for_skill(strength.skill),
            "strength {index}: its seed rating is the curve's at its skill"
        );
        // Weakest first: each strength is more skilled than the one before.
        if let Some(before) = last {
            assert!(strength.skill > before, "strength {index}");
        }
        last = Some(strength.skill);
    }
}

#[test]
fn r747000_expected_elo_reads_the_table_by_straight_lines() {
    // The table spans the whole range, and a higher skill is never worth fewer points.
    assert_eq!(SKILL_ELO_TABLE.first().map(|row| row.0), Some(AI_SKILL_MIN));
    assert_eq!(SKILL_ELO_TABLE.last().map(|row| row.0), Some(AI_SKILL_MAX));
    for pair in SKILL_ELO_TABLE.windows(2) {
        assert!(pair[0].0 < pair[1].0, "{pair:?}: skills ascend");
        assert!(
            pair[0].1 < pair[1].1,
            "{pair:?}: a higher skill is worth more Elo"
        );
    }
    // A row is read as it is, the middle of two rows as their mean, an end as the end it is past.
    for &(skill, elo) in SKILL_ELO_TABLE {
        assert_eq!(expected_elo_for_skill(skill), elo);
    }
    for pair in SKILL_ELO_TABLE.windows(2) {
        let middle = expected_elo_for_skill((pair[0].0 + pair[1].0) / 2.0);
        assert!((middle - (pair[0].1 + pair[1].1) / 2.0).abs() < 1e-9, "{pair:?}");
    }
    // The reference, full skill, is the table's anchor.
    assert_eq!(expected_elo_for_skill(AI_SKILL_MAX), SKILL_ELO_ANCHOR);
    let (_, lowest) = SKILL_ELO_TABLE[0];
    let (_, highest) = SKILL_ELO_TABLE[SKILL_ELO_TABLE.len() - 1];
    assert_eq!(expected_elo_for_skill(-1.0), lowest);
    assert_eq!(expected_elo_for_skill(2.0), highest);
    assert_eq!(expected_elo_for_skill(f64::NAN), highest);
}
