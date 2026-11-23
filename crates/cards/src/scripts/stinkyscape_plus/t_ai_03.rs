//! SPEC §8.12 S+ #T-AI-3 Jlockeed Loitering Munition, a card of the Stinkyscape+ set (R1420, R3535000). Built by S25 (#3571).
//!   Base:    "Evasive"
//!   Radiant: "Evasive, Lifesteal"
//! Engine: a (1) Unit 2/1 → 4/2, Jlockeed, AI, Token (no printed rarity, R482). Every word is a catalog keyword
//!   (§6.1): Evasive (R3543002) and, on the Radiant face, Lifesteal, so there is no script to run. Declares no
//!   numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-t-ai-03";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face adds Lifesteal and doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #T-AI-3 Jlockeed Loitering Munition — SPEC §8.12 row T-AI-3 (R3543002).
#[cfg(test)]
mod tests {
    use super::ID;
    use crate::{next_turn_of, weapon_board};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// What attacks into it.
    const ATTACKER: &str = "stand-in (1) 3/3";

    #[test]
    fn evasive_no_attack_may_target_it() {
        for radiant in [false, true] {
            let mut s = weapon_board(
                "s-t-ai-03",
                json!({ "field": [{ "def": ID, "radiant": radiant }] }),
                json!({ "field": [ATTACKER] }),
            );
            let munition = s.card(ID).id.clone();
            s.end_turn();
            assert_eq!(s.state().active, P2);
            let targets: Vec<String> = legal_actions(s.state(), P2)
                .into_iter()
                .filter_map(|action| match action {
                    ActionBody::Attack { target_id, .. } => Some(target_id),
                    _ => None,
                })
                .collect();
            assert!(!targets.contains(&munition), "radiant {radiant}: {targets:?}");
            assert_eq!(targets, vec!["hero-p1".to_string()], "radiant {radiant}");
        }
    }

    #[test]
    fn radiant_lifesteal_heals_its_hero() {
        for radiant in [false, true] {
            let mut s = weapon_board(
                "s-t-ai-03-steal",
                json!({ "field": [{ "def": ID, "radiant": radiant }], "health": 20 }),
                json!({}),
            );
            next_turn_of(&mut s, P1);
            s.attack(ID, "hero");
            // The base face prints no Lifesteal; the Radiant face heals its hero for its 4 damage.
            s.expect_health(P1, if radiant { 24 } else { 20 });
            s.expect_health(P2, HERO_HEALTH - if radiant { 4 } else { 2 });
        }
    }
}
