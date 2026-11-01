//! Admin script: what people wear, by account age, as rows to paste into config (SPEC §14.2,
//! R1834006), as `timing-fit` prints the think times a ladder bot copies (R1442).
//!
//! ```text
//! jackioh-server cosmetics-fit
//! ```
//!
//! Reads the loadouts people's seats froze over the last `BOT_COSMETIC_FIT_WINDOW_DAYS`
//! (`matches.loadouts`, migration 0061; a ladder bot's seat is never one) and what each of those
//! people owns now (the ledger), and buckets them by account age (`BOT_COSMETIC_AGE_BUCKETS_DAYS`):
//! a seat by its account's age as its match began, a person by their age at their latest match.
//! Each bucket of at least `BOT_COSMETIC_FIT_MIN_SAMPLES` seats prints one `CosmeticFit`, to be
//! pasted into `BOT_COSMETIC_FITS` in `crates/server/src/config.rs` by hand (CLAUDE.md rule 9), and
//! a bucket under it prints a comment, so its row keeps the prior. Ownership is read now, so a full
//! art or an earned foil, finish or border granted after a match counts as not worn there;
//! `full_art_wear` is the share that printed that one of its card's tiers (R2655000).
//!
//! The output is aggregates only: no account id or username.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::bail;

use crate::bots::cosmetics::age_bucket;
use crate::cli::{connect_one, database_url};
use crate::config::{
    BOT_COSMETIC_AGE_BUCKETS_DAYS, BOT_COSMETIC_FIT_MIN_SAMPLES, BOT_COSMETIC_FIT_WINDOW_DAYS,
};
use crate::cosmetics::{Cosmetic, CosmeticKind, full_art_of, full_art_picked, registry};
use crate::db::store::{Db, LoadoutSample, StoreError};
use crate::economy::store::OwnedCosmetics;
use crate::env::quoted;
use crate::loadout::{CARD_SLOT_KINDS, DECK_SLOT_KINDS};

const USAGE: &str = "Usage: jackioh-server cosmetics-fit

  Prints what people wear by account age (R1834006): one CosmeticFit row per age bucket, from the
  loadouts people's seats froze over the last BOT_COSMETIC_FIT_WINDOW_DAYS and what they own, to
  paste into BOT_COSMETIC_FITS in crates/server/src/config.rs. Takes no options; reads
  DATABASE_URL.";

const HEADER: &str =
    "// jackioh-server cosmetics-fit: paste into BOT_COSMETIC_FITS, crates/server/src/config.rs";

const MS_PER_DAY: i64 = 86_400_000;

/// One bucket's fit (`config::CosmeticFit`'s fields, as values).
#[derive(Clone, Debug, PartialEq)]
pub struct BucketCosmetics {
    pub max_age_days: Option<i64>,
    pub samples: usize,
    pub slots: Vec<(String, f64)>,
    pub per_card_share: f64,
    pub cards_per_deck_mean: f64,
    pub card_picks: Vec<(String, f64)>,
    /// Each earned foil, finish and border: the cosmetic, its card, and its wear there.
    pub earned_wear: Vec<(String, String, f64)>,
    pub full_art_wear: Vec<(String, f64)>,
    /// Each holding: the cosmetic, its card for a per-card one, and its share.
    pub ownership: Vec<(String, Option<String>, f64)>,
}

fn buckets() -> Vec<Option<i64>> {
    BOT_COSMETIC_AGE_BUCKETS_DAYS
        .iter()
        .map(|bound| Some(*bound))
        .chain([None])
        .collect()
}

fn share(count: usize, of: usize) -> f64 {
    if of == 0 { 0.0 } else { count as f64 / of as f64 }
}

/// Whether a card's entry picks something of its own on the card (its full art aside).
fn picks_on_the_card(picks: &crate::economy::store::CardPicks) -> bool {
    CARD_SLOT_KINDS.iter().any(|kind| picks.get(*kind).is_some())
}

