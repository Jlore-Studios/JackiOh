//! C #77 Anti-Magic Monkey (SPEC §8.6 row 77, BUILD M9 Classic row C 77). (2) Unit 5/5 → 10/10, Common.
//!   Base:    "Stack / Aura: Spells cost ({surcharge}) more." (1)
//!   Radiant: "Stack / Aura: Spells cost ({surcharge}) more." (2)
//!   Engine:  "Cost (§6.3, R65) on both players' Spells (the Spell type, not Field Spells) where a play
//!            takes them from (a hand, or a graveyard a permission lets its owner play from, §6.3 Play),
//!            as a price for a play; a cast pays nothing (R70). Tunes: surcharge 1 ↑ (Radiant 2)."
//!
//! The aura is B5 E15's price rule (`Script.costAura`): a flat rung of the declared surcharge
//! (`param`, R386) on every player's cards of the Spell type — a card's type is its running face's
//! (B2.7) — read by R65's `effectiveCost` wherever a play would take the card from. An X-cost Spell
//! costs exactly its X and takes no rule (R65). The rule is laid only while the Monkey acts on the
//! field: dormant under a Stack pile it is not on the field for effects (§3.2, R13), and it is gone the
//! moment it leaves. Stack is printed on both faces (§10.4 layer 1), so it may be played onto an
//! occupied unit zone (§6.2).

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-077";

