//! The collection economy's pure math (SPEC §14, #1298): the set-track level curve (§14.7) and
//! pity (§14.4), which sits on top of a card pack's tiered roll (R1339001). No store, no clock, no
//! I/O: the pack roll calls [`apply_pity`] with the slots it rolled from its own seeded, logged
//! draws, and the tracks call the level curve with the account's total XP. The numbers live in
//! [`crate::config`]; this module only computes with them (CLAUDE.md rule 9).
//!
//! - [`PityCounters`] is one rarity counter set per account, shared by every card pack it opens
//!   (R1298000): each counts card packs, not slots, since its rarity last dropped.
//! - [`apply_pity`] is pity, pure from the counters and the slots' rolled rarities to the slots
//!   after any force and the next counters.

use jackioh_engine::wire::PrintedRarity;
use serde::{Deserialize, Serialize};

use crate::config::{
    PACK_FLOORED_SLOT, PACK_SLOTS, PITY_EPIC_PACKS, PITY_LEGENDARY_PACKS, PITY_MYTHIC_PACKS,
    SET_TRACK_XP_PER_LEVEL,
};

// SPEC §14.4: each rarity's threshold is a longer wait than the commoner one's, and Mythic's the
// longest of all.
const _: () = assert!(PITY_EPIC_PACKS < PITY_LEGENDARY_PACKS && PITY_LEGENDARY_PACKS < PITY_MYTHIC_PACKS);

/// The rarities a pity counter tracks (R1298000): Epic, Legendary and Mythic.
const TRACKED_RARITIES: usize = 3;

// A pack that forces all three tracked rarities has a slot for each, and every force finds one
// (`forced_slot`).
const _: () = assert!(PACK_SLOTS >= TRACKED_RARITIES);

/// One pity counter set per account (R1298000, SPEC §14.4): each counter is the card packs
/// since a pack held its rarity, whatever that slot then granted. All three count every card
/// pack the account opens, global and per-set alike.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PityCounters {
    pub epic: usize,
    pub legendary: usize,
    pub mythic: usize,
}

/// Pity (SPEC §14.4, R1339001): from the pack's pity counters and its slots' rolled rarities to
/// the slots after pity and the next counters. Pity changes no roll: a pack in which a counter
/// would reach its threshold, without its rarity, forces one slot to that rarity, Mythic first,
/// then Legendary, then Epic, each into `forced_slot`'s slot. Then each counter resets when the
/// pack holds its rarity and adds one when it does not, so a Mythic leaves the Epic and Legendary
/// counters running, and a Legendary the Epic one.
pub fn apply_pity(
    counters: PityCounters,
    mut slots: Vec<PrintedRarity>,
) -> (Vec<PrintedRarity>, PityCounters) {
    let mut forced = vec![false; slots.len()];
    for (rarity, counter, threshold) in [
        (PrintedRarity::Mythic, counters.mythic, PITY_MYTHIC_PACKS),
        (PrintedRarity::Legendary, counters.legendary, PITY_LEGENDARY_PACKS),
        (PrintedRarity::Epic, counters.epic, PITY_EPIC_PACKS),
    ] {
        if counter + 1 >= threshold
            && !slots.contains(&rarity)
            && let Some(at) = forced_slot(&slots, &forced, rarity)
        {
            slots[at] = rarity;
            forced[at] = true;
        }
    }
    let next = |counter: usize, rarity| if slots.contains(&rarity) { 0 } else { counter + 1 };
    let pity = PityCounters {
        epic: next(counters.epic, PrintedRarity::Epic),
        legendary: next(counters.legendary, PrintedRarity::Legendary),
        mythic: next(counters.mythic, PrintedRarity::Mythic),
    };
    (slots, pity)
}

