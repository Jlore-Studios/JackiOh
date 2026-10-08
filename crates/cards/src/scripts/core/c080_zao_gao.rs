//! #80 Zao Gao (SPEC §8.3, §5.2, §7; R11, R16, R21, R64, R215, R275, R276, R354). Spell, CN, cost 2.
//!   Base:    "Discard 2 random cards. Summon 2 Rush Tokens, each with 2 random keywords."
//!   Radiant: "Discard 2 random cards. Summon 2 Radiant Rush Tokens, each with 3 random keywords."
//!
//! Patch v0.1.1 (issue #27) changed three things, and R354 records how they are read: the discard is
//! random ("not of your choice", so R682's random default, never a prompt), the Radiant face's tokens
//! gain a third keyword on top of being Radiant ("Modify": the change is added to the face the
//! Radiant pass gave it, R276), and the card is tagged CN.
//!
//! The discard is `discardRandom`: each of the two cards is one uniform pick from the match rng over
//! the hand as it then stands, so the two are different cards, and fewer than 2 in hand discards what
//! there is (§8's Engine cell). A discarded card is the printed card again, keeping only its
//! `costMod`, `costOverride` and radiant flag (R215), and a unit-token card among the discards ceases
//! to exist instead of reaching the graveyard (R11) — `discard`'s rules. Nothing asks the player
//! anything, so an Echo repeat discards at random again, and an empty hand simply summons.
//!
//! R21: each token rolls DISTINCT keywords from the pool (thirteen with R346's Pierce and R636's Windfury), and the two
//! tokens roll independently. `grantRandomKeywords` is that rule already — it recomputes the pool per
//! draw off the unit's §10.4 keywords, so it never repeats inside one grant and never offers a keyword
//! the unit already has: a Rush Token (printed Rush, §7) draws its two from the other eleven, and a
//! Radiant one (printed Rush and Cleave) its three from the other ten. §8's Engine cell puts the order
//! in words: "each token is summoned Radiant and then rolls its keywords" — `summon` sets the flag as
//! it creates the card and rolls only once it has landed, so the roll reads the Radiant face.
//!
//! R64: a summon with no named zone takes the leftmost empty, unlocked, unreserved unit zone and
//! fizzles silently when the row has none, so a board with one free zone gets one token.
//!
//! The keywords are rolled by `summon` itself (`randomKeywords`), since `summon` returns nothing a
//! script can reference and `TargetSpec` has no "last summoned" case: rolling inside the summon keeps
//! the rng draws adjacent to the summon they belong to (replay parity, §9.3) and has no fizzle
//! hazard. The discards come first, so their draws precede the tokens' in the rng stream.

use jackioh_engine::effects::{discard_random, summon};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-080";

/// §7: the token this card makes, taken from the catalog rather than restated (TS
/// `cardDef("core-t-rush").id`, read once as the card's scripts are built).
fn rush_token_id() -> String {
    crate::card_def("core-t-rush").id
}

const TOKEN_COUNT: i32 = 2;

/// What a face summons: whether the tokens are Radiant (R276, R354), and how many keywords each rolls
/// (R21) — the declared number `keywords`, 2 and 3 on the Radiant face.
#[derive(Clone, Copy)]
struct Tokens {
    radiant: bool,
    keywords: i32,
}

/// One Rush Token with its rolled keywords (R21, R64).
fn rush_token(def_id: &str, tokens: Tokens) -> Effect {
    summon(json_as(json!({
        "defId": def_id,
        "radiant": tokens.radiant,
        "randomKeywords": tokens.keywords
    })))
}

/// The faces differ only in the tokens they summon. The discard is the declared number `discard`
/// (R386), less being better.
fn zao_gao(rush_token_def: String, radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let tokens = Tokens {
                radiant,
                keywords: param(&*ctx, "keywords"),
            };
            // R16, R354: "random" is stated, so the match rng picks and nobody is asked.
            let mut effects = vec![discard_random(json_as(json!({ "count": param(&*ctx, "discard") })))];
            effects.extend((0..TOKEN_COUNT).map(|_| rush_token(&rush_token_def, tokens)));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let rush = rush_token_id();
    CardScripts {
        base: zao_gao(rush.clone(), false),
        radiant: zao_gao(rush, true),
    }
}

