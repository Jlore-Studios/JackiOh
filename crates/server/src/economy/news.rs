//! News (SPEC §14.13, #2875): every shipped patch and every alert, each with one reward an account
//! claims once.
//!
//! - The entries are data, `crates/server/news.json`, compiled in like the quest pool, oldest first,
//!   and [`parse`] is its check: a slug id, a title, a summary of at most
//!   `NEWS_SUMMARY_MAX_SENTENCES` sentences, an RFC 3339 `postedAt`, a patch's version in
//!   `patches.json`, and a reward of a known kind ([[R2875003]]). The server refuses to boot on a
//!   file that fails it.
//! - An entry lives `NEWS_TTL_DAYS` from its `postedAt`, on the server's clock ([`live_at`],
//!   [[R2875001]]); [`board`] is what `GET /api/news` and a claim answer.
//! - A claim ([`claim_news_once`], [[R2875000]]) pays the reward as ledger rows with the reason
//!   `news` and writes migration 0077's `news_claims` row, once per account and entry. Unread is
//!   unclaimed ([[R2875002]]).
//!
//! Nothing here checks `ECONOMY_LIVE` (§14.12): the claim's route refuses while it is off. Every
//! number is read through its [`crate::config`] constant (CLAUDE.md rule 9).

use std::collections::BTreeSet;

use serde::Deserialize;
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::idempotency::{RequestKey, Spent, keep, replayed};
use super::packs::inventory_of;
use super::roll::PackKind;
use super::store::{EconomyReason, InventoryGrant, InventoryItem, JloreGrant, JloreReason};
use crate::api::catalog::PATCHES_JSON;
use crate::api::http::{ApiError, ApiErrorCode};
use crate::app::{App, now_ms};
use crate::config::{
    MS_PER_DAY, NEWS_REWARD_CARD_PACKS, NEWS_REWARD_CARD_TOKENS, NEWS_REWARD_COSMETIC_PACKS,
    NEWS_REWARD_COSMETIC_TOKENS, NEWS_REWARD_JLORE, NEWS_SUMMARY_MAX_SENTENCES, NEWS_TTL_DAYS,
};
use crate::cosmetics::{CosmeticRarity, is_slug};
use crate::db::store::Tx;

pub const NEWS_JSON: &str = include_str!("../../news.json");

/// Unit conversion, not configuration: `time` counts nanoseconds, the server milliseconds.
const NANOS_PER_MS: i128 = 1_000_000;

/// What an entry tells of: a shipped patch, which names its version, or an alert, which names none.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NewsKind {
    Patch,
    Alert,
}

impl NewsKind {
    fn as_str(self) -> &'static str {
        match self {
            NewsKind::Patch => "patch",
            NewsKind::Alert => "alert",
        }
    }
}

/// R2875003: an entry's one reward, from what the economy already grants (§14): Jlore (§14.1), an
/// unopened card pack, a set's or the global one (§14.3), a card token, a cosmetic token of a tier
/// (§14.5) or an unopened cosmetic pack (§14.6). A card pack's set ships (R1420), and a cosmetic
/// token is Rare, Epic or Legendary, as the cycle pays them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NewsReward {
    Jlore,
    CardPack(PackKind),
    CardToken,
    CosmeticToken(CosmeticRarity),
    CosmeticPack,
}

impl NewsReward {
    /// What the reward pays: its kind's named amount.
    pub fn amount(self) -> i64 {
        match self {
            NewsReward::Jlore => NEWS_REWARD_JLORE,
            NewsReward::CardPack(_) => NEWS_REWARD_CARD_PACKS,
            NewsReward::CardToken => NEWS_REWARD_CARD_TOKENS,
            NewsReward::CosmeticToken(_) => NEWS_REWARD_COSMETIC_TOKENS,
            NewsReward::CosmeticPack => NEWS_REWARD_COSMETIC_PACKS,
        }
    }

