//! The ladder bots' cosmetics (R1834005; #1834): a bot wears a loadout drawn as a person's, so its
//! seat never stands out (R1452), from a wardrobe drawn from the pool's seed, never a collection
//! (R1454: it owns nothing and earns nothing).
//!
//! A bot's **wardrobe** is what it may wear past the starters, in the ledger's shape
//! ([`OwnedCosmetics`]) but kept on its `bots` row (`bots.cosmetics`, migration 0061), never in the
//! ledger. It holds a non-starter only when people own it widely and long enough ([`eligible`]),
//! each eligible one held at the share of people of its account age that own it ([`fit_for`]).
//! Its seat freezes through `loadout::freeze_loadout` as what it owns (R1834002).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use jackioh_engine::Rng;

use crate::config::{
    BOT_COSMETIC_AGE_BUCKETS_DAYS, BOT_COSMETIC_FIT_MIN_SAMPLES, BOT_COSMETIC_MIN_AGE_DAYS,
    BOT_COSMETIC_MIN_OWNERS, BOT_COSMETIC_PRIOR, BOT_COSMETIC_PRIOR_DEFAULT_SHARE,
    BOT_COSMETIC_PRIOR_FULL_ART_WEAR, BOT_COSMETIC_PRIOR_OWNERSHIP, BOT_COSMETIC_REDRAW_DAYS,
    BOT_COSMETIC_SLOT_CHANGE_SHARE, CosmeticFit, MS_PER_DAY,
};
use crate::cosmetics::{Cosmetic, CosmeticKind, full_art_of, owns};
use crate::db::store::{BotDeck, BotRow, FrozenTrio};
use crate::economy::store::{CardCosmetic, CardPicks, CosmeticOwners, FullArtPick, OwnedCosmetics};
use crate::loadout::{AccountPicks, CARD_SLOT_KINDS, DECK_SLOT_KINDS, DeckSlots, setting_key};

/// R1834005: what a bot may wear and what it wears (`bots.cosmetics`, migration 0061): its
/// wardrobe, account picks, cards' picks, last draw timestamp, and wardrobe seed ([`draw_wardrobe`]).
/// Decks' slots ride on the decks.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BotCosmetics {
    pub wardrobe: OwnedCosmetics,
    pub account: AccountPicks,
    #[serde(default)]
    pub cards: Vec<CardPicks>,
    pub drawn_at: i64,
    #[serde(default)]
    pub seed: String,
}

/// One place a non-starter can be owned: a cosmetic, on a card for a per-card one.
pub type Holding = (String, Option<String>);

/// R1834005: the bound of the age bucket an account of `age_days` falls in, `None` past the last.
pub fn age_bucket(age_days: i64) -> Option<i64> {
    BOT_COSMETIC_AGE_BUCKETS_DAYS
        .iter()
        .copied()
        .find(|bound| age_days <= *bound)
}

