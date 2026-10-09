//! M #96 Meditative Journey (SPEC §8.8 row 96; §6.3 Exile, Shuffle into; R11, R80, R113, R311,
//! R1246, R1247): (2) Spell, Rare.
//!
//! Base:    "Choose up to {cards|card|cards} in your hand to go on a journey: exile them and shuffle a
//!          Journey Complete into your deck."
//! Radiant: "Choose up to {cards|card|cards} in your hand to go on a journey: exile them and shuffle a
//!          Radiant Journey Complete into your deck. Draw {draw|card|cards}."
//! Engine: as the Spell resolves, a `pick` prompt of up to `cards` cards of your hand (`choose_pick`,
//! the chooser's alone, §10.8; `cards` tunes, so it is a resolution prompt, R386). Its step exiles each
//! picked card in the order picked (a unit-token card ceases to exist instead, R11), then shuffles one
//! Journey Complete (M #96.1) into your deck, Radiant on the Radiant face (R1247), its `memory.journey`
//! the picked ids in that order (R1246); the shuffle-in is public and recorded in your list (R311), and
//! a full deck refuses it with the cards staying in exile (R80). With nothing picked nothing happens.
//! The Radiant face then draws `draw`, after the step (R113), or at once with an empty hand.

use jackioh_engine::effects::choose_pick;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-096";

/// M #96.1 Journey Complete, which reads the same key (R1246).
const JOURNEY_COMPLETE: &str = "meditative-096-1";
const JOURNEY_KEY: &str = "journey";
const STEP: &str = "journey";

fn journey(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut effects = vec![choose_pick(json_as(json!({
                "step": STEP,
                "from": [{ "zone": "hand" }],
                "max": param(&*ctx, "cards"),
                "prompt": "Choose cards in your hand to go on a journey",
            })))];
            if radiant {
                effects.push(draw(json_as(json!({ "count": param(&*ctx, "draw") }))));
            }
            effects
        })),
        resume: IndexMap::from([(
            STEP,
            hook(move |ctx| {
                let picked: Vec<String> = ctx
                    .targets
                    .iter()
                    .filter_map(|selection| match selection {
                        Selection::Instance { instance_id } => Some(instance_id.clone()),
                        _ => None,
                    })
                    .collect();
                if picked.is_empty() {
                    return Vec::new();
                }
                let mut effects: Vec<Effect> = picked
                    .iter()
                    .map(|id| {
                        exile(json_as(
                            json!({ "target": { "of": "instance", "instanceId": id } }),
                        ))
                    })
                    .collect();
                effects.push(shuffle_into(json_as(json!({
                    "defId": JOURNEY_COMPLETE,
                    "count": 1,
                    "radiant": radiant,
                    "memory": { JOURNEY_KEY: picked },
                }))));
                effects
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: journey(false),
        radiant: journey(true),
    }
}
