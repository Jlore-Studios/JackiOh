//! M #50 CN Tech (SPEC §8.8 row 50): (0) Spell, CN, Rare.
//!
//! Base:    "Add the highest win rate card to your hand."
//! Radiant: "Add the highest win rate card to your hand. It is Radiant."
//! Engine: ME-STATS (docs/meditative-set.md M5, MD-D1) — the game's win-rate snapshot, the server's
//! cached card stats as the match started (R3049000; R377's in-deck wins and games per card id), read
//! through `win_rate_leader`: the highest Wilson lower bound (R3049001), never CN Tech itself (R387).
//! A game handed no snapshot (practice, hotseat, the Sandbox, the run) reads its shipped catalog's
//! frozen table, which a replay of a game recorded before #3049 folds against (R1266000), else none,
//! and ranks it by the exact rate, as its version did (R1100).
//!
//! The fallback (#3869, R3869005): when no row qualifies it adds a random non-token card of the sets
//! that ship, never CN Tech itself (R387), drawn from the match rng, Radiant on the Radiant face, as
//! R142 had it before #2521. Through v0.6.0g it added Core #35 Lunar Eclipse instead (R2521007;
//! `script_v0_6_0g`, kept, R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-050";

/// R767000, R2521007: the card added through v0.6.0g when the table named no leader, Core #35 Lunar
/// Eclipse, 33/48, the leader of v0.4.0's provisional table.
pub const FALLBACK_LEADER_V0_6_0G: &str = "core-035";

/// The Cry; `kept`, the script through v0.6.0g.
fn tech(radiant: bool, kept: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| match win_rate_leader(&*ctx.state, ID) {
            Some(id) => vec![add_to_hand(json_as(json!({ "defId": id, "radiant": radiant })))],
            // R3869005: no leader, a random non-token card of the sets that ship, never this one.
            None if !kept => vec![add_random_from_catalog(json_as(json!({ "query": { "excludeDefId": ID }, "radiant": radiant })))],
            None => vec![add_to_hand(json_as(json!({ "defId": FALLBACK_LEADER_V0_6_0G, "radiant": radiant })))],
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: tech(false, false),
        radiant: tech(true, false),
    }
}

/// R767000: through v0.6.0g a table with no leader added Lunar Eclipse (R2521007); `KEPT_SCRIPTS` folds
/// those replays under it.
pub fn script_v0_6_0g() -> CardScripts {
    CardScripts {
        base: tech(false, true),
        radiant: tech(true, true),
    }
}

// M #50 CN Tech — SPEC §8.8 row 50, BUILD M10 row M 50: "Add the highest win rate card (the
// game's snapshot's pick, R377's in-deck rate) to your hand (MD-D1), never itself (R387), the Radiant
// face's Radiant (MD-D3); a random non-token card when the table names none (R3869005), hidden
// from the opponent (R440), burned by a full hand".
#[cfg(test)]
mod tests {
    use super::*;
    use crate::filler;
    use crate::js;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const TECH: &str = "meditative-050";
    const LEADER: &str = "core-001"; // Big D-fender: a deckable non-token card
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

        /// The def CN Tech added to p1's hand in `s`.
        fn added_by(mut s: Scenario) -> String {
            let before = s.hand(PlayerId::P1).len();
            s.play(TECH, json!({}));
            let added: Vec<CardInstance> = s.hand(PlayerId::P1).into_iter().skip(before - 1).collect();
            assert_eq!(added.len(), 1);
            added[0].def_id.clone()
        }

        /// The def CN Tech added to p1's hand in a game handed no snapshot, with `rows` the table such
        /// a game reads (a shipped version's frozen table, R1266000), ranked by the exact rate (R1100).
        fn picked(rows: Vec<(&str, i32, i32)>, seed: &str) -> String {
            register_win_rates(table(rows));
            added_by(casting(seed, false))
        }

