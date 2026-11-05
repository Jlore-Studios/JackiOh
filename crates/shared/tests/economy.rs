//! The economy's decisions over plain records (`src/economy/open.rs`, README.md's host contract): the
//! opening a store writes and a host answers, held to the rolls it is made of, and the JSON boundary a
//! browser or a host that is not Rust calls (`src/economy/boundary.rs`).

use indexmap::IndexMap;
use jackioh_engine::config::MAX_COPIES;
use jackioh_engine::wire::PrintedRarity;
use jackioh_shared::config::{CARD_TOKEN_BAR_SIZE, CHASE_SHARD_PACKS, PACK_SLOTS, PITY_MYTHIC_PACKS};
use jackioh_shared::cosmetics::{OwnedCosmetics, registry};
use jackioh_shared::economy::boundary::{EconomyAnswer, EconomyCall, apply};
use jackioh_shared::economy::cosmetic_pack::{CosmeticHoldings, roll_cosmetic_pack};
use jackioh_shared::economy::ledger::{AccountCounters, Cause, Counter, InventoryItem, LedgerGrant};
use jackioh_shared::economy::math::PityCounters;
use jackioh_shared::economy::open::{
    CosmeticPackInputs, EconomyRefusal, PackInputs, PackRecords, RefusalReason, open_card_pack,
    open_cosmetic_pack,
};
use jackioh_shared::economy::roll::{Holdings, PackKind, PoolCard, SlotGrant, roll_opening};
use serde_json::{Value, json};

const NOW: i64 = 1_700_000_000_000;
const SEED: &str = "economy-test-seed";

/// A pool of `count` cards, one rarity after another from Common.
fn pool_of(count: usize) -> Vec<PoolCard> {
    (0..count)
        .map(|at| PoolCard {
            id: format!("card-{at}"),
            rarity: PrintedRarity::ALL[at % PrintedRarity::ALL.len()],
        })
        .collect()
}

fn inputs(pool: Vec<PoolCard>) -> PackInputs {
    PackInputs {
        pack: PackKind::Global,
        pool,
        registry: registry().to_vec(),
        catalog: "v-test".to_string(),
    }
}

fn records(held: i64) -> PackRecords {
    PackRecords {
        held,
        ..PackRecords::default()
    }
}

fn refusal(result: Result<impl std::fmt::Debug, EconomyRefusal>) -> EconomyRefusal {
    result.expect_err("a refusal")
}

/// R1301003, R1339002: an opening is `roll_opening` over the holdings in the records and the seed, and the
/// log it carries replays it.
#[test]
fn r1301003_r1339002_an_opening_is_the_roll_over_the_records_and_its_log_replays_it() {
    let pool = pool_of(40);
    let mut owned = IndexMap::new();
    owned.insert("card-3".to_string(), i64::from(MAX_COPIES));
    let records = PackRecords {
        held: 2,
        owned: owned.clone(),
        ..PackRecords::default()
    };
    let inputs = inputs(pool.clone());
    let opening = open_card_pack(&records, &inputs, SEED, NOW).expect("an opening");
    let holdings = Holdings {
        owned,
        ..Holdings::default()
    };
    let rolled = roll_opening(&pool, &holdings, &inputs.registry, SEED).expect("a roll");
    assert_eq!(opening.slots, rolled.slots);
    assert_eq!(opening.pity, rolled.pity);
    assert_eq!(opening.log.draws, rolled.draws);
    assert_eq!((opening.at, opening.pack.as_str()), (NOW, "global"));
    // The log replays the opening however the registry has moved: its droppable entries, whole.
    let replay = roll_opening(&pool, &holdings, &opening.log.cosmetics, &opening.log.seed).expect("a replay");
    assert_eq!(replay.slots, opening.log.slots);
    // The same records, inputs and seed grant the same, with another clock only `at` moves.
    let again = open_card_pack(&records, &inputs, SEED, NOW + 5).expect("an opening");
    assert_eq!(
        (again.grants.clone(), again.slots.clone()),
        (opening.grants.clone(), opening.slots.clone())
    );
    assert_eq!(again.at, NOW + 5);
}

