//! SPEC §8.12 S+ #10 Drone Assembler v5 (Unit 0/4 → 0/8, Weapon, cost 4, Legendary), a card of the held Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Starship Piece, Equip
//!            Activate 1: Target an enemy, summon a Drone Corvette to attack it"
//!   Radiant: "Starship Piece, Equip
//!            Activate 1: Target an enemy, summon a Radiant Drone Corvette to attack it"
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it fuses
//! into it (R1086), which then takes the Activate (R1751000) and is a piece only if it was one (R1852001).
//! The Activate is 1 use a turn (R384, R1857003). R3537000: its target is a declared enemy Unit,
//! never a hero (R81, R1862000). R3537001: the Drone Corvette is summoned into its controller's side by R64
//! (the leftmost open unit zone, else the leftmost empty Locked one, R688; a Flying Unit with its unit row
//! full, the backrow, R2248000) and makes one forced attack on that Unit (R53), so it has no summoning
//! sickness; the Activate is offered while it has a zone to enter and an enemy Unit stands (R430). The
//! Radiant face summons the Radiant Drone Corvette.

use jackioh_engine::effects::{forced_attacks, summon};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-010";

/// "Activate 1": 1 use a turn (R384, R1857003).
const USES: i32 = 1;

/// The Drone Corvette token (S §8.10 T-drone-corvette).
const CORVETTE: &str = "stinkyscape-t-drone-corvette";

/// R3537001: whether the Drone Corvette has a zone to enter, as a summon with no named zone finds it
/// (`first_entry_zone`): a Flying Unit's unit row, else its backrow (R64, R688, R2248000).
fn has_entry_zone(state: &GameState, player: PlayerId) -> bool {
    first_entry_zone(state, player, Row::Units).is_some() || first_entry_zone(state, player, Row::Backrow).is_some()
}

/// R3537000, R3537001: "Target an enemy, summon a [Radiant] Drone Corvette to attack it".
fn assemble(radiant: bool) -> Script {
    let label = if radiant {
        "Summon a Radiant Drone Corvette to attack an enemy Unit"
    } else {
        "Summon a Drone Corvette to attack an enemy Unit"
    };
    let activation = ActivationDecl {
        id: "assemble".to_string(),
        label: label.to_string(),
        uses: ActivationUses::Count(USES),
        cost: None,
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
                summon(json_as(json!({ "defId": CORVETTE, "radiant": radiant }))),
                // R53: only the drone this list summoned, at the chosen Unit.
                forced_attacks(json_as(json!({
                    "attackers": { "side": "self", "defId": CORVETTE, "summonedThisScript": true },
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
        base: assemble(false),
        radiant: assemble(true),
    }
}

// S+ #10 Drone Assembler v5 — SPEC §8.12 row 10 (R3537000, R3537001; R53, R64, R81, R384, R1086, R1751000, R2248000).
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

    const DRONE_ATTACK: i32 = 8;
    const RADIANT_DRONE_ATTACK: i32 = 16;
    const DRONE_HEALTH: i32 = 8;

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

    /// The drones this Activate summoned: (instance id, lane).
    fn summoned(events: &[GameEvent]) -> Vec<(String, i32)> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned {
                    instance_id,
                    def_id,
                    lane,
                    ..
                } if def_id == CORVETTE => Some((instance_id.clone(), *lane)),
                _ => None,
            })
            .collect()
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
                panic!("one Drone Corvette: {drones:?}");
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
                panic!("one Drone Corvette: {drones:?}");
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
        fn r384_the_uses_are_spent_a_turn_and_come_back_the_next() {
            let mut s = assembler(false, json!({ "field": [TOUGH] }));
            let target = s.unit_at(P2, 1).id;
            for _ in 0..USES {
                assert_eq!(activations(&s), 1);
                s.activate(ID, at(&target));
            }
            assert_eq!(activations(&s), 0);
            s.expect_refused(|s| s.activate(ID, at(&target)));
            let drones = (1..=UNIT_ZONES).filter_map(|lane| s.unit(P1, lane)).filter(|c| c.def_id == CORVETTE).count();
            assert_eq!(drones, USES as usize);
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
            s.play(ID, json!({ "zone": 1, "equip": true }));
            s.activate(host.as_str(), at(&target));
            let drones = summoned(s.last_events());
            assert_eq!(drones.len(), 1);
            assert_eq!(hits(s.last_events()).first(), Some(&(target, DRONE_ATTACK)));
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
                panic!("one Radiant Drone Corvette: {drones:?}");
            };
            assert!(s.card(drone.as_str()).radiant);
            assert_eq!(s.stats(drone.as_str()).attack, RADIANT_DRONE_ATTACK);
            assert_eq!(s.stats(drone.as_str()).health, 2 * DRONE_HEALTH);
            assert_eq!(hits(s.last_events()).first(), Some(&(target.clone(), RADIANT_DRONE_ATTACK)));
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
