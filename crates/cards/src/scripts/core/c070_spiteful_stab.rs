//! #70 Spiteful Stab (SPEC §8.3, §3, §4.4, §10.9, R72, R81, R90): a Spell, cost 2 (3 until issue #3837).
//! Issue #4154 gave both faces Pierce (R346, R4154213): printed in the catalog and read off the Spell as
//! it resolves, and stated on the hit (`ignoreArmor`) as #44 True Strike's is, so §4.4 step 2 skips
//! Armor; through v0.6.6h it had none, which `script_v0_6_6h` keeps (R767000).
//!
//! Base: "Deal 2 damage to a target, +1 per full 5 health your hero is below 30, +1 per card in your
//! exile". Radiant (R275): 4, per full 3, +2 per card; the target and the 30 baseline are kept.
//! R72: "'Cards in exile' means your own exile pile; missing health counts from 30 even when the hero
//! has more" (§2's 30, BUILD §2): the controller's own pile (§3), never `counters.exiled` (R55).
//! R280: the `preview` is each face's text, its value the damage now, from public facts (§10.8).
//! §8 Conventions: a bare `target` is a unit only (R90), so the heroes are named. R81: the pick travels
//! in the `play` action. §4.4: one damage instance (R63). R662: glows in hand once a scaling term adds.

use jackioh_engine::effects::damage;
use jackioh_engine::prelude::*;

pub const ID: &str = "core-070";

/// §8 Conventions: every legal unit and hero on either side (R90: a bare `target` is units only).
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "hero"] }))]
}

/// One face's numbers: the flat damage, the health step, and what each card in your exile adds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Stab {
    base: i32,
    step: i32,
    per_exiled: i32,
}

/// The declared numbers `damage`, `healthStep` and `perExile` (R386), as the card stands.
fn numbers_of<C: ParamContext + ?Sized>(ctx: &C) -> Stab {
    Stab {
        base: param(ctx, "damage"),
        step: param(ctx, "healthStep"),
        per_exiled: param(ctx, "perExile"),
    }
}

/// R72: missing health floors at 0, and the exile is the controller's OWN pile.
fn stab_amount(state: &GameState, controller: PlayerId, face: Stab) -> i32 {
    let missing = (HERO_HEALTH - hero_of(state, controller).health).max(0);
    // `missing` is never negative and `step` positive, so `/` floors.
    face.base + missing / face.step + face.per_exiled * zone_count(state, controller, OffFieldZone::Exile)
}

