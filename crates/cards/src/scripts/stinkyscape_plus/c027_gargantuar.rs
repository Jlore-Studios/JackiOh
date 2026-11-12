//! SPEC §8.12 S+ #27 Gargantuar (Unit, Swarm 12/12 → 24/24, cost 3, Rare), a card of the held Stinkyscape+ set (R1420, R3535000).
//!   Base:    "After this attacks, Freeze it.
//!            Evolves when this falls below half health.
//!            Evolve: Summon an Imp."
//!   Radiant: "Evolves when this falls below half health.
//!            Evolve: Summon a Radiant Imp."
//!
//! "After this attacks" is the `after_attack` hook (R1849001, R2285001): once the state check closing
//! the combat has run, a Gargantuar still on the field freezes itself (`freeze`, R1174000); the Radiant
//! face prints no such line. "Falls below half health" is a standing condition, read at the settled
//! state check (`evolves_when`, R3536001): its current Health under half its maximum, so a hit that
//! kills it leaves a dead card that evolves nothing. The Evolve text runs once (R3536000): a `summon`
//! of S+ #27.1 Imp, Radiant on the Radiant face, into the leftmost open unit zone (R64), no Cry (R1);
//! a full unit row summons nothing and it has evolved all the same.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-027";

/// S+ #27.1 Imp.
const IMP: &str = "stinkyscapeplus-027-1"; // Imp

/// R3536001: its current Health is under half its maximum (the Health the layers give it now).
fn below_half() -> HasHook {
    read_hook(|a| {
        let view = unit_view(a.state, a.self_);
        view.health * 2 < view.max_health
    })
}

/// "Evolve: Summon an Imp" (a Radiant Imp on the Radiant face).
fn summon_imp(radiant: bool) -> Hook {
    hook(move |_ctx| {
        let args = if radiant {
            json!({ "defId": IMP, "radiant": true })
        } else {
            json!({ "defId": IMP })
        };
        vec![summon(json_as(args))]
    })
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            after_attack: Some(hook(|_ctx| {
                vec![freeze(json_as(json!({ "target": { "of": "self" } })))]
            })),
            evolves_when: Some(below_half()),
            evolve: Some(summon_imp(false)),
            ..Script::default()
        },
        radiant: Script {
            evolves_when: Some(below_half()),
            evolve: Some(summon_imp(true)),
            ..Script::default()
        },
    }
}

// S+ #27 Gargantuar — SPEC §8.12 row 27.
#[cfg(test)]
mod tests {
    use super::{ID, IMP};
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const WALL: &str = "stand-in (4) 7/7";
    const BIG_WALL: &str = "stand-in (4) 14/14";

    fn board(radiant: bool, p1_field: Vec<Value>, p2_field: Vec<Value>) -> Scenario {
        let mut field = vec![json!({ "def": ID, "radiant": radiant })];
        field.extend(p1_field);
        scenario(json!({
            "p1": { "hand": [VANILLA], "field": field, "library": crate::filler(4) },
            "p2": { "hand": [VANILLA], "field": p2_field, "library": crate::filler(4) },
        }))
    }

    fn imps(s: &Scenario) -> Vec<CardInstance> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned {
                    instance_id, def_id, ..
                } if def_id == IMP => Some(s.card(instance_id).clone()),
                _ => None,
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn after_attacking_it_is_frozen() {
            let mut s = board(false, vec![], vec![json!(WALL)]);
            let gargantuar = s.unit_at(P1, 1);
            assert!(s.card(&gargantuar).frozen.is_none());
            s.attack(&gargantuar, s.unit_at(P2, 1));
            assert!(
                s.card(&gargantuar).frozen.is_some(),
                "After this attacks, Freeze it"
            );
        }

        #[test]
        fn falling_below_half_in_combat_summons_one_imp_in_the_leftmost_open_zone() {
            let mut s = board(false, vec![], vec![json!(WALL)]);
            let gargantuar = s.unit_at(P1, 1);
            s.attack(&gargantuar, s.unit_at(P2, 1));
            // 12 health less the wall's 7 is 5: below half.
            assert_eq!(s.card(&gargantuar).damage, 7);
            assert_eq!(s.card(&gargantuar).evolved, Some(true));
            let imps = imps(&s);
            assert_eq!(imps.len(), 1);
            assert_eq!(s.unit_at(P1, 2).id, imps[0].id, "R64: the leftmost open zone");
            assert!(!imps[0].radiant);
            s.expect_stats(&imps[0], json!({ "attack": 1, "health": 1 }));
            assert!(s.state().pending.is_none(), "it opens no prompt");
        }

        #[test]
        fn at_half_health_or_more_it_evolves_nothing() {
            let mut s = board(false, vec![], vec![json!("stand-in (1) 6/6")]);
            let gargantuar = s.unit_at(P1, 1);
            s.attack(&gargantuar, s.unit_at(P2, 1));
            // 12 health less 6 is exactly half: not below it.
            assert_eq!(s.card(&gargantuar).damage, 6);
            assert_ne!(s.card(&gargantuar).evolved, Some(true));
            assert!(imps(&s).is_empty());
        }

        #[test]
        fn a_full_unit_row_summons_nothing_but_it_still_evolves() {
            let mut s = board(
                false,
                vec![json!(VANILLA), json!(VANILLA), json!(VANILLA), json!(VANILLA)],
                vec![json!(WALL)],
            );
            let gargantuar = s.unit_at(P1, 1);
            s.attack(&gargantuar, s.unit_at(P2, 1));
            assert_eq!(s.card(&gargantuar).evolved, Some(true));
            assert!(imps(&s).is_empty());
        }

        #[test]
        fn a_lethal_hit_evolves_nothing() {
            let mut s = board(false, vec![], vec![json!("stand-in (4) 12/12")]);
            let gargantuar = s.unit_at(P1, 1);
            s.attack(&gargantuar, s.unit_at(P2, 1));
            s.expect_in_zone(&gargantuar, "graveyard");
            assert!(imps(&s).is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn its_attack_freezes_nothing() {
            let mut s = board(true, vec![], vec![json!(WALL)]);
            let gargantuar = s.unit_at(P1, 1);
            s.expect_stats(&gargantuar, json!({ "attack": 24, "health": 24 }));
            s.attack(&gargantuar, s.unit_at(P2, 1));
            assert!(
                s.card(&gargantuar).frozen.is_none(),
                "the Radiant face prints no Freeze"
            );
            // 24 health less 7 is 17: not below half, so nothing evolves.
            assert_ne!(s.card(&gargantuar).evolved, Some(true));
            assert!(imps(&s).is_empty());
        }

        #[test]
        fn falling_below_half_summons_a_radiant_imp() {
            let mut s = board(true, vec![], vec![json!(BIG_WALL)]);
            let gargantuar = s.unit_at(P1, 1);
            s.attack(&gargantuar, s.unit_at(P2, 1));
            // 24 health less 14 is 10: below half of 24.
            assert_eq!(s.card(&gargantuar).evolved, Some(true));
            let imps = imps(&s);
            assert_eq!(imps.len(), 1);
            assert!(imps[0].radiant);
            assert_eq!(s.unit_at(P1, 2).id, imps[0].id);
            s.expect_stats(&imps[0], json!({ "attack": 2, "health": 2 }));
            assert!(s.state().pending.is_none());
        }
    }
}
