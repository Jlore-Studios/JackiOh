//! Catalog loader. Copies the card catalog into `public.cards`, stamping every row with the catalog
//! version. `jackioh-server seed-catalog` (TS `db:seed-catalog`), and the second step of
//! `jackioh-server release`; the port of `apps/server/src/db/seed-catalog.ts`.
//!
//! SPEC §9.4: "catalog is static, versioned, shipped with the client". The client renders card text
//! from its own bundled copy; this table exists so `collection.card_id` and `loadout_deck_cards.card_id`
//! have something to reference and so the server can run L6 ("every card exists in the current catalog
//! version and is not banned") in SQL. Nothing here is a source of truth for card rules.
//!
//! With no argument it seeds the catalog compiled into this binary (`jackioh_cards::catalog_json()`,
//! crates/cards/catalog.json) at the version compiled in with it (`jackioh_cards::catalog_version()`,
//! the newest entry of crates/cards/patches/patches.json). `CATALOG_VERSION` in the environment, when
//! set, must name that same version: the server refuses to boot on a mismatch (SURFACE §11.3), so the
//! seed refuses too, before it writes anything. With a path, it seeds that file instead (a fixture
//! catalog), at `CATALOG_VERSION`, which is then required, as in TS.

use anyhow::{Result, anyhow};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Connection;
use sqlx::postgres::PgConnection;

use crate::env::quoted;

/// Where the compiled-in catalog came from, for the messages that name it.
const COMPILED_CATALOG: &str = "crates/cards/catalog.json";

/// The subset of BUILD M4-T1's per-card schema that `public.cards` stores.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogEntry {
    pub id: String,
    pub index: String,
    pub name: String,
    pub set: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub tags: Vec<String>,
    pub rarity: String,
    pub token: bool,
    pub cost: Value,
}

fn is_entry(value: &Value) -> bool {
    let Some(row) = value.as_object() else { return false };
    let text = |key: &str| row.get(key).is_some_and(Value::is_string);
    text("id")
        && text("index")
        && text("name")
        && text("set")
        && text("type")
        && row
            .get("tags")
            .and_then(Value::as_array)
            .is_some_and(|tags| tags.iter().all(Value::is_string))
        && text("rarity")
        && row.get("token").is_some_and(Value::is_boolean)
        && row.contains_key("cost")
}

fn entry_of(value: &Value) -> CatalogEntry {
    let text = |key: &str| value[key].as_str().unwrap_or_default().to_string();
    CatalogEntry {
        id: text("id"),
        index: text("index"),
        name: text("name"),
        set: text("set"),
        type_: text("type"),
        tags: value["tags"]
            .as_array()
            .map(|tags| tags.iter().filter_map(Value::as_str).map(str::to_string).collect())
            .unwrap_or_default(),
        rarity: text("rarity"),
        token: value["token"].as_bool().unwrap_or(false),
        cost: value["cost"].clone(),
    }
}

/// The three shapes a catalog file may arrive in, reduced to a list of entries. `None` means
/// none of the three.
///
/// The third is the one that matters: the catalog is a **record keyed by card id**, which is what
/// `CardDefs` is and what `src/api/catalog.rs` parses it as. Reading only an array meant the seeder
/// and the API disagreed about the shape of the one file they share, and the seeder lost —
/// `db:seed-catalog` could not load the real catalog at all.
///
/// `text` is the file's text again: a record is re-read into an `IndexMap` so its entries keep the
/// file's order, as `Object.entries` keeps it (serde_json's own map sorts its keys).
fn catalog_rows(parsed: &Value, text: &str, path: &str) -> Result<Option<Vec<Value>>> {
    if let Some(rows) = parsed.as_array() {
        return Ok(Some(rows.clone()));
    }
    let Some(object) = parsed.as_object() else { return Ok(None) };

    if let Some(wrapped) = object.get("cards").and_then(Value::as_array) {
        return Ok(Some(wrapped.clone()));
    }

    let entries: IndexMap<String, Value> = serde_json::from_str(text)?;
    if entries.is_empty() {
        return Ok(None);
    }

    // The key and the entry's own `id` must agree. `seed_catalog` inserts `card.id`, so a record
    // that disagrees with itself would seed a row under an id nothing else in the catalog uses,
    // and `collection.card_id`'s foreign key would point at the wrong card rather than fail.
    for (key, value) in &entries {
        if let Some(id) = value.get("id").and_then(Value::as_str)
            && id != key
        {
            return Err(anyhow!(
                "{path}: entry keyed {} carries id {}",
                quoted(key),
                quoted(id)
            ));
        }
    }
    Ok(Some(entries.into_values().collect()))
}

