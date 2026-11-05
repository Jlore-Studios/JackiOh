//! Writing an economy outcome (SPEC §14.11, #3104): the store's half of every decision `jackioh_shared::economy`
//! makes. A decision answers the changes to make as [`LedgerGrant`]s, in order; this module writes each as the
//! holding's new value beside its ledger row, in the caller's transaction, and decides nothing. It also reads
//! the account's counter rows into the plain [`AccountCounters`] a decision takes.

use jackioh_shared::economy::ledger::{AccountCounters, Cause, Counter, LedgerGrant};
use jackioh_shared::economy::math::PityCounters;

use crate::api::http::ApiError;
use crate::db::store::{CollectionEntry, CollectionGrant, Tx};
use crate::economy::store::{CosmeticGrant, CounterEntry, CounterGrant, EconomyReason, InventoryGrant};

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
    ApiError::internal(match grant {
        LedgerGrant::Inventory { item, delta, .. } => format!("the inventory refused {delta} of {item:?}"),
        LedgerGrant::Cards { .. } => "the collection refused an opening's cards".to_string(),
        LedgerGrant::Cosmetic { card_id, cosmetic_id } => {
            format!("{cosmetic_id} was already owned on {card_id:?}")
        }
        LedgerGrant::Counter { counter, delta, .. } => format!("the counter {counter:?} refused {delta}"),
    })
}

/// Writes `grants` in order in the caller's transaction: an inventory grant to the inventory, cards to the
/// collection (absolute quantities, then their ledger's deltas), a cosmetic to the cosmetics, a counter to its
/// counter. A holding that cannot take a grant is `refused`'s error, writing nothing more. Answers the count
/// the first inventory grant left, which an opening reads as the packs still held.
pub async fn write_grants(
    tx: &mut Tx<'_>,
    writing: &Writing<'_>,
    grants: &[LedgerGrant],
    refused: impl Fn(&LedgerGrant) -> ApiError,
) -> Result<Option<i64>, ApiError> {
    let profile_id = writing.profile_id.to_string();
    let mut first_count = None;
    for grant in grants {
        match grant {
            LedgerGrant::Inventory {
                item,
                key,
                delta,
                cause,
            } => {
                let count = tx
                    .economy_inventory_grant(&InventoryGrant {
                        profile_id: profile_id.clone(),
                        item: *item,
                        key: key.clone(),
                        delta: *delta,
                        reason: reason_of(*cause),
                        reference: writing.reference.clone(),
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
