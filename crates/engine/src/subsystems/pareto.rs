//! ME-PARETO (docs/meditative-set.md M5, MD-D28–MD-D30, R1125, R1126): the engine-side judge of
//! "the AI optimal move" and the Cane's attack.
//!
//! The judge is §10.7's scorer (`subsystems::scorer`), never `crates/ai`: the engine cannot depend on it
//! (CLAUDE.md rule 4), and the AI's pick runs on a budget, a clock and a determinization rng, so the
//! server, WASM and a replay could disagree. It scores only plays, from the player's own concealed view
//! before the play. `reduce` stores the verdict (`state.play_judgement`, `StaticFlags.judges_plays`),
//! which never reaches a view (§10.8): the Cane tells only that a better-scored card was in hand.
//!
//! R1844001 (Stinkyscape #147 IN33DH3, #147.2 Wardrum Blob): the Pareto hint marks the hand cards the
//! judge would call optimal now (`pareto_hint_ids`), so a hinted play never sets off an opposing Pareto
//! Optimality; it reads only what `judge_play` reads, their concealed copy.

use serde::{Deserialize, Serialize};

use crate::effects::combat::{ForcedAttackRandomArgs, forced_attack_random};
use crate::effects::targets::TargetSpec;
use crate::script::Effect;
use crate::state::{GameState, PlayJudgement, find_instance};
use crate::wire::{ActionBody, PlayerId, opponent_of};

/// Whether a card of `player`'s opponent judges their plays now.
pub fn watching(state: &GameState, player: PlayerId) -> bool {
    crate::query::acting_with_flag(state, opponent_of(player), |flags| flags.judges_plays)
}

/// The judge's verdict on the play `player` is about to make of `instance_id`: true when its card's
/// score is at least every other playable card's (ties optimal, MD-D28), or when there is nothing to
/// judge against, so no trigger fires on a verdict never earned.
pub fn judge_play(state: &GameState, player: PlayerId, instance_id: &str) -> bool {
    let Some(mut base) = crate::subsystems::scorer::dry_run_base(state, player) else {
        return true;
    };
    // The copy carries the last verdict, which a trial's trigger must not read as this play's (trials
    // run through `run_play_steps`, never `reduce`, so nothing overwrites it there).
    base.play_judgement = None;
    let mut candidates: Vec<String> = Vec::new();
    for action in crate::reduce::legal_actions(state, player) {
        if let ActionBody::Play { instance_id: id, .. } = action
            && !candidates.contains(&id)
        {
            candidates.push(id);
        }
    }
    let Some(mine) = crate::subsystems::scorer::score_instance(state, player, instance_id, Some(&mut base))
    else {
        return true;
    };
    for id in &candidates {
        if id == instance_id {
            continue;
        }
        let Some(other) = crate::subsystems::scorer::score_instance(state, player, id, Some(&mut base))
        else {
            continue;
        };
        if other.score > mine.score {
            return false;
        }
    }
    true
}

/// R1844001: whether `player` is shown the Pareto hint now — on their own turn, the one the verb
/// showed it for, or while a card of theirs that hints plays acts on their field.
pub fn pareto_hint_active(state: &GameState, player: PlayerId) -> bool {
    state.active == player
        && (state.players[player].pareto_hint == Some(state.turn)
            || crate::query::acting_with_flag(state, player, |flags| flags.hints_plays))
}

/// R1844001: the cards `player` may play now that `judge_play` would call optimal (ties share the
/// mark), in `legal_actions`' order; none while the hint is off.
pub fn pareto_hint_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    if !pareto_hint_active(state, player) {
        return Vec::new();
    }
    let Some(mut base) = crate::subsystems::scorer::dry_run_base(state, player) else {
        return Vec::new();
    };
    // As `judge_play`'s copy: the last verdict is no trial's.
    base.play_judgement = None;
    let mut scored: Vec<(String, f64)> = Vec::new();
    for action in crate::reduce::legal_actions(state, player) {
        if let ActionBody::Play { instance_id: id, .. } = action
            && !scored.iter().any(|(held, _)| *held == id)
            && let Some(card) = crate::subsystems::scorer::score_instance(state, player, &id, Some(&mut base))
        {
            scored.push((id, card.score));
        }
    }
    let best = scored
        .iter()
        .map(|(_, score)| *score)
        .fold(f64::NEG_INFINITY, f64::max);
    scored
        .into_iter()
        .filter(|(_, score)| *score >= best)
        .map(|(id, _)| id)
        .collect()
}

/// The verdict on the last judged play, if one was stored.
pub fn judgement_of(state: &GameState) -> Option<&PlayJudgement> {
    state.play_judgement.as_ref()
}

/// `cane_strike`'s argument: the Cane Pareto summoned, by id.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaneArgs {
    pub cane: String,
}

/// MD-D30, R1126: the Cane's attack for Pareto Optimality's trigger, if it acts and is its controller's:
/// out of its backrow zone it animates (R383's move, home reserved) or, with no open unit zone, does not
/// attack; then one forced attack on a random enemy it may attack (R53).
pub fn cane_strike(args: CaneArgs) -> Effect {
    Effect::new("caneStrike", move |ctx| {
        let Some(cane) = find_instance(ctx.state, &args.cane).cloned() else {
            return;
        };
        if cane.controller != ctx.controller {
            return;
        }
        if !crate::zones::acts_on_field(ctx.state, &cane) {
            return;
        }
        if !crate::animated::animate_card(ctx, &cane, crate::animated::AnimateOptions { position: None }) {
            return;
        }
        let effect = forced_attack_random(ForcedAttackRandomArgs {
            attacker: TargetSpec::Instance {
                instance_id: cane.id.clone(),
            },
            among: None,
            times: None,
        });
        (effect.apply)(ctx);
    })
}

/// MD-D30, R1126: after that combat's state check, the Cane returns to its home zone — unless it is
/// now its controller's turn, when it stays a Unit.
pub fn cane_return(args: CaneArgs) -> Effect {
    Effect::new("caneReturn", move |ctx| {
        if ctx.state.active == ctx.controller {
            return;
        }
        let Some(cane) = find_instance(ctx.state, &args.cane).cloned() else {
            return;
        };
        if cane.controller != ctx.controller {
            return;
        }
        let _ = crate::animated::return_home(ctx, &cane);
    })
}
