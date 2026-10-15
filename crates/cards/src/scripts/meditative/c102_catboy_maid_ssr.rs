//! M #102 Catboy Maid SSR+ (SPEC §8.8 row 102): (1) Unit, CN, Felinor, Common, 1/1 → 2/2. The
//! designer's #99.
//!
//! Base:    "Lucky 1\nCry: Draw 1-2. Add 1-3 to your Jade Counter."
//! Radiant: "Lucky 2\nCry: Draw 1-2. Add 1-6 to your Jade Counter."
//! Engine (MD-G3, R1439):
//! - **Two rolls:** the Cry rolls how many cards to draw, 1 or 2, as C+ #53 Book of Tokens rolls its
//!   count, then how much to add, uniform over 1–3 (1–6 on the Radiant face). Each is its own
//!   `rng.lucky` roll, higher better, with the card's Lucky as it stands (printed, as a Nerf or a
//!   Buff has moved it, plus any given, R386, R1438: `lucky_on`) and its controller's Luck (R987).
//! - **The draws** are §2.4's: the hand cap burns the extra card and an empty deck deals fatigue.
//! - **The amount** goes through ME-JADE's `add_jade` (MD-C2), so reaching 5 summons a Jade Beauty
//!   and reaching 10 makes it Radiant (MD-C3, R962).
//!
//! The ranges are the printed text's, not declared numbers (as Book of Tokens' count is not), so a
//! Nerf or a Buff moves only the Lucky.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-102";

/// The printed "Draw 1-2": the fewest and most cards one Cry draws.
const MIN_DRAWS: i32 = 1;
const MAX_DRAWS: i32 = 2;

/// The printed "Add 1-3" (base) and "Add 1-6" (Radiant): the least and most one Cry adds (patch
/// v0.3.10, #831; 1-4 and 1-10 before).
pub const MIN_JADE: i32 = 1;
pub const MAX_JADE: i32 = 3;
pub const MAX_JADE_RADIANT: i32 = 6;

/// One uniform roll over `low..=high`.
fn uniform(rng: &mut Rng, low: i32, high: i32) -> i32 {
    low + rng.int(high - low + 1)
}

