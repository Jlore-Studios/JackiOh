//! SPEC §9.11, R378: the AI's development run. A development game is an All Random game with two AI
//! pilots on this spec's resources, dealt exactly as the server deals a live one, and it is filed as
//! a record of its own source, which no live figure counts unless it is asked for. The games here run
//! at a small budget so the file stays fast; `cargo jackioh stats` plays at the browser's.
//!
//! Port of `packages/ai/test/dev-run.test.ts`. Its third case, "reads the run's options, and refuses
//! one it does not know", tests `scripts/stats.ts`'s `parseDevRunArgs`, which is the CLI's own
//! argument parser now (`crates/tools/src/stats.rs`, SURFACE §12, part 22): this test binary cannot
//! link the tools crate (it depends on this one), so that case belongs with the CLI's tests.

use jackioh_ai::{
    AI_BUDGET, AI_DEV_RUN, AiDeckOptions, DevRunOptions, MatchHooks, SearchBudget, SeatController, build_ai_deck,
    dev_game_config, dev_game_record, dev_record_id, play_match,
};
use jackioh_engine::testkit::{
    CardStatsFilter, DECK_SIZE, DEFAULT_CARD_STATS_FILTER, FoldArgs, GameRecord, PerPlayer, PilotFilter, SourceFilter,
    card_stats, create_rng, json, json_as, summarize_game,
};

use super::support::register_cards;

const QUICK: SearchBudget = SearchBudget {
    nodes: 30,
    lethal_nodes: 10,
    determinizations: 1,
    beam_width: 1,
    root_branching: 6,
    branching: 2,
    max_depth: 2,
    finalists: 1,
};

/// TS `expect(record).toMatchObject(fields)`: every field named holds its value, read as the wire writes it.
fn assert_matches_object(record: &GameRecord, fields: serde_json::Value) {
    let written = serde_json::to_value(record).expect("a record serialises");
    for (key, value) in fields.as_object().expect("an object literal") {
        assert_eq!(&written[key], value, "{key}");
    }
}

mod the_ais_development_run_9_11 {
    use super::*;

    #[test]
    fn r378_deals_game_n_as_all_random_deals_it_to_two_ai_seats_on_this_specs_resources() {
        register_cards();
        let config = dev_game_config(7, AI_DEV_RUN.series, None);
        assert_eq!(config.seed, format!("{}:7", AI_DEV_RUN.series));
        // R258: the server's deal, `${seed}:p1-deck` and `${seed}:p2-deck`, nothing banned.
        let unbanned: AiDeckOptions = json_as(json!({ "banned": [] }));
        assert_eq!(
            config.decks,
            (
                build_ai_deck(&mut create_rng(&format!("{}:p1-deck", config.seed), 0), DECK_SIZE, &unbanned),
                build_ai_deck(&mut create_rng(&format!("{}:p2-deck", config.seed), 0), DECK_SIZE, &unbanned),
            )
        );
        assert!(config.handicaps.is_none());
        assert_eq!(
            config.controllers,
            PerPlayer::new(
                SeatController::Ai {
                    budget: Some(AI_BUDGET)
                },
                SeatController::Ai {
                    budget: Some(AI_BUDGET)
                },
            )
        );
        assert_ne!(dev_game_config(7, "other", None).decks, config.decks);
    }

    #[test]
    fn r378_files_each_finished_game_as_a_development_record_of_the_patch_it_tests() {
        register_cards();
        let options = DevRunOptions {
            series: "dev-test".to_string(),
            patch: "v0.2.5".to_string(),
            budget: Some(QUICK),
        };
        let records: Vec<Option<GameRecord>> = [1, 2].into_iter().map(|n| dev_game_record(n, &options)).collect();

        for (at, record) in records.iter().enumerate() {
            let n = at as i32 + 1;
            let record = record
                .as_ref()
                .unwrap_or_else(|| panic!("dev-test:{n} did not finish"));
            let config = dev_game_config(n, &options.series, options.budget);
            assert_matches_object(
                record,
                json!({
                    "id": format!("dev:v0.2.5:dev-test:{n}"),
                    "source": "dev",
                    "mode": "random",
                    "patch": "v0.2.5",
                    "pilots": { "p1": "ai", "p2": "ai" },
                }),
            );
            // The game half is the engine's own reading of the game the AI played.
            let played = play_match(&config, &mut MatchHooks::default());
            let summary = summarize_game(&json_as::<FoldArgs>(json!({
                "seed": config.seed,
                "decks": config.decks,
                "log": played.log,
            })));
            assert_eq!(Some(record.game.clone()), summary);
            assert_eq!(Some(record.game.winner), played.result.map(|result| result.winner));
            assert_eq!(record.game.seats.p1.deck, config.decks.0);
        }
        // Seeded: the same game is the same record.
        assert_eq!(dev_game_record(1, &options), records[0]);
        // The next patch's run on the same seeds files other records, so an import adds them rather
        // than skipping them as games it has already loaded.
        let next_patch = dev_game_record(
            1,
            &DevRunOptions {
                patch: "v0.2.6".to_string(),
                ..options.clone()
            },
        );
        let next = next_patch.as_ref().expect("the game finished");
        assert_matches_object(next, json!({ "id": "dev:v0.2.6:dev-test:1", "patch": "v0.2.6" }));
        assert_ne!(
            next_patch.as_ref().map(|record| record.id.clone()),
            records[0].as_ref().map(|record| record.id.clone())
        );
        assert_eq!(
            Some(dev_record_id("v0.2.5", "dev-test:1")),
            records[0].as_ref().map(|record| record.id.clone())
        );

        // R378: a live query reads none of them; a development query reads both.
        let finished: Vec<GameRecord> = records.into_iter().flatten().collect();
        assert_eq!(card_stats(&finished, &DEFAULT_CARD_STATS_FILTER).games, 0);
        let dev = CardStatsFilter {
            source: SourceFilter::Dev,
            ..DEFAULT_CARD_STATS_FILTER
        };
        assert_eq!(card_stats(&finished, &dev).games, 2);
        let dev_humans = CardStatsFilter {
            source: SourceFilter::Dev,
            pilot: PilotFilter::Human,
            ..DEFAULT_CARD_STATS_FILTER
        };
        assert_eq!(card_stats(&finished, &dev_humans).games, 0);
    }
}
