//! The combat-only reads in the AI's trade checks (#1094): the move order's winning trades
//! (`candidate_actions`) and the opponent's reply (`simulate_reply`) read a combat the way
//! `resolve_combat` does (SPEC §4.3–§4.4): the attacker's strike on a Unit adds the board's combat-only
//! attack bonus and its Poisonous (`attack_mod_for`; M #52 Economic Anxiety, R1120) and is multiplied
//! across its own lane (`lane_multiplier_for`; M #97.5 Bunker, R1283), and the strike back is the
//! defender's attack times its own multiplier, never bonused.
//!
//! The cards: Mr. Vanilla (core-008) 4/4 Human, Big Felinor (core-043) 3/10 Felinor, Genn
//! (classic-086) 14/14 untagged, Economic Anxiety (meditative-052, +3 while attacking a Unit that
//! shares no tag; Radiant: the controller's Units only, with Poisonous) and Bunker
//! (meditative-097-5) 5/10, Armor 3, Can't attack, First Strike, 3× on the Unit across its lane.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, on_field, register_cards};

const VANILLA: &str = "core-008";
const BIG_FELINOR: &str = "core-043";
const GENN: &str = "classic-086";
const ANXIETY: &str = "meditative-052";
const BUNKER: &str = "meditative-097-5";
const LIBRARY: &[&str] = &["core-008", "core-011", "core-008", "core-011"];

fn board(setup: Value) -> GameState {
    register_cards();
    scenario(setup).state().clone()
}

/// Turns `player`'s Economic Anxiety to its Radiant face (`SideSetup.backrow` takes no `radiant`).
fn make_radiant(state: &mut GameState, player: PlayerId) {
    let id = state.players[player]
        .backrow
        .iter()
        .flatten()
        .find(|card| card.def_id == ANXIETY)
        .map(|card| card.id.clone())
        .expect("Economic Anxiety stands in the backrow");
    find_instance_mut(state, &id).expect("the Field Spell").radiant = true;
}

mod move_order {
    use super::*;

    /// Where `candidate_actions` puts the AI's attack on the enemy's only unit and its first play.
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

    /// Big Felinor into Mr. Vanilla, Economic Anxiety in the AI's backrow when `anxiety`: a Felinor
    /// and a Human share no tag, so the bonus makes the 3 a 6 that kills, and the strike back of 4
    /// leaves the 3/10 standing.
    fn felinor_into_vanilla(anxiety: bool) -> GameState {
        let backrow: Vec<&str> = if anxiety { vec![ANXIETY] } else { vec![] };
        board(json!({
            "seed": "ai-combat-mods-bonus",
            "p1": { "hand": [VANILLA], "field": [BIG_FELINOR], "backrow": backrow },
            "p2": { "field": [VANILLA] },
        }))
    }

    #[test]
    fn a_kill_economic_anxietys_bonus_makes_is_a_good_trade_ahead_of_the_plays() {
        let (attack, play) = places(&felinor_into_vanilla(true));
        assert!(attack < play, "the kill at {attack}, the first play at {play}");
    }

    #[test]
    fn the_same_hit_without_economic_anxiety_waits_behind_the_plays() {
        let (attack, play) = places(&felinor_into_vanilla(false));
        assert!(attack > play, "the chip at {attack}, the first play at {play}");
    }

    /// Mr. Vanilla into Big Felinor behind Economic Anxiety: 4 + 3 is short of the 3/10's health,
    /// so only the Radiant face's Poisonous makes it a kill; the strike back of 3 leaves the 4/4
    /// standing either way.
    #[test]
    fn the_radiant_faces_poisonous_makes_a_kill_ahead_of_the_plays() {
        for (radiant, ahead) in [(true, true), (false, false)] {
            let mut state = board(json!({
                "seed": "ai-combat-mods-poisonous",
                "p1": { "hand": [VANILLA], "field": [VANILLA], "backrow": [ANXIETY] },
                "p2": { "field": [BIG_FELINOR] },
            }));
            if radiant {
                make_radiant(&mut state, AI);
            }
            let (attack, play) = places(&state);
            assert_eq!(
                attack < play,
                ahead,
                "Radiant {radiant}: the attack at {attack}, the first play at {play}"
            );
        }
    }

