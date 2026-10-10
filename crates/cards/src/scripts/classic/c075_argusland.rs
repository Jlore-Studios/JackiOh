//! C #75 Argusland (SPEC §8.6 row 75; §4.4 step 2; R18, R44, R125, R386). Field Spell, cost 1, Rare.
//!   Base:    "Aura: Damage to your hero is divided by {divisor}, rounded up." (2: halved)
//!   Radiant: the same with divisor 4 (quartered).
//!   Engine:  a hero damage multiplier after Armor and before the hit caps; several multiply. Fatigue is
//!            damage and is reduced too (R125); losing health is not (R18).
//!
//! The engine's hero guard (`Script.heroGuard`): the pipeline, R44's lethal projection and the lethal
//! window all read the same divided amount. Both faces run one script; `param` reads the face's divisor.

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-075";

pub fn script() -> CardScripts {
    let base = Script {
        hero_guard: Some(read_hook(|args| {
            vec![HeroGuard {
                divisor: Some(param(&args, "divisor")),
                ..HeroGuard::default()
            }]
        })),
        ..Script::default()
    };
    CardScripts {
        // The same script: the Radiant face's 4 is its declared `divisor`, which `param` reads off the running face.
        radiant: base.clone(),
        base,
    }
}

/// `describe("C #75 Argusland")`.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const ARGUSLAND: &str = "classic-075";
    const GAMBIT: &str = "classic-052"; // C #52 Final Gambit: redirects a lethal hit on your hero.
    const GARY: &str = "core-004"; // 1/1
    const VANILLA: &str = "core-008"; // 4/4
    const PANTHER: &str = "core-032"; // 5/4 Rush
    const POINTMASTER: &str = "core-020"; // 7/1 First Strike
    const MENACE: &str = "core-019"; // 9/9 Taunt
    const LUNAR: &str = "core-035"; // (1) Spell: deal 3 damage to a target.
    const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
    const BLOOD_BEAN: &str = "core-027"; // Cast on draw: make a random hand card Radiant. Lose 5 health.
    const GOING_LONG: &str = "core-084"; // Field Spell: your hero has Armor 2.
    const ANTI_ONESHOT: &str = "core-073"; // Field Spell: your hero can't take more than 5 damage at once.
    const MAGIC_JAMMED: &str = "core-036"; // (1) Spell: destroy target backrow card; lock its zone.
    const MIND_CONTROL: &str = "core-049"; // (4) Spell: steal target enemy permanent.
    const MY_PAWN: &str = "core-096"; // Trap: cancels a declared attack that would be lethal to your hero (R44).
    const FILLER: &str = "core-005";

    use crate::scenario;

    use crate::js;

    fn hero_hits(s: &Scenario, player: PlayerId) -> Vec<i32> {
        let hero = format!("hero-{player}");
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id, amount, .. } if *target_id == hero => Some(*amount),
                _ => None,
            })
            .collect()
    }

    /// A Radiant Argusland, as a backrow entry.
    fn radiant_argusland() -> Value {
        json!({ "def": ARGUSLAND, "radiant": true })
    }

    /// p2 attacks p1's hero with `attacker`, p1 guarded by the backrow it is given.
    fn attack_on_guarded(attacker: &str, backrow: Vec<Value>, health: i32) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [FILLER], "backrow": backrow, "health": health, "library": [FILLER, FILLER, FILLER] },
            "p2": { "hand": [FILLER], "field": [attacker] },
            "active": "p2",
        }));
        s.attack(attacker, "hero");
        s
    }

    /// runs one script on both faces: a hero guard whose divisor is the declared number
    #[test]
    fn runs_one_script_on_both_faces_a_hero_guard_whose_divisor_is_the_declared_number() {
        let def = crate::card_def(ARGUSLAND);
        assert_eq!(def.id, ARGUSLAND);
        assert_eq!(def.type_, CardType::FieldSpell);
        assert_eq!(
            js(&def.params),
            json!([{ "key": "divisor", "base": 2, "radiant": 4, "better": "up", "step": 1, "min": 2 }])
        );
        let scripts = script();
        // The Radiant face is the base face, its guard the same one.
        let guard = scripts.base.hero_guard.as_ref().expect("a hero guard");
        assert!(Arc::ptr_eq(guard, scripts.radiant.hero_guard.as_ref().expect("a hero guard")));
    }

    /// `describe("base")`.
    mod base {
        use super::*;

        /// halves a hit on your hero, rounded up: a 5 becomes 3
        #[test]
        fn halves_a_hit_on_your_hero_rounded_up_a_5_becomes_3() {
            let mut s = attack_on_guarded(PANTHER, vec![json!(ARGUSLAND)], 30);
            assert_eq!(hero_hits(&s, P1), vec![3]);
            s.expect_health(P1, 27);
        }

        /// rounds up, so a 1 stays 1
        #[test]
        fn rounds_up_so_a_1_stays_1() {
            let mut s = attack_on_guarded(GARY, vec![json!(ARGUSLAND)], 30);
            assert_eq!(hero_hits(&s, P1), vec![1]);
            s.expect_health(P1, 29);
        }

        /// a Spell's damage to your hero is halved too: Lunar Eclipse's 3 becomes 2
        #[test]
        fn a_spells_damage_to_your_hero_is_halved_too_lunar_eclipses_3_becomes_2() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "backrow": [ARGUSLAND] },
                "p2": { "hand": [LUNAR, FILLER] },
                "active": "p2",
            }));
            s.play(LUNAR, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
            assert_eq!(hero_hits(&s, P1), vec![2]);
            s.expect_health(P1, 28);
        }

        /// §4.4 step 2: it divides after Armor — Going Long's 2 off a 7 leaves 5, halved to 3
        #[test]
        fn s4_4_step_2_it_divides_after_armor_going_longs_2_off_a_7_leaves_5_halved_to_3() {
            let mut s = attack_on_guarded(POINTMASTER, vec![json!(ARGUSLAND), json!(GOING_LONG)], 30);
            assert_eq!(hero_hits(&s, P1), vec![3]);
            s.expect_health(P1, 27);
        }

        /// §4.4 step 3: it divides before the hit caps — a 9 halved to 5 meets Anti-oneshot Armor's 5
        #[test]
        fn s4_4_step_3_it_divides_before_the_hit_caps_a_9_halved_to_5_meets_anti_oneshot_armors_5() {
            let mut s = attack_on_guarded(MENACE, vec![json!(ARGUSLAND), json!(ANTI_ONESHOT)], 30);
            // Capped first it would be 5 halved to 3.
            assert_eq!(hero_hits(&s, P1), vec![5]);
            s.expect_health(P1, 25);
        }

        /// R125 fatigue is damage and is halved: the 1st and 2nd empty draws deal 1 and 1, not 1 and 2
        #[test]
        fn r125_fatigue_is_damage_and_is_halved_the_1st_and_2nd_empty_draws_deal_1_and_1_not_1_and_2() {
            let mut s = scenario(json!({ "p1": { "hand": [STOCKPILE, FILLER], "backrow": [ARGUSLAND], "health": 20 }, "p2": { "hand": [FILLER] } }));
            s.play(STOCKPILE, json!({}));
            assert_eq!(hero_hits(&s, P1), vec![1, 1]);
            s.expect_health(P1, 20);
        }

        /// R18 losing health is not damage: Blood Ridden Glowy Jelly Bean's 5 is lost whole
        #[test]
        fn r18_losing_health_is_not_damage_blood_ridden_glowy_jelly_beans_5_is_lost_whole() {
            let mut s = scenario(json!({
                "p1": { "hand": [STOCKPILE, FILLER], "backrow": [ARGUSLAND], "library": [BLOOD_BEAN, FILLER, FILLER], "health": 20 },
                "p2": { "hand": [FILLER] },
            }));
            s.play(STOCKPILE, json!({}));
            assert!(hero_hits(&s, P1).is_empty());
            s.expect_health(P1, 17);
        }

        /// damage to a Unit is untouched: Lunar Eclipse deals its whole 3 to your Mr. Vanilla
        #[test]
        fn damage_to_a_unit_is_untouched_lunar_eclipse_deals_its_whole_3_to_your_mr_vanilla() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [VANILLA], "backrow": [ARGUSLAND] },
                "p2": { "hand": [LUNAR, FILLER] },
                "active": "p2",
            }));
            let vanilla = s.unit(P1, 1).expect("Mr. Vanilla should be on the board");
            s.play(LUNAR, json!({ "targets": [{ "pick": "instance", "instanceId": vanilla.id }] }));
            s.expect_stats(&vanilla, json!({ "health": 1 }));
        }

        /// guards only your hero: your opponent's takes the whole hit
        #[test]
        fn guards_only_your_hero_your_opponents_takes_the_whole_hit() {
            let mut s = scenario(json!({ "p1": { "hand": [FILLER], "field": [VANILLA], "backrow": [ARGUSLAND] }, "p2": { "hand": [FILLER] } }));
            s.attack(VANILLA, "hero");
            assert_eq!(hero_hits(&s, P2), vec![4]);
        }

        /// it guards its controller's hero: stolen by Snom Bunny Mind Control, it halves hits on the thief's hero
        #[test]
        fn it_guards_its_controllers_hero_stolen_by_snom_bunny_mind_control_it_halves_hits_on_the_thiefs_hero() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [VANILLA], "backrow": [ARGUSLAND], "library": [FILLER] },
                "p2": { "hand": [MIND_CONTROL, FILLER], "field": [PANTHER] },
                "active": "p2",
            }));
            let argusland = s.card(ARGUSLAND).id.clone();
            s.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": argusland }] }));
            assert_eq!(s.card(ARGUSLAND).controller, P2);
            s.attack(PANTHER, "hero");
            assert_eq!(hero_hits(&s, P1), vec![5]);
            // p2 has nothing left it can do, so its turn has ended on its own (R82).
            assert_eq!(s.state().active, P1);
            s.attack(VANILLA, "hero");
            assert_eq!(hero_hits(&s, P2), vec![2]);
        }

        /// several multiply: two Arguslands divide by 4, so a 9 becomes 3
        #[test]
        fn several_multiply_two_arguslands_divide_by_4_so_a_9_becomes_3() {
            let s = attack_on_guarded(MENACE, vec![json!(ARGUSLAND), json!(ARGUSLAND)], 30);
            assert_eq!(hero_hits(&s, P1), vec![3]);
        }

        /// several multiply across faces: a base and a Radiant one divide by 8, so a 9 becomes 2
        #[test]
        fn several_multiply_across_faces_a_base_and_a_radiant_one_divide_by_8_so_a_9_becomes_2() {
            let s = attack_on_guarded(MENACE, vec![json!(ARGUSLAND), radiant_argusland()], 30);
            assert_eq!(hero_hits(&s, P1), vec![2]);
        }

        /// §4.4 step 4a the lethal window reads the halved amount: a 7 halved to 4 is lethal at 4 and Final Gambit fires
        #[test]
        fn s4_4_step_4a_the_lethal_window_reads_the_halved_amount_a_7_halved_to_4_is_lethal_at_4_and_final_gambit_fires() {
            let mut s = attack_on_guarded(POINTMASTER, vec![json!(ARGUSLAND), json!(GAMBIT)], 4);
            s.expect_events(json!(["trapFired", "redirected"]));
            s.expect_in_zone(GAMBIT, "graveyard");
            s.expect_health(P1, 14);
        }

        /// §4.4 step 4a ... and at 5 health the same hit is not lethal, so Final Gambit stays set
        #[test]
        fn s4_4_step_4a_and_at_5_health_the_same_hit_is_not_lethal_so_final_gambit_stays_set() {
            let mut s = attack_on_guarded(POINTMASTER, vec![json!(ARGUSLAND), json!(GAMBIT)], 5);
            assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::TrapFired));
            s.expect_in_zone(GAMBIT, "field");
            assert_ne!(s.card(GAMBIT).face_up, Some(true));
            s.expect_health(P1, 1);
        }

        /// R44 My Pawn's lethal projection reads the halved amount: a 4 at 4 health is a 2, so the attack stands
        #[test]
        fn r44_my_pawns_lethal_projection_reads_the_halved_amount_a_4_at_4_health_is_a_2_so_the_attack_stands() {
            let mut s = attack_on_guarded(VANILLA, vec![json!(ARGUSLAND), json!(MY_PAWN)], 4);
            assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::TrapFired));
            s.expect_health(P1, 2);
        }

        /// is gone when it leaves: destroyed by Magic Jammed, the next hit lands whole
        #[test]
        fn is_gone_when_it_leaves_destroyed_by_magic_jammed_the_next_hit_lands_whole() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "backrow": [ARGUSLAND] },
                "p2": { "hand": [MAGIC_JAMMED, FILLER], "field": [VANILLA] },
                "active": "p2",
            }));
            let argus = s.card(ARGUSLAND).clone();
            s.play(MAGIC_JAMMED, json!({ "targets": [{ "pick": "instance", "instanceId": argus.id }] }));
            s.expect_in_zone(&argus, "graveyard");
            s.attack(VANILLA, "hero");
            assert_eq!(hero_hits(&s, P1), vec![4]);
            s.expect_health(P1, 26);
        }

        /// R386 an Upgrade makes the divisor 3 (a 7 becomes 3); a Degrade never takes it below 2 (a 7 stays 4)
        #[test]
        fn r386_an_upgrade_makes_the_divisor_3_a_7_becomes_3_a_degrade_never_takes_it_below_2_a_7_stays_4() {
            let mut up = scenario(json!({
                "p1": { "hand": [FILLER], "backrow": [ARGUSLAND] },
                "p2": { "hand": [FILLER], "field": [POINTMASTER] },
                "active": "p2",
            }));
            step_param(up.card_mut(ARGUSLAND), "divisor", 1);
            up.attack(POINTMASTER, "hero");
            assert_eq!(hero_hits(&up, P1), vec![3]);

            let mut down = scenario(json!({
                "p1": { "hand": [FILLER], "backrow": [ARGUSLAND] },
                "p2": { "hand": [FILLER], "field": [POINTMASTER] },
                "active": "p2",
            }));
            step_param(down.card_mut(ARGUSLAND), "divisor", -1);
            down.attack(POINTMASTER, "hero");
            assert_eq!(hero_hits(&down, P1), vec![4]);
        }
    }

    /// `describe("radiant")`.
    mod radiant {
        use super::*;

        /// quarters a hit on your hero, rounded up: a 9 becomes 3, a 7 becomes 2
        #[test]
        fn quarters_a_hit_on_your_hero_rounded_up_a_9_becomes_3_a_7_becomes_2() {
            let nine = attack_on_guarded(MENACE, vec![radiant_argusland()], 30);
            assert_eq!(hero_hits(&nine, P1), vec![3]);
            let seven = attack_on_guarded(POINTMASTER, vec![radiant_argusland()], 30);
            assert_eq!(hero_hits(&seven, P1), vec![2]);
        }

        /// rounds up, so a 1 stays 1
        #[test]
        fn rounds_up_so_a_1_stays_1() {
            let s = attack_on_guarded(GARY, vec![radiant_argusland()], 30);
            assert_eq!(hero_hits(&s, P1), vec![1]);
        }

        /// §4.4 steps 2 and 3: after Armor (7 − 2 = 5, quartered to 2) and before a cap
        #[test]
        fn s4_4_steps_2_and_3_after_armor_7_2_5_quartered_to_2_and_before_a_cap() {
            let armored = attack_on_guarded(POINTMASTER, vec![radiant_argusland(), json!(GOING_LONG)], 30);
            assert_eq!(hero_hits(&armored, P1), vec![2]);
            let capped = attack_on_guarded(MENACE, vec![radiant_argusland(), json!(ANTI_ONESHOT)], 30);
            assert_eq!(hero_hits(&capped, P1), vec![3]);
        }

        /// R125 fatigue is quartered too, and R18 losing health still is not
        #[test]
        fn r125_fatigue_is_quartered_too_and_r18_losing_health_still_is_not() {
            let mut s = scenario(json!({
                "p1": { "hand": [STOCKPILE, FILLER], "backrow": [radiant_argusland()], "library": [BLOOD_BEAN], "health": 20 },
                "p2": { "hand": [FILLER] },
            }));
            s.play(STOCKPILE, json!({}));
            // The Bean is cast on draw (lose 5) and its draw is replaced; the next two are fatigue 1 and 2, each quartered to 1.
            assert_eq!(hero_hits(&s, P1), vec![1, 1]);
            s.expect_health(P1, 20 - 5 - 1 - 1 + 2);
        }

        /// damage to a Unit is untouched
        #[test]
        fn damage_to_a_unit_is_untouched() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [VANILLA], "backrow": [radiant_argusland()] },
                "p2": { "hand": [LUNAR, FILLER] },
                "active": "p2",
            }));
            let vanilla = s.unit(P1, 1).expect("Mr. Vanilla should be on the board");
            s.play(LUNAR, json!({ "targets": [{ "pick": "instance", "instanceId": vanilla.id }] }));
            s.expect_stats(&vanilla, json!({ "health": 1 }));
        }

        /// R386 its divisor is the declared number: a Degrade takes 4 to 3, so a 7 becomes 3
        #[test]
        fn r386_its_divisor_is_the_declared_number_a_degrade_takes_4_to_3_so_a_7_becomes_3() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "backrow": [radiant_argusland()] },
                "p2": { "hand": [FILLER], "field": [POINTMASTER] },
                "active": "p2",
            }));
            step_param(s.card_mut(ARGUSLAND), "divisor", -1);
            s.attack(POINTMASTER, "hero");
            assert_eq!(hero_hits(&s, P1), vec![3]);
        }
    }
}
