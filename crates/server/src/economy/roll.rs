//! The card pack roll (SPEC §14.3, §14.4; #1301): pure, from a pack's pool, the account's
//! holdings and a seed to what each slot grants and the next pity counters. No store, no clock,
//! no I/O: `economy::packs` reads the holdings, draws the seed and writes the grants, and a
//! replay is [`roll_opening`] over the same holdings and seed (R1301003).
//!
//! - [`slot_rarities`]: the slots' rarities from the pity math's pack (R1301000).
//! - [`draw_card`]: a slot's card from the pool, leaning toward the cards the account does not
//!   own, from the nearest rarity the pool holds when it lacks the slot's (R1301001).
//! - [`duplicate_grant`]: the dupe chain past an unlock, a duplicate's cosmetic or the bars
//!   (R1301002).
//! - [`roll_opening`]: the whole opening, in R1301003's draw order. Every list it walks is a
//!   `Vec` or an `IndexMap` in catalog or registry order, so the same inputs draw the same.

use indexmap::IndexMap;
use jackioh_engine::config::MAX_COPIES;
use jackioh_engine::rng::Rng;
use jackioh_engine::wire::PrintedRarity;
use jackioh_engine::{CardDefs, SetName, set_ships};
use serde::{Deserialize, Serialize};

use crate::config::{
    COSMETIC_DUPLICATE_RATES, GLOBAL_PACK_NAME, PACK_FILLER_RARE_RATE, PACK_SLOTS, PACK_UNOWNED_WEIGHTS,
};
use crate::cosmetics::{Cosmetic, pack_cosmetics};
use crate::economy::math::{PackDraws, PackOutcome, PityCounters, roll_pack};
use crate::economy::store::CardCosmetic;

/// The slots a pack's tracked rarities take (R1301000): one each for Epic, Legendary and Mythic.
const TRACKED_RARITIES: usize = 3;

// A pack holding all three tracked rarities has a slot for each.
const _: () = assert!(PACK_SLOTS >= TRACKED_RARITIES);

/// Which card pack (§14.3): a set's, whose pool is that set, or the global one, whose pool is
/// every set that ships.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackKind {
    Set(SetName),
    Global,
}

impl PackKind {
    /// The pack a request names: `GLOBAL_PACK_NAME`, or a set that ships by its name (R1420).
    pub fn parse(text: &str) -> Option<PackKind> {
        if text == GLOBAL_PACK_NAME {
            return Some(PackKind::Global);
        }
        text.parse::<SetName>()
            .ok()
            .filter(|set| set_ships(*set))
            .map(PackKind::Set)
    }

    /// The name a request gives the pack, which its opening's log records.
    pub fn name(self) -> &'static str {
        match self {
            PackKind::Set(set) => set.as_str(),
            PackKind::Global => GLOBAL_PACK_NAME,
        }
    }
}

/// A card a pack can hold, with its printed rarity.
#[derive(Clone, Debug, PartialEq)]
pub struct PoolCard {
    pub id: String,
    pub rarity: PrintedRarity,
}

/// A pack's pool, in catalog order (§14.3): the non-token cards of its set or, for the global
/// pack, of every set that ships, never a card of a set that has not shipped (§14, R1420).
pub fn pack_pool(defs: &CardDefs, pack: PackKind) -> Vec<PoolCard> {
    defs.iter()
        .filter(|(_, def)| !def.is_token() && set_ships(def.set))
        .filter(|(_, def)| match pack {
            PackKind::Set(set) => def.set == set,
            PackKind::Global => true,
        })
        .filter_map(|(id, def)| {
            Some(PoolCard {
                id: id.clone(),
                rarity: def.rarity.as_str().parse().ok()?,
            })
        })
        .collect()
}

/// What the account holds as the pack opens: its copies of each card, its per-card cosmetics
/// and its pity counters (R1298000).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Holdings {
    pub owned: IndexMap<String, i64>,
    pub cosmetics: Vec<CardCosmetic>,
    pub pity: PityCounters,
}

