//! Face-down plays as Traps (ME-ALTPLAY, R1040–R1046).
//!
//! While Knowledge Breaker (a Unit permission) or Paranoia (a Spell permission) acts on its
//! controller's field, `legal_actions` offers Units or Spells as face-down plays into the backrow;
//! a set Unit reveals at its controller's next start of turn and runs its Cry, a set Spell at its
//! chosen timing. The fixtures in `fixtures/alt_play.rs` stand in for the real cards.
//!
//! Port of `packages/engine/test/alt-play.test.ts`.

use jackioh_engine::testkit::*;

use super::fixtures::alt_play::{bolt, breaker, filler, paranoia, register_alt_play};
use super::fixtures::harness::{put, slot};
use super::fixtures::play_pipeline_b::{pb_reduce, plays_of};
use super::fixtures::prompt_harness::{act, answer_keys, board, must, open_as, round_trip};

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

/// Past the mulligans in p1's main phase with the alt-play cards registered and p1 at 4 mana.
fn playing(seed: &str) -> GameState {
    let state = board(seed);
    register_alt_play();
    state
}

fn hero_health(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].hero.health
}

/// A fresh instance of `def_id` in `player`'s hand.
fn in_hand(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
    let card = new_instance(state, def_id, player, Zone::Hand { player });
    state.players[player].hand.push(card.clone());
    card
}

/// Every offered `play` for this card, as JSON.
fn offered(state: &GameState, instance_id: &str, player: PlayerId) -> Vec<Value> {
    plays_of(state, instance_id, player)
        .iter()
        .map(|play| serde_json::to_value(play).expect("a play serialises"))
        .collect()
}

/// The offered plays that set the card face-down.
fn face_down_offered(state: &GameState, instance_id: &str, player: PlayerId) -> Vec<Value> {
    offered(state, instance_id, player)
        .into_iter()
        .filter(|play| play.get("faceDown").is_some_and(|timing| !timing.is_null()))
        .collect()
}

/// A refusal's text (`""` when the action was not refused).
fn refusal(result: &ReduceResult) -> String {
    result.error.clone().unwrap_or_default()
}

/// Play this card face-down with this timing into this backrow lane, panicking on refusal.
fn set_card(state: &GameState, id: &str, player: &str, lane: i32, timing: &str) -> GameState {
    act(
        state,
        json!({
            "type": "play",
            "instanceId": id,
            "zone": { "row": "backrow", "lane": lane },
            "faceDown": timing,
            "playerId": player,
        }),
        None,
    )
}

mod r1040_face_down_unit_offer {
    use super::*;

