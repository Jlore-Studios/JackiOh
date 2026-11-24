//! `cargo jackioh assault-sim --days N [--from <date>] [--skill S,…] [--budget tiny|full] [--all-levels]
//! [--levels L,…]`
//! (#4583, IA10): whole Infinite Assault days on the content the browser deals (the WebAssembly module's
//! `assault_content()`), each fight played to its end. Day `n` is the `n`th UTC date from `--from`
//! (`DEFAULT_FROM` by default). Each day starts one attempt at level 1 with the day's Assault deck
//! (`assault_day_deck`, IA5) and the record empty, and walks it as a player would: `fight`, the fight's
//! game, the `fightOver` its record reports. A loss ends the day's attempt; `--all-levels` plays every
//! level of the day instead, each from its own fight, so the later levels are measured too, and
//! `--levels` only the levels it lists.
//!
//! Each fight is `assault_fight_setup`'s game, the enemy's marked cards dealt Radiant and Buffed
//! (R2702000), played by `play_match` with the AI in both seats: the player's at R747000's skill (each
//! `--skill`, full by default), the enemy's at full skill, both at the budget. Its record must end clean
//! (no throw, refusal, fallback or caught search panic, and a result) and fold to its hash, and its report
//! goes back through `assault_fight_report`; an attempt's file must `assault_fold` to its end. A lost
//! fight is data; an unclean fight or a file that does not fold is a fault, which exits 1 and names the
//! command that plays its day again.
//!
//! For each skill it prints, level by level, the fights played and won, the turns, the player's HP left
//! and the enemy's; then each Commander's fights and wins; then each upgrade's effect: the fights at the
//! levels where some day holds it, won with it and won without it at those same levels (R4583000). The
//! AI plays the player's seat better or worse than a person would, so the table is a guide to tuning the
//! strawmen (IA2, IA3), not a target. Nothing reads a clock, and each day's results are kept in date
//! order, so the output is the same on any thread count.
//!
//! A pull request's tests hold the difficulty to wide bands on `BAND_DAYS` days (R4583000): level 1
//! clearly winnable, level 20 clearly hard, and each Commander harder than the level before it. A
//! deliberate retune of the escalation (`crates/run/src/config.rs`'s `ASSAULT_*`) moves the bands in the
//! same patch.
//!
//! ```text
//! cargo jackioh assault-sim --days 7                                    # a week from DEFAULT_FROM
//! cargo jackioh assault-sim --days 3 --from 2027-01-01 --budget tiny --all-levels
//! cargo test -p jackioh-tools --bin jackioh assault_sim                     # a pull request's
//! cargo test -p jackioh-tools --bin jackioh assault_sim -- --ignored --nocapture   # the daily one
//! ```

use std::collections::BTreeMap;

use anyhow::bail;
use jackioh_ai::{
    AI_BUDGET, AI_SKILL_MAX, MatchConfig, MatchHooks, MatchSetup, SearchBudget, SeatController, play_match,
};
use jackioh_engine::config::PERCENT;
use jackioh_engine::{
    CreateGameOptions, PerPlayer, PerPlayerOpt, PlayerId, SetupKind, fold, hash_state, registered_catalog,
    validate_setup,
};
use jackioh_run::assault::{
    AssaultAction, AssaultContent, AssaultDay, AssaultDeckSource, AssaultFightSetup, AssaultFile,
    AssaultPhase, AssaultRecord, AssaultSetup, AssaultState, AssaultUpgradeKind,
};
use jackioh_run::config::{ASSAULT_COMMANDER_LEVELS, ASSAULT_LEVELS};
use jackioh_run::{
    assault_create, assault_day, assault_day_seed, assault_fight_report, assault_fight_setup, assault_fold,
    assault_reduce,
};
use rayon::prelude::*;

use crate::arena::TEST_BUDGET;

/// The first day a sweep plays when `--from` names none: the day the plan was written (IA0).
const DEFAULT_FROM: &str = "2026-11-23";

/// The days of each month, February in a common year, to step from one date to the next.
const MONTH_DAYS: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
/// February's month, which takes a day more in a leap year.
const FEBRUARY: u32 = 2;
/// Leap years are divisible by 4, except centuries not divisible by 400.
const LEAP: (u32, u32, u32) = (4, 100, 400);
/// The months of a year.
const MONTHS: u32 = 12;

