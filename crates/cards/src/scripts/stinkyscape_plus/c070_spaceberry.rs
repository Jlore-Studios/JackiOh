//! SPEC §8.12 S+ #70 Spaceberry, a card of the Stinkyscape+ set (R1420, R3535000). (1) Spell, Fruit, Epic.
//!   Base:    "Cast on draw: Shuffle {draw} Cast on draw into your deck."
//!   Radiant: "Cast on draw: Shuffle {draw} Radiant Cast on draw into your deck."
//!
//! Cast on draw (§6.2, `StaticFlags.cast_on_draw`). R3566007: `shuffle_random_from_catalog` draws
//! {draw} cards from the `castOnDraw` pool: the non-token cards of the shipped sets whose script casts on
//! draw (R380, R382, R1420), never this card (R387), so it cannot chain forever. The Radiant face shuffles
//! Radiant ones.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-070";

fn spaceberry(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |ctx| {
            vec![shuffle_random_from_catalog(json_as(json!({
                "query": { "castOnDraw": true },
                "count": param(ctx, "draw"),
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: spaceberry(false),
        radiant: spaceberry(true),
    }
}

// S+ #70 Spaceberry — SPEC §8.12 row 70 (R3566007).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const FILLER: &str = "core-005";
    /// "Shuffle {draw} …": the declared `draw`.
    const DRAWN: usize = 3;

    fn drawn(seed: &str, radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [FILLER], "library": [{ "def": ID, "radiant": radiant }, FILLER, FILLER], "field": ["core-019"] },
            "p2": { "hand": [FILLER], "field": ["core-019"], "library": filler(4) },
        }));
        s.start_turn();
        s
    }

    /// The cards the Spaceberry shuffled in, by their `shuffledIn` events: they may be drawn again at
    /// once (R58, R70: a card cast on draw is replaced by the next draw), so the library no longer holds them.
    fn shuffled(s: &Scenario) -> Vec<CardInstance> {
        s.events()
            .iter()
            .filter(|event| event.event_type().as_str() == "shuffledIn")
            .map(crate::js)
            .filter_map(|event| {
                event["instanceId"]
                    .as_str()
                    .and_then(|id| find_instance(s.state(), id).cloned())
            })
            .collect()
    }

    #[test]
    fn r3566007_drawn_it_shuffles_3_cast_on_draw_cards_never_itself() {
        crate::register_all();
        for n in 0..10 {
            let mut s = drawn(&format!("r3566007-{n}"), false);
            s.expect_in_zone(ID, "graveyard");
            let made = shuffled(&s);
            assert!(made.len() >= DRAWN, "{n}: three shuffled in: {made:?}");
            for card in &made {
                assert_ne!(card.def_id, ID, "{n}: never itself");
                assert!(!card.radiant, "{n}");
                let def = catalog::def_of(None, &card.def_id);
                assert!(!def.token, "{n}: {}", card.def_id);
                assert!(set_ships(def.set), "{n}: {}", card.def_id);
                let flags = jackioh_engine::scripts::registered_entry(&card.def_id)
                    .and_then(|entry| entry.base.static_flags.clone());
                assert_eq!(
                    flags.and_then(|flags| flags.cast_on_draw),
                    Some(true),
                    "{n}: {} casts on draw",
                    card.def_id
                );
            }
        }
    }

    #[test]
    fn r3566007_radiant_shuffles_radiant_ones() {
        crate::register_all();
        let s = drawn("r3566007-radiant", true);
        let made = shuffled(&s);
        assert!(made.len() >= DRAWN, "{made:?}");
        assert!(made.iter().all(|card| card.radiant), "{made:?}");
    }
}
