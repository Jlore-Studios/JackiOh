//! Admin script: the invite-cluster watch (SPEC §9.8, "watch for clusters of accounts sharing an IP
//! hash").
//!
//! ```text
//! jackioh-server invite-clusters [--window=<seconds>] [--min-accounts=<N>]
//! ```
//!
//! Reads `public.code_attempts` through the store (DATABASE_URL) and prints, as Markdown, every IP
//! hash that distinct accounts made at least `--min-accounts` redemption attempts from in the last
//! `--window` seconds, with the accounts and the attempts behind it. The ranked ladder pays for
//! rating, so several invite-gated accounts behind one address is the cheapest way to farm it; this
//! is where a person looks for that.
//!
//! It is advisory and read-only. It bans nothing and disables nothing, because a campus, a
//! household or a carrier's NAT puts many honest players behind one hash, and it writes nothing.
//! What it prints is counts and the first `INVITE_CLUSTER_HASH_PREFIX_LEN` characters of each hash:
//! no full hash, no profile id, no email, no address (the table holds none of the last: §9.4 hashes
//! the address with the server's pepper before it is stored). The defaults are in `config.rs`
//! (CLAUDE.md rule 9).

use anyhow::bail;

use crate::app::now_ms;
use crate::cli::{connect_one, database_url};
use crate::config::{
    CODE_ATTEMPT_RETENTION_DAYS, INVITE_CLUSTER_HASH_PREFIX_LEN, INVITE_CLUSTER_MIN_ACCOUNTS,
    INVITE_CLUSTER_WINDOW_SECONDS,
};
use crate::db::store::{Db, StoreError};
use crate::env::quoted;

/// The usage text, with the defaults and the longest window read off `config.rs`.
fn usage() -> String {
    format!(
        "Usage: jackioh-server invite-clusters [options]

  --window=<seconds>    How far back to look. Default: {INVITE_CLUSTER_WINDOW_SECONDS}; at most {MAX_WINDOW_SECONDS}, the
                        {CODE_ATTEMPT_RETENTION_DAYS} days code_attempts rows are kept.
  --min-accounts=<N>    List an IP hash behind at least N distinct accounts. Default: {INVITE_CLUSTER_MIN_ACCOUNTS}.

Advisory and read-only: it prints counts and shortened hashes for a person to review, and
changes nothing."
    )
}

const SECONDS_PER_DAY: i64 = 24 * 60 * 60;

/// The longest `--window` there is anything to read for: `code_attempts` rows older than
/// `CODE_ATTEMPT_RETENTION_DAYS` are purged. It also keeps the window's arithmetic in range.
const MAX_WINDOW_SECONDS: i64 = CODE_ATTEMPT_RETENTION_DAYS * SECONDS_PER_DAY;

/// The most accounts `--min-accounts` takes: a count the store's `int` could hold. Past it nothing
/// could be listed.
const MAX_MIN_ACCOUNTS: i64 = i32::MAX as i64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InviteClustersOptions {
    pub window_seconds: i64,
    pub min_accounts: i64,
}

/// One listed IP hash: the first `INVITE_CLUSTER_HASH_PREFIX_LEN` characters of it and its counts.
/// The report never holds more of a hash than this.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClusterRow {
    pub ip_hash_prefix: String,
    pub accounts: i64,
    pub attempts: i64,
}

/// `--name=value` with a lowercase-and-hyphen name and a value that is not empty, as the other
/// scripts read theirs (`card_stats::flag` takes no hyphen in a name, and this has one).
fn named(arg: &str) -> Option<(&str, &str)> {
    let (name, value) = arg.strip_prefix("--")?.split_once('=')?;
    let name_ok = !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_lowercase() || byte == b'-');
    (name_ok && !value.is_empty()).then_some((name, value))
}

/// The whole number in `min..=max` that `--name=value` holds, or a refusal ending with the usage.
fn bounded(name: &str, raw: &str, min: i64, max: i64) -> anyhow::Result<i64> {
    match raw.parse::<i64>() {
        Ok(value) if (min..=max).contains(&value) => Ok(value),
        _ => bail!(
            "--{name} must be a whole number from {min} to {max} (got {}).\n\n{}",
            quoted(raw),
            usage()
        ),
    }
}

