//! Pick N different entries of a list (Meditative #49.1 YileGPT Unleashed, MD-C23).
//!
//! Call to Chaos's Radiant draw (R423) made into a verb: N different entries of a list (R60),
//! drawn from the match rng as the effect resolves, resolved in the list's order and named to both
//! players before they resolve (R436). When N reaches the list's length it does every entry, drawing
//! nothing (MD-C23).
//!
//! The entries are Rust values, not JSON: each holds the effects it resolves into, so the argument
//! travels as a struct, never through `json_as`. The roll happens when the effect resolves, so the
//! rng cursor moves with the resolution and a replay that stops on a prompt in between still lines
//! up (§10.7). What was picked is the part's memo (picked indices as a JSON array): resuming builds
//! the same effects again and rolls nothing a second time. The announcement heads the part, so a
//! resumed part, which goes on after what it had already run, never announces it again (R436).
//!
//! Port pattern: `subsystems/call_to_chaos.rs` (`roll_chaos_effects`, `announce_roll`, `call_to_chaos`).

use serde_json::json;

use crate::resolve::lazy_part;
use crate::script::{Effect, EffectPart, Memo};
use crate::wire::GameEvent;

/// One entry of a pick-N list: the clause as the card prints it (what the roll names to both
/// players) and the effects it resolves into.
#[derive(Clone)]
pub struct RandomEntry {
    pub label: String,
    pub effects: Vec<Effect>,
}

/// `doRandomEntries`' argument: the list and how many different entries of it to do.
#[derive(Clone)]
pub struct DoRandomEntriesArgs {
    pub entries: Vec<RandomEntry>,
    pub count: i32,
}

/// R436: tell both players what was picked, before any of it resolves — the picked clauses by
/// their printed labels, in the order they resolve.
fn announce_pick(labels: Vec<String>) -> Effect {
    Effect::new("doRandomEntries:announce", move |ctx| {
        let (instance_id, def_id) = match ctx.live_self() {
            Some(card) => (card.id.clone(), card.def_id.clone()),
            None => (String::new(), ctx.def_id.clone().unwrap_or_default()),
        };
        let player = ctx.controller;
        ctx.sink.events.push(GameEvent::ChaosRolled {
            player,
            instance_id,
            def_id,
            effects: labels.clone(),
        });
    })
}

/// A roll kept across a pause (`EffectPart.memo`), read back defensively: it came through JSON.
fn picked_indices(memo: &Memo) -> Option<Vec<usize>> {
    let list = memo.as_ref()?.as_array()?;
    Some(
        list.iter()
            .filter_map(|index| index.as_u64().map(|index| index as usize))
            .collect(),
    )
}

/// Do `count` different entries of `entries`, in the list's order, named to both players first.
pub fn do_random_entries(args: DoRandomEntriesArgs) -> Effect {
    lazy_part("doRandomEntries", move |ctx, memo| {
        let picked: Vec<usize> = match picked_indices(memo) {
            Some(indices) => indices
                .into_iter()
                .filter(|index| *index < args.entries.len())
                .collect(),
            None => {
                let len = args.entries.len();
                let n = args.count.max(0) as usize;
                if n >= len {
                    // MD-C23: N at or above the list's length does every entry, with no rng draw.
                    (0..len).collect()
                } else {
                    let mut left: Vec<usize> = (0..len).collect();
                    let mut drawn = Vec::with_capacity(n);
                    for _ in 0..n {
                        let at = ctx.sink.rng.int(left.len() as i32) as usize;
                        if at < left.len() {
                            drawn.push(left.remove(at));
                        }
                    }
                    drawn.sort();
                    drawn
                }
            }
        };
        let mut effects = vec![announce_pick(
            picked
                .iter()
                .map(|index| args.entries[*index].label.clone())
                .collect(),
        )];
        for index in &picked {
            effects.extend(args.entries[*index].effects.clone());
        }
        EffectPart {
            effects,
            memo: Some(json!(picked)),
        }
    })
}
