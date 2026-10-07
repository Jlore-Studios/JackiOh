//! C #85 King Wagtoggle (SPEC §8.6 row 85). (4) Unit, Legendary, 5/5 → 10/10.
//!   Base:    "Cry: Swap decks with your opponent."
//!   Radiant: "Cry: Swap decks with your opponent. Then Recruit {recruits|card|cards}." — recruits 1
//!   Engine:  "R73's library swap: contents swap, each swapped card's owner becomes the player whose
//!            deck now holds it (R12), and fatigue counters stay. The Radiant recruits (Recruit, §6.3)
//!            the first permanent from the top of your new deck. Tunes: Radiant recruits 1 ↑."
//!
//! `swapLibrary` is #87 Pocket Chaos's swap (R73): the libraries change places whole, empty ones too, and
//! each card's owner becomes the player whose library now holds it (R12); the fatigue counts are the
//! players' and stay. What each player may know of their new library is R311's: the list shows only
//! what its owner was shown going in, and no event carries a position. The Radiant face then Recruits
//! (§6.3) from the new library: the first permanent from the top, summoned per R64 (a Trap face-down,
//! R33); none, or a full row, summons nothing. "Recruit N" is N scans, the declared `recruits` read
//! through `param` (R386).
//!
//! Rulings: R12, R33, R73, R311, R386. Its proof: `test/classic/085-king-wagtoggle.test.ts`.

