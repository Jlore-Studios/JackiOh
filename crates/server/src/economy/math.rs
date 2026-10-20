//! The collection economy's pure math (SPEC §14, #1298): the set-track level curve (§14.7)
//! and the pity roll (§14.4). No store, no clock, no I/O: the pack roll calls [`roll_pack`] with
//! its own seeded, logged draws, and the tracks call the level curve with the account's total XP.
//! The numbers live in [`crate::config`]; this module only computes with them (CLAUDE.md rule 9).
//!
//! - [`PityCounters`] is one rarity counter set per account, shared by every card pack it opens
//!   (R1298000): each counts card packs, not slots, since its rarity last dropped.
//! - [`roll_pack`] is the pity math, pure from the counters and a draw to the rarities and the
//!   next counters.

use jackioh_engine::wire::PrintedRarity;
use serde::{Deserialize, Serialize};

use crate::config::{
    PACK_BASE_RATES, PITY_EPIC_PACKS, PITY_LEGENDARY_PACKS, PITY_MYTHIC_PACKS, SET_TRACK_XP_PER_LEVEL,
};

// SPEC §14.4: each rarity's threshold is a longer wait than the commoner one's, and Mythic's the
// longest of all.
const _: () = assert!(PITY_EPIC_PACKS < PITY_LEGENDARY_PACKS && PITY_LEGENDARY_PACKS < PITY_MYTHIC_PACKS);

/// One pity counter set per account (R1298000, SPEC §14.4): each counter is the card packs
/// since a pack held its rarity, whatever that slot then granted. All three count every card
/// pack the account opens, global and per-set alike.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PityCounters {
    pub epic: usize,
    pub legendary: usize,
    pub mythic: usize,
}

/// One draw per tracked rarity for a single card pack (§14.4). The pack roll deals these from
/// its own seeded, logged generator; the pity math takes them as input so it stays pure.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PackDraws {
    pub epic: f64,
    pub legendary: f64,
    pub mythic: f64,
}

/// Which tracked rarities one card pack holds (§14.4): a pack holds a rarity when a slot rolls
/// it, and the Rare-or-better slot (§14.3) means a pack never holds less than Rare. Slot
/// assignment itself is the pack roll's, not this module's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PackOutcome {
    pub has_epic: bool,
    pub has_legendary: bool,
    pub has_mythic: bool,
}

impl PackOutcome {
    /// The pack's best rarity: Mythic, Legendary or Epic when one dropped, else the
    /// Rare-or-better slot's floor, Rare (SPEC §14.3).
    pub fn best(&self) -> PrintedRarity {
        if self.has_mythic {
            PrintedRarity::Mythic
        } else if self.has_legendary {
            PrintedRarity::Legendary
        } else if self.has_epic {
            PrintedRarity::Epic
        } else {
            PrintedRarity::Rare
        }
    }
}

/// One rarity's roll: with counter `counter`, threshold `threshold`, base chance `base` and
/// draw `draw`, the threshold pack forces the drop (`counter + 1 >= threshold` gives
/// `(true, 0)`), else the draw is read against the base, and a pack with no drop adds one.
/// Only the dropped rarity's counter resets (§14.4).
fn roll_one(counter: usize, threshold: usize, base: f64, draw: f64) -> (bool, usize) {
    if counter + 1 >= threshold || draw < base {
        (true, 0)
    } else {
        (false, counter + 1)
    }
}

