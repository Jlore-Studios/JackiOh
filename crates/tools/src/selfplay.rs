//! `cargo jackioh selfplay`: the learned evaluation's game logs (#466, #1006; `docs/learned-eval.md`
//! D1, D3). The AI plays itself at `AI_GATE_BUDGET`, the browser's budget, on a development run's
//! deals (R258: `dev_game_config`'s All Random decks, nothing banned), through the arena's own play
//! path (`arena::play_game`, each seat seeing only `redact`, R185). Game `n`'s seed is `<base>:<n>`.
//!
//! It stores logs, not features (D3): a log replays exactly from its seed, so `cargo jackioh
//! positions` turns the same games into any version of the features in seconds. Each game is one
//! JSON line, in game order (`SelfplayGame`): its seed, decks, log, final hash, result and turns, the
//! catalog version and the commit it was played on. A game is a function of its seed, so the file is
//! byte-identical for the same base and games whatever `RAYON_NUM_THREADS` says (R1006001). Games are
//! played and written in chunks, so a long export keeps what it has played; each chunk, and any game
//! slower than a minute, is named on stderr. A turn that takes more than `SELFPLAY_TURN_ACTIONS`
//! actions (an endless combo) ends its game without a result (R1006005), and so does a decision
//! whose search runs a board to one of the engine's settle caps, at the cap's first panic (§10.3,
//! R1006007).
//!
//! A base that names a training gate's hold-out deals (`promote::is_gate_seed_base`, R850001) or
//! the improve lane's dev set (`improve-dev`, the screens' deals) is refused, so no screen ever plays
//! a deal the network was fitted on (R1006002).

use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Instant;

use anyhow::{Context, bail};
use jackioh_ai::{AI_GATE_BUDGET, ai_generation, dev_game_config};
use jackioh_engine::{Action, GameResult, PerPlayer, PlayerId};
use serde::{Deserialize, Serialize};

use crate::arena::{AgentSpec, ArenaGame, ArenaOutcome, SeedSet, play_game};
use rayon::prelude::*;

#[derive(clap::Args, Debug)]
pub struct Args {
    /// Games to play: `--from` to `--from + games − 1`.
    #[arg(long)]
    pub games: i32,
    /// The first game's number (game n's seed is `<base>:<n>`), so shards of one export never overlap.
    #[arg(long, default_value_t = 1)]
    pub from: i32,
    /// The seed base. Never a training gate's (`improve`, `unban`, …) nor the dev set's (`improve-dev`).
    #[arg(long)]
    pub seed: String,
    /// The directory the log file goes to: `<out>/selfplay-<base>-<first>-<last>.jsonl`.
    #[arg(long)]
    pub out: PathBuf,
}

/// One self-play game, as `positions` reads it back (`FoldArgs`' `seed`, `decks` and `log`, plus what
/// checks and labels it).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SelfplayGame {
    pub seed: String,
    pub decks: (Vec<String>, Vec<String>),
    pub log: Vec<Action>,
    /// `hash_state` of the final state: a log that no longer folds to it is dropped (R1006003).
    pub hash: String,
    /// `None` when the game ended without a result (the action ceiling, or a controller that threw).
    pub result: Option<GameResult>,
    pub turns: i32,
    /// The catalog version the game was played on (R767000): `positions` folds it under that catalog.
    pub catalog: String,
    /// The commit the binary was built from, or `"unknown"`.
    pub commit: String,
    /// R1367000: the AI generation (`crates/ai/generation.json`) the binary holds, which played both
    /// seats. Absent from a game written before it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai_generation: Option<i32>,
    /// R1371000: the scheme `catalog` is named by (`VERSIONS_SCHEME`); a game written before the
    /// version retcon has none, and its catalog is read through `patches/aliases.json`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub versions: Option<u32>,
}

impl SelfplayGame {
    /// R1371000: the catalog the game was played on, as it is named now.
    pub fn catalog_now(&self) -> &str {
        match self.versions {
            Some(_) => &self.catalog,
            None => jackioh_cards::versions::current_version_name(&self.catalog),
        }
    }
}

