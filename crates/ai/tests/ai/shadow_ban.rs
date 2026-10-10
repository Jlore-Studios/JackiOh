//! The AI's shadow ban and the sweep that decides it (SPEC §9.9, R186; docs/polish/3-ai.md B24, B25).
//!
//! B24: the table itself. Every entry names a real non-token card of any set and a reason that starts
//! with the sweep flags that put it there; the AI never deals itself a banned card; and enough
//! cards stay unbanned to build a 30-card Hard deck. A banned card stays legal for every player.
//!
//! B25: `sweepFlags` raises each flag exactly at its AI_SWEEP threshold, tested on both sides of
//! every bound with synthetic stats, and one real `sweepCard` run on a plain card flags no error.
//!
//! Port of `packages/ai/test/shadowBan.test.ts`. The sweep's fixtures (`SweepStats`, `SweepResult`,
//! `SweepPass2`) are built from TS's object literals as JSON and read back as JSON, so the tests pin
//! the shapes the sweep's reports carry (`--report` joins them as JSON), whatever the Rust layout.
//! TS's per-test `{ timeout }` has no `cargo test` twin.

use std::cell::Cell;

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::ai_pool;

const FLAGS: &[&str] = &["error", "timeout", "neverPlayed", "selfHarm"];

/// TS's `REASON = /^(error|timeout|neverPlayed|selfHarm)(, (error|timeout|neverPlayed|selfHarm))*: \S/`,
/// by hand (no regex crate in a pure crate).
fn matches_reason(reason: &str) -> bool {
    let Some((head, rest)) = reason.split_once(": ") else {
        return false;
    };
    head.split(", ").all(|flag| FLAGS.contains(&flag))
        && rest.chars().next().is_some_and(|c| !c.is_whitespace())
}

/// A value as its JSON, for comparisons that pin the wire shape rather than a Rust type's name.
fn js<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialisable")
}

/// Every JSON number as a float, so an `f64` field written `0.0` equals TS's literal `0`.
fn normalized(value: Value) -> Value {
    match value {
        Value::Number(n) => Value::from(n.as_f64().unwrap_or(f64::NAN)),
        Value::Array(items) => Value::Array(items.into_iter().map(normalized).collect()),
        Value::Object(map) => Value::Object(map.into_iter().map(|(k, v)| (k, normalized(v))).collect()),
        other => other,
    }
}

/// TS's `{ ...base, ...over }` on two JSON objects (an absent `over` changes nothing).
fn spread(base: Value, over: Option<&Value>) -> Value {
    let mut out = base;
    if let (Some(target), Some(Value::Object(extra))) = (out.as_object_mut(), over) {
        for (key, value) in extra {
            target.insert(key.clone(), value.clone());
        }
    }
    out
}

/// Jest's `toMatchObject`: every key `expected` names holds an equal value (objects recursively).
fn assert_match_object(actual: &Value, expected: &Value) {
    match (actual, expected) {
        (Value::Object(have), Value::Object(want)) => {
            for (key, value) in want {
                let got = have.get(key).unwrap_or(&Value::Null);
                assert_match_object(got, value);
            }
        }
        _ => assert_eq!(normalized(actual.clone()), normalized(expected.clone())),
    }
}

/// The ids of a shadow-ban-shaped table, in table order.
fn ids_of(table: &[(&str, &str)]) -> Vec<String> {
    table.iter().map(|(id, _)| id.to_string()).collect()
}

fn banned_ids() -> Vec<String> {
    SHADOW_BAN_IDS.iter().map(|id| id.to_string()).collect()
}

// ---------------------------------------------------------------------------------------------
// B24
// ---------------------------------------------------------------------------------------------

mod the_shadow_ban_b24 {
    use super::*;

    /// R186 B24: every entry is a real non-token card with a reason that starts with its sweep flags
    #[test]
    fn r186_b24_every_entry_is_a_real_non_token_card_with_a_reason_that_starts_with_its_sweep_flags() {
        jackioh_cards::register_all();
        let pool: IndexSet<String> = ai_pool().into_iter().collect();
        for &(def_id, reason) in SHADOW_BAN {
            assert!(pool.contains(def_id), "{def_id} is not a non-token card");
            assert!(matches_reason(reason), "{def_id}: {reason}");
        }
    }

    /// R186 B24: SHADOW_BAN_IDS is exactly the table's keys, sorted
    #[test]
    fn r186_b24_shadow_ban_ids_is_exactly_the_tables_keys_sorted() {
        let mut keys = ids_of(SHADOW_BAN);
        keys.sort();
        assert_eq!(banned_ids(), keys);
        assert_eq!(
            banned_ids().into_iter().collect::<IndexSet<String>>().len(),
            SHADOW_BAN_IDS.len()
        );
    }

    /// R186 B24: the unbanned pool holds at least AI_DECK.minPool cards, enough for a Hard deck
    #[test]
    fn r186_b24_the_unbanned_pool_holds_at_least_ai_deck_min_pool_cards_enough_for_a_hard_deck() {
        jackioh_cards::register_all();
        let banned = banned_ids();
        let unbanned: Vec<String> = ai_pool().into_iter().filter(|id| !banned.contains(id)).collect();
        assert!(unbanned.len() >= AI_DECK.min_pool as usize);
        assert!(unbanned.len() >= AI_DIFFICULTY.hard.deck_size as usize);
    }