// #80 Zao Gao — SPEC §8.3, R11, R16, R21, R64, R215, R275, R276, R354, §5.2, §7, §9.3.
//
// BUILD M4-T4: "Discards 2 random cards; two Rush Tokens each with two distinct pool keywords;
// radiant the tokens are Radiant 6/6 Rush, Cleave, roll three keywords, and roll no keyword they
// have".
//
// Patch v0.1.1 (issue #27, R354): the discard is random, not the player's choice, so no prompt opens;
// the Radiant face's Radiant Rush Tokens each roll a third keyword; and the card is tagged CN.
// Radiant: "Discard 2 random cards. Summon 2 Radiant Rush Tokens, each with 3 random keywords." Each
// token is summoned on its Radiant face (§7: 6/6, Rush, Cleave) and then rolls its three keywords,
// which never repeat one it has (R21) — so neither Rush nor Cleave is ever one of them.
#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexSet;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const ZAO_GAO: &str = "core-080";
    const RUSH_TOKEN: &str = "core-t-rush";

    const DISCARDABLE: [&str; 3] = ["core-001", "core-002", "core-003"];
    const LIBRARY: [&str; 4] = ["core-008", "core-008", "core-008", "core-008"];

    /// R21's pool as keyword KINDS; "Armor 1" is its one numbered entry (§6.1).
    fn pool_kinds() -> IndexSet<String> {
        RANDOM_KEYWORD_POOL
            .iter()
            .map(|entry| if *entry == "Armor 1" { "Armor" } else { *entry })
            .map(str::to_string)
            .collect()
    }

    /// The live keyword list of the unit in a lane, read through `viewFor`'s §10.4 layers.
    fn keyword_kinds(s: &Scenario, lane: i32) -> Vec<String> {
        let units = s.view(P1).you.units;
        let view = usize::try_from(lane - 1).ok().and_then(|index| units.get(index).cloned()).flatten();
        let Some(view) = view else {
            panic!("p1 has no unit in lane {lane}");
        };
        view.keywords.iter().map(|keyword| keyword.kind().as_str().to_string()).collect()
    }

    /// §7: the Rush Token's printed keywords on each face.
    const BASE_PRINTED: [&str; 1] = ["Rush"];
    const RADIANT_PRINTED: [&str; 2] = ["Rush", "Cleave"];

    /// R21 on one token: its printed keywords (§7) plus exactly `count` more, all distinct, all from the
    /// pool, and none of them one it already printed.
    fn expect_pool_keywords(s: &Scenario, lane: i32, count: usize, printed: &[&str]) {
        let kinds = keyword_kinds(s, lane);
        for kind in printed {
            assert!(kinds.iter().any(|have| have == kind), "{kinds:?} lacks {kind}");
        }
        // R21: "no repeats on one unit".
        assert_eq!(kinds.iter().collect::<IndexSet<_>>().len(), kinds.len(), "{kinds:?}");
        let granted: Vec<&String> = kinds.iter().filter(|kind| !printed.contains(&kind.as_str())).collect();
        assert_eq!(granted.len(), count, "{kinds:?}");
        let pool = pool_kinds();
        for kind in &granted {
            assert!(pool.contains(*kind), "{kind} is not in the pool");
        }

        // The `keywordGranted` events for this token are the roll itself, never a printed keyword.
        let token_id = s.unit(P1, lane).expect("a token in that lane").id.clone();
        let rolled: Vec<String> = s
            .events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::KeywordGranted { instance_id, keyword, .. } if *instance_id == token_id => {
                    Some(keyword.kind().as_str().to_string())
                }
                _ => None,
            })
            .collect();
        assert_eq!(rolled.len(), count, "{rolled:?}");
        for kind in printed {
            assert!(!rolled.iter().any(|got| got == kind), "{rolled:?} holds {kind}");
        }
    }

    fn occupied_lanes(s: &Scenario) -> Vec<i32> {
        s.state()
            .players
            .p1
            .units
            .iter()
            .enumerate()
            .filter_map(|(index, pile)| pile.as_ref().map(|_| index as i32 + 1))
            .collect()
    }

    fn board(radiant_face: bool, hand: &[&str], seed: Option<&str>) -> Scenario {
        let mut cards = vec![json!({ "def": ZAO_GAO, "radiant": radiant_face })];
        cards.extend(hand.iter().map(|card| json!(card)));
        let mut options = json!({
            "p1": {
                "hand": cards,
                "library": LIBRARY
            },
            // R82: the opponent keeps something to do, so nothing auto-ends under the assertions.
            "p2": { "hand": ["core-005"], "field": ["core-019"], "library": LIBRARY }
        });
        if let Some(seed) = seed {
            options["seed"] = json!(seed);
        }
        scenario(options)
    }

    /// The def ids of p1's graveyard, sorted.
    fn graveyard(s: &Scenario) -> Vec<String> {
        let mut ids: Vec<String> = s.pile(P1, "graveyard").iter().map(|card| card.def_id.clone()).collect();
        ids.sort();
        ids
    }

    mod r354_n80_zao_gao_card_data {
        use super::*;

        #[test]
        fn r354_is_tagged_cn_and_prints_the_patch_s_random_discard_on_both_faces() {
            let def = crate::card_def(ID);
            assert_eq!(def.tags, vec![Tag::Cn]);
            // The faces as printed: their declared numbers filled in (R386, R482).
            assert_eq!(
                fill_params(&def, FaceKind::Base, None),
                "Discard 2 random cards. Summon 2 Rush Tokens, each with 2 random keywords."
            );
            assert_eq!(
                fill_params(&def, FaceKind::Radiant, None),
                "Discard 2 random cards. Summon 2 Radiant Rush Tokens, each with 3 random keywords."
            );
        }
    }

    #[test]
    fn r386_an_upgrade_discards_1_and_a_degrade_3() {
        for (upgrade, discards) in [(true, 1), (false, 3)] {
            crate::register_all();
            let mut s = board(false, &DISCARDABLE, None);
            let moved = if upgrade {
                crate::upgrade_number(&mut s, ZAO_GAO, "discard")
            } else {
                crate::degrade_number(&mut s, ZAO_GAO, "discard")
            };
            assert_eq!(moved, discards);
            s.play(ZAO_GAO, json!({}));
            let discarded = s.pile(P1, "graveyard").into_iter().filter(|card| card.def_id != ZAO_GAO).count();
            assert_eq!(discarded, discards as usize);
        }
    }

    #[test]
    fn r386_an_upgrade_rolls_3_keywords_a_token_and_a_degrade_1() {
        for (upgrade, keywords) in [(true, 3), (false, 1)] {
            crate::register_all();
            let mut s = board(false, &DISCARDABLE, None);
            let moved = if upgrade {
                crate::upgrade_number(&mut s, ZAO_GAO, "keywords")
            } else {
                crate::degrade_number(&mut s, ZAO_GAO, "keywords")
            };
            assert_eq!(moved, keywords);
            s.play(ZAO_GAO, json!({}));
            expect_pool_keywords(&s, 1, keywords as usize, &BASE_PRINTED);
            expect_pool_keywords(&s, 2, keywords as usize, &BASE_PRINTED);
        }
    }

    mod n80_zao_gao_base {
        use super::*;

        #[test]
        fn r354_r16_the_discard_is_random_no_prompt_opens_and_two_of_the_hand_go_to_the_graveyard() {
            crate::register_all();
            let mut s = board(false, &DISCARDABLE, None);

            s.play(ZAO_GAO, json!({}));

            assert!(s.state().pending.is_none());
            assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::PromptOpened));
            // Two different cards of the three, and Zao Gao itself (§10.5 step 7).
            let discarded: Vec<CardInstance> =
                s.pile(P1, "graveyard").into_iter().filter(|card| card.def_id != ZAO_GAO).collect();
            assert_eq!(discarded.len(), 2);
            assert_eq!(discarded.iter().map(|card| card.id.clone()).collect::<IndexSet<_>>().len(), 2);
            for card in &discarded {
                assert!(DISCARDABLE.contains(&card.def_id.as_str()), "{}", card.def_id);
            }
            assert_eq!(s.pile(P1, "hand").len(), 1);
            s.expect_events(json!(["cardPlayed", "discarded", "discarded", "summoned", "summoned"]));
        }

        #[test]
        fn r354_the_picks_come_off_the_match_rng_the_same_seed_discards_the_same_two_cards_and_a_seed_moves_them() {
            crate::register_all();
            let picked = |seed: &str| -> Vec<String> {
                let mut s = board(false, &DISCARDABLE, Some(seed));
                s.play(ZAO_GAO, json!({}));
                s.pile(P1, "hand").iter().map(|card| card.def_id.clone()).collect()
            };
            assert_eq!(picked("core-080-rng-a"), picked("core-080-rng-a"));
            let kept: IndexSet<Option<String>> =
                (0..12).map(|n| picked(&format!("core-080-rng-{n}")).first().cloned()).collect();
            // Over a dozen seeds, more than one card is the one left in hand: it is not a fixed choice.
            assert!(kept.len() > 1, "{kept:?}");
        }

        #[test]
        fn r64_two_rush_tokens_stand_in_the_leftmost_free_zones() {
            crate::register_all();
            let mut s = board(false, &DISCARDABLE, None);

            s.play(ZAO_GAO, json!({}));

            assert_eq!(occupied_lanes(&s), vec![1, 2]);
            assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id.clone()), Some(RUSH_TOKEN.to_string()));
            assert_eq!(s.unit(P1, 2).map(|unit| unit.def_id.clone()), Some(RUSH_TOKEN.to_string()));
            assert_eq!(s.state().work.len(), 0);
        }

        #[test]
        fn s7_the_base_face_s_tokens_are_base_rush_tokens_3_3_not_radiant() {
            crate::register_all();
            let mut s = board(false, &DISCARDABLE, None);

            s.play(ZAO_GAO, json!({}));

            for lane in [1, 2] {
                let token = s.unit(P1, lane).expect("a token in that lane").clone();
                assert!(!token.radiant);
                s.expect_stats(&token, json!({ "attack": 3, "health": 3, "maxHealth": 3 }));
            }
        }

        #[test]
        fn r21_each_rush_token_carries_two_distinct_keywords_from_the_pool_rolled_independently() {
            crate::register_all();
            let mut s = board(false, &DISCARDABLE, None);

            s.play(ZAO_GAO, json!({}));

            expect_pool_keywords(&s, 1, 2, &BASE_PRINTED);
            expect_pool_keywords(&s, 2, 2, &BASE_PRINTED);
        }

        #[test]
        fn s8_3_fewer_than_2_in_hand_what_there_is_is_discarded() {
            crate::register_all();
            let mut s = board(false, &[DISCARDABLE[0]], None);

            s.play(ZAO_GAO, json!({}));

            let mut expected = vec![DISCARDABLE[0].to_string(), ZAO_GAO.to_string()];
            expected.sort();
            assert_eq!(graveyard(&s), expected);
            assert_eq!(s.pile(P1, "hand").len(), 0);
            assert_eq!(occupied_lanes(&s), vec![1, 2]);
        }

        #[test]
        fn an_empty_hand_discards_nothing_and_the_two_tokens_are_summoned_all_the_same() {
            crate::register_all();
            let mut s = board(false, &[], None);

            s.play(ZAO_GAO, json!({}));

            assert!(s.state().pending.is_none());
            assert_eq!(graveyard(&s), vec![ZAO_GAO.to_string()]);
            assert_eq!(occupied_lanes(&s), vec![1, 2]);
        }

        #[test]
        fn r11_a_unit_token_card_discarded_from_the_hand_ceases_to_exist_instead_of_reaching_the_graveyard() {
            crate::register_all();
            let mut s = board(false, &[RUSH_TOKEN, RUSH_TOKEN], None);

            s.play(ZAO_GAO, json!({}));

            assert_eq!(s.pile(P1, "hand").len(), 0);
            assert_eq!(graveyard(&s), vec![ZAO_GAO.to_string()]);
        }

        #[test]
        fn r64_a_nearly_full_board_gets_fewer_tokens_and_the_extra_summon_fizzles() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [ZAO_GAO, DISCARDABLE[0], DISCARDABLE[1], DISCARDABLE[2]],
                    // Four of the five unit zones are taken, so only one token fits.
                    "field": ["core-019", "core-019", "core-019", "core-019"],
                    "library": LIBRARY
                },
                "p2": { "hand": ["core-005"], "field": ["core-019"], "library": LIBRARY }
            }));

            s.play(ZAO_GAO, json!({}));

            assert_eq!(occupied_lanes(&s), vec![1, 2, 3, 4, 5]);
            assert_eq!(s.unit(P1, 5).map(|unit| unit.def_id.clone()), Some(RUSH_TOKEN.to_string()));
            // The discard happened either way: it is not conditional on the summons.
            assert_eq!(s.pile(P1, "graveyard").len(), 3); // two discards plus Zao Gao itself
        }

        #[test]
        fn r64_a_full_board_summons_nothing_and_the_discard_still_happens() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [ZAO_GAO, DISCARDABLE[0], DISCARDABLE[1], DISCARDABLE[2]],
                    "field": ["core-019", "core-019", "core-019", "core-019", "core-019"],
                    "library": LIBRARY
                },
                "p2": { "hand": ["core-005"], "field": ["core-019"], "library": LIBRARY }
            }));

            s.play(ZAO_GAO, json!({}));

            assert_eq!(s.state().players.p1.units.iter().filter(|pile| pile.is_some()).count(), 5);
            assert_ne!(s.unit(P1, 5).map(|unit| unit.def_id.clone()), Some(RUSH_TOKEN.to_string()));
            assert_eq!(s.pile(P1, "hand").len(), 1);
        }
    }

    mod n80_zao_gao_radiant {
        use super::*;

        #[test]
        fn r276_the_radiant_face_is_its_own_script_not_the_base_object() {
            // TS `expect(radiant).not.toBe(base)`: object identity, here the identity of each face's Cry.
            let scripts = script();
            let (Some(base_cry), Some(radiant_cry)) = (&scripts.base.cry, &scripts.radiant.cry) else {
                panic!("both faces have a Cry");
            };
            assert!(!Arc::ptr_eq(base_cry, radiant_cry));
        }

        #[test]
        fn r354_a_radiant_zao_gao_discards_two_random_cards_and_summons_two_radiant_rush_tokens() {
            crate::register_all();
            let mut s = board(true, &DISCARDABLE, None);

            s.play(ZAO_GAO, json!({}));

            assert!(s.state().pending.is_none());
            assert_eq!(
                s.pile(P1, "graveyard").iter().filter(|card| card.def_id != ZAO_GAO).count(),
                2
            );
            assert_eq!(occupied_lanes(&s), vec![1, 2]);
            for lane in [1, 2] {
                let token = s.unit(P1, lane).expect("a token in that lane").clone();
                assert_eq!(token.def_id, RUSH_TOKEN);
                // §7: the Radiant Rush Token is 6/6 with Rush and Cleave.
                assert!(token.radiant);
                s.expect_stats(&token, json!({ "attack": 6, "health": 6, "maxHealth": 6 }));
            }
        }

        #[test]
        fn r354_r21_a_radiant_zao_gao_s_tokens_roll_three_pool_keywords_each_never_rush_or_cleave() {
            crate::register_all();
            let mut s = board(true, &DISCARDABLE, None);

            s.play(ZAO_GAO, json!({}));

            expect_pool_keywords(&s, 1, 3, &RADIANT_PRINTED);
            expect_pool_keywords(&s, 2, 3, &RADIANT_PRINTED);
        }

        #[test]
        fn r21_the_roll_reads_the_radiant_face_on_every_seed_cleave_is_never_rolled_onto_a_token_that_prints_it() {
            crate::register_all();
            // Each token's three draws come from the ten pool keywords a Radiant Rush Token lacks; were the
            // roll made before the flag set (or off the base face), Cleave would be offered and, over
            // enough seeds, rolled.
            const SEEDS: i32 = 40;
            for n in 0..SEEDS {
                let mut s = board(true, &DISCARDABLE, Some(&format!("core-080-radiant-roll-{n}")));
                s.play(ZAO_GAO, json!({}));
                expect_pool_keywords(&s, 1, 3, &RADIANT_PRINTED);
                expect_pool_keywords(&s, 2, 3, &RADIANT_PRINTED);
            }
        }

        #[test]
        fn r215_the_spent_spell_goes_to_the_graveyard_radiant() {
            crate::register_all();
            let mut s = board(true, &DISCARDABLE, None);
            let self_card = s.card(ZAO_GAO).clone();
            assert!(self_card.radiant);

            s.play(ZAO_GAO, json!({}));

            s.expect_in_zone(&self_card, "graveyard");
            assert!(s.card(&self_card).radiant);
        }

        #[test]
        fn an_empty_hand_on_the_radiant_face_discards_nothing_and_its_tokens_are_radiant() {
            crate::register_all();
            let mut s = board(true, &[], None);

            s.play(ZAO_GAO, json!({}));

            assert!(s.state().pending.is_none());
            assert_eq!(occupied_lanes(&s), vec![1, 2]);
            assert_eq!(s.unit(P1, 1).map(|unit| unit.radiant), Some(true));
            assert_eq!(s.unit(P1, 2).map(|unit| unit.radiant), Some(true));
        }

        #[test]
        fn r64_a_nearly_full_board_gets_one_radiant_token_and_the_extra_summon_fizzles() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": ZAO_GAO, "radiant": true }, DISCARDABLE[0], DISCARDABLE[1], DISCARDABLE[2]],
                    "field": ["core-019", "core-019", "core-019", "core-019"],
                    "library": LIBRARY
                },
                "p2": { "hand": ["core-005"], "field": ["core-019"], "library": LIBRARY }
            }));

            s.play(ZAO_GAO, json!({}));

            assert_eq!(occupied_lanes(&s), vec![1, 2, 3, 4, 5]);
            let token = s.unit(P1, 5).expect("a token in lane 5").clone();
            assert_eq!(token.def_id, RUSH_TOKEN);
            assert!(token.radiant);
            expect_pool_keywords(&s, 5, 3, &RADIANT_PRINTED);
        }
    }
}