/// R1834005: the fit a bot of `age_days` draws from: its bucket's row of `fits` with
/// `BOT_COSMETIC_FIT_MIN_SAMPLES` seats behind it, else the prior.
pub fn fit_for(fits: &'static [CosmeticFit], age_days: i64) -> &'static CosmeticFit {
    let bucket = age_bucket(age_days);
    fits.iter()
        .find(|fit| fit.max_age_days == bucket && fit.samples >= BOT_COSMETIC_FIT_MIN_SAMPLES)
        .unwrap_or(&BOT_COSMETIC_PRIOR)
}

/// R1834005: the non-starters a bot may own as of `now`, sorted: each held, where it is held, by at
/// least `BOT_COSMETIC_MIN_OWNERS` people (`owners`, `economy_cosmetic_owners`), and first granted
/// to one at least `BOT_COSMETIC_MIN_AGE_DAYS` before `now`. A cosmetic of no kind a loadout wears
/// (a portrait) and one no grant dates are never eligible.
pub fn eligible(registry: &[Cosmetic], owners: &[CosmeticOwners], now: i64) -> Vec<Holding> {
    let shipped_by = now - BOT_COSMETIC_MIN_AGE_DAYS * MS_PER_DAY;
    let worn = |cosmetic: &Cosmetic| {
        cosmetic.kind != CosmeticKind::Portrait && !cosmetic.default && !cosmetic.starter
    };
    let mut out: Vec<Holding> = owners
        .iter()
        .filter(|row| row.owners >= BOT_COSMETIC_MIN_OWNERS)
        .filter(|row| row.first_granted_at.is_some_and(|at| at <= shipped_by))
        .filter(|row| {
            registry.iter().any(|cosmetic| {
                cosmetic.id == row.cosmetic_id
                    && worn(cosmetic)
                    && cosmetic.kind.is_per_card() == row.card_id.is_some()
            })
        })
        .map(|row| (row.cosmetic_id.clone(), row.card_id.clone()))
        .collect();
    out.sort();
    out.dedup();
    out
}

fn holdings(owned: &OwnedCosmetics) -> BTreeSet<Holding> {
    owned
        .cards
        .iter()
        .map(|row| (row.cosmetic_id.clone(), Some(row.card_id.clone())))
        .chain(owned.account.iter().map(|id| (id.clone(), None)))
        .collect()
}

fn owned_of(held: &BTreeSet<Holding>) -> OwnedCosmetics {
    let mut owned = OwnedCosmetics::default();
    for (cosmetic_id, card) in held {
        match card {
            Some(card_id) => owned.cards.push(CardCosmetic {
                card_id: card_id.clone(),
                cosmetic_id: cosmetic_id.clone(),
            }),
            None => owned.account.push(cosmetic_id.clone()),
        }
    }
    owned
}

/// The fit's share of `fit`'s people that own `holding`, the prior's when it lists none.
fn ownership_share(fit: &CosmeticFit, (cosmetic_id, card): &Holding) -> f64 {
    fit.ownership
        .iter()
        .find(|row| row.cosmetic == cosmetic_id && row.card == card.as_deref())
        .map_or(BOT_COSMETIC_PRIOR_OWNERSHIP, |row| row.share)
}

/// `holding`'s roll for the bot of `seed`, in [0, 1): one stream per holding, so it is the same
/// roll at every draw from that seed, whatever else is eligible.
fn holding_roll(seed: &str, (cosmetic_id, card): &Holding) -> f64 {
    let card = card.as_deref().unwrap_or("");
    Rng::new(&format!("{seed}:cosmetics:wardrobe:{cosmetic_id}:{card}"), 0).next()
}

/// R1834005: a bot's wardrobe, from `seed` (`BotCosmetics::seed`): what it `held` that is still
/// eligible, and each other eligible holding whose roll falls under the fit's ownership share.
pub fn draw_wardrobe(
    seed: &str,
    fit: &CosmeticFit,
    eligible: &[Holding],
    held: &OwnedCosmetics,
) -> OwnedCosmetics {
    let held = holdings(held);
    let kept: BTreeSet<Holding> = eligible
        .iter()
        .filter(|holding| {
            held.contains(*holding) || holding_roll(seed, holding) < ownership_share(fit, holding)
        })
        .cloned()
        .collect();
    owned_of(&kept)
}

/// The weight `fit` gives `cosmetic` in its kind's slot among `candidates`: its share when the fit
/// lists the kind, else the prior's split (the default `BOT_COSMETIC_PRIOR_DEFAULT_SHARE`, the rest
/// even among the others).
fn slot_weight(
    fit: &CosmeticFit,
    registry: &[Cosmetic],
    cosmetic: &Cosmetic,
    candidates: &[&Cosmetic],
) -> f64 {
    let listed = |id: &str| fit.slots.iter().find(|(slot_id, _)| *slot_id == id);
    let kind_listed = fit.slots.iter().any(|(id, _)| {
        registry
            .iter()
            .any(|entry| entry.id == *id && entry.kind == cosmetic.kind)
    });
    if kind_listed {
        return listed(&cosmetic.id).map_or(0.0, |(_, share)| *share);
    }
    let others = candidates.iter().filter(|entry| !entry.default).count();
    match (cosmetic.default, others) {
        (true, 0) => 1.0,
        (true, _) => BOT_COSMETIC_PRIOR_DEFAULT_SHARE,
        (false, others) => (1.0 - BOT_COSMETIC_PRIOR_DEFAULT_SHARE) / others as f64,
    }
}

/// One cosmetic of `candidates`, weighted by `slot_weight`; `None` when none weighs anything.
fn pick_weighted<'r>(
    rng: &mut Rng,
    fit: &CosmeticFit,
    registry: &[Cosmetic],
    candidates: &[&'r Cosmetic],
) -> Option<&'r Cosmetic> {
    let weights: Vec<f64> = candidates
        .iter()
        .map(|cosmetic| slot_weight(fit, registry, cosmetic, candidates))
        .collect();
    pick_by(rng, candidates, &weights)
}

