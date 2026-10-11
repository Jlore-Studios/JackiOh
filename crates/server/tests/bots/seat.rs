//! R1447 (SPEC §9.12): the bot seat's own rules, apart from the actor. A think time stays inside
//! `BOT_THINK_MIN_MS`..`BOT_THINK_MAX_MS` and leaves `BOT_CLOCK_MARGIN_MS` on the acting clock, and
//! so does the search's deadline; R188's guard names the concession and both draws; the fallback
//! move ends the turn when it can and never concedes.
//!
//! `jackioh_server::bots::seat`, on the real cards.

use jackioh_engine::wire::PLAYER_IDS;
use jackioh_engine::{Action, ActionBody, ActionType, Rng};
use jackioh_server::actor::engine::{begin_game, create_game, create_game_args, reduce};
use jackioh_server::bots::seat::{decision_deadline_ms, fallback_action, r188_forbidden, think_ms};
use jackioh_server::config::{
    BOT_CLOCK_MARGIN_MS, BOT_DECISION_DEADLINE_MS, BOT_THINK_MAX_MS, BOT_THINK_MIN_MS,
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

#[test]
fn r1447_a_think_time_stays_inside_its_bounds_and_the_clock() {
    let mut rng = Rng::new("think-bounds", 0);
    for at in 0..400 {
        let first = at % 2 == 0;
        let cursor = rng.cursor();
        let free = think_ms(&mut rng, ActionType::Play, first, 1.0, None);
        assert!((BOT_THINK_MIN_MS..=BOT_THINK_MAX_MS).contains(&free), "{free}");
        // Two draws a think time (Box–Muller).
        assert_eq!(rng.cursor(), cursor + 2);
    }

    // The same draws at twice the speed multiple wait at least as long; at the clock's edge the
    // wait leaves BOT_CLOCK_MARGIN_MS.
    for at in 0..100u32 {
        let slow = think_ms(
            &mut Rng::new("think-speed", at * 2),
            ActionType::EndTurn,
            true,
            2.0,
            None,
        );
        let plain = think_ms(
            &mut Rng::new("think-speed", at * 2),
            ActionType::EndTurn,
            true,
            1.0,
            None,
        );
        assert!(slow >= plain, "{slow} < {plain}");
        let left = BOT_CLOCK_MARGIN_MS + 3_000;
        let tight = think_ms(
            &mut Rng::new("think-speed", at * 2),
            ActionType::EndTurn,
            true,
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
            ActionType::EndTurn,
            false,
            1.0,
            Some(edge)
        ),
        BOT_THINK_MIN_MS / 2
    );
    assert_eq!(
        think_ms(
            &mut Rng::new("think-edge", 0),
            ActionType::EndTurn,
            false,
            1.0,
            Some(0)
        ),
        0
    );
}

#[test]
fn r1447_the_decision_deadline_leaves_the_clock_margin() {
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
fn r1447_r188_forbids_a_concession_a_draw_offer_an_accepted_draw_and_an_emote() {
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
fn r1447_the_fallback_move_answers_the_mulligan_ends_the_turn_and_never_concedes() {
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
