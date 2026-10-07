//! `cli/card_stats.rs` (stats-cards) and `cli/import_dev_records.rs` (stats-import), SPEC §9.11,
//! R377 and R378, against the fake store: what the options read, that a development run is read
//! only when asked for, and that a file can add development records and nothing else. The store's
//! own filtering is `tests/store/contract.rs`'s, against both stores; the arithmetic is the
//! engine's `wire::stats`.
//! (← `apps/server/test/db/card-stats.test.ts`)

use std::path::PathBuf;

use indexmap::IndexMap;
use serde_json::{json, Value};

use jackioh_engine::wire::GameRecord;
use jackioh_server::cli::card_stats::{card_stats_report, parse_card_stats_args, render_card_stats, CardStatsOptions};
use jackioh_server::cli::import_dev_records::{import_dev_records, run_file_path};
use jackioh_server::db::store::Db;

/// `{ ...base, ...partial }` on two JSON objects.
fn spread(mut base: Value, partial: Value) -> Value {
    if let (Value::Object(into), Value::Object(from)) = (&mut base, partial) {
        for (key, value) in from {
            into.insert(key, value);
        }
    }
    base
}

fn game_record_json(id: &str, partial: Value) -> Value {
    spread(
        json!({
            "id": id,
            "source": "live",
            "mode": "bo1",
            "patch": "v0.2.5",
            "pilots": { "p1": "human", "p2": "human" },
            "game": {
                "first": "p1",
                "winner": "p1",
                "reason": "hero-death",
                "turns": 9,
                "seats": {
                    "p1": { "deck": ["core-001"], "opening": ["core-001"], "drawn": [], "played": ["core-001"] },
                    "p2": { "deck": ["core-002"], "opening": [], "drawn": ["core-002"], "played": [] }
                }
            }
        }),
        partial,
    )
}

fn record_of(value: Value) -> GameRecord {
    serde_json::from_value(value).expect("a well-formed game record")
}

fn game_record(id: &str, partial: Value) -> GameRecord {
    record_of(game_record_json(id, partial))
}

fn dev(id: &str, partial: Value) -> GameRecord {
    record_of(game_record_json(
        id,
        spread(json!({ "source": "dev", "mode": "random", "pilots": { "p1": "ai", "p2": "ai" } }), partial),
    ))
}