pub async fn read_catalog(path: &str) -> Result<Vec<CatalogEntry>> {
    let text = tokio::fs::read_to_string(path).await.map_err(|_| {
        anyhow!(
            "cannot read {path}. With no argument the catalog compiled into this binary \
             ({COMPILED_CATALOG}) is seeded; pass an explicit path as the first argument only to seed \
             a fixture catalog instead."
        )
    })?;
    parse_catalog(&text, path)
}

/// `read_catalog` after the read: the text of a catalog, named `path` in every refusal. The
/// compiled-in catalog goes through here too, so the seeder holds it to the same schema as a file.
pub fn parse_catalog(text: &str, path: &str) -> Result<Vec<CatalogEntry>> {
    let parsed: Value = serde_json::from_str(text)?;
    let Some(rows) = catalog_rows(&parsed, text, path)? else {
        return Err(anyhow!(
            "{path}: expected an array of cards, a {{ \"cards\": [...] }} wrapper, or an object keyed \
             by card id"
        ));
    };

    let mut entries = Vec::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        if !is_entry(row) {
            return Err(anyhow!("{path}: entry {i} does not match the M4-T1 card schema"));
        }
        entries.push(entry_of(row));
    }
    Ok(entries)
}

pub async fn seed_catalog(
    connection_string: &str,
    catalog_version: &str,
    entries: &[CatalogEntry],
) -> Result<usize> {
    let mut client = PgConnection::connect(connection_string).await?;
    let written = write_catalog(&mut client, catalog_version, entries).await;
    let closed = client.close().await;
    let written = written?;
    closed?;
    Ok(written)
}

async fn write_catalog(client: &mut PgConnection, catalog_version: &str, entries: &[CatalogEntry]) -> Result<usize> {
    // One transaction: either the whole catalog version is present or none of it is, so a queue or
    // save request can never see half a catalog (§9.4, "stale catalog version is rejected"). An
    // error drops `tx` uncommitted, which rolls it back.
    let mut tx = client.begin().await?;
    for card in entries {
        sqlx::query(
            "insert into public.cards
           (id, card_index, name, set_id, type, tags, rarity, token, cost, catalog_version)
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9::jsonb, $10)
         on conflict (id) do update set
           card_index = excluded.card_index,
           name = excluded.name,
           set_id = excluded.set_id,
           type = excluded.type,
           tags = excluded.tags,
           rarity = excluded.rarity,
           token = excluded.token,
           cost = excluded.cost,
           catalog_version = excluded.catalog_version",
        )
        .bind(&card.id)
        .bind(&card.index)
        .bind(&card.name)
        .bind(&card.set)
        .bind(&card.type_)
        .bind(&card.tags)
        .bind(&card.rarity)
        .bind(card.token)
        .bind(serde_json::to_string(&card.cost)?)
        .bind(catalog_version)
        .execute(&mut *tx)
        .await?;
    }
    // The version the server and the client compare against (`app.assert_catalog_version`).
    sqlx::query(
        "insert into app.settings (key, value) values ('catalog_version', $1::jsonb)
       on conflict (key) do update set value = excluded.value, updated_at = now()",
    )
    .bind(serde_json::to_string(catalog_version)?)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(entries.len())
}

/// TS `main()`: the connection string, the version, the catalog, one transaction, one line.
pub async fn run(args: Vec<String>) -> Result<()> {
    let connection_string = std::env::var("DATABASE_URL").unwrap_or_default();
    if connection_string.is_empty() {
        return Err(anyhow!("DATABASE_URL is not set (see docs/architecture.md, env-var contract)."));
    }
    let configured = std::env::var("CATALOG_VERSION").unwrap_or_default();

    let (catalog_version, entries) = match args.first() {
        None => {
            let compiled = jackioh_cards::catalog_version();
            if !configured.is_empty() && configured != compiled {
                return Err(anyhow!(
                    "CATALOG_VERSION is {configured}, but this build's catalog is {compiled} \
                     (crates/cards/patches/patches.json). The server refuses to boot on that mismatch, \
                     so nothing is seeded: set CATALOG_VERSION={compiled}, or leave it unset."
                ));
            }
            (compiled.to_string(), parse_catalog(jackioh_cards::catalog_json(), COMPILED_CATALOG)?)
        }
        Some(path) => {
            if configured.is_empty() {
                return Err(anyhow!(
                    "CATALOG_VERSION is not set (see docs/architecture.md, env-var contract)."
                ));
            }
            (configured, read_catalog(path).await?)
        }
    };

    let count = seed_catalog(&connection_string, &catalog_version, &entries).await?;
    println!("seed-catalog: wrote {count} cards at catalog version {catalog_version}");
    Ok(())
}

