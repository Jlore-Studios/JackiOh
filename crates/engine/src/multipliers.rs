//! Trigger multipliers (docs/meditative-set.md M5, ME-TRIG; R820–R823): how many extra times a
//! player's hooks of one kind run, a pure-read script hook a card asks while it acts on its
//! controller's field: `TurnHooks` (Meditative #9 Joint Filing) as the turn's hooks are queued (R821),
//! `CryAndDeath` (Meditative #10 Double Counting) as a Cry runs (R822, R823) and a Death is collected.
//!
//! Multipliers never stack: the highest acting one holds (R820), as Hearthstone's Drakkari Enchanter
//! and Brann Bronzebeard do. A face-down card acts on nothing, so it multiplies nothing.

use crate::config::TRIGGER_EXTRA_NONE;
use crate::script::{HookArgs, Script, TriggerExtraHook};
use crate::state::GameState;
use crate::wire::PlayerId;

/// Which of a player's hooks a multiplier reaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Multiplied {
    /// Start of turn and End of turn (R821).
    TurnHooks,
    /// Cry and Death (R822, R823).
    CryAndDeath,
}

impl Multiplied {
    fn hook_of(self, script: &Script) -> Option<&TriggerExtraHook> {
        match self {
            Multiplied::TurnHooks => script.turn_hook_extra.as_ref(),
            Multiplied::CryAndDeath => script.cry_death_extra.as_ref(),
        }
    }
}

/// R820: the extra runs `player`'s hooks of `kind` get now: the highest any card acting on their side
/// gives (`zones::acting_on`, a Unit standing in the backrow among them, R1081000, R1824000; not a
/// face-down card, R1487000), or `TRIGGER_EXTRA_NONE` when none does.
pub fn extra_runs(state: &GameState, player: PlayerId, kind: Multiplied) -> i32 {
    let cards = crate::zones::acting_on(state, player, crate::zones::StandingAt::AmongUnits);
    let mut highest = TRIGGER_EXTRA_NONE;
    for card in cards {
        if crate::preview::lasting_text_off(state, card) {
            continue;
        }
        let script = crate::scripts::script_of(state, card);
        let Some(hook) = kind.hook_of(&script) else {
            continue;
        };
        let extra = hook(HookArgs {
            state,
            self_: card,
            radiant: card.radiant,
        });
        highest = highest.max(extra);
    }
    highest
}
