//! Issue #2727: in a hand a Buff and a Nerf move a card's Cloaked and Crater in opposite directions.
//! R2727000: a Buff's keyword row may add Cloaked, last, to a Unit, a Spell or a Field Spell in a hand that
//! lacks it of its own, and a Nerf's there takes off a Cloaked the card has of its own, never a lent one.
//! R2727001: a Buff's row in a hand may also take off a Crater the card has of its own, from a card of any
//! type.
//!
//! The Cloaked and Crater cards are `fixtures/keywords_783.rs`'; this file's own `fx-` defs are indexed
//! from 27270 (BUILD §0).

use jackioh_engine::alt_play::is_cloaked;
use jackioh_engine::effects::{TuneDirection, TuneRow, applicable_changes, tune_once};
use jackioh_engine::testkit::*;
use jackioh_engine::zones::is_locked;

use super::fixtures::catalog::spell_def;
use super::fixtures::harness::{act, act_result, apply_now, both, events_of_type, put, set_library, slot};
use super::fixtures::keywords_783::{
    cloak_units, cloaked_unit, crater_field, crater_trap, crater_unit, plain_spell, plain_unit,
    register_keywords_783,
};
use super::fixtures::play_pipeline_b::plays_of;
use super::fixtures::prompt_harness::board;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

/// How many Buffs or Nerfs a "never" claim is tried over, one per cursor.
const SAMPLES: u32 = 200;

/// A (1) Field Spell that does nothing.
fn quiet_field() -> CardDef {
    spell_def(27270, json!({ "type": "Field Spell" }))
}

/// A (1) Trap that does nothing.
fn quiet_trap() -> CardDef {
    spell_def(27271, json!({ "type": "Trap" }))
}

/// A (1) Field Trap that does nothing.
fn quiet_field_trap() -> CardDef {
    spell_def(27272, json!({ "type": "Field Trap" }))
}

/// A (0) Spell that does nothing: a Buff can change nothing on it but its keywords.
fn free_spell() -> CardDef {
    spell_def(27273, json!({ "cost": 0 }))
}

/// A (0) Trap with Crater that does nothing: a Buff can change nothing on it but its keywords.
fn free_crater_trap() -> CardDef {
    let face = json!({ "keywords": [{ "kind": "Crater" }], "text": "Crater" });
    spell_def(
        27274,
        json!({ "type": "Trap", "cost": 0, "base": face, "radiant": face }),
    )
}

/// p1's main phase on turn 3 with 4 mana a side, #783's cards and this file's registered.
fn playing(seed: &str) -> GameState {
    let state = board(seed);
    register_keywords_783();
    let defs = [
        quiet_field(),
        quiet_trap(),
        quiet_field_trap(),
        free_spell(),
        free_crater_trap(),
    ];
    let mut catalog = registered_catalog().clone();
    let mut scripts = registered_scripts().clone();
    for def in &defs {
        catalog.insert(def.id.clone(), def.clone());
        scripts.insert(def.id.clone(), both(Script::default()));
    }
    register_catalog(catalog);
    register_scripts(scripts);
    state
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
        .map(|play| serde_json::to_value(play).expect("a play serializes"))
        .filter(|play| play.get("faceDown").is_some_and(|timing| !timing.is_null()))
        .collect()
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

/// The set card's Activate: Reveal (R783006).
fn reveal(state: &GameState, id: &str) -> GameState {
    act(
        state,
        json!({ "type": "activate", "instanceId": id, "ability": "reveal", "playerId": "p1" }),
    )
}

/// Play this card from p1's hand into this zone, answering the whole result.
fn play_into(state: &GameState, id: &str, row: &str, lane: i32) -> ReduceResult {
    act_result(
        state,
        json!({
            "type": "play",
            "instanceId": id,
            "zone": { "row": row, "lane": lane },
            "playerId": "p1",
        }),
    )
}

/// How many zones on the board are Locked.
fn locked_count(state: &GameState) -> usize {
    [P1, P2]
        .into_iter()
        .flat_map(|player| {
            [Row::Units, Row::Backrow]
                .into_iter()
                .flat_map(move |row| (1..=5).map(move |lane| slot(player, row, lane)))
        })
        .filter(|at| is_locked(state, *at))
        .count()
}

/// The card as it stands now.
fn now(state: &GameState, id: &str) -> CardInstance {
    must(find_instance(state, id).cloned(), "the card")
}

/// One Buff or Nerf on `id`, drawn from the match rng at the state's cursor.
fn tune(state: &mut GameState, id: &str, direction: TuneDirection) {
    let card = now(state, id);
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed.clone(), state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(&mut sink, None, HookOptions::default());
        tune_once(&mut ctx, &card, direction, true);
    }
    state.rng_cursor = rng.cursor();
}

