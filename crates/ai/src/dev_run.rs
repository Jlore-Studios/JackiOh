//! SPEC §9.11, R378: an internal AI development run. AI-against-AI games played on a build before its
//! patch ships, each filed as a game record (R376) of its own source, so the card win rates of a
//! patch's pre-release run can be compared with those of the live games played on it once it ships.
//!
//! A development game is an All Random game (R258) with two AI pilots: both decks are dealt by the
//! game's weighted random deck-builder with nothing banned, from the game seed and the seat
//! (`${seed}:p1-deck`, `${seed}:p2-deck`), exactly as the server deals a live one; both seats play on
//! this spec's own resources, no handicap (R180), at the browser's budget. So a run files under All
//! Random, and compares with that mode's live games like for like. Its source is "dev", which no live
//! figure counts unless it is asked for (R378). A card the shadow ban keeps out of the AI's own decks
//! (R186) is dealt here like any other — All Random bans nothing — and is one the AI is known to play
//! badly, so its development figures say more about the AI than about the card.
//!
//! Pure and seeded like the rest of src/ (CLAUDE.md rule 4): `cargo jackioh stats` loops over the
//! games and writes the file.
//!
//! Port of `packages/ai/src/devRun.ts`.

use jackioh_engine::config::DECK_SIZE;
use jackioh_engine::{
    DEV_RECORD_ID_PREFIX, FoldArgs, GameMode, GameRecord, GameSource, PerPlayer, Pilot, Rng, summarize_game,
};
use serde::{Deserialize, Serialize};

use crate::config::AI_BUDGET;
use crate::deck::{AiDeckOptions, build_ai_deck};
use crate::match_::{MatchConfig, MatchHooks, SeatController, play_match};
use crate::types::SearchBudget;

/// `AI_DEV_RUN`'s shape.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiDevRun {
    /// Games a run plays when it is not told how many.
    pub games: i32,
    /// The default seed series: game n's seed is `${series}:${n}`. No gate or tuning run plays it.
    pub series: &'static str,
}

pub const AI_DEV_RUN: AiDevRun = AiDevRun {
    games: 200,
    series: "dev",
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DevRunOptions {
    /// The seed series; a run told another one plays other games.
    pub series: String,
    /// R388's version the run tests: the record's patch.
    pub patch: String,
    /// The AI's budget; the browser's (`AI_BUDGET`) unless a test asks for less.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<SearchBudget>,
}

/// Game n of a series: its seed, its two dealt decks, and two AI seats at this spec's resources. TS
/// took `Pick<DevRunOptions, "series" | "budget">`; here the two fields are the arguments.
pub fn dev_game_config(n: i32, series: &str, budget: Option<SearchBudget>) -> MatchConfig {
    let seed = format!("{series}:{n}");
    let budget = budget.unwrap_or(AI_BUDGET);
    // R258: All Random's deal, `build_ai_deck` with nothing banned, as `crates/server`'s engine port deals it,
    // less the AI's soft gate (R1390): an AI pilots both seats, so neither deck holds a gated set's card.
    let deal = |seat: &str| -> Vec<String> {
        build_ai_deck(
            &mut Rng::new(&format!("{seed}:{seat}-deck"), 0),
            DECK_SIZE,
            &AiDeckOptions {
                banned: Some(Vec::new()),
                ..AiDeckOptions::default()
            },
        )
    };
    let decks = (deal("p1"), deal("p2"));
    MatchConfig {
        seed,
        decks,
        handicaps: None,
        controllers: PerPlayer {
            p1: SeatController::Ai { budget: Some(budget) },
            p2: SeatController::Ai { budget: Some(budget) },
        },
        max_actions: None,
    }
}

/// R378: a development record's id, `dev:<patch>:<seed>`. The patch is part of it, so the same seeds
/// played again for another patch are other records, which `stats:import` adds rather than skipping
/// as a run it has already loaded.
pub fn dev_record_id(patch: &str, seed: &str) -> String {
    format!("{DEV_RECORD_ID_PREFIX}{patch}:{seed}")
}

/// R376, R378: plays game n and files it. `None` when the game did not finish — it hit the match's
/// action limit or an AI panicked — since only a finished game is a record.
pub fn dev_game_record(n: i32, options: &DevRunOptions) -> Option<GameRecord> {
    let config = dev_game_config(n, &options.series, options.budget);
    let played = play_match(&config, &mut MatchHooks::default());
    played.result?;
    let game = summarize_game(&FoldArgs {
        seed: config.seed.clone(),
        decks: config.decks.clone(),
        log: played.log,
        ..FoldArgs::default()
    })?;
    Some(GameRecord {
        id: dev_record_id(&options.patch, &config.seed),
        source: GameSource::Dev,
        mode: GameMode::Random,
        patch: options.patch.clone(),
        pilots: PerPlayer {
            p1: Pilot::Ai,
            p2: Pilot::Ai,
        },
        game,
    })
}
