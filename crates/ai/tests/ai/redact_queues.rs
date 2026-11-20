//! R185, R602: what `redact` keeps of the queues and the board that a hidden card touches. The
//! opponent's hidden cards' work items and delayed effects go, and the work cursor stays on the
//! same next item; a live face-down card's Health on the seat's units stays as shown.

use jackioh_ai::*;
use jackioh_engine::prelude::{AuraEntry, StatMod, aura_hook};
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, card_by_id, dealt_game, register_cards};

/// A continuation of `card`'s, as a played card's owed step or a delayed effect carries it.
fn resume_of(card: &CardInstance) -> Value {
    json!({
        "defId": card.def_id,
        "hook": "fixture",
        "step": "0",
        "radiant": false,
        "instanceId": card.id,
        "data": {},
    })
}

fn work_of(id: &str, card: &CardInstance) -> WorkItem {
    json_as(json!({ "id": id, "seq": 0, "owner": card.controller, "resume": resume_of(card) }))
}

fn delayed_of(id: &str, card: &CardInstance) -> DelayedEffect {
    json_as(json!({
        "id": id,
        "seq": 0,
        "owner": card.controller,
        "at": { "phase": "end", "player": card.controller },
        "resume": resume_of(card),
    }))
}

/// The seat's own hand cards (it reads them) and one of the opponent's (it does not).
fn cards(state: &GameState) -> (CardInstance, CardInstance, CardInstance) {
    let own = &state.players[AI].hand;
    let hidden = state.players[HUMAN].hand[0].clone();
    (own[0].clone(), own[1].clone(), hidden)
}

fn work_ids(state: &GameState) -> Vec<String> {
    state.work.iter().map(|item| item.id.clone()).collect()
}

mod r185_the_queues_a_hidden_card_holds_an_entry_in {
    use super::*;

    #[test]
    fn r185_redact_drops_a_hidden_cards_work_and_keeps_the_cursor_on_the_same_next_item() {
        let state = dealt_game("redact-work-cursor");
        let (first, second, hidden) = cards(&state);
        // The hidden item done before the cursor, and the hidden item the cursor stands on.
        for (layout, cursor, next) in [
            (["hidden", "first", "second"], 2, "second"),
            (["first", "hidden", "second"], 1, "second"),
        ] {
            let mut queued = state.clone();
            queued.work = layout
                .iter()
                .map(|name| match *name {
                    "hidden" => work_of(name, &hidden),
                    "first" => work_of(name, &first),
                    _ => work_of(name, &second),
                })
                .collect();
            queued.work_cursor = cursor;
            let seen = redact(&queued, AI);
            assert_eq!(work_ids(&seen), vec!["first", "second"], "{layout:?}");
            assert_eq!(
                seen.work.get(seen.work_cursor).map(|item| item.id.as_str()),
                Some(next),
                "{layout:?} at {cursor}"
            );
        }
    }

    #[test]
    fn r185_redact_drops_a_hidden_cards_delayed_effect_and_keeps_a_readable_ones() {
        let mut state = dealt_game("redact-delayed");
        let (first, _, hidden) = cards(&state);
        state.delayed = vec![delayed_of("hidden", &hidden), delayed_of("first", &first)];
        let seen = redact(&state, AI);
        let ids: Vec<&str> = seen.delayed.iter().map(|entry| entry.id.as_str()).collect();
        assert_eq!(ids, vec!["first"]);
    }
}

/// A token Field Trap, live face-down, whose aura gives enemy Units +2 Health: no shipped card's live
/// face-down text changes a Health (C #88 Siphon Squad's changes Attack). A token, so no deck, pool or
/// determinization deals it; registered on this test's thread only.
const BOLSTER: &str = "ai-redact-bolster";

fn register_bolster() {
    register_cards();
    let mut def: CardDef = find_def(None, "classic-088").cloned().expect("Siphon Squad");
    def.id = BOLSTER.to_string();
    def.name = "Bolster (fixture)".to_string();
    def.token = true;
    let mut defs = registered_catalog().clone();
    defs.insert(def.id.clone(), def);
    register_catalog(defs);
    let face = || Script {
        aura: Some(aura_hook(|read| {
            vec![AuraEntry {
                applies: Box::new(move |unit: &CardInstance| {
                    unit.zone.z() == ZoneName::Field && unit.controller != read.self_.controller
                }),
                mod_: StatMod {
                    max_health: Some(2),
                    ..StatMod::default()
                },
            }]
        })),
        live_face_down: true,
        ..Script::default()
    };
    let mut scripts = registered_scripts().clone();
    scripts.insert(
        BOLSTER.to_string(),
        CardScripts {
            base: face(),
            radiant: face(),
        },
    );
    register_scripts(scripts);
}

mod r602_a_live_face_down_cards_health_on_the_seats_units {
    use super::*;

    #[test]
    fn r602_r403_redact_keeps_the_health_a_live_face_down_card_gives_the_seats_units() {
        register_bolster();
        let state = scenario(json!({
            "seed": "redact-live-health",
            "active": "p1",
            "p1": { "field": ["core-019", "core-011"] },
            "p2": { "backrow": [{ "def": BOLSTER, "faceUp": false }] },
        }))
        .state()
        .clone();
        let seen = redact(&state, AI);
        let units: Vec<CardInstance> = active_units_of(&state, AI).into_iter().cloned().collect();
        assert_eq!(units.len(), 2);
        for unit in units {
            let truth = unit_view(&state, &unit);
            let printed = find_def(None, &unit.def_id)
                .and_then(|def| def.base.health)
                .expect("a printed Health");
            assert_eq!(truth.max_health, printed + 2, "the aura is live face-down");
            let seen_unit = card_by_id(&seen, &unit.id).cloned().expect("the unit is public");
            let shown = unit_view(&seen, &seen_unit);
            assert_eq!(
                (shown.max_health, shown.health),
                (truth.max_health, truth.health),
                "{}",
                unit.def_id
            );
        }
    }
}
