//! S #12 Otherworldly Nightmare (SPEC §8.10 row S 12; §2.4, ST8, R1837000): (1) Spell,
//! Otherworldly, Epic.
//!   Base:    "Add a Shadow Word Death, a Shadow Word Pain and a Psychic Scream to your hand."
//!   Radiant: "Add a Radiant Shadow Word Death, a Radiant Shadow Word Pain and a Radiant Psychic
//!            Scream to your hand."
//!
//! Engine: three `add_to_hand`, S #12.1 Shadow Word Death, S #12.2 Shadow Word Pain and S #12.3
//! Psychic Scream, in that order (C+ #32 Otherworldly Removal's pattern), a full hand burning what
//! does not fit (§2.4). Radiant: each `radiant: true` — ST8's fix of the designer's copy of C+ #32's
//! line: never Execute, Brawl or Blade Storm (R275: the three Radiant).

use jackioh_engine::effects::add_to_hand;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-012";

/// S #12.1 Shadow Word Death, S #12.2 Shadow Word Pain, S #12.3 Psychic Scream — the order they are
/// added in.
const TOKENS: [&str; 3] = ["stinkyscape-012-1", "stinkyscape-012-2", "stinkyscape-012-3"];

fn otherworldly_nightmare(radiant: bool) -> Script {
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
        base: otherworldly_nightmare(false),
        radiant: otherworldly_nightmare(true),
    }
}

// S #12 Otherworldly Nightmare — SPEC §8.10 row S 12, BUILD M12 row S 12.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{def_ids, scenario};

    const P1: PlayerId = PlayerId::P1;

    const NIGHTMARE: &str = "stinkyscape-012";
    const DEATH: &str = "stinkyscape-012-1";
    const PAIN: &str = "stinkyscape-012-2";
    const SCREAM: &str = "stinkyscape-012-3";
    const FILLER: &str = "core-005";

    mod base {
        use super::*;

        #[test]
        fn adds_a_death_a_pain_and_a_scream_in_that_order() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [NIGHTMARE, FILLER] }, "p2": { "hand": [FILLER] } }));

            s.play(NIGHTMARE, json!({}));

            assert_eq!(def_ids(&s.hand(P1)), [FILLER, DEATH, PAIN, SCREAM]);
            assert!(s.hand(P1).iter().all(|card| !card.radiant));
        }

        #[test]
        fn r4_a_full_hand_burns_them() {
            crate::register_all();
            let mut hand = vec![json!(NIGHTMARE)];
            hand.extend((0..9).map(|_| json!(FILLER)));
            let mut s = scenario(json!({ "p1": { "hand": hand }, "p2": { "hand": [FILLER] } }));

            s.play(NIGHTMARE, json!({}));

            let held = def_ids(&s.hand(P1));
            assert_eq!(held.len(), 10);
            assert_eq!(held[9], DEATH);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn adds_all_three_radiant_and_no_execute_brawl_or_blade_storm() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": NIGHTMARE, "radiant": true }, FILLER] },
                "p2": { "hand": [FILLER] },
            }));

            s.play(NIGHTMARE, json!({}));

            let fresh: Vec<CardInstance> =
                s.hand(P1).into_iter().filter(|card| card.def_id != FILLER).collect();
            let ids: Vec<String> = fresh.iter().map(|card| card.def_id.clone()).collect();
            assert_eq!(ids, [DEATH, PAIN, SCREAM]);
            assert!(fresh.iter().all(|card| card.radiant));
            for id in ["classicplus-032-1", "classicplus-032-2", "classicplus-032-3"] {
                assert!(!ids.contains(&id.to_string()), "no {id}");
            }
        }
    }
}
