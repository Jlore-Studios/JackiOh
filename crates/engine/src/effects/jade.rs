//! ME-JADE (Meditative #39.2 Jade, #39.4 Red Jade, #39.5 Jade Beauty; docs/meditative-set.md M5):
//! each player's public Jade Counter, which only rises, so each crossing of `JADE_BEAUTY_AT` (summon a
//! Jade Beauty, R64, no Cry) and `JADE_ASCEND_AT` (make every Jade Beauty its player controls Radiant,
//! or summon a Radiant one) happens once a game, in that order (MD-C3, R962). A full row summons
//! nothing, and that threshold is spent all the same. `PlayerState.jade` starts absent (D14).

use serde::{Deserialize, Serialize};

use crate::config::{JADE_ASCEND_AT, JADE_BEAUTY_AT, JADE_BEAUTY_DEF_ID};
use crate::effects::radiant::{RadiantTarget, set_radiant};
use crate::effects::summon::{SummonArgs, summon};
use crate::effects::targets::{PlayerSpec, player_of};
use crate::script::Effect;
use crate::wire::{GameEvent, Row};
use crate::zones::{card_at, slots_of, standing_units_of};

/// `addJade`'s argument: how much the counter rises by, and whose (default the controller's own,
/// which is what a resolving Jade or Red Jade adds to, MD-C2).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct AddJadeArgs {
    pub amount: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// R961, R962, MD-C2, MD-C3: raise a player's Jade Counter by `amount` (a public `jadeChanged`); an add
/// of 0 or less changes nothing.
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
            // R1081000: a Jade Beauty standing in the backrow is one of them, after the unit row's.
            let beauties: Vec<String> = slots_of(player, Row::Units)
                .into_iter()
                .filter_map(|slot| card_at(ctx.sink.state, slot).cloned())
                .chain(standing_units_of(ctx.sink.state, player).into_iter().cloned())
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
