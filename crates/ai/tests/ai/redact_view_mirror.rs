//! R185, R177, R97, R227 and R448 (#1593): `redact` hides what the seat's view hides. An
//! enemy face-down card the seat's own prompt offers stays a placeholder and its option reads as
//! the view shows it; the events the queues keep lose what `redact_event` strips of a hidden card;
//! what the seat may read is unchanged.
//!
//! Port of nothing: these mirror `view_for`'s contract, which had no AI-side test.

use jackioh_ai::*;
use jackioh_engine::testkit::*;
use jackioh_engine::view_for::{HIDDEN_COST, HIDDEN_ID, HIDDEN_OPTION_LABEL, HIDDEN_POSITION};

use super::support::{AI, HUMAN, card_by_id, every_card, register_cards};

/// Classic #70 Book of Plague: the (1) Spell whose target prompt reaches the backrow.
const BOOK: &str = "classic-070";
/// Core #96 My Pawn: a (1) Trap.
const PAWN: &str = "core-096";
/// Core #60 Bear Honeypot: a (1) Trap.
const HONEYPOT: &str = "core-060";
/// Core #41 Sheepish: the face-down backrow card the event test hides.
const SHEEPISH: &str = "core-041";
/// Core #6 Mana Well: a Field Spell, public in the backrow.
const MANA_WELL: &str = "core-006";
/// Core #8 Vanilla: a face-up unit.
const VANILLA: &str = "core-008";
/// Core #19 Menace: a face-up unit.
const MENACE: &str = "core-019";

/// A Book of Plague cast whose target prompt offers the five permanents: Vanilla, Sheepish,
/// Menace, Mana Well and the face-down `trap`.
fn plague_prompt(trap: &str) -> Scenario {
    register_cards();
    let mut s = scenario(json!({
        "seed": "redact-view-mirror",
        "p1": { "hand": [BOOK, "core-010"], "field": [VANILLA], "backrow": [{ "def": SHEEPISH, "faceUp": false }],
                "library": ["core-020", "core-020", "core-020"] },
        "p2": { "hand": ["core-002"], "field": [MENACE], "backrow": [MANA_WELL, { "def": trap, "faceUp": false, "lane": 2 }] },
    }));
    s.play(BOOK, json!({}));
    assert_eq!(
        s.state().pending.as_ref().map(|pending| pending.player_id),
        Some(AI)
    );
    s
}

fn option_for<'a>(state: &'a GameState, id: &str) -> Option<&'a PromptOption> {
    state
        .pending
        .as_ref()?
        .options
        .iter()
        .find(|option| matches!(&option.selection, Selection::Instance { instance_id } if instance_id == id))
}

/// The id of the card of this definition the state holds.
fn id_of(state: &GameState, def: &str) -> String {
    every_card(state)
        .into_iter()
        .find(|card| card.def_id == def)
        .map(|card| card.id.clone())
        .unwrap_or_else(|| panic!("no {def} in the state"))
}

fn captured_by(owner: &CardInstance, seq: usize, event: &Value) -> QueuedTrigger {
    json_as(
        json!({ "id": format!("t{seq}"), "seq": seq, "instanceId": owner.id, "hook": "trigger",
        "resume": { "defId": owner.def_id, "hook": "trigger", "step": "start", "radiant": false,
                    "instanceId": owner.id, "data": { "event": event } } }),
    )
}

mod r185_the_ai_reads_what_its_view_shows {
    use super::*;

    /// R185, R177: an enemy face-down card the seat's own prompt offers stays a placeholder and its
    /// option reads as the view's.
    #[test]
    fn r185_r177_an_enemy_face_down_card_the_seats_own_prompt_offers_stays_a_placeholder_and_its_option_reads_as_the_views()
     {
        let s = plague_prompt(PAWN);
        let state = s.state().clone();
        let pawn = id_of(&state, PAWN);
        assert_eq!(
            option_for(&state, &pawn).map(|option| option.label.clone()),
            Some(def_of(Some(&state), PAWN).name.clone())
        );
        assert!(hidden_instance_ids(&state, AI).contains(&pawn));
        let seen = redact(&state, AI);
        assert_eq!(
            card_by_id(&seen, &pawn).map(|card| card.def_id.clone()),
            Some(HIDDEN_DEF_ID.to_string())
        );
        let option = option_for(&seen, &pawn).expect("the pawn stays an option");
        assert_eq!(option.label, HIDDEN_OPTION_LABEL);
        assert_eq!(option.key, format!("instance:{pawn}"));
        assert_eq!(option.radiant, None);
        let pending = serde_json::to_value(&seen.pending).expect("the pending serializes");
        assert!(!pending.to_string().contains(PAWN));
        assert!(!pending.to_string().contains(&def_of(Some(&state), PAWN).name));
        let world = determinize(
            &seen,
            AI,
            &mut create_rng("redact-view-mirror-world", 0),
            DeterminizeOptions::default(),
        );
        assert_eq!(
            serde_json::to_value(&view_for(&world, AI).pending).expect("the world's pending serializes"),
            serde_json::to_value(&view_for(&state, AI).pending).expect("the view's pending serializes")
        );
        assert_eq!(redact(&seen, AI), seen);
    }