    /// R186 B24: buildAiDeck never deals a banned card, over 200 seeds at every tier's size and cap
    #[test]
    fn r186_b24_build_ai_deck_never_deals_a_banned_card_over_200_seeds_at_every_tiers_size_and_cap() {
        jackioh_cards::register_all();
        let banned: IndexSet<String> = banned_ids().into_iter().collect();
        for difficulty in [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard] {
            let h = AI_DIFFICULTY[difficulty];
            for n in 1..=200 {
                let deck = build_ai_deck(
                    &mut create_rng(&format!("shadow-ban:{difficulty}:{n}"), 0),
                    h.deck_size,
                    &AiDeckOptions {
                        mana_cap: Some(h.mana_cap),
                        ..Default::default()
                    },
                );
                for id in &deck {
                    assert!(!banned.contains(id), "{difficulty} seed {n}: {id}");
                }
            }
        }
    }

    /// R186 B24: a banned card stays legal for a human's deck
    #[test]
    fn r186_b24_a_banned_card_stays_legal_for_a_humans_deck() {
        jackioh_cards::register_all();
        let banned = banned_ids();
        let others: Vec<String> = ai_pool().into_iter().filter(|id| !banned.contains(id)).collect();
        let size = DECK_SIZE as usize;
        let deck: Vec<String> = banned
            .iter()
            .take(size)
            .cloned()
            .chain(others)
            .take(size)
            .collect();
        let opponent = build_ai_deck(
            &mut create_rng("shadow-ban-legal", 0),
            DECK_SIZE,
            &AiDeckOptions::default(),
        );
        assert_eq!(deck.iter().collect::<IndexSet<_>>().len(), size);
        // `create_game` panics where TS's `createGame` threw (SURFACE §6.1); not panicking is TS's `not.toThrow()`.
        let _ = create_game(&CreateGameArgs {
            seed: "shadow-ban-legal".to_string(),
            decks: (deck, opponent),
            ..Default::default()
        });
    }
}

// ---------------------------------------------------------------------------------------------
// B25
// ---------------------------------------------------------------------------------------------

/// TS's `stats(overrides)`: a clean `SweepStats` over seedsPerCard games, as JSON.
fn stats(overrides: Value) -> Value {
    spread(
        json!({
            "defId": "core-011",
            "games": AI_SWEEP.seeds_per_card,
            "drawnGames": AI_SWEEP.seeds_per_card,
            "affordableTurns": 0,
            "plays": 0,
            "errors": 0,
            "timeouts": 0,
            "evalDeltaSum": 0,
            "evalDeltaCount": 0,
        }),
        Some(&overrides),
    )
}

/// `sweepFlags` of a stats literal, as its JSON (`["error", …]`).
fn flags_of(stats: &Value) -> Value {
    js(sweep_flags(&json_as::<SweepStats>(stats.clone())))
}

/// `halfFlags` of a stats literal, as its JSON.
fn half_of(stats: &Value) -> Value {
    js(half_flags(&json_as::<SweepStats>(stats.clone())))
}

mod sweep_flags_b25 {
    use super::*;

    /// B25: clean stats raise no flag
    #[test]
    fn b25_clean_stats_raise_no_flag() {
        assert_eq!(flags_of(&stats(json!({}))), json!([]));
        assert_eq!(
            flags_of(&stats(
                json!({ "affordableTurns": 10, "plays": 3, "evalDeltaSum": 30, "evalDeltaCount": 3 })
            )),
            json!([])
        );
    }

    /// B25: error is raised by the first error and not before
    #[test]
    fn b25_error_is_raised_by_the_first_error_and_not_before() {
        assert!(
            !flags_of(&stats(json!({ "errors": 0 })))
                .as_array()
                .is_some_and(|f| f.contains(&json!("error")))
        );
        assert_eq!(flags_of(&stats(json!({ "errors": 1 }))), json!(["error"]));
    }

    /// B25: timeout is raised by the first timeout and not before
    #[test]
    fn b25_timeout_is_raised_by_the_first_timeout_and_not_before() {
        assert!(
            !flags_of(&stats(json!({ "timeouts": 0 })))
                .as_array()
                .is_some_and(|f| f.contains(&json!("timeout")))
        );
        assert_eq!(flags_of(&stats(json!({ "timeouts": 1 }))), json!(["timeout"]));
    }

    /// B25: neverPlayed needs minAffordableTurns affordable turns and no play
    #[test]
    fn b25_never_played_needs_min_affordable_turns_affordable_turns_and_no_play() {
        let at = AI_SWEEP.min_affordable_turns;
        assert_eq!(
            flags_of(&stats(json!({ "affordableTurns": at, "plays": 0 }))),
            json!(["neverPlayed"])
        );
        assert_eq!(
            flags_of(&stats(json!({ "affordableTurns": at - 1, "plays": 0 }))),
            json!([])
        );
        assert_eq!(
            flags_of(&stats(
                json!({ "affordableTurns": at, "plays": 1, "evalDeltaSum": 0, "evalDeltaCount": 1 })
            )),
            json!([])
        );
    }

    /// B25: selfHarm needs a mean evaluation delta strictly below selfHarmDelta over at least minHarmPlays plays
    #[test]
    fn b25_self_harm_needs_a_mean_evaluation_delta_strictly_below_self_harm_delta_over_at_least_min_harm_plays_plays()
     {
        // Counts stay integers (the fields are whole numbers); sums are the floats TS's arithmetic makes.
        let bound = AI_SWEEP.self_harm_delta;
        let plays = AI_SWEEP.min_harm_plays as i64;
        let base = |extra: Value| {
            stats(spread(
                json!({ "affordableTurns": 5, "plays": plays }),
                Some(&extra),
            ))
        };
        assert_eq!(
            flags_of(&base(
                json!({ "evalDeltaSum": bound * plays as f64, "evalDeltaCount": plays })
            )),
            json!([])
        );
        assert_eq!(
            flags_of(&base(
                json!({ "evalDeltaSum": (bound - 1.0) * plays as f64, "evalDeltaCount": plays })
            )),
            json!(["selfHarm"])
        );
        // One play fewer than minHarmPlays is not enough, however bad the plays were.
        let fewer = plays - 1;
        assert_eq!(
            flags_of(&base(
                json!({ "plays": fewer, "evalDeltaSum": (bound - 100.0) * fewer as f64, "evalDeltaCount": fewer })
            )),
            json!([])
        );
        assert_eq!(
            flags_of(&base(
                json!({ "evalDeltaSum": bound * 10.0, "evalDeltaCount": 0 })
            )),
            json!([])
        );
    }

