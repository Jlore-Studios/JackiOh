//! C+ #53 Book of Tokens (SPEC §8.7 row 53). (1) Spell, Book, Epic.
//!   Base:    "Summon 1-2 Rush Tokens." — the count is random (balance patch 1)
//!   Radiant: "Lucky 1 / Summon 1-2 Radiant Rush Tokens."
//!   Engine:  "Rush Tokens (§7) placed per R64; a full board takes fewer. Tunes: none."
//!
//! One laneless `summon` per token: each takes the leftmost empty, unlocked, unreserved unit zone (R64)
//! and fizzles silently on a full row (§3.2), as Core #15's do. The token's stats are its own (§7).
//! The count is one draw from the match rng: 1 token `ONE_TOKEN_CHANCE` (60%) of the time and 2 the
//! rest, since #2657 (50/50 through v0.5.9f, `script_v0_5_9f`, kept, R767000), so the curve stays near a
//! single 1-cost token summon; a tuned count would be a param, and there is none. Lucky X (§6.1) rolls X more times and
//! keeps the most tokens: the Radiant face prints Lucky 1, and the Lucky read is the running card's
//! own, so a Degrade or an Upgrade of that number moves it (B3.4), as Two Grapes reads its Lucky.

use jackioh_engine::effects::summon;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-053";

const RUSH_TOKEN: &str = "core-t-rush";

/// The printed "1-2": the fewest and most tokens one cast summons.
const MIN_TOKENS: i32 = 1;
const MAX_TOKENS: i32 = 2;
/// #2657: the percent chance a roll comes to `MIN_TOKENS`, `MAX_TOKENS` the rest, 10 points over even.
const ONE_TOKEN_CHANCE: i32 = 60;

/// §6.1: the Lucky X the running card has now, printed plus given (R1438; its Radiant face prints
/// Lucky 1).
fn lucky_of(ctx: &EffectContext<'_>) -> i32 {
    ctx.live_self().map_or(0, |card| lucky_on(&*ctx.state, card))
}

/// One roll of the count: `ONE_TOKEN_CHANCE` percent `MIN_TOKENS`, else `MAX_TOKENS`; with `even`, as
/// v0.5.9f rolled it, one even pick of the two.
fn roll_of(rng: &mut Rng, even: bool) -> i32 {
    if even {
        MIN_TOKENS + rng.int(MAX_TOKENS - MIN_TOKENS + 1)
    } else if rng.int(PERCENT) < ONE_TOKEN_CHANCE {
        MIN_TOKENS
    } else {
        MAX_TOKENS
    }
}