    #[test]
    fn r1040_offered_in_each_open_backrow_zone_and_refused_otherwise() {
        let mut state = playing("r1040-offer");
        register_alt_play();
        put(&mut state, &breaker().id, slot(P1, Row::Units, 1), json!({}));
        let unit = in_hand(&mut state, &filler().id, P1);

        // Each open backrow zone is offered at the Unit's only timing.
        let mut lanes: Vec<i64> = face_down_offered(&state, &unit.id, P1)
            .iter()
            .map(|play| play["zone"]["lane"].as_i64().unwrap_or(-1))
            .collect();
        lanes.sort();
        lanes.dedup();
        assert_eq!(
            face_down_offered(&state, &unit.id, P1)
                .iter()
                .filter(|play| play["faceDown"] == json!("startOfNextTurn"))
                .count(),
            lanes.len()
        );
        assert!(!lanes.is_empty());

        // No permission: nothing face-down is offered, and the play is refused.
        let mut bare = playing("r1040-bare");
        register_alt_play();
        let lonely = in_hand(&mut bare, &filler().id, P1);
        assert_eq!(face_down_offered(&bare, &lonely.id, P1), Vec::<Value>::new());
        assert!(
            refusal(&pb_reduce(
                &bare,
                json!({
                    "type": "play",
                    "instanceId": lonely.id,
                    "zone": { "row": "backrow", "lane": 1 },
                    "faceDown": "startOfNextTurn",
                    "playerId": "p1",
                }),
            ))
            .contains("face-down")
        );

        // The opponent's Units get no permission from our breaker.
        let enemy = in_hand(&mut state, &filler().id, P2);
        assert_eq!(face_down_offered(&state, &enemy.id, P2), Vec::<Value>::new());

        // A face-down play declares no target.
        assert!(
            refusal(&pb_reduce(
                &state,
                json!({
                    "type": "play",
                    "instanceId": unit.id,
                    "zone": { "row": "backrow", "lane": 1 },
                    "faceDown": "startOfNextTurn",
                    "targets": [{ "pick": "hero", "player": "p2" }],
                    "playerId": "p1",
                }),
            ))
            .contains("face-down")
        );

        // Into a closed zone: fill the backrow, then the zone is refused. A Unit cannot be put
        // into the backrow (`place_on_field` wants a carrier), so Paranoia stands in.
        for lane in 1..=5 {
            put(
                &mut state,
                &paranoia().id,
                slot(P1, Row::Backrow, lane),
                json!({}),
            );
        }
        assert!(
            refusal(&pb_reduce(
                &state,
                json!({
                    "type": "play",
                    "instanceId": unit.id,
                    "zone": { "row": "backrow", "lane": 1 },
                    "faceDown": "startOfNextTurn",
                    "playerId": "p1",
                }),
            ))
            .contains("backrow zone")
        );

        // A Unit has only the start of the next turn: the end of this turn is refused.
        let mut fresh = playing("r1040-timing");
        register_alt_play();
        put(&mut fresh, &breaker().id, slot(P1, Row::Units, 1), json!({}));
        let other = in_hand(&mut fresh, &filler().id, P1);
        assert!(
            refusal(&pb_reduce(
                &fresh,
                json!({
                    "type": "play",
                    "instanceId": other.id,
                    "zone": { "row": "backrow", "lane": 1 },
                    "faceDown": "endOfThisTurn",
                    "playerId": "p1",
                }),
            ))
            .contains("reveal")
        );
    }

    #[test]
    fn r1040_a_set_unit_pays_its_price_takes_a_fresh_id_counts_as_a_field_trap_play_and_is_dormant() {
        let mut state = playing("r1040-set");
        register_alt_play();
        put(&mut state, &breaker().id, slot(P1, Row::Units, 1), json!({}));
        let unit = in_hand(&mut state, &filler().id, P1);
        let mana = state.players.p1.mana.current;

        let next = set_card(&state, &unit.id, "p1", 2, "startOfNextTurn");
        // It paid its Unit price of (1).
        assert_eq!(next.players.p1.mana.current, mana - 1);
        // It took a fresh id (R227) and sits face-down in the backrow.
        let placed = must(next.players.p1.backrow[1].clone(), "a card in backrow lane 2");
        assert_ne!(placed.id, unit.id);
        assert_eq!(placed.face_up, None);
        // It counts as a Field Trap play.
        let by_type = next
            .players
            .p1
            .turn_log
            .played_by_type
            .clone()
            .unwrap_or_default();
        assert_eq!(by_type.get(&CardType::FieldTrap).copied().unwrap_or(0), 1);
        assert_eq!(
            placed.set_as.as_ref().map(|set| set.reveal),
            Some(RevealAt::StartOfNextTurn)
        );
        // It is dormant: its own text does not act.
        assert!(jackioh_engine::alt_play::is_dormant(&placed));
        assert_eq!(
            jackioh_engine::faces::card_type_of(&next, &placed),
            CardType::FieldTrap
        );
    }
}

mod r1041_set_unit_reveal {
    use super::*;

    /// Set a second breaker while one grants, then run to its controller's next start of turn.
    fn set_second_breaker(seed: &str) -> (GameState, CardInstance) {
        let mut state = playing(seed);
        register_alt_play();
        put(&mut state, &breaker().id, slot(P1, Row::Units, 1), json!({}));
        let other = in_hand(&mut state, &breaker().id, P1);
        let next = set_card(&state, &other.id, "p1", 2, "startOfNextTurn");
        (next, other)
    }

