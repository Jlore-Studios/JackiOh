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

// ---------------------------------------------------------------------------------------------
// R1370: a deck that leans on a set
// ---------------------------------------------------------------------------------------------

mod r1370_a_deck_that_leans_on_a_set {
    use super::*;
    use jackioh_ai::build_ai_deck_traced;
    use jackioh_engine::testkit::{SHIPPED_SETS, SetName, newest_shipped_set};

    /// ceil(size × share), as the deck builder rounds every floor.
    fn floor_of(size: i32, share: f64) -> usize {
        (f64::from(size) * share).ceil() as usize
    }

    fn of_set(deck: &[String], set: SetName) -> usize {
        defs_of(deck).iter().filter(|def| def.set == set).count()
    }

    fn units_in(deck: &[String]) -> usize {
        defs_of(deck)
            .iter()
            .filter(|def| def.type_ == CardType::Unit)
            .count()
    }

    fn tagged_in(deck: &[String], tag: &str) -> usize {
        defs_of(deck)
            .iter()
            .filter(|def| def.tags.iter().any(|t| t.as_str() == tag))
            .count()
    }

    /// The tags a theme may be (at least `minThemeSize` cards of the pool `banned` leaves), each with
    /// how many cards the pool holds of it.
    fn themes(banned: &[String]) -> Vec<(String, usize)> {
        register_cards();
        let pool: Vec<CardDef> = query(&CatalogQueryArgs::default())
            .into_iter()
            .filter(|def| !banned.contains(&def.id))
            .cloned()
            .collect();
        let mut counts: IndexMap<String, usize> = IndexMap::new();
        for def in &pool {
            for tag in &def.tags {
                if *tag != Tag::Token {
                    *counts.entry(tag.as_str().to_string()).or_insert(0) += 1;
                }
            }
        }
        let mut themes: Vec<(String, usize)> = counts
            .into_iter()
            .filter(|(_, count)| *count >= AI_DECK.min_theme_size as usize)
            .collect();
        themes.sort();
        themes
    }

    /// Over `seeds` seeds: the deck is whole and distinct, at least `leanMinShare` of it is of `lean`,
    /// at least `minUnitShare` of it are units, and a theme the pool holds enough cards of for its
    /// floor is at least `themeMinShare` of it.
    fn every_floor_holds(size: i32, base: Value, lean: SetName, theme: Option<(&str, usize)>, seeds: usize) {
        register_cards();
        let mut literal = base;
        literal["leanSet"] = json!(lean);
        if let Some((tag, _)) = theme {
            literal["theme"] = json!(tag);
        }
        let opts = options(literal.clone());
        for n in 1..=seeds {
            let deck = build_ai_deck(
                &mut create_rng(&format!("r1370:{size}:{lean}:{theme:?}:{n}"), 0),
                size,
                &opts,
            );
            let at = format!("{literal} seed {n}");
            assert_eq!(deck.len(), size as usize, "{at}");
            assert_eq!(deck.iter().collect::<IndexSet<_>>().len(), size as usize, "{at}");
            assert!(
                of_set(&deck, lean) >= floor_of(size, AI_DECK.lean_min_share),
                "{at}: {deck:?}"
            );
            assert!(
                units_in(&deck) >= floor_of(size, AI_DECK.min_unit_share),
                "{at}: {deck:?}"
            );
            if let Some((tag, in_pool)) = theme {
                let needed = floor_of(size, AI_DECK.theme_min_share);
                if in_pool >= needed {
                    assert!(tagged_in(&deck, tag) >= needed, "{at}: {deck:?}");
                }
            }
        }
    }

