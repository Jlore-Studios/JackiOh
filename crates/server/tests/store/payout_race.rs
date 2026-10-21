//! SPEC §14.7: one account's overlapping payouts, against both stores.
//!
//! §14.7 says "a level pays once, as ledger rows, when its track reaches it", and [[R1301003]]
//! serialises one account's economy writes under its lock. [`pay_game_rewards`] takes that lock
//! before reading the account's tracks and quests; these prove two payouts that overlap still pay
//! each level exactly once:
//!
//! - two payouts that each cross the same level pay it once, leaving the track's level at its XP;
//! - two payouts that both cross `SET_TRACK_UNLOCK_LEVEL` unlock one older set each,
//!   nearest-older first, and still pay every reached level exactly once (the second payout finds
//!   the first payout's unlocked track active and carries its XP there too).
//!
//! ONE suite, TWO stores, like `contract.rs`: every case runs against `Db::Fake` always, and
//! against `Db::Pg` too when `DATABASE_URL` is set (`tests/db/run.sh`, one case at a time over one
//! database).
//!
//! Every pair is started before either is awaited (`tokio::join!`); the assertions are on the end
//! state under overlap: every reached level paid once, every track's level its XP's. Note that two
//! game payouts for one account also serialize on the account's wallet row (`jlore_grant`'s
//! `for update`, every game paying nonzero Jlore), so on both stores they run one after the other;
//! the account lock these tests exercise is the intended mechanism, and it is what a future
//! zero-Jlore payout — or a lock-free reader like the dev tools — would rely on. The fake
//! serialises transactions on its one lock (its `lock_account` is only the seams).

use jackioh_engine::PlayerId;
use jackioh_engine::wire::{SHIPPED_SETS, newest_shipped_set};
use jackioh_server::config::{
    SET_TRACK_FIRST_LEVEL, SET_TRACK_UNLOCK_LEVEL, SET_TRACK_XP_PER_LEVEL, XP_PER_ACTION,
};
use jackioh_server::db::store::{Db, Profile, ProfileCreateInput};
use jackioh_server::economy::math::total_xp_for_level;
use jackioh_server::economy::quests::GameTally;
use jackioh_server::economy::rewards::{GameMode, GameSummary, game_xp, pay_game_rewards, track_level};
use jackioh_server::economy::store::{EconomyReason, InventoryItem, JloreReason, TrackGrant};
use jackioh_server::economy::track_rewards::reward_for_level;

use super::contract::{PG_SERIAL, StoreHarness};

/// The cases, once against each store: `memory::<case>` always (`cargo test`'s half),
/// `postgres::<case>` when `DATABASE_URL` is set (`tests/db/run.sh`'s half). Each starts with the
/// harness's `reset`.
macro_rules! both_stores {
    ($($case:ident),+ $(,)?) => {
        mod memory {
            $(
                #[tokio::test]
                async fn $case() {
                    let harness = super::StoreHarness::memory();
                    harness.reset().await;
                    super::$case(&harness).await;
                    harness.close().await;
                }
            )+
        }

        mod postgres {
            $(
                #[tokio::test]
                async fn $case() {
                    let _one_at_a_time = super::PG_SERIAL.lock().await;
                    let Some(harness) = super::StoreHarness::postgres().await else {
                        return;
                    };
                    harness.reset().await;
                    super::$case(&harness).await;
                    harness.close().await;
                }
            )+
        }
    };
}

/// A pending profile is enough: the reward hook pays any account, whatever its status.
async fn profile(harness: &StoreHarness) -> Profile {
    let email = format!("payout-race-{}@example.test", uuid::Uuid::new_v4());
    let user_id = harness.new_user_id(&email).await;
    let mut tx = harness
        .db
        .begin(None)
        .await
        .unwrap_or_else(|error| panic!("{}: begin a transaction: {error}", harness.name));
    let created = tx
        .profiles_create(&ProfileCreateInput {
            user_id,
            email,
            rating: 1000.0,
            at: harness.now(),
        })
        .await
        .unwrap_or_else(|error| panic!("{}: create a profile: {error}", harness.name));
    tx.commit()
        .await
        .unwrap_or_else(|error| panic!("{}: commit: {error}", harness.name));
    created
}

