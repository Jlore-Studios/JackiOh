//! The Meditative+ batch "outside data and the autopilot" (issue #1183, docs/meditative-plus-set.md
//! MP5): the lowest win rate (R1183000), the two-pick fuse (R1183001) and the code-block Discover
//! (R1183005), each through `fixtures::outside_data`; the autopilot's tests (R1183002, R1183003) are
//! `autopilot.rs`'s, the subsystem's own home. The real cards (M+ #2 NA Tech, #25 KY's Intuition)
//! cover the same cases again.
//! Beside them, the win-rate table a fold under a shipped catalog reads (R1266000, issue #1266), the
//! snapshot a game is handed and keeps (R3049000) and the Wilson bounds its ends are ranked by
//! (R3049001, issue #3049).

use jackioh_engine::effects::CODE_OFFERS_KEY;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::outside_data::{
    FIRST_UNIT, SECOND_UNIT, code_seer, fuse_same, fuse_two, register_outside_data,
};

const FILLER: &str = "fx-10";

fn filler(count: usize) -> Value {
    json!(vec![FILLER; count])
}

fn table(cards: Vec<(&str, i32, i32)>) -> WinRateTable {
    WinRateTable {
        patch: "v0.3.X".to_string(),
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

/// The leader and the trailer of the table a state handed no snapshot reads.
fn leader(exclude_def_ids: &[&str]) -> Option<String> {
    leader_of(registered_win_rates(), exclude_def_ids)
}

fn trailer(exclude_def_ids: &[&str]) -> Option<String> {
    trailer_of(registered_win_rates(), exclude_def_ids)
}

/// The events of one type in the scenario's log.
fn count_of(s: &Scenario, type_: GameEventType) -> usize {
    s.events()
        .iter()
        .filter(|event| event.event_type() == type_)
        .count()
}

mod r1183000_the_lowest_win_rate {
    use super::*;

    #[test]
    fn r1183000_the_trailer_is_the_lowest_exact_rate_past_the_floor() {
        register_outside_data();
        register_win_rates(table(vec![
            ("fx-1", 30, 40),
            ("fx-2", 11, 40),
            ("fx-3", 10, 40),
            // A thin row never trails, however low.
            ("fx-4", 0, 19),
        ]));
        // 10/40 is below 11/40 by one win: compared exactly, never as a rounded float.
        assert_eq!(trailer(&[]), Some("fx-3".to_string()));
        assert_eq!(leader(&[]), Some("fx-1".to_string()));
    }

    #[test]
    fn r1183000_r3049001_ties_on_the_bound_go_to_more_games_then_the_catalog_id() {
        register_outside_data();
        // Every win in 20 games and in 40 has the same upper bound, one whole: more games, as
        // R1100's leader breaks it.
        register_win_rates(table(vec![("fx-5", 20, 20), ("fx-6", 40, 40)]));
        assert_eq!(trailer(&[]), Some("fx-6".to_string()));
        register_win_rates(table(vec![("fx-1", 30, 40), ("fx-7", 10, 40), ("fx-6", 10, 40)]));
        assert_eq!(trailer(&[]), Some("fx-6".to_string()));
    }

    #[test]
    fn r1183000_the_trailer_skips_the_ids_it_is_given_tokens_and_missing_ids() {
        register_outside_data();
        register_win_rates(table(vec![
            ("fx-1", 30, 40),
            ("fx-token-rush", 0, 40),
            ("no-such-card", 0, 40),
        ]));
        // With the leader skipped, the one eligible row is gone: no trailer.
        assert_eq!(trailer(&["fx-1"]), None);
        assert_eq!(trailer(&[]), Some("fx-1".to_string()));
        register_win_rates(table(vec![]));
        assert_eq!(trailer(&[]), None);
    }
}

mod r1266000_a_fold_reads_its_versions_table {
    use super::*;

    #[test]
    fn r1266000_a_fold_reads_the_table_its_shipped_catalog_carries_never_the_builds() {
        register_outside_data();
        register_win_rates(table(vec![("fx-1", 30, 40), ("fx-2", 10, 40)]));
        let empty: &'static ShippedCatalog = Box::leak(Box::new(ShippedCatalog::new(
            "v-empty",
            registered_catalog().clone(),
            IndexMap::new(),
        )));
        let kept: &'static ShippedCatalog = Box::leak(Box::new(
            ShippedCatalog::new("v-kept", registered_catalog().clone(), IndexMap::new())
                .with_win_rates(table(vec![("fx-2", 30, 40), ("fx-1", 10, 40)])),
        ));
        assert_eq!(leader(&[]), Some("fx-1".to_string()));
        // A version that shipped with the empty table names no card, whatever the build's holds.
        assert_eq!(with_shipped(Some(empty), || leader(&[])), None);
        // One that shipped with its own reads both ends off it.
        assert_eq!(
            with_shipped(Some(kept), || (leader(&[]), trailer(&[]))),
            (Some("fx-2".to_string()), Some("fx-1".to_string()))
        );
        assert_eq!(
            leader(&[]),
            Some("fx-1".to_string()),
            "the build's again once the fold returns"
        );
        clear_overrides();
    }
}