    /// B25: several flags come out in the order error, timeout, neverPlayed, selfHarm
    #[test]
    fn b25_several_flags_come_out_in_the_order_error_timeout_never_played_self_harm() {
        let all = stats(json!({
            "errors": 2,
            "timeouts": 1,
            "affordableTurns": AI_SWEEP.min_affordable_turns,
            "plays": 0,
            "evalDeltaSum": (AI_SWEEP.self_harm_delta - 10.0) * AI_SWEEP.min_harm_plays as f64,
            "evalDeltaCount": AI_SWEEP.min_harm_plays,
        }));
        assert_eq!(
            flags_of(&all),
            json!(["error", "timeout", "neverPlayed", "selfHarm"])
        );
    }

    /// B25: sweepCard on Tempo Timmy over 2 seeds plays 2 games and flags no error
    #[test]
    fn b25_sweep_card_on_tempo_timmy_over_2_seeds_plays_2_games_and_flags_no_error() {
        jackioh_cards::register_all();
        let result = js(sweep_card(
            "core-011",
            &SweepOptions {
                seeds: Some(2),
                now: None,
                tier: None,
            },
        ));
        assert_eq!(result["defId"], json!("core-011"));
        assert_eq!(result["games"], json!(2));
        assert!(
            !result["flags"]
                .as_array()
                .is_some_and(|f| f.contains(&json!("error")))
        );
        assert_eq!(result["errors"], json!(0));
        assert!(result["drawnGames"].as_i64() <= result["games"].as_i64());
        let mut rest = result.clone();
        let flags = rest
            .as_object_mut()
            .and_then(|map| map.remove("flags"))
            .unwrap_or(Value::Null);
        assert_eq!(flags_of(&rest), flags);
    }
}

// ---------------------------------------------------------------------------------------------
// every tier: a ban holds at every difficulty, so a card is judged at the fewest resources and
// the most, and a card no tier could afford is reported rather than passed
// ---------------------------------------------------------------------------------------------

/// TS's `result(overrides)`: a pass-1 `SweepResult` at Easy, its flags and `unswept` from its stats.
fn result(overrides: Value) -> Value {
    let base = stats(overrides.clone());
    let unswept = base["affordableTurns"].as_f64() == Some(0.0);
    let derived = json!({ "tier": "easy", "flags": flags_of(&base), "unswept": unswept });
    spread(spread(base, Some(&derived)), Some(&overrides))
}

/// A pass-2 result of `forced` at `tier` (fixture numbers, no game played): `cards[0]` is the forced
/// card's entry unless it names another defId; each entry defaults to seedsPerCardAtRisk games.
fn pass2_of(forced: &str, tier: &str, cards: Vec<Value>, suspects: Value) -> Value {
    let cards: Vec<Value> = cards
        .into_iter()
        .map(|card| {
            stats(spread(
                json!({ "defId": forced, "games": AI_SWEEP.seeds_per_card_at_risk, "drawnGames": 0 }),
                Some(&card),
            ))
        })
        .collect();
    json!({ "forced": forced, "tier": tier, "games": AI_SWEEP.seeds_per_card_at_risk, "cards": cards, "suspects": suspects })
}

fn results(values: &[Value]) -> Vec<SweepResult> {
    values.iter().map(|value| json_as(value.clone())).collect()
}

fn passes(values: &[Value]) -> Vec<SweepPass2> {
    values.iter().map(|value| json_as(value.clone())).collect()
}

/// `sweepVerdict(pass1, pass2)`, as its JSON.
fn verdict(pass1: &[Value], pass2: &[Value]) -> Value {
    js(sweep_verdict(&results(pass1), &passes(pass2)))
}

fn reason_of(verdict: &Value) -> Option<String> {
    verdict["reason"].as_str().map(String::from)
}

mod the_sweep_judges_a_card_at_every_tier_r186 {
    use super::*;

    /// R186 AI_SWEEP sweeps at Easy and at Hard
    #[test]
    fn r186_ai_sweep_sweeps_at_easy_and_at_hard() {
        let tiers: Vec<Difficulty> = AI_SWEEP.tiers.to_vec();
        assert_eq!(tiers, vec![Difficulty::Easy, Difficulty::Hard]);
    }

