//! SPEC §8.12 S+ #158 Quick Chaos, a card of the Stinkyscape+ set (R1420, R3535000). (1) Spell, Rare.
//!   Base:    "Cast on draw: Cast a random Spell with random targets."
//!   Radiant: "Cast on draw: Cast a random Radiant Spell with random targets."
//!
//! Cast on draw (§6.2, `StaticFlags.cast_on_draw`). `cast_random` draws one Spell from the collectible
//! Spells of the shipped sets (tokens out, R380, R382, R1420), never this card (R387), and casts it free
//! (R70) with every choice of the cast random, so nothing pauses for a prompt (R3557005). The Radiant
//! face casts it Radiant. Declares no numbers.

use jackioh_engine::effects::{CastRandomArgs, CastRandomCount, CastRandomQuery, cast_random};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-158";

/// "Cast a random Spell": one cast.
const CASTS: i32 = 1;

fn quick_chaos(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |_ctx| {
            vec![cast_random(CastRandomArgs {
                query: CastRandomQuery::Fixed(json_as(json!({ "type": [CardType::Spell] }))),
                count: Some(CastRandomCount::Fixed(CASTS)),
                radiant: Some(radiant),
                target_enemies: None,
                afterward: None,
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: quick_chaos(false),
        radiant: quick_chaos(true),
    }
}

// S+ #158 Quick Chaos — SPEC §8.12 row 158 (R3557005).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const FILLER: &str = "core-005";

    fn drawn(seed: &str, radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [FILLER], "library": [{ "def": ID, "radiant": radiant }, FILLER, FILLER], "field": ["core-019"] },
            "p2": { "hand": [FILLER], "field": ["core-019"], "library": filler(4) },
        }));
        s.start_turn();
        s
    }

    /// The Spells cast during the draw, by definition.
    fn casts(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter(|event| event.event_type().as_str() == "cardPlayed")
            .map(crate::js)
            .filter_map(|event| event["defId"].as_str().map(str::to_string))
            .collect()
    }

    #[test]
    fn r3557005_a_random_spell_is_cast_and_no_prompt_opens() {
        crate::register_all();
        for n in 0..10 {
            let mut s = drawn(&format!("r3557005-{n}"), false);
            s.expect_in_zone(ID, "graveyard");
            assert!(s.state().pending.is_none(), "{n}: nothing paused");
            let cast = casts(&s);
            assert!(cast.iter().any(|id| id == ID), "{n}: it was cast on draw");
            let others: Vec<&String> = cast.iter().filter(|id| id.as_str() != ID).collect();
            // The one random Spell is the first cast after it; a Spell that casts others (Call to Chaos)
            // leaves more cards played behind it, which are its own.
            assert!(!others.is_empty(), "{n}: one random Spell");
            let def = catalog::def_of(None, others[0]);
            assert_eq!(def.type_, CardType::Spell);
            assert!(!def.token);
            assert_ne!(def.id, ID);
        }
    }

    #[test]
    fn the_radiant_face_casts_a_radiant_spell() {
        crate::register_all();
        let s = drawn("r3557005-radiant", true);
        assert!(s.state().pending.is_none());
        let cast: Vec<CardInstance> = s
            .events()
            .iter()
            .filter(|event| event.event_type().as_str() == "cardPlayed")
            .map(crate::js)
            .filter_map(|event| event["instanceId"].as_str().and_then(|id| find_instance(s.state(), id).cloned()))
            .collect();
        assert!(cast.iter().any(|card| card.def_id != ID), "a random Spell was cast");
        assert!(cast.iter().all(|card| card.radiant), "every card cast is Radiant: {cast:?}");
    }
}
