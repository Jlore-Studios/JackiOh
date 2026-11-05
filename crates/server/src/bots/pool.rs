//! The bot pool (SPEC §9.12, R1454): active ladder bot count, identities, decks and trios.
//!
//! The pool holds `clamp(BOT_POOL_MIN, BOT_POOL_PER_WEEKLY_ACTIVE × weekly active people, BOT_POOL_MAX)`
//! active bots, sized at boot and every refresh. An excess retires the youngest, which finish their
//! match; a shortfall reinstates the latest retired first, so the ladder meets the same faces.
//! New bots draw strength (R610), persona (R645), schedule, and username (`bots::names`).
//! Bo1 decks draw from human decklists and random decks (R750000, R1453), each kept only when legal
//! and dealt again on a new catalog or deck size (R1705001), then the Conquest trio (R641000, R253).
//! Refreshes draw cosmetics (R1834005), kept on the `bots` row and never in the ledger, and manage
//! top-bot grinding (R1555000). The queue's sweeper runs `release_stale_starts` whatever
//! `LADDER_BOTS` says, as R1450's open-row sweep would hold a bot too long.

use std::sync::Arc;
use std::time::Duration;

use jackioh_shared::bots::fallback::{BotMode, RatedGameRecord, TicketRecord, holds_decks, holds_trio};
use jackioh_shared::bots::names::never_lists;
use jackioh_shared::bots::pool::{
    BotLadder, BotRecord, bot_target_strength, draw_decks, draw_new_bot, draw_trio, plan_grind, plan_pool,
    pool_target,
};

use crate::api::queue::{new_seed, new_uuid};
use crate::api::ranked::{SeasonDeps, build_season_id};
use crate::app::{App, now_ms};
use crate::bots::cosmetics;
use crate::config::{
    BOT_COSMETIC_FITS, BOT_POOL_ACTIVE_WINDOW_DAYS, BOT_POOL_REFRESH_INTERVAL_SECONDS,
    BOT_STALE_START_SECONDS, MS_PER_DAY, MS_PER_SECOND, USERNAME_CHANGE_COOLDOWN_MS,
};
use crate::db::store::{
    BotDeck, BotRow, FrozenTrio, QueueMode, RatedGameRow, StoreError, Ticket, UsernameClaim,
    UsernameClaimOutcome,
};
use crate::ranked::ladder::Ladder;
use crate::username::{matches_lists, normalize_username, username_key};

// The store's rows as the pure decisions read them (`jackioh_shared::bots`): a Roblox DataStore script
// would implement the same traits, or hand over `BotFacts`.

impl From<Ladder> for BotLadder {
    fn from(ladder: Ladder) -> Self {
        match ladder {
            Ladder::Constructed => BotLadder::Constructed,
            Ladder::Random => BotLadder::Random,
        }
    }
}

impl From<QueueMode> for BotMode {
    fn from(mode: QueueMode) -> Self {
        match mode {
            QueueMode::Bo1 => BotMode::Bo1,
            QueueMode::Bo3 => BotMode::Bo3,
            QueueMode::Random => BotMode::Random,
        }
    }
}

impl BotRecord for BotRow {
    fn profile_id(&self) -> &str {
        &self.profile_id
    }
    fn created_at(&self) -> i64 {
        self.created_at
    }
    fn retired_at(&self) -> Option<i64> {
        self.retired_at
    }
    fn grinding(&self) -> bool {
        self.grinding
    }
    fn rating_for(&self, ladder: BotLadder) -> f64 {
        match ladder {
            BotLadder::Constructed => self.rating,
            BotLadder::Random => self.random_rating,
        }
    }
    fn active_from_hour(&self) -> i32 {
        self.active_from_hour
    }
    fn active_hours(&self) -> i32 {
        self.active_hours
    }
    fn in_match_id(&self) -> Option<&str> {
        self.in_match_id.as_deref()
    }
    fn decks(&self) -> &[BotDeck] {
        &self.decks
    }
    fn decks_catalog_version(&self) -> Option<&str> {
        self.decks_catalog_version.as_deref()
    }
    fn trio(&self) -> Option<&FrozenTrio> {
        self.trio.as_ref()
    }
    fn trio_catalog_version(&self) -> Option<&str> {
        self.trio_catalog_version.as_deref()
    }
}

impl TicketRecord for Ticket {
    fn id(&self) -> &str {
        &self.id
    }
    fn rating(&self) -> f64 {
        self.rating
    }
    fn mode(&self) -> BotMode {
        self.mode.into()
    }
    fn enqueued_at(&self) -> i64 {
        self.enqueued_at
    }
}

impl RatedGameRecord for RatedGameRow {
    fn between_bots(&self) -> bool {
        self.sides.0.bot_id.is_some() && self.sides.1.bot_id.is_some()
    }
    fn ended_at(&self) -> i64 {
        self.ended_at
    }
}

