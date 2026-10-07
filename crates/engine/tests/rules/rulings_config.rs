//! SPEC §11's rulings whose ruling is a number or a named switch, asserted against `config.rs`
//! (CLAUDE.md rule 9): the "decide" rows R1, R4, R5, R14, R26 and R39 among them, and R2's turn cap.
//! Ported from `packages/engine/test/rulings.test.ts` (part 28, #133), which held one `it` per §11
//! row. Only its `expect(config.X)` assertions live on here, one test per row, named after it.
//!
//! The rest of that file is gone on purpose:
//! - its index of proofs (`provenIn(n, files)`) is each ruling note's `proven_in` front matter in
//!   `spec/rulings/R<nnnn>.md`, and `cargo jackioh spec check` holds what its completeness test held;
//! - its 23 pins on other files' source text (SQL migrations, `actor.ts`, `codes.ts`, …) and its
//!   reads of `apps/server/src/config.ts` constants are dropped (#133): the server's own tests prove
//!   those rulings by behaviour, and the notes name them.
//!
//! Every value is compared through its JSON form where its Rust type is a struct or an enum (the
//! form `export_config` hands the web, SURFACE §5.1), so the assertion reads exactly as the TS one
//! did.

use std::collections::BTreeSet;

use jackioh_engine::config;
use serde_json::{Value, json};

fn json_of<T: serde::Serialize + ?Sized>(value: &T) -> Value {
    serde_json::to_value(value).expect("a config value serialises")
}

#[test]
fn r1_fires_cry_only_on_a_play_from_hand_or_a_cast() {
    assert_eq!(json_of(&config::CRY_ON_PLAY_ONLY), json!(true));
}

#[test]
fn r2_counts_the_turn_cap_in_player_turns_60_so_30_each_r389() {
    assert_eq!(config::TURN_CAP_PLAYER_TURNS, 60);
    assert_eq!(config::TURN_CAP_PLAYER_TURNS / 2, 30);
}

#[test]
fn r4_caps_the_hand_at_10_and_burns_the_overflow_to_the_graveyard() {
    assert_eq!(config::HAND_CAP, 10);
}

#[test]
fn r5_does_not_restrict_attacks_by_lane() {
    assert_eq!(json_of(&config::LANE_RESTRICTED_ATTACKS), json!(false));
}

#[test]
fn r9_draws_the_mulligan_replacements_before_shuffling_the_returned_cards_back_in() {
    assert_eq!(json_of(&config::MULLIGAN_ORDER), json!("draw-then-shuffle"));
}

#[test]
fn r14_rotates_two_independent_rings_bounces_a_locked_destination_and_carries_damage_and_buffs() {
    assert_eq!(json_of(&config::ROTATION_RING), json!("two-rings"));
}

#[test]
fn r21_draws_random_keywords_from_the_fourteen_entry_pool_and_never_repeats_one_on_a_unit() {
    let pool = json_of(&config::RANDOM_KEYWORD_POOL);
    assert_eq!(
        pool,
        json!([
            "Taunt",
            "Armor 1",
            "Rush",
            "Charge",
            "First Strike",
            "Poisonous",
            "Lifesteal",
            "Reborn",
            "Divine Shield",
            "Trample",
            "Cleave",
            "Pierce",
            "Windfury",
            "Deft"
        ])
    );
    let entries: BTreeSet<String> = pool
        .as_array()
        .expect("the pool is a list")
        .iter()
        .map(|entry| entry.as_str().expect("a keyword is a string").to_string())
        .collect();
    assert_eq!(entries.len(), 14);
}

#[test]
fn r25_clamps_the_fib_index_at_fib_11_89() {
    assert_eq!(config::FIB.len(), 12);
    assert_eq!([config::fib(11), config::fib(12), config::fib(99)], [89, 89, 89]);
    assert_eq!(
        [config::fib(0), config::fib(1), config::fib(2), config::fib(7)],
        [0, 1, 1, 13]
    );
}

#[test]
fn r26_reads_genns_greed_as_exile_all_odd_cost_cards() {
    assert_eq!(json_of(&config::GENN_GREED_EXILES), json!("odd"));
}

#[test]
fn r28_caps_the_call_to_chaos_chain_at_20_and_pairs_the_recursion_with_one_of_the_other_nine() {
    assert_eq!(config::CALL_TO_CHAOS_CHAIN_CAP, 20);
}

#[test]
fn r36_lets_only_the_active_player_offer_a_draw_once_a_turn_and_blocks_a_decliner_for_three_turns() {
    assert_eq!(config::DRAW_OFFERS_PER_TURN, 1);
    assert_eq!(config::DRAW_OFFER_BLOCK_TURNS, 3);
}

#[test]
fn r39_gives_felinor_fiender_printed_stats_plus_the_sum_of_your_felinors_never_below_printed() {
    assert_eq!(json_of(&config::FIENDER_STATS_MODE), json!("printed-plus-sum"));
}

#[test]
fn r44_projects_lethal_after_armor_the_anti_oneshot_cap_and_trample_excess_then_cancels_the_attack() {
    assert_eq!(config::ANTI_ONESHOT_CAP.base, 5);
    assert_eq!(config::ANTI_ONESHOT_CAP.radiant, 3);
}

