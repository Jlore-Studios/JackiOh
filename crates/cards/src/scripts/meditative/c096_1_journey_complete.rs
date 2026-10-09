//! M #96.1 Journey Complete (SPEC §8.8 row 96.1, §7; §6.2 Cast on draw, §6.3 Add to hand, Make Radiant;
//! R40, R58, R70, R746, R766, R1248): (2) Spell, Token, printed Rare.
//!
//! Base:    "Cast on draw: Return the cards that went on this journey from exile to hand. They become
//!          Radiant."
//! Radiant: "Cast on draw: Return the cards that went on this journey from exile to hand. They become
//!          Radiant and cost ({discount}) less."
//! Engine: §6.2 Cast on draw (`static_flags.cast_on_draw`), a cast (R40, R70) under R58's chain cap. It
//! reads the `memory.journey` M #96 wrote on it (R1246) and, for each id in that order whose card is
//! still in an exile pile, `add_to_hand` moves it to its owner's hand (R746), Radiant (§6.3 Make
//! Radiant, R74), keeping its `tuning`; on the Radiant face it costs `discount` less, a price that lands
//! after R766 reset it in exile. The hand cap burns the rest (§2.4). One that remembers nothing — made
//! by any other card, a copy, or one that reached a graveyard or an exile pile (R766) — does nothing
//! (R1248).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-096-1";

/// What M #96 Meditative Journey writes (R1246).
const JOURNEY_KEY: &str = "journey";

fn journey_complete(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |ctx| {
            let went: Vec<String> = ctx
                .live_self()
                .and_then(|me| me.memory.get(JOURNEY_KEY))
                .and_then(Value::as_array)
                .map(|ids| ids.iter().filter_map(Value::as_str).map(str::to_string).collect())
                .unwrap_or_default();
            let discount = param(&*ctx, "discount");
            went.into_iter()
                .filter(|id| {
                    find_instance(ctx.state, id).is_some_and(|card| card.zone.z() == ZoneName::Exile)
                })
                .map(|id| {
                    let mut args = json!({
                        "instance": { "of": "instance", "instanceId": id },
                        "radiant": true,
                    });
                    if radiant {
                        args["costMod"] = json!(-discount);
                    }
                    add_to_hand(json_as(args))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: journey_complete(false),
        radiant: journey_complete(true),
    }
}
