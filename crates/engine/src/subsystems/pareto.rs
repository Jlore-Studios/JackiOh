//! ME-PARETO (docs/meditative-set.md M5, MD-D28–MD-D30, R1125, R1126): the engine-side judge of
//! "the AI optimal move" and the Cane's attack.
//!
//! The judge is §10.7's scorer (`subsystems::scorer`), never `crates/ai` — the engine cannot depend
//! on it (CLAUDE.md rule 4), and the AI's own pick runs on a budget, a wall clock and a
//! determinization rng, so the server, WASM and a replay could disagree. It scores each opponent
//! play against their other playable cards, from their own concealed view, on the state before the
//! play. Ties count as optimal, and only plays are judged.
//!
//! While a card with `StaticFlags.judges_plays` acts on a player's field, `reduce` judges each
//! `play` the opponent makes and stores `state.play_judgement`; Pareto Optimality's trigger reads
//! it off its play's `cardResolved` and sends the remembered Cane. The verdict never reaches a view
//! (§10.8): the Cane's attack tells the opponent only that a better-scored card was in their hand.

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
/// score is at least every other playable card's, so ties count as optimal (MD-D28).
///
/// True when there is nothing to judge against — the viewer cannot play now, or the card is not
/// theirs to play — so no trigger fires on a verdict that was never earned.
pub fn judge_play(state: &GameState, player: PlayerId, instance_id: &str) -> bool {
    let Some(mut base) = crate::subsystems::scorer::dry_run_base(state, player) else {
        return true;
    };
    // The copy carries the last verdict along; a trigger answering a trial's resolution must not
    // read it as this play's (the trials play through `run_play_steps`, never `reduce`, so no new
    // verdict overwrites it there).
    base.play_judgement = None;
    let mut candidates: Vec<String> = Vec::new();
    for action in crate::reduce::legal_actions(state, player) {
        if let ActionBody::Play { instance_id: id, .. } = action
            && !candidates.contains(&id)
        {
            candidates.push(id);
        }
    }
    let Some(mine) =
        crate::subsystems::scorer::score_instance(state, player, instance_id, Some(&mut base))
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

/// MD-D30, R1126: the Cane's attack for Pareto Optimality's trigger. The Cane must be acting and its
/// controller's; in its backrow zone it animates (R383's move, home reserved), and when no open unit
/// zone takes it the attack does not happen. Then one forced attack on a random enemy it may attack
/// (R53). On its controller's turn, already a Unit, it just attacks.
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
        if !crate::animated::animate_card(
            ctx,
            &cane,
            crate::animated::AnimateOptions { position: None },
        ) {
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
