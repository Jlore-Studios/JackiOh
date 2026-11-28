//! FvH #50 Field Kit (SPEC §8.14 row 50): (2) Field Spell, Legendary.
//!   Base:    "Durability 2\nActivate: Deal {damage} damage to an enemy. Summon a random (1) or (2) Cost
//!            Trap." (2)
//!   Radiant: "Pierce, Durability 2\nActivate: Deal {damage} damage to an enemy. Summon a random Radiant (1)
//!            or (2) Cost Trap." (4)
//!
//! Engine: Activate, once a turn (R384), FV2 #50's reading: one declared enemy, Unit or hero (R81),
//! `damage` of the declared `damage`; then, whatever the damage found, `summon_random` of a non-token Trap
//! or Field Trap of (1) or (2) Cost, face-down into its controller's leftmost open backrow zone (R64); no
//! open backrow zone, nothing (R129). Radiant: its printed Pierce read off the source (R346) and a Radiant
//! Trap. Durability 2 is the engine's keyword (R5236000, R5236002). `dealsDamage` (R1748001). Its (1) and
//! (2) are a pool's prices and its 2 a keyword count, fixed (R1748003). Tunes: damage 2 ↑ (Radiant 4).

use crate::query::TRAP_TYPES;
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-050";

/// "Activate" is once a turn (R384).
const USES: i32 = 1;
/// "A random (1) or (2) Cost Trap": the pool's prices.
const CHEAPEST_TRAP: i32 = 1;
const DEAREST_TRAP: i32 = 2;

