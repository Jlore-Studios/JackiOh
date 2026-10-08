//! C+ #17 Conjure Rush Token++ (SPEC §8.7 row 17): summon a Rush Token (Radiant: 3) with {keywords} (6)
//! different random keywords from R21's pool it lacks; a summon with no open zone rolls nothing (R129).

use jackioh_engine::effects::summon;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-017";

/// §7's shared Rush Token, 3/3 with Rush.
const RUSH_TOKEN: &str = "core-t-rush";

/// "Summon 3 Rush Tokens" on the Radiant face: the declared number `tokens` (R386), tuned on the
/// Radiant face only — the base face's "a Rush Token" is 1 and never moves (R749), so both faces read
/// the one number.
fn conjure() -> Hook {
    hook(|ctx| {
        let keywords = param(&*ctx, "keywords");
        (0..param(&*ctx, "tokens"))
            .map(|_| summon(json_as(json!({ "defId": RUSH_TOKEN, "randomKeywords": keywords }))))
            .collect()
    })
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(conjure()),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(conjure()),
            ..Script::default()
        },
    }
}

// C+ #17 Conjure Rush Token++ — SPEC §8.7 row 17, BUILD M9 Classic+ row C+ 17: "As C+ #15 with 6 different random
// keywords the token lacks, out of the 11 R21 leaves it; the keyword count reads through `param()`; radiant 3 tokens
// with 6 each".

