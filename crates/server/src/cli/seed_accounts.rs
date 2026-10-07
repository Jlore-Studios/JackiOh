//! Admin script: creates ready-to-play test accounts. `jackioh-server seed-accounts [count]` (TS
//! `db:seed-accounts`), the port of `apps/server/src/db/seed-accounts.ts`.
//!
//! Not part of BUILD. It exists because the ordinary path to a playable account has three manual
//! steps -- sign up, click a confirmation link in a real inbox, redeem an invite code -- and none
//! of them is what you want when you are testing the game itself.
//!
//! It skips the inbox on purpose. `POST /auth/v1/admin/users` with `email_confirm: true` creates an
//! account whose email is already verified, which satisfies §9.4 step 1 ("reject unless the account
//! is pending with a verified email") without an email ever being sent. That also sidesteps
//! Supabase's Site URL entirely -- a project whose Site URL still points at localhost mails a
//! confirmation link nobody on a deployed site can use.
//!
//! It then flips `profiles.status` to 'active'. That is a real activation, not a shortcut around
//! one: migration 0002's `profiles_grant_launch_collection` trigger fires on exactly that
//! transition and calls `app.grant_launch_collection`, so the account ends up with the same
//! entitlement ledger a redeemed invite code would have produced. What it skips is the invite gate
//! (§9.4's six-step redemption), which is the point -- that path has its own tests.
//!
//! These accounts skip the invite gate, so the script refuses to run unless you opt in for one
//! named project, and the password comes from your environment, never from this file:
//!
//!   SEED_ACCOUNTS_PROJECT   must equal SUPABASE_URL's host (e.g. abcd.supabase.co). Set it only in
//!                           the .env of a dev or e2e project, never for the production one.
//!   SEED_ACCOUNTS_PASSWORD  the password every seeded account gets, at least
//!                           AUTH_PASSWORD_MIN_LENGTH characters. It is never printed.
//!
//! It also refuses under NODE_ENV=production.

use std::time::Duration;

use anyhow::{Result, anyhow};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::postgres::{PgConnection, PgRow};
use sqlx::{Connection, Row};

use jackioh_engine::validator::TRIO_DECKS;

use crate::cli::mint_code::is_integer;
use crate::config::{AUTH_PASSWORD_MAX_LENGTH, AUTH_PASSWORD_MIN_LENGTH, MAX_SAVED_DECKS, MAX_SAVED_TRIOS};
use crate::env::{load_env, js_number, quoted};

const DEFAULT_COUNT: i64 = 2;
const EMAIL_DOMAIN: &str = "example.com";
/// The most accounts one run makes.
const MAX_COUNT: i64 = 20;
/// How many times, and how far apart, the profile row `app.handle_new_user` writes is looked for.
const PROFILE_POLL_ATTEMPTS: usize = 20;
const PROFILE_POLL_INTERVAL: Duration = Duration::from_millis(250);
/// The admin listing's first page, which the search for an existing account reads.
const ADMIN_USERS_PER_PAGE: usize = 200;

/// The opt-in: the host of the one Supabase project this run may seed.
pub const SEED_PROJECT_VAR: &str = "SEED_ACCOUNTS_PROJECT";
/// The password every seeded account gets. Read from the environment only.
pub const SEED_PASSWORD_VAR: &str = "SEED_ACCOUNTS_PASSWORD";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SeededAccount {
    pub email: String,
    pub user_id: String,
    pub created: bool,
}

/// What the gate hands back: the password, once every check has passed.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SeedAccountsSettings {
    pub password: String,
}

