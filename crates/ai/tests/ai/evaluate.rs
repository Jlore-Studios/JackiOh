//! The AI's board evaluation (SPEC §9.9 "Evaluation"; docs/polish/3-ai.md B13).
//!
//! The weights in AI_EVAL are tuning defaults the builder may change, so nothing here pins a number
//! `evaluate` returns. What is pinned is the ordering the behaviour states: a won game over any
//! unfinished one over a lost one; more own health, a bigger own board and a smaller enemy board
//! each strictly better; and a Taunt blocker that absorbs a lethal board strictly better than none.
//! `face_threat` is pinned exactly where its surface text fixes the arithmetic.
//!
//! Port of `packages/ai/test/evaluate.test.ts`. TS's `evaluate(state, seat)` defaults are written
//! out (`NextSwing::Enemy`, `AI_EVAL`) by `eval`.

use jackioh_ai::{AI_EVAL, NextSwing, evaluate, face_threat, unit_worth};
use jackioh_engine::testkit::{
    AI_DIFFICULTY, CardInstance, GameOverReason, GameResult, GameState, HERO_HEALTH, Phase, PlayerId, Value,
    Winner, json,
};

use super::support::{AI, clone, register_cards, scenario};

fn board(opts: Value) -> GameState {
    register_cards();
    let mut options = json!({ "seed": "evaluate" });
    if let (Some(base), Value::Object(extra)) = (options.as_object_mut(), opts) {
        for (key, value) in extra {
            base.insert(key, value);
        }
    }
    scenario(options).state().clone()
}

/// `evaluate(state, seat)` with TS's defaults.
fn eval(state: &GameState, seat: PlayerId) -> f64 {
    evaluate(state, seat, NextSwing::Enemy, &AI_EVAL)
}

fn with_health(state: &GameState, player: PlayerId, health: i32) -> GameState {
    let mut out = clone(state);
    out.players[player].hero.health = health;
    out
}

fn finished(state: &GameState, winner: Winner) -> GameState {
    let mut out = clone(state);
    out.result = Some(GameResult {
        winner,
        reason: if winner == Winner::Draw {
            GameOverReason::TurnCap
        } else {
            GameOverReason::HeroDeath
        },
    });
    out.phase = Phase::Over;
    out
}

/// Jest's `toBeCloseTo(expected, digits)`: within half a unit of the last digit.
fn assert_close(received: f64, expected: f64, digits: i32) {
    assert!(
        (expected - received).abs() < 10f64.powi(-digits) / 2.0,
        "{received} is not close to {expected}"
    );
}

fn base() -> Value {
    json!({
        "p1": { "hand": ["core-008"], "field": ["core-011"] },
        "p2": { "hand": ["core-005"], "field": ["core-008"] },
    })
}

mod evaluate_b13 {
    use super::*;

    #[test]
    fn b13_a_won_game_scores_above_any_unfinished_one_and_a_lost_game_below_any() {
        let best = board(json!({
            "p1": { "field": ["core-019", "core-025", "core-020", "core-011", "core-008"], "hand": ["core-044"], "health": HERO_HEALTH },
            "p2": { "health": 1 },
        }));
        let worst = board(json!({
            "p1": { "health": 1 },
            "p2": { "field": ["core-019", "core-025", "core-020", "core-011", "core-008"], "hand": ["core-044", "core-035"] },
        }));
        let middle = board(base());

        for unfinished in [&best, &worst, &middle] {
            assert!(eval(&finished(unfinished, Winner::P1), AI) > eval(&best, AI));
            assert!(eval(&finished(unfinished, Winner::P2), AI) < eval(&worst, AI));
        }
        assert!(eval(&best, AI) > eval(&worst, AI));
    }

    #[test]
    fn b13_a_drawn_game_scores_ai_eval_drawn_between_any_win_and_any_loss() {
        let base = board(base());
        let drawn = eval(&finished(&base, Winner::Draw), AI);
        assert_eq!(drawn, AI_EVAL.drawn);
        assert!(drawn < eval(&finished(&base, Winner::P1), AI));
        assert!(drawn > eval(&finished(&base, Winner::P2), AI));
    }

