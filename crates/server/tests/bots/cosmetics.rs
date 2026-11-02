//! R1834005 (SPEC §14.2): the ladder bots' cosmetics. A bot's wardrobe holds a non-starter only when
//! `BOT_COSMETIC_MIN_OWNERS` people own it, where it is held, and the first of them got it
//! `BOT_COSMETIC_MIN_AGE_DAYS` ago; it wears a loadout drawn from that wardrobe at the fit's shares
//! (the prior's while `BOT_COSMETIC_FITS` is empty); one seed draws one wardrobe and one loadout; a
//! deck dealt again draws its slots and its cards' picks again, and every `BOT_COSMETIC_REDRAW_DAYS`
//! the bot ages; and none of it ever reaches the ledger (R1454: a bot owns nothing).
//!
//! `jackioh_server::bots::cosmetics`, pure, over the shipped registry (and a test one built on it),
//! and `refresh_pool` against the fake store.

use std::collections::{BTreeMap, BTreeSet};

use jackioh_engine::config::DECK_SIZE;
use jackioh_server::app::now_ms;
use jackioh_server::bots::cosmetics::{
    Holding, draw_account, draw_card_picks, draw_slots, draw_wardrobe, eligible, fit_for, redraw,
};
use jackioh_server::bots::pool::refresh_pool;
use jackioh_server::config::{
    BOT_COSMETIC_FIT_MIN_SAMPLES, BOT_COSMETIC_FITS, BOT_COSMETIC_MIN_AGE_DAYS, BOT_COSMETIC_MIN_OWNERS,
    BOT_COSMETIC_PRIOR, BOT_COSMETIC_PRIOR_CARDS_PER_DECK, BOT_COSMETIC_PRIOR_DEFAULT_SHARE,
    BOT_COSMETIC_PRIOR_FULL_ART_WEAR, BOT_COSMETIC_PRIOR_OWNERSHIP, BOT_COSMETIC_REDRAW_DAYS, BOT_STRENGTHS,
    CosmeticFit, CosmeticOwnership, CosmeticWear, PersonaName,
};
use jackioh_server::cosmetics::{Cosmetic, CosmeticKind, CosmeticRarity, JLORIOUS_CARD_BACK, owns, registry};
use jackioh_server::db::store::{BotDeck, BotRow, FrozenDeck, FrozenTrio, NewBot, ProfileCreateInput};
use jackioh_server::economy::store::{
    CardCosmetic, CardPicks, CosmeticGrant, CosmeticOwners, EconomyLedger, EconomyReason, OwnedCosmetics,
};
use jackioh_server::loadout::{AccountPicks, CARD_SLOT_KINDS, DECK_SLOT_KINDS, DeckSlots};

use crate::support::deps::empty_test_app;
use crate::support::store::q;

/// Unit conversion, not configuration: a day in milliseconds.
const DAY: i64 = 86_400_000;
/// The pure tests' clock, epoch ms: any fixed time does.
const NOW: i64 = 1_790_000_000_000;
/// How many bots a pool's shares are measured over.
const POOL: usize = 2_000;
/// How far a share measured over `POOL` bots may stray from the fit's: over four standard
/// deviations of the widest (a share of one half, about 0.011 over `POOL`).
const TOLERANCE: f64 = 0.05;
/// How many seeds a test draws over where it needs many draws but no shares.
const SEEDS: usize = 200;
/// How many refresh seeds `r1834005_never_in_the_ledger` tries for one that gives its bot a
/// wardrobe; about one in four does.
const SEED_SEARCH: usize = 1_000;

/// The six full arts' holdings, each on its card.
fn full_arts() -> Vec<Holding> {
    registry()
        .iter()
        .filter(|cosmetic| cosmetic.kind == CosmeticKind::FullArt)
        .map(|cosmetic| (cosmetic.id.clone(), cosmetic.card.clone()))
        .collect()
}

fn holding(cosmetic: &str, card: Option<&str>) -> Holding {
    (cosmetic.to_string(), card.map(str::to_string))
}

/// What `owned` holds, as holdings.
fn holdings(owned: &OwnedCosmetics) -> BTreeSet<Holding> {
    owned
        .cards
        .iter()
        .map(|row| holding(&row.cosmetic_id, Some(&row.card_id)))
        .chain(owned.account.iter().map(|id| holding(id, None)))
        .collect()
}

/// A wardrobe of `cards` (each `(cosmetic, card)`) and `account`.
fn wardrobe(cards: &[(&str, &str)], account: &[&str]) -> OwnedCosmetics {
    OwnedCosmetics {
        cards: cards
            .iter()
            .map(|(cosmetic_id, card_id)| CardCosmetic {
                card_id: (*card_id).to_string(),
                cosmetic_id: (*cosmetic_id).to_string(),
            })
            .collect(),
        account: account.iter().map(|id| (*id).to_string()).collect(),
    }
}

/// The shipped registry with two cosmetics nobody starts with: a per-card foil and an account
/// playmat (at the global roll's floor, as R1661000 asks of an account cosmetic).
fn test_registry() -> Vec<Cosmetic> {
    let earned = |id: &str, kind: CosmeticKind, rarity: CosmeticRarity| Cosmetic {
        id: id.to_string(),
        kind,
        name: id.to_string(),
        rarity,
        default: false,
        starter: false,
        chase: false,
        jlorious: false,
        card: None,
    };
    let mut cosmetics = registry().to_vec();
    cosmetics.push(earned("foil-prism", CosmeticKind::Foil, CosmeticRarity::Rare));
    cosmetics.push(earned("table-velvet", CosmeticKind::Table, CosmeticRarity::Epic));
    cosmetics
}

/// `DECK_SIZE` catalog-shaped card ids from `core-{first}` on.
fn deck_of(first: usize) -> Vec<String> {
    let size = usize::try_from(DECK_SIZE).expect("a deck size");
    (first..first + size).map(|n| format!("core-{n:03}")).collect()
}

/// A Best-of-1 deck as the pool deals it: no slots yet.
fn dealt(cards: &[String]) -> BotDeck {
    BotDeck {
        cards: cards.to_vec(),
        portrait: "vanilla".to_string(),
        slots: None,
    }
}