    /// R185, R177: two states that differ only in which trap the prompt offers face-down redact to
    /// one hash and one decision.
    #[test]
    fn r185_r177_two_states_that_differ_only_in_which_trap_the_prompt_offers_face_down_redact_to_one_hash_and_one_decision()
     {
        let pawn = plague_prompt(PAWN).state().clone();
        let honeypot = plague_prompt(HONEYPOT).state().clone();
        assert_ne!(hash_state(&pawn), hash_state(&honeypot));
        let pawn_seen = redact(&pawn, AI);
        let honeypot_seen = redact(&honeypot, AI);
        assert_eq!(hash_state(&pawn_seen), hash_state(&honeypot_seen));
        let mut pawn_options = AiOptions::with_budget(create_rng("redact-view-mirror-decide", 0), AI_BUDGET);
        let mut honeypot_options =
            AiOptions::with_budget(create_rng("redact-view-mirror-decide", 0), AI_BUDGET);
        assert_eq!(
            decide(&pawn_seen, AI, &mut pawn_options),
            decide(&honeypot_seen, AI, &mut honeypot_options)
        );
    }

    /// R185, R686: the cards the seat may read keep their definitions and options, a revealed enemy
    /// trap included.
    #[test]
    fn r185_r686_the_cards_the_seat_may_read_keep_their_definitions_and_options_a_revealed_enemy_trap_included()
     {
        let s = plague_prompt(PAWN);
        let state = s.state().clone();
        let seen = redact(&state, AI);
        for def in [VANILLA, SHEEPISH, MENACE, MANA_WELL] {
            let id = id_of(&state, def);
            assert!(!hidden_instance_ids(&state, AI).contains(&id), "{def}");
            assert_eq!(
                card_by_id(&seen, &id).map(|card| card.def_id.clone()),
                Some(def.to_string())
            );
            assert_eq!(option_for(&seen, &id), option_for(&state, &id));
        }
        let mut revealed = state.clone();
        let pawn = id_of(&revealed, PAWN);
        find_instance_mut(&mut revealed, &pawn)
            .expect("the pawn")
            .revealed = Some(true);
        assert!(!hidden_instance_ids(&revealed, AI).contains(&pawn));
        let seen = redact(&revealed, AI);
        assert_eq!(
            card_by_id(&seen, &pawn).map(|card| card.def_id.clone()),
            Some(PAWN.to_string())
        );
        assert_eq!(option_for(&seen, &pawn), option_for(&revealed, &pawn));
    }