/// The state after one Buff or Nerf on `id` at the first cursor whose draw `landed` accepts.
fn tuned_where(
    state: &GameState,
    id: &str,
    direction: TuneDirection,
    landed: impl Fn(&GameState) -> bool,
) -> GameState {
    let at = |cursor: u32| {
        let mut tuned = state.clone();
        tuned.rng_cursor = cursor;
        tune(&mut tuned, id, direction);
        tuned
    };
    at(cursor_where(&state.seed, |cursor| landed(&at(cursor))))
}

/// One Buff or Nerf on `id` at each of the first `SAMPLES` cursors.
fn tuned_each(state: &GameState, id: &str, direction: TuneDirection) -> Vec<GameState> {
    (0..SAMPLES)
        .map(|cursor| {
            let mut tuned = state.clone();
            tuned.rng_cursor = cursor;
            tune(&mut tuned, id, direction);
            tuned
        })
        .collect()
}

/// The keyword kinds the card has of its own: printed, given or tuned, no aura's.
fn own_kinds(state: &GameState, id: &str) -> Vec<KeywordKind> {
    card_keywords(state, &now(state, id))
        .iter()
        .map(Keyword::kind)
        .collect()
}

/// Whether the card's tuning adds a keyword of this kind (a Buff's or a Nerf's addition).
fn tuning_adds(state: &GameState, id: &str, kind: KeywordKind) -> bool {
    now(state, id)
        .tuning
        .and_then(|tuning| tuning.add_keywords)
        .is_some_and(|added| added.iter().any(|keyword| keyword.kind() == kind))
}

/// p1's "Aura: The Units in your hand have Cloaked." face-up in its backrow zone 1.
fn lend_cloaked(state: &mut GameState) -> CardInstance {
    let lender = put(state, &cloak_units().id, slot(P1, Row::Backrow, 1), json!({}));
    if let Some(card) = find_instance_mut(state, &lender.id) {
        card.face_up = Some(true);
    }
    now(state, &lender.id)
}

mod r2727000_a_buff_in_a_hand_may_give_cloaked {
    use super::*;

