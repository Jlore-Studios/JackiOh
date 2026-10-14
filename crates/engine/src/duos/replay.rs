//! A Duos game's replay (#936 B24): `(options, log)` folds to the state live play reached, and
//! `duos_hash` makes two folds comparable, with the same canonical JSON and FNV-1a a 1v1 hash uses
//! (`replay::canonical`, `replay::fnv1a32_utf16`). Unlike a 1v1 fold, a refused action stops the
//! fold: a Duos log is what a server accepted, so a refusal in it is a fold that went wrong.

use serde::{Deserialize, Serialize};

use crate::replay::{canonical, fnv1a32_utf16};
use crate::wire::{DuosAction, DuosCreateOptions};

use super::reduce::{duos_begin, duos_reduce};
use super::state::{DuosState, duos_create};

/// What a Duos fold starts from: the setup and the accepted actions, in order.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DuosReplayInput {
    pub options: DuosCreateOptions,
    pub log: Vec<DuosAction>,
}

/// Where a Duos fold got to: the state, its hash, and the index of the action it was refused at.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DuosReplayResult {
    pub state: DuosState,
    pub hash: String,
    pub refused_at: Option<usize>,
}

/// FNV-1a over the canonical JSON of the whole state, both duels' nonce logs included (they are
/// what the same log folds to, so they hash alike).
pub fn duos_hash(state: &DuosState) -> String {
    let value = match serde_json::to_value(state) {
        Ok(value) => value,
        Err(error) => panic!("duos_hash: a DuosState did not serialise: {error}"),
    };
    fnv1a32_utf16(&canonical(&value))
}

/// Create, begin, then apply the log in order, stopping at the first refused action. Panics as
/// `duos_create` does on a setup `create_game` refuses.
pub fn duos_fold(input: &DuosReplayInput) -> DuosReplayResult {
    let mut state = duos_begin(&duos_create(&input.options)).state;
    let mut refused_at: Option<usize> = None;
    for (at, action) in input.log.iter().enumerate() {
        let result = duos_reduce(&state, action);
        if result.error.is_some() {
            refused_at = Some(at);
            break;
        }
        state = result.state;
    }
    let hash = duos_hash(&state);
    DuosReplayResult {
        state,
        hash,
        refused_at,
    }
}
