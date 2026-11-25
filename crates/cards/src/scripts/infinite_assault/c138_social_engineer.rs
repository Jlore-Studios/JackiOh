//! SPEC §8.13 IA #138 Social Engineer ((3) Unit, Epic, 4/6 → 8/12), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cry: Swap control of this and a random enemy Unit."
//!   Radiant: "Cry: Swap control of this and the enemy Unit with the highest Attack."
//!
//! The Cry picks the enemy half of `swapControl` (R4590003): a random enemy Unit on the base face
//! (R60, `ctx.rng`), the one with the highest Attack on the Radiant face, ties broken at random.
//! With no enemy Unit nothing happens. Both cards move at once by R15 and are summoning sick (R171).

use jackioh_engine::effects::swap_control;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-138";

fn engineer(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let opponent = opponent_of(ctx.controller);
            let mut candidates: Vec<CardInstance> = active_units_of(ctx.state, opponent)
                .into_iter()
                .filter(|unit| !is_slumbering(unit))
                .cloned()
                .collect();
            if candidates.is_empty() {
                return vec![];
            }
            let pick = if radiant {
                let top = candidates
                    .iter()
                    .map(|unit| unit_view(ctx.state, unit).attack)
                    .max()
                    .unwrap_or(0);
                candidates.retain(|unit| unit_view(ctx.state, unit).attack == top);
                ctx.rng.pick(&candidates).cloned()
            } else {
                ctx.rng.pick(&candidates).cloned()
            };
            let Some(unit) = pick else {
                return vec![];
            };
            vec![swap_control(SwapControlArgs { instance_id: unit.id })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: engineer(false),
        radiant: engineer(true),
    }
}

// IA #138 Social Engineer — SPEC §8.13 row 138 (R4590003).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const WEAK: &str = "stand-in (1) 2/2 R4/4";
    const STRONG: &str = "stand-in (3) 6/6 R12/12";

    fn board(radiant: bool, seed: &str, field: Value) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 },
            "p2": { "hand": ["core-005"], "field": field },
        }))
    }

    /// Every Unit of `player`'s side, in lane order.
    fn field(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        jackioh_engine::zones::units_on_field_of(s.state(), player)
            .into_iter()
            .cloned()
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r4590003_cry_swaps_control_with_a_random_enemy_unit() {
            let mut s = board(false, "social-engineer", json!([WEAK, STRONG]));
            s.play(ID, json!({ "zone": 1 }));
            let mine = field(&s, P1);
            let theirs = field(&s, P2);
            assert_eq!(mine.len(), 1, "one Unit came back");
            assert_ne!(mine[0].def_id, ID, "this left");
            assert_eq!(mine[0].controller, P1);
            assert_eq!(theirs.len(), 2, "one Unit went over");
            assert_eq!(
                theirs.iter().filter(|unit| unit.def_id == ID).count(),
                1,
                "this arrived"
            );
            assert!(theirs.iter().all(|unit| unit.controller == P2));
        }

        #[test]
        fn r4590003_with_no_enemy_unit_nothing_happens() {
            let mut s = board(false, "social-engineer-alone", json!([]));
            s.play(ID, json!({ "zone": 1 }));
            let mine = field(&s, P1);
            assert_eq!(mine.len(), 1, "this stays");
            assert_eq!(mine[0].def_id, ID);
            assert_eq!(mine[0].controller, P1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4590003_cry_swaps_control_with_the_highest_attack_enemy_unit() {
            let mut s = board(true, "social-engineer-radiant", json!([WEAK, STRONG]));
            s.play(ID, json!({ "zone": 1 }));
            let mine = field(&s, P1);
            let theirs = field(&s, P2);
            // The Radiant face always takes the 6-Attack Unit, leaving the weak one.
            assert_eq!(theirs.len(), 2);
            assert_eq!(theirs[0].def_id, WEAK, "the weak Unit stays");
            assert_eq!(theirs[1].def_id, ID, "this arrived");
            assert_eq!(mine.len(), 1);
            assert_eq!(mine[0].def_id, STRONG, "the strongest Unit came back");
            assert_eq!(mine[0].controller, P1);
        }
    }
}