/// R1006005: the most actions one turn of a self-play game may take. A whole game takes about 60
/// (131 at the 99th percentile of 448 games, 151 at the most), so only an endless combo reaches it:
/// The Coin and a fused card that hands back another, played by an AI that finds each loop worth
/// it (game 474 of `learned-dev`, #1006), which would otherwise run to `ARENA_MAX_ACTIONS` at a
/// second a decision. Such a game ends without a result, and `positions` drops it.
pub(crate) const SELFPLAY_TURN_ACTIONS: usize = 100;

/// R1006002: whether a base names deals a screen or a gate plays: a training gate's hold-out deals
/// (`promote::is_gate_seed_base`) or the improve lane's dev set (`--set improve-dev`).
pub(crate) fn refused_base(base: &str) -> Option<String> {
    if crate::promote::is_gate_seed_base(base) {
        return Some(format!(
            "selfplay: the seed base \"{base}\" names a training gate's hold-out deals (R850001, R1006002)"
        ));
    }
    let dev = SeedSet::ImproveDev.base();
    if base
        .strip_prefix(dev)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(':'))
    {
        return Some(format!(
            "selfplay: the seed base \"{base}\" names the improve lane's dev set, which the screens play (R1006002)"
        ));
    }
    None
}

/// The arena games `games` from `from` on `base`: the dev run's deals (R258), both seats this build's
/// AI at full skill.
fn setups(base: &str, from: i32, games: i32) -> Vec<ArenaGame> {
    (from..from + games)
        .map(|n| {
            let config = dev_game_config(n, base, Some(AI_GATE_BUDGET));
            ArenaGame {
                n,
                seed: config.seed,
                decks: config.decks,
                seats: PerPlayer::new(AgentSpec::SelfAi, AgentSpec::SelfAi),
                skills: PerPlayer::new(jackioh_ai::AI_SKILL_MAX, jackioh_ai::AI_SKILL_MAX),
                a_seat: PlayerId::P1,
                labels: ("self".to_string(), "self".to_string()),
                paired: false,
                turn_actions: Some(SELFPLAY_TURN_ACTIONS),
                stop_unsettled: true,
            }
        })
        .collect()
}

/// The commit this checkout is on, for the logs' provenance; `"unknown"` without git.
fn commit() -> String {
    build_commit().unwrap_or_else(|| "unknown".to_string())
}

/// R1367000: the commit this tool was built from, which the games it plays file as their build:
/// `$GITHUB_SHA` on CI, else this checkout's `HEAD`; `None` without git. Read once per run.
pub(crate) fn build_commit() -> Option<String> {
    static COMMIT: OnceLock<Option<String>> = OnceLock::new();
    COMMIT
        .get_or_init(|| {
            if let Ok(sha) = std::env::var("GITHUB_SHA")
                && !sha.is_empty()
            {
                return Some(sha);
            }
            std::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(env!("CARGO_MANIFEST_DIR"))
                .output()
                .ok()
                .filter(|out| out.status.success())
                .and_then(|out| String::from_utf8(out.stdout).ok())
                .map(|text| text.trim().to_string())
                .filter(|text| !text.is_empty())
        })
        .clone()
}

fn game_of(outcome: ArenaOutcome, catalog: &str, commit: &str) -> SelfplayGame {
    SelfplayGame {
        seed: outcome.game.seed,
        decks: outcome.game.decks,
        log: outcome.log,
        hash: outcome.hash,
        result: outcome.result,
        turns: outcome.turns,
        catalog: catalog.to_string(),
        commit: commit.to_string(),
        versions: Some(jackioh_engine::config::VERSIONS_SCHEME),
        ai_generation: ai_generation(),
    }
}

/// The export's file name: `selfplay-<base>-<first>-<last>.jsonl`, the base's `:` and `/` as `_`.
pub(crate) fn file_name(base: &str, from: i32, games: i32) -> String {
    let safe: String = base
        .chars()
        .map(|c| if c == ':' || c == '/' || c == '\\' { '_' } else { c })
        .collect();
    format!("selfplay-{safe}-{from}-{}.jsonl", from + games - 1)
}

