//! Cloaked (#783, R783004–R783008): a Unit, Spell or Field Spell with Cloaked in its controller's hand
//! may be set face-down as a Trap at its own price, and revealed only by its own Activate: Reveal, offered
//! while the card can come out whole. The fixtures in `fixtures/keywords_783.rs` stand in for #781's cards.

use jackioh_engine::testkit::*;

use super::fixtures::harness::{act, act_ok, act_result, apply_now, put, slot};
use super::fixtures::keywords_783::{
    REVEAL_ECHO, board_783 as playing, cloak_spells, cloak_units, cloaked_asker, cloaked_bolt,
    cloaked_charger, cloaked_echo, cloaked_field, cloaked_field_trap, cloaked_mourner, cloaked_rusher,
    cloaked_token, cloaked_trap, cloaked_unit, crater_cloaked, grunt, plain_spell, plain_unit, second_wind,
    set_resolver, summon_set, toe_cracker, trap_discount,
};
use super::fixtures::prompt_harness::{answer_keys, cast_now, open_as, round_trip};

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

fn hero_health(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].hero.health
}

fn in_hand(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
    let card = new_instance(state, def_id, player, Zone::Hand { player });
    state.players[player].hand.push(card.clone());
    card
}

fn face_down_offered(state: &GameState, instance_id: &str, player: PlayerId) -> Vec<Value> {
    plays_in(state, player, instance_id)
        .iter()
        .map(|play| serde_json::to_value(play).expect("a play serializes"))
        .filter(|play| play.get("faceDown").is_some_and(|timing| !timing.is_null()))
        .collect()
}

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
    );
    must(
        state.players.p1.backrow[(lane - 1) as usize].clone(),
        "the set card in its backrow zone",
    )
}

fn reveal_body(id: &str, player: &str) -> Value {
    json!({ "type": "activate", "instanceId": id, "ability": "reveal", "playerId": player })
}

fn reveal_offered(state: &GameState, id: &str, player: PlayerId) -> bool {
    legal_actions(state, player).iter().any(|action| {
        matches!(action, ActionBody::Activate { instance_id, ability, .. }
            if instance_id == id && ability.as_deref() == Some("reveal"))
    })
}

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

fn backrow_as_seen(state: &GameState, owner: PlayerId, viewer: PlayerId, lane: i32) -> Value {
    let view = serde_json::to_value(view_for(state, viewer)).expect("a view serializes");
    let side = if owner == viewer { "you" } else { "opponent" };
    view[side]["backrow"][(lane - 1) as usize].clone()
}

fn attack_count(state: &GameState, id: &str) -> usize {
    legal_actions(state, P1)
        .iter()
        .filter(|action| matches!(action, ActionBody::Attack { attacker_id, .. } if attacker_id == id))
        .count()
}

