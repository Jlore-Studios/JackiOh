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
                    "hand": [{ "def": TECH }, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                    "library": filler(4),
                },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
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
