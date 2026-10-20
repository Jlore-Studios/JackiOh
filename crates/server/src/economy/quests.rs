//! Daily quests (SPEC §14.9, #1304): the pool, the rhythm quests arrive on, and what a finished
//! game moves.
//!
//! - The pool is data, `crates/server/quests.json`, compiled in like the cosmetics registry: each
//!   entry names a goal read from a finished game's public facts ([`QuestGoal`]), its target and
//!   its Jlore, from `QUEST_JLORE_MIN` to `QUEST_JLORE_MAX`, a harder entry of the same goal paying
//!   more. [`problems`] is its check. A shipped id is never renamed or removed: a held quest and
//!   its ledger rows name it.
//! - Arrivals ([`top_up`], [[R1304000]]): an account's first visit starts its clock, and one quest
//!   falls due every `QUEST_INTERVAL_HOURS` from then. A top-up brings every quest that fell due
//!   while there was room, each dated when it fell due, so a long absence returns exactly
//!   `QUEST_MAX_HELD`; one that fell due with no room is skipped, never banked. Each is drawn
//!   uniformly from the entries not held, its fresh seed logged in its row's `ref`.
//! - Progress ([`move_quests`], [[R1304001]]): the reward hook (`rewards`) tops the account up at
//!   the game's end, then moves every held quest by what the game did, never past its target.
//! - Payout ([`complete_quest`]): a quest that reaches its target leaves the held quests and pays
//!   its Jlore and `QUEST_XP_PER_JLORE` XP for each Jlore, onto every active track (§14.7), as
//!   ledger rows. The store lets a held quest go once, and only that pays, so it pays once.
//!
//! Nothing here checks `ECONOMY_LIVE` (§14.12): its callers top up and pay only while it is on
//! ([[R1304002]]). Every number is read through its [`crate::config`] constant (CLAUDE.md rule 9).

use std::collections::BTreeSet;
use std::sync::OnceLock;

use jackioh_engine::wire::{CardDefs, CardType, SeatSummary, SetName, set_ships};
use serde::{Deserialize, Serialize};

use super::rewards::{GameSummary, XpPayout, pay_xp};
use super::store::{EconomyReason, JloreGrant, JloreReason, QuestBook, QuestChange, QuestGrant};
use crate::config::{QUEST_INTERVAL_HOURS, QUEST_JLORE_MAX, QUEST_JLORE_MIN, QUEST_MAX_HELD, QUEST_XP_PER_JLORE};
use crate::cosmetics::is_slug;
use crate::db::store::{StoreResult, Tx};

/// The pool's text, compiled in.
pub const QUESTS_JSON: &str = include_str!("../../quests.json");

/// What a quest counts, read from a finished game's public facts: its result, the cards a seat
/// played from its hand, its Traps that fired and its attacks.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum QuestGoal {
    /// Games won.
    WinGames,
    /// Games finished, won, lost or drawn.
    PlayGames,
    /// Cards played from the hand.
    PlayCards,
    /// Units played.
    PlayUnits,
    /// Spells and Field Spells played.
    PlaySpells,
    /// Traps and Field Traps played.
    PlayTraps,
    /// The seat's Traps that fired, whoever's effect fired them.
    FireTraps,
    /// Cards played on their Radiant face.
    PlayRadiant,
    /// Attacks declared.
    AttackWithUnits,
    /// Cards of one set played; the set ships (R1420).
    PlaySetCards { set: SetName },
}

/// One entry of the pool.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestDef {
    /// A lower-case slug, unique in the pool and never renamed.
    pub id: String,
    /// The English sentence the client shows, and translates.
    pub title: String,
    pub goal: QuestGoal,
    /// The count of the goal that completes it, at least 1.
    pub target: i64,
    /// The Jlore it pays, from `QUEST_JLORE_MIN` to `QUEST_JLORE_MAX`.
    pub jlore: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PoolFile {
    quests: Vec<QuestDef>,
}

