//! TG #37 Seance (SPEC §8.11 row TG 37; §2.4, §6.3 Add to hand, R12, R57, R78, R81, R97, R1242,
//! R2198001): (2) Spell, Legendary, a card of the held Training Grounds set (R1420, R2194000).
//!   Base:    "Add an exact copy of a permanent to your hand, keeping its cost changes."
//!   Radiant: "Add an exact copy of a permanent to your hand. It costs ({setCost})." — 0.
//!
//! Engine: any permanent on either side's field (Core #34's, R81), then `add_to_hand { copyOf, exact }`
//! (R1242, R2198001): a fresh card in its caster's hand, theirs to own and control (R12), the source
//! staying, with R57's whole copy (face, buffs, keywords, Vanilla, `statsOverride`, `tuning`) and its
//! price (`costMod`, `costOverride`); damage, exertion, counters, memory, Brittle and times-played start
//! afresh. A full hand burns it (§2.4). The Radiant `costOverride: setCost` replaces the copied price.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-037";

fn seance(priced: bool) -> Script {
    Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "backrow"] }))],
        cry: Some(hook(move |ctx| {
            let Some(Selection::Instance { instance_id }) = ctx.targets.first() else {
                return vec![];
            };
            let mut args = json!({ "copyOf": instance_id, "player": "self", "exact": true });
            if priced {
                args["costOverride"] = json!(param(&*ctx, "setCost"));
            }
            vec![add_to_hand(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: seance(false),
        radiant: seance(true),
    }
}

// TG #37 Seance — SPEC §8.11 row TG 37, BUILD M13 row TG 37.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{degrade_number, filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SEANCE: &str = "traininggrounds-037";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4; Radiant 12/12.
    const WEAPONS: &str = "core-014"; // (4) Field Spell.
    const TRAP: &str = "core-041"; // Sheepish, a Trap.
    const FILLER: &str = "core-005";

    fn board(radiant: bool, hand: Value, theirs: Value, their_backrow: Value) -> Scenario {
        let mut cards = vec![json!({ "def": SEANCE, "radiant": radiant })];
        cards.extend(hand.as_array().cloned().unwrap_or_default());
        scenario(json!({
            "p1": { "hand": cards, "mana": 10, "library": filler(4) },
            "p2": { "hand": filler(1), "field": theirs, "backrow": their_backrow, "library": filler(4) },
        }))
    }

    fn on(card: &CardInstance) -> Value {
        json!({ "targets": [{ "pick": "instance", "instanceId": card.id }] })
    }

    /// The copies of `def_id` in p1's hand.
    fn copies(s: &Scenario, def_id: &str) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id == def_id).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r2198001_a_buffed_radiant_enemy_unit_gives_a_radiant_copy_with_its_buffs_and_keywords_undamaged() {
            let mut s = board(false, json!([]), json!([{ "def": VANILLA, "radiant": true, "lane": 1, "damage": 3 }]), json!([]));
            let theirs = s.unit_at(P2, 1);
            {
                let card = s.card_mut(&theirs);
                card.buffs = AttackHealth { attack: 2, health: 1 };
                card.granted_keywords.push(Keyword::Taunt);
            }
            s.play(SEANCE, on(&theirs));
            let made = copies(&s, VANILLA);
            assert_eq!(made.len(), 1);
            let copy = &made[0];
            assert!(copy.radiant);
            assert_eq!(copy.buffs, AttackHealth { attack: 2, health: 1 });
            assert!(copy.granted_keywords.contains(&Keyword::Taunt));
            assert_eq!(copy.damage, 0);
            assert_eq!((copy.owner, copy.controller), (P1, P1), "the caster's (R12)");
            assert_eq!(current_stats(s.state(), copy), Some(AttackHealth { attack: 14, health: 13 }));
        }

        #[test]
        fn r2198001_a_unit_that_cost_one_less_gives_a_copy_that_costs_one_less() {
            let mut s = board(false, json!([]), json!([VANILLA]), json!([]));
            let theirs = s.unit_at(P2, 1);
            s.card_mut(&theirs).cost_mod = -1;
            s.play(SEANCE, on(&theirs));
            let copy = &copies(&s, VANILLA)[0];
            assert_eq!(cost_now(s.state(), copy), 0, "printed (1), one less");
        }

        #[test]
        fn the_source_stays() {
            let mut s = board(false, json!([]), json!([VANILLA]), json!([]));
            let theirs = s.unit_at(P2, 1);
            s.play(SEANCE, on(&theirs));
            s.expect_in_zone(&theirs, "field");
            assert_eq!(s.card(&theirs).controller, P2);
        }

        #[test]
        fn a_field_spell_or_a_trap_can_be_copied() {
            let mut s = board(false, json!([]), json!([]), json!([{ "def": WEAPONS, "lane": 1 }, { "def": TRAP, "lane": 2 }]));
            let weapons = s.backrow(P2, 1).unwrap();
            s.play(SEANCE, on(&weapons));
            assert_eq!(copies(&s, WEAPONS).len(), 1);

            let mut s = board(false, json!([]), json!([]), json!([{ "def": TRAP, "lane": 2 }]));
            let trap = s.backrow(P2, 2).unwrap();
            s.play(SEANCE, on(&trap));
            assert_eq!(copies(&s, TRAP).len(), 1);
        }

        #[test]
        fn r2198001_brittle_and_times_played_counts_are_not_copied() {
            let mut s = board(false, json!([]), json!([VANILLA]), json!([]));
            let theirs = s.unit_at(P2, 1);
            {
                let card = s.card_mut(&theirs);
                card.brittle = Some(BrittleCounter { count: 2, since: 1, printed: None });
                card.times_played = Some(3);
            }
            s.play(SEANCE, on(&theirs));
            let copy = &copies(&s, VANILLA)[0];
            assert!(copy.brittle.is_none());
            assert!(copy.times_played.is_none());
        }

        #[test]
        fn a_full_hand_burns_it() {
            // With the Seance gone, a hand of HAND_CAP fillers is full as the copy arrives.
            let mut s = board(false, json!(vec![FILLER; HAND_CAP as usize]), json!([VANILLA]), json!([]));
            let theirs = s.unit_at(P2, 1);
            s.play(SEANCE, on(&theirs));
            assert!(copies(&s, VANILLA).is_empty(), "burned (§2.4)");
            assert!(s.state().players[P1].graveyard.iter().any(|card| card.def_id == VANILLA));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r2198001_the_copy_costs_zero_whatever_its_price() {
            let mut s = board(true, json!([]), json!([VANILLA]), json!([]));
            let theirs = s.unit_at(P2, 1);
            s.card_mut(&theirs).cost_mod = 2;
            s.play(SEANCE, on(&theirs));
            let copy = &copies(&s, VANILLA)[0];
            assert_eq!(cost_now(s.state(), copy), 0);
        }

        #[test]
        fn it_is_still_an_exact_copy() {
            let mut s = board(true, json!([]), json!([VANILLA]), json!([]));
            let theirs = s.unit_at(P2, 1);
            s.card_mut(&theirs).buffs = AttackHealth { attack: 1, health: 1 };
            s.play(SEANCE, on(&theirs));
            assert_eq!(copies(&s, VANILLA)[0].buffs, AttackHealth { attack: 1, health: 1 });
        }

        #[test]
        fn set_cost_reads_through_param() {
            let mut s = board(true, json!([]), json!([VANILLA]), json!([]));
            assert_eq!(degrade_number(&mut s, SEANCE, "setCost"), 1);
            let theirs = s.unit_at(P2, 1);
            s.card_mut(&theirs).cost_mod = 3;
            s.play(SEANCE, on(&theirs));
            assert_eq!(cost_now(s.state(), &copies(&s, VANILLA)[0]), 1);
        }
    }
}