    /// Genn into the enemy's Bunker in lane 1: 14 − 3 kills the 5/10 from any lane, and Genn
    /// survives the Bunker's 5 back from another lane but not its 15 across its own.
    #[test]
    fn a_trade_into_a_bunkers_lane_strike_waits_behind_the_plays() {
        for (lane, ahead) in [(1, false), (2, true)] {
            let state = board(json!({
                "seed": "ai-combat-mods-bunker",
                "p1": { "hand": [VANILLA], "field": [{ "def": GENN, "lane": lane }] },
                "p2": { "field": [{ "def": BUNKER, "lane": 1 }] },
            }));
            let (attack, play) = places(&state);
            assert_eq!(
                attack < play,
                ahead,
                "Genn in lane {lane}: the attack at {attack}, the first play at {play}"
            );
        }
    }
}

mod reply {
    use super::*;

    const TURN: i32 = 9;

    /// p1's turn 9 with vanilla libraries on both sides, ended through the reducer: p2's turn 10.
    fn handed_over(p1: Value, p2: Value) -> GameState {
        register_cards();
        let mut options =
            json!({ "seed": "ai-combat-mods-reply", "active": "p1", "turn": TURN, "p1": p1, "p2": p2 });
        options["p1"]["library"] = json!(LIBRARY);
        options["p2"]["library"] = json!(LIBRARY);
        let s = scenario(options);
        let ended = act(s.state(), AI, &ActionBody::EndTurn);
        assert_eq!(ended.active, HUMAN);
        ended
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

    /// The AI's Mr. Vanilla behind its Economic Anxiety, facing the enemy's Big Felinor: the base face
    /// bonuses the enemy's attacker too, so its 3 + 3 kills the 4/4 and it survives the 4 back, a
    /// unit worth more than 3 to the face. The Radiant face bonuses only its controller's Units, so
    /// the enemy's 3 chips and it goes face instead.
    #[test]
    fn the_reply_takes_the_kill_the_base_bonus_gives_the_enemy_and_not_the_radiant_one() {
        for (radiant, kills) in [(false, true), (true, false)] {
            let mut state = handed_over(
                json!({ "field": [VANILLA], "backrow": [ANXIETY] }),
                json!({ "field": [BIG_FELINOR] }),
            );
            if radiant {
                make_radiant(&mut state, AI);
            }
            let after = reply(&state);
            assert_eq!(on_field(&after, AI, VANILLA), !kills, "Radiant {radiant}");
            let health = if kills { 30 } else { 27 };
            assert_eq!(after.players[AI].hero.health, health, "Radiant {radiant}");
        }
    }

    /// The AI's Bunker in Defense Position (its Taunt and Armor 4, §4.1) in lane 1, facing Genn in
    /// lane `genn_lane`: 14 − 4 kills the 5/10, so the Taunt leaves Genn that one attack.
    fn genn_at_a_bunker(genn_lane: i32) -> GameState {
        handed_over(
            json!({ "field": [{ "def": BUNKER, "lane": 1, "position": "DEF" }] }),
            json!({ "field": [{ "def": GENN, "lane": genn_lane }] }),
        )
    }

    /// Across its lane the Bunker strikes first for 5 × 3 = 15 and Genn falls before it lands: the
    /// trade is a loss, so the reply ends its turn instead.
    #[test]
    fn the_reply_does_not_walk_into_a_bunkers_lane_strike() {
        let after = reply(&genn_at_a_bunker(1));
        assert!(on_field(&after, HUMAN, GENN), "Genn did not attack");
        let bunker = active_units_of(&after, AI)
            .into_iter()
            .find(|unit| unit.def_id == BUNKER)
            .expect("the Bunker stands");
        assert_eq!(bunker.damage, 0);
        assert_eq!(after.players[AI].hero.health, 30);
        assert_eq!(after.active, AI);
    }

    /// From another lane the Bunker strikes first for its 5 alone, which Genn survives, and Genn's 14
    /// kills it.
    #[test]
    fn the_reply_still_kills_a_bunker_from_another_lane() {
        let after = reply(&genn_at_a_bunker(2));
        assert!(!on_field(&after, AI, BUNKER), "the Bunker fell");
        let genn = active_units_of(&after, HUMAN)
            .into_iter()
            .find(|unit| unit.def_id == GENN)
            .expect("Genn stands");
        assert_eq!(genn.damage, 5);
    }
}
