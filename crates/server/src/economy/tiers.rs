//! The tiered roll (SPEC §14.3, §14.6; #1339, R1339000): every drop the economy makes, a card pack
//! slot's card rarity, a duplicate's cosmetic rarity and a cosmetic pack slot's, is one integer
//! draw below `TIER_WEIGHT_TOTAL` against a table that weighs only the tiers above the lowest. The
//! lowest tier is the fallback and takes whatever weight is left, and a floor lifts every result
//! below it to the floor and leaves the tiers above alone. The table is exactly what one roll does:
//! pity sits on top of it (`economy::math`, R1339001), and the item within the tier is the caller's
//! own draw. No store, no clock, no I/O.
//!
//! - [`roll_tier`]: the one pure function every roll goes through, from a table, its fallback, an
//!   optional floor and a draw to a tier.
//! - [`card_tier`] and [`cosmetic_tier`]: one draw on `CARD_TIER_WEIGHTS` or
//!   `COSMETIC_TIER_WEIGHTS`.
//! - [`card_odds`] and [`cosmetic_odds`]: where one roll lands, counted over every draw, which the
//!   odds `GET`s answer, the fallback included.

use jackioh_engine::rng::Rng;
use jackioh_engine::wire::PrintedRarity;
use serde::{Deserialize, Serialize};

use crate::config::{
    CARD_TIER_WEIGHTS, COSMETIC_PACK_SLOTS, COSMETIC_TIER_WEIGHTS, PACK_FLOORED_SLOT, PACK_SLOTS,
    TIER_WEIGHT_TOTAL, TierWeights,
};
use crate::cosmetics::CosmeticRarity;

// A draw is one `Rng::int` below the total, so the total fits its `i32`.
const _: () = assert!(TIER_WEIGHT_TOTAL > 0 && TIER_WEIGHT_TOTAL <= i32::MAX as u32);
// The floored slot is a slot of both packs.
const _: () = assert!(PACK_FLOORED_SLOT < PACK_SLOTS && PACK_FLOORED_SLOT < COSMETIC_PACK_SLOTS);

/// The weight `set` leaves its fallback: `TIER_WEIGHT_TOTAL` less the set tiers' weights. `None`
/// when they sum past the total, a table no roll may read.
pub fn fallback_weight<T>(set: &[(T, u32)]) -> Option<u32> {
    let mut sum: u32 = 0;
    for (_, weight) in set {
        sum = sum.checked_add(*weight)?;
    }
    TIER_WEIGHT_TOTAL.checked_sub(sum)
}

/// One tiered roll (R1339000): `draw`, below `TIER_WEIGHT_TOTAL`, lands on the first tier of `set`
/// whose running sum of weights passes it, and past them all on `fallback`; a result below `floor`
/// becomes `floor`. `None` when the set weights sum past the total or the draw is not below it.
pub fn roll_tier<T: Copy + Ord>(set: &[(T, u32)], fallback: T, floor: Option<T>, draw: u32) -> Option<T> {
    fallback_weight(set)?;
    if draw >= TIER_WEIGHT_TOTAL {
        return None;
    }
    let mut edge = 0;
    let mut landed = fallback;
    for (tier, weight) in set {
        edge += weight;
        if draw < edge {
            landed = *tier;
            break;
        }
    }
    Some(match floor {
        Some(floor) if landed < floor => floor,
        _ => landed,
    })
}

/// One roll's draw: an integer below `TIER_WEIGHT_TOTAL`, one draw of `rng`.
pub fn tier_draw(rng: &mut Rng) -> u32 {
    rng.int(TIER_WEIGHT_TOTAL as i32) as u32
}

/// `weights` as a card rarity table, rarest first; Common is its fallback.
pub fn card_set(weights: TierWeights) -> [(PrintedRarity, u32); 4] {
    [
        (PrintedRarity::Mythic, weights.mythic),
        (PrintedRarity::Legendary, weights.legendary),
        (PrintedRarity::Epic, weights.epic),
        (PrintedRarity::Rare, weights.rare),
    ]
}