/// One of `candidates`, each at its weight of `weights`; `None` when none weighs anything.
fn pick_by<'r>(rng: &mut Rng, candidates: &[&'r Cosmetic], weights: &[f64]) -> Option<&'r Cosmetic> {
    let total: f64 = weights.iter().sum();
    let roll = rng.next() * total;
    if total <= 0.0 {
        return None;
    }
    let mut at = 0.0;
    for (cosmetic, weight) in candidates.iter().zip(weights) {
        at += weight;
        if roll < at {
            return Some(cosmetic);
        }
    }
    candidates
        .iter()
        .zip(weights)
        .rev()
        .find(|(_, weight)| **weight > 0.0)
        .map(|(cosmetic, _)| *cosmetic)
}

/// The cosmetics of `kind` a wardrobe owns on `card` (every card, `None`).
fn owned_of_kind<'r>(
    registry: &'r [Cosmetic],
    wardrobe: &OwnedCosmetics,
    kind: CosmeticKind,
    card: Option<&str>,
) -> Vec<&'r Cosmetic> {
    registry
        .iter()
        .filter(|cosmetic| cosmetic.kind == kind && owns(cosmetic, card, wardrobe))
        .collect()
}

/// One of `candidates` (what a bot owns, of `kinds`, on `card`), drawn so that each cosmetic is
/// taken at its share of `shares`. Non-starters are offered first at their share over undrawn seats
/// and ownership share (`ownership_share`), then starters and defaults by their shares.
fn draw_by_shares<'r>(
    rng: &mut Rng,
    registry: &'r [Cosmetic],
    fit: &CosmeticFit,
    shares: &[(&str, f64)],
    kinds: &[CosmeticKind],
    card: Option<&str>,
    candidates: &[&'r Cosmetic],
) -> Option<&'r Cosmetic> {
    let share_of = |id: &str| {
        shares
            .iter()
            .find(|(listed, _)| *listed == id)
            .map_or(0.0, |(_, share)| *share)
    };
    let mut undrawn = 1.0;
    for cosmetic in registry
        .iter()
        .filter(|cosmetic| kinds.contains(&cosmetic.kind) && !cosmetic.default && !cosmetic.starter)
    {
        let share = share_of(&cosmetic.id);
        if share <= 0.0 {
            continue;
        }
        let holding = (cosmetic.id.clone(), card.map(str::to_string));
        let reach = undrawn * ownership_share(fit, &holding);
        undrawn -= share;
        if !candidates.iter().any(|owned| owned.id == cosmetic.id) {
            continue;
        }
        if reach <= 0.0 || rng.chance(share / reach) {
            return Some(cosmetic);
        }
    }
    let common: Vec<&Cosmetic> = candidates
        .iter()
        .copied()
        .filter(|cosmetic| cosmetic.default || cosmetic.starter)
        .collect();
    let weights: Vec<f64> = common.iter().map(|cosmetic| share_of(&cosmetic.id)).collect();
    pick_by(rng, &common, &weights)
}

