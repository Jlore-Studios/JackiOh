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

use jackioh_ai::{AI_EVAL, GREEDY_EVAL, NextSwing, evaluate, face_threat, face_threat_with, unit_worth};
use jackioh_engine::testkit::{
    AI_DIFFICULTY, CardInstance, GameOverReason, GameResult, GameState, HERO_HEALTH, Keyword, Phase,
    PlayerId, Value, Winner, json,
};

use super::support::{AI, HUMAN, clone, register_cards, scenario};

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

    /// The first unit of p1's units lane 1.
    fn first_unit(state: &GameState) -> CardInstance {
        state.players[AI].units[0]
            .as_ref()
            .and_then(|pile| pile.first())
            .cloned()
            .expect("a unit stands on p1's field")
    }

    #[test]
    fn printed_taunt_unit_in_defense_position_keeps_its_inherent_taunt() {
        // Midrange Menace is 9/9 with a printed Taunt and no Armor.
        let attack = board(json!({ "p1": { "field": ["core-019"] } }));
        let defense = board(json!({ "p1": { "field": [{ "def": "core-019", "position": "DEF" }] } }));
        let (atk, def) = (first_unit(&attack), first_unit(&defense));
        // Switching costs it what it costs a vanilla body: its attack share, less the position's Armor
        // that `positionGrants` keeps. Its own Taunt is not part of the loss.
        let lost = (1.0 - AI_EVAL.defense_attack_share) * AI_EVAL.attack * 9.0;
        let kept = AI_EVAL.position_grants * AI_EVAL.armor_point;
        assert_close(
            unit_worth(&attack, &atk, &AI_EVAL) - unit_worth(&defense, &def, &AI_EVAL),
            lost - kept,
            10,
        );
        // Against a vanilla body of the same stats, in the same position, it is worth its Taunt more,
        // less the share of that Taunt the position grants the body anyway.
        let mut vanilla = def.clone();
        vanilla.vanilla = true;
        let diff = unit_worth(&defense, &def, &AI_EVAL) - unit_worth(&defense, &vanilla, &AI_EVAL);
        assert_close(diff, (1.0 - AI_EVAL.position_grants) * AI_EVAL.keyword.taunt, 10);
        assert!(diff > 0.0);
    }

    #[test]
    fn defense_position_grants_scale_with_position_grants_weight() {
        let vanilla_def = board(json!({ "p1": { "field": [{ "def": "core-008", "position": "DEF" }] } }));
        let taunt_def = board(json!({ "p1": { "field": [{ "def": "core-019", "position": "DEF" }] } }));
        let (v_unit, t_unit) = (first_unit(&vanilla_def), first_unit(&taunt_def));
        for share in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let mut w = AI_EVAL;
            w.position_grants = share;
            // Mr. Vanilla, 4/4: the position's Taunt and Armor 1 count for `share` of themselves.
            let body =
                w.health * 4.0 + w.armor_point + w.attack * 4.0 * w.defense_attack_share + w.keyword.taunt;
            assert_close(
                unit_worth(&vanilla_def, &v_unit, &w),
                body - (1.0 - share) * (w.keyword.taunt + w.armor_point),
                10,
            );
            // Midrange Menace, 9/9: its Taunt is its own, so only the position's Armor is discounted.
            let body =
                w.health * 9.0 + w.armor_point + w.attack * 9.0 * w.defense_attack_share + w.keyword.taunt;
            assert_close(
                unit_worth(&taunt_def, &t_unit, &w),
                body - (1.0 - share) * w.armor_point,
                10,
            );
        }
    }

    #[test]
    fn greedy_eval_still_discounts_a_taunt_units_own_taunt_in_defense_position() {
        // The frozen baseline: Midrange Menace in Defense Position loses a Taunt and a point of Armor.
        let defense = board(json!({ "p1": { "field": [{ "def": "core-019", "position": "DEF" }] } }));
        let unit = first_unit(&defense);
        let w = GREEDY_EVAL;
        let body = w.health * 9.0 + w.armor_point + w.attack * 9.0 * w.defense_attack_share + w.keyword.taunt;
        assert_close(
            unit_worth(&defense, &unit, &w),
            body - (1.0 - w.position_grants) * (w.keyword.taunt + w.armor_point),
            10,
        );
    }

    #[test]
    fn granted_taunt_unit_in_defense_position_keeps_its_granted_taunt() {
        let defense = board(json!({ "p1": { "field": [{ "def": "core-008", "position": "DEF" }] } }));
        let plain = first_unit(&defense);
        let mut granted = plain.clone();
        granted.granted_keywords.push(Keyword::Taunt);
        let diff = unit_worth(&defense, &granted, &AI_EVAL) - unit_worth(&defense, &plain, &AI_EVAL);
        assert_close(diff, (1.0 - AI_EVAL.position_grants) * AI_EVAL.keyword.taunt, 10);
        assert!(diff > 0.0);
    }

    /// #45 Deft Duelist (4/3, Charge, Deft) on p1's field in Attack and in Defense Position, each
    /// board with its unit.
    fn duelist() -> ((GameState, CardInstance), (GameState, CardInstance)) {
        let attack = board(json!({ "p1": { "field": ["core-045"] } }));
        let defense = board(json!({ "p1": { "field": [{ "def": "core-045", "position": "DEF" }] } }));
        let (atk, def) = (first_unit(&attack), first_unit(&defense));
        ((attack, atk), (defense, def))
    }

    /// p2's Deft Duelist in Defense Position, p1's board empty.
    fn enemy_duelist_in_defense() -> GameState {
        board(json!({ "p1": {}, "p2": { "field": [{ "def": "core-045", "position": "DEF" }] } }))
    }

    #[test]
    fn a_deft_unit_in_defense_position_keeps_its_whole_attack_and_differs_only_by_the_position_grants() {
        // R49: its switch back to Attack spends no attack, so Defense Position costs it no attack share.
        let ((attack, atk), (defense, def)) = duelist();
        assert_close(
            unit_worth(&defense, &def, &AI_EVAL) - unit_worth(&attack, &atk, &AI_EVAL),
            AI_EVAL.position_grants * (AI_EVAL.keyword.taunt + AI_EVAL.armor_point),
            10,
        );
    }

    #[test]
    fn greedy_eval_still_reads_a_deft_unit_in_defense_position_as_one_that_cannot_attack() {
        // The frozen baseline: Deft Duelist in Defense Position loses its attack share, as Mr. Vanilla
        // does, and threatens nothing.
        let ((attack, atk), (defense, def)) = duelist();
        let w = GREEDY_EVAL;
        assert_close(
            unit_worth(&attack, &atk, &w) - unit_worth(&defense, &def, &w),
            (1.0 - w.defense_attack_share) * w.attack * 4.0
                - w.position_grants * (w.keyword.taunt + w.armor_point),
            10,
        );
        let state = enemy_duelist_in_defense();
        assert_eq!(face_threat_with(&state, PlayerId::P2, &w), 0);
        assert_eq!(face_threat_with(&state, PlayerId::P2, &AI_EVAL), 4);
    }

    #[test]
    fn deft_defense_attacks_moves_the_score_by_the_threat_and_the_attack_share_it_counts() {
        // Off, the enemy Duelist threatens nothing and counts half its attack; on, it threatens 4 and
        // counts all of it. Nothing else on the board reads the flag.
        let state = enemy_duelist_in_defense();
        let mut off = AI_EVAL;
        off.deft_defense_attacks = false;
        assert_close(
            evaluate(&state, AI, NextSwing::Enemy, &off) - evaluate(&state, AI, NextSwing::Enemy, &AI_EVAL),
            AI_EVAL.threat_per_damage * 4.0 + (1.0 - AI_EVAL.defense_attack_share) * AI_EVAL.attack * 4.0,
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
    fn a_deft_unit_in_defense_position_is_an_attacker_and_one_without_deft_is_not() {
        // R49: Deft Duelist (4 attack) switches to Attack and still swings; Mr. Vanilla (4) cannot.
        let deft = board(json!({ "p1": {}, "p2": { "field": [{ "def": "core-045", "position": "DEF" }] } }));
        assert_eq!(face_threat(&deft, PlayerId::P2), 4);
        let vanilla =
            board(json!({ "p1": {}, "p2": { "field": [{ "def": "core-008", "position": "DEF" }] } }));
        assert_eq!(face_threat(&vanilla, PlayerId::P2), 0);
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
    fn b13_a_windfury_unit_threatens_each_of_its_two_attacks() {
        // R636: Windfurious Prime (5 attack, Windfury) swings twice, so against 9 health it is lethal.
        let state = board(json!({ "p1": { "health": 9 }, "p2": { "field": ["meditative-091-1"] } }));
        assert_eq!(face_threat(&state, PlayerId::P2), 10);
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

    use jackioh_engine::SetName;
    use jackioh_engine::testkit::preview_sets;

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

mod duos_life_r964000 {
    use super::*;
    use jackioh_ai::{EvalWeights, starting_life};
    use jackioh_engine::testkit::{DUOS_OWN_HEALTH, DUOS_POOL_HEALTH, DuosLife};

    /// What a probed weight adds, so a term that counts moves the score by far more than rounding.
    const PROBE: f64 = 1000.0;
    /// A turn past AI_EVAL.closing_from, where the closing term counts.
    const CLOSING_TURN: i32 = 20;

    /// `state` with `player` a Duos seat at `own` HP and a `pool` of `cap`, as the engine's `duos_seat`
    /// sets one up: a spent seat's hero health reads the pool (§13.4).
    fn in_duos(state: &GameState, player: PlayerId, own: i32, pool: i32, cap: i32, spent: bool) -> GameState {
        let mut out = clone(state);
        out.players[player].hero.health = if spent { pool } else { own };
        out.players[player].duos = Some(DuosLife {
            pool,
            pool_cap: cap,
            spent,
        });
        out
    }

    /// Whether the term `probe` raises counts in AI's score of `state`.
    fn counts(state: &GameState, probe: fn(&mut EvalWeights)) -> bool {
        let mut probed = AI_EVAL;
        probe(&mut probed);
        evaluate(state, AI, NextSwing::Enemy, &probed) != eval(state, AI)
    }

    fn lethal_threat(state: &GameState) -> bool {
        counts(state, |w| w.lethal_threat += PROBE)
    }

    fn lethal_pressure(state: &GameState) -> bool {
        counts(state, |w| w.lethal_pressure += PROBE)
    }

    fn closing(state: &GameState) -> bool {
        counts(state, |w| w.closing_weight += PROBE)
    }

    /// Three 4-mana 7/7s and Mr. Vanilla 4/4.
    fn board_of_25() -> Value {
        json!({ "field": ["core-025", "core-025", "core-025", "core-008"] })
    }

    #[test]
    fn r964000_an_unspent_seat_at_20_own_hp_and_a_20_pool_is_not_under_lethal_threat_from_25() {
        let state = board(json!({ "p2": board_of_25() }));
        assert_eq!(face_threat(&state, HUMAN), 25);
        assert!(
            !lethal_threat(&in_duos(&state, AI, 20, 20, 20, false)),
            "20 + 20 is 40"
        );
        assert!(!lethal_threat(&state), "a 1v1 hero at {HERO_HEALTH}");
        assert!(
            lethal_threat(&in_duos(&state, AI, 5, 20, 20, false)),
            "5 + 20 is 25"
        );
        assert!(
            lethal_threat(&in_duos(&state, AI, 20, 20, 20, true)),
            "spent: the pool alone"
        );
        assert!(lethal_threat(&with_health(&state, AI, 20)), "a 1v1 hero at 20");
    }

    #[test]
    fn r964000_lethal_pressure_counts_the_enemy_teams_pool() {
        let state = board(json!({ "p1": board_of_25() }));
        assert_eq!(face_threat(&state, AI), 25);
        assert!(!lethal_pressure(&in_duos(&state, HUMAN, 20, 20, 20, false)));
        assert!(lethal_pressure(&in_duos(&state, HUMAN, 20, 20, 20, true)));
        assert!(lethal_pressure(&with_health(&state, HUMAN, 20)));
    }

    #[test]
    fn r964000_closing_reads_no_damage_on_a_duos_opponent_at_its_starting_life() {
        let state = board(json!({ "turn": CLOSING_TURN }));
        assert!(!closing(&state), "a 1v1 hero at {HERO_HEALTH}");
        assert!(!closing(&in_duos(&state, HUMAN, DUOS_OWN_HEALTH, 20, 20, false)));
        assert!(closing(&in_duos(&state, HUMAN, DUOS_OWN_HEALTH, 15, 20, false)));
        assert!(closing(&with_health(&state, HUMAN, 20)));
    }

    #[test]
    fn r964000_more_pool_is_strictly_better_for_the_seat_and_strictly_worse_on_the_enemy() {
        let state = board(base());
        for pool in 1..DUOS_POOL_HEALTH {
            let (less, more) = (pool, pool + 1);
            assert!(
                eval(&in_duos(&state, AI, 20, more, 20, false), AI)
                    > eval(&in_duos(&state, AI, 20, less, 20, false), AI),
                "own pool {more} over {less}"
            );
            assert!(
                eval(&in_duos(&state, HUMAN, 20, more, 20, false), AI)
                    < eval(&in_duos(&state, HUMAN, 20, less, 20, false), AI),
                "enemy pool {more} under {less}"
            );
        }
    }

    #[test]
    fn r964000_starting_life_is_hero_health_in_1v1_and_own_hp_plus_the_pool_cap_in_duos() {
        let state = board(base());
        assert_eq!(starting_life(&state, AI), HERO_HEALTH);
        let duos = in_duos(&state, AI, 20, 7, 30, false);
        // The pool's cap, not what is left of it.
        assert_eq!(starting_life(&duos, AI), DUOS_OWN_HEALTH + 30);
        assert_eq!(starting_life(&duos, HUMAN), HERO_HEALTH);
    }
}

mod banked_state_1086 {
    use super::*;

    use jackioh_engine::testkit::{JADE_ASCEND_AT, JADE_BEAUTY_AT, Scenario, active_units_of};

    const JADE: &str = "meditative-039-2"; // (0) Spell token: add 1 to your Jade Counter, gain 1 mana
    const BEAUTY: &str = "meditative-039-5"; // the 20/20 the counter's fifth step summons (R962)
    const KPOP: &str = "core-050"; // (2) Unit 1/1, Cry: Allure an enemy permanent (R437's mark)
    const FULLSEND: &str = "core-078"; // (4) Spell: end of turn, exile your hand (an unmarked delayed effect)
    const RIFT: &str = "meditative-019-1"; // (0) Spell token: take an extra turn after this one (R846)
    const VANILLA: &str = "core-008"; // (1) Unit 4/4, no keywords
    const PUNISH: &str = "classic-020"; // Field Spell: Activate, a Unit is destroyed at your next turn's start
    const DOOM: &str = "destroy a Unit at the start of your next turn";
    const ROCK: &str = "core-066"; // Unit 10/10 Indestructible

    fn at_jade(state: &GameState, jade: i32) -> GameState {
        let mut out = clone(state);
        out.players[AI].jade = if jade == 0 { None } else { Some(jade) };
        out
    }

    fn jade_board() -> Scenario {
        register_cards();
        scenario(json!({
            "seed": "evaluate-banked",
            "p1": { "hand": [JADE, VANILLA] },
            "p2": { "hand": [VANILLA], "field": [VANILLA] },
        }))
    }

    fn jade_play_gain(from: i32) -> (f64, GameState) {
        let mut s = jade_board();
        s.state_mut().players[AI].jade = if from == 0 { None } else { Some(from) };
        let before = s.state().clone();
        s.play(JADE, json!({}));
        let after = s.state().clone();
        (eval(&after, AI) - eval(&before, AI), after)
    }

    #[test]
    fn r962_each_jade_step_toward_the_summon_raises_the_score_and_four_are_worth_more_than_a_card() {
        let plain = jade_board().state().clone();
        let mut last = eval(&plain, AI);
        for jade in 1..JADE_BEAUTY_AT {
            let now = eval(&at_jade(&plain, jade), AI);
            assert!(now > last, "jade {jade}: {now} is not above {last}");
            last = now;
        }
        let banked = eval(&at_jade(&plain, JADE_BEAUTY_AT - 1), AI) - eval(&plain, AI);
        assert_close(banked, f64::from(JADE_BEAUTY_AT - 1) * AI_EVAL.jade_step, 9);
        assert!(
            banked > AI_EVAL.hand_card,
            "four steps {banked} are not worth more than a card"
        );
        // The enemy's counter counts against the seat.
        let mut theirs = clone(&plain);
        theirs.players[HUMAN].jade = Some(JADE_BEAUTY_AT - 1);
        assert!(eval(&theirs, AI) < eval(&plain, AI));
    }

    #[test]
    fn r962_the_jade_that_crosses_the_threshold_is_the_sharpest_step_the_counter_takes() {
        let (crossing, after) = jade_play_gain(JADE_BEAUTY_AT - 1);
        assert!(
            active_units_of(&after, AI)
                .iter()
                .any(|unit| unit.def_id == BEAUTY),
            "setup: the fifth step should summon a Jade Beauty"
        );
        for from in 0..JADE_BEAUTY_AT - 1 {
            let (step, _) = jade_play_gain(from);
            assert!(
                crossing > step,
                "{from} -> {}: {step} is not below the crossing's {crossing}",
                from + 1
            );
        }
    }

    #[test]
    fn r962_past_the_summon_the_steps_count_toward_the_ascension_and_none_past_it() {
        let plain = jade_board().state().clone();
        let five = eval(&at_jade(&plain, JADE_BEAUTY_AT), AI);
        assert_close(eval(&plain, AI), five, 9);
        assert_close(
            eval(&at_jade(&plain, JADE_ASCEND_AT - 1), AI) - five,
            f64::from(JADE_ASCEND_AT - 1 - JADE_BEAUTY_AT) * AI_EVAL.jade_step,
            9,
        );
        assert_close(
            eval(&at_jade(&plain, JADE_ASCEND_AT + 2), AI),
            eval(&plain, AI),
            9,
        );
    }

    #[test]
    fn r437_a_k_pop_fanatics_pending_allure_banks_a_share_of_the_marked_unit() {
        register_cards();
        let mut s = scenario(json!({
            "seed": "evaluate-banked",
            "p1": { "hand": [KPOP, VANILLA] },
            "p2": { "hand": [VANILLA], "field": [{ "def": VANILLA, "lane": 2 }] },
        }));
        let prey = s.unit(HUMAN, 2).expect("setup: p2 should hold the 4/4 in lane 2");
        s.play(
            KPOP,
            json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }),
        );
        let armed = s.state().clone();
        assert_eq!(armed.delayed.len(), 1, "setup: the Allure should wait");
        assert!(
            armed
                .marks
                .as_ref()
                .is_some_and(|marks| marks.iter().any(|mark| mark.instance_id == prey.id))
        );
        let mut bare = clone(&armed);
        bare.delayed.clear();
        bare.marks = None;

        assert!(eval(&armed, AI) > eval(&bare, AI));
        let live = s.unit(HUMAN, 2).expect("the 4/4 is still p2's until the steal");
        assert_close(
            eval(&armed, AI) - eval(&bare, AI),
            AI_EVAL.marked_share * unit_worth(&armed, &live, &AI_EVAL),
            9,
        );
        // The marked unit's controller reads the same mark as a loss.
        assert!(eval(&armed, HUMAN) < eval(&bare, HUMAN));
    }

    /// The Power to Punish's base delayed destroy aimed at p2's `def` in lane 2: the armed state, the
    /// same state with no delayed effect or mark, and the marked unit.
    fn punished(def: &str) -> (GameState, GameState, CardInstance) {
        register_cards();
        let mut s = scenario(json!({
            "seed": "evaluate-banked",
            "p1": { "hand": [VANILLA], "backrow": [PUNISH] },
            "p2": { "hand": [VANILLA], "field": [{ "def": def, "lane": 2 }] },
        }));
        let prey = s
            .unit(HUMAN, 2)
            .expect("setup: p2 should hold the unit in lane 2");
        s.activate(
            PUNISH,
            json!({ "modes": [DOOM], "targets": [{ "pick": "instance", "instanceId": prey.id }] }),
        );
        let armed = s.state().clone();
        assert!(
            armed
                .marks
                .as_ref()
                .is_some_and(|marks| marks.iter().any(|mark| mark.instance_id == prey.id)),
            "setup: the doomed unit should wear the red mark"
        );
        let mut bare = clone(&armed);
        bare.delayed.clear();
        bare.marks = None;
        let live = s
            .unit(HUMAN, 2)
            .expect("the unit is still on the field until the destroy");
        (armed, bare, live)
    }

    #[test]
    fn r437_the_power_to_punishs_pending_destroy_banks_a_share_of_the_marked_unit() {
        let (armed, bare, live) = punished(VANILLA);
        assert_close(
            eval(&armed, AI) - eval(&bare, AI),
            AI_EVAL.marked_share * unit_worth(&armed, &live, &AI_EVAL),
            9,
        );
        assert!(eval(&armed, HUMAN) < eval(&bare, HUMAN));
    }

    #[test]
    fn r46_a_pending_destroy_marked_on_an_indestructible_unit_banks_nothing() {
        let (armed, bare, _) = punished(ROCK);
        assert_eq!(eval(&armed, AI), eval(&bare, AI));
        assert_eq!(eval(&armed, HUMAN), eval(&bare, HUMAN));
    }

    #[test]
    fn r437_a_delayed_effect_that_marks_nothing_adds_nothing() {
        register_cards();
        let mut s = scenario(json!({
            "seed": "evaluate-banked",
            "p1": { "hand": [FULLSEND, VANILLA] },
            "p2": { "hand": [VANILLA] },
        }));
        s.play(FULLSEND, json!({}));
        let armed = s.state().clone();
        assert!(
            !armed.delayed.is_empty(),
            "setup: /fullsend's exile should wait for the end of turn"
        );
        let mut bare = clone(&armed);
        bare.delayed.clear();
        assert_eq!(eval(&armed, AI), eval(&bare, AI));
    }

    #[test]
    fn r846_an_extra_turn_in_the_bank_is_worth_ai_eval_extra_turn() {
        register_cards();
        let mut s = scenario(json!({
            "seed": "evaluate-banked",
            "p1": { "hand": [RIFT, VANILLA] },
            "p2": { "hand": [VANILLA] },
        }));
        s.play(RIFT, json!({}));
        let owed = s.state().clone();
        assert_eq!(
            owed.players[AI].extra_turns,
            Some(1),
            "setup: the Rift should owe a turn"
        );
        let mut none = clone(&owed);
        none.players[AI].extra_turns = None;
        assert_close(eval(&owed, AI) - eval(&none, AI), AI_EVAL.extra_turn, 9);
        assert!(eval(&owed, HUMAN) < eval(&none, HUMAN));
    }

    #[test]
    fn r637_a_temporary_hand_card_is_worth_less_than_the_same_card_kept() {
        let kept = board(json!({ "p1": { "hand": [VANILLA] }, "p2": { "hand": [VANILLA] } }));
        let empty = board(json!({ "p1": {}, "p2": { "hand": [VANILLA] } }));
        let mut temporary = clone(&kept);
        temporary.players[AI].hand[0]
            .granted_keywords
            .push(Keyword::Temporary);
        assert!(eval(&temporary, AI) < eval(&kept, AI));
        assert_close(
            eval(&kept, AI) - eval(&temporary, AI),
            AI_EVAL.temporary_discount * (eval(&kept, AI) - eval(&empty, AI)),
            9,
        );
        // R185: the enemy's hand is unseen, so its Temporary cards move nothing.
        let mut theirs = clone(&kept);
        theirs.players[HUMAN].hand[0]
            .granted_keywords
            .push(Keyword::Temporary);
        assert_eq!(eval(&theirs, AI), eval(&kept, AI));
    }

    #[test]
    fn greedy_eval_gives_every_banked_term_nothing() {
        let greedy = |state: &GameState| evaluate(state, AI, NextSwing::Enemy, &GREEDY_EVAL);
        let plain = jade_board().state().clone();
        assert_eq!(greedy(&at_jade(&plain, JADE_BEAUTY_AT - 1)), greedy(&plain));
        let mut owed = clone(&plain);
        owed.players[AI].extra_turns = Some(1);
        assert_eq!(greedy(&owed), greedy(&plain));
        let mut temporary = clone(&plain);
        temporary.players[AI].hand[1]
            .granted_keywords
            .push(Keyword::Temporary);
        assert_eq!(greedy(&temporary), greedy(&plain));
    }
}

mod hand_faces_1143 {
    use super::*;

    use jackioh_engine::testkit::AttackHealth;

    const BIG: &str = "core-025"; // (4) Unit 7/7 Armor 7, Radiant 14/14 Armor 7, Reborn
    const LAR: &str = "classicplus-076"; // (1) Unit 1/1, Radiant 2/2: Brother Lar
    const VANILLA: &str = "core-008"; // (1) Unit 4/4, no keywords
    const STOCKPILE: &str = "core-005"; // (1) Spell on both faces

    /// The same three cards in both hands.
    fn hands() -> GameState {
        board(json!({ "p1": { "hand": [BIG, LAR, STOCKPILE] }, "p2": { "hand": [BIG, LAR, STOCKPILE] } }))
    }

    /// `state` with `player`'s hand card at `index` made Radiant, which swaps its face in hand (R74).
    fn radiant(state: &GameState, player: PlayerId, index: usize) -> GameState {
        let mut out = clone(state);
        out.players[player].hand[index].radiant = true;
        out
    }

    /// `state` with `player`'s hand card at `index` carrying `buffs`, which ride onto the field with it.
    fn buffed(state: &GameState, player: PlayerId, index: usize, buffs: AttackHealth) -> GameState {
        let mut out = clone(state);
        out.players[player].hand[index].buffs = buffs;
        out
    }

    #[test]
    fn a_radiant_hand_unit_outscores_itself_un_radiant_by_more_the_more_its_radiant_face_adds() {
        let plain = hands();
        let gain = |index: usize| eval(&radiant(&plain, AI, index), AI) - eval(&plain, AI);
        let (big, lar, spell) = (gain(0), gain(1), gain(2));
        assert!(
            lar > AI_EVAL.radiant_in_hand,
            "Brother Lar's 2/2 {lar} is not above the flat bonus"
        );
        assert!(big > lar, "the 14/14's {big} is not above Brother Lar's {lar}");
        // 7 attack, 7 health and Reborn more than the 7/7 Armor 7 it prints, at the share.
        assert_close(
            big,
            AI_EVAL.radiant_in_hand
                + AI_EVAL.radiant_face_share
                    * (7.0 * AI_EVAL.attack + 7.0 * AI_EVAL.health + AI_EVAL.keyword.reborn),
            9,
        );
        // A Spell has no unit face to read: it keeps the flat bonus alone.
        assert_close(spell, AI_EVAL.radiant_in_hand, 9);
    }

    #[test]
    fn a_buffed_hand_unit_outscores_itself_unbuffed_by_its_buffs_worth_at_the_share() {
        let plain = board(json!({ "p1": { "hand": [VANILLA] }, "p2": { "hand": [VANILLA] } }));
        let buffs = AttackHealth { attack: 2, health: 1 };
        let gain = eval(&buffed(&plain, AI, 0, buffs), AI) - eval(&plain, AI);
        assert!(gain > 0.0, "a +2/+1 hand unit gains {gain}");
        assert_close(
            gain,
            AI_EVAL.radiant_face_share * (2.0 * AI_EVAL.attack + AI_EVAL.health),
            9,
        );
    }

    #[test]
    fn the_enemys_hidden_hand_scores_the_same_whatever_its_radiant_flags_or_buffs() {
        let plain = hands();
        let buffs = AttackHealth { attack: 2, health: 1 };
        assert_eq!(eval(&radiant(&plain, HUMAN, 0), AI), eval(&plain, AI));
        assert_eq!(eval(&radiant(&plain, HUMAN, 1), AI), eval(&plain, AI));
        assert_eq!(eval(&buffed(&plain, HUMAN, 0, buffs), AI), eval(&plain, AI));
    }

    #[test]
    fn greedy_eval_reads_no_hand_face_a_radiant_hand_unit_keeps_only_the_flat_bonus() {
        let greedy = |state: &GameState| evaluate(state, AI, NextSwing::Enemy, &GREEDY_EVAL);
        let plain = hands();
        let buffs = AttackHealth { attack: 2, health: 1 };
        assert_close(
            greedy(&radiant(&plain, AI, 0)) - greedy(&plain),
            GREEDY_EVAL.radiant_in_hand,
            9,
        );
        assert_eq!(greedy(&buffed(&plain, AI, 0, buffs)), greedy(&plain));
    }
}

mod r844_coming_refreshes_1224 {
    use super::*;
    use jackioh_ai::coming_mana;

    /// `state` with `player`'s next `turns` refreshes lost (R844; 0 covers only the turn it started).
    fn losing(state: &GameState, player: PlayerId, turns: i32) -> GameState {
        let mut out = clone(state);
        let side = &mut out.players[player];
        side.lost_refresh_through = Some(side.turns_started + turns);
        out
    }

    /// `state` with `player`'s next-turn rider at `rider` (§2.3).
    fn riding(state: &GameState, player: PlayerId, rider: i32) -> GameState {
        let mut out = clone(state);
        out.players[player].mana.next_turn_mod = rider;
        out
    }

    /// The crystals `player`'s refresh gives at `state`, its maximum.
    fn crystals(state: &GameState, player: PlayerId) -> f64 {
        f64::from(state.players[player].mana.max)
    }

    #[test]
    fn r844_each_refresh_still_to_lose_costs_its_side_lost_refresh_per_crystal_it_would_have_given() {
        let plain = board(base());
        for turns in [1, 2] {
            let lost = losing(&plain, AI, turns);
            let price = AI_EVAL.lost_refresh * f64::from(turns) * crystals(&plain, AI);
            assert!(
                eval(&lost, AI) < eval(&plain, AI),
                "{turns} lost refreshes cost nothing"
            );
            assert_close(eval(&plain, AI) - eval(&lost, AI), price, 9);
            assert_close(eval(&lost, HUMAN) - eval(&plain, HUMAN), price, 9);

            let theirs = losing(&plain, HUMAN, turns);
            let theirs_price = AI_EVAL.lost_refresh * f64::from(turns) * crystals(&plain, HUMAN);
            assert_close(eval(&theirs, AI) - eval(&plain, AI), theirs_price, 9);
            assert_close(eval(&plain, HUMAN) - eval(&theirs, HUMAN), theirs_price, 9);
        }
    }

    #[test]
    fn r844_a_refresh_already_lost_this_turn_is_not_priced_again() {
        let plain = board(base());
        assert_eq!(eval(&losing(&plain, AI, 0), AI), eval(&plain, AI));
    }

    #[test]
    fn r844_a_lost_refresh_takes_the_next_turn_rider_with_it() {
        let plain = board(base());
        let both = riding(&losing(&plain, AI, 1), AI, 2);
        assert_close(
            eval(&plain, AI) - eval(&both, AI),
            AI_EVAL.lost_refresh * (crystals(&plain, AI) + 2.0),
            9,
        );
    }

    #[test]
    fn r844_a_lost_refresh_is_priced_from_the_states_crystals_alike_at_every_tier() {
        let plain = board(base());
        let lost = losing(&plain, AI, 1);
        let mut hard = clone(&lost);
        hard.players[AI].handicap = Some(AI_DIFFICULTY.hard);
        assert_eq!(eval(&hard, AI), eval(&lost, AI));
    }

    #[test]
    fn a_next_turn_rider_moves_its_sides_score_by_next_turn_mana_per_crystal() {
        let plain = board(base());
        for rider in [2, -1] {
            let worth = AI_EVAL.next_turn_mana * f64::from(rider);
            let mine = riding(&plain, AI, rider);
            let theirs = riding(&plain, HUMAN, rider);
            assert_close(eval(&mine, AI) - eval(&plain, AI), worth, 9);
            assert_close(eval(&mine, HUMAN) - eval(&plain, HUMAN), -worth, 9);
            assert_close(eval(&theirs, AI) - eval(&plain, AI), -worth, 9);
            assert_close(eval(&theirs, HUMAN) - eval(&plain, HUMAN), worth, 9);
        }
        assert!(eval(&riding(&plain, AI, 2), AI) > eval(&plain, AI));
        assert!(eval(&riding(&plain, AI, -1), AI) < eval(&plain, AI));
    }

    #[test]
    fn a_next_turn_rider_below_the_crystals_counts_only_down_to_a_refresh_of_0() {
        let plain = board(base());
        assert_close(
            eval(&plain, AI) - eval(&riding(&plain, AI, -10), AI),
            AI_EVAL.next_turn_mana * crystals(&plain, AI),
            9,
        );
    }

    #[test]
    fn r844_coming_mana_is_the_seats_coming_refreshes_less_the_enemys() {
        let plain = board(base());
        assert_eq!(coming_mana(&plain, AI, &AI_EVAL), 0.0);
        let lost = losing(&plain, AI, 1);
        assert_close(
            coming_mana(&lost, AI, &AI_EVAL),
            -AI_EVAL.lost_refresh * crystals(&plain, AI),
            9,
        );
        assert_close(
            coming_mana(&lost, HUMAN, &AI_EVAL),
            AI_EVAL.lost_refresh * crystals(&plain, AI),
            9,
        );
        let hindered = riding(&plain, HUMAN, -1);
        assert_close(coming_mana(&hindered, AI, &AI_EVAL), AI_EVAL.next_turn_mana, 9);
        assert_eq!(coming_mana(&lost, AI, &GREEDY_EVAL), 0.0);
        assert_eq!(coming_mana(&hindered, AI, &GREEDY_EVAL), 0.0);
    }

    #[test]
    fn greedy_eval_prices_no_coming_refresh_and_no_rider() {
        let greedy = |state: &GameState| evaluate(state, AI, NextSwing::Enemy, &GREEDY_EVAL);
        let plain = board(base());
        for changed in [
            losing(&plain, AI, 1),
            losing(&plain, HUMAN, 2),
            riding(&plain, AI, 2),
            riding(&plain, HUMAN, -1),
            riding(&losing(&plain, AI, 1), AI, 2),
        ] {
            assert_eq!(greedy(&changed), greedy(&plain));
        }
    }
}

mod dormant_cards_1506 {
    use super::*;
    use jackioh_ai::{EvalWeights, redact};

    /// The dormant terms on whatever AI_EVAL holds (#1506's screen decides those), so these tests
    /// pin the term, not its tuning.
    const DORMANT: EvalWeights = EvalWeights {
        dormant_share: 0.5,
        dormant_card: 1.0,
        ..AI_EVAL
    };

    const ROCK: &str = "core-066"; // Unit 10/10 Indestructible
    const VANILLA: &str = "core-008"; // (1) Unit 4/4, no keywords
    const TIMMY: &str = "core-011"; // Unit 3/3 Rush, First Strike: every unit pile's top here
    const SHEEPISH: &str = "core-041"; // Trap
    const PAWN: &str = "core-096"; // Trap
    const WELL: &str = "core-006"; // Field Spell: every backrow pile's top here

    fn scored(state: &GameState, w: &EvalWeights) -> f64 {
        evaluate(state, AI, NextSwing::Enemy, w)
    }

    /// `state` with `player`'s lane-1 unit pile cut to its top `keep` cards.
    fn cut(state: &GameState, player: PlayerId, keep: usize) -> GameState {
        let mut out = clone(state);
        if let Some(pile) = out.players[player].units[0].as_mut() {
            pile.truncate(keep);
        }
        out
    }

    /// `state` with nothing dormant: every unit pile cut to its top, no backrow pile.
    fn bare(state: &GameState) -> GameState {
        let mut out = clone(state);
        for player in [AI, HUMAN] {
            for pile in out.players[player].units.iter_mut().flatten() {
                pile.truncate(1);
            }
            out.players[player].backrow_piles = None;
        }
        out
    }

    #[test]
    fn each_card_dormant_under_an_own_unit_pile_adds_its_share_of_its_unit_worth() {
        let three = board(json!({
            "p1": { "field": [ROCK, { "def": VANILLA, "stack": true }, { "def": TIMMY, "stack": true }] },
            "p2": { "field": [VANILLA] },
        }));
        let two = cut(&three, AI, 2);
        let one = cut(&three, AI, 1);
        let pile = three.players[AI].units[0].clone().unwrap_or_default();
        assert_eq!(pile.len(), 3, "setup: a three-deep pile in lane 1");
        // A second card dormant under the pile scores strictly differently, and a third again.
        assert!(scored(&three, &DORMANT) > scored(&two, &DORMANT));
        assert!(scored(&two, &DORMANT) > scored(&one, &DORMANT));
        assert_close(
            scored(&two, &DORMANT) - scored(&one, &DORMANT),
            DORMANT.dormant_share * unit_worth(&two, &pile[1], &DORMANT),
            9,
        );
        assert_close(
            scored(&three, &DORMANT) - scored(&two, &DORMANT),
            DORMANT.dormant_share * unit_worth(&three, &pile[2], &DORMANT),
            9,
        );
    }

    #[test]
    fn the_enemys_dormant_units_count_dormant_card_each_whatever_they_are() {
        let vanilla = board(json!({
            "p1": { "field": [VANILLA] },
            "p2": { "field": [VANILLA, { "def": TIMMY, "stack": true }] },
        }));
        let rock = board(json!({
            "p1": { "field": [VANILLA] },
            "p2": { "field": [ROCK, { "def": TIMMY, "stack": true }] },
        }));
        assert_close(
            scored(&bare(&vanilla), &DORMANT) - scored(&vanilla, &DORMANT),
            DORMANT.dormant_card,
            9,
        );
        // Count only: a 10/10 beneath scores as a 4/4 does.
        assert_eq!(scored(&rock, &DORMANT), scored(&vanilla, &DORMANT));
    }

    #[test]
    fn a_card_dormant_under_a_backrow_pile_counts_dormant_card_on_either_side_whatever_it_is() {
        let own = board(json!({
            "p1": { "backrow": [SHEEPISH, { "def": WELL, "stack": true }] },
            "p2": { "field": [VANILLA] },
        }));
        assert_close(
            scored(&own, &DORMANT) - scored(&bare(&own), &DORMANT),
            DORMANT.dormant_card,
            9,
        );
        let sheepish = board(json!({
            "p1": { "field": [VANILLA] },
            "p2": { "backrow": [SHEEPISH, { "def": WELL, "stack": true }] },
        }));
        let pawn = board(json!({
            "p1": { "field": [VANILLA] },
            "p2": { "backrow": [PAWN, { "def": WELL, "stack": true }] },
        }));
        assert_close(
            scored(&bare(&sheepish), &DORMANT) - scored(&sheepish, &DORMANT),
            DORMANT.dormant_card,
            9,
        );
        assert_eq!(scored(&pawn, &DORMANT), scored(&sheepish, &DORMANT));
        // R447: the seat's view holds a placeholder beneath, and scores alike.
        assert_eq!(
            scored(&redact(&sheepish, AI), &DORMANT),
            scored(&sheepish, &DORMANT)
        );
    }

    #[test]
    fn greedy_eval_counts_no_dormant_card() {
        let piled = board(json!({
            "p1": {
                "field": [VANILLA, { "def": TIMMY, "stack": true }],
                "backrow": [SHEEPISH, { "def": WELL, "stack": true }],
            },
            "p2": {
                "field": [VANILLA, { "def": TIMMY, "stack": true }],
                "backrow": [PAWN, { "def": WELL, "stack": true }],
            },
        }));
        assert_eq!(scored(&piled, &GREEDY_EVAL), scored(&bare(&piled), &GREEDY_EVAL));
    }
}

mod shown_cost_fatigue_windfury_1602 {
    use super::*;
    use jackioh_ai::{DeterminizeOptions, EvalWeights, KeywordWeights, determinize, redact};
    use jackioh_engine::testkit::{KeywordKind, create_rng};

    /// #1602's three terms at their starts, whatever AI_EVAL holds (each term's screen decides
    /// that), so these tests pin the terms, not their tuning.
    const PRICED: EvalWeights = EvalWeights {
        enemy_face_down_per_cost: 0.8,
        fatigue_draw: 1.0,
        keyword: KeywordWeights {
            windfury: 1.0,
            ..AI_EVAL.keyword
        },
        ..AI_EVAL
    };

    /// The same weights with the three terms off.
    const UNPRICED: EvalWeights = EvalWeights {
        enemy_face_down_per_cost: 0.0,
        fatigue_draw: 0.0,
        keyword: KeywordWeights {
            windfury: 0.0,
            ..AI_EVAL.keyword
        },
        ..AI_EVAL
    };

    const VANILLA: &str = "core-008"; // (1) Unit 4/4, no keywords
    const SHEEPISH: &str = "core-041"; // (1) Trap
    const PAWN: &str = "core-096"; // (1) Trap
    const PRIME: &str = "meditative-091-1"; // Windfurious Prime: 5/10 Rush, Windfury

    fn scored(state: &GameState, w: &EvalWeights) -> f64 {
        evaluate(state, AI, NextSwing::Enemy, w)
    }

    /// A 4/4 on each side.
    fn even() -> GameState {
        board(json!({ "p1": { "field": [VANILLA] }, "p2": { "field": [VANILLA] } }))
    }

    /// `even()` plus p2's face-down `trap`, its shown cost moved by `cost_mod` (R65, R351).
    fn backed(trap: &str, cost_mod: i32) -> GameState {
        board(json!({
            "p1": { "field": [VANILLA] },
            "p2": { "field": [VANILLA], "backrow": [{ "def": trap, "faceUp": false, "costMod": cost_mod }] },
        }))
    }

    /// `state` with p2's backrow card revealed (R686): both seats read it while it stays armed.
    fn revealed(state: &GameState) -> GameState {
        let mut out = clone(state);
        if let Some(card) = out.players[HUMAN].backrow.iter_mut().flatten().next() {
            card.revealed = Some(true);
        }
        out
    }

    /// `state` with `player`'s fatigue count at `count` (§2.4).
    fn tired(state: &GameState, player: PlayerId, count: i32) -> GameState {
        let mut out = clone(state);
        out.players[player].fatigue_count = count;
        out
    }

    /// `state` with `player`'s lane-1 unit granted Windfury.
    fn windfurious(state: &GameState, player: PlayerId) -> GameState {
        let mut out = clone(state);
        if let Some(unit) = out.players[player].units[0]
            .as_mut()
            .and_then(|pile| pile.first_mut())
        {
            unit.granted_keywords.push(Keyword::Windfury);
        }
        out
    }

    /// `player`'s lane-1 unit.
    fn lane_one(state: &GameState, player: PlayerId) -> CardInstance {
        state.players[player].units[0]
            .as_ref()
            .and_then(|pile| pile.first())
            .cloned()
            .expect("setup: a unit in lane 1")
    }

    #[test]
    fn an_enemy_face_down_card_scores_lower_by_each_crystal_of_cost_it_shows() {
        let cheap = backed(SHEEPISH, 0);
        let dear = backed(SHEEPISH, 4);
        // Without the term the two backs score alike: only it tells them apart.
        assert_eq!(scored(&cheap, &UNPRICED), scored(&dear, &UNPRICED));
        assert!(scored(&dear, &PRICED) < scored(&cheap, &PRICED));
        assert_close(
            scored(&cheap, &PRICED) - scored(&dear, &PRICED),
            4.0 * PRICED.enemy_face_down_per_cost,
            9,
        );
        // Sheepish shows (1): the term takes that many crystals off the enemy's side.
        assert_close(
            scored(&cheap, &UNPRICED) - scored(&cheap, &PRICED),
            PRICED.enemy_face_down_per_cost,
            9,
        );
    }

    #[test]
    fn an_unrevealed_card_is_priced_by_the_cost_it_shows_alone_and_a_revealed_one_by_its_face() {
        let sheepish = backed(SHEEPISH, 4);
        let pawn = backed(PAWN, 4);
        // Unrevealed: two cards that show the same cost score alike, and so do the seat's own view
        // (a placeholder that keeps only the shown cost, R185, R351) and a world sampled from it (R762).
        assert_eq!(scored(&sheepish, &PRICED), scored(&pawn, &PRICED));
        let view = redact(&sheepish, AI);
        assert_eq!(scored(&view, &PRICED), scored(&sheepish, &PRICED));
        let world = determinize(
            &view,
            AI,
            &mut create_rng("shown-cost-1602", 0),
            DeterminizeOptions::default(),
        );
        assert_eq!(scored(&world, &PRICED), scored(&sheepish, &PRICED));
        assert_ne!(scored(&sheepish, &PRICED), scored(&sheepish, &UNPRICED));
        // Revealed (R686): the seat reads the card, so its face prices it and the face-down term
        // leaves it alone.
        let shown = revealed(&sheepish);
        assert_eq!(scored(&shown, &PRICED), scored(&shown, &UNPRICED));
        assert_eq!(scored(&redact(&shown, AI), &PRICED), scored(&shown, &PRICED));
    }

    #[test]
    fn each_fatigue_draw_a_side_has_taken_counts_fatigue_draw_against_it() {
        let state = even();
        let base = scored(&state, &PRICED);
        assert!(scored(&tired(&state, AI, 3), &PRICED) < base);
        assert!(scored(&tired(&state, HUMAN, 3), &PRICED) > base);
        assert_close(
            base - scored(&tired(&state, AI, 3), &PRICED),
            3.0 * PRICED.fatigue_draw,
            9,
        );
        assert_close(
            scored(&tired(&state, HUMAN, 3), &PRICED) - base,
            3.0 * PRICED.fatigue_draw,
            9,
        );
        // Symmetric: the same count on both sides cancels.
        assert_close(scored(&tired(&tired(&state, AI, 3), HUMAN, 3), &PRICED), base, 9);
        // Orthogonal to the library terms: a stocked library moves the step not at all.
        let stocked = board(json!({
            "p1": { "field": [VANILLA], "library": ["core-001", "core-002", "core-003"] },
            "p2": { "field": [VANILLA] },
        }));
        assert_close(
            scored(&stocked, &PRICED) - scored(&tired(&stocked, AI, 3), &PRICED),
            3.0 * PRICED.fatigue_draw,
            9,
        );
        assert_eq!(
            scored(&tired(&state, AI, 3), &UNPRICED),
            scored(&state, &UNPRICED)
        );
    }

    #[test]
    fn a_windfury_unit_is_worth_the_windfury_weight_more_and_the_table_names_it() {
        assert_eq!(
            PRICED.keyword.of(KeywordKind::Windfury),
            Some(PRICED.keyword.windfury)
        );
        let state = even();
        let own = windfurious(&state, AI);
        let enemy = windfurious(&state, HUMAN);
        assert_close(
            unit_worth(&own, &lane_one(&own, AI), &PRICED)
                - unit_worth(&state, &lane_one(&state, AI), &PRICED),
            PRICED.keyword.windfury,
            9,
        );
        // Our Windfury raises the score and theirs lowers it ...
        assert!(scored(&own, &PRICED) > scored(&state, &PRICED));
        assert!(scored(&enemy, &PRICED) < scored(&state, &PRICED));
        // ... and the weight is all the term adds to the second swing `face_threat` already counts.
        assert_close(
            scored(&own, &PRICED) - scored(&own, &UNPRICED),
            PRICED.keyword.windfury,
            9,
        );
        assert_close(
            scored(&enemy, &UNPRICED) - scored(&enemy, &PRICED),
            PRICED.keyword.windfury,
            9,
        );
        // A printed Windfury reads the same.
        let prime = board(json!({ "p1": { "field": [PRIME] }, "p2": { "field": [VANILLA] } }));
        assert_close(
            unit_worth(&prime, &lane_one(&prime, AI), &PRICED)
                - unit_worth(&prime, &lane_one(&prime, AI), &UNPRICED),
            PRICED.keyword.windfury,
            9,
        );
    }

    #[test]
    fn greedy_eval_prices_no_shown_cost_no_fatigue_and_no_windfury() {
        assert_eq!(
            scored(&backed(SHEEPISH, 0), &GREEDY_EVAL),
            scored(&backed(SHEEPISH, 4), &GREEDY_EVAL)
        );
        let state = even();
        assert_eq!(
            scored(&tired(&state, AI, 3), &GREEDY_EVAL),
            scored(&state, &GREEDY_EVAL)
        );
        let own = windfurious(&state, AI);
        assert_eq!(
            unit_worth(&own, &lane_one(&own, AI), &GREEDY_EVAL),
            unit_worth(&state, &lane_one(&state, AI), &GREEDY_EVAL)
        );
    }
}
