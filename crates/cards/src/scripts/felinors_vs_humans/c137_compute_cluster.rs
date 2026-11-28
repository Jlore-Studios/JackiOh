//! SPEC §8.14 FvH #137 Plagiarized: Compute Cluster, a frozen copy of IA #203 Compute Cluster (R2195003,
//! FV4), a card of the held Felinors vs Humans set, built by #5235.
//!
//! The code is a copy of the IA card's as it stood when the set was founded, not a re-export, so a later IA
//! patch does not move it (R2195003). Its `ID` is its own, so every read of it names this card; it is named
//! `Plagiarized: Compute Cluster`, tagged Plagiarized only, and a token it makes stays the IA token. Its
//! change from IA #203: none. The IA file's notes, as they stood:
//!
//! IA #203 Compute Cluster (SPEC §8.13 row 203), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Start of turn: Your Units gain +{buff}/+{buff}."
//!   Radiant: "Start of turn: Your Units gain +{buff}/+{buff}."
//!
//! Engine: a Field Spell's `start_of_turn` hook (its controller's turn, R68, before the turn's
//! draw, R62): a permanent `buff_all_units` of the declared `buff` on its controller's Units.
//! Built by C9 (#4593).

use jackioh_engine::effects::buff_all_units;
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-137";

fn cluster() -> Script {
    Script {
        start_of_turn: Some(hook(|ctx| {
            let gain = param(&*ctx, "buff");
            vec![buff_all_units(json_as(json!({ "attack": gain, "health": gain })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same script: the Radiant face's +2/+2 is its declared `buff`.
    let radiant = cluster();
    CardScripts {
        base: cluster(),
        radiant,
    }
}

// FvH #137 Plagiarized: Compute Cluster — SPEC §8.14 row 137; IA #203's tests (SPEC §8.13 row 203), on this
// card.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    fn started(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "field": [VANILLA],
                "backrow": [{ "def": ID, "radiant": radiant }],
                "hand": [FILLER],
                "library": crate::filler(4),
                "mana": 10,
            },
            "p2": {
                "field": [VANILLA],
                "hand": [FILLER],
                "library": crate::filler(4),
            },
        }));
        s.end_turn();
        s.end_turn();
        s
    }

    fn stats(s: &Scenario, player: PlayerId) -> (i32, i32) {
        let unit = s.unit_at(player, 1);
        let view = unit_view(s.state(), &unit);
        (view.attack, view.health)
    }

    mod base {
        use super::*;

        #[test]
        fn own_units_gain_1_1_at_the_owners_start_only() {
            let s = started(false, "cluster-base");
            assert_eq!(stats(&s, P1), (5, 5));
            assert_eq!(stats(&s, P2), (4, 4));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn own_units_gain_2_2_at_the_owners_start_only() {
            let s = started(true, "cluster-radiant");
            assert_eq!(stats(&s, P1), (6, 6));
            assert_eq!(stats(&s, P2), (4, 4));
        }
    }
}
