//! Daily quests (SPEC §14.9, #1304): the store's half. The pool (`crates/shared/quests.json`), the rhythm
//! quests arrive on, what a finished game counts toward each quest and the decisions over an account's quests
//! (a top-up, a replace, a dev give) are `jackioh_shared::economy::quests` since #3104, re-exported here so
//! the server names them as it always has; this module reads an account's quests, calls the decision and
//! writes its grants in the caller's transaction.
//!
//! Nothing here checks `ECONOMY_LIVE` (§14.12): its callers top up and pay only while it is on
//! ([[R1304002]]). Every number is read through its [`crate::config`] constant (CLAUDE.md rule 9).

pub use jackioh_shared::economy::quests::{
    GameTally, PlayedCard, QUESTS_JSON, QuestDef, QuestGoal, QuestRefusal, TopUp, draw_quest, give_quest,
    parse, pool, problems, progress_of, quest_def, quest_interval_ms, quest_xp, seed_reference, shipped,
    tally_seat, top_up, top_up_book,
};

use jackioh_shared::economy::ledger::{Cause, LedgerGrant, QuestChange};
use jackioh_shared::economy::quests::{replace_quest as decide_replace, replaces_left as left_of};
use jackioh_shared::economy::rewards::{complete_quest as decide_complete, utc_day_bounds};

use super::grants::{Writing, store_refusal, write_grants};
use super::store::{EconomyReason, QuestBook};
use crate::api::crypto::SystemIds;
use crate::db::store::{StoreError, StoreResult, Tx};

/// A draw's seed, fresh from the operating system as every roll's is (§14.11), so no seed an account can read
/// foretells its next quest.
pub fn fresh_seed() -> String {
    SystemIds.seed()
}

/// [[R1304000]]: tops the account up at `now`, in the caller's transaction, and answers its
/// quests after. With nothing due it writes nothing. The store's read locks the account's
/// schedule to the end of the transaction, so a second top-up of the same account waits for this
/// one and then reads what it wrote: two at once bring each quest that fell due once.
pub async fn top_up_quests(tx: &mut Tx<'_>, profile_id: &str, now: i64) -> StoreResult<QuestBook> {
    let book = tx.economy_quests_get(profile_id).await?;
    let topped = top_up_book(&book, &fresh_seed(), now);
    if topped.grants.is_empty() {
        return Ok(book);
    }
    let writing = Writing {
        profile_id,
        reference: &None,
        dev: false,
        at: now,
    };
    write_grants(tx, &writing, &topped.grants, store_refusal).await?;
    tx.economy_quests_get(profile_id).await
}

/// Completes the held quest `def` and pays it, in the caller's transaction: its Jlore, and its XP
/// onto every active track (§14.7), each row naming the quest and marked `dev` when a dev tool
/// completed it (§14.10). False, paying nothing, when the account does not hold it: the store
/// lets a held quest go once, so a quest pays once.
pub async fn complete_quest(
    tx: &mut Tx<'_>,
    profile_id: &str,
    def: &QuestDef,
    reason: EconomyReason,
    reference: Option<&str>,
    dev: bool,
    at: i64,
) -> StoreResult<bool> {
    let book = tx.economy_quests_get(profile_id).await?;
    let tracks = tx.economy_tracks_get(profile_id).await?;
    let cause = match reason {
        EconomyReason::Dev => Cause::Dev,
        _ => Cause::Game,
    };
    let Some(grants) = decide_complete(&book, &tracks, &def.id, cause, reference, at) else {
        return Ok(false);
    };
    let writing = Writing {
        profile_id,
        reference: &None,
        dev,
        at,
    };
    write_grants(tx, &writing, &grants, store_refusal).await?;
    Ok(true)
}

/// How a replace ended ([[R2210000]]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplaceOutcome {
    /// The quest left and this one took its slot.
    Replaced(&'static QuestDef),
    /// The day's `QUEST_REPLACES_PER_DAY` replaces are already used.
    Spent,
    /// The account does not hold that quest.
    NotHeld,
    /// Every other entry of the pool is held, so there is none to swap in.
    NoneLeft,
}

/// [[R2210000]]: the replaces the account has left in the UTC day `now` falls in, never below 0:
/// `QUEST_REPLACES_PER_DAY` less the quests it replaced that day, which are its `removed` rows with
/// the reason `quest` (going live's `launch` removals, [[R1310002]], count for nothing). Read under
/// the account's economy lock, it is the count a replace spends from.
pub async fn replaces_left(tx: &mut Tx<'_>, profile_id: &str, now: i64) -> StoreResult<i64> {
    Ok(left_of(removed_today(tx, profile_id, now).await?))
}

async fn removed_today(tx: &mut Tx<'_>, profile_id: &str, now: i64) -> StoreResult<i64> {
    let (from, until) = utc_day_bounds(now);
    tx.economy_quests_removed_count(profile_id, from, until).await
}

/// [[R2210000]]: replaces the held quest `quest_id` at `now`, in the caller's transaction. The
/// account is locked first (R1301003), so a second replace at once waits and then reads what this one
/// wrote: at most `QUEST_REPLACES_PER_DAY` land in a UTC day. The decision tops the account up, then
/// removes the quest with its progress and draws one as an arrival is to take its slot; a refusal writes
/// nothing, and the caller drops the transaction.
pub async fn replace_quest(
    tx: &mut Tx<'_>,
    profile_id: &str,
    quest_id: &str,
    now: i64,
) -> StoreResult<ReplaceOutcome> {
    tx.economy_lock_account(profile_id).await?;
    let book = tx.economy_quests_get(profile_id).await?;
    let removed = removed_today(tx, profile_id, now).await?;
    let grants = match decide_replace(&book, removed, quest_id, &fresh_seed(), now) {
        Ok(grants) => grants,
        Err(QuestRefusal::Spent) => return Ok(ReplaceOutcome::Spent),
        Err(QuestRefusal::NotHeld) => return Ok(ReplaceOutcome::NotHeld),
        Err(QuestRefusal::NoneLeft | QuestRefusal::Full) => return Ok(ReplaceOutcome::NoneLeft),
    };
    let writing = Writing {
        profile_id,
        reference: &None,
        dev: false,
        at: now,
    };
    write_grants(tx, &writing, &grants, store_refusal).await?;
    // The arrival is the last write: the quest that took the slot.
    let arrived = grants.iter().rev().find_map(|grant| match grant {
        LedgerGrant::Quest {
            quest_id,
            change: QuestChange::Arrived,
            ..
        } => quest_def(quest_id),
        _ => None,
    });
    arrived.map(ReplaceOutcome::Replaced).ok_or_else(|| {
        StoreError::Other("economy: a replace drew a quest the pool does not hold".to_string())
    })
}
