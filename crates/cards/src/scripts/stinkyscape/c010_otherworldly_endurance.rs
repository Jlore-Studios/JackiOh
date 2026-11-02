//! S #10 Otherworldly Endurance (SPEC §8.10 row S 10; §2.4, R1837000): (2) Spell, Otherworldly, Epic.
//!   Base:    "Add an Inner Fire, a Divine Spirit and a Power Word Shield to your hand."
//!   Radiant: "Add a Radiant Inner Fire, a Radiant Divine Spirit and a Radiant Power Word Shield
//!            to your hand."
//!
//! Engine: three `add_to_hand`, S #10.1 Inner Fire, S #10.2 Divine Spirit and S #10.3 Power Word
//! Shield, in that order (C+ #32 Otherworldly Removal's pattern), a full hand burning what does not
//! fit (§2.4). Radiant: each `radiant: true` (R1867001).

use jackioh_engine::effects::add_to_hand;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-010";

/// S #10.1 Inner Fire, S #10.2 Divine Spirit, S #10.3 Power Word Shield — the order they are added in.
const TOKENS: [&str; 3] = ["stinkyscape-010-1", "stinkyscape-010-2", "stinkyscape-010-3"];

fn otherworldly_endurance(radiant: bool) -> Script {
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
        base: otherworldly_endurance(false),
        radiant: otherworldly_endurance(true),
    }
}

// S #10 Otherworldly Endurance — SPEC §8.10 row S 10, BUILD M12 row S 10 (R1867001).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{def_ids, scenario};

    const P1: PlayerId = PlayerId::P1;

    const ENDURANCE: &str = "stinkyscape-010";
    const INNER_FIRE: &str = "stinkyscape-010-1";
    const DIVINE_SPIRIT: &str = "stinkyscape-010-2";
    const SHIELD: &str = "stinkyscape-010-3";
    const FILLER: &str = "core-005";

    mod base {
        use super::*;

        #[test]
        fn adds_an_inner_fire_a_divine_spirit_and_a_power_word_shield_in_that_order() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [ENDURANCE, FILLER] }, "p2": { "hand": [FILLER] } }));

            s.play(ENDURANCE, json!({}));

            assert_eq!(def_ids(&s.hand(P1)), [FILLER, INNER_FIRE, DIVINE_SPIRIT, SHIELD]);
            assert!(s.hand(P1).iter().all(|card| !card.radiant));
        }

        #[test]
        fn r4_a_hand_with_room_for_one_keeps_the_inner_fire_and_burns_the_others() {
            crate::register_all();
            let mut hand = vec![json!(ENDURANCE)];
            hand.extend((0..9).map(|_| json!(FILLER)));
            let mut s = scenario(json!({ "p1": { "hand": hand }, "p2": { "hand": [FILLER] } }));

            s.play(ENDURANCE, json!({}));

            let held = def_ids(&s.hand(P1));
            assert_eq!(held.len(), 10);
            assert_eq!(held[9], INNER_FIRE);
            let burned: Vec<String> = s
                .events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Burned { def_id, .. } => Some(def_id.clone()),
                    _ => None,
                })
                .collect();
            assert!(burned.contains(&DIVINE_SPIRIT.to_string()));
            assert!(burned.contains(&SHIELD.to_string()));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1867001_adds_all_three_radiant() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": ENDURANCE, "radiant": true }, FILLER] },
                "p2": { "hand": [FILLER] },
            }));

            s.play(ENDURANCE, json!({}));

            let fresh: Vec<CardInstance> =
                s.hand(P1).into_iter().filter(|card| card.def_id != FILLER).collect();
            let ids: Vec<String> = fresh.iter().map(|card| card.def_id.clone()).collect();
            assert_eq!(ids, [INNER_FIRE, DIVINE_SPIRIT, SHIELD]);
            assert!(fresh.iter().all(|card| card.radiant));
        }
    }
}
