//! SPEC §8.14 FvH #14 Direwolf ((4) Unit, Rare, 7/2 → 14/4), a card of the held Felinors vs Humans set
//! (R5235000).
//!   Base:    "Rush, Frenzy, First Strike"
//!   Radiant: "Rush, Frenzy, First Strike"
//!
//! Engine: every word is a catalog keyword (§6.1), so there is no script to run. Rush lets it attack a Unit
//! the turn it arrives; Frenzy frees one more attack for each kill of the Unit it attacked, still sick on
//! its summon turn (R3543003, R2521002); First Strike strikes first and once (R93), so a defender it kills
//! strikes nothing back. Declares no numbers. Built by S01 (#5251).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-014";

pub fn script() -> CardScripts {
    let base = Script::default();
    // Both faces print the same three keywords; the Radiant line is catalog data.
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// FvH #14 Direwolf — SPEC §8.14 row 14 (R3543003, R93).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    /// Each dies to the Direwolf's strike and would kill it if it struck back.
    const BITER: &str = "stand-in (1) 9/2";
    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    /// p1 plays the Direwolf (`radiant` its face) into lane 1, facing two Biters.
    fn played(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "field": [BITER, BITER], "library": filler(4) },
        }));
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    /// Rush: it attacks a Unit the turn it arrives; First Strike: the Biter dies before it strikes back;
    /// Frenzy: the kill frees a second attack, on a Unit only while it is still summoning sick.
    fn rushes_strikes_first_and_attacks_again(radiant: bool, health: i32) {
        let mut s = played(radiant);
        let wolf = s.unit_at(P1, 1);
        s.expect_refused(|s| s.attack(&wolf, "hero"));
        s.attack(&wolf, s.unit_at(P2, 1));
        assert!(s.unit(P2, 1).is_none(), "the first Biter died");
        s.expect_stats(&wolf, json!({ "health": health }));
        s.attack(&wolf, s.unit_at(P2, 2));
        assert!(s.unit(P2, 2).is_none(), "the second Biter died");
        s.expect_stats(&wolf, json!({ "health": health }));
        s.expect_refused(|s| s.attack(&wolf, "hero"));
    }

    #[test]
    fn declares_no_tuned_numbers() {
        crate::register_all();
        assert!(crate::card_def(ID).params.is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn r3543003_r93_it_rushes_strikes_first_and_attacks_again_after_a_kill() {
            rushes_strikes_first_and_attacks_again(false, 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3543003_r93_it_rushes_strikes_first_and_attacks_again_after_a_kill() {
            rushes_strikes_first_and_attacks_again(true, 4);
        }
    }
}
