//! C #33 Joro (SPEC §8.6 row 33, §6.2 Replacement, §6.3 Redirect; R64, R97, R121,
//! R177, R275, R347, R394, R450, R651). Unit, cost 0, Legendary, 1/1 → 2/2.
//!   Base:    "While this is in your hand: When your opponent targets one of your Units with a Spell,
//!            summon this and make it the new target."
//!   Radiant: "Indestructible\nWhile this is in your hand: …" (the same).
//!   Engine:  "A hand trigger (§6.2 hand and deck triggers) on the replacement point "a friendly unit is
//!            targeted" (§6.2 Replacement): an enemy Spell played or cast whose declared target, or a
//!            Spell's prompt answer, picks one of your units (R651). Joro is summoned (the leftmost open
//!            zone, R64; with none, nothing happens), no Cry, summoning sick, and the pick is redirected
//!            to it (Redirect, §6.3). "Targeted" means chosen by a Spell: a Spell's declared target or a
//!            Spell's prompted pick; random picks, "all" effects, attacks and forced attacks (R121)
//!            summon nothing. One Joro answers one targeting; a Spell that names several of your units
//!            redirects the first. Joro answers from the hand only, a Yu-Gi-Oh hand trap (R394). Its
//!            Radiant face doubles to 2/2 and adds Indestructible, an endless decoy. Tunes: none."
//!
//! THE WHOLE CARD IS ONE REPLACEMENT, declared as data (B5 E5, `Script.replacements`): at "targeted",
//! standing in its controller's hand (`where: "hand"`), answering only a Spell's targeting
//! (`by: "spell"`, R651), it interposes — the engine summons it into its controller's leftmost empty,
//! unlocked, unreserved unit zone (R64), with no Cry and summoning sick, and moves the pick to it
//! (`redirected`). The engine asks only for the opponent's Spell choices of one of its controller's
//! units on the field: a Spell's declared target (§10.5 step 1) or a Spell's prompt answer; never an
//! attack, a Unit's Cry pick, an activation, a Trap's pick, a random pick, an "all" effect, a forced
//! attack (R121) or its own controller's pick. With no open zone it does nothing and stays in hand.
//! A Joro in a deck, a graveyard or on the field is not in the hand and answers nothing.
//! The first Joro in the hand answers; one answers one targeting.
//!
//! Nothing about it shows while it waits (R97, R177): a hand card is its owner's to read, and a
//! replacement is decided in the engine, not offered as a choice, so neither the opponent's view nor
//! their `legalActions` changes with a Joro in the hand.
//!
//! The Radiant face's Indestructible is the catalog's (2/2, holding R275's doubling), so the
//! script is the same; it survives the hit it draws, and R347 keeps Taunt off it.

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-033";

pub fn script() -> CardScripts {
    // `ReplacementDef` holds a `when` closure, so it is built in Rust, not through `json_as`.
    let base = Script {
        replacements: vec![ReplacementDef {
            id: "joro".into(),
            on: ReplacementMoment::Targeted,
            where_: Some(ReplacementWhere::Hand),
            when: None,
            instead: ReplacementInstead { interpose: Some(true), ..ReplacementInstead::default() },
            then: None,
            by: Some(ReplacementBy::Spell),
        }],
        ..Script::default()
    };

    // The same script: the Radiant face differs only in what the engine reads off the catalog (its
    // Indestructible).
    CardScripts { radiant: base.clone(), base }
}

// C #33 Joro — SPEC §8.6 row 33, BUILD M9 Classic row C 33: "From your hand only (R394): when your
// opponent targets one of your Units with a Spell (R651), by a declared target of a Spell played or
// cast (§10.5 step 1) or by a Spell's prompt answer, Joro is summoned into your leftmost open zone
// (no Cry, summoning sick) and the pick moves to it (`redirected`); an attack (§4.2 step 2), a Unit's
// Cry pick, an activation or a Trap's pick summons nothing; a random pick or an "all" effect targets
// nothing; a Spell naming several of your Units moves the first only; one Joro answers one
// targeting; your own picks never set it off; no open unit zone → nothing happens and Joro stays in
// hand; it never answers from the deck or the field; the opponent's view and `legalActions` carry no
// sign of Joro in your hand until it is summoned (R97); radiant 2/2: Indestructible, so it survives
// the redirected hit, with no Taunt (R347); no tuned numbers".
//
// "A friendly unit is targeted" is one replacement point the engine runs for an attack (§4.2 step 2,
// `combat.ts`), for a play's declared targets (§10.5 step 1, `playSteps.ts`) and for every target
// prompt (`targetingPoint.ts`); each carries the targeting card's type, and a `by: "spell"`
// replacement answers only a Spell's. The Unit-Cry prompt case uses C #76 Plague Bringer, the
// activation case C #20 The Power to Punish, the Trap case C #5 Tesla.
#[cfg(test)]
mod tests {
    use super::{script, ID};
    use jackioh_engine::testkit::*;