    /// R186 a flag at any tier bans the card, and the reason names every tier that flagged it
    #[test]
    fn r186_a_flag_at_any_tier_bans_the_card_and_the_reason_names_every_tier_that_flagged_it() {
        let at = AI_SWEEP.ban_affordable_turns;
        let easy = result(json!({ "defId": "core-078", "tier": "easy", "affordableTurns": 4, "plays": 0 }));
        let hard = result(json!({ "defId": "core-078", "tier": "hard", "affordableTurns": 5, "plays": 0 }));
        let judged = verdict(
            &[easy, hard],
            &[
                pass2_of(
                    "core-078",
                    "easy",
                    vec![json!({ "affordableTurns": at + 2 })],
                    json!([]),
                ),
                pass2_of(
                    "core-078",
                    "hard",
                    vec![json!({ "affordableTurns": at + 5 })],
                    json!([]),
                ),
            ],
        );
        assert_eq!(judged["flags"], json!(["neverPlayed"]));
        assert_eq!(judged["unswept"], json!(false));
        let reason = reason_of(&judged).unwrap_or_default();
        assert!(matches_reason(&reason), "{reason}");
        assert!(reason.contains(&format!(
            "easy: affordable in hand on {} turns over 24 pass-2 games, never played",
            at + 2
        )));
        assert!(reason.contains(&format!(
            "hard: affordable in hand on {} turns over 24 pass-2 games, never played",
            at + 5
        )));

        let only_hard = verdict(
            &[
                result(
                    json!({ "defId": "core-078", "tier": "easy", "affordableTurns": 6, "plays": 2, "evalDeltaSum": 4, "evalDeltaCount": 2 }),
                ),
                result(
                    json!({ "defId": "core-078", "tier": "hard", "errors": 1, "affordableTurns": 4, "plays": 1, "evalDeltaCount": 1 }),
                ),
            ],
            &[],
        );
        assert_eq!(only_hard["flags"], json!(["error"]));
        let reason = reason_of(&only_hard).unwrap_or_default();
        assert!(
            reason.starts_with("error: hard: 1 engine or search error"),
            "{reason}"
        );
        assert!(!reason.contains("easy:"));
    }

    /// R186 a card clean at every tier has no reason; one never affordable anywhere is unswept, not clean
    #[test]
    fn r186_a_card_clean_at_every_tier_has_no_reason_one_never_affordable_anywhere_is_unswept_not_clean() {
        let clean = verdict(
            &[
                result(
                    json!({ "tier": "easy", "affordableTurns": 5, "plays": 3, "evalDeltaSum": 9, "evalDeltaCount": 3 }),
                ),
                result(
                    json!({ "tier": "hard", "affordableTurns": 7, "plays": 4, "evalDeltaSum": 8, "evalDeltaCount": 4 }),
                ),
            ],
            &[],
        );
        assert_match_object(&clean, &json!({ "flags": [], "unswept": false, "reason": null }));

        let never_affordable = verdict(
            &[
                result(json!({ "tier": "easy" })),
                result(json!({ "tier": "hard" })),
            ],
            &[],
        );
        assert_match_object(
            &never_affordable,
            &json!({ "flags": [], "unswept": true, "reason": null }),
        );

        let affordable_only_at_hard = verdict(
            &[
                result(json!({ "tier": "easy" })),
                result(json!({ "tier": "hard", "affordableTurns": 4, "plays": 2, "evalDeltaCount": 2 })),
            ],
            &[],
        );
        assert_eq!(affordable_only_at_hard["unswept"], json!(false));
    }

    // One test per tier, each with its own 300 s. The two shared one test and ran 267 s on a developer
    // sandbox and 313 s on the night bot's, past its limit; the time is all Easy's (251 s of the 267
    // for two seeds, Hard's two take 12 s). Whether a card is affordable at four crystals follows from
    // its cost, never from the seed, so Easy's check plays one game.

    /// R186 GIGA Glowy Jelly Bean (6 mana) is unswept at Easy's four crystals
    #[test]
    fn r186_giga_glowy_jelly_bean_6_mana_is_unswept_at_easys_four_crystals() {
        jackioh_cards::register_all();
        let easy = js(sweep_card(
            "core-029",
            &SweepOptions {
                seeds: Some(1),
                now: None,
                tier: Some(Difficulty::Easy),
            },
        ));
        assert_eq!(easy["tier"], json!("easy"));
        assert_eq!(easy["affordableTurns"], json!(0));
        assert_eq!(easy["unswept"], json!(true));
        assert_eq!(easy["flags"], json!([]));
    }

    /// R186 GIGA Glowy Jelly Bean (6 mana) is judged at Hard's seven crystals
    #[test]
    fn r186_giga_glowy_jelly_bean_6_mana_is_judged_at_hards_seven_crystals() {
        jackioh_cards::register_all();
        let hard = js(sweep_card(
            "core-029",
            &SweepOptions {
                seeds: Some(2),
                now: None,
                tier: Some(Difficulty::Hard),
            },
        ));
        assert_eq!(hard["tier"], json!("hard"));
        assert_eq!(hard["errors"], json!(0));
        assert!(hard["affordableTurns"].as_i64().unwrap_or(0) > 0);
        assert_eq!(hard["unswept"], json!(false));
    }
}

// ---------------------------------------------------------------------------------------------
// R390: two passes. Pass 1 finds the cards at risk, pass 2 sweeps them again with more games and
// deals them more often as filler, and a judgement ban needs pass 2's numbers. Fixture results
// throughout, so no rule here waits on a whole sweep; one real pass-2 game at the end.
// ---------------------------------------------------------------------------------------------

mod the_two_pass_sweep_r390 {
    use super::*;

    /// R390 pass 2's numbers: 24 games per at-risk card and tier, at-risk filler ×4, bans at 6 affordable turns and 8 plays
    #[test]
    fn r390_pass_2s_numbers_24_games_per_at_risk_card_and_tier_at_risk_filler_x4_bans_at_6_affordable_turns_and_8_plays()
     {
        assert_eq!(AI_SWEEP.seeds_per_card_at_risk as i64, 24);
        assert_eq!(AI_SWEEP.at_risk_boost as f64, 4.0);
        assert_eq!(
            AI_SWEEP.ban_affordable_turns as i64,
            2 * AI_SWEEP.min_affordable_turns as i64
        );
        assert_eq!(AI_SWEEP.ban_harm_plays as i64, 2 * AI_SWEEP.min_harm_plays as i64);
    }