/// One slot of `kind`, drawn among what `wardrobe` owns so that, across the bots, each cosmetic is
/// worn at `fit`'s share of seats (`draw_by_shares`), the kind's default when nothing weighs
/// anything. A kind the fit lists nothing of takes the prior's split among what the wardrobe owns
/// (`slot_weight`).
fn draw_slot(
    rng: &mut Rng,
    registry: &[Cosmetic],
    fit: &CosmeticFit,
    wardrobe: &OwnedCosmetics,
    kind: CosmeticKind,
) -> String {
    let candidates = owned_of_kind(registry, wardrobe, kind, None);
    let fitted = fit.slots.iter().any(|(id, _)| {
        registry
            .iter()
            .any(|cosmetic| cosmetic.id == *id && cosmetic.kind == kind)
    });
    let drawn = if fitted {
        draw_by_shares(rng, registry, fit, fit.slots, &[kind], None, &candidates)
    } else {
        pick_weighted(rng, fit, registry, &candidates)
    };
    drawn
        .map(|cosmetic| cosmetic.id.clone())
        .unwrap_or_else(|| crate::cosmetics::default_of(registry, kind).to_string())
}

/// R1834005: a bot's account picks from `seed`, as a person's `cosmetics` settings group holds
/// them: the six kinds, each drawn by `draw_slot`, and the Full-art cards switch on, its default.
pub fn draw_account(
    seed: &str,
    registry: &[Cosmetic],
    fit: &CosmeticFit,
    wardrobe: &OwnedCosmetics,
) -> AccountPicks {
    let mut rng = Rng::new(&format!("{seed}:cosmetics:account"), 0);
    let mut account = AccountPicks::default();
    for kind in CARD_SLOT_KINDS.into_iter().chain(DECK_SLOT_KINDS) {
        let id = draw_slot(&mut rng, registry, fit, wardrobe, kind);
        account.picks.insert(setting_key(kind).to_string(), id);
    }
    account
}

/// R1834005: one deck's three slots from `seed` (one per deck), each drawn by `draw_slot`.
pub fn draw_slots(
    seed: &str,
    registry: &[Cosmetic],
    fit: &CosmeticFit,
    wardrobe: &OwnedCosmetics,
) -> DeckSlots {
    let mut rng = Rng::new(&format!("{seed}:cosmetics:slots"), 0);
    let mut slot = |kind| Some(draw_slot(&mut rng, registry, fit, wardrobe, kind));
    DeckSlots {
        table: slot(CosmeticKind::Table),
        field: slot(CosmeticKind::Field),
        card_back: slot(CosmeticKind::CardBack),
    }
}

/// Whether `fit` holds people's picks on cards (`card_picks`, `earned_wear`): the prior holds none.
fn fits_card_picks(fit: &CosmeticFit) -> bool {
    !fit.card_picks.is_empty() || !fit.earned_wear.is_empty()
}

/// One pick of `card`'s own: under `fit`, a default or starter foil, finish or border drawn at its
/// share of people's picks (`card_picks`), or under the prior, the prior's split. Never the
/// account's own pick (no card wears the account's, R1953001), listing only what differs from its
/// seat (R1834002).
fn draw_card_pick<'r>(
    rng: &mut Rng,
    registry: &'r [Cosmetic],
    fit: &CosmeticFit,
    wardrobe: &OwnedCosmetics,
    account: &AccountPicks,
    card: &str,
) -> Option<&'r Cosmetic> {
    let candidates_of = |kind: CosmeticKind| {
        let seat = account.get(kind).unwrap_or_default();
        owned_of_kind(registry, wardrobe, kind, Some(card))
            .into_iter()
            .filter(move |cosmetic| cosmetic.id != seat)
    };
    if !fits_card_picks(fit) {
        let kind = rng.pick(&CARD_SLOT_KINDS).copied()?;
        let candidates: Vec<&Cosmetic> = candidates_of(kind).collect();
        return pick_weighted(rng, fit, registry, &candidates);
    }
    let candidates: Vec<&Cosmetic> = CARD_SLOT_KINDS
        .into_iter()
        .flat_map(candidates_of)
        .filter(|cosmetic| cosmetic.default || cosmetic.starter)
        .collect();
    draw_by_shares(
        rng,
        registry,
        fit,
        fit.card_picks,
        &CARD_SLOT_KINDS,
        Some(card),
        &candidates,
    )
}