    /// R185, R97, R227, R448: a hidden card's events a queue keeps lose what the view strips.
    #[test]
    fn r185_r97_r227_r448_a_hidden_cards_events_a_queue_keeps_lose_what_the_view_strips() {
        register_cards();
        let mut s = scenario(json!({
            "seed": "redact-view-mirror-events",
            "p1": { "field": ["core-011"], "library": ["core-020", "core-053"] },
            "p2": { "hand": ["core-002"], "backrow": [{ "def": SHEEPISH, "faceUp": false }], "library": ["core-005"] },
        }));
        let unit = id_of(s.state(), "core-011");
        let trap = id_of(s.state(), SHEEPISH);
        let hand = s.state().players[HUMAN].hand[0].id.clone();
        find_instance_mut(s.state_mut(), &trap).expect("the trap").set_as = Some(SetAs {
            reveal: RevealAt::Cloaked,
            set_turn: 1,
            echo: None,
            revealing: None,
        });
        let former = "c99".to_string();
        let events: Vec<Value> = vec![
            json!({ "type": "cardAnnounced", "player": "p2", "instanceId": former, "defId": "classic-088",
                "cardType": "Field Trap", "costPaid": 3, "targets": [unit], "row": "backrow", "lane": 1, "faceDown": true }),
            json!({ "type": "cardPlayed", "player": "p2", "instanceId": trap, "defId": SHEEPISH,
                "costPaid": 1, "x": 2, "embiggened": true, "formerId": former }),
            json!({ "type": "cardResolved", "player": "p2", "instanceId": trap, "defId": SHEEPISH,
                "permanent": true, "costPaid": 1, "radiant": true,
                "answer": { "by": hand, "at": { "pick": "hero", "player": "p1" } } }),
            json!({ "type": "shuffledIn", "player": "p1", "instanceId": unit, "defId": "core-011", "position": 1 }),
            json!({ "type": "costChanged", "instanceId": hand, "cost": 4 }),
        ];
        let owner = find_instance(s.state(), &unit).expect("the unit").clone();
        let parsed: Vec<GameEvent> = events.iter().map(|event| json_as(event.clone())).collect();
        let state = s.state_mut();
        state.applied = vec![AppliedAction {
            nonce: "redact-view-mirror".to_string(),
            events: parsed,
        }];
        for (k, event) in events.iter().enumerate() {
            state.trigger_queue.push(captured_by(&owner, 900 + k, event));
            state.dispatch.push(json_as(
                json!({ "id": format!("d{k}"), "seq": 900 + k, "event": event }),
            ));
        }
        let state = s.state().clone();
        let seen = redact(&state, AI);
        let shown: Vec<Value> = view_for(&state, AI)
            .events
            .iter()
            .map(|event| serde_json::to_value(event).expect("the view's event serializes"))
            .collect();
        for (k, ((kept, dispatched_item), expected)) in seen
            .trigger_queue
            .iter()
            .zip(seen.dispatch.iter())
            .zip(shown.iter())
            .enumerate()
        {
            for field in [
                "cardType",
                "targets",
                "costPaid",
                "formerId",
                "x",
                "embiggened",
                "answer",
                "radiant",
                "position",
                "cost",
            ] {
                let expected = expected.get(field).cloned();
                let captured = kept
                    .resume
                    .data
                    .get("event")
                    .and_then(|event| event.get(field))
                    .cloned();
                let dispatched = serde_json::to_value(&dispatched_item.event)
                    .expect("the dispatched event serializes")
                    .get(field)
                    .cloned();
                assert_eq!(captured, expected, "the kept event {k}'s {field}");
                assert_eq!(dispatched, expected, "the dispatched event {k}'s {field}");
            }
        }
        let announce = seen.trigger_queue[0]
            .resume
            .data
            .get("event")
            .expect("the announce is kept");
        assert_eq!(announce.get("defId"), Some(&Value::from(HIDDEN_DEF_ID)));
        assert_eq!(announce.get("cardType"), Some(&Value::from("Trap")));
        assert_eq!(
            announce.get("targets"),
            Some(&Value::from(vec![Value::from(HIDDEN_ID)]))
        );
        assert_eq!(announce.get("costPaid"), Some(&Value::from(3)));
        let play = seen.trigger_queue[1]
            .resume
            .data
            .get("event")
            .expect("the play is kept");
        assert!(play.get("formerId").is_none());
        assert!(play.get("x").is_none());
        assert!(play.get("embiggened").is_none());
        assert_eq!(play.get("costPaid"), Some(&Value::from(1)));
        let resolve = seen.trigger_queue[2]
            .resume
            .data
            .get("event")
            .expect("the resolve is kept");
        assert!(resolve.get("answer").is_none());
        assert!(resolve.get("radiant").is_none());
        let shuffled = seen.trigger_queue[3]
            .resume
            .data
            .get("event")
            .expect("the shuffle is kept");
        assert_eq!(shuffled.get("position"), Some(&Value::from(HIDDEN_POSITION)));
        let cost = seen.trigger_queue[4]
            .resume
            .data
            .get("event")
            .expect("the cost is kept");
        assert_eq!(cost.get("cost"), Some(&Value::from(HIDDEN_COST)));
        assert_eq!(redact(&seen, AI), seen);
    }

    /// R185, R1180005: the card that answers the seat's own spell stays named while it is in the
    /// seat's own hand, and a deck's goes.
    #[test]
    fn r185_r1180005_the_card_that_answers_the_seats_own_spell_stays_named_while_it_is_in_the_seats_own_hand_and_a_decks_goes()
     {
        let mut s = plague_prompt(PAWN);
        let book = id_of(s.state(), BOOK);
        let by_hand = s.state().players[AI].hand[0].id.clone();
        let by_deck = s.state().players[AI].library[0].id.clone();
        let owner = find_instance(s.state(), &id_of(s.state(), VANILLA))
            .expect("vanilla")
            .clone();
        let hand_answer = json!({ "by": by_hand, "at": { "pick": "hero", "player": "p2" } });
        for (k, answer) in [
            hand_answer.clone(),
            json!({ "by": by_deck, "at": { "pick": "hero", "player": "p2" } }),
        ]
        .into_iter()
        .enumerate()
        {
            let event = json!({ "type": "cardResolved", "player": "p1", "instanceId": book,
                "defId": BOOK, "permanent": false, "costPaid": 1, "answer": answer });
            s.state_mut()
                .trigger_queue
                .push(captured_by(&owner, 900 + k, &event));
        }
        let seen = redact(s.state(), AI);
        let answers: Vec<Option<Value>> = seen
            .trigger_queue
            .iter()
            .map(|entry| {
                entry
                    .resume
                    .data
                    .get("event")
                    .and_then(|event| event.get("answer"))
                    .cloned()
            })
            .collect();
        assert_eq!(answers, vec![Some(hand_answer), None]);
    }
}
