//! SPEC §8.12 S+ #125 Omura, a card of the Stinkyscape+ set (R1420, R3535000). Built by S13 (#3560).
//! 7/17
//! **Tribute 3**
//! **Ranged**
//! **Taunt**
//! **Armor 8**
//! **Activate 1:** Deal 12 **Pierce Trample** damage. Has a 3-turn cooldown.
//! \~\~\~
//! 14/34
//! **Tribute 3**
//! **Ranged**
//! **Taunt**
//! **Armor 16**
//! **Activate 1:** Deal 24 **Pierce Trample** damage. Has a 3-turn cooldown.
//!
//! Tribute 3 is the script's `tribute` (R101). R3560008: the Activate aims at one declared enemy
//! Unit (R1862000); Pierce is `ignoreArmor`, Trample's excess hits that Unit's controller's hero
//! (R346). R3550003: a 3-turn cooldown sits out two turns after a use.

use jackioh_engine::effects::{DamageEffectArgs, DamageFlagArgs, TargetSpec, damage};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-125";

/// "Tribute 3" (R101).
const TRIBUTE: i32 = 3;

/// "Activate 1": one use a turn (R384).
const USES: i32 = 1;

/// A 3-turn cooldown: the use sits out two turns (R3550003).
const COOLDOWN_TURNS: i32 = 3;

fn omura() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE),
            ..StaticFlags::default()
        }),
        activations: vec![ActivationDecl {
            id: "omura".into(),
            label: "Deal Pierce Trample damage to an enemy Unit".into(),
            uses: ActivationUses::Count(USES),
            cost: Some(ActivationCost {
                cooldown_turns: Some(COOLDOWN_TURNS),
                ..ActivationCost::default()
            }),
            targets: vec![TargetDecl::target(
                1,
                1,
                json!({ "side": "enemy", "of": ["unit"] }),
            )],
            modes: vec![],
            can_activate: None,
            has: None,
            run: hook(|ctx| {
                vec![damage(DamageEffectArgs {
                    to: TargetSpec::Chosen { index: None },
                    amount: param(ctx, "damage"),
                    flags: DamageFlagArgs {
                        ignore_armor: Some(true),
                        trample: Some(true),
                        ..DamageFlagArgs::default()
                    },
                })]
            }),
        }],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: omura(),
        radiant: omura(),
    }
}

// S+ #125 Omura — SPEC §8.12 row 125.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hits, next_turn_of, weapon_board};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BODY: &str = "stand-in (1) 4/4 R12/12";
    const ARMORED: &str = "stand-in (0) 40/60 Armor 5";

    fn at(target: &str) -> Value {
        json!({ "ability": "omura", "targets": pick::id(target) })
    }

    fn omura_on(p2: Value) -> Scenario {
        weapon_board("s125", json!({ "field": [ID] }), p2)
    }

    fn omura_radiant_on(p2: Value) -> Scenario {
        weapon_board(
            "s125-radiant",
            json!({ "field": [{ "def": ID, "radiant": true }] }),
            p2,
        )
    }

    #[test]
    fn r3560008_pierce_trample_excess_hits_the_hero() {
        for radiant in [false, true] {
            let damage = if radiant { 24 } else { 12 };
            let mut s = if radiant {
                omura_radiant_on(json!({ "field": [BODY] }))
            } else {
                omura_on(json!({ "field": [BODY] }))
            };
            let target = s.unit_at(P2, 1).id;
            s.activate(ID, at(&target));
            // A 4-health Unit takes 4, the excess hits the hero.
            assert_eq!(
                hits(s.last_events()),
                vec![
                    (target.clone(), damage.min(4)),
                    ("hero-p2".to_string(), damage - 4)
                ],
                "radiant {radiant}"
            );
            s.expect_health(P2, 30 - (damage - 4));
        }
        // Pierce: Armor takes nothing of a lethal hit's body damage.
        let mut s = omura_on(json!({ "field": [ARMORED] }));
        let wall = s.unit_at(P2, 1).id;
        s.activate(ID, at(&wall));
        assert_eq!(s.stats(wall.as_str()).health, 60 - 12);
        s.expect_health(P2, 30);
    }

    #[test]
    fn r3550003_it_sits_out_two_turns_after_a_use() {
        let mut s = omura_on(json!({ "field": [BODY, BODY] }));
        let first = s.unit_at(P2, 1).id;
        let second = s.unit_at(P2, 2).id;
        s.activate(ID, at(&first));
        // The same turn it is the Activate's one use that refuses the second; the cooldown sits out the
        // turns after it.
        s.expect_refused_with(|s| s.activate(ID, at(&second)), "already been used");
        // A cooldown of 3 sits out two of its user's turns (R3550003): the first two turns after the use
        // are refused, and the third has it back.
        next_turn_of(&mut s, P1);
        s.expect_refused_with(|s| s.activate(ID, at(&second)), "cooling down");
        next_turn_of(&mut s, P1);
        s.expect_refused_with(|s| s.activate(ID, at(&second)), "cooling down");
        next_turn_of(&mut s, P1);
        s.activate(ID, at(&second));
        assert_eq!(hits(s.last_events()).len(), 2);
    }

    mod radiant {
        use super::*;

        #[test]
        fn armor_16_is_printed() {
            let s = omura_radiant_on(json!({}));
            assert_eq!(s.stats(ID).armor, 16);
        }
    }
}
