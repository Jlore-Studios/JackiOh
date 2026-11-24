//! R4592000: add copies of played cards to hand (Infinite Assault, Scraper).
//!
//! Adds copies of cards a player played face-up this game to the running card's controller's hand.

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::effects::targets::{PlayerSpec, player_of};
use crate::play_history::{FaceUpPlay, last_plays, plays_last_turn};
use crate::prelude::json_as;
use crate::script::Effect;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct AddPlayCopiesArgs {
    pub of: PlayerSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_turn: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copies: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_mod: Option<i32>,
}

pub fn add_play_copies(args: AddPlayCopiesArgs) -> Effect {
    Effect::new("addPlayCopies", move |ctx| {
        let whose = player_of(ctx, args.of);
        let picked: Vec<FaceUpPlay> = if args.last_turn == Some(true) {
            let plays = plays_last_turn(ctx.sink.state, whose);
            if plays.is_empty() {
                Vec::new()
            } else if plays.len() == 1 {
                plays
            } else {
                match ctx.rng.pick(&plays).cloned() {
                    Some(play) => vec![play],
                    None => Vec::new(),
                }
            }
        } else {
            let count = args.last.unwrap_or(1).max(0) as usize;
            last_plays(ctx.sink.state, whose, count)
        };

        let copies = args.copies.unwrap_or(1).max(0);
        for pick in &picked {
            for _ in 0..copies {
                let mut payload = json!({
                    "defId": pick.def_id,
                    "radiant": pick.radiant,
                });
                if let Some(cost_mod) = args.cost_mod {
                    payload["costMod"] = json!(cost_mod);
                }
                if pick.chinese == Some(true) {
                    payload["chinese"] = json!(true);
                }
                let effect = crate::effects::add_to_hand::add_to_hand(json_as(payload));
                (effect.apply)(ctx);
            }
        }
    })
}