#[derive(clap::Args, Debug)]
pub struct Args {
    /// How many days to play for each skill.
    #[arg(long)]
    days: u32,
    /// The first day, YYYY-MM-DD.
    #[arg(long, default_value = DEFAULT_FROM)]
    from: String,
    /// R747000: the player's skills, one table each (full by default).
    #[arg(long, value_delimiter = ',', value_parser = crate::run_sim::parse_skill)]
    skill: Vec<f64>,
    /// Both seats' search budget: `tiny` is a test's, `full` the practice AI's.
    #[arg(long, value_parser = ["tiny", "full"], default_value = "full")]
    budget: String,
    /// Play every level of each day, not only those an attempt reaches before its first loss.
    #[arg(long)]
    all_levels: bool,
    /// Play only these levels of each day, each from its own fight (implies `--all-levels`'s walk).
    #[arg(long, value_delimiter = ',', value_parser = parse_level)]
    levels: Vec<u8>,
}

/// A level of the day, from 1 to `ASSAULT_LEVELS`.
fn parse_level(text: &str) -> Result<u8, String> {
    let level: u8 = text.parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
    if !(1..=ASSAULT_LEVELS).contains(&level) {
        return Err(format!("a level is between 1 and {ASSAULT_LEVELS}, not {level}"));
    }
    Ok(level)
}

/// The levels a day plays: those listed, in order, or every level with `all_levels`, or `None` for the
/// attempt's walk to its first loss.
fn levels_of(all_levels: bool, levels: &[u8]) -> Option<Vec<u8>> {
    if !levels.is_empty() {
        return Some(levels.to_vec());
    }
    all_levels.then(|| (1..=ASSAULT_LEVELS).collect())
}

/// The search budget `--budget` names: `tiny` is the tests' (`arena.rs`'s `TEST_BUDGET`), `full` the
/// practice AI's.
fn budget_of(name: &str) -> SearchBudget {
    match name {
        "tiny" => TEST_BUDGET,
        _ => AI_BUDGET,
    }
}

/// The day after `date` (a date `assault_day_seed` accepts), or why it is refused.
fn next_date(date: &str) -> Result<String, String> {
    assault_day_seed(date)?;
    let number = |from: usize, to: usize| date[from..to].parse::<u32>().map_err(|e| e.to_string());
    let (mut year, mut month, mut day) = (number(0, 4)?, number(5, 7)?, number(8, 10)?);
    let leap = year % LEAP.0 == 0 && (year % LEAP.1 != 0 || year % LEAP.2 == 0);
    let last = MONTH_DAYS[(month - 1) as usize] + u32::from(leap && month == FEBRUARY);
    day += 1;
    if day > last {
        day = 1;
        month += 1;
        if month > MONTHS {
            month = 1;
            year += 1;
        }
    }
    Ok(format!("{year:04}-{month:02}-{day:02}"))
}

/// The `days` dates from `from`, in order.
fn dates(from: &str, days: u32) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut date = from.to_string();
    for _ in 0..days {
        assault_day_seed(&date)?;
        let next = next_date(&date)?;
        out.push(std::mem::replace(&mut date, next));
    }
    Ok(out)
}

