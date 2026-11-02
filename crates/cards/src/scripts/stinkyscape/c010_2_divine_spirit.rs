//! S #10.2 Divine Spirit (SPEC §8.10 row S 10.2; §10.4, R1867003): (2) Spell, Other-Origins,
//! Token (printed Epic).
//!   Base:    "Double a Unit's Health."
//!   Radiant: "Triple a Unit's Health."
//!
//! Engine: Hearthstone's Divine Spirit — a declared target, any Unit (R81); `buff` its Health by its
//! Health now, read through the layers as the Cry resolves (§10.4), current and maximum rising
//! together, so a damaged Unit doubles what it has left (R1867003). Radiant: twice that much, for
//! three times the total.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-010-2";

/// R81: any Unit on either side, picked with the play.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

fn divine_spirit(radiant: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            let Some(unit) = instance_of(ctx, &json_as::<TargetSpec>(json!({ "of": "chosen" }))) else {
                return vec![];
            };
            let health = unit_view(&*ctx.state, &unit).health.max(0);
            let gain = if radiant { health * 2 } else { health };
            vec![buff(json_as(json!({ "target": { "of": "chosen" }, "health": gain })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: divine_spirit(false),
        radiant: divine_spirit(true),
    }
}

// S #10.2 Divine Spirit — SPEC §8.10 row S 10.2, BUILD M12 row S 10.2 (R1867003).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{js, scenario};

    const P1: PlayerId = PlayerId::P1;

    const SPIRIT: &str = "stinkyscape-010-2";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const IMMUNE: &str = "classicplus-019-1"; // Radiant: Immune to Spells.
    const FILLER: &str = "core-005";

    fn stats_override(attack: i32, health: i32) -> Value {
        json!({ "attack": attack, "health": health })
    }

    mod base {
        use super::*;

        #[test]
        fn r1867003_a_3_4_becomes_3_8() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [SPIRIT, FILLER],
                    "field": [{ "def": VANILLA, "statsOverride": stats_override(3, 4) }],
                },
                "p2": { "hand": [FILLER] },
            }));
            let unit = s.unit_at(P1, 1);

            s.play(SPIRIT, json!({ "targets": pick::instance(&unit) }));

            s.expect_stats(&unit, json!({ "attack": 3, "health": 8, "maxHealth": 8 }));
        }

        #[test]
        fn r1867003_a_unit_with_2_health_left_of_4_becomes_3_6_with_4_left() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [SPIRIT, FILLER],
                    "field": [{ "def": VANILLA, "statsOverride": stats_override(3, 4), "damage": 2 }],
                },
                "p2": { "hand": [FILLER] },
            }));
            let unit = s.unit_at(P1, 1);

            s.play(SPIRIT, json!({ "targets": pick::instance(&unit) }));

            s.expect_stats(&unit, json!({ "attack": 3, "health": 4, "maxHealth": 6 }));
        }

        #[test]
        fn s8_10_a_unit_immune_to_spells_cannot_be_chosen() {
            crate::register_all();
            let s = scenario(json!({
                "p1": { "hand": [SPIRIT, FILLER], "field": [VANILLA] },
                "p2": { "hand": [FILLER], "field": [{ "def": IMMUNE, "radiant": true }] },
            }));
            let immune = s.card(IMMUNE).id.clone();
            let offered: Vec<String> = legal_actions(s.state(), P1)
                .iter()
                .map(js)
                .filter(|action| action["type"] == "play" && action["instanceId"] == s.card(SPIRIT).id.as_str())
                .flat_map(|action| action["targets"].as_array().cloned().unwrap_or_default())
                .filter(|selection| selection["pick"] == "instance")
                .filter_map(|selection| selection["instanceId"].as_str().map(str::to_string))
                .collect();

            assert!(offered.contains(&s.unit_at(P1, 1).id));
            assert!(!offered.contains(&immune));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1867003_a_3_4_becomes_3_12() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": SPIRIT, "radiant": true }, FILLER],
                    "field": [{ "def": VANILLA, "statsOverride": stats_override(3, 4) }],
                },
                "p2": { "hand": [FILLER] },
            }));
            let unit = s.unit_at(P1, 1);

            s.play(SPIRIT, json!({ "targets": pick::instance(&unit) }));

            s.expect_stats(&unit, json!({ "attack": 3, "health": 12, "maxHealth": 12 }));
        }

        #[test]
        fn r1867003_a_unit_with_2_health_left_of_4_becomes_3_8_with_6_left() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": SPIRIT, "radiant": true }, FILLER],
                    "field": [{ "def": VANILLA, "statsOverride": stats_override(3, 4), "damage": 2 }],
                },
                "p2": { "hand": [FILLER] },
            }));
            let unit = s.unit_at(P1, 1);

            s.play(SPIRIT, json!({ "targets": pick::instance(&unit) }));

            s.expect_stats(&unit, json!({ "attack": 3, "health": 6, "maxHealth": 8 }));
        }
    }
}
