//! AI decks (SPEC §9.9 "Decks", R184; docs/polish/3-ai.md B22, B23).
//!
//! B22: `build_ai_deck(rng, size, options)` deals exactly `size` distinct non-token ids of every set (R380), every
//! `include`d id and no banned one, and the same seed always deals the same deck. B23: over many
//! seeds the cost curve sits within AI_DECK.curveTolerance of `curve_targets`, units make up at least
//! AI_DECK.minUnitShare, and a Human-themed deck is at least AI_DECK.themeMinShare Human.
//!
//! Seeds are `create_rng("deck-test:<size>:<n>")`, n = 1..200, so a failing seed is reproducible.
//!
//! Port of `packages/ai/test/deck.test.ts`: TS's `it` inside a `for` over sizes (and mana caps) is one
//! `#[test]` per value, named with it. A bucket is read by its literal (`"0-1"`, `"2"`, `"3"`, `"4+"`)
//! through serde, and `curve_targets`' and `AI_DECK.curve`'s per-bucket numbers likewise, so the test
//! holds whatever Rust shape carries TS's `Record<CostBucket, number>`. TS's timeouts are dropped.

use std::panic::catch_unwind;

use indexmap::{IndexMap, IndexSet};
use jackioh_ai::{
    AI_DECK, AiDeckOptions, CostBucket, SHADOW_BAN_IDS, build_ai_deck, cost_bucket, curve_targets,
};
use jackioh_engine::testkit::{
    AI_DIFFICULTY, CardDef, CardType, CatalogQueryArgs, CreateGameOptions, DECK_SIZE, Difficulty, MAX_MANA,
    PerPlayerOpt, Tag, Value, create_game, create_rng, def_of, json, json_as, query, query_cost,
    registered_catalog, validate_deck,
};

use super::support::{ai_pool, register_cards};

const SEEDS: usize = 200;
const SIZES: [i32; 3] = [20, 25, 30];
const BUCKETS: [&str; 4] = ["0-1", "2", "3", "4+"];

/// TS's object literal as `AiDeckOptions`.
fn options(literal: Value) -> AiDeckOptions {
    json_as(literal)
}

fn decks(size: i32, options: &AiDeckOptions, label: &str) -> Vec<Vec<String>> {
    register_cards();
    (0..SEEDS)
        .map(|i| {
            build_ai_deck(
                &mut create_rng(&format!("deck-test:{label}:{size}:{}", i + 1), 0),
                size,
                options,
            )
        })
        .collect()
}

fn defs_of(deck: &[String]) -> Vec<CardDef> {
    deck.iter().map(|id| def_of(None, id).clone()).collect()
}

