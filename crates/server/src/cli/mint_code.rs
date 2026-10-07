//! Admin script: mints one invite code and prints the plaintext exactly once (SPEC §9.4).
//! `jackioh-server mint-code` (TS `codes:mint`), the port of `apps/server/src/db/mint-code.ts`.
//!
//! Step 7 of docs/architecture.md's bring-up checklist, which BUILD M6-T1 owes as "an admin
//! script". Every account starts `pending` and a pending account can do nothing but look at the
//! code screen, so until a code exists there is no way into the game.
//!
//! Why this reads the whole environment through `load_env()` when it only needs two variables:
//! the hash stored here has to be byte-identical to the one the running server computes when it
//! redeems, and `load_env` plus the `{CODE_PEPPER}:code` derivation below is the single contract
//! that guarantees it. `migrate` and `seed-catalog` read `DATABASE_URL` directly because a
//! connection string that is wrong fails loudly on the first query; a pepper that is merely
//! *different* mints a well-formed code that nobody can ever redeem, and nothing reports it.

use anyhow::{Result, anyhow};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;

use crate::api::codes::{MintDeps, MintInput, mint_invite_code};
use crate::app::now_ms;
use crate::db::store::Db;
use crate::env::{Env, js_number, load_env, quoted};

const MS_PER_DAY: i64 = 24 * 60 * 60 * 1000;

const USAGE: &str = "Usage: jackioh-server mint-code [options]
       (from a checkout: cargo run --release -p jackioh-server -- mint-code [options])

  --max-uses=N          How many accounts this code activates. Default 1 (R161).
  --expires-in-days=N   Expire the code N days from now. Default: never expires.

Prints the plaintext code once. Only its keyed hash is stored, so a lost code
cannot be recovered — mint another.";

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MintOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_uses: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_in_days: Option<i64>,
}

/// Parses `--flag=value` arguments, rejecting anything it does not recognise.
pub fn parse_mint_args(argv: &[String]) -> Result<MintOptions> {
    let mut options = MintOptions::default();

    for arg in argv {
        let Some((name, raw)) = flag_of(arg) else {
            return Err(anyhow!("Unrecognised argument {}.\n\n{USAGE}", quoted(arg)));
        };

        // The name is checked before the value, so `--label=bring-up` is reported as the unknown
        // option it is rather than as a malformed number.
        if name != "max-uses" && name != "expires-in-days" {
            return Err(anyhow!("Unrecognised option --{name}.\n\n{USAGE}"));
        }

        let value = js_number(raw);
        if !is_integer(value) || value < 1.0 {
            return Err(anyhow!(
                "--{name} must be a positive integer (got {}).\n\n{USAGE}",
                quoted(raw)
            ));
        }

        if name == "max-uses" {
            options.max_uses = Some(value as i64);
        } else {
            options.expires_in_days = Some(value as i64);
        }
    }

    Ok(options)
}

/// TS `main()`: parse, read the environment, mint over one connection, print, close.
pub async fn run(args: Vec<String>) -> Result<()> {
    let options = parse_mint_args(&args)?;
    let source: IndexMap<String, String> = std::env::vars().collect();
    let env = load_env(&source).map_err(|error| anyhow!("{error}"))?;

    // One connection: this process runs a single insert and exits.
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&env.database_url)
        .await?;
    let db = Db::Pg(pool.clone());
    let minted = mint(&db, &env, &options).await;
    pool.close().await;
    minted
}

async fn mint(db: &Db, env: &Env, options: &MintOptions) -> Result<()> {
    let expires_at = match options.expires_in_days {
        None => None,
        Some(days) => Some(
            days.checked_mul(MS_PER_DAY)
                .and_then(|span| span.checked_add(now_ms()))
                .ok_or_else(|| {
                    anyhow!("--expires-in-days={days} is further away than a timestamp can hold")
                })?,
        ),
    };

    // §9.4, §9.8: one pepper in the environment — the same one `src/app.rs` keys with, so this code
    // hashes to what redemption will look up. TS `MintDeps` was the store, ids, hashes and timers; ids
    // and the clock are the server's own functions now (SURFACE §11.3), so minting takes the store and
    // the pepper (`api::codes::MintDeps`).
    let max_uses = options
        .max_uses
        .map(|uses| i32::try_from(uses).unwrap_or(i32::MAX));
    let minted = mint_invite_code(
        MintDeps {
            db,
            code_pepper: &env.code_pepper,
        },
        MintInput { max_uses, expires_at },
    )
    .await
    .map_err(|error| anyhow!("{error}"))?;

    // The plaintext goes to stdout and the metadata to stderr, so `mint-code > code.txt`
    // captures the code alone.
    let expiry = match expires_at {
        None => "never expires".to_string(),
        Some(at) => format!("expires {}", iso_string(at)?),
    };
    eprintln!(
        "codes:mint: id {}, max uses {}, {expiry}\n\
         codes:mint: this is the only time the code is shown; the database holds only its hash.",
        minted.id,
        options.max_uses.unwrap_or(1),
    );
    println!("{}", minted.formatted);
    Ok(())
}

/// TS's `/^--(?<name>[a-z-]+)=(?<value>.*)$/u`: `--`, a name of lower-case letters and hyphens up
/// to the first `=`, then a value with no line terminator (`.` does not match one).
fn flag_of(arg: &str) -> Option<(&str, &str)> {
    let rest = arg.strip_prefix("--")?;
    let eq = rest.find('=')?;
    let (name, value) = (&rest[..eq], &rest[eq + 1..]);
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_lowercase() || c == '-') {
        return None;
    }
    if value
        .chars()
        .any(|c| matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
    {
        return None;
    }
    Some((name, value))
}

/// JS `Number.isInteger`.
pub(crate) fn is_integer(value: f64) -> bool {
    value.is_finite() && value.fract() == 0.0
}

/// `new Date(ms).toISOString()`: `2026-10-07T12:34:56.789Z`.
fn iso_string(ms: i64) -> Result<String> {
    let at = time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)?;
    let format = time::format_description::parse_borrowed::<1>(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z",
    )?;
    Ok(at.format(&format)?)
}