/// Under a fit holding people's picks on cards, `card`'s earned foils, finishes and borders: each
/// owned on the card is worn at its `earned_wear`, a kind taking the first that rolls. Answers
/// whether the card took a pick.
fn wear_earned(
    rng: &mut Rng,
    registry: &[Cosmetic],
    fit: &CosmeticFit,
    wardrobe: &OwnedCosmetics,
    picks: &mut CardPicks,
) -> bool {
    let card = picks.card_id.clone();
    let mut took = false;
    for cosmetic in registry.iter().filter(|cosmetic| {
        CARD_SLOT_KINDS.contains(&cosmetic.kind)
            && !cosmetic.default
            && !cosmetic.starter
            && owns(cosmetic, Some(&card), wardrobe)
    }) {
        let wear = fit
            .earned_wear
            .iter()
            .find(|row| row.cosmetic == cosmetic.id && row.card == card)
            .map_or(0.0, |row| row.share);
        if picks.get(cosmetic.kind).is_none() && rng.chance(wear) {
            picks.set(cosmetic.kind, Some(cosmetic.id.clone()));
            took = true;
        }
    }
    took
}

/// The share of seats that own `full_art` and play its card that print it.
fn full_art_wear(fit: &CosmeticFit, full_art: &str) -> f64 {
    fit.full_art_wear
        .iter()
        .find(|(id, _)| *id == full_art)
        .map_or(BOT_COSMETIC_PRIOR_FULL_ART_WEAR, |(_, share)| *share)
}

/// R1834005: a bot's cards' own picks from `seed` over its `decks`: each card first wears what it
/// earned (`wear_earned`), then at `per_card_share` up to `cards_per_deck_mean` cards receive picks
/// other than the account's (`draw_card_pick`), and full arts are worn at `full_art_wear`.
pub fn draw_card_picks(
    seed: &str,
    registry: &[Cosmetic],
    fit: &CosmeticFit,
    wardrobe: &OwnedCosmetics,
    account: &AccountPicks,
    decks: &[&[String]],
) -> Vec<CardPicks> {
    let mut cards: Vec<CardPicks> = Vec::new();
    let mut every: BTreeSet<&str> = BTreeSet::new();
    for (index, deck) in decks.iter().enumerate() {
        let mut rng = Rng::new(&format!("{seed}:cosmetics:cards:{index}"), 0);
        let distinct: Vec<&str> = deck
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<&str>>()
            .into_iter()
            .collect();
        let mut earned = 0;
        if fits_card_picks(fit) {
            for card in distinct.iter().filter(|card| !every.contains(*card)) {
                let mut picks = CardPicks {
                    card_id: card.to_string(),
                    ..CardPicks::default()
                };
                if wear_earned(&mut rng, registry, fit, wardrobe, &mut picks) {
                    cards.push(picks);
                    earned += 1;
                }
            }
        }
        every.extend(distinct.iter().copied());
        if !rng.chance(fit.per_card_share) || distinct.is_empty() {
            continue;
        }
        let many = (fit.cards_per_deck_mean.round().max(1.0) as usize).min(distinct.len());
        for card in rng
            .shuffle(&distinct)
            .into_iter()
            .take(many.saturating_sub(earned))
        {
            if cards.iter().any(|picks| picks.card_id == card) {
                continue;
            }
            if let Some(picked) = draw_card_pick(&mut rng, registry, fit, wardrobe, account, card) {
                let at = entry(&mut cards, card);
                cards[at].set(picked.kind, Some(picked.id.clone()));
            }
        }
    }
    wear_full_arts(
        seed,
        registry,
        fit,
        wardrobe,
        &OwnedCosmetics::default(),
        &mut cards,
        every,
    );
    cards.retain(|picks| !picks.is_empty());
    cards.sort_by(|a, b| a.card_id.cmp(&b.card_id));
    cards
}

fn entry(cards: &mut Vec<CardPicks>, card: &str) -> usize {
    match cards.iter().position(|picks| picks.card_id == card) {
        Some(at) => at,
        None => {
            cards.push(CardPicks {
                card_id: card.to_string(),
                ..CardPicks::default()
            });
            cards.len() - 1
        }
    }
}

