//! S #10.2 Divine Spirit (SPEC §8.10 row S 10.2; §10.4, R1867003): (2) Spell, Other-Origins,
//! Token (printed Epic).
//!   Base:    "Double a Unit's Health."
//!   Radiant: "Triple a Unit's Health."
//!
//! Engine: Hearthstone's Divine Spirit — a declared target, any Unit (R81); `buff` its Health by its
//! Health now, read through the layers as the Cry resolves (§10.4), current and maximum rising
//! together, so a damaged Unit doubles what it has left (R1867003). Radiant: twice that much, for
//! three times the total. R4079000: the multiplied Health goes no higher than `MULTIPLIED_STAT_CAP`,
//! and a Unit at or above it gains none.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-010-2";

/// R81: any Unit on either side, picked with the play.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

/// `capped` (R4079000): the multiplied Health stops at `MULTIPLIED_STAT_CAP`.
fn divine_spirit(radiant: bool, capped: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            let Some(unit) = instance_of(ctx, &json_as::<TargetSpec>(json!({ "of": "chosen" }))) else {
                return vec![];
            };
            let health = unit_view(&*ctx.state, &unit).health.max(0);
            let gain = if radiant { health * 2 } else { health };
            if !capped {
                return vec![buff(json_as(json!({ "target": { "of": "chosen" }, "health": gain })))];
            }
            let gain = gain.min(MULTIPLIED_STAT_CAP - health).max(0);
            if gain == 0 {
                return vec![];
            }
            vec![buff(json_as(json!({ "target": { "of": "chosen" }, "health": gain })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: divine_spirit(false, true),
        radiant: divine_spirit(true, true),
    }
}

/// R767000: through v0.6.6d the multiplying had no cap (R4079000, issue #4079); `KEPT_SCRIPTS` folds
/// those replays under it.
pub fn script_v0_6_6d() -> CardScripts {
    CardScripts {
        base: divine_spirit(false, false),
        radiant: divine_spirit(true, false),
    }
}

// S #10.2 Divine Spirit — SPEC §8.10 row S 10.2, BUILD M12 row S 10.2 (R1867003).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{js, scenario};

    const P1: PlayerId = PlayerId::P1;

    const SPIRIT: &str = "stinkyscape-010-2";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const IMMUNE: &str = "classicplus-019-1"; // Top Loser: Radiant: Immune to Spells.
    const FILLER: &str = "core-005";

    fn stats_override(attack: i32, health: i32) -> Value {
        json!({ "attack": attack, "health": health })
    }

    mod base {
        use super::*;

        #[test]
        fn r1867003_a_3_4_becomes_3_8() {
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

    /// p1's Unit at `health`, Divine Spirit on `radiant`'s face in hand, cast on it.
    fn spirited(radiant: bool, health: i32) -> (Scenario, CardInstance) {
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": SPIRIT, "radiant": radiant }, FILLER],
                "field": [{ "def": VANILLA, "statsOverride": stats_override(3, health) }],
            },
            "p2": { "hand": [FILLER] },
        }));
        let unit = s.unit_at(P1, 1);
        s.play(SPIRIT, json!({ "targets": pick::instance(&unit) }));
        (s, unit)
    }

    #[test]
    fn r4079000_divine_spirit_multiplies_health_only_up_to_the_cap() {
        for radiant in [false, true] {
            let (mut s, unit) = spirited(radiant, 600);
            s.expect_stats(
                &unit,
                json!({ "health": MULTIPLIED_STAT_CAP, "maxHealth": MULTIPLIED_STAT_CAP }),
            );
        }
        let (mut s, unit) = spirited(true, 300);
        s.expect_stats(&unit, json!({ "health": 900, "maxHealth": 900 }));
    }

    mod v0_6_6d {
        use super::*;
        use crate::scripts::stinkyscape::c010_2_divine_spirit::{ID, script_v0_6_6d};

        /// R767000: a game on v0.6.6d, before the cap, doubled past it.
        #[test]
        fn r767000_a_game_on_v0_6_6d_doubles_health_past_the_cap() {
            crate::under_kept(ID, script_v0_6_6d(), || {
                let (mut s, unit) = spirited(false, 600);
                s.expect_stats(&unit, json!({ "health": 1200, "maxHealth": 1200 }));
            });
        }
    }
}
