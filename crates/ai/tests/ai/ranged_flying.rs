//! Patch v0.3.7's combat keywords in the AI's own combat reads (#841): a Ranged attacker's kill of a
//! unit that is not Ranged takes no strike back (R783000), in the move order and in the opponent's
//! reply, a Flying Taunt in Attack Position binds only the Flying and Ranged attackers (R783002) and a
//! landed one every attacker (R783003), in `face_threat` and the lethal solver's `ready_gap`, and
//! `KeywordWeights` weighs each keyword the patch shipped.
//!
//! No card prints Ranged or Flying yet (§6.1: the random keyword pool only), so each board grants the
//! keyword to a unit on the field. The cards: Mr. Vanilla (core-008) 4/4, Tempo Timmy (core-011) 3/3,
//! Pointmaster (core-020) 7/1 and Midrange Menace (core-019) 9/9 Taunt.

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

mod r783000_move_order {
    use super::*;

    /// The AI's Mr. Vanilla 4/4 ready on the field, another in hand, and the enemy's Mr. Vanilla: a
    /// kill either way, which the AI's survives only when it is Ranged and the enemy's is not. Returns
    /// the enemy unit's id.
    fn trade(attacker_ranged: bool, defender_ranged: bool) -> (GameState, String) {
        let mut state = board(json!({
            "seed": "ai-ranged-order",
            "p1": { "hand": ["core-008"], "field": ["core-008"] },
            "p2": { "field": ["core-008"] },
        }));
        if attacker_ranged {
            grant(&mut state, AI, "core-008", Keyword::Ranged);
        }
        if defender_ranged {
            grant(&mut state, HUMAN, "core-008", Keyword::Ranged);
        }
        let target = active_units_of(&state, HUMAN)[0].id.clone();
        (state, target)
    }

    /// Where `candidate_actions` puts the attack on `target` and the first play.
    fn places(state: &GameState, target: &str) -> (usize, usize) {
        let actions = candidate_actions(state, AI);
        let attack = actions
            .iter()
            .position(|action| matches!(action, ActionBody::Attack { target_id, .. } if target_id == target))
            .expect("the attack on the enemy unit is a candidate");
        let play = actions
            .iter()
            .position(|action| matches!(action, ActionBody::Play { .. }))
            .expect("the hand card's play is a candidate");
        (attack, play)
    }

    #[test]
    fn r783000_a_ranged_attackers_kill_is_a_good_trade_ahead_of_the_plays() {
        let (state, target) = trade(true, false);
        let (attack, play) = places(&state, &target);
        assert!(
            attack < play,
            "the free kill at {attack}, the first play at {play}"
        );
    }

    #[test]
    fn r783000_the_same_kill_without_ranged_or_into_a_ranged_defender_waits_behind_the_plays() {
        for (attacker_ranged, defender_ranged) in [(false, false), (true, true)] {
            let (state, target) = trade(attacker_ranged, defender_ranged);
            let (attack, play) = places(&state, &target);
            assert!(
                attack > play,
                "Ranged {attacker_ranged} into Ranged {defender_ranged}: the trade at {attack}, the first play at {play}"
            );
        }
    }
}

mod r783000_reply {
    use super::*;

