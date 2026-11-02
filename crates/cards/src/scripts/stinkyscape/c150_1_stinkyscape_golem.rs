//! SPEC §7, §8.10 Stinkyscape 150.1 Stinkyscape Golem (R11, R275, R783000, R1852000). (4) Unit, Token
//! (printed Legendary). Summoned by S #150 Call to Chaos (Stinky Edition)'s ninth entry, as Core #95.1
//! Chaos Golem is by #95's.
//!   Base:    10/10 "Starship Piece, Flying, Ranged, Rush".
//!   Radiant: 20/20, the same keywords.
//!
//! No script: the keywords and stats of each face are printed on the catalog and §10.4 layer 1 reads
//! them off the face the radiant flag picks. The behavior is the engine's: Rush §4.1 (unit targets
//! only), Flying and Ranged (R783000), and Starship Piece, whose death fuses a copy of it into its
//! owner's Dormant Starship (R1852000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-150-1";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face differs in data only (20/20), which layer 1 reads off `def.radiant`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::{js, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// Core #19 Midrange Menace, a 9/9 Taunt; 18/18 on its Radiant face.
    const MENACE: &str = "core-019";
    const VANILLA: &str = "core-008";
    /// Core #10 Rapid Replenish, a (0) Spell that keeps p1's turn from ending by itself.
    const ANCHOR: &str = "core-010";

    fn golem_of(s: &Scenario, player: PlayerId) -> CardInstance {
        s.unit(player, 1)
            .unwrap_or_else(|| panic!("expected {player:?}'s Stinkyscape Golem in lane 1"))
    }

    fn keywords_of(s: &Scenario, card: &CardInstance) -> Vec<String> {
        let mut out: Vec<String> = s
            .stats(card)
            .keywords
            .iter()
            .map(|keyword| js(keyword)["kind"].as_str().unwrap_or_default().to_string())
            .collect();
        out.sort();
        out
    }

    #[test]
    fn a_10_10_with_starship_piece_flying_ranged_and_rush() {
        let mut s = scenario(json!({ "p1": { "field": [ID], "hand": [ANCHOR] } }));
        let golem = golem_of(&s, P1);
        s.expect_stats(&golem, json!({ "attack": 10, "health": 10, "maxHealth": 10 }));
        assert_eq!(keywords_of(&s, &golem), vec!["Flying", "Ranged", "Rush", "Starship Piece"]);
        assert!(crate::card_def(ID).token);
    }

    #[test]
    fn rush_it_attacks_a_unit_the_turn_it_lands_and_ranged_takes_no_strike_back() {
        let mut s = scenario(json!({
            "p1": { "field": [ID], "hand": [ANCHOR] },
            "p2": { "field": [MENACE] },
        }));
        let golem = golem_of(&s, P1);
        let turn = s.state().turn;
        s.card_mut(&golem).summoned_turn = Some(turn);
        s.expect_refused(|s| s.attack(&golem, "hero"));
        let menace = s.unit(P2, 1).expect("p2's Menace");
        s.attack(&golem, &menace);
        s.expect_in_zone(&menace, "graveyard");
        s.expect_stats(&golem, json!({ "health": 10 }));
    }

    #[test]
    fn flying_a_unit_without_flying_or_ranged_cannot_attack_it_in_attack_position() {
        let mut s = scenario(json!({
            "p1": { "field": [VANILLA], "hand": [ANCHOR] },
            "p2": { "field": [ID] },
        }));
        let vanilla = s.unit(P1, 1).expect("p1's Vanilla");
        let golem = golem_of(&s, P2);
        s.expect_refused(|s| s.attack(&vanilla, &golem));
    }

    #[test]
    fn r1852000_its_death_fuses_a_copy_into_its_owner_s_dormant_starship() {
        let mut s = scenario(json!({
            "p1": { "field": [{ "def": MENACE, "radiant": true }], "hand": [ANCHOR] },
            "p2": { "field": [{ "def": ID, "position": "DEF" }] },
        }));
        let menace = s.unit(P1, 1).expect("p1's Menace");
        let golem = golem_of(&s, P2);
        s.attack(&menace, &golem);
        s.expect_in_zone(&golem, "gone");
        let fused: Vec<serde_json::Value> = s
            .events()
            .iter()
            .filter(|event| event.event_type().as_str() == "starshipFused")
            .map(js)
            .collect();
        assert_eq!(fused.len(), 1);
        assert_eq!(fused[0]["player"], json!("p2"));
        assert_eq!(fused[0]["defId"], json!(ID));
    }

    #[test]
    fn radiant_a_20_20_with_the_same_keywords() {
        let mut s = scenario(json!({
            "p1": { "field": [{ "def": ID, "radiant": true }], "hand": [ANCHOR] },
        }));
        let golem = golem_of(&s, P1);
        s.expect_stats(&golem, json!({ "attack": 20, "health": 20, "maxHealth": 20 }));
        assert_eq!(keywords_of(&s, &golem), vec!["Flying", "Ranged", "Rush", "Starship Piece"]);
    }
}
