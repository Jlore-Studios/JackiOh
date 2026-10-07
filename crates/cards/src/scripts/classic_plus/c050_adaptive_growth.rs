//! C+ #50 Adaptive Growth (SPEC §8.7 row 50). (1) Spell, Epic.
//!   Base:    "Cast on draw: If you control fewer Units than your opponent, give all Units
//!            −{debuff}/−{debuff}. Otherwise, give all Units +{buff}/+{buff}." — debuff 2, buff 2
//!   Radiant: "Cast on draw: If you control fewer Units than your opponent, give enemy Units
//!            −{debuff}/−{debuff}. Otherwise, give your Units +{buff}/+{buff}." — debuff 3, buff 3
//!   Engine:  "Cast on draw (§6.2, R58); the counts are read as it resolves (Units on top of their
//!            piles). Permanent buffs (§10.4 layer 4); −2/−2 lowers max health, so units at 0 die at
//!            the state check. Tunes: buff 2 ↑ (Radiant 3); debuff 2 ↑ (Radiant 3)."
//!
//! Played from a hand it does the same (§6.2: Cast on draw is a cast, R70, and the card stays playable).
//! `conditionMet` (R195) is the same `fewer` the Cry branches on, so the glow and the branch agree; its
//! proofs are in packages/cards/test/condition-active.test.ts.

use jackioh_engine::effects::buff_all_units;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-050";

/// "If you control fewer Units than your opponent": the tops of their piles, read now.
fn fewer(state: &GameState, player: PlayerId) -> bool {
    active_units_of(state, player).len() < active_units_of(state, opponent_of(player)).len()
}

pub fn script() -> CardScripts {
    let condition_met: ConditionHook =
        condition_hook(|c| c.zone == ConditionZone::Hand && fewer(c.state, c.controller));
    let base = Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(|ctx| {
            let amount = if fewer(&*ctx.state, ctx.controller) {
                -param(&*ctx, "debuff")
            } else {
                param(&*ctx, "buff")
            };
            vec![buff_all_units(json_as(json!({ "side": "both", "attack": amount, "health": amount })))]
        })),
        condition_met: Some(condition_met.clone()),
        ..Script::default()
    };
    let radiant = Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(|ctx| {
            if fewer(&*ctx.state, ctx.controller) {
                let amount = -param(&*ctx, "debuff");
                return vec![buff_all_units(json_as(
                    json!({ "side": "enemy", "attack": amount, "health": amount }),
                ))];
            }
            let amount = param(&*ctx, "buff");
            vec![buff_all_units(json_as(json!({ "side": "self", "attack": amount, "health": amount })))]
        })),
        // TS `conditionMet: base.conditionMet`: the same hook.
        condition_met: base.condition_met.clone(),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// C+ #50 Adaptive Growth — SPEC §8.7 row 50, BUILD M9 Classic+ row C+ 50: "Cast on draw (R70, R58): if
