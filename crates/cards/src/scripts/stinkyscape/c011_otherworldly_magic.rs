//! S #11 Otherworldly Magic (SPEC §8.10 row S 11; §2.4, R1837000): (2) Spell, Otherworldly, Epic (#2521:
//! was (1)).
//!   Base:    "Add a Frostbolt, an Arcane Missiles and a Flame Strike to your hand."
//!   Radiant: "Add a Radiant Frostbolt, a Radiant Arcane Missiles and a Radiant Flame Strike to your hand."
//!
//! Engine: three `add_to_hand`, S #11.1 Frostbolt, S #11.2 Arcane Missiles and S #11.3 Flame Strike,
//! in that order (C+ #32 Otherworldly Removal's pattern), a full hand burning what does not fit
//! (§2.4). Radiant: each `radiant: true` (S #10's reading: all three Radiant, as the designer wrote
//! it here).

use jackioh_engine::effects::add_to_hand;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-011";

/// S #11.1 Frostbolt, S #11.2 Arcane Missiles, S #11.3 Flame Strike — the order they are added in.
const TOKENS: [&str; 3] = ["stinkyscape-011-1", "stinkyscape-011-2", "stinkyscape-011-3"];

fn otherworldly_magic(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            TOKENS
                .iter()
                .map(|def_id| {
                    let mut args = json!({ "defId": def_id });
                    if radiant {
                        args["radiant"] = json!(true);
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
        base: otherworldly_magic(false),
        radiant: otherworldly_magic(true),
    }
}

// S #11 Otherworldly Magic — SPEC §8.10 row S 11, BUILD M12 row S 11: "adds one base Frostbolt,
// Arcane Missiles and Flame Strike (S 11.1 to 11.3) to its caster's hand, in that order; a full
// hand burns them; radiant all three are Radiant".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{def_ids, scenario};

    const P1: PlayerId = PlayerId::P1;

    const MAGIC: &str = "stinkyscape-011";
    const FROSTBOLT: &str = "stinkyscape-011-1";
    const MISSILES: &str = "stinkyscape-011-2";
    const STRIKE: &str = "stinkyscape-011-3";
    const FILLER: &str = "core-005";

    mod base {
        use super::*;

        #[test]
        fn adds_a_frostbolt_arcane_missiles_and_flame_strike_in_that_order() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [MAGIC, FILLER] }, "p2": { "hand": [FILLER] } }));

            s.play(MAGIC, json!({}));

            assert_eq!(def_ids(&s.hand(P1)), [FILLER, FROSTBOLT, MISSILES, STRIKE]);
            assert!(s.hand(P1).iter().all(|card| !card.radiant));
        }

        #[test]
        fn r4_a_full_hand_burns_them() {
            crate::register_all();
            let mut hand = vec![json!(MAGIC)];
            hand.extend((0..9).map(|_| json!(FILLER)));
            let mut s = scenario(json!({ "p1": { "hand": hand }, "p2": { "hand": [FILLER] } }));

            s.play(MAGIC, json!({}));

            let held = def_ids(&s.hand(P1));
            assert_eq!(held.len(), 10);
            assert_eq!(held[9], FROSTBOLT);
            let burned: Vec<String> = s
                .events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Burned { def_id, .. } => Some(def_id.clone()),
                    _ => None,
                })
                .collect();
            assert!(burned.contains(&MISSILES.to_string()));
            assert!(burned.contains(&STRIKE.to_string()));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn adds_all_three_radiant() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": MAGIC, "radiant": true }, FILLER] },
                "p2": { "hand": [FILLER] },
            }));

            s.play(MAGIC, json!({}));

            let fresh: Vec<CardInstance> =
                s.hand(P1).into_iter().filter(|card| card.def_id != FILLER).collect();
            let ids: Vec<String> = fresh.iter().map(|card| card.def_id.clone()).collect();
            assert_eq!(ids, [FROSTBOLT, MISSILES, STRIKE]);
            assert!(fresh.iter().all(|card| card.radiant));
        }
    }
}
