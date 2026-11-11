//! A practice claim checked by stepping the machine over the claimed game (SPEC §14.8, §9.9, #1303): a
//! practice game runs on the player's device, so the server takes no word of the client's for its
//! result. It folds the claimed log from the setup its seed was issued for, and only a game that folds
//! to the claimed result pays (R1303000–R1303003). It re-runs no AI move (R2560000): an AI action is
//! held to its nonce and seat and reduced like any other, since practice pay is held to the day's Jlore
//! cap (R1483000). Every engine call is made under the catalog the caller set (R767000), and nothing
//! here reads a clock or the store.

use serde::{Deserialize, Serialize};

use jackioh_engine::{Action, SeatSummary};

use super::host::{PracticeEvent, PracticeHost, PracticeOutput};
use super::types::{PracticeConfig, PracticeRecord, PracticeResult};

/// Why a practice claim pays nothing, as the check finds it.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimRefusal {
    /// The log does not fold from the seed's setup: an action the engine refuses, one under the wrong
    /// nonce or seat, one after the end, or a setup that does not begin.
    DoesNotFold,
    /// The log folds to a game that has not ended.
    NotFinished,
    /// The game ended voided by a Glitch (R679), which records nothing (R1303000).
    Voided,
    /// The game ended, but not with the result claimed.
    ResultMismatch,
}

/// A claim that passed: what the game recorded (the human's actions the economy counts, the AI's moves,
/// the board the human took away), and what the quests read (§14.9).
#[derive(Clone, Debug, PartialEq)]
pub struct Checked {
    pub record: PracticeRecord,
    /// What the seat the human ended in did, as the engine's `summarize_game` reads it off the log
    /// (R376, R1304001); `None` when the fold reaches no result.
    pub seat: Option<SeatSummary>,
}

/// A practice claim checked: `log` folded from `config` on `catalog`, each action under its player's next
/// nonce and from the seat that player plays then (R677), to a finished game whose result for the human
/// is `claimed`. No AI action is re-run (R2560000).
pub fn check_claim(
    config: &PracticeConfig,
    catalog: &str,
    claimed: PracticeResult,
    log: &[Action],
) -> Result<Checked, ClaimRefusal> {
    let (mut host, _) =
        PracticeHost::start(config, catalog, None, 0).map_err(|_| ClaimRefusal::DoesNotFold)?;
    let (mut record, mut voided) = (None, false);
    for action in log {
        let event = PracticeEvent::Fold {
            action: action.clone(),
        };
        for output in host.step(event, 0) {
            match output {
                PracticeOutput::Failed { .. } => return Err(ClaimRefusal::DoesNotFold),
                PracticeOutput::Finished { record: finished } => record = Some(*finished),
                PracticeOutput::Voided { .. } => voided = true,
                _ => {}
            }
        }
    }
    if voided {
        return Err(ClaimRefusal::Voided);
    }
    let Some(record) = record else {
        return Err(ClaimRefusal::NotFinished);
    };
    if record.result != claimed {
        return Err(ClaimRefusal::ResultMismatch);
    }
    // The quests read what the game did off the same setup and log, as a ladder result reads its
    // match's.
    let seat =
        jackioh_engine::summarize_game(&host.fold_args()).map(|game| game.seats[record.human_seat].clone());
    Ok(Checked { record, seat })
}
