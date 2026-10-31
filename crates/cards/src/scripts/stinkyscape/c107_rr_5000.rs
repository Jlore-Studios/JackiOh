//! SPEC §8.10 Stinkyscape 107 RR-5000, a card of the Stinkyscape set.
//! (2) Unit, Rare, 4/4 → 8/8: Starship Piece, Ranged.
//! "Start of turn: Gain +{attackGain} Attack." — attackGain 4, Radiant 8 (R1860005).
//!
//! Engine: Starship Piece: its death fuses a copy into its owner's Dormant Starship, summoned
//! first if they have none (R1852000). Ranged (R783000). Start of turn (R62): `buff` +N Attack on
//! itself, lasting, so it grows every one of its controller's turns. The Radiant face runs the
//! same script: its gain is the declared `attackGain` (R386).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-107";

pub fn script() -> CardScripts {
    let base = Script {
        start_of_turn: Some(hook(|ctx| {
            vec![buff(json_as(json!({
                "target": { "of": "self" },
                "attack": param(ctx, "attackGain"),
            })))]
        })),
        ..Script::default()
    };
    // One script: the Radiant +8 is the declared `attackGain`, which `param` reads off the running
    // face; its stats are the catalog's.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Stinkyscape #107 RR-5000 — SPEC §8.10 row 107, BUILD M12 row S 107 (R783000).
#[cfg(test)]
mod tests {
    use super::*;

    use crate::scenario;
    use crate::filler;

    const RR: &str = "stinkyscape-107";
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).

    mod base {
        use super::*;

        #[test]
        fn r1860005_start_of_turn_gains_4_attack_lasting_and_stacking() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "field": [RR], "library": filler(4) },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            s.expect_stats(RR, json!({ "attack": 4 }));

            s.end_turn();
            s.end_turn();
            s.expect_stats(RR, json!({ "attack": 8 }));
            s.end_turn();
            s.end_turn();
            s.expect_stats(RR, json!({ "attack": 12 }));
        }

        #[test]
        fn r1860005_the_opponent_s_turn_start_gains_nothing() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "field": [RR], "library": filler(4) },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            s.end_turn();

            s.expect_stats(RR, json!({ "attack": 4 }));
        }

        #[test]
        fn attack_gain_reads_through_param() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "field": [RR], "library": filler(4) },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            let gain = crate::upgrade_number(&mut s, RR, "attackGain");
            let before = s.stats(RR).attack;
            s.end_turn();
            s.end_turn();

            assert_eq!(s.stats(RR).attack, before + gain);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1860005_radiant_start_of_turn_gains_8_attack() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "field": [{ "def": RR, "radiant": true }], "library": filler(4) },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            s.expect_stats(RR, json!({ "attack": 8 }));

            s.end_turn();
            s.end_turn();
            s.expect_stats(RR, json!({ "attack": 16 }));
        }
    }
}
