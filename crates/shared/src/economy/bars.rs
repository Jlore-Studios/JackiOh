//! The token bars (SPEC §14.5; #1305, #1953, #2728): every opened slot adds points to a card token bar
//! and a cosmetic token bar, and a full bar pays. The card bar pays a card token at one size; the cosmetic
//! bar walks `COSMETIC_TOKEN_CYCLE`, each step at its own size (R2728000). A bar past its size pays for
//! every size it holds, so one that kept counting before bars paid (R1301002) settles whole (R1305000).
//!
//! [`settle_bars`] is the decision an opening makes over the counters it read: what each bar pays, the
//! points each keeps and the [`LedgerGrant`]s that write it, in the order the store writes them (R1305000,
//! R1305001); [`fill_bar`] is a dev account's fill (R1310003), which pays as a pack's duplicates filling the
//! bar would.

use serde::{Deserialize, Serialize};

use jackioh_engine::wire::PrintedRarity;

use crate::config::{
    CARD_TOKEN_BAR_SIZE, COSMETIC_TOKEN_CYCLE, CosmeticTokenStep, TOKEN_BAR_POINTS_BY_CARD_RARITY,
    TOKEN_BAR_POINTS_BY_COSMETIC_RARITY,
};
use crate::cosmetics::CosmeticRarity;
use crate::economy::ledger::{AccountCounters, Cause, Counter, InventoryItem, LedgerGrant};
use crate::economy::roll::Slot;

/// A bar's points and its size, as the token and cosmetic pack screens read it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct BarState {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub points: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub size: i64,
}

/// A bar's points split into payouts and overflow (R1305000): `pays` fills at `size`, and
/// `remainder` is what the bar keeps. A bar past its size pays for every size it holds, so a bar
/// that kept counting before bars paid (R1301002) settles whole.
pub fn settle_bar(start: i64, gained: i64, size: i64) -> (i64, i64) {
    let total = start + gained;
    (total / size, total % size)
}

/// R2728000: the cosmetic token bar settled step by step from the cycle's stored `position`: each
/// step pays when the points reach its own size (`CosmeticTokenStep::bar_size`), taken off before
/// the next step's size is compared, so one gain can pay several steps of sizes that differ, and
/// a bar past its step's size settles whole (R1305000). The steps paid, in order, and the points
/// kept.
pub fn settle_cosmetic_bar(start: i64, gained: i64, position: i64) -> (Vec<CosmeticTokenStep>, i64) {
    let mut points = start + gained;
    let mut paid = Vec::new();
    loop {
        let step = cycle_at(position + count(&paid));
        if points < step.bar_size() {
            return (paid, points);
        }
        points -= step.bar_size();
        paid.push(step);
    }
}

/// The step the cosmetic token cycle stands at after `position` steps paid (§14.5).
pub fn cycle_at(position: i64) -> CosmeticTokenStep {
    // `rem_euclid` of a length is below the length, so the index is in range.
    COSMETIC_TOKEN_CYCLE[position.rem_euclid(COSMETIC_TOKEN_CYCLE.len() as i64) as usize]
}

/// What the cosmetic token cycle pays at `position`, its steps paid so far (§14.5; a Legendary token among
/// them, R2728000): the inventory row it adds to. A pack opening's bar and a dev account's fill
/// (R1310003) both pay through it.
pub fn cycle_step(position: i64) -> (InventoryItem, String) {
    match cycle_at(position) {
        CosmeticTokenStep::Rare => (
            InventoryItem::CosmeticToken,
            CosmeticRarity::Rare.as_str().to_string(),
        ),
        CosmeticTokenStep::Epic => (
            InventoryItem::CosmeticToken,
            CosmeticRarity::Epic.as_str().to_string(),
        ),
        CosmeticTokenStep::Legendary => (
            InventoryItem::CosmeticToken,
            CosmeticRarity::Legendary.as_str().to_string(),
        ),
        CosmeticTokenStep::CosmeticPack => (InventoryItem::CosmeticPack, String::new()),
    }
}

