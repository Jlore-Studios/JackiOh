//! SPEC §8.13 IA #141 Logic Bomb ((2) Unit, Epic, 0/6 → 0/12), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Can't attack. In 3 turns: Destroy every enemy Unit."
//!   Radiant: "Can't attack. In 2 turns: Destroy every enemy Unit."
//!
//! M+ #4 Supply Drop's shape: the Cry arms a `delay` for the start of the controller's Nth turn
//! (R1181000), watching the bomb itself — killing the bomb defuses it (R4590007, R78). When it fires
//! it destroys the Units of whoever does not control the bomb then: the enemy side while the bomb is
//! the caster's, the caster's own once stolen. Can't attack is catalog data.

use jackioh_engine::effects::{delay, destroy_all};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-141";

/// The step answering the armed countdown.
const DETONATE: &str = "detonate";

/// "In 3 turns" (base) or "In 2 turns" (Radiant): the card's declared `turns`, in the controller's
/// own turns (R1181000).
fn bomb() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let Some(this) = &ctx.self_ else {
                return vec![];
            };
            let turns = param(&*ctx, "turns");
            vec![delay(json_as(json!({
                "at": { "phase": "start", "player": "self" },
                "turns": turns,
                "step": DETONATE,
                "hook": RESUME_HOOK,
                "data": { "instanceId": this.id },
                "watch": this.id,
                "label": format!("Logic Bomb: destroys every enemy Unit in {turns} turns"),
            })))]
        })),
        resume: IndexMap::from([(
            DETONATE,
            hook(|ctx| {
                let instance_id = ctx.data.get("instanceId").and_then(Value::as_str);
                let bomb = instance_id.and_then(|id| find_instance(&*ctx.state, id));
                let Some(bomb) = bomb else {
                    return vec![];
                };
                // R4590007: the Units of whoever does not control the bomb when it fires.
                let side = if bomb.controller == ctx.controller {
                    "enemy"
                } else {
                    "self"
                };
                vec![destroy_all(json_as(json!({
                    "side": side,
                    "rows": ["units"],
                })))]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The Radiant face's 2 is its declared `turns`.
    let base = bomb();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #141 Logic Bomb — SPEC §8.13 row 141 (R4590007).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FILLER: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.
    const HIT_JOB: &str = "core-016"; // Hit Job: Destroy a Unit.

    fn board(radiant: bool, seed: &str, p2_hand: Value) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "mana": 10,
                "library": crate::filler(4),
            },
            "p2": { "hand": p2_hand, "field": [VANILLA, VANILLA], "mana": 10, "library": crate::filler(4) },
        }))
    }

    /// Both sides pass `rounds` full rounds (p1 then p2).
    fn pass_rounds(s: &mut Scenario, rounds: usize) {
        for _ in 0..rounds {
            s.end_turn().end_turn();
        }
    }

    fn p1_badges(s: &Scenario, viewer: PlayerId) -> Vec<String> {
        let view = s.view(viewer);
        let seat = if viewer == P1 { view.you } else { view.opponent };
        seat.modifiers.into_iter().map(|badge| badge.label).collect()
    }

    fn enemy_units(s: &Scenario) -> usize {
        jackioh_engine::zones::units_on_field_of(s.state(), P2).len()
    }

    mod base {
        use super::*;

        #[test]
        fn r4590007_it_fires_on_turn_3_not_turn_2_and_shows_a_badge() {
            let mut s = board(false, "logic-bomb", json!([FILLER]));
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(
                p1_badges(&s, P1),
                vec!["Logic Bomb: destroys every enemy Unit in 3 turns".to_string()]
            );
            pass_rounds(&mut s, 2);
            assert_eq!(s.state().turn, 3);
            assert_eq!(enemy_units(&s), 2, "alive at the start of the third turn");
            s.end_turn().end_turn();
            assert_eq!(s.state().turn, 4);
            assert_eq!(enemy_units(&s), 0, "destroyed at the start of the fourth turn");
        }

        #[test]
        fn r4590007_it_is_defused_when_the_bomb_is_killed() {
            let mut s = board(false, "logic-bomb-defused", json!([HIT_JOB, FILLER]));
            s.play(ID, json!({ "zone": 1 }));
            s.end_turn();
            let bomb = s.unit_at(P1, 1);
            s.play(HIT_JOB, pick::targets(pick::instance(&bomb)));
            s.expect_in_zone(&bomb, "graveyard");
            s.end_turn();
            pass_rounds(&mut s, 3);
            assert_eq!(enemy_units(&s), 2, "the defused bomb destroys nothing");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4590007_it_fires_a_turn_earlier() {
            let mut s = board(true, "logic-bomb-radiant", json!([FILLER]));
            s.play(ID, json!({ "zone": 1 }));
            pass_rounds(&mut s, 1);
            assert_eq!(s.state().turn, 2);
            assert_eq!(enemy_units(&s), 2, "alive at the start of the second turn");
            s.end_turn().end_turn();
            assert_eq!(s.state().turn, 3);
            assert_eq!(enemy_units(&s), 0, "destroyed at the start of the third turn");
        }
    }
}
