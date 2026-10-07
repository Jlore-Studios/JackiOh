//! Hand-built puzzles P1–P14 (docs/polish/3-ai.md B17–B19, the puzzle table), and P15–P17 for The
//! Coin (SPEC §2.1, §7, R244, R245), which the seat going second holds from its first turn.
//!
//! Every puzzle is a `scenario({ active: "p1", turn: 9, … })` with the AI as p1, p2 at 30 health
//! unless stated and every p1 unit not summoning sick (the harness default). The AI plays one whole
//! turn through `playAiTurn` at AI_BUDGET, the budget the browser plays at, and each puzzle holds
//! its stated assertion. The gate is at least 12 of the 14 passing: all 14 are written, and a
//! puzzle may be removed at reconcile only with a written reason in this file.
//!
//!   B17 lethal   P1–P6: the enemy hero is dead at the end of the AI's turn, and the AI's first
//!                decision already says "lethal".
//!   B18 tactics  P7–P10, P12, P13: no bad trade; survive a scripted all-out attack next turn;
//!                spend the mana on the stronger play; spend removal on the biggest threat.
//!   B19 prompts  P11 discovers True Strike off Reminisce and casts it for lethal; P14 answers
//!                Masochism Mask's start-of-turn prompt at 3 health without losing health or
//!                summoning the Spikey Pillow, through the search (reason "prompt").
//!   The Coin     P15 plays it for the mana a lethal line needs; P16 plays it to put a 2-drop down
//!                with 1 mana; P17 keeps it when the extra mana would buy nothing.
//!
//! Port of `packages/ai/test/puzzles.test.ts`. TS's per-test `{ timeout }` has no `cargo test` twin.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{
    AI, HUMAN, PuzzleRun, act, all_out_attack, card_by_id, in_graveyard, on_field, run_puzzle, trace,
};

/// TS's `{ rng: createRng(seed), budget: AI_BUDGET }`, with no clock.
fn ai_options(seed: &str) -> AiOptions<'static> {
    AiOptions { rng: create_rng(seed, 0), budget: AI_BUDGET, should_stop: None }
}

/// `runPuzzle(name, setup)` at its default budget (AI_BUDGET).
fn puzzle(name: &str, setup: Value) -> PuzzleRun {
    jackioh_cards::register_all();
    run_puzzle(name, setup, None)
}

fn expect_enemy_dead(run: &PuzzleRun) {
    assert_eq!(run.end.result.as_ref().map(|result| result.winner), Some(Winner::from(AI)), "{}", trace(&run.turn));
    assert!(run.end.players[HUMAN].hero.health <= 0, "{}", trace(&run.turn));
}

fn expect_first_reason_lethal(run: &PuzzleRun) {
    assert!(!run.turn.decisions.is_empty(), "{}", trace(&run.turn));
    assert_eq!(run.turn.decisions.first().map(|decision| decision.reason), Some(DecisionReason::Lethal), "{}", trace(&run.turn));
}

/// After the AI's turn, p2 swings with everything; p1 must still be standing.
fn expect_survives_all_out(run: &PuzzleRun) -> GameState {
    assert!(run.end.result.is_none(), "{}", trace(&run.turn));
    let after = all_out_attack(&run.end, HUMAN);
    assert_ne!(after.result.as_ref().map(|result| result.winner), Some(Winner::from(HUMAN)), "{}", trace(&run.turn));
    assert!(after.players[AI].hero.health > 0, "{}", trace(&run.turn));
    after
}

// ---------------------------------------------------------------------------------------------
// B17: lethal
// ---------------------------------------------------------------------------------------------

mod lethal_b17 {
    use super::*;

    /// B17 P1: two attackers into an empty board finish a 6-health hero, lethal from the first decision
    #[test]
    fn b17_p1_two_attackers_into_an_empty_board_finish_a_6_health_hero_lethal_from_the_first_decision() {
        let run = puzzle("P1", json!({ "p1": { "field": ["core-011", "core-008"] }, "p2": { "health": 6 } }));
        expect_enemy_dead(&run);
        expect_first_reason_lethal(&run);
    }