/// The points each opened slot of a card pack adds to each token bar, whatever it grants: its
/// slot's card rarity's, pity's force included, in `TOKEN_BAR_POINTS_BY_CARD_RARITY` (R1953000,
/// R1999000).
pub fn card_bar_points(rarity: PrintedRarity) -> i64 {
    let points = TOKEN_BAR_POINTS_BY_CARD_RARITY;
    match rarity {
        PrintedRarity::Common => points.common,
        PrintedRarity::Rare => points.rare,
        PrintedRarity::Epic => points.epic,
        PrintedRarity::Legendary => points.legendary,
        PrintedRarity::Mythic => points.mythic,
    }
}

/// The points each opened slot of a cosmetic pack adds to each token bar, whatever it grants: its
/// rolled cosmetic rarity's, not the one it fell to, in `TOKEN_BAR_POINTS_BY_COSMETIC_RARITY`
/// (R1306001, R1953000, R1999000). A cosmetic rarity is never read as a card's (R1333000), so it
/// has a table of its own.
pub fn cosmetic_bar_points(rarity: CosmeticRarity) -> i64 {
    let points = TOKEN_BAR_POINTS_BY_COSMETIC_RARITY;
    match rarity {
        CosmeticRarity::Common => points.common,
        CosmeticRarity::Rare => points.rare,
        CosmeticRarity::Epic => points.epic,
        CosmeticRarity::Legendary => points.legendary,
        CosmeticRarity::Mythic => points.mythic,
    }
}

/// The points each of a card pack's slots adds to each token bar, in slot order: its slot's card
/// rarity's ([`card_bar_points`]), whatever it grants (R1953000, R1999000).
pub fn slot_bar_points(slots: &[Slot]) -> Vec<i64> {
    slots.iter().map(|slot| card_bar_points(slot.rarity)).collect()
}

/// One token bar across an opening (R1305000): its points `before` the opening and `after` it (the
/// overflow a pay leaves kept), its `size` and how many times it `pays` in this opening. Every
/// opening moves both bars, since every slot adds points (R1999000). The cosmetic bar's `size` is
/// the size of the step it stands at after the opening (R2728000).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct BarSettle {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub before: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub after: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub size: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub pays: i64,
}

/// What an opening did to both token bars (R1305000, R1305001), for the client to animate from where each
/// stood to where the opening left it: the cosmetic bar's `cosmetic_paid` are the cycle's steps it paid, in
/// order (so its length is `cosmetic.pays`), `cosmetic_sizes` the size of each step its sweep crosses
/// (R2728000), and `slot_points` are the points each slot added to each bar, in slot order, so the client
/// sends each slot's points to the bars without counting (R1953000, R1999000).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct BarsSettled {
    pub card: BarSettle,
    pub cosmetic: BarSettle,
    pub cosmetic_paid: Vec<CosmeticTokenStep>,
    /// R2728000: the size of each step the cosmetic bar's sweep crosses: each step paid, in order,
    /// then the step it stands at after (`cosmetic.size`), so `cosmetic.pays + 1` entries.
    #[cfg_attr(feature = "ts", ts(type = "Array<number>"))]
    pub cosmetic_sizes: Vec<i64>,
    #[cfg_attr(feature = "ts", ts(type = "Array<number>"))]
    pub slot_points: Vec<i64>,
}

/// A length as a ledger delta.
fn count<T>(items: &[T]) -> i64 {
    i64::try_from(items.len()).unwrap_or(i64::MAX)
}

