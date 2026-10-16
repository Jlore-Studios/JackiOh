//! R346's Pierce in the AI's own combat reads (#1038): a hit whose source pierces skips §4.4 step 2,
//! the Armor, on a unit and on a hero alike, in `face_threat` and the lethal solver's `ready_gap`, in
//! the move order's winning trades (the hit and the strike back) and in the opponent's reply (its
//! blows and its face hit). Defense Position's Armor 1 (§4.1) is the Armor most boards here pierce.
//!
//! Each board grants Pierce to a vanilla body on the field, as `ranged_flying.rs` grants Ranged. The
//! cards: Mr. Vanilla (core-008) 4/4, Zoomerbin Oomen (core-067) 1/2, Plague Doctor (classic-059) 2/3
//! and Plague Crawler (classic-053) 2/2.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, in_graveyard, on_field, register_cards};

const LIBRARY: &[&str] = &["core-008", "core-011", "core-008", "core-011"];

fn board(setup: Value) -> GameState {
    register_cards();
    scenario(setup).state().clone()
}

/// Grants `keyword` to `player`'s unit of `def_id` on the field.
fn grant(state: &mut GameState, player: PlayerId, def_id: &str, keyword: Keyword) {
    let id = active_units_of(state, player)
        .into_iter()
        .find(|unit| unit.def_id == def_id)
        .map(|unit| unit.id.clone())
        .expect("the unit stands on the field");
    find_instance_mut(state, &id)
        .expect("the unit")
        .granted_keywords
        .push(keyword);
}

mod r346_face {
    use super::*;

    #[test]
    fn r346_face_threat_counts_a_pierce_hit_in_full_through_the_heros_armor() {
        for (pierce, threat) in [(true, 1), (false, 0)] {
            let mut state = board(json!({
                "seed": "ai-pierce-face",
                "p1": { "armor": 2 },
                "p2": { "field": ["core-067"] },
            }));
            if pierce {
                grant(&mut state, HUMAN, "core-067", Keyword::Pierce);
            }
            assert_eq!(face_threat(&state, HUMAN), threat, "Pierce {pierce}");
        }
    }

    #[test]
    fn r346_ready_gap_counts_a_pierce_hit_through_the_heros_armor() {
        for (pierce, gap) in [(true, 0.0), (false, 1.0)] {
            let mut state = board(json!({
                "seed": "ai-pierce-gap",
                "p1": { "field": ["core-067"] },
                "p2": { "health": 1, "armor": 1 },
            }));
            if pierce {
                grant(&mut state, AI, "core-067", Keyword::Pierce);
            }
            assert_eq!(ready_gap(&state, AI), gap, "Pierce {pierce}");
        }
    }
}

mod r346_move_order {
    use super::*;

    /// Where `candidate_actions` puts the attack on the enemy's only unit and the first play.
    fn places(state: &GameState) -> (usize, usize) {
        let target = active_units_of(state, HUMAN)[0].id.clone();
        let actions = candidate_actions(state, AI);
        let attack = actions
            .iter()
            .position(|action| matches!(action, ActionBody::Attack { target_id, .. } if *target_id == target))
            .expect("the attack on the enemy unit is a candidate");
        let play = actions
            .iter()
            .position(|action| matches!(action, ActionBody::Play { .. }))
            .expect("the hand card's play is a candidate");
        (attack, play)
    }

    /// Plague Doctor 2/3 against Zoomerbin Oomen 1/2 in Defense Position (Armor 1): a kill only
    /// when the Doctor pierces; it survives the strike back of 1 either way.
    fn into_defense(pierce: bool) -> GameState {
        let mut state = board(json!({
            "seed": "ai-pierce-order",
            "p1": { "hand": ["core-008"], "field": ["classic-059"] },
            "p2": { "field": [{ "def": "core-067", "position": "DEF" }] },
        }));
        if pierce {
            grant(&mut state, AI, "classic-059", Keyword::Pierce);
        }
        state
    }

