//! ME-EMOTE (docs/meditative-set.md M5, MD-D29): emotes as engine actions (R1127).
//!
//! An emote becomes an `Emote` action with an `emoted` event, legal for either seat only while a
//! card hears it — never with a prompt or a mulligan open, never unheard. The random policy never
//! emotes, an auto-end due ignores the emote, and the view carries `emotesHeard` only as `true`.
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

/// P1's main phase with the hearing judge on P1's backrow.
fn heard(seed: &str) -> GameState {
    let mut state = combat_judge_game(seed, None);
    state.turn = 4;
    state.active = P1;
    state.phase = Phase::Main;
    put(
        &mut state,
        &judge_emote.id,
        slot(P1, Row::Backrow, 1),
        Default::default(),
    );
    state
}

#[test]
fn r1127_legal_either_seat_only_while_heard() {
    let state = heard("r1127-heard");
    // Either seat lists every emote while a card hears them.
    for player in [P1, P2] {
        let emotes: Vec<EmoteId> = jackioh_engine::reduce::legal_actions(&state, player)
            .into_iter()
            .filter_map(|body| match body {
                ActionBody::Emote { emote } => Some(emote),
                _ => None,
            })
            .collect();
        assert_eq!(emotes, EMOTE_IDS.to_vec(), "seat {player} lists every emote");
        let out = jackioh_engine::reduce::reduce(&state, &emote_action(player, "n1"));
        assert!(out.error.is_none());
        assert_eq!(out.events.len(), 1);
        assert_eq!(
            out.events[0],
            GameEvent::Emoted {
                player,
                emote: EmoteId::Greetings,
            }
        );
    }
}

#[test]
fn r1127_refused_unheard_in_prompt_or_mulligan() {
    // No card hears them: the action is refused, and no seat lists one.
    let mut state = combat_judge_game("r1127-unheard", None);
    state.turn = 4;
    state.active = P1;
    state.phase = Phase::Main;
    for player in [P1, P2] {
        assert!(
            !jackioh_engine::reduce::legal_actions(&state, player)
                .iter()
                .any(|body| matches!(body, ActionBody::Emote { .. })),
            "seat {player} lists no emote unheard"
        );
        let out = jackioh_engine::reduce::reduce(&state, &emote_action(player, "n1"));
        assert_eq!(out.error.as_deref(), Some("no card hears emotes"));
        assert!(out.events.is_empty());
    }

    // Heard, but a prompt is open: refused, unlisted.
    let mut state = heard("r1127-prompt");
    state.pending = Some(json_as(json!({
        "id": "q1",
        "playerId": "p1",
        "kind": "answer",
        "prompt": "test prompt",
        "options": [{ "key": "a", "label": "A", "selection": "None" }],
        "min": 1,
        "max": 1,
        "resume": { "defId": "cj-judge-emote", "hook": "q", "step": "q", "radiant": false },
    })));
    let out = jackioh_engine::reduce::reduce(&state, &emote_action(P1, "n1"));
    assert_eq!(out.error.as_deref(), Some("no card hears emotes"));

    // Heard, but the mulligans are open: the same refusal.
    let mut state = heard("r1127-mulligan");
    state.phase = Phase::Mulligan;
    let seat = json!({
        "prompt": {
            "id": "m1",
            "playerId": "p1",
            "kind": "mulligan",
            "prompt": "mulligan",
            "options": [],
            "min": 0,
            "max": 10,
            "resume": { "defId": "cj-judge-emote", "hook": "q", "step": "q", "radiant": false },
        },
        "keep": null,
    });
    state.mulligan = Some(json_as(json!({ "p1": seat, "p2": seat })));
    let out = jackioh_engine::reduce::reduce(&state, &emote_action(P1, "n1"));
    assert!(out.error.is_some());
}

#[test]
fn r1127_policy_never_emotes_auto_end_ignores() {
    // The random policy never picks the emote, though it is legal.
    assert!(
        jackioh_engine::subsystems::ai_policy::AI_SKIPPED_ACTIONS.contains(&ActionType::Emote),
        "the random policy skips emotes"
    );
    // An auto-end due is not held up by the emote: with nothing else to do, the emote resolves and
    // the turn still ends by itself.
    let mut state = heard("r1127-policy");
    state.players[P1].hand.clear();
    state.players[P1].mana.current = 0;
    let out = jackioh_engine::reduce::reduce(&state, &emote_action(P1, "n1"));
    assert!(out.error.is_none());
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
    let state = heard("r1127-view");
    let view = serde_json::to_value(view_for(&state, P1)).expect("serialises");
    assert_eq!(view["you"]["emotesHeard"], serde_json::json!(true));
    // The opponent hears nothing: their judge is nobody's.
    let mut state = combat_judge_game("r1127-view-off", None);
    state.turn = 4;
    state.active = P1;
    state.phase = Phase::Main;
    let view = serde_json::to_value(view_for(&state, P1)).expect("serialises");
    assert!(view["you"].get("emotesHeard").is_none());
}