/// R1454: a drawn name as the account will hold it, when it is one a person could hold (R1432) that the
/// bots' own list (`never.txt`) lets by; else none, and the draw tries again.
pub fn bot_name_filter(name: &str) -> Option<String> {
    let base = normalize_username(name).ok()?;
    let (anywhere, token) = never_lists();
    (!matches_lists(&base, &anywhere, &token)).then_some(base)
}

/// What one refresh did, for its log line and the tests.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PoolRefresh {
    pub target: usize,
    pub retired: usize,
    pub reinstated: usize,
    pub created: usize,
    /// Bots dealt decks (new, stale catalog, or invalid size, R1705001).
    pub dealt: usize,
    /// R641000: bots whose Conquest trio was dealt (R1705001).
    pub trios: usize,
    /// R1555000: bots that grind after it.
    pub grinding: usize,
    /// R1834005: bots whose cosmetics were drawn (new, redealt, or aged).
    pub dressed: usize,
}

/// R1454: claims `name` for a new bot via username route rules; falls back to `Player#n` on failure.
async fn claim_name(app: &App, profile_id: &str, name: &str, now: i64) -> Result<(), StoreError> {
    let Ok(base) = normalize_username(name) else {
        return Ok(());
    };
    let key = username_key(&base);
    let mut tx = app.db.begin(None).await?;
    let expected_tag = tx.profiles_username_tag_for(profile_id, &key).await?;
    let outcome = tx
        .profiles_claim_username(&UsernameClaim {
            profile_id: profile_id.to_string(),
            base,
            key,
            expected_tag,
            at: now,
            cooldown_ms: USERNAME_CHANGE_COOLDOWN_MS,
        })
        .await?;
    if matches!(outcome, UsernameClaimOutcome::Claimed { .. }) {
        tx.commit().await?;
    }
    Ok(())
}

/// R1454: releases bots and matched players whose start timed out after `BOT_STALE_START_SECONDS`,
/// reconciling the seat ledger immediately (R1523000).
pub async fn release_stale_starts(app: &App, now: i64) -> Result<usize, StoreError> {
    let mut tx = app.db.begin(None).await?;
    let released = tx
        .bots_release_stale_starts(now - BOT_STALE_START_SECONDS * MS_PER_SECOND)
        .await?;
    tx.commit().await?;
    for bot_id in &released {
        tracing::warn!(event = "bots.start.released", botId = %bot_id);
    }
    if !released.is_empty() {
        // R1523000: the bots and players let go here leave the seat ledger too, which a restart
        // filled from their flags.
        crate::seats::reconcile(app).await;
    }
    Ok(released.len())
}