        /// The def CN Tech added to p1's hand in a game handed `rows` as its snapshot (R3049000),
        /// ranked by the Wilson bounds (R3049001).
        fn picked_handed(rows: Vec<(&str, i32, i32)>, seed: &str) -> String {
            added_by(scenario(json!({
                "seed": seed,
                "p1": { "hand": [{ "def": TECH }, FILLER], "library": filler(4) },
                "p2": { "hand": [FILLER], "library": filler(4) },
                "winRates": table(rows),
            })))
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
            let rows = || vec![("core-008", 25, 32), (LEADER, 300, 450)];
            assert_eq!(picked_handed(rows(), "cn-tech-bound"), LEADER);
            // A game handed none ranks the table it reads by the rate, as its version did (R1100).
            assert_eq!(picked(rows(), "cn-tech-bound"), "core-008");
            clear_overrides();
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
        fn r3049001_a_tie_on_the_bound_goes_to_more_games_then_to_the_catalog_id() {
            // No win in 20 games and none in 40 share a lower bound of nothing.
            assert_eq!(
                picked_handed(vec![("core-008", 0, 20), (LEADER, 0, 40)], "cn-tech-tie-games"),
                LEADER
            );
            assert_eq!(
                picked_handed(vec![("core-008", 10, 20), ("core-002", 10, 20)], "cn-tech-tie-id"),
                "core-002"
            );
        }

        #[test]
        fn r1266000_r3049001_a_game_on_v0_4_0_still_adds_its_frozen_tables_leader_by_the_rate() {
            crate::register_all();
            clear_overrides();
            let shipped = crate::shipped_catalog("v0.4.0").expect("v0.4.0 is carried");
            jackioh_engine::with_shipped(Some(shipped), || {
                let s = casting("cn-tech-v0-4-0", false);
                assert_eq!(s.state().win_rates, None, "a game recorded before #3049");
                // Lunar Eclipse leads v0.4.0's table at 33 wins in 48 games; ranked by the bound,
                // Fig of Life's 92 in 146 would, so this tells the two rankings apart.
                assert_eq!(
                    leader_of(shipped.win_rates(), WinRateRanking::Bound, &[TECH]).as_deref(),
                    Some("core-047")
                );
                assert_eq!(win_rate_leader(s.state(), TECH).as_deref(), Some("core-035"));
                assert_eq!(added_by(s), "core-035");
            });
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

        /// R3869005: whether `id` is a card the fallback may add: a non-token card of a set that ships,
        /// never CN Tech.
        fn a_random_stand_in(id: &str) -> bool {
            let def = crate::card_def(id);
            !def.token && set_ships(def.set) && id != TECH
        }

        #[test]
        fn r3869005_with_no_leader_a_random_card_drawn_from_the_match_rng() {
            let added: Vec<String> = (0..8).map(|n| picked(vec![], &format!("cn-tech-random-{n}"))).collect();
            for id in &added {
                assert!(a_random_stand_in(id), "{id}");
            }
            let mut distinct = added.clone();
            distinct.sort();
            distinct.dedup();
            assert!(distinct.len() > 1, "not the same card on every seed: {added:?}");
            register_win_rates(table(vec![]));
            let mut s = casting("cn-tech-random", false);
            let cursor = s.state().rng_cursor;
            s.play(TECH, json!({}));
            assert!(s.state().rng_cursor > cursor, "the fallback draws from the match rng");
        }

        #[test]
        fn r3869005_a_table_with_no_qualifying_row_falls_back_too() {
            // Thin rows only (fewer than CN_TECH_MIN_GAMES games) and CN Tech's own: no leader.
            let added = picked(vec![("core-008", 19, 19), (TECH, 40, 40)], "cn-tech-thin");
            assert!(a_random_stand_in(&added), "{added}");
        }

        #[test]
        fn r767000_the_script_kept_for_v0_6_0g_adds_lunar_eclipse_with_no_leader() {
            register_win_rates(table(vec![]));
            crate::under_kept(TECH, script_v0_6_0g(), || {
                for n in 0..3 {
                    assert_eq!(added_by(casting(&format!("cn-tech-kept-{n}"), false)), FALLBACK_LEADER_V0_6_0G);
                }
            });
            clear_overrides();
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
        fn r3869005_the_random_card_is_hidden_from_the_opponent() {
            register_win_rates(table(vec![]));
            let mut s = casting("cn-tech-fallback", false);
            let before = s.hand(PlayerId::P1).len();
            s.play(TECH, json!({}));
            let added: Vec<CardInstance> = s.hand(PlayerId::P1).into_iter().skip(before - 1).collect();
            assert_eq!(added.len(), 1);
            assert!(a_random_stand_in(&added[0].def_id), "{}", added[0].def_id);
            assert!(!added[0].radiant);
            // The opponent sees a count, never the card (R440).
            let view: Value = js(&view_for(s.state(), PlayerId::P2));
            assert_eq!(view["opponent"]["hand"], json!({ "count": 2 }));
            assert!(!view.to_string().contains(&added[0].id), "the opponent never sees its id");
        }

        #[test]
        fn r3869005_the_radiant_face_adds_a_radiant_random_card() {
            register_win_rates(table(vec![]));
            let mut s = casting("cn-tech-fallback-radiant", true);
            let before = s.hand(PlayerId::P1).len();
            s.play(TECH, json!({}));
            let added: Vec<CardInstance> = s.hand(PlayerId::P1).into_iter().skip(before - 1).collect();
            assert_eq!(added.len(), 1);
            assert!(a_random_stand_in(&added[0].def_id), "{}", added[0].def_id);
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
        fn r1101_r3049000_r3869005_no_table_is_compiled_in_so_a_game_handed_none_adds_a_random_card() {
            crate::register_all();
            clear_overrides();
            assert!(registered_win_rates().cards.is_empty(), "no table ships with the build");
            let mut s = casting("cn-tech-compiled", false);
            assert_eq!(s.state().win_rates, None, "the scenario was handed none");
            let before = s.hand(PlayerId::P1).len();
            s.play(TECH, json!({}));
            let added: Vec<CardInstance> = s.hand(PlayerId::P1).into_iter().skip(before - 1).collect();
            assert_eq!(added.len(), 1);
            assert!(a_random_stand_in(&added[0].def_id), "{}", added[0].def_id);
        }
    }
}