    #[test]
    fn r1370_without_a_lean_set_these_seeds_deal_the_decks_they_always_did_and_take_no_extra_rng_draw() {
        register_cards();
        // Dealt by the deck builder before R1370, with the cursor its rng stopped at.
        let pins: [(&str, i32, Value, u32, &[&str]); 4] = [
            (
                "r1370-pin:human",
                20,
                json!({ "banned": [] }),
                21,
                &[
                    "classicplus-006",
                    "core-089",
                    "classic-020",
                    "classicplus-013",
                    "classic-024",
                    "classic-084",
                    "classic-086",
                    "classic-007",
                    "classic-089",
                    "classic-079",
                    "classicplus-012",
                    "classicplus-037",
                    "classicplus-078",
                    "core-012",
                    "core-007",
                    "core-053",
                    "classicplus-002",
                    "classicplus-031",
                    "core-073",
                    "classicplus-007",
                ],
            ),
            (
                "r1370-pin:easy",
                20,
                json!({ "manaCap": 4 }),
                21,
                &[
                    "core-089",
                    "classicplus-019",
                    "classic-021",
                    "classic-069",
                    "core-096",
                    "core-068",
                    "classic-012",
                    "core-015",
                    "core-004",
                    "core-053",
                    "classic-001",
                    "classic-082",
                    "classicplus-036",
                    "core-025",
                    "classic-049",
                    "classicplus-067",
                    "classic-004",
                    "classicplus-066",
                    "core-038",
                    "core-059",
                ],
            ),
            (
                "r1370-pin:hard",
                30,
                json!({ "manaCap": 7 }),
                31,
                &[
                    "classicplus-006",
                    "classicplus-049",
                    "core-040",
                    "core-004",
                    "classic-086",
                    "classic-001",
                    "classicplus-041",
                    "classicplus-012",
                    "classic-043",
                    "core-099",
                    "classicplus-030",
                    "core-077",
                    "classic-082",
                    "classic-089",
                    "core-005",
                    "classic-038",
                    "core-019",
                    "classic-055",
                    "classicplus-007",
                    "classic-076",
                    "classic-021",
                    "classicplus-001",
                    "classic-061",
                    "classicplus-005",
                    "classicplus-044",
                    "core-026",
                    "classic-014",
                    "core-095",
                    "core-094",
                    "classicplus-017",
                ],
            ),
            (
                "r1370-pin:themed",
                20,
                json!({ "banned": [], "theme": "Human" }),
                20,
                &[
                    "classic-006",
                    "core-091",
                    "core-004",
                    "classicplus-028",
                    "classic-066",
                    "core-066",
                    "core-088",
                    "core-099",
                    "classic-017",
                    "classic-073",
                    "classic-059",
                    "classicplus-034",
                    "core-001",
                    "core-070",
                    "core-067",
                    "core-074",
                    "classicplus-059",
                    "core-097",
                    "classic-061",
                    "classicplus-012",
                ],
            ),
        ];
        for (seed, size, literal, cursor, expected) in pins {
            for literal in [literal.clone(), {
                // An explicit `null` reads as absent.
                let mut with_null = literal.clone();
                with_null["leanSet"] = Value::Null;
                with_null
            }] {
                let mut rng = create_rng(seed, 0);
                let traced = build_ai_deck_traced(&mut rng, size, &options(literal.clone()));
                assert_eq!(traced.deck, expected, "{seed} {literal}");
                assert_eq!(rng.cursor(), cursor, "{seed} {literal}");
                assert_eq!(traced.lean_forced, 0, "{seed} {literal}");
            }
        }
        // The option is absent unless set, and absent from the options' JSON.
        assert_eq!(AiDeckOptions::default().lean_set, None);
        assert_eq!(
            serde_json::to_value(AiDeckOptions::default()).ok(),
            Some(json!({}))
        );
    }

    #[test]
    fn r1370_lean_set_reads_and_writes_as_lean_set_naming_the_set() {
        let leaning = options(json!({ "leanSet": "Classic+" }));
        assert_eq!(leaning.lean_set, Some(SetName::ClassicPlus));
        assert_eq!(
            serde_json::to_value(&leaning).ok(),
            Some(json!({ "leanSet": "Classic+" }))
        );
        assert_eq!(AI_DECK.lean_min_share, 0.5);
    }

    #[test]
    fn r1370_at_20_cards_half_the_deck_is_of_the_newest_set_on_every_seed_with_no_theme_or_a_rolled_one() {
        let newest = newest_shipped_set();
        assert_eq!(floor_of(DECK_SIZE, AI_DECK.lean_min_share), 10);
        // A human's random deck (nothing banned), with a theme rolled (absent) and with none (null).
        every_floor_holds(DECK_SIZE, json!({ "banned": [] }), newest, None, SEEDS);
        every_floor_holds(
            DECK_SIZE,
            json!({ "banned": [], "theme": null }),
            newest,
            None,
            SEEDS,
        );
    }

    #[test]
    fn r1370_at_20_cards_with_every_theme_the_pool_offers_the_lean_the_theme_and_the_units_all_hold() {
        let newest = newest_shipped_set();
        let themes = themes(&[]);
        assert!(themes.len() > 5, "{themes:?}");
        for (tag, in_pool) in &themes {
            every_floor_holds(
                DECK_SIZE,
                json!({ "banned": [] }),
                newest,
                Some((tag, *in_pool)),
                40,
            );
        }
    }