fn catboy_maid(max_jade: i32) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            // R1438, R987: the Lucky on the card as it stands, and its controller's Luck.
            let lucky = ctx.live_self().map_or(0, |card| lucky_on(&*ctx.state, card))
                + luck_of(&*ctx.state, ctx.controller);
            let draws = ctx.rng.lucky(
                lucky,
                |rng| uniform(rng, MIN_DRAWS, MAX_DRAWS),
                |a, b| a.max(b),
            );
            let jade = ctx.rng.lucky(
                lucky,
                |rng| uniform(rng, MIN_JADE, max_jade),
                |a, b| a.max(b),
            );
            vec![
                draw(json_as(json!({ "count": draws }))),
                add_jade(json_as(json!({ "amount": jade }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: catboy_maid(MAX_JADE),
        radiant: catboy_maid(MAX_JADE_RADIANT),
    }
}

// M #102 Catboy Maid SSR+ — SPEC §8.8 row 102, BUILD M10 row M 102: "Lucky 1; Cry (played): draws 1
// or 2 and adds 1 to 4 to your Jade Counter, every value seen over the seeds (MD-G3); each roll is two
// draws keeping the higher; given Lucky 1 (R1438), each roll is three draws; a full hand burns the
// extra card and an empty deck deals fatigue (§2.4); reaching 5 summons a Jade Beauty (MD-C3); radiant
// 2/2 and Lucky 2: three draws a roll and 1 to 10 added, and reaching 10 makes your Jade Beauty
// Radiant".
#[cfg(test)]
mod tests {
    use super::{ID, MAX_DRAWS, MAX_JADE, MAX_JADE_RADIANT, MIN_DRAWS, MIN_JADE};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-016";
    const BEAUTY: &str = "meditative-039-5";
    const FENG_SHUI: &str = "meditative-040";

    /// p1 holds the Maid (Radiant on `radiant`) and one filler, with three cards to draw.
    fn holding(seed: &str, radiant: bool) -> Scenario {
        crate::scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "library": [FILLER, FILLER, FILLER],
            },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }))
    }

    /// One Cry: `(cards drawn, Jade Counter after, match-rng draws taken)`.
    fn cry(mut s: Scenario) -> (i32, i32, u32) {
        let hand = s.hand(P1).len() as i32;
        let before = s.state().rng_cursor;
        s.play(ID, json!({}));
        // The Maid left the hand; what came back is what the Cry drew.
        let drawn = s.hand(P1).len() as i32 - (hand - 1);
        let jade = s.view(P1).you.jade.unwrap_or(0);
        (drawn, jade, s.state().rng_cursor - before)
    }

    /// The roll `rng.lucky` makes with Lucky `lucky` over `low..=high`, replayed from `cursor`.
    fn replayed(seed: &str, cursor: u32, lucky: i32, ranges: [(i32, i32); 2]) -> [i32; 2] {
        let mut rng = create_rng(seed, cursor);
        ranges.map(|(low, high)| rng.lucky(lucky, |rng| low + rng.int(high - low + 1), |a, b| a.max(b)))
    }

    #[test]
    fn is_a_1_cn_felinor_1_1_lucky_1_radiant_2_2_lucky_2() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.cost, CardCost::Fixed(1));
        assert_eq!(def.type_, CardType::Unit);
        assert_eq!(def.tags, vec![Tag::Cn, Tag::Felinor]);
        assert_eq!(
            [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
            [Some(1), Some(1), Some(2), Some(2)]
        );
        assert_eq!(def.base.keywords, vec![Keyword::Lucky { n: 1 }]);
        assert_eq!(def.radiant.keywords, vec![Keyword::Lucky { n: 2 }]);
        // R1444: it prints Lucky, so it is a Luck-based card.
        assert!(is_luck_based(&def));
    }

    mod base {
        use super::*;

        #[test]
        fn r1439_the_cry_draws_1_or_2_and_adds_1_to_3_every_value_seen_over_the_seeds() {
            let mut drawn_seen: Vec<i32> = Vec::new();
            let mut jade_seen: Vec<i32> = Vec::new();
            for n in 0..60 {
                let (drawn, jade, _) = cry(holding(&format!("catboy-maid-{n}"), false));
                assert!((MIN_DRAWS..=MAX_DRAWS).contains(&drawn), "{drawn}");
                assert!((MIN_JADE..=MAX_JADE).contains(&jade), "{jade}");
                if !drawn_seen.contains(&drawn) {
                    drawn_seen.push(drawn);
                }
                if !jade_seen.contains(&jade) {
                    jade_seen.push(jade);
                }
            }
            drawn_seen.sort();
            jade_seen.sort();
            assert_eq!(drawn_seen, vec![1, 2]);
            assert_eq!(jade_seen, vec![1, 2, 3]);
        }

        #[test]
        fn r1439_each_roll_is_two_draws_keeping_the_higher_the_draws_first() {
            for n in 0..12 {
                let seed = format!("catboy-maid-roll-{n}");
                let s = holding(&seed, false);
                let cursor = s.state().rng_cursor;
                let (drawn, jade, taken) = cry(s);
                // Lucky 1: each of the two rolls is two draws of the match rng.
                assert_eq!(taken, 4);
                let [draws, amount] = replayed(&seed, cursor, 1, [(MIN_DRAWS, MAX_DRAWS), (MIN_JADE, MAX_JADE)]);
                assert_eq!([drawn, jade], [draws, amount]);
            }
        }

        #[test]
        fn r1439_r1438_given_lucky_1_each_roll_is_three_draws() {
            for n in 0..12 {
                let seed = format!("catboy-maid-given-{n}");
                let mut s = holding(&seed, false);
                crate::give_lucky(&mut s, ID, 1);
                let cursor = s.state().rng_cursor;
                let (drawn, jade, taken) = cry(s);
                assert_eq!(taken, 6);
                let [draws, amount] = replayed(&seed, cursor, 2, [(MIN_DRAWS, MAX_DRAWS), (MIN_JADE, MAX_JADE)]);
                assert_eq!([drawn, jade], [draws, amount]);
            }
        }

        #[test]
        fn r1439_r987_feng_shui_s_luck_reaches_both_rolls() {
            // Luck 1 beside the printed Lucky 1: each roll keeps the best of three draws. (The
            // Maid is p1's first play, which Feng Shui judges neutral, R981: nothing else draws.)
            for n in 0..12 {
                let seed = format!("catboy-maid-luck-{n}");
                let s = crate::scenario(json!({
                    "seed": seed,
                    "p1": {
                        "hand": [ID, FILLER],
                        "backrow": [FENG_SHUI],
                        "library": [FILLER, FILLER, FILLER],
                    },
                    "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
                }));
                assert_eq!(luck_of(s.state(), P1), 1);
                let cursor = s.state().rng_cursor;
                let (drawn, jade, taken) = cry(s);
                assert_eq!(taken, 6);
                let [draws, amount] = replayed(&seed, cursor, 2, [(MIN_DRAWS, MAX_DRAWS), (MIN_JADE, MAX_JADE)]);
                assert_eq!([drawn, jade], [draws, amount]);
            }
        }

        #[test]
        fn r1439_a_full_hand_burns_the_second_card() {
            crate::register_all();
            let mut burned = 0;
            for n in 0..20 {
                // The Maid and nine fillers: once it is played the hand holds nine, so a draw of 2
                // fills it with the first and burns the second (§2.4).
                let mut hand = vec![json!(ID)];
                hand.extend((0..HAND_CAP - 1).map(|_| json!(FILLER)));
                let mut s = crate::scenario(json!({
                    "seed": format!("catboy-maid-burn-{n}"),
                    "p1": { "hand": hand, "library": [FILLER, FILLER, FILLER] },
                    "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
                }));
                s.play(ID, json!({}));
                assert_eq!(s.hand(P1).len() as i32, HAND_CAP);
                let burns = s
                    .last_events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::Burned { owner, .. } if *owner == P1))
                    .count();
                assert!(burns <= 1);
                burned += burns;
            }
            assert!(burned > 0);
        }

        #[test]
        fn r1439_an_empty_deck_deals_fatigue() {
            let mut s = crate::scenario(json!({
                "seed": "catboy-maid-fatigue",
                "p1": { "hand": [ID, FILLER] },
                "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
            }));
            let health = s.view(P1).you.hero.health;
            s.play(ID, json!({}));
            let fatigues = s
                .last_events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Fatigue { player, .. } if *player == P1))
                .count();
            assert!((1..=2).contains(&fatigues));
            assert!(s.view(P1).you.hero.health < health);
            // The Jade Counter still rises.
            assert!(s.view(P1).you.jade.is_some_and(|jade| jade >= MIN_JADE));
        }

        #[test]
        fn r1439_r962_reaching_5_summons_a_jade_beauty() {
            for n in 0..20 {
                let mut s = holding(&format!("catboy-maid-beauty-{n}"), false);
                s.state_mut().players.p1.jade = Some(4);
                s.play(ID, json!({}));
                // Any add from 4 reaches 5. The Maid takes lane 1, so the Beauty lands beside it
                // (R64), on its base face: the base face adds at most 3, short of 10.
                assert!(s.view(P1).you.jade.is_some_and(|jade| (5..10).contains(&jade)));
                let beauty = s.unit(P1, 2);
                assert_eq!(beauty.as_ref().map(|unit| unit.def_id.as_str()), Some(BEAUTY));
                assert!(beauty.is_some_and(|unit| !unit.radiant));
                assert_eq!(s.view(P2).opponent.jade, s.view(P1).you.jade);
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1439_lucky_2_three_draws_a_roll_and_1_to_6_added() {
            let mut jade_seen: Vec<i32> = Vec::new();
            for n in 0..40 {
                let seed = format!("catboy-maid-radiant-{n}");
                let s = holding(&seed, true);
                let cursor = s.state().rng_cursor;
                let (drawn, jade, taken) = cry(s);
                assert_eq!(taken, 6);
                assert!((MIN_DRAWS..=MAX_DRAWS).contains(&drawn));
                assert!((MIN_JADE..=MAX_JADE_RADIANT).contains(&jade));
                let [draws, amount] = replayed(&seed, cursor, 2, [(MIN_DRAWS, MAX_DRAWS), (MIN_JADE, MAX_JADE_RADIANT)]);
                assert_eq!([drawn, jade], [draws, amount]);
                if !jade_seen.contains(&jade) {
                    jade_seen.push(jade);
                }
            }
            // Past the base face's 3: the Radiant range reaches higher.
            assert!(jade_seen.iter().any(|jade| *jade > MAX_JADE));
        }

        #[test]
        fn r1439_r962_reaching_10_makes_your_jade_beauty_radiant() {
            let mut ascended = 0;
            for n in 0..40 {
                let mut s = holding(&format!("catboy-maid-ascend-{n}"), true);
                s.state_mut().players.p1.jade = Some(4);
                s.play(ID, json!({}));
                let jade = s.view(P1).you.jade.unwrap_or(0);
                let beauty = s.unit(P1, 2);
                assert_eq!(beauty.as_ref().map(|unit| unit.def_id.as_str()), Some(BEAUTY));
                // 4 + the roll: 10 or more crosses both thresholds in the one add (MD-C3).
                assert_eq!(beauty.is_some_and(|unit| unit.radiant), jade >= 10);
                if jade >= 10 {
                    ascended += 1;
                }
            }
            assert!(ascended > 0);
        }

        #[test]
        fn is_2_2_with_lucky_2() {
            let mut s = holding("catboy-maid-radiant-stats", true);
            s.play(ID, json!({}));
            let maid = s.unit(P1, 1).map(|unit| unit.id).unwrap_or_default();
            s.expect_stats(&maid, json!({ "attack": 2, "health": 2 }));
            assert_eq!(lucky_on(s.state(), s.card(&maid)), 2);
        }
    }
}