    #[test]
    fn r1041_reveals_animates_runs_its_cry_with_a_prompt_and_may_attack() {
        let (mut state, _set) = set_second_breaker("r1041-reveal");
        // To the end of p1's turn and through p2's: no reveal yet.
        state = act(&state, json!({ "type": "endTurn", "playerId": "p1" }), None);
        assert!(state.pending.is_none());
        state = act(&state, json!({ "type": "endTurn", "playerId": "p2" }), None);
        // p1's next start of turn fires the reveal: the Cry asks for its target.
        let pending = open_as(&state, PromptKind::Target, P1);
        assert!(pending.prompt.contains("Cry"));
        let before = hero_health(&state, P2);
        let answered = answer_keys(&mut state, &["hero:p2"]);
        assert_eq!(answered.error, None);
        assert_eq!(hero_health(&state, P2), before - 2);
        // It animated into the units row, face-up and no longer set, keeping its set turn.
        let granter = must(
            state.players.p1.units[0].iter().flatten().next(),
            "the granter in lane 1",
        )
        .id
        .clone();
        let live = must(
            state
                .players
                .p1
                .units
                .iter()
                .flatten()
                .flatten()
                .find(|card| card.def_id == breaker().id && card.id != granter),
            "the revealed breaker",
        )
        .clone();
        assert_eq!(live.face_up, Some(true));
        assert_eq!(live.set_as, None);
        assert_eq!(live.summoned_turn, Some(3));
        // It may attack: its set turn is past, so it is not summoning sick.
        let attacks: Vec<_> = legal_actions(&state, P1)
            .into_iter()
            .filter(
                |action| matches!(action, ActionBody::Attack { attacker_id, .. } if attacker_id == &live.id),
            )
            .collect();
        assert!(!attacks.is_empty());
    }

    #[test]
    fn r1041_with_no_open_unit_zone_it_stays_face_up_and_reveals_next_turn() {
        let (mut state, _set) = set_second_breaker("r1041-full");
        // Fill every remaining unit zone: the granter plus four fillers.
        for lane in 2..=5 {
            put(&mut state, &filler().id, slot(P1, Row::Units, lane), json!({}));
        }
        // Run to p1's next start of turn: the reveal fires but cannot animate.
        state = act(&state, json!({ "type": "endTurn", "playerId": "p1" }), None);
        state = act(&state, json!({ "type": "endTurn", "playerId": "p2" }), None);
        assert!(state.pending.is_none());
        let stuck = must(
            state.players.p1.backrow[1].clone(),
            "the set card in backrow lane 2",
        );
        assert_eq!(stuck.face_up, Some(true));
        assert!(stuck.set_as.is_some());
        // A full round later it fires again, still face-up in the backrow.
        state = act(&state, json!({ "type": "endTurn", "playerId": "p1" }), None);
        state = act(&state, json!({ "type": "endTurn", "playerId": "p2" }), None);
        assert!(state.pending.is_none());
        let again = must(state.players.p1.backrow[1].clone(), "the set card still there");
        assert_eq!(again.face_up, Some(true));
        assert!(again.set_as.is_some());
    }
}

mod r1044_set_spell_timings {
    use super::*;

    use jackioh_engine::modifiers::add_modifier;

    /// A sink lent with the state to one engine call, as `reduce` starts one.
    struct Bench {
        events: Vec<GameEvent>,
        rng: Rng,
    }

    impl Bench {
        fn new(state: &GameState) -> Bench {
            Bench {
                events: Vec::new(),
                rng: Rng::new(&state.seed, state.rng_cursor),
            }
        }

        fn sink<'a>(&'a mut self, state: &'a mut GameState) -> EngineSink<'a> {
            EngineSink::new(state, &mut self.events, &mut self.rng)
        }
    }

    /// Paranoia stands in p1's backrow; a bolt waits in p1's hand and a filler stands for p2.
    fn paranoid(seed: &str) -> (GameState, CardInstance, CardInstance) {
        let mut state = playing(seed);
        register_alt_play();
        put(&mut state, &paranoia().id, slot(P1, Row::Backrow, 1), json!({}));
        let target = put(&mut state, &filler().id, slot(P2, Row::Units, 1), json!({}));
        let spell = in_hand(&mut state, &bolt().id, P1);
        (state, spell, target)
    }

    /// The set bolt reveals with a prompt; answer it at the enemy filler for 2, destroying it.
    fn answer_reveal(state: &mut GameState, target: &CardInstance) {
        let pending = open_as(state, PromptKind::Target, P1);
        assert!(pending.prompt.contains("Reveal"));
        let key = format!("instance:{}", target.id);
        let answered = answer_keys(state, &[key.as_str()]);
        assert_eq!(answered.error, None);
    }

