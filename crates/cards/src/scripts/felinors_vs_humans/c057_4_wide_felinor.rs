//! FvH #57.4 Wide Felinor (SPEC §8.14 row 57.4; §6.1 Taunt): (1) Unit, Felinor, Other-Origins, Token
//! (printed Epic), 4/4 → 8/8, a token FvH #57.3 Felinor Companion and #58 Call of the Shaped Felinors
//! summon.
//!   Base:    "Taunt"
//!   Radiant: "Taunt"
//!
//! Keywords only, the catalog's data, so there is nothing to run. Declares no numbers. Built by S05
//! (#5255).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-057-4";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face doubles the stats and keeps Taunt: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #57.4 Wide Felinor — SPEC §8.14 row 57.4.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const ATTACKER: &str = "stand-in (1) 3/3";
    /// A textless (0) Spell: something left to play, so no turn ends by itself.
    const FILLER: &str = "stand-in (0) Spell";

    /// p2's Wide Felinor stands in lane 1; p1's 3/3 may attack.
    fn guarded(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [FILLER], "field": [ATTACKER] },
            "p2": { "hand": [FILLER], "field": [{ "def": ID, "radiant": radiant }] },
        }))
    }

    /// Taunt reaches play: p1's attacker may not pass the Felinor to hit p2's hero, and may hit it.
    fn taunt_holds(s: &mut Scenario) {
        let attacker = s.unit_at(P1, 1);
        let felinor = s.unit_at(P2, 1);
        assert!(has_keyword(&s.stats(&felinor).keywords, KeywordKind::Taunt));
        s.expect_refused(|s| s.attack(&attacker, "hero"));
        s.expect_health(P2, HERO_HEALTH);
        s.attack(&attacker, &felinor);
    }

    mod base {
        use super::*;

        #[test]
        fn its_taunt_keeps_the_enemy_off_your_hero() {
            let mut s = guarded(false);
            taunt_holds(&mut s);
            let felinor = s.unit_at(P2, 1);
            s.expect_stats(&felinor, json!({ "health": 4 - 3 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn its_taunt_keeps_the_enemy_off_your_hero() {
            let mut s = guarded(true);
            taunt_holds(&mut s);
            let felinor = s.unit_at(P2, 1);
            s.expect_stats(&felinor, json!({ "health": 8 - 3 }));
        }
    }
}
