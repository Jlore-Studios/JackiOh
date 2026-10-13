//! Cloaked (#783, R783004–R783008): a Unit, Spell or Field Spell that has Cloaked in its controller's
//! hand may be set face-down as a Trap at its own price, hidden like any set card, and revealed only by
//! its own Activate: Reveal, which is offered while the card can come out whole. An aura may lend the
//! keyword to the cards of a type in its controller's hand. The fixtures in `fixtures/keywords_783.rs`
//! stand in for the cards #781 prints.

use jackioh_engine::testkit::*;

use super::fixtures::harness::{put, slot};
use super::fixtures::keywords_783::{
    cloak_spells, cloak_units, cloaked_bolt, cloaked_field, cloaked_field_trap, cloaked_mourner,
    cloaked_trap, cloaked_unit, grunt, plain_spell, plain_unit, register_keywords_783, second_wind,
    toe_cracker, trap_discount,
};
use super::fixtures::play_pipeline_b::{pb_reduce, plays_of};
use super::fixtures::prompt_harness::{act, answer_keys, board, cast_now, must, open_as, round_trip};

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

/// p1's main phase on turn 3 with 4 mana a side and #783's cards registered.
fn playing(seed: &str) -> GameState {
    let state = board(seed);
    register_keywords_783();
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

/// The offered plays of this card that set it face-down, as JSON.
fn face_down_offered(state: &GameState, instance_id: &str, player: PlayerId) -> Vec<Value> {
    plays_of(state, instance_id, player)
        .iter()
        .map(|play| serde_json::to_value(play).expect("a play serialises"))
        .filter(|play| play.get("faceDown").is_some_and(|timing| !timing.is_null()))
        .collect()
}

/// A refusal's text (`""` when the action was not refused).
fn refusal(result: &ReduceResult) -> String {
    result.error.clone().unwrap_or_default()
}

/// Set this card face-down by its Cloaked into this backrow lane; the set card as it stands.
fn cloak(state: &mut GameState, id: &str, lane: i32) -> CardInstance {
    *state = act(
        state,
        json!({
            "type": "play",
            "instanceId": id,
            "zone": { "row": "backrow", "lane": lane },
            "faceDown": "cloaked",
            "playerId": "p1",
        }),
        None,
    );
    must(
        state.players.p1.backrow[(lane - 1) as usize].clone(),
        "the set card in its backrow zone",
    )
}

fn reveal_body(id: &str, player: &str) -> Value {
    json!({ "type": "activate", "instanceId": id, "ability": "reveal", "playerId": player })
}

/// Whether `legal_actions` offers `player` this card's Reveal.
fn reveal_offered(state: &GameState, id: &str, player: PlayerId) -> bool {
    legal_actions(state, player).iter().any(|action| {
        matches!(action, ActionBody::Activate { instance_id, ability, .. }
            if instance_id == id && ability.as_deref() == Some("reveal"))
    })
}

/// The open prompt answered with the option that picks this instance.
fn answer_instance(state: &mut GameState, id: &str) {
    let pending = must(state.pending.clone(), "an open prompt");
    let key = must(
        pending.options.iter().find(
            |option| matches!(&option.selection, Selection::Instance { instance_id } if instance_id == id),
        ),
        "an option picking that card",
    )
    .key
    .clone();
    let answered = answer_keys(state, &[key.as_str()]);
    assert_eq!(answered.error, None);
}

/// The player's backrow zone as `view_for(viewer)` shows it, as JSON.
fn backrow_as_seen(state: &GameState, owner: PlayerId, viewer: PlayerId, lane: i32) -> Value {
    let view = serde_json::to_value(view_for(state, viewer)).expect("a view serialises");
    let side = if owner == viewer { "you" } else { "opponent" };
    view[side]["backrow"][(lane - 1) as usize].clone()
}

fn attacks_of(state: &GameState, id: &str) -> usize {
    legal_actions(state, P1)
        .iter()
        .filter(|action| matches!(action, ActionBody::Attack { attacker_id, .. } if attacker_id == id))
        .count()
}

fn end_turn(state: &GameState, player: &str) -> GameState {
    act(state, json!({ "type": "endTurn", "playerId": player }), None)
}

fn remove(state: &mut GameState, id: &str) {
    let mut card = must(find_instance(state, id).cloned(), "the card to remove");
    jackioh_engine::zones::remove_from_any_zone(state, &mut card);
}

mod r783004_cloaked_set {
    use super::*;

    #[test]
    fn r783004_unit_spell_and_field_spell_are_offered_cloaked_into_open_backrow_zones() {
        let mut state = playing("r783004-offer");
        let unit = in_hand(&mut state, &cloaked_unit().id, P1);
        let spell = in_hand(&mut state, &cloaked_bolt().id, P1);
        let field = in_hand(&mut state, &cloaked_field().id, P1);
        let plain = in_hand(&mut state, &plain_unit().id, P1);
        // One backrow zone taken: the other four are offered.
        put(
            &mut state,
            &cloaked_trap().id,
            slot(P1, Row::Backrow, 1),
            json!({}),
        );

        for card in [&unit, &spell, &field] {
            let offered = face_down_offered(&state, &card.id, P1);
            assert!(offered.iter().all(|play| play["faceDown"] == json!("cloaked")));
            let mut lanes: Vec<i64> = offered
                .iter()
                .map(|play| play["zone"]["lane"].as_i64().unwrap_or(-1))
                .collect();
            lanes.sort();
            assert_eq!(lanes, vec![2, 3, 4, 5], "{}", card.def_id);
            // No targets or modes are declared when it is set.
            assert!(
                offered
                    .iter()
                    .all(|play| play.get("targets").is_none_or(Value::is_null))
            );
        }
        // A card without Cloaked has no face-down play.
        assert_eq!(face_down_offered(&state, &plain.id, P1), Vec::<Value>::new());
        // A Unit set by its Cloaked has its own timing only.
        assert!(
            refusal(&pb_reduce(
                &state,
                json!({
                    "type": "play",
                    "instanceId": unit.id,
                    "zone": { "row": "backrow", "lane": 2 },
                    "faceDown": "startOfNextTurn",
                    "playerId": "p1",
                }),
            ))
            .contains("reveal")
        );
    }

    #[test]
    fn r783004_no_cloaked_play_of_a_trap_or_field_trap_or_from_a_graveyard() {
        let mut state = playing("r783004-no-trap");
        let trap = in_hand(&mut state, &cloaked_trap().id, P1);
        let field_trap = in_hand(&mut state, &cloaked_field_trap().id, P1);
        // A Trap or a Field Trap is set as itself, never by Cloaked.
        for card in [&trap, &field_trap] {
            assert!(!plays_of(&state, &card.id, P1).is_empty());
            assert_eq!(face_down_offered(&state, &card.id, P1), Vec::<Value>::new());
            assert!(
                refusal(&pb_reduce(
                    &state,
                    json!({
                        "type": "play",
                        "instanceId": card.id,
                        "zone": { "row": "backrow", "lane": 1 },
                        "faceDown": "cloaked",
                        "playerId": "p1",
                    }),
                ))
                .contains("face-down")
            );
        }

        // From a graveyard, a Cloaked card is played as it is printed and never set.
        put(&mut state, &second_wind().id, slot(P1, Row::Units, 1), json!({}));
        let buried = new_instance(&mut state, &cloaked_bolt().id, P1, Zone::Graveyard { player: P1 });
        state.players.p1.graveyard.push(buried.clone());
        put(&mut state, &grunt().id, slot(P2, Row::Units, 1), json!({}));
        assert!(!plays_of(&state, &buried.id, P1).is_empty());
        assert_eq!(face_down_offered(&state, &buried.id, P1), Vec::<Value>::new());
        assert!(
            refusal(&pb_reduce(
                &state,
                json!({
                    "type": "play",
                    "instanceId": buried.id,
                    "zone": { "row": "backrow", "lane": 1 },
                    "faceDown": "cloaked",
                    "playerId": "p1",
                }),
            ))
            .contains("face-down")
        );
    }

    #[test]
    fn r783004_set_with_a_fresh_id_dormant_and_hidden_from_the_opponent() {
        let mut state = playing("r783004-set");
        let unit = in_hand(&mut state, &cloaked_unit().id, P1);
        let mana = state.players.p1.mana.current;
        let set = cloak(&mut state, &unit.id, 2);

        // Its own price, a fresh id (R227), face-down and stamped `cloaked`.
        assert_eq!(state.players.p1.mana.current, mana - 3);
        assert_ne!(set.id, unit.id);
        assert_eq!(set.face_up, None);
        assert_eq!(set.set_as.map(|stamp| stamp.reveal), Some(RevealAt::Cloaked));
        // Dormant: a Field Trap with no text of its own, no Cry run.
        assert!(jackioh_engine::alt_play::is_dormant(&set));
        assert_eq!(
            jackioh_engine::faces::card_type_of(&state, &set),
            CardType::FieldTrap
        );
        assert!(state.pending.is_none());

        // Its controller reads it, its `cloaked` timing and its Reveal (R1046).
        let own = backrow_as_seen(&state, P1, P1, 2);
        assert_eq!(own["defId"], json!(cloaked_unit().id));
        assert_eq!(own["revealAt"], json!("cloaked"));
        assert!(
            own["activations"]
                .as_array()
                .is_some_and(|list| list.iter().any(|entry| entry["ability"] == json!("reveal")))
        );
        // The opponent sees a face-down zone and its cost, nothing more.
        let theirs = backrow_as_seen(&state, P1, P2, 2);
        assert_eq!(theirs["faceDown"], json!(true));
        for hidden in ["defId", "instanceId", "revealAt", "activations"] {
            assert!(theirs.get(hidden).is_none(), "{hidden} is hidden");
        }
    }
}

mod r783005_cloaked_price {
    use super::*;

    #[test]
    fn r783005_trap_price_rules_miss_it_and_it_counts_as_a_trap_play() {
        let mut state = playing("r783005-price");
        state.players.p1.mana.current = 10;
        state.players.p1.mana.max = 10;
        put(&mut state, &toe_cracker().id, slot(P1, Row::Units, 1), json!({}));
        cast_now(&mut state, &trap_discount().id, P1, false);
        let unit = in_hand(&mut state, &cloaked_unit().id, P1);
        let spell = in_hand(&mut state, &cloaked_bolt().id, P1);

        // Toe Cracker's "Your Traps cost (0)" and the next-Trap discount reach a Trap in hand...
        let trap = in_hand(&mut state, &cloaked_trap().id, P1);
        assert_eq!(
            jackioh_engine::mana::effective_cost(&state, &trap, Default::default()),
            0
        );

        // ...but a Cloaked card is priced as the card it is in hand: (3) and (1).
        let mana = state.players.p1.mana.current;
        let set_unit = cloak(&mut state, &unit.id, 2);
        assert_eq!(state.players.p1.mana.current, mana - 3);
        let set_spell = cloak(&mut state, &spell.id, 3);
        assert_eq!(state.players.p1.mana.current, mana - 4);

        // It still counts as a Trap play: a Field Trap for the Unit, a Trap for the Spell, two plays.
        let log = &state.players.p1.turn_log;
        assert_eq!(log.cards_played, 2);
        let by_type = log.played_by_type.clone().unwrap_or_default();
        assert_eq!(by_type.get(&CardType::FieldTrap).copied().unwrap_or(0), 1);
        assert_eq!(by_type.get(&CardType::Trap).copied().unwrap_or(0), 1);
        // A Combo played now counts both as cards played earlier this turn (§6.2 Combo X).
        assert_eq!(jackioh_engine::query::played_earlier(&state, P1, &trap), 2);
        assert_eq!(set_unit.set_as.map(|stamp| stamp.reveal), Some(RevealAt::Cloaked));
        assert_eq!(
            set_spell.set_as.map(|stamp| stamp.reveal),
            Some(RevealAt::Cloaked)
        );
    }
}

mod r783006_cloaked_reveal {
    use super::*;

    #[test]
    fn r783006_a_unit_reveals_cries_with_a_prompt_and_attacks_next_turn() {
        let mut state = playing("r783006-unit");
        let unit = in_hand(&mut state, &cloaked_unit().id, P1);
        let set = cloak(&mut state, &unit.id, 2);
        assert!(reveal_offered(&state, &set.id, P1));

        // Two turns on, it is still hidden: Cloaked has no timed reveal.
        state = end_turn(&state, "p1");
        state = end_turn(&state, "p2");
        assert!(state.pending.is_none());
        let still = must(state.players.p1.backrow[1].clone(), "the set card");
        assert_eq!(still.id, set.id);
        assert_eq!(still.face_up, None);

        // Its Reveal: it takes its own lane's unit zone and its Cry asks for its target.
        state = act(&state, reveal_body(&set.id, "p1"), None);
        let pending = open_as(&state, PromptKind::Target, P1);
        assert!(pending.prompt.contains("Cry"));
        let before = hero_health(&state, P2);
        assert_eq!(answer_keys(&mut state, &["hero:p2"]).error, None);
        assert_eq!(hero_health(&state, P2), before - 2);

        let revealed = must(
            state.players.p1.units[1]
                .as_ref()
                .and_then(|pile| pile.first())
                .cloned(),
            "the revealed Unit in unit lane 2",
        );
        assert_eq!(revealed.id, set.id);
        assert_eq!(revealed.set_as, None);
        assert_eq!(revealed.face_up, Some(true));
        assert_eq!(revealed.summoned_turn, Some(3));
        assert!(state.players.p1.backrow[1].is_none());
        // Set on an earlier turn, it may attack at once.
        assert!(attacks_of(&state, &set.id) > 0);
    }

    #[test]
    fn r783006_revealed_the_turn_it_was_set_it_is_sick() {
        let mut state = playing("r783006-sick");
        let unit = in_hand(&mut state, &cloaked_unit().id, P1);
        let set = cloak(&mut state, &unit.id, 1);
        state = act(&state, reveal_body(&set.id, "p1"), None);
        assert_eq!(answer_keys(&mut state, &["hero:p2"]).error, None);
        let revealed = must(
            state.players.p1.units[0]
                .as_ref()
                .and_then(|pile| pile.first())
                .cloned(),
            "the revealed Unit in unit lane 1",
        );
        assert_eq!(revealed.id, set.id);
        assert_eq!(attacks_of(&state, &set.id), 0);
        // Its Reveal is spent with the card's set form: nothing more to reveal.
        assert!(!reveal_offered(&state, &set.id, P1));
    }

    #[test]
    fn r783006_no_open_unit_zone_no_reveal_and_refused() {
        let mut state = playing("r783006-full");
        let unit = in_hand(&mut state, &cloaked_unit().id, P1);
        let set = cloak(&mut state, &unit.id, 3);
        let mut fillers: Vec<CardInstance> = Vec::new();
        for lane in 1..=5 {
            fillers.push(put(
                &mut state,
                &grunt().id,
                slot(P1, Row::Units, lane),
                json!({}),
            ));
        }

        assert!(!reveal_offered(&state, &set.id, P1));
        let own = backrow_as_seen(&state, P1, P1, 3);
        let reveal = must(
            own["activations"]
                .as_array()
                .and_then(|list| list.iter().find(|entry| entry["ability"] == json!("reveal")))
                .cloned(),
            "the Reveal on its controller's view",
        );
        assert_eq!(reveal["usable"], json!(false));
        assert!(refusal(&pb_reduce(&state, reveal_body(&set.id, "p1"))).contains("can't be activated"));

        // A zone opens — not its own lane's — and the Unit comes out into the leftmost open one (R64).
        remove(&mut state, &fillers[0].id);
        assert!(reveal_offered(&state, &set.id, P1));
        state = act(&state, reveal_body(&set.id, "p1"), None);
        assert_eq!(answer_keys(&mut state, &["hero:p2"]).error, None);
        let revealed = must(
            state.players.p1.units[0]
                .as_ref()
                .and_then(|pile| pile.first())
                .cloned(),
            "the revealed Unit in unit lane 1",
        );
        assert_eq!(revealed.id, set.id);
    }

    #[test]
    fn r783006_a_spell_resolves_with_prompts_lands_in_the_graveyard_and_round_trips() {
        let mut state = playing("r783006-spell");
        let spell = in_hand(&mut state, &cloaked_bolt().id, P1);
        let target = put(&mut state, &grunt().id, slot(P2, Row::Units, 1), json!({}));
        let set = cloak(&mut state, &spell.id, 2);
        assert_eq!(jackioh_engine::faces::card_type_of(&state, &set), CardType::Trap);

        state = act(&state, reveal_body(&set.id, "p1"), None);
        open_as(&state, PromptKind::Target, P1);
        // A paused reveal survives JSON and finishes the same (§9.3).
        let mut copy = round_trip(&state);
        answer_instance(&mut copy, &target.id);
        answer_instance(&mut state, &target.id);
        for after in [&state, &copy] {
            assert_eq!(find_instance(after, &target.id).map(|card| card.damage), Some(2));
            assert!(after.players.p1.backrow[1].is_none());
            let landed = must(
                after.players.p1.graveyard.iter().find(|card| card.id == set.id),
                "the Spell in its owner's graveyard",
            );
            assert_eq!(landed.def_id, cloaked_bolt().id);
            assert_eq!(landed.set_as, None);
        }
        assert_eq!(
            serde_json::to_value(&state).ok(),
            serde_json::to_value(&copy).ok()
        );
    }

    #[test]
    fn r783006_no_legal_target_no_reveal() {
        let mut state = playing("r783006-no-target");
        let spell = in_hand(&mut state, &cloaked_bolt().id, P1);
        // A card still to play, so the turn does not end by itself.
        in_hand(&mut state, &plain_unit().id, P1);
        // With no unit anywhere it may still be set: a set card declares nothing.
        let set = cloak(&mut state, &spell.id, 1);
        assert!(!reveal_offered(&state, &set.id, P1));
        assert!(refusal(&pb_reduce(&state, reveal_body(&set.id, "p1"))).contains("can't be activated"));

        // A unit arrives: the Reveal is offered.
        put(&mut state, &grunt().id, slot(P2, Row::Units, 1), json!({}));
        assert!(reveal_offered(&state, &set.id, P1));
    }

    #[test]
    fn r783006_a_field_spell_turns_face_up_cries_and_its_aura_starts() {
        let mut state = playing("r783006-field");
        let field = in_hand(&mut state, &cloaked_field().id, P1);
        let ally = put(&mut state, &grunt().id, slot(P1, Row::Units, 1), json!({}));
        let set = cloak(&mut state, &field.id, 4);
        // Set, its aura is dormant.
        assert_eq!(unit_view(&state, &ally).attack, 3);
        assert!(reveal_offered(&state, &set.id, P1));

        let before = hero_health(&state, P2);
        state = act(&state, reveal_body(&set.id, "p1"), None);
        assert!(state.pending.is_none());
        // Its Cry runs as it reveals.
        assert_eq!(hero_health(&state, P2), before - 1);
        let revealed = must(
            state.players.p1.backrow[3].clone(),
            "the Field Spell where it stood",
        );
        assert_eq!(revealed.id, set.id);
        assert_eq!(revealed.face_up, Some(true));
        assert_eq!(revealed.set_as, None);
        assert_eq!(
            jackioh_engine::faces::card_type_of(&state, &revealed),
            CardType::FieldSpell
        );
        assert_eq!(unit_view(&state, &live_of(&state, &ally.id)).attack, 4);
        assert!(!reveal_offered(&state, &set.id, P1));
    }

    #[test]
    fn r783006_the_opponent_cannot_reveal_it_and_it_never_reveals_by_itself() {
        let mut state = playing("r783006-theirs");
        let unit = in_hand(&mut state, &cloaked_unit().id, P1);
        let set = cloak(&mut state, &unit.id, 2);
        for name in ["p1", "p2", "p1", "p2"] {
            state = end_turn(&state, name);
            assert!(state.pending.is_none());
            let still = must(state.players.p1.backrow[1].clone(), "the set card");
            assert_eq!(still.id, set.id);
            assert_eq!(still.set_as.map(|stamp| stamp.reveal), Some(RevealAt::Cloaked));
        }
        // p1's turn again; p2 is not its controller.
        assert!(!reveal_offered(&state, &set.id, P2));
        let refused = jackioh_engine::subsystems::activate::why_cannot_activate_ability(
            &state,
            P2,
            &set.id,
            Some("reveal"),
        );
        assert!(refused.is_err());
        state = end_turn(&state, "p1");
        assert!(!reveal_offered(&state, &set.id, P2));
        assert!(refusal(&pb_reduce(&state, reveal_body(&set.id, "p2"))).contains("not yours"));
        // Nor on its controller's view during the opponent's turn: offered only on p1's own turn.
        assert!(!reveal_offered(&state, &set.id, P1));
    }

    fn live_of(state: &GameState, id: &str) -> CardInstance {
        must(find_instance(state, id).cloned(), "the card")
    }
}

mod r783007_hand_auras {
    use super::*;

    #[test]
    fn r783007_an_aura_cloaks_hand_units_or_spells_while_it_acts_a_set_card_stays_set() {
        let mut state = playing("r783007-units");
        let lender = put(
            &mut state,
            &cloak_units().id,
            slot(P1, Row::Backrow, 1),
            json!({}),
        );
        if let Some(card) = find_instance_mut(&mut state, &lender.id) {
            card.face_up = Some(true);
        }
        let unit = in_hand(&mut state, &plain_unit().id, P1);
        let spell = in_hand(&mut state, &plain_spell().id, P1);
        let theirs = in_hand(&mut state, &plain_unit().id, P2);

        // While it acts, the Units in its controller's hand have Cloaked; Spells and the opponent's
        // cards do not.
        assert!(jackioh_engine::alt_play::is_cloaked(&state, &unit));
        assert!(!face_down_offered(&state, &unit.id, P1).is_empty());
        assert_eq!(face_down_offered(&state, &spell.id, P1), Vec::<Value>::new());
        assert!(!jackioh_engine::alt_play::is_cloaked(&state, &theirs));

        // Set one; the aura leaves the field; the set card stays set, its Reveal still its own.
        let set = cloak(&mut state, &unit.id, 2);
        remove(&mut state, &lender.id);
        let other = in_hand(&mut state, &plain_unit().id, P1);
        assert!(!jackioh_engine::alt_play::is_cloaked(&state, &other));
        assert_eq!(face_down_offered(&state, &other.id, P1), Vec::<Value>::new());
        let still = must(state.players.p1.backrow[1].clone(), "the set card");
        assert_eq!(still.id, set.id);
        assert_eq!(still.set_as.map(|stamp| stamp.reveal), Some(RevealAt::Cloaked));
        assert!(reveal_offered(&state, &set.id, P1));

        // The Spell form: Spells cloaked, Units not.
        let mut spells = playing("r783007-spells");
        let lender = put(
            &mut spells,
            &cloak_spells().id,
            slot(P1, Row::Backrow, 1),
            json!({}),
        );
        if let Some(card) = find_instance_mut(&mut spells, &lender.id) {
            card.face_up = Some(true);
        }
        let unit = in_hand(&mut spells, &plain_unit().id, P1);
        let spell = in_hand(&mut spells, &plain_spell().id, P1);
        assert_eq!(face_down_offered(&spells, &unit.id, P1), Vec::<Value>::new());
        assert!(!face_down_offered(&spells, &spell.id, P1).is_empty());
        let set = cloak(&mut spells, &spell.id, 3);
        let before = hero_health(&spells, P2);
        spells = act(&spells, reveal_body(&set.id, "p1"), None);
        assert_eq!(hero_health(&spells, P2), before - 1);
        assert!(spells.players.p1.graveyard.iter().any(|card| card.id == set.id));
    }
}

mod r783008_leaving_unrevealed {
    use super::*;

    #[test]
    fn r783008_destroyed_or_bounced_unrevealed_no_cry_no_death() {
        // Destroyed while set: its printed card in its owner's graveyard, no Death.
        let mut state = playing("r783008-destroy");
        let unit = in_hand(&mut state, &cloaked_mourner().id, P1);
        let set = cloak(&mut state, &unit.id, 2);
        let before = hero_health(&state, P2);
        cast_now_effect(
            &mut state,
            jackioh_engine::effects::destroy(json_as(
                json!({ "target": { "of": "instance", "instanceId": set.id } }),
            )),
        );
        let landed = must(
            state
                .players
                .p1
                .graveyard
                .iter()
                .find(|card| card.id == set.id)
                .cloned(),
            "the set card in the graveyard",
        );
        assert_eq!(landed.def_id, cloaked_mourner().id);
        assert_eq!(landed.set_as, None);
        assert_eq!(hero_health(&state, P2), before);

        // Bounced while set: back in hand as its printed card, Cloaked again, no Cry.
        let mut bounced = playing("r783008-bounce");
        let unit = in_hand(&mut bounced, &cloaked_mourner().id, P1);
        let set = cloak(&mut bounced, &unit.id, 2);
        let before = hero_health(&bounced, P2);
        cast_now_effect(
            &mut bounced,
            jackioh_engine::effects::bounce(json_as(
                json!({ "target": { "of": "instance", "instanceId": set.id } }),
            )),
        );
        let back = must(
            bounced
                .players
                .p1
                .hand
                .iter()
                .find(|card| card.id == set.id)
                .cloned(),
            "the set card in its owner's hand",
        );
        assert_eq!(back.set_as, None);
        assert!(jackioh_engine::alt_play::is_cloaked(&bounced, &back));
        assert_eq!(hero_health(&bounced, P2), before);
        assert!(bounced.pending.is_none());
    }

    /// Apply one effect for p1 and settle, as a resolving card's would.
    fn cast_now_effect(state: &mut GameState, effect: Effect) {
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed.clone(), state.rng_cursor);
        {
            let mut sink = EngineSink::new(state, &mut events, &mut rng);
            {
                let mut ctx = make_context(
                    &mut sink,
                    None,
                    HookOptions {
                        controller: Some(P1),
                        ..Default::default()
                    },
                );
                (effect.apply)(&mut ctx);
            }
            settle(&mut sink, SettleOptions::default());
        }
        state.rng_cursor = rng.cursor();
    }
}