/// `pierce`: whether the hit states Pierce (R346, since issue #4154).
fn spiteful_stab(face: FaceKind, pierce: bool) -> Script {
    let def = crate::card_def(ID);
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            let amount = stab_amount(ctx.state, ctx.controller, numbers_of(&*ctx));
            let mut hit = json!({ "to": { "of": "chosen" }, "amount": amount });
            if pierce {
                hit["ignoreArmor"] = json!(true);
            }
            vec![damage(json_as(hit))]
        })),
        // R280: the label is the face's damage line, its declared numbers as the card stands (R386): the
        // text's last line, under its Pierce (issue #4154).
        preview: Some(condition_hook(move |ctx| {
            vec![PreviewValue {
                label: fill_params(&def, face, params_view(ctx.state, ctx.self_).as_ref())
                    .rsplit('\n')
                    .next()
                    .unwrap_or_default()
                    .to_string(),
                value: stab_amount(ctx.state, ctx.controller, numbers_of(&ctx)),
                display: None,
                ids: None,
            }]
        })),
        // R662: the scaling has kicked in.
        condition_met: Some(condition_hook(|ctx| {
            let numbers = numbers_of(&ctx);
            ctx.zone == ConditionZone::Hand && stab_amount(ctx.state, ctx.controller, numbers) > numbers.base
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: spiteful_stab(FaceKind::Base, true),
        radiant: spiteful_stab(FaceKind::Radiant, true),
    }
}

/// R767000: the scripts through v0.6.6h, whose hit did not pierce (issue #4154). `KEPT_SCRIPTS` folds
/// those replays under it.
pub fn script_v0_6_6h() -> CardScripts {
    CardScripts {
        base: spiteful_stab(FaceKind::Base, false),
        radiant: spiteful_stab(FaceKind::Radiant, false),
    }
}

// #70 Spiteful Stab — SPEC §8.3, BUILD M4-T4. ; radiant 4 + floor(missing/3) + 2 × exile (R275 doubled the
// exile term).
#[cfg(test)]
mod tests {
    use crate::js;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const STAB: &str = "core-070"; // Spiteful Stab: a Spell
    const SPONGE: &str = "core-019"; // Midrange Menace: no Armor: a target that survives
    const FODDER: &str = "core-010"; // Rapid Replenish: filler for an exile pile

    /// R82: a turn left with only end, concede and offer-draw auto-ends after every action, so each
    /// scenario keeps this free Spell in p1's hand, never played, as one legal action on the turn.
    const ANCHOR: &str = "core-010"; // Rapid Replenish: a Spell, always an affordable play

    /// The refusal's word, lower-case (a hand check: no regex crate).
    const TARGET_TEXT: &str = "target";

    /// `scenario(opts)` with ANCHOR appended to p1's hand.
    fn board(opts: Value) -> Scenario {
        crate::register_all();
        let mut opts = opts;
        let mut p1 = opts.get("p1").cloned().unwrap_or_else(|| json!({}));
        let mut hand = p1.get("hand").and_then(Value::as_array).cloned().unwrap_or_default();
        hand.push(json!(ANCHOR));
        p1["hand"] = Value::Array(hand);
        opts["p1"] = p1;
        scenario(opts)
    }

    /// N cards in a pile fixture, so a test names the exile COUNT and not particular cards (R72).
    fn pile_of(count: usize) -> Vec<&'static str> {
        vec![FODDER; count]
    }

    mod spiteful_stab {
        use super::*;

        #[test]
        fn s8_conventions_declares_one_target_over_all_units_and_heroes_on_either_side_r90() {
            crate::register_all();
            let scripts = super::super::script();
            let decl = json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit", "hero"] } }]);
            assert_eq!(serde_json::to_value(&scripts.base.targets).unwrap(), decl);
            assert_eq!(serde_json::to_value(&scripts.radiant.targets).unwrap(), decl);
            assert!(scripts.base.modes.is_empty());
        }

        // Base: 2 + floor(missing / 5) + own exile count

        #[test]
        fn s8_3_deals_the_bare_2_at_30_health_with_an_empty_exile() {
            let mut s = board(json!({ "p1": { "hand": [STAB] } }));
            s.play(STAB, json!({ "targets": pick::hero("p2") }));
            s.expect_health(P2, 28);
        }

        #[test]
        fn r72_adds_one_per_full_5_health_missing_from_30_20_health_is_2_2_4() {
            let mut s = board(json!({ "p1": { "hand": [STAB], "health": 20 } }));
            s.play(STAB, json!({ "targets": pick::hero("p2") }));
            s.expect_health(P2, 26);
        }

        #[test]
        fn r72_the_division_floors_23_health_is_missing_7_so_2_floor_7_5_3() {
            let mut s = board(json!({ "p1": { "hand": [STAB], "health": 23 } }));
            s.play(STAB, json!({ "targets": pick::hero("p2") }));
            s.expect_health(P2, 27);
        }

        #[test]
        fn r72_missing_health_counts_from_30_even_when_the_hero_has_more_35_health_is_missing_0() {
            let mut s = board(json!({ "p1": { "hand": [STAB], "health": 35 } }));
            s.play(STAB, json!({ "targets": pick::hero("p2") }));
            s.expect_health(P2, 28);
        }

        #[test]
        fn s8_3_adds_one_per_card_in_the_exile_pile_3_exiled_is_2_3_5() {
            let mut s = board(json!({ "p1": { "hand": [STAB], "exile": pile_of(3) } }));
            assert_eq!(s.pile(P1, "exile").len(), 3);
            s.play(STAB, json!({ "targets": pick::hero("p2") }));
            s.expect_health(P2, 25);
        }

        #[test]
        fn r72_cards_in_exile_is_your_own_pile_the_opponents_exile_adds_nothing() {
            let mut s = board(json!({ "p1": { "hand": [STAB] }, "p2": { "exile": pile_of(4) } }));
            s.play(STAB, json!({ "targets": pick::hero("p2") }));
            s.expect_health(P2, 28);
        }

        #[test]
        fn s8_3_the_two_bonuses_stack_13_health_missing_17_and_2_exiled_is_2_3_2_7() {
            let mut s = board(json!({ "p1": { "hand": [STAB], "health": 13, "exile": pile_of(2) } }));
            s.play(STAB, json!({ "targets": pick::hero("p2") }));
            s.expect_health(P2, 23);
        }

        #[test]
        fn s8_conventions_the_target_may_be_a_unit_on_either_side() {
            let mut s = board(json!({
                "p1": { "hand": [STAB], "health": 20, "exile": pile_of(1) },
                "p2": { "field": [SPONGE] }
            }));
            let targets = pick::unit_at(&s, P2, 1);
            s.play(STAB, json!({ "targets": targets }));
            // 2 + floor(10/5) + 1 = 5.
            s.expect_stats(SPONGE, json!({ "health": 4, "maxHealth": 9 }));
        }

        /// R767000: through v0.6.6h the hit had no Pierce (issue #4154).
        #[test]
        fn r767000_s4_4_under_the_script_kept_for_v0_6_6h_armor_absorbs_the_one_instance_once() {
            // v0.6.6h printed no Pierce either, which the Spell's hit would read off it (R346).
            let unprinted = |def: &mut CardDef| {
                def.base.keywords.clear();
                def.radiant.keywords.clear();
            };
            crate::under_kept_as(STAB, super::super::script_v0_6_6h(), unprinted, || {
                // #25 prints Armor 7: 2 + floor(10/5) = 4 is absorbed whole, a non-event (R63), not four hits.
                let mut s = board(json!({ "p1": { "hand": [STAB], "health": 20 }, "p2": { "field": ["core-025"] } }));
                let targets = pick::unit_at(&s, P2, 1);
                s.play(STAB, json!({ "targets": targets }));
                s.expect_stats("core-025", json!({ "health": 7, "maxHealth": 7 }));
            });
        }

        #[test]
        fn r4154213_r346_both_faces_pierce_armor_with_the_one_instance() {
            for radiant in [false, true] {
                // #25 prints Armor 7: the 4 (Radiant 4 + floor(10/3) = 7) skips it (§4.4 step 2).
                let mut s = board(json!({
                    "p1": { "hand": [{ "def": STAB, "radiant": radiant }], "health": 20 },
                    "p2": { "field": ["core-025"] },
                }));
                let targets = pick::unit_at(&s, P2, 1);
                s.play(STAB, json!({ "targets": targets }));
                if radiant {
                    s.expect_in_zone("core-025", "graveyard");
                } else {
                    s.expect_stats("core-025", json!({ "health": 3, "maxHealth": 7 }));
                }
            }
        }

        #[test]
        fn r4154213_both_faces_print_pierce() {
            crate::register_all();
            let def = crate::card_def(STAB);
            assert!(has_keyword(&def.base.keywords, KeywordKind::Pierce));
            assert!(has_keyword(&def.radiant.keywords, KeywordKind::Pierce));
        }

        #[test]
        fn s10_5_the_spell_goes_to_the_graveyard_and_counts_as_played() {
            let mut s = board(json!({ "p1": { "hand": [STAB] } }));
            s.play(STAB, json!({ "targets": pick::hero("p2") }));
            s.expect_in_zone(STAB, "graveyard")
                .expect_mana(P1, 2)
                .expect_events(json!(["cardPlayed", "damage", "enteredGraveyard"]));
        }

        #[test]
        fn r90_refuses_a_play_that_names_no_target_because_a_hero_is_always_legal() {
            let mut s = board(json!({ "p1": { "hand": [STAB] } }));
            s.expect_refused_with(|s| s.play(STAB, json!({})), TARGET_TEXT);
        }

        // Radiant: "Deal 4 …, +1 per full 3 health …, +2 per card in your exile" (R275)

        #[test]
        fn s8_3_the_radiant_base_is_4_at_30_health_with_an_empty_exile() {
            let mut s = board(json!({ "p1": { "hand": [{ "def": STAB, "radiant": true }] } }));
            s.play(STAB, json!({ "targets": pick::hero("p2") }));
            s.expect_health(P2, 26);
        }

        #[test]
        fn s8_3_the_radiant_step_is_a_full_3_health_21_health_is_missing_9_so_4_3_7() {
            let mut s = board(json!({ "p1": { "hand": [{ "def": STAB, "radiant": true }], "health": 21 } }));
            s.play(STAB, json!({ "targets": pick::hero("p2") }));
            s.expect_health(P2, 23);
        }

        #[test]
        fn s8_3_the_radiant_division_floors_too_23_health_is_missing_7_so_4_floor_7_3_6() {
            let mut s = board(json!({ "p1": { "hand": [{ "def": STAB, "radiant": true }], "health": 23 } }));
            s.play(STAB, json!({ "targets": pick::hero("p2") }));
            s.expect_health(P2, 24);
        }

        #[test]
        fn s8_3_the_radiant_face_adds_two_per_card_in_your_exile_23_health_and_2_exiled_is_4_2_4_10() {
            let mut s = board(json!({
                "p1": { "hand": [{ "def": STAB, "radiant": true }], "health": 23, "exile": pile_of(2) }
            }));
            s.play(STAB, json!({ "targets": pick::hero("p2") }));
            s.expect_health(P2, 20);
        }

        #[test]
        fn r72_the_radiant_faces_exile_is_your_own_pile_too_the_opponents_exile_adds_nothing() {
            let mut s = board(json!({
                "p1": { "hand": [{ "def": STAB, "radiant": true }] },
                "p2": { "exile": pile_of(3) }
            }));
            s.play(STAB, json!({ "targets": pick::hero("p2") }));
            s.expect_health(P2, 26);
        }

        #[test]
        fn r72_the_radiant_face_measures_from_30_as_well_35_health_is_still_missing_0() {
            let mut s = board(json!({ "p1": { "hand": [{ "def": STAB, "radiant": true }], "health": 35 } }));
            s.play(STAB, json!({ "targets": pick::hero("p2") }));
            s.expect_health(P2, 26);
        }

        #[test]
        fn s8_conventions_the_radiant_face_still_targets_a_unit_on_either_side() {
            let mut s = board(json!({
                "p1": { "hand": [{ "def": STAB, "radiant": true }], "health": 24, "exile": pile_of(1) },
                "p2": { "field": [SPONGE] },
            }));
            let targets = pick::unit_at(&s, P2, 1);
            s.play(STAB, json!({ "targets": targets }));
            // 4 + floor(6/3) + 2 × 1 = 8.
            s.expect_stats(SPONGE, json!({ "health": 1, "maxHealth": 9 }));
        }
    }

    mod spiteful_stab_glows_once_its_scaling_adds_damage_r662 {
        use super::*;

        fn stab_glows(s: &Scenario) -> bool {
            let id = s.card(STAB).id.clone();
            hand_glows(s, &id, P1)
        }

        fn dealt(s: &Scenario) -> Vec<i64> {
            s.events()
                .iter()
                .map(js)
                .filter(|event| event["type"] == "damage" && event["targetId"] == "hero-p2")
                .filter_map(|event| event["amount"].as_i64())
                .collect()
        }

        #[test]
        fn r662_base_at_full_health_with_an_empty_exile_it_does_not_glow_and_deals_its_base_2() {
            let mut s = board(json!({ "p1": { "hand": [STAB] } }));
            assert!(!stab_glows(&s));
            s.play(STAB, json!({ "targets": pick::hero("p2") }));
            assert_eq!(dealt(&s), [2]);
        }

        #[test]
        fn r662_base_5_below_30_or_one_card_in_exile_and_it_glows_and_deals_3() {
            let mut hurt = board(json!({ "p1": { "hand": [STAB], "health": 25 } }));
            assert!(stab_glows(&hurt));
            hurt.play(STAB, json!({ "targets": pick::hero("p2") }));
            assert_eq!(dealt(&hurt), [3]);

            let exiled = board(json!({ "p1": { "hand": [STAB], "exile": pile_of(1) } }));
            assert!(stab_glows(&exiled));
            // 4 below 30 is not a full step of 5.
            assert!(!stab_glows(&board(json!({ "p1": { "hand": [STAB], "health": 26 } }))));
        }

        #[test]
        fn r662_radiant_a_full_step_is_3_so_27_glows_and_28_does_not() {
            let mut off = board(json!({ "p1": { "hand": [{ "def": STAB, "radiant": true }], "health": 28 } }));
            assert!(!stab_glows(&off));
            off.play(STAB, json!({ "targets": pick::hero("p2") }));
            assert_eq!(dealt(&off), [4]);

            let mut on = board(json!({ "p1": { "hand": [{ "def": STAB, "radiant": true }], "health": 27 } }));
            assert!(stab_glows(&on));
            on.play(STAB, json!({ "targets": pick::hero("p2") }));
            assert_eq!(dealt(&on), [5]);
        }

        /// R386: one of the three declared numbers stepped, then a stab from `health` with 1 card exiled.
        fn tuned_stab(upgrade: bool, key: &str, health: i32) -> i64 {
            let mut s = board(json!({ "p1": { "hand": [STAB], "health": health, "exile": pile_of(1) } }));
            if upgrade {
                crate::upgrade_number(&mut s, STAB, key);
            } else {
                crate::degrade_number(&mut s, STAB, key);
            }
            s.play(STAB, json!({ "targets": pick::hero("p2") }));
            dealt(&s)[0]
        }

        #[test]
        fn r386_each_number_moves_the_damage() {
            // From 20 health the printed stab is 2 + floor(10 / 5) + 1 = 5.
            assert_eq!(tuned_stab(true, "damage", 20), 6);
            assert_eq!(tuned_stab(false, "damage", 20), 4);
            assert_eq!(tuned_stab(true, "perExile", 20), 6);
            // From 22 it is 2 + floor(8 / 5) + 1 = 4, and a step of 4 makes it 2 + 2 + 1.
            assert_eq!(tuned_stab(true, "healthStep", 22), 5);
            // From 25 it is 2 + floor(5 / 5) + 1 = 4, and a step of 6 makes it 2 + 0 + 1.
            assert_eq!(tuned_stab(false, "healthStep", 25), 3);
        }

        #[test]
        fn r280_r386_the_preview_label_prints_the_moved_numbers() {
            let mut s = board(json!({ "p1": { "hand": [STAB] } }));
            crate::upgrade_number(&mut s, STAB, "damage");
            let card = s.card(STAB).id.clone();
            let HandView::Cards(hand) = s.view(P1).you.hand else {
                panic!("the viewer's own hand travels in full");
            };
            let shown = hand.into_iter().find(|entry| entry.instance_id == card).and_then(|entry| entry.preview);
            let first = shown.unwrap_or_default().into_iter().next().map(|entry| (entry.label, entry.value));
            assert_eq!(
                first,
                Some((
                    "Deal 3 damage, +1 per full 5 health your hero is below 30, +1 per card in your exile.".to_string(),
                    3
                ))
            );
        }
    }
}
