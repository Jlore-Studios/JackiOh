//! M #50 CN Tech (SPEC §8.8 row 50): (0) Spell, CN, Rare.
//!
//! Base:    "Add the highest win rate card to your hand."
//! Radiant: "Add the highest win rate card to your hand. It is Radiant."
//! Engine: ME-STATS (docs/meditative-set.md M5, MD-D1, MD-D2) — the compiled table
//! `crates/cards/data/win_rates.json` (R377's in-deck wins and games per card id) read through
//! `win_rate_leader`, never CN Tech itself (R387); a random non-token card when no row qualifies
//! (R142). The table ships empty (provisional, no live figures offline), so the fallback is what a
//! fresh patch plays until someone runs `cargo jackioh winrates`.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-050";

fn tech(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| match win_rate_leader(ID) {
            Some(id) => vec![add_to_hand(json_as(json!({ "defId": id, "radiant": radiant })))],
            None => vec![add_random_from_catalog(json_as(
                json!({ "query": {}, "radiant": radiant }),
            ))],
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: tech(false),
        radiant: tech(true),
    }
}

// M #50 CN Tech — SPEC §8.8 row 50, BUILD M10 row M 50: "Add the highest win rate card (the
// compiled table's pick, R377's in-deck rate) to your hand (MD-D1), never itself (R387), the Radiant
// face's Radiant (MD-D3); a random card when the table names none, hidden from the opponent (R440),
// burned by a full hand".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const TECH: &str = "meditative-050";
    const LEADER: &str = "core-001"; // Big D-fender, a deckable non-token card.
    const FILLER: &str = "core-005";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    fn table(cards: Vec<(&str, i32, i32)>) -> WinRateTable {
        WinRateTable {
            patch: "v0.3.3".to_string(),
            source: WinRateSource::Provisional,
            cards: cards
                .into_iter()
                .map(|(id, wins, games)| WinRateRow {
                    id: id.to_string(),
                    wins,
                    games,
                })
                .collect(),
        }
    }

    /// p1 holds CN Tech (base unless `radiant_face`); both sides keep cards in hand so no turn
    /// auto-ends.
    fn casting(seed: &str, radiant_face: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": TECH, "radiant": radiant_face }, FILLER],
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    mod m50_cn_tech {
        use super::*;

        #[test]
        fn adds_the_highest_win_rate_card_never_itself() {
            register_win_rates(table(vec![(LEADER, 30, 40), (TECH, 40, 40)]));
            let mut s = casting("cn-tech", false);
            let before = s.hand(PlayerId::P1).len();
            s.play(TECH, json!({}));
            let added: Vec<CardInstance> = s.hand(PlayerId::P1).into_iter().skip(before - 1).collect();
            assert_eq!(added.len(), 1);
            assert_eq!(added[0].def_id, LEADER);
            assert!(!added[0].radiant);
        }

        /// The def CN Tech added to p1's hand under `rows`.
        fn picked(rows: Vec<(&str, i32, i32)>, seed: &str) -> String {
            register_win_rates(table(rows));
            let mut s = casting(seed, false);
            let before = s.hand(PlayerId::P1).len();
            s.play(TECH, json!({}));
            let added: Vec<CardInstance> = s.hand(PlayerId::P1).into_iter().skip(before - 1).collect();
            assert_eq!(added.len(), 1);
            added[0].def_id.clone()
        }

        #[test]
        fn r1100_only_cards_with_at_least_20_games_qualify() {
            // A perfect 19 games loses to a 50% rate over 20.
            assert_eq!(
                picked(vec![("core-008", 19, 19), (LEADER, 10, 20)], "cn-tech-min"),
                LEADER
            );
        }

        #[test]
        fn r1100_the_best_exact_rate_wins() {
            // 21/40 beats 20/40 by one win: compared exactly, never as a rounded float.
            assert_eq!(
                picked(vec![("core-008", 20, 40), (LEADER, 21, 40)], "cn-tech-rate"),
                LEADER
            );
        }

        #[test]
        fn r1100_a_tie_goes_to_more_games_then_to_the_catalog_id() {
            assert_eq!(
                picked(vec![("core-008", 10, 20), (LEADER, 20, 40)], "cn-tech-tie-games"),
                LEADER
            );
            assert_eq!(
                picked(vec![("core-008", 10, 20), ("core-002", 10, 20)], "cn-tech-tie-id"),
                "core-002"
            );
        }

        #[test]
        fn r1100_glitch_tokens_and_ids_no_longer_in_the_catalog_never() {
            assert_eq!(
                picked(
                    vec![
                        ("classic-t-glitch", 40, 40),
                        ("core-065-1", 40, 40),
                        ("core-999-gone", 40, 40),
                        (LEADER, 1, 40),
                    ],
                    "cn-tech-never"
                ),
                LEADER
            );
        }

        #[test]
        fn r1101_the_same_table_and_seed_give_the_same_card() {
            // The fallback draws from the match rng: the same seed makes the same card, whoever
            // runs the engine (the server natively, the client through WASM).
            let one = picked(vec![], "cn-tech-same");
            let two = picked(vec![], "cn-tech-same");
            assert_eq!(one, two);
            register_win_rates(table(vec![]));
            let mut s = casting("cn-tech-same", false);
            let cursor = s.state().rng_cursor;
            s.play(TECH, json!({}));
            assert!(
                s.state().rng_cursor > cursor,
                "the fallback drew from the match rng"
            );
        }

        #[test]
        fn the_added_card_is_hidden_from_the_opponent() {
            register_win_rates(table(vec![(LEADER, 30, 40)]));
            let mut s = casting("cn-tech-hidden", false);
            s.play(TECH, json!({}));
            let view: Value =
                serde_json::to_value(view_for(s.state(), PlayerId::P2)).expect("the view serialises");
            assert_eq!(view["opponent"]["hand"], json!({ "count": 2 }));
            let added = s
                .hand(PlayerId::P1)
                .into_iter()
                .find(|card| card.def_id == LEADER)
                .expect("the leader in hand");
            let text = view.to_string();
            assert!(!text.contains(&added.id), "the opponent never sees its id");
            assert!(!text.contains(LEADER), "nor its def");
        }

        #[test]
        fn the_radiant_face_adds_it_radiant() {
            register_win_rates(table(vec![(LEADER, 30, 40)]));
            let mut s = casting("cn-tech-radiant", true);
            let before = s.hand(PlayerId::P1).len();
            s.play(TECH, json!({}));
            let added: Vec<CardInstance> = s.hand(PlayerId::P1).into_iter().skip(before - 1).collect();
            assert_eq!(added.len(), 1);
            assert_eq!(added[0].def_id, LEADER);
            assert!(added[0].radiant);
        }

        #[test]
        fn falls_back_to_a_random_non_token_card_hidden_from_the_opponent() {
            register_win_rates(table(vec![]));
            let mut s = casting("cn-tech-fallback", false);
            let before = s.hand(PlayerId::P1).len();
            s.play(TECH, json!({}));
            let added: Vec<CardInstance> = s.hand(PlayerId::P1).into_iter().skip(before - 1).collect();
            assert_eq!(added.len(), 1);
            let def = crate::card_def(&added[0].def_id);
            assert!(!def.token, "no token (R142)");
            // The opponent sees a count, never the card (R440).
            let view: Value =
                serde_json::to_value(view_for(s.state(), PlayerId::P2)).expect("the view serialises");
            assert_eq!(view["opponent"]["hand"], json!({ "count": 2 }));
        }

        #[test]
        fn a_full_hand_burns_the_card() {
            register_win_rates(table(vec![(LEADER, 30, 40)]));
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "cn-tech-burn",
                "p1": {
                    "hand": [{ "def": TECH }, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                    "library": filler(4),
                },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            // Eleven in hand, ten once CN Tech is cast: the hand is full (HAND_CAP) as the card arrives.
            s.play(TECH, json!({}));
            assert_eq!(s.hand(PlayerId::P1).len(), 10);
            let burned: Vec<&CardInstance> = s
                .state()
                .players
                .p1
                .graveyard
                .iter()
                .filter(|card| card.def_id == LEADER)
                .collect();
            assert_eq!(burned.len(), 1);
        }

        #[test]
        fn r1101_engine_reads_compiled_table() {
            crate::register_all();
            // The compiled table ships empty (provisional): the engine reads no leader off it.
            clear_overrides();
            assert_eq!(win_rate_leader(TECH), None);
            assert_eq!(crate::WIN_RATES.source, WinRateSource::Provisional);
            assert!(crate::WIN_RATES.cards.is_empty());
        }
    }
}
