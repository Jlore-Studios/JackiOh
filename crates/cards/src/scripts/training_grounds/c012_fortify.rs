//! TG #12 Fortify (SPEC §8.11 row TG 12; §6.1 Armor X, Taunt, Divine Shield, R78, R90, R347): (1)
//! Spell, Common, a card of the held Training Grounds set (R1420, R2194000).
//!   Base:    "Give a Unit +{health} Health, +{armor} Armor and Taunt." — 3, 1.
//!   Radiant: "… +{health} Health, +{armor} Armor, Taunt and Divine Shield." — 6, 2.
//! Targets any Unit (R90; S #10.3); grants Armor n (§6.1, R78) and Taunt (R347 Indestructible ignores; S #15). Radiant grants Divine Shield.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-012";

fn fortify(radiant: bool) -> Script {
    Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))],
        cry: Some(hook(move |ctx| {
            let mut effects = vec![
                buff(json_as(json!({ "target": { "of": "chosen" }, "health": param(&*ctx, "health") }))),
                grant_keyword(json_as(json!({
                    "target": { "of": "chosen" },
                    "keyword": { "kind": "Armor", "n": param(&*ctx, "armor") },
                }))),
                grant_keyword(json_as(json!({ "target": { "of": "chosen" }, "keyword": { "kind": "Taunt" } }))),
            ];
            if radiant {
                effects.push(grant_keyword(json_as(json!({
                    "target": { "of": "chosen" },
                    "keyword": { "kind": "Divine Shield" },
                }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: fortify(false),
        radiant: fortify(true),
    }
}

// TG #12 Fortify — SPEC §8.11 row TG 12, BUILD M13 row TG 12.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario, upgrade_number};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FORTIFY: &str = "traininggrounds-012";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const MOON: &str = "traininggrounds-007"; // Chosen by the Moon: Taunt, Armor 3 (once P3 builds it).
    const ARMORED: &str = "core-025"; // (4) Unit 7/7, Armor 7.
    const MENACE: &str = "core-019"; // (3) Unit 9/9, Taunt.
    const ROCK: &str = "core-066"; // (4) Unit 10/10, Indestructible.
    const SHIELDED: &str = "core-056"; // (2) Jilliax 3/2: Rush, Taunt, Lifesteal, Divine Shield.

    fn board(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": FORTIFY, "radiant": radiant }], "field": mine, "mana": 10, "library": filler(4) },
            "p2": { "hand": filler(1), "field": theirs, "library": filler(4) },
        }))
    }

    fn on(card: &CardInstance) -> Value {
        json!({ "targets": [{ "pick": "instance", "instanceId": card.id }] })
    }

    fn armor(s: &Scenario, card: &CardInstance) -> i32 {
        s.stats(card).armor
    }

    fn keyword_count(s: &Scenario, card: &CardInstance, kind: KeywordKind) -> usize {
        s.stats(card).keywords.iter().filter(|keyword| keyword.kind() == kind).count()
    }

    fn offered(s: &Scenario) -> Vec<Selection> {
        let fortify = s.card(FORTIFY).id.clone();
        legal_actions(s.state(), P1)
            .into_iter()
            .flat_map(|action| match action {
                ActionBody::Play { instance_id, targets, .. } if instance_id == fortify => targets.unwrap_or_default(),
                _ => Vec::new(),
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn it_offers_any_unit_on_either_side_and_no_hero() {
            let s = board(false, json!([VANILLA]), json!([VANILLA]));
            let offered = offered(&s);
            for unit in [s.unit_at(P1, 1), s.unit_at(P2, 1)] {
                assert!(offered.contains(&Selection::Instance { instance_id: unit.id }));
            }
            assert!(!offered.iter().any(|pick| matches!(pick, Selection::Hero { .. })));
        }

        #[test]
        fn the_unit_gains_three_health_current_and_max() {
            let mut s = board(false, json!([{ "def": VANILLA, "lane": 1, "damage": 1 }]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(FORTIFY, on(&unit));
            s.expect_stats(&unit, json!({ "attack": 4, "health": 6 }));
            assert_eq!(s.stats(&unit).max_health, 7);
        }

        #[test]
        fn its_armor_rises_by_one_on_top_of_its_printed_armor() {
            let mut s = board(false, json!([ARMORED, VANILLA]), json!([]));
            let (armored, vanilla) = (s.unit_at(P1, 1), s.unit_at(P1, 2));
            s.play(FORTIFY, on(&armored));
            assert_eq!(armor(&s, &armored), 8);
            assert_eq!(armor(&s, &vanilla), 0);
        }

        #[test]
        fn it_gains_taunt() {
            let mut s = board(false, json!([VANILLA]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(FORTIFY, on(&unit));
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Taunt), 1);
        }

        #[test]
        fn taunt_once_on_a_unit_that_has_it() {
            let mut s = board(false, json!([MENACE]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(FORTIFY, on(&unit));
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Taunt), 1);
        }

        #[test]
        fn never_taunt_on_an_indestructible_unit() {
            let mut s = board(false, json!([ROCK]), json!([]));
            let rock = s.unit_at(P1, 1);
            s.play(FORTIFY, on(&rock));
            assert_eq!(keyword_count(&s, &rock, KeywordKind::Taunt), 0, "R347");
            assert_eq!(armor(&s, &rock), 1);
        }

        #[test]
        fn an_enemy_unit_may_be_fortified() {
            let mut s = board(false, json!([]), json!([VANILLA]));
            let theirs = s.unit_at(P2, 1);
            s.play(FORTIFY, on(&theirs));
            assert_eq!(armor(&s, &theirs), 1);
            assert_eq!(s.stats(&theirs).max_health, 7);
        }

        #[test]
        fn both_numbers_read_through_param() {
            let mut s = board(false, json!([VANILLA]), json!([]));
            assert_eq!(upgrade_number(&mut s, FORTIFY, "health"), 4);
            assert_eq!(upgrade_number(&mut s, FORTIFY, "armor"), 2);
            let unit = s.unit_at(P1, 1);
            s.play(FORTIFY, on(&unit));
            assert_eq!(s.stats(&unit).max_health, 8);
            assert_eq!(armor(&s, &unit), 2);
        }

        #[test]
        fn no_divine_shield_on_the_base_face() {
            let mut s = board(false, json!([VANILLA]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(FORTIFY, on(&unit));
            assert_eq!(keyword_count(&s, &unit, KeywordKind::DivineShield), 0);
        }

        #[test]
        fn a_moon_s_printed_armor_takes_the_one_on_top() {
            // Chosen by the Moon prints Armor 3 (§8.11 row TG 7); read off the catalog, so this holds
            // whether or not P3 has written its script yet.
            let mut s = board(false, json!([MOON]), json!([]));
            let moon = s.unit_at(P1, 1);
            let printed = armor(&s, &moon);
            s.play(FORTIFY, on(&moon));
            assert_eq!(armor(&s, &moon), printed + 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn six_health_two_armor_and_taunt() {
            let mut s = board(true, json!([VANILLA]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(FORTIFY, on(&unit));
            assert_eq!(s.stats(&unit).max_health, 10);
            assert_eq!(armor(&s, &unit), 2);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Taunt), 1);
        }

        #[test]
        fn divine_shield_a_spent_one_coming_back() {
            let mut s = board(true, json!([SHIELDED]), json!([]));
            let shielded = s.unit_at(P1, 1);
            s.card_mut(&shielded).divine_shield_spent = Some(true);
            s.play(FORTIFY, on(&shielded));
            assert_ne!(s.card(&shielded).divine_shield_spent, Some(true));
            assert!(s.stats(&shielded).keywords.iter().any(|k| k.kind() == KeywordKind::DivineShield));
        }

        #[test]
        fn divine_shield_on_a_unit_without_one() {
            let mut s = board(true, json!([VANILLA]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(FORTIFY, on(&unit));
            assert_eq!(keyword_count(&s, &unit, KeywordKind::DivineShield), 1);
        }
    }
}