/// Everything wrong with `quests`, one sentence each; empty when the pool holds.
pub fn problems(quests: &[QuestDef]) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for (at, quest) in quests.iter().enumerate() {
        let id = quest.id.as_str();
        if !seen.insert(id) {
            out.push(format!("the id {id:?} is listed twice"));
        }
        if !is_slug(id) {
            out.push(format!(
                "the id {id:?} is not a lower-case slug (a letter, then letters, digits and hyphens)"
            ));
        }
        if quest.title.trim().is_empty() {
            out.push(format!("the quest {id:?} has no title"));
        }
        if quest.target < 1 {
            out.push(format!(
                "the quest {id:?} has the target {}, not at least 1",
                quest.target
            ));
        }
        if !(QUEST_JLORE_MIN..=QUEST_JLORE_MAX).contains(&quest.jlore) {
            out.push(format!(
                "the quest {id:?} pays {} Jlore, outside {QUEST_JLORE_MIN} to {QUEST_JLORE_MAX}",
                quest.jlore
            ));
        }
        if let QuestGoal::PlaySetCards { set } = quest.goal
            && !set_ships(set)
        {
            out.push(format!(
                "the quest {id:?} counts {} cards, a set that does not ship (R1420)",
                set.as_str()
            ));
        }
        // §14.9: a harder goal pays more, so of two entries of one goal the larger target pays
        // strictly more, and two with the same target are one quest twice.
        for earlier in quests[..at].iter().filter(|earlier| earlier.goal == quest.goal) {
            let (easier, harder) = if earlier.target <= quest.target {
                (earlier, quest)
            } else {
                (quest, earlier)
            };
            if easier.target == harder.target {
                out.push(format!(
                    "the quests {:?} and {id:?} have the same goal and target",
                    earlier.id
                ));
            } else if harder.jlore <= easier.jlore {
                out.push(format!(
                    "the quest {:?} is harder than {:?} and pays no more",
                    harder.id, easier.id
                ));
            }
        }
    }
    out
}

/// The pool in `json`, or every problem with it joined by "; ".
pub fn parse(json: &str) -> Result<Vec<QuestDef>, String> {
    let file: PoolFile = serde_json::from_str(json).map_err(|error| error.to_string())?;
    let found = problems(&file.quests);
    if found.is_empty() {
        Ok(file.quests)
    } else {
        Err(found.join("; "))
    }
}

/// The pool this build ships.
pub fn shipped() -> Result<Vec<QuestDef>, String> {
    parse(QUESTS_JSON)
}

/// The shipped pool, read once; its unit test holds it to [`problems`].
pub fn pool() -> &'static [QuestDef] {
    static POOL: OnceLock<Vec<QuestDef>> = OnceLock::new();
    POOL.get_or_init(|| shipped().expect("quests.json passes quests::problems"))
}

/// The pool's entry `id`, if the pool has one.
pub fn quest_def(id: &str) -> Option<&'static QuestDef> {
    pool().iter().find(|quest| quest.id == id)
}

/// A quest's XP (§14.9): `QUEST_XP_PER_JLORE` for each Jlore it pays.
pub fn quest_xp(def: &QuestDef) -> i64 {
    i64::from(def.jlore) * i64::from(QUEST_XP_PER_JLORE)
}

// ---------------------------------------------------------------------------
// What a game did, as the quests read it
// ---------------------------------------------------------------------------

/// One card a seat played from its hand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayedCard {
    pub card_type: CardType,
    pub set: SetName,
    /// Played on its Radiant face (R1147000).
    pub radiant: bool,
}

/// What one seat did in a finished game, as the quests count it: the public facts of the game
/// the engine's `summarize_game` reads off its log (R376), and the attacks its log holds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GameTally {
    pub played: Vec<PlayedCard>,
    pub traps_fired: i64,
    pub attacks: i64,
}

