//! SPEC §8.12 S+ #147 Scourge Mortar v4 (Unit 0/3 → 0/6, Weapon, cost 3, Epic), a card of the held Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Starship Piece, Equip
//!            Activate: Toxic {toxic} an enemy Unit" — toxic 4
//!   Radiant: "Starship Piece, Equip
//!            Activate 2: Toxic {toxic} an enemy Unit" — toxic 4
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it fuses
//! into it (R1086), which then takes the Activate (R1751000) and is a piece only if it was one (R1852001).
//! The Activate is 1 use a turn on the base face, 2 on the Radiant face (R384, R1857003). R3544000: its
//! target is a declared enemy Unit, never a hero (R81, R1862000); the verb adds its Toxic and deals no
//! damage (R3540000). The number is the declared `toxic` (R386), read through `param`.

use jackioh_engine::effects::{AfflictArgs, TargetSpec, toxic};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-147";

/// "Activate": 1 use a turn on the base face (R384, R1857003).
const USES: i32 = 1;

/// "Activate 2": 2 uses a turn on the Radiant face (R384, R1857003).
const RADIANT_USES: i32 = 2;

/// R3544000: "Toxic N an enemy unit" puts N Toxic on the chosen enemy Unit alone.
fn mortar(uses: i32) -> Script {
    let activation = ActivationDecl {
        id: "toxic".into(),
        label: "Toxic an enemy Unit".into(),
        uses: ActivationUses::Count(uses),
        cost: None,
        // "an enemy Unit": enemy Units only, never a hero.
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit"] }))],
        modes: vec![],
        can_activate: Some(condition_hook(|c| {
            // R430: "Toxic N an enemy unit" needs an enemy Unit to name.
            !active_units_of(c.state, opponent_of(c.controller)).is_empty()
        })),
        has: None,
        run: hook(|ctx| {
            vec![toxic(AfflictArgs {
                to: TargetSpec::Chosen { index: None },
                amount: param(ctx, "toxic"),
            })]
        }),
    };
    Script {
        activations: vec![activation],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: mortar(USES),
        radiant: mortar(RADIANT_USES),
    }
}