/// `weights` as a cosmetic rarity table, rarest first; Common is its fallback. Its own scale,
/// never a card's (R1333000).
pub fn cosmetic_set(weights: TierWeights) -> [(CosmeticRarity, u32); 4] {
    [
        (CosmeticRarity::Mythic, weights.mythic),
        (CosmeticRarity::Legendary, weights.legendary),
        (CosmeticRarity::Epic, weights.epic),
        (CosmeticRarity::Rare, weights.rare),
    ]
}

/// A card pack slot's card rarity (R1339000): one draw on `CARD_TIER_WEIGHTS`, lifted to `floor`.
pub fn card_tier(floor: Option<PrintedRarity>, rng: &mut Rng) -> PrintedRarity {
    // Config's asserts keep the table within the total and the draw is below it, so the roll
    // always lands.
    roll_tier(
        &card_set(CARD_TIER_WEIGHTS),
        PrintedRarity::Common,
        floor,
        tier_draw(rng),
    )
    .unwrap_or(PrintedRarity::Common)
}

/// A duplicate's or a cosmetic pack slot's cosmetic rarity (R1339000): one draw on
/// `COSMETIC_TIER_WEIGHTS`, lifted to `floor`.
pub fn cosmetic_tier(floor: Option<CosmeticRarity>, rng: &mut Rng) -> CosmeticRarity {
    // As `card_tier`: the roll always lands.
    roll_tier(
        &cosmetic_set(COSMETIC_TIER_WEIGHTS),
        CosmeticRarity::Common,
        floor,
        tier_draw(rng),
    )
    .unwrap_or(CosmeticRarity::Common)
}

/// One roll's chance of landing on each tier, commonest first, as the odds `GET`s answer them
/// (R1339000): each the count of draws landing there over `TIER_WEIGHT_TOTAL`, the fallback
/// included, so the client shows the server's numbers and computes none (CLAUDE.md rule 7).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TierOdds {
    pub common: f64,
    pub rare: f64,
    pub epic: f64,
    pub legendary: f64,
    pub mythic: f64,
}

/// Where a roll on `set` lands, counted over every draw below `TIER_WEIGHT_TOTAL`, each tier at
/// its place commonest first. Each chance is one division of a count, so it is exact to the float.
fn odds_of<T: Copy + Ord>(
    set: &[(T, u32)],
    fallback: T,
    floor: Option<T>,
    place: impl Fn(T) -> usize,
) -> TierOdds {
    let mut counts = [0_u32; 5];
    for draw in 0..TIER_WEIGHT_TOTAL {
        if let Some(count) =
            roll_tier(set, fallback, floor, draw).and_then(|tier| counts.get_mut(place(tier)))
        {
            *count += 1;
        }
    }
    let [common, rare, epic, legendary, mythic] =
        counts.map(|count| f64::from(count) / f64::from(TIER_WEIGHT_TOTAL));
    TierOdds {
        common,
        rare,
        epic,
        legendary,
        mythic,
    }
}

/// A card pack slot's odds (R1339000), `floor` its floor or `None`.
pub fn card_odds(floor: Option<PrintedRarity>) -> TierOdds {
    odds_of(
        &card_set(CARD_TIER_WEIGHTS),
        PrintedRarity::Common,
        floor,
        |rarity| {
            PrintedRarity::ALL
                .iter()
                .position(|each| *each == rarity)
                .unwrap_or(0)
        },
    )
}

/// A duplicate's or a cosmetic pack slot's odds (R1339000), `floor` its floor or `None`.
pub fn cosmetic_odds(floor: Option<CosmeticRarity>) -> TierOdds {
    odds_of(
        &cosmetic_set(COSMETIC_TIER_WEIGHTS),
        CosmeticRarity::Common,
        floor,
        |rarity| usize::from(rarity.tier()),
    )
}