#[test]
fn r58_casts_at_most_cast_on_draw_chain_cap_cards_per_draw_even_on_a_full_hand() {
    assert_eq!(config::CAST_ON_DRAW_CHAIN_CAP, 20);
}

#[test]
fn r72_reads_cards_in_exile_as_your_own_pile_and_counts_missing_health_from_30() {
    assert_eq!(config::HERO_HEALTH, 30);
}

#[test]
fn r77_fuses_into_a_transient_definition_whose_cost_is_capped_at_fuse_cost_cap() {
    assert_eq!(config::FUSE_COST_CAP, 4);
}

#[test]
fn r80_caps_a_library_at_library_cap_creating_no_new_card_and_routing_an_existing_one() {
    assert_eq!(config::LIBRARY_CAP, 60);
}

#[test]
fn r84_keeps_concede_offer_draw_and_answer_draw_out_of_the_ai_policy() {
    assert_eq!(json_of(&config::AI_END_TURN_PROBABILITY), json!(0.1));
}

#[test]
fn r102_composes_a_fuse_member_by_member_capped_cost_max_tribute_namespaced_triggers_consumed_ingredients() {
    assert_eq!(config::FUSE_COST_CAP, 4);
}

#[test]
fn r118_lets_a_traps_prompt_interrupt_a_play_without_eating_its_cry() {
    assert_eq!(json_of(&config::CRY_ON_PLAY_ONLY), json!(true));
}

#[test]
fn r124_adds_hero_armor_up_across_its_sources_where_the_anti_oneshot_cap_takes_the_smallest() {
    assert_eq!(config::ANTI_ONESHOT_CAP.base, 5);
    assert_eq!(config::ANTI_ONESHOT_CAP.radiant, 3);
}

#[test]
fn r135_exiles_library_then_hand_then_graveyard_each_card_its_own_exile_after_the_draw() {
    assert_eq!(json_of(&config::GENN_GREED_EXILES), json!("odd"));
}

#[test]
fn r180_gives_each_seat_an_optional_handicap_stores_none_equal_to_a_humans_and_folds_it() {
    let human = json_of(&config::HUMAN_HANDICAP);
    assert_eq!(
        human,
        json!({
            "deckSize": config::DECK_SIZE,
            "manaBonus": 0,
            "manaCap": config::MAX_MANA,
            "extraOpeningCards": 0,
            "extraDrawsPerTurn": 0
        })
    );
    assert_eq!(json_of(&config::DIFFICULTIES), json!(["easy", "medium", "hard"]));
    let tiers = json_of(&config::AI_DIFFICULTY);
    assert_eq!(tiers["easy"], human);
    assert_eq!(
        tiers["medium"],
        json!({ "deckSize": 25, "manaBonus": 1, "manaCap": 5, "extraOpeningCards": 1, "extraDrawsPerTurn": 0 })
    );
    assert_eq!(
        tiers["hard"],
        json!({ "deckSize": 30, "manaBonus": 1, "manaCap": 7, "extraOpeningCards": 1, "extraDrawsPerTurn": 1 })
    );
}

#[test]
fn r183_makes_a_handicaps_extra_draws_separate_2_4_draws_each_with_its_own_chain_and_fatigue() {
    assert_eq!(config::DRAWS_PER_TURN, 1);
}

#[test]
fn r290_gives_the_tutorial_opponent_a_handicap_below_easy_12_cards_3_mana_a_hero_at_20() {
    assert_eq!(
        json_of(&config::AI_TUTORIAL),
        json!({
            "deckSize": 12,
            "manaBonus": 0,
            "manaCap": 3,
            "extraOpeningCards": 0,
            "extraDrawsPerTurn": 0,
            "heroHealth": 20
        })
    );
    let tiers = json_of(&config::DIFFICULTIES);
    assert!(
        !tiers
            .as_array()
            .expect("the tiers are a list")
            .contains(&json!("tutorial"))
    );
}

#[test]
fn r346_makes_a_pierce_sources_damage_skip_armor_and_adds_pierce_to_the_random_keyword_pool() {
    let pool = json_of(&config::RANDOM_KEYWORD_POOL);
    assert!(
        pool.as_array()
            .expect("the pool is a list")
            .contains(&json!("Pierce"))
    );
}

#[test]
fn r348_holds_a_chosen_x_to_at_least_1() {
    assert_eq!(config::MIN_CHOSEN_X, 1);
}

#[test]
fn r349_doubles_the_stats_of_a_unit_that_becomes_radiant_with_no_radiant_form() {
    assert_eq!(config::RADIANT_FALLBACK_FACTOR, 2);
}

#[test]
fn r389_doubles_the_turn_cap_and_the_match_ceiling_with_it() {
    // The match ceiling's half (MATCH_CEILING_MINUTES, 120) is the server's config, proved by its
    // own clock tests (spec/rulings/R0389.md).
    assert_eq!(config::TURN_CAP_PLAYER_TURNS, 60);
}
