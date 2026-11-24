//! Audit probes for the economy (SPEC §14). Temporary: only tests that prove a reported finding stay.

use std::collections::BTreeSet;

use indexmap::IndexMap;
use jackioh_shared::cosmetics::{
    CardCosmetic, Cosmetic, OwnedCosmetics, levels_below, offered, owns, registry, shipped_defs,
};
use jackioh_shared::economy::ledger::{AccountCounters, Counter, InventoryItem, LedgerGrant};
use jackioh_shared::economy::math::PityCounters;
use jackioh_shared::economy::open::{
    CosmeticPackInputs, PackInputs, PackRecords, open_card_pack, open_cosmetic_pack,
};
use jackioh_shared::economy::roll::{PackKind, PoolCard, pack_pool};

/// An account as the store holds it, moved by an opening's grants the way `write_grants` writes them.
#[derive(Clone)]
struct Account {
    owned: IndexMap<String, i64>,
    cosmetics: OwnedCosmetics,
    counters: AccountCounters,
    inventory: IndexMap<(String, String), i64>,
}

impl Account {
    fn new() -> Account {
        Account {
            owned: IndexMap::new(),
            cosmetics: OwnedCosmetics {
                cards: Vec::new(),
                account: Vec::new(),
            },
            counters: AccountCounters {
                pity: PityCounters {
                    epic: 0,
                    legendary: 0,
                    mythic: 0,
                },
                card_bar: 0,
                cosmetic_bar: 0,
                cosmetic_cycle: 0,
                chase_shard: 0,
            },
            inventory: IndexMap::new(),
        }
    }

    fn held(&self, item: InventoryItem, key: &str) -> i64 {
        self.inventory
            .get(&(format!("{item:?}"), key.to_string()))
            .copied()
            .unwrap_or(0)
    }

    fn records(&self, item: InventoryItem, key: &str) -> PackRecords {
        PackRecords {
            held: self.held(item, key),
            owned: self.owned.clone(),
            cosmetics: self.cosmetics.clone(),
            counters: self.counters,
        }
    }

    /// Writes `grants` in order; the problems a store or the rules would have with them.
    fn write(&mut self, grants: &[LedgerGrant], reg: &[Cosmetic], pool: &[PoolCard]) -> Vec<String> {
        let mut problems = Vec::new();
        for grant in grants {
            match grant {
                LedgerGrant::Inventory { item, key, delta, .. } => {
                    let count = self
                        .inventory
                        .entry((format!("{item:?}"), key.clone()))
                        .or_insert(0);
                    *count += delta;
                    if *count < 0 {
                        problems.push(format!("inventory {item:?} {key:?} below 0"));
                    }
                }
                LedgerGrant::Cards { cards } => {
                    for card in cards {
                        if !pool.iter().any(|entry| entry.id == card.card_id) {
                            problems.push(format!("card {} is not in the pool", card.card_id));
                        }
                        let before = self.owned.get(&card.card_id).copied().unwrap_or(0);
                        if before + card.delta != card.quantity || card.quantity > 1 {
                            problems.push(format!(
                                "card {}: {before} + {} != {} (or past MAX_COPIES)",
                                card.card_id, card.delta, card.quantity
                            ));
                        }
                        self.owned.insert(card.card_id.clone(), card.quantity);
                    }
                }
                LedgerGrant::Cosmetic { card_id, cosmetic_id } => {
                    let Some(cosmetic) = reg.iter().find(|entry| entry.id == *cosmetic_id) else {
                        problems.push(format!("unknown cosmetic {cosmetic_id}"));
                        continue;
                    };
                    if cosmetic.default || cosmetic.starter || cosmetic.earned_only() || cosmetic.is_retired() {
                        problems.push(format!("{cosmetic_id} is never dropped"));
                    }
                    if cosmetic.kind.is_per_card() != card_id.is_some() {
                        problems.push(format!("{cosmetic_id} granted on {card_id:?}"));
                    }
                    if owns(cosmetic, card_id.as_deref(), &self.cosmetics) {
                        problems.push(format!("{cosmetic_id} was already owned on {card_id:?}"));
                    }
                    match card_id {
                        Some(card) => {
                            if !offered(cosmetic, Some(card)) {
                                problems.push(format!("{cosmetic_id} is not offered on {card}"));
                            }
                            if self
                                .cosmetics
                                .cards
                                .iter()
                                .any(|row| row.card_id == *card && row.cosmetic_id == *cosmetic_id)
                            {
                                problems.push(format!("a second row of {cosmetic_id} on {card}"));
                            }
                            // R3015003: a level is granted with every level below it owned there.
                            for below in levels_below(reg, cosmetic) {
                                if !owns(below, Some(card), &self.cosmetics) {
                                    problems.push(format!(
                                        "{cosmetic_id} on {card} without {} below it",
                                        below.id
                                    ));
                                }
                            }
                            self.cosmetics.cards.push(CardCosmetic {
                                card_id: card.clone(),
                                cosmetic_id: cosmetic_id.clone(),
                            });
                        }
                        None => {
                            if self.cosmetics.account.contains(cosmetic_id) {
                                problems.push(format!("a second row of {cosmetic_id} on the account"));
                            }
                            self.cosmetics.account.push(cosmetic_id.clone());
                        }
                    }
                }
                LedgerGrant::Counter { counter, delta, .. } => {
                    let pity = |held: usize| usize::try_from(held as i64 + delta).unwrap_or(usize::MAX);
                    let counters = &mut self.counters;
                    match counter {
                        Counter::PityEpic => counters.pity.epic = pity(counters.pity.epic),
                        Counter::PityLegendary => counters.pity.legendary = pity(counters.pity.legendary),
                        Counter::PityMythic => counters.pity.mythic = pity(counters.pity.mythic),
                        Counter::CardTokenBar => counters.card_bar += delta,
                        Counter::CosmeticTokenBar => counters.cosmetic_bar += delta,
                        Counter::CosmeticTokenCycle => counters.cosmetic_cycle += delta,
                        Counter::ChaseShardBar => counters.chase_shard += delta,
                    }
                    if counters.card_bar < 0 || counters.cosmetic_bar < 0 || counters.chase_shard < 0 {
                        problems.push(format!("counter {counter:?} below 0"));
                    }
                }
                other => problems.push(format!("an opening wrote {other:?}")),
            }
        }
        problems
    }
}