/// A bucket's literal, as TS's `CostBucket` writes it.
fn bucket_name(bucket: CostBucket) -> String {
    serde_json::to_value(bucket)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// TS `Record<CostBucket, number>` read by literal.
fn per_bucket(record: Value) -> IndexMap<String, f64> {
    BUCKETS
        .iter()
        .map(|name| (name.to_string(), record[*name].as_f64().unwrap_or(f64::NAN)))
        .collect()
}

fn targets_of(size: i32, mana_cap: i32) -> Value {
    serde_json::to_value(curve_targets(size, mana_cap)).expect("curve_targets serialises")
}

/// Mean share per bucket over a set of decks.
fn mean_shares(all: &[Vec<String>]) -> IndexMap<String, f64> {
    let mut sums: IndexMap<String, f64> = BUCKETS.iter().map(|name| (name.to_string(), 0.0)).collect();
    for deck in all {
        for def in defs_of(deck) {
            *sums.entry(bucket_name(cost_bucket(&def))).or_insert(0.0) += 1.0 / deck.len() as f64;
        }
    }
    for name in BUCKETS {
        if let Some(sum) = sums.get_mut(name) {
            *sum /= all.len() as f64;
        }
    }
    sums
}

fn mean_share_where(all: &[Vec<String>], test: impl Fn(&CardDef) -> bool) -> f64 {
    let total: f64 = all
        .iter()
        .map(|deck| defs_of(deck).iter().filter(|def| test(def)).count() as f64 / deck.len() as f64)
        .sum();
    total / all.len() as f64
}

fn is_shadow_banned(id: &str) -> bool {
    SHADOW_BAN_IDS.iter().any(|banned| banned == &id)
}

// ---------------------------------------------------------------------------------------------
// B22
// ---------------------------------------------------------------------------------------------

mod build_ai_deck_b22 {
    use super::*;

    fn deals_exactly(size: i32) {
        let pool: IndexSet<String> = ai_pool().into_iter().collect();
        for (at, deck) in decks(size, &AiDeckOptions::default(), "default")
            .iter()
            .enumerate()
        {
            assert_eq!(deck.len(), size as usize, "seed {}", at + 1);
            assert_eq!(
                deck.iter().collect::<IndexSet<_>>().len(),
                size as usize,
                "seed {}",
                at + 1
            );
            for id in deck {
                assert!(pool.contains(id), "seed {}: {id}", at + 1);
                assert!(!is_shadow_banned(id), "seed {}: {id} is shadow-banned", at + 1);
            }
        }
    }

    #[test]
    fn r380_b22_deals_exactly_20_distinct_non_token_ids_of_every_set_with_no_banned_card_over_200_seeds() {
        deals_exactly(SIZES[0]);
    }

    #[test]
    fn r380_b22_deals_exactly_25_distinct_non_token_ids_of_every_set_with_no_banned_card_over_200_seeds() {
        deals_exactly(SIZES[1]);
    }

    #[test]
    fn r380_b22_deals_exactly_30_distinct_non_token_ids_of_every_set_with_no_banned_card_over_200_seeds() {
        deals_exactly(SIZES[2]);
    }

    #[test]
    fn r380_b22_the_decks_reach_every_set_core_classic_and_classic_plus_alike() {
        let dealt: IndexSet<String> = decks(30, &options(json!({})), "sets")
            .iter()
            .flatten()
            .map(|id| def_of(None, id).set.to_string())
            .collect();
        let mut sets: Vec<String> = dealt.into_iter().collect();
        sets.sort();
        assert_eq!(sets, vec!["Classic", "Classic+", "Core"]);
    }

    #[test]
    fn r390_b22_boost_multiplies_its_ids_weights_so_they_are_dealt_far_more_often_without_it_the_deal_is_unchanged()
     {
        let ids: Vec<String> = ai_pool()
            .into_iter()
            .filter(|id| !is_shadow_banned(id))
            .skip(40)
            .take(20)
            .collect();
        let dealt_of = |all: &[Vec<String>]| -> usize {
            all.iter()
                .map(|deck| deck.iter().filter(|id| ids.contains(id)).count())
                .sum()
        };
        let plain = decks(20, &options(json!({})), "boost");
        let boosted = decks(20, &options(json!({ "boost": { "ids": ids, "by": 4 } })), "boost");
        assert!(dealt_of(&boosted) > 2 * dealt_of(&plain));
        assert_eq!(
            decks(20, &options(json!({ "boost": { "ids": ids, "by": 1 } })), "boost"),
            plain
        );
    }

    #[test]
    fn b22_the_same_seed_deals_the_same_deck_and_different_seeds_deal_different_decks() {
        register_cards();
        for size in SIZES {
            let one = build_ai_deck(
                &mut create_rng("deck-test-same", 0),
                size,
                &AiDeckOptions::default(),
            );
            let two = build_ai_deck(
                &mut create_rng("deck-test-same", 0),
                size,
                &AiDeckOptions::default(),
            );
            assert_eq!(two, one);
        }
        let distinct: IndexSet<String> = decks(20, &AiDeckOptions::default(), "default")
            .into_iter()
            .map(|mut deck| {
                deck.sort();
                deck.join(",")
            })
            .collect();
        assert!(distinct.len() > SEEDS / 2);
    }

    #[test]
    fn b22_every_include_id_is_dealt_over_200_seeds() {
        // Only unbanned ids may be forced in, so pick three the shadow ban leaves alone.
        let include: Vec<&str> = [
            "core-019", "core-044", "core-072", "core-008", "core-011", "core-020", "core-025",
        ]
        .into_iter()
        .filter(|id| !is_shadow_banned(id))
        .take(3)
        .collect();
        assert_eq!(include.len(), 3);
        for (at, deck) in decks(25, &options(json!({ "include": include })), "include")
            .iter()
            .enumerate()
        {
            for id in &include {
                assert!(deck.iter().any(|dealt| dealt == id), "seed {}", at + 1);
            }
            assert_eq!(deck.iter().collect::<IndexSet<_>>().len(), 25);
        }
    }

    #[test]
    fn b22_an_explicit_ban_list_is_honoured_instead_of_the_default() {
        let banned: Vec<String> = ai_pool().into_iter().take(15).collect();
        for (at, deck) in decks(30, &options(json!({ "banned": banned })), "banned")
            .iter()
            .enumerate()
        {
            for id in &banned {
                assert!(!deck.contains(id), "seed {}", at + 1);
            }
            assert_eq!(deck.len(), 30);
        }
    }

    #[test]
    fn b22_banned_empty_lifts_the_ban_so_a_card_a_ban_would_keep_out_can_be_dealt() {
        let target = "core-087";
        let kept = decks(20, &options(json!({ "banned": [target] })), "lift-kept");
        assert!(kept.iter().all(|deck| !deck.iter().any(|id| id == target)));
        let lifted = build_ai_deck(
            &mut create_rng("deck-test-lift", 0),
            20,
            &options(json!({ "banned": [], "include": [target] })),
        );
        assert!(lifted.iter().any(|id| id == target));
        // And with no ban at all, every dealt card is still a non-token card.
        let pool: IndexSet<String> = ai_pool().into_iter().collect();
        for deck in decks(30, &options(json!({ "banned": [] })), "lift") {
            for id in &deck {
                assert!(pool.contains(id));
            }
        }
    }

    #[test]
    fn b22_throws_when_the_unbanned_pool_is_too_small_to_fill_the_deck() {
        let pool = ai_pool();
        let banned: Vec<String> = pool[..pool.len() - 10].to_vec();
        let built = catch_unwind(|| {
            build_ai_deck(
                &mut create_rng("deck-test-small", 0),
                20,
                &options(json!({ "banned": banned })),
            )
        });
        assert!(built.is_err());
    }

    #[test]
    fn b22_throws_for_a_deck_larger_than_the_whole_pool() {
        let size = ai_pool().len() as i32 + 1;
        let built = catch_unwind(|| {
            build_ai_deck(
                &mut create_rng("deck-test-huge", 0),
                size,
                &options(json!({ "banned": [] })),
            )
        });
        assert!(built.is_err());
    }

    #[test]
    fn r184_b22_every_dealt_deck_is_a_legal_deck_for_its_tiers_handicap() {
        register_cards();
        let catalog = registered_catalog();
        for difficulty in [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard] {
            let h = AI_DIFFICULTY[difficulty];
            for n in 1..=20 {
                let deck = build_ai_deck(
                    &mut create_rng(&format!("deck-test-legal:{difficulty}:{n}"), 0),
                    h.deck_size,
                    &options(json!({ "manaCap": h.mana_cap })),
                );
                let checked = validate_deck(&deck, catalog, "p2", h.deck_size);
                assert!(checked.is_ok(), "{difficulty} {n}: {checked:?}");
            }
        }
        let human = build_ai_deck(
            &mut create_rng("deck-test-legal-human", 0),
            DECK_SIZE,
            &options(json!({ "banned": [] })),
        );
        let hard = build_ai_deck(
            &mut create_rng("deck-test-legal-hard", 0),
            AI_DIFFICULTY.hard.deck_size,
            &options(json!({ "manaCap": AI_DIFFICULTY.hard.mana_cap })),
        );
        // `create_game` panics where TS threw.
        create_game(&CreateGameOptions {
            seed: "deck-test-legal".to_string(),
            decks: (human, hard),
            handicaps: Some(PerPlayerOpt {
                p1: None,
                p2: Some(AI_DIFFICULTY.hard),
            }),
            ..CreateGameOptions::default()
        });
    }
}

// ---------------------------------------------------------------------------------------------
// B23
// ---------------------------------------------------------------------------------------------

mod the_curve_and_the_theme_b23 {
    use super::*;

    fn bucket_of(def_id: &str) -> String {
        bucket_name(cost_bucket(def_of(None, def_id)))
    }

    #[test]
    fn b23_cost_bucket_reads_query_cost_x_is_0_an_embiggen_card_is_its_base_price() {
        register_cards();
        assert_eq!(bucket_of("core-008"), "0-1"); // 1
        assert_eq!(bucket_of("core-020"), "2"); // 2
        assert_eq!(bucket_of("core-019"), "3"); // 3
        assert_eq!(bucket_of("core-025"), "4+"); // 4
        assert_eq!(bucket_of("core-029"), "4+"); // 6
        assert_eq!(bucket_of("core-024"), "0-1"); // X
        assert_eq!(bucket_of("core-084"), "2"); // { base: 2, embiggen: 4 }
        assert_eq!(bucket_of("core-021"), "0-1"); // 0
    }

    #[test]
    fn b23_curve_targets_are_whole_numbers_summing_to_the_deck_size_each_within_one_card_of_its_share() {
        let curve = per_bucket(serde_json::to_value(AI_DECK.curve).expect("AI_DECK.curve serialises"));
        for size in SIZES {
            for mana_cap in [MAX_MANA, 5, 7] {
                let targets = targets_of(size, mana_cap);
                let shift = AI_DECK.curve_shift_per_mana * f64::from((mana_cap - MAX_MANA).max(0));
                let shares: IndexMap<&str, f64> = [
                    ("0-1", curve["0-1"] - shift),
                    ("2", curve["2"]),
                    ("3", curve["3"]),
                    ("4+", curve["4+"] + shift),
                ]
                .into_iter()
                .collect();
                let mut sum = 0.0;
                for bucket in BUCKETS {
                    let target = &targets[bucket];
                    assert!(target.as_i64().is_some(), "{size}/{mana_cap} {bucket}");
                    let target = target.as_f64().unwrap_or(f64::NAN);
                    assert!(target >= 0.0);
                    assert!(
                        (target - shares[bucket] * f64::from(size)).abs() < 1.0,
                        "{size}/{mana_cap} {bucket}"
                    );
                    sum += target;
                }
                assert_eq!(sum, f64::from(size), "{size}/{mana_cap}");
            }
        }
    }

    #[test]
    fn b23_a_higher_mana_cap_moves_share_from_the_cheap_bucket_to_the_expensive_one() {
        let low = per_bucket(targets_of(30, MAX_MANA));
        let high = per_bucket(targets_of(30, 7));
        assert!(high["0-1"] < low["0-1"]);
        assert!(high["4+"] > low["4+"]);
    }

    fn curve_within_tolerance(size: i32, mana_cap: i32) {
        let all = decks(
            size,
            &options(json!({ "manaCap": mana_cap })),
            &format!("curve-{mana_cap}"),
        );
        let shares = mean_shares(&all);
        let targets = per_bucket(targets_of(size, mana_cap));
        for bucket in BUCKETS {
            let target = targets[bucket] / f64::from(size);
            assert!(
                (shares[bucket] - target).abs() <= AI_DECK.curve_tolerance,
                "{bucket}: mean {:.3} vs target {:.3}",
                shares[bucket],
                target
            );
        }
    }

    #[test]
    fn b23_size_20_mana_cap_4_each_buckets_mean_share_is_within_curve_tolerance_of_its_target() {
        curve_within_tolerance(20, MAX_MANA);
    }

    #[test]
    fn b23_size_20_mana_cap_5_each_buckets_mean_share_is_within_curve_tolerance_of_its_target() {
        curve_within_tolerance(20, 5);
    }

    #[test]
    fn b23_size_20_mana_cap_7_each_buckets_mean_share_is_within_curve_tolerance_of_its_target() {
        curve_within_tolerance(20, 7);
    }

    #[test]
    fn b23_size_25_mana_cap_4_each_buckets_mean_share_is_within_curve_tolerance_of_its_target() {
        curve_within_tolerance(25, MAX_MANA);
    }

    #[test]
    fn b23_size_25_mana_cap_5_each_buckets_mean_share_is_within_curve_tolerance_of_its_target() {
        curve_within_tolerance(25, 5);
    }

    #[test]
    fn b23_size_25_mana_cap_7_each_buckets_mean_share_is_within_curve_tolerance_of_its_target() {
        curve_within_tolerance(25, 7);
    }

    #[test]
    fn b23_size_30_mana_cap_4_each_buckets_mean_share_is_within_curve_tolerance_of_its_target() {
        curve_within_tolerance(30, MAX_MANA);
    }

    #[test]
    fn b23_size_30_mana_cap_5_each_buckets_mean_share_is_within_curve_tolerance_of_its_target() {
        curve_within_tolerance(30, 5);
    }

    #[test]
    fn b23_size_30_mana_cap_7_each_buckets_mean_share_is_within_curve_tolerance_of_its_target() {
        curve_within_tolerance(30, 7);
    }

    fn units_hold_their_share(size: i32) {
        let share = mean_share_where(&decks(size, &AiDeckOptions::default(), "default"), |def| {
            def.type_ == CardType::Unit
        });
        assert!(share >= AI_DECK.min_unit_share);
    }

    #[test]
    fn b23_size_20_units_make_up_at_least_min_unit_share_on_average() {
        units_hold_their_share(20);
    }

    #[test]
    fn b23_size_25_units_make_up_at_least_min_unit_share_on_average() {
        units_hold_their_share(25);
    }

    #[test]
    fn b23_size_30_units_make_up_at_least_min_unit_share_on_average() {
        units_hold_their_share(30);
    }

    fn is_human(def: &CardDef) -> bool {
        def.tags.contains(&Tag::Human)
    }

    #[test]
    fn b23_a_human_themed_deck_is_at_least_theme_min_share_human_well_above_the_pools_own_share() {
        register_cards();
        let pool = query(&CatalogQueryArgs::default());
        let pool_share = pool.iter().filter(|def| is_human(def)).count() as f64 / pool.len() as f64;
        for size in SIZES {
            let share = mean_share_where(
                &decks(size, &options(json!({ "theme": "Human" })), "human"),
                is_human,
            );
            assert!(share >= AI_DECK.theme_min_share, "size {size}");
            assert!(share > pool_share, "size {size}");
        }
    }

    #[test]
    fn b23_at_the_human_mana_cap_a_card_that_can_never_be_cast_is_rarely_dealt() {
        register_cards();
        let pool = query(&CatalogQueryArgs::default());
        let uncastable = |def: &CardDef| -> bool { query_cost(def) > MAX_MANA + AI_DECK.cost_slack };
        let pool_share = pool.iter().filter(|def| uncastable(def)).count() as f64 / pool.len() as f64;
        assert!(pool_share > 0.0);
        let dealt = mean_share_where(
            &decks(20, &options(json!({ "manaCap": MAX_MANA })), "uncastable"),
            uncastable,
        );
        assert!(dealt < pool_share);
    }

    #[test]
    fn b23_a_theme_whose_every_card_is_banned_cannot_lean_the_deck_which_is_still_dealt_whole() {
        register_cards();
        let humans: Vec<String> = query(&json_as::<CatalogQueryArgs>(json!({ "tags": ["Human"] })))
            .iter()
            .map(|def| def.id.clone())
            .collect();
        assert!(humans.len() as f64 >= AI_DECK.min_theme_size as f64);
        for n in 1..=20 {
            let deck = build_ai_deck(
                &mut create_rng(&format!("deck-test-banned-theme:{n}"), 0),
                20,
                &options(json!({ "theme": "Human", "banned": humans })),
            );
            assert_eq!(deck.len(), 20);
            assert_eq!(deck.iter().collect::<IndexSet<_>>().len(), 20);
            for id in &deck {
                assert!(!humans.contains(id), "seed {n}: {id}");
            }
        }
    }

    #[test]
    fn b23_theme_null_rolls_no_theme_so_the_human_share_stays_below_a_themed_decks() {
        let themed = mean_share_where(
            &decks(20, &options(json!({ "theme": "Human" })), "themed"),
            is_human,
        );
        // TS's `theme: null` (no theme), told apart from an absent theme (roll one): `Some(None)`.
        let no_theme = AiDeckOptions {
            theme: Some(None),
            ..AiDeckOptions::default()
        };
        let plain = mean_share_where(&decks(20, &no_theme, "plain"), is_human);
        assert!(plain < themed);
    }
}
