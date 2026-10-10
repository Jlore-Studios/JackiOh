//! `cli/seed_catalog.rs` (`seed-catalog`), in two halves.
//!
//! The first (every `cargo test`): `read_catalog` against the file it actually has to read. The
//! catalog (`crates/cards/catalog.json`) is a record keyed by card id — the `CardDefs` shape the API
//! parses.
//!
//! The second (`test:db` only: it runs when `DATABASE_URL` is set): `seed_catalog` writes the real
//! catalog into `public.cards`, where every migration's constraints apply. `cards_tags_check` (0002)
//! admits 'Jlockeed' (R278, #13 and #14) from 0010, Book, Pancake and AI (B2.4) from 0015, Plague
//! from 0020, Catalyst, Prime and Acclaimed from 0026 and the Meditative set's Wincon (R1411) from
//! 0029. The seed runs in one transaction, so one such row fails the whole catalog. The first half
//! compares the tags with the migrations' text; the second checks that the database really accepts
//! them. Both count the catalog of the sets that ship (`SHIPPED_SETS`), so they hold before and after
//! the patch that ships the Meditative set (R1420).

use std::path::{Path, PathBuf};

use indexmap::{IndexMap, IndexSet};
use serde_json::{Value, json};

use jackioh_engine::{SHIPPED_SETS, SetName, Tag, set_ships};
use jackioh_server::cli::seed_catalog::read_catalog;

const REAL_CATALOG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../cards/catalog.json");
const MIGRATIONS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/migrations");

/// The ASCII whitespace JS's `\s` matches in the migrations' text, skipped from `at`; how many.
fn skip_space(bytes: &[u8], at: &mut usize) -> usize {
    let start = *at;
    while *at < bytes.len() && bytes[*at].is_ascii_whitespace() {
        *at += 1;
    }
    *at - start
}

/// One match of `/constraint\s+cards_tags_check\s+check\s*\(([\s\S]*?)\]::text\[\]/i` starting
/// at `start` in the lower-cased text: the captured body's range and the match's end.
fn tag_check_at(lower: &str, start: usize) -> Option<(usize, usize, usize)> {
    let bytes = lower.as_bytes();
    let mut at = start + "constraint".len();
    if skip_space(bytes, &mut at) == 0 || !lower[at..].starts_with("cards_tags_check") {
        return None;
    }
    at += "cards_tags_check".len();
    if skip_space(bytes, &mut at) == 0 || !lower[at..].starts_with("check") {
        return None;
    }
    at += "check".len();
    skip_space(bytes, &mut at);
    if !lower[at..].starts_with('(') {
        return None;
    }
    at += 1;
    let close = lower[at..].find("]::text[]")?;
    Some((at, at + close, at + close + "]::text[]".len()))
}

/// Every `cards_tags_check` body one migration holds, in order.
fn tag_check_bodies(sql: &str) -> Vec<String> {
    // ASCII lower-casing keeps every byte offset, so a range found in `lower` cuts `sql` too.
    let lower = sql.to_ascii_lowercase();
    let mut bodies = Vec::new();
    let mut from = 0;
    while let Some(offset) = lower[from..].find("constraint") {
        let start = from + offset;
        match tag_check_at(&lower, start) {
            Some((body_start, body_end, end)) => {
                bodies.push(sql[body_start..body_end].to_owned());
                from = end;
            }
            None => from = start + 1,
        }
    }
    bodies
}

/// `body` with every SQL line comment dropped.
fn without_line_comments(body: &str) -> String {
    let mut out = String::new();
    let mut rest = body;
    while let Some(at) = rest.find("--") {
        out.push_str(&rest[..at]);
        let comment = &rest[at..];
        rest = match comment.find('\n') {
            Some(newline) => &comment[newline..],
            None => "",
        };
    }
    out.push_str(rest);
    out
}

/// Every single-quoted string in `body`.
fn quoted_strings(body: &str) -> Vec<String> {
    let mut strings = Vec::new();
    let mut rest = body;
    while let Some(open) = rest.find('\'') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('\'') else { break };
        strings.push(after[..close].to_owned());
        rest = &after[close + 1..];
    }
    strings
}

