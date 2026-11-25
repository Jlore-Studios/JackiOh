//! IA #207 Recursive Self-Improvement (SPEC §8.13 row 207), a card of the held Infinite
//! Assault set (R4577000).
//!   Base:    "Give a friendly Unit +{buff}/+{buff}. Shuffle a copy of this into your deck that
//!            gives +{more}/+{more} more."
//!   Radiant: "Give a friendly Unit +{buff}/+{buff}. Shuffle a copy of this into your deck that
//!            gives +{more}/+{more} more."
//!
//! Engine: one declared friendly Unit, aimed to help (R656); a Cry (R1): a permanent `buff` of
//! the declared `buff` plus the memory `extra` the cast card carries, then `shuffle_into` of a
//! fresh card of the same face whose memory `extra` carries that bonus plus the declared `more`
//! (R4593004, R1246). Built by C9 (#4593).

use jackioh_engine::effects::{buff, shuffle_into};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-207";

/// The memory carrying the running bonus the shuffled copy gives on top of its face's `buff`.
const EXTRA: &str = "extra";

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl {
        aim: Some(TargetAim::Help),
        ..TargetDecl::target(1, 1, json!({ "side": "ally", "of": ["unit"] }))
    }]
}

fn improvement(radiant_face: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            let Some(_) = instance_of(ctx, &TargetSpec::Chosen { index: None }) else {
                return vec![];
            };
            let extra = recalled(ctx, EXTRA).and_then(|stored| stored.as_i64()).unwrap_or(0) as i32;
            let gain = param(&*ctx, "buff") + extra;
            let mut memory = IndexMap::new();
            memory.insert(EXTRA.to_string(), json!(extra + param(&*ctx, "more")));
            vec![
                buff(json_as(json!({
                    "target": { "of": "chosen" },
                    "attack": gain,
                    "health": gain,
                }))),
                shuffle_into(json_as(json!({
                    "defId": ID,
                    "count": 1,
                    "player": "self",
                    "radiant": radiant_face,
                    "memory": memory,
                }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: improvement(false),
        radiant: improvement(true),
    }
}

// IA #207 Recursive Self-Improvement — SPEC §8.13 row 207 (R4593004).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::{EXTRA, ID};
    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    fn cast(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "field": [VANILLA],
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "library": crate::filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        let unit = s.unit_at(P1, 1);
        s.play(ID, pick::targets(pick::instance(&unit)));
        s
    }

    fn copies(s: &Scenario) -> Vec<CardInstance> {
        s.state().players[P1]
            .library
            .iter()
            .filter(|card| card.def_id == ID)
            .cloned()
            .collect()
    }

    fn stats(s: &Scenario) -> (i32, i32) {
        let unit = s.unit_at(P1, 1);
        let view = unit_view(s.state(), &unit);
        (view.attack, view.health)
    }

    /// Move the shuffled copy from the library to the hand, as a draw would leave it.
    fn draw_copy(s: &mut Scenario) {
        let id = copies(s)[0].id.clone();
        let state = s.state_mut();
        let at = state.players[P1]
            .library
            .iter()
            .position(|card| card.id == id)
            .expect("the copy is in the library");
        let card = state.players[P1].library.remove(at);
        state.players[P1].hand.push(card);
    }

    mod base {
        use super::*;

        #[test]
        fn r4593004_gives_2_2_and_shuffles_a_copy_remembering_2_more() {
            let mut s = cast(false, "improvement-base");
            assert_eq!(stats(&s), (6, 6));
            let shuffled = copies(&s);
            assert_eq!(shuffled.len(), 1);
            assert_eq!(
                shuffled[0].memory.get(EXTRA).and_then(Value::as_i64),
                Some(2)
            );
            draw_copy(&mut s);
            let unit = s.unit_at(P1, 1);
            s.play(ID, pick::targets(pick::instance(&unit)));
            assert_eq!(stats(&s), (10, 10), "the copy gives buff plus extra");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4593004_gives_4_4_and_shuffles_a_copy_remembering_4_more() {
            let mut s = cast(true, "improvement-radiant");
            assert_eq!(stats(&s), (8, 8));
            let shuffled = copies(&s);
            assert_eq!(shuffled.len(), 1);
            assert!(shuffled[0].radiant, "the copy keeps the face");
            assert_eq!(
                shuffled[0].memory.get(EXTRA).and_then(Value::as_i64),
                Some(4)
            );
            draw_copy(&mut s);
            let unit = s.unit_at(P1, 1);
            s.play(ID, pick::targets(pick::instance(&unit)));
            assert_eq!(stats(&s), (16, 16), "the copy gives buff plus extra");
        }
    }
}
