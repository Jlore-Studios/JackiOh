//! SPEC §8.14 FvH #17 Jlockeed Shredder-14 ((8) Unit, Jlockeed, Epic; 16/22 → 32/44), a card of the
//! Felinors vs Humans set (R5235000).
//!   Base:    "Float reduces cost.\nEnd of turn: Deal {damage} damage to all enemies, four times." (2)
//!   Radiant: "Pierce\nFloat reduces cost.\nEnd of turn: Deal {damage} damage to all enemies, four times." (4)
//!
//! "Float reduces cost" is the engine's hand trigger (`float_reduces_cost`, R1563000). The end of turn is
//! its controller's (R62) and S+ #21 Shredder-12's shape (R3553000): one `damage_all` per pass, each
//! reading its targets afresh, every enemy Unit then the enemy hero, no state check between one pass's
//! hits (R59); "four times" is four passes. The Radiant face's Pierce and doubled stats are catalog
//! data, so both faces run one script.

use jackioh_engine::effects::damage_all;
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-017";

/// "four times": the end-of-turn passes over every enemy, a printed word (R1748003).
const FOUR_TIMES: i32 = 4;

pub fn script() -> CardScripts {
    let base = Script {
        hand_triggers: vec![float_reduces_cost("felinorsvshumans-017-float-hand")],
        end_of_turn: Some(hook(|ctx| {
            let amount = param(&*ctx, "damage");
            (0..FOUR_TIMES)
                .map(|_| damage_all(json_as(json!({ "side": "enemy", "amount": amount, "heroes": true }))))
                .collect()
        })),
        ..Script::default()
    };
    // The same script: Pierce, the stats and the 4 are the Radiant face's catalog data.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #17 Jlockeed Shredder-14 — SPEC §8.14 row 17 (R1563000, R3553000, R59, R62).
#[cfg(test)]
mod tests {
    use super::{FOUR_TIMES, ID};
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::{filler, hits, scenario};

    /// A 0/40, so four passes of either face leave it standing and Pierce has no overkill to carry.
    const WALL: &str = "stand-in (1) 0/40";
    const VANILLA: &str = "stand-in (1) 4/4";
    const FILLER: &str = "core-005"; // Stockpile: a Spell, each side's spare card (§2.5).

    /// p1's Shredder (`radiant` its face) on the field, two enemy WALLs across.
    fn shredder_board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "field": [{ "def": ID, "radiant": radiant }], "hand": [FILLER], "library": filler(2) },
            "p2": {
                "field": [{ "def": WALL, "lane": 1 }, { "def": WALL, "lane": 2 }],
                "hand": [FILLER],
                "library": filler(2),
            },
        }))
    }

    /// The pass shape: `amount` per hit, two Units then the hero, four times over.
    fn expect_four_passes(s: &mut Scenario, amount: i32) {
        let hits = hits(s.last_events());
        assert_eq!(hits.len(), 3 * FOUR_TIMES as usize);
        assert!(hits.iter().all(|hit| hit.1 == amount), "{hits:?}");
        let heroes: Vec<usize> = hits
            .iter()
            .enumerate()
            .filter(|(_, hit)| hit.0 == "hero-p2")
            .map(|(at, _)| at)
            .collect();
        assert_eq!(heroes, vec![2, 5, 8, 11]);
        for lane in [1, 2] {
            let id = s.unit_at(P2, lane).id;
            s.expect_stats(id.as_str(), json!({ "health": 40 - FOUR_TIMES * amount, "maxHealth": 40 }));
        }
        s.expect_health(P2, 30 - FOUR_TIMES * amount);
        s.expect_health(P1, 30);
    }

    #[test]
    fn r1563000_a_float_lowers_its_cost_in_hand_never_in_the_deck() {
        crate::stinkyscape_family::drone_float_two_less(ID, 8);
    }

    #[test]
    fn r1563000_the_opponents_float_lowers_nothing() {
        crate::stinkyscape_family::drone_float_opponent(ID, 8);
    }

    mod base {
        use super::*;

        #[test]
        fn r3553000_four_times_is_four_passes_over_every_enemy_the_hero_last_each_time() {
            let mut s = shredder_board(false);
            s.end_turn();
            expect_four_passes(&mut s, 2);
        }

        #[test]
        fn r59_a_unit_the_first_pass_kills_still_takes_the_other_three() {
            let mut s = scenario(json!({
                "p1": { "field": [ID], "hand": [FILLER], "library": filler(2) },
                "p2": { "field": ["stand-in (1) 1/1"], "hand": [FILLER], "library": filler(2) },
            }));
            let unit = s.unit_at(P2, 1).id;
            s.end_turn();
            assert_eq!(hits(s.last_events()).iter().filter(|hit| hit.0 == unit).count(), FOUR_TIMES as usize);
            s.expect_in_zone(unit.as_str(), "graveyard");
        }

        #[test]
        fn r62_the_opponents_end_of_turn_is_silent() {
            let mut s = scenario(json!({
                "p1": { "field": [VANILLA], "hand": [FILLER], "library": filler(2) },
                "p2": { "field": [ID], "hand": [FILLER], "library": filler(2) },
            }));
            s.end_turn();
            assert!(hits(s.last_events()).is_empty());
            s.expect_health(P1, 30);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3553000_it_deals_4_four_times() {
            let mut s = shredder_board(true);
            s.end_turn();
            expect_four_passes(&mut s, 4);
        }

        #[test]
        fn its_pierce_reaches_play() {
            let s = shredder_board(true);
            assert!(unit_has(s.state(), s.card(ID), KeywordKind::Pierce));
            let base = shredder_board(false);
            assert!(!unit_has(base.state(), base.card(ID), KeywordKind::Pierce));
        }
    }
}
