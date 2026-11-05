//! Daily quests (SPEC §14.9, #1304, #3104), pure: the pool, the rhythm quests arrive on, what a finished game
//! counts toward each, and the decisions over an account's [`QuestBook`]: a top-up, a replace and a dev
//! give, each answered as [`LedgerGrant`]s for the store to write (`crates/shared/README.md`, the host
//! contract).
//!
//! - The pool is data, `crates/shared/quests.json`, compiled in like the cosmetics registry, and
//!   [`problems`] is its check. Every set that ships has a `playSetCards` entry, so the release
//!   that ships a set adds its quest ([[R2253000]]); a unit test holds it.
//! - Arrivals ([`top_up`], [[R1304000]]): an account's first visit starts its clock, and one quest
//!   falls due every `QUEST_INTERVAL_HOURS` from then. Each is drawn uniformly among the entries the account
//!   does not hold by the engine's `Rng` over the host's seed, which the arrival's row logs in its `ref`
//!   as `seed:<seed>:<cursor>`, so the draw replays.
//!
//! Nothing here checks `ECONOMY_LIVE` (§14.12): its callers top up and pay only while it is on
//! ([[R1304002]]). Every number is read through its [`crate::config`] constant (CLAUDE.md rule 9).

use std::collections::BTreeSet;
use std::sync::OnceLock;

use jackioh_engine::rng::Rng;
use jackioh_engine::wire::{CardDefs, CardType, SeatSummary, SetName, set_ships};
use serde::{Deserialize, Serialize};

use crate::config::{
    MS_PER_SECOND, QUEST_INTERVAL_HOURS, QUEST_JLORE_MAX, QUEST_JLORE_MIN, QUEST_MAX_HELD,
    QUEST_REPLACES_PER_DAY, QUEST_XP_PER_JLORE,
};
use crate::cosmetics::is_slug;
use crate::economy::ledger::{Cause, HeldQuest, LedgerGrant, QuestBook, QuestChange};

pub const QUESTS_JSON: &str = include_str!("../../quests.json");

/// What a quest counts, read from a finished game's public facts.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
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
    /// Position switches by Units the seat controlled, as its record counts them (R1256000, R2253000).
    PositionSwitches,
    /// Cards of the seat's that an effect made Radiant, as its record counts them (R1256000, R2253000).
    RadiantUpgrades,
    /// Cards of one set played; the set ships (R1420).
    PlaySetCards { set: SetName },
}

/// One entry of the pool.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestDef {
    /// A lower-case slug, unique in the pool and, once shipped, never renamed or removed: a held
    /// quest and its ledger rows name it.
    pub id: String,
    /// The English sentence the client shows, and translates.
    pub title: String,
    pub goal: QuestGoal,
    /// The count of the goal that completes it, at least 1.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
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

pub fn shipped() -> Result<Vec<QuestDef>, String> {
    parse(QUESTS_JSON)
}

/// The shipped pool, read once; its unit test holds it to [`problems`].
pub fn pool() -> &'static [QuestDef] {
    static POOL: OnceLock<Vec<QuestDef>> = OnceLock::new();
    POOL.get_or_init(|| shipped().expect("quests.json passes quests::problems"))
}

pub fn quest_def(id: &str) -> Option<&'static QuestDef> {
    pool().iter().find(|quest| quest.id == id)
}

/// A quest's XP (§14.9): `QUEST_XP_PER_JLORE` for each Jlore it pays.
pub fn quest_xp(def: &QuestDef) -> i64 {
    i64::from(def.jlore) * i64::from(QUEST_XP_PER_JLORE)
}

// What a game did, as the quests read it

/// One card a seat played from its hand.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PlayedCard {
    pub card_type: CardType,
    pub set: SetName,
    /// Played on its Radiant face (R1147000).
    pub radiant: bool,
}