    #[test]
    fn r2727000_a_buff_cloaks_a_unit_a_spell_and_a_field_spell_in_a_hand_which_then_set_and_reveal() {
        for (def, lane) in [(plain_unit(), 2), (plain_spell(), 3), (quiet_field(), 4)] {
            let mut state = playing(&format!("r2727000-cloak-{}", def.id));
            // A card still to play, so the turn does not end by itself.
            in_hand(&mut state, &plain_unit().id, P1);
            let card = in_hand(&mut state, &def.id, P1);
            assert!(!is_cloaked(&state, &card), "{}", def.id);
            assert_eq!(face_down_offered(&state, &card.id, P1), Vec::<Value>::new());

            let mut state = tuned_where(&state, &card.id, TuneDirection::Upgrade, |tuned| {
                is_cloaked(tuned, &now(tuned, &card.id))
            });
            assert!(own_kinds(&state, &card.id).contains(&KeywordKind::Cloaked));
            assert!(tuning_adds(&state, &card.id, KeywordKind::Cloaked));
            let offered = face_down_offered(&state, &card.id, P1);
            assert!(!offered.is_empty(), "{}", def.id);
            assert!(offered.iter().all(|play| play["faceDown"] == json!("cloaked")));

            // Set by its Cloaked: a Trap for the Spell, a Field Trap for the others (R783004).
            let set = cloak(&mut state, &card.id, lane);
            assert_eq!(set.set_as.map(|stamp| stamp.reveal), Some(RevealAt::Cloaked));
            let set_type = jackioh_engine::faces::card_type_of(&state, &set);
            let expected = if def.id == plain_spell().id {
                CardType::Trap
            } else {
                CardType::FieldTrap
            };
            assert_eq!(set_type, expected, "{}", def.id);

            // Its Reveal brings it out as itself (R783006).
            let before = state.players[P2].hero.health;
            let state = reveal(&state, &set.id);
            assert!(state.pending.is_none(), "{}", def.id);
            let index = (lane - 1) as usize;
            if def.id == plain_unit().id {
                let revealed = must(
                    state.players.p1.units[index]
                        .as_ref()
                        .and_then(|pile| pile.first())
                        .cloned(),
                    "the revealed Unit in its unit zone",
                );
                assert_eq!(revealed.id, set.id);
                assert_eq!(revealed.set_as, None);
            } else if def.id == plain_spell().id {
                assert_eq!(state.players[P2].hero.health, before - 1);
                assert!(state.players.p1.graveyard.iter().any(|card| card.id == set.id));
            } else {
                let revealed = must(
                    state.players.p1.backrow[index].clone(),
                    "the Field Spell where it stood",
                );
                assert_eq!(revealed.id, set.id);
                assert_eq!(revealed.face_up, Some(true));
                assert_eq!(
                    jackioh_engine::faces::card_type_of(&state, &revealed),
                    CardType::FieldSpell
                );
            }
        }
    }

    #[test]
    fn r2727000_a_buff_never_cloaks_a_trap_a_field_trap_a_vanilla_card_or_one_cloaked_of_its_own() {
        let mut state = playing("r2727000-never");
        let trap = in_hand(&mut state, &quiet_trap().id, P1);
        let field_trap = in_hand(&mut state, &quiet_field_trap().id, P1);
        let vanilla = in_hand(&mut state, &plain_unit().id, P1);
        if let Some(card) = find_instance_mut(&mut state, &vanilla.id) {
            card.vanilla = true;
        }
        let cloaked = in_hand(&mut state, &cloaked_unit().id, P1);
        for card in [&trap, &field_trap, &vanilla] {
            for tuned in tuned_each(&state, &card.id, TuneDirection::Upgrade) {
                assert!(!is_cloaked(&tuned, &now(&tuned, &card.id)), "{}", card.def_id);
                assert_eq!(face_down_offered(&tuned, &card.id, P1), Vec::<Value>::new());
            }
        }
        for tuned in tuned_each(&state, &cloaked.id, TuneDirection::Upgrade) {
            assert!(!tuning_adds(&tuned, &cloaked.id, KeywordKind::Cloaked));
        }
    }

    #[test]
    fn r2727000_a_buff_never_cloaks_a_card_in_a_deck_or_on_the_field() {
        let mut state = playing("r2727000-deck-field");
        let mut cards = set_library(
            &mut state,
            P1,
            &[plain_unit().id, plain_spell().id, quiet_field().id],
        );
        cards.push(put(
            &mut state,
            &plain_unit().id,
            slot(P1, Row::Units, 1),
            json!({}),
        ));
        cards.push(put(
            &mut state,
            &quiet_field().id,
            slot(P1, Row::Backrow, 2),
            json!({}),
        ));
        for card in &cards {
            for tuned in tuned_each(&state, &card.id, TuneDirection::Upgrade) {
                let kinds = own_kinds(&tuned, &card.id);
                assert!(
                    !kinds.contains(&KeywordKind::Cloaked),
                    "{}: {kinds:?}",
                    card.def_id
                );
            }
        }
    }

