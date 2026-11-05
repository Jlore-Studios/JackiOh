//! Writing an economy outcome (SPEC §14.11, #3104): the store's half of every decision `jackioh_shared::economy`
//! makes. A decision answers the changes to make as [`LedgerGrant`]s, in order; this module writes each as the
//! holding's new value beside its ledger row, in the caller's transaction, and decides nothing. It also reads
//! the account's counter rows into the plain [`AccountCounters`] a decision takes.

use jackioh_shared::economy::ledger::{AccountCounters, Cause, Counter, LedgerGrant, QuestChange};
use jackioh_shared::economy::math::PityCounters;

use crate::api::http::ApiError;
use crate::db::store::{CollectionEntry, CollectionGrant, StoreError, Tx};
use crate::economy::store::{
    CosmeticGrant, CounterEntry, CounterGrant, EconomyReason, InventoryGrant, JloreGrant, JloreReason,
    QuestGrant, TrackGrant,
};

/// The collection ledger's reason for a card a pack unlocks: migration 0002's `pack`, one of its
/// closed set, as `api::collection::LAUNCH_GRANT_REASON` is its `launch`.
pub const PACK_GRANT_REASON: &str = "pack";

/// The account's counters from its rows (§14.4–§14.6): each account-wide counter's value, 0 for one never
/// written; a pity counter is kept per account (R1298000), never per kind of pack.
pub fn counters_of(rows: &[CounterEntry]) -> AccountCounters {
    let value = |counter: Counter| {
        rows.iter()
            .find(|entry| entry.counter == counter && entry.pack.is_empty())
            .map_or(0, |entry| entry.value)
    };
    let pity = |counter: Counter| usize::try_from(value(counter)).unwrap_or_default();
    AccountCounters {
        pity: PityCounters {
            epic: pity(Counter::PityEpic),
            legendary: pity(Counter::PityLegendary),
            mythic: pity(Counter::PityMythic),
        },
        card_bar: value(Counter::CardTokenBar),
        cosmetic_bar: value(Counter::CosmeticTokenBar),
        cosmetic_cycle: value(Counter::CosmeticTokenCycle),
        chase_shard: value(Counter::ChaseShardBar),
    }
}

/// The ledger reason a grant's cause writes.
fn reason_of(cause: Cause) -> EconomyReason {
    match cause {
        Cause::PackOpen => EconomyReason::PackOpen,
        Cause::TokenBar => EconomyReason::TokenBar,
        Cause::Dev => EconomyReason::Dev,
        Cause::Game => EconomyReason::Game,
        Cause::Quest => EconomyReason::Quest,
        Cause::Track => EconomyReason::Track,
    }
}

/// The Jlore ledger reason a grant's cause writes: only a game, a quest, a track level and a dev tool pay Jlore.
fn jlore_reason_of(cause: Cause) -> Option<JloreReason> {
    match cause {
        Cause::Game => Some(JloreReason::Game),
        Cause::Quest => Some(JloreReason::Quest),
        Cause::Track => Some(JloreReason::Track),
        Cause::Dev => Some(JloreReason::Dev),
        Cause::PackOpen | Cause::TokenBar => None,
    }
}

/// What every row of one outcome shares: the account, the `ref` its economy rows carry (the opening's log),
/// whether a dev tool wrote it (§14.10) and the stamp.
pub struct Writing<'a> {
    pub profile_id: &'a str,
    pub reference: &'a Option<String>,
    pub dev: bool,
    pub at: i64,
}

/// The error for a grant the store's holding could not take, when its caller has no words of its own: a
/// fault of the server, not of the player.
pub fn internal_refusal(grant: &LedgerGrant) -> ApiError {
    ApiError::internal(refusal_text(grant))
}

/// The error for a grant the store refused, as a store error: where a payout runs in a store transaction and
/// answers a [`StoreError`].
pub fn store_refusal(grant: &LedgerGrant) -> StoreError {
    StoreError::Other(format!("economy: {}", refusal_text(grant)))
}

/// The words for a grant a holding refused, or whose cause names no reason its ledger takes.
fn refusal_text(grant: &LedgerGrant) -> String {
    match grant {
        LedgerGrant::Inventory { item, delta, .. } => format!("the inventory refused {delta} of {item:?}"),
        LedgerGrant::Cards { .. } => "the collection refused an opening's cards".to_string(),
        LedgerGrant::Cosmetic { card_id, cosmetic_id } => {
            format!("{cosmetic_id} was already owned on {card_id:?}")
        }
        LedgerGrant::Counter { counter, delta, .. } => format!("the counter {counter:?} refused {delta}"),
        LedgerGrant::Jlore { delta, cause, .. } => format!("the wallet refused {delta} Jlore for {cause:?}"),
        LedgerGrant::Track { set_id, .. } => format!("the {set_id} track refused a grant"),
        LedgerGrant::Quest { quest_id, change, .. } => format!("the quest {quest_id} refused {change:?}"),
        LedgerGrant::QuestDue { .. } => "the quest schedule refused a date".to_string(),
    }
}