/// How far a seeded share may land from its exact chance in the economy's tests, in fraction
/// points: two points hold a few thousand draws without fitting a seed's noise.
#[cfg(test)]
pub(crate) const RATE_TOLERANCE: f64 = 0.02;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{COSMETIC_PACK_FLOOR_RARITY, PACK_FLOOR_RARITY};
    use jackioh_engine::wire::PrintedRarity::{Common, Epic, Legendary, Mythic, Rare};

    /// The draws a seeded run takes per table.
    const RUN: usize = 200_000;
    /// How far a seeded run's share may land from its weight over the total: a 200,000-draw run's
    /// standard error is at most about 0.0011, so this holds four of them.
    const TIER_TOLERANCE: f64 = 0.005;
    /// How much `r1339000_raising_one_tier_lowers_only_the_fallback` raises Epic by.
    const RAISE: u32 = 50;

    /// How many draws below the total land on each tier of `set`, commonest first.
    fn counts<T: Copy + Ord>(
        set: &[(T, u32)],
        fallback: T,
        floor: Option<T>,
        place: impl Fn(T) -> usize,
    ) -> [u32; 5] {
        let mut counts = [0_u32; 5];
        for draw in 0..TIER_WEIGHT_TOTAL {
            let tier = roll_tier(set, fallback, floor, draw).expect("the table is within the total");
            counts[place(tier)] += 1;
        }
        counts
    }

    fn card_place(rarity: PrintedRarity) -> usize {
        PrintedRarity::ALL
            .iter()
            .position(|each| *each == rarity)
            .expect("a printed rarity")
    }

    fn cosmetic_place(rarity: CosmeticRarity) -> usize {
        usize::from(rarity.tier())
    }

    /// A table's weights commonest first, its fallback's computed.
    fn expected(weights: TierWeights) -> [u32; 5] {
        [
            TIER_WEIGHT_TOTAL - weights.set_total(),
            weights.rare,
            weights.epic,
            weights.legendary,
            weights.mythic,
        ]
    }

    fn chances(odds: TierOdds) -> [f64; 5] {
        [odds.common, odds.rare, odds.epic, odds.legendary, odds.mythic]
    }

    #[test]
    fn r1339000_each_tier_lands_at_its_weight() {
        let mut rng = Rng::new("r1339000", 0);
        let mut cards = [0_usize; 5];
        let mut cosmetics = [0_usize; 5];
        for _ in 0..RUN {
            cards[card_place(card_tier(None, &mut rng))] += 1;
            cosmetics[cosmetic_place(cosmetic_tier(None, &mut rng))] += 1;
        }
        assert_eq!(rng.cursor() as usize, 2 * RUN, "one draw per roll");
        for (weights, landed) in [(CARD_TIER_WEIGHTS, cards), (COSMETIC_TIER_WEIGHTS, cosmetics)] {
            for (tier, (weight, count)) in expected(weights).into_iter().zip(landed).enumerate() {
                let share = count as f64 / RUN as f64;
                let chance = f64::from(weight) / f64::from(TIER_WEIGHT_TOTAL);
                assert!(
                    (share - chance).abs() <= TIER_TOLERANCE,
                    "tier {tier} landed {share}, its weight says {chance} ({weights:?})"
                );
            }
        }
        // The fallback's share is the weight the set tiers leave.
        assert_eq!(
            fallback_weight(&card_set(CARD_TIER_WEIGHTS)),
            Some(TIER_WEIGHT_TOTAL - CARD_TIER_WEIGHTS.set_total())
        );
        assert_eq!(
            fallback_weight(&cosmetic_set(COSMETIC_TIER_WEIGHTS)),
            Some(TIER_WEIGHT_TOTAL - COSMETIC_TIER_WEIGHTS.set_total())
        );
        // Counted over every draw, one roll lands exactly on the weights, and the odds say so.
        for (weights, odds, landed) in [
            (
                CARD_TIER_WEIGHTS,
                card_odds(None),
                counts(&card_set(CARD_TIER_WEIGHTS), Common, None, card_place),
            ),
            (
                COSMETIC_TIER_WEIGHTS,
                cosmetic_odds(None),
                counts(
                    &cosmetic_set(COSMETIC_TIER_WEIGHTS),
                    CosmeticRarity::Common,
                    None,
                    cosmetic_place,
                ),
            ),
        ] {
            assert_eq!(landed, expected(weights));
            assert_eq!(
                chances(odds),
                expected(weights).map(|weight| f64::from(weight) / f64::from(TIER_WEIGHT_TOTAL))
            );
        }
    }

    #[test]
    fn r1339000_raising_one_tier_lowers_only_the_fallback() {
        let raised = TierWeights {
            epic: CARD_TIER_WEIGHTS.epic + RAISE,
            ..CARD_TIER_WEIGHTS
        };
        let before = counts(&card_set(CARD_TIER_WEIGHTS), Common, None, card_place);
        let after = counts(&card_set(raised), Common, None, card_place);
        assert_eq!(after[card_place(Epic)], before[card_place(Epic)] + RAISE);
        assert_eq!(after[card_place(Common)], before[card_place(Common)] - RAISE);
        for rarity in [Rare, Legendary, Mythic] {
            assert_eq!(after[card_place(rarity)], before[card_place(rarity)], "{rarity}");
        }
    }

    #[test]
    fn r1339000_a_floor_lifts_what_is_below_it_and_leaves_the_rest() {
        let cards = card_set(CARD_TIER_WEIGHTS);
        let cosmetics = cosmetic_set(COSMETIC_TIER_WEIGHTS);
        for draw in 0..TIER_WEIGHT_TOTAL {
            let free = roll_tier(&cards, Common, None, draw).expect("a card tier");
            let floored = roll_tier(&cards, Common, Some(PACK_FLOOR_RARITY), draw).expect("a card tier");
            assert_eq!(floored, free.max(PACK_FLOOR_RARITY), "draw {draw}");
            let free = roll_tier(&cosmetics, CosmeticRarity::Common, None, draw).expect("a cosmetic tier");
            let floored = roll_tier(
                &cosmetics,
                CosmeticRarity::Common,
                Some(COSMETIC_PACK_FLOOR_RARITY),
                draw,
            )
            .expect("a cosmetic tier");
            assert_eq!(floored, free.max(COSMETIC_PACK_FLOOR_RARITY), "draw {draw}");
        }
        let total = f64::from(TIER_WEIGHT_TOTAL);
        let w = CARD_TIER_WEIGHTS;
        let floored = card_odds(Some(PACK_FLOOR_RARITY));
        assert_eq!(floored.common, 0.0);
        assert_eq!(
            floored.rare,
            f64::from(TIER_WEIGHT_TOTAL - w.epic - w.legendary - w.mythic) / total
        );
        assert_eq!(floored.epic, card_odds(None).epic);
        assert_eq!(floored.legendary, card_odds(None).legendary);
        assert_eq!(floored.mythic, card_odds(None).mythic);
        let w = COSMETIC_TIER_WEIGHTS;
        let floored = cosmetic_odds(Some(COSMETIC_PACK_FLOOR_RARITY));
        assert_eq!((floored.common, floored.rare), (0.0, 0.0));
        assert_eq!(
            floored.epic,
            f64::from(TIER_WEIGHT_TOTAL - w.legendary - w.mythic) / total
        );
        assert_eq!(floored.legendary, cosmetic_odds(None).legendary);
        assert_eq!(floored.mythic, cosmetic_odds(None).mythic);
    }

    #[test]
    fn r1339000_set_weights_past_the_total_are_refused() {
        let past = [(Mythic, TIER_WEIGHT_TOTAL), (Rare, 1)];
        assert_eq!(fallback_weight(&past), None);
        assert_eq!(roll_tier(&past, Common, None, 0), None);
        // Weights that overflow the sum are refused too, not wrapped.
        let overflowing = [(Mythic, u32::MAX), (Rare, 2)];
        assert_eq!(fallback_weight(&overflowing), None);
        assert_eq!(roll_tier(&overflowing, Common, None, 0), None);
        // Weights summing to exactly the total leave the fallback nothing, and still roll.
        let full = [(Mythic, TIER_WEIGHT_TOTAL - 1), (Rare, 1)];
        assert_eq!(fallback_weight(&full), Some(0));
        assert_eq!(roll_tier(&full, Common, None, TIER_WEIGHT_TOTAL - 1), Some(Rare));
        // A draw not below the total is no roll.
        assert_eq!(
            roll_tier(&card_set(CARD_TIER_WEIGHTS), Common, None, TIER_WEIGHT_TOTAL),
            None
        );
    }
}
