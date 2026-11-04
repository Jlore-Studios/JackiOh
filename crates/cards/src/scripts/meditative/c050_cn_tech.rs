//! M #50 CN Tech (SPEC §8.8 row 50): (0) Spell, CN, Rare.
//!
//! Base:    "Add the highest win rate card to your hand."
//! Radiant: "Add the highest win rate card to your hand. It is Radiant."
//! Engine: ME-STATS (docs/meditative-set.md M5, MD-D1) — the game's win-rate snapshot, the server's
//! cached card stats as the match started (R3049000; R377's in-deck wins and games per card id), read
//! through `win_rate_leader`: the highest Wilson lower bound (R3049001), never CN Tech itself (R387).
//! A game handed no snapshot (practice, hotseat, the Sandbox, the run) reads its shipped catalog's
//! frozen table, which a replay of a game recorded before #3049 folds against (R1266000), else none.
//!
//! The fallback (#2521, R2521007): when no row qualifies it adds `FALLBACK_LEADER`, Core #35 Lunar
//! Eclipse, the leader of the table every version since v0.4.0 has shipped (v0.4.0's provisional run,
//! 33 wins in 48 games), Radiant on the Radiant face, and draws nothing: no random card any more (it
//! was one before, R142).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-050";

/// R2521007: the card added when the table names no leader: Core #35 Lunar Eclipse, 33/48, the
/// leader of `crates/cards/patches/win_rates/v0.5.9b.json` (v0.4.0's provisional run).
pub const FALLBACK_LEADER: &str = "core-035";