/// The tags `public.cards.cards_tags_check` admits once every migration has run: the array in the
/// last migration, in apply order, that adds the check.
fn admitted_tags() -> (String, Vec<String>) {
    let mut files: Vec<String> = std::fs::read_dir(MIGRATIONS)
        .expect("the migrations directory")
        .filter_map(|entry| entry.ok().and_then(|entry| entry.file_name().into_string().ok()))
        .filter(|name| name.ends_with(".sql"))
        .collect();
    files.sort();
    let mut found: Option<(String, Vec<String>)> = None;
    for file in files {
        let sql = std::fs::read_to_string(Path::new(MIGRATIONS).join(&file)).expect("a migration reads");
        for body in tag_check_bodies(&sql) {
            found = Some((file.clone(), quoted_strings(&without_line_comments(&body))));
        }
    }
    found.unwrap_or_else(|| panic!("no migration in {MIGRATIONS} adds cards_tags_check"))
}

/// A card that satisfies the M4-T1 subset `is_entry` checks.
fn card(id: &str) -> Value {
    let index: String = id
        .chars()
        .rev()
        .take(3)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    json!({
        "id": id,
        "index": index,
        "name": format!("Card {id}"),
        "set": "Core",
        "type": "Unit",
        "tags": ["Human"],
        "rarity": "Common",
        "token": false,
        "cost": 2
    })
}

