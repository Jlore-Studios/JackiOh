//! T-AI-3 Hallucination's verb (SPEC §8.7 row T-AI-3; R57, R60, R129, R385): copies of `count`
//! different random cards of a player's deck, added to the running card's controller's hand as new
//! cards they own, each carrying its source's definition, radiant flag, `statsOverride` and `tuning`
//! (R57's copy, R386) — never a Brittle count or a cost rider — the source staying where it is. A copy
//! that reaches the hand is given Brittle `brittle` (B3.3); one the hand cap burns is not (§2.4, R586).
//!
//! R586: the copies go to the hand in the order drawn, never the deck's, so their order says nothing of where
//! the sources lay (§9.1). A deck of no more than `count` cards gives a copy of each with no draw
//! (R129), ordered by definition id for the same reason. The other player reads only that cards
//! reached the hand (`addedToHand` under the sentinel, R97); the deck they came from is untouched.
//!
//! Port of `packages/engine/src/effects/libraryCopies.ts`.

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::draw::{AddToHandOutcome, add_to_hand};
use crate::effects::brittle::give_brittle;
use crate::effects::targets::{PlayerSpec, player_of};
use crate::prelude::json_as;
use crate::script::Effect;
use crate::state::{CardInstance, new_instance};
use crate::wire::Zone;

/// `addLibraryCopies`'s arguments: whose deck the copies come `of`, how many, and the Brittle a copy
/// that reaches the hand is given.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddLibraryCopiesArgs {
    pub of: PlayerSpec,
    pub count: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brittle: Option<i32>,
}

pub fn add_library_copies(args: AddLibraryCopiesArgs) -> Effect {
    Effect::new("addLibraryCopies", move |ctx| {
        let whose = player_of(ctx, args.of);
        let count = args.count.max(0);
        let library = ctx.sink.state.players[whose].library.clone();
        let picked: Vec<CardInstance> = if library.len() <= count as usize {
            // R586, R129: every card, with no draw, ordered by definition id (a stable sort, as TS's).
            let mut sorted = library;
            sorted.sort_by(|a, b| a.def_id.cmp(&b.def_id));
            sorted
        } else {
            ctx.sink.rng.shuffle(&library).into_iter().take(count as usize).collect()
        };
        let controller = ctx.controller;
        for source in &picked {
            let mut copy = new_instance(&mut *ctx.state, &source.def_id, controller, Zone::Hand { player: controller });
            copy.radiant = source.radiant;
            if let Some(stats) = source.stats_override {
                copy.stats_override = Some(stats);
            }
            // `copyTuning`: a deep copy of the source's tuning, absent when it has none.
            if let Some(tuning) = source.tuning.clone() {
                copy.tuning = Some(tuning);
            }
            let landed = add_to_hand(ctx, &mut copy);
            if matches!(landed, AddToHandOutcome::Hand)
                && let Some(brittle) = args.brittle
            {
                let effect = give_brittle(json_as(json!({ "instanceId": copy.id, "n": brittle })));
                (effect.apply)(ctx);
            }
        }
    })
}