/// Each card of `among` whose full art `wardrobe` owns and `before` did not prints it at the fit's
/// wear, from `seed`: the account's switch is on, so one that does not is switched off on its card.
fn wear_full_arts<'c>(
    seed: &str,
    registry: &[Cosmetic],
    fit: &CosmeticFit,
    wardrobe: &OwnedCosmetics,
    before: &OwnedCosmetics,
    cards: &mut Vec<CardPicks>,
    among: impl IntoIterator<Item = &'c str>,
) {
    let mut rng = Rng::new(&format!("{seed}:cosmetics:full-art"), 0);
    for card in among {
        let Some(full_art) = full_art_of(registry, card) else {
            continue;
        };
        if !owns(full_art, Some(card), wardrobe) || owns(full_art, Some(card), before) {
            continue;
        }
        if !rng.chance(full_art_wear(fit, &full_art.id)) {
            let at = entry(cards, card);
            cards[at].full_art = Some(FullArtPick::On(false));
        }
    }
}

/// R1834005: what [`redraw`] changes of a bot: its cosmetics, and its decks and trio when a deck of
/// theirs was given its slots (`None` for what it left alone).
#[derive(Clone, Debug, PartialEq)]
pub struct BotDraw {
    pub cosmetics: BotCosmetics,
    pub decks: Option<Vec<BotDeck>>,
    pub trio: Option<FrozenTrio>,
}

/// Every deck a bot plays, its Best-of-1 decks then its trio's, by their cards.
fn decks_of<'b>(decks: &'b [BotDeck], trio: Option<&'b FrozenTrio>) -> Vec<&'b [String]> {
    let trio = trio.into_iter().flat_map(|trio| {
        let (a, b, c) = &trio.decks;
        [a.cards.as_slice(), b.cards.as_slice(), c.cards.as_slice()]
    });
    decks
        .iter()
        .map(|deck| deck.cards.as_slice())
        .chain(trio)
        .collect()
}

/// `slots` aged: each slot changes at `BOT_COSMETIC_SLOT_CHANGE_SHARE`, and one the wardrobe no
/// longer owns is drawn again.
fn aged_slot(
    rng: &mut Rng,
    registry: &[Cosmetic],
    fit: &CosmeticFit,
    wardrobe: &OwnedCosmetics,
    kind: CosmeticKind,
    held: Option<&str>,
) -> String {
    let change = rng.chance(BOT_COSMETIC_SLOT_CHANGE_SHARE);
    let still = held.filter(|id| {
        registry
            .iter()
            .any(|cosmetic| cosmetic.kind == kind && cosmetic.id == *id && owns(cosmetic, None, wardrobe))
    });
    match (change, still) {
        (false, Some(id)) => id.to_string(),
        _ => draw_slot(rng, registry, fit, wardrobe, kind),
    }
}