/// One seat's tally: its plays from `seat` (none when the game could not be read), looked up in
/// `defs`, and `attacks`, the caller's count of the seat's logged attacks. A played id `defs` does
/// not hold (a fused or crafted definition, R179, R882) counts as no card.
pub fn tally_seat(seat: Option<&SeatSummary>, attacks: i64, defs: &CardDefs) -> GameTally {
    let Some(seat) = seat else {
        return GameTally {
            attacks,
            ..GameTally::default()
        };
    };
    let radiant = seat.played_radiant.as_deref().unwrap_or_default();
    let played = seat
        .played
        .iter()
        .enumerate()
        .filter_map(|(at, id)| {
            let def = defs.get(id)?;
            Some(PlayedCard {
                card_type: def.type_,
                set: def.set,
                radiant: radiant.get(at).copied().unwrap_or(false),
            })
        })
        .collect();
    GameTally {
        played,
        traps_fired: i64::from(seat.traps_fired.unwrap_or(0)),
        attacks,
    }
}

/// How far one finished game moves a quest of `goal`, for a seat whose result is `won`
/// (`Some(true)` a win, `Some(false)` a loss, `None` a draw) and whose game `tally` holds.
pub fn progress_of(goal: &QuestGoal, won: Option<bool>, tally: &GameTally) -> i64 {
    let count = |keep: &dyn Fn(&PlayedCard) -> bool| {
        i64::try_from(tally.played.iter().filter(|card| keep(card)).count()).unwrap_or(i64::MAX)
    };
    match goal {
        QuestGoal::WinGames => i64::from(won == Some(true)),
        QuestGoal::PlayGames => 1,
        QuestGoal::PlayCards => count(&|_| true),
        QuestGoal::PlayUnits => count(&|card| card.card_type == CardType::Unit),
        QuestGoal::PlaySpells => count(&|card| matches!(card.card_type, CardType::Spell | CardType::FieldSpell)),
        QuestGoal::PlayTraps => count(&|card| matches!(card.card_type, CardType::Trap | CardType::FieldTrap)),
        QuestGoal::FireTraps => tally.traps_fired,
        QuestGoal::PlayRadiant => count(&|card| card.radiant),
        QuestGoal::AttackWithUnits => tally.attacks,
        QuestGoal::PlaySetCards { set } => count(&|card| card.set == *set),
    }
}

// ---------------------------------------------------------------------------
// The rhythm (R1304000)
// ---------------------------------------------------------------------------

/// `QUEST_INTERVAL_HOURS` in milliseconds, the store's unit of time.
pub fn quest_interval_ms() -> i64 {
    QUEST_INTERVAL_HOURS * crate::social::MS_PER_HOUR
}

/// What a top-up brings: when each arriving quest fell due, oldest first, and when the next will.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopUp {
    pub arrivals: Vec<i64>,
    pub next_due_at: i64,
}

/// [[R1304000]]: the quests an account holding `held` gains at `now`, its next quest due at
/// `next_due_at` (`None` before its first visit). A first visit brings none and starts the clock:
/// the first falls due an interval later. Then each one due by `now` arrives while there is room,
/// dated when it fell due, and one due with no room is skipped: the next falls due on the same
/// rhythm, after `now`.
pub fn top_up(held: usize, next_due_at: Option<i64>, now: i64) -> TopUp {
    let interval = quest_interval_ms();
    let Some(mut due) = next_due_at else {
        return TopUp {
            arrivals: Vec::new(),
            next_due_at: now.saturating_add(interval),
        };
    };
    let (mut held, mut arrivals) = (held, Vec::new());
    while due <= now && held < QUEST_MAX_HELD {
        arrivals.push(due);
        held += 1;
        due = due.saturating_add(interval);
    }
    if due <= now {
        due = due.saturating_add(((now - due) / interval + 1).saturating_mul(interval));
    }
    TopUp {
        arrivals,
        next_due_at: due,
    }
}