/// No quest may arrive or complete while the tracks are counted: a win-goal quest's XP would move
/// them. Quest arrivals stay suppressed to `at`, far past the payouts.
async fn suppress_quests(harness: &StoreHarness, profile: &Profile, at: i64) {
    let mut tx = harness
        .db
        .begin(Some(&profile.id))
        .await
        .unwrap_or_else(|error| panic!("{}: begin a transaction: {error}", harness.name));
    tx.economy_quests_set_next_due(&profile.id, Some(at + 86_400_000))
        .await
        .unwrap_or_else(|error| panic!("{}: suppress quests: {error}", harness.name));
    tx.commit()
        .await
        .unwrap_or_else(|error| panic!("{}: commit: {error}", harness.name));
}

/// The actions of one finished ladder loss whose [`game_xp`] is exactly the XP of one level step. A
/// win cannot be: its bonus alone (`XP_WIN_BONUS`) passes a step.
const STEP_ACTIONS: i64 = (SET_TRACK_XP_PER_LEVEL / XP_PER_ACTION) as i64;

/// One finished ladder game for the account: a loss of [`STEP_ACTIONS`] actions, so [`game_xp`] is
/// exactly the XP of one level step.
fn summary(profile_id: &str, match_id: &str, at: i64) -> GameSummary {
    GameSummary {
        mode: GameMode::Ladder,
        profile_id: profile_id.to_string(),
        player: PlayerId::P1,
        won: Some(false),
        actions: STEP_ACTIONS,
        match_id: match_id.to_string(),
        at,
        bot: false,
        tally: GameTally::default(),
    }
}

/// [`pay_game_rewards`] in its own transaction, as the practice claim and the dev grant each run
/// it; the ladder hook runs it in the transaction that records the result.
async fn payout(db: &Db, name: &str, summary: GameSummary) {
    let mut tx = db
        .begin(Some(&summary.profile_id))
        .await
        .unwrap_or_else(|error| panic!("{name}: begin a transaction: {error}"));
    pay_game_rewards(&mut tx, &summary)
        .await
        .unwrap_or_else(|error| panic!("{name}: pay {}: {error}", summary.match_id));
    tx.commit()
        .await
        .unwrap_or_else(|error| panic!("{name}: commit: {error}"));
}

fn is_race(reference: &Option<String>) -> bool {
    matches!(reference.as_deref(), Some("race-a") | Some("race-b"))
}

/// Two overlapping payouts that each cross the same level pay it once: the track ends at its XP's
/// level, and the level's reward is in the ledger exactly once.
async fn overlapping_payouts_pay_each_level_once(harness: &StoreHarness) {
    let step = game_xp(GameMode::Ladder, STEP_ACTIONS, false);
    assert_eq!(
        step,
        total_xp_for_level(2) - total_xp_for_level(1),
        "{}: the payouts each carry one level step",
        harness.name,
    );
    let profile = profile(harness).await;
    let now = harness.now();
    suppress_quests(harness, &profile, now).await;
    let newest = newest_shipped_set();
    // The track stands one reward short of level 2: both payouts cross it.
    let mut tx = harness
        .db
        .begin(Some(&profile.id))
        .await
        .unwrap_or_else(|error| panic!("{}: begin a transaction: {error}", harness.name));
    tx.economy_tracks_grant(&TrackGrant {
        profile_id: profile.id.clone(),
        set_id: newest.as_str().to_string(),
        xp: 0,
        levels: 0,
        rewards: 1,
        reason: EconomyReason::Game,
        reference: None,
        dev: false,
        at: now,
    })
    .await
    .unwrap_or_else(|error| panic!("{}: seed the track: {error}", harness.name));
    tx.commit()
        .await
        .unwrap_or_else(|error| panic!("{}: commit: {error}", harness.name));

    tokio::join!(
        payout(&harness.db, harness.name, summary(&profile.id, "race-a", now)),
        payout(&harness.db, harness.name, summary(&profile.id, "race-b", now + 1)),
    );

    let mut tx = harness
        .db
        .begin(Some(&profile.id))
        .await
        .unwrap_or_else(|error| panic!("{}: begin a transaction: {error}", harness.name));
    let tracks = tx
        .economy_tracks_get(&profile.id)
        .await
        .unwrap_or_else(|error| panic!("{}: read the tracks: {error}", harness.name));
    let track = tracks
        .iter()
        .find(|track| track.set_id == newest.as_str())
        .unwrap_or_else(|| panic!("{}: the newest track", harness.name));
    assert_eq!(
        (track.xp, track.level, track.rewards_claimed),
        (2 * step, 2, 2),
        "{}: both payouts' XP lands, the level reached once",
        harness.name,
    );
    assert_eq!(
        track.level,
        track_level(track.xp),
        "{}: the track's level is its XP's",
        harness.name,
    );
    let ledger = tx
        .economy_ledger_get(&profile.id)
        .await
        .unwrap_or_else(|error| panic!("{}: read the ledger: {error}", harness.name));
    tx.commit()
        .await
        .unwrap_or_else(|error| panic!("{}: commit: {error}", harness.name));

    let reward = reward_for_level(2);
    let track_packs: Vec<_> = ledger
        .inventory
        .iter()
        .filter(|grant| grant.reason == EconomyReason::Track)
        .collect();
    assert_eq!(
        track_packs.len(),
        usize::try_from(reward.set_packs).unwrap_or(usize::MAX),
        "{}: level 2's packs pay once",
        harness.name,
    );
    for pack in &track_packs {
        assert_eq!(pack.item, InventoryItem::SetPack, "{}", harness.name);
        assert_eq!(pack.key, newest.as_str(), "{}", harness.name);
        assert_eq!(pack.delta, 1, "{}", harness.name);
        assert!(!pack.dev, "{}", harness.name);
        assert!(is_race(&pack.reference), "{}", harness.name);
    }
    let track_jlore: Vec<_> = ledger
        .jlore
        .iter()
        .filter(|grant| grant.reason == JloreReason::Track)
        .collect();
    assert_eq!(
        track_jlore.len(),
        usize::from(reward.jlore != 0),
        "{}: level 2's Jlore pays once",
        harness.name,
    );
    let track_tokens: Vec<_> = ledger
        .inventory
        .iter()
        .filter(|grant| grant.reason == EconomyReason::Track && grant.item == InventoryItem::CardToken)
        .collect();
    assert_eq!(
        track_tokens.len(),
        usize::try_from(reward.card_tokens).unwrap_or(usize::MAX),
        "{}: level 2's tokens pay once",
        harness.name,
    );
}

