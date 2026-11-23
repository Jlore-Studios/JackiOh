//! SPEC §8.12 S+ #10 Drone Assembler v5 (Unit 0/4 → 0/8, Weapon, cost 4, Legendary), a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Starship Piece
//!            Activate 2: Summon a Drone Interceptor to attack an enemy. Has an extra turn cooldown."
//!   Radiant: "Starship Piece
//!            Activate 2: Summon a Radiant Drone Interceptor to attack an enemy. Has an extra turn cooldown."
//!
//! Starship Piece is a catalog keyword the engine plays, and Equip was one through v0.6.7k (R4285001): Equipped onto one of your Units it fuses
//! into it (R1086), which then takes the Activate (R1751000) and is a piece only if it was one (R1852001).
//! The Activate is 2 uses a turn (R384, R1857003) with an extra turn cooldown (R1841000): both uses are
//! its turn's, and a turn it was used in is followed by one of its user's turns it sits out (R3996001).
//! R3537000: its target is a declared enemy Unit, never a hero (R81, R1862000). R3537001: the Drone
//! Interceptor is summoned into its controller's side by R64 (the leftmost open unit zone, else the
//! leftmost empty Locked one, R688; a Flying Unit with its unit row full, the backrow, R2248000) and makes
//! one forced attack on that Unit (R53), so it has no summoning sickness; the Activate is offered while it
//! has a zone to enter and an enemy Unit stands (R430). The Radiant face summons the Radiant Drone
//! Interceptor. Through v0.6.5b it was "Activate: Summon a Drone Corvette to attack an enemy", once a turn
//! with no cooldown; `script_v0_6_5b` keeps it for those replays (R767000).

use jackioh_engine::effects::{forced_attacks, summon};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-010";

/// "Activate 2": 2 uses a turn (R384, R1857003).
const USES: i32 = 2;
/// Through v0.6.5b, "Activate": 1 use a turn.
const USES_V0_6_5B: i32 = 1;

/// The Drone Interceptor token (S §8.10 T-drone-interceptor), and its name for the Activate's label.
const INTERCEPTOR: (&str, &str) = ("stinkyscape-t-drone-interceptor", "Drone Interceptor");
/// The Drone Corvette token (S §8.10 T-drone-corvette), what it summoned through v0.6.5b.
const CORVETTE: (&str, &str) = ("stinkyscape-t-drone-corvette", "Drone Corvette");

/// R3537001: whether the drone has a zone to enter, as a summon with no named zone finds it
/// (`first_entry_zone`): a Flying Unit's unit row, else its backrow (R64, R688, R2248000).
fn has_entry_zone(state: &GameState, player: PlayerId) -> bool {
    first_entry_zone(state, player, Row::Units).is_some() || first_entry_zone(state, player, Row::Backrow).is_some()
}