    #[test]
    fn r1044_each_timing_reveals_at_its_point_then_lands_in_the_graveyard() {
        // End of this turn: set on p1's turn 3, revealed by ending it. The set leaves p1 with
        // nothing to do, so the turn ends by itself (R82) and the reveal's prompt is already open.
        let (state, spell, target) = paranoid("r1044-end-this");
        let mut state = set_card(&state, &spell.id, "p1", 2, "endOfThisTurn");
        answer_reveal(&mut state, &target);
        assert!(state.players.p2.graveyard.iter().any(|card| card.id == target.id));
        assert!(
            state
                .players
                .p1
                .graveyard
                .iter()
                .any(|card| card.def_id == bolt().id)
        );

        // Start of the next turn: silent through p2, fires on p1's next start (the set ends p1's
        // turn by itself, R82, so p2 is already active).
        let (state, spell, target) = paranoid("r1044-start-next");
        let mut state = set_card(&state, &spell.id, "p1", 2, "startOfNextTurn");
        assert!(state.pending.is_none());
        state = act(&state, json!({ "type": "endTurn", "playerId": "p2" }), None);
        answer_reveal(&mut state, &target);
        assert!(state.players.p2.graveyard.iter().any(|card| card.id == target.id));
        assert!(
            state
                .players
                .p1
                .graveyard
                .iter()
                .any(|card| card.def_id == bolt().id)
        );

        // End of the next turn: silent until p1's next turn ends (p2 is already active).
        let (state, spell, target) = paranoid("r1044-end-next");
        let mut state = set_card(&state, &spell.id, "p1", 2, "endOfNextTurn");
        assert!(state.pending.is_none());
        state = act(&state, json!({ "type": "endTurn", "playerId": "p2" }), None);
        assert!(state.pending.is_none());
        state = act(&state, json!({ "type": "endTurn", "playerId": "p1" }), None);
        answer_reveal(&mut state, &target);
        assert!(state.players.p2.graveyard.iter().any(|card| card.id == target.id));
        assert!(
            state
                .players
                .p1
                .graveyard
                .iter()
                .any(|card| card.def_id == bolt().id)
        );
    }

    #[test]
    fn r1044_priced_as_a_trap_and_fizzles_without_a_required_target() {
        let (mut state, spell, _target) = paranoid("r1044-price");
        // A (1) discount for Traps reaches the set Spell's price but not its face-up one:
        // with no mana the face-down play is offered and the face-up play is not.
        state.players.p1.mana.current = 0;
        let turn = state.turn;
        let mut bench = Bench::new(&state);
        add_modifier(
            &mut bench.sink(&mut state),
            P1,
            ModifierExpiry::ThisTurn { turn },
            ModifierKind::CostDiscount {
                amount: 1,
                only_type: Some(CardType::Trap),
                min_current_cost: None,
                once_per_turn: None,
            },
        );
        assert!(!face_down_offered(&state, &spell.id, P1).is_empty());
        assert_eq!(
            offered(&state, &spell.id, P1)
                .into_iter()
                .filter(|play| play.get("faceDown").is_none_or(|timing| timing.is_null()))
                .count(),
            0
        );

        // With no Units on either board the required target cannot be met at reveal:
        // it fizzles with no prompt and lands in the graveyard.
        let mut fizzle = playing("r1044-fizzle");
        register_alt_play();
        put(&mut fizzle, &paranoia().id, slot(P1, Row::Backrow, 1), json!({}));
        let lonely = in_hand(&mut fizzle, &bolt().id, P1);
        // The set ends p1's turn by itself (R82): the reveal fizzles with no prompt there.
        let fizzle = set_card(&fizzle, &lonely.id, "p1", 2, "endOfThisTurn");
        assert!(fizzle.pending.is_none());
        assert!(
            fizzle
                .players
                .p1
                .graveyard
                .iter()
                .any(|card| card.def_id == bolt().id)
        );
    }

    #[test]
    fn r1044_a_paused_reveal_survives_a_json_round_trip() {
        let (state, spell, target) = paranoid("r1044-trip");
        // The set ends p1's turn by itself (R82), so the reveal's prompt is already open.
        let state = set_card(&state, &spell.id, "p1", 2, "endOfThisTurn");
        open_as(&state, PromptKind::Target, P1);
        let mut copy = round_trip(&state);
        let key = format!("instance:{}", target.id);
        let answered = answer_keys(&mut copy, &[key.as_str()]);
        assert_eq!(answered.error, None);
        assert!(copy.players.p2.graveyard.iter().any(|card| card.id == target.id));
    }
}

