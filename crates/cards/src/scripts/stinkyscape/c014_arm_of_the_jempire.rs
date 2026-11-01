//! S #14 Arm of the Jempire (SPEC §8.10 row 14): (2) Unit, Human, Common, 4/5 → 8/10, of the
//! Stinkyscape set.
//!   Base:    "When one of your Taunt Units is attacked, give it +{buff} Attack."
//!   Radiant: "When one of your Taunt Units is attacked, give it +{buff} Attack."
//! Engine: `ally_attacked` (R1849008): once §4.2 step 4's trap window closes on a declared attack that
//! still stands on a Unit of its controller's, the hook reads the defender it is handed
//! (`ally_attacked_of`) and, when that Unit has Taunt through the layers (§10.4), gives it a plain
//! +{buff} Attack buff (`buff`, kept until it leaves the field, R78) before the combat, which reads it in
//! the strike back (R94). Two attacks stack two buffs. An attack on the hero and a forced attack wake
//! nothing (R121); a trap that re-aims the attack at another side's Unit leaves this side's hooks asleep.
//! Radiant: the same text, its 8/10 and +4 the catalog face's and its declared `buff`.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-014";

pub fn script() -> CardScripts {
    let base = Script {
        ally_attacked: Some(hook(|ctx| {
            let Some(facts) = ally_attacked_of(ctx) else {
                return Vec::new();
            };
            let taunted = find_instance(ctx.state, &facts.defender_id)
                .is_some_and(|defender| unit_has(ctx.state, defender, KeywordKind::Taunt));
            if !taunted {
                return Vec::new();
            }
            vec![buff(json_as(json!({
                "target": { "of": "instance", "instanceId": facts.defender_id },
                "attack": param(&*ctx, "buff"),
            })))]
        })),
        ..Script::default()
    };
    // The same script: the Radiant +4 is its declared `buff`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S #14 Arm of the Jempire — SPEC §8.10 row 14, BUILD M12 row S 14.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;

    const FILLER: &str = "core-005"; // (1) Spell: a card in hand, so no turn ends by itself.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const MENACE: &str = "core-019"; // (3) 9/9 Taunt.
    const SHREDDER: &str = "core-013"; // (3) 6/10: lives through a strike back of 9, not one of 11.
    const MOTHS: &str = "core-009"; // (2) 1/14: at its controller's start of turn every enemy Unit attacks it.
    const TRAITOROUS_BLOOD: &str = "meditative-064"; // Trap: re-aims an attack at the attacker's neighbour.

    fn board(radiant: bool, p1_field: Value, p2_field: Value) -> Scenario {
        let mut field = vec![json!({ "def": ID, "radiant": radiant })];
        field.extend(p1_field.as_array().cloned().unwrap_or_default());
        crate::scenario(json!({
            "active": "p2",
            "p1": { "hand": [FILLER], "field": field, "library": crate::filler(4) },
            "p2": { "hand": [FILLER], "field": p2_field, "library": crate::filler(4) },
        }))
    }

    fn attack_of(s: &Scenario, card: &CardInstance) -> i32 {
        s.stats(card).attack
    }

    mod base {
        use super::*;

        #[test]
        fn buffs_your_taunt_unit_attacked_before_the_combat() {
            let mut s = board(false, json!([MENACE]), json!([SHREDDER]));
            let menace = s.unit_at(P1, 2);
            let shredder = s.unit_at(P2, 1);
            s.attack(&shredder, &menace);
            assert_eq!(attack_of(&s, &menace), 11);
            s.expect_in_zone(&shredder, "graveyard");
            s.expect_stats(&menace, json!({ "health": 3 }));
        }

        #[test]
        fn a_unit_without_taunt_gets_nothing() {
            let mut s = board(false, json!([VANILLA]), json!([VANILLA, VANILLA]));
            let mine = s.unit_at(P1, 2);
            let arm = s.unit_at(P1, 1);
            s.attack(s.unit_at(P2, 1), &mine);
            s.attack(s.unit_at(P2, 2), &arm);
            assert_eq!(attack_of(&s, &arm), 4, "the Arm has no Taunt of its own");
            assert!(!s.events().iter().any(|event| matches!(event, GameEvent::Buffed { .. })));
        }

        #[test]
        fn an_attack_on_your_hero_gives_nothing() {
            // A Flying Taunt Unit in Attack Position binds only Flying and Ranged attackers (§6.1
            // Flying), so the Vanilla may go past it to the hero.
            let mut s = board(false, json!([MENACE]), json!([VANILLA]));
            let menace = s.unit_at(P1, 2);
            crate::give_flying(&mut s, &menace.id);
            s.attack(s.unit_at(P2, 1), "hero");
            s.expect_health(P1, HERO_HEALTH - 4);
            assert_eq!(attack_of(&s, &menace), 9);
        }

        #[test]
        fn a_forced_attack_gives_nothing() {
            let mut s = board(false, json!([MOTHS]), json!([VANILLA]));
            let moths = s.unit_at(P1, 2);
            s.card_mut(&moths.id).granted_keywords.push(Keyword::Taunt);
            // p1's start of turn: the Vanilla is made to attack the Moths, which no one declared (R121).
            s.end_turn();
            assert!(s.last_events().iter().any(|event| matches!(
                event,
                GameEvent::AttackDeclared { target_id, forced: true, .. } if *target_id == moths.id
            )));
            assert_eq!(attack_of(&s, &moths), 1);
        }

        #[test]
        fn a_trap_that_redirects_the_attack_gives_nothing() {
            let mut s = crate::scenario(json!({
                "active": "p2",
                "p1": {
                    "hand": [FILLER],
                    "field": [ID, MENACE],
                    "backrow": [{ "def": TRAITOROUS_BLOOD, "faceUp": false }],
                    "library": crate::filler(4),
                },
                "p2": { "hand": [FILLER], "field": [VANILLA, VANILLA], "library": crate::filler(4) },
            }));
            let menace = s.unit_at(P1, 2);
            let attacker = s.unit_at(P2, 1);
            s.attack(&attacker, &menace);
            assert!(s.last_events().iter().any(|event| matches!(event, GameEvent::Redirected { .. })));
            assert_eq!(attack_of(&s, &menace), 9);
            s.expect_stats(&menace, json!({ "health": 9 }));
        }

        #[test]
        fn two_attacks_stack() {
            let mut s = board(false, json!([MENACE]), json!([VANILLA, VANILLA]));
            let menace = s.unit_at(P1, 2);
            s.attack(s.unit_at(P2, 1), &menace);
            s.attack(s.unit_at(P2, 2), &menace);
            assert_eq!(attack_of(&s, &menace), 13);
        }

        #[test]
        fn buff_reads_through_param() {
            let mut s = board(false, json!([MENACE]), json!([VANILLA]));
            let arm = s.unit_at(P1, 1);
            step_param(s.card_mut(&arm.id), "buff", 1);
            let menace = s.unit_at(P1, 2);
            s.attack(s.unit_at(P2, 1), &menace);
            assert_eq!(attack_of(&s, &menace), 12);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn buffs_by_4_and_is_8_10() {
            let mut s = board(true, json!([MENACE]), json!([VANILLA]));
            s.expect_stats(s.unit_at(P1, 1), json!({ "attack": 8, "health": 10 }));
            let menace = s.unit_at(P1, 2);
            s.attack(s.unit_at(P2, 1), &menace);
            assert_eq!(attack_of(&s, &menace), 13);
        }
    }
}