fn lines(records: &[GameRecord]) -> String {
    records
        .iter()
        .map(|record| serde_json::to_string(record).expect("a record serialises"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|arg| (*arg).to_owned()).collect()
}

/// The ids the fake's `game_records` table holds, in table order (TS `store.tables.gameRecords`).
async fn stored_ids(db: &Db) -> Vec<String> {
    let Db::Fake(data) = db else { panic!("these tests run on the fake store") };
    data.lock().await.game_records.iter().map(|record| record.id.clone()).collect()
}

mod stats_cards_options {
    use super::*;

    fn filter_json(options: &CardStatsOptions) -> Value {
        serde_json::to_value(&options.filter).expect("a filter serialises")
    }

    #[test]
    fn r378_reads_live_games_of_every_mode_patch_and_pilot_when_it_names_nothing() {
        let options = parse_card_stats_args(&args(&[])).expect("no arguments parse");
        assert_eq!(filter_json(&options), json!({ "source": "live", "mode": null, "patch": null, "pilot": "unified" }));
        assert_eq!(options.card, None);
        assert!(!options.json);
    }

    #[test]
    fn r377_takes_any_combination_of_match_type_patch_and_pilot_and_one_card() {
        let options = parse_card_stats_args(&args(&[
            "--source=all",
            "--mode=random",
            "--patch=v0.2.5",
            "--pilot=ai",
            "--card=core-001",
            "--json",
        ]))
        .expect("every option parses");
        assert_eq!(filter_json(&options), json!({ "source": "all", "mode": "random", "patch": "v0.2.5", "pilot": "ai" }));
        assert_eq!(options.card.as_deref(), Some("core-001"));
        assert!(options.json);

        let human = parse_card_stats_args(&args(&["--pilot=human"])).expect("--pilot=human parses");
        assert_eq!(filter_json(&human)["pilot"], json!("human"));
        let dev = parse_card_stats_args(&args(&["--source=dev"])).expect("--source=dev parses");
        assert_eq!(filter_json(&dev)["source"], json!("dev"));
    }

    fn refusal(argv: &[&str]) -> String {
        parse_card_stats_args(&args(argv)).err().expect("the arguments are refused").to_string()
    }

    #[test]
    fn refuses_an_option_or_a_value_it_does_not_know_rather_than_reading_something_else() {
        assert!(refusal(&["--source=practice"]).contains("--source must be one of live, dev, all"));
        assert!(refusal(&["--mode=bo5"]).contains("--mode must be one of bo1, bo3, random"));
        assert!(refusal(&["--pilot=robot"]).contains("--pilot"));
        assert!(refusal(&["--deck=1"]).contains("Unrecognised option --deck"));
        assert!(refusal(&["--patch"]).contains("Unrecognised argument"));
        assert!(refusal(&["v0.2.5"]).contains("Unrecognised argument"));
    }
}

mod stats_cards_report {
    use super::*;

    async fn seeded() -> Db {
        let db = Db::fake();
        let mut winner_p2 = game_record_json("x", json!({}))["game"].clone();
        winner_p2["winner"] = json!("p2");
        let records = [
            game_record("live-1", json!({})),
            game_record("live-2", json!({ "patch": "v0.1.1" })),
            dev("dev:1", json!({})),
            dev("dev:2", json!({ "game": winner_p2 })),
        ];
        let mut tx = db.begin(None).await.expect("a transaction");
        for record in &records {
            tx.game_records_insert(record).await.expect("the record is written");
        }
        tx.commit().await.expect("the records commit");
        db
    }

    fn options(argv: &[&str]) -> CardStatsOptions {
        parse_card_stats_args(&args(argv)).expect("the options parse")
    }

    /// The `inDeck` tally of `core-001`'s row, as JSON.
    fn first_in_deck(report: &jackioh_engine::wire::CardStatsReport) -> Value {
        let row = report.cards.iter().find(|stats| stats.card == "core-001").expect("core-001 has a row");
        serde_json::to_value(row).expect("a row serialises")["inDeck"].clone()
    }

    #[tokio::test]
    async fn r378_counts_a_development_run_only_when_it_is_asked_for_by_name() {
        let db = seeded().await;
        assert_eq!(card_stats_report(&db, &options(&[])).await.expect("a report").games, 2);
        assert_eq!(card_stats_report(&db, &options(&["--source=dev"])).await.expect("a report").games, 2);
        assert_eq!(card_stats_report(&db, &options(&["--source=all"])).await.expect("a report").games, 4);
    }

    #[tokio::test]
    async fn r378_compares_a_pre_release_run_with_the_live_games_of_the_same_patch_one_query_each() {
        let db = seeded().await;
        let live = card_stats_report(&db, &options(&["--patch=v0.2.5"])).await.expect("a report");
        let prerelease =
            card_stats_report(&db, &options(&["--patch=v0.2.5", "--source=dev"])).await.expect("a report");
        assert_eq!(live.games, 1);
        assert_eq!(prerelease.games, 2);
        assert_eq!(first_in_deck(&live), json!({ "games": 1, "wins": 1, "draws": 0 }));
        assert_eq!(first_in_deck(&prerelease), json!({ "games": 2, "wins": 1, "draws": 0 }));
    }

    /// JS `line.split(/\s{2,}/)`: the line cut at every run of two or more whitespace characters.
    fn split_wide(line: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut gap = String::new();
        for ch in line.chars() {
            if ch.is_whitespace() {
                gap.push(ch);
                continue;
            }
            if gap.chars().count() >= 2 {
                parts.push(std::mem::take(&mut current));
            } else {
                current.push_str(&gap);
            }
            gap.clear();
            current.push(ch);
        }
        if gap.chars().count() >= 2 {
            parts.push(current);
            parts.push(String::new());
        } else {
            current.push_str(&gap);
            parts.push(current);
        }
        parts
    }

    #[tokio::test]
    async fn r377_prints_every_breakdown_of_one_card_named_with_its_games_beside_each_rate() {
        let db = seeded().await;
        let one = options(&["--card=core-001", "--patch=v0.2.5"]);
        let report = card_stats_report(&db, &one).await.expect("a report");
        assert_eq!(report.cards.iter().map(|stats| stats.card.clone()).collect::<Vec<_>>(), vec!["core-001"]);

        let names = |card: &str| (card == "core-001").then(|| "Big D Fender".to_owned());
        let text = render_card_stats(&report, &one, &names);
        assert!(text.contains("Card win rates: live games, every mode, patch v0.2.5, human and AI pilots (unified)."));
        let row = text.split('\n').find(|line| line.starts_with("core-001 Big D Fender")).expect("core-001's row");
        assert_eq!(
            split_wide(row)[1..].to_vec(),
            vec!["100.0% (1)", "100.0% (1)", "100.0% (1)", "— (0)", "100.0% (1)", "— (0)", "—"]
        );

        let absent = options(&["--card=core-099"]);
        let nobody = |_: &str| -> Option<String> { None };
        let absent_report = card_stats_report(&db, &absent).await.expect("a report");
        assert!(render_card_stats(&absent_report, &absent, &nobody).contains("No deck the filter counts held core-099."));
        let as_json = options(&["--json"]);
        let parsed: Value =
            serde_json::from_str(&render_card_stats(&report, &as_json, &nobody)).expect("--json prints JSON");
        assert_eq!(parsed, serde_json::to_value(&report).expect("a report serialises"));
    }
}

mod stats_import {
    use super::*;

    #[tokio::test]
    async fn r378_adds_a_development_run_s_records_and_the_same_run_twice_adds_nothing_more() {
        let db = Db::fake();
        let run = [dev("dev:run:1", json!({})), dev("dev:run:2", json!({}))];
        let first = import_dev_records(&db, &format!("{}\n", lines(&run))).await.expect("the run imports");
        assert_eq!((first.read, first.written, first.skipped), (2, 2, 0));
        let again = import_dev_records(&db, &lines(&run)).await.expect("the run imports again");
        assert_eq!((again.read, again.written, again.skipped), (2, 0, 2));
        assert_eq!(stored_ids(&db).await, vec!["dev:run:1", "dev:run:2"]);
    }

    async fn refusal(db: &Db, contents: &str) -> String {
        import_dev_records(db, contents).await.err().expect("the file is refused").to_string()
    }

    #[tokio::test]
    async fn r378_refuses_a_file_with_a_live_record_in_it_and_writes_none_of_it() {
        let db = Db::fake();
        let forged = lines(&[dev("dev:run:1", json!({})), game_record("forged-live", json!({}))]);
        assert!(refusal(&db, &forged).await.contains("forged-live is a live record"));
        assert!(stored_ids(&db).await.is_empty());
        let half = format!("{}\n{{\"id\":\"half\"}}", lines(&[dev("dev:run:1", json!({}))]));
        assert!(refusal(&db, &half).await.starts_with("line 2: "));
        // A development record may not take the place of a live game, whose id is its match id.
        let misnamed = lines(&[dev("dev:run:1", json!({})), dev("5f0c3c1e-0000-4000-8000-000000000001", json!({}))]);
        assert!(refusal(&db, &misnamed).await.contains("does not begin \"dev:\""));
        assert!(stored_ids(&db).await.is_empty());
    }

    #[test]
    fn reads_a_relative_path_from_the_directory_the_command_was_started_in_where_ai_stats_out_wrote_it() {
        // pnpm ran the TS script in apps/server and passed the caller's directory as INIT_CWD.
        let init_cwd: IndexMap<String, String> = [("INIT_CWD".to_owned(), "/repo".to_owned())].into_iter().collect();
        let none: IndexMap<String, String> = IndexMap::new();
        assert_eq!(
            run_file_path("v0.2.5-dev.jsonl", &init_cwd, "/repo/apps/server"),
            PathBuf::from("/repo/v0.2.5-dev.jsonl")
        );
        assert_eq!(run_file_path("runs/a.jsonl", &none, "/repo/apps/server"), PathBuf::from("/repo/apps/server/runs/a.jsonl"));
        assert_eq!(run_file_path("/runs/a.jsonl", &init_cwd, "/repo/apps/server"), PathBuf::from("/runs/a.jsonl"));
    }
}