/// R1834005: what a bot draws at a refresh at `now`, or `None` when nothing is due: draws everything
/// if undrawn; redraws slots and card picks if a deck has no slots; or ages after
/// `BOT_COSMETIC_REDRAW_DAYS` (updates wardrobe, changes slots at `BOT_COSMETIC_SLOT_CHANGE_SHARE`).
pub fn redraw(
    seed: &str,
    bot: &BotRow,
    fits: &'static [CosmeticFit],
    registry: &[Cosmetic],
    eligible: &[Holding],
    now: i64,
) -> Option<BotDraw> {
    if bot.decks.is_empty() {
        return None;
    }
    let fit = fit_for(fits, (now - bot.created_at) / MS_PER_DAY);
    let redealt = bot.decks.iter().any(|deck| deck.slots.is_none())
        || bot.trio.as_ref().is_some_and(|trio| {
            let (a, b, c) = &trio.decks;
            [a, b, c].iter().any(|deck| deck.slots.is_none())
        });
    let held = bot.cosmetics.as_ref();
    let ages = held.is_some_and(|held| now - held.drawn_at >= BOT_COSMETIC_REDRAW_DAYS * MS_PER_DAY);
    if held.is_some() && !redealt && !ages {
        return None;
    }

    // The wardrobe rolls the seed of the bot's first draw, at every draw after it too.
    let wardrobe_seed = held
        .map(|held| held.seed.as_str())
        .filter(|kept| !kept.is_empty())
        .unwrap_or(seed);
    let nothing = OwnedCosmetics::default();
    let wardrobe = match held {
        Some(held) if !ages => {
            // Not aging: keep what is still eligible, gain nothing.
            let still: BTreeSet<Holding> = eligible.iter().cloned().collect();
            owned_of(&holdings(&held.wardrobe).intersection(&still).cloned().collect())
        }
        Some(held) => draw_wardrobe(wardrobe_seed, fit, eligible, &held.wardrobe),
        None => draw_wardrobe(wardrobe_seed, fit, eligible, &nothing),
    };

    let mut rng = Rng::new(&format!("{seed}:cosmetics:age"), 0);
    let account = match held {
        None => draw_account(seed, registry, fit, &wardrobe),
        Some(held) if ages => {
            let mut account = held.account.clone();
            for kind in CARD_SLOT_KINDS.into_iter().chain(DECK_SLOT_KINDS) {
                let id = aged_slot(&mut rng, registry, fit, &wardrobe, kind, held.account.get(kind));
                account.picks.insert(setting_key(kind).to_string(), id);
            }
            account
        }
        Some(held) => held.account.clone(),
    };

    // Each deck's slots: drawn for a deck with none, aged for one that has them when the bot ages.
    let mut dressed = 0usize;
    let mut dress = |slots: &Option<DeckSlots>, rng: &mut Rng| -> Option<DeckSlots> {
        let index = dressed;
        dressed += 1;
        match slots {
            None => Some(draw_slots(
                &format!("{seed}:deck:{index}"),
                registry,
                fit,
                &wardrobe,
            )),
            Some(slots) if ages => {
                let mut slot = |kind| Some(aged_slot(rng, registry, fit, &wardrobe, kind, slots.get(kind)));
                Some(DeckSlots {
                    table: slot(CosmeticKind::Table),
                    field: slot(CosmeticKind::Field),
                    card_back: slot(CosmeticKind::CardBack),
                })
            }
            Some(slots) => Some(slots.clone()),
        }
    };
    let decks: Vec<BotDeck> = bot
        .decks
        .iter()
        .map(|deck| BotDeck {
            slots: dress(&deck.slots, &mut rng),
            ..deck.clone()
        })
        .collect();
    let trio = bot.trio.as_ref().map(|trio| {
        let (a, b, c) = &trio.decks;
        let mut again = |deck: &crate::db::store::FrozenDeck| crate::db::store::FrozenDeck {
            slots: dress(&deck.slots, &mut rng),
            ..deck.clone()
        };
        FrozenTrio {
            decks: (again(a), again(b), again(c)),
            ..trio.clone()
        }
    });

    let played = decks_of(&decks, trio.as_ref());
    let cards = match held {
        Some(held) if !redealt => {
            let mut cards: Vec<CardPicks> = held
                .cards
                .iter()
                .cloned()
                .map(|mut picks| {
                    for kind in CARD_SLOT_KINDS {
                        let owned = picks.get(kind).is_some_and(|id| {
                            registry.iter().any(|cosmetic| {
                                cosmetic.kind == kind
                                    && cosmetic.id == id
                                    && owns(cosmetic, Some(&picks.card_id), &wardrobe)
                            })
                        });
                        if !owned {
                            picks.set(kind, None);
                        }
                    }
                    picks
                })
                .collect();
            let every: BTreeSet<&str> = played
                .iter()
                .flat_map(|deck| deck.iter().map(String::as_str))
                .collect();
            wear_full_arts(seed, registry, fit, &wardrobe, &held.wardrobe, &mut cards, every);
            cards.retain(|picks| !picks.is_empty());
            cards.sort_by(|a, b| a.card_id.cmp(&b.card_id));
            cards
        }
        _ => draw_card_picks(seed, registry, fit, &wardrobe, &account, &played),
    };

    let changed_decks = decks != bot.decks;
    let changed_trio = trio != bot.trio;
    Some(BotDraw {
        cosmetics: BotCosmetics {
            wardrobe,
            account,
            cards,
            drawn_at: match held {
                Some(held) if !ages => held.drawn_at,
                _ => now,
            },
            seed: wardrobe_seed.to_string(),
        },
        decks: changed_decks.then_some(decks),
        trio: if changed_trio { trio } else { None },
    })
}
