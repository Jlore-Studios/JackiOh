//! `cargo jackioh stats from-selfplay <log>… --out <file>`: the learn lane's self-play exports filed as
//! development records for the card statistics (SPEC §9.11, R378; #1676, R1676000). Each game of a
//! `selfplay` file (`selfplay.rs`, R1006001) becomes one `GameRecord` line, `source: "dev"`, `mode:
//! "random"`, both pilots `ai`: what `cargo jackioh stats` files for a game of its own run, over the
//! same kind of game (R258's All Random deals, the AI at the browser's budget playing both seats).
//! `stats report`, `stats report --games` and `jackioh-server stats-import` read the file as they
//! read a `stats --out` file.
//!
//! A game is folded from its seed under the catalog it was played on (`catalog_for`, R767000) and its
//! final hash and result checked, as `positions` folds it (R1006003): a game whose log no longer
//! reaches them (an engine change since), one played on a catalog this build does not carry, one
//! without a result and one its self-play run marked with faults (R1489000) files nothing, and is
//! counted, never guessed at. The counts are printed at the end.
//!
//! The record's patch is the catalog version its game names (R1371000: as it is named now). Its build
//! (R1367000) is the commit and the AI generation the log carries, so no git history is read and every
//! checkout writes the same file; a log written by a binary without git (`unknown`) files no build.
//!
//! The id is `dev:<patch>:selfplay:<seed>`, the seed being the export's `<base>:<n>`. It never collides
//! with a `stats` run's id (`dev:<patch>:<series>:<n>`, since `stats --series` refuses a series that
//! begins `selfplay:`), nor with the gather lane's (`dev:<patch>:gather:<n>`: the bases `dev` and
//! `gather` are legal for `selfplay`, and their ids stay apart through the `selfplay:` between), nor
//! with the arena's (`dev:<patch>:arena:…`). It is not a forced run's id (R1284000), so `stats-import`
//! takes it.
//!
//! The records are written in the order the files are named and the games read, so the output is
//! byte-identical whatever the thread count, like `selfplay`'s own file.
//!
//! Tooling, so it may read and write files; `crates/ai` and the engine stay pure.

use std::io::{BufRead, BufReader, BufWriter, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use anyhow::Context;
use jackioh_ai::dev_record_id;
use jackioh_engine::{
    FoldArgs, GameMode, GameRecord, GameSource, PerPlayer, Pilot, RecordBuild, catalog_for, fold, hash_state,
    summarize_game, with_shipped,
};
use rayon::prelude::*;
use serde::Serialize;

use crate::positions::Dropped;
use crate::selfplay::{SelfplayGame, UNKNOWN_COMMIT};

/// R1676000: the series an export's records are filed under, ahead of the game's seed. `stats --series`
/// refuses one that begins with it, so no `stats` run can file an id these records hold.
pub(crate) const SELFPLAY_SERIES_PREFIX: &str = "selfplay:";

/// Games read and folded at a time: a chunk's records are written before the next is read.
const CHUNK_GAMES: usize = 2000;

/// `cargo jackioh stats from-selfplay <log>… --out <file>`.
#[derive(clap::Args, Debug, Clone, PartialEq)]
pub struct Args {
    /// `selfplay` files (one game per line), read in the order named. Name each shard once: the
    /// records are not deduplicated.
    #[arg(required = true, value_name = "LOG")]
    pub logs: Vec<PathBuf>,
    /// The record file to write (one GameRecord per line, which `stats-import` reads).
    #[arg(long, value_name = "FILE")]
    pub out: PathBuf,
}

/// A run's counts, printed at the end.
#[derive(Serialize, Default, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Counts {
    pub games: usize,
    pub filed: usize,
    pub no_result: usize,
    pub not_replayed: usize,
    pub earlier_patch: usize,
    pub unclean: usize,
    pub unreadable: usize,
}

impl Counts {
    fn add(&mut self, filed: &Result<GameRecord, Dropped>) {
        self.games += 1;
        match filed {
            Ok(_) => self.filed += 1,
            Err(Dropped::NoResult) => self.no_result += 1,
            Err(Dropped::NotReplayed) => self.not_replayed += 1,
            Err(Dropped::EarlierPatch) => self.earlier_patch += 1,
            Err(Dropped::Unclean) => self.unclean += 1,
        }
    }
}

