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
//! A run can instead force one card into one seat's deck (R1284000, `forced_dev_game_config`), so a card
//! a patch changes is in the sample in every game. Its records are for `cargo jackioh stats compare`,
//! not R378's sample of All Random.
//!
//! Pure and seeded like the rest of src/ (CLAUDE.md rule 4): `cargo jackioh stats` loops over the
//! games and writes the file.
//!
//! Port of `packages/ai/src/devRun.ts`.

use jackioh_engine::config::DECK_SIZE;
use jackioh_engine::{
    DEV_RECORD_ID_PREFIX, FoldArgs, GameMode, GameRecord, GameSource, PerPlayer, Pilot, PlayerId, Rng,
    summarize_game,
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
    /// R1284000: a card dealt to one seat in every game (`forced_dev_game_config`); `None` deals R258's decks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub force: Option<String>,
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
    }
}

/// Game n of a series: its seed, its two dealt decks, and two AI seats at this spec's resources. TS
/// took `Pick<DevRunOptions, "series" | "budget">`; here the two fields are the arguments.
pub fn dev_game_config(n: i32, series: &str, budget: Option<SearchBudget>) -> MatchConfig {
    let seed = format!("{series}:{n}");
    let decks = (dev_deck(&seed, "p1", &[]), dev_deck(&seed, "p2", &[]));
    dev_match(seed, decks, budget)
}

/// R1284000: p1 holds the forced card in an odd game, p2 in an even one.
fn forced_seat(n: i32) -> PlayerId {
    if n % 2 == 1 { PlayerId::P1 } else { PlayerId::P2 }
}

/// Game n of a series with `card` in one seat's deck (R1284000). Both decks are R258's deal with `card`
/// banned, so the other seat never holds it and no deal depends on the card's own numbers. The holder's
/// card at the slot `${seed}:force` picks is replaced by `card`. `card` must be a deckable card: the
/// caller resolves it (`cargo jackioh stats --force`).
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

/// R378: a development record's id, `dev:<patch>:<seed>`. The patch is part of it, so the same seeds
/// played again for another patch are other records, which `stats:import` adds rather than skipping
/// as a run it has already loaded.
pub fn dev_record_id(patch: &str, seed: &str) -> String {
    format!("{DEV_RECORD_ID_PREFIX}{patch}:{seed}")
}

/// R376, R378: plays game n and files it. `None` when the game did not finish — it hit the match's
/// action limit or an AI panicked — since only a finished game is a record.
pub fn dev_game_record(n: i32, options: &DevRunOptions) -> Option<GameRecord> {
    let config = match options.force.as_deref() {
        None => dev_game_config(n, &options.series, options.budget),
        Some(card) => forced_dev_game_config(n, &options.series, options.budget, card),
    };
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
