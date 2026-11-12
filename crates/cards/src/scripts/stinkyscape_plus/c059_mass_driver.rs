//! SPEC §8.12 S+ #59 Mass Driver (R3556007; R62, R1181000, R1420, R1840002, R3543001, R3549001):
//! (2) Field Spell, Mindustrial, Epic, a card of the Stinkyscape+ set.
//!   Base:    "Flexible
//!            0/6
//!            Activate: In {count|turn|turns} add two Minerals that cost (0) and a Resource to your hand."
//!   Radiant: "Flexible
//!            0/12
//!            Activate: In {count|turn|turns} add two Radiant Minerals that cost (0) and a Radiant
//!            Resource to your hand."
//!
//! Flexible with stats 0/6 (0/12 Radiant): played into a unit zone it is a 0/6 Unit, and its text
//! keeps working (R3543001). The Activate queues an unwatched `delay` (R1181000): at the start of
//! its controller's Nth own turn it delivers two random S+ Minerals at (0)
//! (`add_random_from_catalog`, as S+ #53 Laser Drill's) and a Resource (`gain_resources`,
//! R1840002) — Radiant ones on the Radiant face (R3549001). It lands if the Driver is gone
//! (R3556007); a full hand burns what does not fit (§2.4).

use jackioh_engine::prelude::*;
use jackioh_engine::prompts::RESUME_HOOK;

pub const ID: &str = "stinkyscapeplus-059";

/// "two Minerals".
const MINERALS: i32 = 2;
/// "a Resource".
const RESOURCES: i32 = 1;
/// "Minerals that cost (0)".
const MINERAL_COST: i32 = 0;
/// The delivery's resume step.
const DELIVER: &str = "deliver";
/// The Activate's id.
const DRIVE: &str = "drive";

/// R3549001: "a random Mineral": the held set's, tagged Mineral (as S+ #53's).
fn minerals() -> CatalogQueryArgs {
    json_as(json!({ "set": SetName::StinkyscapePlus, "tags": [Tag::Mineral] }))
}