fn end_turn(state: &GameState, player: &str) -> GameState {
    act(state, json!({ "type": "endTurn", "playerId": player }))
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
            assert!(
                offered
                    .iter()
                    .all(|play| play.get("targets").is_none_or(Value::is_null))
            );
        }
        assert_eq!(face_down_offered(&state, &plain.id, P1), Vec::<Value>::new());
        // A Unit set by its Cloaked has its own timing only.
        assert!(
            refusal(&act_result(
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
        for card in [&trap, &field_trap] {
            assert!(!plays_in(&state, P1, &card.id).is_empty());
            assert_eq!(face_down_offered(&state, &card.id, P1), Vec::<Value>::new());
            assert!(
                refusal(&act_result(
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
        assert!(!plays_in(&state, P1, &buried.id).is_empty());
        assert_eq!(face_down_offered(&state, &buried.id, P1), Vec::<Value>::new());
        assert!(
            refusal(&act_result(
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

    /// R783005 (#3514): a next-Trap discount that names Traps and Field Traps is neither used nor spent
    /// by a Cloaked set: the Unit, the Spell and the Field Spell each pay their own price, and the
    /// discount is still there for the next Trap.
    #[test]
    fn r783005_cloaking_neither_uses_nor_spends_the_next_trap_discount() {
        let mut state = playing("r783005-discount");
        state.players.p1.mana.current = 10;
        state.players.p1.mana.max = 10;
        cast_now(&mut state, &trap_discount().id, P1, false);
        let mods = state.players.p1.mods.len();
        for (def, lane, price) in [
            (cloaked_unit(), 2, 3),
            (cloaked_bolt(), 3, 1),
            (cloaked_field(), 4, 2),
        ] {
            let card = in_hand(&mut state, &def.id, P1);
            let mana = state.players.p1.mana.current;
            cloak(&mut state, &card.id, lane);
            assert_eq!(mana - state.players.p1.mana.current, price, "{}", def.id);
            assert_eq!(state.players.p1.mods.len(), mods, "{} left the discount", def.id);
        }

        // The next Trap still meets it: its (1) less 2, floored at 0.
        let trap = in_hand(&mut state, &cloaked_trap().id, P1);
        assert_eq!(
            jackioh_engine::mana::effective_cost(&state, &trap, Default::default()),
            0
        );
    }

    /// R783005 (#3790): `query::played_type_of`, the read every card answering a play types it by, reads
    /// a card set by its Cloaked as the Trap it was set as (a Field Trap for a Unit or a Field Spell, a
    /// Trap for a Spell), as the turn's count does, and a face-up play as its own type. A set card gone
    /// from the game reads as its printed card (R783008).
    #[test]
    fn r783005_a_play_set_by_its_cloaked_reads_as_the_trap_it_was_set_as_and_a_face_up_play_as_its_own_type()
    {
        use jackioh_engine::query::played_type_of;
        let mut state = playing("r783005-played-type");
        state.players.p1.mana.current = 10;
        state.players.p1.mana.max = 10;
        let plays = [
            (cloaked_unit(), Row::Backrow, 2, true, CardType::FieldTrap),
            (cloaked_bolt(), Row::Backrow, 3, true, CardType::Trap),
            (cloaked_field(), Row::Backrow, 4, true, CardType::FieldTrap),
            (plain_unit(), Row::Units, 1, false, CardType::Unit),
        ];
        for (def, row, lane, face_down, expected) in plays {
            let card = in_hand(&mut state, &def.id, P1);
            let mut body = json!({
                "type": "play",
                "instanceId": card.id,
                "zone": { "row": row, "lane": lane },
                "playerId": "p1",
            });
            if face_down {
                body["faceDown"] = json!("cloaked");
            }
            let result = act_result(&state, body);
            assert_eq!(result.error, None, "{}", def.id);
            let mut typed = 0;
            for event in &result.events {
                match event {
                    GameEvent::CardPlayed { def_id, .. } | GameEvent::CardResolved { def_id, .. } => {
                        assert_eq!(def_id, &def.id);
                        assert_eq!(
                            played_type_of(&result.state, event),
                            Some(expected),
                            "{}: {event:?}",
                            def.id
                        );
                        typed += 1;
                    }
                    _ => assert_eq!(played_type_of(&result.state, event), None, "{event:?}"),
                }
            }
            assert_eq!(typed, 2, "{}: its cardPlayed and its cardResolved", def.id);
            state = result.state;
        }

        // R783008: a set card that has left the game is its printed card again.
        let gone: GameEvent = json_as(json!({
            "type": "cardResolved",
            "player": "p1",
            "instanceId": "c99999",
            "defId": cloaked_unit().id,
            "permanent": true,
            "costPaid": 3,
        }));
        assert_eq!(played_type_of(&state, &gone), Some(CardType::Unit));
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
        state = act(&state, reveal_body(&set.id, "p1"));
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
        // R4237000: it entered its unit zone on its reveal turn, though it was set on turn 3.
        assert_eq!(revealed.summoned_turn, Some(5));
        assert!(state.players.p1.backrow[1].is_none());
        // R4237000: revealed, it is summoning sick that turn, and attacks on its controller's next.
        assert_eq!(attack_count(&state, &set.id), 0);
        state = end_turn(&state, "p1");
        state = end_turn(&state, "p2");
        assert!(attack_count(&state, &set.id) > 0);
    }

    #[test]
    fn r783006_revealed_the_turn_it_was_set_it_is_sick() {
        let mut state = playing("r783006-sick");
        let unit = in_hand(&mut state, &cloaked_unit().id, P1);
        let set = cloak(&mut state, &unit.id, 1);
        state = act(&state, reveal_body(&set.id, "p1"));
        assert_eq!(answer_keys(&mut state, &["hero:p2"]).error, None);
        let revealed = must(
            state.players.p1.units[0]
                .as_ref()
                .and_then(|pile| pile.first())
                .cloned(),
            "the revealed Unit in unit lane 1",
        );
        assert_eq!(revealed.id, set.id);
        assert_eq!(attack_count(&state, &set.id), 0);
        // Its Reveal went with its set form.
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
        assert!(refusal(&act_result(&state, reveal_body(&set.id, "p1"))).contains("can't be activated"));

        // A zone opens — not its own lane's — and the Unit comes out into the leftmost open one (R64).
        remove(&mut state, &fillers[0].id);
        assert!(reveal_offered(&state, &set.id, P1));
        state = act(&state, reveal_body(&set.id, "p1"));
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

    /// R783006, R64, R688 (#3514): a Unit revealed while its own lane's unit zone is Locked does not take
    /// that zone though it is empty: it comes out into the leftmost open one, as a summon with no named
    /// zone prefers an unlocked zone.
    #[test]
    fn r783006_a_unit_whose_own_lane_is_locked_comes_out_in_the_leftmost_open_zone() {
        let mut state = playing("r783006-locked");
        let unit = in_hand(&mut state, &cloaked_unit().id, P1);
        let set = cloak(&mut state, &unit.id, 3);
        assert!(state.players.p1.units.iter().all(Option::is_none));
        state.players.p1.locks.units[2] = true;

        assert!(reveal_offered(&state, &set.id, P1));
        state = act(&state, reveal_body(&set.id, "p1"));
        assert_eq!(answer_keys(&mut state, &["hero:p2"]).error, None);
        let revealed = must(
            state.players.p1.units[0]
                .as_ref()
                .and_then(|pile| pile.first())
                .cloned(),
            "the revealed Unit in unit lane 1",
        );
        assert_eq!(revealed.id, set.id);
        assert!(state.players.p1.units[2].is_none(), "its own Locked lane");
        assert!(state.players.p1.backrow[2].is_none());
    }

    #[test]
    fn r783006_a_spell_resolves_with_prompts_lands_in_the_graveyard_and_round_trips() {
        let mut state = playing("r783006-spell");
        let spell = in_hand(&mut state, &cloaked_bolt().id, P1);
        let target = put(&mut state, &grunt().id, slot(P2, Row::Units, 1), json!({}));
        let set = cloak(&mut state, &spell.id, 2);
        assert_eq!(jackioh_engine::faces::card_type_of(&state, &set), CardType::Trap);

        state = act(&state, reveal_body(&set.id, "p1"));
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
        assert!(refusal(&act_result(&state, reveal_body(&set.id, "p1"))).contains("can't be activated"));

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
        state = act(&state, reveal_body(&set.id, "p1"));
        assert!(state.pending.is_none());
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
        assert!(refusal(&act_result(&state, reveal_body(&set.id, "p2"))).contains("not yours"));
        // Nor to p1 on p2's turn: offered only on its controller's own turn.
        assert!(!reveal_offered(&state, &set.id, P1));
    }

    #[test]
    fn r783006_only_a_card_set_by_its_cloaked_has_the_reveal_a_timed_set_card_has_none() {
        let mut state = playing("r783006-timed");
        let unit = in_hand(&mut state, &cloaked_unit().id, P1);
        let set = cloak(&mut state, &unit.id, 2);
        assert!(reveal_offered(&state, &set.id, P1));
        // ME-ALTPLAY's timed reveals, which no card grants and the wire keeps for old records: such a
        // set card has no Activate: Reveal, though a unit zone is open for it.
        for timing in RevealAt::ALL {
            let mut old = state.clone();
            if let Some(stamp) = find_instance_mut(&mut old, &set.id).and_then(|card| card.set_as.as_mut()) {
                stamp.reveal = timing;
            }
            assert!(!reveal_offered(&old, &set.id, P1), "{timing:?}");
        }
    }

    fn live_of(state: &GameState, id: &str) -> CardInstance {
        must(find_instance(state, id).cloned(), "the card")
    }
}

/// R4237000 (issue #4237): a Unit revealed from Cloaked enters its unit zone on its reveal turn, whatever
/// turn it was set on, so it is summoning sick then unless it has Rush or Charge (§4.1, R83, R171).
mod r4237000_a_revealed_unit_is_summoning_sick {
    use super::*;

    fn live_of(state: &GameState, id: &str) -> CardInstance {
        must(find_instance(state, id).cloned(), "the card")
    }

    fn attack_body(attacker: &str, target: &str) -> Value {
        json!({ "type": "attack", "attackerId": attacker, "targetId": target, "playerId": "p1" })
    }

    fn offered(state: &GameState, attacker: &str, target: &str) -> bool {
        legal_actions(state, P1).iter().any(|action| {
            matches!(action, ActionBody::Attack { attacker_id, target_id }
                if attacker_id == attacker && target_id == target)
        })
    }

    /// p1's view of its unit in this lane.
    fn unit_seen(state: &GameState, lane: i32) -> jackioh_engine::wire::UnitView {
        must(
            view_for(state, P1).you.units[(lane - 1) as usize].clone(),
            "p1's unit in that lane, as p1 sees it",
        )
    }

    /// `def_id` set by p1 into backrow lane 2 on turn 3 beside an enemy grunt in p2's unit lane 1, both
    /// turns ended, then revealed on p1's turn 5: the state, the set card and the grunt.
    fn revealed_later(seed: &str, def_id: &str) -> (GameState, CardInstance, CardInstance) {
        let mut state = playing(seed);
        let enemy = put(&mut state, &grunt().id, slot(P2, Row::Units, 1), json!({}));
        let unit = in_hand(&mut state, def_id, P1);
        let set = cloak(&mut state, &unit.id, 2);
        assert_eq!(state.turn, 3);
        state = end_turn(&state, "p1");
        state = end_turn(&state, "p2");
        assert_eq!(state.turn, 5);
        state = act(&state, reveal_body(&set.id, "p1"));
        (state, set, enemy)
    }

    #[test]
    fn r4237000_a_unit_revealed_a_turn_after_its_set_is_sick_that_turn_and_attacks_the_next() {
        let (mut state, set, enemy) = revealed_later("r4237000-sick", &cloaked_unit().id);
        assert_eq!(answer_keys(&mut state, &["hero:p2"]).error, None);
        assert_eq!(live_of(&state, &set.id).summoned_turn, Some(state.turn));

        // `legalActions` offers no attack by it, and the reducer refuses one at either target.
        assert_eq!(attack_count(&state, &set.id), 0);
        for target in ["hero-p2", enemy.id.as_str()] {
            assert!(
                refusal(&act_result(&state, attack_body(&set.id, target))).contains("summoning sick"),
                "an attack on {target} is refused as sick"
            );
        }

        // Its view: asleep and told why it cannot attack; it may still act, by its unspent switch (§4.1).
        let seen = unit_seen(&state, 2);
        assert_eq!(seen.instance_id, set.id);
        assert_eq!(seen.sick, Some(true));
        assert!(
            seen.attack_reason
                .is_some_and(|reason| reason.contains("summoning sick"))
        );
        assert!(seen.can_act);

        // A sick Unit may still switch to Defense.
        let switched = act(
            &state,
            json!({ "type": "switchPosition", "instanceId": set.id, "playerId": "p1" }),
        );
        assert_eq!(live_of(&switched, &set.id).position, Some(Position::Def));

        // On its controller's next turn it is awake and attacks.
        state = end_turn(&state, "p1");
        state = end_turn(&state, "p2");
        assert_eq!(unit_seen(&state, 2).sick, None);
        assert!(attack_count(&state, &set.id) > 0);
        let before = hero_health(&state, P2);
        state = act(&state, attack_body(&set.id, "hero-p2"));
        let attack = must(cloaked_unit().base.attack, "the Cloaked Unit's attack");
        assert_eq!(hero_health(&state, P2), before - attack);
    }

    #[test]
    fn r4237000_revealed_with_rush_it_may_attack_a_unit_not_the_hero_and_with_charge_the_hero_too() {
        for (def, hero_too) in [(cloaked_rusher(), false), (cloaked_charger(), true)] {
            let (state, set, enemy) = revealed_later(&format!("r4237000-{}", def.id), &def.id);
            assert!(state.pending.is_none(), "{}: no Cry, no prompt", def.id);
            assert_eq!(
                live_of(&state, &set.id).summoned_turn,
                Some(state.turn),
                "{}",
                def.id
            );
            // Sick, but neither Rush nor Charge is asleep (R849000).
            assert_eq!(unit_seen(&state, 2).sick, None, "{}", def.id);

            assert!(
                offered(&state, &set.id, &enemy.id),
                "{}: it may attack the grunt",
                def.id
            );
            assert_eq!(
                offered(&state, &set.id, "hero-p2"),
                hero_too,
                "{}: the hero",
                def.id
            );
            assert_eq!(
                act_result(&state, attack_body(&set.id, &enemy.id)).error,
                None,
                "{}: its attack on the grunt",
                def.id
            );
            let on_hero = act_result(&state, attack_body(&set.id, "hero-p2"));
            if hero_too {
                assert_eq!(on_hero.error, None, "{}: Charge hits the hero", def.id);
            } else {
                assert!(
                    refusal(&on_hero).contains("Rush cannot hit the hero"),
                    "{}",
                    def.id
                );
            }
        }
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

        // While it acts, the Units in its controller's hand have Cloaked; not Spells, not p2's cards.
        assert!(jackioh_engine::alt_play::is_cloaked(&state, &unit));
        assert!(!face_down_offered(&state, &unit.id, P1).is_empty());
        assert_eq!(face_down_offered(&state, &spell.id, P1), Vec::<Value>::new());
        assert!(!jackioh_engine::alt_play::is_cloaked(&state, &theirs));

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
        spells = act(&spells, reveal_body(&set.id, "p1"));
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

    #[test]
    fn r2855000_a_set_unit_token_destroyed_exiled_or_bounced_unrevealed_ceases_to_exist() {
        for verb in ["destroy", "exile", "bounce"] {
            let mut state = playing(&format!("r2855000-{verb}"));
            let token = in_hand(&mut state, &cloaked_token().id, P1);
            let set = cloak(&mut state, &token.id, 2);
            // Set, it is the Field Trap R783004 makes it, and still the unit token it is printed as.
            assert_eq!(
                jackioh_engine::faces::card_type_of(&state, &set),
                CardType::FieldTrap,
                "{verb}"
            );
            assert!(jackioh_engine::zones::is_unit_token(&state, &set), "{verb}");
            let target = json!({ "target": { "of": "instance", "instanceId": set.id } });
            let effect = match verb {
                "destroy" => jackioh_engine::effects::destroy(json_as(target)),
                "exile" => jackioh_engine::effects::exile(json_as(target)),
                _ => jackioh_engine::effects::bounce(json_as(target)),
            };
            cast_now_effect(&mut state, effect);
            // R11: it ceased to exist, in no graveyard, exile pile, hand or backrow zone.
            let side = &state.players.p1;
            for (pile, cards) in [
                ("graveyard", &side.graveyard),
                ("exile", &side.exile),
                ("hand", &side.hand),
            ] {
                assert!(
                    cards.iter().all(|card| card.id != set.id),
                    "{verb}: the set token is in p1's {pile}"
                );
            }
            assert!(side.backrow[1].is_none(), "{verb}: its zone is empty");
            assert!(find_instance(&state, &set.id).is_none(), "{verb}: it is nowhere");
        }
    }

    /// Apply one effect for p1 and settle, as a resolving card's would.
    fn cast_now_effect(state: &mut GameState, effect: Effect) -> Vec<GameEvent> {
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
        events
    }

    /// R4423000: destroyed while set by its Cloaked, the card died as the Field Trap it was set as
    /// (R3777000), which its `destroyed` says and the rules keep, though its definition prints a Unit.
    #[test]
    fn r4423000_a_set_cloaked_unit_destroyed_unrevealed_died_as_a_field_trap() {
        let mut state = playing("r4423000-destroy");
        let unit = in_hand(&mut state, &cloaked_mourner().id, P1);
        let set = cloak(&mut state, &unit.id, 2);
        let events = cast_now_effect(
            &mut state,
            jackioh_engine::effects::destroy(json_as(
                json!({ "target": { "of": "instance", "instanceId": set.id } }),
            )),
        );
        let died = must(
            events
                .iter()
                .find(|event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == set.id))
                .cloned(),
            "the set card's destroyed",
        );
        assert!(
            matches!(
                died,
                GameEvent::Destroyed {
                    died_as: Some(CardType::FieldTrap),
                    ..
                }
            ),
            "{died:?}"
        );
        assert!(!jackioh_engine::query::is_unit_death(
            &state,
            &died.as_rules_read()
        ));
    }
}

/// R3806000: under a card that resolves set Spells (Meditative #99 Paranoia's shape), a Spell set by its
/// Cloaked resolves as it is set, stays set and hidden, and resolves again as it reveals; a set Unit or
/// Field Spell runs no Cry at its set.
mod r3806000_set_time_resolution {
    use super::*;

    /// p1's turn with 10 mana and a Unit kept in hand so a set never ends the turn by itself (R82), and
    /// the resolver acting in backrow lane 1 when `resolver` is on.
    fn board_with(seed: &str, resolver: bool) -> GameState {
        let mut state = playing(seed);
        if resolver {
            let lender = put(
                &mut state,
                &set_resolver().id,
                slot(P1, Row::Backrow, 1),
                json!({}),
            );
            if let Some(card) = find_instance_mut(&mut state, &lender.id) {
                card.face_up = Some(true);
            }
        }
        state.players.p1.mana.current = 10;
        state.players.p1.mana.max = 10;
        in_hand(&mut state, &plain_unit().id, P1);
        state
    }

    fn resolving(seed: &str) -> GameState {
        board_with(seed, true)
    }

    fn set_body(id: &str, lane: i32) -> Value {
        json!({
            "type": "play",
            "instanceId": id,
            "zone": { "row": "backrow", "lane": lane },
            "faceDown": "cloaked",
            "playerId": "p1",
        })
    }

    fn named_to_p2(state: &GameState, def_id: &str) -> bool {
        serde_json::to_string(&view_for(state, P2))
            .expect("a view serializes")
            .contains(def_id)
    }

    #[test]
    fn r3806000_a_set_unit_or_field_spell_runs_no_cry_at_its_set_and_cries_on_its_reveal() {
        // With the resolver out or not: it resolves set Spells, never another card's Cry.
        for resolver in [false, true] {
            let mut state = board_with("r3806000-no-cry", resolver);
            let unit = in_hand(&mut state, &cloaked_unit().id, P1);
            let field = in_hand(&mut state, &cloaked_field().id, P1);
            let before = hero_health(&state, P2);
            let set_unit = cloak(&mut state, &unit.id, 2);
            assert!(state.pending.is_none(), "the Unit's Cry asks nothing at its set");
            // The Field Spell's Cry needs no choice, and still runs nothing at its set.
            let set_field = cloak(&mut state, &field.id, 3);
            assert!(state.pending.is_none());
            assert_eq!(hero_health(&state, P2), before, "no Cry ran at either set");

            // Revealed, each cries as before (R783006).
            state = act(&state, reveal_body(&set_unit.id, "p1"));
            open_as(&state, PromptKind::Target, P1);
            assert_eq!(answer_keys(&mut state, &["hero:p2"]).error, None);
            assert_eq!(hero_health(&state, P2), before - 2);
            state = act(&state, reveal_body(&set_field.id, "p1"));
            assert_eq!(hero_health(&state, P2), before - 3);
        }
    }

    #[test]
    fn r3806000_a_spell_set_under_the_resolver_resolves_with_its_prompt_stays_set_and_hidden_then_resolves_again()
     {
        let mut state = resolving("r3806000-spell");
        let spell = in_hand(&mut state, &cloaked_bolt().id, P1);
        let first = put(&mut state, &grunt().id, slot(P2, Row::Units, 1), json!({}));
        let second = put(&mut state, &grunt().id, slot(P2, Row::Units, 2), json!({}));
        let set = cloak(&mut state, &spell.id, 2);

        // Its text resolves as it is set: its Target prompt is p1's, and p2 is never told which card asks.
        let asked = open_as(&state, PromptKind::Target, P1);
        assert!(asked.prompt.starts_with("Set: "), "{}", asked.prompt);
        assert!(!named_to_p2(&state, &cloaked_bolt().id));
        // A paused set survives JSON and finishes the same (§9.3).
        let mut copy = round_trip(&state);
        answer_instance(&mut copy, &first.id);
        answer_instance(&mut state, &first.id);
        for after in [&state, &copy] {
            assert_eq!(find_instance(after, &first.id).map(|card| card.damage), Some(2));
            let still = must(
                after.players.p1.backrow[1].clone(),
                "the set Spell in backrow lane 2",
            );
            assert_eq!(still.id, set.id);
            assert_eq!(still.face_up, None);
            assert_eq!(still.set_as.and_then(|stamp| stamp.revealing), None);
            assert_eq!(jackioh_engine::faces::card_type_of(after, &still), CardType::Trap);
            assert!(reveal_offered(after, &set.id, P1));
        }
        assert_eq!(
            serde_json::to_value(&state).ok(),
            serde_json::to_value(&copy).ok()
        );

        // One Trap play, no Spell play (R70), and still a face-down card to p2.
        let by_type = state
            .players
            .p1
            .turn_log
            .played_by_type
            .clone()
            .unwrap_or_default();
        assert_eq!(by_type.get(&CardType::Trap).copied(), Some(1));
        assert_eq!(by_type.get(&CardType::Spell), None);
        assert!(!named_to_p2(&state, &cloaked_bolt().id));
        let seen = backrow_as_seen(&state, P1, P2, 2);
        assert_eq!(seen["faceDown"], json!(true));
        assert!(seen.get("defId").is_none());

        // Its Reveal asks again and resolves again, then it goes to the graveyard.
        state = act(&state, reveal_body(&set.id, "p1"));
        let asked = open_as(&state, PromptKind::Target, P1);
        assert!(asked.prompt.starts_with("Reveal: "), "{}", asked.prompt);
        answer_instance(&mut state, &second.id);
        assert_eq!(find_instance(&state, &second.id).map(|card| card.damage), Some(2));
        assert_eq!(find_instance(&state, &first.id).map(|card| card.damage), Some(2));
        assert!(state.players.p1.backrow[1].is_none());
        assert!(state.players.p1.graveyard.iter().any(|card| card.id == set.id));
    }

    #[test]
    fn r3806000_r703_with_the_resolver_out_a_spell_whose_needed_target_has_no_pick_is_not_set() {
        let mut state = resolving("r3806000-r703");
        let spell = in_hand(&mut state, &cloaked_bolt().id, P1);
        // No Unit anywhere: its needed target has no pick, so it would resolve with none.
        assert_eq!(face_down_offered(&state, &spell.id, P1), Vec::<Value>::new());
        let refused = act_result(&state, set_body(&spell.id, 2));
        assert!(
            refusal(&refused).contains("cannot resolve as it is set"),
            "{}",
            refusal(&refused)
        );
        put(&mut state, &grunt().id, slot(P2, Row::Units, 1), json!({}));
        assert!(!face_down_offered(&state, &spell.id, P1).is_empty());
    }

    #[test]
    fn r3806000_its_own_echo_repeats_at_both_resolutions() {
        let mut state = resolving("r3806000-echo");
        let spell = in_hand(&mut state, &cloaked_echo().id, P1);
        let before = hero_health(&state, P2);
        let set = cloak(&mut state, &spell.id, 2);
        assert!(state.pending.is_none());
        assert_eq!(hero_health(&state, P2), before - 2, "1, and its Echo 1 again");
        let still = must(find_instance(&state, &set.id).cloned(), "the set Spell");
        assert_eq!(still.set_as.and_then(|stamp| stamp.revealing), None);
        state = act(&state, reveal_body(&set.id, "p1"));
        assert_eq!(hero_health(&state, P2), before - 4);
        assert!(state.players.p1.graveyard.iter().any(|card| card.id == set.id));
    }

    #[test]
    fn r3806000_r113_a_text_that_asks_as_it_resolves_at_its_set_goes_dormant_once_its_tail_has_run() {
        let mut state = resolving("r3806000-asker");
        let spell = in_hand(&mut state, &cloaked_asker().id, P1);
        let before = hero_health(&state, P2);
        let set = cloak(&mut state, &spell.id, 2);
        // Its own text asks part-way through: it is still resolving as it was set, its Reveal not offered.
        open_as(&state, PromptKind::Mode, P1);
        let mid = must(find_instance(&state, &set.id).cloned(), "the set Spell");
        assert_eq!(mid.set_as.and_then(|stamp| stamp.revealing), Some(true));
        assert_eq!(mid.face_up, None);
        assert!(!named_to_p2(&state, &cloaked_asker().id));
        // Answered (and round-tripped), its tail runs and then it is a dormant set Trap again (R113).
        let mut copy = round_trip(&state);
        for after in [&mut state, &mut copy] {
            let key = must(after.pending.clone(), "the question").options[0].key.clone();
            assert_eq!(answer_keys(after, &[key.as_str()]).error, None);
            assert_eq!(hero_health(after, P2), before - 1);
            let still = must(find_instance(after, &set.id).cloned(), "the set Spell");
            assert_eq!(still.set_as.and_then(|stamp| stamp.revealing), None);
            assert!(reveal_offered(after, &set.id, P1));
        }
        assert_eq!(
            serde_json::to_value(&state).ok(),
            serde_json::to_value(&copy).ok()
        );
        state = act(&state, reveal_body(&set.id, "p1"));
        let key = must(state.pending.clone(), "the question again").options[0]
            .key
            .clone();
        assert_eq!(answer_keys(&mut state, &[key.as_str()]).error, None);
        assert_eq!(hero_health(&state, P2), before - 2);
        assert!(state.players.p1.graveyard.iter().any(|card| card.id == set.id));
    }
}

mod r783009_what_a_hidden_cloaked_card_may_be {
    use super::*;
    use jackioh_engine::alt_play::{lent_cloaked_def_ids, printed_cloaked_def_ids};

    /// This file's cards among `ids`, in their order.
    fn ours(ids: Vec<String>) -> Vec<String> {
        ids.into_iter().filter(|id| id.starts_with("kw-")).collect()
    }

    fn turn_face_up(state: &mut GameState, id: &str) {
        if let Some(card) = find_instance_mut(state, id) {
            card.face_up = Some(true);
        }
    }

    #[test]
    fn r783009_the_cards_that_print_cloaked_on_a_face_that_is_no_trap_in_catalog_order() {
        let _state = playing("r783009-printed");
        // Not the Trap nor the Field Trap that print it, nor the token, which no catalog query yields.
        assert_eq!(
            ours(printed_cloaked_def_ids()),
            vec![
                cloaked_unit().id,
                cloaked_bolt().id,
                cloaked_field().id,
                crater_cloaked().id,
                cloaked_mourner().id,
                cloaked_echo().id,
                cloaked_asker().id,
                cloaked_rusher().id,
                cloaked_charger().id,
            ]
        );
    }

    #[test]
    fn r783009_r783007_the_cards_an_aura_lends_cloaked_in_a_hand_and_none_while_no_aura_lends_it() {
        let mut state = playing("r783009-lent");
        // An aura that lends something else (the field's +1 Attack): no card is lent Cloaked, and the
        // cards that print it are left to `printed_cloaked_def_ids`.
        let field = put(
            &mut state,
            &cloaked_field().id,
            slot(P1, Row::Backrow, 1),
            json!({}),
        );
        turn_face_up(&mut state, &field.id);
        assert_eq!(lent_cloaked_def_ids(&state, P1), Vec::<String>::new());
        assert_eq!(lent_cloaked_def_ids(&state, P2), Vec::<String>::new());

        let lender = put(
            &mut state,
            &cloak_units().id,
            slot(P1, Row::Backrow, 2),
            json!({}),
        );
        turn_face_up(&mut state, &lender.id);
        let lent = lent_cloaked_def_ids(&state, P1);
        // Every Unit in p1's hand, and a card that prints Cloaked reads as Cloaked there too.
        for card in [plain_unit(), grunt(), cloaked_unit(), cloaked_bolt()] {
            assert!(lent.contains(&card.id), "{}", card.id);
        }
        // Not a Spell the aura passes over, nor a Trap or Field Trap, which a Cloaked play never sets.
        for card in [plain_spell(), cloak_units(), cloaked_trap(), cloaked_field_trap()] {
            assert!(!lent.contains(&card.id), "{}", card.id);
        }
        // p2's hand is lent nothing: only the cards that print Cloaked read as Cloaked there.
        assert_eq!(lent_cloaked_def_ids(&state, P2), printed_cloaked_def_ids());
    }
}

mod r1045_reveal_echo {
    use super::*;

    #[test]
    fn r1045_a_spell_revealed_while_a_radiant_paranoia_acts_resolves_with_its_echo() {
        let mut state = playing("r1045-reveal-echo");
        let lender = put(
            &mut state,
            &cloak_spells().id,
            slot(P1, Row::Backrow, 1),
            json!({ "radiant": true }),
        );
        if let Some(card) = find_instance_mut(&mut state, &lender.id) {
            card.face_up = Some(true);
        }
        let spell = in_hand(&mut state, &plain_spell().id, P1);
        let set = cloak(&mut state, &spell.id, 3);
        let before = hero_health(&state, P2);
        state = act(&state, reveal_body(&set.id, "p1"));
        // The Spell deals 1 as it resolves, then once more for each Echo the Radiant aura adds.
        assert_eq!(hero_health(&state, P2), before - (1 + REVEAL_ECHO));
        assert!(state.players.p1.graveyard.iter().any(|card| card.id == set.id));
    }
}

mod r3777000_a_set_card_is_a_backrow_card {
    use super::*;

    /// The backrow cards P2 may name for "target backrow card" (#36 Magic Jammed's declaration), either side.
    fn backrow_picks(state: &mut GameState) -> Vec<String> {
        let declarer = in_hand(state, &plain_spell().id, P2);
        let decl = TargetDecl::target(1, 1, json!({ "side": "any", "of": ["backrow"] }));
        jackioh_engine::play_choices::legal_selections_for(state, P2, &declarer, &decl)
            .into_iter()
            .filter_map(|selection| match selection {
                Selection::Instance { instance_id } => Some(instance_id),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn r3777000_a_set_unit_spell_and_field_spell_are_each_a_backrow_card_to_target() {
        let mut state = playing("r3777000-set");
        state.players.p1.mana.current = 10;
        state.players.p1.mana.max = 10;
        let mut set = Vec::new();
        for (def, lane) in [(cloaked_unit(), 2), (cloaked_bolt(), 3), (cloaked_field(), 4)] {
            let card = in_hand(&mut state, &def.id, P1);
            set.push(cloak(&mut state, &card.id, lane).id);
        }

        let picks = backrow_picks(&mut state);
        for id in &set {
            assert!(picks.contains(id), "{id} among {picks:?}");
        }
    }

    #[test]
    fn r3777000_revealed_into_a_unit_zone_the_unit_is_no_backrow_card() {
        let mut state = playing("r3777000-revealed");
        let unit = in_hand(&mut state, &cloaked_unit().id, P1);
        let set = cloak(&mut state, &unit.id, 2);
        assert!(backrow_picks(&mut state).contains(&set.id));

        state = act(&state, reveal_body(&set.id, "p1"));
        assert_eq!(answer_keys(&mut state, &["hero:p2"]).error, None);

        assert!(state.players.p1.backrow[1].is_none());
        assert!(!backrow_picks(&mut state).contains(&set.id));
    }
}

/// R4327000 (issue #4327): a summon's `setCloaked` sets its fresh Unit face-down by its Cloaked, standing
/// as a Cloaked play does; R4327001: neither that set nor its reveal is a Unit summoned.
mod r4327000_a_summon_set_by_its_cloaked {
    use super::*;

    /// What `summon_set` summons, in its order: a Cloaked mourner, a Cloaked token and a plain unit the
    /// summon gives Cloaked.
    fn summoned_defs() -> [String; 3] {
        [cloaked_mourner().id, cloaked_token().id, plain_unit().id]
    }

    /// p1 casts `summon_set`, every backrow zone taken by a set Trap first when `full_backrow`: the
    /// state and the cast's events.
    fn cast_set(seed: &str, full_backrow: bool) -> (GameState, Vec<GameEvent>) {
        let mut state = playing(seed);
        if full_backrow {
            for lane in 1..=5 {
                put(
                    &mut state,
                    &cloaked_trap().id,
                    slot(P1, Row::Backrow, lane),
                    json!({}),
                );
            }
        }
        let spell = in_hand(&mut state, &summon_set().id, P1);
        let cast = act_ok(
            &state,
            json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" }),
        );
        (cast.state, cast.events)
    }

    /// The card set in p1's backrow lane `lane`.
    fn set_in(state: &GameState, lane: i32) -> CardInstance {
        must(
            state.players.p1.backrow[(lane - 1) as usize].clone(),
            "a card set in that backrow zone",
        )
    }

    #[test]
    fn r4327000_each_summon_is_set_face_down_in_the_leftmost_backrow_zones_inert_and_hidden() {
        let before = hero_health(&playing("r4327000-set"), P2);
        let (state, events) = cast_set("r4327000-set", false);
        assert!(state.pending.is_none(), "no Cry, no prompt");
        assert_eq!(hero_health(&state, P2), before, "the mourner's Cry does not run");
        assert!(
            state.players.p1.units.iter().all(Option::is_none),
            "nothing takes the unit row"
        );

        for (lane, def_id) in (1..).zip(summoned_defs()) {
            let set = set_in(&state, lane);
            assert_eq!(set.def_id, def_id, "lane {lane}");
            assert_eq!(
                set.set_as.map(|stamp| stamp.reveal),
                Some(RevealAt::Cloaked),
                "{def_id}"
            );
            assert_eq!(set.face_up, None, "{def_id} is face-down");
            assert_eq!(set.position, None, "{def_id} stands in no position");
            assert_eq!(
                jackioh_engine::faces::card_type_of(&state, &set),
                CardType::FieldTrap,
                "{def_id}"
            );
            assert!(
                jackioh_engine::alt_play::is_dormant(&set),
                "{def_id}'s text is inert"
            );
            assert!(
                !jackioh_engine::zones::is_standing_unit(&state, &set),
                "{def_id} is no Unit"
            );

            // Its controller reads it and its Reveal; the opponent a card back.
            let own = backrow_as_seen(&state, P1, P1, lane);
            assert_eq!(own["defId"], json!(def_id));
            assert_eq!(own["revealAt"], json!("cloaked"));
            let theirs = backrow_as_seen(&state, P1, P2, lane);
            assert_eq!(theirs["faceDown"], json!(true), "{def_id}");
            for hidden in ["defId", "instanceId", "revealAt", "activations", "keywords"] {
                assert!(theirs.get(hidden).is_none(), "{def_id}: {hidden} is hidden");
            }
        }

        // The rider's Cloaked lands on the plain unit, unannounced.
        let plain = set_in(&state, 3);
        assert!(plain.granted_keywords.contains(&Keyword::Cloaked));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, GameEvent::KeywordGranted { .. })),
            "no keywordGranted names a set card"
        );

        // P2 reads three summons of hidden cards.
        let seen = serde_json::to_value(view_for(&state, P2)).expect("a view serializes");
        let summoned: Vec<&Value> = seen["events"]
            .as_array()
            .map(|events| {
                events
                    .iter()
                    .filter(|event| event["type"] == json!("summoned"))
                    .collect()
            })
            .unwrap_or_default();
        assert_eq!(summoned.len(), 3);
        for event in summoned {
            assert_eq!(event["instanceId"], json!(HIDDEN_ID), "{event}");
            assert_eq!(event["defId"], json!(HIDDEN_ID), "{event}");
        }
    }

    #[test]
    fn r4327000_r4237000_its_reveal_is_offered_and_the_unit_comes_out_summoning_sick() {
        let (mut state, _) = cast_set("r4327000-reveal", false);
        for lane in 1..=3 {
            let set = set_in(&state, lane);
            assert!(reveal_offered(&state, &set.id, P1), "{}", set.def_id);
            assert!(
                !reveal_offered(&state, &set.id, P2),
                "{}: only its controller",
                set.def_id
            );
        }

        // Revealed a turn later it enters its unit zone that turn, summoning sick, and its Cry runs as
        // any Cloaked reveal's does (R783006).
        let mourner = set_in(&state, 1);
        state = end_turn(&state, "p1");
        state = end_turn(&state, "p2");
        let before = hero_health(&state, P2);
        state = act(&state, reveal_body(&mourner.id, "p1"));
        assert_eq!(hero_health(&state, P2), before - 1, "the mourner's Cry pings");
        let revealed = must(
            find_instance(&state, &mourner.id).cloned(),
            "the revealed mourner",
        );
        assert!(matches!(
            revealed.zone,
            Zone::Field {
                row: Row::Units,
                lane: 1,
                ..
            }
        ));
        assert_eq!(revealed.set_as, None);
        assert_eq!(revealed.summoned_turn, Some(state.turn));
        assert_eq!(attack_count(&state, &mourner.id), 0, "summoning sick");
    }

    #[test]
    fn r4327000_r2855000_unrevealed_a_set_token_ceases_to_exist_and_a_card_reaches_its_graveyard_as_printed()
    {
        let (mut state, _) = cast_set("r4327000-leave", false);
        let before = hero_health(&state, P2);
        let mourner = set_in(&state, 1);
        let token = set_in(&state, 2);
        assert!(jackioh_engine::zones::is_unit_token(&state, &token));

        let destroy = |id: &str| {
            jackioh_engine::effects::destroy(json_as(
                json!({ "target": { "of": "instance", "instanceId": id } }),
            ))
        };
        apply_now(&mut state, destroy(&token.id));
        assert!(find_instance(&state, &token.id).is_none(), "the token is nowhere");
        assert!(state.players.p1.backrow[1].is_none());

        apply_now(&mut state, destroy(&mourner.id));
        let landed = must(
            state
                .players
                .p1
                .graveyard
                .iter()
                .find(|card| card.id == mourner.id)
                .cloned(),
            "the mourner in its owner's graveyard",
        );
        assert_eq!(landed.def_id, cloaked_mourner().id);
        assert_eq!(landed.set_as, None);
        assert_eq!(hero_health(&state, P2), before, "no Death");
    }

    #[test]
    fn r4327000_with_no_backrow_zone_open_each_is_summoned_face_up_into_the_unit_row() {
        let (state, events) = cast_set("r4327000-full", true);
        for (lane, def_id) in (1..).zip(summoned_defs()) {
            let unit = must(
                state.players.p1.units[(lane - 1) as usize]
                    .as_ref()
                    .and_then(|pile| pile.first().cloned()),
                "a unit in that lane",
            );
            assert_eq!(unit.def_id, def_id, "unit lane {lane}");
            assert_eq!(unit.set_as, None, "{def_id}");
            assert!(jackioh_engine::zones::is_standing_unit(&state, &unit), "{def_id}");
        }
        // Face-up, its rider is announced as any summon's is.
        let plain = must(
            state.players.p1.units[2]
                .as_ref()
                .and_then(|pile| pile.first().cloned()),
            "the plain unit",
        );
        assert!(events.iter().any(|event| matches!(event,
            GameEvent::KeywordGranted { instance_id, .. } if *instance_id == plain.id)));
    }

    #[test]
    fn r4327001_a_set_summon_is_no_unit_arrival_and_its_reveal_no_summon() {
        let (state, events) = cast_set("r4327001-arrival", false);
        let summons: Vec<&GameEvent> = events
            .iter()
            .filter(|event| matches!(event, GameEvent::Summoned { .. }))
            .collect();
        assert_eq!(summons.len(), 3);
        for event in summons {
            assert!(
                matches!(
                    event,
                    GameEvent::Summoned {
                        row: Row::Backrow,
                        ..
                    }
                ),
                "{event:?}"
            );
            assert_eq!(
                jackioh_engine::query::enemy_unit_arrival(&state, P2, event),
                None,
                "{event:?}"
            );
        }

        let token = set_in(&state, 2);
        let reveal = act_ok(&state, reveal_body(&token.id, "p1"));
        assert!(reveal.events.iter().any(|event| matches!(event,
            GameEvent::Animated { instance_id, .. } if *instance_id == token.id)));
        assert!(
            !reveal
                .events
                .iter()
                .any(|event| matches!(event, GameEvent::Summoned { .. })),
            "a reveal is no summon"
        );
    }

    #[test]
    fn r4327000_a_named_lane_or_a_card_that_is_no_unit_lands_as_without_the_option() {
        let mut state = playing("r4327000-not-set");
        // A Unit named into a lane takes that unit zone face-up.
        apply_now(
            &mut state,
            jackioh_engine::effects::summon(json_as(
                json!({ "defId": cloaked_mourner().id, "setCloaked": true, "lane": 2 }),
            )),
        );
        let named = must(
            state.players.p1.units[1]
                .as_ref()
                .and_then(|pile| pile.first().cloned()),
            "the mourner in unit lane 2",
        );
        assert_eq!(named.set_as, None);
        assert!(state.players.p1.backrow.iter().all(Option::is_none));

        // A Field Spell is summoned face-up into the backrow, as any summon of one is.
        apply_now(
            &mut state,
            jackioh_engine::effects::summon(json_as(
                json!({ "defId": cloaked_field().id, "setCloaked": true }),
            )),
        );
        let field = set_in(&state, 1);
        assert_eq!(field.def_id, cloaked_field().id);
        assert_eq!(field.set_as, None);
        assert_eq!(field.face_up, Some(true));
    }
}