    /// B17 P2: Pointmaster clears the Defense-Position blocker and the rest go face for 6
    #[test]
    fn b17_p2_pointmaster_clears_the_defense_position_blocker_and_the_rest_go_face_for_6() {
        let run = puzzle(
            "P2",
            json!({
                "p1": { "field": ["core-020", "core-011", "core-008"] },
                "p2": { "field": [{ "def": "core-008", "position": "DEF" }], "health": 6 },
            }),
        );
        expect_enemy_dead(&run);
        expect_first_reason_lethal(&run);
    }

    /// B17 P3: Plastic Surgery on Tempo Timmy, then the attack, for 6
    #[test]
    fn b17_p3_plastic_surgery_on_tempo_timmy_then_the_attack_for_6() {
        let run = puzzle(
            "P3",
            json!({ "p1": { "hand": ["core-063"], "mana": 1, "field": ["core-011"] }, "p2": { "health": 6 } }),
        );
        expect_enemy_dead(&run);
        expect_first_reason_lethal(&run);
        assert_eq!(
            run.turn.decisions.first().map(|decision| decision.action.action_type()),
            Some(ActionType::Play),
            "{}",
            trace(&run.turn)
        );
    }

    /// B17 P4: True Strike to the face past a Taunt the attacker cannot get through
    #[test]
    fn b17_p4_true_strike_to_the_face_past_a_taunt_the_attacker_cannot_get_through() {
        let run = puzzle(
            "P4",
            json!({
                "p1": { "hand": ["core-044"], "mana": 1, "field": ["core-011"] },
                "p2": { "field": ["core-019"], "health": 4 },
            }),
        );
        expect_enemy_dead(&run);
        expect_first_reason_lethal(&run);
    }

    /// B17 P5: Deft Duelist's Charge from hand into an empty board for 4
    #[test]
    fn b17_p5_deft_duelists_charge_from_hand_into_an_empty_board_for_4() {
        let run = puzzle("P5", json!({ "p1": { "hand": ["core-045"], "mana": 2 }, "p2": { "health": 4 } }));
        expect_enemy_dead(&run);
        expect_first_reason_lethal(&run);
    }

    /// B17 P6: Lunar Eclipse to the face plus Tempo Timmy's attack for 6
    #[test]
    fn b17_p6_lunar_eclipse_to_the_face_plus_tempo_timmys_attack_for_6() {
        let run = puzzle(
            "P6",
            json!({ "p1": { "hand": ["core-035"], "mana": 1, "field": ["core-011"] }, "p2": { "health": 6 } }),
        );
        expect_enemy_dead(&run);
        expect_first_reason_lethal(&run);
    }
}

// ---------------------------------------------------------------------------------------------
// B18: tactics
// ---------------------------------------------------------------------------------------------

mod tactics_b18 {
    use super::*;

    /// B18 P7: Mr. Vanilla does not throw itself into the 7/7 with Armor 7
    #[test]
    fn b18_p7_mr_vanilla_does_not_throw_itself_into_the_7_7_with_armor_7() {
        let run = puzzle("P7", json!({ "p1": { "field": ["core-008"] }, "p2": { "field": ["core-025"] } }));
        assert!(on_field(&run.end, AI, "core-008"), "{}", trace(&run.turn));
    }

    /// B18 P8: at 8 health against 7 + 3 on board, the 7/7 deals with the threat and p1 survives the swing back
    #[test]
    fn b18_p8_at_8_health_against_7_plus_3_on_board_the_7_7_deals_with_the_threat_and_p1_survives_the_swing_back() {
        let run = puzzle(
            "P8",
            json!({ "p1": { "health": 8, "field": ["core-025"] }, "p2": { "field": ["core-020", "core-011"] } }),
        );
        expect_survives_all_out(&run);
    }

    /// B18 P9: at 5 health with an empty board, Jilliax goes down to block 4 + 3 and p1 survives the swing back
    #[test]
    fn b18_p9_at_5_health_with_an_empty_board_jilliax_goes_down_to_block_4_plus_3_and_p1_survives_the_swing_back() {
        let run = puzzle(
            "P9",
            json!({ "p1": { "health": 5, "hand": ["core-056"], "mana": 2 }, "p2": { "field": ["core-045", "core-011"] } }),
        );
        expect_survives_all_out(&run);
    }