/// The content the browser deals from (the WebAssembly module's `assault_content()`).
fn browser_content() -> Result<AssaultContent, String> {
    let text = jackioh_wasm_core::assault_content()?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

/// The day's Assault deck, as the browser deals it (IA5).
fn day_deck(date: &str) -> Result<Vec<String>, String> {
    let date_json = serde_json::to_string(date).map_err(|e| e.to_string())?;
    let text = jackioh_wasm_core::assault_day_deck(&date_json)?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

/// The match a fight's setup plays: the human (P1) is the AI at `skill`, the enemy (P2) the AI at full
/// skill, both at `budget`, and the enemy's marks dealt (R2702000).
fn fight_config(setup: &AssaultFightSetup, skill: f64, budget: SearchBudget) -> MatchConfig {
    MatchConfig {
        seed: setup.seed.clone(),
        decks: (setup.human_deck.clone(), setup.enemy_deck.clone()),
        handicaps: Some(PerPlayerOpt {
            p1: Some(setup.human_handicap),
            p2: Some(setup.enemy_handicap),
        }),
        controllers: PerPlayer::new(
            SeatController::Ai {
                budget: Some(budget),
                skill: Some(skill),
            },
            SeatController::Ai {
                budget: Some(budget),
                skill: None,
            },
        ),
        max_actions: None,
        setup: MatchSetup {
            deck_marks: Some(PerPlayerOpt {
                p1: None,
                p2: Some(setup.enemy_marks.clone()),
            }),
            ..MatchSetup::default()
        },
    }
}

/// One fight played: its level's facts and how it went.
#[derive(Clone, Debug, PartialEq)]
struct FightRow {
    level: u8,
    commander: Option<String>,
    upgrades: Vec<AssaultUpgradeKind>,
    won: bool,
    turns: i32,
    /// The player's hero health at the end, 0 when it fell below.
    hp_left: i32,
    /// The enemy's hero health at the end, 0 when it fell below.
    enemy_hp_left: i32,
}

/// The fight `state` stands on, played to its end, as its row and the `fightOver` its record reports; or
/// why the fight is not clean.
fn play_fight(
    state: &AssaultState,
    day: &AssaultDay,
    skill: f64,
    budget: SearchBudget,
) -> Result<(FightRow, AssaultAction), String> {
    let setup = assault_fight_setup(state, day).ok_or("the fight has no setup")?;
    let config = fight_config(&setup, skill, budget);
    // `create_game` panics on a setup it refuses, outside the match's `catch_unwind`: refuse it here.
    let options = CreateGameOptions {
        seed: config.seed.clone(),
        decks: config.decks.clone(),
        handicaps: config.handicaps.clone(),
        deck_marks: config.setup.deck_marks.clone(),
        ..CreateGameOptions::default()
    };
    validate_setup(&options, registered_catalog(), SetupKind::New)
        .map_err(|e| format!("the engine refuses the setup: {e}"))?;
    let record = play_match(&config, &mut MatchHooks::default());
    if let Some(thrown) = record.thrown.first() {
        return Err(format!("{} threw: {}", thrown.seat.as_str(), thrown.message));
    }
    if let Some(rejected) = record.rejected.first() {
        return Err(format!(
            "{}'s \"{}\" was refused: {}",
            rejected.seat.as_str(),
            rejected.action.action_type().as_str(),
            rejected.error
        ));
    }
    if record.fallbacks > 0 {
        return Err(format!("{} decisions fell back", record.fallbacks));
    }
    if record.sim_panics > 0 {
        return Err(format!("the searches caught {} panics", record.sim_panics));
    }
    if record.result.is_none() {
        return Err(format!(
            "the game had no result after {} actions",
            record.log.len()
        ));
    }
    let game = config.fold_args(record.log);
    let replayed = fold(&game);
    if let Some(error) = replayed.errors.first() {
        return Err(format!("the fold refuses {}: {}", error.nonce, error.error));
    }
    if hash_state(&replayed.state) != record.hash {
        return Err("the record folds to another hash".to_string());
    }
    let action = assault_fight_report(&game, PlayerId::P1)?;
    let AssaultAction::FightOver { won, turns } = action else {
        return Err("the report is no fightOver".to_string());
    };
    let health = |seat: PlayerId| replayed.state.players[seat].hero.health.max(0);
    let row = FightRow {
        level: setup.level.level,
        commander: setup.level.commander.clone(),
        upgrades: setup
            .level
            .upgrades
            .iter()
            .filter(|u| u.stacks > 0)
            .map(|u| u.kind)
            .collect(),
        won,
        turns,
        hp_left: health(PlayerId::P1),
        enemy_hp_left: health(PlayerId::P2),
    };
    Ok((row, action))
}

/// The day's first attempt at level 1 with the day's Assault deck and an empty record.
fn first_attempt(day: &AssaultDay) -> Result<(AssaultSetup, AssaultRecord), String> {
    let setup = AssaultSetup {
        date: day.date.clone(),
        deck: day_deck(&day.date)?,
        source: AssaultDeckSource::DayDeck,
        start_level: 1,
        attempt: 1,
    };
    let record = AssaultRecord {
        date: day.date.clone(),
        best: 0,
        checkpoints: Vec::new(),
        attempts: 0,
        repelled_on: None,
    };
    Ok((setup, record))
}

/// A day's fights: its attempt walked to its end, or each of `levels` from its own fight, each fight
/// played by `fight`, the attempt's file checked to fold to its end.
fn play_day(
    date: &str,
    content: &AssaultContent,
    levels: Option<&[u8]>,
    fight: &(dyn Fn(&AssaultState, &AssaultDay) -> Result<(FightRow, AssaultAction), String> + Sync),
) -> Result<Vec<FightRow>, String> {
    let day = assault_day(date, content)?;
    let (setup, record) = first_attempt(&day)?;
    let start = assault_create(&setup, &record, &day)?;
    let mut rows = Vec::new();
    if let Some(levels) = levels {
        for &level in levels {
            let state = AssaultState {
                level,
                phase: AssaultPhase::Fight,
                ..start.clone()
            };
            rows.push(fight(&state, &day).map_err(|e| format!("level {level}: {e}"))?.0);
        }
        return Ok(rows);
    }
    let mut state = start;
    let mut actions = Vec::new();
    while state.phase == AssaultPhase::Briefing {
        state = assault_reduce(&state, &day, &AssaultAction::Fight)?;
        actions.push(AssaultAction::Fight);
        let level = state.level;
        let (row, over) = fight(&state, &day).map_err(|e| format!("level {level}: {e}"))?;
        state = assault_reduce(&state, &day, &over)?;
        actions.push(over);
        rows.push(row);
    }
    let file = AssaultFile {
        version: jackioh_cards::catalog_version().to_string(),
        setup,
        record,
        actions,
    };
    if assault_fold(&file, &day)? != state {
        return Err("the attempt's file folds to another state".to_string());
    }
    Ok(rows)
}

/// Fights played and won, and their sums.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Sums {
    fights: usize,
    won: usize,
    turns: i64,
    hp_left: i64,
    enemy_hp_left: i64,
}

impl Sums {
    fn add(&mut self, row: &FightRow) {
        self.fights += 1;
        self.won += usize::from(row.won);
        self.turns += i64::from(row.turns);
        self.hp_left += i64::from(row.hp_left);
        self.enemy_hp_left += i64::from(row.enemy_hp_left);
    }
}

/// `sum / n` to one decimal place, or a dash for none.
fn mean(sum: i64, n: usize) -> String {
    if n == 0 {
        return "-".to_string();
    }
    let n = n as f64;
    format!("{:.1}", sum as f64 / n)
}

/// `won of fights (percent)`, or a dash for none.
fn rate(won: usize, fights: usize) -> String {
    if fights == 0 {
        return "-".to_string();
    }
    format!("{won}/{fights} ({}%)", won * PERCENT as usize / fights)
}

/// An upgrade's serialized name (`openingCards`, …).
fn upgrade_name(kind: AssaultUpgradeKind) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| format!("{kind:?}"))
}