    const JORO: &str = "classic-033";
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt
    const SEVEN: &str = "core-025"; // (4) Unit 7/7
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const TIMMY: &str = "core-011"; // (1) Unit 3/3
    const ECLIPSE: &str = "core-035"; // (1) Spell: "Deal 3 damage to a target."
    const STAB: &str = "core-070"; // (3) Spell: "Deal 2 damage to a target, +1 per ..."
    const HIT_JOB: &str = "core-016"; // (3) Spell: destroy target Unit
    const BIG_FELINOR: &str = "core-043"; // (4) Unit: "Cry: Destroy all non-Felinor Units."
    const MOTHS: &str = "core-009"; // (2) Unit 1/14: "Start of turn: Every enemy Unit attacks this."
    const TWINSPELL: &str = "core-079"; // (2) Field Spell: "Your next Spell gains Echo +1."
    const TOXINS: &str = "classic-042"; // (2) Field Spell: "Activate: Place a Plague Counter on each of 2 random Units."
    const STOCKPILE: &str = "core-005"; // (1) Spell, a spare card (§2.5)
    const CRAWLER: &str = "classic-053"; // (1) Unit: "Cry: Place a Plague Counter on another permanent."
    const BRINGER: &str = "classic-076"; // (2) Unit: "Cry: Place 2 Plague Counters; draw 1."
    const PUNISH: &str = "classic-020"; // (2) Field Spell: "Activate: Choose one: Deal 2 damage; ..."
    const TESLA: &str = "classic-005"; // (2) Field Trap: "Activates when your opponent summons a Unit: Deal 4 damage to it."

    use crate::js;

    fn redirects(s: &Scenario) -> Vec<Value> {
        s.events().iter().map(js).filter(|event| event["type"] == "redirected").collect()
    }

    /// `opts[key]`, or `fallback` where the TS default (`??`) applies.
    fn or(opts: &Value, key: &str, fallback: Value) -> Value {
        if opts[key].is_null() { fallback } else { opts[key].clone() }
    }

    /// p2 (active) holds removal; p1 has a Vanilla in lane 1 and Joro in hand.
    /// `opts`: `radiantFace`, `myField`, `myHand`, `myLibrary`, as the TS helper's.
    fn under_attack(opts: Value) -> Scenario {
        let radiant_face = opts["radiantFace"] == true;
        scenario(json!({
            "active": "p2",
            "p1": {
                "hand": or(&opts, "myHand", json!([{ "def": JORO, "radiant": radiant_face }, STOCKPILE])),
                "field": or(&opts, "myField", json!([VANILLA])),
                "library": or(&opts, "myLibrary", json!([])),
            },
            "p2": { "hand": [STOCKPILE, HIT_JOB, ECLIPSE], "field": [SEVEN], "library": [STOCKPILE] },
        }))
    }

    mod c_33_joro {
        use super::*;

        #[test]
        fn is_one_replacement_at_targeted_from_the_hand_answering_only_a_spell_the_same_on_both_faces_with_no_numbers() {
            crate::register_all();
            assert!(crate::card_def(ID).params.is_none());
            let scripts = script();
            // TS `toEqual([{ id: "joro", on: "targeted", where: "hand", by: "spell", instead: { interpose: true } }])`:
            // `ReplacementDef` is not serialisable (its `when` is a closure), so field by field.
            for face in [&scripts.base, &scripts.radiant] {
                // TS `expect(radiant).toBe(base)`: the Radiant face is the same declaration.
                assert_eq!(face.replacements.len(), 1);
                let joro = &face.replacements[0];
                assert_eq!(joro.id, "joro");
                assert_eq!(joro.on, ReplacementMoment::Targeted);
                assert_eq!(joro.where_, Some(ReplacementWhere::Hand));
                assert_eq!(joro.by, Some(ReplacementBy::Spell));
                assert_eq!(js(&joro.instead), json!({ "interpose": true }));
                assert!(joro.when.is_none());
                assert!(joro.then.is_none());
            }
        }

