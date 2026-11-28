//! SPEC §8.14 FvH #1 NGMI Joe ((2) Unit, Human, Common, 8/8 → 16/16), a card of the held Felinors vs Humans
//! set (R5235000).
//!   Base:    "End of turn: Nerf this."
//!   Radiant: "Start of turn: Nerf this."
//!
//! Engine: at the end of its controller's turn (R62; the Radiant face at the start of its controller's
//! turn instead), `degrade` on itself: the usual Nerf (R386, R1320), one change drawn from what is live on
//! the field (R3888000), which may now add Fragile (R5236011) or Unlucky 1 (R5236021) (FV2 #1). Declares no
//! numbers. Built by S01 (#5251).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-001";

/// "Nerf this": one Degrade of the card itself (R386).
fn nerf_this() -> Hook {
    hook(|_ctx| vec![degrade(json_as(json!({ "target": { "of": "self" } })))])
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            end_of_turn: Some(nerf_this()),
            ..Script::default()
        },
        radiant: Script {
            start_of_turn: Some(nerf_this()),
            ..Script::default()
        },
    }
}

// FvH #1 NGMI Joe — SPEC §8.14 row 1 (R62, R386, R3888000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    /// NGMI Joe (`radiant` its face) stands on p1's side on p1's turn.
    fn standing(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [FILLER], "field": [{ "def": ID, "radiant": radiant }], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    #[test]
    fn declares_no_tuned_numbers() {
        crate::register_all();
        assert!(crate::card_def(ID).params.is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn r62_r386_it_nerfs_itself_once_at_the_end_of_its_controllers_turn_only() {
            let mut s = standing(false);
            assert_eq!(s.count("degraded"), 0);
            s.end_turn();
            assert_eq!(s.count("degraded"), 1, "its controller's end of turn");
            s.end_turn();
            assert_eq!(s.count("degraded"), 1, "nothing at the opponent's end nor its controller's start");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r62_r386_it_nerfs_itself_once_at_the_start_of_its_controllers_turn_only() {
            let mut s = standing(true);
            s.end_turn();
            assert_eq!(s.count("degraded"), 0, "nothing at its controller's end of turn");
            s.end_turn();
            assert_eq!(s.count("degraded"), 1, "its controller's start of turn");
            s.end_turn();
            assert_eq!(s.count("degraded"), 1, "nothing at its controller's end of turn");
        }
    }
}