fn tech(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let id = win_rate_leader(&*ctx.state, ID).unwrap_or_else(|| FALLBACK_LEADER.to_string());
            vec![add_to_hand(json_as(json!({ "defId": id, "radiant": radiant })))]
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
// game's snapshot's pick, R377's in-deck rate) to your hand (MD-D1), never itself (R387), the Radiant
// face's Radiant (MD-D3); Lunar Eclipse when the table names none (R2521007), no random card, hidden
// from the opponent (R440), burned by a full hand".
#[cfg(test)]
mod tests {
    use super::*;
    use crate::filler;
    use crate::js;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const TECH: &str = "meditative-050";
    const LEADER: &str = "core-001"; // Big D-fender, a deckable non-token card.
    const FILLER: &str = "core-005";

    fn table(cards: Vec<(&str, i32, i32)>) -> WinRateTable {
        WinRateTable {
            patch: "v0.2.7".to_string(),
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

        const NA_TECH: &str = "meditativeplus-002";
        /// R1266000: the earlier version a game is played on, and the build that folds it.
        const PLAYED_ON: &str = "v-r1266000-before";
        const BUILD: &str = "v-r1266000-build";
        /// The actions a game runs to, if it has not ended first.
        const STEPS: usize = 120;
        /// The dearest card `tech_deck` fills with, so CN Tech and NA Tech are cast early.
        const FILL_COST: i32 = 1;

        /// CN Tech and NA Tech beside the first deckable cards of at most `FILL_COST`, each once
        /// (§2.6 L3).
        fn tech_deck() -> Vec<String> {
            let mut deck = vec![TECH.to_string(), NA_TECH.to_string()];
            deck.extend(
                crate::CATALOG
                    .values()
                    .filter(|def| deckable(def) && query_cost(def) <= FILL_COST)
                    .filter(|def| def.id != TECH && def.id != NA_TECH)
                    .take(DECK_SIZE as usize - deck.len())
                    .map(|def| def.id.clone()),
            );
            deck
        }

        /// R1266000: a game of two `tech_deck`s on `PLAYED_ON`, casting a Tech whenever one can be and a
        /// random legal action otherwise: its input and final hash.
        fn played_on_before(seed: &str) -> (ReplayInput, String) {
            let shipped = shipped_catalog(PLAYED_ON).expect("the test carries PLAYED_ON");
            with_shipped(Some(shipped), || {
                let mut game = ReplayInput {
                    seed: seed.to_string(),
                    decks: (tech_deck(), tech_deck()),
                    ..ReplayInput::default()
                };
                let mut state = fold(&game).state;
                let mut policy = Rng::new(&format!("policy-{seed}"), 0);
                while state.result.is_none() && game.log.len() < STEPS {
                    let player = seat_to_act(&state).unwrap_or(state.active);
                    let actions: Vec<ActionBody> = legal_actions(&state, player)
                        .into_iter()
                        .filter(|action| {
                            !matches!(
                                action.action_type(),
                                ActionType::Concede | ActionType::OfferDraw | ActionType::AnswerDraw
                            )
                        })
                        .collect();
                    let tech = actions.iter().find(|action| {
                        matches!(action, ActionBody::Play { instance_id, .. }
                            if find_instance(&state, instance_id)
                                .is_some_and(|card| card.def_id == TECH || card.def_id == NA_TECH))
                    });
                    let chosen = match tech {
                        Some(action) => action.clone(),
                        None => actions[policy.int(actions.len() as i32) as usize].clone(),
                    };
                    let action = Action::new(chosen, player, format!("a{}", game.log.len()));
                    let result = reduce(&state, &action);
                    assert_eq!(result.error, None, "{seed}");
                    game.log.push(action);
                    state = result.state;
                }
                (game, hash_state(&state))
            })
        }

        #[test]
        fn r1266000_a_game_on_an_earlier_version_folds_against_the_table_it_was_played_with() {
            crate::register_all();
            register_shipped(vec![ShippedCatalog::new(
                PLAYED_ON,
                crate::CATALOG.clone(),
                IndexMap::new(),
            )]);
            // Played on a version that shipped with the empty table.
            register_win_rates(table(vec![]));
            let (game, hash) = played_on_before("r1266000-fold");
            assert_eq!(
                hash_state(&fold(&game).state),
                hash,
                "the build folds it alike while its table is empty too"
            );
            register_win_rates(table(vec![(LEADER, 30, 40), ("core-008", 10, 40)]));
            assert_ne!(
                hash_state(&fold(&game).state),
                hash,
                "the build's table plays the game another way, or this proves nothing"
            );
            let opened = replay_open(
                &game,
                &ReplayRecord {
                    catalog_version: PLAYED_ON.to_string(),
                    final_hash: hash,
                },
                BUILD,
            );
            assert!(matches!(opened, ReplayOpen::Ready { .. }), "{opened:?}");
            clear_overrides();
        }

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
        fn r3049000_adds_the_leader_of_the_snapshot_the_game_was_handed() {
            register_win_rates(table(vec![("core-008", 30, 40)]));
            let mut s = scenario(json!({
                "seed": "cn-tech-snapshot",
                "p1": { "hand": [{ "def": TECH }, FILLER], "library": filler(4) },
                "p2": { "hand": [FILLER], "library": filler(4) },
                "winRates": table(vec![(LEADER, 30, 40), ("core-008", 10, 40)]),
            }));
            // A table registered after the game was made changes nothing: the game reads its own.
            register_win_rates(table(vec![("core-008", 40, 40)]));
            let before = s.hand(PlayerId::P1).len();
            s.play(TECH, json!({}));
            let added: Vec<CardInstance> = s.hand(PlayerId::P1).into_iter().skip(before - 1).collect();
            assert_eq!(added.len(), 1);
            assert_eq!(added[0].def_id, LEADER);
            clear_overrides();
        }

        #[test]
        fn r3049001_the_highest_lower_bound_leads_not_the_highest_rate() {
            // 25 wins in 32 games is the better rate, 300 in 450 the surer one.
            assert_eq!(
                picked(vec![("core-008", 25, 32), (LEADER, 300, 450)], "cn-tech-bound"),
                LEADER
            );
        }

        #[test]
        fn r1100_r3049001_a_tie_on_the_bound_goes_to_more_games_then_to_the_catalog_id() {
            // No win in 20 games and none in 40 share a lower bound of nothing.
            assert_eq!(
                picked(vec![("core-008", 0, 20), (LEADER, 0, 40)], "cn-tech-tie-games"),
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
        fn r2521007_with_no_leader_the_same_card_on_every_seed_and_no_random_draw() {
            // The fallback is a named card: every seed makes the same one, whoever runs the engine,
            // and the match rng is not drawn.
            for n in 0..4 {
                assert_eq!(picked(vec![], &format!("cn-tech-same-{n}")), FALLBACK_LEADER);
            }
            register_win_rates(table(vec![]));
            let mut s = casting("cn-tech-same", false);
            let cursor = s.state().rng_cursor;
            s.play(TECH, json!({}));
            assert_eq!(s.state().rng_cursor, cursor, "the fallback draws nothing");
        }

        #[test]
        fn r2521007_a_table_with_no_qualifying_row_falls_back_too() {
            // Thin rows only (fewer than CN_TECH_MIN_GAMES games) and CN Tech's own: no leader.
            assert_eq!(
                picked(vec![("core-008", 19, 19), (TECH, 40, 40)], "cn-tech-thin"),
                FALLBACK_LEADER
            );
        }

        #[test]
        fn the_added_card_is_hidden_from_the_opponent() {
            register_win_rates(table(vec![(LEADER, 30, 40)]));
            let mut s = casting("cn-tech-hidden", false);
            s.play(TECH, json!({}));
            let view: Value = js(&view_for(s.state(), PlayerId::P2));
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
        fn r2521007_falls_back_to_lunar_eclipse_hidden_from_the_opponent() {
            register_win_rates(table(vec![]));
            let mut s = casting("cn-tech-fallback", false);
            let before = s.hand(PlayerId::P1).len();
            s.play(TECH, json!({}));
            let added: Vec<CardInstance> = s.hand(PlayerId::P1).into_iter().skip(before - 1).collect();
            assert_eq!(added.len(), 1);
            assert_eq!(added[0].def_id, FALLBACK_LEADER);
            assert!(!added[0].radiant);
            // The opponent sees a count, never the card (R440).
            let view: Value = js(&view_for(s.state(), PlayerId::P2));
            assert_eq!(view["opponent"]["hand"], json!({ "count": 2 }));
            assert!(!view.to_string().contains(&added[0].id), "the opponent never sees its id");
        }

        #[test]
        fn r2521007_the_radiant_face_falls_back_to_a_radiant_lunar_eclipse() {
            register_win_rates(table(vec![]));
            let mut s = casting("cn-tech-fallback-radiant", true);
            let before = s.hand(PlayerId::P1).len();
            s.play(TECH, json!({}));
            let added: Vec<CardInstance> = s.hand(PlayerId::P1).into_iter().skip(before - 1).collect();
            assert_eq!(added.len(), 1);
            assert_eq!(added[0].def_id, FALLBACK_LEADER);
            assert!(added[0].radiant);
        }

        #[test]
        fn a_full_hand_burns_the_card() {
            register_win_rates(table(vec![(LEADER, 30, 40)]));
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
        fn r1101_r3049000_no_table_is_compiled_in_so_a_game_handed_none_adds_lunar_eclipse() {
            crate::register_all();
            clear_overrides();
            assert!(registered_win_rates().cards.is_empty(), "no table ships with the build");
            let mut s = casting("cn-tech-compiled", false);
            assert_eq!(s.state().win_rates, None, "the scenario was handed none");
            let before = s.hand(PlayerId::P1).len();
            s.play(TECH, json!({}));
            let added: Vec<CardInstance> = s.hand(PlayerId::P1).into_iter().skip(before - 1).collect();
            assert_eq!(added.len(), 1);
            assert_eq!(added[0].def_id, FALLBACK_LEADER);
        }
    }
}