    /// Mr. Vanilla against Mr. Vanilla, the enemy's Ranged (and the AI's too when `p1_ranged`), with
    /// the AI's turn 9 ended through the reducer: the enemy's turn 10.
    fn handed_over(p1_ranged: bool) -> GameState {
        let mut state = board(json!({
            "seed": "ai-ranged-reply",
            "active": "p1",
            "turn": 9,
            "p1": { "field": ["core-008"], "library": LIBRARY },
            "p2": { "field": ["core-008"], "library": LIBRARY },
        }));
        grant(&mut state, HUMAN, "core-008", Keyword::Ranged);
        if p1_ranged {
            grant(&mut state, AI, "core-008", Keyword::Ranged);
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

    #[test]
    fn r783000_the_opponents_ranged_unit_takes_a_free_kill_over_the_face() {
        let after = reply(&handed_over(false));
        assert!(in_graveyard(&after, AI, "core-008"));
        assert!(on_field(&after, HUMAN, "core-008"));
        assert_eq!(after.players[AI].hero.health, 30);
    }

    #[test]
    fn r783000_into_our_ranged_unit_the_trade_is_even_and_the_opponent_goes_face() {
        let after = reply(&handed_over(true));
        assert!(on_field(&after, AI, "core-008"));
        assert_eq!(after.players[AI].hero.health, 26);
    }
}

mod r783002_r783003_taunts {
    use super::*;

    /// The AI's Midrange Menace 9/9 Taunt with Flying, in `position`, against the enemy's Tempo Timmy
    /// 3/3 and Pointmaster 7/1, the Pointmaster Flying too when `flying_pointmaster`.
    fn flying_menace(position: &str, flying_pointmaster: bool) -> GameState {
        let mut state = board(json!({
            "seed": "ai-flying-taunt",
            "p1": { "field": [{ "def": "core-019", "position": position }] },
            "p2": { "field": ["core-011", "core-020"] },
        }));
        grant(&mut state, AI, "core-019", Keyword::Flying);
        if flying_pointmaster {
            grant(&mut state, HUMAN, "core-020", Keyword::Flying);
        }
        state
    }

    #[test]
    fn r783002_a_flying_taunt_in_attack_position_holds_no_ground_attacker() {
        assert_eq!(face_threat(&flying_menace("ATK", false), HUMAN), 10);
    }

    #[test]
    fn r783002_a_flying_taunt_holds_the_flying_attacker_it_survives_and_lets_the_ground_one_past() {
        assert_eq!(face_threat(&flying_menace("ATK", true), HUMAN), 3);
    }

    #[test]
    fn r783003_a_flying_taunt_in_defense_position_has_landed_and_soaks_every_attacker() {
        assert_eq!(face_threat(&flying_menace("DEF", false), HUMAN), 0);
    }

    #[test]
    fn r783002_ready_gap_counts_the_ground_attackers_past_a_lone_flying_taunt() {
        let mut state = board(json!({
            "seed": "ai-flying-gap",
            "p1": { "field": ["core-011", "core-020"] },
            "p2": { "field": ["core-019"], "health": 10 },
        }));
        grant(&mut state, HUMAN, "core-019", Keyword::Flying);
        assert_eq!(ready_gap(&state, AI), 0.0);
    }
}

mod keyword_weights {
    use super::*;

    #[test]
    fn ai_eval_weighs_every_keyword_v0_3_7_shipped_and_the_frozen_baseline_gives_each_nothing() {
        for kind in [
            KeywordKind::Ranged,
            KeywordKind::Flying,
            KeywordKind::Cloaked,
            KeywordKind::Crater,
            KeywordKind::Flashback,
        ] {
            assert!(AI_EVAL.keyword.of(kind).is_some(), "{kind:?}");
            assert_eq!(GREEDY_EVAL.keyword.of(kind), Some(0.0), "{kind:?}");
        }
    }

    #[test]
    fn r783000_r783002_a_ranged_or_flying_unit_is_worth_its_keyword_weight_more_than_the_vanilla() {
        let state = board(json!({ "p1": { "field": ["core-008"] } }));
        let plain = active_units_of(&state, AI)[0].clone();
        for (keyword, weight) in [
            (Keyword::Ranged, AI_EVAL.keyword.ranged),
            (Keyword::Flying, AI_EVAL.keyword.flying),
        ] {
            let mut granted = plain.clone();
            granted.granted_keywords.push(keyword);
            let diff = unit_worth(&state, &granted, &AI_EVAL) - unit_worth(&state, &plain, &AI_EVAL);
            assert!((diff - weight).abs() < 1e-9, "{diff} against {weight}");
            assert!(diff > 0.0);
        }
    }
}
