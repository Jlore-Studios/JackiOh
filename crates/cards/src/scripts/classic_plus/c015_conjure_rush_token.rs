//! C+ #15 Conjure Rush Token (SPEC §8.7 row 15): summon a Rush Token (Radiant: 3) with {keywords} random
//! keywords from R21's pool it lacks; a summon with no open zone rolls nothing (R129).

use jackioh_engine::effects::summon;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-015";

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

// C+ #15 Conjure Rush Token — SPEC §8.7 row 15, BUILD M9 Classic+ row C+ 15: "Summons Core's Rush Token
// (3/3 Rush) into your leftmost open zone with one random keyword from R21's pool it lacks (never Rush
// again); a full board summons nothing and draws nothing (R129); the keyword count reads through
// `param()`; radiant 3 Rush Tokens, fewer on a nearly full board, each with its own random keyword".

/// `describe("C+ #15 Conjure Rush Token")`.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const CARD: &str = "classicplus-015";
    const RUSH_TOKEN: &str = "core-t-rush";
    const FILLER: &str = "core-005"; // a card in hand, so a turn never auto-ends under the test
    const BODY: &str = "core-019"; // Midrange Menace, 9/9 Taunt: a unit to fill a zone

    use crate::scenario;

    use crate::js;

    /// The Rush Tokens on p1's side, lane order.
    fn tokens(s: &Scenario) -> Vec<CardInstance> {
        (1..=5)
            .filter_map(|lane| s.unit(P1, lane))
            .filter(|unit| unit.def_id == RUSH_TOKEN)
            .collect()
    }

    /// The keywords a token gained beyond its printed Rush, as R21's pool writes them.
    fn gained(s: &Scenario, token: &str) -> Vec<String> {
        s.stats(token)
            .keywords
            .iter()
            .map(keyword_key)
            .filter(|key| key != "Rush")
            .collect()
    }

    fn expect_from_pool(keys: &[String]) {
        for key in keys {
            assert!(RANDOM_KEYWORD_POOL.contains(&key.as_str()), "{key} is in R21's pool");
        }
        let distinct: IndexSet<&String> = keys.iter().collect();
        assert_eq!(distinct.len(), keys.len());
    }

    /// is a (1) Spell that declares no play-time choice
    #[test]
    fn is_a_1_spell_that_declares_no_play_time_choice() {
        crate::register_all();
        let def = crate::card_def(CARD);
        assert_eq!(def.id, CARD);
        assert_eq!(js(&def.cost), json!(1));
        assert_eq!(def.type_, CardType::Spell);
        let scripts = script();
        assert!(scripts.base.targets.is_empty());
        assert!(scripts.radiant.targets.is_empty());
    }

    /// `describe("base")`.
    mod base {
        use super::*;

        /// summons Core's Rush Token, 3/3 Rush on its base face, into the leftmost open zone
        #[test]
        fn summons_cores_rush_token_3_3_rush_on_its_base_face_into_the_leftmost_open_zone() {
            let mut s = scenario(json!({ "p1": { "hand": [CARD, FILLER], "field": [{ "def": BODY, "lane": 1 }] }, "p2": { "hand": [FILLER] } }));
            s.play(CARD, json!({}));
            let token = tokens(&s).into_iter().next().expect("a Rush Token");
            assert_eq!(s.unit(P1, 2).map(|unit| unit.id), Some(token.id.clone()));
            assert!(!token.radiant);
            s.expect_stats(&token, json!({ "attack": 3, "health": 3 }));
            assert!(s.stats(&token).keywords.iter().map(keyword_key).any(|key| key == "Rush"));
        }

        /// R21 it gains one random keyword from the pool, never Rush again, across seeds
        #[test]
        fn r21_it_gains_one_random_keyword_from_the_pool_never_rush_again_across_seeds() {
            let mut seen: IndexSet<String> = IndexSet::new();
            for seed in 1..=24 {
                let mut s = scenario(json!({ "seed": format!("c15-{seed}"), "p1": { "hand": [CARD, FILLER] }, "p2": { "hand": [FILLER] } }));
                s.play(CARD, json!({}));
                let token = tokens(&s).into_iter().next().expect("no token");
                let keys = gained(&s, &token.id);
                assert_eq!(keys.len(), 1);
                expect_from_pool(&keys);
                assert!(!keys.iter().any(|key| key == "Rush"));
                seen.extend(keys);
            }
            // The roll is random: two dozen seeds land on more than one keyword.
            assert!(seen.len() > 1);
        }

        /// R64 R129 a full board summons nothing and draws nothing from the rng
        #[test]
        fn r64_r129_a_full_board_summons_nothing_and_draws_nothing_from_the_rng() {
            let mut s = scenario(json!({
                "p1": { "hand": [CARD, FILLER], "field": [BODY, BODY, BODY, BODY, BODY] },
                "p2": { "hand": [FILLER] },
            }));
            let cursor = s.state().rng_cursor;
            s.play(CARD, json!({}));
            assert!(tokens(&s).is_empty());
            assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Summoned));
            assert_eq!(s.state().rng_cursor, cursor);
            s.expect_in_zone(CARD, "graveyard");
        }

        /// R386 the keyword count reads through param: an Upgrade gives two, and a Degrade never goes below one
        #[test]
        fn r386_the_keyword_count_reads_through_param_an_upgrade_gives_two_and_a_degrade_never_goes_below_one() {
            let mut up = scenario(json!({ "p1": { "hand": [CARD, FILLER] }, "p2": { "hand": [FILLER] } }));
            step_param(up.card_mut(CARD), "keywords", 1);
            up.play(CARD, json!({}));
            let up_token = tokens(&up).into_iter().next().expect("no token");
            let keys = gained(&up, &up_token.id);
            assert_eq!(keys.len(), 2);
            expect_from_pool(&keys);

            let mut down = scenario(json!({ "p1": { "hand": [CARD, FILLER] }, "p2": { "hand": [FILLER] } }));
            step_param(down.card_mut(CARD), "keywords", -1);
            down.play(CARD, json!({}));
            let down_token = tokens(&down).into_iter().next().expect("no token");
            assert_eq!(gained(&down, &down_token.id).len(), 1);
        }

        /// §9.3 the rolled keyword replays from a JSON copy to the same hash
        #[test]
        fn s9_3_the_rolled_keyword_replays_from_a_json_copy_to_the_same_hash() {
            let s = scenario(json!({ "seed": "c15-replay", "p1": { "hand": [CARD, FILLER] }, "p2": { "hand": [FILLER] } }));
            let action: Action = json_as(json!({
                "type": "play",
                "instanceId": s.card(CARD).id,
                "playerId": "p1",
                "nonce": "c15-replay",
            }));
            let thawed: GameState = serde_json::from_value(js(s.state())).expect("the state round-trips through JSON");
            let live = reduce(s.state(), &action);
            assert!(live.error.is_none());
            assert_eq!(hash_state(&reduce(&thawed, &action).state), hash_state(&live.state));
        }
    }

    /// `describe("radiant")`.
    mod radiant {
        use super::*;

        /// summons three Rush Tokens, each with its own random keyword
        #[test]
        fn summons_three_rush_tokens_each_with_its_own_random_keyword() {
            let mut s = scenario(json!({ "p1": { "hand": [{ "def": CARD, "radiant": true }, FILLER] }, "p2": { "hand": [FILLER] } }));
            s.play(CARD, json!({}));
            let made = tokens(&s);
            assert_eq!(made.len(), 3);
            let zones: Vec<Value> = made.iter().map(|token| js(&s.card(token).zone)).collect();
            assert_eq!(
                zones,
                vec![
                    json!({ "z": "field", "player": "p1", "row": "units", "lane": 1 }),
                    json!({ "z": "field", "player": "p1", "row": "units", "lane": 2 }),
                    json!({ "z": "field", "player": "p1", "row": "units", "lane": 3 }),
                ]
            );
            for token in &made {
                assert!(!token.radiant);
                let keys = gained(&s, &token.id);
                assert_eq!(keys.len(), 1);
                expect_from_pool(&keys);
            }
        }

        /// R21 each token rolls its own keyword: across seeds the three do not always share one
        #[test]
        fn r21_each_token_rolls_its_own_keyword_across_seeds_the_three_do_not_always_share_one() {
            let mut differ = 0;
            for seed in 1..=8 {
                let mut s = scenario(json!({
                    "seed": format!("c15-own-{seed}"),
                    "p1": { "hand": [{ "def": CARD, "radiant": true }, FILLER] },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(CARD, json!({}));
                let rolled: Vec<String> = tokens(&s).iter().map(|token| gained(&s, &token.id).join(",")).collect();
                assert_eq!(rolled.len(), 3);
                if rolled.iter().collect::<IndexSet<&String>>().len() > 1 {
                    differ += 1;
                }
            }
            assert!(differ > 0);
        }

        /// R64 a nearly full board summons fewer: one open zone, one token
        #[test]
        fn r64_a_nearly_full_board_summons_fewer_one_open_zone_one_token() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": CARD, "radiant": true }, FILLER], "field": [BODY, BODY, { "def": BODY, "lane": 4 }, BODY] },
                "p2": { "hand": [FILLER] },
            }));
            s.play(CARD, json!({}));
            let made = tokens(&s);
            assert_eq!(made.len(), 1);
            assert_eq!(s.unit(P1, 5).map(|unit| unit.def_id).as_deref(), Some(RUSH_TOKEN));
        }

        /// R386 an Upgrade gives each of the three two keywords
        #[test]
        fn r386_an_upgrade_gives_each_of_the_three_two_keywords() {
            let mut s = scenario(json!({ "p1": { "hand": [{ "def": CARD, "radiant": true }, FILLER] }, "p2": { "hand": [FILLER] } }));
            step_param(s.card_mut(CARD), "keywords", 1);
            s.play(CARD, json!({}));
            let made = tokens(&s);
            assert_eq!(made.len(), 3);
            for token in &made {
                assert_eq!(gained(&s, &token.id).len(), 2);
            }
        }
    }
}
