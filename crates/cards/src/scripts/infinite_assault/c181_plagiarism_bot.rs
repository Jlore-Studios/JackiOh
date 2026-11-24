//! SPEC §8.13 IA #181 Plagiarism Bot ((2) Unit, Rare, 3/3 → 6/6), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Once per turn, when your opponent plays a Spell, add a copy of it to your hand."
//!   Radiant: "Twice per turn, when your opponent plays a Spell, add a copy of it to your hand."
//!
//! Built by C8 (#4592). R4592000, R4592002.

use jackioh_engine::catalog::def_of;
use jackioh_engine::effects::{RememberArgs, add_to_hand, remember};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-181";

const MEMORY_KEY: &str = "plagiarism";

fn plagiarism(cap: i64) -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("plagiarism-bot", &[GameEventType::CardPlayed], move |ctx, event| {
                let GameEvent::CardPlayed { instance_id, .. } = event else {
                    return vec![];
                };
                let current_turn = ctx.state.turn as i64;
                let (turn, count) = match ctx.live_self().and_then(|this| this.memory.get(MEMORY_KEY)) {
                    Some(val) => (
                        val.get("turn").and_then(|t| t.as_i64()).unwrap_or(0),
                        val.get("count").and_then(|c| c.as_i64()).unwrap_or(0),
                    ),
                    None => (0, 0),
                };
                let next_count = if turn == current_turn { count + 1 } else { 1 };
                vec![
                    remember(RememberArgs {
                        key: MEMORY_KEY.to_string(),
                        value: json!({ "turn": current_turn, "count": next_count }),
                    }),
                    add_to_hand(json_as(json!({ "copyOf": instance_id }))),
                ]
            })
            .with_when(move |ctx, event| match event {
                GameEvent::CardPlayed { player, def_id, .. } => {
                    if *player == ctx.controller {
                        return false;
                    }
                    if def_of(Some(&*ctx.state), def_id).type_ != CardType::Spell {
                        return false;
                    }
                    let current_turn = ctx.state.turn as i64;
                    let (turn, count) = match ctx.live_self().and_then(|this| this.memory.get(MEMORY_KEY)) {
                        Some(val) => (
                            val.get("turn").and_then(|t| t.as_i64()).unwrap_or(0),
                            val.get("count").and_then(|c| c.as_i64()).unwrap_or(0),
                        ),
                        None => (0, 0),
                    };
                    let used = if turn == current_turn { count } else { 0 };
                    used < cap
                }
                _ => false,
            }),
        ],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: plagiarism(1),
        radiant: plagiarism(2),
    }
}

// IA #181 Plagiarism Bot — SPEC §8.13 row 181.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const SPELL: &str = "core-016"; // Hit Job

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "field": [{ "def": ID, "radiant": radiant, "lane": 1 }],
                "mana": 10,
            },
            "p2": { "hand": [SPELL, SPELL, SPELL], "mana": 10 },
            "active": "p2",
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn triggers_once_per_turn_on_enemy_spell() {
            let mut s = board(false);
            s.play(SPELL, json!({}));
            assert_eq!(s.hand(P1).len(), 1);
            assert_eq!(s.hand(P1)[0].def_id, SPELL);
            // Second spell in same turn does not trigger
            s.play(SPELL, json!({}));
            assert_eq!(s.hand(P1).len(), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_6_6_and_triggers_twice_per_turn() {
            let mut s = board(true);
            s.expect_stats(s.unit_at(P1, 1), json!({ "attack": 6, "health": 6 }));
            s.play(SPELL, json!({}));
            assert_eq!(s.hand(P1).len(), 1);
            s.play(SPELL, json!({}));
            assert_eq!(s.hand(P1).len(), 2);
            // Third does not trigger
            s.play(SPELL, json!({}));
            assert_eq!(s.hand(P1).len(), 2);
        }
    }
}
