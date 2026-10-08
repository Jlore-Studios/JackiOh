//! Meditative #12 Fear Mongerer (SPEC §8.8 row 12; docs/meditative-set.md M6 #12; ME-TRIG (c); R824).
//! (2) Unit, Human, Epic, 6/8 → 12/16.
//!   Base:    "Cry: Trigger your End of turn effects."
//!   Radiant: "Cry: Trigger your End of turn effects {times|time|times}." (times 2)
//!
//! `trigger_turn_hooks` queues its controller's end-of-turn hooks as §2.2's end-of-turn trigger step
//! would — field cards and the return-flagged Spells in their graveyard (R153, R155), Meditative #9's
//! multiplier included (R821) — and the loop resolves them after the Cry's list. The turn goes on; the
//! trap window and the delayed effects do not run (R62). The rounds run one after another (R824).
//! `times` is tuned on the Radiant face only (R749): the base face's "your End of turn effects" is once.

use jackioh_engine::effects::{TriggerTurnHooksArgs, trigger_turn_hooks};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-012";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![trigger_turn_hooks(TriggerTurnHooksArgs {
                times: param(&*ctx, "times"),
            })]
        })),
        ..Script::default()
    };
    CardScripts {
        // The same script: the base face reads its printed `times` (1, R749), the Radiant face its 2.
        radiant: base.clone(),
        base,
    }
}
