//! Admin script: folds the game records written before the card statistics' totals into them
//! (R1044000, migration 0041).
//!
//! ```text
//! jackioh-server stats-fold
//! ```
//!
//! Every record the server writes is folded as it is written; a record written before migration 0041
//! is not, and until it is, the totals miss it and the retention purge keeps its detail. The server's
//! retention loop folds them at boot (`api::retention::run_purge`); this runs the same fold by hand,
//! `STATS_FOLD_BATCH_RECORDS` records to a transaction, and says how many it folded. Run again, it
//! folds nothing: each record is marked folded with its counts, in one transaction.

use anyhow::bail;

use crate::api::game_records::fold_pending;
use crate::cli::{connect_one, database_url};

const USAGE: &str = "Usage: jackioh-server stats-fold";

/// `stats-fold`'s arguments after the subcommand (`main.rs`): none.
pub async fn run(args: Vec<String>) -> anyhow::Result<()> {
    if !args.is_empty() {
        bail!(USAGE);
    }
    let db = connect_one(&database_url()?).await?;
    let folded = fold_pending(&db).await;
    db.close().await;
    println!("stats:fold: folded {} records into the totals", folded?);
    Ok(())
}