pub fn script() -> CardScripts {
    let base = Script {
        cost_aura: Some(read_hook(|args| {
            vec![CostAura {
                rule: CostRule {
                    types: Some(vec![CardType::Spell]),
                    amount: Some(param(&args, "surcharge")),
                    ..CostRule::default()
                },
                whose: CostAuraWhose::All,
                ban: None,
            }]
        })),
        ..Script::default()
    };

    // The same script: the Radiant face differs only in its declared surcharge (2) and its doubled stats.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C #77 Anti-Magic Monkey — SPEC §8.6 row 77, BUILD M9 Classic row C 77: "Stack; Aura: every Spell (the
// Spell type, not a Field Spell or a Trap) either player could play (in a hand, or in a graveyard a
// permission lets its owner play from, R65) costs (1) more; X-cost Spells untouched (R65); casts pay
// nothing (R70); it is off while dormant under a Stack pile and gone when it leaves; a hidden hand
// card's `costChanged` shows −1 to the other player (R177); radiant 10/10: (2) more; its tuned number
// (surcharge) reads through `param()` (R386)".
//
// The aura is B5 E15's price rule, read by R65's `effectiveCost` wherever a play takes a card from: a
// hand, or a graveyard C #28 Second Wind's permission opens (its Radiant face, so a played card lands
// in the graveyard as usual), whose plays are sent to `reduce` since the harness's `play` takes a hand
// card.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;
    use std::sync::Arc;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MONKEY: &str = "classic-077";
    const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
    const ECLIPSE: &str = "core-035"; // (1) Spell: Deal 3 damage to a target …
    const DIVIDEND: &str = "core-024"; // (X) Spell, Choose one
    const ARMOR: &str = "core-073"; // (2) Field Spell
    const HONEYPOT: &str = "core-060"; // (1) Trap
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const FIENDER: &str = "core-092"; // (2) Stack Unit
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy target Unit.
    const CN_VIRUS: &str = "core-090-1"; // (1) Spell, Cast on draw
    const RISKY_DIE: &str = "classic-079"; // (1) Spell: Draw 3; they cost (1) less; …
    const FILLER: &str = "core-010"; // (0) Spell
    const SECOND_WIND: &str = "classic-028"; // Radiant Aura: you may play cards from your graveyard that cost (1) or more.

    use crate::scenario;

    fn def() -> CardDef {
        crate::register_all();
        crate::card_def(ID)
    }

    use crate::js;

    use crate::matches_object;

    fn must<T>(value: Option<T>, what: &str) -> T {
        match value {
            Some(found) => found,
            None => panic!("missing: {what}"),
        }
    }

    /// The cost a player's own view gives a card in their hand (R65).
    fn hand_cost(s: &Scenario, player: PlayerId, def_id: &str) -> Option<i64> {
        let view = js(&s.view(player));
        let hand = view["you"]["hand"].as_array()?.clone();
        hand.iter().find(|card| card["defId"] == json!(def_id)).and_then(|card| card["cost"].as_i64())
    }

    fn played_cost(events: &[GameEvent], def_id: Option<&str>) -> Option<i32> {
        events.iter().find_map(|event| match event {
            GameEvent::CardPlayed { def_id: played, cost_paid, .. }
                if def_id.is_none_or(|wanted| played.as_str() == wanted) =>
            {
                Some(*cost_paid)
            }
            _ => None,
        })
    }

    /// is a (2) 5/5 Stack Unit (10/10 Radiant), its surcharge a declared number
    #[test]
    fn is_a_2_5_5_stack_unit_10_10_radiant_its_surcharge_a_declared_number() {
        assert_eq!(js(&def().cost), json!(2));
        assert_eq!(js(&def().base.keywords), json!([{ "kind": "Stack" }]));
        assert_eq!(js(&def().radiant.keywords), json!([{ "kind": "Stack" }]));
        assert_eq!(
            js(&def().params),
            json!([{ "key": "surcharge", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1 }])
        );
        let scripts = script();
        assert!(Arc::ptr_eq(
            scripts.base.cost_aura.as_ref().expect("a cost aura"),
            scripts.radiant.cost_aura.as_ref().expect("a cost aura")
        ));
    }

    /// base
    mod base {
        use super::*;

        /// every Spell in either player's hand costs (1) more, and a play pays it
        #[test]
        fn every_spell_in_either_player_s_hand_costs_1_more_and_a_play_pays_it() {
            let mut s = scenario(json!({ "p1": { "hand": [STOCKPILE, FILLER], "field": [MONKEY] }, "p2": { "hand": [ECLIPSE] } }));
            assert_eq!(hand_cost(&s, P1, STOCKPILE), Some(2));
            assert_eq!(hand_cost(&s, P1, FILLER), Some(1));
            assert_eq!(hand_cost(&s, P2, ECLIPSE), Some(2));
            s.play(STOCKPILE, json!({})).expect_mana(P1, 2);
            assert_eq!(played_cost(s.last_events(), None), Some(2));
        }

        /// the Spell type only: a Field Spell, a Trap and a Unit cost what they print
        #[test]
        fn the_spell_type_only_a_field_spell_a_trap_and_a_unit_cost_what_they_print() {
            let s = scenario(json!({ "p1": { "hand": [ARMOR, HONEYPOT, VANILLA], "field": [MONKEY] } }));
            assert_eq!(hand_cost(&s, P1, ARMOR), Some(2));
            assert_eq!(hand_cost(&s, P1, HONEYPOT), Some(1));
            assert_eq!(hand_cost(&s, P1, VANILLA), Some(1));
        }

        /// R65 a Spell played from a graveyard C #28 Second Wind permits pays the surcharge too
        #[test]
        fn r65_a_spell_played_from_a_graveyard_c_n28_second_wind_permits_pays_the_surcharge_too() {
            let s = scenario(json!({
                "p1": {
                    "hand": [FILLER],
                    "field": [MONKEY],
                    "backrow": [{ "def": SECOND_WIND, "radiant": true }],
                    "graveyard": [STOCKPILE],
                    "library": [VANILLA, VANILLA]
                }
            }));
            let stockpile = s.card(STOCKPILE).clone();
            let action: Action = json_as(json!({
                "type": "play",
                "playerId": "p1",
                "instanceId": stockpile.id,
                "nonce": "c77-graveyard"
            }));
            let result = reduce(s.state(), &action);
            assert!(result.error.is_none());
            let played = must(
                result.events.iter().find(|event| matches!(event, GameEvent::CardPlayed { .. })),
                "a cardPlayed",
            );
            assert!(matches_object(
                &js(played),
                &json!({ "instanceId": stockpile.id, "from": "graveyard", "costPaid": 2 })
            ));
            assert_eq!(result.state.players.p1.mana.current, 2);
        }

        /// R65 an X-cost Spell is untouched: it costs exactly its X, up to all your mana
        #[test]
        fn r65_an_x_cost_spell_is_untouched_it_costs_exactly_its_x_up_to_all_your_mana() {
            let mut s = scenario(json!({ "p1": { "hand": [DIVIDEND, FILLER], "field": [MONKEY] } }));
            assert_eq!(hand_cost(&s, P1, DIVIDEND), Some(0));
            let id = s.card(DIVIDEND).id.clone();
            let xs: Vec<i64> = legal_actions(s.state(), P1)
                .iter()
                .map(js)
                .filter(|action| action["type"] == json!("play") && action["instanceId"] == json!(id))
                .map(|action| action["x"].as_i64().unwrap_or(0))
                .collect();
            // TS `Math.max(...[])` is -Infinity, which no number equals.
            assert_eq!(xs.iter().copied().max(), Some(4));
            s.play(DIVIDEND, json!({ "x": 4, "modes": ["damage"], "targets": [{ "pick": "hero", "player": "p2" }] }))
                .expect_mana(P1, 0);
        }

        /// R70 a cast pays nothing: a Spell cast on draw is played for (0)
        #[test]
        fn r70_a_cast_pays_nothing_a_spell_cast_on_draw_is_played_for_0() {
            let mut s = scenario(json!({
                "p1": { "hand": [STOCKPILE, FILLER], "field": [MONKEY], "library": [CN_VIRUS, VANILLA, VANILLA] }
            }));
            s.play(STOCKPILE, json!({}));
            assert_eq!(played_cost(s.last_events(), Some(CN_VIRUS)), Some(0));
        }

        /// §3.2 R13 it is off while dormant under a Stack pile
        #[test]
        fn s3_2_r13_it_is_off_while_dormant_under_a_stack_pile() {
            let s = scenario(json!({
                "p1": { "hand": [STOCKPILE, FILLER], "field": [MONKEY, { "def": FIENDER, "stack": true }] }
            }));
            assert_eq!(hand_cost(&s, P1, STOCKPILE), Some(1));
        }

        /// §6.2 Stack: it may be played onto an occupied unit zone, and its aura starts as it lands
        #[test]
        fn s6_2_stack_it_may_be_played_onto_an_occupied_unit_zone_and_its_aura_starts_as_it_lands() {
            let mut s = scenario(json!({ "p1": { "hand": [MONKEY, STOCKPILE], "field": [VANILLA] } }));
            s.play(MONKEY, json!({ "zone": 1 }));
            assert_eq!(s.unit(P1, 1).map(|card| card.def_id), Some(MONKEY.to_string()));
            assert_eq!(hand_cost(&s, P1, STOCKPILE), Some(2));
        }

        /// gone when it leaves: destroyed, a Spell costs what it prints again
        #[test]
        fn gone_when_it_leaves_destroyed_a_spell_costs_what_it_prints_again() {
            let mut s = scenario(json!({ "p1": { "hand": [HIT_JOB, STOCKPILE], "field": [MONKEY], "mana": 9 } }));
            assert_eq!(hand_cost(&s, P1, HIT_JOB), Some(4));
            let monkey_id = s.card(MONKEY).id.clone();
            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": monkey_id }] }));
            s.expect_in_zone(MONKEY, "graveyard");
            assert_eq!(hand_cost(&s, P1, STOCKPILE), Some(1));
        }

        /// R65 two Monkeys add up: a Spell costs (2) more
        #[test]
        fn r65_two_monkeys_add_up_a_spell_costs_2_more() {
            let s = scenario(json!({ "p1": { "hand": [STOCKPILE], "field": [MONKEY, MONKEY] } }));
            assert_eq!(hand_cost(&s, P1, STOCKPILE), Some(3));
        }

        /// a Spell you cannot afford with the surcharge is not offered, and its play is refused
        #[test]
        fn a_spell_you_cannot_afford_with_the_surcharge_is_not_offered_and_its_play_is_refused() {
            let mut s = scenario(json!({ "p1": { "hand": [STOCKPILE, FILLER], "field": [MONKEY], "mana": 1 } }));
            let id = s.card(STOCKPILE).id.clone();
            let offered = legal_actions(s.state(), P1)
                .iter()
                .map(js)
                .any(|action| action["type"] == json!("play") && action["instanceId"] == json!(id));
            assert!(!offered);
            s.expect_refused_with(|s| s.play(STOCKPILE, json!({})), "costs 2");
        }

        /// R177 a hidden hand card's `costChanged` shows −1 and the sentinel to the other player
        #[test]
        fn r177_a_hidden_hand_card_s_costchanged_shows_1_and_the_sentinel_to_the_other_player() {
            // Radiant Risky Die keeps a drawn (1) Spell: (1) − 1 + the Monkey's (1) = (1), not more than (1).
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": RISKY_DIE, "radiant": true }, FILLER],
                    "field": [MONKEY],
                    "library": [ECLIPSE, VANILLA, VANILLA]
                }
            }));
            s.play(RISKY_DIE, json!({}));
            let eclipse = s.card(ECLIPSE).clone();
            s.expect_in_zone(&eclipse, "hand");
            let own: Vec<Value> = js(&s.view(P1))["events"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|event| event["type"] == json!("costChanged") && event["instanceId"] == json!(eclipse.id))
                .collect();
            assert_eq!(json!(own), json!([{ "type": "costChanged", "instanceId": eclipse.id, "cost": 1 }]));
            let theirs: Vec<Value> = js(&s.view(P2))["events"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|event| event["type"] == json!("costChanged"))
                .collect();
            assert!(!theirs.is_empty());
            for event in &theirs {
                assert!(matches_object(event, &json!({ "instanceId": "hidden", "cost": -1 })));
            }
        }

        /// R386 its surcharge is the declared number: an Upgrade's step makes Spells cost (2) more
        #[test]
        fn r386_its_surcharge_is_the_declared_number_an_upgrade_s_step_makes_spells_cost_2_more() {
            let mut s = scenario(json!({ "p1": { "hand": [STOCKPILE], "field": [MONKEY] } }));
            step_param(s.card_mut(MONKEY), "surcharge", 1);
            assert_eq!(hand_cost(&s, P1, STOCKPILE), Some(3));
        }
    }

    /// radiant
    mod radiant {
        use super::*;

        /// is a 10/10 with Stack
        #[test]
        fn is_a_10_10_with_stack() {
            let mut s = scenario(json!({ "p1": { "hand": [{ "def": MONKEY, "radiant": true }, FILLER] } }));
            s.play(MONKEY, json!({})).expect_stats(MONKEY, json!({ "attack": 10, "health": 10 }));
            let kinds: Vec<Value> = s.stats(MONKEY).keywords.iter().map(|keyword| js(keyword)["kind"].clone()).collect();
            assert_eq!(json!(kinds), json!(["Stack"]));
        }

        /// every Spell in either player's hand costs (2) more
        #[test]
        fn every_spell_in_either_player_s_hand_costs_2_more() {
            let s = scenario(json!({
                "p1": { "hand": [STOCKPILE], "field": [{ "def": MONKEY, "radiant": true }] },
                "p2": { "hand": [ECLIPSE, ARMOR] }
            }));
            assert_eq!(hand_cost(&s, P1, STOCKPILE), Some(3));
            assert_eq!(hand_cost(&s, P2, ECLIPSE), Some(3));
            assert_eq!(hand_cost(&s, P2, ARMOR), Some(2));
        }

        /// R386 its declared surcharge steps from 2: an Upgrade makes it (3), a Degrade (1)
        #[test]
        fn r386_its_declared_surcharge_steps_from_2_an_upgrade_makes_it_3_a_degrade_1() {
            let mut up = scenario(json!({ "p1": { "hand": [STOCKPILE], "field": [{ "def": MONKEY, "radiant": true }] } }));
            step_param(up.card_mut(MONKEY), "surcharge", 1);
            assert_eq!(hand_cost(&up, P1, STOCKPILE), Some(4));
            let mut down = scenario(json!({ "p1": { "hand": [STOCKPILE], "field": [{ "def": MONKEY, "radiant": true }] } }));
            step_param(down.card_mut(MONKEY), "surcharge", -1);
            assert_eq!(hand_cost(&down, P1, STOCKPILE), Some(2));
        }

        /// R70 a cast still pays nothing
        #[test]
        fn r70_a_cast_still_pays_nothing() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [STOCKPILE, FILLER],
                    "field": [{ "def": MONKEY, "radiant": true }],
                    "library": [CN_VIRUS, VANILLA, VANILLA],
                    "mana": 9
                }
            }));
            s.play(STOCKPILE, json!({}));
            assert_eq!(played_cost(s.last_events(), Some(CN_VIRUS)), Some(0));
        }
    }
}
