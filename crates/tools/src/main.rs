//! `jackioh`, the project's command-line tools (SURFACE §12), run as `cargo jackioh <cmd>`, one
//! subcommand per row of §12's table. Exit codes: 0 on success, 1 on a failed check (a `run` that
//! returns `Err`), 2 on bad usage (clap). Output goes to stdout (a fuzz wave's progress lines go to
//! stderr, `seed_watch.rs`); nothing is interactive.

mod agent;
mod arena;
mod assault_sim;
mod board;
mod card_lists;
mod carried;
mod catalog;
mod changed;
mod claims;
mod classify;
mod duos_fuzz;
mod duos_gate;
mod fork_screen;
mod fused_pairs;
mod fuzz;
mod fx_census;
mod gate;
mod golden;
mod names;
mod patches;
mod perf;
mod replay;
mod retcon;
mod run_sim;
mod sample_size;
mod seed_watch;
mod selfplay;
mod spec;
mod spec_cards;
mod stats;
mod steer;
mod sweep;
mod technical;
mod think;
mod trace;
mod win_probability;
mod winrates;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "jackioh", version, about = "JackiOh's tools (SURFACE §12)")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Random-policy games as `fuzz.test.ts` deals them, I1–I23 each step, replay hash checked (part 22).
    Fuzz(fuzz::Args),
    /// Each triggered card's text fused onto each fusable Unit and played until the board settles; a daily sample, or one pair (#2381).
    FusedPairs(fused_pairs::Args),
    /// Each card's era on every older carried catalog, played and folded back as a replay pinned to it (#4382).
    Carried(carried::Args),
    /// Whole dungeon runs with the AI in the player's seat, and how each kind of fight goes (#3065).
    RunSim(run_sim::Args),
    /// Whole Infinite Assault days with the AI in the player's seat, and how each level goes (#4583).
    AssaultSim(assault_sim::Args),
    /// What each card's plays and hooks do on the board, from seeded games, with a suggested motif (#2788).
    Fx(fx_census::Args),
    /// Fold `{seed, decks, log, …}` from stdin; print `{"hash", "errors"}` (part 22).
    Replay(replay::Args),
    /// Hash one state object from stdin by the engine's hash_state; print `{"hash"}` (#4766).
    HashState(replay::HashStateArgs),
    /// Fold `{seed, decks, log, state, …}` from stdin and hash its `state`; print `{"replayHash", "browserHash", "errors"}` (#4925).
    ReplayCheck(replay::ReplayCheckArgs),
    /// Print one gate game turn by turn (part 22).
    Trace(trace::Args),
    /// Fold `{seed, decks, log, …}` to one position and print it and the AI's decision there (#834).
    Think(think::Args),
    /// `catalog check` (the catalog's data checks) and `catalog loc <path>` (part 22).
    Catalog(catalog::Args),
    /// What a change between two commits does: its kind, and for cards its tier (#1365, R1365000).
    Classify(classify::Args),
    /// The cards a pull request changes, as fuzz's gate reads them; none off a merge commit (#5099).
    ChangedCards(changed::Args),
    /// Print the newest version in `patches.json` (part 22).
    CatalogVersion(catalog::VersionArgs),
    /// The card patch history: `patches <version> <date> "<title>"`, `patches check`, `patches ship` (part 22), `patches log <base>..<head>` (R1369000).
    Patches(patches::Args),
    /// The AI's quality gates and `gate merge <dir>` (part 22).
    Gate(gate::Args),
    /// The engine's and the replay fold's speed on a frozen corpus, each median held to its budget (#3429).
    Perf(perf::Args),
    /// R186's shadow-ban sweep (part 22).
    Sweep(sweep::Args),
    /// R378's development run for the card statistics (part 22).
    Stats(stats::Args),
    /// Golden traces: `golden check`, `golden record` (part 23; `bless` retired in #2578).
    Golden(golden::Args),
    /// The spec graph: `spec check`, `spec index` (part 28).
    Spec(spec::Args),
    /// A card's name comments, written from catalog.json (#3111).
    Names(names::Args),
    /// The training arena (part 29).
    Arena(arena::Args),
    /// The agent protocol on stdin and stdout (part 29).
    Agent(agent::Args),
    /// Self-play game logs (#1006), which `classify` compares across two builds (R1365000).
    Selfplay(selfplay::Args),
    /// `winrates --refresh <report.json>`: the win rates of record from production's windowed card figures, once per patch (R3882000).
    Winrates(winrates::Args),
}

fn main() -> ExitCode {
    let parsed = Cli::parse();
    let result = match parsed.command {
        Command::Fuzz(args) => fuzz::run(args),
        Command::FusedPairs(args) => fused_pairs::run(args),
        Command::Carried(args) => carried::run(args),
        Command::RunSim(args) => run_sim::run(args),
        Command::AssaultSim(args) => assault_sim::run(args),
        Command::Fx(args) => fx_census::run(args),
        Command::Replay(args) => replay::run(args),
        Command::HashState(args) => replay::run_hash_state(args),
        Command::ReplayCheck(args) => replay::run_replay_check(args),
        Command::Trace(args) => trace::run(args),
        Command::Think(args) => think::run(args),
        Command::Catalog(args) => catalog::run(args),
        Command::Classify(args) => classify::run(args),
        Command::ChangedCards(args) => changed::run(args),
        Command::CatalogVersion(args) => catalog::run_version(args),
        Command::Patches(args) => patches::run(args),
        Command::Gate(args) => gate::run(args),
        Command::Perf(args) => perf::run(args),
        Command::Sweep(args) => sweep::run(args),
        Command::Stats(args) => stats::run(args),
        Command::Golden(args) => golden::run(args),
        Command::Spec(args) => spec::run(args),
        Command::Names(args) => names::run(args),
        Command::Arena(args) => arena::run(args),
        Command::Agent(args) => agent::run(args),
        Command::Selfplay(args) => selfplay::run(args),
        Command::Winrates(args) => winrates::run(args),
    };
    // R3925000: the measurement behind `RESOLUTION_EVENT_CAP`, the most events one interaction emitted in
    // the run (crates/tools/README.md, `JACKIOH_LOOP_PEAK`).
    if std::env::var_os("JACKIOH_LOOP_PEAK").is_some() {
        let peak = jackioh_engine::testkit::loop_peak::peak();
        let kind = peak.kind.map_or("setup", |kind| kind.as_str());
        eprintln!(
            "loop peak: {} events in one interaction ({kind}, turn {}); {} interaction(s) capped",
            peak.events, peak.turn, peak.capped
        );
    }
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::FAILURE
        }
    }
}
