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
use jackioh_ai::{AI_SKILL_MAX, MatchConfig, SearchBudget};
use jackioh_engine::config::PERCENT;
use jackioh_engine::{PerPlayerOpt, PlayerId};
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

use crate::run_sim::{ai_fight_config, budget_of, mean, play_clean_match};

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
    let mut config = ai_fight_config(
        setup.seed.clone(),
        (setup.human_deck.clone(), setup.enemy_deck.clone()),
        PerPlayerOpt {
            p1: Some(setup.human_handicap),
            p2: Some(setup.enemy_handicap),
        },
        PerPlayerOpt {
            p1: None,
            p2: Some(setup.enemy_marks.clone()),
        },
        skill,
        budget,
    );
    // R4597001: the enemy's seat names the Assault's set, so its deck may hold the set's cards.
    config.setup.deck_sets = (!setup.enemy_sets.is_empty()).then(|| PerPlayerOpt {
        p1: None,
        p2: Some(setup.enemy_sets.clone()),
    });
    config
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
    let (game, replayed) = play_clean_match(&fight_config(&setup, skill, budget))?;
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

/// A fight player: the fight `state` stands on, as its row and the `fightOver` its record reports.
type FightFn<'a> =
    dyn Fn(&AssaultState, &AssaultDay) -> Result<(FightRow, AssaultAction), String> + Sync + 'a;

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
    fight: &FightFn<'_>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::TEST_BUDGET;

    /// The days, from `DEFAULT_FROM`, whose fights the bands play.
    const BAND_DAYS: u32 = 6;
    /// R4583000: the fewest level-1 fights, in percent, the day's Assault deck may win before level 1 is
    /// no longer clearly winnable (measured at `TEST_BUDGET` from `DEFAULT_FROM` on the set's real cards
    /// since the release, R4597002: 6 of 6; on the placeholder before it, 3 of 6).
    const LEVEL_ONE_WIN_PERCENT_MIN: usize = 25;
    /// R4583000: the most level-20 fights, in percent, it may win before level 20 is no longer clearly
    /// hard (measured at `TEST_BUDGET` from `DEFAULT_FROM` since the release, R4597002: 1 of 6, from 2 of
    /// 6 before its re-tune; on the placeholder before it, none of 12).
    const LEVEL_TWENTY_WIN_PERCENT_MAX: usize = 20;
    /// The days the daily sweep walks, at full budget.
    const DAILY_DAYS: u32 = 4;

    /// The fights at `level` in `rows`, summed.
    fn sums_at(rows: &[FightRow], level: u8) -> Sums {
        let mut sums = Sums::default();
        rows.iter()
            .filter(|row| row.level == level)
            .for_each(|row| sums.add(row));
        sums
    }

    /// R4583000: a level's margin, the player's HP left less the enemy's, summed over its fights; the
    /// lower, the harder.
    fn margin(sums: &Sums) -> i64 {
        sums.hp_left - sums.enemy_hp_left
    }

    /// R4583000: level 1's band, broken when it won under `LEVEL_ONE_WIN_PERCENT_MIN` percent of its
    /// fights or was not played.
    fn level_one_band(rows: &[FightRow]) -> Option<String> {
        let first = sums_at(rows, 1);
        (first.fights == 0 || first.won * (PERCENT as usize) < first.fights * LEVEL_ONE_WIN_PERCENT_MIN).then(|| {
            format!(
                "level 1 won {} of {}, under {LEVEL_ONE_WIN_PERCENT_MIN}%: it is no longer clearly winnable",
                first.won, first.fights
            )
        })
    }

    /// R4583000: the top level's band, broken when it won over `LEVEL_TWENTY_WIN_PERCENT_MAX` percent of
    /// its fights or was not played.
    fn top_level_band(rows: &[FightRow]) -> Option<String> {
        let last = sums_at(rows, ASSAULT_LEVELS);
        (last.fights == 0 || last.won * (PERCENT as usize) > last.fights * LEVEL_TWENTY_WIN_PERCENT_MAX).then(|| {
            format!(
                "level {ASSAULT_LEVELS} won {} of {}, over {LEVEL_TWENTY_WIN_PERCENT_MAX}%: it is no longer clearly hard",
                last.won, last.fights
            )
        })
    }

    /// R4583000: a Commander level's band, broken when its mean margin is no lower than the level before
    /// it, or either was not played.
    fn commander_band(rows: &[FightRow], commander: u8) -> Option<String> {
        let (before, boss) = (sums_at(rows, commander - 1), sums_at(rows, commander));
        // Compares the means without dividing: boss / b < before / c, as boss × c < before × b.
        let harder = before.fights > 0
            && boss.fights > 0
            && margin(&boss) * (before.fights as i64) < margin(&before) * (boss.fights as i64);
        (!harder).then(|| {
            format!(
                "level {commander}'s margin {} over {} fights is no lower than level {}'s {} over {}: the Commander is no longer harder",
                margin(&boss),
                boss.fights,
                commander - 1,
                margin(&before),
                before.fights
            )
        })
    }

    /// Every band `rows` breaks, each named with its numbers.
    fn broken_bands(rows: &[FightRow]) -> Vec<String> {
        let mut broken: Vec<String> = [level_one_band(rows), top_level_band(rows)]
            .into_iter()
            .flatten()
            .collect();
        broken.extend(
            ASSAULT_COMMANDER_LEVELS
                .iter()
                .filter_map(|&c| commander_band(rows, c)),
        );
        broken
    }

    /// `levels` on each of `BAND_DAYS` days, played to its end at `TEST_BUDGET` and full skill, every
    /// fight on its own core, back in input order.
    fn play_bands(content: &AssaultContent, levels: &[u8]) -> Vec<FightRow> {
        let dates = dates(DEFAULT_FROM, BAND_DAYS).expect("the band days");
        let jobs: Vec<(u8, &String)> = levels
            .iter()
            .flat_map(|&level| dates.iter().map(move |date| (level, date)))
            .collect();
        jobs.into_par_iter()
            .map(|(level, date)| {
                // The bands pin tuned outcomes, not rolls (#3117), as run-sim's do.
                let _unshifted = jackioh_engine::testkit::hold_pool_shift(0);
                let fight = |state: &AssaultState, day: &AssaultDay| {
                    play_fight(state, day, AI_SKILL_MAX, TEST_BUDGET)
                };
                play_day(date, content, Some(&[level]), &fight).map_err(|e| format!("{date}: {e}"))
            })
            .collect::<Result<Vec<_>, _>>()
            .expect("the band fights end clean")
            .into_iter()
            .flatten()
            .collect()
    }

    fn content() -> AssaultContent {
        jackioh_cards::register_all();
        browser_content().expect("the browser's content")
    }

    /// IA10: the escalation's numbers are strawmen to tune, not to break. `levels` are played from their
    /// own fights on the content the browser deals, as `assault-sim --levels` plays them, and `band`
    /// must hold. Each band is a test of its own, so the shards of ci.yml's `tools, wasm` share them.
    fn hold_band(levels: &[u8], band: impl Fn(&[FightRow]) -> Option<String>) {
        let content = content();
        let rows = play_bands(&content, levels);
        if let Some(broken) = band(&rows) {
            panic!(
                "the Assault broke its band (change the bands only with a deliberate retune):\n{broken}\n{}",
                render("the band", &rows, &content)
            );
        }
    }

    #[test]
    fn r4583000_level_1_is_clearly_winnable() {
        hold_band(&[1], level_one_band);
    }

    #[test]
    fn r4583000_level_20_is_clearly_hard() {
        hold_band(&[ASSAULT_LEVELS], top_level_band);
    }

    /// The `n`th Commander level and the level before it, the Commander's band held.
    fn hold_commander_band(n: usize) {
        let commander = ASSAULT_COMMANDER_LEVELS[n];
        hold_band(&[commander - 1, commander], |rows| {
            commander_band(rows, commander)
        });
    }

    #[test]
    fn r4583000_the_rank_1_commander_is_harder_than_level_4() {
        hold_commander_band(0);
    }

    #[test]
    fn r4583000_the_rank_2_commander_is_harder_than_level_9() {
        hold_commander_band(1);
    }

    #[test]
    fn r4583000_the_rank_3_commander_is_harder_than_level_14() {
        hold_commander_band(2);
    }

    #[test]
    fn r4583000_the_rank_4_commander_is_harder_than_level_19() {
        hold_commander_band(3);
    }

    /// A fight at `level` that ends `won`, with `hp_left` and `enemy_hp_left`.
    fn row(level: u8, won: bool, hp_left: i32, enemy_hp_left: i32) -> FightRow {
        FightRow {
            level,
            commander: None,
            upgrades: Vec::new(),
            won,
            turns: 1,
            hp_left,
            enemy_hp_left,
        }
    }

    /// Rows that hold every band: level 1 won, level 20 lost, each Commander lost by more than the
    /// level before it.
    fn holding() -> Vec<FightRow> {
        let mut rows = vec![row(1, true, 10, 0), row(ASSAULT_LEVELS, false, 0, 30)];
        for commander in ASSAULT_COMMANDER_LEVELS {
            rows.push(row(commander - 1, false, 0, 5));
            if commander != ASSAULT_LEVELS {
                rows.push(row(commander, false, 0, 10));
            }
        }
        rows
    }

    #[test]
    fn r4583000_each_band_names_the_edit_that_breaks_it() {
        assert_eq!(broken_bands(&holding()), Vec::<String>::new());
        let mut easy_one = holding();
        easy_one[0] = row(1, false, 0, 5);
        let broken = broken_bands(&easy_one);
        assert_eq!(broken.len(), 1, "{broken:?}");
        assert!(broken[0].starts_with("level 1 won 0 of 1"), "{broken:?}");
        let mut soft_top = holding();
        soft_top[1] = row(ASSAULT_LEVELS, true, 1, 0);
        let broken = broken_bands(&soft_top);
        // A won level 20 is also no harder than level 19.
        assert_eq!(broken.len(), 2, "{broken:?}");
        assert!(broken[0].starts_with("level 20 won 1 of 1"), "{broken:?}");
        let mut soft_boss = holding();
        soft_boss.retain(|row| row.level != ASSAULT_COMMANDER_LEVELS[1]);
        soft_boss.push(row(ASSAULT_COMMANDER_LEVELS[1], false, 0, 5));
        let broken = broken_bands(&soft_boss);
        assert_eq!(broken.len(), 1, "{broken:?}");
        assert!(broken[0].starts_with("level 10's margin -5"), "{broken:?}");
        assert_eq!(broken_bands(&[]).len(), 2 + ASSAULT_COMMANDER_LEVELS.len());
    }

    #[test]
    fn r4583000_an_upgrade_is_weighed_against_its_own_levels() {
        let held = |level: u8, won: bool| FightRow {
            upgrades: vec![AssaultUpgradeKind::HeroArmor],
            ..row(level, won, 0, 0)
        };
        let rows = [
            held(3, false),
            row(3, true, 0, 0),
            row(3, true, 0, 0),
            row(9, true, 0, 0),
        ];
        let effects = upgrade_effects(&rows);
        assert_eq!(effects.len(), 1);
        let (kind, with, without) = effects[0];
        assert_eq!(kind, AssaultUpgradeKind::HeroArmor);
        assert_eq!((with.fights, with.won), (1, 0));
        // Level 9 holds no armor anywhere, so it is not weighed.
        assert_eq!((without.fights, without.won), (2, 2));
        let text = render("t", &rows, &content());
        assert!(text.contains("  heroArmor: 0/1 (0%), 2/2 (100%)\n"), "{text}");
        assert!(text.contains("   3  2/3 (66%)"), "{text}");
    }

    #[test]
    fn the_dates_step_over_months_leap_days_and_years() {
        assert_eq!(next_date("2026-11-30").unwrap(), "2026-12-01");
        assert_eq!(next_date("2026-12-31").unwrap(), "2027-01-01");
        assert_eq!(next_date("2028-02-28").unwrap(), "2028-02-29");
        assert_eq!(next_date("2027-02-28").unwrap(), "2027-03-01");
        assert_eq!(next_date("2100-02-28").unwrap(), "2100-03-01");
        assert_eq!(next_date("2000-02-28").unwrap(), "2000-02-29");
        assert!(next_date("2026-13-01").is_err());
        assert_eq!(
            dates("2026-11-29", 3).unwrap(),
            ["2026-11-29", "2026-11-30", "2026-12-01"]
        );
        assert!(dates("tomorrow", 1).is_err());
        assert_eq!(levels_of(false, &[]), None);
        assert_eq!(
            levels_of(true, &[]).map(|l| l.len()),
            Some(ASSAULT_LEVELS as usize)
        );
        assert_eq!(levels_of(true, &[3]), Some(vec![3]));
        assert!(parse_level("0").is_err() && parse_level("21").is_err() && parse_level("20").is_ok());
    }

    #[test]
    fn an_attempt_walks_to_its_first_loss_and_its_file_folds() {
        // Stub fights, no game: won through level 6, lost at 7. The walk's own `assault_fold` check
        // runs on its file.
        let content = content();
        let fight = |state: &AssaultState, _day: &AssaultDay| {
            let won = state.level < 7;
            let r = row(state.level, won, 1, 0);
            Ok((r, AssaultAction::FightOver { won, turns: 1 }))
        };
        let rows = play_day(DEFAULT_FROM, &content, None, &fight).expect("the walk ends clean");
        let levels: Vec<u8> = rows.iter().map(|row| row.level).collect();
        assert_eq!(levels, (1..=7).collect::<Vec<u8>>());
        let all = play_day(DEFAULT_FROM, &content, levels_of(true, &[]).as_deref(), &fight).unwrap();
        assert_eq!(all.len(), ASSAULT_LEVELS as usize);
        let won = |state: &AssaultState, _day: &AssaultDay| {
            Ok((
                row(state.level, true, 1, 0),
                AssaultAction::FightOver { won: true, turns: 1 },
            ))
        };
        let repelled = play_day(DEFAULT_FROM, &content, None, &won).unwrap();
        assert_eq!(repelled.len(), ASSAULT_LEVELS as usize);
    }

    #[test]
    #[ignore = "daily: DAILY_DAYS whole days at full budget; the r4583000 band tests play the band levels at the tiny budget per pull request"]
    fn the_daily_sweep_walks_whole_days_at_full_budget_and_prints_the_report() {
        let content = content();
        let dates = dates(DEFAULT_FROM, DAILY_DAYS).unwrap();
        let (text, faults) = sweep(&dates, &[AI_SKILL_MAX], "full", None, "", &content);
        println!("{text}");
        assert!(faults.is_empty(), "{faults:?}");
    }
}