// you control fewer Units than your opponent, every Unit on both sides gets −2/−2 (max health falls,
// units at 0 die at the state check, an Indestructible one too, R69), otherwise every Unit gets +2/+2,
// all permanent; equal counts take the second branch; played from a hand it does the same;
// `conditionMet` in hand answers whether you control fewer Units now (R195); both numbers read through
// `param()`; radiant fewer: enemy Units −3/−3; otherwise your Units +3/+3".
// The `conditionMet` proofs live in packages/cards/test/condition-active.test.ts (README §5).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const GROWTH: &str = "classicplus-050";
    const VANILLA: &str = "core-008"; // Mr. Vanilla 4/4
    const ROCK: &str = "core-066"; // The Rock, Indestructible
    const FIENDER: &str = "core-092"; // Felinor Fiender, Stack
    const FILLER: &str = "core-005";

    use crate::scenario;

    /// TS `s.unit(p, lane) ?? ""`: the unit's instance id, or "" (which no card answers to).
    fn unit_id(s: &Scenario, player: PlayerId, lane: i32) -> String {
        s.unit(player, lane).map(|card| card.id).unwrap_or_default()
    }

    /// Adaptive Growth on top of p1's library, p2 about to end their turn so p1 draws it.
    fn drawn(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "active": "p2",
            "turn": 10,
            "p1": { "field": mine, "hand": [FILLER], "library": [{ "def": GROWTH, "radiant": radiant }, FILLER, FILLER] },
            "p2": { "field": theirs, "hand": [FILLER], "library": [FILLER] },
        }))
    }

    /// Adaptive Growth in p1's hand, to play.
    fn held(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "p1": { "field": mine, "hand": [{ "def": GROWTH, "radiant": radiant }, FILLER] },
            "p2": { "field": theirs, "hand": [FILLER] },
        }))
    }

    #[test]
    fn is_a_1_spell_that_casts_itself_on_draw_on_both_faces() {
        crate::register_all();
        assert_eq!(crate::card_def(super::ID).type_, CardType::Spell);
        let scripts = super::script();
        assert_eq!(scripts.base.static_flags.as_ref().and_then(|flags| flags.cast_on_draw), Some(true));
        assert_eq!(scripts.radiant.static_flags.as_ref().and_then(|flags| flags.cast_on_draw), Some(true));
    }

    mod base {
        use super::*;

        #[test]
        fn r58_r70_drawn_it_is_cast_at_once_and_the_draw_goes_on_to_the_next_card() {
            let mut s = drawn(false, json!([VANILLA]), json!([VANILLA]));
            s.end_turn();
            s.expect_in_zone(GROWTH, "graveyard");
            let growth = s.card(GROWTH).id.clone();
            assert!(s.events().iter().any(
                |event| matches!(event, GameEvent::CardPlayed { instance_id, .. } if *instance_id == growth)
            ));
            assert_eq!(s.hand(PlayerId::P1).iter().filter(|card| card.def_id == FILLER).count(), 2);
        }

        #[test]
        fn s10_4_fewer_units_than_the_opponent_every_unit_on_both_sides_gets_2_2_max_health_too() {
            let mut s = drawn(false, json!([VANILLA]), json!([VANILLA, VANILLA]));
            s.end_turn();
            let mine = unit_id(&s, PlayerId::P1, 1);
            s.expect_stats(&mine, json!({ "attack": 2, "health": 2, "maxHealth": 2 }));
            let theirs1 = unit_id(&s, PlayerId::P2, 1);
            s.expect_stats(&theirs1, json!({ "attack": 2, "health": 2, "maxHealth": 2 }));
            let theirs2 = unit_id(&s, PlayerId::P2, 2);
            s.expect_stats(&theirs2, json!({ "attack": 2, "health": 2, "maxHealth": 2 }));
        }

        #[test]
        fn s4_5_a_unit_brought_to_0_max_health_dies_at_the_state_check() {
            let mut s = drawn(
                false,
                json!([]),
                json!([{ "def": VANILLA, "statsOverride": { "attack": 2, "health": 2 } }]),
            );
            let token = unit_id(&s, PlayerId::P2, 1);
            s.end_turn();
            s.expect_in_zone(&token, "graveyard");
            assert!(
                s.events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == token))
            );
        }

        #[test]
        fn r69_an_indestructible_unit_at_0_max_health_dies_too() {
            let mut s = drawn(
                false,
                json!([]),
                json!([{ "def": ROCK, "statsOverride": { "attack": 10, "health": 2 } }]),
            );
            let rock = unit_id(&s, PlayerId::P2, 1);
            s.end_turn();
            s.expect_in_zone(&rock, "graveyard");
        }

        #[test]
        fn otherwise_every_unit_on_both_sides_gets_2_2_permanently() {
            let mut s = drawn(false, json!([VANILLA, VANILLA]), json!([VANILLA]));
            s.end_turn();
            let mine = unit_id(&s, PlayerId::P1, 1);
            s.expect_stats(&mine, json!({ "attack": 6, "health": 6 }));
            let theirs = unit_id(&s, PlayerId::P2, 1);
            s.expect_stats(&theirs, json!({ "attack": 6, "health": 6 }));
            s.end_turn().end_turn();
            let theirs = unit_id(&s, PlayerId::P2, 1);
            s.expect_stats(&theirs, json!({ "attack": 6, "health": 6 }));
        }

        #[test]
        fn equal_counts_take_the_second_branch_2_2_an_empty_board_included() {
            let mut s = drawn(false, json!([VANILLA]), json!([VANILLA]));
            s.end_turn();
            let mine = unit_id(&s, PlayerId::P1, 1);
            s.expect_stats(&mine, json!({ "attack": 6, "health": 6 }));

            let mut empty = drawn(false, json!([]), json!([]));
            empty.end_turn();
            assert!(!empty.last_events().iter().any(|event| matches!(event, GameEvent::Buffed { .. })));
            empty.expect_in_zone(GROWTH, "graveyard");
        }

        #[test]
        fn s3_2_the_counts_read_the_tops_of_the_piles_a_stack_pile_is_one_unit() {
            let mut s = drawn(
                false,
                json!([VANILLA, { "def": FIENDER, "stack": true }]),
                json!([VANILLA, VANILLA]),
            );
            let buried = s.state().players.p1.units[0]
                .as_ref()
                .and_then(|pile| pile.iter().find(|card| card.def_id == VANILLA))
                .map(|card| card.id.clone())
                .unwrap_or_else(|| panic!("no pile"));
            s.end_turn();
            let theirs = unit_id(&s, PlayerId::P2, 1);
            s.expect_stats(&theirs, json!({ "attack": 2, "health": 2 }));
            // The dormant card is not on the field for effects (R13): it keeps its 4/4.
            assert_eq!(s.card(&buried).buffs, AttackHealth { attack: 0, health: 0 });
        }

        #[test]
        fn s6_2_played_from_a_hand_it_does_the_same() {
            let mut fewer = held(false, json!([VANILLA]), json!([VANILLA, VANILLA]));
            fewer.play(GROWTH, json!({}));
            let mine = unit_id(&fewer, PlayerId::P1, 1);
            fewer.expect_stats(&mine, json!({ "attack": 2, "health": 2 }));

            let mut more = held(false, json!([VANILLA]), json!([]));
            more.play(GROWTH, json!({}));
            let mine = unit_id(&more, PlayerId::P1, 1);
            more.expect_stats(&mine, json!({ "attack": 6, "health": 6 }));
        }

        #[test]
        fn r386_an_upgrade_moves_both_numbers_3_3_and_3_3() {
            let mut up = held(false, json!([VANILLA]), json!([]));
            step_param(up.card_mut(GROWTH), "buff", 1);
            up.play(GROWTH, json!({}));
            let mine = unit_id(&up, PlayerId::P1, 1);
            up.expect_stats(&mine, json!({ "attack": 7, "health": 7 }));

            let mut down = held(false, json!([VANILLA]), json!([VANILLA, VANILLA]));
            step_param(down.card_mut(GROWTH), "debuff", 1);
            down.play(GROWTH, json!({}));
            let mine = unit_id(&down, PlayerId::P1, 1);
            down.expect_stats(&mine, json!({ "attack": 1, "health": 1 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn fewer_units_only_enemy_units_get_3_3() {
            let mut s = drawn(true, json!([VANILLA]), json!([VANILLA, { "def": VANILLA, "damage": 1 }]));
            s.end_turn();
            let mine = unit_id(&s, PlayerId::P1, 1);
            s.expect_stats(&mine, json!({ "attack": 4, "health": 4 }));
            let theirs = unit_id(&s, PlayerId::P2, 1);
            s.expect_stats(&theirs, json!({ "attack": 1, "health": 1 }));
            assert!(s.unit(PlayerId::P2, 2).is_none());
        }

        #[test]
        fn otherwise_only_your_units_get_3_3() {
            let mut s = drawn(true, json!([VANILLA]), json!([VANILLA]));
            s.end_turn();
            let mine = unit_id(&s, PlayerId::P1, 1);
            s.expect_stats(&mine, json!({ "attack": 7, "health": 7 }));
            let theirs = unit_id(&s, PlayerId::P2, 1);
            s.expect_stats(&theirs, json!({ "attack": 4, "health": 4 }));
        }

        #[test]
        fn r386_the_radiant_debuff_and_buff_read_through_param() {
            let mut fewer = held(
                true,
                json!([VANILLA]),
                json!([{ "def": VANILLA, "statsOverride": { "attack": 9, "health": 9 } }, VANILLA]),
            );
            step_param(fewer.card_mut(GROWTH), "debuff", 1);
            fewer.play(GROWTH, json!({}));
            let theirs = unit_id(&fewer, PlayerId::P2, 1);
            fewer.expect_stats(&theirs, json!({ "attack": 5, "health": 5 }));

            let mut more = held(true, json!([VANILLA]), json!([]));
            step_param(more.card_mut(GROWTH), "buff", -1);
            more.play(GROWTH, json!({}));
            let mine = unit_id(&more, PlayerId::P1, 1);
            more.expect_stats(&mine, json!({ "attack": 6, "health": 6 }));
        }
    }
}