/// R1676000: `game` folded under its catalog (R767000) and, when it reaches its hash with its
/// result, filed as the development record `dev_game_record` files.
pub(crate) fn record_of(game: &SelfplayGame, build_catalog: &str) -> Result<GameRecord, Dropped> {
    let shipped = catalog_for(game.catalog_now(), build_catalog).map_err(|_| Dropped::EarlierPatch)?;
    if game.result.is_none() {
        return Err(Dropped::NoResult);
    }
    if !game.faults.is_empty() {
        return Err(Dropped::Unclean);
    }
    let args = FoldArgs {
        seed: game.seed.clone(),
        decks: game.decks.clone(),
        log: game.log.clone(),
        ..FoldArgs::default()
    };
    let summary = catch_unwind(AssertUnwindSafe(|| {
        with_shipped(shipped, || {
            let replay = fold(&args);
            let reached = replay.errors.is_empty()
                && hash_state(&replay.state) == game.hash
                && replay.state.result == game.result;
            if reached { summarize_game(&args) } else { None }
        })
    }));
    let summary = summary.ok().flatten().ok_or(Dropped::NotReplayed)?;
    let patch = game.catalog_now().to_string();
    let pilots = PerPlayer::new(Pilot::Ai, Pilot::Ai);
    // R1367000: the build that played it; a binary built without git names no commit.
    let commit = Some(game.commit.as_str()).filter(|commit| *commit != UNKNOWN_COMMIT);
    let build = RecordBuild::of(commit, game.ai_generation, &pilots);
    Ok(GameRecord {
        id: dev_record_id(&patch, &format!("{SELFPLAY_SERIES_PREFIX}{}", game.seed)),
        source: GameSource::Dev,
        mode: GameMode::Random,
        patch,
        pilots,
        game: summary,
        build,
    })
}

/// Files every game of `logs` into `out`, one record per line in game order, and counts what it left out.
pub(crate) fn write_records(logs: &[PathBuf], out: &Path) -> anyhow::Result<Counts> {
    jackioh_cards::register_all();
    let build_catalog = jackioh_cards::catalog_version();
    if let Some(dir) = out.parent()
        && !dir.as_os_str().is_empty()
    {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("stats from-selfplay: cannot create {}", dir.display()))?;
    }
    let file = std::fs::File::create(out)
        .with_context(|| format!("stats from-selfplay: cannot create {}", out.display()))?;
    let mut writer = BufWriter::new(file);

    let mut counts = Counts::default();
    for path in logs {
        let file = std::fs::File::open(path)
            .with_context(|| format!("stats from-selfplay: cannot open {}", path.display()))?;
        let mut lines = BufReader::new(file).lines();
        loop {
            let mut chunk: Vec<SelfplayGame> = Vec::with_capacity(CHUNK_GAMES);
            for line in lines.by_ref().take(CHUNK_GAMES) {
                let line =
                    line.with_context(|| format!("stats from-selfplay: cannot read {}", path.display()))?;
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<SelfplayGame>(&line) {
                    Ok(game) => chunk.push(game),
                    Err(_) => {
                        counts.games += 1;
                        counts.unreadable += 1;
                    }
                }
            }
            if chunk.is_empty() {
                break;
            }
            let filed: Vec<Result<GameRecord, Dropped>> = chunk
                .par_iter()
                .map(|game| record_of(game, build_catalog))
                .collect();
            for result in &filed {
                counts.add(result);
                if let Ok(record) = result {
                    writeln!(writer, "{}", jackioh_cards::versions::game_record_line(record))?;
                }
            }
        }
    }
    writer
        .flush()
        .with_context(|| format!("stats from-selfplay: cannot write {}", out.display()))?;
    Ok(counts)
}

