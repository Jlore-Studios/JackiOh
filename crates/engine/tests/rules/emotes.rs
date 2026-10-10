//! ME-EMOTE (docs/meditative-set.md M5, MD-D29): emotes as engine actions (R1127).
//!
//! An emote becomes an `Emote` action with an `emoted` event, legal for either seat — on its own
//! turn or the other's — only while the other seat controls an acting card that hears it ("whenever
//! your opponent emotes"): never with a prompt or a mulligan open, never unheard, never to its own
//! controller's judge. The random policy never emotes, an auto-end due ignores the emote, and the
//! view carries `emotesHeard` only as `true`, on the seat that is heard.
//!
//! Port of `packages/engine/test/emotes.test.ts`.

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat_judge::{combat_judge_game, judge_emote};
use crate::rules::fixtures::harness::put;
use crate::rules::fixtures::harness::slot;

fn emote_action(player: PlayerId, nonce: &str) -> Action {
    Action::new(
        ActionBody::Emote {
            emote: EmoteId::Greetings,
        },
        player,
        nonce.to_string(),
    )
}

/// P1's main phase with no judge yet.
fn main_phase(seed: &str) -> GameState {
    let mut state = combat_judge_game(seed, None);
    state.turn = 4;
    state.active = P1;
    state.phase = Phase::Main;
    state
}

/// P1's main phase with the hearing judge on `judge_side`'s backrow, so the other seat is heard.
fn heard_against(seed: &str, judge_side: PlayerId) -> GameState {
    let mut state = main_phase(seed);
    put(
        &mut state,
        &judge_emote.id,
        slot(judge_side, Row::Backrow, 1),
        Default::default(),
    );
    state
}

/// The emotes `player` is offered.
fn listed(state: &GameState, player: PlayerId) -> Vec<EmoteId> {
    jackioh_engine::reduce::legal_actions(state, player)
        .into_iter()
        .filter_map(|body| match body {
            ActionBody::Emote { emote } => Some(emote),
            _ => None,
        })
        .collect()
}

#[test]
fn r1127_legal_either_seat_only_while_heard() {
    // Each seat in turn is the judge's opponent: P2 off its turn, P1 on its own.
    for (judge_side, emoter) in [(P1, P2), (P2, P1)] {
        let mut state = heard_against("r1127-heard", judge_side);
        // Nothing but the emote happens: no automatic turn end can follow it here.
        state.players[P1].auto_end_turn = Some(false);
        assert_eq!(
            listed(&state, emoter),
            EMOTE_IDS.to_vec(),
            "seat {emoter} lists every emote while the judge across hears it"
        );
        let out = jackioh_engine::reduce::reduce(&state, &emote_action(emoter, "n1"));
        assert_eq!(out.error, None);
        assert_eq!(
            out.events,
            vec![GameEvent::Emoted {
                player: emoter,
                emote: EmoteId::Greetings,
            }]
        );

        // The judge's own controller is not its opponent: its emotes stay R643's relay.
        assert!(
            listed(&state, judge_side).is_empty(),
            "seat {judge_side} lists no emote to its own judge"
        );
        let out = jackioh_engine::reduce::reduce(&state, &emote_action(judge_side, "n2"));
        assert_eq!(out.error.as_deref(), Some("no card hears emotes"));
        assert!(out.events.is_empty());
    }
}

#[test]
fn r1127_refused_unheard_in_prompt_or_mulligan() {
    // No card hears them: the action is refused, and no seat lists one.
    let state = main_phase("r1127-unheard");
    for player in [P1, P2] {
        assert!(
            listed(&state, player).is_empty(),
            "seat {player} lists no emote unheard"
        );
        let out = jackioh_engine::reduce::reduce(&state, &emote_action(player, "n1"));
        assert_eq!(out.error.as_deref(), Some("no card hears emotes"));
        assert!(out.events.is_empty());
    }

    // Heard, but a prompt is open: refused, unlisted, as every action but the prompt's own answer
    // and the game's ending ones is (BUILD M1-T3).
    let mut state = heard_against("r1127-prompt", P1);
    state.pending = Some(json_as(json!({
        "id": "q1",
        "playerId": "p1",
        "kind": "answer",
        "prompt": "test prompt",
        "options": [{ "key": "a", "label": "A", "selection": { "pick": "none" } }],
        "min": 1,
        "max": 1,
        "resume": { "defId": "cj-judge-emote", "hook": "q", "step": "q", "radiant": false, "data": {} },
    })));
    assert!(
        listed(&state, P2).is_empty(),
        "no emote is listed with a prompt open"
    );
    let out = jackioh_engine::reduce::reduce(&state, &emote_action(P2, "n1"));
    assert_eq!(out.error.as_deref(), Some("a prompt is open: answer it first"));

    // Heard, but the mulligans are open: refused, unlisted, the same way.
    let mut state = heard_against("r1127-mulligan", P1);
    state.phase = Phase::Mulligan;
    let seat = json!({
        "prompt": {
            "id": "m1",
            "playerId": "p2",
            "kind": "mulligan",
            "prompt": "mulligan",
            "options": [],
            "min": 0,
            "max": 10,
            "resume": { "defId": "cj-judge-emote", "hook": "q", "step": "q", "radiant": false, "data": {} },
        },
        "keep": null,
    });
    state.mulligan = Some(json_as(json!({ "p1": seat, "p2": seat })));
    assert!(
        listed(&state, P2).is_empty(),
        "no emote is listed with the mulligans open"
    );
    let out = jackioh_engine::reduce::reduce(&state, &emote_action(P2, "n1"));
    assert_eq!(
        out.error.as_deref(),
        Some("the mulligan is open: answer it first")
    );
}

#[test]
fn r1127_policy_never_emotes_auto_end_ignores() {
    // The random policy never picks the emote, though it is legal.
    assert!(
        jackioh_engine::subsystems::ai_policy::AI_SKIPPED_ACTIONS.contains(&ActionType::Emote),
        "the random policy skips emotes"
    );
    // An auto-end due is not held up by the emote: with nothing else to do, the emote resolves and
    // the turn still ends by itself. P1 is heard, by P2's judge.
    let mut state = heard_against("r1127-policy", P2);
    state.players[P1].hand.clear();
    state.players[P1].mana.current = 0;
    let out = jackioh_engine::reduce::reduce(&state, &emote_action(P1, "n1"));
    assert_eq!(out.error, None);
    let kinds: Vec<&str> = out
        .events
        .iter()
        .map(|event| event.event_type().as_str())
        .collect();
    assert!(kinds.contains(&"emoted"), "the emote resolved: {kinds:?}");
    assert!(
        kinds.contains(&"turnAutoEnded"),
        "the turn still auto-ended: {kinds:?}"
    );
}

#[test]
fn r1127_view_emotes_heard_only_true() {
    // P1's judge hears P2: P2's view says so, P1's says nothing.
    let state = heard_against("r1127-view", P1);
    let view = serde_json::to_value(view_for(&state, P2)).expect("serialises");
    assert_eq!(view["emotesHeard"], serde_json::json!(true));
    let view = serde_json::to_value(view_for(&state, P1)).expect("serialises");
    assert!(view.get("emotesHeard").is_none());
    // No judge: neither view carries the field.
    let state = main_phase("r1127-view-off");
    for player in [P1, P2] {
        let view = serde_json::to_value(view_for(&state, player)).expect("serialises");
        assert!(view.get("emotesHeard").is_none());
    }
}