        mod base {
            use super::*;

            #[test]
            fn r651_a_spells_declared_pick_of_your_unit_summons_joro_into_your_leftmost_open_zone_and_moves_to_it() {
                crate::register_all();
                let mut s = under_attack(json!({}));
                let joro = s.card(JORO).id.clone();
                let vanilla = s.card(VANILLA).id.clone();
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": vanilla }] }));
                let seen: Vec<Value> =
                    redirects(&s).iter().map(|event| json!([event["what"], event["fromId"], event["toId"]])).collect();
                assert_eq!(seen, vec![json!(["target", vanilla, joro])]);
                s.expect_in_zone(&joro, "graveyard");
                s.expect_in_zone(&vanilla, "field");
            }

            #[test]
            fn r651_an_attack_declared_at_one_of_your_units_summons_nothing() {
                crate::register_all();
                let mut s = under_attack(json!({}));
                let vanilla = s.card(VANILLA).id.clone();
                s.attack(SEVEN, &vanilla);
                assert!(redirects(&s).is_empty());
                s.expect_in_zone(JORO, "hand");
                s.expect_in_zone(&vanilla, "graveyard");
            }

            #[test]
            fn r64_the_leftmost_open_zone() {
                crate::register_all();
                let mut s = under_attack(json!({ "myField": [VANILLA, TIMMY] }));
                let joro = s.card(JORO).id.clone();
                let timmy = s.card(TIMMY).id.clone();
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": timmy }] }));
                assert_eq!(redirects(&s).len(), 1);
                // Joro stood in lane 3 when Hit Job destroyed it.
                let summoned = s
                    .events()
                    .iter()
                    .map(js)
                    .find(|event| event["type"] == "summoned" && event["instanceId"] == joro)
                    .expect("Joro's summoned event");
                assert_eq!(summoned["row"], "units");
                assert_eq!(summoned["lane"], 3);
            }

            #[test]
            fn no_open_unit_zone_nothing_happens_and_joro_stays_in_hand() {
                crate::register_all();
                let mut s = under_attack(json!({ "myField": [VANILLA, TIMMY, VANILLA, TIMMY, MENACE] }));
                let joro = s.card(JORO).id.clone();
                let menace = s.card(MENACE).id.clone();
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": menace }] }));
                assert!(redirects(&s).is_empty());
                s.expect_in_zone(&joro, "hand");
            }

            #[test]
            fn an_attack_on_your_hero_targets_no_unit() {
                crate::register_all();
                let mut s = under_attack(json!({ "myField": [] }));
                s.attack(SEVEN, "hero");
                assert!(redirects(&s).is_empty());
                s.expect_in_zone(JORO, "hand");
            }

            #[test]
            fn one_joro_answers_one_targeting_with_two_in_hand_one_spell_summons_one() {
                crate::register_all();
                let mut s = under_attack(json!({ "myHand": [JORO, JORO, STOCKPILE] }));
                let vanilla = s.card(VANILLA).id.clone();
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": vanilla }] }));
                assert_eq!(redirects(&s).len(), 1);
                assert_eq!(s.hand(PlayerId::P1).iter().filter(|card| card.def_id == JORO).count(), 1);
            }

            #[test]
            fn r394_it_never_answers_from_the_deck() {
                crate::register_all();
                let mut s = under_attack(json!({ "myHand": [STOCKPILE], "myLibrary": [JORO] }));
                let vanilla = s.card(VANILLA).id.clone();
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": vanilla }] }));
                assert!(redirects(&s).is_empty());
                s.expect_in_zone(JORO, "library");
            }

            #[test]
            fn r394_it_never_answers_from_the_field() {
                crate::register_all();
                let mut s = under_attack(json!({ "myHand": [STOCKPILE], "myField": [VANILLA, JORO] }));
                let vanilla = s.card(VANILLA).id.clone();
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": vanilla }] }));
                assert!(redirects(&s).is_empty());
                s.expect_in_zone(&vanilla, "graveyard");
            }