    /// R390 at risk is a flag at half strength: affordable on minAffordableTurns turns and played at most once, or a mean below half of selfHarmDelta
    #[test]
    fn r390_at_risk_is_a_flag_at_half_strength_affordable_on_min_affordable_turns_turns_and_played_at_most_once_or_a_mean_below_half_of_self_harm_delta()
     {
        let at = AI_SWEEP.min_affordable_turns;
        assert_eq!(
            half_of(&stats(json!({ "affordableTurns": at, "plays": 0 }))),
            json!(["neverPlayed"])
        );
        assert_eq!(
            half_of(&stats(
                json!({ "affordableTurns": at, "plays": 1, "evalDeltaCount": 1 })
            )),
            json!(["neverPlayed"])
        );
        assert_eq!(
            half_of(&stats(
                json!({ "affordableTurns": at, "plays": 2, "evalDeltaCount": 2 })
            )),
            json!([])
        );
        assert_eq!(
            half_of(&stats(json!({ "affordableTurns": at - 1, "plays": 0 }))),
            json!([])
        );
        let half = AI_SWEEP.self_harm_delta / 2.0;
        assert_eq!(
            half_of(&stats(
                json!({ "affordableTurns": 9, "plays": 2, "evalDeltaSum": (half - 1.0) * 2.0, "evalDeltaCount": 2 })
            )),
            json!(["selfHarm"])
        );
        assert_eq!(
            half_of(&stats(
                json!({ "affordableTurns": 9, "plays": 2, "evalDeltaSum": half * 2.0, "evalDeltaCount": 2 })
            )),
            json!([])
        );
        // One play is enough to put a card at risk (to ban it takes banHarmPlays).
        assert_eq!(
            half_of(&stats(
                json!({ "affordableTurns": 9, "plays": 1, "evalDeltaSum": half - 1.0, "evalDeltaCount": 1 })
            )),
            json!(["neverPlayed", "selfHarm"])
        );
    }

    /// R390 the at-risk list is a pure function of pass 1's results, the ban and the watch list, sorted
    #[test]
    fn r390_the_at_risk_list_is_a_pure_function_of_pass_1s_results_the_ban_and_the_watch_list_sorted() {
        let pass1 = vec![
            result(
                json!({ "defId": "core-030", "tier": "easy", "affordableTurns": 5, "plays": 1, "evalDeltaCount": 1 }),
            ),
            result(
                json!({ "defId": "core-030", "tier": "hard", "affordableTurns": 9, "plays": 6, "evalDeltaCount": 6 }),
            ),
            result(
                json!({ "defId": "core-011", "tier": "easy", "affordableTurns": 9, "plays": 6, "evalDeltaCount": 6 }),
            ),
            result(
                json!({ "defId": "core-012", "tier": "hard", "affordableTurns": 9, "plays": 3, "evalDeltaSum": -90, "evalDeltaCount": 3 }),
            ),
        ];
        let ban: &[(&str, &str)] = &[(
            "core-099",
            "neverPlayed: easy: affordable in hand on 21 turns, never played",
        )];
        let watch: &[(&str, &str)] = &[(
            "classic-020",
            "at risk: easy: pass 1 affordable on 4 turns over 8 games, played 1 time(s)",
        )];
        assert_eq!(
            at_risk_ids(&results(&pass1), ban, watch),
            vec!["classic-020", "core-012", "core-030", "core-099"]
        );
        let mut reversed = pass1.clone();
        reversed.reverse();
        assert_eq!(
            at_risk_ids(&results(&reversed), ban, watch),
            at_risk_ids(&results(&pass1), ban, watch)
        );
        assert_eq!(at_risk_ids(&[], &[], &[]), Vec::<String>::new());
        // By default today's tables count: every banned or watched card is at risk from the start.
        let at_risk = at_risk_ids(&[], SHADOW_BAN, SHADOW_WATCH);
        for id in banned_ids().into_iter().chain(ids_of(SHADOW_WATCH)) {
            assert!(at_risk.contains(&id), "{id}");
        }
    }

    /// R390 pass 2's filler keeps out the cards banned for error or timeout and lifts the ban for neverPlayed and selfHarm
    #[test]
    fn r390_pass_2s_filler_keeps_out_the_cards_banned_for_error_or_timeout_and_lifts_the_ban_for_never_played_and_self_harm()
     {
        let ban: &[(&str, &str)] = &[
            (
                "core-042",
                "neverPlayed: hard: affordable in hand on 21 turns, never played",
            ),
            (
                "core-051",
                "error, neverPlayed: easy: 1 engine or search error(s) over 8 games, affordable in hand on 4 turns, never played",
            ),
            (
                "core-055",
                "timeout: hard: 1 decision(s) over 2000 ms or game(s) past 600 actions",
            ),
            (
                "core-057",
                "selfHarm: easy: mean evaluate change -60.0 over 9 play(s) over 24 pass-2 games",
            ),
        ];
        let core_051 = ban
            .iter()
            .find(|(id, _)| *id == "core-051")
            .map(|(_, reason)| *reason)
            .unwrap_or("");
        assert_eq!(js(ban_flags(core_051)), json!(["error", "neverPlayed"]));
        let pass1 = results(&[
            result(json!({ "defId": "classic-003", "errors": 2 })),
            result(json!({ "defId": "classic-004", "timeouts": 1 })),
            result(json!({ "defId": "classic-005", "affordableTurns": 5 })),
        ]);
        assert_eq!(
            pass2_keep_out(&pass1, ban),
            vec!["classic-003", "classic-004", "core-051", "core-055"]
        );
    }