/// Checks the opt-in and reads the password, or fails with one error listing every problem. Pure,
/// so a test can drive it without a project. TS's `env: Pick<ServerEnv, "SUPABASE_URL" |
/// "NODE_ENV">` is its two values.
pub fn seed_accounts_settings(
    source: &IndexMap<String, String>,
    supabase_url: &str,
    node_env: &str,
) -> Result<SeedAccountsSettings> {
    let mut problems: Vec<String> = Vec::new();

    if node_env == "production" {
        problems.push("NODE_ENV is production: these accounts skip the invite gate.".to_string());
    }

    let url = reqwest::Url::parse(supabase_url)
        .map_err(|error| anyhow!("SUPABASE_URL is not a URL ({error}): {}", quoted(supabase_url)))?;
    let host = url.host_str().unwrap_or_default().to_string();
    let opt_in = source.get(SEED_PROJECT_VAR).map(|value| value.trim()).unwrap_or_default();
    if opt_in != host {
        problems.push(format!(
            "{SEED_PROJECT_VAR} must equal SUPABASE_URL's host ({host}) to seed this project{} \
             Set it only for a dev or e2e project, never for the production one.",
            if opt_in.is_empty() {
                " (it is not set).".to_string()
            } else {
                format!(" (it is {}).", quoted(opt_in))
            },
        ));
    }

    let secret = source.get(SEED_PASSWORD_VAR).cloned().unwrap_or_default();
    // `.length` in TS counts UTF-16 code units; the upper bound is in bytes, as TextEncoder counts.
    let units = secret.encode_utf16().count();
    if units < AUTH_PASSWORD_MIN_LENGTH as usize {
        problems.push(format!(
            "{SEED_PASSWORD_VAR} must be at least {AUTH_PASSWORD_MIN_LENGTH} characters{} \
             Generate one with: openssl rand -base64 18",
            if units == 0 { " (it is not set)." } else { "." },
        ));
    } else if secret.len() > AUTH_PASSWORD_MAX_LENGTH as usize {
        problems.push(format!("{SEED_PASSWORD_VAR} must be at most {AUTH_PASSWORD_MAX_LENGTH} bytes."));
    }

    if !problems.is_empty() {
        let listed: Vec<String> = problems.iter().map(|problem| format!("  - {problem}")).collect();
        return Err(anyhow!("refusing to seed accounts:\n{}", listed.join("\n")));
    }
    Ok(SeedAccountsSettings { password: secret })
}

fn email_for(index: i64) -> String {
    format!("player{index}@{EMAIL_DOMAIN}")
}

/// The part of `ServerEnv` the Supabase admin calls need.
struct AdminEnv<'a> {
    supabase_url: &'a str,
    supabase_secret_key: &'a str,
}

/// Creates one already-confirmed account, or returns the existing one. Supabase answers a duplicate
/// with 422 `email_exists`, which is not a failure here: the script is meant to be re-runnable.
async fn create_or_find_user(
    http: &reqwest::Client,
    env: &AdminEnv<'_>,
    email: &str,
    secret: &str,
) -> Result<(String, bool)> {
    let admin = |request: reqwest::RequestBuilder| {
        request
            .header("Content-Type", "application/json")
            .header("apikey", env.supabase_secret_key)
            .header("Authorization", format!("Bearer {}", env.supabase_secret_key))
    };

    let created = admin(http.post(format!("{}/auth/v1/admin/users", env.supabase_url)))
        .body(json!({ "email": email, "password": secret, "email_confirm": true }).to_string())
        .send()
        .await?;
    let status = created.status();
    let body: Value = created.json().await.unwrap_or_else(|_| json!({}));

    if status.is_success()
        && let Some(id) = body.get("id").and_then(Value::as_str)
    {
        return Ok((id.to_string(), true));
    }

    // Already there: find it by email so a re-run is a no-op rather than an error.
    let listed = admin(http.get(format!(
        "{}/auth/v1/admin/users?page=1&per_page={ADMIN_USERS_PER_PAGE}",
        env.supabase_url
    )))
    .send()
    .await?;
    let users: Value = listed.json().await.unwrap_or_else(|_| json!({}));
    let found = users
        .get("users")
        .and_then(Value::as_array)
        .and_then(|users| users.iter().find(|user| user.get("email").and_then(Value::as_str) == Some(email)));
    if let Some(id) = found.and_then(|user| user.get("id")).and_then(Value::as_str) {
        return Ok((id.to_string(), false));
    }

    Err(anyhow!("could not create or find {email}: {} {body}", status.as_u16()))
}

/// A profile id as the `uuid` it is in Postgres. `pg` sent every parameter untyped and let the
/// column decide; sqlx sends a `String` as `text`, which `uuid = text` refuses, so the SQL is kept
/// as TS wrote it and the id is bound as a `uuid` instead.
fn uuid_of(id: &str) -> Result<uuid::Uuid> {
    uuid::Uuid::parse_str(id).map_err(|error| anyhow!("{} is not a uuid: {error}", quoted(id)))
}

