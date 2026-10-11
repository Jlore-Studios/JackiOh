//! Admin script: log-normal fits of human think times, per bucket, to paste into config (SPEC §9.11,
//! R1442).
//!
//! ```text
//! jackioh-server timing-fit
//! ```
//!
//! Reads every row of `public.action_timings` (DATABASE_URL), leaves out the moves an AI made
//! (`pilot = 'ai'`), groups the rest by rank bucket, action kind and whether the move was the
//! seat's first of its turn, and fits each group of at least `TIMING_FIT_MIN_SAMPLES` moves with a
//! log-normal: the maximum-likelihood `mu` and `sigma` of `ln(think_ms)`. It prints one
//! `ThinkTimeFit` per bucket, to be pasted into `THINK_TIME_FITS` in `crates/server/src/config.rs`
//! by hand, as `cargo jackioh sweep` prints its rows (CLAUDE.md rule 9): no number reaches the
//! ladder bots (#636) without a person reading it first. Every row is read into memory, which is
//! fine at today's scale.

use anyhow::{anyhow, bail};
use indexmap::IndexMap;

use jackioh_engine::ActionType;

use crate::cli::card_stats::literal;
use crate::config::TIMING_FIT_MIN_SAMPLES;
use crate::db::store::{ActionTimingRow, Db, Pilot, StoreError};
use crate::env::quoted;
use crate::ranked::ladder::RankTier;

const USAGE: &str = "Usage: jackioh-server timing-fit

  Prints log-normal fits of the human think times in action_timings (R1442), one ThinkTimeFit row
  per bucket, to paste into THINK_TIME_FITS in crates/server/src/config.rs. Takes no options; reads
  DATABASE_URL.";

/// The comment line the printed rows open with.
const HEADER: &str = "// jackioh-server timing-fit: paste into THINK_TIME_FITS, crates/server/src/config.rs";

/// One bucket's fit (`config::ThinkTimeFit`'s fields, as values).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BucketFit {
    pub rank_bucket: Option<RankTier>,
    pub action_kind: ActionType,
    pub first_in_turn: bool,
    pub samples: usize,
    pub mu: f64,
    pub sigma: f64,
}

/// R1442: every bucket of human moves with at least `min_samples` of them, in key order (no rank
/// first, then the tiers lowest first; the action kinds in `ActionType`'s order; later moves before
/// first ones). A think time of 0 counts as 1 ms, so its logarithm is defined.
pub fn fit_think_times(rows: &[ActionTimingRow], min_samples: usize) -> Vec<BucketFit> {
    let mut buckets: IndexMap<(Option<RankTier>, ActionType, bool), Vec<f64>> = IndexMap::new();
    for row in rows.iter().filter(|row| row.pilot != Pilot::Ai) {
        buckets
            .entry((row.rank_bucket, row.action_kind, row.first_in_turn))
            .or_default()
            .push((row.think_ms.max(1) as f64).ln());
    }
    buckets.sort_keys();
    buckets
        .into_iter()
        .filter(|(_, logs)| logs.len() >= min_samples)
        .map(|((rank_bucket, action_kind, first_in_turn), logs)| {
            let n = logs.len() as f64;
            let mu = logs.iter().sum::<f64>() / n;
            let sigma = (logs.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / n).sqrt();
            BucketFit {
                rank_bucket,
                action_kind,
                first_in_turn,
                samples: logs.len(),
                mu,
                sigma,
            }
        })
        .collect()
}

/// The fits as rows of `THINK_TIME_FITS`, after one comment line.
pub fn render_fits(fits: &[BucketFit]) -> String {
    let mut out = String::from(HEADER);
    out.push('\n');
    for fit in fits {
        let rank = match fit.rank_bucket {
            Some(tier) => format!("Some({:?})", literal(&tier)),
            None => "None".to_string(),
        };
        out.push_str(&format!(
            "    ThinkTimeFit {{ rank_bucket: {rank}, action_kind: {:?}, first_in_turn: {}, samples: {}, mu: {:.4}, sigma: {:.4} }},\n",
            fit.action_kind.as_str(),
            fit.first_in_turn,
            fit.samples,
            fit.mu,
            fit.sigma,
        ));
    }
    out
}

/// Every action timing the store holds.
pub async fn timings(db: &Db) -> Result<Vec<ActionTimingRow>, StoreError> {
    let mut tx = db.begin(None).await?;
    let rows = tx.play_telemetry_timings().await?;
    tx.commit().await?;
    Ok(rows)
}

/// `timing-fit`'s arguments after the subcommand (`main.rs`): none.
pub async fn run(args: Vec<String>) -> anyhow::Result<()> {
    if let Some(arg) = args.first() {
        bail!("Unrecognised argument {}.\n\n{USAGE}", quoted(arg));
    }
    let connection_string = std::env::var("DATABASE_URL").unwrap_or_default();
    if connection_string.is_empty() {
        bail!("DATABASE_URL is not set (see docs/architecture.md, env-var contract).");
    }
    // One connection: this process runs one read and exits.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&connection_string)
        .await
        .map_err(|error| anyhow!("{error}"))?;
    let db = Db::Pg(pool);
    let rows = timings(&db).await;
    db.close().await;
    let fits = fit_think_times(&rows?, TIMING_FIT_MIN_SAMPLES);
    print!("{}", render_fits(&fits));
    if fits.is_empty() {
        eprintln!("timing-fit: no bucket holds {TIMING_FIT_MIN_SAMPLES} human moves yet");
    }
    Ok(())
}