/// The pity math (SPEC §14.4, R1298000): from the pack's pity counters and one draw per
/// tracked rarity to the rarities the pack holds and the next counters. The three rarities
/// roll independently: a Mythic leaves the Epic and Legendary counters running, and a
/// Legendary the Epic one.
pub fn roll_pack(counters: PityCounters, draws: PackDraws) -> (PackOutcome, PityCounters) {
    let (has_epic, epic) = roll_one(counters.epic, PITY_EPIC_PACKS, PACK_BASE_RATES.epic, draws.epic);
    let (has_legendary, legendary) = roll_one(
        counters.legendary,
        PITY_LEGENDARY_PACKS,
        PACK_BASE_RATES.legendary,
        draws.legendary,
    );
    let (has_mythic, mythic) = roll_one(
        counters.mythic,
        PITY_MYTHIC_PACKS,
        PACK_BASE_RATES.mythic,
        draws.mythic,
    );
    (
        PackOutcome {
            has_epic,
            has_legendary,
            has_mythic,
        },
        PityCounters {
            epic,
            legendary,
            mythic,
        },
    )
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
        CARD_TOKEN_BAR_POINTS_PER_DUPLICATE, CARD_TOKEN_BAR_SIZE, CARD_TOKEN_BAR_TARGET_PACKS,
        COSMETIC_TOKEN_BAR_POINTS_PER_DUPLICATE, COSMETIC_TOKEN_BAR_SIZE, COSMETIC_TOKEN_BAR_TARGET_PACKS,
        PACK_RATE_TOLERANCE, PACK_SLOTS, PACK_TARGET_RATES,
    };
    use jackioh_engine::rng::Rng;

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

    #[test]
    fn r1298000_guaranteed_slot_never_common() {
        let draws = PackDraws {
            epic: 0.999,
            legendary: 0.999,
            mythic: 0.999,
        };
        let (outcome, _) = roll_pack(PityCounters::default(), draws);
        assert!(outcome.best() >= PrintedRarity::Rare);
    }

    #[test]
    fn r1298000_hard_pity_fires_on_threshold_pack() {
        let draws = PackDraws {
            epic: 0.999,
            legendary: 0.999,
            mythic: 0.999,
        };
        let (outcome, next) = roll_pack(
            PityCounters {
                epic: PITY_EPIC_PACKS - 1,
                legendary: 0,
                mythic: 0,
            },
            draws,
        );
        assert!((outcome.has_epic, next.epic) == (true, 0));
        let (outcome, next) = roll_pack(
            PityCounters {
                epic: 0,
                legendary: PITY_LEGENDARY_PACKS - 1,
                mythic: 0,
            },
            draws,
        );
        assert!((outcome.has_legendary, next.legendary) == (true, 0));
        let (outcome, next) = roll_pack(
            PityCounters {
                epic: 0,
                legendary: 0,
                mythic: PITY_MYTHIC_PACKS - 1,
            },
            draws,
        );
        assert!((outcome.has_mythic, next.mythic) == (true, 0));
    }

    #[test]
    fn r1298000_mythic_leaves_other_counters_running() {
        let (outcome, next) = roll_pack(
            PityCounters {
                epic: 5,
                legendary: 7,
                mythic: PITY_MYTHIC_PACKS - 1,
            },
            PackDraws {
                epic: 0.999,
                legendary: 0.999,
                mythic: 0.0,
            },
        );
        assert_eq!(
            outcome,
            PackOutcome {
                has_epic: false,
                has_legendary: false,
                has_mythic: true,
            }
        );
        assert_eq!(
            next,
            PityCounters {
                epic: 6,
                legendary: 8,
                mythic: 0,
            }
        );
    }

    /// The seeded simulation (SPEC §14.4): 200,000 packs through the pity math from
    /// `Rng::new("jackioh-economy-1298", 0)`, one draw per tracked rarity per pack in
    /// epic/legendary/mythic order. Each rarity's share of packs holding it must land within
    /// `PACK_RATE_TOLERANCE` of `PACK_TARGET_RATES`, and no counter may ever reach its
    /// threshold (hard pity fires on the threshold pack, so the largest counter seen stays one
    /// below it).
    #[test]
    fn pity_simulation_hits_target_rates_and_caps_counters() {
        const PACKS: usize = 200_000;
        let mut rng = Rng::new("jackioh-economy-1298", 0);
        let mut counters = PityCounters::default();
        let (mut epics, mut legendaries, mut mythics) = (0_usize, 0_usize, 0_usize);
        let (mut max_epic, mut max_legendary, mut max_mythic) = (0_usize, 0_usize, 0_usize);
        for _ in 0..PACKS {
            let draws = PackDraws {
                epic: rng.next(),
                legendary: rng.next(),
                mythic: rng.next(),
            };
            let (outcome, next) = roll_pack(counters, draws);
            counters = next;
            max_epic = max_epic.max(counters.epic);
            max_legendary = max_legendary.max(counters.legendary);
            max_mythic = max_mythic.max(counters.mythic);
            epics += usize::from(outcome.has_epic);
            legendaries += usize::from(outcome.has_legendary);
            mythics += usize::from(outcome.has_mythic);
        }
        let packs = PACKS as f64;
        assert!(
            (epics as f64 / packs - PACK_TARGET_RATES.epic).abs() <= PACK_RATE_TOLERANCE,
            "epic rate {} outside tolerance of {}",
            epics as f64 / packs,
            PACK_TARGET_RATES.epic
        );
        assert!(
            (legendaries as f64 / packs - PACK_TARGET_RATES.legendary).abs() <= PACK_RATE_TOLERANCE,
            "legendary rate {} outside tolerance of {}",
            legendaries as f64 / packs,
            PACK_TARGET_RATES.legendary
        );
        assert!(
            (mythics as f64 / packs - PACK_TARGET_RATES.mythic).abs() <= PACK_RATE_TOLERANCE,
            "mythic rate {} outside tolerance of {}",
            mythics as f64 / packs,
            PACK_TARGET_RATES.mythic
        );
        assert!(max_epic < PITY_EPIC_PACKS);
        assert!(max_legendary < PITY_LEGENDARY_PACKS);
        assert!(max_mythic < PITY_MYTHIC_PACKS);
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