mod r3049000_the_games_snapshot {
    use super::*;
    use jackioh_engine::subsystems::glitch::reset_match;

    use crate::rules::fixtures::catalog::vanilla_deck;
    use crate::rules::fixtures::harness::sink_for;

    /// A scenario handed `win_rates`, or none.
    fn handed(win_rates: Option<&WinRateTable>) -> Scenario {
        scenario(json!({
            "p1": { "hand": [FILLER], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
            "winRates": win_rates,
        }))
    }

    /// The vanilla decks, handed `win_rates`.
    fn options(seed: &str, win_rates: Option<WinRateTable>) -> CreateGameOptions {
        CreateGameOptions {
            seed: seed.to_string(),
            decks: (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21)),
            win_rates,
            ..Default::default()
        }
    }

    /// Rows for every deckable card the catalog holds, a few under the floor, with figures drawn
    /// from `seed`, beside a token's and a vanished id's.
    fn every_card(seed: &str) -> WinRateTable {
        let mut rng = Rng::new(seed, 0);
        let mut cards: Vec<(String, i32, i32)> = registered_catalog()
            .values()
            .filter(|def| deckable(def))
            .map(|def| {
                let games = CN_TECH_MIN_GAMES - 5 + rng.int(300);
                (def.id.clone(), rng.int(games + 1), games)
            })
            .collect();
        cards.push(("fx-token-rush".to_string(), 40, 40));
        cards.push(("no-such-card".to_string(), 0, 40));
        table(
            cards
                .iter()
                .map(|(id, wins, games)| (id.as_str(), *wins, *games))
                .collect(),
        )
    }

