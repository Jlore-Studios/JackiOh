//! C+ #51 Jlockheed's J15 Fighter (SPEC §8.7 row 51). (3) Unit, Jlockeed, Epic, 7/2 → 14/4.
//!   Base:    "First Strike, Rush. Can't be in Defense Position. Can't be attacked."
//!   Radiant: "First Strike, Rush, Divine Shield" and the same.
//!   Engine:  "Can't be attacked (§6.1, §4.2 step 2): no attack may name it, a forced one included (a
//!            forced attack on it does not happen, and "a random enemy" never draws it); it is still
//!            targeted by effects and hit by "all" effects. A Taunt it gains binds nobody, since it
//!            cannot be attacked. The Defense restriction is a position validator flag. Tunes: none."
//!
//! Both restrictions are static flags the engine reads everywhere (`restrictions.cannotBeAttacked`,
//! #65.1's `neverDefense`); the keywords are the catalog's. Nothing else to do.

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-051";

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            never_defense: Some(true),
            cant_be_attacked: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    // The same script: the Radiant face differs only in its stats and Divine Shield, both catalog data.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// C+ #51 Jlockheed's J15 Fighter — SPEC §8.7 row 51, BUILD M9 Classic+ row C+ 51: "First Strike, Rush;
// cannot be in Defense Position; can't be attacked: never the target of a declared or forced attack (a
// random-enemy forced attack never draws it), while effects still target it and "all" effects hit it;
// a Taunt given to it binds no attacker; radiant 14/4 with Divine Shield too".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const J15: &str = "classicplus-051";
    const VANILLA: &str = "core-008"; // Mr. Vanilla 4/4
    const HIT_JOB: &str = "core-016"; // (3) Spell: destroy target Unit
    const NETHER: &str = "core-088"; // (4) Spell: destroy all permanents
    const HONEYPOT: &str = "core-060"; // Trap: Radiant fires on any card the opponent plays; Rush Tokens attack a Unit
    const FILLER: &str = "core-005";

    /// `_harness.ts` registers every card on import; the engine's testkit cannot, so this does.
    fn scenario(opts: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(opts)
    }

    fn kinds(s: &Scenario, card: &str) -> Vec<String> {
        s.stats(card)
            .keywords
            .iter()
            .map(|keyword| keyword.kind().as_str().to_string())
            .collect()
    }

    /// TS `s.unit(p, lane) ?? ""`: the unit's instance id, or "" (which no card answers to).
    fn unit_id(s: &Scenario, player: PlayerId, lane: i32) -> String {
        s.unit(player, lane).map(|card| card.id).unwrap_or_default()
    }

    /// J15 on p1's side; p2 to act with a 4/4 that may attack.
    fn defended(radiant: bool, p1_field: &[&str]) -> Scenario {
        let mut field = vec![json!({ "def": J15, "radiant": radiant })];
        field.extend(p1_field.iter().map(|def| json!(def)));
        scenario(json!({
            "active": "p2",
            "p1": { "field": field, "hand": [FILLER] },
            "p2": { "field": [VANILLA], "hand": [HIT_JOB, NETHER, FILLER] },
        }))
    }

    #[test]
    fn is_a_7_2_first_strike_rush_jlockeed_unit_both_faces_run_one_script_of_two_flags() {
        crate::register_all();
        let def = crate::card_def(super::ID);
        assert_eq!(def.id, J15);
        assert_eq!(def.tags, vec![Tag::Jlockeed]);
        let scripts = super::script();
        assert_eq!(
            serde_json::to_value(scripts.base.static_flags.as_ref().expect("static flags")).unwrap(),
            json!({ "neverDefense": true, "cantBeAttacked": true })
        );
        // TS `expect(radiant).toBe(base)`: the one script, flags and nothing else.
        assert_eq!(scripts.radiant.static_flags, scripts.base.static_flags);
        assert!(scripts.radiant.cry.is_none() && scripts.base.cry.is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn s6_1_7_2_with_first_strike_and_rush() {
            let mut s = defended(false, &[]);
            s.expect_stats(J15, json!({ "attack": 7, "health": 2 }));
            let found = kinds(&s, J15);
            assert!(found.contains(&"First Strike".to_string()));
            assert!(found.contains(&"Rush".to_string()));
            assert!(!found.contains(&"Divine Shield".to_string()));
        }

        #[test]
        fn s4_1_a_switch_to_defense_position_is_refused() {
            let mut s = scenario(json!({ "p1": { "field": [J15], "hand": [FILLER] } }));
            s.expect_refused_with(|s| s.switch_position(J15), "Defense Position");
        }

        #[test]
        fn s4_2_step_2_a_declared_attack_on_it_is_refused_the_hero_stays_open() {
            let mut s = defended(false, &[]);
            s.expect_refused_with(|s| s.attack(VANILLA, J15), "cannot be attacked");
            s.attack(VANILLA, "hero");
            s.expect_health(PlayerId::P1, 26);
            s.expect_stats(J15, json!({ "health": 2 }));
        }

        #[test]
        fn e35_a_random_enemy_forced_attack_never_draws_it_the_hero_and_the_other_units_only() {
            let s = defended(false, &[VANILLA]);
            let attacker = s.unit(PlayerId::P2, 1).unwrap_or_else(|| panic!("no attacker"));
            let mut targets: Vec<String> = random_attack_targets(s.state(), &attacker, json_as(json!("enemies")))
                .iter()
                .map(|target| match target {
                    DamageTarget::Hero { player } => format!("hero-{player}"),
                    DamageTarget::Unit { instance } => instance.def_id.clone(),
                })
                .collect();
            targets.sort();
            assert_eq!(targets, vec![VANILLA.to_string(), "hero-p1".to_string()]);
            let units: Vec<String> = random_attack_targets(s.state(), &attacker, json_as(json!("enemyUnits")))
                .iter()
                .map(|target| match target {
                    DamageTarget::Unit { instance } => instance.def_id.clone(),
                    DamageTarget::Hero { .. } => String::new(),
                })
                .collect();
            assert_eq!(units, vec![VANILLA.to_string()]);
        }

        #[test]
        fn r53_a_forced_attack_on_it_does_not_happen_a_radiant_bear_honeypots_rush_tokens_never_hit_it() {
            let mut s = scenario(json!({
                "p1": { "hand": [J15, FILLER] },
                "p2": { "backrow": [{ "def": HONEYPOT, "radiant": true, "faceUp": false }], "hand": [FILLER] },
            }));
            s.play(J15, json!({}));
            assert!(s.events().iter().any(|event| matches!(event, GameEvent::TrapFired { .. })));
            assert!(
                s.events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::Summoned { player: PlayerId::P2, .. }))
                    .count()
                    > 0
            );
            assert!(!s.events().iter().any(|event| matches!(event, GameEvent::AttackDeclared { .. })));
            s.expect_stats(J15, json!({ "health": 2 }));
        }

        #[test]
        fn e35_a_taunt_it_gains_binds_no_attacker() {
            let mut s = defended(false, &[VANILLA]);
            let id = s.card(J15).id.clone();
            find_instance_mut(s.state_mut(), &id)
                .expect("the card is in the state")
                .granted_keywords
                .push(Keyword::Taunt);
            assert!(kinds(&s, J15).contains(&"Taunt".to_string()));
            let target = unit_id(&s, PlayerId::P1, 2);
            s.attack(VANILLA, &target);
            assert!(s.events().iter().any(|event| matches!(event, GameEvent::AttackDeclared { .. })));
        }

        #[test]
        fn e35_an_effect_still_targets_it_hit_job_destroys_it() {
            let mut s = defended(false, &[]);
            let id = s.card(J15).id.clone();
            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": id }] }));
            s.expect_in_zone(J15, "graveyard");
        }

        #[test]
        fn e35_an_all_effect_still_hits_it_twisting_nether_destroys_it() {
            let mut s = defended(false, &[]);
            s.play(NETHER, json!({}));
            s.expect_in_zone(J15, "graveyard");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn s5_2_14_4_with_first_strike_rush_and_divine_shield_still_never_attacked() {
            let mut s = defended(true, &[]);
            s.expect_stats(J15, json!({ "attack": 14, "health": 4 }));
            let found = kinds(&s, J15);
            for kind in ["First Strike", "Rush", "Divine Shield"] {
                assert!(found.contains(&kind.to_string()), "{kind} in {found:?}");
            }
            s.expect_refused_with(|s| s.attack(VANILLA, J15), "cannot be attacked");
        }

        #[test]
        fn s4_1_the_radiant_face_still_refuses_defense_position() {
            let mut s = scenario(json!({ "p1": { "field": [{ "def": J15, "radiant": true }], "hand": [FILLER] } }));
            s.expect_refused_with(|s| s.switch_position(J15), "Defense Position");
        }
    }
}