/// A row's one column as text, or `None` for SQL NULL.
fn text_at(row: &PgRow, column: &str) -> Result<Option<String>> {
    Ok(row.try_get::<Option<String>, _>(column)?)
}

/// Three legal decks and a trio of them, so a seeded account can queue immediately in any mode —
/// Best of 1 with any of the decks, Conquest with the trio (SPEC §9.5, R257) — instead of building
/// 60 cards by hand before it can play once.
///
/// Saved through `app.upsert_deck` and `app.upsert_trio` (migration 0007) rather than by writing
/// `public.decks` and `public.trios` directly, because those functions are the one write path the
/// server uses too: they take the profile lock, apply the caps and refuse a shape the builder could
/// not have produced. A saved deck is only a draft (R250), so the legality that matters here is the
/// queue's (R253): `DECK_SIZE` cards each, and — for the trio — no card in two decks.
///
/// `MAX_COPIES` is 1, so the format is singleton and the three decks need 60 DISTINCT non-token
/// cards. The launch grant gives every active profile all 100 of them, so ordering by id and slicing
/// is enough; no deck here is trying to be good, only legal.
///
/// Re-runnable: a profile that already holds a deck or a trio is left alone, so a second run neither
/// piles up starters nor touches decks a tester has built since.
async fn save_starter_decks(client: &mut PgConnection, profile_id: &str, catalog_version: &str) -> Result<()> {
    let held = sqlx::query(
        "select ((select count(*) from public.decks where profile_id = $1)
           + (select count(*) from public.trios where profile_id = $1))::text as n",
    )
    .bind(uuid_of(profile_id)?)
    .fetch_all(&mut *client)
    .await?;
    let n = match held.first() {
        Some(row) => text_at(row, "n")?.unwrap_or_else(|| "0".to_string()),
        None => "0".to_string(),
    };
    if n.parse::<i64>().unwrap_or(0) > 0 {
        return Ok(());
    }

    let rows = sqlx::query("select id from public.cards where not token and catalog_version = $1 order by id")
        .bind(catalog_version)
        .fetch_all(&mut *client)
        .await?;
    let mut ids: Vec<String> = Vec::with_capacity(rows.len());
    for row in &rows {
        ids.push(row.try_get::<String, _>("id")?);
    }

    // The database's own copy of `DECK_SIZE` (migration 0003), which `app.upsert_deck` checks a deck
    // against: read rather than taken from the engine, so the seed fills exactly what the database
    // will accept.
    let sizes = sqlx::query("select (app.setting('deck_size'))::text::int as deck_size")
        .fetch_all(&mut *client)
        .await?;
    let deck_size = match sizes.first() {
        Some(row) => row.try_get::<Option<i32>, _>("deck_size")?,
        None => None,
    };
    let Some(deck_size) = deck_size else {
        return Err(anyhow!("app.settings has no deck_size: is migration 0003 applied?"));
    };
    let deck_size = deck_size as usize;
    let trio_decks = TRIO_DECKS as usize;

    let needed = deck_size * trio_decks;
    if ids.len() < needed {
        return Err(anyhow!(
            "need {needed} distinct non-token cards for {trio_decks} decks of {deck_size}, but the catalog has {}",
            ids.len()
        ));
    }

    let mut deck_ids: Vec<String> = Vec::with_capacity(trio_decks);
    for i in 0..trio_decks {
        let deck_id = uuid::Uuid::new_v4().to_string();
        let cards = &ids[i * deck_size..(i + 1) * deck_size];
        let saved = sqlx::query(
            "select app.upsert_deck($1::uuid, $2::uuid, $3::text, $4::jsonb, $5::text, now(), $6::int) as outcome",
        )
        .bind(profile_id)
        .bind(&deck_id)
        .bind(format!("Starter {}", i + 1))
        .bind(serde_json::to_string(cards)?)
        .bind(catalog_version)
        .bind(MAX_SAVED_DECKS as i32)
        .fetch_all(&mut *client)
        .await?;
        let outcome = match saved.first() {
            Some(row) => text_at(row, "outcome")?,
            None => None,
        };
        if outcome.as_deref() != Some("created") {
            return Err(anyhow!(
                "app.upsert_deck answered {} for starter deck {}",
                outcome.as_deref().unwrap_or("undefined"),
                i + 1
            ));
        }
        deck_ids.push(deck_id);
    }

    let trio = sqlx::query(
        "select app.upsert_trio($1::uuid, $2::uuid, $3::text, $4::uuid, $5::uuid, $6::uuid, now(), $7::int) as outcome",
    )
    .bind(profile_id)
    .bind(uuid::Uuid::new_v4().to_string())
    .bind("Starter trio")
    .bind(deck_ids.first())
    .bind(deck_ids.get(1))
    .bind(deck_ids.get(2))
    .bind(MAX_SAVED_TRIOS as i32)
    .fetch_all(&mut *client)
    .await?;
    let outcome = match trio.first() {
        Some(row) => text_at(row, "outcome")?,
        None => None,
    };
    if outcome.as_deref() != Some("created") {
        return Err(anyhow!(
            "app.upsert_trio answered {} for the starter trio",
            outcome.as_deref().unwrap_or("undefined")
        ));
    }
    Ok(())
}