fn face(radiant: bool) -> Script {
    let kit = ActivationDecl {
        id: "kit".to_string(),
        label: if radiant {
            "Deal damage to an enemy, then summon a random Radiant (1) or (2) Cost Trap".to_string()
        } else {
            "Deal damage to an enemy, then summon a random (1) or (2) Cost Trap".to_string()
        },
        uses: ActivationUses::Count(USES),
        cost: None,
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit", "hero"] }))],
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(move |ctx| {
            vec![
                damage(json_as(json!({ "to": { "of": "chosen" }, "amount": param(&*ctx, "damage") }))),
                summon_random(json_as(json!({
                    "query": {
                        "type": TRAP_TYPES,
                        "costRange": { "min": CHEAPEST_TRAP, "max": DEAREST_TRAP },
                    },
                    "radiant": radiant,
                }))),
            ]
        }),
    };
    Script {
        activations: vec![kit],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// FvH #50 Field Kit — SPEC §8.14 row 50 (R384, R81, R64, R129, R346, R5236000, R5236002).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, run_as, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A textless Spell each seat can always play, so no turn ends by itself (§2.5).
    const SPELL: &str = "stand-in (1) Spell";
    const FIELD_SPELL: &str = "stand-in (1) Field Spell";
    const ARMORED: &str = "stand-in (4) 7/7 Armor 7";
    /// The heroes' health before any hit (§2.1).
    const HERO: i32 = 30;
    /// Its printed `damage`, base and Radiant.
    const DAMAGE: i32 = 2;
    const RADIANT_DAMAGE: i32 = 4;
    /// Its printed Durability.
    const DURABILITY: i32 = 2;
    /// How many seeds the random Trap is drawn under.
    const SEEDS: usize = 6;

    /// p1 holds the Kit in backrow lane 1 and `others` more Field Spells after it; p2 an armored 7/7.
    fn kit(seed: &str, radiant: bool, others: usize) -> Scenario {
        let mut backrow = vec![json!({ "def": ID, "radiant": radiant, "faceUp": true })];
        backrow.extend(vec![json!(FIELD_SPELL); others]);
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [SPELL], "backrow": backrow, "library": filler(6) },
            "p2": { "hand": [SPELL], "field": [ARMORED], "library": filler(6) },
        }))
    }

    /// The cards the last step summoned into p1's backrow: (instance id, def id, lane).
    fn traps(s: &Scenario) -> Vec<(String, String, i32)> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { player, instance_id, def_id, row, lane, .. }
                    if *player == P1 && *row == Row::Backrow =>
                {
                    Some((instance_id.clone(), def_id.clone(), *lane))
                }
                _ => None,
            })
            .collect()
    }

    /// The Trap one use summoned: one card, a (1) or (2) Cost Trap or Field Trap, into lane 2, not shown.
    fn one_cheap_trap(s: &Scenario, seed: &str) -> String {
        let summoned = traps(s);
        assert_eq!(summoned.len(), 1, "{seed}: {summoned:?}");
        let (instance_id, def_id, lane) = &summoned[0];
        let def = crate::card_def(def_id);
        assert!(TRAP_TYPES.contains(&def.type_), "{seed}: {def_id}");
        assert!((CHEAPEST_TRAP..=DEAREST_TRAP).contains(&query_cost(&def)), "{seed}: {def_id}");
        assert!(!def.token, "{seed}: {def_id}");
        assert_eq!(*lane, 2, "{seed}");
        assert_ne!(s.card(instance_id.as_str()).face_up, Some(true), "{seed}: set face-down");
        instance_id.clone()
    }

    mod base {
        use super::*;

        #[test]
        fn r81_r64_it_deals_2_to_the_chosen_enemy_then_summons_a_random_1_or_2_cost_trap() {
            for n in 0..SEEDS {
                let seed = format!("fvh-050-base-{n}");
                let mut s = kit(&seed, false, 0);
                s.activate(ID, json!({ "targets": pick::hero(P2) }));
                s.expect_health(P2, HERO - DAMAGE);
                let trap = one_cheap_trap(&s, &seed);
                assert!(!s.card(trap.as_str()).radiant, "{seed}");
            }
        }

        #[test]
        fn r346_the_base_face_prints_no_pierce_armor_takes_the_hit_and_the_trap_comes_all_the_same() {
            let mut s = kit("fvh-050-armor", false, 0);
            let armored = s.card(ARMORED).clone();
            s.activate(ID, json!({ "targets": pick::instance(&armored) }));
            s.expect_stats(&armored, json!({ "health": 7 }));
            one_cheap_trap(&s, "fvh-050-armor");
        }

        #[test]
        fn r81_its_own_side_is_no_target() {
            let mut s = kit("fvh-050-own", false, 0);
            s.expect_refused(|s| s.activate(ID, json!({ "targets": pick::hero(P1) })));
            s.expect_health(P1, HERO);
        }

        #[test]
        fn r129_with_no_open_backrow_zone_the_damage_lands_and_no_trap_is_summoned() {
            let mut s = kit("fvh-050-full", false, 4);
            s.activate(ID, json!({ "targets": pick::hero(P2) }));
            s.expect_health(P2, HERO - DAMAGE);
            assert!(traps(&s).is_empty());
        }

        #[test]
        fn r5236002_two_uses_spend_its_durability_2_and_it_goes_to_the_graveyard_by_a_plain_move() {
            let mut s = kit("fvh-050-walk", false, 4);
            let card = s.card(ID).clone();
            s.activate(ID, json!({ "targets": pick::hero(P2) }));
            assert_eq!(durability_left(s.state(), s.card(&card.id)), Some(DURABILITY - 1));
            run_as(&mut s, refresh_activates(), P1);
            s.activate(ID, json!({ "targets": pick::hero(P2) }));
            s.expect_health(P2, HERO - DURABILITY * DAMAGE);
            s.expect_in_zone(&card, "graveyard");
            assert_eq!(s.count("destroyed"), 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r81_it_deals_4_then_summons_a_random_radiant_1_or_2_cost_trap() {
            for n in 0..SEEDS {
                let seed = format!("fvh-050-radiant-{n}");
                let mut s = kit(&seed, true, 0);
                s.activate(ID, json!({ "targets": pick::hero(P2) }));
                s.expect_health(P2, HERO - RADIANT_DAMAGE);
                let trap = one_cheap_trap(&s, &seed);
                assert!(s.card(trap.as_str()).radiant, "{seed}");
            }
        }

        #[test]
        fn r346_its_printed_pierce_skips_the_enemy_units_armor() {
            let mut s = kit("fvh-050-pierce", true, 0);
            let armored = s.card(ARMORED).clone();
            s.activate(ID, json!({ "targets": pick::instance(&armored) }));
            s.expect_stats(&armored, json!({ "health": 7 - RADIANT_DAMAGE }));
        }
    }
}
