//! #68 Twisted Sorcerer (SPEC §8.3, §4.4, §10.9, R75, R81, R90).
//!
//! Base: "Cry: deal 4 damage to a target, 8 if your hero is below 10". Radiant: "8, or 16" (R275
//! doubled it from "6, or 12"). A cell that changes only numbers changes only those (§8
//! Conventions): the target, the threshold and "your hero" are kept.
//! R75 and §5.3: the source prints "Spell, Unit"; it is a Unit, so the script hangs off `cry`.
//! §8 Conventions: "target" is any legal unit or hero on either side; a bare `target` is a unit only
//! (`pick_kinds_for`, R90), so the heroes are named. R81: the pick travels in the `play` action, the
//! hook reads it as `{ of: "chosen" }` and this card opens no prompt. The hero's health is read when
//! the Cry runs (§10.9: a hook reads state, never writes). §4.4: the damage is one pipeline instance.
//! Fizzle: the heroes are in the filter, so the target set is never empty.
//! R195, the yellow glow: in hand the card glows exactly when its Cry would deal the high number;
//! `hero_is_low` is the one predicate the Cry and `condition_met` both read.

use jackioh_engine::effects::damage;
use jackioh_engine::prelude::*;

pub const ID: &str = "core-068";

/// §8 Conventions: every legal unit and hero on either side (R90: a bare `target` is units only).
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "hero"] }))]
}

/// "If your hero is below 10", strictly: at exactly the threshold the small number is dealt. The 10 is
/// the declared number `threshold` (R386), more being better (the high number comes sooner). §10.9: a
/// hook may read state to compute an effect's arguments; it never writes, and R195's `condition_met`
/// reads the same thing through this function.
fn hero_is_low(state: &GameState, controller: PlayerId, threshold: i32) -> bool {
    hero_of(state, controller).health < threshold
}