    /// B18 P10: at 3 health facing Tempo Timmy, Fig of Life heals and p1 survives the swing back
    #[test]
    fn b18_p10_at_3_health_facing_tempo_timmy_fig_of_life_heals_and_p1_survives_the_swing_back() {
        let run = puzzle(
            "P10",
            json!({ "p1": { "health": 3, "hand": ["core-047"], "mana": 3 }, "p2": { "field": ["core-011"] } }),
        );
        expect_survives_all_out(&run);
    }

    /// B18 P12: with 4 mana on empty boards the AI plays the 4-mana 7/7, not the 1-drop
    #[test]
    fn b18_p12_with_4_mana_on_empty_boards_the_ai_plays_the_4_mana_7_7_not_the_1_drop() {
        let run = puzzle("P12", json!({ "p1": { "hand": ["core-025", "core-011"], "mana": 4 }, "p2": {} }));
        assert!(on_field(&run.end, AI, "core-025"), "{}", trace(&run.turn));
    }

    /// B18 P13: Hit Job goes on Midrange Menace, the biggest threat
    #[test]
    fn b18_p13_hit_job_goes_on_midrange_menace_the_biggest_threat() {
        let run = puzzle(
            "P13",
            json!({
                // Hit Job costs (3) since patch v0.2.0.
                "p1": { "hand": ["core-016"], "mana": 3, "field": ["core-011"] },
                "p2": { "field": ["core-019", "core-008"] },
            }),
        );
        assert!(in_graveyard(&run.end, HUMAN, "core-019"), "{}", trace(&run.turn));
    }
}

// ---------------------------------------------------------------------------------------------
// B19: prompts through the search
// ---------------------------------------------------------------------------------------------

mod prompts_through_the_search_b19 {
    use super::*;

    /// B19 P11: Reminisce discovers True Strike from the graveyard, and True Strike finishes the hero
    #[test]
    fn b19_p11_reminisce_discovers_true_strike_from_the_graveyard_and_true_strike_finishes_the_hero() {
        jackioh_cards::register_all();
        let s = scenario(json!({
            "seed": "puzzle-P11",
            "active": "p1",
            "turn": 9,
            "p1": { "hand": ["core-072"], "mana": 1, "graveyard": ["core-044", "core-008", "core-005"] },
            "p2": { "health": 4, "field": ["core-019"] },
        }));
        let true_strike = s
            .pile(AI, "graveyard")
            .into_iter()
            .find(|card| card.def_id == "core-044")
            .unwrap_or_else(|| panic!("True Strike is not in p1's graveyard"));

        let start = s.state().clone();
        let turn = play_ai_turn(&start, AI, &mut ai_options("puzzle:P11"));
        let end = turn.state.clone();
        let run = PuzzleRun { start, turn, end };
        expect_enemy_dead(&run);

        let discover = run.turn.decisions.iter().find(|decision| matches!(decision.action, ActionBody::Answer { .. }));
        assert!(discover.is_some(), "{}", trace(&run.turn));
        let selection = match discover.map(|decision| &decision.action) {
            Some(ActionBody::Answer { selection, .. }) => selection.clone(),
            _ => Vec::new(),
        };
        assert_eq!(
            selection,
            vec![Selection::Instance { instance_id: true_strike.id.clone() }],
            "{}",
            trace(&run.turn)
        );
        // The discovered True Strike is the card the AI then played.
        assert!(
            run.turn.actions.iter().any(
                |action| matches!(&action.body, ActionBody::Play { instance_id, .. } if *instance_id == true_strike.id)
            ),
            "{}",
            trace(&run.turn)
        );
        assert_ne!(
            card_by_id(&run.turn.state, &true_strike.id).map(|card| card.zone.z()),
            Some(ZoneName::Hand),
            "{}",
            trace(&run.turn)
        );
    }