/// What one slot grants (§14.3's dupe chain, R1301002).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SlotGrant {
    /// A new card: a copy of it.
    Card,
    /// A duplicate: a per-card cosmetic of the rarity it rolled, for its card.
    Cosmetic {
        cosmetic_id: String,
        cosmetic_rarity: PrintedRarity,
    },
    /// A spent duplicate: nothing of the rarity it rolled was left to own, so its points go to
    /// both token bars (§14.5).
    Bars { cosmetic_rarity: PrintedRarity },
}

/// One slot of an opened pack: the rarity it rolled, which the pity counters read (§14.4), the
/// card it drew and what that granted.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Slot {
    pub rarity: PrintedRarity,
    pub card_id: String,
    pub grant: SlotGrant,
}

/// An opening rolled: its slots in order, the pity counters after it and the draws it took.
#[derive(Clone, Debug, PartialEq)]
pub struct Opening {
    pub slots: Vec<Slot>,
    pub pity: PityCounters,
    pub draws: u32,
}

/// What every economy row of an opening logs in its `ref` (R1301003, §14.11): everything the roll
/// read but the holdings before it (the pack, the catalog version its pool came from, the cosmetics
/// it could drop and the seed) and its result (the slots, the pity counters after it and the draws
/// taken). [`roll_opening`] over that catalog's pool, the holdings before it and `cosmetics`, with
/// `seed`, replays it however the live catalog or registry has moved since.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OpeningLog {
    pub pack: String,
    /// The catalog version the pool was built from (`jackioh_cards::catalog_version()` then;
    /// `jackioh_cards::catalog_defs_at` holds every older one).
    pub catalog: String,
    pub seed: String,
    /// The registry's droppable entries the roll picked from ([`pack_cosmetics`]), whole, in
    /// registry order.
    pub cosmetics: Vec<Cosmetic>,
    pub slots: Vec<Slot>,
    pub pity: PityCounters,
    pub draws: u32,
}

impl OpeningLog {
    /// The log of `opening`, rolled from `pack`'s pool in catalog `catalog`, over `registry`, with
    /// `seed`.
    pub fn of(
        pack: PackKind,
        catalog: &str,
        registry: &[Cosmetic],
        seed: &str,
        opening: &Opening,
    ) -> OpeningLog {
        OpeningLog {
            pack: pack.name().to_string(),
            catalog: catalog.to_string(),
            seed: seed.to_string(),
            cosmetics: pack_cosmetics(registry),
            slots: opening.slots.clone(),
            pity: opening.pity,
            draws: opening.draws,
        }
    }
}

/// A slot that holds none of the pack's tracked rarities (R1301000): Rare at
/// `PACK_FILLER_RARE_RATE`, else Common. One draw.
pub fn filler_rarity(rng: &mut Rng) -> PrintedRarity {
    if rng.next() < PACK_FILLER_RARE_RATE {
        PrintedRarity::Rare
    } else {
        PrintedRarity::Common
    }
}

/// The slots' rarities (R1301000): the first is the pack's best, its Rare-or-better slot (§14.3),
/// each other tracked rarity the pack holds takes the next, highest first, and the rest are
/// fillers, one draw each. So a pack holds a tracked rarity exactly when the pity math says it
/// does (§14.4).
pub fn slot_rarities(outcome: PackOutcome, rng: &mut Rng) -> Vec<PrintedRarity> {
    let best = outcome.best();
    let mut rarities = vec![best];
    for (held, rarity) in [
        (outcome.has_legendary, PrintedRarity::Legendary),
        (outcome.has_epic, PrintedRarity::Epic),
    ] {
        if held && rarity < best {
            rarities.push(rarity);
        }
    }
    while rarities.len() < PACK_SLOTS {
        rarities.push(filler_rarity(rng));
    }
    rarities
}