/// R1834006: what people wear, one row per age bucket that holds a seat, in bucket order (the
/// youngest first, the one past every bound last), from people's `samples` and what each owns
/// (`owned`, by profile id; a person missing from it owns nothing). `registry` orders the shares
/// and names the non-starters. A bot's seat is never a sample (`matches_loadout_samples`).
pub fn fit_cosmetics(
    registry: &[Cosmetic],
    samples: &[LoadoutSample],
    owned: &BTreeMap<String, OwnedCosmetics>,
) -> Vec<BucketCosmetics> {
    let none = OwnedCosmetics::default();
    let owned_by = |profile_id: &str| owned.get(profile_id).unwrap_or(&none);
    let bucket_of = |sample: &LoadoutSample| {
        age_bucket((sample.match_created_at - sample.account_created_at) / MS_PER_DAY)
    };
    // Each person's bucket: their age at their latest match (the samples come oldest first).
    let mut people: BTreeMap<&str, Option<i64>> = BTreeMap::new();
    for sample in samples {
        people.insert(sample.profile_id.as_str(), bucket_of(sample));
    }

    let mut rows = Vec::new();
    for bucket in buckets() {
        let seats: Vec<&LoadoutSample> = samples
            .iter()
            .filter(|sample| bucket_of(sample) == bucket)
            .collect();
        if seats.is_empty() {
            continue;
        }
        // Each cosmetic's share of its slot, in the registry's order.
        let mut slots = Vec::new();
        for cosmetic in registry {
            let kind = cosmetic.kind;
            if !CARD_SLOT_KINDS.contains(&kind) && !DECK_SLOT_KINDS.contains(&kind) {
                continue;
            }
            let wearing = seats
                .iter()
                .filter(|sample| sample.loadout.seat.get(kind) == cosmetic.id)
                .count();
            if wearing > 0 {
                slots.push((cosmetic.id.clone(), share(wearing, seats.len())));
            }
        }
        // The seats with a card of their own picks, and how many cards each holds.
        let picked: Vec<usize> = seats
            .iter()
            .map(|sample| {
                sample
                    .loadout
                    .cards
                    .iter()
                    .filter(|picks| picks_on_the_card(picks))
                    .count()
            })
            .filter(|cards| *cards > 0)
            .collect();
        let cards_per_deck_mean = if picked.is_empty() {
            0.0
        } else {
            picked.iter().sum::<usize>() as f64 / picked.len() as f64
        };
        // Each default's and starter's foil, finish and border's share of the seats' picks on their
        // cards of one, in the registry's order. An earned one is owned on one card only, so its
        // share of the picks pooled over every card would hold on none: it is `earned_wear`'s.
        let every_card = |cosmetic: &Cosmetic| {
            CARD_SLOT_KINDS.contains(&cosmetic.kind) && (cosmetic.default || cosmetic.starter)
        };
        let mut on_cards: BTreeMap<&str, usize> = BTreeMap::new();
        for picks in seats.iter().flat_map(|sample| &sample.loadout.cards) {
            for kind in CARD_SLOT_KINDS {
                if let Some(id) = picks.get(kind) {
                    *on_cards.entry(id).or_default() += 1;
                }
            }
        }
        let card_picks_total: usize = registry
            .iter()
            .filter(|cosmetic| every_card(cosmetic))
            .filter_map(|cosmetic| on_cards.get(cosmetic.id.as_str()))
            .sum();
        let card_picks = registry
            .iter()
            .filter(|cosmetic| every_card(cosmetic))
            .filter_map(|cosmetic| {
                let count = on_cards.get(cosmetic.id.as_str()).copied()?;
                Some((cosmetic.id.clone(), share(count, card_picks_total)))
            })
            .collect();
        // Each earned foil, finish and border on each card a seat's person owns it on: of the seats
        // that own it there and play the card, the share whose card wears it, in the registry's
        // order, then by card.
        let mut earned_wear = Vec::new();
        for earned in registry.iter().filter(|cosmetic| {
            CARD_SLOT_KINDS.contains(&cosmetic.kind) && !cosmetic.default && !cosmetic.starter
        }) {
            let on: BTreeSet<&str> = seats
                .iter()
                .flat_map(|sample| &owned_by(&sample.profile_id).cards)
                .filter(|row| row.cosmetic_id == earned.id)
                .map(|row| row.card_id.as_str())
                .collect();
            for card in on {
                let could: Vec<&&LoadoutSample> = seats
                    .iter()
                    .filter(|sample| sample.deck.iter().any(|id| id == card))
                    .filter(|sample| {
                        owned_by(&sample.profile_id)
                            .cards
                            .iter()
                            .any(|row| row.card_id == card && row.cosmetic_id == earned.id)
                    })
                    .collect();
                if could.is_empty() {
                    continue;
                }
                let worn = could
                    .iter()
                    .filter(|sample| {
                        sample.loadout.card(card).and_then(|picks| picks.get(earned.kind))
                            == Some(earned.id.as_str())
                    })
                    .count();
                earned_wear.push((earned.id.clone(), card.to_string(), share(worn, could.len())));
            }
        }
        // Each full art: of the seats that own it and play its card, the share that printed it.
        let mut full_art_wear = Vec::new();
        for full_art in registry
            .iter()
            .filter(|cosmetic| cosmetic.kind == CosmeticKind::FullArt)
        {
            let Some(card) = full_art.card.as_deref() else {
                continue;
            };
            let could: Vec<&&LoadoutSample> = seats
                .iter()
                .filter(|sample| sample.deck.iter().any(|id| id == card))
                .filter(|sample| {
                    owned_by(&sample.profile_id)
                        .cards
                        .iter()
                        .any(|row| row.card_id == card && row.cosmetic_id == full_art.id)
                })
                .collect();
            if could.is_empty() {
                continue;
            }
            let printed = could
                .iter()
                .filter(|sample| {
                    sample
                        .loadout
                        .card(card)
                        .and_then(|picks| picks.full_art.as_ref())
                        .and_then(|pick| full_art_picked(registry, card, pick))
                        .is_some_and(|worn| worn.id == full_art.id)
                })
                .count();
            full_art_wear.push((full_art.id.clone(), share(printed, could.len())));
        }
        // Each non-starter's share of the bucket's people, where they hold it.
        let mut holders: BTreeMap<(String, Option<String>), usize> = BTreeMap::new();
        let bucket_people: Vec<&str> = people
            .iter()
            .filter(|(_, at)| **at == bucket)
            .map(|(profile_id, _)| *profile_id)
            .collect();
        for profile_id in &bucket_people {
            let held = owned_by(profile_id);
            let places: BTreeSet<(String, Option<String>)> = held
                .cards
                .iter()
                .map(|row| (row.cosmetic_id.clone(), Some(row.card_id.clone())))
                .chain(held.account.iter().map(|id| (id.clone(), None)))
                .filter(|(id, _)| {
                    registry.iter().any(|cosmetic| {
                        cosmetic.id == *id
                            && !cosmetic.default
                            && !cosmetic.starter
                            && cosmetic.kind != CosmeticKind::Portrait
                    })
                })
                .collect();
            for place in places {
                *holders.entry(place).or_default() += 1;
            }
        }
        let ownership = holders
            .into_iter()
            .map(|((cosmetic, card), count)| (cosmetic, card, share(count, bucket_people.len())))
            .collect();
        rows.push(BucketCosmetics {
            max_age_days: bucket,
            samples: seats.len(),
            slots,
            per_card_share: share(picked.len(), seats.len()),
            cards_per_deck_mean,
            card_picks,
            earned_wear,
            full_art_wear,
            ownership,
        });
    }
    // A full art the registry knows of no card (never shipped so) names nothing: `full_art_of`
    // keeps the two in step.
    debug_assert!(rows.iter().all(|row| row.full_art_wear.iter().all(|(id, _)| {
        registry.iter().any(|cosmetic| {
            cosmetic.id == *id
                && cosmetic
                    .card
                    .as_deref()
                    .and_then(|card| full_art_of(registry, card))
                    .is_some()
        })
    })));
    rows
}

