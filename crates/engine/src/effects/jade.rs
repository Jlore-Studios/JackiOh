//! ME-JADE (Meditative #39.2 Jade, #39.4 Red Jade, #39.5 Jade Beauty; docs/meditative-set.md M5,
//! Group C's systems): each player's public Jade Counter, which only rises.
//!
//! `PlayerState.jade` starts absent (D14: a game without a Jade serializes as before) and `add_jade`
//! writes it. Inside the same add, the counter's crossings of `JADE_BEAUTY_AT` (5: summon a Jade
//! Beauty, R64, no Cry) and `JADE_ASCEND_AT` (10: make every Jade Beauty its player controls
//! Radiant, or summon a Radiant one when they control none) run in that order (MD-C3, R962). Each
//! crossing happens once a game, because the counter never falls. A full row summons nothing, and
//! that threshold is spent all the same.

use serde::{Deserialize, Serialize};

use crate::config::{JADE_ASCEND_AT, JADE_BEAUTY_AT, JADE_BEAUTY_DEF_ID};
use crate::effects::radiant::{RadiantTarget, set_radiant};
use crate::effects::summon::{SummonArgs, summon};
use crate::effects::targets::{PlayerSpec, player_of};
use crate::script::Effect;
use crate::wire::{GameEvent, Row};
use crate::zones::{card_at, slots_of};

/// `addJade`'s argument: how much the counter rises by, and whose (default the controller's own,
/// which is what a resolving Jade or Red Jade adds to, MD-C2).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AddJadeArgs {
    pub amount: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// R961, R962, MD-C2, MD-C3: raise a player's Jade Counter by `amount` and report it with the
/// public `jadeChanged` event. An add of 0 or less changes and reports nothing.
pub fn add_jade(args: AddJadeArgs) -> Effect {
    Effect::new("addJade", move |ctx| {
        if args.amount <= 0 {
            return;
        }
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let before = ctx.sink.state.players[player].jade.unwrap_or(0);
        let after = before + args.amount;
        ctx.sink.state.players[player].jade = Some(after);
        ctx.sink
            .events
            .push(GameEvent::JadeChanged { player, value: after });
        // MD-C3: 5 first, then 10, inside the same add.
        if before < JADE_BEAUTY_AT && JADE_BEAUTY_AT <= after {
            let beauty = summon(SummonArgs {
                player: args.player,
                def_id: Some(JADE_BEAUTY_DEF_ID.to_string()),
                ..SummonArgs::default()
            });
            (beauty.apply)(ctx);
        }
        if before < JADE_ASCEND_AT && JADE_ASCEND_AT <= after {
            let beauties: Vec<String> = slots_of(player, Row::Units)
                .into_iter()
                .filter_map(|slot| card_at(ctx.sink.state, slot).cloned())
                .filter(|card| card.def_id == JADE_BEAUTY_DEF_ID)
                .map(|card| card.id)
                .collect();
            if beauties.is_empty() {
                let beauty = summon(SummonArgs {
                    player: args.player,
                    def_id: Some(JADE_BEAUTY_DEF_ID.to_string()),
                    radiant: Some(true),
                    ..SummonArgs::default()
                });
                (beauty.apply)(ctx);
            } else {
                for instance_id in beauties {
                    let ascend = set_radiant(RadiantTarget {
                        instance_id: Some(instance_id),
                        target: None,
                    });
                    (ascend.apply)(ctx);
                }
            }
        }
    })
}