    #[test]
    fn r2727000_cloaked_is_the_rows_last_addition_after_flashback() {
        let mut state = playing("r2727000-order");
        let card = in_hand(&mut state, &free_spell().id, P1);
        // At (0) with no stats, numbers or keywords, the keyword row is all a Buff can change: one row,
        // no row draw (R129), then one draw over its items, Flashback and Cloaked in that order (R442).
        assert_eq!(
            applicable_changes(&state, &card, TuneDirection::Upgrade),
            vec![TuneRow::Keyword]
        );
        for (at, kind) in [(0, KeywordKind::Flashback), (1, KeywordKind::Cloaked)] {
            let seed = state.seed.clone();
            let mut tuned = state.clone();
            tuned.rng_cursor = cursor_where(&seed, |cursor| Rng::new(&seed, cursor).int(2) == at);
            tune(&mut tuned, &card.id, TuneDirection::Upgrade);
            assert_eq!(own_kinds(&tuned, &card.id), vec![kind]);
        }
    }

    #[test]
    fn r2727000_a_unit_an_aura_cloaks_may_take_a_buffs_cloaked_and_keeps_it_after_the_aura_leaves() {
        let mut state = playing("r2727000-lent");
        in_hand(&mut state, &plain_unit().id, P1);
        let lender = lend_cloaked(&mut state);
        let card = in_hand(&mut state, &plain_unit().id, P1);
        // Lent, not its own: the row still offers it Cloaked.
        assert!(is_cloaked(&state, &card));
        assert!(!own_kinds(&state, &card.id).contains(&KeywordKind::Cloaked));

        let mut state = tuned_where(&state, &card.id, TuneDirection::Upgrade, |tuned| {
            tuning_adds(tuned, &card.id, KeywordKind::Cloaked)
        });
        let mut gone = now(&state, &lender.id);
        jackioh_engine::zones::remove_from_any_zone(&mut state, &mut gone);

        // The aura has left: a fresh Unit in hand is plain, the Buffed one keeps its own Cloaked.
        let other = in_hand(&mut state, &plain_unit().id, P1);
        assert!(!is_cloaked(&state, &other));
        assert!(is_cloaked(&state, &now(&state, &card.id)));
        assert!(!face_down_offered(&state, &card.id, P1).is_empty());
    }

    #[test]
    fn r2727000_a_buffed_cloaked_card_set_and_bounced_unrevealed_is_cloaked_again_in_the_hand() {
        let mut state = playing("r2727000-bounce");
        in_hand(&mut state, &plain_unit().id, P1);
        let card = in_hand(&mut state, &plain_unit().id, P1);
        let mut state = tuned_where(&state, &card.id, TuneDirection::Upgrade, |tuned| {
            is_cloaked(tuned, &now(tuned, &card.id))
        });
        let set = cloak(&mut state, &card.id, 2);
        apply_now(
            &mut state,
            jackioh_engine::effects::bounce(json_as(
                json!({ "target": { "of": "instance", "instanceId": set.id } }),
            )),
        );
        // R783008: back in hand unrevealed, it is the card it was set as, its Buff included.
        let back = must(
            state
                .players
                .p1
                .hand
                .iter()
                .find(|held| held.id == set.id)
                .cloned(),
            "the set card in its owner's hand",
        );
        assert_eq!(back.set_as, None);
        assert!(is_cloaked(&state, &back));
        assert!(!face_down_offered(&state, &back.id, P1).is_empty());
    }
}

mod r2727000_a_nerf_in_a_hand_takes_off_its_own_cloaked {
    use super::*;