fn pool() -> Vec<PoolCard> {
    jackioh_cards::register_all();
    pack_pool(shipped_defs(), PackKind::Global)
}

/// §14.6: what `POST /api/cosmetic-packs/open` answers as `unopened` is the count `write_grants` reads off
/// the opening's first inventory grant (`crates/server/src/economy/grants.rs`, `first_count`), "the cosmetic
/// packs still held" (`OpenedCosmeticPack`). §14.5: the cosmetic token bar's Cosmetic Pack step "adds an
/// unopened cosmetic pack to the inventory", "in the opening's own transaction". So when the opening itself
/// fills that step, the packs still held are one more than the first grant left.
#[test]
fn a_cosmetic_pack_whose_bar_pays_a_cosmetic_pack_answers_the_packs_held_after_it() {
    let pool = pool();
    let reg = registry().to_vec();
    let mut account = Account::new();
    account
        .inventory
        .insert((format!("{:?}", InventoryItem::CosmeticPack), String::new()), 1);
    // The cycle stands at its last step, the Cosmetic Pack (position 5 of 6), one point short of its size.
    account.counters.cosmetic_cycle = 5;
    account.counters.cosmetic_bar = 99;
    let inputs = CosmeticPackInputs {
        pool: pool.clone(),
        registry: reg.clone(),
        catalog: "audit".to_string(),
    };
    let records = account.records(InventoryItem::CosmeticPack, "");
    let opening = open_cosmetic_pack(&records, &inputs, "audit-unopened", 0).expect("a pack is held");

    // The server's `write_grants`: the count the first inventory grant left.
    let mut first_count = None;
    let mut running: IndexMap<(String, String), i64> = account.inventory.clone();
    for grant in &opening.grants {
        if let LedgerGrant::Inventory { item, key, delta, .. } = grant {
            let count = running.entry((format!("{item:?}"), key.clone())).or_insert(0);
            *count += delta;
            first_count.get_or_insert(*count);
        }
    }
    let problems = account.write(&opening.grants, &reg, &pool);
    assert!(problems.is_empty(), "{problems:?}");
    let held_after = account.held(InventoryItem::CosmeticPack, "");
    assert_eq!(held_after, 1, "the bar paid a cosmetic pack in this opening");
    assert_eq!(
        first_count,
        Some(held_after),
        "the opening answers `unopened` {first_count:?}, but the account holds {held_after}"
    );
}

/// A small deterministic generator for the probes' picks.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self, below: usize) -> usize {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % below.max(1)
    }
}

/// Probe: card packs on an account that owns every card (every slot a duplicate), opened one after another.
#[test]
fn probe_card_packs_grant_only_what_a_store_and_the_rules_take() {
    let pool = pool();
    let reg = registry().to_vec();
    let mut all = BTreeSet::new();
    for start in 0..3 {
        let mut account = Account::new();
        for card in &pool {
            account.owned.insert(card.id.clone(), 1);
        }
        let mut lcg = Lcg(start as u64 + 7);
        // Some retired full arts and a few rows of levels, as an older account holds them.
        if start > 0 {
            for cosmetic in reg.iter().filter(|c| c.is_retired() && c.card.is_some()) {
                if lcg.next(3) == 0 {
                    account.cosmetics.cards.push(CardCosmetic {
                        card_id: cosmetic.card.clone().unwrap_or_default(),
                        cosmetic_id: cosmetic.id.clone(),
                    });
                }
            }
        }
        account
            .inventory
            .insert((format!("{:?}", InventoryItem::GlobalPack), String::new()), 100_000);
        let inputs = PackInputs {
            pack: PackKind::Global,
            pool: pool.clone(),
            registry: reg.clone(),
            catalog: "audit".to_string(),
        };
        for n in 0..1500 {
            let records = account.records(InventoryItem::GlobalPack, "");
            let opening = open_card_pack(&records, &inputs, &format!("probe-{start}-{n}"), 0).expect("opens");
            for problem in account.write(&opening.grants, &reg, &pool) {
                all.insert(problem);
            }
            assert_eq!(account.counters.pity, opening.pity);
        }
    }
    assert!(all.is_empty(), "{} problems, e.g. {:?}", all.len(), all.iter().take(12).collect::<Vec<_>>());
}