/// What one seat did in a finished game, as the quests count it: the public facts of the game
/// the engine's `summarize_game` reads off its log (R376), and the attacks its log holds.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct GameTally {
    pub played: Vec<PlayedCard>,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub traps_fired: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub attacks: i64,
    /// R2253000: the seat's `positionSwitches` (R1256000), 0 when its record has none.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub position_switches: i64,
    /// R2253000: the seat's `radiantUpgrades` (R1256000), 0 when its record has none.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub radiant_upgrades: i64,
}

/// One seat's tally: its plays from `seat` (none when the game could not be read), looked up in
/// `defs`, and `attacks`, the caller's count of the seat's logged attacks. A played id `defs` does
/// not hold (a fused or crafted definition, R179, R882) counts as no card. R2089000: a Created card
/// the seat played from its hand is a play as a dealt one is, so its `createdPlayed` counts after
/// `played`; one it only summoned or cast is not.
pub fn tally_seat(seat: Option<&SeatSummary>, attacks: i64, defs: &CardDefs) -> GameTally {
    let Some(seat) = seat else {
        return GameTally {
            attacks,
            ..GameTally::default()
        };
    };
    let radiant = seat.played_radiant.as_deref().unwrap_or_default();
    let card = |id: &str, radiant: bool| {
        let def = defs.get(id)?;
        Some(PlayedCard {
            card_type: def.type_,
            set: def.set,
            radiant,
        })
    };
    let played = seat
        .played
        .iter()
        .enumerate()
        .filter_map(|(at, id)| card(id, radiant.get(at).copied().unwrap_or(false)))
        .chain(
            seat.created_played
                .iter()
                .flatten()
                .filter_map(|play| card(&play.card, play.radiant)),
        )
        .collect();
    GameTally {
        played,
        traps_fired: i64::from(seat.traps_fired.unwrap_or(0)),
        position_switches: i64::from(seat.position_switches.unwrap_or(0)),
        radiant_upgrades: i64::from(seat.radiant_upgrades.unwrap_or(0)),
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
        QuestGoal::PlaySpells => {
            count(&|card| matches!(card.card_type, CardType::Spell | CardType::FieldSpell))
        }
        QuestGoal::PlayTraps => count(&|card| matches!(card.card_type, CardType::Trap | CardType::FieldTrap)),
        QuestGoal::FireTraps => tally.traps_fired,
        QuestGoal::PlayRadiant => count(&|card| card.radiant),
        QuestGoal::AttackWithUnits => tally.attacks,
        QuestGoal::PositionSwitches => tally.position_switches,
        QuestGoal::RadiantUpgrades => tally.radiant_upgrades,
        QuestGoal::PlaySetCards { set } => count(&|card| card.set == *set),
    }
}

// The rhythm (R1304000)

// The rhythm (R1304000)

/// `QUEST_INTERVAL_HOURS` in milliseconds, the store's unit of time.
pub fn quest_interval_ms() -> i64 {
    // Unit conversion, not configuration: sixty minutes of sixty seconds to the hour.
    QUEST_INTERVAL_HOURS * 60 * 60 * MS_PER_SECOND
}

/// What a top-up brings: when each arriving quest fell due, oldest first, and when the next will.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopUp {
    pub arrivals: Vec<i64>,
    pub next_due_at: i64,
}

/// [[R1304000]]: the quests an account holding `held` gains at `now`, its next quest due at
/// `next_due_at` (`None` before its first visit). A first visit brings none and starts the clock: the
/// first falls due an interval later. Then each one due by `now` arrives while there is room, dated
/// when it fell due, and one due with no room is skipped: the next falls due on the same rhythm.
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

/// [[R1304000]]: one of the pool's entries not in `held`, uniformly (one draw of `rng`); `None`, with no draw,
/// when every entry is held.
pub fn draw_quest(held: &[String], rng: &mut Rng) -> Option<&'static QuestDef> {
    let open: Vec<&'static QuestDef> = pool().iter().filter(|quest| !held.contains(&quest.id)).collect();
    rng.pick(&open).copied()
}

