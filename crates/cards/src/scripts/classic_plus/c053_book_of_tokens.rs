//! C+ #53 Book of Tokens (SPEC §8.7 row 53). (1) Spell, Book, Epic.
//!   Base:    "Summon 1-2 Rush Tokens." — the count is random (balance patch 1)
//!   Radiant: "Lucky 1 / Summon 1-2 Radiant Rush Tokens."
//!   Engine:  "Rush Tokens (§7) placed per R64; a full board takes fewer. Tunes: none."
//!
//! One laneless `summon` per token: each takes the leftmost empty, unlocked, unreserved unit zone (R64)
//! and fizzles silently on a full row (§3.2), as Core #15's do. The token's stats are its own (§7).
//! The count is one draw from the match rng (1 or 2), so the curve stays near a single 1-cost token
//! summon; a tuned count would be a param, and there is none. Lucky X (§6.1) rolls X more times and
//! keeps the most tokens: the Radiant face prints Lucky 1, and the Lucky read is the running card's
//! own, so a Degrade or an Upgrade of that number moves it (B3.4), as Two Grapes reads its Lucky.

use jackioh_engine::effects::summon;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-053";

/// TS `cardDef("core-t-rush").id`.
const RUSH_TOKEN: &str = "core-t-rush";

/// The printed "1-2": the fewest and most tokens one cast summons.
const MIN_TOKENS: i32 = 1;
const MAX_TOKENS: i32 = 2;

/// §6.1: the Lucky X the running card has now (its Radiant face prints Lucky 1).
fn lucky_of(ctx: &EffectContext<'_>) -> i32 {
    let Some(card) = ctx.live_self() else {
        return 0;
    };
    numbered_keywords_on(&*ctx.state, card)
        .iter()
        .find(|keyword| matches!(keyword.key, NumberedKey::Lucky))
        .map(|keyword| keyword.value)
        .unwrap_or(0)
}

fn book_of_tokens(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let roll = |rng: &mut Rng| -> i32 { MIN_TOKENS + rng.int(MAX_TOKENS - MIN_TOKENS + 1) };
            let lucky = lucky_of(ctx);
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
        base: book_of_tokens(false),
        radiant: book_of_tokens(true),
    }
}

// C+ #53 Book of Tokens — SPEC §8.7 row 53, BUILD M9 Classic+ row C+ 53: "Summons 1-2 Rush Tokens (3/3
// Rush), the count random, into your leftmost open zones, one on a nearly full board, none on a full
// one; the count is no declared number, so tuning moves nothing; radiant 1-2 Radiant Rush Tokens
// (6/6 Rush, Cleave)".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const BOOK: &str = "classicplus-053";
    const RUSH: &str = "core-t-rush";
    const VANILLA: &str = "core-008";
    const FILLER: &str = "core-005";

    use crate::scenario;

    /// TS `book({ radiant?, field?, seed? })`.
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

    #[test]
    fn is_a_1_spell_book() {
        crate::register_all();
        let def = crate::card_def(super::ID);
        assert_eq!(def.id, BOOK);
        assert_eq!(def.type_, CardType::Spell);
        assert_eq!(def.tags, vec![Tag::Book]);
        assert_eq!(def.refs, Some(vec![RUSH.to_string()]));
    }

    mod base {
        use super::*;

        #[test]
        fn r64_summons_1_2_rush_tokens_3_3_rush_into_your_leftmost_open_zones_the_count_random_but_never_above_the_curve() {
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
            // Both faces of the roll come up across seeds: genuinely 1-2, never 3.
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