    #[test]
    fn b13_a_win_the_engine_really_reached_scores_above_the_state_before_it_from_the_winners_side_only() {
        register_cards();
        let mut s = scenario(json!({
            "seed": "evaluate-kill",
            "p1": { "field": ["core-011"], "hand": ["core-008"] },
            "p2": { "health": 3, "hand": ["core-005"] },
        }));
        let before = s.state().clone();
        s.attack("core-011", "hero");
        let after = s.state().clone();

        assert_eq!(after.result.map(|result| result.winner), Some(Winner::P1));
        assert!(eval(&after, AI) > eval(&before, AI));
        assert!(eval(&after, PlayerId::P2) < eval(&before, PlayerId::P2));
    }

    #[test]
    fn b13_more_own_hero_health_strictly_raises_the_score_at_every_health_from_1_to_30() {
        let base = board(base());
        for health in 1..HERO_HEALTH {
            let lower = eval(&with_health(&base, AI, health), AI);
            let higher = eval(&with_health(&base, AI, health + 1), AI);
            assert!(higher > lower, "{} over {health}", health + 1);
        }
    }

    #[test]
    fn b13_less_enemy_hero_health_raises_the_score() {
        let base = board(base());
        assert!(
            eval(&with_health(&base, PlayerId::P2, 10), AI) > eval(&with_health(&base, PlayerId::P2, 20), AI)
        );
    }

    #[test]
    fn b13_a_bigger_own_board_strictly_raises_the_score() {
        let p2 = base()["p2"].clone();
        let empty = board(json!({ "p1": {}, "p2": p2 }));
        let one = board(json!({ "p1": { "field": ["core-011"] }, "p2": p2 }));
        let two = board(json!({ "p1": { "field": ["core-011", "core-020"] }, "p2": p2 }));
        assert!(eval(&one, AI) > eval(&empty, AI));
        assert!(eval(&two, AI) > eval(&one, AI));
    }

    #[test]
    fn b13_a_smaller_enemy_board_strictly_raises_the_score() {
        let p1 = base()["p1"].clone();
        let two = board(json!({ "p1": p1, "p2": { "field": ["core-008", "core-020"] } }));
        let one = board(json!({ "p1": p1, "p2": { "field": ["core-008"] } }));
        let none = board(json!({ "p1": p1, "p2": {} }));
        assert!(eval(&one, AI) > eval(&two, AI));
        assert!(eval(&none, AI) > eval(&one, AI));
    }

    #[test]
    fn b13_each_unseen_card_in_the_enemys_hand_lowers_the_score() {
        let p1 = base()["p1"].clone();
        let none = board(json!({ "p1": p1, "p2": { "field": ["core-008"] } }));
        let one = board(json!({ "p1": p1, "p2": { "field": ["core-008"], "hand": ["core-005"] } }));
        let three = board(
            json!({ "p1": p1, "p2": { "field": ["core-008"], "hand": ["core-005", "core-044", "core-035"] } }),
        );
        assert!(eval(&one, AI) < eval(&none, AI));
        assert!(eval(&three, AI) < eval(&one, AI));
    }

    #[test]
    fn b13_a_lethal_enemy_board_scores_lower_than_the_same_board_plus_our_taunt_blocker_that_absorbs_it() {
        let enemy = json!({ "field": ["core-011", "core-020"] });
        let exposed = board(json!({ "p1": { "health": 5 }, "p2": enemy }));
        let blocked = board(json!({ "p1": { "health": 5, "field": ["core-019"] }, "p2": enemy }));

        assert!(face_threat(&exposed, PlayerId::P2) >= 5);
        assert!(face_threat(&blocked, PlayerId::P2) < 5);
        assert!(eval(&blocked, AI) > eval(&exposed, AI));
    }

    #[test]
    fn r180_b13_evaluate_reads_no_handicap_so_every_tier_scores_a_state_alike() {
        let base = board(base());
        let mut hard = clone(&base);
        hard.players.p1.handicap = Some(AI_DIFFICULTY.hard);
        let mut medium = clone(&base);
        medium.players.p2.handicap = Some(AI_DIFFICULTY.medium);
        assert_eq!(eval(&hard, AI), eval(&base, AI));
        assert_eq!(eval(&medium, AI), eval(&base, AI));
    }

