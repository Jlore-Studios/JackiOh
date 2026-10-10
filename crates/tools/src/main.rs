//! `jackioh`, the project's command-line tools (SURFACE §12), run as `cargo jackioh <cmd>`
//! (`.cargo/config.toml`'s alias). One subcommand per row of §12's table; each module defines its
//! own `#[derive(clap::Args)] pub struct Args` and `pub fn run(args: Args) -> anyhow::Result<()>`,
//! and is owned by the part §12 names. Written once by part 1.
//!
//! Exit codes: 0 on success, 1 on a failed check (a `run` that returns `Err`), 2 on bad usage
//! (clap). Output goes to stdout; nothing is interactive.

mod agent;
mod arena;
mod catalog;
mod fuzz;
mod gate;
mod golden;
mod luau;
mod patches;
mod promote;
mod replay;
mod spec;
mod stats;
mod sweep;
mod trace;

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
    /// Random-policy games as `fuzz.test.ts` deals them, I1–I4 each step, replay hash checked (part 22).
    Fuzz(fuzz::Args),
    /// Fold `{seed, decks, log, …}` from stdin; print `{"hash", "errors"}` (part 22).
    Replay(replay::Args),
    /// Print one gate game turn by turn (part 22).
    Trace(trace::Args),
    /// `catalog check` (the catalog's data checks) and `catalog loc <path>` (part 22).
    Catalog(catalog::Args),
    /// Print the newest version in `patches.json` (part 22).
    CatalogVersion(catalog::VersionArgs),
    /// The card patch history: `patches <version> <date> "<title>"`, `patches check`, `patches ship` (part 22).
    Patches(patches::Args),
    /// The AI's quality gates and `gate merge <dir>` (part 22).
    Gate(gate::Args),
    /// R186's shadow-ban sweep (part 22).
    Sweep(sweep::Args),
    /// R378's development run for the card statistics (part 22).
    Stats(stats::Args),
    /// Golden traces: `golden check`, `golden bless`, `golden record` (part 23).
    Golden(golden::Args),
    /// Two builds compared: `luau diff`, `luau bench` (v0.4.0's proofs, #442).
    Luau(luau::Args),
    /// The spec graph: `spec check`, `spec index` (part 28).
    Spec(spec::Args),
    /// The training arena (part 29).
    Arena(arena::Args),
    /// The agent protocol on stdin and stdout (part 29).
    Agent(agent::Args),
    /// The promotion gate (part 29).
    Promote(promote::Args),
}

fn main() -> ExitCode {
    let parsed = Cli::parse();
    let result = match parsed.command {
        Command::Fuzz(args) => fuzz::run(args),
        Command::Replay(args) => replay::run(args),
        Command::Trace(args) => trace::run(args),
        Command::Catalog(args) => catalog::run(args),
        Command::CatalogVersion(args) => catalog::run_version(args),
        Command::Patches(args) => patches::run(args),
        Command::Gate(args) => gate::run(args),
        Command::Sweep(args) => sweep::run(args),
        Command::Stats(args) => stats::run(args),
        Command::Golden(args) => golden::run(args),
        Command::Luau(args) => luau::run(args),
        Command::Spec(args) => spec::run(args),
        Command::Arena(args) => arena::run(args),
        Command::Agent(args) => agent::run(args),
        Command::Promote(args) => promote::run(args),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::FAILURE
        }
    }
}
