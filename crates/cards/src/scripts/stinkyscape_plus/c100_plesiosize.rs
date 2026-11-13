//! SPEC §8.12 S+ #100 Plesiosize, a card of the Stinkyscape+ set (R1420, R3535000). Built by S08 (#3555).
//! Give a unit in your hand \+6/+6 and make it GIGA.  
//! \~\~\~  
//! Give a unit in your hand \+12/+12 and make it GIGA.
//!
//! "A Unit in your hand" is a declared hand pick (R81, R3555000). In the text's order it gains +N/+N
//! (`buff`, carried onto the field) and GIGA (`grant_keyword`), kept onto the field (R1878003),
//! where it enters GIGA (R3546002). The base face reads `buff`, the Radiant face `buff2`.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-100";

fn plesiosize(key: &'static str) -> Script {
    Script {
        targets: vec![TargetDecl::hand(1, 1, json!({ "of": ["hand"], "type": "Unit" }))],
        cry: Some(hook(move |ctx| {
            let n = param(&*ctx, key);
            vec![
                buff(json_as(json!({
                    "target": { "of": "chosen" },
                    "attack": n,
                    "health": n,
                }))),
                grant_keyword(json_as(json!({
                    "target": { "of": "chosen" },
                    "keyword": { "kind": "GIGA" },
                }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: plesiosize("buff"),
        radiant: plesiosize("buff2"),
    }
}

// S+ #100 Plesiosize — SPEC §8.12 row S+ 100 (R81, R1878003, R3546002, R3555000).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::weapon_board;

    const P1: PlayerId = PlayerId::P1;

    const UNIT: &str = "stand-in (1) 4/4 R12/12";
    const SPELL: &str = "stand-in (0) Spell";

    /// p1 holds Plesiosize (radiant or not) with the named other cards, mana to cast it.
    fn holding(radiant: bool, seed: &str, others: Value) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend(others.as_array().cloned().unwrap_or_default());
        weapon_board(seed, json!({ "hand": hand, "mana": 10 }), json!({}))
    }

    /// The instance id of p1's held card with this def id.
    fn held_id(s: &Scenario, def: &str) -> String {
        s.hand(P1).iter().find(|card| card.def_id == def).unwrap().id.clone()
    }

    mod base {
        use super::*;

        #[test]
        fn r3555000_the_chosen_unit_gains_6_6_and_giga_onto_the_field() {
            let mut s = holding(false, "s100-base", json!([UNIT]));
            let unit = held_id(&s, UNIT);
            s.play(ID, json!({ "targets": [{ "pick": "instance", "instanceId": unit }] }));
            assert!(s.state().pending.is_none());
            assert_eq!(s.card(unit.as_str()).buffs.attack, 6);
            assert_eq!(s.card(unit.as_str()).buffs.health, 6);
            assert!(
                s.card(unit.as_str())
                    .granted_keywords
                    .iter()
                    .any(|keyword| keyword.kind() == KeywordKind::Giga)
            );
            s.play(unit.as_str(), json!({ "zone": 1 }));
            assert_eq!(s.card(unit.as_str()).giga, Some(true));
            s.expect_stats(unit.as_str(), json!({ "attack": 10, "maxHealth": 10 }));
        }

        #[test]
        fn r3555000_only_units_are_offered() {
            let s = holding(false, "s100-offered", json!([UNIT, SPELL]));
            let plesio = held_id(&s, ID);
            let unit = held_id(&s, UNIT);
            let spell = held_id(&s, SPELL);
            let offered: Vec<String> = legal_actions(s.state(), P1)
                .into_iter()
                .filter_map(|action| match action {
                    ActionBody::Play {
                        instance_id, targets, ..
                    } if instance_id == plesio => targets,
                    _ => None,
                })
                .flatten()
                .filter_map(|target| match target {
                    Selection::Instance { instance_id } => Some(instance_id),
                    _ => None,
                })
                .collect();
            assert!(offered.contains(&unit), "the Unit is offered");
            assert!(!offered.contains(&spell), "the Spell is never offered");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3555000_the_chosen_unit_gains_12_12_and_giga_onto_the_field() {
            let mut s = holding(true, "s100-radiant", json!([UNIT]));
            let unit = held_id(&s, UNIT);
            s.play(ID, json!({ "targets": [{ "pick": "instance", "instanceId": unit }] }));
            assert!(s.state().pending.is_none());
            assert_eq!(s.card(unit.as_str()).buffs.attack, 12);
            assert_eq!(s.card(unit.as_str()).buffs.health, 12);
            assert!(
                s.card(unit.as_str())
                    .granted_keywords
                    .iter()
                    .any(|keyword| keyword.kind() == KeywordKind::Giga)
            );
            s.play(unit.as_str(), json!({ "zone": 1 }));
            assert_eq!(s.card(unit.as_str()).giga, Some(true));
            s.expect_stats(unit.as_str(), json!({ "attack": 16, "maxHealth": 16 }));
        }
    }
}
