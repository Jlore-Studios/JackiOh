//! SPEC §8.13 IA #55 Heat Sink ((2) Unit, Common, 2/6 → 4/12), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Taunt. Whenever your hero takes damage on your turn, this gains +{attack} Attack." (attack 1)
//!   Radiant: "Taunt. Whenever your hero takes damage on your turn, this gains +{attack} Attack." (attack 2)
//!
//! Taunt is the catalog's. The trigger answers the `Damage` event, as S #5 Masochism Unmasked's does:
//! a hit on its controller's hero while they are the active player (R1875000), and only when Health
//! gets through (`amount > 0`, R4587005; armor and no-damage hits give nothing). Each such hit gains
//! the declared `attack` as a permanent buff.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-055";

/// R4587005, R1875000: a hit on its controller's hero, on their own turn, that gets through to
/// Health — the S #5 Masochism Unmasked reader, narrowed to hits with `amount > 0`.
fn hit_on_own_turn(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    let GameEvent::Damage { target_id, amount, .. } = event else {
        return false;
    };
    *amount > 0
        && ctx.state.active == ctx.controller
        && *target_id == format!("hero-{}", ctx.controller)
}

fn heat_sink() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("heat-sink", &[GameEventType::Damage], |ctx, _event| {
                vec![buff(json_as(
                    json!({ "target": { "of": "self" }, "attack": param(&*ctx, "attack") }),
                ))]
            })
            .with_when(|ctx, event| hit_on_own_turn(ctx, event)),
        ],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = heat_sink();
    // The same script: the Radiant face's +2 is its declared `attack`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #55 Heat Sink — SPEC §8.13 row 55 (R4587005).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const GRUNT: &str = "infiniteassault-053"; // Overvolt Grunt
    const AFTERBURNER: &str = "infiniteassault-056"; // Afterburner

    fn board(radiant: bool, hand: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": hand, "field": [{ "def": ID, "radiant": radiant }], "library": crate::filler(4) },
            "p2": { "library": crate::filler(4) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn a_hit_on_your_hero_on_your_turn_gives_1_attack() {
            let mut s = board(false, json!([GRUNT]));
            s.play(GRUNT, json!({ "zone": 2 }));
            let sink = s.unit_at(P1, 1);
            s.expect_stats(&sink, json!({ "attack": 3, "health": 6 }));
        }

        #[test]
        fn r4587005_a_hit_on_the_opponents_turn_gives_nothing() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "field": [ID], "library": crate::filler(4) },
                "p2": { "hand": [AFTERBURNER], "library": crate::filler(4) },
            }));
            s.play(AFTERBURNER, json!({}));
            // p2's Afterburner hits p1's hero for 4 on p2's turn: not its turn, no buff.
            s.expect_health(P1, HERO_HEALTH - 4);
            let sink = s.unit_at(P1, 1);
            s.expect_stats(&sink, json!({ "attack": 2, "health": 6 }));
        }

        #[test]
        fn r4587005_a_hit_your_armor_takes_whole_gives_nothing() {
            let mut s = scenario(json!({
                "p1": { "hand": [GRUNT], "field": [ID], "armor": 5, "library": crate::filler(4) },
                "p2": { "library": crate::filler(4) },
            }));
            s.play(GRUNT, json!({ "zone": 2 }));
            s.expect_health(P1, HERO_HEALTH);
            let sink = s.unit_at(P1, 1);
            s.expect_stats(&sink, json!({ "attack": 2, "health": 6 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn a_hit_on_your_hero_on_your_turn_gives_2_attack() {
            let mut s = board(true, json!([GRUNT]));
            s.play(GRUNT, json!({ "zone": 2 }));
            let sink = s.unit_at(P1, 1);
            s.expect_stats(&sink, json!({ "attack": 6, "health": 12 }));
        }
    }
}