            #[test]
            fn r121_a_forced_attack_is_the_effects_not_a_targeting_moths_to_the_flame_draws_no_joro() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [JORO, STOCKPILE], "field": [MOTHS], "library": [STOCKPILE] },
                    "p2": { "hand": [STOCKPILE], "field": [VANILLA], "library": [STOCKPILE] },
                }));
                s.end_turn(); // p1's start of turn: p2's Vanilla is forced to attack the Moths
                assert!(s.events().iter().map(js).any(|event| event["type"] == "attackDeclared"));
                assert!(redirects(&s).is_empty());
                s.expect_in_zone(JORO, "hand");
            }

            #[test]
            fn an_all_effect_targets_nothing_big_felinors_sweep_draws_no_joro() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [JORO, STOCKPILE], "field": [VANILLA] },
                    "p2": { "hand": [BIG_FELINOR, STOCKPILE] },
                }));
                s.play(BIG_FELINOR, json!({}));
                assert!(redirects(&s).is_empty());
                s.expect_in_zone(JORO, "hand");
                s.expect_in_zone(VANILLA, "graveyard");
            }

            #[test]
            fn your_own_spells_never_set_it_off() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [JORO, ECLIPSE, STOCKPILE], "field": [MENACE] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                let menace = s.card(MENACE).id.clone();
                s.play(ECLIPSE, json!({ "targets": [{ "pick": "instance", "instanceId": menace }] }));
                assert!(redirects(&s).is_empty());
                s.expect_in_zone(JORO, "hand");
                s.expect_stats(MENACE, json!({ "health": 6 }));
            }

            #[test]
            fn r97_r177_the_opponents_view_and_legal_actions_carry_no_sign_of_joro_in_your_hand() {
                crate::register_all();
                let with_joro = under_attack(json!({ "myHand": [JORO, STOCKPILE] }));
                let with_vanilla = under_attack(json!({ "myHand": [VANILLA, STOCKPILE] }));
                assert_eq!(with_joro.view(PlayerId::P2), with_vanilla.view(PlayerId::P2));
                assert_eq!(
                    legal_actions(with_joro.state(), PlayerId::P2),
                    legal_actions(with_vanilla.state(), PlayerId::P2),
                );
            }

            #[test]
            fn r651_a_spells_prompt_answer_pick_of_your_unit_moves_to_joro_an_echo_repeats_fresh_target() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [JORO, STOCKPILE], "field": [MENACE] },
                    "p2": { "hand": [TWINSPELL, ECLIPSE, STOCKPILE] },
                }));
                let joro = s.card(JORO).id.clone();
                let menace = s.card(MENACE).id.clone();
                s.play(TWINSPELL, json!({}));
                s.play(ECLIPSE, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
                assert_eq!(js(&s.state().pending)["kind"], "target");
                s.answer(json!([{ "pick": "instance", "instanceId": menace }]));
                let seen: Vec<Value> = redirects(&s).iter().map(|event| json!([event["what"], event["toId"]])).collect();
                assert_eq!(seen, vec![json!(["target", joro])]);
                s.expect_stats(&menace, json!({ "health": 9 }));
            }

            #[test]
            fn a_spell_naming_several_of_your_units_moves_the_first_only() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [JORO, STOCKPILE], "field": [MENACE, SEVEN] },
                    "p2": { "hand": [ECLIPSE, STAB, STOCKPILE], "mana": 10 },
                }));
                let ingredients = json!([js(s.card(ECLIPSE)), js(s.card(STAB))]);
                let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
                let mut events = Vec::new();
                let fused = {
                    let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
                    subsystems::fuse(&mut sink, json_as(json!({ "ingredients": ingredients, "toHand": "p2" })))
                };
                let fused = fused.expect("the crafted two-target Spell").id;
                let menace = s.card(MENACE).id.clone();
                let seven = s.card(SEVEN).id.clone();
                let joro = s.card(JORO).id.clone();
                s.play(
                    &fused,
                    json!({
                        "targets": [
                            { "pick": "instance", "instanceId": menace },
                            { "pick": "instance", "instanceId": seven },
                        ],
                    }),
                );
                let seen: Vec<Value> = redirects(&s).iter().map(|event| json!([event["fromId"], event["toId"]])).collect();
                assert_eq!(seen, vec![json!([menace, joro])]);
            }

            #[test]
            fn r651_a_units_declared_cry_pick_draws_no_joro_plague_crawlers_token() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [JORO, STOCKPILE], "field": [MENACE] },
                    "p2": { "hand": [CRAWLER, STOCKPILE] },
                }));
                let menace = s.card(MENACE).id.clone();
                s.play(CRAWLER, json!({ "targets": [{ "pick": "instance", "instanceId": menace }] }));
                assert!(redirects(&s).is_empty());
                s.expect_in_zone(JORO, "hand");
                assert_eq!(s.unit(PlayerId::P1, 1).and_then(|unit| unit.counters.plague), Some(1));
            }

            #[test]
            fn r651_a_units_prompt_pick_draws_no_joro_plague_bringers_placements() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [JORO, STOCKPILE], "field": [MENACE] },
                    "p2": { "hand": [BRINGER, STOCKPILE] },
                }));
                let menace = s.card(MENACE).id.clone();
                s.play(BRINGER, json!({}));
                // R689: one `target` prompt names the single permanent both placements land on.
                assert_eq!(js(&s.state().pending)["kind"], "target");
                s.answer(json!([{ "pick": "instance", "instanceId": menace }]));
                assert!(redirects(&s).is_empty());
                s.expect_in_zone(JORO, "hand");
                assert_eq!(s.unit(PlayerId::P1, 1).and_then(|unit| unit.counters.plague), Some(2));
            }

            #[test]
            fn r651_an_activations_pick_draws_no_joro_the_power_to_punishs_damage() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [JORO, STOCKPILE], "field": [MENACE] },
                    "p2": { "hand": [STOCKPILE], "backrow": [PUNISH] },
                }));
                let menace = s.card(MENACE).id.clone();
                s.activate(
                    PUNISH,
                    json!({
                        "modes": ["deal damage"],
                        "targets": [{ "pick": "instance", "instanceId": menace }],
                    }),
                );
                assert!(redirects(&s).is_empty());
                s.expect_in_zone(JORO, "hand");
                assert_eq!(s.stats(&menace).health, 7);
            }

            #[test]
            fn r651_a_traps_firing_draws_no_joro_tesla_answers_the_summon_itself() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p1",
                    "p1": { "hand": [JORO, VANILLA, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE], "backrow": [{ "def": TESLA, "faceUp": false, "lane": 3 }] },
                }));
                let vanilla = s.card(VANILLA).id.clone();
                s.play(VANILLA, json!({}));
                assert!(redirects(&s).is_empty());
                s.expect_in_zone(JORO, "hand");
                // Tesla fired at the summoned Vanilla for 4.
                s.expect_in_zone(&vanilla, "graveyard");
            }

            #[test]
            fn a_random_pick_targets_nothing_c_42s_random_plague_counters_draw_no_joro() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [JORO, STOCKPILE], "field": [MENACE] },
                    "p2": { "hand": [STOCKPILE], "backrow": [TOXINS] },
                }));
                s.activate(TOXINS, json!({}));
                assert!(redirects(&s).is_empty());
                s.expect_in_zone(JORO, "hand");
                assert_eq!(s.unit(PlayerId::P1, 1).and_then(|unit| unit.counters.plague), Some(1));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r275_indestructible_2_2_it_survives_the_redirected_spell() {
                crate::register_all();
                let mut s = under_attack(json!({ "radiantFace": true }));
                let joro = s.card(JORO).id.clone();
                let vanilla = s.card(VANILLA).id.clone();
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": vanilla }] }));
                assert_eq!(redirects(&s).len(), 1);
                s.expect_in_zone(&joro, "field");
                s.expect_stats(&joro, json!({ "attack": 2, "health": 2, "maxHealth": 2 }));
                assert!(s.stats(&joro).keywords.iter().any(|keyword| js(keyword)["kind"] == "Indestructible"));
                // Summoned this turn: summoning sick.
                assert_eq!(s.card(&joro).summoned_turn, Some(s.state().turn));
            }

            #[test]
            fn r347_no_taunt_even_in_defense_position() {
                crate::register_all();
                let mut s = under_attack(json!({ "radiantFace": true }));
                let joro = s.card(JORO).id.clone();
                let vanilla = s.card(VANILLA).id.clone();
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": vanilla }] }));
                assert!(!s.stats(&joro).keywords.iter().any(|keyword| js(keyword)["kind"] == "Taunt"));
            }

            #[test]
            fn an_endless_decoy_it_answers_from_the_hand_only_and_once_on_the_field_it_answers_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [STOCKPILE], "field": [VANILLA, { "def": JORO, "radiant": true }] },
                    "p2": { "hand": [HIT_JOB, STOCKPILE], "field": [SEVEN], "library": [STOCKPILE] },
                }));
                let vanilla = s.card(VANILLA).id.clone();
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": vanilla }] }));
                assert!(redirects(&s).is_empty());
                s.expect_in_zone(&vanilla, "graveyard");
            }
        }
    }
}