    #[test]
    fn r3049000_a_game_reads_its_own_snapshot_before_any_other_table() {
        register_outside_data();
        register_win_rates(table(vec![("fx-1", 30, 40), ("fx-2", 10, 40)]));
        let snapshot = table(vec![("fx-2", 30, 40), ("fx-1", 10, 40)]);
        let own = handed(Some(&snapshot));
        let none = handed(None);
        assert_eq!(own.state().win_rates, Some(kept_for_game(&snapshot)));
        assert_eq!(none.state().win_rates, None);
        let ends = |state: &GameState| (win_rate_leader(state, ""), win_rate_trailer(state, &[]));
        let fx = |id: &str| Some(id.to_string());
        assert_eq!(ends(own.state()), (fx("fx-2"), fx("fx-1")));
        assert_eq!(ends(none.state()), (fx("fx-1"), fx("fx-2")));
        // A fold under a shipped catalog reads the game's snapshot too: only a game handed none
        // reads the version's frozen table.
        let frozen: &'static ShippedCatalog = Box::leak(Box::new(
            ShippedCatalog::new("v-frozen", registered_catalog().clone(), IndexMap::new())
                .with_win_rates(table(vec![("fx-3", 30, 40), ("fx-4", 10, 40)])),
        ));
        assert_eq!(
            with_shipped(Some(frozen), || ends(own.state())),
            (fx("fx-2"), fx("fx-1"))
        );
        assert_eq!(
            with_shipped(Some(frozen), || ends(none.state())),
            (fx("fx-3"), fx("fx-4"))
        );
        // The figures are the game's setup, never in a view.
        for player in [P1, P2] {
            let view = serde_json::to_string(&view_for(own.state(), player)).expect("serializes");
            assert!(!view.contains("winRates"), "{player:?}'s view");
        }
        clear_overrides();
    }

    #[test]
    fn r3049000_a_game_keeps_the_ends_and_every_pick_reads_the_same() {
        register_outside_data();
        let full = every_card("r3049000-keep");
        let kept = kept_for_game(&full);
        assert!(kept.cards.len() > WIN_RATE_SNAPSHOT_ENDS);
        assert!(kept.cards.len() <= 2 * WIN_RATE_SNAPSHOT_ENDS);
        assert!(kept.cards.windows(2).all(|pair| pair[0].id < pair[1].id), "sorted by id");
        assert!(kept.cards.iter().all(|row| full.cards.contains(row)));
        assert_eq!((&kept.patch, kept.source), (&full.patch, full.source));
        assert_eq!(kept_for_game(&kept), kept, "keeping it again keeps it all");
        // Every pick passing over up to three of the kept cards reads the same card off both.
        let ids: Vec<&str> = kept.cards.iter().map(|row| row.id.as_str()).collect();
        let mut excludes: Vec<Vec<&str>> = vec![vec![]];
        for (at, &first) in ids.iter().enumerate() {
            excludes.push(vec![first]);
            for (by, &second) in ids.iter().enumerate().skip(at + 1) {
                excludes.push(vec![first, second]);
                for &third in ids.iter().skip(by + 1) {
                    excludes.push(vec![first, second, third]);
                }
            }
        }
        for skipped in &excludes {
            assert_eq!(leader_of(&kept, skipped), leader_of(&full, skipped), "{skipped:?}");
            assert_eq!(trailer_of(&kept, skipped), trailer_of(&full, skipped), "{skipped:?}");
        }
        assert!(leader_of(&full, &[]).is_some() && trailer_of(&full, &[]).is_some());
        // The game holds the kept rows alone.
        let state = create_game(&options("r3049000-keep", Some(full.clone())));
        assert_eq!(state.win_rates, Some(kept));
        clear_overrides();
    }

    #[test]
    fn r3049000_a_glitch_reset_keeps_the_snapshot() {
        register_outside_data();
        let snapshot = every_card("r3049000-reset");
        let created = create_game(&options("r3049000-reset", Some(snapshot.clone())));
        let mut state = begin_game(&created).state;
        {
            let mut sink = sink_for(&mut state);
            reset_match(&mut sink);
        }
        assert_eq!(state.resets, Some(1));
        assert_eq!(state.win_rates, Some(kept_for_game(&snapshot)));
        assert_eq!(state.win_rates, created.win_rates);
        clear_overrides();
    }

    #[test]
    fn r3049000_a_fold_replays_the_records_snapshot() {
        register_outside_data();
        let snapshot = every_card("r3049000-fold");
        let input = |win_rates: Option<WinRateTable>| ReplayInput {
            seed: "r3049000-fold".to_string(),
            decks: (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21)),
            win_rates,
            ..ReplayInput::default()
        };
        let folded = fold(&input(Some(snapshot.clone()))).state;
        let kept = kept_for_game(&snapshot);
        assert_eq!(folded.win_rates.as_ref(), Some(&kept));
        // The record keeps what the game kept, and folds to the same state.
        let refolded = fold(&input(folded.win_rates.clone())).state;
        assert_eq!(hash_state(&refolded), hash_state(&folded));
        // A game handed none holds none and hashes as before the field existed.
        let bare = fold(&input(None)).state;
        assert_eq!(bare.win_rates, None);
        assert_ne!(hash_state(&bare), hash_state(&folded));
        assert!(!serde_json::to_string(&bare).expect("serializes").contains("winRates"));
        clear_overrides();
    }
}

