//! Meditative #13 Gatling Pea (SPEC §8.8 row 13; docs/meditative-set.md M6 #13; R825).
//! (2) Unit, Epic, 2/6 → 4/12.
//!   Base:    "Armor 2 / End of turn: Deal {damage} damage to the enemy hero. Then this damage permanently
//!            goes up by {growth}." (damage 1, growth 1)
//!   Radiant: "Armor 4 / …" (damage 2, growth 2)
//!
//! At its controller's end of turn: one hit on the enemy hero through its Armor and caps (§4.4), then
//! C+ #41 KY's Constant's `set_number` on its own `damage` to the damage plus the growth (R386's
//! `tuning.set`, reported by `numberChanged`). "Permanently" is the card's own tuning, kept in every
//! zone and by a copy (R57, R78), and moved by a Nerf or Buff; the growth comes after the hit, even one
//! Armor took whole, and every extra run grows it (R825). Armor is catalog data. Both faces run one
//! script; `param` reads the face's numbers.

use jackioh_engine::effects::{damage, set_number};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-013";

pub fn script() -> CardScripts {
    let base = Script {
        end_of_turn: Some(hook(|ctx| {
            let hit = param(&*ctx, "damage");
            let growth = param(&*ctx, "growth");
            vec![
                damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": hit }))),
                set_number(json_as(json!({
                    "target": { "of": "self" },
                    "which": "param:damage",
                    "value": hit + growth,
                }))),
            ]
        })),
        ..Script::default()
    };
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// Meditative #13 Gatling Pea — SPEC §8.8 row 13, BUILD M10 row M 13: "Armor 2; at your end of turn
// only, one hit of 1 on the enemy hero (their Armor applies), then its damage becomes 2, 3 and so
// on, one more each turn (`numberChanged`), kept through a bounce, a copy (R57) and a return to
// the field (R78; R825); it grows even when the hit is fully absorbed; each M 9 or M 12 run hits
// and grows; a Nerf or Buff moves the same number; damage and growth read through `param()`;
// radiant Armor 4, damage 2, growing by 2".
//
// Core #17 Flood bounces it; Core #33 Unstable Clone Machine copies it; C+ #71/#72 are the Buff and
// Nerf verbs' cards, exercised here through the tuning helpers (R386), which step the same number
// deterministically.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FLOOD: &str = "core-017"; // (4) Spell: Bounce all Units.
    const CLONE_MACHINE: &str = "core-033"; // Field Spell: shuffle 3 copies of each played card.
    const VANILLA: &str = "core-008"; // a (1) 4/4 with no text.
    const SPARE: &str = "core-010"; // (0) Spell, never played: it only keeps the turn open (R82).

    /// A side with mana to play and a stocked library, so no turn auto-ends (§2.5) or fatigues;
    /// `extra` replaces its own entries.
    fn side(extra: Value) -> Value {
        crate::merged(
            json!({ "mana": 10, "hand": [SPARE], "library": [VANILLA, VANILLA, VANILLA] }),
            extra,
        )
    }

    /// A card's declared number as its tuning leaves it (R386), from any zone.
    fn tuned(s: &Scenario, card: &CardInstance, key: &str) -> i32 {
        jackioh_engine::prelude::param_value(
            s.state(),
            Some(card),
            key,
            jackioh_engine::prelude::ParamValueOptions::default(),
        )
    }

    fn number(s: &Scenario, card: &str, key: &str) -> i32 {
        tuned(s, &s.card(card).clone(), key)
    }

    fn number_changed(s: &Scenario) -> bool {
        s.events().iter().any(|event| matches!(event, GameEvent::NumberChanged { .. }))
    }

    #[test]
    fn is_a_2_cost_epic_armor_2_unit_hitting_1_and_growing_by_1() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.cost, CardCost::Fixed(2));
        assert_eq!(def.type_, CardType::Unit);
        assert_eq!(def.rarity, Rarity::Epic);
        assert_eq!((def.base.attack, def.base.health), (Some(2), Some(6)));
        assert_eq!((def.radiant.attack, def.radiant.health), (Some(4), Some(12)));
        assert_eq!(crate::js(&def.base.keywords), json!([{ "kind": "Armor", "n": 2 }]));
        assert_eq!(crate::js(&def.radiant.keywords), json!([{ "kind": "Armor", "n": 4 }]));
        assert_eq!(
            crate::js(&def.params),
            json!([
                { "key": "damage", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1 },
                { "key": "growth", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1 },
            ])
        );
        let scripts = super::script();
        assert!(scripts.base.end_of_turn.is_some());
        assert!(scripts.radiant.end_of_turn.is_some());
    }

    mod base {
        use super::*;

        #[test]
        fn r825_armor_2() {
            let s = scenario(json!({
                "p1": side(json!({ "field": [ID] })),
                "p2": side(json!({})),
            }));
            assert_eq!(s.stats(ID).armor, 2);
        }

        #[test]
        fn r825_hits_1_then_grows_to_2_then_2_then_3() {
            let mut s = scenario(json!({
                "p1": side(json!({ "field": [ID] })),
                "p2": side(json!({})),
            }));
            s.end_turn();
            s.expect_health(P2, 29);
            assert!(number_changed(&s));
            assert_eq!(number(&s, ID, "damage"), 2);
            s.end_turn();
            s.end_turn();
            s.expect_health(P2, 27);
            assert_eq!(number(&s, ID, "damage"), 3);
        }

        #[test]
        fn r825_nothing_at_the_opponents_end_of_turn() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": side(json!({ "field": [ID] })),
                "p2": side(json!({})),
            }));
            s.end_turn();
            s.expect_health(P1, 30);
            assert_eq!(number(&s, ID, "damage"), 1);
        }

        #[test]
        fn r825_grows_even_when_armor_takes_the_whole_hit() {
            let mut s = scenario(json!({
                "p1": side(json!({ "field": [ID] })),
                "p2": side(json!({ "armor": 1 })),
            }));
            s.end_turn();
            s.expect_health(P2, 30);
            assert_eq!(number(&s, ID, "damage"), 2);
        }

        #[test]
        fn r825_a_bounce_a_copy_and_a_replay_keep_the_grown_number() {
            let mut s = scenario(json!({
                "p1": side(json!({
                    "backrow": [CLONE_MACHINE],
                    "field": [ID],
                    "hand": [FLOOD, SPARE],
                    "library": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA],
                })),
                "p2": side(json!({})),
            }));
            // The end of turn grows the field one to damage 2.
            s.end_turn();
            s.expect_health(P2, 29);
            assert_eq!(number(&s, ID, "damage"), 2);
            s.play(FLOOD, json!({}));
            s.expect_in_zone(ID, "hand");
            assert_eq!(number(&s, ID, "damage"), 2);
            // The replay shuffles three copies at damage 2 and keeps 2 itself.
            s.play(ID, json!({}));
            assert_eq!(number(&s, ID, "damage"), 2);
            // Three copies went in; at most the turn's one draw took one out.
            let grown = s
                .pile("p1", "library")
                .into_iter()
                .filter(|card| card.def_id == ID)
                .filter(|card| tuned(&s, card, "damage") == 2)
                .count();
            assert!(grown >= 2, "the replay's copies keep damage 2, found {grown}");
            s.end_turn();
            s.expect_health(P2, 27);
            assert_eq!(number(&s, ID, "damage"), 3);
        }

        #[test]
        fn r825_a_nerf_and_a_buff_move_the_grown_number() {
            let mut s = scenario(json!({
                "p1": side(json!({ "field": [ID] })),
                "p2": side(json!({})),
            }));
            s.end_turn();
            assert_eq!(number(&s, ID, "damage"), 2);
            assert_eq!(crate::degrade_number(&mut s, ID, "damage"), 1);
            assert_eq!(crate::upgrade_number(&mut s, ID, "damage"), 2);
        }

        #[test]
        fn r386_growth_reads_through_param_a_buff_grows_by_2() {
            let mut s = scenario(json!({
                "p1": side(json!({ "field": [ID] })),
                "p2": side(json!({})),
            }));
            assert_eq!(crate::upgrade_number(&mut s, ID, "growth"), 2);
            s.end_turn();
            s.expect_health(P2, 29);
            assert_eq!(number(&s, ID, "damage"), 3);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r825_armor_4_hits_2_grows_to_4() {
            let mut s = scenario(json!({
                "p1": side(json!({ "field": [{ "def": ID, "radiant": true }] })),
                "p2": side(json!({})),
            }));
            assert_eq!(s.stats(ID).armor, 4);
            s.end_turn();
            s.expect_health(P2, 28);
            assert_eq!(number(&s, ID, "damage"), 4);
        }
    }
}