/// The read goes through `hero_of`, so no card file spells out `PlayerState` (BUILD M3-T1).
///
/// The declared number `damage` is dealt normally, `lowDamage` when the controller's hero is below the
/// threshold at resolution (R386): base "4 …, 8 if …", Radiant "8 …, 16" (R275), read off the running face.
fn sorcerer() -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            let low = hero_is_low(ctx.state, ctx.controller, param(&*ctx, "threshold"));
            let amount = if low { param(&*ctx, "lowDamage") } else { param(&*ctx, "damage") };
            vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": amount })))]
        })),
        // R195: hand only — the glow says playing it now deals `lowDamage`.
        condition_met: Some(condition_hook(|ctx| {
            ctx.zone == ConditionZone::Hand && hero_is_low(ctx.state, ctx.controller, param(&ctx, "threshold"))
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = sorcerer();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// #68 Twisted Sorcerer — SPEC §8.3, BUILD M4-T4: "4 damage, 8 when hero < 10 at resolution;
// radiant 8 / 16" (R275 doubled the radiant numbers from 6 / 12). R75 and §5.3: "Spell, Unit" is
// read as a Unit, so this is a Cry. R195's yellow glow: both answers of the hook are in
// `tests/cross/condition_active.rs` (README §5).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SOURCERER: &str = "core-068"; // Unit 5/5 → 10/10, cost 2.
    const SPONGE: &str = "core-019"; // Midrange Menace, 9/9 → 18/18, no Armor: a target that survives.

    /// R82: a turn whose only legal actions are ending it, conceding and offering a draw auto-ends, and
    /// `reduce` checks that after EVERY action, so a play that leaves no unit would hand the turn over
    /// under the assertion. Each scenario keeps one free 0-cost Spell in p1's hand, never played, so
    /// one legal action always remains.
    const ANCHOR: &str = "core-010"; // Rapid Replenish, Spell, cost 0 — always an affordable play.

    /// The refusal's word, checked by hand (no regex crate): it is lower-case.
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

    /// A declared pick of a unit on the board (R81: it travels in the play action, never a prompt).
    fn on_unit(s: &Scenario, player: PlayerId, lane: i32) -> Value {
        let Some(unit) = s.unit(player, lane) else {
            panic!("no unit for {player} in lane {lane}");
        };
        json!([{ "pick": "instance", "instanceId": unit.id }])
    }

    fn at_enemy_hero() -> Value {
        json!([{ "pick": "hero", "player": "p2" }])
    }

    mod twisted_sorcerer {
        use super::*;

        // The declaration (§8 Conventions, R81, R90)

        #[test]
        fn s8_conventions_declares_one_target_over_all_units_and_heroes_on_either_side_r90() {
            crate::register_all();
            let scripts = super::super::script();
            // R90: a bare `target` declaration means a unit only, so the heroes have to be named.
            let decl = json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit", "hero"] } }]);
            assert_eq!(serde_json::to_value(&scripts.base.targets).unwrap(), decl);
            assert_eq!(serde_json::to_value(&scripts.radiant.targets).unwrap(), decl);
            assert!(scripts.base.modes.is_empty());
        }

        // Base: 4, or 8 below 10 (§4.4, §8.3)

        #[test]
        fn s8_3_deals_4_to_a_chosen_enemy_unit_with_the_hero_at_full_health() {
            let mut s = board(json!({ "p1": { "hand": [SOURCERER] }, "p2": { "field": [SPONGE] } }));
            let targets = on_unit(&s, P2, 1);
            s.play(SOURCERER, json!({ "targets": targets }));
            s.expect_stats(SPONGE, json!({ "health": 5, "maxHealth": 9 }))
                .expect_stats(SOURCERER, json!({ "attack": 5, "health": 5 }));
        }

        #[test]
        fn s8_3_deals_4_to_a_chosen_hero_on_either_side() {
            let mut s = board(json!({ "p1": { "hand": [SOURCERER] } }));
            s.play(SOURCERER, json!({ "targets": at_enemy_hero() }));
            s.expect_health(P2, 26);

            let mut own = board(json!({ "p1": { "hand": [SOURCERER] } }));
            own.play(SOURCERER, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
            own.expect_health(P1, 26);
        }

        #[test]
        fn s8_3_deals_8_when_the_controllers_hero_is_below_10_at_resolution() {
            let mut s = board(json!({ "p1": { "hand": [SOURCERER], "health": 9 }, "p2": { "field": [SPONGE] } }));
            let targets = on_unit(&s, P2, 1);
            s.play(SOURCERER, json!({ "targets": targets }));
            s.expect_stats(SPONGE, json!({ "health": 1, "maxHealth": 9 }));
        }

        #[test]
        fn s8_3_below_10_is_strict_exactly_10_still_deals_4() {
            let mut s = board(json!({ "p1": { "hand": [SOURCERER], "health": 10 }, "p2": { "field": [SPONGE] } }));
            let targets = on_unit(&s, P2, 1);
            s.play(SOURCERER, json!({ "targets": targets }));
            s.expect_stats(SPONGE, json!({ "health": 5, "maxHealth": 9 }));
        }

        #[test]
        fn s8_3_the_threshold_reads_the_controllers_hero_not_the_opponents() {
            // A low opponent must not raise the amount: "your hero" is the controller's (§8 Conventions).
            let mut s = board(json!({
                "p1": { "hand": [SOURCERER], "health": 30 },
                "p2": { "field": [SPONGE], "health": 3 }
            }));
            let targets = on_unit(&s, P2, 1);
            s.play(SOURCERER, json!({ "targets": targets }));
            s.expect_stats(SPONGE, json!({ "health": 5, "maxHealth": 9 }));
        }

        #[test]
        fn s4_4_the_damage_is_one_pipeline_instance_so_armor_absorbs_it() {
            // #25 4-mana 7/7 prints Armor 7: §4.4 step 2 subtracts it, and R63 makes a 0 hit a non-event.
            let mut s = board(json!({ "p1": { "hand": [SOURCERER] }, "p2": { "field": ["core-025"] } }));
            let targets = on_unit(&s, P2, 1);
            s.play(SOURCERER, json!({ "targets": targets }));
            s.expect_stats("core-025", json!({ "health": 7, "maxHealth": 7 }));
        }

        #[test]
        fn r90_refuses_a_play_that_names_no_target_because_a_hero_is_always_legal() {
            // §8's "empty target set fizzles" cannot arise for this card: both heroes are in the filter, so
            // `legal_selections_for` is never empty and R90's "asks for what the board has" is still 1.
            let mut s = board(json!({ "p1": { "hand": [SOURCERER] } }));
            s.expect_refused_with(|s| s.play(SOURCERER, json!({})), TARGET_TEXT);
        }

        // Radiant: "8, or 16" (§8 Conventions — only the numbers move; R275)

        #[test]
        fn r275_the_radiant_face_is_10_10_and_deals_8_with_the_hero_at_full_health() {
            let mut s = board(json!({
                "p1": { "hand": [{ "def": SOURCERER, "radiant": true }] },
                "p2": { "field": [SPONGE] }
            }));
            let targets = on_unit(&s, P2, 1);
            s.play(SOURCERER, json!({ "targets": targets }));
            s.expect_stats(SOURCERER, json!({ "attack": 10, "health": 10, "maxHealth": 10 }))
                .expect_stats(SPONGE, json!({ "health": 1, "maxHealth": 9 }));
        }

        #[test]
        fn r275_the_radiant_face_deals_16_when_the_controllers_hero_is_below_10() {
            let mut s = board(json!({
                "p1": { "hand": [{ "def": SOURCERER, "radiant": true }], "health": 9 },
                "p2": { "field": [{ "def": SPONGE, "radiant": true }] },
            }));
            let targets = on_unit(&s, P2, 1);
            s.play(SOURCERER, json!({ "targets": targets }));
            s.expect_stats(SPONGE, json!({ "health": 2, "maxHealth": 18 }));
        }

        #[test]
        fn s8_3_the_radiant_threshold_is_the_same_strict_below_10_at_10_it_deals_8() {
            let mut s = board(json!({
                "p1": { "hand": [{ "def": SOURCERER, "radiant": true }], "health": 10 },
                "p2": { "field": [{ "def": SPONGE, "radiant": true }] },
            }));
            let targets = on_unit(&s, P2, 1);
            s.play(SOURCERER, json!({ "targets": targets }));
            s.expect_stats(SPONGE, json!({ "health": 10, "maxHealth": 18 }));
        }

        #[test]
        fn s8_3_the_radiant_threshold_reads_the_controllers_hero_not_the_opponents() {
            let mut s = board(json!({
                "p1": { "hand": [{ "def": SOURCERER, "radiant": true }], "health": 30 },
                "p2": { "field": [{ "def": SPONGE, "radiant": true }], "health": 3 },
            }));
            let targets = on_unit(&s, P2, 1);
            s.play(SOURCERER, json!({ "targets": targets }));
            s.expect_stats(SPONGE, json!({ "health": 10, "maxHealth": 18 }));
        }

        #[test]
        fn r275_the_radiant_face_still_hits_a_hero_for_8_and_for_16() {
            let mut high = board(json!({ "p1": { "hand": [{ "def": SOURCERER, "radiant": true }] } }));
            high.play(SOURCERER, json!({ "targets": at_enemy_hero() }));
            high.expect_health(P2, 22);

            let mut low = board(json!({ "p1": { "hand": [{ "def": SOURCERER, "radiant": true }], "health": 1 } }));
            low.play(SOURCERER, json!({ "targets": at_enemy_hero() }));
            low.expect_health(P2, 14);
        }

        #[test]
        fn s4_4_the_radiant_8_is_one_instance_too_armor_7_lets_1_through() {
            let mut s = board(json!({
                "p1": { "hand": [{ "def": SOURCERER, "radiant": true }] },
                "p2": { "field": ["core-025"] }
            }));
            let targets = on_unit(&s, P2, 1);
            s.play(SOURCERER, json!({ "targets": targets }));
            s.expect_stats("core-025", json!({ "health": 6, "maxHealth": 7 }));
        }
    }

    #[test]
    fn r386_an_upgrade_deals_5_or_10_and_a_degrade_3_or_6() {
        for (upgrade, key, health, dealt) in [
            (true, "damage", 30, 5),
            (true, "lowDamage", 5, 10),
            (false, "damage", 30, 3),
            (false, "lowDamage", 5, 6),
        ] {
            let mut s = board(json!({ "p1": { "hand": [SOURCERER], "health": health } }));
            if upgrade {
                crate::upgrade_number(&mut s, SOURCERER, key);
            } else {
                crate::degrade_number(&mut s, SOURCERER, key);
            }
            s.play(SOURCERER, json!({ "targets": at_enemy_hero() }));
            s.expect_health(P2, 30 - dealt);
        }
    }

    #[test]
    fn r386_an_upgrade_moves_the_threshold_to_12_and_a_degrade_to_8() {
        // At 10 health the printed "below 10" deals 4; an upgraded "below 12" deals 8, and at 9 health a
        // degraded "below 8" deals 4.
        for (upgrade, threshold, health, dealt) in [(true, 12, 10, 8), (false, 8, 9, 4)] {
            let mut s = board(json!({ "p1": { "hand": [SOURCERER], "health": health } }));
            let moved = if upgrade {
                crate::upgrade_number(&mut s, SOURCERER, "threshold")
            } else {
                crate::degrade_number(&mut s, SOURCERER, "threshold")
            };
            assert_eq!(moved, threshold);
            s.play(SOURCERER, json!({ "targets": at_enemy_hero() }));
            s.expect_health(P2, 30 - dealt);
        }
    }
}