use jackioh_engine::effects::{recruit, swap_library};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-085";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|_ctx| vec![swap_library()])),
        ..Script::default()
    };

    let radiant = Script {
        cry: Some(hook(|ctx| {
            vec![
                swap_library(),
                recruit(json_as(json!({ "count": param(&*ctx, "recruits") }))),
            ]
        })),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C #85 King Wagtoggle (SPEC §8.6 row 85; BUILD M9 row C 85). (4) Unit, Legendary, 5/5 → 10/10: Cry:
// swap decks with your opponent. Radiant: then Recruit {recruits} (1, ↑).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const KING: &str = "classic-085";
    const VANILLA: &str = "core-008"; // 4/4
    const TIMMY: &str = "core-011";
    const MENACE: &str = "core-019";
    const STOCKPILE: &str = "core-005";
    const NOTEBOOK: &str = "core-051-1"; // Spell 1: draw 1
    const MANA_WELL: &str = "core-006"; // Field Spell
    const SHEEPISH: &str = "core-041"; // Trap

    use crate::scenario;

    use crate::js;

    use crate::matches_object;

    fn must<T>(value: Option<T>, what: &str) -> T {
        match value {
            Some(found) => found,
            None => panic!("missing: {what}"),
        }
    }

    /// TS `const P2_HAND: SideSetup = { hand: [STOCKPILE] }`.
    fn p2_hand() -> Value {
        json!({ "hand": [STOCKPILE] })
    }

    /// TS `{ ...P2_HAND, library }`.
    fn p2_with(library: Value) -> Value {
        let mut side = p2_hand();
        side["library"] = library;
        side
    }

    fn library_ids(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.pile(player, "library").into_iter().map(|card| card.id).collect()
    }

    fn library_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.pile(player, "library").into_iter().map(|card| card.def_id).collect()
    }

    /// base
    mod base {
        use super::*;

        /// R73 Cry: the decks swap whole, and each card's owner becomes the player whose deck now holds it (R12)
        #[test]
        fn r73_cry_the_decks_swap_whole_and_each_card_s_owner_becomes_the_player_whose_deck_now_holds_it_r12() {
            let mut s = scenario(json!({
                "p1": { "hand": [KING, STOCKPILE], "library": [VANILLA, TIMMY] },
                "p2": p2_with(json!([MENACE, STOCKPILE, MANA_WELL]))
            }));
            let theirs = library_ids(&s, P2);
            let mine = library_ids(&s, P1);
            s.play(KING, json!({}));
            assert_eq!(library_ids(&s, P1), theirs);
            assert_eq!(library_ids(&s, P2), mine);
            assert!(s.pile(P1, "library").iter().all(|card| card.owner == P1));
            assert!(s.pile(P2, "library").iter().all(|card| card.owner == P2));
            // No event carries a position: the swap is one `swapped`.
            let swaps: Vec<Value> = s
                .last_events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Swapped { .. }))
                .map(js)
                .collect();
            assert_eq!(json!(swaps), json!([{ "type": "swapped", "what": "library" }]));
            assert!(!s.last_events().iter().any(|event| matches!(event, GameEvent::ShuffledIn { .. })));
        }

        /// R73 the fatigue counts stay with the players, and an empty deck swaps too
        #[test]
        fn r73_the_fatigue_counts_stay_with_the_players_and_an_empty_deck_swaps_too() {
            let mut s = scenario(json!({
                "p1": { "hand": [NOTEBOOK, KING, STOCKPILE], "library": [], "mana": 5 },
                "p2": p2_with(json!([MENACE, TIMMY, VANILLA]))
            }));
            s.play(NOTEBOOK, json!({}));
            assert_eq!(s.state().players.p1.fatigue_count, 1);
            s.play(KING, json!({}));
            assert_eq!(s.pile(P1, "library").len(), 3);
            assert!(s.pile(P2, "library").is_empty());
            assert_eq!(
                [s.state().players.p1.fatigue_count, s.state().players.p2.fatigue_count],
                [1, 0]
            );
        }

        /// R311 each player's deck list names only what they were shown: the new deck is unknown cards
        #[test]
        fn r311_each_player_s_deck_list_names_only_what_they_were_shown_the_new_deck_is_unknown_cards() {
            let mut s = scenario(json!({
                "p1": { "hand": [KING, STOCKPILE], "library": [VANILLA, TIMMY] },
                "p2": p2_with(json!([MENACE, STOCKPILE, MANA_WELL]))
            }));
            let shown = js(&s.view(P1))["you"]["ownLibrary"]["cards"].as_array().map(|cards| cards.len());
            assert!(shown.is_some_and(|count| count > 0));
            s.play(KING, json!({}));
            for viewer in [P1, P2] {
                let list = js(&s.view(viewer))["you"]["ownLibrary"].clone();
                assert_eq!(list["cards"], json!([]));
                assert_eq!(list["unknown"], json!(s.pile(viewer, "library").len()));
            }
        }

        /// §8.6 a 5/5
        #[test]
        fn s8_6_a_5_5() {
            let mut s = scenario(json!({ "p1": { "hand": [KING, STOCKPILE], "library": [VANILLA] }, "p2": p2_hand() }));
            s.play(KING, json!({}));
            s.expect_stats(KING, json!({ "attack": 5, "health": 5 }));
            assert!(s.unit(P1, 2).is_none());
        }
    }

    /// radiant
    mod radiant {
        use super::*;

        /// §6.3 then it Recruits the first permanent from your new deck
        #[test]
        fn s6_3_then_it_recruits_the_first_permanent_from_your_new_deck() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": KING, "radiant": true }, STOCKPILE], "library": [TIMMY] },
                "p2": p2_with(json!([STOCKPILE, MENACE, VANILLA]))
            }));
            s.play(KING, json!({}));
            s.expect_stats(KING, json!({ "attack": 10, "health": 10 }));
            let recruit = must(s.unit(P1, 2), "the recruit");
            assert_eq!(recruit.def_id, MENACE);
            assert_eq!(recruit.owner, P1);
            assert_eq!(library_defs(&s, P1), [STOCKPILE, VANILLA]);
        }

        /// §6.3 no permanent in the new deck, or a full row, recruits nothing
        #[test]
        fn s6_3_no_permanent_in_the_new_deck_or_a_full_row_recruits_nothing() {
            let mut none = scenario(json!({
                "p1": { "hand": [{ "def": KING, "radiant": true }, STOCKPILE], "library": [TIMMY] },
                "p2": p2_with(json!([STOCKPILE]))
            }));
            none.play(KING, json!({}));
            assert!(none.unit(P1, 2).is_none());

            let mut full = scenario(json!({
                "p1": {
                    "hand": [{ "def": KING, "radiant": true }, STOCKPILE],
                    "field": [
                        { "def": TIMMY, "lane": 1 },
                        { "def": TIMMY, "lane": 2 },
                        { "def": TIMMY, "lane": 3 },
                        { "def": TIMMY, "lane": 4 }
                    ],
                    "library": []
                },
                "p2": p2_with(json!([MENACE]))
            }));
            full.play(KING, json!({ "zone": 5 }));
            assert_eq!(library_defs(&full, P1), [MENACE]);
        }

        /// R33 a Trap it recruits lands face-down, and the other player is not told which
        #[test]
        fn r33_a_trap_it_recruits_lands_face_down_and_the_other_player_is_not_told_which() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": KING, "radiant": true }, STOCKPILE], "library": [TIMMY] },
                "p2": p2_with(json!([SHEEPISH]))
            }));
            s.play(KING, json!({}));
            let trap = must(s.backrow(P1, 1), "the recruited Trap");
            assert_eq!(trap.def_id, SHEEPISH);
            assert!(trap.face_up != Some(true));
            let theirs = js(&s.view(P2));
            assert!(matches_object(&theirs["opponent"]["backrow"][0], &json!({ "faceDown": true })));
            assert!(!serde_json::to_string(&theirs["events"]).expect("serialisable").contains(SHEEPISH));
        }

        /// R386 its tuned number: one Upgrade makes it Recruit 2
        #[test]
        fn r386_its_tuned_number_one_upgrade_makes_it_recruit_2() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": KING, "radiant": true }, STOCKPILE], "library": [TIMMY] },
                "p2": p2_with(json!([MENACE, VANILLA, TIMMY]))
            }));
            step_param(s.card_mut(KING), "recruits", 1);
            s.play(KING, json!({}));
            assert_eq!(
                [s.unit(P1, 2).map(|card| card.def_id), s.unit(P1, 3).map(|card| card.def_id)],
                [Some(MENACE.to_string()), Some(VANILLA.to_string())]
            );
            assert_eq!(library_defs(&s, P1), [TIMMY]);
        }
    }
}