/// A trio as the pool deals it: no slots yet.
fn dealt_trio(decks: [&[String]; 3]) -> FrozenTrio {
    let frozen = |slot: usize, cards: &[String]| FrozenDeck {
        name: format!("Deck {}", slot + 1),
        cards: cards.to_vec(),
        portrait: Some(Some("vanilla".to_string())),
        slots: None,
    };
    let [a, b, c] = decks;
    FrozenTrio {
        name: "Conquest".to_string(),
        decks: (frozen(0, a), frozen(1, b), frozen(2, c)),
    }
}

fn bot(id: &str, created_at: i64, decks: Vec<BotDeck>) -> BotRow {
    BotRow {
        profile_id: id.to_string(),
        strength: 0,
        speed: 1.0,
        persona: PersonaName::Balanced,
        active_from_hour: 0,
        active_hours: 24,
        decks,
        decks_catalog_version: None,
        trio: None,
        trio_catalog_version: None,
        created_at,
        retired_at: None,
        rating: 1000.0,
        in_match_id: None,
        grinding: false,
        grind_wins: 0,
        cosmetics: None,
    }
}

/// The card ids `cards` gives a foil, a finish or a border of their own.
fn picked(cards: &[CardPicks]) -> Vec<&str> {
    cards
        .iter()
        .filter(|picks| CARD_SLOT_KINDS.iter().any(|kind| picks.get(*kind).is_some()))
        .map(|picks| picks.card_id.as_str())
        .collect()
}

/// The registry's entry `id` of `kind`.
fn entry<'r>(registry: &'r [Cosmetic], kind: CosmeticKind, id: &str) -> &'r Cosmetic {
    registry
        .iter()
        .find(|cosmetic| cosmetic.kind == kind && cosmetic.id == id)
        .unwrap_or_else(|| panic!("{id} is no {kind:?} of the registry"))
}

#[track_caller]
fn close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() <= TOLERANCE,
        "{what}: {actual:.4} is not within {TOLERANCE} of {expected:.4}"
    );
}

// ---------------------------------------------------------------------------------------------
// One seed, one wardrobe and one loadout
// ---------------------------------------------------------------------------------------------

#[test]
fn r1834005_deterministic_from_the_seed() {
    let registry = registry();
    let eligible = full_arts();
    let (a, b, c, d) = (deck_of(1), deck_of(26), deck_of(51), deck_of(76));
    let mut dressed = bot("bot-determined", NOW - 40 * DAY, vec![dealt(&a), dealt(&b)]);
    dressed.trio = Some(dealt_trio([&b, &c, &d]));
    let fit = fit_for(BOT_COSMETIC_FITS, 40);
    let none = OwnedCosmetics::default();
    let seed = "pool-seed:bot-determined";

    let first = redraw(seed, &dressed, BOT_COSMETIC_FITS, registry, &eligible, NOW).expect("a first draw");
    assert_eq!(
        redraw(seed, &dressed, BOT_COSMETIC_FITS, registry, &eligible, NOW),
        Some(first.clone())
    );

    // Each draw on its own, twice over, and what `redraw` drew from it.
    let wardrobe = draw_wardrobe(seed, fit, &eligible, &none);
    assert_eq!(draw_wardrobe(seed, fit, &eligible, &none), wardrobe);
    assert_eq!(first.cosmetics.wardrobe, wardrobe);
    let account = draw_account(seed, registry, fit, &wardrobe);
    assert_eq!(draw_account(seed, registry, fit, &wardrobe), account);
    assert_eq!(first.cosmetics.account, account);
    let slots = draw_slots(&format!("{seed}:deck:0"), registry, fit, &wardrobe);
    assert_eq!(
        draw_slots(&format!("{seed}:deck:0"), registry, fit, &wardrobe),
        slots
    );
    let decks = first.decks.as_ref().expect("its decks, given slots");
    assert_eq!(decks[0].slots.as_ref(), Some(&slots));
    let played: Vec<&[String]> = vec![&a, &b, &b, &c, &d].into_iter().map(Vec::as_slice).collect();
    let cards = draw_card_picks(seed, registry, fit, &wardrobe, &account, &played);
    assert_eq!(
        draw_card_picks(seed, registry, fit, &wardrobe, &account, &played),
        cards
    );
    assert_eq!(first.cosmetics.cards, cards);

    // Another seed (the next refresh's, or another bot's) draws another loadout.
    let other = redraw(
        "pool-seed:bot-other",
        &dressed,
        BOT_COSMETIC_FITS,
        registry,
        &eligible,
        NOW,
    )
    .expect("a first draw");
    assert_ne!(other, first);
    assert!(
        other.cosmetics.account != first.cosmetics.account || other.decks != first.decks,
        "two seeds dressed the bot alike"
    );
}

// ---------------------------------------------------------------------------------------------
// The floor: widely enough owned, and long enough
// ---------------------------------------------------------------------------------------------

/// Every holding the floor test names, owned by every bot: a wardrobe that still lacks one lacks
/// it because it is not eligible.
static EVERYONE_OWNS: CosmeticFit = CosmeticFit {
    max_age_days: None,
    samples: BOT_COSMETIC_FIT_MIN_SAMPLES,
    slots: &[],
    per_card_share: 1.0,
    cards_per_deck_mean: BOT_COSMETIC_PRIOR_CARDS_PER_DECK,
    card_picks: &[],
    earned_wear: &[],
    full_art_wear: &[],
    ownership: &[
        CosmeticOwnership {
            cosmetic: "foil-prism",
            card: Some("core-002"),
            share: 1.0,
        },
        CosmeticOwnership {
            cosmetic: "table-velvet",
            card: None,
            share: 1.0,
        },
        CosmeticOwnership {
            cosmetic: "full-art-big-d-fender",
            card: Some("core-001"),
            share: 1.0,
        },
        CosmeticOwnership {
            cosmetic: "full-art-gary-the-gambler",
            card: Some("core-004"),
            share: 1.0,
        },
        CosmeticOwnership {
            cosmetic: "full-art-mr-vanilla",
            card: Some("core-008"),
            share: 1.0,
        },
        CosmeticOwnership {
            cosmetic: "full-art-tempo-timmy",
            card: Some("core-011"),
            share: 1.0,
        },
        CosmeticOwnership {
            cosmetic: "portrait-gary",
            card: None,
            share: 1.0,
        },
    ],
};

