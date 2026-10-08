//! #70 Spiteful Stab (SPEC §8.3, §3, §4.4, §10.9, R72, R81, R90).
//!
//! Base face: "Deal 2 damage to a target, +1 per full 5 health your hero is below 30, +1 per card in
//! your exile". Radiant face (R275): "Deal 4 damage to a target, +1 per full 3 health your hero is
//! below 30, +2 per card in your exile" — the base amount, the health step and the exile term move;
//! the target and the 30 baseline are kept.
//!
//! R72 is the whole of the arithmetic: "'Cards in exile' means your own exile pile; missing health
//! counts from 30 even when the hero has more". So
//!     missing = max(0, HERO_HEALTH - health)                           // floors at 0 above 30
//!     amount  = base + floor(missing / step) + perExiled * exileCount  // "per FULL n"
//! with `base`/`step`/`perExiled` of 2/5/1 on the base face and 4/3/2 on the radiant one. HERO_HEALTH
//! is the §2 starting health in `config.rs`, which is the 30 both R72 and the card text mean; the
//! engine never hard-codes a rules constant (BUILD §2).
//!
//! R280: the whole formula is the card's `preview` label — each face's text as printed — and its
//! value the damage `stab_amount` comes to now, the same function the Cry deals. It reads the
//! controller's hero health and exile count, both public (§10.8).
//!
//! §3's zone table puts it plainly: the exile count "feeds Echoes of the Forgotten and Spiteful
//! Stab". It is the controller's own pile — the opponent's exile is not counted, and neither is the
//! game-wide `counters.exiled`, which counts exiles by both players (R55's counter, for #100).
//!
//! §8's Conventions: "'target' means the player picks at play time from all legal units and heroes on
//! either side unless narrowed", and a bare `target` declaration means a unit only (`pick_kinds_for` in
//! `play_choices.rs`, R90), so the heroes are named. R81: the pick travels in the `play` action, so
//! the hook reads `{ of: "chosen" }` and nothing pauses. The amount is computed in the hook, at
//! resolution, which §10.9 allows: a hook may READ state to build an effect's arguments, never write.
//!
//! §4.4: one damage instance through the pipeline, however large it grew — Armor, Divine Shield, the
//! anti-oneshot cap and Indestructible all still apply, and R63's zero rule applies if it is reduced
//! to nothing.
//!
//! THE GLOW (R662). In hand it lights up when either scaling term adds something: the amount
//! `stab_amount` comes to now is more than the face's base damage, so a full step of missing health
//! (5, Radiant 3) or a card in its controller's exile. Built on `stab_amount`, the function the Cry
//! deals with, so the two cannot disagree.

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

/// §8: "Deal 2 …, +1 per full 5 health …, +1 per card …"; radiant "Deal 4 …, per full 3 …, +2 per
/// card …": the declared numbers `damage`, `healthStep` and `perExile` (R386), as the card stands.
fn numbers_of<C: ParamContext + ?Sized>(ctx: &C) -> Stab {
    Stab {
        base: param(ctx, "damage"),
        step: param(ctx, "healthStep"),
        per_exiled: param(ctx, "perExile"),
    }
}

/// §10.9: a hook may read state to compute an effect's arguments; it never writes. Both reads go
/// through the engine's read-only board surface (`hero_of`, `zone_count` in engine/src/query.rs), so
/// this file names the two facts R72 needs rather than the fields they live in (BUILD M3-T1).
/// R72: missing health is measured from 30 even when the hero is above it, so it floors at 0, and the
/// exile is the controller's OWN pile, never the opponent's and never the game-wide counter.
fn stab_amount(state: &GameState, controller: PlayerId, face: Stab) -> i32 {
    let missing = (HERO_HEALTH - hero_of(state, controller).health).max(0);
    // `missing` is never negative and `step` positive, so `/` is TS's `Math.floor(missing / step)`.
    face.base + missing / face.step + face.per_exiled * zone_count(state, controller, OffFieldZone::Exile)
}