    #[test]
    fn r2727000_a_nerf_takes_off_a_printed_or_a_buffed_cloaked_never_a_lent_one_and_never_adds_it() {
        // Printed.
        let mut state = playing("r2727000-nerf-printed");
        in_hand(&mut state, &plain_unit().id, P1);
        let printed = in_hand(&mut state, &cloaked_unit().id, P1);
        assert!(!face_down_offered(&state, &printed.id, P1).is_empty());
        let state = tuned_where(&state, &printed.id, TuneDirection::Degrade, |tuned| {
            !is_cloaked(tuned, &now(tuned, &printed.id))
        });
        assert_eq!(face_down_offered(&state, &printed.id, P1), Vec::<Value>::new());

        // Buffed on, then Nerfed off.
        let mut state = playing("r2727000-nerf-buffed");
        in_hand(&mut state, &plain_unit().id, P1);
        let card = in_hand(&mut state, &plain_unit().id, P1);
        let state = tuned_where(&state, &card.id, TuneDirection::Upgrade, |tuned| {
            is_cloaked(tuned, &now(tuned, &card.id))
        });
        let state = tuned_where(&state, &card.id, TuneDirection::Degrade, |tuned| {
            !is_cloaked(tuned, &now(tuned, &card.id))
        });
        assert!(!tuning_adds(&state, &card.id, KeywordKind::Cloaked));
        assert_eq!(face_down_offered(&state, &card.id, P1), Vec::<Value>::new());

        // Lent by an aura: never taken off, and a Nerf never gives a card Cloaked of its own.
        let mut state = playing("r2727000-nerf-lent");
        in_hand(&mut state, &plain_unit().id, P1);
        lend_cloaked(&mut state);
        let unit = in_hand(&mut state, &plain_unit().id, P1);
        let spell = in_hand(&mut state, &plain_spell().id, P1);
        for tuned in tuned_each(&state, &unit.id, TuneDirection::Degrade) {
            assert!(is_cloaked(&tuned, &now(&tuned, &unit.id)));
        }
        for tuned in tuned_each(&state, &spell.id, TuneDirection::Degrade) {
            assert!(!own_kinds(&tuned, &spell.id).contains(&KeywordKind::Cloaked));
        }
    }
}

mod r2727001_a_buff_in_a_hand_may_take_crater_off {
    use super::*;

    fn has_crater(state: &GameState, id: &str) -> bool {
        unit_has(state, &now(state, id), KeywordKind::Crater)
    }

    #[test]
    fn r2727001_a_unit_or_a_field_spell_buffed_free_of_crater_in_a_hand_locks_nothing_as_it_arrives() {
        for (def, row, lane) in [(crater_unit(), Row::Units, 2), (crater_field(), Row::Backrow, 2)] {
            let mut state = playing(&format!("r2727001-arrive-{}", def.id));
            // A card still to play, so the turn does not end by itself.
            in_hand(&mut state, &plain_unit().id, P1);
            let card = in_hand(&mut state, &def.id, P1);
            assert!(has_crater(&state, &card.id));
            let state = tuned_where(&state, &card.id, TuneDirection::Upgrade, |tuned| {
                !has_crater(tuned, &card.id)
            });
            let row_name = if row == Row::Units { "units" } else { "backrow" };
            let result = play_into(&state, &card.id, row_name, lane);
            assert_eq!(result.error, None, "{}", def.id);
            assert!(!is_locked(&result.state, slot(P1, row, lane)), "{}", def.id);
            assert_eq!(locked_count(&result.state), 0, "{}", def.id);
        }
    }

    #[test]
    fn r2727001_a_trap_buffed_free_of_crater_in_a_hand_locks_nothing_as_it_fires() {
        for buffed in [false, true] {
            let mut state = playing(&format!("r2727001-trap-{buffed}"));
            // A card still to play, so the turn does not end by itself.
            in_hand(&mut state, &plain_unit().id, P1);
            let theirs = in_hand(&mut state, &plain_unit().id, P2);
            let trap = in_hand(&mut state, &crater_trap().id, P1);
            if buffed {
                state = tuned_where(&state, &trap.id, TuneDirection::Upgrade, |tuned| {
                    !has_crater(tuned, &trap.id)
                });
            }
            state = act(
                &state,
                json!({
                    "type": "play",
                    "instanceId": trap.id,
                    "zone": { "row": "backrow", "lane": 3 },
                    "playerId": "p1",
                }),
            );
            state = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
            // Mana enough for p2's play, whatever its turn began with.
            state.players.p2.mana.current = 4;
            let result = act_result(
                &state,
                json!({
                    "type": "play",
                    "instanceId": theirs.id,
                    "zone": { "row": "units", "lane": 1 },
                    "playerId": "p2",
                }),
            );
            assert_eq!(result.error, None);
            assert_eq!(
                events_of_type(&result.events, GameEventType::TrapFired).len(),
                1,
                "buffed {buffed}"
            );
            assert_eq!(
                is_locked(&result.state, slot(P1, Row::Backrow, 3)),
                !buffed,
                "buffed {buffed}"
            );
            assert_eq!(
                locked_count(&result.state),
                usize::from(!buffed),
                "buffed {buffed}"
            );
        }
    }