/// R1305000: the writes come in the order a store makes them: the pack spent, the cards, the cosmetics, the
/// pity counters, then both bars; the cards carry the quantity held after.
#[test]
fn r1305000_the_grants_come_in_the_order_the_store_writes_them() {
    let opening = open_card_pack(&records(1), &inputs(pool_of(60)), SEED, NOW).expect("an opening");
    let rank = |grant: &LedgerGrant| match grant {
        LedgerGrant::Inventory {
            cause: Cause::PackOpen,
            ..
        } => 0,
        LedgerGrant::Cards { .. } => 1,
        LedgerGrant::Cosmetic { .. } => 2,
        LedgerGrant::Counter {
            counter,
            cause: Cause::PackOpen,
            ..
        } if counter.is_pity() => 3,
        _ => 4,
    };
    let ranks: Vec<u8> = opening.grants.iter().map(rank).collect();
    assert!(ranks.windows(2).all(|pair| pair[0] <= pair[1]), "{ranks:?}");
    assert_eq!(
        opening.grants[0],
        LedgerGrant::Inventory {
            item: InventoryItem::GlobalPack,
            key: String::new(),
            delta: -1,
            cause: Cause::PackOpen,
        }
    );
    // A fresh account's five slots are five new cards, one copy each at most, none a zero delta.
    let new_cards = opening
        .slots
        .iter()
        .filter(|slot| slot.grant == SlotGrant::Card)
        .count();
    let Some(LedgerGrant::Cards { cards }) = opening.grants.get(1) else {
        panic!("cards second: {:?}", opening.grants);
    };
    assert_eq!(cards.iter().map(|card| card.delta).sum::<i64>(), new_cards as i64);
    assert!(cards.iter().all(|card| card.quantity == card.delta));
    assert!(
        opening
            .grants
            .iter()
            .all(|grant| !matches!(grant, LedgerGrant::Counter { delta: 0, .. }))
    );
    // Every slot adds its points to both bars (R1999000).
    assert_eq!(opening.bars.slot_points.len(), PACK_SLOTS);
}

/// R1298000, R1339001: pity moves by the counter's change, a counter that does not move writes nothing, and
/// the one that fires resets.
#[test]
fn r1298000_r1339001_a_pity_counter_writes_its_change_and_a_still_one_writes_nothing() {
    let mut records = records(1);
    records.counters.pity = PityCounters {
        epic: 2,
        legendary: 3,
        mythic: PITY_MYTHIC_PACKS - 1,
    };
    let opening = open_card_pack(&records, &inputs(pool_of(60)), SEED, NOW).expect("an opening");
    // The threshold pack holds a Mythic (forced if not rolled): the Mythic counter resets.
    assert!(
        opening
            .slots
            .iter()
            .any(|slot| slot.rarity == PrintedRarity::Mythic)
    );
    assert_eq!(opening.pity.mythic, 0);
    let pity: Vec<(Counter, i64)> = opening
        .grants
        .iter()
        .filter_map(|grant| match grant {
            LedgerGrant::Counter { counter, delta, .. } if counter.is_pity() => Some((*counter, *delta)),
            _ => None,
        })
        .collect();
    assert!(
        pity.contains(&(Counter::PityMythic, -(PITY_MYTHIC_PACKS as i64 - 1))),
        "{pity:?}"
    );
}