    #[test]
    fn b13_evaluate_is_pure_it_neither_mutates_the_state_nor_varies_between_calls() {
        let base = board(base());
        let before = serde_json::to_string(&base).unwrap();
        let first = eval(&base, AI);
        assert_eq!(eval(&base, AI), first);
        assert_eq!(serde_json::to_string(&base).unwrap(), before);
    }
}

mod evaluate_the_strength_passs_terms {
    use super::*;

    #[test]
    fn every_point_of_damage_on_the_enemy_hero_is_worth_at_least_ai_eval_enemy_health() {
        let state = board(base());
        for health in [30, 20, 10, 2] {
            let gain = eval(&with_health(&state, PlayerId::P2, health - 1), AI)
                - eval(&with_health(&state, PlayerId::P2, health), AI);
            assert!(gain >= AI_EVAL.enemy_health, "from {health}");
        }
    }

    #[test]
    fn scored_where_the_seat_moves_first_the_enemys_threat_counts_only_its_answerable_share() {
        // Mr. Vanilla threatens 4 into an empty board at 30 health: no lethal either way, and p1 has no
        // attacker, so the frames differ only in the threat term.
        let state = board(json!({ "p1": {}, "p2": { "field": ["core-008"] } }));
        assert_eq!(face_threat(&state, PlayerId::P2), 4);
        assert_eq!(face_threat(&state, AI), 0);
        let threat = AI_EVAL.threat_per_damage * 4.0;
        assert_close(
            evaluate(&state, AI, NextSwing::Seat, &AI_EVAL)
                - evaluate(&state, AI, NextSwing::Enemy, &AI_EVAL),
            (1.0 - AI_EVAL.answerable_threat) * threat,
            10,
        );
    }

    #[test]
    fn defense_positions_taunt_and_armor_add_no_worth_of_their_own_the_unit_loses_only_its_attack_share() {
        let attack = board(json!({ "p1": { "field": ["core-008"] } }));
        let defense = board(json!({ "p1": { "field": [{ "def": "core-008", "position": "DEF" }] } }));
        let unit_in = |state: &GameState| -> Option<CardInstance> {
            state.players[AI].units[0]
                .as_ref()
                .and_then(|pile| pile.first())
                .cloned()
        };
        let (Some(atk), Some(def)) = (unit_in(&attack), unit_in(&defense)) else {
            panic!("Mr. Vanilla is not on p1's field");
        };
        // Mr. Vanilla is 4/4 with no printed Taunt or Armor.
        let lost = (1.0 - AI_EVAL.defense_attack_share) * AI_EVAL.attack * 4.0;
        let kept = AI_EVAL.position_grants * (AI_EVAL.keyword.taunt + AI_EVAL.armor_point);
        assert_close(
            unit_worth(&attack, &atk, &AI_EVAL) - unit_worth(&defense, &def, &AI_EVAL),
            lost - kept,
            10,
        );
    }
}

mod face_threat_b13 {
    use super::*;

    /// 3 and 7 attack, both in Attack Position.
    fn enemy() -> Value {
        json!(["core-011", "core-020"])
    }

    #[test]
    fn b13_sums_the_enemys_attackers_into_our_face_when_nothing_blocks() {
        assert_eq!(
            face_threat(
                &board(json!({ "p1": {}, "p2": { "field": enemy() } })),
                PlayerId::P2
            ),
            10
        );
        assert_eq!(
            face_threat(&board(json!({ "p1": {}, "p2": {} })), PlayerId::P2),
            0
        );
    }

    #[test]
    fn b13_hero_armor_is_subtracted_from_each_hit_not_from_the_total() {
        assert_eq!(
            face_threat(
                &board(json!({ "p1": { "armor": 2 }, "p2": { "field": enemy() } })),
                PlayerId::P2
            ),
            6
        );
    }