/// The rarities a slot of `rolled` takes its card from, in order (R1301001): its own, then each
/// lower one from the nearest down, then each higher one from the nearest up.
pub fn fallback_order(rolled: PrintedRarity) -> Vec<PrintedRarity> {
    let mut order = vec![rolled];
    order.extend(
        PrintedRarity::ALL
            .iter()
            .rev()
            .copied()
            .filter(|rarity| *rarity < rolled),
    );
    order.extend(
        PrintedRarity::ALL
            .iter()
            .copied()
            .filter(|rarity| *rarity > rolled),
    );
    order
}

/// A card's weight in a slot's draw (§14.3): `PACK_UNOWNED_WEIGHTS.unowned` while the account
/// owns fewer than `MAX_COPIES` of it, else `.owned`.
fn weight(owned: &IndexMap<String, i64>, card_id: &str) -> f64 {
    if owned.get(card_id).copied().unwrap_or(0) < i64::from(MAX_COPIES) {
        PACK_UNOWNED_WEIGHTS.unowned
    } else {
        PACK_UNOWNED_WEIGHTS.owned
    }
}

/// A slot's card (§14.3, R1301001): one of the pool's cards of the first rarity in
/// `fallback_order(rarity)` the pool holds, each by its weight. One draw; an empty pool is `None`
/// and draws nothing.
pub fn draw_card<'p>(
    pool: &'p [PoolCard],
    rarity: PrintedRarity,
    owned: &IndexMap<String, i64>,
    rng: &mut Rng,
) -> Option<&'p PoolCard> {
    let candidates: Vec<&PoolCard> = fallback_order(rarity)
        .into_iter()
        .map(|step| pool.iter().filter(|card| card.rarity == step).collect::<Vec<_>>())
        .find(|cards| !cards.is_empty())?;
    let total: f64 = candidates.iter().map(|card| weight(owned, &card.id)).sum();
    let mut point = rng.next() * total;
    for card in &candidates {
        point -= weight(owned, &card.id);
        if point < 0.0 {
            return Some(card);
        }
    }
    // Rounding can leave the point a hair past the last edge.
    candidates.last().copied()
}

/// `COSMETIC_DUPLICATE_RATES` by rarity, commonest first.
pub fn duplicate_rates() -> [(PrintedRarity, f64); 5] {
    let rates = COSMETIC_DUPLICATE_RATES;
    [
        (PrintedRarity::Common, rates.common),
        (PrintedRarity::Rare, rates.rare),
        (PrintedRarity::Epic, rates.epic),
        (PrintedRarity::Legendary, rates.legendary),
        (PrintedRarity::Mythic, rates.mythic),
    ]
}

/// The cosmetic rarity a duplicate's draw lands on (R1301002): the first whose running sum of
/// [`duplicate_rates`] passes the draw, and Mythic past the last edge. Its own scale: the card's
/// rarity plays no part.
pub fn cosmetic_rarity_at(draw: f64) -> PrintedRarity {
    let mut edge = 0.0;
    for (rarity, rate) in duplicate_rates() {
        edge += rate;
        if draw < edge {
            return rarity;
        }
    }
    PrintedRarity::Mythic
}

/// What a duplicate of `card_id` grants once its cosmetic rarity is rolled (R1301002): one of
/// `droppable` of that rarity the account does not own on that card, picked uniformly (one draw),
/// or, with none, points to the bars (no draw). `droppable` is `cosmetics::pack_cosmetics`'s; one
/// that names a card, a full art, is of that card alone (R1313000).
pub fn duplicate_grant(
    card_id: &str,
    cosmetic_rarity: PrintedRarity,
    droppable: &[Cosmetic],
    owned: &[CardCosmetic],
    rng: &mut Rng,
) -> SlotGrant {
    let unowned: Vec<&Cosmetic> = droppable
        .iter()
        .filter(|cosmetic| cosmetic.rarity.printed() == cosmetic_rarity)
        .filter(|cosmetic| cosmetic.card.as_deref().is_none_or(|card| card == card_id))
        .filter(|cosmetic| {
            !owned
                .iter()
                .any(|held| held.card_id == card_id && held.cosmetic_id == cosmetic.id)
        })
        .collect();
    match rng.pick(&unowned) {
        Some(cosmetic) => SlotGrant::Cosmetic {
            cosmetic_id: cosmetic.id.clone(),
            cosmetic_rarity,
        },
        None => SlotGrant::Bars { cosmetic_rarity },
    }
}