/// [[R1304000]]: the `seed % n`-th of the `n` pool entries not in `held`, so a fresh seed draws
/// uniformly among them; `None` when every entry is held.
pub fn pick_quest(held: &[String], seed: u64) -> Option<&'static QuestDef> {
    let open: Vec<&'static QuestDef> = pool()
        .iter()
        .filter(|quest| !held.contains(&quest.id))
        .collect();
    let count = u64::try_from(open.len()).ok().filter(|count| *count > 0)?;
    open.get(usize::try_from(seed % count).ok()?).copied()
}

/// A draw's seed, fresh from the operating system's random source as every roll's is (§14.11), so
/// no seed an account can read foretells its next quest.
pub fn fresh_seed() -> u64 {
    let mut bytes = [0_u8; 8];
    getrandom::fill(&mut bytes).expect("the operating system's random source failed");
    u64::from_le_bytes(bytes)
}

/// The `ref` an arrival's ledger row logs its draw's seed in (§14.11).
pub fn seed_reference(seed: u64) -> String {
    format!("seed:{seed:016x}")
}

/// The pool entry `seed` draws for an account holding `held`, as the arrival row that holds it,
/// for the caller to write; `None` when every entry is held.
pub fn arrival(
    profile_id: &str,
    held: &[String],
    seed: u64,
    reason: EconomyReason,
    at: i64,
) -> Option<(&'static QuestDef, QuestGrant)> {
    let def = pick_quest(held, seed)?;
    let grant = QuestGrant {
        profile_id: profile_id.to_string(),
        quest_id: def.id.clone(),
        change: QuestChange::Arrived,
        progress: 0,
        reason,
        reference: Some(seed_reference(seed)),
        dev: reason == EconomyReason::Dev,
        at,
    };
    Some((def, grant))
}

/// [[R1304000]]: tops the account up at `now`, in the caller's transaction, and answers its
/// quests after. With nothing due it writes nothing. Otherwise it writes the schedule as it read
/// it before reading it again, so a second top-up of the same account waits for this one's
/// transaction and then reads what it wrote: two at once never hold past `QUEST_MAX_HELD`.
pub async fn top_up_quests(tx: &mut Tx<'_>, profile_id: &str, now: i64) -> StoreResult<QuestBook> {
    let read = tx.economy_quests_get(profile_id).await?;
    if Some(top_up(read.held.len(), read.next_due_at, now).next_due_at) == read.next_due_at {
        return Ok(read);
    }
    tx.economy_quests_set_next_due(profile_id, read.next_due_at)
        .await?;
    let book = tx.economy_quests_get(profile_id).await?;
    let due = top_up(book.held.len(), book.next_due_at, now);
    if Some(due.next_due_at) == book.next_due_at {
        return Ok(book);
    }
    let mut held: Vec<String> = book.held.iter().map(|quest| quest.quest_id.clone()).collect();
    for at in due.arrivals {
        let Some((def, grant)) = arrival(profile_id, &held, fresh_seed(), EconomyReason::Quest, at) else {
            break;
        };
        tx.economy_quests_change(&grant).await?;
        held.push(def.id.clone());
    }
    tx.economy_quests_set_next_due(profile_id, Some(due.next_due_at))
        .await?;
    tx.economy_quests_get(profile_id).await
}

