//! TG #13 Explosive Runes (SPEC §8.11 row 13). (1) Trap, Common. "Reveals when an enemy Unit attacks
//! your hero: Deal {damage} damage to all enemies." (Radiant: Pierce, 6 damage.)
//! C+ #1 trigger: declared attack (forced opens no window, R121) on hero in §4.2 step 4 trap window after exertion (R44).
//! S #13 damage_all before state check (R59); killing attacker resolves no combat (R220). Radiant states Pierce (§6.1, R2199001).

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-013";

/// R2199002: a declared attack by an enemy Unit on this trap's controller's hero.
fn attacks_your_hero(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    let GameEvent::AttackDeclared { attacker_id, target_id, forced, .. } = event else {
        return false;
    };
    if *forced {
        return false;
    }
    let target = attack_target_of(ctx.state, target_id);
    find_instance(ctx.state, attacker_id).map(|attacker| attacker.controller) == Some(opponent_of(ctx.controller))
        && matches!(target, Some(AttackTarget::Hero { player }) if player == ctx.controller)
}

fn explosive_runes(pierce: bool) -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("explosive-runes", &[GameEventType::AttackDeclared], move |ctx, _event| {
                let mut args = json!({ "amount": param(&*ctx, "damage"), "side": "enemy", "heroes": true });
                if pierce {
                    args["ignoreArmor"] = json!(true);
                }
                vec![damage_all(json_as(args))]
            })
            .with_when(attacks_your_hero),
        ],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: explosive_runes(false),
        radiant: explosive_runes(true),
    }
}

