//! The finished games a host keeps (R768, R767000, R768000): which of them this build can fold, how one
//! is opened and paged, and the seat the human played at each step. The store is the host's: every call
//! takes what it read (the kept games, the verdicts of the ones opened) and the machine keeps nothing
//! between calls. A host that opens a game keeps the `ReplayOpen` it was answered with, which holds the
//! folded checkpoints, and hands it back for the next page (the browser's worker holds the last few).

use std::panic::{AssertUnwindSafe, catch_unwind};

use serde::{Deserialize, Serialize};

use jackioh_engine::{
    Action, CreateGameOptions, PlayerId, ReplayOpen, ReplayRecord, ReplayRefusal, ReplayStep, SetupKind,
};

use super::deal::deal;
use super::host::{catalog_of, replay_input};
use super::types::{PracticeReplay, PracticeReplayListing};

/// R768: what the host learned of a kept game when it opened it: `None` when it folds, else part 1's
/// refusal. The list reads it, since listing folds no game.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct ReplayVerdict {
    pub game: u32,
    pub refusal: Option<ReplayRefusal>,
}

/// R768, R767000: whether this build can fold a kept game: one played on its own catalog version (`build`),
/// or on a version it carries when the game kept its setup (dealing it again would read this build's
/// catalog).
pub fn foldable(replay: &PracticeReplay, build: &str) -> bool {
    let catalog = catalog_of(&replay.catalog, replay.versions);
    catalog == build || (replay.setup.is_some() && jackioh_engine::catalog_for(&catalog, build).is_ok())
}

/// R768: no kept game is folded to be listed (newest first); one unopened is listed unavailable only
/// when this build cannot fold its catalog version (R767000).
pub fn list_replays(
    replays: &[PracticeReplay],
    verdicts: &[ReplayVerdict],
    build: &str,
) -> Vec<PracticeReplayListing> {
    replays
        .iter()
        .rev()
        .map(|replay| {
            let unavailable = if foldable(replay, build) {
                verdicts
                    .iter()
                    .find(|verdict| verdict.game == replay.summary.game)
                    .and_then(|verdict| verdict.refusal)
            } else {
                Some(ReplayRefusal::EarlierPatch)
            };
            PracticeReplayListing {
                summary: replay.summary.clone(),
                unavailable,
            }
        })
        .collect()
}

/// R768, R767000: the setup a kept game folds from: the one it kept, else (a game kept before setups
/// were, on this build's catalog only) the setup its config deals.
fn fold_options(replay: &PracticeReplay) -> Result<CreateGameOptions, String> {
    let Some(setup) = &replay.setup else {
        return Ok(deal(&replay.config)?.options);
    };
    Ok(CreateGameOptions {
        seed: replay.config.seed.clone(),
        decks: setup.decks.clone(),
        handicaps: Some(setup.handicaps.clone()),
        dealt: setup.dealt.clone(),
        last_boards: setup.last_boards.clone(),
        sandbox: replay.config.is_sandbox().then_some(true),
        ..CreateGameOptions::default()
    })
}

/// R768: a kept game folded once and checked against its catalog and hash. Refused before any deck is
/// dealt when this build cannot fold its catalog (another catalog may lack a preset the config names);
/// a setup that no longer deals no longer folds.
pub fn open_replay(replay: &PracticeReplay, build: &str) -> ReplayOpen {
    if !foldable(replay, build) {
        return ReplayOpen::Refused {
            reason: ReplayRefusal::EarlierPatch,
        };
    }
    let catalog = catalog_of(&replay.catalog, replay.versions);
    let opened = catch_unwind(AssertUnwindSafe(|| -> Result<ReplayOpen, String> {
        let options = fold_options(replay)?;
        // The setup is checked first, under the catalog the game was played on (R767000): the engine
        // would panic on a deck its seat's handicap does not allow.
        let shipped = jackioh_engine::catalog_for(&catalog, build).ok().flatten();
        jackioh_engine::with_shipped(shipped, || {
            jackioh_engine::validate_setup(
                &options,
                jackioh_engine::registered_catalog(),
                SetupKind::Recorded,
            )
        })
        .map_err(|error| error.to_string())?;
        let record = ReplayRecord {
            catalog_version: catalog.clone(),
            final_hash: replay.hash.clone(),
        };
        Ok(jackioh_engine::replay_open(
            &replay_input(&options, replay.log.clone()),
            &record,
            build,
        ))
    }));
    match opened {
        Ok(Ok(open)) => open,
        _ => ReplayOpen::Refused {
            reason: ReplayRefusal::RulesChanged,
        },
    }
}

/// R677, R768: the seat the human played at step `from` of a kept game, and for how many steps up to
/// `count`, so a page stops where a Glitch swap moved the human. The last step's is the one it ended in.
pub fn replay_seat_run(log: &[Action], end_seat: PlayerId, from: usize, count: usize) -> (PlayerId, usize) {
    let seat_at = |step: usize| match log.get(step) {
        None => end_seat,
        Some(action) if action.nonce.starts_with('h') => action.player_id,
        Some(action) => action.player_id.opponent(),
    };
    let seat = seat_at(from);
    let mut run = 1;
    while run < count && from + run <= log.len() && seat_at(from + run) == seat {
        run += 1;
    }
    (seat, run)
}

/// What a replay request is answered with: the open (for the host to keep for the next page) and the
/// page, or why there is none.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct ReplayAnswer {
    pub open: ReplayOpen,
    pub page: ReplayPageResult,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "page", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ReplayPageResult {
    /// Steps `from`, … as the human saw them (R677): fewer than asked at a Glitch swap or the reduce
    /// budget, so the next page starts at `from + steps.len()`.
    Page {
        from: u32,
        steps: Vec<ReplayStep>,
        total: u32,
    },
    /// Why there is no page: the game cannot be replayed, or the steps asked are not in it.
    Refused { message: String },
}

/// R768: steps `from`, … (`count` of them) of a kept game, `open` being what an earlier call answered
/// (`None`: fold the game now).
pub fn page_replay(
    replay: &PracticeReplay,
    open: Option<ReplayOpen>,
    from: i64,
    count: i64,
    build: &str,
) -> ReplayAnswer {
    let game = replay.summary.game;
    let open = open.unwrap_or_else(|| open_replay(replay, build));
    let page = match &open {
        ReplayOpen::Refused { reason } => ReplayPageResult::Refused {
            message: format!("practice game {game} cannot be replayed: {reason}"),
        },
        ReplayOpen::Ready { steps, checkpoints } => {
            let asked = usize::try_from(from)
                .ok()
                .zip(usize::try_from(count).ok())
                .filter(|(from, count)| from < steps && *count >= 1);
            match asked {
                None => ReplayPageResult::Refused {
                    message: format!("no step {from} ({count} asked) in a replay of {steps} steps"),
                },
                Some((from, count)) => {
                    let (seat, run) = replay_seat_run(&replay.log, replay.end_seat, from, count);
                    let page = jackioh_engine::replay_page(checkpoints, seat, from, run);
                    ReplayPageResult::Page {
                        from: u32::try_from(page.from).unwrap_or(u32::MAX),
                        steps: page.steps,
                        total: u32::try_from(*steps).unwrap_or(u32::MAX),
                    }
                }
            }
        }
    };
    ReplayAnswer { open, page }
}