// S+ #147 Scourge Mortar v4 — SPEC §8.12 row 147 (R3544000; R81, R384, R1086, R1751000, R3540000).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hits, next_turn_of, upgrade_number, weapon_board};
    use jackioh_engine::query::{hero_toxic_of, toxic_on};
    use jackioh_engine::subsystems::activate::activate_actions_for;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A durable enemy: it survives any Toxic's tick.
    const TOUGH: &str = "stand-in (1) 2/30";
    /// An equipped host.
    const HOST: &str = "stand-in (1) 4/4 R12/12";

    /// The printed Toxic, and the Radiant face's Radiant health.
    const TOXIC: i32 = 4;
    const RADIANT_HEALTH: i32 = 6;

    fn at(target: &str) -> Value {
        json!({ "ability": "toxic", "targets": pick::id(target) })
    }

    fn at_hero() -> Value {
        json!({ "ability": "toxic", "targets": [{ "pick": "hero", "player": "p2" }] })
    }

    fn mortar_on(radiant: bool, p2: Value) -> Scenario {
        weapon_board(
            "s147",
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            p2,
        )
    }

    /// The Activate's offers: one per enemy Unit it may name.
    fn offers(s: &Scenario) -> usize {
        activate_actions_for(s.state(), P1, s.card(ID)).len()
    }

    fn toxic_of(s: &Scenario, id: &str) -> i32 {
        toxic_on(s.card(id))
    }

    mod base {
        use super::*;

        #[test]
        fn r3544000_toxics_the_chosen_enemy_unit_alone() {
            let mut s = mortar_on(false, json!({ "field": [TOUGH, TOUGH] }));
            let target = s.unit_at(P2, 1).id;
            let beside = s.unit_at(P2, 2).id;
            s.activate(ID, at(&target));
            assert_eq!(toxic_of(&s, &target), TOXIC);
            assert_eq!(toxic_of(&s, &beside), 0);
            // A verb, no damage, and none on the enemy hero.
            assert!(hits(s.last_events()).is_empty());
            assert_eq!(hero_toxic_of(s.state(), P2), 0);
        }

        #[test]
        fn r3544000_its_target_is_an_enemy_unit_never_your_own_or_the_hero() {
            let mut s = weapon_board(
                "s147-target",
                json!({ "field": [ID, HOST] }),
                json!({ "field": [TOUGH, TOUGH] }),
            );
            let mine = s.unit_at(P1, 2).id;
            s.expect_refused(|s| s.activate(ID, at(&mine)));
            s.expect_refused(|s| s.activate(ID, at_hero()));
            // One offer for each enemy Unit it may name.
            assert_eq!(offers(&s), 2);
        }

        #[test]
        fn r3544000_it_is_not_offered_while_the_enemy_has_no_unit() {
            let mut s = mortar_on(false, json!({}));
            assert_eq!(offers(&s), 0);
            s.expect_refused(|s| s.activate(ID, at_hero()));
            s.expect_health(P2, HERO_HEALTH);
        }

        #[test]
        fn r384_the_uses_are_spent_a_turn_and_come_back_the_next() {
            let mut s = mortar_on(false, json!({ "field": [TOUGH] }));
            let target = s.unit_at(P2, 1).id;
            for _ in 0..USES {
                assert_eq!(offers(&s), 1);
                s.activate(ID, at(&target));
            }
            // A second use adds to the first (R3540000).
            assert_eq!(toxic_of(&s, &target), USES * TOXIC);
            assert_eq!(offers(&s), 0);
            s.expect_refused(|s| s.activate(ID, at(&target)));
            next_turn_of(&mut s, P1);
            assert_eq!(offers(&s), 1);
        }

        #[test]
        fn r1086_r1751000_an_equipped_host_takes_the_activate() {
            let mut s = weapon_board(
                "s147-equip",
                json!({ "hand": [ID], "field": [HOST], "mana": 10 }),
                json!({ "field": [TOUGH] }),
            );
            let host = s.unit_at(P1, 1).id;
            let target = s.unit_at(P2, 1).id;
            s.play(ID, json!({ "zone": 1, "equip": true }));
            s.activate(host.as_str(), at(&target));
            assert_eq!(toxic_of(&s, &target), TOXIC);
            assert!(hits(s.last_events()).is_empty());
        }

        #[test]
        fn toxic_reads_through_param() {
            let mut s = mortar_on(false, json!({ "field": [TOUGH] }));
            let amount = upgrade_number(&mut s, ID, "toxic");
            let target = s.unit_at(P2, 1).id;
            s.activate(ID, at(&target));
            assert_eq!(toxic_of(&s, &target), amount);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r384_it_has_two_uses_a_turn_each_adding_its_toxic() {
            let mut s = mortar_on(true, json!({ "field": [TOUGH] }));
            let target = s.unit_at(P2, 1).id;
            for _ in 0..RADIANT_USES {
                assert_eq!(offers(&s), 1);
                s.activate(ID, at(&target));
            }
            assert_eq!(toxic_of(&s, &target), RADIANT_USES * TOXIC);
            assert_eq!(offers(&s), 0);
            s.expect_refused(|s| s.activate(ID, at(&target)));
            next_turn_of(&mut s, P1);
            assert_eq!(offers(&s), 1);
        }

        #[test]
        fn r3544000_the_hero_is_refused_here_too() {
            let mut s = mortar_on(true, json!({ "field": [TOUGH] }));
            assert_eq!(offers(&s), 1);
            s.expect_refused(|s| s.activate(ID, at_hero()));
            assert_eq!(hero_toxic_of(s.state(), P2), 0);
        }

        #[test]
        fn its_health_is_doubled() {
            let s = mortar_on(true, json!({}));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (0, RADIANT_HEALTH));
        }
    }
}