mod r1045_permission_leaves_and_echo {
    use super::*;

    #[test]
    fn r1045_reveals_after_the_permission_leaves_and_a_bounced_one_is_a_spell() {
        let mut state = playing("r1045-leaves");
        register_alt_play();
        let granted = put(&mut state, &paranoia().id, slot(P1, Row::Backrow, 1), json!({}));
        let target = put(&mut state, &filler().id, slot(P2, Row::Units, 1), json!({}));
        let spell = in_hand(&mut state, &bolt().id, P1);
        let mut state = set_card(&state, &spell.id, "p1", 2, "startOfNextTurn");
        // Paranoia leaves the field: the set Spell stays set and still reveals on time.
        let backrow = state.players.p1.backrow[0].take();
        assert_eq!(backrow.as_ref().map(|card| card.id.clone()), Some(granted.id));
        if let Some(mut aura) = backrow {
            aura.zone = Zone::Graveyard { player: P1 };
            state.players.p1.graveyard.push(aura);
        }
        assert_eq!(face_down_offered(&state, &spell.id, P1), Vec::<Value>::new());
        // The set already ended p1's turn by itself (R82); p2 ends theirs next.
        assert!(state.pending.is_none());
        state = act(&state, json!({ "type": "endTurn", "playerId": "p2" }), None);
        let pending = open_as(&state, PromptKind::Target, P1);
        assert!(pending.prompt.contains("Reveal"));
        let key = format!("instance:{}", target.id);
        let answered = answer_keys(&mut state, &[key.as_str()]);
        assert_eq!(answered.error, None);
        assert!(state.players.p2.graveyard.iter().any(|card| card.id == target.id));

        // A bounced set card is its printed Spell again: the reset clears the face-down form.
        let mut bounced = playing("r1045-bounce");
        register_alt_play();
        put(&mut bounced, &paranoia().id, slot(P1, Row::Backrow, 1), json!({}));
        let spell = in_hand(&mut bounced, &bolt().id, P1);
        let mut bounced = set_card(&bounced, &spell.id, "p1", 2, "startOfNextTurn");
        let set = must(bounced.players.p1.backrow[1].clone(), "the set bolt");
        assert!(set.set_as.is_some());
        let returning = bounced.players.p1.backrow[1].take();
        if let Some(mut card) = returning {
            jackioh_engine::zones::reset_instance(&mut card);
            card.zone = Zone::Hand { player: P1 };
            bounced.players.p1.hand.push(card);
        }
        let home = must(
            bounced
                .players
                .p1
                .hand
                .iter()
                .find(|card| card.def_id == bolt().id),
            "the bounced bolt",
        )
        .clone();
        assert_eq!(home.set_as, None);
        assert_eq!(
            jackioh_engine::faces::card_type_of(&bounced, &home),
            CardType::Spell
        );
    }