mod r3049001_the_bounds {
    use super::*;

    #[test]
    fn r3049001_the_bounds_are_wilsons_to_a_millionth() {
        let scale = WIN_RATE_BOUND_SCALE as f64;
        for (wins, games) in [(25, 32), (78, 120), (0, 20), (20, 20), (7, 43)] {
            let (lower, upper) = wilson_bounds(wins, games).expect("bounds");
            let (low, high) = wilson_interval(i64::from(wins), i64::from(games));
            assert!((lower as f64 - low * scale).abs() <= 1.0, "{wins}/{games}: {lower} {low}");
            assert!((upper as f64 - high * scale).abs() <= 1.0, "{wins}/{games}: {upper} {high}");
        }
        assert_eq!(wilson_bounds(0, 20).map(|(lower, _)| lower), Some(0));
        assert_eq!(
            wilson_bounds(20, 20).map(|(_, upper)| upper),
            Some(WIN_RATE_BOUND_SCALE)
        );
        for (wins, games) in [(5, 0), (-1, 20), (21, 20)] {
            assert_eq!(wilson_bounds(wins, games), None, "{wins}/{games}");
        }
        for wins in [0, i32::MAX / 2, i32::MAX] {
            let (lower, upper) = wilson_bounds(wins, i32::MAX).expect("bounds at i32::MAX");
            assert!(lower <= upper && upper <= WIN_RATE_BOUND_SCALE);
        }
        assert_eq!(
            WIN_RATE_BOUND_Z_NUM as f64 / WIN_RATE_BOUND_Z_DEN as f64,
            STATS_INTERVAL_Z
        );
    }

    #[test]
    fn r3049001_the_leader_is_the_highest_lower_bound_and_the_trailer_the_lowest_upper_bound() {
        register_outside_data();
        // fx-1 has the best rate (78%) in few games and fx-3 the worst (10%): neither is an end.
        let rows = table(vec![
            ("fx-1", 25, 32),
            ("fx-2", 300, 450),
            ("fx-3", 2, 20),
            ("fx-4", 50, 250),
        ]);
        assert_eq!(leader_of(&rows, &[]), Some("fx-2".to_string()));
        assert_eq!(trailer_of(&rows, &[]), Some("fx-4".to_string()));
        // Passed over, each end gives way to the next bound.
        assert_eq!(leader_of(&rows, &["fx-2"]), Some("fx-1".to_string()));
        assert_eq!(trailer_of(&rows, &["fx-4"]), Some("fx-3".to_string()));
        clear_overrides();
    }
}

mod r1183001_a_fuse_of_two_picks {
    use super::*;

    fn casting(seed: &str, spell: &str, fillers: usize) -> Scenario {
        register_outside_data();
        let mut hand = vec![json!(spell)];
        hand.extend(vec![json!(FILLER); fillers]);
        scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    #[test]
    fn r1183001_two_different_cards_fused_into_one_hand_card_at_its_set_cost() {
        for n in 0..6 {
            let mut s = casting(&format!("od-fuse-{n}"), &fuse_two.id, 1);
            s.play(fuse_two.id.as_str(), json!({}));
            let fused: Vec<CardInstance> = s
                .hand(P1)
                .into_iter()
                .filter(|card| card.def_id != FILLER)
                .collect();
            assert_eq!(fused.len(), 1, "one fused card in hand");
            let def = def_of(Some(s.state()), &fused[0].def_id);
            let mut names: Vec<&str> = def.name.split(" + ").collect();
            names.sort();
            assert_eq!(
                names,
                vec!["Fixture Unit 1", "Fixture Unit 2"],
                "the picks are {FIRST_UNIT} and {SECOND_UNIT}, never one twice"
            );
            assert_eq!(fused[0].cost_override, Some(2), "it costs (2)");
        }
    }

    #[test]
    fn r1183001_a_pick_only_an_earlier_pick_could_fill_fuses_nothing() {
        let mut s = casting("od-fuse-same", &fuse_same.id, 1);
        let before = s.hand(P1).len();
        s.play(fuse_same.id.as_str(), json!({}));
        assert_eq!(s.hand(P1).len(), before - 1, "nothing was added");
        assert_eq!(count_of(&s, GameEventType::Fused), 0);
    }

    #[test]
    fn r1183001_a_full_hand_burns_the_fused_card_with_no_price() {
        let mut s = casting("od-fuse-burn", &fuse_two.id, HAND_CAP as usize);
        s.play(fuse_two.id.as_str(), json!({}));
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
        let burned: Vec<&CardInstance> = s
            .state()
            .players
            .p1
            .graveyard
            .iter()
            .filter(|card| card.def_id != fuse_two.id)
            .collect();
        assert_eq!(burned.len(), 1, "the fused card burned");
        assert_eq!(burned[0].cost_override, None, "a burned card keeps no price");
    }
}

mod r1183005_the_code_discover {
    use super::*;

