//! SPEC §8.12 S+ #23 GIGA-ify, a card of the Stinkyscape+ set (R1420, R3535000). Built by S08 (#3555).
//! Make a permanent in your hand **GIGA** and **Indestructible.**  
//! \~\~  
//! Make two permanents in your hand **GIGA** and **Indestructible**
//!
//! "A permanent in your hand" is a declared hand pick (R81, R3555000): a Unit, Field Spell, Trap or
//! Field Trap — never a Spell. Each pick is given GIGA and Indestructible in the hand and keeps them
//! onto the field (R1878003), where it enters GIGA (R3546002). The Radiant face picks two; fewer
//! resolve on what there is (R90).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-023";

/// The base face marks one permanent; the Radiant face two.
const BASE_PICKS: i32 = 1;
/// The Radiant face marks two permanents.
const RADIANT_PICKS: i32 = 2;

fn giga_ify(picks: i32) -> Script {
    Script {
        targets: vec![TargetDecl::hand(
            picks,
            picks,
            json!({ "of": ["hand"], "type": ["Unit", "Field Spell", "Trap", "Field Trap"] }),
        )],
        cry: Some(hook(move |_ctx| {
            (0..picks)
                .flat_map(|index| {
                    vec![
                        grant_keyword(json_as(json!({
                            "target": { "of": "chosen", "index": index },
                            "keyword": { "kind": "GIGA" },
                        }))),
                        grant_keyword(json_as(json!({
                            "target": { "of": "chosen", "index": index },
                            "keyword": { "kind": "Indestructible" },
                        }))),
                    ]
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: giga_ify(BASE_PICKS),
        radiant: giga_ify(RADIANT_PICKS),
    }
}

// S+ #23 GIGA-ify — SPEC §8.12 row S+ 23 (R81, R90, R1878003, R3546002, R3555000).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;
    use jackioh_engine::zones::{ZoneSlot, is_empty};

    use crate::weapon_board;

    const P1: PlayerId = PlayerId::P1;

    const UNIT: &str = "stand-in (1) 4/4 R12/12";
    const TRAP: &str = "stand-in (1) Trap";
    const SPELL: &str = "stand-in (0) Spell";

    /// p1 holds GIGA-ify (radiant or not) with the named other cards, mana to cast it.
    fn holding(radiant: bool, seed: &str, others: Value) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend(others.as_array().cloned().unwrap_or_default());
        weapon_board(seed, json!({ "hand": hand, "mana": 10 }), json!({}))
    }

    /// The instance id of p1's held card with this def id.
    fn held_id(s: &Scenario, def: &str) -> String {
        s.hand(P1).iter().find(|card| card.def_id == def).unwrap().id.clone()
    }

    fn keyword_kinds(s: &Scenario, id: &str) -> Vec<String> {
        s.card(id)
            .granted_keywords
            .iter()
            .map(|keyword| keyword.kind().to_string())
            .collect()
    }

    fn play_with(s: &mut Scenario, picks: Vec<String>) {
        let targets: Vec<Value> = picks
            .iter()
            .map(|id| json!({ "pick": "instance", "instanceId": id }))
            .collect();
        s.play(ID, json!({ "targets": targets }));
    }

    mod base {
        use super::*;

        #[test]
        fn r3555000_the_chosen_permanent_is_giga_and_indestructible_and_plays_into_four_zones() {
            let mut s = holding(false, "s23-giga", json!([UNIT, SPELL]));
            let unit = held_id(&s, UNIT);
            play_with(&mut s, vec![unit.clone()]);
            assert!(s.state().pending.is_none());
            for keyword in ["GIGA", "Indestructible"] {
                assert!(keyword_kinds(&s, &unit).contains(&keyword.to_string()), "hand has {keyword}");
            }
            // Kept onto the field (R1878003), where it enters GIGA (R3546002).
            s.play(unit.as_str(), json!({ "zone": 1 }));
            assert_eq!(s.card(unit.as_str()).giga, Some(true));
            for (row, lane) in [(Row::Units, 1), (Row::Units, 2), (Row::Backrow, 1), (Row::Backrow, 2)] {
                assert!(
                    !is_empty(s.state(), ZoneSlot { player: P1, row, lane }),
                    "{row} {lane} is filled"
                );
            }
            s.expect_in_zone(ID, "graveyard");
        }

        #[test]
        fn r3555000_only_permanents_are_offered() {
            let s = holding(false, "s23-offered", json!([UNIT, TRAP, SPELL]));
            let unit = held_id(&s, UNIT);
            let trap = held_id(&s, TRAP);
            let spell = held_id(&s, SPELL);
            let giga = held_id(&s, ID);
            let offered: Vec<String> = legal_actions(s.state(), P1)
                .into_iter()
                .filter_map(|action| match action {
                    ActionBody::Play {
                        instance_id, targets, ..
                    } if instance_id == giga => targets,
                    _ => None,
                })
                .flatten()
                .filter_map(|target| match target {
                    Selection::Instance { instance_id } => Some(instance_id),
                    _ => None,
                })
                .collect();
            assert!(offered.contains(&unit), "the Unit is offered");
            assert!(offered.contains(&trap), "the Trap is offered");
            assert!(!offered.contains(&spell), "the Spell is never offered");
        }

        #[test]
        fn r90_with_no_permanent_it_still_resolves() {
            let mut s = holding(false, "s23-empty", json!([SPELL]));
            s.play(ID, json!({}));
            assert!(s.state().pending.is_none());
            s.expect_in_zone(ID, "graveyard");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3555000_the_radiant_face_marks_two() {
            let mut s = holding(true, "s23-radiant", json!([UNIT, TRAP, SPELL]));
            let unit = held_id(&s, UNIT);
            let trap = held_id(&s, TRAP);
            play_with(&mut s, vec![unit.clone(), trap.clone()]);
            assert!(s.state().pending.is_none());
            for id in [&unit, &trap] {
                for keyword in ["GIGA", "Indestructible"] {
                    assert!(keyword_kinds(&s, id).contains(&keyword.to_string()));
                }
            }
            let spell = held_id(&s, SPELL);
            assert!(!keyword_kinds(&s, &spell).contains(&"GIGA".to_string()));
            s.expect_in_zone(ID, "graveyard");
        }
    }
}
