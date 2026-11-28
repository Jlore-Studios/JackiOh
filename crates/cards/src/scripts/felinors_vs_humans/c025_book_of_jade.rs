//! SPEC §8.14 FvH #25 Book of Jade. (1) Spell, Book, Epic. Built by S03 (#5253).
//!   Base:    "Add an Auspicious Rock to your hand. {chance}% chance: Add another." — 66
//!   Radiant: "Add a Radiant Auspicious Rock to your hand. {chance}% chance: Add another." — 66
//!
//! R5253001 (FV2 #25): the designer's "1-2 Rocks {33% 1, 66% 2}" read as one Rock (M #39.1, as M #40
//! Feng Shui adds it), then one roll of the declared `chance` (R386) for a second. The roll is a luck
//! roll, drawn in the hook as C+ #53 Book of Tokens rolls its count (nothing pauses this list): the
//! card's net Lucky (R1438, R5236020) and its controller's Luck (R987) roll it again, a net above 0
//! keeping a success and one below keeping a miss. The Radiant face's Rocks are Radiant. A full hand
//! burns what does not fit (R317).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-025";

/// M #39.1 Auspicious Rock.
const ROCK: &str = "meditative-039-1";

fn rock(radiant: bool) -> Effect {
    add_to_hand(AddToHandArgs {
        def_id: Some(ROCK.to_string()),
        radiant: radiant.then_some(true),
        ..AddToHandArgs::default()
    })
}

/// R1438, R5236020, R987: the running card's net Lucky plus its controller's Luck.
fn net_luck(ctx: &EffectContext<'_>) -> i32 {
    ctx.live_self().map_or(0, |card| lucky_on(&*ctx.state, card)) + luck_of(&*ctx.state, ctx.controller)
}

/// R5253001: one Rock, and a second on a `chance` roll.
fn jade(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let chance = param(&*ctx, "chance");
            let luck = net_luck(ctx);
            let another = ctx.rng.lucky(luck, |rng| rng.int(PERCENT) < chance, |a, b| a || b);
            let mut effects = vec![rock(radiant)];
            if another {
                effects.push(rock(radiant));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: jade(false),
        radiant: jade(true),
    }
}

// FvH #25 Book of Jade — SPEC §8.14 row 25 (R5253001, R1438, R987, R5236020, R317).
#[cfg(test)]
mod tests {
    use super::{ID, ROCK};
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-005"; // Stockpile

    fn holding(radiant: bool, seed: &str, fillers: usize) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend((0..fillers).map(|_| json!(FILLER)));
        scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// p1's Auspicious Rocks in hand: whether each is Radiant.
    fn rocks(s: &Scenario) -> Vec<bool> {
        s.hand(P1)
            .into_iter()
            .filter(|card| card.def_id == ROCK)
            .map(|card| card.radiant)
            .collect()
    }

    /// One cast on `seed`, with `extra` keywords given to the Book first: the Rocks it added and the
    /// draws it took.
    fn cast(radiant: bool, seed: &str, extra: &[Keyword]) -> (usize, u32) {
        let mut s = holding(radiant, seed, 1);
        s.card_mut(ID).granted_keywords.extend(extra.iter().cloned());
        let before = s.state().rng_cursor;
        s.play(ID, json!({}));
        (rocks(&s).len(), s.state().rng_cursor - before)
    }

    mod base {
        use super::*;

        #[test]
        fn r5253001_one_rock_always_and_a_second_66_percent_of_the_time() {
            let mut twos = 0;
            for n in 0..200 {
                let (count, _) = cast(false, &format!("jade-odds-{n}"), &[]);
                assert!(count == 1 || count == 2, "{count}");
                if count == 2 {
                    twos += 1;
                }
            }
            // 132 expected of 200; the band is about four standard deviations wide.
            assert!((105..=159).contains(&twos), "{twos} of 200 added two");
        }

        #[test]
        fn r5253001_the_rocks_are_plain_auspicious_rocks() {
            let mut s = holding(false, "jade-plain", 1);
            s.play(ID, json!({}));
            let found = rocks(&s);
            assert!(!found.is_empty());
            assert!(found.iter().all(|radiant| !radiant), "{found:?}");
        }

        #[test]
        fn r386_chance_reads_through_param() {
            for n in 0..12 {
                let seed = format!("jade-sure-{n}");
                let mut s = holding(false, &seed, 1);
                set_param(s.card_mut(ID), "chance", PERCENT);
                s.play(ID, json!({}));
                assert_eq!(rocks(&s).len(), 2, "{seed}");
            }
        }

        #[test]
        fn r1438_a_given_lucky_rolls_once_more_and_keeps_a_success() {
            let mut more = 0;
            for n in 0..40 {
                let seed = format!("jade-lucky-{n}");
                let ((plain, plain_draws), (lucky, lucky_draws)) =
                    (cast(false, &seed, &[]), cast(false, &seed, &[Keyword::Lucky { n: 1 }]));
                assert!(lucky >= plain, "{seed}");
                if lucky == plain {
                    assert_eq!(lucky_draws, plain_draws + 1, "{seed}: one more roll");
                } else {
                    more += 1;
                }
            }
            assert!(more > 0);
        }

        #[test]
        fn r5236020_a_given_unlucky_rolls_once_more_and_keeps_a_miss() {
            let mut fewer = 0;
            for n in 0..40 {
                let seed = format!("jade-unlucky-{n}");
                let ((plain, plain_draws), (unlucky, unlucky_draws)) =
                    (cast(false, &seed, &[]), cast(false, &seed, &[Keyword::Unlucky { n: 1 }]));
                assert!(unlucky <= plain, "{seed}");
                if unlucky == plain {
                    assert_eq!(unlucky_draws, plain_draws + 1, "{seed}: one more roll");
                } else {
                    fewer += 1;
                }
            }
            assert!(fewer > 0);
        }

        #[test]
        fn r317_a_full_hand_burns_the_rock_that_does_not_fit() {
            // Nine cards left once the Book is played, and a sure second Rock: one fits, one burns.
            let mut s = holding(false, "jade-full", 9);
            set_param(s.card_mut(ID), "chance", PERCENT);
            s.play(ID, json!({}));
            assert_eq!(rocks(&s).len(), 1);
            let burned = s
                .last_events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Burned { def_id, .. } if def_id == ROCK))
                .count();
            assert_eq!(burned, 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5253001_the_rocks_are_radiant() {
            for n in 0..8 {
                let mut s = holding(true, &format!("jade-radiant-{n}"), 1);
                s.play(ID, json!({}));
                let found = rocks(&s);
                assert!(found.len() == 1 || found.len() == 2, "{found:?}");
                assert!(found.iter().all(|radiant| *radiant), "{found:?}");
            }
        }
    }
}
