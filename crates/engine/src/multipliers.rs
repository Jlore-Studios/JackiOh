//! Trigger multipliers (docs/meditative-set.md M5, ME-TRIG; R820–R823): how many extra times a
//! player's hooks of one kind run.
//!
//! Two kinds, each a pure-read script hook a card asks while it acts on its controller's field:
//!   - `TurnHooks` (`Script.turn_hook_extra`, Meditative #9 Joint Filing): the player's start-of-turn
//!     and end-of-turn hooks, read as they are queued (`triggers::queue_hooks_in_trigger_order`, R821);
//!   - `CryAndDeath` (`Script.cry_death_extra`, Meditative #10 Double Counting): a permanent's Cry as it
//!     runs for the player (`play_steps`, `cry_trigger`, R822, R823) and the Death of a card that died
//!     under their control (`state_check::collect`, R823).
//!
//! Multipliers never stack: the highest acting one holds (R820), as Hearthstone's Drakkari Enchanter
//! and Brann Bronzebeard do. A face-down card acts on nothing, so it multiplies nothing.

use crate::config::TRIGGER_EXTRA_NONE;
use crate::script::{HookArgs, Script, TriggerExtraHook};
use crate::state::GameState;
use crate::wire::{PlayerId, Row};

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

/// R820: the extra runs `player`'s hooks of `kind` get now — the highest any card acting on their side
/// of the field gives (the top of each unit pile, then each backrow card that is not face-down), or
/// `TRIGGER_EXTRA_NONE` when none does.
pub fn extra_runs(state: &GameState, player: PlayerId, kind: Multiplied) -> i32 {
    let mut cards = crate::zones::active_units_of(state, player);
    for slot in crate::zones::slots_of(player, Row::Backrow) {
        if let Some(card) = crate::zones::card_at(state, slot) {
            cards.push(card);
        }
    }
    let mut highest = TRIGGER_EXTRA_NONE;
    for card in cards {
        if crate::preview::is_face_down(state, card) {
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