    /// R390 pass 1 alone never bans for neverPlayed or selfHarm, however strong its numbers; error and timeout ban as before
    #[test]
    fn r390_pass_1_alone_never_bans_for_never_played_or_self_harm_however_strong_its_numbers_error_and_timeout_ban_as_before()
     {
        let strong = result(json!({
            "defId": "core-078",
            "affordableTurns": 30,
            "plays": 0,
            "evalDeltaSum": AI_SWEEP.self_harm_delta * 100.0,
            "evalDeltaCount": 10,
        }));
        assert_eq!(strong["flags"], json!(["neverPlayed", "selfHarm"]));
        assert_match_object(&verdict(&[strong], &[]), &json!({ "flags": [], "reason": null }));
        assert_eq!(
            verdict(&[result(json!({ "defId": "core-078", "errors": 1 }))], &[])["flags"],
            json!(["error"])
        );
        assert_eq!(
            verdict(&[result(json!({ "defId": "core-078", "timeouts": 1 }))], &[])["flags"],
            json!(["timeout"])
        );
    }

    /// R390 R601 neverPlayed needs 6 affordable turns and no play over pass 2's games at that tier, forced and filler games summed
    #[test]
    fn r390_r601_never_played_needs_6_affordable_turns_and_no_play_over_pass_2s_games_at_that_tier_forced_and_filler_games_summed()
     {
        let at = AI_SWEEP.ban_affordable_turns;
        let p1 = vec![result(json!({ "defId": "core-078", "affordableTurns": 4 }))];
        let reason = |pass2: Vec<Value>| -> Option<String> { reason_of(&verdict(&p1, &pass2)) };
        assert!(
            reason(vec![pass2_of(
                "core-078",
                "easy",
                vec![json!({ "affordableTurns": at })],
                json!([])
            )])
            .is_some_and(|text| text.starts_with("neverPlayed: easy: "))
        );
        assert_eq!(
            reason(vec![pass2_of(
                "core-078",
                "easy",
                vec![json!({ "affordableTurns": at - 1 })],
                json!([])
            )]),
            None
        );
        assert_eq!(
            reason(vec![pass2_of(
                "core-078",
                "easy",
                vec![json!({ "affordableTurns": at + 9, "plays": 1, "evalDeltaCount": 1 })],
                json!([])
            )]),
            None
        );
        // Its own pass 2 saw it affordable on 3 turns; another card's pass 2 dealt it as filler for 3 more.
        let own = pass2_of(
            "core-078",
            "easy",
            vec![json!({ "affordableTurns": at / 2 })],
            json!([]),
        );
        let filler = pass2_of(
            "core-030",
            "easy",
            vec![
                json!({ "defId": "core-030", "affordableTurns": 9, "plays": 4, "evalDeltaCount": 4 }),
                json!({ "defId": "core-078", "games": 5, "affordableTurns": at / 2 }),
            ],
            json!([]),
        );
        assert_eq!(reason(vec![own.clone()]), None);
        assert_eq!(
            reason(vec![own.clone(), filler.clone()]),
            Some(format!(
                "neverPlayed: easy: affordable in hand on {at} turns over 29 pass-2 games, never played"
            ))
        );
        // A play in any pass-2 game at that tier clears it, a forced one or a filler one.
        let played = pass2_of(
            "core-030",
            "easy",
            vec![
                json!({ "defId": "core-030" }),
                json!({ "defId": "core-078", "games": 1, "plays": 1, "evalDeltaCount": 1 }),
            ],
            json!([]),
        );
        assert_eq!(reason(vec![own, filler, played]), None);
        // Pass 2 at the other tier is no evidence for this one.
        assert_eq!(
            reason(vec![
                pass2_of(
                    "core-078",
                    "hard",
                    vec![json!({ "affordableTurns": at - 1 })],
                    json!([])
                ),
                pass2_of(
                    "core-078",
                    "easy",
                    vec![json!({ "affordableTurns": at - 1 })],
                    json!([])
                ),
            ]),
            None
        );
    }

    /// R390 R601 selfHarm needs 8 plays over pass 2's games, averaging below selfHarmDelta
    #[test]
    fn r390_r601_self_harm_needs_8_plays_over_pass_2s_games_averaging_below_self_harm_delta() {
        let plays = AI_SWEEP.ban_harm_plays as i64;
        let bound = AI_SWEEP.self_harm_delta;
        let p1 = vec![result(json!({
            "defId": "core-078",
            "affordableTurns": 4,
            "plays": 2,
            "evalDeltaSum": bound * 2.0,
            "evalDeltaCount": 2,
        }))];
        let harm = |count: i64, each: f64| -> Option<String> {
            reason_of(&verdict(
                &p1,
                &[pass2_of(
                    "core-078",
                    "hard",
                    vec![
                        json!({ "affordableTurns": 20, "plays": count, "evalDeltaSum": each * count as f64, "evalDeltaCount": count }),
                    ],
                    json!([]),
                )],
            ))
        };
        assert_eq!(
            harm(plays, bound - 1.0),
            Some(
                "selfHarm: hard: mean evaluate change -41.0 over 8 play(s) over 24 pass-2 games".to_string()
            )
        );
        assert_eq!(harm(plays - 1, bound - 100.0), None);
        assert_eq!(harm(plays, bound), None);
    }