/// Completes the held quest `def` and pays it, in the caller's transaction: its Jlore, and its XP
/// onto every active track (§14.7), each row naming the quest and marked `dev` when a dev tool
/// completed it (§14.10). False, paying nothing, when the account does not hold it: the store
/// lets a held quest go once, so a quest pays once.
pub async fn complete_quest(
    tx: &mut Tx<'_>,
    profile_id: &str,
    def: &QuestDef,
    reason: EconomyReason,
    reference: Option<&str>,
    dev: bool,
    at: i64,
) -> StoreResult<bool> {
    let completed = tx
        .economy_quests_change(&QuestGrant {
            profile_id: profile_id.to_string(),
            quest_id: def.id.clone(),
            change: QuestChange::Completed,
            progress: 0,
            reason,
            reference: reference.map(str::to_string),
            dev,
            at,
        })
        .await?;
    if !completed {
        return Ok(false);
    }
    if def.jlore != 0 {
        tx.economy_jlore_grant(&JloreGrant {
            profile_id: profile_id.to_string(),
            delta: i64::from(def.jlore),
            reason: JloreReason::Quest,
            reference: Some(def.id.clone()),
            dev,
            at,
        })
        .await?;
    }
    pay_xp(
        tx,
        &XpPayout {
            profile_id,
            xp: quest_xp(def),
            reason: EconomyReason::Quest,
            reference: &def.id,
            dev,
            at,
        },
    )
    .await?;
    Ok(true)
}