/// `cargo jackioh stats from-selfplay`.
pub fn run(args: &Args) -> anyhow::Result<()> {
    let counts = write_records(&args.logs, &args.out)?;
    println!(
        "{}",
        serde_json::json!({ "out": args.out.display().to_string(), "counts": counts })
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::{
        CardStatsFilter, DEV_RECORD_ID_PREFIX, PLAYER_IDS, PilotFilter, SourceFilter, Tally, Winner,
        card_stats, forced_series,
    };

    use crate::stats::{ReportArgs, game_report, read_records, report};

    /// The real exporter's lines for games `1..=games` of the base `records-test`, on commit `abc123`.
    fn exported(games: i32) -> Vec<SelfplayGame> {
        crate::selfplay::play("records-test", 1, games, "abc123").unwrap()
    }

    /// A scratch directory of its own holding `log.jsonl`, written with `text`.
    fn scratch(text: &str) -> (PathBuf, PathBuf) {
        static SCRATCH: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = SCRATCH.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("jackioh-from-selfplay-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("log.jsonl");
        std::fs::write(&log, text).unwrap();
        (dir, log)
    }

    /// The tally a card's breakdown has over the seats of `record` whose deck holds `card` and whose
    /// summary `held` answers true for: counted the long way, from the game's own summary.
    fn tallied(
        record: &GameRecord,
        card: &str,
        held: impl Fn(&jackioh_engine::SeatSummary) -> bool,
    ) -> Tally {
        let mut tally = Tally::default();
        for seat in PLAYER_IDS {
            let summary = &record.game.seats[seat];
            if summary.deck.iter().any(|id| id == card) && held(summary) {
                tally.games += 1;
                tally.wins += i32::from(record.game.winner == Winner::from(seat));
                tally.draws += i32::from(record.game.winner == Winner::Draw);
            }
        }
        tally
    }

    #[test]
    fn r1676000_a_selfplay_game_files_the_record_a_development_run_files_with_its_card_figures() {
        jackioh_cards::register_all();
        let build = jackioh_cards::catalog_version();
        let game = exported(1).remove(0);
        assert!(game.result.is_some(), "the base's first game finishes");
        assert!(game.faults.is_empty(), "{:?}", game.faults);

        let record = record_of(&game, build).expect("the game files");
        let summary = summarize_game(&FoldArgs {
            seed: game.seed.clone(),
            decks: game.decks.clone(),
            log: game.log.clone(),
            ..FoldArgs::default()
        })
        .unwrap();
        let expected = GameRecord {
            id: format!("dev:{build}:selfplay:records-test:1"),
            source: GameSource::Dev,
            mode: GameMode::Random,
            patch: build.to_string(),
            pilots: PerPlayer::new(Pilot::Ai, Pilot::Ai),
            game: summary,
            build: Some(RecordBuild {
                commit: "abc123".to_string(),
                ai_generation: jackioh_ai::ai_generation(),
            }),
        };
        assert_eq!(record, expected);

        // The card figures the game yields, over the filter a development report reads.
        let filter = CardStatsFilter {
            source: SourceFilter::Dev,
            mode: Some(GameMode::Random),
            patch: Some(build.to_string()),
            pilot: PilotFilter::Ai,
        };
        let stats = card_stats(std::slice::from_ref(&record), &filter);
        assert_eq!((stats.games, stats.decks), (1, 2));
        let mut cards: Vec<String> = PLAYER_IDS
            .into_iter()
            .flat_map(|seat| record.game.seats[seat].deck.clone())
            .collect();
        cards.sort();
        cards.dedup();
        let rows: Vec<&str> = stats.cards.iter().map(|row| row.card.as_str()).collect();
        assert_eq!(rows, cards);
        for row in &stats.cards {
            let card = row.card.as_str();
            assert_eq!(row.in_deck, tallied(&record, card, |_| true), "{card}");
            assert_eq!(
                row.opening_hand,
                tallied(&record, card, |seat| seat.opening.iter().any(|id| id == card)),
                "{card}"
            );
            assert_eq!(
                row.played,
                tallied(&record, card, |seat| seat.played.iter().any(|id| id == card)),
                "{card}"
            );
            assert_eq!(
                row.drawn_not_played,
                tallied(&record, card, |seat| {
                    let reached = seat.opening.iter().chain(&seat.drawn).any(|id| id == card);
                    reached && !seat.played.iter().any(|id| id == card)
                }),
                "{card}"
            );
        }
        assert!(stats.cards.iter().any(|row| row.played.games > 0));
        assert!(stats.cards.iter().any(|row| row.opening_hand.games > 0));
        assert!(stats.cards.iter().any(|row| row.drawn_not_played.games > 0));

        // R1367000: a binary built without git names no commit; a line without a generation, none.
        let mut unknown = game.clone();
        unknown.commit = UNKNOWN_COMMIT.to_string();
        assert_eq!(record_of(&unknown, build).unwrap().build, None);
        let mut ungenerated = game.clone();
        ungenerated.ai_generation = None;
        assert_eq!(
            record_of(&ungenerated, build).unwrap().build,
            Some(RecordBuild {
                commit: "abc123".to_string(),
                ai_generation: None,
            })
        );
    }

    #[test]
    fn r1676000_a_game_that_does_not_replay_has_no_result_or_carries_faults_files_nothing_and_is_counted() {
        jackioh_cards::register_all();
        let build = jackioh_cards::catalog_version();
        let games = exported(2);
        for game in &games {
            assert!(game.result.is_some() && game.faults.is_empty(), "{}", game.seed);
        }
        let mut wrong_hash = games[0].clone();
        wrong_hash.hash = "00000000".to_string();
        let mut no_result = games[0].clone();
        no_result.result = None;
        let mut faulted = games[0].clone();
        faulted.faults = vec!["p1: 1 fallbacks".to_string()];
        let mut other_catalog = games[0].clone();
        other_catalog.catalog = "0.0.0-never".to_string();
        other_catalog.versions = Some(jackioh_engine::config::VERSIONS_SCHEME);

        let mut lines: Vec<String> = [&games[0], &wrong_hash, &no_result, &faulted, &other_catalog]
            .into_iter()
            .map(|game| serde_json::to_string(game).unwrap())
            .collect();
        lines.push("not json".to_string());
        lines.push(serde_json::to_string(&games[1]).unwrap());
        let (dir, log) = scratch(&format!("{}\n", lines.join("\n")));
        let out = dir.join("records.jsonl");

        let counts = write_records(std::slice::from_ref(&log), &out).unwrap();
        assert_eq!(
            counts,
            Counts {
                games: 7,
                filed: 2,
                no_result: 1,
                not_replayed: 1,
                earlier_patch: 1,
                unclean: 1,
                unreadable: 1,
            }
        );

        // `stats-import`'s parser reads exactly what the two clean games file.
        let text = std::fs::read_to_string(&out).unwrap();
        let read = jackioh_cards::versions::read_game_record_lines(&text).unwrap();
        let expected: Vec<GameRecord> = games.iter().map(|game| record_of(game, build).unwrap()).collect();
        assert_eq!(read, expected);
        for record in &read {
            assert_eq!(record.source, GameSource::Dev);
            assert!(record.id.starts_with(DEV_RECORD_ID_PREFIX), "{}", record.id);
            assert_eq!(forced_series(record), None, "{}", record.id);
        }

        // `stats report` and `stats report --games` read the file.
        let args = ReportArgs {
            files: vec![out.clone()],
            source: SourceFilter::Dev,
            mode: None,
            patch: None,
            pilot: None,
            card: None,
            games: false,
            window: None,
            json: false,
        };
        let cards = report(&args).unwrap();
        assert_eq!((cards.games, cards.decks), (2, 4));
        let figures = game_report(
            &read_records(&args.files).unwrap(),
            &CardStatsFilter {
                source: SourceFilter::Dev,
                mode: None,
                patch: None,
                pilot: PilotFilter::Unified,
            },
        );
        assert_eq!(figures.groups.len(), 1);
        let group = &figures.groups[0];
        assert_eq!(group.patch, build);
        assert_eq!(group.mode, GameMode::Random);
        assert_eq!(group.source, GameSource::Dev);
        assert_eq!(group.games, 2);

        // The file is byte-identical whatever the thread count.
        let one = rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap();
        let four = rayon::ThreadPoolBuilder::new().num_threads(4).build().unwrap();
        let (a, b) = (dir.join("one.jsonl"), dir.join("four.jsonl"));
        one.install(|| write_records(std::slice::from_ref(&log), &a).unwrap());
        four.install(|| write_records(std::slice::from_ref(&log), &b).unwrap());
        assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
        assert_eq!(std::fs::read(&a).unwrap(), text.as_bytes());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