    /// R390 an error or timeout bans only its game's forced card; at-risk filler of that game is a suspect, banned only if its own games repeat it
    #[test]
    fn r390_an_error_or_timeout_bans_only_its_games_forced_card_at_risk_filler_of_that_game_is_a_suspect_banned_only_if_its_own_games_repeat_it()
     {
        let seed = "sweep2:easy:core-030:7";
        let suspect =
            json!({ "defId": "core-078", "seed": seed, "forced": "core-030", "errors": 1, "timeouts": 0 });
        let forced = pass2_of(
            "core-030",
            "easy",
            vec![
                json!({ "errors": 1, "affordableTurns": 9, "plays": 5, "evalDeltaCount": 5 }),
                json!({ "defId": "core-078", "games": 3, "affordableTurns": 2, "plays": 1, "evalDeltaCount": 1 }),
            ],
            json!([suspect]),
        );
        assert_eq!(
            verdict(
                &[result(
                    json!({ "defId": "core-030", "affordableTurns": 4, "plays": 2, "evalDeltaCount": 2 })
                )],
                std::slice::from_ref(&forced)
            )["flags"],
            json!(["error"])
        );
        let filler_own = pass2_of(
            "core-078",
            "easy",
            vec![json!({ "affordableTurns": 9, "plays": 4, "evalDeltaCount": 4 })],
            json!([]),
        );
        let p1 = vec![result(
            json!({ "defId": "core-078", "affordableTurns": 4, "plays": 1, "evalDeltaCount": 1 }),
        )];
        assert_eq!(verdict(&p1, &[forced.clone(), filler_own])["flags"], json!([]));
        let repeated = pass2_of(
            "core-078",
            "easy",
            vec![json!({ "errors": 1, "affordableTurns": 9, "plays": 4, "evalDeltaCount": 4 })],
            json!([]),
        );
        assert_match_object(
            &verdict(&p1, &[forced.clone(), repeated.clone()]),
            &json!({ "flags": ["error"] }),
        );
        assert_eq!(
            reason_of(&verdict(&p1, &[forced, repeated])),
            Some("error: easy: 1 engine or search error(s) over 35 games".to_string())
        );
    }

    /// R390 R600 the watch list holds the cards at risk by their own numbers that were not banned, with those numbers
    #[test]
    fn r390_r600_the_watch_list_holds_the_cards_at_risk_by_their_own_numbers_that_were_not_banned_with_those_numbers()
     {
        // At risk by pass 1, cleared in pass 2: watched, its numbers named.
        let p1 = vec![result(
            json!({ "defId": "core-078", "affordableTurns": 4, "plays": 1, "evalDeltaCount": 1 }),
        )];
        let cleared = verdict(
            &p1,
            &[pass2_of(
                "core-078",
                "easy",
                vec![json!({ "affordableTurns": 20, "plays": 7, "evalDeltaCount": 7 })],
                json!([]),
            )],
        );
        assert_eq!(cleared["reason"], Value::Null);
        assert_eq!(
            cleared["watch"],
            json!(
                "at risk: easy: pass 1 affordable on 4 turns over 8 games, played 1 time(s), mean evaluate change 0.0"
            )
        );
        // At risk only because it was banned or watched before, and clean in both passes now: off the list.
        let clean = verdict(
            &[result(
                json!({ "defId": "core-099", "affordableTurns": 6, "plays": 4, "evalDeltaCount": 4 }),
            )],
            &[pass2_of(
                "core-099",
                "easy",
                vec![json!({ "affordableTurns": 20, "plays": 9, "evalDeltaCount": 9 })],
                json!([]),
            )],
        );
        assert_match_object(&clean, &json!({ "reason": null, "watch": null }));
        // At risk by pass 2's own numbers: watched.
        let second = verdict(
            &[result(
                json!({ "defId": "core-099", "affordableTurns": 6, "plays": 4, "evalDeltaCount": 4 }),
            )],
            &[pass2_of(
                "core-099",
                "easy",
                vec![json!({ "affordableTurns": 5, "plays": 1, "evalDeltaCount": 1 })],
                json!([]),
            )],
        );
        let watch = second["watch"].as_str().unwrap_or_default();
        assert!(
            watch.starts_with("at risk: easy: pass 2 affordable on 5 turns over 24 games, played 1 time(s)"),
            "{watch}"
        );
        // Banned: never watched.
        assert_match_object(
            &verdict(
                &p1,
                &[pass2_of(
                    "core-078",
                    "easy",
                    vec![json!({ "affordableTurns": 9 })],
                    json!([]),
                )],
            ),
            &json!({ "flags": ["neverPlayed"], "watch": null }),
        );
    }

    /// R390 a pass-2 result comes through JSON whole, so slices join in `--report`
    #[test]
    fn r390_a_pass_2_result_comes_through_json_whole_so_slices_join_in_report() {
        let fixture = pass2_of(
            "core-078",
            "easy",
            vec![json!({ "affordableTurns": 3 })],
            json!([{ "defId": "core-030", "seed": "sweep2:easy:core-078:1", "forced": "core-078", "errors": 0, "timeouts": 1 }]),
        );
        // TS's `JSON.parse(JSON.stringify(fixture))`, through the Rust type the report reads it into.
        let typed: SweepPass2 = json_as(fixture.clone());
        let text = serde_json::to_string(&typed).expect("serialisable");
        let back: Value = serde_json::from_str(&text).expect("parses");
        assert_eq!(normalized(back), normalized(fixture));
    }