    #[test]
    fn r1370_at_a_handicaps_size_the_floor_rounds_up_and_holds_beside_the_shadow_ban_with_and_without_a_theme()
     {
        let newest = newest_shipped_set();
        let banned: Vec<String> = SHADOW_BAN_IDS.iter().map(|id| id.to_string()).collect();
        for difficulty in [Difficulty::Medium, Difficulty::Hard] {
            let h = AI_DIFFICULTY[difficulty];
            let base = json!({ "manaCap": h.mana_cap });
            every_floor_holds(h.deck_size, base.clone(), newest, None, SEEDS);
            for (tag, in_pool) in themes(&banned) {
                every_floor_holds(h.deck_size, base.clone(), newest, Some((&tag, in_pool)), 20);
            }
            // The shadow ban still applies to the AI's own decks (R186).
            for n in 1..=50 {
                let deck = build_ai_deck(
                    &mut create_rng(&format!("r1370:ban:{difficulty}:{n}"), 0),
                    h.deck_size,
                    &options(json!({ "manaCap": h.mana_cap, "leanSet": newest })),
                );
                assert!(
                    deck.iter().all(|id| !is_shadow_banned(id)),
                    "{difficulty} {n}: {deck:?}"
                );
            }
        }
        // Medium's 25 cards owe 13 of the set, rounded up from 12.5; Hard's 30 owe 15.
        assert_eq!(
            floor_of(AI_DIFFICULTY.medium.deck_size, AI_DECK.lean_min_share),
            13
        );
        assert_eq!(floor_of(AI_DIFFICULTY.hard.deck_size, AI_DECK.lean_min_share), 15);
    }

    #[test]
    fn r1370_every_shipped_set_can_be_leaned_on_not_only_the_newest() {
        for set in SHIPPED_SETS {
            every_floor_holds(DECK_SIZE, json!({ "banned": [] }), set, None, 100);
        }
    }

    #[test]
    fn r1370_a_set_short_of_cards_gives_all_it_has_and_the_deck_is_still_dealt_whole() {
        register_cards();
        let newest = newest_shipped_set();
        let mut of_newest: Vec<String> = query(&CatalogQueryArgs::default())
            .into_iter()
            .filter(|def| def.set == newest)
            .map(|def| def.id.clone())
            .collect();
        of_newest.sort();
        // Ban all but six of the set's cards, short of the ten a 20-card deck owes.
        let kept: Vec<String> = of_newest[..6].to_vec();
        let banned: Vec<String> = of_newest[6..].to_vec();
        for n in 1..=SEEDS {
            let deck = build_ai_deck(
                &mut create_rng(&format!("r1370:short:{n}"), 0),
                DECK_SIZE,
                &options(json!({ "banned": banned, "leanSet": newest })),
            );
            assert_eq!(deck.len(), DECK_SIZE as usize, "seed {n}");
            for id in &kept {
                assert!(deck.contains(id), "seed {n}: {id} is missing from {deck:?}");
            }
            assert_eq!(of_set(&deck, newest), kept.len(), "seed {n}");
            assert!(
                units_in(&deck) >= floor_of(DECK_SIZE, AI_DECK.min_unit_share),
                "seed {n}"
            );
        }
        // A set the pool holds none of (Meditative, which has not shipped, R1420) gives none: the floor
        // narrows nothing and no card weighs more, so with no theme the deck is the one dealt without it.
        assert!(!jackioh_engine::testkit::set_ships(SetName::Meditative));
        for n in 1..=50 {
            let plain = build_ai_deck(
                &mut create_rng(&format!("r1370:none:{n}"), 0),
                DECK_SIZE,
                &options(json!({ "banned": [], "theme": null })),
            );
            let leaning = build_ai_deck(
                &mut create_rng(&format!("r1370:none:{n}"), 0),
                DECK_SIZE,
                &options(json!({ "banned": [], "theme": null, "leanSet": "Meditative" })),
            );
            assert_eq!(leaning, plain, "seed {n}");
        }
    }

    #[test]
    fn r1370_the_lean_boost_seldom_leaves_the_floor_to_force_a_card() {
        register_cards();
        let newest = newest_shipped_set();
        let seeds = 500;
        let forced = (1..=seeds)
            .filter(|n| {
                build_ai_deck_traced(
                    &mut create_rng(&format!("r1370:forced:{n}"), 0),
                    DECK_SIZE,
                    &options(json!({ "banned": [], "leanSet": newest })),
                )
                .lean_forced
                    > 0
            })
            .count();
        // `AI_DECK.leanBoost`'s measurement: fewer than one deck in ten needs the floor to force a card.
        assert!(forced * 10 < seeds, "{forced} of {seeds} decks were forced");
        // And it is a floor, not a weight: some deck the weights left short was forced to it.
        assert!(forced > 0, "no deck needed the floor, so this measured nothing");
    }
}