/// [[R1304001]]: one finished game moves the account's quests, in the reward hook's transaction.
/// The account is topped up at the game's end first, so a quest that fell due as it ended counts
/// it too. Each held quest then moves by what the game did, never past its target, and one that
/// reaches its target completes and pays.
pub async fn move_quests(tx: &mut Tx<'_>, summary: &GameSummary) -> StoreResult<()> {
    let book = top_up_quests(tx, &summary.profile_id, summary.at).await?;
    for held in &book.held {
        let Some(def) = quest_def(&held.quest_id) else {
            continue;
        };
        let step = progress_of(&def.goal, summary.won, &summary.tally).min((def.target - held.progress).max(0));
        if step > 0
            && !tx
                .economy_quests_change(&QuestGrant {
                    profile_id: summary.profile_id.clone(),
                    quest_id: def.id.clone(),
                    change: QuestChange::Progressed,
                    progress: step,
                    reason: EconomyReason::Game,
                    reference: Some(summary.match_id.clone()),
                    dev: false,
                    at: summary.at,
                })
                .await?
        {
            continue;
        }
        if held.progress.saturating_add(step) >= def.target {
            complete_quest(
                tx,
                &summary.profile_id,
                def,
                EconomyReason::Game,
                Some(&summary.match_id),
                false,
                summary.at,
            )
            .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::wire::SHIPPED_SETS;

    const T: i64 = 1_000_000;

    fn interval() -> i64 {
        quest_interval_ms()
    }

    fn entry(id: &str, goal: QuestGoal, target: i64, jlore: i32) -> QuestDef {
        QuestDef {
            id: id.to_owned(),
            title: id.to_owned(),
            goal,
            target,
            jlore,
        }
    }

    #[test]
    fn r1304000_a_quest_arrives_one_interval_after_the_last() {
        let i = interval();
        let first = top_up(0, None, T);
        assert_eq!(first, TopUp { arrivals: vec![], next_due_at: T + i });
        assert_eq!(
            top_up(0, Some(T + i), T + i - 1),
            TopUp { arrivals: vec![], next_due_at: T + i }
        );
        assert_eq!(
            top_up(0, Some(T + i), T + i),
            TopUp { arrivals: vec![T + i], next_due_at: T + 2 * i }
        );
    }

    #[test]
    fn r1304000_none_arrives_at_the_cap() {
        let i = interval();
        assert_eq!(
            top_up(QUEST_MAX_HELD, Some(T), T + 10 * i),
            TopUp { arrivals: vec![], next_due_at: T + 11 * i }
        );
    }

    #[test]
    fn r1304000_a_long_absence_returns_exactly_the_cap() {
        let i = interval();
        let away = top_up(0, Some(T), T + 100 * i);
        let rhythm: Vec<i64> = (0..QUEST_MAX_HELD)
            .map(|n| T + i64::try_from(n).expect("a count") * i)
            .collect();
        assert_eq!(away.arrivals, rhythm);
        assert_eq!(away.next_due_at, T + 101 * i);
        let one_held = top_up(1, Some(T), T + 100 * i);
        assert_eq!(one_held.arrivals, rhythm[..QUEST_MAX_HELD - 1].to_vec());
        assert_eq!(one_held.next_due_at, T + 101 * i);
    }

    #[test]
    fn r1304000_a_quest_is_drawn_from_the_entries_not_held() {
        let ids: Vec<String> = pool().iter().map(|quest| quest.id.clone()).collect();
        let (last, rest) = ids.split_last().expect("a pool");
        for seed in 0..64 {
            assert_eq!(pick_quest(rest, seed).map(|quest| &quest.id), Some(last));
        }
        let drawn: BTreeSet<&str> = (0..u64::try_from(ids.len()).expect("a count"))
            .filter_map(|seed| pick_quest(&[], seed))
            .map(|quest| quest.id.as_str())
            .collect();
        assert_eq!(drawn.len(), ids.len(), "seeds 0..n cover the pool");
        assert_eq!(pick_quest(&ids, 7), None);
        assert_eq!(seed_reference(255), "seed:00000000000000ff");
    }

    #[test]
    fn the_shipped_pool_passes_its_check_and_every_payout_is_in_range() {
        let quests = shipped().unwrap_or_else(|error| panic!("quests.json: {error}"));
        assert!(quests.len() >= 15, "a first pool of at least 15");
        for quest in &quests {
            assert!(
                (QUEST_JLORE_MIN..=QUEST_JLORE_MAX).contains(&quest.jlore),
                "{} pays {}",
                quest.id,
                quest.jlore
            );
        }
        assert_eq!(pool(), quests.as_slice());

        let one = |quest: QuestDef| problems(&[quest]);
        assert_eq!(
            one(entry("a", QuestGoal::WinGames, 1, QUEST_JLORE_MAX + 1)),
            vec![format!(
                "the quest \"a\" pays {} Jlore, outside {QUEST_JLORE_MIN} to {QUEST_JLORE_MAX}",
                QUEST_JLORE_MAX + 1
            )]
        );
        assert_eq!(
            one(entry("a", QuestGoal::WinGames, 1, QUEST_JLORE_MIN - 1)).len(),
            1
        );
        assert_eq!(
            one(entry("a", QuestGoal::PlaySetCards { set: SetName::Boss }, 1, QUEST_JLORE_MIN)),
            vec!["the quest \"a\" counts Boss cards, a set that does not ship (R1420)".to_owned()]
        );
        assert_eq!(
            one(entry("a", QuestGoal::WinGames, 0, QUEST_JLORE_MIN)),
            vec!["the quest \"a\" has the target 0, not at least 1".to_owned()]
        );
        assert_eq!(
            one(entry("A b", QuestGoal::WinGames, 1, QUEST_JLORE_MIN)).len(),
            1
        );
        assert_eq!(
            problems(&[
                entry("a", QuestGoal::WinGames, 1, QUEST_JLORE_MIN),
                entry("a", QuestGoal::PlayGames, 1, QUEST_JLORE_MIN),
            ]),
            vec!["the id \"a\" is listed twice".to_owned()]
        );
        assert_eq!(
            problems(&[
                entry("easy", QuestGoal::WinGames, 1, QUEST_JLORE_MIN),
                entry("hard", QuestGoal::WinGames, 3, QUEST_JLORE_MIN),
            ]),
            vec!["the quest \"hard\" is harder than \"easy\" and pays no more".to_owned()]
        );
        assert_eq!(
            problems(&[
                entry("a", QuestGoal::WinGames, 2, QUEST_JLORE_MIN),
                entry("b", QuestGoal::WinGames, 2, QUEST_JLORE_MAX),
            ]),
            vec!["the quests \"a\" and \"b\" have the same goal and target".to_owned()]
        );
        // Entries of different goals, or of different sets, never compare.
        assert!(
            problems(&[
                entry("a", QuestGoal::PlaySetCards { set: SetName::Core }, 8, QUEST_JLORE_MIN),
                entry("b", QuestGoal::PlaySetCards { set: SetName::Classic }, 8, QUEST_JLORE_MIN),
                entry("c", QuestGoal::PlayCards, 8, QUEST_JLORE_MIN),
            ])
            .is_empty()
        );

        // Every goal kind is one `progress_of` knows: a kind it does not is refused, not dropped.
        let unknown = r#"{"quests":[{"id":"a","title":"A","goal":{"kind":"dealDamage"},"target":1,"jlore":50}]}"#;
        assert!(parse(unknown).is_err_and(|error| error.contains("dealDamage")));
        let stray = r#"{"quests":[{"id":"a","title":"A","goal":{"kind":"winGames"},"target":1,"jlore":50,"xp":9}]}"#;
        assert!(parse(stray).is_err_and(|error| error.contains("unknown field")));
    }

    #[test]
    fn each_goal_kind_reads_its_own_count() {
        let card = |card_type: CardType, set: SetName, radiant: bool| PlayedCard {
            card_type,
            set,
            radiant,
        };
        let tally = GameTally {
            played: vec![
                card(CardType::Unit, SetName::Core, false),
                card(CardType::Unit, SetName::Classic, true),
                card(CardType::Spell, SetName::Core, false),
                card(CardType::FieldSpell, SetName::Meditative, false),
                card(CardType::Trap, SetName::Core, true),
                card(CardType::FieldTrap, SetName::ClassicPlus, false),
            ],
            traps_fired: 4,
            attacks: 7,
        };
        let won = Some(true);
        for (goal, count) in [
            (QuestGoal::WinGames, 1),
            (QuestGoal::PlayGames, 1),
            (QuestGoal::PlayCards, 6),
            (QuestGoal::PlayUnits, 2),
            (QuestGoal::PlaySpells, 2),
            (QuestGoal::PlayTraps, 2),
            (QuestGoal::FireTraps, 4),
            (QuestGoal::PlayRadiant, 2),
            (QuestGoal::AttackWithUnits, 7),
            (QuestGoal::PlaySetCards { set: SetName::Core }, 3),
            (QuestGoal::PlaySetCards { set: SetName::MeditativePlus }, 0),
        ] {
            assert_eq!(progress_of(&goal, won, &tally), count, "{goal:?}");
        }
        assert_eq!(progress_of(&QuestGoal::WinGames, Some(false), &tally), 0);
        assert_eq!(progress_of(&QuestGoal::WinGames, None, &tally), 0);
        assert_eq!(progress_of(&QuestGoal::PlayGames, None, &tally), 1);
        assert!(SHIPPED_SETS.iter().all(|set| set_ships(*set)));
    }

    #[test]
    fn a_seat_s_tally_reads_its_plays_through_the_catalog() {
        let defs = &*jackioh_cards::CATALOG;
        let (unit, spell) = (
            defs.values()
                .find(|def| def.type_ == CardType::Unit && !def.is_token())
                .expect("a Unit"),
            defs.values()
                .find(|def| def.type_ == CardType::Spell && !def.is_token())
                .expect("a Spell"),
        );
        let seat = SeatSummary {
            played: vec![unit.id.clone(), "t-1".to_owned(), spell.id.clone()],
            played_radiant: Some(vec![true, false]),
            traps_fired: Some(2),
            ..SeatSummary::default()
        };
        assert_eq!(
            tally_seat(Some(&seat), 5, defs),
            GameTally {
                played: vec![
                    PlayedCard {
                        card_type: CardType::Unit,
                        set: unit.set,
                        radiant: true
                    },
                    PlayedCard {
                        card_type: CardType::Spell,
                        set: spell.set,
                        radiant: false
                    },
                ],
                traps_fired: 2,
                attacks: 5,
            }
        );
        assert_eq!(
            tally_seat(None, 3, defs),
            GameTally {
                attacks: 3,
                ..GameTally::default()
            }
        );
    }
}