/// R1301003: no pack held is a refusal that writes nothing; a pool of no card is another.
#[test]
fn r1301003_an_empty_hand_and_an_empty_pool_are_refusals() {
    let none = refusal(open_card_pack(&records(0), &inputs(pool_of(10)), SEED, NOW));
    assert_eq!(none.reason, RefusalReason::NoPackHeld);
    assert_eq!(none.message, "no unopened pack of that kind");
    let empty = refusal(open_card_pack(&records(1), &inputs(Vec::new()), SEED, NOW));
    assert_eq!(empty.reason, RefusalReason::EmptyPool);
    assert_eq!(empty.message, "the global pack's pool holds no card");
    let cosmetic = CosmeticPackInputs {
        pool: pool_of(10),
        registry: registry().to_vec(),
        catalog: "v-test".to_string(),
    };
    let none = refusal(open_cosmetic_pack(&records(0), &cosmetic, SEED, NOW));
    assert_eq!(none.message, "no unopened cosmetic pack");
}

/// R1306000, R1306002, R1306003: a cosmetic pack is `roll_cosmetic_pack` over the records; its Chase Shard bar
/// gains one and a full bar holds; both token bars settle.
#[test]
fn r1306002_a_cosmetic_pack_gains_one_shard_and_a_full_bar_holds() {
    let inputs = CosmeticPackInputs {
        pool: pool_of(40),
        registry: registry().to_vec(),
        catalog: "v-test".to_string(),
    };
    let opening = open_cosmetic_pack(&records(1), &inputs, SEED, NOW).expect("an opening");
    let rolled = roll_cosmetic_pack(
        &inputs.pool,
        &CosmeticHoldings::of(IndexMap::new(), &OwnedCosmetics::default()),
        &inputs.registry,
        SEED,
    );
    assert_eq!(opening.slots, rolled.slots);
    assert_eq!(opening.log.draws, rolled.draws);
    assert_eq!((opening.shard_before, opening.shard_bar.points), (0, 1));
    assert_eq!(opening.shard_bar.size, CHASE_SHARD_PACKS);
    assert!(opening.grants.contains(&LedgerGrant::Counter {
        counter: Counter::ChaseShardBar,
        delta: 1,
        cause: Cause::PackOpen,
    }));
    assert_eq!(
        opening.grants[0],
        LedgerGrant::Inventory {
            item: InventoryItem::CosmeticPack,
            key: String::new(),
            delta: -1,
            cause: Cause::PackOpen,
        }
    );
    // A full bar holds: nothing is written to it.
    let mut full = records(1);
    full.counters.chase_shard = CHASE_SHARD_PACKS;
    let opening = open_cosmetic_pack(&full, &inputs, SEED, NOW).expect("an opening");
    assert_eq!(opening.shard_bar.points, CHASE_SHARD_PACKS);
    assert!(!opening.grants.iter().any(|grant| matches!(
        grant,
        LedgerGrant::Counter {
            counter: Counter::ChaseShardBar,
            ..
        }
    )));
}

/// R1305000, R1305001: a bar the opening fills pays its token and keeps the overflow, in the grants.
#[test]
fn r1305000_a_full_card_bar_pays_a_token_inside_the_opening() {
    let mut records = records(1);
    records.counters.card_bar = CARD_TOKEN_BAR_SIZE - 1;
    let opening = open_card_pack(&records, &inputs(pool_of(60)), SEED, NOW).expect("an opening");
    assert_eq!(opening.bars.card.pays, 1);
    assert!(opening.grants.contains(&LedgerGrant::Inventory {
        item: InventoryItem::CardToken,
        key: String::new(),
        delta: 1,
        cause: Cause::TokenBar,
    }));
    assert_eq!(
        opening.bars.card.after,
        opening.bars.slot_points.iter().sum::<i64>() - 1
    );
}

// The boundary: JSON in, JSON out, as a browser and a host that is not Rust make a call.

/// A call as a host sends it and the answer as it reads it.
fn call(request: Value) -> Value {
    let request: EconomyCall = serde_json::from_value(request).expect("an EconomyCall");
    serde_json::to_value(apply(&request)).expect("an EconomyAnswer")
}