    /// R390 a real pass-2 game: named seed, boosted at-risk filler, the ban lifted for judgement bans and kept for bugs, timeouts charged to the forced card and listed against the filler
    #[test]
    fn r390_a_real_pass_2_game_named_seed_boosted_at_risk_filler_the_ban_lifted_for_judgement_bans_and_kept_for_bugs_timeouts_charged_to_the_forced_card_and_listed_against_the_filler()
     {
        // Every card on today's ban is at risk and lifted; ten more at-risk cards are kept out as if a
        // pass 1 had flagged them `error`. A fake clock makes every AI decision 2.5 s long.
        jackioh_cards::register_all();
        let banned = banned_ids();
        let keep_out: Vec<String> = ai_pool()
            .into_iter()
            .filter(|id| !banned.contains(id) && id != "core-012")
            .take(10)
            .collect();
        let mut at_risk: Vec<String> = banned
            .iter()
            .cloned()
            .chain(keep_out.iter().cloned())
            .chain(["core-012".to_string()])
            .collect();
        at_risk.sort();
        let clock = Cell::new(0.0_f64);
        let now = || {
            clock.set(clock.get() + 2500.0);
            clock.get()
        };
        let timed = js(sweep_at_risk(
            "core-012",
            &at_risk,
            &keep_out,
            &SweepOptions {
                seeds: Some(1),
                now: Some(&now),
                tier: Some(Difficulty::Easy),
            },
        ));
        assert_eq!(timed["forced"], json!("core-012"));
        assert_eq!(timed["games"], json!(1));
        let cards = timed["cards"].as_array().cloned().unwrap_or_default();
        let ids: Vec<String> = cards
            .iter()
            .map(|card| card["defId"].as_str().unwrap_or_default().to_string())
            .collect();
        assert_eq!(ids.first().map(String::as_str), Some("core-012"));
        for id in &ids {
            assert!(at_risk.contains(id), "{id}");
        }
        for id in &keep_out {
            assert!(!ids.contains(id), "{id}");
        }
        assert!(
            ids.iter().any(|id| banned.contains(id)),
            "dealt: {}",
            ids.join(", ")
        );

        let forced = cards.first().cloned().unwrap_or(Value::Null);
        let filler: Vec<Value> = cards.iter().skip(1).cloned().collect();
        assert!(forced["timeouts"].as_i64().unwrap_or(0) > 0);
        for card in &filler {
            assert_match_object(card, &json!({ "games": 1, "errors": 0, "timeouts": 0 }));
        }
        let suspects = timed["suspects"].as_array().cloned().unwrap_or_default();
        let suspect_ids: Vec<Value> = suspects.iter().map(|entry| entry["defId"].clone()).collect();
        let filler_ids: Vec<Value> = filler.iter().map(|card| card["defId"].clone()).collect();
        assert_eq!(suspect_ids, filler_ids);
        for entry in &suspects {
            assert_match_object(
                entry,
                &json!({ "seed": "sweep2:easy:core-012:1", "forced": "core-012", "errors": 0, "timeouts": forced["timeouts"] }),
            );
        }

        // The same game without a clock: the clock only measures, so the deal and the play are the same.
        let plain = js(sweep_at_risk(
            "core-012",
            &at_risk,
            &keep_out,
            &SweepOptions {
                seeds: Some(1),
                now: None,
                tier: Some(Difficulty::Easy),
            },
        ));
        assert_eq!(plain["suspects"], json!([]));
        let untimed = |cards: &Value| -> Vec<Value> {
            cards
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|card| spread(card, Some(&json!({ "timeouts": 0 }))))
                .collect()
        };
        assert_eq!(untimed(&plain["cards"]), untimed(&timed["cards"]));
    }
}

// ---------------------------------------------------------------------------------------------
// V12 (docs/v0.3.0/README.md §6): the table at cutover
// ---------------------------------------------------------------------------------------------

/// `crates/ai/generation.json`: the AI generation this tree holds (training/README.md).
const GENERATION_JSON: &str = include_str!("../../generation.json");

/// `packages/ai/src/shadowBan.ts`'s `SHADOW_BAN` at 91cc43c, the source `generation.json` names for
/// generation 0: TypeScript's eleven entries, in its order.
const TS_SHADOW_BAN: &[(&str, &str)] = &[
    (
        "core-042",
        "neverPlayed: hard: affordable in hand on 21 turns, never played",
    ),
    (
        "core-051",
        "neverPlayed: hard: affordable in hand on 20 turns, never played",
    ),
    (
        "core-055",
        "neverPlayed: hard: affordable in hand on 22 turns, never played",
    ),
    (
        "core-057",
        "neverPlayed: hard: affordable in hand on 4 turns, never played",
    ),
    (
        "core-076",
        "neverPlayed: hard: affordable in hand on 15 turns, never played",
    ),
    (
        "core-078",
        "neverPlayed: easy: affordable in hand on 31 turns, never played",
    ),
    (
        "core-082",
        "neverPlayed: hard: affordable in hand on 6 turns, never played",
    ),
    (
        "core-091",
        "neverPlayed: hard: affordable in hand on 19 turns, never played",
    ),
    (
        "core-093",
        "neverPlayed: hard: affordable in hand on 25 turns, never played",
    ),
    (
        "core-094",
        "neverPlayed: hard: affordable in hand on 13 turns, never played",
    ),
    (
        "core-099",
        "neverPlayed: easy: affordable in hand on 21 turns, never played; hard: affordable in hand on 13 turns, never played",
    ),
];

mod v12 {
    use super::*;

    /// The unban lane's generation-4 table: removals only from TypeScript's eleven — every retained
    /// entry is verbatim in it, and the seven it dropped are exactly the ones the sweep of record
    /// cleared or the eval now sees (`SHADOW_BAN`'s own doc comment).
    const UNBAN_LANE_SHADOW_BAN: &[(&str, &str)] = &[
        (
            "core-042",
            "neverPlayed: hard: affordable in hand on 21 turns, never played",
        ),
        (
            "core-055",
            "neverPlayed: hard: affordable in hand on 22 turns, never played",
        ),
        (
            "core-076",
            "neverPlayed: hard: affordable in hand on 15 turns, never played",
        ),
        (
            "core-078",
            "neverPlayed: easy: affordable in hand on 31 turns, never played",
        ),
    ];

    #[test]
    fn v12_the_unban_lanes_table_is_the_four_entries_it_left_of_typescripts_eleven() {
        assert_eq!(SHADOW_BAN, UNBAN_LANE_SHADOW_BAN);
        for entry in SHADOW_BAN {
            assert!(
                TS_SHADOW_BAN.contains(entry),
                "{} removed or rewritten — the lane removes only",
                entry.0
            );
        }
        let record: Value = serde_json::from_str(GENERATION_JSON).expect("generation.json is JSON");
        if record["generation"].as_i64() == Some(0) {
            assert_eq!(record["lane"].as_str(), Some("port"));
        }
    }
}