fn mass_driver(radiant: bool) -> Script {
    let label = if radiant {
        "In 2 turns: add two Radiant Minerals at (0) and a Radiant Resource"
    } else {
        "In 2 turns: add two Minerals at (0) and a Resource"
    };
    Script {
        activations: vec![ActivationDecl {
            id: DRIVE.to_string(),
            label: label.to_string(),
            uses: ActivationUses::Count(1),
            cost: None,
            targets: vec![],
            modes: vec![],
            can_activate: None,
            has: None,
            run: hook(move |ctx| {
                let name = def_of(Some(&*ctx.state), ID).name.clone();
                let turns = param(&*ctx, "count");
                vec![delay(json_as(json!({
                    "at": { "phase": "start", "player": "self" },
                    "turns": turns,
                    "step": DELIVER,
                    "hook": RESUME_HOOK,
                    "label": format!("{name}: two Minerals and a Resource in {turns} turns"),
                })))]
            }),
        }],
        resume: IndexMap::from([(
            DELIVER,
            hook(move |_ctx| {
                vec![
                    add_random_from_catalog(json_as(json!({
                        "query": minerals(),
                        "count": MINERALS,
                        "costOverride": MINERAL_COST,
                        "radiant": radiant,
                    }))),
                    gain_resources(json_as(json!({ "count": RESOURCES, "radiant": radiant }))),
                ]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: mass_driver(false),
        radiant: mass_driver(true),
    }
}

// S+ #59 Mass Driver — SPEC §8.12 row S+ 59 (R3556007; R62, R1181000, R1840002, R3543001, R3549001).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, next_turn_of, weapon_board};

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-005";

    /// p1 holds the Driver; `"mana": 10` plays and activates it from hand.
    fn driver(radiant: bool, seed: &str) -> Scenario {
        let mut s = weapon_board(
            seed,
            json!({
                "mana": 10,
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "library": filler(6),
            }),
            json!({
                "hand": [FILLER],
                "library": filler(6),
            }),
        );
        s.play(ID, json!({ "zone": 1 }));
        s.activate(ID, json!({ "ability": DRIVE }));
        s
    }

    /// What the delivery added: Minerals and Resources gained since.
    fn delivered(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1)
            .into_iter()
            .filter(|card| {
                crate::card_def(&card.def_id).tags.contains(&Tag::Mineral)
                    || crate::resources_in_hand(s, P1).iter().any(|held| held.id == card.id)
            })
            .collect()
    }

    fn is_mineral(card: &CardInstance) -> bool {
        crate::card_def(&card.def_id).tags.contains(&Tag::Mineral)
    }

    mod base {
        use super::*;

        #[test]
        fn r3556007_the_activate_delivers_two_free_minerals_and_a_resource_in_two_turns() {
            let mut s = driver(false, "s59-deliver");
            // Not at the first own start: "in 2 turns" (R1181000).
            next_turn_of(&mut s, P1);
            assert!(delivered(&s).is_empty(), "nothing at the first own start");
            next_turn_of(&mut s, P1);
            let cards = delivered(&s);
            let minerals: Vec<&CardInstance> = cards.iter().filter(|card| is_mineral(card)).collect();
            let resources = cards.len() - minerals.len();
            assert_eq!(minerals.len(), MINERALS as usize, "two Minerals");
            assert_eq!(resources, RESOURCES as usize, "and a Resource");
            for mineral in minerals {
                assert!(!mineral.radiant, "no Radiant face's Radiant card");
                assert_eq!(effective_cost(s.state(), mineral, Default::default()), 0, "at (0)");
            }
        }

        #[test]
        fn r3556007_the_delivery_lands_after_the_driver_is_destroyed() {
            let mut s = driver(false, "s59-gone");
            let id = s.card(ID).id.clone();
            crate::run_as(
                &mut s,
                jackioh_engine::effects::destroy(json_as(json!({
                    "target": { "of": "instance", "instanceId": id },
                }))),
                P1,
            );
            // The mark collects at the turn's state checks; the unwatched delay stands regardless.
            next_turn_of(&mut s, P1);
            assert!(s.card(ID).zone.z() != ZoneName::Field, "the Driver is gone");
            assert!(delivered(&s).is_empty(), "nothing at the first own start");
            next_turn_of(&mut s, P1);
            assert_eq!(delivered(&s).len(), (MINERALS + RESOURCES) as usize, "still delivered");
        }

        #[test]
        fn r3556007_played_into_a_unit_zone_it_is_a_0_6_unit() {
            let mut s = weapon_board(
                "s59-unit",
                json!({
                    "mana": 10,
                    "hand": [ID, FILLER],
                    "library": filler(6),
                }),
                json!({
                    "hand": [FILLER],
                    "library": filler(6),
                }),
            );
            s.play(ID, json!({ "zone": 1, "row": "units" }));
            let played = s.card(ID);
            assert!(
                matches!(played.zone, Zone::Field { row: Row::Units, .. }),
                "it lands in the unit zone"
            );
            assert_eq!(
                jackioh_engine::faces::card_type_of(s.state(), played),
                CardType::Unit,
                "a Unit there"
            );
            s.expect_stats(ID, json!({ "health": 6, "maxHealth": 6 }));
            // Its text keeps working there: the delivery lands.
            s.activate(ID, json!({ "ability": DRIVE }));
            next_turn_of(&mut s, P1);
            next_turn_of(&mut s, P1);
            assert_eq!(delivered(&s).len(), (MINERALS + RESOURCES) as usize);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3556007_radiant_all_radiant_and_0_12() {
            let mut s = driver(true, "s59-radiant");
            s.expect_stats(ID, json!({ "health": 12, "maxHealth": 12 }));
            next_turn_of(&mut s, P1);
            next_turn_of(&mut s, P1);
            let cards = delivered(&s);
            assert_eq!(cards.len(), (MINERALS + RESOURCES) as usize);
            for card in &cards {
                assert!(card.radiant, "Radiant {}", card.def_id);
            }
            let minerals: Vec<&CardInstance> = cards.iter().filter(|card| is_mineral(card)).collect();
            assert_eq!(minerals.len(), MINERALS as usize);
            for mineral in minerals {
                assert_eq!(effective_cost(s.state(), mineral, Default::default()), 0, "at (0)");
            }
        }
    }
}