/// The slot a force of `rarity` takes (R1339001), in slot order from the floored slot
/// (`PACK_FLOORED_SLOT`) on, never one an earlier force took: the first below `rarity` whose loss
/// takes no tracked rarity from the pack (a Common or a Rare, or a rarity another slot also
/// holds), so the force raises it; else the first whose rarity another slot also holds. Either
/// way the pack keeps every tracked rarity it rolled. `PACK_SLOTS >= TRACKED_RARITIES` means one
/// of the two always exists: past the earlier forces, the slots left outnumber the tracked
/// rarities neither forced nor this one, so a slot is Common or Rare or two share a rarity.
fn forced_slot(slots: &[PrintedRarity], forced: &[bool], rarity: PrintedRarity) -> Option<usize> {
    let order: Vec<usize> = std::iter::once(PACK_FLOORED_SLOT)
        .chain((0..slots.len()).filter(|at| *at != PACK_FLOORED_SLOT))
        .filter(|at| *at < slots.len() && !forced[*at])
        .collect();
    let shared = |at: usize| slots.iter().filter(|other| **other == slots[at]).count() > 1;
    let spare = |at: usize| slots[at] < PrintedRarity::Epic || shared(at);
    order
        .iter()
        .copied()
        .find(|at| slots[*at] < rarity && spare(*at))
        .or_else(|| order.iter().copied().find(|at| shared(*at)))
}

/// The level curve's exact total in `i128`: `total_xp_for_level` below saturates to
/// `i64::MAX` past level 429,496,730, and the correction loops in `level_for_xp` compare
/// against this, never the saturated total, so they cannot walk that plateau one level at
/// a time (at `i64::MAX` that walk would run from the ~429M guess to `u32::MAX`).
fn exact_total_xp_for_level(level: u32) -> i128 {
    let step = i128::from(SET_TRACK_XP_PER_LEVEL);
    step * i128::from(level.saturating_sub(1)) * i128::from(level) / 2
}

/// The total XP a set track holds at `level` (SPEC §14.7): going from level n to the next costs
/// `SET_TRACK_XP_PER_LEVEL` × n, so level 1 is 0 XP and the totals are the triangular numbers.
/// `i64` throughout so the uncapped curve cannot overflow on any level a track reaches.
pub fn total_xp_for_level(level: u32) -> i64 {
    i64::try_from(exact_total_xp_for_level(level)).unwrap_or(i64::MAX)
}

/// The level a set track with `total_xp` XP stands on (SPEC §14.7): the highest level whose
/// total is at most the XP. `xp <= 0` is level 1. The closed form is corrected by at most a few
/// `while` steps each way, never a bare cast, so boundaries land exactly and huge XP ends.
pub fn level_for_xp(total_xp: i64) -> u32 {
    if total_xp <= 0 {
        return 1;
    }
    let step = f64::from(SET_TRACK_XP_PER_LEVEL);
    let guess = ((1.0 + (1.0 + 8.0 * total_xp as f64 / step).sqrt()) / 2.0).floor();
    let mut level = guess.clamp(1.0, f64::from(u32::MAX)) as u32;
    let target = i128::from(total_xp);
    while level > 1 && exact_total_xp_for_level(level) > target {
        level -= 1;
    }
    while level < u32::MAX && exact_total_xp_for_level(level.saturating_add(1)) <= target {
        level += 1;
    }
    level
}

/// The XP from `total_xp` to the next level (SPEC §14.7).
pub fn xp_to_next_level(total_xp: i64) -> i64 {
    total_xp_for_level(level_for_xp(total_xp).saturating_add(1)) - total_xp
}

