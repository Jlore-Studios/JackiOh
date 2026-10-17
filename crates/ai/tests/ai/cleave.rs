//! Cleave in the AI's own combat reads (R63): a bound Cleave attacker splashes its attack onto
//! the defender's live neighbouring Taunts, so a killed neighbour stops binding the remaining
//! attackers (`damage_past_taunts`); and the opponent's reply prices the splash, ranking a double
//! kill above a single kill and a kill-plus-pop above a bare kill (`trade_value`).
//!
//! The cards: Nurse Cleaver (classic-073) 3/6 Cleave, Zoomerbin Oomen (core-067) 1/2, Paul Allen's
//! Ghost (classic-089) 5/6 Divine Shield, Jay Fungus (classicplus-049) 3/6 Taunt, Midrange Menace
//! (core-019) 9/9 Taunt, Voidwalker (classic-050) 6/3, and Tempo Timmy (core-011) 3/3, the non-Cleave
//! control attacker. The shield is the Ghost rather than Jilliax: Jilliax's Taunt would bind every
//! reply attack to itself, and its direct hit pops the shield even without Cleave, leaving the
//! control with nothing to prove. Field entries without a `lane` fill consecutive lanes, so two
//! listed units are adjacent; the reply boards pin lanes to leave lane 3 empty.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, on_field, register_cards};

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

mod threat {
    use super::*;

    /// Two Jay Fungi (3/6 Taunt) in lanes 1-2; p2's Voidwalker (6) and Menace (9).
    fn wall() -> GameState {
        board(json!({
            "seed": "ai-cleave-threat",
            "p1": { "field": ["classicplus-049", "classicplus-049"] },
            "p2": { "field": ["classic-050", "core-019"] },
        }))
    }

    #[test]
    fn cleave_kill_of_a_neighbour_taunt_frees_the_remaining_attackers() {
        let mut state = wall();
        grant(&mut state, HUMAN, "classic-050", Keyword::Cleave);
        // The 6-attack cleaver kills the first Taunt and splashes the second dead, freeing the
        // 9-attack Menace for the face.
        assert_eq!(face_threat(&state, HUMAN), 9);
    }

    #[test]
    fn without_cleave_the_same_board_threatens_nothing() {
        // Each attacker is spent on one Taunt: with no splash nothing reaches the face.
        assert_eq!(face_threat(&wall(), HUMAN), 0);
    }
}

mod reply_cleave {
    use super::*;

    /// Oomens in lanes 1, 2 and 4, each damaged to 1 health (worth about 2.2): lane 3 empty,
    /// so a Cleave into lane 1 or 2 kills two. A lone kill loses to the face (3.0); only the double
    /// kill (about 4.4) beats it, so the test fails unless the reply prices the splash.
    fn oomen_field() -> Value {
        json!([
            { "def": "core-067", "lane": 1, "damage": 1 },
            { "def": "core-067", "lane": 2, "damage": 1 },
            { "def": "core-067", "lane": 4, "damage": 1 },
        ])
    }

    /// An Oomen in lane 1, the Ghost (Divine Shield) in lane 2, a Plague Crawler (2/2, worth
    /// about 4.4) in lane 4. The Crawler's lone kill beats the Oomen's (about 3.2), but the Oomen
    /// beside the Ghost is a kill plus a pop (about 5.2) — so the test fails unless the reply prices
    /// the splash.
    fn ghost_field() -> Value {
        json!([
            { "def": "core-067", "lane": 1 },
            { "def": "classic-089", "lane": 2 },
            { "def": "classic-053", "lane": 4 },
        ])
    }

    /// The AI's turn 9, with `field` on its side and p2's `attacker` on the field, ended through
    /// the reducer: p2's turn 10, which `simulate_reply` then plays.
    fn handed_over(field: Value, armor: i32, attacker: &str) -> GameState {
        let state = board(json!({
            "seed": "ai-cleave-reply",
            "active": "p1",
            "turn": 9,
            "p1": { "field": field, "armor": armor, "library": LIBRARY },
            "p2": { "field": [attacker], "library": LIBRARY },
        }));
        let ended = act(&state, AI, &ActionBody::EndTurn);
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

    /// Oomens in the AI's graveyard.
    fn dead_oomens(state: &GameState) -> usize {
        state.players[AI]
            .graveyard
            .iter()
            .filter(|card| card.def_id == "core-067")
            .count()
    }

    /// Whether `player`'s unit zone `lane` holds a card of `def_id`.
    fn unit_in_lane(state: &GameState, player: PlayerId, lane: i32, def_id: &str) -> bool {
        active_units_of(state, player)
            .iter()
            .any(|unit| unit.def_id == def_id && slot_of(state, unit).is_some_and(|slot| slot.lane == lane))
    }

    #[test]
    fn reply_spends_a_cleave_attack_on_the_double_kill() {
        let after = reply(&handed_over(oomen_field(), 0, "classic-073"));
        // The lane-1/-2 double kill (about 4.4) outranks the face (3.0), which outranks a lone
        // kill (about 2.2): one swing, two dead, the third standing, the hero untouched.
        assert_eq!(dead_oomens(&after), 2);
        assert!(unit_in_lane(&after, AI, 4, "core-067"));
        assert_eq!(after.players[AI].hero.health, 30);
        assert!(on_field(&after, HUMAN, "classic-073"));
    }

    #[test]
    fn reply_without_cleave_kills_exactly_one() {
        // Armor 2 puts the face (1.0) below the lone kill (about 2.2), as in the shield boards.
        let after = reply(&handed_over(oomen_field(), 2, "core-011"));
        assert_eq!(dead_oomens(&after), 1);
    }

    #[test]
    fn reply_pops_a_neighbour_shield_rather_than_a_bare_kill() {
        let after = reply(&handed_over(ghost_field(), 2, "classic-073"));
        // Kill plus pop (about 5.2) outranks the bare lane-4 kill (about 4.4) and the armored
        // face (1.0): the Oomen dies, the Crawler stands, the Ghost is unshielded.
        assert_eq!(dead_oomens(&after), 1);
        assert!(unit_in_lane(&after, AI, 4, "classic-053"));
        let ghost = active_units_of(&after, AI)
            .into_iter()
            .find(|unit| unit.def_id == "classic-089")
            .expect("the Ghost stands");
        assert!(!has_keyword(
            &unit_view(&after, ghost).keywords,
            KeywordKind::DivineShield
        ));
        assert!(on_field(&after, HUMAN, "classic-073"));
    }

    #[test]
    fn reply_without_cleave_leaves_the_shield_up() {
        let after = reply(&handed_over(ghost_field(), 2, "core-011"));
        // The bare Crawler kill (about 4.4) outranks the Oomen's (about 3.2), the pop (2.0) and
        // the armored face (1.0): the Crawler dies, the Oomen stands, the shield stays up.
        assert_eq!(dead_oomens(&after), 0);
        assert!(unit_in_lane(&after, AI, 1, "core-067"));
        let ghost = active_units_of(&after, AI)
            .into_iter()
            .find(|unit| unit.def_id == "classic-089")
            .expect("the Ghost stands");
        assert!(has_keyword(
            &unit_view(&after, ghost).keywords,
            KeywordKind::DivineShield
        ));
    }
}