/// Two overlapping payouts that both cross `SET_TRACK_UNLOCK_LEVEL` unlock one older set each,
/// nearest-older first (one per payout, §14.7): the first payout unlocks the second-newest set, the
/// second payout runs after the first commits, finds that track active and carrying its XP too, and
/// unlocks the next one. Every reached level pays once, and every track's level is its XP's.
/// Without the lock both payouts read the same stale tracks and both unlock and pay the same set.
async fn overlapping_payouts_unlock_one_set_once(harness: &StoreHarness) {
    let step = game_xp(GameMode::Ladder, STEP_ACTIONS, false);
    let seed_xp = total_xp_for_level(SET_TRACK_UNLOCK_LEVEL) - step;
    let seed_level = track_level(seed_xp);
    assert_eq!(
        seed_level,
        i64::from(SET_TRACK_UNLOCK_LEVEL) - 1,
        "{}: the seed stands one payout short of the unlock",
        harness.name,
    );
    let profile = profile(harness).await;
    let now = harness.now();
    suppress_quests(harness, &profile, now).await;
    let newest = newest_shipped_set();
    let older = SHIPPED_SETS[SHIPPED_SETS.len() - 2];
    let next_older = SHIPPED_SETS[SHIPPED_SETS.len() - 3];
    let mut tx = harness
        .db
        .begin(Some(&profile.id))
        .await
        .unwrap_or_else(|error| panic!("{}: begin a transaction: {error}", harness.name));
    tx.economy_tracks_grant(&TrackGrant {
        profile_id: profile.id.clone(),
        set_id: newest.as_str().to_string(),
        xp: seed_xp,
        levels: seed_level - SET_TRACK_FIRST_LEVEL,
        rewards: seed_level,
        reason: EconomyReason::Game,
        reference: None,
        dev: false,
        at: now,
    })
    .await
    .unwrap_or_else(|error| panic!("{}: seed the track: {error}", harness.name));
    tx.commit()
        .await
        .unwrap_or_else(|error| panic!("{}: commit: {error}", harness.name));

    tokio::join!(
        payout(&harness.db, harness.name, summary(&profile.id, "race-a", now)),
        payout(&harness.db, harness.name, summary(&profile.id, "race-b", now + 1)),
    );

    let mut tx = harness
        .db
        .begin(Some(&profile.id))
        .await
        .unwrap_or_else(|error| panic!("{}: begin a transaction: {error}", harness.name));
    let tracks = tx
        .economy_tracks_get(&profile.id)
        .await
        .unwrap_or_else(|error| panic!("{}: read the tracks: {error}", harness.name));
    let track = tracks
        .iter()
        .find(|track| track.set_id == newest.as_str())
        .unwrap_or_else(|| panic!("{}: the newest track", harness.name));
    assert_eq!(
        (track.xp, track.level, track.rewards_claimed),
        (
            seed_xp + 2 * step,
            i64::from(SET_TRACK_UNLOCK_LEVEL),
            i64::from(SET_TRACK_UNLOCK_LEVEL)
        ),
        "{}: both payouts' XP lands, the unlock level reached once",
        harness.name,
    );
    assert_eq!(
        track.level,
        track_level(track.xp),
        "{}: the newest track's level is its XP's",
        harness.name,
    );
    let unlocked = tracks
        .iter()
        .find(|track| track.set_id == older.as_str())
        .unwrap_or_else(|| panic!("{}: the unlocked track", harness.name));
    assert_eq!(
        (unlocked.xp, unlocked.level, unlocked.rewards_claimed),
        (2 * step, 2, 2),
        "{}: the older track carries both payouts' XP, its levels paid once",
        harness.name,
    );
    assert_eq!(
        unlocked.level,
        track_level(unlocked.xp),
        "{}: the unlocked track's level is its XP's",
        harness.name,
    );
    // The second payout's unlock: the next-older set, carrying that payout's XP alone.
    let next_unlocked = tracks
        .iter()
        .find(|track| track.set_id == next_older.as_str())
        .unwrap_or_else(|| panic!("{}: the next unlocked track", harness.name));
    assert_eq!(
        (
            next_unlocked.xp,
            next_unlocked.level,
            next_unlocked.rewards_claimed
        ),
        (step, 2, 2),
        "{}: the next-older track carries one payout's XP",
        harness.name,
    );
    assert_eq!(
        next_unlocked.level,
        track_level(next_unlocked.xp),
        "{}: the next unlocked track's level is its XP's",
        harness.name,
    );
    let ledger = tx
        .economy_ledger_get(&profile.id)
        .await
        .unwrap_or_else(|error| panic!("{}: read the ledger: {error}", harness.name));
    tx.commit()
        .await
        .unwrap_or_else(|error| panic!("{}: commit: {error}", harness.name));

    // The first payout pays the newest track's unlock level and the unlocked track's first two
    // levels; the second payout pays the next-older track's first two. Each exactly once.
    let paid = [
        (i64::from(SET_TRACK_UNLOCK_LEVEL), newest.as_str()),
        (1, older.as_str()),
        (2, older.as_str()),
        (1, next_older.as_str()),
        (2, next_older.as_str()),
    ];
    let mut jlore: Vec<i64> = ledger
        .jlore
        .iter()
        .filter(|grant| grant.reason == JloreReason::Track)
        .inspect(|grant| assert!(is_race(&grant.reference), "{}", harness.name))
        .map(|grant| grant.delta)
        .collect();
    jlore.sort();
    let mut expected_jlore: Vec<i64> = paid
        .iter()
        .map(|(level, _)| reward_for_level(*level).jlore)
        .filter(|jlore| *jlore != 0)
        .collect();
    expected_jlore.sort();
    assert_eq!(
        jlore, expected_jlore,
        "{}: the reached levels' Jlore pays once",
        harness.name
    );

    let mut packs: Vec<(String, String, i64)> = ledger
        .inventory
        .iter()
        .filter(|grant| grant.reason == EconomyReason::Track)
        .inspect(|grant| assert!(is_race(&grant.reference), "{}", harness.name))
        .map(|grant| (format!("{:?}", grant.item), grant.key.clone(), grant.delta))
        .collect();
    packs.sort();
    let mut expected_packs = Vec::new();
    for (level, set) in &paid {
        let reward = reward_for_level(*level);
        for _ in 0..reward.set_packs {
            expected_packs.push(("SetPack".to_string(), set.to_string(), 1));
        }
        for _ in 0..reward.card_tokens {
            expected_packs.push(("CardToken".to_string(), String::new(), 1));
        }
    }
    expected_packs.sort();
    assert_eq!(
        packs, expected_packs,
        "{}: the reached levels' packs pay once",
        harness.name
    );
}

both_stores!(
    overlapping_payouts_pay_each_level_once,
    overlapping_payouts_unlock_one_set_once,
);