/// The XP the level `total_xp` stands on spans, from its start to the next level (SPEC §14.7):
/// `SET_TRACK_XP_PER_LEVEL` × that level, so a track's XP bar is this long.
pub fn level_span_xp(total_xp: i64) -> i64 {
    let level = level_for_xp(total_xp);
    total_xp_for_level(level.saturating_add(1)) - total_xp_for_level(level)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        CARD_TIER_WEIGHTS, CARD_TOKEN_BAR_POINTS_PER_DUPLICATE, CARD_TOKEN_BAR_SIZE,
        CARD_TOKEN_BAR_TARGET_PACKS, COSMETIC_TOKEN_BAR_POINTS_PER_DUPLICATE, COSMETIC_TOKEN_BAR_SIZE,
        COSMETIC_TOKEN_BAR_TARGET_PACKS, TIER_WEIGHT_TOTAL,
    };
    use crate::economy::roll::slot_tiers;
    use crate::economy::tiers::RATE_TOLERANCE;
    use jackioh_engine::rng::Rng;
    use jackioh_engine::wire::PrintedRarity::{Common, Epic, Legendary, Mythic, Rare};

    #[test]
    fn level_curve_reference_points() {
        assert_eq!(total_xp_for_level(1), 0);
        assert_eq!(total_xp_for_level(10), 4_500);
        assert_eq!(total_xp_for_level(100), 495_000);
        assert_eq!(level_for_xp(0), 1);
        assert_eq!(level_for_xp(4_499), 9);
        assert_eq!(level_for_xp(4_500), 10);
        assert_eq!(level_for_xp(495_000), 100);
        assert_eq!(xp_to_next_level(0), 100);
        assert_eq!(xp_to_next_level(4_500), 1_000);
    }

    #[test]
    fn level_span_reference_points() {
        assert_eq!(level_span_xp(0), 100);
        assert_eq!(level_span_xp(4_499), 900);
        assert_eq!(level_span_xp(4_500), 1_000);
        assert_eq!(level_span_xp(4_500), xp_to_next_level(4_500));
    }

    #[test]
    fn level_for_xp_huge_xp_ends() {
        // `total_xp_for_level` saturates to `i64::MAX` past level 429,496,730; comparing the
        // correction loops against it walked that plateau to `u32::MAX` at `i64::MAX`.
        assert_eq!(total_xp_for_level(429_496_730), 9_223_372_032_559_808_500);
        assert_eq!(total_xp_for_level(429_496_731), i64::MAX);
        assert_eq!(level_for_xp(i64::MAX), 429_496_730);
        assert_eq!(level_for_xp(9_223_372_032_559_808_500), 429_496_730);
        assert_eq!(level_for_xp(9_223_372_032_559_808_499), 429_496_729);
    }

    #[test]
    fn pity_thresholds_are_10_20_and_30_packs() {
        // #1333's table: pity never exceeds 30 packs.
        assert_eq!(PITY_EPIC_PACKS, 10);
        assert_eq!(PITY_LEGENDARY_PACKS, 20);
        assert_eq!(PITY_MYTHIC_PACKS, 30);
    }

    /// The tracked rarities with their thresholds, rarest last.
    const TRACKED: [(PrintedRarity, usize); TRACKED_RARITIES] = [
        (Epic, PITY_EPIC_PACKS),
        (Legendary, PITY_LEGENDARY_PACKS),
        (Mythic, PITY_MYTHIC_PACKS),
    ];

    /// Counters with `rarity`'s at `count` and the others at zero.
    fn counting(rarity: PrintedRarity, count: usize) -> PityCounters {
        let mut counters = PityCounters::default();
        *counter_mut(&mut counters, rarity) = count;
        counters
    }

    fn counter_mut(counters: &mut PityCounters, rarity: PrintedRarity) -> &mut usize {
        match rarity {
            Mythic => &mut counters.mythic,
            Legendary => &mut counters.legendary,
            _ => &mut counters.epic,
        }
    }

    fn counter_of(mut counters: PityCounters, rarity: PrintedRarity) -> usize {
        *counter_mut(&mut counters, rarity)
    }

    /// Every counter one pack short of its threshold, so this pack forces all three.
    fn all_due() -> PityCounters {
        PityCounters {
            epic: PITY_EPIC_PACKS - 1,
            legendary: PITY_LEGENDARY_PACKS - 1,
            mythic: PITY_MYTHIC_PACKS - 1,
        }
    }

    /// The slots in the order a force reads them: the floored one, then the rest in order.
    fn force_order() -> Vec<usize> {
        std::iter::once(PACK_FLOORED_SLOT)
            .chain((0..PACK_SLOTS).filter(|at| *at != PACK_FLOORED_SLOT))
            .collect()
    }

    /// A pack of Commons with `rarity` at each of `at`.
    fn pack_with(rarity: PrintedRarity, at: &[usize]) -> Vec<PrintedRarity> {
        let mut slots = vec![Common; PACK_SLOTS];
        for at in at {
            slots[*at] = rarity;
        }
        slots
    }

    #[test]
    fn r1298000_hard_pity_fires_on_threshold_pack() {
        let rares = vec![Rare; PACK_SLOTS];
        for (rarity, threshold) in TRACKED {
            let (slots, next) = apply_pity(counting(rarity, threshold - 1), rares.clone());
            assert_eq!(
                slots.iter().filter(|slot| **slot == rarity).count(),
                1,
                "{rarity}: {slots:?}"
            );
            assert_eq!(slots[PACK_FLOORED_SLOT], rarity, "{rarity}: {slots:?}");
            assert_eq!(counter_of(next, rarity), 0, "{rarity}");
            // A pack before, nothing is forced and the counter runs on.
            let (slots, next) = apply_pity(counting(rarity, threshold - 2), rares.clone());
            assert_eq!(slots, rares, "{rarity}");
            assert_eq!(counter_of(next, rarity), threshold - 1, "{rarity}");
        }
    }

    #[test]
    fn r1298000_mythic_leaves_other_counters_running() {
        let counters = PityCounters {
            epic: 5,
            legendary: 7,
            mythic: PITY_MYTHIC_PACKS - 1,
        };
        let after = PityCounters {
            epic: 6,
            legendary: 8,
            mythic: 0,
        };
        // A Mythic rolled: nothing is forced.
        let rolled = pack_with(Mythic, &[force_order()[1]]);
        assert_eq!(apply_pity(counters, rolled.clone()), (rolled, after));
        // A Mythic forced resets its own counter alone too.
        let (slots, next) = apply_pity(counters, vec![Rare; PACK_SLOTS]);
        assert_eq!(slots[PACK_FLOORED_SLOT], Mythic);
        assert_eq!(next, after);
    }

    #[test]
    fn r1339001_forces_take_the_floored_slot_first_highest_first() {
        let order = force_order();
        let mut rolled = vec![Common; PACK_SLOTS];
        rolled[PACK_FLOORED_SLOT] = Rare;
        let (slots, next) = apply_pity(all_due(), rolled);
        let mut expected = vec![Common; PACK_SLOTS];
        for (at, rarity) in order.iter().zip([Mythic, Legendary, Epic]) {
            expected[*at] = rarity;
        }
        assert_eq!(slots, expected);
        assert_eq!(next, PityCounters::default());
    }

    #[test]
    fn r1339001_a_force_only_raises_and_keeps_what_the_pack_rolled() {
        let order = force_order();
        let floored = PACK_FLOORED_SLOT;
        // A floored Legendary is not below an Epic: the Epic raises the next slot.
        let (slots, next) = apply_pity(
            counting(Epic, PITY_EPIC_PACKS - 1),
            pack_with(Legendary, &[floored]),
        );
        let mut expected = pack_with(Legendary, &[floored]);
        expected[order[1]] = Epic;
        assert_eq!(slots, expected);
        assert_eq!(next.epic, 0);
        // A Mythic forced over the pack's only Legendary would lose it: the next slot takes it, and
        // the Legendary resets its counter.
        let (slots, next) = apply_pity(
            PityCounters {
                legendary: 4,
                ..counting(Mythic, PITY_MYTHIC_PACKS - 1)
            },
            pack_with(Legendary, &[floored]),
        );
        let mut expected = pack_with(Legendary, &[floored]);
        expected[order[1]] = Mythic;
        assert_eq!(slots, expected);
        assert_eq!((next.legendary, next.mythic), (0, 0));
        // Two Epics: the floored one may go, since the other keeps the pack's Epic.
        let (slots, next) = apply_pity(
            PityCounters {
                epic: 3,
                ..counting(Mythic, PITY_MYTHIC_PACKS - 1)
            },
            pack_with(Epic, &[floored, order[1]]),
        );
        let mut expected = pack_with(Epic, &[floored, order[1]]);
        expected[floored] = Mythic;
        assert_eq!(slots, expected);
        assert_eq!((next.epic, next.mythic), (0, 0));
        // Nothing below a Legendary: it takes the floored slot, whose Mythic another slot keeps.
        let (slots, next) = apply_pity(
            counting(Legendary, PITY_LEGENDARY_PACKS - 1),
            vec![Mythic; PACK_SLOTS],
        );
        let mut expected = vec![Mythic; PACK_SLOTS];
        expected[floored] = Legendary;
        assert_eq!(slots, expected);
        assert_eq!((next.legendary, next.mythic), (0, 0));
        // A pack that already holds its rarity is left alone.
        let rolled = pack_with(Epic, &[order[PACK_SLOTS - 1]]);
        let (slots, next) = apply_pity(counting(Epic, PITY_EPIC_PACKS - 1), rolled.clone());
        assert_eq!(slots, rolled);
        assert_eq!(next.epic, 0);
    }

    /// The seeded simulation (SPEC §14.4, R1339001): 200,000 card packs, each `slot_tiers` from
    /// `Rng::new("jackioh-economy-1339", 0)` with `apply_pity` on top. It reports each tracked
    /// rarity's share of packs holding it with pity and before it, `1 − (1 − w/TIER_WEIGHT_TOTAL)^
    /// PACK_SLOTS` (30.4%, 9.6% and 4.9%), and tunes nothing to them: the rate with pity is at
    /// least the rate before it, within `RATE_TOLERANCE`, and no counter ever reaches its threshold
    /// (pity fires on the threshold pack, so the largest counter seen stays one below it).
    #[test]
    fn r1339001_the_simulation_reports_pity_on_top_and_caps_every_counter() {
        const PACKS: usize = 200_000;
        let mut rng = Rng::new("jackioh-economy-1339", 0);
        let mut counters = PityCounters::default();
        let mut held = [0_usize; TRACKED_RARITIES];
        let mut most = [0_usize; TRACKED_RARITIES];
        for _ in 0..PACKS {
            let (slots, next) = apply_pity(counters, slot_tiers(&mut rng));
            counters = next;
            for (at, (rarity, _)) in TRACKED.into_iter().enumerate() {
                held[at] += usize::from(slots.contains(&rarity));
                most[at] = most[at].max(counter_of(counters, rarity));
            }
        }
        let weights = [
            CARD_TIER_WEIGHTS.epic,
            CARD_TIER_WEIGHTS.legendary,
            CARD_TIER_WEIGHTS.mythic,
        ];
        for (at, (rarity, threshold)) in TRACKED.into_iter().enumerate() {
            let with_pity = held[at] as f64 / PACKS as f64;
            let slot = f64::from(weights[at]) / f64::from(TIER_WEIGHT_TOTAL);
            let before_pity = 1.0 - (1.0 - slot).powi(PACK_SLOTS as i32);
            println!("{rarity}: {with_pity:.4} of packs with pity, {before_pity:.4} before it");
            assert!(
                with_pity >= before_pity - RATE_TOLERANCE,
                "{rarity}: {with_pity} with pity, under {before_pity} before it"
            );
            assert!(most[at] < threshold, "{rarity}'s counter reached {}", most[at]);
        }
    }

    /// The bar sizes as the simulation's collection assumes it (SPEC §14.5): a complete
    /// collection, every slot a spent duplicate, which is the stated approximation of §14.5's
    /// mostly-owned collection. Then each bar fills exactly at its target pack count: points
    /// per duplicate times slots per pack times target packs equals the bar's size.
    #[test]
    fn bar_points_fill_bars_at_target_packs() {
        assert_eq!(
            CARD_TOKEN_BAR_POINTS_PER_DUPLICATE * PACK_SLOTS as i64 * CARD_TOKEN_BAR_TARGET_PACKS as i64,
            CARD_TOKEN_BAR_SIZE
        );
        assert_eq!(
            COSMETIC_TOKEN_BAR_POINTS_PER_DUPLICATE
                * PACK_SLOTS as i64
                * COSMETIC_TOKEN_BAR_TARGET_PACKS as i64,
            COSMETIC_TOKEN_BAR_SIZE
        );
    }
}