    /// The reward as the client reads it: `{ kind, amount }`, with a card pack's `pack` (its name,
    /// as a request gives it) and a cosmetic token's `tier`.
    pub fn view(self) -> Value {
        let amount = self.amount();
        match self {
            NewsReward::Jlore => json!({ "kind": "jlore", "amount": amount }),
            NewsReward::CardPack(pack) => json!({ "kind": "cardPack", "amount": amount, "pack": pack.name() }),
            NewsReward::CardToken => json!({ "kind": "cardToken", "amount": amount }),
            NewsReward::CosmeticToken(tier) => {
                json!({ "kind": "cosmeticToken", "amount": amount, "tier": tier.as_str() })
            }
            NewsReward::CosmeticPack => json!({ "kind": "cosmeticPack", "amount": amount }),
        }
    }
}

/// One entry, as [`parse`] read it.
#[derive(Clone, Debug, PartialEq)]
pub struct NewsEntry {
    /// A lower-case slug, unique in the file and, once shipped, never renamed: a claim names it.
    pub id: String,
    pub kind: NewsKind,
    /// A patch's version, one of `patches.json`'s; `None` for an alert.
    pub version: Option<String>,
    /// The English title the client shows as sent.
    pub title: String,
    /// The English summary the client shows as sent, `NEWS_SUMMARY_MAX_SENTENCES` sentences at most.
    pub summary: String,
    /// When it was posted, in milliseconds.
    pub posted_at: i64,
    pub reward: NewsReward,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewsFile {
    news: Vec<RawEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawEntry {
    id: String,
    kind: NewsKind,
    #[serde(default)]
    version: Option<String>,
    title: String,
    summary: String,
    posted_at: String,
    #[serde(default)]
    reward: Option<RawReward>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawReward {
    kind: String,
    #[serde(default)]
    pack: Option<String>,
    #[serde(default)]
    tier: Option<String>,
}

/// One change of a patch, as `patches.json` records it: a card `added`, or `changed`.
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct PatchChange {
    pub id: String,
    pub kind: String,
}

/// What a patch entry's default reward reads of its patch in `patches.json`: its version, the
/// cards that play differently (`plays`) and its catalog changes.
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct PatchRecord {
    pub version: String,
    #[serde(default)]
    pub plays: Vec<String>,
    #[serde(default)]
    pub changes: Vec<PatchChange>,
}

impl PatchRecord {
    /// The patch adds a card, or reprints one: a card whose catalog entry changed and that plays
    /// differently (R2499000's reprint).
    pub fn adds_or_reprints_cards(&self) -> bool {
        self.changes.iter().any(|change| {
            change.kind == "added" || (change.kind == "changed" && self.plays.contains(&change.id))
        })
    }
}

/// Every patch of `patches.json`, in the order they were made.
pub fn patch_records() -> Result<Vec<PatchRecord>, String> {
    serde_json::from_str(PATCHES_JSON).map_err(|cause| format!("patches.json is not a patch list: {cause}"))
}

/// R2875003: the reward of an entry whose author named none: a global card pack for a patch that
/// adds or reprints cards ([`PatchRecord::adds_or_reprints_cards`]), and Jlore for the rest, any
/// other patch and every alert.
pub fn default_reward(kind: NewsKind, record: Option<&PatchRecord>) -> NewsReward {
    match (kind, record) {
        (NewsKind::Patch, Some(record)) if record.adds_or_reprints_cards() => {
            NewsReward::CardPack(PackKind::Global)
        }
        _ => NewsReward::Jlore,
    }
}

/// How many sentences `text` holds: each `.`, `!` or `?` followed by whitespace or the end, so a
/// version's dots (`v0.5.11`) end none.
pub fn sentences(text: &str) -> usize {
    let chars: Vec<char> = text.chars().collect();
    chars
        .iter()
        .enumerate()
        .filter(|(at, c)| {
            matches!(c, '.' | '!' | '?') && chars.get(at + 1).is_none_or(|next| next.is_whitespace())
        })
        .count()
}

/// A named reward, or why it names none.
fn reward_of(raw: &RawReward) -> Result<NewsReward, String> {
    let reward = match raw.kind.as_str() {
        "jlore" => NewsReward::Jlore,
        // A card pack that names no pack is the global one.
        "cardPack" => NewsReward::CardPack(match raw.pack.as_deref() {
            None => PackKind::Global,
            Some(name) => PackKind::parse(name).ok_or_else(|| format!("no card pack ships as {name:?}"))?,
        }),
        "cardToken" => NewsReward::CardToken,
        "cosmeticToken" => {
            let tier = raw
                .tier
                .as_deref()
                .ok_or_else(|| "a cosmetic token names its tier".to_string())?;
            match CosmeticRarity::named(tier) {
                Some(rarity @ (CosmeticRarity::Rare | CosmeticRarity::Epic | CosmeticRarity::Legendary)) => {
                    NewsReward::CosmeticToken(rarity)
                }
                _ => return Err(format!("no cosmetic token is {tier:?}")),
            }
        }
        "cosmeticPack" => NewsReward::CosmeticPack,
        other => return Err(format!("no reward is {other:?}")),
    };
    if raw.pack.is_some() && !matches!(reward, NewsReward::CardPack(_)) {
        return Err("only a card pack names a pack".to_string());
    }
    if raw.tier.is_some() && !matches!(reward, NewsReward::CosmeticToken(_)) {
        return Err("only a cosmetic token names a tier".to_string());
    }
    Ok(reward)
}

/// R2875003: the news file's entries, oldest first, or everything wrong with it, one problem each,
/// naming its entry's id, joined with `"; "`.
pub fn parse(json: &str) -> Result<Vec<NewsEntry>, String> {
    let file: NewsFile =
        serde_json::from_str(json).map_err(|cause| format!("news.json is not a news file: {cause}"))?;
    let records = patch_records()?;
    let mut problems = Vec::new();
    let mut seen = BTreeSet::new();
    let mut entries = Vec::new();
    let mut last_posted = None;
    for raw in file.news {
        let id = raw.id.as_str();
        let mut problem = |why: String| problems.push(format!("{id:?}: {why}"));
        if !is_slug(id) {
            problem("the id is not a slug".to_string());
        }
        if !seen.insert(id.to_string()) {
            problem("the id is listed twice".to_string());
        }
        if raw.title.trim().is_empty() {
            problem("the title is empty".to_string());
        }
        if !raw.summary.ends_with(['.', '!', '?']) {
            problem("the summary does not end a sentence".to_string());
        }
        let count = sentences(&raw.summary);
        if !(1..=NEWS_SUMMARY_MAX_SENTENCES).contains(&count) {
            problem(format!(
                "the summary holds {count} sentences, not 1 to {NEWS_SUMMARY_MAX_SENTENCES}"
            ));
        }
        let posted_at = OffsetDateTime::parse(&raw.posted_at, &Rfc3339)
            .ok()
            .and_then(|at| i64::try_from(at.unix_timestamp_nanos() / NANOS_PER_MS).ok());
        match (posted_at, last_posted) {
            (None, _) => problem(format!("postedAt {:?} is no RFC 3339 time", raw.posted_at)),
            (Some(at), Some(last)) if at < last => {
                problem("it is posted before the entry above it (the file is oldest first)".to_string())
            }
            (Some(_), _) => last_posted = posted_at,
        }
        let record = match (raw.kind, raw.version.as_deref()) {
            (NewsKind::Patch, None) => {
                problem("a patch names its version".to_string());
                None
            }
            (NewsKind::Patch, Some(version)) => {
                let record = records.iter().find(|record| record.version == version);
                if record.is_none() {
                    problem(format!("no patch of patches.json is {version:?}"));
                }
                record
            }
            (NewsKind::Alert, Some(_)) => {
                problem("an alert names no version".to_string());
                None
            }
            (NewsKind::Alert, None) => None,
        };
        let reward = match &raw.reward {
            None => Some(default_reward(raw.kind, record)),
            Some(named) => reward_of(named).map_err(&mut problem).ok(),
        };
        if let (Some(posted_at), Some(reward)) = (posted_at, reward) {
            entries.push(NewsEntry {
                id: raw.id,
                kind: raw.kind,
                version: raw.version,
                title: raw.title,
                summary: raw.summary,
                posted_at,
                reward,
            });
        }
    }
    if problems.is_empty() {
        Ok(entries)
    } else {
        Err(problems.join("; "))
    }
}

/// The entries this build ships ([`NEWS_JSON`]), checked.
pub fn shipped() -> Result<Vec<NewsEntry>, String> {
    parse(NEWS_JSON)
}

/// R2875001: an entry is live from its `postedAt` until `NEWS_TTL_DAYS` after it, on the server's
/// clock: before it, it is not posted yet, and after it, it has expired, claimed or not.
pub fn live_at(entry: &NewsEntry, now: i64) -> bool {
    entry.posted_at <= now && now < entry.posted_at.saturating_add(NEWS_TTL_DAYS * MS_PER_DAY)
}

/// What `GET /api/news` and a claim answer: `{ news }`, the entries live at `now` newest first (of
/// two posted at once, the one later in the file first), each marked `claimed` when its id is in
/// `claimed` ([[R2875002]]: the client counts the unclaimed ones and nothing else).
pub fn board(entries: &[NewsEntry], claimed: &[String], now: i64) -> Value {
    let mut live: Vec<&NewsEntry> = entries.iter().rev().filter(|entry| live_at(entry, now)).collect();
    live.sort_by_key(|entry| std::cmp::Reverse(entry.posted_at));
    let news: Vec<Value> = live
        .into_iter()
        .map(|entry| {
            json!({
                "id": entry.id,
                "kind": entry.kind.as_str(),
                "version": entry.version,
                "title": entry.title,
                "summary": entry.summary,
                "postedAt": entry.posted_at,
                "reward": entry.reward.view(),
                "claimed": claimed.contains(&entry.id),
            })
        })
        .collect();
    json!({ "news": news })
}

fn already_claimed() -> ApiError {
    ApiError::new(ApiErrorCode::Conflict, "That reward is already claimed.")
}

/// Pays `entry`'s reward to the account in `tx`, its rows' reason `news` and `ref` the entry's id
/// (§14.11): Jlore in the wallet, every other reward in the inventory.
async fn pay(tx: &mut Tx<'_>, profile_id: &str, entry: &NewsEntry, at: i64) -> Result<(), ApiError> {
    let amount = entry.reward.amount();
    let (item, key) = match entry.reward {
        NewsReward::Jlore => {
            tx.economy_jlore_grant(&JloreGrant {
                profile_id: profile_id.to_string(),
                delta: amount,
                reason: JloreReason::News,
                reference: Some(entry.id.clone()),
                dev: false,
                at,
            })
            .await?
            .ok_or_else(|| ApiError::internal("the wallet refused a news reward"))?;
            return Ok(());
        }
        NewsReward::CardPack(pack) => inventory_of(pack),
        NewsReward::CardToken => (InventoryItem::CardToken, String::new()),
        NewsReward::CosmeticToken(tier) => (InventoryItem::CosmeticToken, tier.as_str().to_string()),
        NewsReward::CosmeticPack => (InventoryItem::CosmeticPack, String::new()),
    };
    tx.economy_inventory_grant(&InventoryGrant {
        profile_id: profile_id.to_string(),
        item,
        key,
        delta: amount,
        reason: EconomyReason::News,
        reference: Some(entry.id.clone()),
        dev: false,
        at,
    })
    .await?
    .ok_or_else(|| ApiError::internal("the inventory refused a news reward"))?;
    Ok(())
}

/// R2875000: claims the news entry `news_id`'s reward for the account, once, under an optional
/// `Idempotency-Key` (R2124000), in one transaction: right after the account's lock, before any
/// check, a key already committed answers what it kept. An entry this build does not hold, or not
/// posted yet, is 404; one past its `NEWS_TTL_DAYS` is 409 ([[R2875001]]), as is a second claim,
/// which `news_claims`' key refuses whoever races it. A refusal pays nothing. Answers [`board`]
/// with the entry claimed.
pub async fn claim_news_once(
    app: &App,
    profile_id: &str,
    news_id: &str,
    request: Option<&RequestKey>,
) -> Result<Spent<Value>, ApiError> {
    let now = now_ms();
    let mut tx = app.db.begin(Some(profile_id)).await?;
    tx.economy_lock_account(profile_id).await?;
    if let Some(before) = replayed(&mut tx, profile_id, request).await? {
        return Ok(before);
    }
    let entry = app
        .news
        .iter()
        .find(|entry| entry.id == news_id && entry.posted_at <= now)
        .ok_or_else(|| ApiError::new(ApiErrorCode::NotFound, "That news isn't posted."))?;
    if !live_at(entry, now) {
        return Err(ApiError::new(ApiErrorCode::Conflict, "That news has expired."));
    }
    let mut claimed = tx.economy_news_claims_get(profile_id).await?;
    if claimed.iter().any(|id| id == news_id) {
        return Err(already_claimed());
    }
    if !tx.economy_news_claim(profile_id, news_id, now).await? {
        return Err(already_claimed());
    }
    pay(&mut tx, profile_id, entry, now).await?;
    claimed.push(news_id.to_string());
    let answer = board(&app.news, &claimed, now);
    keep(&mut tx, profile_id, request, &answer).await?;
    tx.commit().await?;
    tracing::info!(event = "news.claimed", "profileId" = %profile_id, news = news_id);
    Ok(Spent::Now(answer))
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::wire::SetName;

    /// One patch entry of the shipped file's shape, posted at `posted_at`.
    fn entry(id: &str, posted_at: i64, reward: NewsReward) -> NewsEntry {
        NewsEntry {
            id: id.to_string(),
            kind: NewsKind::Alert,
            version: None,
            title: "A title".to_string(),
            summary: "A summary.".to_string(),
            posted_at,
            reward,
        }
    }

    fn record(version: &str) -> PatchRecord {
        patch_records()
            .expect("patches.json parses")
            .into_iter()
            .find(|record| record.version == version)
            .unwrap_or_else(|| panic!("{version} is in patches.json"))
    }

    #[test]
    fn the_shipped_news_passes_its_check() {
        let entries = shipped().unwrap_or_else(|problems| panic!("news.json: {problems}"));
        assert!(!entries.is_empty());
    }

    #[test]
    fn parse_names_every_broken_entry() {
        let file = r#"{ "news": [
            { "id": "Not-A-Slug", "kind": "alert", "title": "T", "summary": "Fine.", "postedAt": "2026-11-01T00:00:00Z" },
            { "id": "twice", "kind": "alert", "title": "T", "summary": "Fine.", "postedAt": "2026-11-01T00:00:00Z" },
            { "id": "twice", "kind": "alert", "title": "T", "summary": "Fine.", "postedAt": "2026-11-01T00:00:00Z" },
            { "id": "untitled", "kind": "alert", "title": " ", "summary": "Fine.", "postedAt": "2026-11-01T00:00:00Z" },
            { "id": "open-ended", "kind": "alert", "title": "T", "summary": "No end", "postedAt": "2026-11-01T00:00:00Z" },
            { "id": "wordy", "kind": "alert", "title": "T", "summary": "One. Two! Three?", "postedAt": "2026-11-01T00:00:00Z" },
            { "id": "undated", "kind": "alert", "title": "T", "summary": "Fine.", "postedAt": "yesterday" },
            { "id": "out-of-order", "kind": "alert", "title": "T", "summary": "Fine.", "postedAt": "2026-10-01T00:00:00Z" },
            { "id": "versionless", "kind": "patch", "title": "T", "summary": "Fine.", "postedAt": "2026-11-01T00:00:00Z" },
            { "id": "unshipped", "kind": "patch", "version": "v9.9.9", "title": "T", "summary": "Fine.", "postedAt": "2026-11-01T00:00:00Z" },
            { "id": "versioned-alert", "kind": "alert", "version": "v0.5.0", "title": "T", "summary": "Fine.", "postedAt": "2026-11-01T00:00:00Z" },
            { "id": "gold", "kind": "alert", "title": "T", "summary": "Fine.", "postedAt": "2026-11-01T00:00:00Z", "reward": { "kind": "gold" } },
            { "id": "packed-jlore", "kind": "alert", "title": "T", "summary": "Fine.", "postedAt": "2026-11-01T00:00:00Z", "reward": { "kind": "jlore", "pack": "global" } },
            { "id": "unshipped-pack", "kind": "alert", "title": "T", "summary": "Fine.", "postedAt": "2026-11-01T00:00:00Z", "reward": { "kind": "cardPack", "pack": "Boss" } },
            { "id": "tiered-pack", "kind": "alert", "title": "T", "summary": "Fine.", "postedAt": "2026-11-01T00:00:00Z", "reward": { "kind": "cosmeticPack", "tier": "Epic" } },
            { "id": "mythic-token", "kind": "alert", "title": "T", "summary": "Fine.", "postedAt": "2026-11-01T00:00:00Z", "reward": { "kind": "cosmeticToken", "tier": "Mythic" } },
            { "id": "tierless-token", "kind": "alert", "title": "T", "summary": "Fine.", "postedAt": "2026-11-01T00:00:00Z", "reward": { "kind": "cosmeticToken" } }
        ] }"#;
        let problems = parse(file).expect_err("every entry but the first `twice` is broken");
        for (id, why) in [
            ("Not-A-Slug", "the id is not a slug"),
            ("twice", "the id is listed twice"),
            ("untitled", "the title is empty"),
            ("open-ended", "the summary does not end a sentence"),
            ("open-ended", "the summary holds 0 sentences"),
            ("wordy", "the summary holds 3 sentences"),
            ("undated", "is no RFC 3339 time"),
            ("out-of-order", "posted before the entry above it"),
            ("versionless", "a patch names its version"),
            ("unshipped", "no patch of patches.json is \"v9.9.9\""),
            ("versioned-alert", "an alert names no version"),
            ("gold", "no reward is \"gold\""),
            ("packed-jlore", "only a card pack names a pack"),
            ("unshipped-pack", "no card pack ships as \"Boss\""),
            ("tiered-pack", "only a cosmetic token names a tier"),
            ("mythic-token", "no cosmetic token is \"Mythic\""),
            ("tierless-token", "a cosmetic token names its tier"),
        ] {
            assert!(
                problems.contains(&format!("{id:?}: {why}")),
                "{id}: {why}, in {problems}"
            );
        }
        assert_eq!(problems.split("; ").count(), 17, "{problems}");
        assert!(parse(r#"{ "news": [], "extra": 1 }"#).is_err(), "an unknown field");
    }

    #[test]
    fn sentences_counts_each_end_and_no_dot_inside_a_version() {
        assert_eq!(sentences("Decks hold 25 cards."), 1);
        assert_eq!(sentences("v0.5.11 ships. It is here!"), 2);
        assert_eq!(sentences("Is it? Yes. Truly!"), 3);
        assert_eq!(sentences("A 2.5 rate"), 0);
        assert_eq!(sentences("Wait...\tThen."), 2);
    }

    #[test]
    fn r2875001_an_entry_lives_from_its_posted_at_for_news_ttl_days() {
        let posted = 1_800_000_000_000;
        let ends = posted + NEWS_TTL_DAYS * MS_PER_DAY;
        let news = entry("live", posted, NewsReward::Jlore);
        assert!(!live_at(&news, posted - 1), "not posted yet");
        assert!(live_at(&news, posted));
        assert!(live_at(&news, ends - 1));
        assert!(!live_at(&news, ends), "expired");
        // The board leaves out an entry not yet posted and one past its days, claimed or not.
        let board = board(
            &[
                entry("expired", posted - NEWS_TTL_DAYS * MS_PER_DAY, NewsReward::Jlore),
                news,
                entry("future", posted + 1, NewsReward::Jlore),
            ],
            &["expired".to_string()],
            posted,
        );
        let ids: Vec<&str> = board["news"]
            .as_array()
            .expect("news")
            .iter()
            .filter_map(|entry| entry["id"].as_str())
            .collect();
        assert_eq!(ids, ["live"]);
    }

    #[test]
    fn r2875002_the_board_is_newest_first_each_marked_claimed() {
        let at = 1_800_000_000_000;
        let board = board(
            &[
                entry("first", at, NewsReward::Jlore),
                entry("second", at, NewsReward::Jlore),
                entry("third", at + 1, NewsReward::CardToken),
            ],
            &["second".to_string()],
            at + 1,
        );
        let rows: Vec<(&str, bool)> = board["news"]
            .as_array()
            .expect("news")
            .iter()
            .map(|entry| (entry["id"].as_str().unwrap_or(""), entry["claimed"] == true))
            .collect();
        assert_eq!(rows, [("third", false), ("second", true), ("first", false)]);
        assert_eq!(board["news"][0]["reward"], json!({ "kind": "cardToken", "amount": 1 }));
        assert_eq!(board["news"][0]["kind"], "alert");
        assert_eq!(board["news"][0]["postedAt"], at + 1);
    }

    #[test]
    fn r2875003_a_left_out_reward_is_a_card_pack_for_a_patch_that_adds_or_reprints_cards_and_jlore_otherwise() {
        let pack = NewsReward::CardPack(PackKind::Global);
        // v0.5.0 adds the Stinkyscape set; v0.5.3 reprints 19 families (R2499000).
        assert_eq!(default_reward(NewsKind::Patch, Some(&record("v0.5.0"))), pack);
        assert_eq!(default_reward(NewsKind::Patch, Some(&record("v0.5.3"))), pack);
        // v0.4.0d changes a card's name alone, and v0.4.4 no card at all.
        assert_eq!(
            default_reward(NewsKind::Patch, Some(&record("v0.4.0d"))),
            NewsReward::Jlore
        );
        assert_eq!(
            default_reward(NewsKind::Patch, Some(&record("v0.4.4"))),
            NewsReward::Jlore
        );
        assert_eq!(default_reward(NewsKind::Alert, None), NewsReward::Jlore);
        // A reward the entry names is kept, and an entry that names none takes the default.
        let file = r#"{ "news": [
            { "id": "patch-v0-5-0", "kind": "patch", "version": "v0.5.0", "title": "T", "summary": "S.", "postedAt": "2026-10-28T00:00:00Z", "reward": { "kind": "cardPack", "pack": "Stinkyscape" } },
            { "id": "patch-v0-5-3", "kind": "patch", "version": "v0.5.3", "title": "T", "summary": "S.", "postedAt": "2026-10-29T00:00:00Z" },
            { "id": "patch-v0-4-4", "kind": "patch", "version": "v0.4.4", "title": "T", "summary": "S.", "postedAt": "2026-10-29T00:00:00Z" },
            { "id": "an-alert", "kind": "alert", "title": "T", "summary": "S.", "postedAt": "2026-10-30T00:00:00Z", "reward": { "kind": "cosmeticToken", "tier": "Epic" } }
        ] }"#;
        let rewards: Vec<NewsReward> = parse(file)
            .expect("a good file")
            .iter()
            .map(|entry| entry.reward)
            .collect();
        assert_eq!(
            rewards,
            [
                NewsReward::CardPack(PackKind::Set(SetName::Stinkyscape)),
                pack,
                NewsReward::Jlore,
                NewsReward::CosmeticToken(CosmeticRarity::Epic),
            ]
        );
    }

    #[test]
    fn r2875003_every_reward_kind_pays_its_named_amount() {
        for (reward, amount, view) in [
            (
                NewsReward::Jlore,
                NEWS_REWARD_JLORE,
                json!({ "kind": "jlore", "amount": NEWS_REWARD_JLORE }),
            ),
            (
                NewsReward::CardPack(PackKind::Global),
                NEWS_REWARD_CARD_PACKS,
                json!({ "kind": "cardPack", "amount": NEWS_REWARD_CARD_PACKS, "pack": "global" }),
            ),
            (
                NewsReward::CardPack(PackKind::Set(SetName::TrainingGrounds)),
                NEWS_REWARD_CARD_PACKS,
                json!({ "kind": "cardPack", "amount": NEWS_REWARD_CARD_PACKS, "pack": "Training Grounds" }),
            ),
            (
                NewsReward::CardToken,
                NEWS_REWARD_CARD_TOKENS,
                json!({ "kind": "cardToken", "amount": NEWS_REWARD_CARD_TOKENS }),
            ),
            (
                NewsReward::CosmeticToken(CosmeticRarity::Legendary),
                NEWS_REWARD_COSMETIC_TOKENS,
                json!({ "kind": "cosmeticToken", "amount": NEWS_REWARD_COSMETIC_TOKENS, "tier": "Legendary" }),
            ),
            (
                NewsReward::CosmeticPack,
                NEWS_REWARD_COSMETIC_PACKS,
                json!({ "kind": "cosmeticPack", "amount": NEWS_REWARD_COSMETIC_PACKS }),
            ),
        ] {
            assert_eq!(reward.amount(), amount, "{reward:?}");
            assert!(amount > 0, "{reward:?} pays something");
            assert_eq!(reward.view(), view, "{reward:?}");
        }
    }
}