/// Writes `grants` in order in the caller's transaction: an inventory grant to the inventory, cards to the
/// collection (absolute quantities, then their ledger's deltas), a cosmetic to the cosmetics, a counter to its
/// counter, Jlore to the wallet, XP and levels to a set track, a quest's change to the quests and the date of
/// the next one to its schedule. A grant carries its own `ref` or takes the outcome's, and its own date or the
/// outcome's. A holding that cannot take a grant is `refused`'s error, writing nothing more; a wallet or a track
/// that takes none, and a quest that arrives held already, are no refusals, as before. Answers the count the
/// first inventory grant left, which an opening reads as the packs still held.
pub async fn write_grants<E: From<StoreError>>(
    tx: &mut Tx<'_>,
    writing: &Writing<'_>,
    grants: &[LedgerGrant],
    refused: impl Fn(&LedgerGrant) -> E,
) -> Result<Option<i64>, E> {
    let profile_id = writing.profile_id.to_string();
    let mut first_count = None;
    for grant in grants {
        match grant {
            LedgerGrant::Inventory {
                item,
                key,
                delta,
                cause,
                reference,
            } => {
                let count = tx
                    .economy_inventory_grant(&InventoryGrant {
                        profile_id: profile_id.clone(),
                        item: *item,
                        key: key.clone(),
                        delta: *delta,
                        reason: reason_of(*cause),
                        reference: reference.clone().or_else(|| writing.reference.clone()),
                        dev: writing.dev,
                        at: writing.at,
                    })
                    .await?
                    .ok_or_else(|| refused(grant))?;
                first_count.get_or_insert(count);
            }
            LedgerGrant::Cards { cards } => {
                let quantities: Vec<CollectionEntry> = cards
                    .iter()
                    .map(|card| CollectionEntry {
                        card_id: card.card_id.clone(),
                        quantity: card.quantity,
                    })
                    .collect();
                let ledger: Vec<CollectionGrant> = cards
                    .iter()
                    .map(|card| CollectionGrant {
                        profile_id: profile_id.clone(),
                        card_id: card.card_id.clone(),
                        delta: card.delta,
                        reason: PACK_GRANT_REASON.to_string(),
                        dev: writing.dev,
                        at: writing.at,
                    })
                    .collect();
                tx.collection_upsert_quantities(writing.profile_id, &quantities)
                    .await?;
                tx.collection_append_grants(&ledger).await?;
            }
            LedgerGrant::Cosmetic { card_id, cosmetic_id } => {
                let owned = tx
                    .economy_cosmetics_grant(&CosmeticGrant {
                        profile_id: profile_id.clone(),
                        card_id: card_id.clone(),
                        cosmetic_id: cosmetic_id.clone(),
                        delta: 1,
                        reason: EconomyReason::PackOpen,
                        reference: writing.reference.clone(),
                        dev: writing.dev,
                        at: writing.at,
                    })
                    .await?;
                if !owned {
                    return Err(refused(grant));
                }
            }
            LedgerGrant::Counter {
                counter,
                delta,
                cause,
            } => {
                tx.economy_counters_grant(&CounterGrant {
                    profile_id: profile_id.clone(),
                    counter: *counter,
                    pack: String::new(),
                    delta: *delta,
                    reason: reason_of(*cause),
                    reference: writing.reference.clone(),
                    dev: writing.dev,
                    at: writing.at,
                })
                .await?
                .ok_or_else(|| refused(grant))?;
            }
            LedgerGrant::Jlore {
                delta,
                cause,
                reference,
            } => {
                let reason = jlore_reason_of(*cause).ok_or_else(|| refused(grant))?;
                tx.economy_jlore_grant(&JloreGrant {
                    profile_id: profile_id.clone(),
                    delta: *delta,
                    reason,
                    reference: reference.clone().or_else(|| writing.reference.clone()),
                    dev: writing.dev,
                    at: writing.at,
                })
                .await?;
            }
            LedgerGrant::Track {
                set_id,
                xp,
                levels,
                rewards,
                cause,
                reference,
            } => {
                tx.economy_tracks_grant(&TrackGrant {
                    profile_id: profile_id.clone(),
                    set_id: set_id.clone(),
                    xp: *xp,
                    levels: *levels,
                    rewards: *rewards,
                    reason: reason_of(*cause),
                    reference: reference.clone().or_else(|| writing.reference.clone()),
                    dev: writing.dev,
                    at: writing.at,
                })
                .await?;
            }
            LedgerGrant::Quest {
                quest_id,
                change,
                progress,
                cause,
                reference,
                at,
            } => {
                let changed = tx
                    .economy_quests_change(&QuestGrant {
                        profile_id: profile_id.clone(),
                        quest_id: quest_id.clone(),
                        change: *change,
                        progress: *progress,
                        reason: reason_of(*cause),
                        reference: reference.clone().or_else(|| writing.reference.clone()),
                        dev: writing.dev,
                        at: *at,
                    })
                    .await?;
                // An arrival the account holds already is no refusal; any other change it cannot make is.
                if !changed && *change != QuestChange::Arrived {
                    return Err(refused(grant));
                }
            }
            LedgerGrant::QuestDue { next_due_at } => {
                tx.economy_quests_set_next_due(writing.profile_id, *next_due_at)
                    .await?;
            }
        }
    }
    Ok(first_count)
}

/// Adds `delta` to one of the account's counters for a spend the server decides (a Chase Shard bar bought
/// empty, R1306002); a zero delta writes nothing, since a ledger row records a change.
pub async fn grant_counter(
    tx: &mut Tx<'_>,
    profile_id: &str,
    counter: Counter,
    delta: i64,
    reason: EconomyReason,
    at: i64,
) -> Result<(), ApiError> {
    if delta == 0 {
        return Ok(());
    }
    tx.economy_counters_grant(&CounterGrant {
        profile_id: profile_id.to_string(),
        counter,
        pack: String::new(),
        delta,
        reason,
        reference: None,
        dev: false,
        at,
    })
    .await?
    .ok_or_else(|| ApiError::internal(format!("the counter {counter:?} refused {delta}")))?;
    Ok(())
}