/// The whole opening (R1301003), pure from the pack's pool, the account's holdings before it, the
/// cosmetics registry and the opening's seed. Its draws, in this order and no other: the pity
/// math's three (Epic, Legendary, Mythic), one per filler slot, then slot by slot the card's one
/// and, for a duplicate, its cosmetic rarity's one and the pick's one when there is something to
/// pick. Ownership is read slot by slot, so a card unlocked earlier in the pack is a duplicate
/// later in it (R1301002). `None` for an empty pool.
pub fn roll_opening(
    pool: &[PoolCard],
    holdings: &Holdings,
    registry: &[Cosmetic],
    seed: &str,
) -> Option<Opening> {
    let droppable = pack_cosmetics(registry);
    let mut rng = Rng::new(seed, 0);
    let draws = PackDraws {
        epic: rng.next(),
        legendary: rng.next(),
        mythic: rng.next(),
    };
    let (outcome, pity) = roll_pack(holdings.pity, draws);
    let rarities = slot_rarities(outcome, &mut rng);
    let mut owned = holdings.owned.clone();
    let mut cosmetics = holdings.cosmetics.clone();
    let mut slots = Vec::with_capacity(rarities.len());
    for rarity in rarities {
        let card = draw_card(pool, rarity, &owned, &mut rng)?;
        let copies = owned.entry(card.id.clone()).or_insert(0);
        let grant = if *copies < i64::from(MAX_COPIES) {
            *copies += 1;
            SlotGrant::Card
        } else {
            let cosmetic_rarity = cosmetic_rarity_at(rng.next());
            let grant = duplicate_grant(&card.id, cosmetic_rarity, &droppable, &cosmetics, &mut rng);
            if let SlotGrant::Cosmetic { cosmetic_id, .. } = &grant {
                cosmetics.push(CardCosmetic {
                    card_id: card.id.clone(),
                    cosmetic_id: cosmetic_id.clone(),
                });
            }
            grant
        };
        slots.push(Slot {
            rarity,
            card_id: card.id.clone(),
            grant,
        });
    }
    Some(Opening {
        slots,
        pity,
        draws: rng.cursor(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{PACK_RATE_TOLERANCE, PITY_MYTHIC_PACKS};
    use crate::cosmetics::{CosmeticKind, CosmeticRarity};
    use jackioh_engine::wire::PrintedRarity::{Common, Epic, Legendary, Mythic, Rare};

    /// Draws enough for a rate to settle inside `PACK_RATE_TOLERANCE`.
    const DRAWS: usize = 20_000;

    fn card(id: &str, rarity: PrintedRarity) -> PoolCard {
        PoolCard {
            id: id.to_owned(),
            rarity,
        }
    }

    /// A per-card cosmetic a pack may drop.
    fn foil(id: &str, rarity: CosmeticRarity) -> Cosmetic {
        Cosmetic {
            id: id.to_owned(),
            kind: CosmeticKind::Foil,
            name: id.to_owned(),
            rarity,
            default: false,
            starter: false,
            card: None,
        }
    }

    /// One droppable foil of each cosmetic rarity the registry has.
    fn four_foils() -> Vec<Cosmetic> {
        vec![
            foil("foil-common", CosmeticRarity::Common),
            foil("foil-rare", CosmeticRarity::Rare),
            foil("foil-epic", CosmeticRarity::Epic),
            foil("foil-legendary", CosmeticRarity::Legendary),
        ]
    }

    fn owning(ids: &[&str]) -> IndexMap<String, i64> {
        ids.iter()
            .map(|id| ((*id).to_owned(), i64::from(MAX_COPIES)))
            .collect()
    }

    fn held(card_id: &str, cosmetic_id: &str) -> CardCosmetic {
        CardCosmetic {
            card_id: card_id.to_owned(),
            cosmetic_id: cosmetic_id.to_owned(),
        }
    }

    #[test]
    fn r1301000_slot_one_holds_the_best_then_the_other_held_rarities() {
        let mut rng = Rng::new("r1301000", 0);
        let all = PackOutcome {
            has_epic: true,
            has_legendary: true,
            has_mythic: true,
        };
        let slots = slot_rarities(all, &mut rng);
        assert_eq!(slots.len(), PACK_SLOTS);
        assert_eq!(slots[..TRACKED_RARITIES], [Mythic, Legendary, Epic]);
        assert!(slots[TRACKED_RARITIES..].iter().all(|rarity| *rarity <= Rare));

        let two = PackOutcome {
            has_epic: true,
            has_legendary: false,
            has_mythic: true,
        };
        let slots = slot_rarities(two, &mut rng);
        assert_eq!(slots.len(), PACK_SLOTS);
        assert_eq!(slots[..2], [Mythic, Epic]);
        assert!(slots[2..].iter().all(|rarity| *rarity <= Rare));
    }

    #[test]
    fn r1301000_a_pack_holding_none_has_a_rare_slot() {
        let mut rng = Rng::new("r1301000", 0);
        for _ in 0..100 {
            let slots = slot_rarities(PackOutcome::default(), &mut rng);
            assert_eq!(slots.len(), PACK_SLOTS);
            assert_eq!(slots[0], Rare);
            assert!(slots.iter().all(|rarity| *rarity <= Rare), "{slots:?}");
        }
    }

    #[test]
    fn r1301000_fillers_roll_rare_at_the_filler_rate() {
        let mut rng = Rng::new("r1301000", 0);
        let rares = (0..DRAWS).filter(|_| filler_rarity(&mut rng) == Rare).count();
        let share = rares as f64 / DRAWS as f64;
        assert!(
            (share - PACK_FILLER_RARE_RATE).abs() <= PACK_RATE_TOLERANCE,
            "Rare share {share} outside tolerance of {PACK_FILLER_RARE_RATE}"
        );
        assert_eq!(rng.cursor() as usize, DRAWS);
    }

    #[test]
    fn r1301001_a_rarity_the_pool_lacks_steps_down_then_up() {
        assert_eq!(fallback_order(Mythic), [Mythic, Legendary, Epic, Rare, Common]);
        assert_eq!(fallback_order(Epic), [Epic, Rare, Common, Legendary, Mythic]);
        assert_eq!(fallback_order(Common), [Common, Rare, Epic, Legendary, Mythic]);

        let mut rng = Rng::new("r1301001", 0);
        let owned = IndexMap::new();
        let mythic_only = [card("m", Mythic)];
        assert_eq!(
            draw_card(&mythic_only, Common, &owned, &mut rng),
            Some(&mythic_only[0])
        );
        // The nearest lower rarity before any higher one.
        let rare_and_legendary = [card("l", Legendary), card("r", Rare)];
        assert_eq!(
            draw_card(&rare_and_legendary, Epic, &owned, &mut rng).map(|card| card.id.as_str()),
            Some("r")
        );
        assert_eq!(rng.cursor(), 2);
        // An empty pool has no card and takes no draw.
        assert_eq!(draw_card(&[], Rare, &owned, &mut rng), None);
        assert_eq!(rng.cursor(), 2);
    }

    #[test]
    fn r1301001_the_slot_keeps_its_rolled_rarity_for_pity() {
        let pool = [card("c", Common), card("l", Legendary)];
        let holdings = Holdings {
            pity: PityCounters {
                mythic: PITY_MYTHIC_PACKS - 1,
                ..PityCounters::default()
            },
            ..Holdings::default()
        };
        let opening =
            roll_opening(&pool, &holdings, &[], "r1301001").unwrap_or_else(|| panic!("the pool holds cards"));
        assert_eq!(opening.slots[0].rarity, Mythic);
        assert_eq!(opening.slots[0].card_id, "l");
        assert_eq!(opening.pity.mythic, 0);
    }

    #[test]
    fn r1301002_an_unowned_card_unlocks_and_is_owned_for_the_rest_of_the_pack() {
        let pool = [card("c", Common)];
        let opening = roll_opening(&pool, &Holdings::default(), &[], "r1301002")
            .unwrap_or_else(|| panic!("the pool holds a card"));
        assert_eq!(opening.slots.len(), PACK_SLOTS);
        assert!(opening.slots.iter().all(|slot| slot.card_id == "c"));
        assert_eq!(opening.slots[0].grant, SlotGrant::Card);
        assert!(
            opening.slots[1..]
                .iter()
                .all(|slot| matches!(slot.grant, SlotGrant::Bars { .. })),
            "{:?}",
            opening.slots
        );
    }

    #[test]
    fn r1301002_a_duplicate_becomes_a_cosmetic_of_the_rolled_rarity_whatever_the_card_s() {
        let foils = four_foils();
        let mut rng = Rng::new("r1301002", 0);
        for rarity in CosmeticRarity::ALL {
            let grant = duplicate_grant("m", rarity.printed(), &foils, &[], &mut rng);
            let expected = foils
                .iter()
                .find(|cosmetic| cosmetic.rarity == rarity)
                .map(|cosmetic| cosmetic.id.clone())
                .unwrap_or_else(|| panic!("a foil of {rarity:?}"));
            assert_eq!(
                grant,
                SlotGrant::Cosmetic {
                    cosmetic_id: expected,
                    cosmetic_rarity: rarity.printed(),
                }
            );
        }

        // A Mythic card the account owns: its duplicates' cosmetics roll on their own scale, and
        // every one granted is of the rarity rolled.
        let pool = [card("m", Mythic)];
        let holdings = Holdings {
            owned: owning(&["m"]),
            ..Holdings::default()
        };
        let mut granted = Vec::new();
        for seed in 0..200 {
            let opening = roll_opening(&pool, &holdings, &foils, &format!("r1301002-{seed}"))
                .unwrap_or_else(|| panic!("the pool holds a card"));
            for slot in opening.slots {
                if let SlotGrant::Cosmetic {
                    cosmetic_id,
                    cosmetic_rarity,
                } = slot.grant
                {
                    let cosmetic = foils
                        .iter()
                        .find(|cosmetic| cosmetic.id == cosmetic_id)
                        .unwrap_or_else(|| panic!("{cosmetic_id} is a fixture foil"));
                    assert_eq!(cosmetic.rarity.printed(), cosmetic_rarity);
                    granted.push(cosmetic_rarity);
                }
            }
        }
        assert!(granted.contains(&Common), "{granted:?}");
        assert!(granted.contains(&Rare), "{granted:?}");
    }

    #[test]
    fn r1301002_a_rolled_rarity_with_nothing_unowned_goes_to_the_bars() {
        let foils = four_foils();
        let mut rng = Rng::new("r1301002", 0);
        // The card's Legendary foil is owned: Bars, with no draw.
        let owned = [held("m", "foil-legendary")];
        assert_eq!(
            duplicate_grant("m", Legendary, &foils, &owned, &mut rng),
            SlotGrant::Bars {
                cosmetic_rarity: Legendary
            }
        );
        assert_eq!(rng.cursor(), 0);
        // Owned on another card is not owned on this one.
        assert_eq!(
            duplicate_grant("c", Legendary, &foils, &owned, &mut rng),
            SlotGrant::Cosmetic {
                cosmetic_id: "foil-legendary".to_owned(),
                cosmetic_rarity: Legendary,
            }
        );
        // No cosmetic is Mythic.
        assert_eq!(
            duplicate_grant("m", Mythic, &foils, &[], &mut rng),
            SlotGrant::Bars {
                cosmetic_rarity: Mythic
            }
        );
        // A starter is owned by everyone already, so the pack leaves it out.
        let starter = Cosmetic {
            starter: true,
            ..foil("foil-starter", CosmeticRarity::Legendary)
        };
        assert_eq!(
            duplicate_grant("m", Legendary, &pack_cosmetics(&[starter]), &[], &mut rng),
            SlotGrant::Bars {
                cosmetic_rarity: Legendary
            }
        );
    }

    #[test]
    fn r1301002_a_full_art_drops_on_its_own_card_alone() {
        let full_art = Cosmetic {
            kind: CosmeticKind::FullArt,
            card: Some("m".to_owned()),
            ..foil("full-art-m", CosmeticRarity::Epic)
        };
        let droppable = pack_cosmetics(&[full_art]);
        let mut rng = Rng::new("r1301002-full-art", 0);
        // A duplicate of its card that rolls Epic drops it.
        assert_eq!(
            duplicate_grant("m", Epic, &droppable, &[], &mut rng),
            SlotGrant::Cosmetic {
                cosmetic_id: "full-art-m".to_owned(),
                cosmetic_rarity: Epic,
            }
        );
        // Any other card's Epic duplicate has nothing to drop, and neither has its card once it
        // owns it.
        assert_eq!(
            duplicate_grant("c", Epic, &droppable, &[], &mut rng),
            SlotGrant::Bars {
                cosmetic_rarity: Epic
            }
        );
        assert_eq!(
            duplicate_grant("m", Epic, &droppable, &[held("m", "full-art-m")], &mut rng),
            SlotGrant::Bars {
                cosmetic_rarity: Epic
            }
        );
    }

    #[test]
    fn r1301002_cosmetic_rarity_follows_the_duplicate_rates() {
        let rates = duplicate_rates();
        let total: f64 = rates.iter().map(|(_, rate)| rate).sum();
        assert!((total - 1.0).abs() < 1e-9, "the duplicate rates sum to {total}");
        assert_eq!(
            rates.map(|(rarity, _)| rarity),
            [Common, Rare, Epic, Legendary, Mythic]
        );
        let mut floor = 0.0;
        for (rarity, rate) in rates {
            let edge = floor + rate;
            assert_eq!(cosmetic_rarity_at(floor), rarity, "at {floor}");
            assert_eq!(
                cosmetic_rarity_at((floor + edge) / 2.0),
                rarity,
                "inside {rarity:?}"
            );
            floor = edge;
        }
        assert_eq!(cosmetic_rarity_at(1.0 - f64::EPSILON), Mythic);
    }

    #[test]
    fn r1301002_the_draw_leans_toward_unowned_cards() {
        let pool = [card("owned", Common), card("new", Common)];
        let owned = owning(&["owned"]);
        let mut rng = Rng::new("r1301002-lean", 0);
        let new = (0..DRAWS)
            .filter(|_| {
                draw_card(&pool, Common, &owned, &mut rng).map(|card| card.id.as_str()) == Some("new")
            })
            .count();
        let share = new as f64 / DRAWS as f64;
        let expected =
            PACK_UNOWNED_WEIGHTS.unowned / (PACK_UNOWNED_WEIGHTS.owned + PACK_UNOWNED_WEIGHTS.unowned);
        assert!(
            (share - expected).abs() <= PACK_RATE_TOLERANCE,
            "unowned share {share} outside tolerance of {expected}"
        );
    }

    #[test]
    fn r1301003_the_same_seed_and_holdings_roll_the_same_opening() {
        let pool: Vec<PoolCard> = (0..40)
            .map(|at| {
                card(
                    &format!("card-{at}"),
                    *PrintedRarity::ALL.get(at % 5).unwrap_or(&Common),
                )
            })
            .collect();
        let holdings = Holdings {
            owned: owning(&["card-0", "card-1", "card-2"]),
            cosmetics: vec![held("card-0", "foil-common")],
            pity: PityCounters {
                epic: 3,
                legendary: 7,
                mythic: 11,
            },
        };
        let foils = four_foils();
        let first = roll_opening(&pool, &holdings, &foils, "r1301003");
        assert!(first.is_some());
        assert_eq!(roll_opening(&pool, &holdings, &foils, "r1301003"), first);
        assert_ne!(roll_opening(&pool, &holdings, &foils, "r1301003-other"), first);
        // An empty pool rolls nothing.
        assert_eq!(roll_opening(&[], &holdings, &foils, "r1301003"), None);
    }

    #[test]
    fn r1301003_the_log_holds_the_result_and_replays_after_the_registry_moves() {
        let pool = vec![card("owned", Common)];
        let holdings = Holdings {
            owned: owning(&["owned"]),
            ..Holdings::default()
        };
        let mut registry = four_foils();
        registry.push(Cosmetic {
            starter: true,
            ..foil("foil-starter", CosmeticRarity::Rare)
        });
        let opening = roll_opening(&pool, &holdings, &registry, "r1301003").expect("an opening");
        assert!(
            opening
                .slots
                .iter()
                .any(|slot| matches!(slot.grant, SlotGrant::Cosmetic { .. }))
        );
        let log = OpeningLog::of(
            PackKind::Set(SetName::Core),
            "catalog-v1",
            &registry,
            "r1301003",
            &opening,
        );
        let log: OpeningLog =
            serde_json::from_str(&serde_json::to_string(&log).expect("a log")).expect("the log read back");
        assert_eq!(log.pack, "Core");
        assert_eq!(log.catalog, "catalog-v1");
        assert_eq!(log.seed, "r1301003");
        // The result itself is logged, and only the droppable entries are.
        assert_eq!(log.slots, opening.slots);
        assert_eq!(log.pity, opening.pity);
        assert_eq!(log.draws, opening.draws);
        assert_eq!(log.cosmetics, four_foils());
        // The live registry moves on: its foils become starters, so a roll over it now drops
        // nothing. The logged entries still replay the opening exactly.
        let moved: Vec<Cosmetic> = registry
            .iter()
            .map(|cosmetic| Cosmetic {
                starter: true,
                ..cosmetic.clone()
            })
            .collect();
        assert_ne!(
            roll_opening(&pool, &holdings, &moved, &log.seed),
            Some(opening.clone())
        );
        let replayed = roll_opening(&pool, &holdings, &log.cosmetics, &log.seed).expect("the replay");
        assert_eq!(replayed.slots, log.slots);
        assert_eq!(replayed.pity, log.pity);
        assert_eq!(replayed.draws, log.draws);
    }

    #[test]
    fn a_pack_names_a_shipped_set_or_the_global_pack() {
        assert_eq!(PackKind::parse(GLOBAL_PACK_NAME), Some(PackKind::Global));
        assert_eq!(PackKind::parse("Core"), Some(PackKind::Set(SetName::Core)));
        assert_eq!(
            PackKind::parse("Classic+"),
            Some(PackKind::Set(SetName::ClassicPlus))
        );
        for unknown in ["Boss", "Boss-X", "Custom", "core", ""] {
            assert_eq!(PackKind::parse(unknown), None, "{unknown:?}");
        }
        for pack in [PackKind::Global, PackKind::Set(SetName::MeditativePlus)] {
            assert_eq!(PackKind::parse(pack.name()), Some(pack));
        }
    }

    #[test]
    fn a_pack_s_pool_is_its_set_s_non_token_cards() {
        let defs: CardDefs = serde_json::from_str(jackioh_cards::catalog_json())
            .unwrap_or_else(|error| panic!("the catalog: {error}"));
        let global = pack_pool(&defs, PackKind::Global);
        let shipped: Vec<&String> = defs
            .iter()
            .filter(|(_, def)| !def.is_token() && set_ships(def.set))
            .map(|(id, _)| id)
            .collect();
        assert_eq!(global.iter().map(|card| &card.id).collect::<Vec<_>>(), shipped);
        let mut per_set = 0;
        for set in jackioh_engine::SHIPPED_SETS {
            let pool = pack_pool(&defs, PackKind::Set(set));
            assert!(!pool.is_empty(), "{set}'s pool");
            assert!(pool.iter().all(|card| defs[&card.id].set == set));
            per_set += pool.len();
        }
        assert_eq!(per_set, global.len());
    }
}
