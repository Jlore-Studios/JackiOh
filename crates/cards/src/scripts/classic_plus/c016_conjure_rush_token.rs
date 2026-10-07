//! C+ #16 Conjure Rush Token+ (SPEC §8.7 row 16): summon a Rush Token (Radiant: 3) with {keywords} (3)
//! different random keywords from R21's pool it lacks; a summon with no open zone rolls nothing (R129).

use jackioh_engine::effects::summon;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-016";

/// §7's shared Rush Token, 3/3 with Rush.
const RUSH_TOKEN: &str = "core-t-rush";

/// "Summon 3 Rush Tokens" on the Radiant face; the base face summons one.
const RADIANT_TOKENS: usize = 3;

fn conjure(tokens: usize) -> Hook {
    hook(move |ctx| {
        let keywords = param(&*ctx, "keywords");
        (0..tokens)
            .map(|_| summon(json_as(json!({ "defId": RUSH_TOKEN, "randomKeywords": keywords }))))
            .collect()
    })
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(conjure(1)),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(conjure(RADIANT_TOKENS)),
            ..Script::default()
        },
    }
}

// C+ #16 Conjure Rush Token+ — SPEC §8.7 row 16, BUILD M9 Classic+ row C+ 16: "As C+ #15 with 3 different random
// keywords the token lacks; the keyword count reads through `param()`; radiant 3 tokens
// with 3 each".

/// `describe("C+ #16 Conjure Rush Token+")`.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const CARD: &str = "classicplus-016";
    const RUSH_TOKEN: &str = "core-t-rush";
    const KEYWORDS: usize = 3;
    const FILLER: &str = "core-005"; // a card in hand, so a turn never auto-ends under the test
    const BODY: &str = "core-019"; // Midrange Menace, a unit to fill a zone

    /// The harness registers every card first (TS's harness did on import).
    fn scenario(opts: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(opts)
    }

    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialises")
    }

    fn step(s: &mut Scenario, card: &str, key: &str, steps: i32) {
        let id = s.card(card).id.clone();
        step_param(find_instance_mut(s.state_mut(), &id).expect("the card is in the game"), key, steps);
    }

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

    /// is a (2) Spell that declares no play-time choice
    #[test]
    fn is_a_2_spell_that_declares_no_play_time_choice() {
        crate::register_all();
        let def = crate::card_def(CARD);
        assert_eq!(def.id, CARD);
        assert_eq!(js(&def.cost), json!(2));
        assert_eq!(def.type_, CardType::Spell);
        let scripts = script();
        assert!(scripts.base.targets.is_empty());
        assert!(scripts.radiant.targets.is_empty());
    }

    /// `describe("base")`.
    mod base {
        use super::*;

        /// R21 summons one Rush Token, 3/3 Rush, with 3 different random keywords it lacks, never Rush again
        #[test]
        fn r21_summons_one_rush_token_3_3_rush_with_3_different_random_keywords_it_lacks_never_rush_again() {
            let mut seen: IndexSet<String> = IndexSet::new();
            for seed in 1..=12 {
                let mut s = scenario(json!({ "seed": format!("c16-{seed}"), "p1": { "hand": [CARD, FILLER] }, "p2": { "hand": [FILLER] } }));
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

        /// R386 the keyword count reads through param: an Upgrade gives one more, a Degrade one fewer
        #[test]
        fn r386_the_keyword_count_reads_through_param_an_upgrade_gives_one_more_a_degrade_one_fewer() {
            let mut up = scenario(json!({ "p1": { "hand": [CARD, FILLER] }, "p2": { "hand": [FILLER] } }));
            step(&mut up, CARD, "keywords", 1);
            up.play(CARD, json!({}));
            assert_eq!(gained(&up, &first_token(&up)).len(), KEYWORDS + 1);

            let mut down = scenario(json!({ "p1": { "hand": [CARD, FILLER] }, "p2": { "hand": [FILLER] } }));
            step(&mut down, CARD, "keywords", -1);
            down.play(CARD, json!({}));
            assert_eq!(gained(&down, &first_token(&down)).len(), KEYWORDS - 1);
        }
    }

    /// `describe("radiant")`.
    mod radiant {
        use super::*;

        /// summons three Rush Tokens into the leftmost open zones, each with its own 3 random keywords
        #[test]
        fn summons_three_rush_tokens_into_the_leftmost_open_zones_each_with_its_own_3_random_keywords() {
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
    }
}