/// Both token bars' points for an opening's slots, `slot_points` each slot's to each bar
/// ([`slot_bar_points`], R1953000), each bar paying when it reaches its size (R1305000, R1305001), the
/// cosmetic bar step by step at each step's own size ([`settle_cosmetic_bar`], R2728000); `counters` is the
/// account's before the opening. The grants are the writes, in order: a bar that pays has its points
/// replaced by what it keeps (`TokenBar`) and its tokens granted, the cosmetic bar's one per step paid, in
/// order, then the cycle's position advanced; a bar that does not keeps what it gained (`PackOpen`). A card
/// pack's opening and a cosmetic pack's both settle through it (R1306001).
pub fn settle_bars(counters: &AccountCounters, slot_points: Vec<i64>) -> (BarsSettled, Vec<LedgerGrant>) {
    // Every slot's points go to both bars (R1953000, R1999000).
    let gained: i64 = slot_points.iter().sum();
    let card_bar_before = counters.card_bar;
    let cosmetic_bar_before = counters.cosmetic_bar;
    let cycle_before = counters.cosmetic_cycle;
    let mut grants = Vec::new();

    // The card token bar (R1305000): the paying write replaces the accrual row, so still one row
    // per bar per opening.
    let (card_pays, card_remainder) = settle_bar(card_bar_before, gained, CARD_TOKEN_BAR_SIZE);
    if card_pays > 0 {
        grants.extend(LedgerGrant::counter(
            Counter::CardTokenBar,
            card_remainder - card_bar_before,
            Cause::TokenBar,
        ));
        grants.push(LedgerGrant::Inventory {
            item: InventoryItem::CardToken,
            key: String::new(),
            delta: card_pays,
            cause: Cause::TokenBar,
            reference: None,
        });
    } else {
        grants.extend(LedgerGrant::counter(
            Counter::CardTokenBar,
            gained,
            Cause::PackOpen,
        ));
    }

    // The cosmetic token bar (R1305000, R1305001): as above, and each payout is the cycle's next
    // step, paid at its own size (R2728000), whose stored position then advances by the payouts.
    let (cosmetic_paid, cosmetic_remainder) = settle_cosmetic_bar(cosmetic_bar_before, gained, cycle_before);
    let cosmetic_pays = count(&cosmetic_paid);
    let resting = cycle_at(cycle_before + cosmetic_pays);
    if cosmetic_pays > 0 {
        grants.extend(LedgerGrant::counter(
            Counter::CosmeticTokenBar,
            cosmetic_remainder - cosmetic_bar_before,
            Cause::TokenBar,
        ));
        for paid in 0..cosmetic_pays {
            let (item, key) = cycle_step(cycle_before + paid);
            grants.push(LedgerGrant::Inventory {
                item,
                key,
                delta: 1,
                cause: Cause::TokenBar,
                reference: None,
            });
        }
        grants.extend(LedgerGrant::counter(
            Counter::CosmeticTokenCycle,
            cosmetic_pays,
            Cause::TokenBar,
        ));
    } else {
        grants.extend(LedgerGrant::counter(
            Counter::CosmeticTokenBar,
            gained,
            Cause::PackOpen,
        ));
    }
    let settled = BarsSettled {
        card: BarSettle {
            before: card_bar_before,
            after: card_remainder,
            size: CARD_TOKEN_BAR_SIZE,
            pays: card_pays,
        },
        cosmetic: BarSettle {
            before: cosmetic_bar_before,
            after: cosmetic_remainder,
            size: resting.bar_size(),
            pays: cosmetic_pays,
        },
        cosmetic_sizes: cosmetic_paid
            .iter()
            .chain([&resting])
            .map(|step| step.bar_size())
            .collect(),
        cosmetic_paid,
        slot_points,
    };
    (settled, grants)
}

/// Which token bar a dev account fills (R1310003).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum BarKind {
    Card,
    Cosmetic,
}

impl BarKind {
    /// The bar a fill's name gives (`card` or `cosmetic`), or `None` for no bar.
    pub fn named(name: &str) -> Option<BarKind> {
        match name {
            "card" => Some(BarKind::Card),
            "cosmetic" => Some(BarKind::Cosmetic),
            _ => None,
        }
    }

    /// The counter that holds the bar's points.
    pub fn counter(self) -> Counter {
        match self {
            BarKind::Card => Counter::CardTokenBar,
            BarKind::Cosmetic => Counter::CosmeticTokenBar,
        }
    }
}