/// Games played and written at a time: a chunk's lines are on disk before the next chunk starts, in
/// game order, so a long export keeps what it has played and its file is the same whatever the chunk.
const CHUNK_GAMES: i32 = 64;

/// A game slower than this, in seconds of wall time, is named on stderr as it finishes.
const SLOW_GAME_SECONDS: f64 = 60.0;

/// Plays games `from..from + games` in chunks of `CHUNK_GAMES`, handing each chunk's lines, in game
/// order, to `sink` before the next chunk starts. Progress and slow games go to stderr.
pub(crate) fn play_chunks(
    base: &str,
    from: i32,
    games: i32,
    commit: &str,
    mut sink: impl FnMut(&[SelfplayGame]) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    jackioh_cards::register_all();
    let catalog = jackioh_cards::catalog_version();
    let end = from + games;
    let mut start = from;
    while start < end {
        let count = CHUNK_GAMES.min(end - start);
        eprintln!(
            "{}",
            serde_json::json!({ "selfplay": "chunk", "from": start, "to": start + count - 1, "of": end - 1 })
        );
        let played: Vec<(ArenaOutcome, f64)> = setups(base, start, count)
            .par_iter()
            .map(|game| {
                let clock = Instant::now();
                play_game(game).map(|outcome| (outcome, clock.elapsed().as_secs_f64()))
            })
            .collect::<anyhow::Result<_>>()?;
        let mut lines = Vec::with_capacity(played.len());
        for (outcome, seconds) in played {
            if seconds > SLOW_GAME_SECONDS {
                eprintln!(
                    "{}",
                    serde_json::json!({
                        "selfplay": "slow game", "seed": outcome.game.seed, "seconds": seconds.round(),
                        "turns": outcome.turns, "actions": outcome.log.len(), "result": outcome.result.is_some(),
                    })
                );
            }
            lines.push(game_of(outcome, catalog, commit));
        }
        sink(&lines)?;
        start += count;
    }
    Ok(())
}

/// Plays the games and returns their lines, in game order (the tests' way in; `run` streams).
#[cfg(test)]
pub(crate) fn play(base: &str, from: i32, games: i32, commit: &str) -> anyhow::Result<Vec<SelfplayGame>> {
    let mut all = Vec::new();
    play_chunks(base, from, games, commit, |chunk| {
        all.extend_from_slice(chunk);
        Ok(())
    })?;
    Ok(all)
}

