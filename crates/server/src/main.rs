//! `jackioh-server`: the server binary and its command-line tools (SURFACE §11.1), replacing
//! `apps/server/package.json`'s scripts. `serve` is the default; `release` is what the Docker
//! image runs on Render (migrate, seed the catalog, serve). Every other subcommand is one of the
//! TS `main()`s under `apps/server/src/db/`, handed the arguments after its name, so each keeps
//! its own argument parser (`parseMintArgs`, `parseCardStatsArgs`, …) exactly as TS has it.
//!
//! A failure prints its message on stderr and exits 1, as every TS `main().catch` did; bad usage
//! exits 2 (clap).

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use jackioh_server::{app, cli, db};

#[derive(Parser)]
#[command(name = "jackioh-server", version, about = "The JackiOh server and its tools (SURFACE §11)")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Serve the HTTP API and the match actors on $PORT (the default; TS `pnpm start`).
    Serve,
    /// Migrate, seed the catalog, then serve: what the Docker image runs (TS `pnpm release` + start).
    Release,
    /// Apply the pending migrations (TS `db:migrate`).
    Migrate(Rest),
    /// Upsert `public.cards` from the compiled-in catalog (TS `db:seed-catalog`).
    SeedCatalog(Rest),
    /// Mint invite codes (TS `codes:mint`).
    MintCode(Rest),
    /// Seed the active test accounts; refused under production (TS `db:seed-accounts`).
    SeedAccounts(Rest),
    /// Open a ranked season (TS `db:season-start`).
    SeasonStart(Rest),
    /// Card win rates off the game records (TS `stats:cards`).
    StatsCards(Rest),
    /// Load an `ai:stats` development run's game records (TS `stats:import`).
    StatsImport(Rest),
}

/// The arguments after the subcommand, passed through untouched to the tool's own parser.
#[derive(clap::Args)]
struct Rest {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

#[tokio::main]
async fn main() -> ExitCode {
    let parsed = Cli::parse();
    let result = match parsed.command.unwrap_or(Command::Serve) {
        Command::Serve => serve().await,
        Command::Release => release().await,
        Command::Migrate(rest) => db::migrate::run(rest.args).await,
        Command::SeedCatalog(rest) => cli::seed_catalog::run(rest.args).await,
        Command::MintCode(rest) => cli::mint_code::run(rest.args).await,
        Command::SeedAccounts(rest) => cli::seed_accounts::run(rest.args).await,
        Command::SeasonStart(rest) => cli::season_start::run(rest.args).await,
        Command::StatsCards(rest) => cli::card_stats::run(rest.args).await,
        Command::StatsImport(rest) => cli::import_dev_records::run(rest.args).await,
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// TS `start(loadServerEnv())`: the environment as `index.ts` reads it (E2E placeholders applied
/// when `E2E` asks), then boot.
async fn serve() -> anyhow::Result<()> {
    let source: indexmap::IndexMap<String, String> = std::env::vars().collect();
    let env = app::load_server_env(&source)?;
    app::serve(env).await
}

/// Render's release: every migration, the catalog, then the server (SURFACE §11.3).
async fn release() -> anyhow::Result<()> {
    db::migrate::run(Vec::new()).await?;
    cli::seed_catalog::run(Vec::new()).await?;
    serve().await
}
