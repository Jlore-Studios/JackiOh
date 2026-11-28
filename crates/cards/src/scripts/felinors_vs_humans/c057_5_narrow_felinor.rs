//! FvH #57.5 Narrow Felinor (SPEC §8.14 row 57.5; §6.1 Charge): (1) Unit, Felinor, Other-Origins, Token
//! (printed Epic), 4/2 → 8/4, a token FvH #57.3 Felinor Companion and #58 Call of the Shaped Felinors
//! summon.
//!   Base:    "Charge"
//!   Radiant: "Charge"
//!
//! Keywords only, the catalog's data: Charge lets it attack Units and heroes the turn it lands.
//! Declares no numbers. Built by S05 (#5255).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-057-5";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face doubles the stats and keeps Charge: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #57.5 Narrow Felinor — SPEC §8.14 row 57.5.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A textless (0) Spell: something left to play, so no turn ends by itself.
    const FILLER: &str = "stand-in (0) Spell";

    /// p1 plays the Narrow Felinor and sends it at p2's hero the same turn.
    fn charged(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        s.play(ID, json!({}));
        let felinor = s.unit_at(P1, 1);
        s.attack(&felinor, "hero");
        s
    }

    mod base {
        use super::*;

        #[test]
        fn its_charge_hits_the_enemy_hero_the_turn_it_lands() {
            let mut s = charged(false);
            s.expect_health(P2, HERO_HEALTH - 4);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn its_charge_hits_the_enemy_hero_the_turn_it_lands() {
            let mut s = charged(true);
            s.expect_health(P2, HERO_HEALTH - 8);
        }
    }
}