    #[test]
    fn r1045_radiant_echo_repeats_once_with_fresh_prompts_however_many_grant() {
        // One Radiant Paranoia: the set bolt resolves twice, asking fresh each time.
        let mut state = playing("r1045-echo");
        register_alt_play();
        put(
            &mut state,
            &paranoia().id,
            slot(P1, Row::Backrow, 1),
            json!({ "radiant": true }),
        );
        let first = put(&mut state, &filler().id, slot(P2, Row::Units, 1), json!({}));
        let second = put(&mut state, &filler().id, slot(P2, Row::Units, 2), json!({}));
        let spell = in_hand(&mut state, &bolt().id, P1);
        // The set ends p1's turn by itself (R82), so the first prompt is already open.
        let mut state = set_card(&state, &spell.id, "p1", 2, "endOfThisTurn");
        open_as(&state, PromptKind::Target, P1);
        let key = format!("instance:{}", first.id);
        let answered = answer_keys(&mut state, &[key.as_str()]);
        assert_eq!(answered.error, None);
        // The Echo repeat asks again with a fresh prompt.
        open_as(&state, PromptKind::Target, P1);
        let key = format!("instance:{}", second.id);
        let answered = answer_keys(&mut state, &[key.as_str()]);
        assert_eq!(answered.error, None);
        assert!(state.players.p2.graveyard.iter().any(|card| card.id == first.id));
        assert!(state.players.p2.graveyard.iter().any(|card| card.id == second.id));

        // Two Radiant Paranoias grant the same +1 once: still exactly two resolutions.
        let mut doubled = playing("r1045-echo-two");
        register_alt_play();
        put(
            &mut doubled,
            &paranoia().id,
            slot(P1, Row::Backrow, 1),
            json!({ "radiant": true }),
        );
        put(
            &mut doubled,
            &paranoia().id,
            slot(P1, Row::Backrow, 2),
            json!({ "radiant": true }),
        );
        let first = put(&mut doubled, &filler().id, slot(P2, Row::Units, 1), json!({}));
        let second = put(&mut doubled, &filler().id, slot(P2, Row::Units, 2), json!({}));
        let spell = in_hand(&mut doubled, &bolt().id, P1);
        let mut doubled = set_card(&doubled, &spell.id, "p1", 3, "endOfThisTurn");
        open_as(&doubled, PromptKind::Target, P1);
        let key = format!("instance:{}", first.id);
        assert_eq!(answer_keys(&mut doubled, &[key.as_str()]).error, None);
        open_as(&doubled, PromptKind::Target, P1);
        let key = format!("instance:{}", second.id);
        assert_eq!(answer_keys(&mut doubled, &[key.as_str()]).error, None);
        assert!(doubled.pending.is_none());
        assert!(
            doubled
                .players
                .p2
                .graveyard
                .iter()
                .any(|card| card.id == first.id)
        );
        assert!(
            doubled
                .players
                .p2
                .graveyard
                .iter()
                .any(|card| card.id == second.id)
        );
    }
}

mod r1046_hidden_from_the_opponent {
    use super::*;

    #[test]
    fn r1046_view_for_shows_the_opponent_only_a_face_down_card_and_its_cost() {
        let mut state = playing("r1046-hidden");
        register_alt_play();
        put(&mut state, &breaker().id, slot(P1, Row::Units, 1), json!({}));
        let unit = in_hand(&mut state, &filler().id, P1);
        let cost =
            jackioh_engine::mana::effective_cost(&state, &unit, jackioh_engine::CostOptions::default());
        let state = set_card(&state, &unit.id, "p1", 2, "startOfNextTurn");

        // The opponent's view of the backrow: face-down, with only its cost to read.
        let foe = view_for(&state, P2);
        let zone = must(
            foe.opponent.backrow.get(1).cloned().flatten(),
            "the set card for p2",
        );
        let shown = serde_json::to_value(&zone).expect("a backrow view serialises");
        assert_eq!(shown["faceDown"], json!(true));
        assert_eq!(shown["cost"], json!(cost));
        assert!(shown.get("instanceId").is_none());
        assert!(shown.get("defId").is_none());
        // The timing is the owner's alone (D3): no field carries it to the opponent.
        assert!(shown.get("revealAt").is_none());

        // The opponent's copy of the set event: a Trap set, its cost, no identity.
        let played: Vec<Value> = foe
            .events
            .iter()
            .map(|event| serde_json::to_value(event).expect("an event serialises"))
            .filter(|event| event["type"] == json!("cardPlayed") || event["type"] == json!("summoned"))
            .collect();
        assert!(!played.is_empty());
        for event in &played {
            assert_eq!(event["instanceId"], json!("hidden"));
            assert_eq!(event["defId"], json!("hidden"));
            assert!(event.get("formerId").is_none());
            assert!(event.get("x").is_none());
            assert!(event.get("embiggened").is_none());
        }

        // The owner's view still reads their own card.
        let own = view_for(&state, P1);
        let mine = must(own.you.backrow.get(1).cloned().flatten(), "the set card for p1");
        let mine_shown = serde_json::to_value(&mine).expect("a backrow view serialises");
        assert_eq!(
            mine_shown["defId"],
            serde_json::to_value(filler().id).expect("an id serialises")
        );
        // The owner's own view names when their set card reveals (D3).
        assert_eq!(mine_shown["revealAt"], json!("startOfNextTurn"));
    }
}