pub async fn seed_accounts(count: i64) -> Result<Vec<SeededAccount>> {
    let source: IndexMap<String, String> = std::env::vars().collect();
    let env = load_env(&source).map_err(|error| anyhow!("{error}"))?;
    let settings = seed_accounts_settings(&source, env.supabase_url.as_str(), env.node_env.as_str())?;

    let mut client = PgConnection::connect(&env.database_url).await?;
    let seeded = seed_with(&mut client, &env, &settings.password, count).await;
    let closed = client.close().await;
    let seeded = seeded?;
    closed?;
    Ok(seeded)
}

async fn seed_with(
    client: &mut PgConnection,
    env: &crate::env::Env,
    secret: &str,
    count: i64,
) -> Result<Vec<SeededAccount>> {
    let http = reqwest::Client::new();
    let admin = AdminEnv {
        supabase_url: env.supabase_url.as_str(),
        supabase_secret_key: env.supabase_secret_key.as_str(),
    };
    let mut out: Vec<SeededAccount> = Vec::new();

    for i in 1..=count {
        let email = email_for(i);
        let (id, created) = create_or_find_user(&http, &admin, &email, secret).await?;

        // `app.handle_new_user` inserts the profile from an auth.users trigger; that runs in
        // Supabase's transaction, not ours, so the row can lag a beat behind the API response.
        let mut profile_exists = false;
        let mut attempt = 0;
        while attempt < PROFILE_POLL_ATTEMPTS && !profile_exists {
            let found = sqlx::query("select 1 from public.profiles where id = $1")
                .bind(uuid_of(&id)?)
                .fetch_all(&mut *client)
                .await?;
            profile_exists = !found.is_empty();
            if !profile_exists {
                tokio::time::sleep(PROFILE_POLL_INTERVAL).await;
            }
            attempt += 1;
        }
        if !profile_exists {
            return Err(anyhow!("no profiles row appeared for {email} ({id}) — is migration 0001 applied?"));
        }

        // The real activation: this UPDATE is what `profiles_grant_launch_collection` watches.
        sqlx::query(
            "update public.profiles set status = 'active', activated_at = coalesce(activated_at, now()) \
             where id = $1 and status <> 'active'",
        )
        .bind(uuid_of(&id)?)
        .execute(&mut *client)
        .await?;

        save_starter_decks(client, &id, env.catalog_version.as_str()).await?;
        out.push(SeededAccount { email, user_id: id, created });
    }

    Ok(out)
}

/// TS `main()`: the one optional argument is how many accounts, 1 to 20.
pub async fn run(args: Vec<String>) -> Result<()> {
    let arg = args.first();
    let count = match arg {
        None => DEFAULT_COUNT as f64,
        Some(raw) => js_number(raw),
    };
    if !is_integer(count) || count < 1.0 || count > MAX_COUNT as f64 {
        return Err(anyhow!(
            "account count must be an integer between 1 and {MAX_COUNT} (got {})",
            arg.map(String::as_str).unwrap_or("undefined")
        ));
    }

    let accounts = seed_accounts(count as i64).await?;
    eprintln!("seed-accounts: {} account(s), all active", accounts.len());
    for account in &accounts {
        println!("{}  {}", account.email, if account.created { "created" } else { "already existed" });
    }
    Ok(())
}

