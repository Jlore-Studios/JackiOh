//! IA #216.1 Singularity (SPEC §8.13 row 216.1), the token IA #216 The Singularity
//! summons, a card of the held Infinite Assault set (R4577000).
//!   Base:    "Taunt, Can't attack\nIn {turns|turn|turns}: If this is still on the field, you win
//!            the game."
//!   Radiant: "Taunt, Can't attack\nIn {turns|turn|turns}: If this is still on the field, you win
//!            the game."
//!
//! Engine: Taunt and Can't attack are the catalog's. "In N turns" starts when the token arrives
//! on the field (R4593005): the arrival's `Summoned` names it, and the trigger plants the
//! arrival controller's watched `delay` (R1181000) of the declared `turns` with its badge, so it
//! drops when the token leaves the field (R174); due, its controller wins the game. Built by C9
//! (#4593).

use jackioh_engine::effects::{delay, win_game};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-216-1";

/// The countdown's badge (R169).
const BADGE: &str = "Singularity: you win the game";
/// The delayed step, which the `delayed` hook runs.
const STEP: &str = "singularity";

/// This card arrived on the field: its `Summoned` names its own id.
fn arrived(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    let Some(me) = ctx.live_self() else {
        return false;
    };
    matches!(event, GameEvent::Summoned { instance_id, .. } if *instance_id == me.id)
}

fn singularity() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("singularity", &[GameEventType::Summoned], |ctx, _event| {
                let Some(me) = ctx.live_self() else {
                    return vec![];
                };
                let id = me.id.clone();
                vec![delay(json_as(json!({
                    "at": { "phase": "start", "player": "self" },
                    "step": STEP,
                    "turns": param(&*ctx, "turns"),
                    "label": BADGE,
                    "watch": id,
                })))]
            })
            .with_when(arrived),
        ],
        delayed: Some(hook(|_ctx| vec![win_game(json_as(json!({})))])),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same script: the Radiant face's 4 turns are its declared `turns`.
    let radiant = singularity();
    CardScripts {
        base: singularity(),
        radiant,
    }
}

// IA #216.1 Singularity — SPEC §8.13 row 216.1 (R4593005).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{next_turn_of, scenario};

    const SPELL: &str = "infiniteassault-216"; // The Singularity: summons this token.
    const HIT_JOB: &str = "core-016"; // Hit Job: Destroy a Unit.
    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    fn summoned(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": SPELL, "radiant": radiant }, HIT_JOB, FILLER],
                "library": crate::filler(6),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        s.play(SPELL, json!({}));
        s
    }

    /// How many of p1's own turns pass after the summon before the game is won, and the winner.
    fn turns_to_win(s: &mut Scenario, bound: usize) -> (usize, Option<Winner>) {
        for turns in 1..=bound {
            next_turn_of(s, P1);
            if let Some(result) = s.state().result {
                return (turns, Some(result.winner));
            }
        }
        (bound, None)
    }

    mod base {
        use super::*;

        #[test]
        fn r4593005_p1_wins_at_the_5th_own_start() {
            let mut s = summoned(false, "singularity-win-base");
            assert_eq!(s.unit_at(P1, 1).def_id, ID);
            assert_eq!(turns_to_win(&mut s, 7), (5, Some(Winner::P1)));
        }

        #[test]
        fn r4593005_nothing_is_won_once_it_leaves_the_field() {
            let mut s = summoned(false, "singularity-win-gone");
            let unit = s.unit_at(P1, 1);
            s.play(HIT_JOB, pick::targets(pick::instance(&unit)));
            assert_eq!(turns_to_win(&mut s, 7), (7, None));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4593005_p1_wins_at_the_4th_own_start() {
            let mut s = summoned(true, "singularity-win-radiant");
            assert_eq!(turns_to_win(&mut s, 6), (4, Some(Winner::P1)));
        }
    }
}
