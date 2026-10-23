//! The card pack roll (SPEC §14.3, §14.4; #1301, #1339): pure, from a pack's pool, the account's
//! holdings and a seed to what each slot grants and the next pity counters. No store, no clock,
//! no I/O: `economy::packs` reads the holdings, draws the seed and writes the grants, and a
//! replay is [`roll_opening`] over the same holdings and seed (R1301003, R1339002).
//!
//! - [`slot_tiers`]: each slot's card rarity, a tiered roll on `CARD_TIER_WEIGHTS`, the floored
//!   slot's at `PACK_FLOOR_RARITY` (R1339000); pity forces on top of them (`math::apply_pity`,
//!   R1339001).
//! - [`draw_card`]: a slot's card from the pool, leaning toward the cards the account does not
//!   own, from the nearest rarity the pool holds when it lacks the slot's (R1301001).
//! - [`global_grant`]: a duplicate's global roll before its per-card one, while an account
//!   cosmetic is left unowned: one draw, and on a hit an unowned account cosmetic of a tiered roll
//!   floored at `GLOBAL_COSMETIC_FLOOR_RARITY` (R1661001).
//! - [`duplicate_grant`]: the dupe chain past an unlock, a duplicate's cosmetic, its rarity its
//!   own tiered roll on `COSMETIC_TIER_WEIGHTS`, or the bars (R1301002).
//! - [`roll_opening`]: the whole opening, in R1339002's draw order. Every list it walks is a
//!   `Vec` or an `IndexMap` in catalog or registry order, so the same inputs draw the same.

use indexmap::IndexMap;
use jackioh_engine::config::MAX_COPIES;
use jackioh_engine::rng::Rng;
use jackioh_engine::wire::PrintedRarity;
use jackioh_engine::{CardDefs, SetName, set_ships};
use serde::{Deserialize, Serialize};

use crate::config::{
    GLOBAL_COSMETIC_FLOOR_RARITY, GLOBAL_COSMETIC_ROLL_WEIGHT, GLOBAL_PACK_NAME, PACK_FLOOR_RARITY,
    PACK_FLOORED_SLOT, PACK_SLOTS, PACK_UNOWNED_WEIGHTS,
};
use crate::cosmetics::{Cosmetic, CosmeticRarity, global_cosmetics, pack_cosmetics};
use crate::economy::math::{PityCounters, apply_pity};
use crate::economy::store::CardCosmetic;
use crate::economy::tiers::{card_tier, cosmetic_tier, tier_draw};

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

/// What the account holds as the pack opens: its copies of each card, its per-card cosmetics,
/// its account cosmetics (R1661001) and its pity counters (R1298000).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Holdings {
    pub owned: IndexMap<String, i64>,
    pub cosmetics: Vec<CardCosmetic>,
    /// Each account cosmetic owned, by id.
    pub account_cosmetics: Vec<String>,
    pub pity: PityCounters,
}

/// What one slot grants (§14.3's dupe chain, R1301002).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SlotGrant {
    /// A new card: a copy of it.
    Card,
    /// A duplicate: a per-card cosmetic of the rarity it rolled, for its card, or an account
    /// cosmetic of the global roll, for the account and on no card (R1661001).
    Cosmetic {
        cosmetic_id: String,
        cosmetic_rarity: CosmeticRarity,
    },
    /// A spent duplicate: nothing of the rarity it rolled was left to own, so its points go to
    /// both token bars (§14.5).
    Bars { cosmetic_rarity: CosmeticRarity },
}

/// One slot of an opened pack: the rarity it rolled, pity's force included, which the pity
/// counters read (§14.4), the card it drew and what that granted.
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
    /// The registry's droppable entries the roll picked from, whole: the per-card ones
    /// ([`pack_cosmetics`]), then the account ones of the global roll ([`global_cosmetics`],
    /// R1661001), each in registry order.
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
            cosmetics: [pack_cosmetics(registry), global_cosmetics(registry)].concat(),
            slots: opening.slots.clone(),
            pity: opening.pity,
            draws: opening.draws,
        }
    }
}

/// Each slot's card rarity (R1339000): a tiered roll on `CARD_TIER_WEIGHTS`, one draw each in slot
/// order, the slot at `PACK_FLOORED_SLOT` floored at `PACK_FLOOR_RARITY`, so it never rolls Common
/// and every pack holds a Rare or better. These are the opening's first draws (R1339002); pity
/// forces on top of them (`math::apply_pity`, R1339001).
pub fn slot_tiers(rng: &mut Rng) -> Vec<PrintedRarity> {
    (0..PACK_SLOTS)
        .map(|at| card_tier((at == PACK_FLOORED_SLOT).then_some(PACK_FLOOR_RARITY), rng))
        .collect()
}

