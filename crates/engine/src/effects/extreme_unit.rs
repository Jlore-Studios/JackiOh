//! The cheapest or the highest-Attack Unit of a side (SPEC §6.3, R4588001; Infinite Assault #86
//! Replica Hatchery, #94 Batch Monarch): the tied best read off the board, one match-rng pick of a tie
//! as the effect resolves. A hook may not roll dice, as `ctx.rng` advances `rngCursor`, which is
//! state, so the pick happens here, in the part (`random_picks.rs` does the same).

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::resolve::lazy_part;
use crate::script::{Effect, EffectPart};
use crate::state::{CardInstance, GameState};
use crate::wire::PlayerId;
use crate::zones::active_units_of;

use super::each::ForEachCardEach;
use super::targets::{PlayerSpec, player_of};

/// `forExtremeUnit`'s pick: the cheapest Unit, or the highest-Attack one.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum UnitExtreme {
    Cheapest,
    HighestAttack,
}

/// R4588001: the tied best Units acting on `player`'s side (R13) — the cheapest by what each costs
/// now (R396), the mightiest by shown Attack — in board order; none when they hold no Unit.
pub fn extreme_units_of(
    state: &GameState,
    player: PlayerId,
    extreme: UnitExtreme,
) -> Vec<CardInstance> {
    let scored: Vec<(i32, CardInstance)> = active_units_of(state, player)
        .into_iter()
        .map(|unit| {
            let score = match extreme {
                UnitExtreme::Cheapest => -crate::mana::cost_now(state, unit),
                UnitExtreme::HighestAttack => crate::layers::unit_view(state, unit).attack,
            };
            (score, unit.clone())
        })
        .collect();
    let Some(best) = scored.iter().map(|(score, _)| *score).max() else {
        return Vec::new();
    };
    scored
        .into_iter()
        .filter(|(score, _)| *score == best)
        .map(|(_, unit)| unit)
        .collect()
}

/// `forExtremeUnit`'s argument.
#[derive(Clone)]
pub struct ForExtremeUnitArgs {
    pub extreme: UnitExtreme,
    pub player: Option<PlayerSpec>,
    pub each: ForEachCardEach,
}

/// R4588001: `each(id)` for the cheapest or the highest-Attack Unit, read once as the list reaches
/// it (`resolve::lazy_part`): the tied one outright, one match-rng pick of a tie, nothing when no
/// Unit answers. The pick is the part's memo, so a pause inside it resumes over the Unit it began
/// with (R113).
pub fn for_extreme_unit(args: ForExtremeUnitArgs) -> Effect {
    lazy_part("forExtremeUnit", move |ctx, memo| {
        let picked: Option<String> = match memo {
            Some(Value::String(id)) => Some(id.clone()),
            Some(_) => None,
            None => {
                let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
                let tied = extreme_units_of(ctx.state, player, args.extreme);
                if tied.len() == 1 {
                    tied.first().map(|unit| unit.id.clone())
                } else if tied.is_empty() {
                    None
                } else {
                    ctx.rng.pick(&tied).map(|unit| unit.id.clone())
                }
            }
        };
        EffectPart {
            effects: picked.iter().map(|id| (args.each)(id)).collect(),
            memo: Some(json!(picked)),
        }
    })
}
