//! Writing an economy outcome (SPEC §14.11, #3104): the store's half of every decision `jackioh_shared::economy`
//! makes. A decision answers the changes to make as [`LedgerGrant`]s, in order; this module writes each as the
//! holding's new value beside its ledger row, in the caller's transaction, and decides nothing. It also reads
//! the account's counter rows into the plain [`AccountCounters`] a decision takes.

use indexmap::IndexMap;
use jackioh_shared::economy::ledger::{AccountCounters, Cause, Counter, LedgerGrant, QuestChange};
use jackioh_shared::economy::math::PityCounters;

use crate::api::http::ApiError;
use crate::db::store::{CollectionEntry, CollectionGrant, StoreError, Tx};
use crate::economy::store::{
    CosmeticGrant, CounterEntry, CounterGrant, EconomyReason, FleetGrant, InventoryGrant, JloreGrant,
    JloreReason, QuestGrant, TrackGrant,
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
        LedgerGrant::Fleet { xp, levels, .. } => {
            format!("the Fleet track refused {xp} XP and {levels} levels")
        }
        LedgerGrant::FleetReward { cosmetic_id, .. } => {
            format!("the Fleet track's reward {cosmetic_id} was refused")
        }
        LedgerGrant::Quest { quest_id, change, .. } => format!("the quest {quest_id} refused {change:?}"),
        LedgerGrant::QuestDue { .. } => "the quest schedule refused a date".to_string(),
    }
}