mod zz_scratch_probe {
    use super::*;

    #[test]
    fn zz_locked_zones_reveal() {
        let mut state = playing("zz-locked");
        let unit = in_hand(&mut state, &cloaked_unit().id, P1);
        let set = cloak(&mut state, &unit.id, 3);
        for lane in 1..=5 {
            state.players.p1.locks.units[(lane - 1) as usize] = true;
        }
        let offered = reveal_offered(&state, &set.id, P1);
        eprintln!("ZZ all unit zones empty+locked: reveal offered = {offered}");
        if offered {
            state = act(&state, reveal_body(&set.id, "p1"), None);
            let _ = answer_keys(&mut state, &["hero:p2"]);
            eprintln!("ZZ units after reveal: {:?} locks {:?} pending {:?}", state.players.p1.units.iter().map(|z| z.as_ref().map(|p| p[0].id.clone())).collect::<Vec<_>>(), state.players.p1.locks.units, state.pending.is_some());
        }
    }

    #[test]
    fn zz_discount_kept() {
        let mut state = playing("zz-discount");
        state.players.p1.mana.current = 10;
        state.players.p1.mana.max = 10;
        cast_now(&mut state, &trap_discount().id, P1, false);
        let mods_before = state.players.p1.mods.len();
        let unit = in_hand(&mut state, &cloaked_unit().id, P1);
        let spell = in_hand(&mut state, &cloaked_bolt().id, P1);
        let field = in_hand(&mut state, &cloaked_field().id, P1);
        let m = state.players.p1.mana.current;
        cloak(&mut state, &unit.id, 2);
        cloak(&mut state, &spell.id, 3);
        eprintln!("ZZ mods before {mods_before} after unit+spell {} mana spent {}", state.players.p1.mods.len(), m - state.players.p1.mana.current);
        let m = state.players.p1.mana.current;
        cloak(&mut state, &field.id, 4);
        eprintln!("ZZ field mana spent {} mods after {}", m - state.players.p1.mana.current, state.players.p1.mods.len());
        let trap = in_hand(&mut state, &cloaked_trap().id, P1);
        eprintln!("ZZ trap cost now {}", jackioh_engine::mana::effective_cost(&state, &trap, Default::default()));
    }

    #[test]
    fn zz_opponent_view_json() {
        let mut state = playing("zz-view");
        let unit = in_hand(&mut state, &cloaked_unit().id, P1);
        let _set = cloak(&mut state, &unit.id, 2);
        let v = serde_json::to_string(&view_for(&state, P2)).unwrap();
        eprintln!("ZZ p2 view mentions cloaked: {} reveal-ability: {}", v.contains("cloaked") || v.contains("Cloaked"), v.contains("\"reveal\""));
        let acts = legal_actions(&state, P2);
        eprintln!("ZZ p2 legal actions count {}", acts.len());
    }
}
