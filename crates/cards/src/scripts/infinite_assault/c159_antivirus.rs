//! SPEC §8.13 IA #159 Antivirus ((2) Unit, Rare, 2/5 → 4/10), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cry: Destroy an enemy backrow card."
//!   Radiant: "Cry: Destroy an enemy backrow card. Then your hero gains {armor} Armor." (armor 5)
//!
//! Engine: the enemy backrow card picked with the play (R81, aimed to harm), destroyed (§6.3) as the
//! Cry resolves; on the Radiant face the controller's hero then gains the declared {armor} Armor.

use jackioh_engine::effects::{destroy, gain_hero_armor};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-159";

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl {
        aim: Some(TargetAim::Harm),
        ..TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["backrow"] }))
    }]
}

fn antivirus(radiant: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            let mut effects = vec![destroy(json_as(json!({ "target": { "of": "chosen" } })))];
            if radiant {
                effects.push(gain_hero_armor(json_as(json!({
                    "amount": param(&*ctx, "armor"),
                }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: antivirus(false),
        radiant: antivirus(true),
    }
}

// IA #159 Antivirus — SPEC §8.13 row 159 (R81).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";
    const FOE_TRAP: &str = "stand-in (1) Trap";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": [UNIT],
                "library": crate::filler(3),
                "mana": 8,
            },
            "p2": { "backrow": [FOE_TRAP], "field": [UNIT], "library": crate::filler(3) },
        }))
    }

    fn play(s: &mut Scenario) {
        let backrow = s.backrow(P2, 1).expect("the enemy backrow card");
        s.play(ID, pick::targets(pick::instance(&backrow)));
    }

    mod base {
        use super::*;

        #[test]
        fn cry_destroys_an_enemy_backrow_card() {
            let mut s = board(false);
            play(&mut s);
            s.expect_stats(&s.unit_at(P1, 2), json!({ "attack": 2, "health": 5 }));
            s.expect_in_zone(FOE_TRAP, "graveyard");
            assert_eq!(s.state().players[P1].hero.armor, 0);
        }

        #[test]
        fn r81_only_enemy_backrow_cards_are_legal_picks() {
            let s = board(false);
            let picks = pick::instance_ids(&s.play_targets(ID));
            let foe_backrow = s.backrow(P2, 1).expect("the enemy backrow card");
            assert!(picks.contains(&foe_backrow.id));
            let foe_unit = s.unit_at(P2, 1);
            assert!(!picks.contains(&foe_unit.id), "an enemy unit is none");
            let own_unit = s.unit_at(P1, 1);
            assert!(!picks.contains(&own_unit.id), "a friendly unit is none");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn then_your_hero_gains_5_armor() {
            let mut s = board(true);
            play(&mut s);
            s.expect_stats(&s.unit_at(P1, 2), json!({ "attack": 4, "health": 10 }));
            s.expect_in_zone(FOE_TRAP, "graveyard");
            assert_eq!(s.state().players[P1].hero.armor, 5);
        }
    }
}