/// The `ref` an arrival's ledger row logs its draw in (§14.11): the host's seed and the cursor of its `Rng`
/// before the draw, which replays it.
pub fn seed_reference(seed: &str, cursor: u32) -> String {
    format!("seed:{seed}:{cursor}")
}

/// The pool entry `rng` draws for an account holding `held`, as the arrival row that holds it, dated `at`;
/// `None` when every entry is held.
fn arrival(
    rng: &mut Rng,
    seed: &str,
    held: &[String],
    cause: Cause,
    at: i64,
) -> Option<(&'static QuestDef, LedgerGrant)> {
    let cursor = rng.cursor();
    let def = draw_quest(held, rng)?;
    let grant = LedgerGrant::Quest {
        quest_id: def.id.clone(),
        change: QuestChange::Arrived,
        progress: 0,
        cause,
        reference: Some(seed_reference(seed, cursor)),
        at,
    };
    Some((def, grant))
}

/// An account's quests after a top-up and the writes that make it so.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct QuestTopUp {
    /// The quests after the top-up, held oldest first.
    pub book: QuestBook,
    /// The writes, in order: each arrival (dated when it fell due), then the schedule. None when nothing was
    /// due and the clock already ran.
    pub grants: Vec<LedgerGrant>,
}

/// [[R1304000]]: tops `book` up at `now`. With nothing due it writes nothing. A first visit starts the clock;
/// else every quest that fell due while there was room arrives, drawn from `seed` in order (an exhausted pool
/// ends the top-up), and the schedule moves to the next due. The store's read locks the account's schedule
/// to the end of the transaction, so two top-ups at once bring each quest that fell due once.
pub fn top_up_book(book: &QuestBook, seed: &str, now: i64) -> QuestTopUp {
    top_up_with(&mut Rng::new(seed, 0), seed, book, now)
}

fn top_up_with(rng: &mut Rng, seed: &str, book: &QuestBook, now: i64) -> QuestTopUp {
    let due = top_up(book.held.len(), book.next_due_at, now);
    if Some(due.next_due_at) == book.next_due_at {
        return QuestTopUp {
            book: book.clone(),
            grants: Vec::new(),
        };
    }
    let mut held: Vec<String> = book.held.iter().map(|quest| quest.quest_id.clone()).collect();
    let mut after = book.clone();
    let mut grants = Vec::new();
    for at in due.arrivals {
        let Some((def, grant)) = arrival(rng, seed, &held, Cause::Quest, at) else {
            break;
        };
        grants.push(grant);
        held.push(def.id.clone());
        after.held.push(HeldQuest {
            quest_id: def.id.clone(),
            progress: 0,
            arrived_at: at,
        });
    }
    // The store's order: by when they arrived, then id.
    after
        .held
        .sort_by(|a, b| (a.arrived_at, &a.quest_id).cmp(&(b.arrived_at, &b.quest_id)));
    after.next_due_at = Some(due.next_due_at);
    grants.push(LedgerGrant::QuestDue {
        next_due_at: Some(due.next_due_at),
    });
    QuestTopUp { book: after, grants }
}

/// [[R2210000]]: the replaces the account has left in the UTC day, never below 0: `QUEST_REPLACES_PER_DAY`
/// less the quests it replaced that day (`removed_today`: its `removed` rows with the reason `quest`; going
/// live's `launch` removals, [[R1310002]], count for nothing).
pub fn replaces_left(removed_today: i64) -> i64 {
    (QUEST_REPLACES_PER_DAY - removed_today).max(0)
}

/// Why a quest was not replaced or given ([[R2210000]]). A refusal writes nothing.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "snake_case")]
pub enum QuestRefusal {
    /// The day's `QUEST_REPLACES_PER_DAY` replaces are already used.
    Spent,
    /// The account does not hold that quest.
    NotHeld,
    /// Every other entry of the pool is held, so there is none to swap in.
    NoneLeft,
    /// The account already holds `QUEST_MAX_HELD`, or the pool has none left to give.
    Full,
}

