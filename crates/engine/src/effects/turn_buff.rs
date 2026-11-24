//! "+N Attack and <keywords> this turn" (R4587000, IA #63 Nitrous Injector): a layer-4
//! Attack gain and granted keywords that hold only while their `turn` is `state.turn` (`layers.rs`
//! reads them, `stats_with_buffs` too). Gone at the next turn; R78's reset clears it, a copy keeps
//! it (R57), and a Vanilla takes its Attack while its keywords stay (R781003).

use serde::{Deserialize, Serialize};

use super::targets::{TargetSpec, resolve_target};
use crate::damage::DamageTarget;
use crate::script::Effect;
use crate::state::{CardInstance, GameState, TurnBuff, find_instance_mut};
use crate::wire::{GameEvent, Keyword, ZoneName};

/// `buffThisTurn`'s argument: `{ target }` with an Attack gain and keywords for this turn.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct BuffThisTurnArgs {
    pub target: TargetSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attack: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<Keyword>>,
}

/// The live card's this-turn buff while its turn is now, else none.
pub fn turn_buff_of<'a>(state: &GameState, card: &'a CardInstance) -> Option<&'a TurnBuff> {
    card.turn_buff
        .as_ref()
        .filter(|buff| buff.turn == state.turn)
}

/// "+N Attack and <keywords> this turn" on one unit on the field: adds to the Attack held for this
/// turn and pushes each keyword it lacks, stacking within the turn (R4587000).
pub fn buff_this_turn(args: BuffThisTurnArgs) -> Effect {
    Effect::new("buffThisTurn", move |ctx| {
        let Some(DamageTarget::Unit { instance }) = resolve_target(ctx, &args.target) else {
            return;
        };
        if instance.zone.z() != ZoneName::Field {
            return;
        }
        let attack = args.attack.unwrap_or(0);
        let keywords = args.keywords.clone().unwrap_or_default();
        if attack == 0 && keywords.is_empty() {
            return;
        }
        let now = ctx.state.turn;
        if let Some(live) = find_instance_mut(ctx.sink.state, &instance.id) {
            let mut held = live.turn_buff.take().filter(|buff| buff.turn == now).unwrap_or(TurnBuff {
                turn: now,
                ..TurnBuff::default()
            });
            held.attack += attack;
            for keyword in &keywords {
                if !held.keywords.iter().any(|held| held.kind() == keyword.kind()) {
                    held.keywords.push(keyword.clone());
                }
            }
            live.turn_buff = Some(held);
        }
        if attack != 0 {
            ctx.sink.events.push(GameEvent::Buffed {
                instance_id: instance.id.clone(),
                attack,
                health: 0,
            });
        }
        for keyword in &keywords {
            ctx.sink.events.push(GameEvent::KeywordGranted {
                instance_id: instance.id.clone(),
                keyword: keyword.clone(),
                lost: None,
            });
        }
    })
}