pub fn run(args: Args) -> anyhow::Result<()> {
    if let Some(refusal) = refused_base(&args.seed) {
        bail!("{refusal}");
    }
    if args.games < 1 {
        bail!("selfplay: --games must be at least 1, not {}", args.games);
    }
    if args.from < 1 {
        bail!("selfplay: --from must be at least 1, not {}", args.from);
    }
    let path = args.out.join(file_name(&args.seed, args.from, args.games));
    std::fs::create_dir_all(&args.out)
        .with_context(|| format!("selfplay: cannot create {}", args.out.display()))?;
    let file = std::fs::File::create(&path)
        .with_context(|| format!("selfplay: cannot create {}", path.display()))?;
    let mut writer = BufWriter::new(file);
    let (mut played, mut finished, mut turns, mut catalog) = (0usize, 0usize, 0i64, None::<String>);
    play_chunks(&args.seed, args.from, args.games, &commit(), |chunk| {
        for game in chunk {
            serde_json::to_writer(&mut writer, game)?;
            writer.write_all(
                b"
",
            )?;
            played += 1;
            finished += usize::from(game.result.is_some());
            turns += i64::from(game.turns);
            catalog.get_or_insert_with(|| game.catalog.clone());
        }
        writer
            .flush()
            .with_context(|| format!("selfplay: cannot write {}", path.display()))
    })?;
    println!(
        "{}",
        serde_json::json!({
            "out": path.display().to_string(),
            "games": played,
            "finished": finished,
            "turns": turns,
            "catalog": catalog,
        })
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r1006002_a_gate_or_dev_set_base_is_refused_and_any_other_is_not() {
        for base in [
            "improve",
            "improve:abc",
            "unban",
            "unban:1",
            "improve-dev",
            "improve-dev:7",
        ] {
            assert!(refused_base(base).is_some(), "{base} should be refused");
        }
        for base in ["learned", "learned:1", "improvement", "dev", "export-2026-10-16"] {
            assert!(refused_base(base).is_none(), "{base} should be allowed");
        }
    }

    /// R1006005: a turn that passes the ceiling ends its game without a result, which `positions`
    /// drops; the export's own ceiling is far above any real turn's.
    #[test]
    fn r1006005_a_turn_past_the_ceiling_ends_the_game_without_a_result() {
        jackioh_cards::register_all();
        let mut game = setups("selfplay-ceiling", 1, 1).remove(0);
        assert_eq!(game.turn_actions, Some(SELFPLAY_TURN_ACTIONS));
        game.turn_actions = Some(1);
        let outcome = play_game(&game).unwrap();
        assert_eq!(outcome.result, None);
        assert!(
            outcome.thrown.iter().any(|why| why.contains("R1006005")),
            "{:?}",
            outcome.thrown
        );
        assert!(outcome.turns <= 2, "it ended in its first turn");
        assert!(game.stop_unsettled, "R1006007");
        let line = game_of(outcome, jackioh_cards::catalog_version(), "test");
        assert_eq!(
            crate::positions::fold_game(
                &line,
                crate::positions::Every::Turn,
                jackioh_cards::catalog_version(),
                false
            ),
            Err(crate::positions::Dropped::NoResult)
        );
    }

    /// R1006007: watched work gives its value, past any panic it catches that is not a settle cap's,
    /// and its watcher stops waiting at a cap's first panic, caught, while the work would run on.
    #[test]
    fn r1006007_the_watcher_stops_waiting_at_a_settle_caps_first_panic() {
        use crate::arena::watch_unsettled;
        use std::panic::catch_unwind;
        assert_eq!(watch_unsettled(|| 7), Some(7));
        assert_eq!(
            watch_unsettled(|| catch_unwind(|| panic!("an endTurn's opponent draw threw")).is_err()),
            Some(true),
            "another panic is the work's own"
        );
        let (release, wait) = std::sync::mpsc::channel::<()>();
        let watched = watch_unsettled(move || {
            let _ = catch_unwind(|| panic!("the resolution loop did not settle in 3 passes (§10.3)"));
            // The search would go on and reach the loop again; this work waits for the test instead.
            let _ = wait.recv();
            7
        });
        assert_eq!(watched, None);
        drop(release);
    }

    /// R1006007: each of the engine's settle caps panics with the words the hook listens for.
    #[test]
    fn r1006007_every_settle_cap_panics_with_the_counted_words() {
        let sources = [
            include_str!("../../engine/src/triggers.rs"),
            include_str!("../../engine/src/state_check.rs"),
            include_str!("../../engine/src/effects/combat.rs"),
        ];
        let caps: Vec<&str> = sources
            .iter()
            .flat_map(|source| source.lines())
            .filter(|line| line.contains("panic!(") && line.contains("passes"))
            .collect();
        assert!(caps.len() >= 4, "{caps:?}");
        for cap in caps {
            assert!(cap.contains(crate::arena::UNSETTLED_PANIC), "{cap}");
        }
    }

    #[test]
    fn the_file_name_names_the_base_and_the_range() {
        assert_eq!(file_name("export:a", 101, 50), "selfplay-export_a-101-150.jsonl");
    }

    /// R1006001: the same base and count give the same lines whatever the thread count.
    #[test]
    fn r1006001_the_same_base_and_count_give_byte_identical_lines_on_one_or_four_threads() {
        let one = rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap();
        let four = rayon::ThreadPoolBuilder::new().num_threads(4).build().unwrap();
        let text = |games: Vec<SelfplayGame>| -> String {
            games
                .iter()
                .map(|game| serde_json::to_string(game).unwrap())
                .collect::<Vec<_>>()
                .join("\n")
        };
        let a = one.install(|| play("selfplay-test", 1, 3, "test").unwrap());
        let b = four.install(|| play("selfplay-test", 1, 3, "test").unwrap());
        assert_eq!(a.len(), 3);
        assert_eq!(text(a), text(b));
    }
}