/// R1454: adjusts bot pool size for active players, deals decks (R1705001) and trios (R641000) to
/// active bots needing them, draws eligible cosmetics (R1834005), and updates grinding bots (R1555000).
pub async fn refresh_pool(app: &App, seed: &str, now: i64) -> Result<PoolRefresh, StoreError> {
    let mut tx = app.db.begin(None).await?;
    let bots = tx.bots_list().await?;
    let weekly = tx
        .results_count_active_humans(now - BOT_POOL_ACTIVE_WINDOW_DAYS * MS_PER_DAY)
        .await?;
    tx.commit().await?;

    let target = pool_target(weekly);
    let change = plan_pool(&bots, target);
    let mut tx = app.db.begin(None).await?;
    for profile_id in &change.retire {
        tx.bots_set_retired(profile_id, Some(now)).await?;
    }
    for profile_id in &change.reinstate {
        tx.bots_set_retired(profile_id, None).await?;
    }
    let mut named: Vec<(String, String)> = Vec::new();
    for index in 0..change.create {
        let (bot, name) = draw_new_bot(&format!("{seed}:{index}"), &new_uuid(), now, &mut |name| {
            bot_name_filter(name)
        });
        tx.bots_create(&bot).await?;
        if let Some(name) = name {
            named.push((bot.profile_id.clone(), name));
        }
    }
    tx.commit().await?;
    for (profile_id, name) in &named {
        claim_name(app, profile_id, name, now).await?;
    }

    let mut tx = app.db.begin(None).await?;
    let bots = tx.bots_list().await?;
    let snapshot = app.catalog.snapshot();
    // The deals read the registered catalog (R388).
    jackioh_cards::register_all();
    let stale: Vec<BotRow> = bots
        .into_iter()
        .filter(|bot| bot.retired_at.is_none())
        // R1705001: a bot dealt decks of another size than `DECK_SIZE` is dealt again.
        .filter(|bot| !holds_decks(bot, &app.catalog.version))
        .collect();
    let lists = if stale.is_empty() {
        Vec::new()
    } else {
        tx.stats_decklists(&app.catalog.version).await?
    };
    tx.commit().await?;
    for bot in &stale {
        let decks = draw_decks(
            &format!("{seed}:{}", bot.profile_id),
            &lists,
            &snapshot,
            bot_target_strength(bot.strength),
        );
        let mut tx = app.db.begin(None).await?;
        tx.bots_set_decks(&bot.profile_id, &decks, &app.catalog.version)
            .await?;
        tx.commit().await?;
    }

    // R641000: after the decks, since a trio starts from them.
    let mut tx = app.db.begin(None).await?;
    let bots = tx.bots_list().await?;
    tx.commit().await?;
    let mut trios = 0;
    for bot in bots.iter().filter(|bot| bot.retired_at.is_none()) {
        if holds_trio(bot, &app.catalog.version) {
            continue;
        }
        let Some(trio) = draw_trio(
            &format!("{seed}:{}", bot.profile_id),
            &bot.decks,
            &snapshot,
            bot_target_strength(bot.strength),
        ) else {
            tracing::warn!(event = "bots.pool.trio_short", botId = %bot.profile_id);
            continue;
        };
        let mut tx = app.db.begin(None).await?;
        tx.bots_set_trio(&bot.profile_id, &trio, &app.catalog.version)
            .await?;
        tx.commit().await?;
        trios += 1;
    }

    // R1834005: after the decks and the trio, since a deck's slots and a card's picks follow them.
    let mut tx = app.db.begin(None).await?;
    let bots = tx.bots_list().await?;
    let owners = tx.economy_cosmetic_owners().await?;
    tx.commit().await?;
    let registry = crate::cosmetics::registry();
    let eligible = cosmetics::eligible(registry, &owners, now);
    let mut dressed = 0;
    for bot in bots.iter().filter(|bot| bot.retired_at.is_none()) {
        let Some(draw) = cosmetics::redraw(
            &format!("{seed}:{}", bot.profile_id),
            bot,
            BOT_COSMETIC_FITS,
            registry,
            &eligible,
            now,
        ) else {
            continue;
        };
        let mut tx = app.db.begin(None).await?;
        tx.bots_set_cosmetics(&bot.profile_id, &draw.cosmetics).await?;
        if let Some(decks) = &draw.decks {
            let version = bot
                .decks_catalog_version
                .as_deref()
                .unwrap_or(&app.catalog.version);
            tx.bots_set_decks(&bot.profile_id, decks, version).await?;
        }
        if let Some(trio) = &draw.trio {
            let version = bot
                .trio_catalog_version
                .as_deref()
                .unwrap_or(&app.catalog.version);
            tx.bots_set_trio(&bot.profile_id, trio, version).await?;
        }
        tx.commit().await?;
        dressed += 1;
    }

    // R1555000: after the pool is sized, so a retired bot stops and a new one may start.
    let mut tx = app.db.begin(None).await?;
    let bots = tx.bots_list().await?;
    let season_id = build_season_id(&SeasonDeps::current());
    let mut grinders: Vec<String> = Vec::new();
    // R2778000: each ladder's grind planned on its own people and ratings; a bot either names grinds.
    for ladder in Ladder::ALL {
        let standings = tx.ranked_standings(&season_id, ladder).await?;
        let people: Vec<f64> = standings
            .iter()
            .filter(|standing| {
                crate::api::ranked::counts(standing, app.env.dev_strict) && standing.rank.ladder.is_some()
            })
            .map(|standing| standing.rating)
            .collect();
        for bot in plan_grind(&bots, &people, ladder.into()) {
            if !grinders.contains(&bot) {
                grinders.push(bot);
            }
        }
    }
    tx.commit().await?;
    let mut tx = app.db.begin(None).await?;
    for bot in &bots {
        let grinding = grinders.contains(&bot.profile_id);
        if bot.grinding != grinding {
            tx.bots_set_grinding(&bot.profile_id, grinding).await?;
        }
    }
    tx.commit().await?;
    Ok(PoolRefresh {
        target,
        retired: change.retire.len(),
        reinstated: change.reinstate.len(),
        created: change.create,
        dealt: stale.len(),
        trios,
        grinding: grinders.len(),
        dressed,
    })
}

/// R1454: refreshes the pool at boot and every `BOT_POOL_REFRESH_INTERVAL_SECONDS`; a failed one is
/// logged and the next tries again. `app.rs` spawns it only when `LADDER_BOTS=on`.
pub async fn run_bot_pool(app: Arc<App>) {
    loop {
        match refresh_pool(&app, &new_seed(), now_ms()).await {
            Ok(done) => tracing::info!(
                event = "bots.pool.refreshed",
                target = done.target,
                retired = done.retired,
                reinstated = done.reinstated,
                created = done.created,
                dealt = done.dealt,
                trios = done.trios,
                grinding = done.grinding,
                dressed = done.dressed,
            ),
            Err(error) => tracing::warn!(event = "bots.pool.failed", message = %error),
        }
        tokio::time::sleep(Duration::from_secs(
            u64::try_from(BOT_POOL_REFRESH_INTERVAL_SECONDS).unwrap_or(0),
        ))
        .await;
    }
}