    #[test]
    fn r2727001_removals_come_first_in_the_row_before_its_additions() {
        let mut state = playing("r2727001-order");
        let card = in_hand(&mut state, &free_crater_trap().id, P1);
        // At (0) a Trap's keyword row is all a Buff can change: Crater off, then Flashback on (R2657005).
        assert_eq!(
            applicable_changes(&state, &card, TuneDirection::Upgrade),
            vec![TuneRow::Keyword]
        );
        for (at, kinds) in [
            (0, vec![]),
            (1, vec![KeywordKind::Crater, KeywordKind::Flashback]),
        ] {
            let seed = state.seed.clone();
            let mut tuned = state.clone();
            tuned.rng_cursor = cursor_where(&seed, |cursor| Rng::new(&seed, cursor).int(2) == at);
            tune(&mut tuned, &card.id, TuneDirection::Upgrade);
            assert_eq!(own_kinds(&tuned, &card.id), kinds);
        }
    }

    #[test]
    fn r2727001_a_crater_a_nerf_gave_in_a_hand_may_come_off_with_a_later_buff() {
        let mut state = playing("r2727001-nerf-then-buff");
        in_hand(&mut state, &plain_unit().id, P1);
        let card = in_hand(&mut state, &plain_unit().id, P1);
        let state = tuned_where(&state, &card.id, TuneDirection::Degrade, |tuned| {
            has_crater(tuned, &card.id)
        });
        assert!(tuning_adds(&state, &card.id, KeywordKind::Crater));
        let state = tuned_where(&state, &card.id, TuneDirection::Upgrade, |tuned| {
            !has_crater(tuned, &card.id)
        });
        assert!(!tuning_adds(&state, &card.id, KeywordKind::Crater));
        let result = play_into(&state, &card.id, "units", 1);
        assert_eq!(result.error, None);
        assert_eq!(locked_count(&result.state), 0);
    }

    #[test]
    fn r2727001_a_buff_leaves_crater_in_a_deck_and_on_the_field_and_a_nerf_never_takes_it_off() {
        let mut state = playing("r2727001-deck-field");
        let mut cards = set_library(&mut state, P1, &[crater_unit().id, crater_trap().id]);
        cards.push(put(
            &mut state,
            &crater_unit().id,
            slot(P1, Row::Units, 1),
            json!({}),
        ));
        cards.push(put(
            &mut state,
            &crater_field().id,
            slot(P1, Row::Backrow, 2),
            json!({}),
        ));
        let held = in_hand(&mut state, &crater_unit().id, P1);
        for card in &cards {
            for tuned in tuned_each(&state, &card.id, TuneDirection::Upgrade) {
                let kinds = own_kinds(&tuned, &card.id);
                assert!(kinds.contains(&KeywordKind::Crater), "{}: {kinds:?}", card.def_id);
            }
        }
        cards.push(held);
        for card in &cards {
            for tuned in tuned_each(&state, &card.id, TuneDirection::Degrade) {
                let kinds = own_kinds(&tuned, &card.id);
                assert!(kinds.contains(&KeywordKind::Crater), "{}: {kinds:?}", card.def_id);
            }
        }
    }
}
