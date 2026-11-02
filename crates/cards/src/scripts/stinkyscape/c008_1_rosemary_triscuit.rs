//! S #8.1 Rosemary Triscuit (SPEC §7 token; §6.3 Refresh X, R364): (0) Spell, Token (printed Rare).
//!   Base and Radiant: "Refresh {refresh} mana." — 2, 4 on the Radiant face.
//!
//! Engine: `refresh_mana` by the declared number (R364: the caster's current mana rises by up to
//! that much, never above max mana; at or above max, nothing happens). Both faces run this one
//! script: the Radiant face's 4 is its declared `refresh`, which `param` reads off the running face.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-008-1";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![refresh_mana(json_as(json!({ "amount": param(&*ctx, "refresh") })))]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's 4 is its declared `refresh` (R386).
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S #8.1 Rosemary Triscuit — SPEC §7 token, BUILD M12 row S 8.1 (R364).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    const TRISCUIT: &str = "stinkyscape-008-1";
    const FILLER: &str = "core-005";

    mod base {
        use super::*;

        #[test]
        fn refreshes_2_spent_mana_never_above_max_mana() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [TRISCUIT, FILLER], "mana": 1 },
                "p2": { "hand": [FILLER] },
            }));

            s.play(TRISCUIT, json!({}));

            s.expect_mana(P1, 3);
            s.expect_in_zone(TRISCUIT, "graveyard");
        }

        #[test]
        fn r364_at_full_mana_nothing_happens() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [TRISCUIT, FILLER] },
                "p2": { "hand": [FILLER] },
            }));
            s.expect_mana(P1, 4);

            s.play(TRISCUIT, json!({}));

            s.expect_mana(P1, 4);
            s.expect_in_zone(TRISCUIT, "graveyard");
        }

        #[test]
        fn r386_an_upgrade_of_refresh_makes_it_refresh_more() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [TRISCUIT, FILLER], "mana": 0 },
                "p2": { "hand": [FILLER] },
            }));
            step_param(s.card_mut(TRISCUIT), "refresh", 1);

            s.play(TRISCUIT, json!({}));

            s.expect_mana(P1, 3);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn refreshes_4_mana() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": TRISCUIT, "radiant": true }, FILLER], "mana": 0 },
                "p2": { "hand": [FILLER] },
            }));

            s.play(TRISCUIT, json!({}));

            s.expect_mana(P1, 4);
        }
    }
}
