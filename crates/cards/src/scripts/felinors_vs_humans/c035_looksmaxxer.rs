//! SPEC §8.14 FvH #35 Looksmaxxer ((2) Unit, Human, Common, 1/1 → 2/2), a card of the held Felinors vs
//! Humans set (R5235000).
//!   Base:    "Cry: Set this Unit's Attack to the highest Attack and its Health to the highest Health among
//!             Units."
//!   Radiant: the same, "Then Buff this twice."
//!
//! Engine: IA #16 Benchmark Run's read (R4577007) over every acting Unit on the field, both sides and
//! itself, through the layers (`unit_view`): the highest shown Attack and, apart, the highest current
//! Health; then `set_stats` on itself to them, a stats overwrite (§6.3 Set stats, R2198000). Radiant: then
//! `upgrade` of itself twice, two Buffs drawn one at a time (R386, R1320), so they land on the copied line.
//! Declares no numbers. Built by S01 (#5251).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-035";

/// "Then Buff this twice."
const RADIANT_BUFFS: i32 = 2;

/// The highest Attack and the highest current Health among the acting Units of both sides.
fn best_looks(ctx: &EffectContext<'_>) -> Option<(i32, i32)> {
    let views: Vec<UnitView> = [ctx.controller, opponent_of(ctx.controller)]
        .into_iter()
        .flat_map(|player| active_units_of(ctx.state, player))
        .map(|unit| unit_view(ctx.state, unit))
        .collect();
    let attack = views.iter().map(|view| view.attack).max()?;
    let health = views.iter().map(|view| view.health).max()?;
    Some((attack, health))
}

fn looksmaxxer(buffs: i32) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut effects = Vec::new();
            if let Some((attack, health)) = best_looks(ctx) {
                effects.push(set_stats(json_as(
                    json!({ "target": { "of": "self" }, "attack": attack, "health": health }),
                )));
            }
            if buffs > 0 {
                effects.push(upgrade(json_as(json!({ "target": { "of": "self" }, "times": buffs }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: looksmaxxer(0),
        radiant: looksmaxxer(RADIANT_BUFFS),
    }
}

// FvH #35 Looksmaxxer — SPEC §8.14 row 35 (R4577007, R2198000, R386).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    /// The highest Attack stands on p1's side; p2's Unit is the healthiest, though wounded.
    const STRONG: &str = "stand-in (1) 7/2";
    const STURDY: &str = "stand-in (1) 1/12";
    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    /// p1 plays the Looksmaxxer (`radiant` its face) into lane 3 beside a Strong, facing a Sturdy with
    /// `wound` damage on it: the Unit it became.
    fn played(radiant: bool, wound: i32) -> (Scenario, CardInstance) {
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": [STRONG],
                "library": filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "field": [{ "def": STURDY, "damage": wound }], "library": filler(4) },
        }));
        s.play(ID, json!({ "zone": 3 }));
        let looks = s.unit_at(P1, 3);
        (s, looks)
    }

    #[test]
    fn declares_no_tuned_numbers() {
        crate::register_all();
        assert!(crate::card_def(ID).params.is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn r4577007_it_takes_the_highest_attack_and_the_highest_health_among_units_of_both_sides() {
            let (mut s, looks) = played(false, 0);
            s.expect_stats(&looks, json!({ "attack": 7, "health": 12 }));
            assert_eq!(s.count("upgraded"), 0, "the base face Buffs nothing");
        }

        #[test]
        fn r4577007_the_highest_health_is_the_current_one_not_the_maximum() {
            let (mut s, looks) = played(false, 3);
            s.expect_stats(&looks, json!({ "attack": 7, "health": 9 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r386_it_copies_the_best_line_then_buffs_itself_twice() {
            let (s, looks) = played(true, 0);
            assert_eq!(s.count("upgraded"), 2, "two Buffs, one at a time");
            // A Buff draws a stat split or a keyword, never a loss: the copied line is the floor.
            let view = s.stats(&looks);
            assert!(view.attack >= 7 && view.health >= 12, "{view:?}");
        }
    }
}
