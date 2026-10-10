//! ME-SECRET's three verbs (Meditative MB05, R860–R865; docs/meditative-set.md, group A's
//! Systems, ME-SECRET): Mind Games keeps its declared mode as a secret record (`keep_secret`), a
//! Fortify Mind's guess is judged against the linked secret (`guess_secret`), and the reward
//! resolves the secret away (`resolve_secret`). Like `effects::translate`, they push events onto
//! `ctx.events` and write `ctx.state`, so the card scripts stay lists of effects (CLAUDE.md rule 5).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::add_to_hand::{AddToHandArgs, add_to_hand};
use super::delay::{DelayArgs, DelayAt, DelayPlayer, delay};
use super::targets::PlayerSpec;
use crate::prompts::RESUME_HOOK;
use crate::script::Effect;
use crate::secrets::{SECRET_KEY, judge, secret_of};
use crate::state::SecretRecord;
use crate::wire::{GameEvent, Phase, PredictOutcome, SecretChoice};

/// The card `keep_secret` hands over with the secret's id riding it (R862).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SecretLink {
    pub def_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_override: Option<i32>,
}

/// `keep_secret`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KeepSecretArgs {
    pub choice: SecretChoice,
    pub step: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<SecretLink>,
}

/// `guess_secret`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GuessSecretArgs {
    pub secret_id: String,
    pub guess: SecretChoice,
}

/// `resolve_secret`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResolveSecretArgs {
    pub secret_id: String,
}

/// R860, R861: file the declared choice as a secret record on the caster, tell both players only
/// that a secret exists, arm the reward at the caster's next start of turn, and hand the linked
/// card to its player carrying the secret's id. The id is minted off `next_id` like an instance's,
/// so replays mint the same one (D14: the counter moves only when a secret is kept).
pub fn keep_secret(args: KeepSecretArgs) -> Effect {
    Effect::new("keepSecret", move |ctx| {
        let controller = ctx.controller;
        let id = format!("secret-{}", ctx.state.next_id);
        ctx.state.next_id += 1;
        ctx.state.secrets.get_or_insert_with(Vec::new).push(SecretRecord {
            id: id.clone(),
            owner: controller,
            choice: Some(args.choice),
            revealed: None,
        });
        ctx.events.push(GameEvent::SecretChosen {
            player: controller,
            secret_id: id.clone(),
        });
        // R861: the reward is the caster's `next` start-of-turn delayed effect — after the refresh,
        // before the triggers and the draw (R62's delayed step) — re-entering the card's `resume`
        // step table at `step`, carrying only the secret's id, never the choice.
        let mut data = IndexMap::new();
        data.insert(SECRET_KEY.to_string(), Value::String(id.clone()));
        (delay(DelayArgs {
            at: DelayAt {
                phase: Phase::Start,
                player: DelayPlayer::SelfSide,
            },
            step: args.step.clone(),
            hook: Some(RESUME_HOOK.to_string()),
            data: Some(data),
            next: Some(true),
            watch: None,
            mark: None,
            hand_watch: None,
        })
        .apply)(ctx);
        // R862: the linked card rides the secret's id in its memory, so the guess it declares is
        // judged against the secret that made it.
        if let Some(link) = &args.link {
            let mut memory = IndexMap::new();
            memory.insert(SECRET_KEY.to_string(), Value::String(id.clone()));
            (add_to_hand(AddToHandArgs {
                def_id: Some(link.def_id.clone()),
                instance: None,
                player: Some(link.player.unwrap_or(PlayerSpec::Enemy)),
                radiant: None,
                cost_override: link.cost_override,
                cost_mod: None,
                temporary: None,
                chinese: None,
                lucky: None,
                copy_of: None,
                memory: Some(memory),
            })
            .apply)(ctx);
        }
    })
}

/// R862, R864: judge `guess` against the secret `secret_id` names. A secret with no record, or
/// none filed, is nothing to judge: no event, no penalty. Otherwise the secret is revealed once —
/// `secretRevealed` when it was still hidden — and `predicted` is public. A winning guess removes
/// the secret and its reward's delayed effect; a tie or a loss leaves the reward to land, both
/// players now reading the choice.
pub fn guess_secret(args: GuessSecretArgs) -> Effect {
    Effect::new("guessSecret", move |ctx| {
        let controller = ctx.controller;
        let Some(record) = secret_of(&ctx.state, &args.secret_id) else {
            return;
        };
        let (owner, choice) = (record.owner, record.choice);
        let Some(choice) = choice else {
            return;
        };
        let outcome = judge(args.guess, choice);
        let hidden = record.revealed != Some(true);
        if hidden {
            ctx.events.push(GameEvent::SecretRevealed {
                player: owner,
                secret_id: args.secret_id.clone(),
                choice,
            });
        }
        ctx.events.push(GameEvent::Predicted {
            player: controller,
            secret_id: args.secret_id.clone(),
            guess: args.guess,
            outcome,
        });
        let Some(secrets) = ctx.state.secrets.as_mut() else {
            return;
        };
        if outcome == PredictOutcome::Won {
            secrets.retain(|secret| secret.id != args.secret_id);
            if secrets.is_empty() {
                ctx.state.secrets = None;
            }
            // R864: the reward's delayed effect goes with the secret — every entry keyed to it.
            let secret_id = json!(args.secret_id);
            ctx.state
                .delayed
                .retain(|entry| entry.resume.data.get(SECRET_KEY) != Some(&secret_id));
        } else if let Some(secret) = secrets.iter_mut().find(|secret| secret.id == args.secret_id) {
            secret.revealed = Some(true);
        }
    })
}

/// R861: resolve the secret `secret_id` names away — revealing it first unless a judged Fortify
/// Mind already did. A secret with no record is already gone: nothing happens.
pub fn resolve_secret(args: ResolveSecretArgs) -> Effect {
    Effect::new("resolveSecret", move |ctx| {
        let Some(record) = secret_of(&ctx.state, &args.secret_id) else {
            return;
        };
        let (owner, choice, hidden) = (record.owner, record.choice, record.revealed != Some(true));
        if hidden && let Some(choice) = choice {
            ctx.events.push(GameEvent::SecretRevealed {
                player: owner,
                secret_id: args.secret_id.clone(),
                choice,
            });
        }
        if let Some(secrets) = ctx.state.secrets.as_mut() {
            secrets.retain(|secret| secret.id != args.secret_id);
            if secrets.is_empty() {
                ctx.state.secrets = None;
            }
        }
    })
}