/// The report of `rows` under `header`: by level, by Commander, by upgrade.
fn render(header: &str, rows: &[FightRow], content: &AssaultContent) -> String {
    let mut out = format!("{header}\n");
    let mut levels: BTreeMap<u8, Sums> = BTreeMap::new();
    for row in rows {
        levels.entry(row.level).or_default().add(row);
    }
    out.push_str("level  won            turns  hp left  enemy hp left\n");
    for (level, sums) in &levels {
        let star = if ASSAULT_COMMANDER_LEVELS.contains(level) {
            "*"
        } else {
            " "
        };
        out.push_str(&format!(
            "{level:>4}{star} {:<14} {:>5}  {:>7}  {:>13}\n",
            rate(sums.won, sums.fights),
            mean(sums.turns, sums.fights),
            mean(sums.hp_left, sums.fights),
            mean(sums.enemy_hp_left, sums.fights)
        ));
    }
    let mut commanders: BTreeMap<String, Sums> = BTreeMap::new();
    for row in rows {
        if let Some(id) = &row.commander {
            commanders.entry(id.clone()).or_default().add(row);
        }
    }
    out.push_str("commander: won\n");
    for commander in &content.commanders {
        if let Some(sums) = commanders
            .get(&commander.id)
            .or_else(|| commanders.get(&commander.name))
        {
            out.push_str(&format!(
                "  {} (rank {}): {}\n",
                commander.name,
                commander.rank,
                rate(sums.won, sums.fights)
            ));
        }
    }
    out.push_str("upgrade: won with it, won without it at the same levels\n");
    for (kind, with, without) in upgrade_effects(rows) {
        out.push_str(&format!(
            "  {}: {}, {}\n",
            upgrade_name(kind),
            rate(with.won, with.fights),
            rate(without.won, without.fights)
        ));
    }
    out
}

