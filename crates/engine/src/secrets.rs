//! ME-SECRET's record and its reads (Meditative MB05, R860–R865; docs/meditative-set.md, group
//! A's Systems, ME-SECRET): Mind Games files its declared mode as a secret record on the caster
//! (`state.secrets { id, owner, choice, revealed }`), and a Fortify Mind's guess is judged against
//! it as rock, paper, scissors.
//!
//! The choice reaches only its owner's view (`secret_views`, R860); the AI's `redact` drops it and
//! `determinize` samples it (R865). Nothing here mutates state: the verbs that write it live in
//! `effects::secret`.

use crate::play_steps;
use crate::state::{GameState, Resume, SecretRecord};
use crate::wire::{PlayerId, PredictOutcome, SecretChoice, SecretView};

/// The `delayed`/`memory` key a reward and its link carry the secret's id under (R861, R862).
pub const SECRET_KEY: &str = "secret";

/// The secret `id` names, when one is filed.
pub fn secret_of<'a>(state: &'a GameState, id: &str) -> Option<&'a SecretRecord> {
    state.secrets.as_ref()?.iter().find(|secret| secret.id == id)
}

/// R864: rock, paper, scissors — Attack beats Greed, Greed beats Defend, Defend beats Attack; the
/// same choice is no win and no loss.
pub fn judge(guess: SecretChoice, choice: SecretChoice) -> PredictOutcome {
    if guess == choice {
        return PredictOutcome::Same;
    }
    let won = matches!(
        (guess, choice),
        (SecretChoice::Attack, SecretChoice::Greed)
            | (SecretChoice::Greed, SecretChoice::Defend)
            | (SecretChoice::Defend, SecretChoice::Attack)
    );
    if won {
        PredictOutcome::Won
    } else {
        PredictOutcome::Lost
    }
}

/// R865: whether plays of `def_id` keep their declared modes secret.
pub fn plays_secretly(state: &GameState, def_id: &str) -> bool {
    let scripts = crate::scripts::script_of(state, def_id);
    scripts.base.secret_modes || scripts.radiant.secret_modes
}

/// R865: blank the modes of an in-flight play of a `secret_modes` card the resume's seat may not
/// read — the other seat's play, and its Echo repeat's fresh picks. Anything else is left alone.
pub fn scrub_secret_modes(state: &GameState, resume: &mut Resume, seat: PlayerId) {
    let Some(mut run) = play_steps::run_of(resume) else {
        return;
    };
    if run.player == seat || !plays_secretly(state, &run.def_id) {
        return;
    }
    run.modes.clear();
    if let Some(repeat) = run.repeat.as_mut() {
        repeat.modes.clear();
    }
    resume.data.insert(
        play_steps::RUN_KEY.to_string(),
        serde_json::to_value(&run).expect("a play run is plain JSON (§10.1)"),
    );
}

/// R860: the secrets `player` holds, as `viewer` reads them — the choice only for the owner, or for
/// anyone once revealed. `None` when none are held, so a game without secrets reads as before (D14).
pub fn secret_views(state: &GameState, player: PlayerId, viewer: PlayerId) -> Option<Vec<SecretView>> {
    let secrets = state.secrets.as_ref()?;
    let shown: Vec<SecretView> = secrets
        .iter()
        .filter(|secret| secret.owner == player)
        .map(|secret| SecretView {
            id: secret.id.clone(),
            choice: if player == viewer || secret.revealed == Some(true) {
                secret.choice
            } else {
                None
            },
        })
        .collect();
    if shown.is_empty() { None } else { Some(shown) }
}