#[test]
fn r2755000_no_bot_wears_the_jlorious_card_back_however_widely_owned() {
    let long_ago = NOW - BOT_COSMETIC_MIN_AGE_DAYS * DAY - 30 * DAY;
    let widely = |cosmetic: &str| CosmeticOwners {
        cosmetic_id: cosmetic.to_string(),
        card_id: None,
        owners: BOT_COSMETIC_MIN_OWNERS * 100,
        first_granted_at: Some(long_ago),
    };
    // A bot is never Jlorious (R640000), so it could not have earned it; the Mythic back that drops
    // stays eligible beside it.
    let owners = vec![widely(JLORIOUS_CARD_BACK), widely("card-back-nebula")];
    assert_eq!(
        eligible(registry(), &owners, NOW),
        vec![holding("card-back-nebula", None)]
    );
}

#[test]
fn r1834005_nothing_below_the_owner_floor_or_too_young() {
    let registry = test_registry();
    let shipped_by = NOW - BOT_COSMETIC_MIN_AGE_DAYS * DAY;
    let long_ago = shipped_by - 30 * DAY;
    let widely = BOT_COSMETIC_MIN_OWNERS * 100;
    let row = |cosmetic: &str, card: Option<&str>, owners: i64, first: Option<i64>| CosmeticOwners {
        cosmetic_id: cosmetic.to_string(),
        card_id: card.map(str::to_string),
        owners,
        first_granted_at: first,
    };
    let owners = vec![
        // At the floor on both counts, listed twice: eligible, once.
        row(
            "full-art-big-d-fender",
            Some("core-001"),
            BOT_COSMETIC_MIN_OWNERS,
            Some(shipped_by),
        ),
        row(
            "full-art-big-d-fender",
            Some("core-001"),
            BOT_COSMETIC_MIN_OWNERS,
            Some(shipped_by),
        ),
        row("foil-prism", Some("core-002"), widely, Some(long_ago)),
        row("table-velvet", None, BOT_COSMETIC_MIN_OWNERS, Some(long_ago)),
        // One person short of the floor, however long ago.
        row(
            "full-art-gary-the-gambler",
            Some("core-004"),
            BOT_COSMETIC_MIN_OWNERS - 1,
            Some(long_ago),
        ),
        // A millisecond too young, however widely owned.
        row(
            "full-art-mr-vanilla",
            Some("core-008"),
            widely,
            Some(shipped_by + 1),
        ),
        // No grant dates it.
        row("full-art-tempo-timmy", Some("core-011"), widely, None),
        // A per-card cosmetic held on no card, and an account cosmetic held on a card.
        row("foil-prism", None, widely, Some(long_ago)),
        row("table-velvet", Some("core-002"), widely, Some(long_ago)),
        // A portrait (no slot of a loadout), a starter, a default and an id the registry lacks.
        row("portrait-gary", None, widely, Some(long_ago)),
        row("foil-rainbow", Some("core-002"), widely, Some(long_ago)),
        row("table-tavern", None, widely, Some(long_ago)),
        row("foil-unheard-of", Some("core-002"), widely, Some(long_ago)),
    ];
    let listed = eligible(&registry, &owners, NOW);
    assert_eq!(
        listed,
        vec![
            holding("foil-prism", Some("core-002")),
            holding("full-art-big-d-fender", Some("core-001")),
            holding("table-velvet", None),
        ]
    );
    // A millisecond later the young full art comes of age.
    assert!(
        eligible(&registry, &owners, NOW + 1).contains(&holding("full-art-mr-vanilla", Some("core-008")))
    );

    // A wardrobe holds only what is eligible: drawn with every holding at a share of one, and even
    // when it held the ineligible ones already, it holds the three and nothing else.
    let eligible_set: BTreeSet<Holding> = listed.iter().cloned().collect();
    let held = wardrobe(
        &[
            ("full-art-gary-the-gambler", "core-004"),
            ("full-art-mr-vanilla", "core-008"),
            ("foil-prism", "core-002"),
        ],
        &["portrait-gary", "table-velvet"],
    );
    let deck: Vec<String> = deck_of(1);
    for n in 0..SEEDS {
        let seed = format!("floor:{n}");
        let everything = draw_wardrobe(&seed, &EVERYONE_OWNS, &listed, &held);
        assert_eq!(holdings(&everything), eligible_set, "{seed}");
        let drawn = draw_wardrobe(&seed, &BOT_COSMETIC_PRIOR, &listed, &OwnedCosmetics::default());
        assert!(holdings(&drawn).is_subset(&eligible_set), "{seed}: {drawn:?}");

        // And what it wears is what it owns, where it wears it.
        for owned in [&everything, &drawn] {
            let account = draw_account(&seed, &registry, &EVERYONE_OWNS, owned);
            for kind in CARD_SLOT_KINDS.into_iter().chain(DECK_SLOT_KINDS) {
                let id = account.get(kind).expect("a pick of every kind");
                assert!(owns(entry(&registry, kind, id), None, owned), "{seed}: {id}");
            }
            let slots = draw_slots(&seed, &registry, &EVERYONE_OWNS, owned);
            for kind in DECK_SLOT_KINDS {
                let id = slots.get(kind).expect("a slot of every kind");
                assert!(owns(entry(&registry, kind, id), None, owned), "{seed}: {id}");
            }
            let cards = draw_card_picks(
                &seed,
                &registry,
                &EVERYONE_OWNS,
                owned,
                &account,
                &[deck.as_slice()],
            );
            for picks in &cards {
                for kind in CARD_SLOT_KINDS {
                    if let Some(id) = picks.get(kind) {
                        let cosmetic = entry(&registry, kind, id);
                        assert!(
                            owns(cosmetic, Some(&picks.card_id), owned),
                            "{seed}: {id} on {}",
                            picks.card_id
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The pool wears what the fit says
// ---------------------------------------------------------------------------------------------

/// A fit with shares of its own: the playmat and the foil listed (the Neon Grid playmat and the
/// Cracked Ice foil at none), every other kind left to the prior's split; half the seats pick
/// two cards of their own; Big D-fender's full art printed by three in ten of the seats that own it
/// and owned by four in ten bots.
static SHARES: [CosmeticFit; 1] = [CosmeticFit {
    max_age_days: Some(30),
    samples: BOT_COSMETIC_FIT_MIN_SAMPLES,
    slots: &[
        ("table-tavern", 0.1),
        ("table-wood", 0.2),
        ("table-felt", 0.7),
        ("foil-none", 0.25),
        ("foil-holo-sweep", 0.25),
        ("foil-rainbow", 0.5),
    ],
    per_card_share: 0.5,
    cards_per_deck_mean: 2.0,
    card_picks: &[],
    earned_wear: &[],
    full_art_wear: &[("full-art-big-d-fender", 0.3)],
    ownership: &[CosmeticOwnership {
        cosmetic: "full-art-big-d-fender",
        card: Some("core-001"),
        share: 0.4,
    }],
}];

/// What `POOL` bots drawn from `fit` wear, as shares of them.
struct Worn {
    /// Each id's share of the account's pick of its kind, and of a deck's slot of its kind.
    account: BTreeMap<String, f64>,
    slots: BTreeMap<String, f64>,
    /// The share of bots that pick something of their own for a card of their deck, and the mean
    /// number of cards those pick.
    per_card: f64,
    cards_per_deck: f64,
    /// Of bots that own Big D-fender's full art and play its card, the share that switch it off.
    full_art_off: f64,
    /// Each eligible holding's share of the wardrobes that hold it.
    owning: BTreeMap<Holding, f64>,
}

fn wear(fit: &CosmeticFit) -> Worn {
    let registry = registry();
    let none = OwnedCosmetics::default();
    let dfender = wardrobe(&[("full-art-big-d-fender", "core-001")], &[]);
    let eligible = vec![
        holding("full-art-big-d-fender", Some("core-001")),
        holding("full-art-gary-the-gambler", Some("core-004")),
    ];
    let deck = deck_of(1);
    let share = |count: usize| count as f64 / POOL as f64;
    let mut account: BTreeMap<String, usize> = BTreeMap::new();
    let mut slots: BTreeMap<String, usize> = BTreeMap::new();
    let mut picking = Vec::new();
    let mut off = 0;
    let mut owning: BTreeMap<Holding, usize> = BTreeMap::new();
    for n in 0..POOL {
        let seed = format!("pool-share:{n}");
        let picks = draw_account(&seed, registry, fit, &none);
        for kind in CARD_SLOT_KINDS.into_iter().chain(DECK_SLOT_KINDS) {
            let id = picks.get(kind).expect("a pick of every kind");
            *account.entry(id.to_string()).or_default() += 1;
        }
        let deck_slots = draw_slots(&seed, registry, fit, &none);
        for kind in DECK_SLOT_KINDS {
            let id = deck_slots.get(kind).expect("a slot of every kind");
            *slots.entry(id.to_string()).or_default() += 1;
        }
        let cards = draw_card_picks(&seed, registry, fit, &none, &picks, &[deck.as_slice()]);
        for card in &cards {
            assert!(
                deck.contains(&card.card_id),
                "{seed}: {} is no card of the deck",
                card.card_id
            );
            for kind in CARD_SLOT_KINDS {
                // A card's own pick is never the account's own pick of its kind.
                assert!(
                    card.get(kind).is_none() || card.get(kind) != picks.get(kind),
                    "{seed}: {card:?} repeats the account's {kind:?}"
                );
            }
        }
        let many = picked(&cards).len();
        if many > 0 {
            picking.push(many);
        }
        let printed = draw_card_picks(&seed, registry, fit, &dfender, &picks, &[deck.as_slice()]);
        if printed
            .iter()
            .any(|card| card.card_id == "core-001" && card.full_art == Some(false.into()))
        {
            off += 1;
        }
        for held in holdings(&draw_wardrobe(&seed, fit, &eligible, &none)) {
            *owning.entry(held).or_default() += 1;
        }
    }
    Worn {
        account: account
            .into_iter()
            .map(|(id, count)| (id, share(count)))
            .collect(),
        slots: slots.into_iter().map(|(id, count)| (id, share(count))).collect(),
        per_card: share(picking.len()),
        cards_per_deck: picking.iter().sum::<usize>() as f64 / picking.len().max(1) as f64,
        full_art_off: share(off),
        owning: eligible
            .into_iter()
            .map(|held| {
                let count = owning.get(&held).copied().unwrap_or(0);
                (held, share(count))
            })
            .collect(),
    }
}

/// The share of bots `fit` gives `cosmetic` in its slot, as `CosmeticFit` says: its listed share
/// when the fit lists its kind (none when it lists the kind and not it), else the prior's split,
/// `BOT_COSMETIC_PRIOR_DEFAULT_SHARE` to the default and the rest evenly among the starters. `wear`
/// draws from an empty wardrobe, so a non-starter (#1833's, R1833000) is worn at a share of none.
fn slot_share(fit: &CosmeticFit, cosmetic: &Cosmetic) -> f64 {
    if !cosmetic.default && !cosmetic.starter {
        return 0.0;
    }
    let registry = registry();
    let of_kind = |id: &str| {
        registry
            .iter()
            .any(|entry| entry.id == id && entry.kind == cosmetic.kind)
    };
    if fit.slots.iter().any(|(id, _)| of_kind(id)) {
        return fit
            .slots
            .iter()
            .find(|(id, _)| *id == cosmetic.id)
            .map_or(0.0, |(_, share)| *share);
    }
    let starters = registry
        .iter()
        .filter(|entry| entry.kind == cosmetic.kind && entry.starter && !entry.default)
        .count();
    if cosmetic.default {
        BOT_COSMETIC_PRIOR_DEFAULT_SHARE
    } else {
        (1.0 - BOT_COSMETIC_PRIOR_DEFAULT_SHARE) / starters as f64
    }
}

/// Every share `wear` measured against what `fit` gives, with `wear_share` the fit's share of seats
/// printing Big D-fender's full art and `owned_share` its share of bots that own it.
fn assert_worn_as(fit: &CosmeticFit, wear_share: f64, owned_share: f64) {
    let worn = wear(fit);
    for cosmetic in registry().iter().filter(|cosmetic| {
        CARD_SLOT_KINDS.contains(&cosmetic.kind) || DECK_SLOT_KINDS.contains(&cosmetic.kind)
    }) {
        let expected = slot_share(fit, cosmetic);
        let measured = worn.account.get(&cosmetic.id).copied().unwrap_or(0.0);
        close(measured, expected, &format!("the account's {}", cosmetic.id));
        if expected == 0.0 {
            assert_eq!(measured, 0.0, "{} is worn at a share of none", cosmetic.id);
        }
        if DECK_SLOT_KINDS.contains(&cosmetic.kind) {
            let measured = worn.slots.get(&cosmetic.id).copied().unwrap_or(0.0);
            close(measured, expected, &format!("a deck's {}", cosmetic.id));
            if expected == 0.0 {
                assert_eq!(measured, 0.0, "{} is worn at a share of none", cosmetic.id);
            }
        }
    }
    close(worn.per_card, fit.per_card_share, "the per-card share");
    close(
        worn.cards_per_deck,
        fit.cards_per_deck_mean.round(),
        "the cards a picking deck picks",
    );
    close(worn.full_art_off, 1.0 - wear_share, "a full art switched off");
    for (held, share) in &worn.owning {
        let expected = if *held == holding("full-art-big-d-fender", Some("core-001")) {
            owned_share
        } else {
            BOT_COSMETIC_PRIOR_OWNERSHIP
        };
        close(*share, expected, &format!("owning {held:?}"));
    }
}

#[test]
fn r1834005_pool_shares_within_tolerance() {
    // No fit is pasted yet, so every age draws from the prior.
    for age in [0, 7, 8, 30, 31, 90, 91, 400] {
        assert_eq!(fit_for(BOT_COSMETIC_FITS, age), &BOT_COSMETIC_PRIOR, "age {age}");
    }
    // The prior: each slot the default at its share and the rest even among the starters, the
    // prior's per-card share and cards, full-art wear and ownership.
    assert_worn_as(
        &BOT_COSMETIC_PRIOR,
        BOT_COSMETIC_PRIOR_FULL_ART_WEAR,
        BOT_COSMETIC_PRIOR_OWNERSHIP,
    );

    // A fit with shares of its own, for its bucket's bots.
    let fit = fit_for(&SHARES, 20);
    assert_eq!(fit, &SHARES[0]);
    assert_worn_as(fit, 0.3, 0.4);
    // Spot checks of the rule `slot_share` restates.
    let worn = wear(fit);
    close(worn.slots["table-felt"], 0.7, "a deck's felt playmat");
    assert!(!worn.slots.contains_key("table-neon-grid"));
    assert!(!worn.account.contains_key("foil-cracked-ice"));
    close(
        worn.account["field-none"],
        BOT_COSMETIC_PRIOR_DEFAULT_SHARE,
        "the unlisted backdrop's default",
    );
}

/// A fit that lists two playmats nobody starts with: Marble worn by two seats in ten and owned by
/// four people in ten, Chessboard worn by one in ten and owned by one in four.
static OWNED_SHARES: CosmeticFit = CosmeticFit {
    max_age_days: Some(90),
    samples: BOT_COSMETIC_FIT_MIN_SAMPLES,
    slots: &[
        ("table-tavern", 0.3),
        ("table-wood", 0.2),
        ("table-felt", 0.2),
        ("table-marble", 0.2),
        ("table-chessboard", 0.1),
    ],
    per_card_share: 0.0,
    cards_per_deck_mean: BOT_COSMETIC_PRIOR_CARDS_PER_DECK,
    card_picks: &[],
    earned_wear: &[],
    full_art_wear: &[],
    ownership: &[
        CosmeticOwnership {
            cosmetic: "table-marble",
            card: None,
            share: 0.4,
        },
        CosmeticOwnership {
            cosmetic: "table-chessboard",
            card: None,
            share: 0.25,
        },
    ],
};

#[test]
fn r1834005_a_fitted_non_starter_is_worn_at_its_share_of_seats() {
    // A fitted share counts every seat, owners or not: a bot that owns a non-starter wears it more
    // often than the share, and the pool, whose wardrobes hold it at its owners' share, at the share.
    let registry = registry();
    let none = OwnedCosmetics::default();
    let eligible = vec![holding("table-marble", None), holding("table-chessboard", None)];
    let share = |count: usize| count as f64 / POOL as f64;
    let mut account: BTreeMap<String, usize> = BTreeMap::new();
    let mut slots: BTreeMap<String, usize> = BTreeMap::new();
    let mut owning: BTreeMap<Holding, usize> = BTreeMap::new();
    for n in 0..POOL {
        let seed = format!("owned-share:{n}");
        let wardrobe = draw_wardrobe(&seed, &OWNED_SHARES, &eligible, &none);
        for held in holdings(&wardrobe) {
            *owning.entry(held).or_default() += 1;
        }
        let picks = draw_account(&seed, registry, &OWNED_SHARES, &wardrobe);
        let deck = draw_slots(&seed, registry, &OWNED_SHARES, &wardrobe);
        assert_owned(&picks, &[&deck], &wardrobe);
        let table = picks.get(CosmeticKind::Table).expect("a playmat");
        *account.entry(table.to_string()).or_default() += 1;
        let table = deck.get(CosmeticKind::Table).expect("a playmat");
        *slots.entry(table.to_string()).or_default() += 1;
    }
    for ownership in OWNED_SHARES.ownership {
        let held = holding(ownership.cosmetic, ownership.card);
        close(
            share(owning.get(&held).copied().unwrap_or(0)),
            ownership.share,
            &format!("owning {held:?}"),
        );
    }
    for (id, expected) in OWNED_SHARES.slots {
        for (what, worn) in [("the account's", &account), ("a deck's", &slots)] {
            close(
                share(worn.get(*id).copied().unwrap_or(0)),
                *expected,
                &format!("{what} {id}"),
            );
        }
    }
}

/// A fit as `cosmetics-fit` prints one: the seats wear the card kinds' defaults alone (an earned
/// foil is owned only on a card, so no seat's slot ever wears one), every seat picks one card of its
/// own, the starter Rainbow foil when a starter, and one person in two owns the earned Glitter foil
/// on Big D-fender's card, three in five of whom wear it there when they play it.
static CARD_SHARES: CosmeticFit = CosmeticFit {
    max_age_days: Some(90),
    samples: BOT_COSMETIC_FIT_MIN_SAMPLES,
    slots: &[
        ("foil-none", 1.0),
        ("finish-plain", 1.0),
        ("border-standard", 1.0),
    ],
    per_card_share: 1.0,
    cards_per_deck_mean: 1.0,
    card_picks: &[("foil-rainbow", 1.0)],
    earned_wear: &[CosmeticWear {
        cosmetic: "foil-glitter",
        card: "core-001",
        share: 0.6,
    }],
    full_art_wear: &[],
    ownership: &[CosmeticOwnership {
        cosmetic: "foil-glitter",
        card: Some("core-001"),
        share: 0.5,
    }],
};

#[test]
fn r1834005_a_fitted_cards_picks_wear_an_earned_foil_at_its_share() {
    // Under a fitted row the seat's slots list the card kinds' defaults alone, yet a bot's cards
    // still pick: the earned foil, worn only where its wardrobe holds it, by the bots that play its
    // card at its owners' share times its wear there (one in two times three in five), however many
    // other cards their deck holds, and a starter on the cards the fit's mean still leaves to dress.
    let registry = registry();
    let none = OwnedCosmetics::default();
    let eligible = vec![holding("foil-glitter", Some("core-001"))];
    let worn_by_people = 0.5 * 0.6;
    let share = |count: usize| count as f64 / POOL as f64;
    for size in [1, 2, 15] {
        let deck: Vec<String> = (1..=size).map(|n| format!("core-{n:03}")).collect();
        let mut worn: BTreeMap<String, usize> = BTreeMap::new();
        for n in 0..POOL {
            let seed = format!("card-share:{size}:{n}");
            let wardrobe = draw_wardrobe(&seed, &CARD_SHARES, &eligible, &none);
            let picks = draw_account(&seed, registry, &CARD_SHARES, &wardrobe);
            assert_eq!(picks.get(CosmeticKind::Foil), Some("foil-none"), "{seed}");
            let cards = draw_card_picks(
                &seed,
                registry,
                &CARD_SHARES,
                &wardrobe,
                &picks,
                &[deck.as_slice()],
            );
            let [card] = cards.as_slice() else {
                panic!("{seed}: one card of its own, the fit's mean, not {cards:?}");
            };
            let foil = card
                .foil
                .clone()
                .unwrap_or_else(|| panic!("{seed}: {card:?} picks no foil"));
            if foil == "foil-glitter" {
                assert_eq!(card.card_id, "core-001", "{seed}: Glitter off its card");
                assert!(
                    holdings(&wardrobe).contains(&holding("foil-glitter", Some("core-001"))),
                    "{seed}: wears a Glitter foil it does not own"
                );
            }
            *worn.entry(foil).or_default() += 1;
        }
        assert_eq!(worn.len(), 2, "{size} cards: {worn:?}");
        let glitter = share(worn.get("foil-glitter").copied().unwrap_or(0));
        close(
            glitter,
            worn_by_people,
            &format!("{size} cards: Glitter on core-001"),
        );
        close(
            share(worn.get("foil-rainbow").copied().unwrap_or(0)),
            1.0 - worn_by_people,
            &format!("{size} cards: Rainbow"),
        );
    }
}

#[test]
fn r1834005_an_aging_bot_owns_at_its_ages_share_not_more() {
    // The pool draws each refresh from a seed of its own; a bot that ages six times still owns Big
    // D-fender's full art at the share of people of its age that do, since its roll for it is the
    // one its first draw made.
    static AGING: [CosmeticFit; 1] = [CosmeticFit {
        max_age_days: Some(90),
        samples: BOT_COSMETIC_FIT_MIN_SAMPLES,
        slots: &[],
        per_card_share: 0.0,
        cards_per_deck_mean: BOT_COSMETIC_PRIOR_CARDS_PER_DECK,
        card_picks: &[],
        earned_wear: &[],
        full_art_wear: &[],
        ownership: &[CosmeticOwnership {
            cosmetic: "full-art-big-d-fender",
            card: Some("core-001"),
            share: 0.4,
        }],
    }];
    const AGINGS: i64 = 6;
    let registry = registry();
    let dfender = holding("full-art-big-d-fender", Some("core-001"));
    let eligible = vec![dfender.clone()];
    let born = NOW - 40 * DAY;
    let deck = deck_of(1);
    let mut owners = 0;
    for n in 0..POOL {
        let mut aging = bot(&format!("bot-aging-{n}"), born, vec![dealt(&deck)]);
        let first = redraw(
            &format!("refresh-0:{n}"),
            &aging,
            &AGING,
            registry,
            &eligible,
            NOW,
        )
        .expect("a first draw");
        let owned_first = holdings(&first.cosmetics.wardrobe).contains(&dfender);
        aging.decks = first.decks.expect("its deck, given slots");
        aging.cosmetics = Some(first.cosmetics);
        for refresh in 1..=AGINGS {
            let at = NOW + refresh * BOT_COSMETIC_REDRAW_DAYS * DAY;
            assert_eq!(fit_for(&AGING, (at - born) / DAY), &AGING[0]);
            let aged = redraw(
                &format!("refresh-{refresh}:{n}"),
                &aging,
                &AGING,
                registry,
                &eligible,
                at,
            )
            .expect("a bot past BOT_COSMETIC_REDRAW_DAYS ages");
            if let Some(decks) = aged.decks {
                aging.decks = decks;
            }
            aging.cosmetics = Some(aged.cosmetics);
        }
        let cosmetics = aging.cosmetics.as_ref().expect("its cosmetics");
        let owned = holdings(&cosmetics.wardrobe).contains(&dfender);
        assert_eq!(
            owned, owned_first,
            "bot {n}: its age's share did not change, nor did it"
        );
        assert_eq!(
            cosmetics.seed,
            format!("refresh-0:{n}"),
            "the first draw's seed is kept"
        );
        if owned {
            owners += 1;
        }
    }
    close(
        owners as f64 / POOL as f64,
        0.4,
        "owning Big D-fender's full art after aging",
    );
}

// ---------------------------------------------------------------------------------------------
// Drawn again: a deck dealt again, and a bot that ages
// ---------------------------------------------------------------------------------------------

/// A bot up to a week old picks something of its own for cards of every deck; one up to a month
/// old owns Big D-fender's full art once it is eligible and never prints it.
static REDRAW_FITS: [CosmeticFit; 2] = [
    CosmeticFit {
        max_age_days: Some(7),
        samples: BOT_COSMETIC_FIT_MIN_SAMPLES,
        slots: &[],
        per_card_share: 1.0,
        cards_per_deck_mean: BOT_COSMETIC_PRIOR_CARDS_PER_DECK,
        card_picks: &[],
        earned_wear: &[],
        full_art_wear: &[],
        ownership: &[],
    },
    CosmeticFit {
        max_age_days: Some(30),
        samples: BOT_COSMETIC_FIT_MIN_SAMPLES,
        slots: &[],
        per_card_share: 1.0,
        cards_per_deck_mean: BOT_COSMETIC_PRIOR_CARDS_PER_DECK,
        card_picks: &[],
        earned_wear: &[],
        full_art_wear: &[("full-art-big-d-fender", 0.0)],
        ownership: &[CosmeticOwnership {
            cosmetic: "full-art-big-d-fender",
            card: Some("core-001"),
            share: 1.0,
        }],
    },
];

/// Every slot of `account` and `slots` is one of its kind that `owned` owns.
#[track_caller]
fn assert_owned(account: &AccountPicks, slots: &[&DeckSlots], owned: &OwnedCosmetics) {
    let registry = registry();
    for kind in CARD_SLOT_KINDS.into_iter().chain(DECK_SLOT_KINDS) {
        let id = account.get(kind).expect("a pick of every kind");
        assert!(owns(entry(registry, kind, id), None, owned), "{id}");
    }
    for deck in slots {
        for kind in DECK_SLOT_KINDS {
            let id = deck.get(kind).expect("a slot of every kind");
            assert!(owns(entry(registry, kind, id), None, owned), "{id}");
        }
    }
}

#[test]
fn r1834005_redealt_decks_draw_again() {
    let registry = registry();
    let nothing: Vec<Holding> = Vec::new();
    let born = NOW - 3 * DAY;
    assert_eq!(fit_for(&REDRAW_FITS, 3), &REDRAW_FITS[0]);
    let (a, b, c) = (deck_of(1), deck_of(26), deck_of(51));
    let mut dressed = bot("bot-redealt", born, vec![dealt(&a), dealt(&b)]);

    // A bot with no decks draws nothing; one with decks and nothing drawn draws everything.
    assert_eq!(
        redraw(
            "bare",
            &bot("bot-bare", born, Vec::new()),
            &REDRAW_FITS,
            registry,
            &nothing,
            NOW
        ),
        None
    );
    let first = redraw("first", &dressed, &REDRAW_FITS, registry, &nothing, NOW).expect("a first draw");
    let decks = first.decks.clone().expect("its decks, given slots");
    assert_eq!(
        decks.iter().map(|deck| &deck.cards).collect::<Vec<_>>(),
        vec![&a, &b]
    );
    let slots: Vec<&DeckSlots> = decks.iter().filter_map(|deck| deck.slots.as_ref()).collect();
    assert_eq!(slots.len(), 2, "every deck wears slots");
    assert_owned(&first.cosmetics.account, &slots, &first.cosmetics.wardrobe);
    assert_eq!(first.trio, None, "it holds no trio");
    assert_eq!(first.cosmetics.drawn_at, NOW);
    let cards = picked(&first.cosmetics.cards);
    assert!(
        cards.iter().any(|card| a.iter().any(|id| id == card)),
        "{cards:?}"
    );
    assert!(
        cards.iter().any(|card| b.iter().any(|id| id == card)),
        "{cards:?}"
    );
    dressed.decks = decks;
    dressed.cosmetics = Some(first.cosmetics.clone());

    // Nothing is due: every deck wears its slots and BOT_COSMETIC_REDRAW_DAYS have not passed.
    for at in [NOW, NOW + BOT_COSMETIC_REDRAW_DAYS * DAY - 1] {
        assert_eq!(
            redraw("nothing-due", &dressed, &REDRAW_FITS, registry, &nothing, at),
            None
        );
    }

    // The second deck is dealt again, with no slots: it draws them, and the cards' picks follow
    // the decks the bot holds now. The other deck, the wardrobe, the account and the clock stay.
    dressed.decks[1] = dealt(&c);
    let later = NOW + DAY;
    let second = redraw("second", &dressed, &REDRAW_FITS, registry, &nothing, later)
        .expect("a deck dealt again draws");
    let decks = second.decks.clone().expect("its decks, the new one given slots");
    assert_eq!(decks[0], dressed.decks[0], "the deck it kept keeps its slots");
    assert_eq!(decks[1].cards, c);
    assert!(decks[1].slots.is_some(), "the deck dealt again wears slots");
    assert_eq!(second.cosmetics.wardrobe, first.cosmetics.wardrobe);
    assert_eq!(second.cosmetics.account, first.cosmetics.account);
    assert_eq!(second.cosmetics.drawn_at, NOW, "no aging, so the clock stays");
    let cards = picked(&second.cosmetics.cards);
    assert!(
        cards.iter().all(|card| a.iter().chain(&c).any(|id| id == card)),
        "a pick of a card it no longer plays: {cards:?}"
    );
    assert!(
        cards.iter().any(|card| c.iter().any(|id| id == card)),
        "{cards:?}"
    );
    dressed.decks = decks;
    dressed.cosmetics = Some(second.cosmetics.clone());

    // A trio dealt (its decks with no slots) draws them, and leaves the Best-of-1 decks be.
    dressed.trio = Some(dealt_trio([&a, &b, &c]));
    let third =
        redraw("third", &dressed, &REDRAW_FITS, registry, &nothing, later).expect("a trio dealt draws");
    assert_eq!(third.decks, None);
    let trio = third.trio.clone().expect("its trio, given slots");
    let (x, y, z) = &trio.decks;
    let trio_slots: Vec<&DeckSlots> = [x, y, z]
        .into_iter()
        .filter_map(|deck| deck.slots.as_ref())
        .collect();
    assert_eq!(trio_slots.len(), 3, "every trio deck wears slots");
    assert_owned(&third.cosmetics.account, &trio_slots, &third.cosmetics.wardrobe);
    dressed.trio = Some(trio);
    dressed.cosmetics = Some(third.cosmetics.clone());

    // BOT_COSMETIC_REDRAW_DAYS after it drew, it ages, as a month-old account: it gains the full
    // art people came to own since (and never prints it), keeps every pick it still owns, and wears
    // only what it owns.
    let due = NOW + BOT_COSMETIC_REDRAW_DAYS * DAY;
    assert_eq!(fit_for(&REDRAW_FITS, (due - born) / DAY), &REDRAW_FITS[1]);
    let now_eligible = vec![holding("full-art-big-d-fender", Some("core-001"))];
    let aged = redraw("aged", &dressed, &REDRAW_FITS, registry, &now_eligible, due)
        .expect("a bot past BOT_COSMETIC_REDRAW_DAYS ages");
    assert_eq!(aged.cosmetics.drawn_at, due);
    assert_eq!(
        holdings(&aged.cosmetics.wardrobe),
        now_eligible.iter().cloned().collect::<BTreeSet<_>>()
    );
    let mut expected = third.cosmetics.cards.clone();
    match expected.iter_mut().find(|picks| picks.card_id == "core-001") {
        Some(picks) => picks.full_art = Some(false.into()),
        None => expected.push(CardPicks {
            card_id: "core-001".to_string(),
            full_art: Some(false.into()),
            ..CardPicks::default()
        }),
    }
    expected.sort_by(|left, right| left.card_id.cmp(&right.card_id));
    assert_eq!(aged.cosmetics.cards, expected);
    let decks = aged.decks.clone().unwrap_or_else(|| dressed.decks.clone());
    let trio = aged.trio.clone().unwrap_or_else(|| trio_of(&dressed));
    let (x, y, z) = &trio.decks;
    let worn: Vec<&DeckSlots> = decks
        .iter()
        .filter_map(|deck| deck.slots.as_ref())
        .chain([x, y, z].into_iter().filter_map(|deck| deck.slots.as_ref()))
        .collect();
    assert_eq!(worn.len(), 5, "every deck still wears slots");
    assert_owned(&aged.cosmetics.account, &worn, &aged.cosmetics.wardrobe);

    // Aged, it is due again only BOT_COSMETIC_REDRAW_DAYS later.
    dressed.decks = decks;
    dressed.trio = Some(trio);
    dressed.cosmetics = Some(aged.cosmetics);
    assert_eq!(
        redraw(
            "after",
            &dressed,
            &REDRAW_FITS,
            registry,
            &now_eligible,
            due + DAY
        ),
        None
    );
}

fn trio_of(bot: &BotRow) -> FrozenTrio {
    bot.trio.clone().expect("the bot's trio")
}

// ---------------------------------------------------------------------------------------------
// Never in the ledger
// ---------------------------------------------------------------------------------------------

/// The bot `r1834005_never_in_the_ledger` makes, by a known id so its refresh seed can be chosen.
const LEDGER_BOT: &str = "00000000-0000-4000-8000-000001834005";

#[tokio::test(start_paused = true)]
async fn r1834005_never_in_the_ledger() {
    let app = empty_test_app().await;
    let now = now_ms();

    // BOT_COSMETIC_MIN_OWNERS people own every full art on its card, granted a day past the floor.
    let granted_at = now - (BOT_COSMETIC_MIN_AGE_DAYS + 1) * DAY;
    let arts: Vec<&Cosmetic> = registry()
        .iter()
        .filter(|cosmetic| cosmetic.kind == CosmeticKind::FullArt)
        .collect();
    for n in 0..BOT_COSMETIC_MIN_OWNERS {
        let input = ProfileCreateInput {
            user_id: format!("r1834005-person-{n}"),
            email: format!("r1834005-person-{n}@example.test"),
            rating: 1000.0,
            at: granted_at,
        };
        let person = q!(app, profiles_create(&input));
        for full_art in &arts {
            let grant = CosmeticGrant {
                profile_id: person.id.clone(),
                card_id: full_art.card.clone(),
                cosmetic_id: full_art.id.clone(),
                delta: 1,
                reason: EconomyReason::PackOpen,
                reference: None,
                dev: false,
                at: granted_at,
            };
            assert!(q!(app, economy_cosmetics_grant(&grant)));
        }
    }
    let owners = q!(app, economy_cosmetic_owners());
    let listed = eligible(registry(), &owners, now);
    assert_eq!(listed, {
        let mut all = full_arts();
        all.sort();
        all
    });

    // One bot of a known id, and a refresh seed that draws it a wardrobe of its own.
    let created_at = now - 40 * DAY;
    q!(
        app,
        bots_create(&NewBot {
            profile_id: LEDGER_BOT.to_string(),
            strength: 0,
            speed: 1.0,
            persona: PersonaName::Balanced,
            active_from_hour: 0,
            active_hours: 24,
            decks: Vec::new(),
            decks_catalog_version: None,
            created_at,
            seed_rating: BOT_STRENGTHS[0].seed_rating,
        })
    );
    let fit = fit_for(BOT_COSMETIC_FITS, (now - created_at) / DAY);
    let (seed, worn) = (0..SEED_SEARCH)
        .map(|n| format!("r1834005-ledger-{n}"))
        .find_map(|seed| {
            let drawn = draw_wardrobe(
                &format!("{seed}:{LEDGER_BOT}"),
                fit,
                &listed,
                &OwnedCosmetics::default(),
            );
            (!drawn.cards.is_empty()).then_some((seed, drawn))
        })
        .expect("a seed that draws the bot a wardrobe");

    let done = refresh_pool(&app, &seed, now).await.expect("a refresh");
    let bots = q!(app, bots_list());
    assert_eq!(done.dressed, bots.len(), "every active bot is dressed");
    let ours = bots
        .iter()
        .find(|bot| bot.profile_id == LEDGER_BOT)
        .expect("the bot");
    let cosmetics = ours.cosmetics.as_ref().expect("its cosmetics, on its row");
    assert_eq!(cosmetics.wardrobe, worn);
    assert_eq!(cosmetics.drawn_at, now);
    assert!(ours.decks.iter().all(|deck| deck.slots.is_some()));
    let (x, y, z) = &ours.trio.as_ref().expect("its trio").decks;
    assert!([x, y, z].into_iter().all(|deck| deck.slots.is_some()));

    // Its wardrobe and picks are on its row and nowhere else: no ledger row, no holding (what
    // `GET /api/cosmetics` reads), no card picks row, no collection, and no owner count of people.
    for bot in &bots {
        assert!(bot.cosmetics.is_some(), "{} is dressed", bot.profile_id);
        assert_eq!(
            q!(app, economy_cosmetics_get(&bot.profile_id)),
            OwnedCosmetics::default()
        );
        assert_eq!(
            q!(app, economy_ledger_get(&bot.profile_id)),
            EconomyLedger::default()
        );
        assert!(q!(app, economy_card_picks_get(&bot.profile_id)).is_empty());
        assert!(q!(app, collection_get(&bot.profile_id)).is_empty());
    }
    assert_eq!(q!(app, economy_cosmetic_owners()), owners);

    // A refresh with nothing due dresses nobody and changes nothing.
    let again = refresh_pool(&app, &format!("{seed}-again"), now)
        .await
        .expect("a refresh");
    assert_eq!(again.dressed, 0);
    assert_eq!(q!(app, bots_list()), bots);
}
