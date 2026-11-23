//! SPEC §8.12 S+ #124 Sei, a card of the Stinkyscape+ set (R1420, R3535000). Built by S13 (#3560).
//! 5/12
//! **Tribute 2**
//! **Ranged**
//! **Taunt**
//! **Armor 5**
//! **Activate 1:** Deal 2 **Pierce Cleave** damage
//! \~\~\~
//! 10/24
//! **Tribute 2**
//! **Ranged**
//! **Taunt**
//! **Armor 10**
//! **Activate 2:** Deal 2 **Pierce Cleave** damage
//!
//! Tribute 2 is the script's `tribute` (R101). R3560008: the Activate aims at one declared enemy
//! Unit (R1862000); Pierce is `ignoreArmor`, Cleave the same hit on each Unit beside it (R346).
//! Once a turn on the base face, twice on the Radiant face (R384).

use jackioh_engine::effects::{
    BoardScope, DamageEffectArgs, DamageFlagArgs, TargetSpec, adjacent_to_aimed, damage,
};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-124";

/// "Tribute 2" (R101).
const TRIBUTE: i32 = 2;

/// "Activate 1" on the base face.
const USES: i32 = 1;
/// "Activate 2" on the Radiant face.
const RADIANT_USES: i32 = 2;

/// R1862000: the chosen enemy Unit's hit, then the same hit on each Unit beside it; Pierce is
/// `ignoreArmor` (R346).
fn shot(ctx: &EffectContext<'_>) -> Vec<Effect> {
    let amount = param(ctx, "damage");
    let hit = |to: TargetSpec| {
        damage(DamageEffectArgs {
            to,
            amount,
            flags: DamageFlagArgs {
                ignore_armor: Some(true),
                ..DamageFlagArgs::default()
            },
        })
    };
    let chosen = TargetSpec::Chosen { index: None };
    let beside = adjacent_to_aimed(ctx, &chosen, &BoardScope::default(), TargetAim::Harm);
    let mut hits = vec![hit(chosen)];
    hits.extend(
        beside
            .into_iter()
            .map(|unit| hit(TargetSpec::Instance { instance_id: unit.id })),
    );
    hits
}

fn sei(uses: i32) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE),
            ..StaticFlags::default()
        }),
        activations: vec![ActivationDecl {
            id: "sei".into(),
            label: "Deal damage to an enemy Unit, with Pierce and Cleave".into(),
            uses: ActivationUses::Count(uses),
            cost: None,
            // R703: the Activate needs its Unit, so it is never offered with no enemy Unit to aim at.
            targets: vec![TargetDecl {
                required: Some(true),
                ..TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit"] }))
            }],
            modes: vec![],
            can_activate: None,
            has: None,
            run: hook(|ctx| shot(ctx)),
        }],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: sei(USES),
        radiant: sei(RADIANT_USES),
    }
}

// S+ #124 Sei — SPEC §8.12 row 124.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hits, next_turn_of, weapon_board};
    use jackioh_engine::subsystems::activate::activate_actions_for;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MENACE: &str = "core-019"; // Midrange Menace: Taunt
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const ARMORED: &str = "stand-in (0) 40/60 Armor 5";

    const DAMAGE: i32 = 2;

    fn at(target: &str) -> Value {
        json!({ "ability": "sei", "targets": pick::id(target) })
    }

    fn sei_on(radiant: bool, p2: Value) -> Scenario {
        weapon_board(
            "s124",
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            p2,
        )
    }

    fn offers(s: &Scenario) -> usize {
        activate_actions_for(s.state(), P1, s.card(ID)).len()
    }

    fn cleaves(radiant: bool, amount: i32) {
        let mut s = sei_on(radiant, json!({ "field": [MENACE, MENACE, MENACE, VANILLA] }));
        let lanes = [1, 2, 3, 4].map(|lane| s.unit_at(P2, lane).id);
        s.activate(ID, at(&lanes[1]));
        let expected: Vec<(String, i32)> = [&lanes[1], &lanes[0], &lanes[2]]
            .iter()
            .map(|id| ((*id).clone(), amount))
            .collect();
        assert_eq!(hits(s.last_events()), expected, "radiant {radiant}");
        s.expect_stats(lanes[3].as_str(), json!({ "health": 4 }));
    }

    mod base {
        use super::*;

        #[test]
        fn r3560008_it_hits_an_enemy_unit_and_its_neighbours_piercing_armor() {
            cleaves(false, DAMAGE);
            // Pierce: Armor takes nothing.
            let mut s = sei_on(false, json!({ "field": [ARMORED] }));
            let wall = s.unit_at(P2, 1).id;
            s.activate(ID, at(&wall));
            assert_eq!(s.stats(wall.as_str()).health, 60 - DAMAGE);
        }

        #[test]
        fn r3560008_not_offered_without_an_enemy_unit() {
            let mut s = sei_on(false, json!({}));
            assert_eq!(offers(&s), 0);
            s.expect_refused(|s| s.activate(ID, at("hero")));
        }

        #[test]
        fn once_a_turn_then_back() {
            let mut s = sei_on(false, json!({ "field": [MENACE, VANILLA] }));
            let first = s.unit_at(P2, 1).id;
            let second = s.unit_at(P2, 2).id;
            s.activate(ID, at(&first));
            s.expect_refused(|s| s.activate(ID, at(&second)));
            next_turn_of(&mut s, P1);
            s.activate(ID, at(&second));
            assert_eq!(hits(s.last_events()), vec![(second, DAMAGE), (first, DAMAGE)]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3560008_it_hits_an_enemy_unit_and_its_neighbours_piercing_armor() {
            cleaves(true, DAMAGE);
        }

        #[test]
        fn radiant_activates_twice_a_turn() {
            let mut s = sei_on(true, json!({ "field": [MENACE, MENACE, VANILLA] }));
            let lanes = [1, 2, 3].map(|lane| s.unit_at(P2, lane).id);
            s.activate(ID, at(&lanes[0]));
            s.activate(ID, at(&lanes[2]));
            assert_eq!(offers(&s), 0);
            s.expect_refused(|s| s.activate(ID, at(&lanes[1])));
            next_turn_of(&mut s, P1);
            s.activate(ID, at(&lanes[1]));
        }
    }
}