/// A bucket's words in the comments: "up to 7 days", "older than 90 days".
fn bucket_words(bucket: Option<i64>) -> String {
    match bucket {
        Some(bound) => format!("accounts up to {bound} days"),
        None => format!(
            "accounts older than {} days",
            BOT_COSMETIC_AGE_BUCKETS_DAYS.last().copied().unwrap_or(0)
        ),
    }
}

/// R1834006: the rows as `BOT_COSMETIC_FITS` holds them, after one comment line, four decimals to a
/// share; a bucket under `min_samples` seats prints a comment instead, so its row keeps the prior.
pub fn render_cosmetic_fits(rows: &[BucketCosmetics], min_samples: usize) -> String {
    let mut out = String::from(HEADER);
    out.push('\n');
    for row in rows {
        if row.samples < min_samples {
            out.push_str(&format!(
                "    // {}: {} seats, under BOT_COSMETIC_FIT_MIN_SAMPLES; keep the prior\n",
                bucket_words(row.max_age_days),
                row.samples
            ));
            continue;
        }
        let max_age = match row.max_age_days {
            Some(bound) => format!("Some({bound})"),
            None => "None".to_string(),
        };
        let pairs = |list: &[(String, f64)]| {
            list.iter()
                .map(|(id, share)| format!("({id:?}, {share:.4})"))
                .collect::<Vec<String>>()
                .join(", ")
        };
        let ownership = row
            .ownership
            .iter()
            .map(|(cosmetic, card, share)| {
                let card = match card {
                    Some(card) => format!("Some({card:?})"),
                    None => "None".to_string(),
                };
                format!("CosmeticOwnership {{ cosmetic: {cosmetic:?}, card: {card}, share: {share:.4} }}")
            })
            .collect::<Vec<String>>()
            .join(", ");
        let earned_wear = row
            .earned_wear
            .iter()
            .map(|(cosmetic, card, share)| {
                format!("CosmeticWear {{ cosmetic: {cosmetic:?}, card: {card:?}, share: {share:.4} }}")
            })
            .collect::<Vec<String>>()
            .join(", ");
        out.push_str(&format!(
            "    CosmeticFit {{ max_age_days: {max_age}, samples: {}, slots: &[{}], per_card_share: {:.4}, cards_per_deck_mean: {:.4}, card_picks: &[{}], earned_wear: &[{earned_wear}], full_art_wear: &[{}], ownership: &[{ownership}] }},\n",
            row.samples,
            pairs(&row.slots),
            row.per_card_share,
            row.cards_per_deck_mean,
            pairs(&row.card_picks),
            pairs(&row.full_art_wear),
        ));
    }
    out
}

