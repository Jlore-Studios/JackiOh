//! C+ #32.1 Execute (SPEC §8.7 row 32.1; §4, §6.3 Destroy, R46, R59, R81). (1) Spell token, printed Epic.
//!   Base:    "Destroy a damaged Unit."
//!   Radiant: "Destroy every damaged enemy Unit."
//!
//! The base face's target is one damaged Unit on either side; with none the Spell fizzles and still
//! counts as played (§8's Conventions). The Radiant face names no target: every damaged enemy Unit it
//! may affect is marked at once (an Immune to Spells Unit is not among them), the one state check after
//! the list collecting them together (R59).

use jackioh_engine::effects::{ForEachCardArgs, cards_in_scope, destroy, for_each_card};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-032-1";

/// R81: one damaged Unit, either side.
fn damaged_unit() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"], "damaged": true }))]
}

/// §4: damage above 0.
fn damaged_enemy_units(ctx: &mut EffectContext<'_>) -> Vec<String> {
    cards_in_scope(ctx, &json_as(json!({ "side": "enemy" })))
        .iter()
        .filter(|unit| unit.damage > 0)
        .map(|unit| unit.id.clone())
        .collect()
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: damaged_unit(),
        cry: Some(hook(|_ctx| vec![destroy(json_as(json!({ "target": { "of": "chosen" } })))])),
        ..Script::default()
    };
    let radiant = Script {
        cry: Some(hook(|_ctx| {
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(|ctx: &mut EffectContext<'_>| damaged_enemy_units(ctx)),
                each: Arc::new(|instance_id: &str| {
                    destroy(json_as(json!({ "target": { "of": "instance", "instanceId": instance_id } })))
                }),
            })]
        })),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// C+ #32.1 Execute — SPEC §8.7 row 32.1, BUILD M9 Classic+ row C+ 32.1.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const EXECUTE: &str = "classicplus-032-1";
    /// #19 Midrange Menace 9/9 and #11 Tempo Timmy 3/3.
    const MENACE: &str = "core-019";
    const TIMMY: &str = "core-011";
    /// C #41 State of the Game, 3/3 Indestructible.
    const UNBREAKABLE: &str = "classic-041";
    /// C #80, Indestructible and no Immutable, so a Spell may target it (State of the Game may not since #2179).
    const BIG_MAX: &str = "classic-080";
    const FILLER: &str = "core-005";

    fn destroyed(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Destroyed { def_id, .. } => Some(def_id.clone()),
                _ => None,
            })
            .collect()
    }

    mod c_n32_1_execute {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn s6_3_destroys_a_damaged_enemy_unit() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [EXECUTE, FILLER] },
                    "p2": { "field": [{ "def": MENACE, "damage": 1 }, TIMMY] },
                }));
                let targets = pick::unit_at(&s, P2, 1);
                s.play(EXECUTE, json!({ "targets": targets }));
                assert_eq!(destroyed(&s), [MENACE]);
                s.expect_in_zone(TIMMY, "field");
            }

            #[test]
            fn s8_7_a_damaged_unit_of_your_own_is_a_target_too() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [EXECUTE, FILLER], "field": [{ "def": TIMMY, "damage": 2 }] } }));
                let targets = pick::unit_at(&s, P1, 1);
                s.play(EXECUTE, json!({ "targets": targets }));
                assert_eq!(destroyed(&s), [TIMMY]);
            }

            #[test]
            fn s4_an_undamaged_unit_is_no_legal_target() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [EXECUTE, FILLER] },
                    "p2": { "field": [MENACE, { "def": TIMMY, "damage": 1 }] },
                }));
                let targets = pick::unit_at(&s, P2, 1);
                s.expect_refused(|s| s.play(EXECUTE, json!({ "targets": targets })));
            }

            #[test]
            fn s8_with_no_damaged_unit_the_spell_fizzles_and_still_counts_as_played() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [EXECUTE, FILLER] }, "p2": { "field": [MENACE] } }));
                s.play(EXECUTE, json!({}));
                assert!(destroyed(&s).is_empty());
                s.expect_in_zone(EXECUTE, "graveyard");
                s.expect_events(json!("cardPlayed"));
            }

            #[test]
            fn r46_r2179000_an_indestructible_damaged_unit_survives_in_attack_position() {
                crate::register_all();
                // An Indestructible Unit is never in Defense Position (R2179000), so the knock-down has no
                // switch to make.
                let mut s = scenario(json!({
                    "p1": { "hand": [EXECUTE, FILLER] },
                    "p2": { "field": [{ "def": BIG_MAX, "damage": 1 }] },
                }));
                let targets = pick::unit_at(&s, P2, 1);
                s.play(EXECUTE, json!({ "targets": targets }));
                s.expect_in_zone(BIG_MAX, "field");
                assert_eq!(s.stats(BIG_MAX).position, Position::Atk);
                assert!(destroyed(&s).is_empty());
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn s8_7_destroys_every_damaged_enemy_unit_and_no_other() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": EXECUTE, "radiant": true }, FILLER], "field": [{ "def": TIMMY, "damage": 1 }] },
                    "p2": { "field": [{ "def": MENACE, "damage": 3 }, { "def": TIMMY, "damage": 1 }, MENACE] },
                }));
                s.play(EXECUTE, json!({}));
                let mut gone = destroyed(&s);
                gone.sort();
                let mut want = vec![MENACE.to_string(), TIMMY.to_string()];
                want.sort();
                assert_eq!(gone, want);
                // The undamaged enemy Menace and your own damaged Timmy stay.
                assert_eq!(s.unit(P2, 3).map(|unit| unit.def_id).as_deref(), Some(MENACE));
                assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id).as_deref(), Some(TIMMY));
            }

            #[test]
            fn r46_an_indestructible_one_is_knocked_down_with_no_damaged_enemy_nothing_happens() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": EXECUTE, "radiant": true }, FILLER] },
                    "p2": { "field": [{ "def": UNBREAKABLE, "damage": 2, "position": "DEF" }] },
                }));
                s.play(EXECUTE, json!({}));
                s.expect_in_zone(UNBREAKABLE, "field");
                assert_eq!(s.stats(UNBREAKABLE).position, Position::Atk);

                let mut none = scenario(json!({
                    "p1": { "hand": [{ "def": EXECUTE, "radiant": true }, FILLER] },
                    "p2": { "field": [MENACE] },
                }));
                none.play(EXECUTE, json!({}));
                assert!(destroyed(&none).is_empty());
                none.expect_in_zone(EXECUTE, "graveyard");
            }
        }
    }
}