fn spiteful_stab(face: FaceKind) -> Script {
    let def = crate::card_def(ID);
    Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            let amount = stab_amount(ctx.state, ctx.controller, numbers_of(&*ctx));
            vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": amount })))]
        })),
        // R280: the label is the face's whole text, the formula as printed (TS `def[face].text`), with
        // its declared numbers as the card stands (R386).
        preview: Some(condition_hook(move |ctx| {
            vec![PreviewValue {
                label: fill_params(&def, face, params_view(ctx.state, ctx.self_).as_ref()),
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
        base: spiteful_stab(FaceKind::Base),
        radiant: spiteful_stab(FaceKind::Radiant),
    }
}

// #70 Spiteful Stab — SPEC §8.3, BUILD M4-T4: "2 + floor(missing/5) + exile count"; radiant
// 4 + floor(missing/3) + 2 × exile (R275 doubled the exile term).
//
// §8.3's row: "Deal 2 damage to a target, +1 per full 5 health your hero is below 30, +1 per card
// in your exile" → "Deal 4 damage to a target, +1 per full 3 health your hero is below 30, +2 per
// card in your exile", Engine cell "`missing = max(0, 30 − health)`, floor division; your own exile
// (R72)". The damage it would deal now, its R280 `preview`, is proved in test/preview.test.ts.
//
// R72 is the whole arithmetic: "'Cards in exile' means your own exile pile; missing health counts
// from 30 even when the hero has more". §3's zone table says the same: the exile count "feeds
// Echoes of the Forgotten and Spiteful Stab". §8's Conventions make the target any legal unit or
// hero on either side, and R81 makes it a play-time pick.
//
// R662's yellow glow (`conditionMet`): in hand once either scaling term adds damage, both faces, checked
// against what it then deals, at the end of this file.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const STAB: &str = "core-070"; // Spell, cost 3.
    const SPONGE: &str = "core-019"; // Midrange Menace, 9/9, no Armor: a target that survives.
    const FODDER: &str = "core-010"; // Rapid Replenish, a Spell: filler for an exile pile.

    /// R82: a turn whose only legal actions are ending it, conceding and offering a draw auto-ends by
    /// itself, and `reduce` runs that check after EVERY action — so a play that empties the hand and
    /// leaves no unit hands the turn over: the opponent draws (taking fatigue on an empty library),
    /// start-of-turn triggers fire, and the numbers under test move underneath the assertion. Every
    /// scenario below therefore keeps one free 0-cost Spell in p1's hand. It is never played; it only
    /// keeps one legal action on the turn. (Reported as a harness gap: `scenario` could hold the turn
    /// open by itself.)
    const ANCHOR: &str = "core-010"; // Rapid Replenish, Spell, cost 0 — always an affordable play.

    /// TS `toThrow(/target/i)`, as a hand check (no regex crate): the refusal's word is lower-case.
    const TARGET_TEXT: &str = "target";

    /// `scenario(opts)` with ANCHOR appended to p1's hand, the shipped cards registered first.
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

    fn at_enemy_hero() -> Value {
        json!([{ "pick": "hero", "player": "p2" }])
    }

    /// N cards in a pile fixture, so a test names the exile COUNT and not particular cards (R72).
    fn pile_of(count: usize) -> Vec<&'static str> {
        vec![FODDER; count]
    }

    fn on_unit(s: &Scenario, player: PlayerId, lane: i32) -> Value {
        let Some(unit) = s.unit(player, lane) else {
            panic!("no unit for {player} in lane {lane}");
        };
        json!([{ "pick": "instance", "instanceId": unit.id }])
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

        // -------------------------------------------------------------------------------------------
        // Base: 2 + floor(missing / 5) + own exile count
        // -------------------------------------------------------------------------------------------

        #[test]
        fn s8_3_deals_the_bare_2_at_30_health_with_an_empty_exile() {
            let mut s = board(json!({ "p1": { "hand": [STAB] } }));
            s.play(STAB, json!({ "targets": at_enemy_hero() }));
            s.expect_health(P2, 28);
        }

        #[test]
        fn r72_adds_one_per_full_5_health_missing_from_30_20_health_is_2_2_4() {
            let mut s = board(json!({ "p1": { "hand": [STAB], "health": 20 } }));
            s.play(STAB, json!({ "targets": at_enemy_hero() }));
            s.expect_health(P2, 26);
        }

        #[test]
        fn r72_the_division_floors_23_health_is_missing_7_so_2_floor_7_5_3() {
            let mut s = board(json!({ "p1": { "hand": [STAB], "health": 23 } }));
            s.play(STAB, json!({ "targets": at_enemy_hero() }));
            s.expect_health(P2, 27);
        }

        #[test]
        fn r72_missing_health_counts_from_30_even_when_the_hero_has_more_35_health_is_missing_0() {
            let mut s = board(json!({ "p1": { "hand": [STAB], "health": 35 } }));
            s.play(STAB, json!({ "targets": at_enemy_hero() }));
            s.expect_health(P2, 28);
        }

        #[test]
        fn s8_3_adds_one_per_card_in_the_exile_pile_3_exiled_is_2_3_5() {
            let mut s = board(json!({ "p1": { "hand": [STAB], "exile": pile_of(3) } }));
            assert_eq!(s.pile(P1, "exile").len(), 3);
            s.play(STAB, json!({ "targets": at_enemy_hero() }));
            s.expect_health(P2, 25);
        }

        #[test]
        fn r72_cards_in_exile_is_your_own_pile_the_opponents_exile_adds_nothing() {
            let mut s = board(json!({ "p1": { "hand": [STAB] }, "p2": { "exile": pile_of(4) } }));
            s.play(STAB, json!({ "targets": at_enemy_hero() }));
            s.expect_health(P2, 28);
        }

        #[test]
        fn s8_3_the_two_bonuses_stack_13_health_missing_17_and_2_exiled_is_2_3_2_7() {
            let mut s = board(json!({ "p1": { "hand": [STAB], "health": 13, "exile": pile_of(2) } }));
            s.play(STAB, json!({ "targets": at_enemy_hero() }));
            s.expect_health(P2, 23);
        }

        #[test]
        fn s8_conventions_the_target_may_be_a_unit_on_either_side() {
            let mut s = board(json!({
                "p1": { "hand": [STAB], "health": 20, "exile": pile_of(1) },
                "p2": { "field": [SPONGE] }
            }));
            let targets = on_unit(&s, P2, 1);
            s.play(STAB, json!({ "targets": targets }));
            // 2 + floor(10/5) + 1 = 5.
            s.expect_stats(SPONGE, json!({ "health": 4, "maxHealth": 9 }));
        }

        #[test]
        fn s4_4_the_whole_amount_is_one_pipeline_instance_so_armor_absorbs_it_once() {
            // #25 4-mana 7/7 prints Armor 7; 2 + floor(10/5) = 4 is fully absorbed and R63 makes it a
            // non-event rather than four separate hits.
            let mut s = board(json!({ "p1": { "hand": [STAB], "health": 20 }, "p2": { "field": ["core-025"] } }));
            let targets = on_unit(&s, P2, 1);
            s.play(STAB, json!({ "targets": targets }));
            s.expect_stats("core-025", json!({ "health": 7, "maxHealth": 7 }));
        }

        #[test]
        fn s10_5_the_spell_goes_to_the_graveyard_and_counts_as_played() {
            let mut s = board(json!({ "p1": { "hand": [STAB] } }));
            s.play(STAB, json!({ "targets": at_enemy_hero() }));
            s.expect_in_zone(STAB, "graveyard")
                .expect_mana(P1, 1)
                .expect_events(json!(["cardPlayed", "damage", "enteredGraveyard"]));
        }

        #[test]
        fn r90_refuses_a_play_that_names_no_target_because_a_hero_is_always_legal() {
            let mut s = board(json!({ "p1": { "hand": [STAB] } }));
            s.expect_refused_with(|s| s.play(STAB, json!({})), TARGET_TEXT);
        }

        // -------------------------------------------------------------------------------------------
        // Radiant: "Deal 4 …, +1 per full 3 health …, +2 per card in your exile" (R275)
        // -------------------------------------------------------------------------------------------

        #[test]
        fn s8_3_the_radiant_base_is_4_at_30_health_with_an_empty_exile() {
            let mut s = board(json!({ "p1": { "hand": [{ "def": STAB, "radiant": true }] } }));
            s.play(STAB, json!({ "targets": at_enemy_hero() }));
            s.expect_health(P2, 26);
        }

        #[test]
        fn s8_3_the_radiant_step_is_a_full_3_health_21_health_is_missing_9_so_4_3_7() {
            let mut s = board(json!({ "p1": { "hand": [{ "def": STAB, "radiant": true }], "health": 21 } }));
            s.play(STAB, json!({ "targets": at_enemy_hero() }));
            s.expect_health(P2, 23);
        }

        #[test]
        fn s8_3_the_radiant_division_floors_too_23_health_is_missing_7_so_4_floor_7_3_6() {
            let mut s = board(json!({ "p1": { "hand": [{ "def": STAB, "radiant": true }], "health": 23 } }));
            s.play(STAB, json!({ "targets": at_enemy_hero() }));
            s.expect_health(P2, 24);
        }

        #[test]
        fn s8_3_the_radiant_face_adds_two_per_card_in_your_exile_23_health_and_2_exiled_is_4_2_4_10() {
            let mut s = board(json!({
                "p1": { "hand": [{ "def": STAB, "radiant": true }], "health": 23, "exile": pile_of(2) }
            }));
            s.play(STAB, json!({ "targets": at_enemy_hero() }));
            s.expect_health(P2, 20);
        }

        #[test]
        fn r72_the_radiant_faces_exile_is_your_own_pile_too_the_opponents_exile_adds_nothing() {
            let mut s = board(json!({
                "p1": { "hand": [{ "def": STAB, "radiant": true }] },
                "p2": { "exile": pile_of(3) }
            }));
            s.play(STAB, json!({ "targets": at_enemy_hero() }));
            s.expect_health(P2, 26);
        }

        #[test]
        fn r72_the_radiant_face_measures_from_30_as_well_35_health_is_still_missing_0() {
            let mut s = board(json!({ "p1": { "hand": [{ "def": STAB, "radiant": true }], "health": 35 } }));
            s.play(STAB, json!({ "targets": at_enemy_hero() }));
            s.expect_health(P2, 26);
        }

        #[test]
        fn s8_conventions_the_radiant_face_still_targets_a_unit_on_either_side() {
            let mut s = board(json!({
                "p1": { "hand": [{ "def": STAB, "radiant": true }], "health": 24, "exile": pile_of(1) },
                "p2": { "field": [SPONGE] },
            }));
            let targets = on_unit(&s, P2, 1);
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
                .map(|event| serde_json::to_value(event).expect("an event serialises"))
                .filter(|event| event["type"] == "damage" && event["targetId"] == "hero-p2")
                .filter_map(|event| event["amount"].as_i64())
                .collect()
        }

        #[test]
        fn r662_base_at_full_health_with_an_empty_exile_it_does_not_glow_and_deals_its_base_2() {
            let mut s = board(json!({ "p1": { "hand": [STAB] } }));
            assert!(!stab_glows(&s));
            s.play(STAB, json!({ "targets": at_enemy_hero() }));
            assert_eq!(dealt(&s), [2]);
        }

        #[test]
        fn r662_base_5_below_30_or_one_card_in_exile_and_it_glows_and_deals_3() {
            let mut hurt = board(json!({ "p1": { "hand": [STAB], "health": 25 } }));
            assert!(stab_glows(&hurt));
            hurt.play(STAB, json!({ "targets": at_enemy_hero() }));
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
            off.play(STAB, json!({ "targets": at_enemy_hero() }));
            assert_eq!(dealt(&off), [4]);

            let mut on = board(json!({ "p1": { "hand": [{ "def": STAB, "radiant": true }], "health": 27 } }));
            assert!(stab_glows(&on));
            on.play(STAB, json!({ "targets": at_enemy_hero() }));
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
            s.play(STAB, json!({ "targets": at_enemy_hero() }));
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
