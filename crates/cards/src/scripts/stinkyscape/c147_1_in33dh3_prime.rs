//! S #147.1 IN33DH3 Prime (SPEC §7 row 147.1, §8.10 row 147.1): (4) Unit, Prime, Human, Token (printed
//! Legendary), 9/5 → 18/10, a token of the Stinkyscape set.
//!   Base:    "Cry: Summon a Wardrum Blob."
//!   Radiant: "Cry: Summon a Radiant Wardrum Blob."
//! Engine: Cry (R1), played or cast (R70): a summon of S #147.2 Wardrum Blob, a Field Spell, into its
//!   controller's leftmost open backrow zone (R64), as M #71's The Cane is; with no open zone nothing.
//!   S #147's Death puts it in the deck (R1421). Radiant: a Radiant Blob.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-147-1";

/// S #147.2 Wardrum Blob.
const BLOB: &str = "stinkyscape-147-2";

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![summon(json_as(json!({ "defId": BLOB, "radiant": radiant })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// S #147.1 IN33DH3 Prime — SPEC §7 row 147.1, BUILD M12 row S 147.1.
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::cast_new;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const PRIME: &str = "stinkyscape-147-1";
    const BLOB: &str = "stinkyscape-147-2";
    const RESERVES: &str = "core-075"; // (0) Field Spell.
    const FILLER: &str = "core-005";

    fn played(radiant: bool, backrow: Value) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": PRIME, "radiant": radiant }, FILLER],
                "backrow": backrow,
                "library": filler(6),
            },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }));
        s.play(PRIME, json!({ "zone": 1 }));
        s
    }

    fn backrow(s: &Scenario) -> Vec<(String, bool)> {
        (1..=5)
            .filter_map(|lane| s.backrow(P1, lane).map(|card| (card.def_id, card.radiant)))
            .collect()
    }

    fn by_hand(s: &mut Scenario, effects: Vec<Effect>) {
        let mut rng = create_rng(&s.state().seed, s.state().rng_cursor);
        let mut events: Vec<GameEvent> = Vec::new();
        {
            let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
            {
                let mut ctx = make_context(
                    &mut sink,
                    None,
                    HookOptions {
                        controller: Some(P1),
                        ..HookOptions::default()
                    },
                );
                apply_effects(&effects, &mut ctx);
            }
            settle(&mut sink, SettleOptions::default());
        }
        s.state_mut().rng_cursor = rng.cursor();
    }

    #[test]
    fn r64_its_cry_summons_a_wardrum_blob_into_your_leftmost_open_backrow_zone() {
        let s = played(false, json!([{ "def": RESERVES, "lane": 1 }]));
        assert_eq!(
            backrow(&s),
            vec![(RESERVES.to_string(), false), (BLOB.to_string(), false)]
        );
        assert_eq!(s.backrow(P1, 2).map(|card| card.def_id).as_deref(), Some(BLOB));
    }

    #[test]
    fn r64_with_no_open_backrow_zone_it_summons_none() {
        let s = played(false, json!(vec![RESERVES; 5]));
        assert_eq!(backrow(&s), vec![(RESERVES.to_string(), false); 5]);
    }

    #[test]
    fn r70_cast_its_cry_summons_a_blob_too() {
        let mut s = scenario(json!({
            "p1": { "hand": [FILLER], "library": filler(6) },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }));
        by_hand(&mut s, vec![cast_new(json_as(json!({ "def": PRIME })))]);
        assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id).as_deref(), Some(PRIME));
        assert_eq!(backrow(&s), vec![(BLOB.to_string(), false)]);
    }

    #[test]
    fn radiant_18_10_and_a_radiant_blob() {
        let s = played(true, json!([]));
        let stats = s.stats(PRIME);
        assert_eq!((stats.attack, stats.health), (18, 10));
        assert_eq!(backrow(&s), vec![(BLOB.to_string(), true)]);
    }
}