/// What `cosmetics-fit` reads, in one transaction: people's frozen loadouts since `since`, and
/// what each of those people owns.
pub async fn read_samples(
    db: &Db,
    since: i64,
) -> Result<(Vec<LoadoutSample>, BTreeMap<String, OwnedCosmetics>), StoreError> {
    let mut tx = db.begin(None).await?;
    let samples = tx.matches_loadout_samples(since).await?;
    let mut owned = BTreeMap::new();
    for sample in &samples {
        if !owned.contains_key(&sample.profile_id) {
            let held = tx.economy_cosmetics_get(&sample.profile_id).await?;
            owned.insert(sample.profile_id.clone(), held);
        }
    }
    tx.commit().await?;
    Ok((samples, owned))
}

pub async fn run(args: Vec<String>) -> anyhow::Result<()> {
    if let Some(arg) = args.first() {
        bail!("Unrecognized argument {}.\n\n{USAGE}", quoted(arg));
    }
    let connection_string = database_url()?;
    let db = connect_one(&connection_string).await?;
    let since = crate::app::now_ms() - BOT_COSMETIC_FIT_WINDOW_DAYS * MS_PER_DAY;
    let read = read_samples(&db, since).await;
    db.close().await;
    let (samples, owned) = read?;
    let rows = fit_cosmetics(registry(), &samples, &owned);
    print!("{}", render_cosmetic_fits(&rows, BOT_COSMETIC_FIT_MIN_SAMPLES));
    if rows.iter().all(|row| row.samples < BOT_COSMETIC_FIT_MIN_SAMPLES) {
        eprintln!("cosmetics-fit: no age bucket holds {BOT_COSMETIC_FIT_MIN_SAMPLES} people's seats yet");
    }
    Ok(())
}
