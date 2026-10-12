//! R1452 (SPEC §9.12): the bot seat's own rules, apart from the actor. A think time stays inside
//! `BOT_THINK_MIN_MS`..`BOT_THINK_MAX_MS` and leaves `BOT_CLOCK_MARGIN_MS` on the acting clock, and
//! so does the search's deadline; R188's guard names the concession and both draws; the fallback
//! move ends the turn when it can and never concedes.
//!
//! `jackioh_server::bots::seat`, on the real cards.

use jackioh_engine::wire::PLAYER_IDS;
use jackioh_engine::{Action, ActionBody, ActionType, Rng};
use jackioh_server::actor::engine::{begin_game, create_game, create_game_args, reduce};
use jackioh_server::bots::seat::{
    THINK_MODEL, ThinkFor, ThinkModel, decision_deadline_ms, fallback_action, never_played, r188_forbidden,
    think_ms,
};
use jackioh_server::bots::signals::standard_normal;
use jackioh_server::config::{
    BOT_CLOCK_MARGIN_MS, BOT_DECISION_DEADLINE_MS, BOT_THINK_MAX_MS, BOT_THINK_MIN_MS,
    BOT_THINK_PRIOR_LN_LEGAL_MEAN, BOT_THINK_PRIORS, THINK_COMPLEXITY_SLOPE, THINK_TIME_FITS, ThinkTimeFit,
    ThinkTimePrior,
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
    let mut state = begin_game(&create_game(&create_game_args(
        "seed-fallback",
        &decks,
        None,
        None,
        None,
    )))
    .state;
    // Both mulligans open: each seat's fallback is its keep.
    for seat in PLAYER_IDS {
        let body = fallback_action(&state, seat).expect("a mulligan is owed");
        assert!(matches!(body, ActionBody::Mulligan { .. }), "{body:?}");
        let result = reduce(&state, &Action::new(body, seat, format!("fallback-{seat}")));
        assert!(result.error.is_none(), "{:?}", result.error);
        state = result.state;
    }
    // The first turn: the active seat ends it; neither seat's fallback is ever one R188 forbids.
    assert_eq!(fallback_action(&state, state.active), Some(ActionBody::EndTurn));
    for seat in PLAYER_IDS {
        if let Some(body) = fallback_action(&state, seat) {
            assert!(!r188_forbidden(&body), "{seat}: {body:?}");
        }
    }
}

/// A model with one later-move play fit of no spread, centred on seven legal actions, and a
/// concession prior centred on `prior_ln_legal_mean`, so each think time is its mean exactly.
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
    // A kind with no fit takes its own prior, centred on the priors' complexity.
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
fn r639000_never_played_is_only_offers_and_emotes() {
    for body in [
        ActionBody::OfferDraw,
        ActionBody::Emote {
            emote: jackioh_engine::wire::EmoteId::Greetings,
        },
    ] {
        assert!(never_played(&body), "{body:?}");
    }
    // A ladder bot's concession and its draw answer, either way, are played (R639000).
    for body in [
        ActionBody::Concede,
        ActionBody::AnswerDraw { accept: true },
        ActionBody::AnswerDraw { accept: false },
        ActionBody::EndTurn,
    ] {
        assert!(!never_played(&body), "{body:?}");
    }
}