/// Writes `grants` in order in the caller's transaction: an inventory grant to the inventory, cards to the
/// collection (absolute quantities, then their ledger's deltas), a cosmetic to the cosmetics, a counter to its
/// counter, Jlore to the wallet, XP and levels to a set track or the Fleet track and a Fleet level's reward to the
/// cosmetics (R3335001), a quest's change to the quests and the date of the next one to its schedule. A grant carries its own `ref` or takes the outcome's, and its own date or the
/// outcome's. A holding that cannot take a grant is `refused`'s error, writing nothing more; a wallet or a track
/// that takes none, a quest that arrives held already and a Fleet reward the account owns already (a dev account's
/// fill, R3335002) are no refusals. Answers the count the
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
            LedgerGrant::Fleet {
                xp,
                levels,
                cause,
                reference,
            } => {
                let written = tx
                    .economy_fleet_grant(&FleetGrant {
                        profile_id: profile_id.clone(),
                        xp: *xp,
                        levels: *levels,
                        reason: reason_of(*cause),
                        reference: reference.clone().or_else(|| writing.reference.clone()),
                        dev: writing.dev,
                        at: writing.at,
                    })
                    .await?;
                if !written {
                    return Err(refused(grant));
                }
            }
            LedgerGrant::FleetReward {
                cosmetic_id,
                reference,
            } => {
                // R3335002: earned only by the track, its reason `track`; one owned already stays owned.
                tx.economy_cosmetics_grant(&CosmeticGrant {
                    profile_id: profile_id.clone(),
                    card_id: None,
                    cosmetic_id: cosmetic_id.clone(),
                    delta: 1,
                    reason: EconomyReason::Track,
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

/// R3212000: [`write_grants`] for several outcomes of one account at once (an Open all's openings, in
/// order), in a fixed number of statements whatever their number: every inventory grant in one, the
/// cards' quantities (each card's last) and their ledger rows in two, the cosmetics in one and the
/// counters in one. Every row is the one `write_grants` writes for its outcome, in the same order in
/// its ledger. Only a card pack's kinds are batched: any other grant is `refused`'s error, writing
/// nothing. A holding that cannot take its grants writes none of them, and is `refused`'s error for its
/// first grant that takes something away (else its first), as one at a time would refuse it first.
pub async fn write_grants_all<E: From<StoreError>>(
    tx: &mut Tx<'_>,
    outcomes: &[(Writing<'_>, &[LedgerGrant])],
    refused: impl Fn(&LedgerGrant) -> E,
) -> Result<(), E> {
    let mut inventory = Vec::new();
    let mut inventory_grants = Vec::new();
    let mut quantities: IndexMap<String, i64> = IndexMap::new();
    let mut ledger = Vec::new();
    let mut cosmetics = Vec::new();
    let mut cosmetic_grants = Vec::new();
    let mut counters = Vec::new();
    let mut counter_grants = Vec::new();
    let Some((first, _)) = outcomes.first() else {
        return Ok(());
    };
    let profile_id = first.profile_id;
    for (writing, grants) in outcomes {
        if writing.profile_id != profile_id {
            return Err(StoreError::Other("economy: a batch grant pays one account".to_string()).into());
        }
        for grant in *grants {
            match grant {
                LedgerGrant::Inventory {
                    item,
                    key,
                    delta,
                    cause,
                    reference,
                } => {
                    inventory.push(InventoryGrant {
                        profile_id: profile_id.to_string(),
                        item: *item,
                        key: key.clone(),
                        delta: *delta,
                        reason: reason_of(*cause),
                        reference: reference.clone().or_else(|| writing.reference.clone()),
                        dev: writing.dev,
                        at: writing.at,
                    });
                    inventory_grants.push(grant);
                }
                LedgerGrant::Cards { cards } => {
                    for card in cards {
                        quantities.insert(card.card_id.clone(), card.quantity);
                        ledger.push(CollectionGrant {
                            profile_id: profile_id.to_string(),
                            card_id: card.card_id.clone(),
                            delta: card.delta,
                            reason: PACK_GRANT_REASON.to_string(),
                            dev: writing.dev,
                            at: writing.at,
                        });
                    }
                }
                LedgerGrant::Cosmetic { card_id, cosmetic_id } => {
                    cosmetics.push(CosmeticGrant {
                        profile_id: profile_id.to_string(),
                        card_id: card_id.clone(),
                        cosmetic_id: cosmetic_id.clone(),
                        delta: 1,
                        reason: EconomyReason::PackOpen,
                        reference: writing.reference.clone(),
                        dev: writing.dev,
                        at: writing.at,
                    });
                    cosmetic_grants.push(grant);
                }
                LedgerGrant::Counter {
                    counter,
                    delta,
                    cause,
                } => {
                    counters.push(CounterGrant {
                        profile_id: profile_id.to_string(),
                        counter: *counter,
                        pack: String::new(),
                        delta: *delta,
                        reason: reason_of(*cause),
                        reference: writing.reference.clone(),
                        dev: writing.dev,
                        at: writing.at,
                    });
                    counter_grants.push(grant);
                }
                LedgerGrant::Jlore { .. }
                | LedgerGrant::Track { .. }
                | LedgerGrant::Fleet { .. }
                | LedgerGrant::FleetReward { .. }
                | LedgerGrant::Quest { .. }
                | LedgerGrant::QuestDue { .. } => return Err(refused(grant)),
            }
        }
    }
    // The error a refused holding answers: its first grant's that takes something away, else its
    // first grant's (a holding never refuses an empty batch).
    let refusal = |grants: &[&LedgerGrant]| -> E {
        grants
            .iter()
            .find(|grant| match grant {
                LedgerGrant::Inventory { delta, .. } | LedgerGrant::Counter { delta, .. } => *delta < 0,
                _ => false,
            })
            .or_else(|| grants.first())
            .map_or_else(
                || StoreError::Other("economy: a batch refused no grant".to_string()).into(),
                |grant| refused(grant),
            )
    };
    if !tx.economy_inventory_write_all(&inventory).await? {
        return Err(refusal(&inventory_grants));
    }
    if !quantities.is_empty() {
        let quantities: Vec<CollectionEntry> = quantities
            .into_iter()
            .map(|(card_id, quantity)| CollectionEntry { card_id, quantity })
            .collect();
        tx.collection_upsert_quantities(profile_id, &quantities).await?;
        tx.collection_append_grants(&ledger).await?;
    }
    if !tx.economy_cosmetics_grant_all(&cosmetics).await? {
        return Err(refusal(&cosmetic_grants));
    }
    if !tx.economy_counters_write_all(&counters).await? {
        return Err(refusal(&counter_grants));
    }
    Ok(())
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