/// A fresh `catalog.json` in its own temporary directory, holding `contents`.
fn file_holding(contents: &Value) -> String {
    let dir: PathBuf = std::env::temp_dir().join(format!("jackioh-catalog-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("a temporary directory");
    let path = dir.join("catalog.json");
    std::fs::write(&path, serde_json::to_string(contents).expect("JSON")).expect("the file is written");
    path.to_string_lossy().into_owned()
}

async fn refusal(path: &str) -> String {
    read_catalog(path)
        .await
        .expect_err("the file is refused")
        .to_string()
}

mod read_catalog_ {
    use super::*;

    #[tokio::test]
    async fn reads_the_real_catalog_json_every_entry_cards_and_tokens_of_every_set_that_ships() {
        let mut raw: IndexMap<String, Value> =
            serde_json::from_str(&std::fs::read_to_string(REAL_CATALOG).expect("the real catalog reads"))
                .expect("the real catalog is a record");
        // R1420: a set that has not shipped is not seeded.
        raw.retain(|_, entry| {
            serde_json::from_value::<jackioh_engine::SetName>(entry["set"].clone())
                .map_or(true, jackioh_engine::set_ships)
        });
        let entries = read_catalog(REAL_CATALOG)
            .await
            .expect("the real catalog is read");
        assert_eq!(entries.len(), raw.len());
        assert_eq!(
            entries.iter().filter(|entry| entry.token).count(),
            raw.values().filter(|entry| entry["token"] == json!(true)).count()
        );
        assert!(entries.iter().any(|entry| entry.id == "core-001"));
        let sets: Vec<String> = entries
            .iter()
            .map(|entry| entry.set.clone())
            .collect::<IndexSet<_>>()
            .into_iter()
            .collect();
        // R1420: the sets that ship, in catalog order: Core, Classic and Classic+, and Meditative
        // from the patch that ships it.
        assert_eq!(
            sets,
            SHIPPED_SETS
                .iter()
                .map(|set| set.as_str().to_owned())
                .collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn accepts_a_bare_array_and_a_cards_wrapper_too() {
        let bare = read_catalog(&file_holding(&json!([card("core-001")])))
            .await
            .expect("a bare array is read");
        assert_eq!(bare.len(), 1);
        let wrapped = read_catalog(&file_holding(&json!({ "cards": [card("core-001")] })))
            .await
            .expect("a { cards } wrapper is read");
        assert_eq!(wrapped.len(), 1);
    }

    #[tokio::test]
    async fn accepts_a_record_keyed_by_card_id() {
        let entries = read_catalog(&file_holding(&json!({ "core-001": card("core-001") })))
            .await
            .expect("a record keyed by id is read");
        assert_eq!(
            entries.iter().map(|entry| entry.id.clone()).collect::<Vec<_>>(),
            vec!["core-001"]
        );
    }

    /// `seed_catalog` inserts `card.id`, so a key that disagrees would seed the wrong row.
    #[tokio::test]
    async fn refuses_a_record_whose_key_disagrees_with_the_entry_s_own_id() {
        let path = file_holding(&json!({ "core-002": card("core-001") }));
        assert!(
            refusal(&path)
                .await
                .contains("keyed \"core-002\" carries id \"core-001\"")
        );
    }

    #[tokio::test]
    async fn still_refuses_a_shape_that_is_none_of_the_three() {
        assert!(
            refusal(&file_holding(&json!({})))
                .await
                .contains("expected an array of cards")
        );
        assert!(
            refusal(&file_holding(&json!(7)))
                .await
                .contains("expected an array of cards")
        );
    }

    #[tokio::test]
    async fn names_the_missing_file_rather_than_throwing_an_enoent() {
        assert!(refusal("/nowhere/catalog.json").await.contains("cannot read"));
    }
}

/// The seed writes the whole catalog in one transaction, so one tag the schema refuses fails every
/// row. This reads the migrations instead of a database, so `cargo test` catches a tag that reaches
/// the catalog before the schema. `test:sql` CHECK 18 and `test:db` (the second half of this file)
/// prove the same against Postgres.
mod r278_the_catalog_s_tags_and_the_cards_table_s_tag_check {
    use super::*;

    #[tokio::test]
    async fn r278_every_tag_the_real_catalog_carries_jlockeed_book_pancake_ai_plague_catalyst_prime_and_acclaimed_included_is_one_the_latest_cards_tags_check_admits()
     {
        let entries = read_catalog(REAL_CATALOG)
            .await
            .expect("the real catalog is read");
        let (file, tags) = admitted_tags();
        assert_eq!(
            file, "0029_meditative_set.sql",
            "0029 re-adds the check with Wincon (R1411)"
        );
        let mut carried: Vec<String> = entries
            .iter()
            .flat_map(|entry| entry.tags.iter().cloned())
            .collect::<IndexSet<_>>()
            .into_iter()
            .collect();
        carried.sort();
        assert!(carried.iter().any(|tag| tag == "Jlockeed"));
        for tag in [
            "Book",
            "Pancake",
            "AI",
            "Plague",
            "Catalyst",
            "Prime",
            "Acclaimed",
        ] {
            assert!(
                carried.iter().any(|carried| carried == tag),
                "the catalog carries {tag}"
            );
        }
        let refused: Vec<&String> = carried.iter().filter(|tag| !tags.contains(tag)).collect();
        assert!(refused.is_empty(), "tags the schema would refuse: {refused:?}");
        // No stale name either: every tag the check admits is one the engine's `Tag` union names
        // (the r1411 test below holds the two lists equal).
        assert!(
            tags.iter()
                .all(|tag| Tag::ALL.iter().any(|each| each.as_str() == tag)),
            "{tags:?}"
        );
    }

    /// R1411: the migration that ships the Meditative set. The check admits exactly the tags the
    /// engine names, in its order, Wincon among them, so every entry of every set the catalog holds
    /// seeds the day its set ships, the Meditative set's included.
    #[tokio::test]
    async fn r1411_the_tag_check_admits_exactly_the_engine_s_tags_so_every_set_the_catalog_holds_seeds_the_day_it_ships()
     {
        let (file, tags) = admitted_tags();
        assert_eq!(file, "0029_meditative_set.sql");
        assert!(tags.iter().any(|tag| tag == "Wincon"), "{tags:?}");
        assert_eq!(
            tags,
            Tag::ALL
                .iter()
                .map(|tag| tag.as_str().to_owned())
                .collect::<Vec<_>>(),
            "the check lists the engine's `Tag` union, in its order"
        );
        // Every set's entries, shipped or not (R1420): the catalog file read whole, unfiltered.
        let raw: IndexMap<String, Value> =
            serde_json::from_str(&std::fs::read_to_string(REAL_CATALOG).expect("the real catalog reads"))
                .expect("the real catalog is a record");
        let refused: Vec<String> = raw
            .values()
            .flat_map(|entry| entry["tags"].as_array().cloned().unwrap_or_default())
            .filter_map(|tag| tag.as_str().map(str::to_owned))
            .filter(|tag| !tags.contains(tag))
            .collect::<IndexSet<_>>()
            .into_iter()
            .collect();
        assert!(refused.is_empty(), "tags the schema would refuse: {refused:?}");
        // The set's other columns: its types and rarities are ones 0002's checks admit, and no check
        // names a set.
        for entry in raw.values().filter(|entry| entry["set"] == json!("Meditative")) {
            assert!(
                ["Unit", "Spell", "Field Spell", "Trap", "Field Trap"]
                    .contains(&entry["type"].as_str().unwrap_or_default()),
                "{}",
                entry["id"]
            );
            assert!(
                ["Common", "Rare", "Epic", "Legendary", "Mythic", "Token"]
                    .contains(&entry["rarity"].as_str().unwrap_or_default()),
                "{}",
                entry["id"]
            );
        }
    }
}

/// `test:db` only: the real catalog into a real Postgres.
mod r278_db_seed_catalog_writes_the_real_catalog_jlockeed_book_pancake_ai_plague_catalyst_prime_and_acclaimed_tags_included {
    use super::*;
    use jackioh_server::cli::seed_catalog::seed_catalog;
    use sqlx::PgPool;

    /// The store contract's fixture catalog version.
    const CATALOG_VERSION: &str = "core-1";

    /// R1420: `n` once the Meditative set ships, and nothing before, since its entries are not
    /// seeded until then. The counts are its census (docs/meditative-set.md M2).
    fn meditative(n: usize) -> usize {
        if set_ships(SetName::Meditative) { n } else { 0 }
    }

    /// The harness's truncate: every table but `public.cards`, including the three that reference it.
    const TRUNCATE: &str = "truncate
  public.series, public.results, public.match_actions, public.tickets, public.matches,
  public.trios, public.decks,
  public.loadout_deck_cards, public.loadout_decks, public.loadouts,
  public.collection_grants, public.collection,
  public.code_attempts, public.invite_codes, public.profiles, auth.users
  restart identity cascade";

    /// The harness's fixture catalog: 64 playable ids and two tokens.
    fn playable_ids() -> Vec<String> {
        (1..=64).map(|i| format!("core-{i:03}")).collect()
    }

    fn token_ids() -> Vec<String> {
        vec!["core-001.1".to_owned(), "core-002.1".to_owned()]
    }

    /// The fixture rows the rest of the database suite runs against.
    async fn seed_cards(admin: &PgPool) {
        sqlx::query(
            "insert into public.cards (id, card_index, name, set_id, type, tags, rarity, token, cost, catalog_version)
     select c.id, c.ord::text, 'Fixture ' || c.id, 'Core', 'Unit', '{}'::text[],
            case when c.token then 'Token' else 'Common' end, c.token, '1'::jsonb, $3::text
       from (
         select t.id, false as token, t.ord from unnest($1::text[]) with ordinality as t(id, ord)
         union all
         select t.id, true, 1000 + t.ord from unnest($2::text[]) with ordinality as t(id, ord)
       ) as c
     on conflict (id) do nothing",
        )
        .bind(playable_ids())
        .bind(token_ids())
        .bind(CATALOG_VERSION)
        .execute(admin)
        .await
        .expect("the fixture cards are seeded");
    }

    /// The suite's other files run against the harness's fixture catalog, and its launch grant gives
    /// every non-token card in `public.cards`. So this puts the fixture rows back, whichever file
    /// runs next.
    async fn restore_fixture_catalog(admin: &PgPool) {
        sqlx::query(TRUNCATE)
            .execute(admin)
            .await
            .expect("the tables are emptied");
        sqlx::query("delete from public.cards")
            .execute(admin)
            .await
            .expect("the real catalog is removed");
        seed_cards(admin).await;
    }

    async fn r278_seeds_every_entry_of_the_shipped_sets_with_the_jlockeed_cards_tagged_and_no_other_row(
        url: &str,
        admin: &PgPool,
    ) {
        let entries = read_catalog(REAL_CATALOG)
            .await
            .expect("the real catalog is read");
        let written = seed_catalog(url, CATALOG_VERSION, &entries)
            .await
            .expect("the catalog is seeded");
        assert_eq!(written, entries.len());

        let ids: Vec<String> = entries.iter().map(|entry| entry.id.clone()).collect();
        let rows: Vec<(String, Vec<String>, String)> = sqlx::query_as(
            "select id, tags, catalog_version from public.cards where id = any($1::text[]) order by id",
        )
        .bind(&ids)
        .fetch_all(admin)
        .await
        .expect("the rows read back");
        assert_eq!(rows.len(), entries.len());
        assert!(rows.iter().all(|(_, _, version)| version == CATALOG_VERSION));
        let jlockeed: Vec<&str> = rows
            .iter()
            .filter(|(_, tags, _)| tags.iter().any(|tag| tag == "Jlockeed"))
            .map(|(id, _, _)| id.as_str())
            .collect();
        let mut expected = vec![
            "classic-004",
            "classicplus-048",
            "classicplus-051",
            "classicplus-052",
            "core-013",
            "core-014",
        ];
        // R1420: the Meditative set's three, #87 Tatches the Totem, #97 Jlockheed's Evil
        // Blueprints and its token #97.9 Jlockheed's Headquarters, once it ships.
        if set_ships(SetName::Meditative) {
            expected.extend(["meditative-087", "meditative-097", "meditative-097-9"]);
        }
        assert_eq!(jlockeed, expected);
        let tagged = |tag: &str| {
            entries
                .iter()
                .filter(|entry| entry.tags.iter().any(|t| t == tag))
                .count()
        };
        let rows_tagged = |tag: &str| {
            rows.iter()
                .filter(|(_, tags, _)| tags.iter().any(|t| t == tag))
                .count()
        };
        for tag in ["Book", "Pancake", "AI", "Plague"] {
            assert_eq!(rows_tagged(tag), tagged(tag), "{tag}");
        }
        assert_eq!(tagged("Plague"), 17 + meditative(2));
        for (tag, theirs) in [("Catalyst", 2), ("Prime", 2), ("Acclaimed", 4)] {
            assert_eq!(rows_tagged(tag), tagged(tag), "{tag}");
            assert_eq!(tagged(tag), 2 + meditative(theirs), "{tag}");
        }
        // R1411: Wincon, the tag 0029 admits, is on the Meditative set's #8 and #20 alone.
        assert_eq!(rows_tagged("Wincon"), tagged("Wincon"));
        assert_eq!(tagged("Wincon"), meditative(2));
        // Each row's tags are the catalog's, so the check admitted them and nothing rewrote them.
        let by_id: IndexMap<&str, &Vec<String>> = entries
            .iter()
            .map(|entry| (entry.id.as_str(), &entry.tags))
            .collect();
        for (id, tags, _) in &rows {
            assert_eq!(Some(&tags), by_id.get(id.as_str()), "{id}");
        }
    }

    async fn r278_a_second_seed_of_the_same_catalog_updates_in_place_and_the_tag_check_still_refuses_an_unknown_tag(
        url: &str,
        admin: &PgPool,
    ) {
        let entries = read_catalog(REAL_CATALOG)
            .await
            .expect("the real catalog is read");
        let again = seed_catalog(url, CATALOG_VERSION, &entries)
            .await
            .expect("a second seed succeeds");
        assert_eq!(again, entries.len());

        let first = entries.first().expect("the catalog is not empty").clone();
        let mut misspelt = first.clone();
        misspelt.tags = vec!["Jlocked".to_owned()];
        let refused = seed_catalog(url, CATALOG_VERSION, &[misspelt])
            .await
            .expect_err("the unknown tag is refused");
        assert!(format!("{refused:#}").contains("cards_tags_check"), "{refused:#}");
        // One transaction: the refused seed left the row as the real catalog wrote it.
        let (tags,): (Vec<String>,) = sqlx::query_as("select tags from public.cards where id = $1")
            .bind(&first.id)
            .fetch_one(admin)
            .await
            .expect("the row reads back");
        assert_eq!(tags, first.tags);
    }

    #[tokio::test]
    async fn r278_seeds_the_real_catalog_and_reseeds_it_in_place() {
        let Ok(url) = std::env::var("DATABASE_URL") else {
            return;
        };
        if url.is_empty() {
            return;
        }
        // A raw superuser connection, for the setup and teardown the store deliberately cannot do.
        let admin = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .expect("the admin connection opens");
        sqlx::query(TRUNCATE)
            .execute(&admin)
            .await
            .expect("the tables are emptied");

        r278_seeds_every_entry_of_the_shipped_sets_with_the_jlockeed_cards_tagged_and_no_other_row(
            &url, &admin,
        )
        .await;
        r278_a_second_seed_of_the_same_catalog_updates_in_place_and_the_tag_check_still_refuses_an_unknown_tag(
            &url, &admin,
        )
        .await;

        restore_fixture_catalog(&admin).await;
        admin.close().await;
    }
}
