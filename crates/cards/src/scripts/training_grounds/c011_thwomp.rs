//! TG #11 Thwomp (SPEC §8.11 row TG 11; §4.4, R81, R90, R781001): (2) Spell, Common, a card of the
//! held Training Grounds set (R1420, R2194000).
//!   Base:    "Deal {damage} damage to a Unit." — 10.
//!   Radiant: the same text — 100.
//! C #12 Book of Blood without Lifesteal: target unit (not hero, R90; R781001 Immutable). Spell Damage adds (§4.4 step 0).

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-011";

pub fn script() -> CardScripts {
    let base = Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))],
        cry: Some(hook(|ctx| {
            vec![damage(json_as(json!({
                "to": { "of": "chosen" },
                "amount": param(&*ctx, "damage"),
            })))]
        })),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// TG #11 Thwomp — SPEC §8.11 row TG 11, BUILD M13 row TG 11.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario, upgrade_number};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const THWOMP: &str = "traininggrounds-011";
    const GREASER: &str = "meditative-100"; // (2) Unit 7/7.
    const BIG: &str = "core-019"; // (3) Midrange Menace 9/9, Taunt.
    const ARMORED: &str = "core-025"; // (4) Unit 7/7, Armor 7.
    const SHIELDED: &str = "core-056"; // (2) Jilliax 3/2: Rush, Taunt, Lifesteal, Divine Shield.
    const ARCH_MAGE: &str = "meditativeplus-008"; // (2) Unit 4/8, Spell Damage +3.

    fn board(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": THWOMP, "radiant": radiant }], "field": mine, "mana": 10, "library": filler(4) },
            "p2": { "hand": filler(1), "field": theirs, "library": filler(4) },
        }))
    }

    fn on(card: &CardInstance) -> Value {
        json!({ "targets": [{ "pick": "instance", "instanceId": card.id }] })
    }

    /// What the last step's hits dealt to `card` (`damage` events, after Armor and the shield): read
    /// off the events, since a lethal hit leaves no damage on a Unit gone to the graveyard.
    fn dealt(s: &Scenario, card: &CardInstance) -> Vec<i32> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id, amount, .. } if *target_id == card.id => Some(*amount),
                _ => None,
            })
            .collect()
    }

    /// Every selection a legal play of the Thwomp offers.
    fn offered(s: &Scenario) -> Vec<Selection> {
        let thwomp = s.card(THWOMP).id.clone();
        legal_actions(s.state(), P1)
            .into_iter()
            .flat_map(|action| match action {
                ActionBody::Play { instance_id, targets, .. } if instance_id == thwomp => targets.unwrap_or_default(),
                _ => Vec::new(),
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn it_offers_every_unit_on_both_sides_and_no_hero() {
            let s = board(false, json!([GREASER]), json!([GREASER]));
            let (mine, theirs) = (s.unit_at(P1, 1), s.unit_at(P2, 1));
            let offered = offered(&s);
            for unit in [&mine, &theirs] {
                assert!(offered.contains(&Selection::Instance {
                    instance_id: unit.id.clone()
                }));
            }
            assert!(!offered.iter().any(|pick| matches!(pick, Selection::Hero { .. })));
        }

        #[test]
        fn one_hit_of_ten_on_the_chosen_unit() {
            let mut s = board(false, json!([]), json!([BIG, GREASER]));
            let (big, other) = (s.unit_at(P2, 1), s.unit_at(P2, 2));
            s.play(THWOMP, on(&big));
            assert_eq!(dealt(&s, &big), vec![10], "one hit of 10");
            assert!(dealt(&s, &other).is_empty());
        }

        #[test]
        fn its_casters_spell_damage_raises_it() {
            let mut s = board(false, json!([ARCH_MAGE]), json!([BIG]));
            let big = s.unit_at(P2, 1);
            s.play(THWOMP, on(&big));
            assert_eq!(dealt(&s, &big), vec![13]);
        }

        #[test]
        fn armor_lowers_it() {
            let mut s = board(false, json!([]), json!([ARMORED]));
            let armored = s.unit_at(P2, 1);
            s.play(THWOMP, on(&armored));
            assert_eq!(dealt(&s, &armored), vec![3], "10 less Armor 7");
        }

        #[test]
        fn a_divine_shield_takes_it_whole_and_is_spent() {
            let mut s = board(false, json!([]), json!([SHIELDED]));
            let shielded = s.unit_at(P2, 1);
            s.play(THWOMP, on(&shielded));
            s.expect_in_zone(&shielded, "field");
            assert_eq!(s.card(&shielded).damage, 0);
            assert_eq!(s.card(&shielded).divine_shield_spent, Some(true));
        }

        #[test]
        fn damage_reads_through_param_a_buff_steps_it_two() {
            let mut s = board(false, json!([]), json!([BIG]));
            assert_eq!(upgrade_number(&mut s, THWOMP, "damage"), 12);
            let big = s.unit_at(P2, 1);
            s.play(THWOMP, on(&big));
            assert_eq!(dealt(&s, &big), vec![12]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn one_hit_of_one_hundred() {
            let mut s = board(true, json!([]), json!([BIG]));
            let big = s.unit_at(P2, 1);
            s.play(THWOMP, on(&big));
            assert_eq!(dealt(&s, &big), vec![100]);
        }
    }
}