impl QuestRefusal {
    /// The sentence a player reads.
    pub fn message(self) -> &'static str {
        match self {
            QuestRefusal::Spent => "You've already replaced a quest today. You can replace another tomorrow.",
            QuestRefusal::NotHeld => "That quest is not held.",
            QuestRefusal::NoneLeft => "There's no other quest to swap in.",
            QuestRefusal::Full => "This account already holds the most quests it may.",
        }
    }
}

/// [[R2210000]]: replaces the held quest `quest_id` at `now`. `book` is read before the top-up, and the
/// account is topped up first (R1304000) from `seed`, so the replace reads what a quest due now brought: at
/// most `QUEST_REPLACES_PER_DAY` land in a UTC day (`removed_today` counts the day's). The quest leaves with
/// its progress, a `removed` row, and one drawn as an arrival is takes its slot, dated `now`; the schedule is
/// untouched by the replace itself. The writes are the top-up's, then the removal, then the arrival; a
/// refusal writes nothing, the top-up's arrivals with it.
pub fn replace_quest(
    book: &QuestBook,
    removed_today: i64,
    quest_id: &str,
    seed: &str,
    now: i64,
) -> Result<Vec<LedgerGrant>, QuestRefusal> {
    let mut rng = Rng::new(seed, 0);
    let topped = top_up_with(&mut rng, seed, book, now);
    if replaces_left(removed_today) <= 0 {
        return Err(QuestRefusal::Spent);
    }
    if !topped.book.held.iter().any(|quest| quest.quest_id == quest_id) {
        return Err(QuestRefusal::NotHeld);
    }
    // Every held id, the replaced one too, so the draw never returns it.
    let held: Vec<String> = topped
        .book
        .held
        .iter()
        .map(|quest| quest.quest_id.clone())
        .collect();
    let Some((_, arrived)) = arrival(&mut rng, seed, &held, Cause::Quest, now) else {
        return Err(QuestRefusal::NoneLeft);
    };
    let mut grants = topped.grants;
    grants.push(LedgerGrant::Quest {
        quest_id: quest_id.to_string(),
        change: QuestChange::Removed,
        progress: 0,
        cause: Cause::Quest,
        reference: None,
        at: now,
    });
    grants.push(arrived);
    Ok(grants)
}