/// R3537000, R3537001: "Summon a [Radiant] `drone` to attack an enemy", `uses` a turn, with an extra
/// turn cooldown (R1841000, R3996001) when `cooldown`.
fn assemble(radiant: bool, (drone, name): (&'static str, &str), uses: i32, cooldown: bool) -> Script {
    let label = if radiant {
        format!("Summon a Radiant {name} to attack an enemy Unit")
    } else {
        format!("Summon a {name} to attack an enemy Unit")
    };
    let activation = ActivationDecl {
        id: "assemble".to_string(),
        label,
        uses: ActivationUses::Count(uses),
        cost: cooldown.then(|| ActivationCost {
            extra_turn_cooldown: Some(true),
            ..ActivationCost::default()
        }),
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit"] }))],
        modes: Vec::new(),
        can_activate: Some(condition_hook(|c| {
            // R430: "summon … to attack it" needs a zone for the drone and an enemy Unit to name.
            has_entry_zone(c.state, c.controller) && !active_units_of(c.state, opponent_of(c.controller)).is_empty()
        })),
        has: None,
        run: hook(move |ctx| {
            // A declaration with no legal pick is passed (R81): no Unit chosen, no drone.
            if ctx.targets.is_empty() {
                return Vec::new();
            }
            vec![
                summon(json_as(json!({ "defId": drone, "radiant": radiant }))),
                // R53: only the drone this list summoned, at the chosen Unit.
                forced_attacks(json_as(json!({
                    "attackers": { "side": "self", "defId": drone, "summonedThisScript": true },
                    "target": { "spec": { "of": "chosen" } },
                }))),
            ]
        }),
    };
    Script {
        activations: vec![activation],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: assemble(false, INTERCEPTOR, USES, true),
        radiant: assemble(true, INTERCEPTOR, USES, true),
    }
}

/// R767000: through v0.6.5b, a Drone Corvette once a turn with no cooldown; `KEPT_SCRIPTS` folds those
/// replays under it.
pub fn script_v0_6_5b() -> CardScripts {
    CardScripts {
        base: assemble(false, CORVETTE, USES_V0_6_5B, false),
        radiant: assemble(true, CORVETTE, USES_V0_6_5B, false),
    }
}

// S+ #10 Drone Assembler v5 — SPEC §8.12 row 10 (R3537000, R3537001, R3996001; R53, R64, R81, R384, R1086, R1751000, R1841000, R2248000).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hits, next_turn_of, weapon_board};
    use jackioh_engine::subsystems::activate::activate_actions_for;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A durable enemy: it survives any drone's hit.
    const TOUGH: &str = "stand-in (1) 2/30";
    const FIELD_SPELL: &str = "core-006"; // Mana Well: fills a backrow zone.

    const DRONE: &str = INTERCEPTOR.0;
    const DRONE_ATTACK: i32 = 2;
    const RADIANT_DRONE_ATTACK: i32 = 4;
    const DRONE_HEALTH: i32 = 2;
    /// What it summoned through v0.6.5b, and that drone's attack.
    const KEPT_DRONE: &str = CORVETTE.0;
    const KEPT_DRONE_ATTACK: i32 = 8;

    fn at(target: &str) -> Value {
        json!({ "ability": "assemble", "targets": pick::id(target) })
    }

    fn assembler(radiant: bool, p2: Value) -> Scenario {
        weapon_board(
            "s010",
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            p2,
        )
    }

    /// The drones of `def` this Activate summoned: (instance id, lane).
    fn summoned_of(events: &[GameEvent], def: &str) -> Vec<(String, i32)> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned {
                    instance_id,
                    def_id,
                    lane,
                    ..
                } if def_id == def => Some((instance_id.clone(), *lane)),
                _ => None,
            })
            .collect()
    }

    fn summoned(events: &[GameEvent]) -> Vec<(String, i32)> {
        summoned_of(events, DRONE)
    }

    fn activations(s: &Scenario) -> usize {
        activate_actions_for(s.state(), P1, s.card(ID)).len()
    }

    mod base {
        use super::*;

        #[test]
        fn r3537001_the_drone_lands_in_the_leftmost_open_zone_and_attacks_the_chosen_unit() {
            let mut s = assembler(false, json!({ "field": [TOUGH, TOUGH] }));
            let target = s.unit_at(P2, 2).id;
            let other = s.unit_at(P2, 1).id;
            // One offer for each enemy Unit it may name.
            assert_eq!(activations(&s), 2);
            s.activate(ID, at(&target));
            let drones = summoned(s.last_events());
            let [(drone, lane)] = drones.as_slice() else {
                panic!("one Drone Interceptor: {drones:?}");
            };
            // The Assembler stands in lane 1: the leftmost open zone is lane 2 (R64).
            assert_eq!(*lane, 2);
            assert!(!s.card(drone.as_str()).radiant);
            // R53: one forced attack, on the chosen Unit alone.
            let struck = hits(s.last_events());
            assert_eq!(struck.first(), Some(&(target.clone(), DRONE_ATTACK)), "{struck:?}");
            assert!(struck.iter().all(|(id, _)| *id != other), "{struck:?}");
            s.expect_stats(target.as_str(), json!({ "health": 30 - DRONE_ATTACK }));
            s.expect_health(P2, HERO_HEALTH);
        }

        #[test]
        fn r3537000_no_hero_may_be_chosen() {
            let at_hero = json!({ "ability": "assemble", "targets": [{ "pick": "hero", "player": "p2" }] });
            let mut s = assembler(false, json!({ "field": [TOUGH] }));
            assert_eq!(activations(&s), 1);
            s.expect_refused(|s| s.activate(ID, at_hero.clone()));
            s.expect_refused(|s| s.activate(ID, json!({ "ability": "assemble", "targets": [] })));
            assert!(summoned(s.events()).is_empty());
            // With no enemy Unit the Activate is not offered at all, the hero standing alone (R430).
            let mut s = assembler(false, json!({}));
            assert_eq!(activations(&s), 0);
            s.expect_refused(|s| s.activate(ID, at_hero.clone()));
            assert!(summoned(s.events()).is_empty());
            s.expect_health(P2, HERO_HEALTH);
        }

        #[test]
        fn r3537001_a_full_unit_row_puts_the_drone_in_the_backrow() {
            // The Assembler and four Units fill the row (R2248000: the drone flies).
            let mut s = weapon_board(
                "s010-backrow",
                json!({ "field": [ID, TOUGH, TOUGH, TOUGH, TOUGH] }),
                json!({ "field": [TOUGH] }),
            );
            let target = s.unit_at(P2, 1).id;
            assert_eq!(activations(&s), 1);
            s.activate(ID, at(&target));
            let drones = summoned(s.last_events());
            let [(drone, _)] = drones.as_slice() else {
                panic!("one Drone Interceptor: {drones:?}");
            };
            let backrow: Vec<String> = (1..=UNIT_ZONES)
                .filter_map(|lane| s.backrow(P1, lane).map(|card| card.id))
                .collect();
            assert_eq!(backrow, vec![drone.clone()]);
            assert_eq!(hits(s.last_events()).first(), Some(&(target, DRONE_ATTACK)));
        }

        #[test]
        fn r3537001_with_no_zone_in_either_row_it_is_not_offered_and_summons_nothing() {
            let mut s = weapon_board(
                "s010-full",
                json!({
                    "field": [ID, TOUGH, TOUGH, TOUGH, TOUGH],
                    "backrow": [FIELD_SPELL, FIELD_SPELL, FIELD_SPELL, FIELD_SPELL, FIELD_SPELL],
                }),
                json!({ "field": [TOUGH] }),
            );
            let target = s.unit_at(P2, 1).id;
            assert_eq!(activations(&s), 0);
            s.expect_refused(|s| s.activate(ID, at(&target)));
            assert!(summoned(s.events()).is_empty());
            s.expect_stats(target.as_str(), json!({ "health": 30 }));
        }

        #[test]
        fn r3996001_both_uses_this_turn_then_it_sits_out_the_next_and_is_back_the_turn_after() {
            let mut s = assembler(false, json!({ "field": [TOUGH] }));
            let target = s.unit_at(P2, 1).id;
            for _ in 0..USES {
                assert_eq!(activations(&s), 1);
                s.activate(ID, at(&target));
            }
            assert_eq!(activations(&s), 0);
            s.expect_refused_with(|s| s.activate(ID, at(&target)), "used 2 times");
            let drones = (1..=UNIT_ZONES).filter_map(|lane| s.unit(P1, lane)).filter(|c| c.def_id == DRONE).count();
            assert_eq!(drones, USES as usize);
            // R1841000: its user's next turn is the one it sits out...
            next_turn_of(&mut s, P1);
            assert_eq!(activations(&s), 0);
            s.expect_refused_with(|s| s.activate(ID, at(&target)), "cooling down");
            // ...and the turn after it has both uses again.
            next_turn_of(&mut s, P1);
            for _ in 0..USES {
                assert_eq!(activations(&s), 1);
                s.activate(ID, at(&target));
            }
            assert_eq!(activations(&s), 0);
        }

        #[test]
        fn r3996001_one_use_starts_the_cooldown_too() {
            let mut s = assembler(false, json!({ "field": [TOUGH] }));
            let target = s.unit_at(P2, 1).id;
            s.activate(ID, at(&target));
            next_turn_of(&mut s, P1);
            s.expect_refused_with(|s| s.activate(ID, at(&target)), "cooling down");
            next_turn_of(&mut s, P1);
            assert_eq!(activations(&s), 1);
        }

        #[test]
        fn r1086_r1751000_an_equipped_host_takes_the_activate() {
            let mut s = weapon_board(
                "s010-equip",
                json!({ "hand": [ID], "field": ["stand-in (1) 4/4 R12/12"], "mana": 10 }),
                json!({ "field": [TOUGH] }),
            );
            let host = s.unit_at(P1, 1).id;
            let target = s.unit_at(P2, 1).id;
            crate::give_equip(&mut s, ID);
            s.play(ID, json!({ "zone": 1, "equip": true }));
            s.activate(host.as_str(), at(&target));
            let drones = summoned(s.last_events());
            assert_eq!(drones.len(), 1);
            assert_eq!(hits(s.last_events()).first(), Some(&(target, DRONE_ATTACK)));
        }

        #[test]
        fn r767000_the_script_kept_for_v0_6_5b_summons_a_corvette_once_a_turn_with_no_cooldown() {
            crate::under_kept(ID, script_v0_6_5b(), || {
                let mut s = assembler(false, json!({ "field": [TOUGH] }));
                let target = s.unit_at(P2, 1).id;
                s.activate(ID, at(&target));
                assert_eq!(summoned_of(s.last_events(), KEPT_DRONE).len(), 1);
                assert!(summoned(s.last_events()).is_empty());
                assert_eq!(hits(s.last_events()).first(), Some(&(target.clone(), KEPT_DRONE_ATTACK)));
                assert_eq!(activations(&s), 0);
                s.expect_refused_with(|s| s.activate(ID, at(&target)), "already been used");
                // No cooldown: its user's next turn has its use back.
                next_turn_of(&mut s, P1);
                assert_eq!(activations(&s), 1);
            });
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3537001_the_drone_is_a_radiant_one_with_its_doubled_stats() {
            let mut s = assembler(true, json!({ "field": [TOUGH, TOUGH] }));
            let target = s.unit_at(P2, 1).id;
            s.activate(ID, at(&target));
            let drones = summoned(s.last_events());
            let [(drone, _)] = drones.as_slice() else {
                panic!("one Radiant Drone Interceptor: {drones:?}");
            };
            assert!(s.card(drone.as_str()).radiant);
            assert_eq!(s.stats(drone.as_str()).attack, RADIANT_DRONE_ATTACK);
            assert_eq!(s.stats(drone.as_str()).health, 2 * DRONE_HEALTH);
            assert_eq!(hits(s.last_events()).first(), Some(&(target.clone(), RADIANT_DRONE_ATTACK)));
        }

        #[test]
        fn r3996001_two_uses_then_the_cooldown_here_too() {
            let mut s = assembler(true, json!({ "field": [TOUGH] }));
            let target = s.unit_at(P2, 1).id;
            for _ in 0..USES {
                s.activate(ID, at(&target));
            }
            assert_eq!(summoned(s.events()).len(), USES as usize);
            assert_eq!(activations(&s), 0);
            next_turn_of(&mut s, P1);
            s.expect_refused_with(|s| s.activate(ID, at(&target)), "cooling down");
        }

        #[test]
        fn r3537000_a_hero_is_refused_here_too() {
            let mut s = assembler(true, json!({ "field": [TOUGH] }));
            s.expect_refused(|s| {
                s.activate(
                    ID,
                    json!({ "ability": "assemble", "targets": [{ "pick": "hero", "player": "p2" }] }),
                )
            });
            assert!(summoned(s.events()).is_empty());
        }

        #[test]
        fn its_health_is_doubled() {
            let s = assembler(true, json!({}));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (0, 8));
        }
    }
}