    #[test]
    fn b13_a_unit_in_defense_position_is_not_an_attacker() {
        let state = board(
            json!({ "p1": {}, "p2": { "field": ["core-011", { "def": "core-020", "position": "DEF" }] } }),
        );
        assert_eq!(face_threat(&state, PlayerId::P2), 3);
    }

    #[test]
    fn b13_a_big_taunt_soaks_up_every_attacker_that_its_health_covers_smallest_first() {
        assert_eq!(
            face_threat(
                &board(json!({ "p1": { "field": ["core-019"] }, "p2": { "field": enemy() } })),
                PlayerId::P2
            ),
            0
        );
    }

    #[test]
    fn b13_a_small_taunt_with_divine_shield_soaks_the_smallest_attacker_plus_one_more_and_the_rest_get_through()
     {
        // Jilliax (3/2 Taunt, Divine Shield) against 3, 3 and 7: the two 3s are spent on it, 7 reaches us.
        let state = board(
            json!({ "p1": { "field": ["core-056"] }, "p2": { "field": ["core-011", "core-008", "core-020"] } }),
        );
        assert_eq!(face_threat(&state, PlayerId::P2), 7);
    }

    #[test]
    fn b13_attack_is_read_through_the_layers_a_0_attack_unit_threatens_nothing_and_its_aura_drains_the_rest()
    {
        // #65.1 Spikey Pillow (0/2, "your units have −2 attack") beside Tempo Timmy (3): only 1 gets through.
        let state = board(json!({ "p1": {}, "p2": { "field": ["core-065-1", "core-011"] } }));
        assert_eq!(face_threat(&state, PlayerId::P2), 1);
        assert_eq!(
            face_threat(
                &board(json!({ "p1": {}, "p2": { "field": ["core-065-1"] } })),
                PlayerId::P2
            ),
            0
        );
    }

    #[test]
    fn b13_the_anti_oneshot_cap_applies_to_each_hit_not_to_the_total() {
        // #73 Anti-oneshot Armor caps each hit on p1's hero at 5: 7 becomes 5, 3 stays 3.
        let state = board(json!({ "p1": { "backrow": ["core-073"] }, "p2": { "field": enemy() } }));
        assert_eq!(face_threat(&state, PlayerId::P2), 8);
    }

    #[test]
    fn b13_our_own_boards_threat_is_read_the_same_way_in_the_other_direction() {
        let state = board(json!({ "p1": { "field": enemy() }, "p2": {} }));
        assert_eq!(face_threat(&state, AI), 10);
        assert_eq!(face_threat(&state, PlayerId::P2), 0);
    }
}

mod r1224_r1225_credit_line {
    use super::*;

    use jackioh_engine::testkit::preview_sets;
    use jackioh_engine::SetName;

    const JLARNA: &str = "meditative-089";

    fn indebted(base: &GameState, owed: Vec<i32>) -> GameState {
        let mut out = clone(base);
        out.players[AI].owed_instalments = if owed.is_empty() { None } else { Some(owed) };
        out
    }

    #[test]
    fn r1224_owed_instalments_lower_the_score() {
        let _preview = preview_sets(&[SetName::Meditative]);
        let plain = board(base());
        let owed = indebted(&plain, vec![1, 1]);
        assert!(eval(&owed, AI) < eval(&plain, AI));
        let expected = AI_EVAL.owed_mana * 2.0;
        assert_close(eval(&plain, AI) - eval(&owed, AI), expected, 9);
    }

    #[test]
    fn r1225_an_unused_base_jlarna_is_worth_nothing_on_its_turn() {
        let _preview = preview_sets(&[SetName::Meditative]);
        let bare = board(json!({ "active": "p1", "p1": {}, "p2": {} }));
        let mut held = board(json!({
            "active": "p1",
            "p1": { "backrow": [JLARNA] },
            "p2": {},
        }));
        // Nothing borrowed this turn, on its controller's turn: the end of turn will tribute it.
        assert_eq!(held.players[AI].turn_log.mana_borrowed, None);
        assert_eq!(eval(&held, AI), eval(&bare, AI));
        // Borrowed: the same card counts again.
        held.players[AI].turn_log.mana_borrowed = Some(1);
        assert!(eval(&held, AI) > eval(&bare, AI));
    }
}