// TG #13 Explosive Runes — SPEC §8.11 row 13, BUILD M13 row TG 13.
// lane 2; p2 is active and attacks.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::{merged, upgrade_number};

    const RUNES: &str = super::ID;
    const TIMMY: &str = "core-011"; // (1) 3/3 Rush, First Strike.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const ARMORED: &str = "core-025"; // (4) 7/7, Armor 7.
    const DEFENDER: &str = "core-003"; // (1) 1/1 Taunt, Divine Shield, Reborn.
    const FILLER: &str = "core-010";
    const STOCKPILE: &str = "core-005";

    fn setup(p1: Value, p2: Value, radiant_face: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "active": "p2",
            "p1": merged(
                json!({
                    "hand": [FILLER],
                    "library": [STOCKPILE, STOCKPILE],
                    "backrow": [{ "def": RUNES, "radiant": radiant_face, "faceUp": false, "lane": 2 }],
                }),
                p1,
            ),
            "p2": merged(json!({ "hand": [FILLER], "library": [STOCKPILE, STOCKPILE] }), p2),
        }))
    }

    fn fired(s: &Scenario) -> bool {
        s.events().iter().any(|event| matches!(event, GameEvent::TrapFired { .. }))
    }

    /// The trigger's `when`, asked with the context `traps::fire_trap` hands it (C #96 My Pawn's
    /// `arms`): the trap as `self`, its controller as the controller. It only reads.
    fn arms(s: &Scenario, event: &GameEvent) -> bool {
        let trap = s.card(RUNES).clone();
        let mut state = s.state().clone();
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            Some(&trap),
            HookOptions { controller: Some(trap.controller), ..Default::default() },
        );
        let when = super::script().base.triggers[0].when.clone().expect("a `when`");
        when(&mut ctx, event)
    }

    fn combat_damage_to_hero(s: &Scenario, player: PlayerId) -> bool {
        let hero = format!("hero-{}", player.as_str());
        s.events().iter().any(|event| {
            matches!(event, GameEvent::Damage { target_id, combat: true, .. } if *target_id == hero)
        })
    }

    #[test]
    fn r33_it_is_set_face_down_and_the_opponent_sees_no_identity_until_it_fires() {
        let s = setup(json!({}), json!({ "field": [VANILLA] }), false);
        assert_eq!(s.backrow(P1, 2).unwrap().face_up, Some(false));
        let seen = view_for(s.state(), P2);
        let backrow = serde_json::to_string(&seen).unwrap();
        assert!(!backrow.contains(RUNES), "p2's view names the face-down trap");
    }

    mod base {
        use super::*;

        #[test]
        fn r2199002_an_enemy_units_attack_on_your_hero_reveals_it_before_any_combat_damage() {
            let mut s = setup(json!({}), json!({ "field": [VANILLA] }), false);
            let attacker = s.unit(P2, 1).unwrap();
            s.attack(&attacker, "hero");

            let types: Vec<String> = s.events().iter().map(|e| e.event_type().as_str().to_string()).collect();
            let declared = types.iter().position(|t| t == "attackDeclared").unwrap();
            let trap = types.iter().position(|t| t == "trapFired").unwrap();
            let first_damage = types.iter().position(|t| t == "damage").unwrap();
            assert!(declared < trap && trap < first_damage);
            s.expect_in_zone(RUNES, "graveyard");
        }

        #[test]
        fn r59_it_deals_3_to_each_enemy_unit_and_the_enemy_hero_before_one_state_check() {
            let mut s = setup(json!({}), json!({ "field": [VANILLA, TIMMY, TIMMY] }), false);
            let attacker = s.unit(P2, 1).unwrap();
            s.attack(&attacker, "hero");

            s.expect_health(P2, 27);
            // Both Timmys took their 3 in the one list, then died together at its check.
            let damage_before_death = s
                .events()
                .iter()
                .take_while(|event| !matches!(event, GameEvent::Destroyed { .. }))
                .filter(|event| matches!(event, GameEvent::Damage { .. }))
                .count();
            assert_eq!(damage_before_death, 4);
            assert!(s.unit(P2, 2).is_none() && s.unit(P2, 3).is_none());
        }

        #[test]
        fn r220_an_attacker_with_3_health_dies_on_its_declared_attack_and_your_hero_takes_nothing() {
            let mut s = setup(json!({}), json!({ "field": [TIMMY] }), false);
            let attacker = s.unit(P2, 1).unwrap();
            s.attack(&attacker, "hero");

            // The attack was the player's declaration (§4.2 step 3, which spends the exertion, R44), and
            // the trap fired after it. The dead attacker's own exertion cannot be read: R78 resets a card
            // that leaves the field.
            let types: Vec<GameEventType> = s.events().iter().map(GameEvent::event_type).collect();
            let declared = s.events().iter().position(|event| {
                matches!(
                    event,
                    GameEvent::AttackDeclared { attacker_id, forced: false, .. } if *attacker_id == attacker.id
                )
            });
            let trap = types.iter().position(|kind| *kind == GameEventType::TrapFired);
            assert!(matches!((declared, trap), (Some(declared), Some(trap)) if declared < trap));
            assert!(!types.contains(&GameEventType::AttackCancelled));
            assert!(fired(&s));
            s.expect_in_zone(TIMMY, "graveyard");
            s.expect_health(P1, 30);
            assert!(!combat_damage_to_hero(&s, P1));
        }

        #[test]
        fn r220_a_4_health_attacker_survives_and_its_attack_resolves() {
            let mut s = setup(json!({}), json!({ "field": [VANILLA] }), false);
            let attacker = s.unit(P2, 1).unwrap();
            s.attack(&attacker, "hero");

            s.expect_stats(VANILLA, json!({ "health": 1 }));
            s.expect_health(P1, 26);
        }

        #[test]
        fn r220_an_armored_attacker_survives_and_its_attack_resolves() {
            let mut s = setup(json!({}), json!({ "field": [ARMORED] }), false);
            let attacker = s.unit(P2, 1).unwrap();
            s.attack(&attacker, "hero");

            s.expect_stats(ARMORED, json!({ "health": 7 }));
            s.expect_health(P1, 23);
        }

        #[test]
        fn r2199002_an_attack_on_one_of_your_units_leaves_it_set() {
            let mut s = setup(json!({ "field": [VANILLA] }), json!({ "field": [TIMMY] }), false);
            let attacker = s.unit(P2, 1).unwrap();
            let target = s.unit(P1, 1).unwrap();
            s.attack(&attacker, &target);

            assert!(!fired(&s));
            assert_eq!(s.backrow(P1, 2).unwrap().def_id, RUNES);
        }

        #[test]
        fn r121_a_forced_attack_on_your_hero_leaves_it_set() {
            // §6.3 Forced attack, run through the engine: p2's own effect makes its 4/4 attack p1's hero
            // (C #9 Moths to the Flame's verb, aimed at the hero). The declaration is the effect's, not
            // the opponent's, so the trap never arms, the hero takes the hit and the trap stays set.
            let mut s = setup(json!({}), json!({ "field": [VANILLA] }), false);
            let attacker = s.unit(P2, 1).unwrap();
            let events = crate::run_as(
                &mut s,
                jackioh_engine::effects::forced_attacks_on(json_as(json!({
                    "target": { "of": "enemyHero" },
                    "attackers": "self",
                }))),
                P2,
            );
            assert!(
                events.iter().any(|event| matches!(
                    event,
                    GameEvent::AttackDeclared { attacker_id, target_id, forced: true, .. }
                        if *attacker_id == attacker.id && target_id == "hero-p1"
                )),
                "the forced attack on p1's hero was made"
            );
            assert!(!events.iter().any(|event| matches!(event, GameEvent::TrapFired { .. })));
            s.expect_health(P1, 26);
            s.expect_stats(VANILLA, json!({ "health": 4 }));
            assert_eq!(s.backrow(P1, 2).map(|card| card.def_id).as_deref(), Some(RUNES));
            assert_eq!(s.backrow(P1, 2).unwrap().face_up, Some(false));

            // The `when` itself, which the engine's run above never reaches (a forced attack opens no
            // trap window): the forced declaration does not arm it, the same one made by the player does.
            let declared_as = |forced: bool| GameEvent::AttackDeclared {
                attacker_id: attacker.id.clone(),
                target_id: "hero-p1".to_string(),
                forced,
                instead_of: None,
            };
            assert!(!arms(&s, &declared_as(true)));
            assert!(arms(&s, &declared_as(false)));
        }

        #[test]
        fn r386_damage_reads_through_param() {
            let mut s = setup(json!({}), json!({ "field": [VANILLA] }), false);
            assert_eq!(upgrade_number(&mut s, RUNES, "damage"), 4);
            let attacker = s.unit(P2, 1).unwrap();
            s.attack(&attacker, "hero");

            s.expect_health(P2, 26);
            s.expect_in_zone(VANILLA, "graveyard");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r2199002_it_deals_6_to_each_enemy() {
            let mut s = setup(json!({}), json!({ "field": [VANILLA, { "def": "core-019", "lane": 2 }] }), true);
            let attacker = s.unit(P2, 1).unwrap();
            s.attack(&attacker, "hero");

            s.expect_health(P2, 24);
            s.expect_stats("core-019", json!({ "health": 3 }));
            s.expect_in_zone(VANILLA, "graveyard");
        }

        #[test]
        fn r2199001_pierce_an_attacker_with_6_health_and_armor_kills_it_and_the_hero_takes_nothing() {
            let mut s = setup(json!({}), json!({ "field": [{ "def": ARMORED, "damage": 1 }] }), true);
            let attacker = s.unit(P2, 1).unwrap();
            s.attack(&attacker, "hero");

            s.expect_in_zone(ARMORED, "graveyard");
            s.expect_health(P1, 30);
            assert!(!combat_damage_to_hero(&s, P1));
        }

        #[test]
        fn r2199001_a_divine_shield_still_stops_a_pierced_hit() {
            let mut s = setup(json!({}), json!({ "field": [DEFENDER] }), true);
            let attacker = s.unit(P2, 1).unwrap();
            s.attack(&attacker, "hero");

            s.expect_health(P1, 29);
            s.expect_in_zone(DEFENDER, "field");
        }
    }
}