fn zero_records(held: i64) -> Value {
    json!({
        "held": held, "owned": {}, "cosmetics": { "cards": [], "account": [] },
        "counters": {
            "pity": { "epic": 0, "legendary": 0, "mythic": 0 },
            "cardBar": 0, "cosmeticBar": 0, "cosmeticCycle": 0, "chaseShard": 0,
        },
    })
}

/// `openCardPack` is `open_card_pack` over JSON: a pool, registry and catalog the call leaves out are the
/// compiled-in ones, and a refusal is an answer.
#[test]
fn the_boundary_opens_a_pack_over_the_compiled_in_pool_and_registry() {
    let answer = call(json!({
        "call": "openCardPack", "records": zero_records(1), "pack": "global", "seed": SEED, "now": NOW,
    }));
    let opening = &answer["opened"];
    assert_eq!(opening["slots"].as_array().map(Vec::len), Some(PACK_SLOTS));
    assert_eq!(opening["log"]["catalog"], jackioh_cards::catalog_version());
    assert_eq!(opening["at"], NOW);
    // Natively: the same inputs, built as the call builds them.
    let native = open_card_pack(
        &records(1),
        &PackInputs {
            pack: PackKind::Global,
            pool: jackioh_shared::economy::roll::pack_pool(
                jackioh_shared::cosmetics::shipped_defs(),
                PackKind::Global,
            ),
            registry: registry().to_vec(),
            catalog: jackioh_cards::catalog_version().to_string(),
        },
        SEED,
        NOW,
    )
    .expect("an opening");
    assert_eq!(*opening, serde_json::to_value(&native).expect("JSON"));
    let refused = call(json!({
        "call": "openCardPack", "records": zero_records(0), "pack": "global", "seed": SEED, "now": NOW,
    }));
    assert_eq!(refused["refused"]["reason"], "no_pack_held");
}

#[test]
fn the_boundary_opens_a_cosmetic_pack_fills_a_bar_and_reads_a_reward() {
    let opened = call(json!({
        "call": "openCosmeticPack", "records": zero_records(1), "seed": SEED, "now": NOW,
    }));
    assert_eq!(
        opened["cosmeticOpened"]["slots"].as_array().map(Vec::len),
        Some(5)
    );
    assert_eq!(opened["cosmeticOpened"]["shardBar"]["points"], 1);
    let filled = call(json!({
        "call": "fillBar", "bar": "card", "counters": zero_records(0)["counters"],
    }));
    assert_eq!(filled["filled"]["pays"], 1);
    // An empty bar has no points to replace, so the fill writes the token alone.
    assert_eq!(filled["filled"]["grants"][0]["kind"], "inventory");
    assert_eq!(filled["filled"]["grants"][0]["item"], "card_token");
    assert_eq!(filled["filled"]["grants"][0]["cause"], "dev");
    let reward = call(json!({ "call": "levelReward", "level": 20 }));
    assert_eq!(
        reward["reward"],
        json!({ "jlore": 100, "setPacks": 1, "cardTokens": 1 })
    );
}

#[test]
fn the_boundary_refuses_a_call_that_does_not_parse() {
    for bad in [
        json!({}),
        json!({ "call": "nope" }),
        json!({ "call": "openCardPack", "seed": "s" }),
    ] {
        assert!(
            serde_json::from_value::<EconomyCall>(bad.clone()).is_err(),
            "{bad}"
        );
    }
    // A pack nobody ships is no pack.
    let bad = json!({
        "call": "openCardPack", "records": zero_records(1), "pack": "no-such-set", "seed": "s", "now": 1,
    });
    let error = serde_json::from_value::<EconomyCall>(bad).expect_err("no such pack");
    assert!(error.to_string().contains("no card pack is named"), "{error}");
    let answer: EconomyAnswer = apply(&EconomyCall::LevelReward { level: 1 });
    assert!(matches!(answer, EconomyAnswer::Reward(_)));
}
