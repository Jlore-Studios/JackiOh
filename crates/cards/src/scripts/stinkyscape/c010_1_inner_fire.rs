//! S #10.1 Inner Fire (SPEC §8.10 row S 10.1; §10.4, R386, R1867002): (1) Spell, Other-Origins,
//! Token (printed Epic).
//!   Base:    "Set a Unit's Attack equal to its Health."
//!   Radiant: "Set a Unit's Attack equal to its Health, plus {shift} if it's yours or minus {shift}
//!            if it's an enemy." — shift 4, the Radiant face alone.
//!
//! Engine: Hearthstone's Inner Fire on existing primitives — a declared target, any Unit (R81); read
//! its Health H and Attack A through the layers as the Cry resolves (§10.4) and `buff` the Attack by
//! max(0, H + shift) − A, never below 0 (R1867002). Shift is 0 on the base face; on the Radiant face
//! the declared `shift`, signed + for a Unit its caster controls and − for an enemy's.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-010-1";

/// R81: any Unit on either side, picked with the play.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

fn inner_fire(radiant: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            let Some(unit) = instance_of(ctx, &json_as::<TargetSpec>(json!({ "of": "chosen" }))) else {
                return vec![];
            };
            let view = unit_view(&*ctx.state, &unit);
            let shift = if radiant {
                let magnitude = param(&*ctx, "shift");
                if unit.controller == ctx.controller {
                    magnitude
                } else {
                    -magnitude
                }
            } else {
                0
            };
            let attack = (view.health + shift).max(0) - view.attack;
            vec![buff(json_as(json!({ "target": { "of": "chosen" }, "attack": attack })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: inner_fire(false),
        radiant: inner_fire(true),
    }
}

// S #10.1 Inner Fire — SPEC §8.10 row S 10.1, BUILD M12 row S 10.1 (R1867002).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{js, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const INNER_FIRE: &str = "stinkyscape-010-1";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const WEAPONS: &str = "core-014"; // (4) Field Spell: your units have +3 Attack.
    const IMMUNE: &str = "classicplus-019-1"; // Radiant: Immune to Spells.
    const FILLER: &str = "core-005";

    fn stats_override(attack: i32, health: i32) -> Value {
        json!({ "attack": attack, "health": health })
    }

    mod base {
        use super::*;

        #[test]
        fn r1867002_a_2_6_becomes_6_6_and_a_7_3_becomes_3_3() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [INNER_FIRE, INNER_FIRE, FILLER],
                    "field": [
                        { "def": VANILLA, "lane": 1, "statsOverride": stats_override(2, 6) },
                        { "def": VANILLA, "lane": 2, "statsOverride": stats_override(7, 3) },
                    ],
                    "mana": 9,
                },
                "p2": { "hand": [FILLER] },
            }));
            let small = s.unit_at(P1, 1);
            let big = s.unit_at(P1, 2);

            s.play(INNER_FIRE, json!({ "targets": pick::instance(&small) }));
            s.play(INNER_FIRE, json!({ "targets": pick::instance(&big) }));

            s.expect_stats(&small, json!({ "attack": 6, "health": 6 }));
            s.expect_stats(&big, json!({ "attack": 3, "health": 3 }));
        }

        #[test]
        fn r1867002_a_unit_under_an_attack_aura_shows_its_health() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [INNER_FIRE], "field": [VANILLA], "backrow": [WEAPONS] },
                "p2": { "hand": [FILLER] },
            }));
            let unit = s.unit_at(P1, 1);
            s.expect_stats(&unit, json!({ "attack": 7 }));

            s.play(INNER_FIRE, json!({ "targets": pick::instance(&unit) }));

            s.expect_stats(&unit, json!({ "attack": 4, "health": 4 }));
        }

        #[test]
        fn s8_10_a_unit_immune_to_spells_cannot_be_chosen() {
            crate::register_all();
            let s = scenario(json!({
                "p1": { "hand": [INNER_FIRE, FILLER], "field": [VANILLA] },
                "p2": { "hand": [FILLER], "field": [{ "def": IMMUNE, "radiant": true }] },
            }));
            let immune = s.card(IMMUNE).id.clone();
            let offered: Vec<String> = legal_actions(s.state(), P1)
                .iter()
                .map(js)
                .filter(|action| action["type"] == "play" && action["instanceId"] == s.card(INNER_FIRE).id.as_str())
                .flat_map(|action| action["targets"].as_array().cloned().unwrap_or_default())
                .filter(|selection| selection["pick"] == "instance")
                .filter_map(|selection| selection["instanceId"].as_str().map(str::to_string))
                .collect();

            assert!(offered.contains(&s.unit_at(P1, 1).id));
            assert!(!offered.contains(&immune));
        }

        #[test]
        fn r386_the_radiant_shift_is_untuned_on_the_base_face() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [INNER_FIRE, FILLER],
                    "field": [{ "def": VANILLA, "lane": 1, "statsOverride": stats_override(2, 6) }],
                },
                "p2": { "hand": [FILLER] },
            }));
            step_param(s.card_mut(INNER_FIRE), "shift", 2);
            let unit = s.unit_at(P1, 1);

            s.play(INNER_FIRE, json!({ "targets": pick::instance(&unit) }));

            s.expect_stats(&unit, json!({ "attack": 6, "health": 6 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1867002_a_friendly_2_6_becomes_10_6() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": INNER_FIRE, "radiant": true }, FILLER],
                    "field": [{ "def": VANILLA, "lane": 1, "statsOverride": stats_override(2, 6) }],
                },
                "p2": { "hand": [FILLER] },
            }));
            let unit = s.unit_at(P1, 1);

            s.play(INNER_FIRE, json!({ "targets": pick::instance(&unit) }));

            s.expect_stats(&unit, json!({ "attack": 10, "health": 6 }));
        }

        #[test]
        fn r1867002_an_enemy_7_3_becomes_0_3_and_an_enemy_2_6_is_unchanged() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": INNER_FIRE, "radiant": true }, FILLER], "mana": 9 },
                "p2": {
                    "hand": [FILLER, FILLER],
                    "field": [
                        { "def": VANILLA, "lane": 1, "statsOverride": stats_override(7, 3) },
                        { "def": VANILLA, "lane": 2, "statsOverride": stats_override(2, 6) },
                    ],
                },
            }));
            let big = s.unit_at(P2, 1);
            let first = s.card(INNER_FIRE).id.clone();

            s.play(&first, json!({ "targets": pick::instance(&big) }));
            s.expect_stats(&big, json!({ "attack": 0, "health": 3 }));
        }

        #[test]
        fn r1867002_an_enemy_2_6_stays_2_6_attack_never_below_0() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": INNER_FIRE, "radiant": true }, FILLER] },
                "p2": {
                    "hand": [FILLER],
                    "field": [{ "def": VANILLA, "lane": 2, "statsOverride": stats_override(2, 6) }],
                },
            }));
            let unit = s.unit_at(P2, 2);

            s.play(INNER_FIRE, json!({ "targets": pick::instance(&unit) }));

            s.expect_stats(&unit, json!({ "attack": 2, "health": 6 }));
        }
    }
}
