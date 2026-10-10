//! Admin script: the action logs still held, folded into the play telemetry's action timings (SPEC
//! §9.11, R1442).
//!
//! ```text
//! jackioh-server timing-backfill
//! ```
//!
//! Every finished match whose log `public.match_actions` still holds (the retention purge keeps a
//! log `MATCH_ACTION_RETENTION_DAYS`) and that has no action timing yet is folded by
//! `actor::telemetry::replay`, the fold a rebuilt actor makes, and its rows are written as the live
//! path writes them, one transaction per match. The log's `at` stamps stand in for the pushes, so a
//! think time runs from one logged action to the next, and the rows carry no clock and no rank
//! bucket, which the log does not hold. Nothing else is backfilled: emotes and a match's signals
//! never reached the log. A match today's engine no longer folds whole (a logged action it refuses,
//! or a deck it no longer builds) is skipped, since its rows would describe another game; a second
//! run folds it again and skips it again.

use std::panic::AssertUnwindSafe;

use anyhow::{anyhow, bail};

use crate::actor::registry::dealt_for;
use crate::actor::telemetry::{LIVE_PILOTS, replay};
use crate::db::store::{Db, PlayTelemetry, StoreError};
use crate::env::quoted;

const USAGE: &str = "Usage: jackioh-server timing-backfill

  Folds every finished match's action log still held into the play telemetry's action timings
  (R1442), skipping a match that already has them. Takes no options; reads DATABASE_URL.";

/// What one run folded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BackfillOutcome {
    /// Matches whose timings were written.
    pub matches: usize,
    /// The action timings written.
    pub rows: usize,
    /// Matches today's engine no longer folds whole.
    pub skipped: usize,
}

/// R1442: folds every unfolded match (`play_telemetry_unfolded`) into its action timings.
pub async fn backfill(db: &Db) -> Result<BackfillOutcome, StoreError> {
    let mut tx = db.begin(None).await?;
    let unfolded = tx.play_telemetry_unfolded().await?;
    tx.commit().await?;

    let mut outcome = BackfillOutcome::default();
    for match_id in unfolded {
        let mut tx = db.begin(None).await?;
        let Some(match_row) = tx.matches_get(&match_id).await? else {
            continue;
        };
        let log = tx.matches_actions(&match_id).await?;
        let mode = tx.matches_mode_of(&match_id).await?;
        // R258: the deal the registry folds the match with, or the fold is another game.
        let replayed = std::panic::catch_unwind(AssertUnwindSafe(|| {
            replay(&match_row, &log, dealt_for(mode))
        }));
        let recorder = match replayed {
            Ok((recorder, 0)) => recorder,
            Ok((_, refused)) => {
                tracing::warn!(event = "telemetry.backfill_skipped", matchId = %match_id, refused = refused);
                outcome.skipped += 1;
                continue;
            }
            Err(_) => {
                tracing::warn!(event = "telemetry.backfill_skipped", matchId = %match_id, refused = "setup");
                outcome.skipped += 1;
                continue;
            }
        };
        let telemetry = PlayTelemetry {
            action_timings: recorder.finish(&LIVE_PILOTS, None).action_timings,
            ..PlayTelemetry::default()
        };
        tx.play_telemetry_insert(&telemetry).await?;
        tx.commit().await?;
        outcome.matches += 1;
        outcome.rows += telemetry.action_timings.len();
    }
    Ok(outcome)
}

/// `timing-backfill`'s arguments after the subcommand (`main.rs`): none.
pub async fn run(args: Vec<String>) -> anyhow::Result<()> {
    if let Some(arg) = args.first() {
        bail!("Unrecognised argument {}.\n\n{USAGE}", quoted(arg));
    }
    let connection_string = std::env::var("DATABASE_URL").unwrap_or_default();
    if connection_string.is_empty() {
        bail!("DATABASE_URL is not set (see docs/architecture.md, env-var contract).");
    }
    // One connection: this process folds one match at a time and exits.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&connection_string)
        .await
        .map_err(|error| anyhow!("{error}"))?;
    let db = Db::Pg(pool);
    let outcome = backfill(&db).await;
    db.close().await;
    let outcome = outcome?;
    println!(
        "timing-backfill: folded {} matches into {} action timings ({} skipped: their logs no longer fold whole)",
        outcome.matches, outcome.rows, outcome.skipped
    );
    Ok(())
}