    const BLOCKS: [(&str, &str); 4] = [
        ("fx-1", "fn one() {}"),
        ("fx-2", "fn two() {}"),
        ("fx-3", "fn three() {}"),
        ("fx-4", "fn four() {}"),
    ];

    fn seeing(seed: &str, blocks: &[(&str, &str)]) -> Scenario {
        register_outside_data();
        register_code_blocks(blocks.to_vec());
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [code_seer.id, FILLER], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(code_seer.id.as_str(), json!({}));
        s
    }

    #[test]
    fn r1183005_three_distinct_cards_each_shown_as_its_code_alone() {
        let s = seeing("od-code", &BLOCKS);
        let pending = s.state().pending.clone().expect("the Discover is open");
        assert_eq!(pending.kind, PromptKind::Discover);
        assert_eq!(pending.options.len(), 3);
        let codes: Vec<&str> = BLOCKS.iter().map(|(_, code)| *code).collect();
        let mut seen: Vec<String> = Vec::new();
        for option in &pending.options {
            assert!(
                codes.contains(&option.label.as_str()),
                "the label is a code block"
            );
            assert!(!seen.contains(&option.label), "three different cards");
            seen.push(option.label.clone());
        }

        // The chooser reads the code and nothing that names the card; the opponent reads nothing.
        let mine = serde_json::to_value(view_for(s.state(), P1).pending).expect("serializes");
        for option in mine["options"].as_array().expect("options") {
            assert_eq!(option["code"], json!(true));
            assert!(option.get("defId").is_none());
            assert!(
                option["key"]
                    .as_str()
                    .is_some_and(|key| key.starts_with("mode:code-"))
            );
        }
        for (id, _) in BLOCKS {
            assert!(!mine.to_string().contains(id), "no id in the chooser's prompt");
        }
        let theirs = serde_json::to_value(view_for(s.state(), P2).pending).expect("serializes");
        assert_eq!(theirs["forYou"], json!(false));
        assert!(theirs.get("options").is_none());
    }

    #[test]
    fn r1183005_the_pick_goes_to_the_hand_radiant() {
        let mut s = seeing("od-code-pick", &BLOCKS);
        let pending = s.state().pending.clone().expect("the Discover is open");
        let behind = pending.resume.data[CODE_OFFERS_KEY][1]
            .as_str()
            .expect("the second place's card")
            .to_string();
        s.answer(json!("code-1"));
        let added: Vec<CardInstance> = s
            .hand(P1)
            .into_iter()
            .filter(|card| card.def_id != FILLER)
            .collect();
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].def_id, behind);
        assert!(added[0].radiant);
    }

    #[test]
    fn r1183005_only_cards_with_a_code_block_are_offered() {
        let s = seeing("od-code-two", &BLOCKS[..2]);
        let pending = s.state().pending.clone().expect("the Discover is open");
        assert_eq!(pending.options.len(), 2);
    }
}