    #[test]
    fn r346_a_pierce_kill_of_a_defense_unit_is_a_good_trade_ahead_of_the_plays() {
        let (attack, play) = places(&into_defense(true));
        assert!(attack < play, "the kill at {attack}, the first play at {play}");
    }

    #[test]
    fn r346_the_same_hit_without_pierce_waits_behind_the_plays() {
        let (attack, play) = places(&into_defense(false));
        assert!(attack > play, "the chip at {attack}, the first play at {play}");
    }

    /// Mr. Vanilla 4/4 with Armor 1 into Mr. Vanilla 4/4: it survives the strike back only while
    /// its Armor takes a point off it.
    #[test]
    fn r346_a_pierce_defenders_strike_back_skips_the_attackers_armor() {
        for (pierce, survives) in [(false, true), (true, false)] {
            let mut state = board(json!({
                "seed": "ai-pierce-strike-back",
                "p1": { "hand": ["core-008"], "field": ["core-008"] },
                "p2": { "field": ["core-008"] },
            }));
            grant(&mut state, AI, "core-008", Keyword::Armor { n: 1 });
            if pierce {
                grant(&mut state, HUMAN, "core-008", Keyword::Pierce);
            }
            let (attack, play) = places(&state);
            assert_eq!(
                attack < play,
                survives,
                "Pierce {pierce}: the trade at {attack}, the first play at {play}"
            );
        }
    }
}

mod r346_reply {
    use super::*;

    /// The AI's turn 9, with `p1` as its side and the enemy's Plague Crawler 2/2 given `keywords`,
    /// ended through the reducer: the enemy's turn 10.
    fn handed_over(p1: Value, keywords: &[Keyword]) -> GameState {
        let mut state = board(json!({
            "seed": "ai-pierce-reply",
            "active": "p1",
            "turn": 9,
            "p1": p1,
            "p2": { "field": ["classic-053"], "library": LIBRARY },
        }));
        for keyword in keywords {
            grant(&mut state, HUMAN, "classic-053", keyword.clone());
        }
        act(&state, AI, &ActionBody::EndTurn)
    }

    fn reply(state: &GameState) -> GameState {
        simulate_reply(
            state,
            AI,
            &create_node_counter(20, None),
            &hidden_card_ids(state, AI),
        )
        .expect("a reply")
    }

    /// The AI's Plague Crawler in Defense Position (Armor 1, Taunt) against the enemy's, which
    /// strikes first: its hit kills only when it pierces, and short of a kill the strike back
    /// destroys it, so the model attacks only when it reads the Pierce.
    fn crawlers(keywords: &[Keyword]) -> GameState {
        handed_over(
            json!({ "field": [{ "def": "classic-053", "position": "DEF" }], "library": LIBRARY }),
            keywords,
        )
    }

    #[test]
    fn r346_the_opponents_pierce_unit_kills_a_defense_unit_in_the_reply() {
        let after = reply(&crawlers(&[Keyword::FirstStrike, Keyword::Pierce]));
        assert!(in_graveyard(&after, AI, "classic-053"));
        assert!(on_field(&after, HUMAN, "classic-053"));
        assert_eq!(after.players[AI].hero.health, 30);
    }

    #[test]
    fn r346_without_pierce_the_opponent_keeps_off_the_defense_unit() {
        let after = reply(&crawlers(&[Keyword::FirstStrike]));
        assert!(on_field(&after, AI, "classic-053"));
        assert!(on_field(&after, HUMAN, "classic-053"));
    }

    #[test]
    fn r346_the_opponents_pierce_unit_hits_the_face_through_the_heros_armor() {
        let side = json!({ "health": 2, "armor": 2, "library": LIBRARY });
        let after = reply(&handed_over(side.clone(), &[Keyword::Pierce]));
        assert!(
            after
                .result
                .is_some_and(|result| result.winner.player() == Some(HUMAN))
        );
        let after = reply(&handed_over(side, &[]));
        assert!(after.result.is_none());
        assert_eq!(after.players[AI].hero.health, 2);
    }
}
