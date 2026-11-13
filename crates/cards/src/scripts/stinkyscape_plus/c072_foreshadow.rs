//! SPEC §8.12 S+ #72 Foreshadow, a card of the Stinkyscape+ set (R1420, R3535000). Built by S08 (#3555).
//! **Indestructable**  
//! **Can’t attack**  
//! **Can’t defend**  
//! **Immutable, Immune to Spells**  
//! **Flexible**  
//! **Tribute 5**  
//! **Ranged**  
//! 20/20  
//! When an enemy unit is summoned attack it and lose 1 mana next turn  
//! \~\~\~  
//! **Indestructable**  
//! **Can’t defend**  
//! **Immutable, Immune to Spells**  
//! **Flexible**  
//! **Tribute 5**  
//! **Ranged**  
//! 40/40  
//! When an enemy unit is summoned attack it and lose 1 mana next turn
//!
//! Tribute 5 is the script's `tribute` (R101); every other keyword is catalog data. An enemy summon is
//! answered at once (R3550000) with one trigger's two effects: a forced attack on the arrival, then 1
//! less mana next turn — each doing what it can, so a summon that can't be attacked still costs the
//! mana (R3555003). Both faces share the script; the Radiant face drops Can't attack in the catalog.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-072";

/// "Tribute 5" (R101).
const TRIBUTE: i32 = 5;

fn foreshadow() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE),
            ..StaticFlags::default()
        }),
        triggers: vec![
            TriggerDef::new(
                "foreshadow",
                &[GameEventType::Summoned, GameEventType::CardResolved],
                |ctx, event| {
                    let Some(target) = enemy_unit_arrival(ctx.state, ctx.controller, event) else {
                        return Vec::new();
                    };
                    vec![
                        forced_attack(json_as(json!({
                            "attacker": { "of": "self" },
                            "target": { "of": "instance", "instanceId": target },
                        }))),
                        next_turn_mana(json_as(json!({ "amount": -param(&*ctx, "mana") }))),
                    ]
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
        base: foreshadow(),
        radiant: foreshadow(),
    }
}

// S+ #72 Foreshadow — SPEC §8.12 row S+ 72 (R53, R101, R3543002, R3550000, R3555003).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::play_choices::tribute_cost_of;
    use jackioh_engine::testkit::*;

    use crate::weapon_board;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FODDER: &str = "stand-in (0) 1/1";
    const VICTIM: &str = "stand-in (0) 1/1";
    const EVASIVE: &str = "stand-in (0) 1/1 Evasive";
    const WALL: &str = "stand-in (0) 1/30";

    /// Foreshadow standing for p1, p2 holding units to summon.
    fn standing(radiant: bool, seed: &str, hand: Value) -> Scenario {
        weapon_board(
            seed,
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            json!({ "hand": hand }),
        )
    }

    /// Foreshadow in p1's hand with `fodder` units to pay its Tribute.
    fn holding(radiant: bool, seed: &str, fodder: usize) -> Scenario {
        weapon_board(
            seed,
            json!({
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": vec![FODDER; fodder],
                "mana": 10,
            }),
            json!({ "field": [WALL] }),
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

    /// Plays Foreshadow into lane 1, paying the units in lanes 1..=5.
    fn play(s: &mut Scenario) {
        let tributes: Vec<String> = (1..=5).map(|lane| s.unit_at(P1, lane).id).collect();
        let tributes: Vec<&str> = tributes.iter().map(String::as_str).collect();
        s.play(ID, json!({ "zone": 1, "tributes": tributes }));
    }

    #[test]
    fn r101_tribute_5() {
        for radiant in [false, true] {
            let mut s = holding(radiant, &format!("s72-tribute-{radiant}"), TRIBUTE as usize);
            assert_eq!(tribute_cost_of(s.state(), s.card(ID)), TRIBUTE, "radiant {radiant}");
            s.expect_refused(|s| s.play(ID, json!({ "zone": 1 })));
            play(&mut s);
            assert_eq!(s.unit_at(P1, 1).def_id, ID, "radiant {radiant}");
        }
    }

    #[test]
    fn r3550000_it_attacks_an_enemy_unit_as_it_is_played() {
        for radiant in [false, true] {
            let mut s = standing(radiant, &format!("s72-ambush-{radiant}"), json!([VICTIM]));
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
            "s72-own",
            json!({ "field": [{ "def": ID }], "hand": [VICTIM] }),
            json!({}),
        );
        let mine = s.hand(P1).iter().find(|card| card.def_id == VICTIM).unwrap().id.clone();
        s.play(mine.as_str(), json!({ "zone": 2 }));
        assert!(forced_attackers(s.events()).is_empty());
    }

    #[test]
    fn r3555003_each_enemy_summon_queues_1_less_mana_next_turn() {
        let mut s = standing(false, "s72-mana", json!([VICTIM, VICTIM]));
        s.end_turn();
        for zone in [1, 2] {
            let victim = s.hand(P2).iter().find(|card| card.def_id == VICTIM).unwrap().id.clone();
            s.play(victim.as_str(), json!({ "zone": zone }));
        }
        assert_eq!(s.state().players[P1].mana.next_turn_mod, -2);
    }

    #[test]
    fn r3555003_an_evasive_summon_is_not_attacked_and_still_costs_the_mana() {
        let mut s = standing(false, "s72-evasive", json!([EVASIVE]));
        let hunter = s.unit_at(P1, 1).id;
        s.end_turn();
        let victim = s.hand(P2).iter().find(|card| card.def_id == EVASIVE).unwrap().id.clone();
        s.play(victim.as_str(), json!({ "zone": 1 }));
        // Evasive bars every attack (R3543002): no combat, but the mana is still lost.
        assert!(forced_attackers(s.events()).is_empty());
        s.expect_in_zone(victim.as_str(), "field");
        assert_eq!(s.state().players[P1].mana.next_turn_mod, -1);
        s.expect_stats(hunter.as_str(), json!({ "health": 20 }));
    }

    mod base {
        use super::*;

        #[test]
        fn cant_declare_an_attack() {
            let mut s = weapon_board(
                "s72-cant",
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
                "s72-may",
                json!({ "field": [{ "def": ID, "radiant": true }] }),
                json!({ "field": ["stand-in (0) 1/60"] }),
            );
            // A setup unit attacks once its sickness wears off.
            s.end_turn();
            s.end_turn();
            let wall = s.unit_at(P2, 1).id;
            s.attack(ID, wall.as_str());
            // 40 past no armor, and Ranged takes no strike back.
            assert_eq!(s.stats(wall.as_str()).health, 60 - 40);
            s.expect_stats(ID, json!({ "health": 40 }));
        }
    }
}