/// R4583000: each upgrade held by some fight, with the fights at the levels where some fight holds it,
/// summed apart by whether the fight holds it, in the order the kinds are first met.
fn upgrade_effects(rows: &[FightRow]) -> Vec<(AssaultUpgradeKind, Sums, Sums)> {
    let mut kinds: Vec<AssaultUpgradeKind> = Vec::new();
    for row in rows {
        for kind in &row.upgrades {
            if !kinds.contains(kind) {
                kinds.push(*kind);
            }
        }
    }
    kinds
        .into_iter()
        .map(|kind| {
            let held_at: Vec<u8> = rows
                .iter()
                .filter(|row| row.upgrades.contains(&kind))
                .map(|row| row.level)
                .collect();
            let (mut with, mut without) = (Sums::default(), Sums::default());
            for row in rows.iter().filter(|row| held_at.contains(&row.level)) {
                if row.upgrades.contains(&kind) {
                    with.add(row);
                } else {
                    without.add(row);
                }
            }
            (kind, with, without)
        })
        .collect()
}

/// `days` days from `from` at each skill and `budget`: one table each, in order, and every fault, each
/// with the command that plays its day again.
fn sweep(
    dates: &[String],
    skills: &[f64],
    budget: &str,
    levels: Option<&[u8]>,
    flag: &str,
    content: &AssaultContent,
) -> (String, Vec<String>) {
    let search = budget_of(budget);
    let mut text = String::new();
    let mut faults = Vec::new();
    for &skill in skills {
        let days: Vec<(&String, Result<Vec<FightRow>, String>)> = dates
            .par_iter()
            .map(|date| {
                let fight = |state: &AssaultState, day: &AssaultDay| play_fight(state, day, skill, search);
                (date, play_day(date, content, levels, &fight))
            })
            .collect();
        let mut rows = Vec::new();
        for (date, day) in days {
            match day {
                Ok(played) => rows.extend(played),
                Err(error) => faults.push(format!(
                    "{date}: {error} (again: cargo jackioh assault-sim --from {date} --days 1 --skill {skill} --budget {budget}{flag})"
                )),
            }
        }
        let header = format!(
            "assault-sim: {} days from {}, budget {budget}, skill {skill}{flag}",
            dates.len(),
            dates.first().map_or("-", String::as_str)
        );
        text.push_str(&render(&header, &rows, content));
        text.push('\n');
    }
    (text, faults)
}

pub fn run(args: Args) -> anyhow::Result<()> {
    jackioh_cards::register_all();
    let content = browser_content().map_err(|e| anyhow::anyhow!("assault-sim: {e}"))?;
    let dates = dates(&args.from, args.days).map_err(|e| anyhow::anyhow!("assault-sim: {e}"))?;
    let skills = if args.skill.is_empty() {
        vec![AI_SKILL_MAX]
    } else {
        args.skill
    };
    let levels = levels_of(args.all_levels, &args.levels);
    let flag = if !args.levels.is_empty() {
        let listed: Vec<String> = args.levels.iter().map(u8::to_string).collect();
        format!(" --levels {}", listed.join(","))
    } else if args.all_levels {
        " --all-levels".to_string()
    } else {
        String::new()
    };
    let (text, faults) = sweep(&dates, &skills, &args.budget, levels.as_deref(), &flag, &content);
    print!("{text}");
    for fault in &faults {
        eprintln!("{fault}");
    }
    if !faults.is_empty() {
        bail!("assault-sim: {} faults", faults.len());
    }
    Ok(())
}