fn book_of_tokens(radiant: bool, even: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let roll = |rng: &mut Rng| -> i32 { roll_of(rng, even) };
            // R987: the controller's Luck rolls extra times beside the card's own Lucky.
            let lucky = lucky_of(ctx) + luck_of(&*ctx.state, ctx.controller);
            let count = if lucky > 0 {
                ctx.rng.lucky(lucky, roll, |a, b| a.max(b))
            } else {
                roll(&mut *ctx.rng)
            };
            (0..count)
                .map(|_| summon(json_as(json!({ "defId": RUSH_TOKEN, "radiant": radiant }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: book_of_tokens(false, false),
        radiant: book_of_tokens(true, false),
    }
}

/// R767000: through v0.5.9f the count was an even pick of 1 and 2; `KEPT_SCRIPTS` folds those replays
/// under it.
pub fn script_v0_5_9f() -> CardScripts {
    CardScripts {
        base: book_of_tokens(false, true),
        radiant: book_of_tokens(true, true),
    }
}

// C+ #53 Book of Tokens — SPEC §8.7 row 53, BUILD M9 Classic+ row C+ 53.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const BOOK: &str = "classicplus-053";
    const RUSH: &str = "core-t-rush";
    const VANILLA: &str = "core-008";
    const FILLER: &str = "core-005";

    use crate::scenario;

    fn book(radiant: bool, field: Value, seed: Option<&str>) -> Scenario {
        let mut opts = json!({
            "p1": { "hand": [{ "def": BOOK, "radiant": radiant }, FILLER], "field": field },
            "p2": { "hand": [FILLER] },
        });
        if let Some(seed) = seed {
            opts["seed"] = json!(seed);
        }
        scenario(opts)
    }

    /// The Rush Tokens on p1's unit row: `(lane, radiant)`.
    fn tokens(s: &Scenario) -> Vec<(i32, bool)> {
        let mut out = Vec::new();
        for lane in 1..=5 {
            if let Some(unit) = s.unit(PlayerId::P1, lane)
                && unit.def_id == RUSH
            {
                out.push((lane, unit.radiant));
            }
        }
        out
    }

    fn kinds(s: &Scenario, card: &str) -> Vec<String> {
        s.stats(card)
            .keywords
            .iter()
            .map(|keyword| keyword.kind().as_str().to_string())
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn one_token_comes_up_60_percent_of_the_time_two_40() {
            crate::register_all();
            let mut ones = 0;
            for n in 0..400 {
                let mut s = book(false, json!([]), Some(&format!("book-of-tokens-odds-{n}")));
                s.play(BOOK, json!({}));
                if tokens(&s).len() == 1 {
                    ones += 1;
                }
            }
            // 240 expected of 400; the band is about four standard deviations wide.
            assert!((200..=280).contains(&ones), "{ones} of 400 summoned one");
        }

        #[test]
        fn r767000_the_script_kept_for_v0_5_9f_rolls_an_even_count() {
            crate::under_kept(BOOK, super::super::script_v0_5_9f(), || {
                let mut ones = 0;
                for n in 0..400 {
                    let mut s = book(false, json!([]), Some(&format!("book-of-tokens-even-{n}")));
                    s.play(BOOK, json!({}));
                    if tokens(&s).len() == 1 {
                        ones += 1;
                    }
                }
                assert!((160..=240).contains(&ones), "{ones} of 400 summoned one");
            });
        }

        #[test]
        fn r987_feng_shui_s_luck_adds_a_roll() {
            crate::register_all();
            let twos = |judge: bool| -> i32 {
                let mut found = 0;
                for n in 0..40 {
                    let mut p1 = json!({ "hand": [{ "def": BOOK }, FILLER] });
                    if judge {
                        p1["backrow"] = json!(["meditative-040"]);
                    }
                    let mut opts = json!({ "p1": p1, "p2": { "hand": [FILLER] } });
                    opts["seed"] = json!(format!("book-of-tokens-luck-{n}"));
                    let mut s = scenario(opts);
                    s.play(BOOK, json!({}));
                    if tokens(&s).len() == 2 {
                        found += 1;
                    }
                }
                found
            };
            // With Luck 1 it keeps the better of two rolls, so 2 comes up more often on the same seeds.
            assert!(twos(true) > twos(false));
        }

        #[test]
        fn r64_summons_1_2_rush_tokens_3_3_rush_into_your_leftmost_open_zones_the_count_random_but_never_above_the_curve()
         {
            let mut counts: IndexSet<usize> = IndexSet::new();
            for n in 0..20 {
                let mut s = book(
                    false,
                    json!([{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 3 }]),
                    Some(&format!("book-of-tokens-{n}")),
                );
                s.play(BOOK, json!({}));
                let found = tokens(&s);
                assert!(!found.is_empty());
                assert!(found.len() <= 2);
                let lanes: Vec<i32> = found.iter().map(|token| token.0).collect();
                assert_eq!(lanes, if found.len() == 1 { vec![2] } else { vec![2, 4] });
                assert!(found.iter().all(|token| !token.1));
                let first = s
                    .unit(PlayerId::P1, found.first().map_or(0, |token| token.0))
                    .map(|card| card.id)
                    .unwrap_or_default();
                s.expect_stats(&first, json!({ "attack": 3, "health": 3 }));
                assert_eq!(kinds(&s, &first), vec!["Rush".to_string()]);
                counts.insert(found.len());
            }
            let mut both: Vec<usize> = counts.into_iter().collect();
            both.sort();
            assert_eq!(both, vec![1, 2]);
        }

        #[test]
        fn s3_2_one_on_a_nearly_full_board() {
            let mut s = book(false, json!([VANILLA, VANILLA, VANILLA, VANILLA]), None);
            s.play(BOOK, json!({}));
            assert_eq!(tokens(&s), vec![(5, false)]);
        }

        #[test]
        fn s3_2_none_on_a_full_board_and_the_spell_still_resolves() {
            let mut s = book(false, json!([VANILLA, VANILLA, VANILLA, VANILLA, VANILLA]), None);
            s.play(BOOK, json!({}));
            assert_eq!(tokens(&s), Vec::<(i32, bool)>::new());
            s.expect_in_zone(BOOK, "graveyard");
        }

        #[test]
        fn r386_the_count_is_no_declared_number_a_tokens_tuning_moves_nothing_the_roll_stays_1_2() {
            let mut s = book(false, json!([]), Some("book-of-tokens-tune"));
            step_param(s.card_mut(BOOK), "tokens", 2);
            s.play(BOOK, json!({}));
            assert!(!tokens(&s).is_empty());
            assert!(tokens(&s).len() <= 2);
        }
    }

    /// R1438: given Lucky 1, the base face rolls its count twice and keeps the most: its first roll is
    /// the plain face's, so it never summons fewer, and each cast takes one more draw.
    #[test]
    fn r1438_given_lucky_1_the_base_face_rolls_twice_and_keeps_the_better() {
        crate::register_all();
        let cast = |n: i32, lucky: bool| -> (usize, u32) {
            let mut s = book(false, json!([]), Some(&format!("book-of-tokens-lucky-{n}")));
            if lucky {
                crate::give_lucky(&mut s, BOOK, 1);
            }
            let before = s.state().rng_cursor;
            s.play(BOOK, json!({}));
            (tokens(&s).len(), s.state().rng_cursor - before)
        };
        let mut more = 0;
        for n in 0..40 {
            let ((plain, plain_draws), (lucky, lucky_draws)) = (cast(n, false), cast(n, true));
            assert!(lucky >= plain);
            assert_eq!(lucky_draws, plain_draws + 1);
            if lucky > plain {
                more += 1;
            }
        }
        assert!(more > 0);
    }

    mod radiant {
        use super::*;

        #[test]
        fn s7_summons_1_2_radiant_rush_tokens_6_6_rush_cleave() {
            let mut counts: IndexSet<usize> = IndexSet::new();
            for n in 0..20 {
                let mut s = book(true, json!([]), Some(&format!("book-of-tokens-radiant-{n}")));
                s.play(BOOK, json!({}));
                let found = tokens(&s);
                assert!(!found.is_empty());
                assert!(found.len() <= 2);
                let lanes: Vec<i32> = found.iter().map(|token| token.0).collect();
                assert_eq!(lanes, if found.len() == 1 { vec![1] } else { vec![1, 2] });
                assert!(found.iter().all(|token| token.1));
                let first = s.unit(PlayerId::P1, 1).map(|card| card.id).unwrap_or_default();
                s.expect_stats(&first, json!({ "attack": 6, "health": 6 }));
                assert_eq!(kinds(&s, &first), vec!["Rush".to_string(), "Cleave".to_string()]);
                counts.insert(found.len());
            }
            let mut both: Vec<usize> = counts.into_iter().collect();
            both.sort();
            assert_eq!(both, vec![1, 2]);
        }

        #[test]
        fn s6_1_lucky_1_the_radiant_face_rolls_twice_and_keeps_the_most_so_it_summons_2_more_often() {
            crate::register_all();
            let def = crate::card_def(super::super::ID);
            assert_eq!(def.radiant.keywords, vec![Keyword::Lucky { n: 1 }]);
            assert_eq!(def.base.keywords, Vec::<Keyword>::new());
            let twos = |radiant: bool| -> i32 {
                let mut found = 0;
                for n in 0..40 {
                    let mut s = book(radiant, json!([]), Some(&format!("book-of-tokens-lucky-{n}")));
                    s.play(BOOK, json!({}));
                    if tokens(&s).len() == 2 {
                        found += 1;
                    }
                }
                found
            };
            assert!(twos(true) > twos(false));
        }

        #[test]
        fn r386_the_radiant_roll_is_untunable_the_same_way() {
            let mut s = book(true, json!([]), Some("book-of-tokens-radiant-tune"));
            step_param(s.card_mut(BOOK), "tokens", 2);
            s.play(BOOK, json!({}));
            assert!(!tokens(&s).is_empty());
            assert!(tokens(&s).len() <= 2);
        }
    }
}