/// A dev tool's quest at once (§14.10), drawn as an arrival is and marked `dev` by the store: the arrival
/// row. `book` is the account's quests, topped up. `Full` when it already holds `QUEST_MAX_HELD` or the pool
/// has none it does not hold.
pub fn give_quest(book: &QuestBook, seed: &str, now: i64) -> Result<LedgerGrant, QuestRefusal> {
    if book.held.len() >= QUEST_MAX_HELD {
        return Err(QuestRefusal::Full);
    }
    let held: Vec<String> = book.held.iter().map(|quest| quest.quest_id.clone()).collect();
    arrival(&mut Rng::new(seed, 0), seed, &held, Cause::Dev, now)
        .map(|(_, grant)| grant)
        .ok_or(QuestRefusal::Full)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::wire::{CreatedUse, SHIPPED_SETS};

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
        assert_eq!(
            first,
            TopUp {
                arrivals: vec![],
                next_due_at: T + i
            }
        );
        assert_eq!(
            top_up(0, Some(T + i), T + i - 1),
            TopUp {
                arrivals: vec![],
                next_due_at: T + i
            }
        );
        assert_eq!(
            top_up(0, Some(T + i), T + i),
            TopUp {
                arrivals: vec![T + i],
                next_due_at: T + 2 * i
            }
        );
    }

    #[test]
    fn r1304000_none_arrives_at_the_cap() {
        let i = interval();
        assert_eq!(
            top_up(QUEST_MAX_HELD, Some(T), T + 10 * i),
            TopUp {
                arrivals: vec![],
                next_due_at: T + 11 * i
            }
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
            let mut rng = Rng::new(&format!("quest-{seed}"), 0);
            assert_eq!(draw_quest(rest, &mut rng).map(|quest| &quest.id), Some(last));
            assert_eq!(rng.cursor(), 1, "one draw");
        }
        let drawn: BTreeSet<&str> = (0..400)
            .filter_map(|seed| draw_quest(&[], &mut Rng::new(&format!("cover-{seed}"), 0)))
            .map(|quest| quest.id.as_str())
            .collect();
        assert_eq!(drawn.len(), ids.len(), "enough seeds cover the pool");
        // An exhausted pool draws nothing, and takes no draw.
        let mut rng = Rng::new("none", 0);
        assert_eq!(draw_quest(&ids, &mut rng), None);
        assert_eq!(rng.cursor(), 0);
        assert_eq!(seed_reference("abc", 3), "seed:abc:3");
    }

    fn held(quest_id: &str, progress: i64, arrived_at: i64) -> HeldQuest {
        HeldQuest {
            quest_id: quest_id.to_string(),
            progress,
            arrived_at,
        }
    }

    /// R1304000: a top-up of a book brings each quest due, dated when it fell due and logged with the seed and
    /// cursor of its draw, then moves the schedule; the book after is the store's order.
    #[test]
    fn r1304000_a_top_up_writes_the_arrivals_then_the_schedule() {
        let i = interval();
        let book = QuestBook {
            held: vec![held("pre-held", 1, T - 3 * i)],
            next_due_at: Some(T),
        };
        let topped = top_up_book(&book, "seed-a", T + i);
        let arrived: Vec<(&str, i64)> = topped
            .grants
            .iter()
            .filter_map(|grant| match grant {
                LedgerGrant::Quest {
                    quest_id,
                    change: QuestChange::Arrived,
                    at,
                    ..
                } => Some((quest_id.as_str(), *at)),
                _ => None,
            })
            .collect();
        assert_eq!(arrived.iter().map(|(_, at)| *at).collect::<Vec<_>>(), [T, T + i]);
        assert_eq!(
            topped.grants.last(),
            Some(&LedgerGrant::QuestDue {
                next_due_at: Some(T + 2 * i)
            })
        );
        assert_eq!(topped.grants.len(), 3, "two arrivals and the schedule");
        assert_eq!(topped.book.next_due_at, Some(T + 2 * i));
        assert_eq!(topped.book.held.len(), 3);
        assert_eq!(topped.book.held[0].quest_id, "pre-held");
        // The draw logs its seed and cursor, and replays: the same seed draws the same quests.
        let LedgerGrant::Quest { reference, .. } = &topped.grants[0] else {
            panic!("an arrival first");
        };
        assert_eq!(reference.as_deref(), Some("seed:seed-a:0"));
        assert_eq!(top_up_book(&book, "seed-a", T + i), topped);
        // No quest is held twice.
        let ids: BTreeSet<&str> = topped
            .book
            .held
            .iter()
            .map(|quest| quest.quest_id.as_str())
            .collect();
        assert_eq!(ids.len(), 3);
        // Nothing due: nothing written. A first visit writes the clock alone.
        let still = QuestBook {
            held: Vec::new(),
            next_due_at: Some(T + 5 * i),
        };
        assert_eq!(
            top_up_book(&still, "s", T),
            QuestTopUp {
                book: still.clone(),
                grants: Vec::new()
            }
        );
        let first = top_up_book(&QuestBook::default(), "s", T);
        assert_eq!(
            first.grants,
            [LedgerGrant::QuestDue {
                next_due_at: Some(T + i)
            }]
        );
        assert!(first.book.held.is_empty());
    }

    /// R2210000: a replace refuses when the day's are spent, the quest is not held or none is left to swap in;
    /// it writes the removal and the arrival, after the top-up's own.
    #[test]
    fn r2210000_a_replace_removes_the_quest_and_draws_another() {
        let first = pool()[0].id.clone();
        let book = QuestBook {
            held: vec![held(&first, 2, T)],
            next_due_at: Some(T + 100 * interval()),
        };
        let grants = replace_quest(&book, 0, &first, "seed-r", T + 10).expect("a replace");
        assert_eq!(grants.len(), 2, "no top-up writes");
        assert_eq!(
            grants[0],
            LedgerGrant::Quest {
                quest_id: first.clone(),
                change: QuestChange::Removed,
                progress: 0,
                cause: Cause::Quest,
                reference: None,
                at: T + 10,
            }
        );
        let LedgerGrant::Quest {
            quest_id,
            change,
            reference,
            at,
            ..
        } = &grants[1]
        else {
            panic!("the arrival");
        };
        assert_ne!(*quest_id, first, "never the quest it replaces");
        assert_eq!((*change, *at), (QuestChange::Arrived, T + 10));
        assert_eq!(reference.as_deref(), Some("seed:seed-r:0"));
        assert_eq!(replace_quest(&book, 0, &first, "seed-r", T + 10), Ok(grants));
        assert_eq!(replace_quest(&book, 1, &first, "s", T), Err(QuestRefusal::Spent));
        assert_eq!(replaces_left(0), QUEST_REPLACES_PER_DAY);
        assert_eq!(replaces_left(QUEST_REPLACES_PER_DAY + 5), 0);
        assert_eq!(
            replace_quest(&book, 0, "no-such", "s", T),
            Err(QuestRefusal::NotHeld)
        );
        // Every other entry held: none to swap in.
        let all = QuestBook {
            held: pool().iter().map(|quest| held(&quest.id, 0, T)).collect(),
            next_due_at: Some(T + 100 * interval()),
        };
        assert_eq!(
            replace_quest(&all, 0, &first, "s", T),
            Err(QuestRefusal::NoneLeft)
        );
    }

    #[test]
    fn a_dev_give_draws_an_arrival_unless_the_account_is_full() {
        let full = QuestBook {
            held: (0..QUEST_MAX_HELD).map(|n| held(&pool()[n].id, 0, T)).collect(),
            next_due_at: None,
        };
        assert_eq!(give_quest(&full, "s", T), Err(QuestRefusal::Full));
        let one = QuestBook {
            held: vec![held(&pool()[0].id, 0, T)],
            next_due_at: None,
        };
        let Ok(LedgerGrant::Quest {
            quest_id,
            change,
            cause,
            at,
            ..
        }) = give_quest(&one, "s", T + 1)
        else {
            panic!("a quest");
        };
        assert_ne!(quest_id, pool()[0].id);
        assert_eq!((change, cause, at), (QuestChange::Arrived, Cause::Dev, T + 1));
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
            one(entry(
                "a",
                QuestGoal::PlaySetCards { set: SetName::Boss },
                1,
                QUEST_JLORE_MIN
            )),
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
                entry(
                    "a",
                    QuestGoal::PlaySetCards { set: SetName::Core },
                    8,
                    QUEST_JLORE_MIN
                ),
                entry(
                    "b",
                    QuestGoal::PlaySetCards {
                        set: SetName::Classic
                    },
                    8,
                    QUEST_JLORE_MIN
                ),
                entry("c", QuestGoal::PlayCards, 8, QUEST_JLORE_MIN),
            ])
            .is_empty()
        );

        // Every goal kind is one `progress_of` knows: a kind it does not is refused, not dropped.
        let unknown =
            r#"{"quests":[{"id":"a","title":"A","goal":{"kind":"dealDamage"},"target":1,"jlore":50}]}"#;
        assert!(parse(unknown).is_err_and(|error| error.contains("dealDamage")));
        let stray =
            r#"{"quests":[{"id":"a","title":"A","goal":{"kind":"winGames"},"target":1,"jlore":50,"xp":9}]}"#;
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
            position_switches: 3,
            radiant_upgrades: 5,
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
            (QuestGoal::PositionSwitches, 3),
            (QuestGoal::RadiantUpgrades, 5),
            (QuestGoal::PlaySetCards { set: SetName::Core }, 3),
            (
                QuestGoal::PlaySetCards {
                    set: SetName::MeditativePlus,
                },
                0,
            ),
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
                position_switches: 0,
                radiant_upgrades: 0,
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

    #[test]
    fn r2089000_created_plays_from_hand_still_count_for_the_quests() {
        let defs = &*jackioh_cards::CATALOG;
        let (unit, spell) = (
            defs.values()
                .find(|def| def.type_ == CardType::Unit && !def.is_token())
                .expect("a Unit"),
            defs.values()
                .find(|def| def.type_ == CardType::Spell && !def.is_token())
                .expect("a Spell"),
        );
        let used = |card: &str, radiant: bool| CreatedUse {
            card: card.to_owned(),
            turn: 3,
            radiant,
        };
        // A Created Unit played from the hand, and a token the seat only summoned, which is used but
        // no play.
        let seat = SeatSummary {
            played: vec![spell.id.clone()],
            created_played: Some(vec![used(&unit.id, true)]),
            created_used: Some(vec![used(&unit.id, true), used("core-t-rush", false)]),
            ..SeatSummary::default()
        };
        assert_eq!(
            tally_seat(Some(&seat), 0, defs).played,
            vec![
                PlayedCard {
                    card_type: CardType::Spell,
                    set: spell.set,
                    radiant: false
                },
                PlayedCard {
                    card_type: CardType::Unit,
                    set: unit.set,
                    radiant: true
                },
            ]
        );
    }

    /// The sets that ship with no `playSetCards` entry in `quests`.
    fn sets_without_a_quest(quests: &[QuestDef]) -> Vec<SetName> {
        SHIPPED_SETS
            .into_iter()
            .filter(|set| {
                !quests
                    .iter()
                    .any(|quest| quest.goal == QuestGoal::PlaySetCards { set: *set })
            })
            .collect()
    }

    #[test]
    fn r2253000_every_shipped_set_has_a_set_quest() {
        let quests = shipped().unwrap_or_else(|error| panic!("quests.json: {error}"));
        assert_eq!(sets_without_a_quest(&quests), Vec::<SetName>::new());
        let without: Vec<QuestDef> = quests
            .into_iter()
            .filter(|quest| quest.id != "stinkyscape-8" && quest.id != "training-grounds-8")
            .collect();
        assert_eq!(
            sets_without_a_quest(&without),
            vec![SetName::Stinkyscape, SetName::TrainingGrounds]
        );
    }

    #[test]
    fn r2253000_switches_and_upgrades_move_their_quests_as_the_record_counts_them() {
        let defs = &*jackioh_cards::CATALOG;
        // A seat's record as stored, read off its JSON: `fields` adds the counts it holds.
        let moved = |fields: &str| {
            let json = format!(r#"{{"deck":[],"opening":[],"drawn":[],"played":[]{fields}}}"#);
            let seat: SeatSummary = serde_json::from_str(&json).expect("a seat summary");
            let tally = tally_seat(Some(&seat), 0, defs);
            [QuestGoal::RadiantUpgrades, QuestGoal::PositionSwitches]
                .map(|goal| progress_of(&goal, Some(true), &tally))
        };
        assert_eq!(moved(r#","radiantUpgrades":2"#), [2, 0]);
        assert_eq!(moved(r#","positionSwitches":2"#), [0, 2]);
        assert_eq!(moved(r#","radiantUpgrades":1,"positionSwitches":3"#), [1, 3]);
        // A record written before R1256000 holds neither count, and a game the server could not read
        // has no seat: neither quest moves.
        assert_eq!(moved(""), [0, 0]);
        let unread = tally_seat(None, 4, defs);
        assert_eq!(progress_of(&QuestGoal::RadiantUpgrades, Some(true), &unread), 0);
        assert_eq!(progress_of(&QuestGoal::PositionSwitches, Some(true), &unread), 0);
    }
}