/// Parses `--flag=value` arguments, refusing anything it does not know.
pub fn parse_invite_clusters_args(argv: &[String]) -> anyhow::Result<InviteClustersOptions> {
    let mut options = InviteClustersOptions {
        window_seconds: INVITE_CLUSTER_WINDOW_SECONDS,
        min_accounts: INVITE_CLUSTER_MIN_ACCOUNTS,
    };
    for arg in argv {
        let Some((name, raw)) = named(arg) else {
            bail!("Unrecognised argument {}.\n\n{}", quoted(arg), usage());
        };
        match name {
            "window" => options.window_seconds = bounded(name, raw, 1, MAX_WINDOW_SECONDS)?,
            "min-accounts" => options.min_accounts = bounded(name, raw, 1, MAX_MIN_ACCOUNTS)?,
            _ => bail!("Unrecognised option --{name}.\n\n{}", usage()),
        }
    }
    Ok(options)
}

/// The IP hashes the options list, biggest cluster first (most accounts, then most attempts, then
/// by hash), each cut to its prefix. One read, in one transaction, committed.
pub async fn invite_clusters_report(
    db: &Db,
    options: &InviteClustersOptions,
) -> Result<Vec<ClusterRow>, StoreError> {
    let since = now_ms() - options.window_seconds * 1000;
    let mut tx = db.begin(None).await?;
    let mut clusters = tx.codes_invite_clusters(since).await?;
    tx.commit().await?;
    clusters.retain(|cluster| cluster.accounts >= options.min_accounts);
    clusters.sort_by(|a, b| {
        b.accounts
            .cmp(&a.accounts)
            .then(b.attempts.cmp(&a.attempts))
            .then_with(|| a.ip_hash.cmp(&b.ip_hash))
    });
    Ok(clusters
        .into_iter()
        .map(|cluster| ClusterRow {
            ip_hash_prefix: cluster
                .ip_hash
                .chars()
                .take(INVITE_CLUSTER_HASH_PREFIX_LEN)
                .collect(),
            accounts: cluster.accounts,
            attempts: cluster.attempts,
        })
        .collect())
}

/// The report as the script prints it: a Markdown table, or one line saying nothing was found.
pub fn render_invite_clusters(rows: &[ClusterRow], options: &InviteClustersOptions) -> String {
    let mut out = String::from("# Invite clusters\n\n");
    if rows.is_empty() {
        out.push_str(&format!(
            "No invite clusters at or above {} accounts in the last {} seconds.\n",
            options.min_accounts, options.window_seconds
        ));
        return out;
    }
    out.push_str(&format!(
        "IP hashes behind at least {} distinct accounts in the last {} seconds.\n\n",
        options.min_accounts, options.window_seconds
    ));
    out.push_str("| IP hash (prefix) | Accounts | Attempts |\n| --- | ---: | ---: |\n");
    for row in rows {
        out.push_str(&format!(
            "| `{}` | {} | {} |\n",
            row.ip_hash_prefix, row.accounts, row.attempts
        ));
    }
    out.push_str(
        "\nAdvisory only: nothing was changed. Many honest players can share one address (a campus, a \
         household, a carrier's NAT), so a person reviews each row before acting. Hashes are \
         shortened to their first characters.\n",
    );
    out
}

/// `invite-clusters`'s arguments after the subcommand (`main.rs`).
pub async fn run(args: Vec<String>) -> anyhow::Result<()> {
    let options = parse_invite_clusters_args(&args)?;
    let connection_string = database_url()?;
    // One connection: this process runs one read and exits.
    let db = connect_one(&connection_string).await?;
    let outcome = invite_clusters_report(&db, &options).await;
    db.close().await;
    let rows = outcome?;
    print!("{}", render_invite_clusters(&rows, &options));
    Ok(())
}
