//! SPEC §9.11, R378: internal AI development runs. Pre-release AI-vs-AI games filed as game
//! records (R376) under All Random (R258) with unhandicapped resources (R180).
//! Unbanned decks (shadow ban R186 dealt normally).
//! Supports forced card evaluation (R1284000, `forced_dev_game_config`).
//! Pure and seeded (CLAUDE.md rule 4).

use jackioh_engine::config::DECK_SIZE;
use jackioh_engine::{
    DEV_RECORD_ID_PREFIX, GameMode, GameRecord, GameSource, PerPlayer, Pilot, PlayerId, RecordBuild, Rng,
    summarize_game,
};
use serde::{Deserialize, Serialize};

use crate::config::AI_BUDGET;
use crate::deck::{AiDeckOptions, build_ai_deck};
use crate::match_::{MatchConfig, MatchHooks, MatchSetup, SeatController, play_match};
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
    /// R1284000: a card dealt to one seat in every game (`forced_dev_game_config`); `None` deals R258's decks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub force: Option<String>,
    /// R1367000: the git commit the running tool was built from, filed as each record's build with
    /// this build's AI generation (`ai_generation`); `None` files no build.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// #1697: setup inputs for each game beyond decks (§9.3): dealt seats (R433), last boards
    /// (R417, R564), and Glitch boards (R678).
    #[serde(flatten)]
    pub setup: MatchSetup,
}

/// `crates/ai/generation.json`, compiled in (CLAUDE.md rule 4: data reaches a pure crate only at
/// compile time).
const GENERATION_JSON: &str = include_str!("../generation.json");

/// R1367000: the AI generation this build holds (`crates/ai/generation.json`'s `generation`), which a
/// game record the AI played a seat of files in its build.
pub fn ai_generation() -> Option<i32> {
    serde_json::from_str::<serde_json::Value>(GENERATION_JSON)
        .ok()?
        .get("generation")?
        .as_i64()
        .and_then(|generation| i32::try_from(generation).ok())
}

/// One seat's dealt deck of game `seed`. R258: All Random's deal, `build_ai_deck` with only `banned` banned,
/// as `crates/server`'s engine port deals it, less the AI's soft gate (R1390): an AI pilots both seats, so
/// neither deck holds a gated set's card.
fn dev_deck(seed: &str, seat: &str, banned: &[String]) -> Vec<String> {
    build_ai_deck(
        &mut Rng::new(&format!("{seed}:{seat}-deck"), 0),
        DECK_SIZE,
        &AiDeckOptions {
            banned: Some(banned.to_vec()),
            ..AiDeckOptions::default()
        },
    )
}

/// The game `seed` with the dealt `decks`, and two AI seats at this spec's resources.
fn dev_match(seed: String, decks: (Vec<String>, Vec<String>), budget: Option<SearchBudget>) -> MatchConfig {
    let budget = budget.unwrap_or(AI_BUDGET);
    MatchConfig {
        seed,
        decks,
        handicaps: None,
        controllers: PerPlayer {
            p1: SeatController::Ai {
                budget: Some(budget),
                skill: None,
            },
            p2: SeatController::Ai {
                budget: Some(budget),
                skill: None,
            },
        },
        max_actions: None,
        setup: MatchSetup::default(),
    }
}

/// Game n of a series with dealt decks and AI pilots.
pub fn dev_game_config(n: i32, series: &str, budget: Option<SearchBudget>) -> MatchConfig {
    let seed = format!("{series}:{n}");
    let decks = (dev_deck(&seed, "p1", &[]), dev_deck(&seed, "p2", &[]));
    dev_match(seed, decks, budget)
}

/// R1284000: p1 holds the forced card in an odd game, p2 in an even one.
fn forced_seat(n: i32) -> PlayerId {
    if n % 2 == 1 { PlayerId::P1 } else { PlayerId::P2 }
}

/// Game n of a series with `card` forced into one seat's deck (R1284000, R258).
/// Replaces the card at slot `${seed}:force`.
pub fn forced_dev_game_config(n: i32, series: &str, budget: Option<SearchBudget>, card: &str) -> MatchConfig {
    let seed = format!("{series}:{n}");
    let banned = [card.to_string()];
    let mut decks = (dev_deck(&seed, "p1", &banned), dev_deck(&seed, "p2", &banned));
    let held = match forced_seat(n) {
        PlayerId::P1 => &mut decks.0,
        PlayerId::P2 => &mut decks.1,
    };
    let slot = Rng::new(&format!("{seed}:force"), 0).int(DECK_SIZE) as usize;
    held[slot] = card.to_string();
    dev_match(seed, decks, budget)
}

/// Game n of the run `options` describes: R258's deal, or R1284000's with `options.force` in one seat,
/// created with the run's setup inputs (#1697).
pub fn dev_run_config(n: i32, options: &DevRunOptions) -> MatchConfig {
    let config = match options.force.as_deref() {
        None => dev_game_config(n, &options.series, options.budget),
        Some(card) => forced_dev_game_config(n, &options.series, options.budget, card),
    };
    MatchConfig {
        setup: options.setup.clone(),
        ..config
    }
}

/// R378: development record ID `dev:<patch>:<seed>`.
pub fn dev_record_id(patch: &str, seed: &str) -> String {
    format!("{DEV_RECORD_ID_PREFIX}{patch}:{seed}")
}

/// R376, R378: plays game n and returns a finished GameRecord.
pub fn dev_game_record(n: i32, options: &DevRunOptions) -> Option<GameRecord> {
    let config = dev_run_config(n, options);
    let played = play_match(&config, &mut MatchHooks::default());
    played.result?;
    let game = summarize_game(&config.fold_args(played.log))?;
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
        build: RecordBuild::of(
            options.commit.as_deref(),
            ai_generation(),
            &PerPlayer::new(Pilot::Ai, Pilot::Ai),
        ),
    })
}