    /// B19 P14: at 3 health the AI answers Masochism Mask with neither 'lose 3' nor the Spikey Pillow, through the search
    #[test]
    fn b19_p14_at_3_health_the_ai_answers_masochism_mask_with_neither_lose_3_nor_the_spikey_pillow_through_the_search() {
        // The table gives p1 "a card in hand and in library". The hand card is a unit, so the Pillow's
        // aura (−2 attack to your units) is a cost the search can see; the library holds two cards so
        // that exiling its bottom card can never leave the turn's draw to fatigue, in either order.
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "puzzle-P14",
            "active": "p2",
            "turn": 10,
            "p1": { "health": 3, "backrow": ["core-065"], "hand": ["core-019"], "library": ["core-008", "core-011"] },
            "p2": { "hand": ["core-005"] },
        }));
        s.end_turn();
        let state = s.state().clone();
        assert_eq!(state.active, AI);
        assert_eq!(
            state.pending.as_ref().map(|pending| pending.player_id),
            Some(AI),
            "Masochism Mask asks at the start of p1's turn"
        );
        assert_eq!(state.pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Mode));
        assert_eq!(state.players[AI].hero.health, 3);

        let decision = decide(&state, AI, &mut ai_options("puzzle:P14"));
        assert_eq!(decision.as_ref().map(|decision| decision.reason), Some(DecisionReason::Prompt));
        assert_eq!(decision.as_ref().map(|decision| decision.action.action_type()), Some(ActionType::Answer));

        let after = act(&state, AI, &decision.expect("a decision").action, None);
        assert!(after.result.is_none());
        assert_eq!(after.players[AI].hero.health, 3);
        assert!(!on_field(&after, AI, "core-065-1"));
    }
}

// ---------------------------------------------------------------------------------------------
// The Coin (R244, R245): a 0-cost Spell token, "gain 1 mana this turn"
// ---------------------------------------------------------------------------------------------

const COIN: &str = "core-t-coin";

fn in_hand(state: &GameState, player: PlayerId, def_id: &str) -> bool {
    state.players[player].hand.iter().any(|card| card.def_id == def_id)
}

mod the_coin_r245 {
    use super::*;

    /// P15: with 1 mana, The Coin pays for Deft Duelist's Charge, and Duelist plus Tempo Timmy finish a 7-health hero
    #[test]
    fn p15_with_1_mana_the_coin_pays_for_deft_duelists_charge_and_duelist_plus_tempo_timmy_finish_a_7_health_hero() {
        let run = puzzle(
            "P15",
            json!({ "p1": { "hand": [COIN, "core-045"], "mana": 1, "field": ["core-011"] }, "p2": { "health": 7 } }),
        );
        expect_enemy_dead(&run);
        expect_first_reason_lethal(&run);
        assert!(in_graveyard(&run.end, AI, COIN), "{}", trace(&run.turn));
    }

    /// P16: with 1 mana on empty boards, the AI plays The Coin and then Pointmaster rather than pass
    #[test]
    fn p16_with_1_mana_on_empty_boards_the_ai_plays_the_coin_and_then_pointmaster_rather_than_pass() {
        let run = puzzle("P16", json!({ "p1": { "hand": [COIN, "core-020"], "mana": 1 }, "p2": {} }));
        assert!(on_field(&run.end, AI, "core-020"), "{}", trace(&run.turn));
        assert!(in_graveyard(&run.end, AI, COIN), "{}", trace(&run.turn));
    }

    /// P17: with 1 mana and only a 1-drop to spend it on, the AI plays the 1-drop and keeps The Coin
    #[test]
    fn p17_with_1_mana_and_only_a_1_drop_to_spend_it_on_the_ai_plays_the_1_drop_and_keeps_the_coin() {
        let run = puzzle("P17", json!({ "p1": { "hand": [COIN, "core-011"], "mana": 1 }, "p2": {} }));
        assert!(on_field(&run.end, AI, "core-011"), "{}", trace(&run.turn));
        assert!(in_hand(&run.end, AI, COIN), "{}", trace(&run.turn));
    }
}