/// What a dev fill does (R1310003): the bar's points `held` and `remainder` after, the tokens it `pays`, and
/// the writes (all `Cause::Dev`'s), in order.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct BarFill {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub held: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub remainder: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub pays: i64,
    pub grants: Vec<LedgerGrant>,
}

/// A dev account's fill of `bar` (R1310003): the bar gains what it lacks of its size, the cosmetic bar's its
/// current step's (R2728000), and pays as a pack's duplicates filling it would. A bar already past its size
/// pays every size it holds first, as an opening would, and the fill tops up the rest.
pub fn fill_bar(bar: BarKind, counters: &AccountCounters) -> BarFill {
    let held = match bar {
        BarKind::Card => counters.card_bar,
        BarKind::Cosmetic => counters.cosmetic_bar,
    };
    let cycle = counters.cosmetic_cycle;
    let (pays, remainder) = match bar {
        BarKind::Card => {
            let size = CARD_TOKEN_BAR_SIZE;
            settle_bar(held, size - held % size, size)
        }
        BarKind::Cosmetic => {
            // The step the bar stands at once what it holds is paid, and what it lacks of that step's
            // size (R2728000).
            let (owed, kept) = settle_cosmetic_bar(held, 0, cycle);
            let lacks = cycle_at(cycle + count(&owed)).bar_size() - kept;
            let (paid, remainder) = settle_cosmetic_bar(held, lacks, cycle);
            (count(&paid), remainder)
        }
    };
    let mut grants = Vec::new();
    grants.extend(LedgerGrant::counter(bar.counter(), remainder - held, Cause::Dev));
    match bar {
        BarKind::Card => grants.push(LedgerGrant::Inventory {
            item: InventoryItem::CardToken,
            key: String::new(),
            delta: pays,
            cause: Cause::Dev,
            reference: None,
        }),
        BarKind::Cosmetic => {
            for paid in 0..pays {
                let (item, key) = cycle_step(cycle + paid);
                grants.push(LedgerGrant::Inventory {
                    item,
                    key,
                    delta: 1,
                    cause: Cause::Dev,
                    reference: None,
                });
            }
            grants.push(LedgerGrant::Counter {
                counter: Counter::CosmeticTokenCycle,
                delta: pays,
                cause: Cause::Dev,
            });
        }
    }
    BarFill {
        held,
        remainder,
        pays,
        grants,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CosmeticTokenStep::{CosmeticPack, Epic, Legendary, Rare};
    use crate::config::{CARD_TOKEN_BAR_SIZE, COSMETIC_TOKEN_BAR_SIZE_RARE, COSMETIC_TOKEN_CYCLE};

    #[test]
    fn r1305000_settle_bar_pays_once_and_keeps_the_overflow() {
        assert_eq!(
            settle_bar(CARD_TOKEN_BAR_SIZE - 1, 3, CARD_TOKEN_BAR_SIZE),
            (1, 2)
        );
        assert_eq!(
            settle_cosmetic_bar(COSMETIC_TOKEN_BAR_SIZE_RARE - 1, 3, 0),
            (vec![Rare], 2)
        );
    }

    #[test]
    fn r1305000_settle_bar_keeps_counting_past_its_size() {
        assert_eq!(
            settle_bar(CARD_TOKEN_BAR_SIZE + 2, 1, CARD_TOKEN_BAR_SIZE),
            (1, 3)
        );
        assert_eq!(
            settle_bar(0, CARD_TOKEN_BAR_SIZE * 2 + 1, CARD_TOKEN_BAR_SIZE),
            (2, 1)
        );
        assert_eq!(settle_bar(7, 0, CARD_TOKEN_BAR_SIZE), (0, 7));
    }

    #[test]
    fn r1953000_points_follow_the_rarity_and_a_bar_at_195_gaining_10_pays_once_keeping_5() {
        let cards = [
            PrintedRarity::Common,
            PrintedRarity::Rare,
            PrintedRarity::Epic,
            PrintedRarity::Legendary,
            PrintedRarity::Mythic,
        ]
        .map(card_bar_points);
        let cosmetics = CosmeticRarity::ALL.map(cosmetic_bar_points);
        assert_eq!(cards, [1, 2, 3, 5, 10]);
        assert_eq!(cosmetics, [1, 2, 3, 5, 10]);
        assert_eq!(cards.iter().sum::<i64>(), 21);
        assert_eq!(settle_bar(195, 10, CARD_TOKEN_BAR_SIZE), (1, 5));
        assert_eq!(
            settle_cosmetic_bar(COSMETIC_TOKEN_BAR_SIZE_RARE - 5, 10, 0),
            (vec![Rare], 5)
        );
    }

    #[test]
    fn r2728000_each_step_pays_at_its_own_size() {
        assert_eq!(
            [Rare, Epic, Legendary, CosmeticPack].map(CosmeticTokenStep::bar_size),
            [50, 75, 100, 100]
        );
        assert_eq!(
            COSMETIC_TOKEN_CYCLE,
            [Rare, Epic, Legendary, Rare, Epic, CosmeticPack]
        );
        // One cycle: 450 points, down from #1953's 1,200.
        assert_eq!(
            COSMETIC_TOKEN_CYCLE
                .iter()
                .map(|step| step.bar_size())
                .sum::<i64>(),
            450
        );
        // At every place of the cycle, a point short pays nothing and the last point pays that
        // place's step, keeping nothing; the place is counted from the stored position, so a
        // second lap pays as the first.
        for position in 0..2 * COSMETIC_TOKEN_CYCLE.len() as i64 {
            let step = COSMETIC_TOKEN_CYCLE[position as usize % COSMETIC_TOKEN_CYCLE.len()];
            let short = step.bar_size() - 1;
            assert_eq!(
                settle_cosmetic_bar(short, 0, position),
                (vec![], short),
                "{position}"
            );
            assert_eq!(
                settle_cosmetic_bar(short, 1, position),
                (vec![step], 0),
                "{position}"
            );
        }
    }

    #[test]
    fn r2728000_overflow_carries_into_the_next_step_at_its_own_size() {
        // (start, gained, position) → (steps paid, points kept).
        assert_eq!(settle_cosmetic_bar(45, 10, 0), (vec![Rare], 5));
        // At the Epic step, 230 points pay Epic (75), Legendary (100) and Rare (50), keeping 5.
        assert_eq!(settle_cosmetic_bar(70, 160, 1), (vec![Epic, Legendary, Rare], 5));
        // The cycle wraps: the pack, then the next lap's Rare.
        assert_eq!(settle_cosmetic_bar(100, 50, 5), (vec![CosmeticPack, Rare], 0));
        // A bar the 200-point bar left at 199 (R1953000) settles whole at the Rare step: Rare and
        // Epic, keeping 74, short of Legendary's 100.
        assert_eq!(settle_cosmetic_bar(199, 0, 0), (vec![Rare, Epic], 74));
        // Two cycles' points pay both laps in order and keep nothing.
        let two_laps: Vec<CosmeticTokenStep> = COSMETIC_TOKEN_CYCLE
            .iter()
            .chain(COSMETIC_TOKEN_CYCLE.iter())
            .copied()
            .collect();
        assert_eq!(settle_cosmetic_bar(0, 900, 0), (two_laps, 0));
        assert_eq!(settle_cosmetic_bar(7, 0, 3), (vec![], 7));
    }

    /// R1305000, R1305001: an opening's grants, in the order the store writes them. A bar that pays has its
    /// points replaced by what it keeps and its tokens granted, the cosmetic bar's per step and then the cycle's
    /// position advanced; a bar that does not pay keeps what it gained, a zero gain writing nothing.
    #[test]
    fn r1305000_r1305001_settle_bars_writes_a_paying_bar_and_a_keeping_one() {
        let counters = AccountCounters {
            card_bar: 195,
            cosmetic_bar: 70,
            cosmetic_cycle: 7,
            ..AccountCounters::default()
        };
        let (settled, grants) = settle_bars(&counters, vec![2, 3, 5]);
        assert_eq!(
            settled.card,
            BarSettle {
                before: 195,
                after: 5,
                size: 200,
                pays: 1
            }
        );
        assert_eq!(settled.cosmetic.pays, 1);
        assert_eq!(settled.cosmetic_paid, [Epic]);
        assert_eq!(settled.cosmetic_sizes, [75, 100]);
        assert_eq!(
            grants,
            [
                LedgerGrant::Counter {
                    counter: Counter::CardTokenBar,
                    delta: -190,
                    cause: Cause::TokenBar
                },
                LedgerGrant::Inventory {
                    item: InventoryItem::CardToken,
                    key: String::new(),
                    delta: 1,
                    cause: Cause::TokenBar,
                    reference: None,
                },
                LedgerGrant::Counter {
                    counter: Counter::CosmeticTokenBar,
                    delta: -65,
                    cause: Cause::TokenBar
                },
                LedgerGrant::Inventory {
                    item: InventoryItem::CosmeticToken,
                    key: "Epic".to_string(),
                    delta: 1,
                    cause: Cause::TokenBar,
                    reference: None,
                },
                LedgerGrant::Counter {
                    counter: Counter::CosmeticTokenCycle,
                    delta: 1,
                    cause: Cause::TokenBar
                },
            ]
        );
        // Neither bar pays: each keeps the gain, and a gain of nothing writes nothing.
        let (settled, grants) = settle_bars(&AccountCounters::default(), vec![1, 1]);
        assert_eq!((settled.card.pays, settled.cosmetic.pays), (0, 0));
        assert_eq!(settled.cosmetic_sizes, [50]);
        assert_eq!(
            grants,
            [
                LedgerGrant::Counter {
                    counter: Counter::CardTokenBar,
                    delta: 2,
                    cause: Cause::PackOpen
                },
                LedgerGrant::Counter {
                    counter: Counter::CosmeticTokenBar,
                    delta: 2,
                    cause: Cause::PackOpen
                },
            ]
        );
        assert!(settle_bars(&AccountCounters::default(), vec![]).1.is_empty());
    }

    /// R1310003: a fill tops the bar up to its size and pays; a bar past its size pays what it holds first.
    #[test]
    fn r1310003_a_fill_pays_as_the_duplicates_filling_the_bar_would() {
        let card = fill_bar(
            BarKind::Card,
            &AccountCounters {
                card_bar: 7,
                ..AccountCounters::default()
            },
        );
        assert_eq!((card.held, card.remainder, card.pays), (7, 0, 1));
        assert_eq!(card.grants.len(), 2);
        // A card bar already past its size pays every size it holds, then tops up the rest.
        let past = fill_bar(
            BarKind::Card,
            &AccountCounters {
                card_bar: 250,
                ..AccountCounters::default()
            },
        );
        assert_eq!((past.remainder, past.pays), (0, 2));
        // The cosmetic bar's current step is its size (R2728000): at the Epic step, 75.
        let cosmetic = fill_bar(
            BarKind::Cosmetic,
            &AccountCounters {
                cosmetic_bar: 10,
                cosmetic_cycle: 1,
                ..AccountCounters::default()
            },
        );
        assert_eq!((cosmetic.remainder, cosmetic.pays), (0, 1));
        assert_eq!(
            cosmetic.grants,
            [
                LedgerGrant::Counter {
                    counter: Counter::CosmeticTokenBar,
                    delta: -10,
                    cause: Cause::Dev
                },
                LedgerGrant::Inventory {
                    item: InventoryItem::CosmeticToken,
                    key: "Epic".to_string(),
                    delta: 1,
                    cause: Cause::Dev,
                    reference: None,
                },
                LedgerGrant::Counter {
                    counter: Counter::CosmeticTokenCycle,
                    delta: 1,
                    cause: Cause::Dev
                },
            ]
        );
    }
}