/// `describe("C+ #17 Conjure Rush Token++")`.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const CARD: &str = "classicplus-017";
    const RUSH_TOKEN: &str = "core-t-rush";
    const KEYWORDS: usize = 6;
    /// The entry's declared step for its keyword count.
    const STEP: usize = 2;
    const FILLER: &str = "core-005"; // a card in hand, so a turn never auto-ends under the test
    const BODY: &str = "core-019"; // Midrange Menace, a unit to fill a zone

    use crate::scenario;

    use crate::js;

    fn tokens(s: &Scenario) -> Vec<CardInstance> {
        (1..=5)
            .filter_map(|lane| s.unit(P1, lane))
            .filter(|unit| unit.def_id == RUSH_TOKEN)
            .collect()
    }

    /// The keywords a token gained beyond its printed Rush: all from R21's pool, all different, never Rush.
    fn gained(s: &Scenario, token: &str) -> Vec<String> {
        let keys: Vec<String> = s
            .stats(token)
            .keywords
            .iter()
            .map(keyword_key)
            .filter(|key| key != "Rush")
            .collect();
        for key in &keys {
            assert!(RANDOM_KEYWORD_POOL.contains(&key.as_str()), "{key} is in R21's pool");
        }
        let distinct: IndexSet<&String> = keys.iter().collect();
        assert_eq!(distinct.len(), keys.len());
        keys
    }

    /// TS `tokens(s)[0] ?? ""`: the id of p1's first Rush Token, or "".
    fn first_token(s: &Scenario) -> String {
        tokens(s).into_iter().next().map(|token| token.id).unwrap_or_default()
    }

    /// is a (4) Spell that declares no play-time choice
    #[test]
    fn is_a_4_spell_that_declares_no_play_time_choice() {
        crate::register_all();
        let def = crate::card_def(CARD);
        assert_eq!(def.id, CARD);
        assert_eq!(js(&def.cost), json!(4));
        assert_eq!(def.type_, CardType::Spell);
        let scripts = script();
        assert!(scripts.base.targets.is_empty());
        assert!(scripts.radiant.targets.is_empty());
    }

    /// `describe("base")`.
    mod base {
        use super::*;

        /// R21 summons one Rush Token, 3/3 Rush, with 6 different random keywords it lacks, never Rush again
        #[test]
        fn r21_summons_one_rush_token_3_3_rush_with_6_different_random_keywords_it_lacks_never_rush_again() {
            let mut seen: IndexSet<String> = IndexSet::new();
            for seed in 1..=12 {
                let mut s = scenario(json!({ "seed": format!("c17-{seed}"), "p1": { "hand": [CARD, FILLER] }, "p2": { "hand": [FILLER] } }));
                s.play(CARD, json!({}));
                let made = tokens(&s);
                assert_eq!(made.len(), 1);
                let token = made.into_iter().next().expect("no token");
                assert_eq!(s.unit(P1, 1).map(|unit| unit.id), Some(token.id.clone()));
                s.expect_stats(&token, json!({ "attack": 3, "health": 3 }));
                assert!(s.stats(&token).keywords.iter().map(keyword_key).any(|key| key == "Rush"));
                let keys = gained(&s, &token.id);
                assert_eq!(keys.len(), KEYWORDS);
                seen.extend(keys);
            }
            assert!(seen.len() > KEYWORDS);
        }

        /// R64 R129 a full board summons nothing and draws nothing from the rng
        #[test]
        fn r64_r129_a_full_board_summons_nothing_and_draws_nothing_from_the_rng() {
            let mut s = scenario(json!({ "p1": { "hand": [CARD, FILLER], "field": [BODY, BODY, BODY, BODY, BODY] }, "p2": { "hand": [FILLER] } }));
            let cursor = s.state().rng_cursor;
            s.play(CARD, json!({}));
            assert!(tokens(&s).is_empty());
            assert_eq!(s.state().rng_cursor, cursor);
            s.expect_in_zone(CARD, "graveyard");
        }

        /// R386 the keyword count reads through param: an Upgrade gives two more, a Degrade two fewer (its step is 2)
        #[test]
        fn r386_the_keyword_count_reads_through_param_an_upgrade_gives_two_more_a_degrade_two_fewer_its_step_is_2() {
            let mut up = scenario(json!({ "p1": { "hand": [CARD, FILLER] }, "p2": { "hand": [FILLER] } }));
            step_param(up.card_mut(CARD), "keywords", 1);
            up.play(CARD, json!({}));
            assert_eq!(gained(&up, &first_token(&up)).len(), KEYWORDS + STEP);

            let mut down = scenario(json!({ "p1": { "hand": [CARD, FILLER] }, "p2": { "hand": [FILLER] } }));
            step_param(down.card_mut(CARD), "keywords", -1);
            down.play(CARD, json!({}));
            assert_eq!(gained(&down, &first_token(&down)).len(), KEYWORDS - STEP);
        }

        /// R21 tuned past the pool, the token gains every keyword R21 leaves it, 13, and no repeat
        #[test]
        fn r21_tuned_past_the_pool_the_token_gains_every_keyword_r21_leaves_it_13_and_no_repeat() {
            let mut s = scenario(json!({ "p1": { "hand": [CARD, FILLER] }, "p2": { "hand": [FILLER] } }));
            step_param(s.card_mut(CARD), "keywords", 4);
            s.play(CARD, json!({}));
            assert_eq!(gained(&s, &first_token(&s)).len(), RANDOM_KEYWORD_POOL.len() - 1);
        }
    }

    /// `describe("radiant")`.
    mod radiant {
        use super::*;

        /// summons three Rush Tokens into the leftmost open zones, each with its own 6 random keywords
        #[test]
        fn summons_three_rush_tokens_into_the_leftmost_open_zones_each_with_its_own_6_random_keywords() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": CARD, "radiant": true }, FILLER], "field": [{ "def": BODY, "lane": 2 }] },
                "p2": { "hand": [FILLER] },
            }));
            s.play(CARD, json!({}));
            let made = tokens(&s);
            let zones: Vec<Value> = made.iter().map(|token| js(&s.card(token).zone)).collect();
            let expected: Vec<Value> = [1, 3, 4]
                .into_iter()
                .map(|lane| json!({ "z": "field", "player": "p1", "row": "units", "lane": lane }))
                .collect();
            assert_eq!(zones, expected);
            for token in &made {
                assert!(!token.radiant);
                assert_eq!(gained(&s, &token.id).len(), KEYWORDS);
            }
        }

        /// R64 a nearly full board summons fewer: one open zone, one token
        #[test]
        fn r64_a_nearly_full_board_summons_fewer_one_open_zone_one_token() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": CARD, "radiant": true }, FILLER], "field": [BODY, BODY, BODY, BODY] },
                "p2": { "hand": [FILLER] },
            }));
            s.play(CARD, json!({}));
            assert_eq!(tokens(&s).len(), 1);
            assert_eq!(s.unit(P1, 5).map(|unit| unit.def_id).as_deref(), Some(RUSH_TOKEN));
        }

        /// R386 an Upgrade summons 4 tokens and a Degrade 2; the base face's one token is never tuned
        #[test]
        fn r386_an_upgrade_summons_4_tokens_and_a_degrade_2_and_the_base_face_s_one_is_never_tuned() {
            for (upgrade, count) in [(true, 4), (false, 2)] {
                let mut s =
                    scenario(json!({ "p1": { "hand": [{ "def": CARD, "radiant": true }, FILLER] }, "p2": { "hand": [FILLER] } }));
                let moved = if upgrade {
                    crate::upgrade_number(&mut s, CARD, "tokens")
                } else {
                    crate::degrade_number(&mut s, CARD, "tokens")
                };
                assert_eq!(moved, count);
                s.play(CARD, json!({}));
                assert_eq!(tokens(&s).len(), count as usize);
            }
            let s = scenario(json!({ "p1": { "hand": [CARD, FILLER] } }));
            assert!(!crate::can_upgrade_number(&s, CARD, "tokens"));
            assert!(!crate::can_degrade_number(&s, CARD, "tokens"));
        }
    }
}