/// Probe: cosmetic packs opened one after another, on a new account and on one that owns every card.
#[test]
fn probe_cosmetic_packs_grant_only_what_a_store_and_the_rules_take() {
    let pool = pool();
    let reg = registry().to_vec();
    let mut all = BTreeSet::new();
    for start in 0..2 {
        let mut account = Account::new();
        let mut lcg = Lcg(start as u64 + 11);
        if start == 1 {
            for card in &pool {
                account.owned.insert(card.id.clone(), 1);
            }
            for cosmetic in reg.iter().filter(|c| c.is_retired() && c.card.is_some()) {
                if lcg.next(2) == 0 {
                    account.cosmetics.cards.push(CardCosmetic {
                        card_id: cosmetic.card.clone().unwrap_or_default(),
                        cosmetic_id: cosmetic.id.clone(),
                    });
                }
            }
        }
        account
            .inventory
            .insert((format!("{:?}", InventoryItem::CosmeticPack), String::new()), 100_000);
        let inputs = CosmeticPackInputs {
            pool: pool.clone(),
            registry: reg.clone(),
            catalog: "audit".to_string(),
        };
        for n in 0..12 {
            let records = account.records(InventoryItem::CosmeticPack, "");
            let opening = open_cosmetic_pack(&records, &inputs, &format!("probe-{start}-{n}"), 0).expect("opens");
            for problem in account.write(&opening.grants, &reg, &pool) {
                all.insert(problem);
            }
        }
    }
    assert!(all.is_empty(), "{} problems, e.g. {:?}", all.len(), all.iter().take(12).collect::<Vec<_>>());
}

/// R1304001: "A game counts toward no quest that falls due after it ended": the hook tops the account up
/// at the moment the game ended, "so a quest that fell due by then is held, and the whole game counts toward
/// it, as toward every other quest held when the game ended". A quest the book already holds that arrived
/// after `summary.at` (a top-up or a replace that committed between the game's end and its payout: a
/// `GET /api/quests`, another game's payout, R2210000's replacement "dated when the replace ran") was not held
/// when the game ended.
#[test]
fn r1304001_a_game_moves_no_quest_that_arrived_after_it_ended() {
    use jackioh_engine::PlayerId;
    use jackioh_shared::economy::fleet::{FleetPlay, FleetTrack};
    use jackioh_shared::economy::ledger::{HeldQuest, QuestBook};
    use jackioh_shared::economy::quests::{GameTally, quest_interval_ms};
    use jackioh_shared::economy::rewards::{GameMode, GameRecords, GameSummary, pay_game};

    jackioh_cards::register_all();
    const ENDED_AT: i64 = 1_800_000_000_000;
    let records = GameRecords {
        paid_today: 0,
        tracks: Vec::new(),
        quests: QuestBook {
            // "Win 3 games", which arrived one second after the game ended.
            held: vec![HeldQuest {
                quest_id: "win-3".to_string(),
                progress: 0,
                arrived_at: ENDED_AT + 1_000,
            }],
            // The top-up that brought it moved the schedule on: nothing is due at the game's end.
            next_due_at: Some(ENDED_AT + 1_000 + quest_interval_ms()),
        },
        fleet: FleetTrack::default(),
    };
    let summary = GameSummary {
        mode: GameMode::Ladder,
        profile_id: "p".to_string(),
        player: PlayerId::P1,
        won: Some(true),
        actions: 10,
        match_id: "match-1".to_string(),
        at: ENDED_AT,
        bot: false,
        tally: GameTally::default(),
        proficiency: Vec::new(),
        fleet: FleetPlay::default(),
    };
    let paid = pay_game(&records, &summary, "seed");
    let moved: Vec<&LedgerGrant> = paid
        .grants
        .iter()
        .filter(|grant| matches!(grant, LedgerGrant::Quest { quest_id, .. } if quest_id == "win-3"))
        .collect();
    assert!(
        moved.is_empty(),
        "a game that ended at {ENDED_AT} moved a quest that arrived at {}: {moved:?}",
        ENDED_AT + 1_000
    );
}
