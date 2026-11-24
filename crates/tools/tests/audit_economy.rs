//! Audit of the economy (SPEC §14): each test asserts a ruled behaviour and fails on this tree.

use indexmap::IndexMap;
use jackioh_engine::PlayerId;
use jackioh_shared::cosmetics::{OwnedCosmetics, registry, shipped_defs};
use jackioh_shared::economy::fleet::{FleetPlay, FleetTrack};
use jackioh_shared::economy::ledger::{
    AccountCounters, HeldQuest, InventoryItem, LedgerGrant, QuestBook,
};
use jackioh_shared::economy::math::PityCounters;
use jackioh_shared::economy::open::{CosmeticPackInputs, PackRecords, open_cosmetic_pack};
use jackioh_shared::economy::quests::{GameTally, quest_interval_ms};
use jackioh_shared::economy::rewards::{GameMode, GameRecords, GameSummary, pay_game};
use jackioh_shared::economy::roll::{PackKind, pack_pool};

/// §14.5: the cosmetic token bar's Cosmetic Pack step "adds an unopened cosmetic pack to the inventory", "in
/// the opening's own transaction". `POST /api/cosmetic-packs/open` answers `unopened`, "the cosmetic packs still
/// held" (`OpenedCosmeticPack`, crates/server/src/economy/cosmetic_packs.rs), from the count `write_grants`
/// reads off the opening's FIRST inventory grant (crates/server/src/economy/grants.rs, `first_count`). When the
/// opening itself fills the cycle's Cosmetic Pack step, a later grant of the same opening adds a pack, so the
/// answer is one short of what the account holds, and the Shop (CosmeticPacks.tsx folds `next.unopened` in and
/// reads nothing again, R3211000) shows "No unopened packs." with no Open button.
#[test]
fn a_cosmetic_pack_whose_bar_pays_a_cosmetic_pack_answers_the_packs_held_after_it() {
    jackioh_cards::register_all();
    let records = PackRecords {
        held: 1,
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
            // The cycle stands at its last step, the Cosmetic Pack (position 5 of 6), one point short of
            // `COSMETIC_TOKEN_BAR_SIZE_PACK`; a pack's five slots add at least seven points.
            cosmetic_bar: 99,
            cosmetic_cycle: 5,
            chase_shard: 0,
        },
    };
    let inputs = CosmeticPackInputs {
        pool: pack_pool(shipped_defs(), PackKind::Global),
        registry: registry().to_vec(),
        catalog: "audit".to_string(),
    };
    let opening = open_cosmetic_pack(&records, &inputs, "audit-unopened", 0).expect("a pack is held");

    // The store's count of cosmetic packs after each inventory grant, as `write_grants` writes them in order.
    let mut held = records.held;
    let mut first_count = None;
    for grant in &opening.grants {
        if let LedgerGrant::Inventory { item, delta, .. } = grant {
            if *item == InventoryItem::CosmeticPack {
                held += delta;
            }
            // `first_count.get_or_insert(count)`: the opening spends its pack first.
            first_count.get_or_insert(held);
        }
    }
    assert_eq!(held, 1, "the bar paid a cosmetic pack in this opening");
    assert_eq!(
        first_count,
        Some(held),
        "the opening answers `unopened` {first_count:?}, but the account holds {held}"
    );
}

/// R1304001: "A game counts toward no quest that falls due after it ended": the hook tops the account up at
/// the moment the game ended, "so a quest that fell due by then is held, and the whole game counts toward it,
/// as toward every other quest held when the game ended". A quest the book already holds that arrived after
/// `summary.at` (a top-up or a replace that committed between the game's end stamp and its payout: a
/// `GET /api/quests`, another game's payout, or R2210000's replacement, "dated when the replace ran") was not
/// held when the game ended.
#[test]
fn r1304001_a_game_moves_no_quest_that_arrived_after_it_ended() {
    jackioh_cards::register_all();
    const ENDED_AT: i64 = 1_800_000_000_000;
    const ARRIVED_AT: i64 = ENDED_AT + 1_000;
    let records = GameRecords {
        paid_today: 0,
        tracks: Vec::new(),
        quests: QuestBook {
            // "Win 3 games", which arrived one second after the game ended.
            held: vec![HeldQuest {
                quest_id: "win-3".to_string(),
                progress: 0,
                arrived_at: ARRIVED_AT,
            }],
            // The top-up that brought it moved the schedule on: nothing is due at the game's end.
            next_due_at: Some(ARRIVED_AT + quest_interval_ms()),
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
        "a game that ended at {ENDED_AT} moved a quest that arrived at {ARRIVED_AT}: {moved:?}"
    );
}
