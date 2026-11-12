//! SPEC §8.12 S+ #73 Ripple, a card of the Stinkyscape+ set (R1420, R3535000). Built by S08 (#3555).
//! **Can’t attack**  
//! **Can’t defend**  
//! **Flexible**  
//! **Ranged**  
//! 4/16  
//! When an enemy unit is summoned attack it  
//! \~\~\~  
//! **Can’t defend**  
//! **Flexible**  
//! **Ranged**  
//! 8/32  
//! When an enemy unit is summoned attack it
//!
//! An enemy summon is answered at once (R3550000) with a forced attack on the arrival — no exertion,
//! Can't attack ignored, no window (R53). No mana is lost: that rider is Foreshadow's alone. Both faces
//! share the script; the Radiant face drops Can't attack in the catalog.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-073";

fn ripple() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new(
                "ripple",
                &[GameEventType::Summoned, GameEventType::CardResolved],
                |ctx, event| {
                    let Some(target) = enemy_unit_arrival(ctx.state, ctx.controller, event) else {
                        return Vec::new();
                    };
                    vec![forced_attack(json_as(json!({
                        "attacker": { "of": "self" },
                        "target": { "of": "instance", "instanceId": target },
                    })))]
                },
            )
            .with_when(|ctx, event| enemy_unit_arrival(ctx.state, ctx.controller, event).is_some()),
        ],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The Radiant face may declare attacks; both faces answer summons the same way.
    CardScripts {
        base: ripple(),
        radiant: ripple(),
    }
}

// S+ #73 Ripple — SPEC §8.12 row S+ 73 (R53, R3550000).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::weapon_board;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VICTIM: &str = "stand-in (0) 1/1";
    const WALL: &str = "stand-in (0) 1/30";

    /// Ripple standing for p1, p2 holding units to summon.
    fn standing(radiant: bool, seed: &str, hand: Value) -> Scenario {
        weapon_board(
            seed,
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            json!({ "hand": hand }),
        )
    }

    /// The attackers of the forced attacks among the events, in order.
    fn forced_attackers(events: &[GameEvent]) -> Vec<String> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::AttackDeclared {
                    attacker_id,
                    forced: true,
                    ..
                } => Some(attacker_id.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn r3550000_it_attacks_an_enemy_summon() {
        for radiant in [false, true] {
            let mut s = standing(radiant, &format!("s73-ambush-{radiant}"), json!([VICTIM]));
            let hunter = s.unit_at(P1, 1).id;
            s.end_turn();
            let victim = s.hand(P2).iter().find(|card| card.def_id == VICTIM).unwrap().id.clone();
            s.play(victim.as_str(), json!({ "zone": 1 }));
            assert_eq!(forced_attackers(s.events()), vec![hunter.clone()], "radiant {radiant}");
            s.expect_in_zone(victim.as_str(), "graveyard");
            // A forced attack exerts nothing (R53), though the base face Can't attack.
            let exertion = &s.card(hunter.as_str()).exertion;
            assert!(!exertion.attacked && exertion.attacks.unwrap_or(0) == 0, "radiant {radiant}");
        }
    }

    #[test]
    fn r3550000_its_own_summons_are_not_attacked() {
        let mut s = weapon_board(
            "s73-own",
            json!({ "field": [{ "def": ID }], "hand": [VICTIM] }),
            json!({}),
        );
        let mine = s.hand(P1).iter().find(|card| card.def_id == VICTIM).unwrap().id.clone();
        s.play(mine.as_str(), json!({ "zone": 2 }));
        assert!(forced_attackers(s.events()).is_empty());
    }

    #[test]
    fn no_mana_is_lost() {
        let mut s = standing(false, "s73-mana", json!([VICTIM]));
        s.end_turn();
        let victim = s.hand(P2).iter().find(|card| card.def_id == VICTIM).unwrap().id.clone();
        s.play(victim.as_str(), json!({ "zone": 1 }));
        assert_eq!(forced_attackers(s.events()).len(), 1);
        assert_eq!(s.state().players[P1].mana.next_turn_mod, 0);
    }

    mod base {
        use super::*;

        #[test]
        fn cant_declare_an_attack() {
            let mut s = weapon_board(
                "s73-cant",
                json!({ "field": [{ "def": ID }] }),
                json!({ "field": [WALL] }),
            );
            let wall = s.unit_at(P2, 1).id;
            s.expect_refused(|s| s.attack(ID, wall.as_str()));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn it_may_declare_an_attack() {
            let mut s = weapon_board(
                "s73-may",
                json!({ "field": [{ "def": ID, "radiant": true }] }),
                json!({ "field": [WALL] }),
            );
            // A setup unit attacks once its sickness wears off.
            s.end_turn();
            s.end_turn();
            let wall = s.unit_at(P2, 1).id;
            s.attack(ID, wall.as_str());
            // 8 past no armor, and Ranged takes no strike back.
            assert_eq!(s.stats(wall.as_str()).health, 30 - 8);
            s.expect_stats(ID, json!({ "health": 32 }));
        }
    }
}