/// The rarities a slot of `rolled` takes its card from, in order (R1301001): its own, then each
/// lower one from the nearest down, then each higher one from the nearest up, with every rarity at
/// or above `floor` ahead of those below it: a floored slot falls to its floor, then up, and below
/// it only when nothing above is held. A slot with no floor passes `Common`.
pub fn fallback_order(rolled: PrintedRarity, floor: PrintedRarity) -> Vec<PrintedRarity> {
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
    // Stable, so each side keeps its order.
    order.sort_by_key(|step| *step < floor);
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
/// `fallback_order(rarity, floor)` the pool holds, each by its weight. One draw; an empty pool is
/// `None` and draws nothing.
pub fn draw_card<'p>(
    pool: &'p [PoolCard],
    rarity: PrintedRarity,
    floor: PrintedRarity,
    owned: &IndexMap<String, i64>,
    rng: &mut Rng,
) -> Option<&'p PoolCard> {
    let candidates: Vec<&PoolCard> = fallback_order(rarity, floor)
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

/// What a duplicate of `card_id` grants once its cosmetic rarity is rolled (R1301002), a tiered
/// roll on `COSMETIC_TIER_WEIGHTS` on its own scale, whatever the card's rarity (R1339000,
/// R1333000): one of `droppable` of that rarity the account does not own on that card, picked
/// uniformly (one draw), or, with none, points to the bars (no draw); a duplicate does not fall to
/// another tier. `droppable` is `cosmetics::pack_cosmetics`'s; one that names a card, a full art,
/// is of that card alone (R1313000), and a Legendary or Mythic one is a chase that drops like any
/// other (R1333000).
pub fn duplicate_grant(
    card_id: &str,
    cosmetic_rarity: CosmeticRarity,
    droppable: &[Cosmetic],
    owned: &[CardCosmetic],
    rng: &mut Rng,
) -> SlotGrant {
    let unowned: Vec<&Cosmetic> = droppable
        .iter()
        .filter(|cosmetic| cosmetic.rarity == cosmetic_rarity)
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

/// A duplicate's global roll (R1661001), taken before its per-card one. With none of `global`
/// (`cosmetics::global_cosmetics`'s) at or above `GLOBAL_COSMETIC_FLOOR_RARITY` left unowned
/// (`owned`, the account's) it is `None` and draws nothing. Else one draw below
/// `TIER_WEIGHT_TOTAL`, and one at or past `GLOBAL_COSMETIC_ROLL_WEIGHT` is a miss, `None`. A hit
/// is a tiered roll on `COSMETIC_TIER_WEIGHTS` floored at `GLOBAL_COSMETIC_FLOOR_RARITY` (one
/// draw), falling as a cosmetic pack's floored slot does (`CosmeticRarity::floored_fallback_order`,
/// as `cosmetic_packs::slot_grant` reads it, R1306001) to the first tier with one unowned, every
/// tier at or above the floor first, then one of that
/// tier's picked uniformly in registry order (one draw): a `SlotGrant::Cosmetic` for the account.
pub fn global_grant(global: &[Cosmetic], owned: &[String], rng: &mut Rng) -> Option<SlotGrant> {
    let unowned: Vec<&Cosmetic> = global
        .iter()
        .filter(|cosmetic| cosmetic.rarity >= GLOBAL_COSMETIC_FLOOR_RARITY)
        .filter(|cosmetic| !owned.contains(&cosmetic.id))
        .collect();
    if unowned.is_empty() || tier_draw(rng) >= GLOBAL_COSMETIC_ROLL_WEIGHT {
        return None;
    }
    // Every one unowned is at the floor or above it, so a tier there always holds one.
    let order = cosmetic_tier(Some(GLOBAL_COSMETIC_FLOOR_RARITY), rng)
        .floored_fallback_order(GLOBAL_COSMETIC_FLOOR_RARITY);
    order.into_iter().find_map(|tier| {
        let of_tier: Vec<&Cosmetic> = unowned
            .iter()
            .copied()
            .filter(|cosmetic| cosmetic.rarity == tier)
            .collect();
        rng.pick(&of_tier).map(|cosmetic| SlotGrant::Cosmetic {
            cosmetic_id: cosmetic.id.clone(),
            cosmetic_rarity: tier,
        })
    })
}

/// The whole opening (R1301003, R1339002), pure from the pack's pool, the account's holdings
/// before it, the cosmetics registry and the opening's seed. Its draws, in this order and no other:
/// [`slot_tiers`]' one per slot, which pity forces on top of with no draw of its own, then slot by
/// slot the card's one and, for a duplicate, the global roll's when an account cosmetic is left
/// unowned ([`global_grant`], R1661001), its tier's and its pick's on a hit, or else its
/// cosmetic tier's one and the pick's one when there is something to pick. A slot keeps its rarity
/// after pity when its card comes from another (R1301001). Ownership is read slot by slot, so a
/// card unlocked earlier in the pack is a duplicate later in it (R1301002), and an account
/// cosmetic dropped earlier is owned later (R1661001). `None` for an empty pool.
pub fn roll_opening(
    pool: &[PoolCard],
    holdings: &Holdings,
    registry: &[Cosmetic],
    seed: &str,
) -> Option<Opening> {
    let droppable = pack_cosmetics(registry);
    let global = global_cosmetics(registry);
    let mut rng = Rng::new(seed, 0);
    let (rarities, pity) = apply_pity(holdings.pity, slot_tiers(&mut rng));
    let mut owned = holdings.owned.clone();
    let mut cosmetics = holdings.cosmetics.clone();
    let mut account_cosmetics = holdings.account_cosmetics.clone();
    let mut slots = Vec::with_capacity(rarities.len());
    for (at, rarity) in rarities.into_iter().enumerate() {
        let floor = if at == PACK_FLOORED_SLOT {
            PACK_FLOOR_RARITY
        } else {
            PrintedRarity::Common
        };
        let card = draw_card(pool, rarity, floor, &owned, &mut rng)?;
        let copies = owned.entry(card.id.clone()).or_insert(0);
        let grant = if *copies < i64::from(MAX_COPIES) {
            *copies += 1;
            SlotGrant::Card
        } else if let Some(grant) = global_grant(&global, &account_cosmetics, &mut rng) {
            if let SlotGrant::Cosmetic { cosmetic_id, .. } = &grant {
                account_cosmetics.push(cosmetic_id.clone());
            }
            grant
        } else {
            let cosmetic_rarity = cosmetic_tier(None, &mut rng);
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
    use crate::config::{PITY_MYTHIC_PACKS, TIER_WEIGHT_TOTAL};
    use crate::cosmetics::CosmeticKind;
    use crate::economy::tiers::{RATE_TOLERANCE, TierOdds, card_odds, cosmetic_odds};
    use jackioh_engine::wire::PrintedRarity::{Common, Epic, Legendary, Mythic, Rare};

    /// Draws enough for a rate to settle inside `RATE_TOLERANCE`.
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
            chase: rarity.is_chase(),
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

    /// An account cosmetic the global roll may drop: a playmat (R1661001).
    fn table(id: &str, rarity: CosmeticRarity) -> Cosmetic {
        Cosmetic {
            kind: CosmeticKind::Table,
            ..foil(id, rarity)
        }
    }

    /// `PACK_SLOTS` playmats of each tier at the floor or above, so no tier runs out inside one
    /// pack and a hit's cosmetic is of the tier it rolled.
    fn tables() -> Vec<Cosmetic> {
        [
            CosmeticRarity::Epic,
            CosmeticRarity::Legendary,
            CosmeticRarity::Mythic,
        ]
        .into_iter()
        .flat_map(|rarity| {
            let name = rarity.as_str().to_lowercase();
            (0..PACK_SLOTS).map(move |at| table(&format!("table-{name}-{at}"), rarity))
        })
        .collect()
    }

    /// The one of `tables` that `grant` drops, an account cosmetic of the global roll, if any.
    fn global_drop<'c>(grant: &SlotGrant, tables: &'c [Cosmetic]) -> Option<&'c Cosmetic> {
        match grant {
            SlotGrant::Cosmetic { cosmetic_id, .. } => tables.iter().find(|table| table.id == *cosmetic_id),
            _ => None,
        }
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

    /// A pool of `count` cards, the five rarities in turn.
    fn pool_of(count: usize) -> Vec<PoolCard> {
        (0..count)
            .map(|at| {
                card(
                    &format!("card-{at}"),
                    *PrintedRarity::ALL.get(at % 5).unwrap_or(&Common),
                )
            })
            .collect()
    }

    /// `odds` commonest first.
    fn chances(odds: TierOdds) -> [f64; 5] {
        [odds.common, odds.rare, odds.epic, odds.legendary, odds.mythic]
    }

    /// Each share of `counts` out of `draws` lands within `RATE_TOLERANCE` of `odds`.
    fn assert_lands_on(counts: [usize; 5], draws: usize, odds: TierOdds, what: &str) {
        for (tier, (count, chance)) in counts.into_iter().zip(chances(odds)).enumerate() {
            let share = count as f64 / draws as f64;
            assert!(
                (share - chance).abs() <= RATE_TOLERANCE,
                "{what}: tier {tier}'s share {share} outside tolerance of {chance}"
            );
        }
    }

    fn card_place(rarity: PrintedRarity) -> usize {
        PrintedRarity::ALL
            .iter()
            .position(|each| *each == rarity)
            .expect("a printed rarity")
    }

    #[test]
    fn r1339000_r1301000_every_slot_rolls_the_card_table_the_first_floored_at_rare() {
        let mut rng = Rng::new("r1339000", 0);
        let mut floored = [0_usize; 5];
        let mut others = [0_usize; 5];
        for _ in 0..DRAWS {
            let slots = slot_tiers(&mut rng);
            assert_eq!(slots.len(), PACK_SLOTS);
            for (at, rarity) in slots.into_iter().enumerate() {
                if at == PACK_FLOORED_SLOT {
                    assert!(rarity >= PACK_FLOOR_RARITY, "{rarity}");
                    floored[card_place(rarity)] += 1;
                } else {
                    others[card_place(rarity)] += 1;
                }
            }
        }
        assert_eq!(floored[card_place(Common)], 0);
        assert_lands_on(
            floored,
            DRAWS,
            card_odds(Some(PACK_FLOOR_RARITY)),
            "the floored slot",
        );
        assert_lands_on(
            others,
            DRAWS * (PACK_SLOTS - 1),
            card_odds(None),
            "the other slots",
        );
        // One draw per slot.
        assert_eq!(rng.cursor() as usize, DRAWS * PACK_SLOTS);
    }

    #[test]
    fn r1339000_the_floored_slot_is_never_common() {
        let pool = pool_of(40);
        let mut holdings = Holdings::default();
        for seed in 0..500 {
            let opening = roll_opening(&pool, &holdings, &[], &format!("r1339000-{seed}"))
                .unwrap_or_else(|| panic!("the pool holds cards"));
            let slot = &opening.slots[PACK_FLOORED_SLOT];
            assert!(slot.rarity >= PACK_FLOOR_RARITY, "{seed}: {slot:?}");
            let drawn = pool
                .iter()
                .find(|card| card.id == slot.card_id)
                .unwrap_or_else(|| panic!("{} is in the pool", slot.card_id));
            assert_eq!(drawn.rarity, slot.rarity, "{seed}: the pool holds every rarity");
            // The counters run on from pack to pack, so pity forces some of them.
            holdings.pity = opening.pity;
        }
    }

    #[test]
    fn r1339000_r1301001_the_floored_slot_falls_no_lower_than_rare_while_it_can() {
        assert_eq!(
            fallback_order(Epic, PACK_FLOOR_RARITY),
            [Epic, Rare, Legendary, Mythic, Common]
        );
        assert_eq!(
            fallback_order(Rare, PACK_FLOOR_RARITY),
            [Rare, Epic, Legendary, Mythic, Common]
        );
        assert_eq!(
            fallback_order(Mythic, PACK_FLOOR_RARITY),
            [Mythic, Legendary, Epic, Rare, Common]
        );
        // A pool with no Rare or Epic: the floored slot's Epic takes the Legendary, an unfloored
        // slot's the Common.
        let pool = [card("c", Common), card("l", Legendary)];
        let owned = IndexMap::new();
        let mut rng = Rng::new("r1339000", 0);
        let id =
            |floor, rng: &mut Rng| draw_card(&pool, Epic, floor, &owned, rng).map(|card| card.id.clone());
        assert_eq!(id(PACK_FLOOR_RARITY, &mut rng).as_deref(), Some("l"));
        assert_eq!(id(Common, &mut rng).as_deref(), Some("c"));
        // With nothing at or above the floor, it takes what is below.
        let commons = [card("c", Common)];
        assert_eq!(
            draw_card(&commons, Rare, PACK_FLOOR_RARITY, &owned, &mut rng),
            Some(&commons[0])
        );
    }

    #[test]
    fn r1339000_r1301002_a_duplicate_rolls_the_cosmetic_table() {
        // One card, owned: every slot is a duplicate, and with nothing to drop each is spent at
        // the cosmetic rarity it rolled.
        let pool = [card("m", Mythic)];
        let holdings = Holdings {
            owned: owning(&["m"]),
            ..Holdings::default()
        };
        let mut counts = [0_usize; 5];
        let openings = DRAWS / PACK_SLOTS;
        for seed in 0..openings {
            let opening = roll_opening(&pool, &holdings, &[], &format!("r1339000-dupe-{seed}"))
                .unwrap_or_else(|| panic!("the pool holds a card"));
            for slot in opening.slots {
                let SlotGrant::Bars { cosmetic_rarity } = slot.grant else {
                    panic!("nothing drops: {slot:?}");
                };
                counts[usize::from(cosmetic_rarity.tier())] += 1;
            }
        }
        assert_lands_on(
            counts,
            openings * PACK_SLOTS,
            cosmetic_odds(None),
            "a duplicate's cosmetic",
        );
    }

    #[test]
    fn r1339002_the_slot_tiers_are_the_opening_s_first_draws() {
        let pool = pool_of(40);
        let ids: Vec<&str> = pool.iter().map(|card| card.id.as_str()).collect();
        let pity = PityCounters {
            epic: 3,
            legendary: 7,
            mythic: PITY_MYTHIC_PACKS - 1,
        };
        for (holdings, seed) in [
            (
                Holdings {
                    pity,
                    ..Holdings::default()
                },
                "r1339002",
            ),
            (
                Holdings {
                    owned: owning(&ids),
                    pity,
                    ..Holdings::default()
                },
                "r1339002-owned",
            ),
        ] {
            let opening = roll_opening(&pool, &holdings, &[], seed).expect("the pool holds cards");
            let (rarities, after) = apply_pity(pity, slot_tiers(&mut Rng::new(seed, 0)));
            assert_eq!(
                opening.slots.iter().map(|slot| slot.rarity).collect::<Vec<_>>(),
                rarities,
                "{seed}"
            );
            assert_eq!(opening.pity, after, "{seed}");
        }
        // Every card owned and nothing to drop: each slot's tier, its card and its duplicate's
        // cosmetic tier, three draws a slot and no pick.
        let owned = Holdings {
            owned: owning(&ids),
            ..Holdings::default()
        };
        let opening = roll_opening(&pool, &owned, &[], "r1339002").expect("the pool holds cards");
        assert_eq!(opening.draws as usize, 3 * PACK_SLOTS);
    }

    #[test]
    fn r1301001_a_rarity_the_pool_lacks_steps_down_then_up() {
        assert_eq!(
            fallback_order(Mythic, Common),
            [Mythic, Legendary, Epic, Rare, Common]
        );
        assert_eq!(
            fallback_order(Epic, Common),
            [Epic, Rare, Common, Legendary, Mythic]
        );
        assert_eq!(
            fallback_order(Common, Common),
            [Common, Rare, Epic, Legendary, Mythic]
        );

        let mut rng = Rng::new("r1301001", 0);
        let owned = IndexMap::new();
        let mythic_only = [card("m", Mythic)];
        assert_eq!(
            draw_card(&mythic_only, Common, Common, &owned, &mut rng),
            Some(&mythic_only[0])
        );
        // The nearest lower rarity before any higher one.
        let rare_and_legendary = [card("l", Legendary), card("r", Rare)];
        assert_eq!(
            draw_card(&rare_and_legendary, Epic, Common, &owned, &mut rng).map(|card| card.id.as_str()),
            Some("r")
        );
        assert_eq!(rng.cursor(), 2);
        // An empty pool has no card and takes no draw.
        assert_eq!(draw_card(&[], Rare, Common, &owned, &mut rng), None);
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
        // Pity forces the Mythic, or the slots rolled one: the pool has none, so its slot draws the
        // Legendary and stays Mythic for the counter.
        let mythic = opening
            .slots
            .iter()
            .find(|slot| slot.rarity == Mythic)
            .unwrap_or_else(|| panic!("pity holds a Mythic: {:?}", opening.slots));
        assert_eq!(mythic.card_id, "l");
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
        // Every foil drops at its own rarity (no foil is Mythic, so a Mythic roll is always
        // spent: `r1301002_a_rolled_rarity_with_nothing_unowned_goes_to_the_bars`).
        for foil in &foils {
            let grant = duplicate_grant("m", foil.rarity, &foils, &[], &mut rng);
            assert_eq!(
                grant,
                SlotGrant::Cosmetic {
                    cosmetic_id: foil.id.clone(),
                    cosmetic_rarity: foil.rarity,
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
                    assert_eq!(cosmetic.rarity, cosmetic_rarity);
                    granted.push(cosmetic_rarity);
                }
            }
        }
        assert!(granted.contains(&CosmeticRarity::Common), "{granted:?}");
        assert!(granted.contains(&CosmeticRarity::Rare), "{granted:?}");
    }

    #[test]
    fn r1301002_a_rolled_rarity_with_nothing_unowned_goes_to_the_bars() {
        let foils = four_foils();
        let mut rng = Rng::new("r1301002", 0);
        // The card's Legendary foil is owned: Bars, with no draw.
        let owned = [held("m", "foil-legendary")];
        assert_eq!(
            duplicate_grant("m", CosmeticRarity::Legendary, &foils, &owned, &mut rng),
            SlotGrant::Bars {
                cosmetic_rarity: CosmeticRarity::Legendary
            }
        );
        assert_eq!(rng.cursor(), 0);
        // Owned on another card is not owned on this one.
        assert_eq!(
            duplicate_grant("c", CosmeticRarity::Legendary, &foils, &owned, &mut rng),
            SlotGrant::Cosmetic {
                cosmetic_id: "foil-legendary".to_owned(),
                cosmetic_rarity: CosmeticRarity::Legendary,
            }
        );
        // No cosmetic is Mythic.
        assert_eq!(
            duplicate_grant("m", CosmeticRarity::Mythic, &foils, &[], &mut rng),
            SlotGrant::Bars {
                cosmetic_rarity: CosmeticRarity::Mythic
            }
        );
        // A starter is owned by everyone already, so the pack leaves it out.
        let starter = Cosmetic {
            starter: true,
            ..foil("foil-starter", CosmeticRarity::Legendary)
        };
        assert_eq!(
            duplicate_grant(
                "m",
                CosmeticRarity::Legendary,
                &pack_cosmetics(&[starter]),
                &[],
                &mut rng
            ),
            SlotGrant::Bars {
                cosmetic_rarity: CosmeticRarity::Legendary
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
            duplicate_grant("m", CosmeticRarity::Epic, &droppable, &[], &mut rng),
            SlotGrant::Cosmetic {
                cosmetic_id: "full-art-m".to_owned(),
                cosmetic_rarity: CosmeticRarity::Epic,
            }
        );
        // Any other card's Epic duplicate has nothing to drop, and neither has its card once it
        // owns it.
        assert_eq!(
            duplicate_grant("c", CosmeticRarity::Epic, &droppable, &[], &mut rng),
            SlotGrant::Bars {
                cosmetic_rarity: CosmeticRarity::Epic
            }
        );
        assert_eq!(
            duplicate_grant(
                "m",
                CosmeticRarity::Epic,
                &droppable,
                &[held("m", "full-art-m")],
                &mut rng
            ),
            SlotGrant::Bars {
                cosmetic_rarity: CosmeticRarity::Epic
            }
        );
    }

    #[test]
    fn r1333000_a_duplicate_s_cosmetic_roll_leaves_the_pity_counters_alone() {
        let pool: Vec<PoolCard> = (0..40)
            .map(|at| {
                card(
                    &format!("card-{at}"),
                    *PrintedRarity::ALL.get(at % 5).unwrap_or(&Common),
                )
            })
            .collect();
        let ids: Vec<&str> = pool.iter().map(|card| card.id.as_str()).collect();
        let pity = PityCounters {
            epic: 3,
            legendary: 7,
            mythic: 11,
        };
        // Every slot a duplicate, rolling its cosmetic: with cosmetics to drop or none.
        let owned = Holdings {
            owned: owning(&ids),
            pity,
            ..Holdings::default()
        };
        // No slot a duplicate, so nothing rolls a cosmetic.
        let unowned = Holdings {
            pity,
            ..Holdings::default()
        };
        let foils = four_foils();
        let mut cosmetics = 0;
        for seed in 0..50 {
            let seed = format!("r1333000-{seed}");
            let rolled = |holdings: &Holdings, registry: &[Cosmetic]| {
                roll_opening(&pool, holdings, registry, &seed).expect("the pool holds cards")
            };
            let dropping = rolled(&owned, &foils);
            let rarities =
                |opening: &Opening| opening.slots.iter().map(|slot| slot.rarity).collect::<Vec<_>>();
            for other in [rolled(&owned, &[]), rolled(&unowned, &foils)] {
                assert_eq!(other.pity, dropping.pity, "{seed}");
                assert_eq!(rarities(&other), rarities(&dropping), "{seed}");
            }
            // The slots' tiers, the opening's first draws, set the counters alone.
            assert_eq!(
                dropping.pity,
                apply_pity(pity, slot_tiers(&mut Rng::new(&seed, 0))).1,
                "{seed}"
            );
            cosmetics += dropping
                .slots
                .iter()
                .filter(|slot| matches!(slot.grant, SlotGrant::Cosmetic { .. }))
                .count();
        }
        assert!(cosmetics > 0, "no duplicate dropped a cosmetic");
    }

    #[test]
    fn r1301002_the_draw_leans_toward_unowned_cards() {
        let pool = [card("owned", Common), card("new", Common)];
        let owned = owning(&["owned"]);
        let mut rng = Rng::new("r1301002-lean", 0);
        let new = (0..DRAWS)
            .filter(|_| {
                draw_card(&pool, Common, Common, &owned, &mut rng).map(|card| card.id.as_str()) == Some("new")
            })
            .count();
        let share = new as f64 / DRAWS as f64;
        let expected =
            PACK_UNOWNED_WEIGHTS.unowned / (PACK_UNOWNED_WEIGHTS.owned + PACK_UNOWNED_WEIGHTS.unowned);
        assert!(
            (share - expected).abs() <= RATE_TOLERANCE,
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
            account_cosmetics: Vec::new(),
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
    fn r1661001_a_duplicate_drops_an_account_cosmetic_at_the_global_weight_epic_or_better() {
        // One card, owned: every slot is a duplicate, and an account cosmetic is left unowned for
        // each, so every one takes the global draw.
        let pool = [card("m", Mythic)];
        let holdings = Holdings {
            owned: owning(&["m"]),
            pity: PityCounters {
                epic: 3,
                legendary: 7,
                mythic: 11,
            },
            ..Holdings::default()
        };
        let tables = tables();
        let mut registry = four_foils();
        registry.extend(tables.clone());
        let openings = DRAWS / PACK_SLOTS;
        let mut hits = [0_usize; 5];
        for seed in 0..openings {
            let seed = format!("r1661001-{seed}");
            let opening = roll_opening(&pool, &holdings, &registry, &seed).expect("the pool holds a card");
            let mut dropped = Vec::new();
            for slot in &opening.slots {
                let Some(table) = global_drop(&slot.grant, &tables) else {
                    continue;
                };
                let SlotGrant::Cosmetic { cosmetic_rarity, .. } = slot.grant else {
                    panic!("a global drop is a cosmetic: {slot:?}");
                };
                assert!(cosmetic_rarity >= GLOBAL_COSMETIC_FLOOR_RARITY, "{slot:?}");
                assert_eq!(table.rarity, cosmetic_rarity);
                // Owned once dropped, for the rest of the pack.
                assert!(!dropped.contains(&table.id), "{seed}: {:?}", opening.slots);
                dropped.push(table.id.clone());
                hits[usize::from(cosmetic_rarity.tier())] += 1;
            }
            // The pity counters come from the slots' tiers alone.
            assert_eq!(
                opening.pity,
                apply_pity(holdings.pity, slot_tiers(&mut Rng::new(&seed, 0))).1,
                "{seed}"
            );
        }
        let rolls = openings * PACK_SLOTS;
        let landed: usize = hits.iter().sum();
        let share = landed as f64 / rolls as f64;
        let chance = f64::from(GLOBAL_COSMETIC_ROLL_WEIGHT) / f64::from(TIER_WEIGHT_TOTAL);
        assert!(
            (share - chance).abs() <= RATE_TOLERANCE,
            "the global share {share} outside tolerance of {chance}"
        );
        // A hit's tier is the floored table's: never below Epic.
        assert_eq!(
            hits[..usize::from(GLOBAL_COSMETIC_FLOOR_RARITY.tier())]
                .iter()
                .sum::<usize>(),
            0
        );
        assert_lands_on(
            hits,
            landed,
            cosmetic_odds(Some(GLOBAL_COSMETIC_FLOOR_RARITY)),
            "a global drop's tier",
        );
    }

    #[test]
    fn r1661001_the_global_draw_follows_the_card_s_and_takes_two_more_on_a_hit() {
        // Every slot a duplicate, and no per-card cosmetic to drop: a slot draws its card and the
        // global roll, then a hit its tier and its pick, and a miss its cosmetic tier alone.
        let pool = [card("m", Mythic)];
        let holdings = Holdings {
            owned: owning(&["m"]),
            ..Holdings::default()
        };
        let tables = tables();
        let (mut hit, mut miss) = (false, false);
        for seed in 0..200 {
            let seed = format!("r1661001-order-{seed}");
            let opening = roll_opening(&pool, &holdings, &tables, &seed).expect("the pool holds a card");
            let hits = opening
                .slots
                .iter()
                .filter(|slot| global_drop(&slot.grant, &tables).is_some())
                .count();
            assert_eq!(opening.draws as usize, 4 * PACK_SLOTS + hits, "{seed}");
            // The first slot's global draw is the one after the slots' tiers and its card's.
            let mut rng = Rng::new(&seed, 0);
            let rarities = slot_tiers(&mut rng);
            draw_card(&pool, rarities[0], PACK_FLOOR_RARITY, &holdings.owned, &mut rng);
            let landed = tier_draw(&mut rng) < GLOBAL_COSMETIC_ROLL_WEIGHT;
            assert_eq!(
                global_drop(&opening.slots[0].grant, &tables).is_some(),
                landed,
                "{seed}"
            );
            hit |= landed;
            miss |= !landed;
        }
        assert!(hit && miss, "the seeds hit and missed");
    }

    #[test]
    fn r1661001_with_nothing_unowned_the_opening_takes_no_draw_of_it() {
        let pool = pool_of(40);
        let ids: Vec<&str> = pool.iter().map(|card| card.id.as_str()).collect();
        // Half the pool owned: unlocks and duplicates both.
        let holdings = Holdings {
            owned: owning(&ids[..ids.len() / 2]),
            ..Holdings::default()
        };
        let tables = tables();
        let with = |tables: Vec<Cosmetic>| [four_foils(), tables].concat();
        // Every account cosmetic owned, every one a starter, or every one below the floor (a
        // registry the check refuses, R1661000): none can drop.
        let owning_all = Holdings {
            account_cosmetics: tables.iter().map(|table| table.id.clone()).collect(),
            ..holdings.clone()
        };
        let starters = with(
            tables
                .iter()
                .map(|table| Cosmetic {
                    starter: true,
                    ..table.clone()
                })
                .collect(),
        );
        let below_floor = with(vec![table("table-rare", CosmeticRarity::Rare)]);
        for seed in 0..200 {
            let seed = format!("r1661001-none-{seed}");
            let today = roll_opening(&pool, &holdings, &four_foils(), &seed);
            assert_eq!(
                roll_opening(&pool, &owning_all, &with(tables.clone()), &seed),
                today
            );
            assert_eq!(roll_opening(&pool, &holdings, &starters, &seed), today);
            assert_eq!(roll_opening(&pool, &holdings, &below_floor, &seed), today);
        }
        // The shipped registry leaves no account cosmetic to drop: on a pool with no full art's
        // card, every slot a duplicate takes its tier's, its card's and its cosmetic tier's three
        // draws, as before.
        let shipped = crate::cosmetics::shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let all = Holdings {
            owned: owning(&ids),
            ..Holdings::default()
        };
        for seed in 0..50 {
            let opening =
                roll_opening(&pool, &all, &shipped, &format!("r1661001-shipped-{seed}")).expect("an opening");
            assert_eq!(opening.draws as usize, 3 * PACK_SLOTS);
        }
    }

    #[test]
    fn r1661001_a_hit_falls_to_the_nearest_tier_with_one_unowned_at_or_above_the_floor() {
        let epic = table("table-epic", CosmeticRarity::Epic);
        let legendary = table("table-legendary", CosmeticRarity::Legendary);
        let global = [epic.clone(), legendary.clone()];
        let mut rng = Rng::new("r1661001-fall", 0);
        let mut hits = (0, 0);
        for _ in 0..DRAWS {
            // The Epic owned: every hit, an Epic roll's included, takes the Legendary.
            if let Some(grant) = global_grant(&global, std::slice::from_ref(&epic.id), &mut rng) {
                assert_eq!(global_drop(&grant, &global), Some(&legendary));
                hits.0 += 1;
            }
            // The Legendary owned: a Legendary or Mythic roll falls to the Epic.
            if let Some(grant) = global_grant(&global, std::slice::from_ref(&legendary.id), &mut rng) {
                assert_eq!(global_drop(&grant, &global), Some(&epic));
                hits.1 += 1;
            }
        }
        assert!(hits.0 > 0 && hits.1 > 0, "{hits:?}");
        // Nothing unowned, or nothing at or above the floor: no grant and no draw.
        let cursor = rng.cursor();
        assert_eq!(
            global_grant(&global, &[epic.id.clone(), legendary.id.clone()], &mut rng),
            None
        );
        assert_eq!(
            global_grant(&[table("table-rare", CosmeticRarity::Rare)], &[], &mut rng),
            None
        );
        assert_eq!(global_grant(&[], &[], &mut rng), None);
        assert_eq!(rng.cursor(), cursor);
    }

    #[test]
    fn r1661001_the_log_holds_the_account_cosmetics_and_replays_their_drops() {
        let pool = vec![card("owned", Common)];
        let holdings = Holdings {
            owned: owning(&["owned"]),
            account_cosmetics: vec!["table-epic-0".to_owned()],
            ..Holdings::default()
        };
        let mut registry = four_foils();
        registry.extend(tables());
        let (seed, opening) = (0..200)
            .map(|seed| format!("r1661001-log-{seed}"))
            .find_map(|seed| {
                let opening = roll_opening(&pool, &holdings, &registry, &seed).expect("an opening");
                opening
                    .slots
                    .iter()
                    .any(|slot| global_drop(&slot.grant, &tables()).is_some())
                    .then_some((seed, opening))
            })
            .expect("a seed that drops an account cosmetic");
        // The owned one never drops again.
        assert!(opening.slots.iter().all(|slot| {
            !matches!(&slot.grant, SlotGrant::Cosmetic { cosmetic_id, .. } if cosmetic_id == "table-epic-0")
        }));
        let log = OpeningLog::of(
            PackKind::Set(SetName::Core),
            "catalog-v1",
            &registry,
            &seed,
            &opening,
        );
        // The per-card ones, then the account ones.
        assert_eq!(log.cosmetics, registry);
        let replayed = roll_opening(&pool, &holdings, &log.cosmetics, &log.seed).expect("the replay");
        assert_eq!(replayed, opening);
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
