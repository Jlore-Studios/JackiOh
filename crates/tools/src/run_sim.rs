//! `cargo jackioh run-sim --seeds N [--from N] [--skill S,…] [--budget tiny|full] [--curses ID,…]
//! [--artifacts ID,…] [--level N]` (#3065, #3481): whole dungeon runs on the catalog's content (§15), each
//! fight played to its end. Run `n` starts from the seed `run_seed("run-sim-{n}")`, the first
//! `RUN_STARTER_SIZE` starting cards, the `--curses`, the `--artifacts` (which count as unlocked) and the
//! `--level`'s unlocks: none, none and level 1 by default. An id the content lacks and a level past the
//! ladder are refused before any fight is played. Today the level changes only the slot check of the
//! starting cards, which are Commons and Rares, so the first ten fit every level. Each fight is
//! `run_fight_setup`'s game, its marked cards dealt Radiant and Buffed (R2702000), played by `play_match`
//! with the AI in both seats: the player's at R747000's skill (each `--skill`, full by default), the
//! enemy's at full skill, both at the budget. Its record must end clean (no throw, refusal, fallback or
//! caught search panic, and a result) and fold to its hash, and its report goes back through
//! `run_fight_report`. Every other choice (a node it may step onto, the artifact, a pack and what to
//! keep, a shop's buys and upgrades, a campfire, an event, a blessing) follows a seeded policy.
//!
//! For each skill it prints, by kind of fight, by act and by tier, the elites apart (R3586001), how many
//! were played and won, the HP they cost and their turns, then the max HP entering each boss, the rows reached, the fights on each run's
//! route (R3583001: 12 to 16 on any route to the final boss), the runs won, the score by term
//! (rows, wins, damage, healing, cards drawn, and what the boss percents and the curse percent add, each
//! mean and max: only a breakdown shows whether stalling pays), the score and the Embers, then the economy: the Coins and Tickets earned, the shops entered, the Coins and
//! Tickets held entering one, the cheapest shop card and the Remove, Radiant and buff prices charged
//! (§15.7, §15.8). The AI plays
//! the player's seat better or worse than a person would, so the table is a guide to tuning the strawmen
//! (§15.13), not a target. A lost fight is data; an unclean fight, a dead end of the policy
//! or a run file that does not fold to the run's end is a fault, which exits 1 and names the command
//! that plays its run again.
//!
//! The policy draws from a stream of its own, never the run's `rng_cursor`, so the run's log still
//! `run_fold`s to the same end; nothing reads a clock, and each run's results are kept in seed order, so
//! the output is the same on any thread count.
//!
//! A pull request's tests also hold the enemy tiers to wide win-rate bands (R3586000, R3586001). A whole
//! run dies by its first mini-boss at the tiny budget, so they play fights one at a time from the first
//! `BAND_SEEDS` runs' starts, the starting deck at full HP, and fail when tier 1 is no longer clearly
//! winnable, the final boss no longer clearly hard, or an elite no harder than its tier's normal fight. A
//! deliberate retune of `RUN_TIERS` or `RUN_ELITE_TIERS` changes the bands in the same patch.
//!
//! They also hold the economy to wide bands (`ECONOMY_RUNS` stub-won runs for the Coins, Tickets and shops,
//! `BAND_SEEDS` AI runs of `ECONOMY_AI_FIGHTS` fights for the score and Embers): a deliberate retune of
//! the prices, the drops or the score row changes them in the same patch. That test adds about 40 s to
//! `cargo test -p jackioh-tools --bin jackioh run_sim` (about 65 s in all, a debug build on four cores).
//!
//! ```text
//! cargo jackioh run-sim --seeds 8                              # eight runs at full skill and budget
//! cargo jackioh run-sim --seeds 4 --skill 1,0.5 --budget tiny  # a quick look at two skills
//! cargo jackioh run-sim --from 3 --seeds 1 --skill 0.5         # one run again
//! cargo jackioh run-sim --seeds 4 --curses temp-curse-2 --artifacts temp-thick-skin --level 3
//! cargo test -p jackioh-tools --bin jackioh run_sim                        # a pull request's
//! cargo test -p jackioh-tools --bin jackioh run_sim -- --ignored --nocapture   # the daily one
//! ```

use anyhow::bail;
use jackioh_ai::{
    AI_BUDGET, AI_SKILL_MAX, AI_SKILL_MIN, MatchConfig, MatchHooks, MatchSetup, SearchBudget, SeatController,
    play_match,
};
use jackioh_engine::config::PERCENT;
use jackioh_engine::{
    CreateGameOptions, PerPlayer, PerPlayerOpt, PlayerId, Rng, SetupKind, fold, hash_state,
    registered_catalog, validate_setup,
};
use jackioh_run::config::{
    RUN_ACTS, RUN_ELITE_TIERS, RUN_SLOT_LADDER, RUN_STARTER_CARDS, RUN_STARTER_SIZE, RUN_TIERS,
};
use jackioh_run::{
    RunAction, RunContent, RunFightSetup, RunFile, RunNodeKind, RunPassiveKind, RunPhase, RunScoreTerms,
    RunSetup, RunShopGood, RunState, RunUnlocks, run_catalog_content, run_check_setup, run_create,
    run_embers, run_enemy, run_fight_report, run_fight_setup, run_fold, run_next_cols, run_node, run_reduce,
    run_score, run_score_terms, run_seed, run_shop_prices,
};
use rayon::prelude::*;

use crate::arena::TEST_BUDGET;

/// The steps one run may take before it is a fault: a whole run with every fight won takes about a
/// hundred, so this fires only on a policy that never moves on.
const MAX_STEPS: usize = 2000;

/// The kinds of fight the table breaks down, each with its row's name (the kind's serialized name).
const KINDS: [(RunNodeKind, &str); 5] = [
    (RunNodeKind::Fight, "fight"),
    (RunNodeKind::Elite, "elite"),
    (RunNodeKind::Ghost, "ghost"),
    (RunNodeKind::MiniBoss, "miniBoss"),
    (RunNodeKind::Boss, "boss"),
];

/// A run's bosses: one at the end of each act (§15.5), the last the final boss.
const BOSSES: usize = RUN_ACTS;

/// A run's acts (§15.5): one table row each.
const ACTS: usize = RUN_ACTS;

#[derive(clap::Args, Debug)]
pub struct Args {
    /// How many runs to play for each skill.
    #[arg(long)]
    seeds: u32,
    /// The first run's number.
    #[arg(long, default_value_t = 0)]
    from: u32,
    /// R747000: the player's skills, one table each (full by default).
    #[arg(long, value_delimiter = ',', value_parser = parse_skill)]
    skill: Vec<f64>,
    /// Both seats' search budget: `tiny` is a test's, `full` the practice AI's.
    #[arg(long, value_parser = ["tiny", "full"], default_value = "full")]
    budget: String,
    /// The curses every run starts with, by id (#3481).
    #[arg(long, value_delimiter = ',')]
    curses: Vec<String>,
    /// The artifacts every run starts with, by id; they count as unlocked (#3481).
    #[arg(long, value_delimiter = ',')]
    artifacts: Vec<String>,
    /// The starting deck's level, from 1 (the first by default).
    #[arg(long)]
    level: Option<usize>,
}

/// What every run of a sweep starts with beyond its seed and starting cards: its curses, its artifacts
/// (counted as unlocked) and the level of its starting deck's slots.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Start {
    curses: Vec<String>,
    artifacts: Vec<String>,
    level: usize,
}

impl Default for Start {
    fn default() -> Self {
        Start {
            curses: Vec::new(),
            artifacts: Vec::new(),
            level: RunUnlocks::default().level,
        }
    }
}

impl Start {
    fn from_args(args: &Args) -> Start {
        Start {
            curses: args.curses.clone(),
            artifacts: args.artifacts.clone(),
            level: args.level.unwrap_or(Start::default().level),
        }
    }

    /// The flags that give this start back, each with its leading space; none for the default start.
    fn flags(&self) -> String {
        let mut out = String::new();
        if self.level != Start::default().level {
            out.push_str(&format!(" --level {}", self.level));
        }
        if !self.curses.is_empty() {
            out.push_str(&format!(" --curses {}", self.curses.join(",")));
        }
        if !self.artifacts.is_empty() {
            out.push_str(&format!(" --artifacts {}", self.artifacts.join(",")));
        }
        out
    }
}

/// R747000: a skill is in `AI_SKILL_MIN..=AI_SKILL_MAX`; a NaN is outside it.
fn parse_skill(text: &str) -> Result<f64, String> {
    let skill: f64 = text
        .parse()
        .map_err(|e: std::num::ParseFloatError| e.to_string())?;
    if !(AI_SKILL_MIN..=AI_SKILL_MAX).contains(&skill) {
        return Err(format!(
            "a skill is between {AI_SKILL_MIN} and {AI_SKILL_MAX}, not {skill}"
        ));
    }
    Ok(skill)
}

/// The search budget `--budget` names: `tiny` is the tests' (`arena.rs`'s `TEST_BUDGET`, which decides
/// clean at a fraction of the time), `full` the practice AI's.
fn budget_of(name: &str) -> SearchBudget {
    match name {
        "tiny" => TEST_BUDGET,
        _ => AI_BUDGET,
    }
}

/// The match a fight's setup plays: the human (P1) is the AI at `skill`, the enemy (P2) the AI at full
/// skill, both at `budget`, and the human's marks dealt (R2702000; an empty list stores nothing).
fn fight_config(setup: &RunFightSetup, skill: f64, budget: SearchBudget) -> MatchConfig {
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
                p1: Some(setup.human_marks.clone()),
                p2: None,
            }),
            ..MatchSetup::default()
        },
    }
}

/// The fight `state` stands on, played to its end, and the `FightOver` its record reports; or why the
/// fight is not clean.
fn play_fight(
    state: &RunState,
    content: &RunContent,
    skill: f64,
    budget: SearchBudget,
) -> Result<RunAction, String> {
    let setup = run_fight_setup(state, content).ok_or("the fight has no setup")?;
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
        let first = record
            .first_sim_panic
            .as_ref()
            .map(|panic| format!(", the first {}'s: {}", panic.seat.as_str(), panic.message))
            .unwrap_or_default();
        return Err(format!("the searches caught {} panics{first}", record.sim_panics));
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
    run_fight_report(&game, PlayerId::P1)
}

/// One index into a list of `len`, drawn from `rng` (0 for an empty list).
fn index(rng: &mut Rng, len: usize) -> usize {
    rng.int(i32::try_from(len).unwrap_or(i32::MAX)) as usize
}

/// The phase's name, for a fault.
fn phase_name(phase: &RunPhase) -> &'static str {
    match phase {
        RunPhase::Map => "map",
        RunPhase::Fight => "fight",
        RunPhase::Reward { .. } => "reward",
        RunPhase::Pack { .. } => "pack",
        RunPhase::ExtractCard => "extractCard",
        RunPhase::ExtractArtifact => "extractArtifact",
        RunPhase::Shop { .. } => "shop",
        RunPhase::Campfire => "campfire",
        RunPhase::Event { .. } => "event",
        RunPhase::Shrine { .. } => "shrine",
        RunPhase::Won => "won",
        RunPhase::Dead => "dead",
    }
}

/// The policy's choices at a phase other than a fight, most wanted first: a random node of those it may
/// step onto (R3583000: every node leads on, so this is never a dead end), then the first; the artifact; a random pack, then the first; a random `keep` of the pack, then its first; no
/// extraction; at a shop each unsold good in a random order, a random card made Radiant, a random card
/// Buffed, then leaving (each but leaving spends, so the shop ends); at a campfire more max HP, then
/// a random card made Radiant, then one Buffed; a random event option, then the last that costs
/// nothing; a random blessing, then none.
fn candidates(state: &RunState, content: &RunContent, rng: &mut Rng) -> Result<Vec<RunAction>, String> {
    let deck = state.deck.len();
    Ok(match &state.phase {
        RunPhase::Map => {
            let cols = run_next_cols(state);
            vec![
                RunAction::Go {
                    col: cols.get(index(rng, cols.len())).copied().unwrap_or(0),
                },
                RunAction::Go {
                    col: cols.first().copied().unwrap_or(0),
                },
            ]
        }
        RunPhase::Reward {
            artifact: Some(_), ..
        } => vec![
            RunAction::TakeArtifact { take: true },
            RunAction::TakeArtifact { take: false },
        ],
        RunPhase::Reward { packs, .. } => vec![
            RunAction::OpenPack {
                index: index(rng, packs.len()),
            },
            RunAction::OpenPack { index: 0 },
        ],
        RunPhase::Pack { cards, keep } => vec![
            RunAction::Keep {
                cards: rng.shuffle(cards).into_iter().take(*keep).collect(),
            },
            RunAction::Keep {
                cards: cards.iter().take(*keep).cloned().collect(),
            },
        ],
        RunPhase::ExtractCard | RunPhase::ExtractArtifact => vec![RunAction::Extract { index: None }],
        RunPhase::Shop { stock } => {
            let unsold: Vec<usize> = stock
                .iter()
                .enumerate()
                .filter(|(_, item)| !item.sold)
                .map(|(at, _)| at)
                .collect();
            let mut out: Vec<RunAction> = rng
                .shuffle(&unsold)
                .into_iter()
                .map(|index| RunAction::Buy { index })
                .collect();
            out.push(RunAction::MakeRadiant {
                index: index(rng, deck),
            });
            out.push(RunAction::Buff {
                index: index(rng, deck),
            });
            out.push(RunAction::Leave);
            out
        }
        RunPhase::Campfire => {
            let mut out = vec![RunAction::GainMaxHp];
            out.push(RunAction::MakeRadiant {
                index: index(rng, deck),
            });
            out.push(RunAction::Buff {
                index: index(rng, deck),
            });
            out
        }
        RunPhase::Event { id } => {
            let options = &content
                .events
                .iter()
                .find(|event| &event.id == id)
                .ok_or_else(|| format!("no event {id} in the content"))?
                .options;
            let mut out = vec![RunAction::Choose {
                index: index(rng, options.len()),
            }];
            out.extend(
                options
                    .iter()
                    .rposition(|option| option.max_hp >= 0 && option.coins >= 0 && option.tickets >= 0)
                    .map(|index| RunAction::Choose { index }),
            );
            out
        }
        RunPhase::Shrine { offers } => vec![
            RunAction::Bless {
                index: Some(index(rng, offers.len())),
            },
            RunAction::Bless { index: None },
        ],
        RunPhase::Fight | RunPhase::Won | RunPhase::Dead => {
            return Err(format!(
                "the policy has no choice at {}",
                phase_name(&state.phase)
            ));
        }
    })
}

/// The policy's choice at a phase other than a fight: the first candidate the run takes, with the run
/// after it.
fn choose(state: &RunState, content: &RunContent, rng: &mut Rng) -> Result<(RunAction, RunState), String> {
    let mut last = String::from("no choice");
    for action in candidates(state, content, rng)? {
        match run_reduce(state, content, &action) {
            Ok(next) => return Ok((action, next)),
            Err(error) => last = error,
        }
    }
    Err(format!(
        "a dead end at {}: the last refusal was {last}",
        phase_name(&state.phase)
    ))
}

/// One fight of a run: its kind, its act, its enemy's tier, the HP entering it, the HP leaving it (the
/// refilled max for a won fight, 0 for a lost one), the report's final HP, whether it was won, and its
/// turns.
#[derive(Clone, Debug, PartialEq, Eq)]
struct FightRow {
    kind: RunNodeKind,
    /// 0-based, `act_of`'s.
    act: usize,
    /// R3586000: its enemy's tier, from 1.
    tier: usize,
    hp_before: i32,
    hp_after: i32,
    final_hp: i32,
    won: bool,
    turns: i32,
}

/// The row the tables count for the fight `state` stands on, which `action` played to `next`.
fn fight_row(state: &RunState, action: &RunAction, next: &RunState) -> Result<FightRow, String> {
    let (final_hp, turns) = match action {
        RunAction::FightOver { hp, turns, .. } => (*hp, *turns),
        _ => (next.hp, 0),
    };
    Ok(FightRow {
        kind: run_node(state).ok_or("the fight stands on no node")?.kind,
        act: act_of(state),
        tier: run_enemy(state).map_or(0, |enemy| enemy.tier),
        hp_before: state.hp,
        hp_after: next.hp,
        final_hp,
        won: !matches!(next.phase, RunPhase::Dead),
        turns,
    })
}

/// One run as played: its file, where it ended, its fights and what went wrong (nothing, for a clean run).
#[derive(Debug)]
struct RunOutcome {
    file: RunFile,
    end: RunState,
    fights: Vec<FightRow>,
    faults: Vec<String>,
}

/// Run `n`'s start: its seed, the first `RUN_STARTER_SIZE` starting cards, `start`'s artifacts and curses,
/// and unlocks at `start`'s level with its artifacts among them (a starting artifact needs no unlock; a
/// relic that is not one needs it to start a run).
fn run_setup(n: u32, start: &Start) -> (RunSetup, RunUnlocks) {
    let setup = RunSetup {
        seed: run_seed(&format!("run-sim-{n}")),
        starter: RUN_STARTER_CARDS[..RUN_STARTER_SIZE]
            .iter()
            .map(|id| (*id).to_string())
            .collect(),
        artifacts: start.artifacts.clone(),
        curses: start.curses.clone(),
    };
    let unlocks = RunUnlocks {
        level: start.level,
        artifacts: start.artifacts.clone(),
        ..RunUnlocks::default()
    };
    (setup, unlocks)
}

/// Why `start` cannot begin a run on `content`, before any fight is played: a curse or an artifact the
/// content lacks, a level past the ladder, or what `run_check_setup` refuses of the whole start.
fn check_start(start: &Start, content: &RunContent) -> Result<(), String> {
    let ids = |ids: Vec<&str>| ids.join(", ");
    if let Some(id) = start
        .curses
        .iter()
        .find(|id| !content.curses.iter().any(|curse| &curse.id == *id))
    {
        return Err(format!(
            "{id} is no curse; the content has {}",
            ids(content.curses.iter().map(|curse| curse.id.as_str()).collect())
        ));
    }
    let relics: Vec<&str> = content
        .passives
        .iter()
        .filter(|passive| {
            matches!(passive.kind, RunPassiveKind::Combat | RunPassiveKind::Economic) && !passive.unobtainable
        })
        .map(|passive| passive.id.as_str())
        .collect();
    if let Some(id) = start.artifacts.iter().find(|id| !relics.contains(&id.as_str())) {
        return Err(format!(
            "{id} is no artifact a run may start with; the content has {}",
            ids(relics)
        ));
    }
    if !(1..=RUN_SLOT_LADDER.len()).contains(&start.level) {
        return Err(format!(
            "there is no level {}: the levels are 1 to {}",
            start.level,
            RUN_SLOT_LADDER.len()
        ));
    }
    let (setup, unlocks) = run_setup(0, start);
    run_check_setup(&setup, &unlocks, content)
}

/// The 0-based act `state` stands in: the mini-boss and boss rows before its row, each the last of an act
/// (§15.5); 0 before the first step. A boss's own row is in its act.
fn act_of(state: &RunState) -> usize {
    let Some(row) = state.row else {
        return 0;
    };
    state
        .map
        .iter()
        .take(row)
        .filter(|nodes| nodes.iter().any(|node| node.kind.is_boss()))
        .count()
}

/// Run `n` from `start`, played until it is won or lost, or until `max_fights` fights when given: each
/// fight by `fight`, every other choice by the policy on its own stream. A fault stops the run; a clean
/// run's file must fold to its end. Only a run `run_create` refuses is an `Err`.
fn play_run(
    n: u32,
    content: &RunContent,
    start: &Start,
    fight: &dyn Fn(&RunState) -> Result<RunAction, String>,
    max_fights: Option<usize>,
) -> Result<RunOutcome, String> {
    let (setup, unlocks) = run_setup(n, start);
    let mut state = run_create(&setup, &unlocks, content)?;
    let mut rng = Rng::new(&format!("run-sim-policy-{n}"), 0);
    let mut log: Vec<RunAction> = Vec::new();
    let mut fights: Vec<FightRow> = Vec::new();
    let mut faults: Vec<String> = Vec::new();
    while !matches!(state.phase, RunPhase::Won | RunPhase::Dead) {
        if max_fights.is_some_and(|max| fights.len() >= max) {
            break;
        }
        if log.len() >= MAX_STEPS {
            faults.push(format!(
                "run {n} ({}) row {:?}: past {MAX_STEPS} steps at {}",
                setup.seed,
                state.row,
                phase_name(&state.phase)
            ));
            break;
        }
        let stepped = if matches!(state.phase, RunPhase::Fight) {
            fight(&state).and_then(|action| {
                let next = run_reduce(&state, content, &action)?;
                fights.push(fight_row(&state, &action, &next)?);
                Ok((action, next))
            })
        } else {
            choose(&state, content, &mut rng)
        };
        match stepped {
            Ok((action, next)) => {
                log.push(action);
                state = next;
            }
            Err(error) => {
                faults.push(format!("run {n} ({}) row {:?}: {error}", setup.seed, state.row));
                break;
            }
        }
    }
    let mut outcome = RunOutcome {
        file: RunFile { setup, unlocks, log },
        end: state,
        fights,
        faults,
    };
    if outcome.faults.is_empty() {
        match run_fold(&outcome.file, content) {
            Ok(folded) if folded == outcome.end => {}
            Ok(_) => outcome.faults.push(format!(
                "run {n} ({}): the run file folds to another end",
                outcome.file.setup.seed
            )),
            Err(error) => outcome.faults.push(format!(
                "run {n} ({}): the run file does not fold: {error}",
                outcome.file.setup.seed
            )),
        }
    }
    Ok(outcome)
}

/// One group of fights' sums over the runs: a kind's, an act's or a tier's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct KindTally {
    fights: usize,
    won: usize,
    hp_lost: i64,
    turns: i64,
}

impl KindTally {
    fn add(&mut self, fight: &FightRow) {
        self.fights += 1;
        self.won += usize::from(fight.won);
        // A won fight refills to a higher max, so the HP leaving it is no loss: what the fight
        // cost is the HP entering it minus the report's final HP. A lost fight loses all of it.
        self.hp_lost += if fight.won {
            i64::from(fight.hp_before - fight.final_hp)
        } else {
            i64::from(fight.hp_before - fight.hp_after)
        };
        self.turns += i64::from(fight.turns);
    }
}

/// One run's score by term (§15.10, `run_score`'s): what each part of a run adds, the percents' share
/// beside the sums. The terms add up to the run's score.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ScoreParts {
    rows: i64,
    wins: i64,
    damage: i64,
    healing: i64,
    drawn: i64,
    /// What the mini-bosses' and the final boss's percents add.
    bosses: i64,
    /// What the curse points' percent adds.
    curses: i64,
}

impl ScoreParts {
    /// Each term with its label, in the order they print.
    fn terms(&self) -> [(&'static str, i64); 7] {
        [
            ("rows", self.rows),
            ("wins", self.wins),
            ("damage", self.damage),
            ("healing", self.healing),
            ("cards drawn", self.drawn),
            ("boss percents", self.bosses),
            ("curse percent", self.curses),
        ]
    }
}

/// `end`'s score by term. The five sums are `run_score`'s own; the percents' shares are what `run_score`
/// adds to them, uncursed first and then cursed, so its percent rules are not repeated here.
fn score_parts(end: &RunState, content: &RunContent) -> ScoreParts {
    let terms = run_score_terms(end);
    let RunScoreTerms {
        rows,
        wins,
        damage,
        healing,
        drawn,
    } = terms;
    let base = terms.sum();
    let uncursed = run_score(
        &RunState {
            curses: Vec::new(),
            ..end.clone()
        },
        content,
    );
    ScoreParts {
        rows,
        wins,
        damage,
        healing,
        drawn,
        bosses: uncursed - base,
        curses: run_score(end, content) - uncursed,
    }
}

/// One shop as a run first stands in it (§15.8): what it holds and what it asks, the prices after the
/// relic discount, as `run_shop_prices` gives them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ShopVisit {
    coins: i32,
    tickets: i32,
    /// The least a Card in the stock costs, `None` for a stock of artifacts only.
    cheapest_card: Option<i32>,
    remove: i32,
    radiant: i32,
    buff: i32,
}

/// Every shop `file`'s log enters, in order, read by folding the log again. A run `run_create` or
/// `run_reduce` refuses ends the list there (a faulty run is reported by its own faults).
fn shop_visits(file: &RunFile, content: &RunContent) -> Vec<ShopVisit> {
    let Ok(mut state) = run_create(&file.setup, &file.unlocks, content) else {
        return Vec::new();
    };
    let mut visits = Vec::new();
    for action in &file.log {
        let Ok(next) = run_reduce(&state, content, action) else {
            break;
        };
        let entered =
            matches!(next.phase, RunPhase::Shop { .. }) && !matches!(state.phase, RunPhase::Shop { .. });
        if entered {
            let prices = run_shop_prices(&next, content);
            let stock = match &next.phase {
                RunPhase::Shop { stock } => stock.as_slice(),
                _ => &[],
            };
            visits.push(ShopVisit {
                coins: next.coins,
                tickets: next.tickets,
                cheapest_card: stock
                    .iter()
                    .zip(&prices.stock_coins)
                    .filter(|(item, _)| matches!(item.good, RunShopGood::Card { .. }))
                    .map(|(_, price)| *price)
                    .min(),
                remove: prices.remove_coins,
                radiant: prices.radiant_tickets,
                buff: prices.buff_tickets,
            });
        }
        state = next;
    }
    visits
}

/// What the table prints, summed over one skill's runs.
#[derive(Debug, Default, PartialEq, Eq)]
struct Tally {
    runs: usize,
    runs_won: usize,
    /// By `KINDS`.
    kinds: [KindTally; KINDS.len()],
    /// By act, from the first.
    acts: [KindTally; ACTS],
    /// R3586001: every fight but an elite by its tier, from tier 1 (`RUN_TIERS`' rows).
    tiers: [KindTally; RUN_TIERS.len()],
    /// R3586001: the elites by their tier, in `RUN_ELITE_TIERS`' order.
    elite_tiers: [KindTally; RUN_ELITE_TIERS.len()],
    /// The max HP entering each boss, summed, and how many runs reached it.
    boss_hp: [(i64, usize); BOSSES],
    /// Each run's rows reached.
    rows: Vec<usize>,
    /// Each run's fights played on its route (R3583001).
    route_fights: Vec<usize>,
    /// The most rows a run's map holds.
    map_rows: usize,
    /// Each run's score at its end.
    scores: Vec<i64>,
    /// Each run's score by term.
    parts: Vec<ScoreParts>,
    /// Each run's Coins earned (§15.7).
    coins: Vec<i64>,
    /// Each run's Tickets earned.
    tickets: Vec<i64>,
    /// Every shop every run entered (§15.8).
    shops: Vec<ShopVisit>,
    /// The runs that entered at least one shop.
    shop_runs: usize,
}

fn tally(outcomes: &[RunOutcome], content: &RunContent) -> Tally {
    let mut out = Tally {
        runs: outcomes.len(),
        ..Tally::default()
    };
    for outcome in outcomes {
        let mut bosses = 0;
        for fight in &outcome.fights {
            if let Some(at) = KINDS.iter().position(|(kind, _)| *kind == fight.kind) {
                out.kinds[at].add(fight);
            }
            if let Some(sums) = out.acts.get_mut(fight.act) {
                sums.add(fight);
            }
            // R3586001: each fight under the row it played, an elite's of its own table.
            let by_tier = if fight.kind == RunNodeKind::Elite {
                RUN_ELITE_TIERS
                    .iter()
                    .position(|(tier, _)| *tier == fight.tier)
                    .and_then(|at| out.elite_tiers.get_mut(at))
            } else {
                fight.tier.checked_sub(1).and_then(|at| out.tiers.get_mut(at))
            };
            if let Some(sums) = by_tier {
                sums.add(fight);
            }
            if fight.kind.is_boss() {
                if let Some((hp, reached)) = out.boss_hp.get_mut(bosses) {
                    *hp += i64::from(fight.hp_before);
                    *reached += 1;
                }
                bosses += 1;
            }
        }
        out.runs_won += usize::from(matches!(outcome.end.phase, RunPhase::Won));
        out.rows.push(outcome.end.row.map_or(0, |r| r + 1));
        out.route_fights.push(outcome.fights.len());
        out.map_rows = out.map_rows.max(outcome.end.map.len());
        out.scores.push(run_score(&outcome.end, content));
        out.parts.push(score_parts(&outcome.end, content));
        out.coins.push(i64::from(outcome.end.stats.coins_earned));
        out.tickets.push(i64::from(outcome.end.stats.tickets_earned));
        let visits = shop_visits(&outcome.file, content);
        out.shop_runs += usize::from(!visits.is_empty());
        out.shops.extend(visits);
    }
    out
}

/// `sum / n` to one decimal, or `-` for none.
fn mean(sum: i64, n: usize) -> String {
    if n == 0 {
        "-".to_string()
    } else {
        format!("{:.1}", sum as f64 / n as f64)
    }
}

/// A list's mean and its max, or `-` for none.
fn mean_and_max(values: &[i64]) -> String {
    let max = values
        .iter()
        .max()
        .map_or_else(|| "-".to_string(), i64::to_string);
    format!("mean {}, max {max}", mean(values.iter().sum(), values.len()))
}

/// A table's header line, its first column named `first`.
fn table_header(first: &str) -> String {
    format!(
        "{:<10}{:>8}{:>6}{:>7}{:>9}{:>7}\n",
        first, "fights", "won", "win %", "HP lost", "turns"
    )
}

/// A table's line for the group of fights `name` names.
fn table_line(name: &str, sums: &KindTally) -> String {
    format!(
        "{:<10}{:>8}{:>6}{:>7}{:>9}{:>7}\n",
        name,
        sums.fights,
        sums.won,
        mean(sums.won as i64 * i64::from(PERCENT), sums.fights),
        mean(sums.hp_lost, sums.fights),
        mean(sums.turns, sums.fights)
    )
}

impl Tally {
    fn render(&self, header: &str) -> String {
        let mut out = format!("{header}\n");
        out.push_str(&table_header("kind"));
        for ((_, name), sums) in KINDS.iter().zip(&self.kinds) {
            out.push_str(&table_line(name, sums));
        }
        out.push('\n');
        out.push_str(&table_header("act"));
        for (at, sums) in self.acts.iter().enumerate() {
            out.push_str(&table_line(&(at + 1).to_string(), sums));
        }
        out.push('\n');
        out.push_str(&table_header("tier"));
        for (at, sums) in self.tiers.iter().enumerate() {
            out.push_str(&table_line(&format!("tier {}", at + 1), sums));
        }
        for ((tier, _), sums) in RUN_ELITE_TIERS.iter().zip(&self.elite_tiers) {
            out.push_str(&table_line(&format!("elite {tier}"), sums));
        }
        out.push_str("(HP lost is the HP entering a fight minus the fight's final HP, so a won fight's refill is no loss and a lost fight loses all of it.)\n");
        for (at, (hp, reached)) in self.boss_hp.iter().enumerate() {
            out.push_str(&format!(
                "max HP entering boss {}: mean {} (n {reached})\n",
                at + 1,
                mean(*hp, *reached)
            ));
        }
        let rows: Vec<i64> = self.rows.iter().map(|&rows| rows as i64).collect();
        out.push_str(&format!(
            "rows reached: {} of {}\n",
            mean_and_max(&rows),
            self.map_rows
        ));
        let route: Vec<i64> = self.route_fights.iter().map(|&fights| fights as i64).collect();
        out.push_str(&format!("fights on the route: {}\n", mean_and_max(&route)));
        out.push_str(&format!("runs won: {} of {}\n", self.runs_won, self.runs));
        for (at, (label, _)) in ScoreParts::default().terms().iter().enumerate() {
            let values: Vec<i64> = self.parts.iter().map(|parts| parts.terms()[at].1).collect();
            out.push_str(&format!("score from {label}: {}\n", mean_and_max(&values)));
        }
        let embers: Vec<i64> = self.scores.iter().map(|&score| run_embers(score)).collect();
        out.push_str(&format!("score: {}\n", mean_and_max(&self.scores)));
        out.push_str(&format!("Embers: {}\n", mean_and_max(&embers)));
        out.push_str(&format!("coins earned: {}\n", mean_and_max(&self.coins)));
        out.push_str(&format!("tickets earned: {}\n", mean_and_max(&self.tickets)));
        out.push_str(&format!(
            "shops: {} visits in {} of {} runs\n",
            self.shops.len(),
            self.shop_runs,
            self.runs
        ));
        let shop_mean = |value: &dyn Fn(&ShopVisit) -> Option<i64>| {
            let values: Vec<i64> = self.shops.iter().filter_map(value).collect();
            mean(values.iter().sum(), values.len())
        };
        out.push_str(&format!(
            "coins held entering a shop: mean {} (n {})\n",
            shop_mean(&|shop| Some(i64::from(shop.coins))),
            self.shops.len()
        ));
        out.push_str(&format!(
            "cheapest shop card: mean {}\n",
            shop_mean(&|shop| shop.cheapest_card.map(i64::from))
        ));
        out.push_str(&format!(
            "tickets held entering a shop: mean {}\n",
            shop_mean(&|shop| Some(i64::from(shop.tickets)))
        ));
        let first = self.shops.first();
        let price = |value: &dyn Fn(&ShopVisit) -> i32| {
            first.map_or_else(|| "-".to_string(), |shop| value(shop).to_string())
        };
        out.push_str(&format!(
            "prices: remove {}, Radiant {} Tickets, buff {} Tickets\n",
            price(&|shop| shop.remove),
            price(&|shop| shop.radiant),
            price(&|shop| shop.buff)
        ));
        out
    }
}

/// `seeds` runs from `from` at each skill and `budget`, each from `start`: one table each, in order, and
/// every fault, each with the command that plays its run again.
fn sweep(
    seeds: u32,
    from: u32,
    skills: &[f64],
    budget: &str,
    start: &Start,
    content: &RunContent,
) -> (String, Vec<String>) {
    let search = budget_of(budget);
    let flags = start.flags();
    let mut text = String::new();
    let mut faults: Vec<String> = Vec::new();
    for &skill in skills {
        let runs: Vec<(u32, Result<RunOutcome, String>)> = (from..from.saturating_add(seeds))
            .into_par_iter()
            .map(|n| {
                let fight = |state: &RunState| play_fight(state, content, skill, search);
                (n, play_run(n, content, start, &fight, None))
            })
            .collect();
        let again = |n: u32| {
            format!(
                " (again: cargo jackioh run-sim --from {n} --seeds 1 --skill {skill} --budget {budget}{flags})"
            )
        };
        let mut outcomes: Vec<RunOutcome> = Vec::new();
        for (n, run) in runs {
            match run {
                Ok(outcome) => {
                    faults.extend(outcome.faults.iter().map(|fault| format!("{fault}{}", again(n))));
                    outcomes.push(outcome);
                }
                Err(error) => faults.push(format!("run {n}: no run: {error}{}", again(n))),
            }
        }
        let header = format!("run-sim: {seeds} runs from {from}, budget {budget}, skill {skill}{flags}");
        text.push_str(&tally(&outcomes, content).render(&header));
        text.push('\n');
    }
    (text, faults)
}

pub fn run(args: Args) -> anyhow::Result<()> {
    jackioh_cards::register_all();
    let content = run_catalog_content(registered_catalog());
    let start = Start::from_args(&args);
    check_start(&start, &content).map_err(|e| anyhow::anyhow!("run-sim: {e}"))?;
    let skills = if args.skill.is_empty() {
        vec![AI_SKILL_MAX]
    } else {
        args.skill
    };
    let (text, faults) = sweep(args.seeds, args.from, &skills, &args.budget, &start, &content);
    print!("{text}");
    for fault in &faults {
        eprintln!("{fault}");
    }
    if !faults.is_empty() {
        bail!("run-sim: {} faults", faults.len());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use jackioh_engine::DeckMark;
    use jackioh_run::config::{
        RUN_ACT_ROWS, RUN_ACT_TIERS, RUN_BUFF_TICKETS, RUN_RADIANT_TICKETS, RUN_ROUTE_FIGHTS_MAX,
        RUN_ROUTE_FIGHTS_MIN, RUN_STARTER_ARTIFACT_POINTS, RUN_STARTER_ARTIFACTS_MAX,
    };
    use jackioh_run::{RunPassiveDef, RunStats, run_artifact_points, run_curse_points};

    use super::*;

    /// The fights the per-PR run plays at `TEST_BUDGET` before it stops.
    const PR_FIGHTS: usize = 3;
    /// The runs the daily sweep plays for each skill, at full budget.
    const DAILY_SEEDS: u32 = 4;
    /// The skills the daily sweep plays (R747000).
    const DAILY_SKILLS: [f64; 2] = [AI_SKILL_MAX, 0.5];
    /// The runs the policy walks to their end with stub fights.
    const POLICY_SEEDS: u32 = 10;

    fn content() -> RunContent {
        jackioh_cards::register_all();
        run_catalog_content(registered_catalog())
    }

    #[test]
    fn a_runs_first_fights_end_clean_and_its_file_folds_to_its_end() {
        let content = content();
        let fight = |state: &RunState| play_fight(state, &content, AI_SKILL_MAX, TEST_BUDGET);
        let outcome =
            play_run(0, &content, &Start::default(), &fight, Some(PR_FIGHTS)).expect("run 0 is created");
        assert!(outcome.faults.is_empty(), "{:#?}", outcome.faults);
        assert!(
            outcome.fights.len() == PR_FIGHTS || outcome.end.phase == RunPhase::Dead,
            "{} fights, then {}",
            outcome.fights.len(),
            phase_name(&outcome.end.phase)
        );
        assert_eq!(run_fold(&outcome.file, &content), Ok(outcome.end.clone()));
    }

    #[test]
    fn a_fights_marks_reach_its_game_and_it_ends_clean() {
        let content = content();
        let (setup, unlocks) = run_setup(0, &Start::default());
        let start = run_create(&setup, &unlocks, &content).expect("run 0 is created");
        let col = start.map[0]
            .iter()
            .position(|node| node.kind == RunNodeKind::Fight)
            .unwrap_or(0);
        let mut state = run_reduce(&start, &content, &RunAction::Go { col }).expect("row 0 is a fight row");
        state.deck[0].radiant = true;
        state.deck[1].buffs = 2;

        let marks = vec![
            DeckMark {
                index: 0,
                radiant: true,
                upgrades: 0,
            },
            DeckMark {
                index: 1,
                radiant: false,
                upgrades: 2,
            },
        ];
        let setup = run_fight_setup(&state, &content).expect("the fight has a setup");
        assert_eq!(setup.human_marks, marks);
        assert_eq!(
            fight_config(&setup, AI_SKILL_MAX, TEST_BUDGET).setup.deck_marks,
            Some(PerPlayerOpt {
                p1: Some(marks),
                p2: None,
            })
        );
        let report = play_fight(&state, &content, AI_SKILL_MAX, TEST_BUDGET);
        assert!(matches!(report, Ok(RunAction::FightOver { .. })), "{report:?}");
    }

    #[test]
    fn the_policy_walks_every_run_to_its_end_and_the_log_folds() {
        let content = content();
        let won = |state: &RunState| -> Result<RunAction, String> {
            Ok(RunAction::FightOver {
                won: true,
                hp: state.hp,
                damage_dealt: 0,
                healing_done: 0,
                cards_drawn: 0,
                turns: 1,
            })
        };
        for n in 0..POLICY_SEEDS {
            let outcome = play_run(n, &content, &Start::default(), &won, None).expect("the run is created");
            assert!(outcome.faults.is_empty(), "run {n}: {:#?}", outcome.faults);
            assert_eq!(outcome.end.phase, RunPhase::Won, "run {n}");
            assert!(
                route_holds_its_bounds(outcome.fights.len()),
                "run {n}: {} fights on the route",
                outcome.fights.len()
            );
            assert_eq!(
                run_fold(&outcome.file, &content),
                Ok(outcome.end.clone()),
                "run {n}"
            );
        }
    }

    /// R3583001: whether a won run's fights, the bosses counted, are within the bounds of every route.
    fn route_holds_its_bounds(fights: usize) -> bool {
        i32::try_from(fights)
            .is_ok_and(|fights| (RUN_ROUTE_FIGHTS_MIN..=RUN_ROUTE_FIGHTS_MAX).contains(&fights))
    }

    /// A won fight for every fight a stub run is asked to play.
    fn won(state: &RunState) -> Result<RunAction, String> {
        Ok(RunAction::FightOver {
            won: true,
            hp: state.hp,
            damage_dealt: 0,
            healing_done: 0,
            cards_drawn: 0,
            turns: 1,
        })
    }

    /// The first curse, and the first Combat artifact any start may take, of the content: a start that
    /// sets all three flags, with its ids read from the content so the designer's lists may change.
    fn full_start(content: &RunContent) -> Start {
        let curse = content
            .curses
            .first()
            .expect("the content has a curse")
            .id
            .clone();
        let artifact = content
            .passives
            .iter()
            .find(|p| p.kind == RunPassiveKind::Combat && p.starter && !p.unobtainable)
            .expect("the content has a starting Combat artifact")
            .id
            .clone();
        Start {
            curses: vec![curse],
            artifacts: vec![artifact],
            level: 3,
        }
    }

    #[test]
    fn the_act_is_the_count_of_boss_rows_before_a_row() {
        let content = content();
        let (setup, unlocks) = run_setup(0, &Start::default());
        let start = run_create(&setup, &unlocks, &content).expect("run 0 is created");
        // §15.5: an act is its rows of nodes, then its boss's row.
        let spans: Vec<usize> = vec![RUN_ACT_ROWS + 1; ACTS];
        assert_eq!(start.map.len(), spans.iter().sum::<usize>());
        let mut ends = Vec::new();
        let mut end = 0;
        for span in &spans {
            end += span;
            ends.push(end - 1);
        }
        for &last in &ends {
            assert!(
                start.map[last].len() == 1 && start.map[last][0].kind.is_boss(),
                "row {last} is no boss row"
            );
        }
        assert_eq!(
            start.map.iter().filter(|nodes| nodes[0].kind.is_boss()).count(),
            ACTS
        );
        assert_eq!(act_of(&start), 0, "before the first step");
        for row in 0..start.map.len() {
            let at = RunState {
                row: Some(row),
                ..start.clone()
            };
            let expected = ends.iter().filter(|&&last| last < row).count();
            assert_eq!(act_of(&at), expected, "row {row}");
        }
    }

    #[test]
    fn a_run_with_a_curse_an_artifact_and_a_level_folds_to_its_end() {
        let content = content();
        let start = full_start(&content);
        for n in 0..3 {
            let outcome = play_run(n, &content, &start, &won, None).expect("the run is created");
            assert!(outcome.faults.is_empty(), "run {n}: {:#?}", outcome.faults);
            let (file, end) = (&outcome.file, &outcome.end);
            assert_eq!(end.phase, RunPhase::Won, "run {n}");
            assert_eq!(run_fold(file, &content).as_ref(), Ok(end), "run {n}");
            assert_eq!(file.setup.curses, start.curses);
            assert_eq!(file.setup.artifacts, start.artifacts);
            assert_eq!(file.unlocks.level, 3);
            assert_eq!(file.unlocks.artifacts, start.artifacts);
            assert_eq!(end.curses, start.curses);
            assert!(run_curse_points(end, &content) > 0);
            assert!(
                end.artifacts.iter().any(|a| a.id == start.artifacts[0]),
                "run {n}"
            );
            for act in 0..ACTS {
                let played: Vec<&FightRow> = outcome.fights.iter().filter(|fight| fight.act == act).collect();
                assert!(
                    played.last().is_some_and(|fight| fight.kind.is_boss()),
                    "run {n} act {act} ends at its boss"
                );
            }
            assert!(
                route_holds_its_bounds(outcome.fights.len()),
                "run {n}: {} fights on the route",
                outcome.fights.len()
            );
            assert!(outcome.fights.iter().all(|fight| fight.act < ACTS), "run {n}");
        }
    }

    #[test]
    fn a_cursed_runs_first_fight_ends_clean() {
        let content = content();
        let start = full_start(&content);
        let fight = |state: &RunState| play_fight(state, &content, AI_SKILL_MAX, TEST_BUDGET);
        let outcome = play_run(0, &content, &start, &fight, Some(1)).expect("run 0 is created");
        assert!(outcome.faults.is_empty(), "{:#?}", outcome.faults);
        assert_eq!(run_fold(&outcome.file, &content), Ok(outcome.end.clone()));
    }

    #[test]
    fn check_start_refuses_what_the_content_lacks() {
        let mut content = content();
        // An enemy's artifact, which no start may take: the placeholder content holds none since the
        // Presence artifacts retired (R3586001).
        content.passives.push(RunPassiveDef {
            id: "test-enemy-banner".into(),
            name: "Test Enemy Banner".into(),
            kind: RunPassiveKind::Enemy,
            text: "An enemy's own.".into(),
            tier: None,
            effect: None,
            fight_effect: None,
            starter: false,
            unobtainable: false,
            god: None,
        });
        let content = content;
        let refused = |start: &Start| check_start(start, &content).expect_err("the start is refused");
        let with = |edit: &dyn Fn(&mut Start)| {
            let mut start = Start::default();
            edit(&mut start);
            start
        };
        assert_eq!(check_start(&Start::default(), &content), Ok(()));
        assert_eq!(check_start(&full_start(&content), &content), Ok(()));
        assert!(refused(&with(&|s| s.curses = vec!["no-such-curse".into()])).contains("no-such-curse"));
        assert!(refused(&with(&|s| s.artifacts = vec!["no-such-relic".into()])).contains("no-such-relic"));
        let enemy = content
            .passives
            .iter()
            .find(|p| p.kind == RunPassiveKind::Enemy)
            .expect("the content has an enemy artifact")
            .id
            .clone();
        assert!(refused(&with(&|s| s.artifacts = vec![enemy.clone()])).contains(&enemy));
        for level in [0, RUN_SLOT_LADDER.len() + 1] {
            assert!(
                refused(&with(&|s| s.level = level)).contains(&format!("level {level}")),
                "level {level}"
            );
        }
        for level in 1..=RUN_SLOT_LADDER.len() {
            assert_eq!(
                check_start(&with(&|s| s.level = level), &content),
                Ok(()),
                "level {level}"
            );
        }
        let relics: Vec<String> = content
            .passives
            .iter()
            .filter(|p| {
                matches!(p.kind, RunPassiveKind::Combat | RunPassiveKind::Economic) && !p.unobtainable
            })
            .map(|p| p.id.clone())
            .collect();
        let too_many: Vec<String> = relics
            .iter()
            .take(RUN_STARTER_ARTIFACTS_MAX + 1)
            .cloned()
            .collect();
        assert_eq!(too_many.len(), RUN_STARTER_ARTIFACTS_MAX + 1);
        assert!(check_start(&with(&|s| s.artifacts = too_many.clone()), &content).is_err());
        // A relic that is no starter is unlocked by naming it.
        let unlocked = content
            .passives
            .iter()
            .find(|p| {
                matches!(p.kind, RunPassiveKind::Combat | RunPassiveKind::Economic)
                    && !p.starter
                    && !p.unobtainable
                    && run_artifact_points(&content, &p.id) <= RUN_STARTER_ARTIFACT_POINTS
            })
            .expect("the content has a relic that is no starter")
            .id
            .clone();
        let start = with(&|s| s.artifacts = vec![unlocked.clone()]);
        assert_eq!(check_start(&start, &content), Ok(()));
        assert_eq!(run_setup(0, &start).1.artifacts, [unlocked]);
    }

    #[test]
    fn a_start_prints_the_flags_that_give_it_back() {
        assert_eq!(Start::default().flags(), "");
        let start = Start {
            curses: vec!["a".into(), "b".into()],
            artifacts: vec!["x".into()],
            level: 2,
        };
        assert_eq!(start.flags(), " --level 2 --curses a,b --artifacts x");
    }

    /// A fight entered at `hp_before`, leaving at `hp_after` (the refilled max for a won fight,
    /// 0 for a lost one), whose report ended at `final_hp`.
    fn row(
        kind: RunNodeKind,
        hp_before: i32,
        hp_after: i32,
        final_hp: i32,
        won: bool,
        turns: i32,
    ) -> FightRow {
        FightRow {
            kind,
            act: 0,
            tier: 1,
            hp_before,
            hp_after,
            final_hp,
            won,
            turns,
        }
    }

    #[test]
    fn the_tally_counts_each_kind_and_prints_a_dash_for_none() {
        let content = content();
        let (setup, unlocks) = run_setup(0, &Start::default());
        let start = run_create(&setup, &unlocks, &content).expect("run 0 is created");
        let file = RunFile {
            setup,
            unlocks,
            log: Vec::new(),
        };
        let lost = RunState {
            row: Some(2),
            hp: 0,
            phase: RunPhase::Dead,
            ..start.clone()
        };
        let won = RunState {
            row: Some(4),
            phase: RunPhase::Won,
            ..start.clone()
        };
        let outcomes = [
            RunOutcome {
                file: file.clone(),
                end: lost.clone(),
                fights: vec![
                    row(RunNodeKind::Fight, 30, 35, 25, true, 10),
                    row(RunNodeKind::Elite, 25, 0, 0, false, 14),
                ],
                faults: Vec::new(),
            },
            RunOutcome {
                file,
                end: won.clone(),
                fights: vec![
                    row(RunNodeKind::Fight, 30, 35, 30, true, 8),
                    row(RunNodeKind::MiniBoss, 22, 27, 12, true, 20),
                ],
                faults: Vec::new(),
            },
        ];
        let sums = tally(&outcomes, &content);
        let of = |kind: RunNodeKind| sums.kinds[KINDS.iter().position(|(k, _)| *k == kind).unwrap()];
        assert_eq!(
            of(RunNodeKind::Fight),
            KindTally {
                fights: 2,
                won: 2,
                hp_lost: 5,
                turns: 18
            }
        );
        assert_eq!(
            of(RunNodeKind::Elite),
            KindTally {
                fights: 1,
                won: 0,
                hp_lost: 25,
                turns: 14
            }
        );
        assert_eq!(
            of(RunNodeKind::MiniBoss),
            KindTally {
                fights: 1,
                won: 1,
                hp_lost: 10,
                turns: 20
            }
        );
        assert_eq!(of(RunNodeKind::Ghost), KindTally::default());
        assert_eq!(of(RunNodeKind::Boss), KindTally::default());
        assert_eq!(sums.boss_hp[0], (22, 1));
        assert_eq!(sums.boss_hp[1], (0, 0));
        assert_eq!(sums.runs, 2);
        assert_eq!(sums.runs_won, 1);
        assert_eq!(sums.rows, [3, 5]);
        assert_eq!(sums.route_fights, [2, 2]);
        assert_eq!(sums.map_rows, start.map.len());
        assert_eq!(
            sums.scores,
            [run_score(&lost, &content), run_score(&won, &content)]
        );

        let text = sums.render("header");
        let line = |name: &str| {
            text.lines()
                .find(|line| line.split_whitespace().next() == Some(name))
                .unwrap_or_else(|| panic!("no {name} line in:\n{text}"))
                .to_string()
        };
        assert!(line("ghost").ends_with('-'), "{text}");
        assert_eq!(
            line("fight").split_whitespace().collect::<Vec<_>>(),
            ["fight", "2", "2", "100.0", "2.5", "9.0"]
        );
        assert!(text.contains("max HP entering boss 1: mean 22.0 (n 1)"), "{text}");
        assert!(text.contains("max HP entering boss 2: mean - (n 0)"), "{text}");
        assert!(text.contains("fights on the route: mean 2.0, max 2\n"), "{text}");
        assert!(text.contains("runs won: 1 of 2"), "{text}");
    }

    #[test]
    fn the_tally_puts_each_mini_boss_and_the_final_boss_in_act_order() {
        let content = content();
        let (setup, unlocks) = run_setup(0, &Start::default());
        let start = run_create(&setup, &unlocks, &content).expect("run 0 is created");
        let outcomes = [RunOutcome {
            file: RunFile {
                setup,
                unlocks,
                log: Vec::new(),
            },
            end: RunState {
                phase: RunPhase::Won,
                ..start
            },
            fights: vec![
                row(RunNodeKind::Fight, 30, 35, 28, true, 8),
                row(RunNodeKind::MiniBoss, 28, 33, 20, true, 18),
                row(RunNodeKind::Elite, 20, 25, 16, true, 12),
                row(RunNodeKind::MiniBoss, 16, 21, 12, true, 16),
                row(RunNodeKind::Boss, 12, 17, 5, true, 22),
            ],
            faults: Vec::new(),
        }];
        let sums = tally(&outcomes, &content);
        assert_eq!(sums.boss_hp, [(28, 1), (16, 1), (12, 1)]);
    }

    fn in_act(act: usize, row: FightRow) -> FightRow {
        FightRow { act, ..row }
    }

    fn at_tier(tier: usize, row: FightRow) -> FightRow {
        FightRow { tier, ..row }
    }

    #[test]
    fn r3586001_the_tally_splits_normal_and_elite_fights_by_tier() {
        // R3586001: every fight but an elite counts under its tier's row, an elite under its own
        // table's, ghosts and bosses among the first; a tier its table lacks counts under none.
        let content = content();
        let (setup, unlocks) = run_setup(0, &Start::default());
        let start = run_create(&setup, &unlocks, &content).expect("run 0 is created");
        let boss_tier = RUN_ACT_TIERS + 1;
        let outcomes = [RunOutcome {
            file: RunFile {
                setup,
                unlocks,
                log: Vec::new(),
            },
            end: RunState {
                phase: RunPhase::Dead,
                ..start
            },
            fights: vec![
                at_tier(1, row(RunNodeKind::Fight, 30, 35, 28, true, 8)),
                at_tier(1, row(RunNodeKind::Ghost, 35, 40, 32, true, 8)),
                at_tier(1, row(RunNodeKind::Elite, 40, 0, 0, false, 12)),
                at_tier(boss_tier, row(RunNodeKind::MiniBoss, 40, 45, 30, true, 16)),
                at_tier(0, row(RunNodeKind::Fight, 30, 35, 28, true, 8)),
                at_tier(boss_tier, row(RunNodeKind::Elite, 30, 35, 28, true, 8)),
            ],
            faults: Vec::new(),
        }];
        let sums = tally(&outcomes, &content);
        let sums_of = |fights, won, hp_lost, turns| KindTally {
            fights,
            won,
            hp_lost,
            turns,
        };
        assert_eq!(sums.tiers[0], sums_of(2, 2, 5, 16));
        assert_eq!(sums.tiers[boss_tier - 1], sums_of(1, 1, 10, 16));
        assert_eq!(sums.elite_tiers[0], sums_of(1, 0, 40, 12));
        let counted = |tally: &[KindTally]| tally.iter().map(|sums| sums.fights).sum::<usize>();
        assert_eq!(counted(&sums.tiers), 3, "tier 0 counts nowhere");
        assert_eq!(
            counted(&sums.elite_tiers),
            1,
            "an elite at a boss's tier counts nowhere"
        );

        let text = sums.render("header");
        // The tier table's lines, after the kind table's own `elite` line.
        let table = &text[text.find("\ntier ").expect("the tier table")..];
        let find = |first: &str, second: &str| {
            table
                .lines()
                .map(|line| line.split_whitespace().collect::<Vec<_>>())
                .find(|words| words.len() > 1 && words[0] == first && words[1] == second)
        };
        let words = |first: &str, second: &str| {
            find(first, second).unwrap_or_else(|| panic!("no {first} {second} line in:\n{text}"))
        };
        assert_eq!(words("tier", "1"), ["tier", "1", "2", "2", "100.0", "2.5", "8.0"]);
        assert_eq!(
            words("tier", &boss_tier.to_string()),
            ["tier", "5", "1", "1", "100.0", "10.0", "16.0"]
        );
        assert_eq!(words("tier", "2").last(), Some(&"-"));
        assert_eq!(words("tier", &RUN_TIERS.len().to_string()).last(), Some(&"-"));
        assert_eq!(
            words("elite", "1"),
            ["elite", "1", "1", "0", "0.0", "40.0", "12.0"]
        );
        for (tier, _) in RUN_ELITE_TIERS {
            assert_eq!(words("elite", &tier.to_string()).len(), 7, "elite {tier}");
        }
        assert!(
            find("elite", &boss_tier.to_string()).is_none(),
            "an elite never stands on a boss's tier: {text}"
        );
    }

    #[test]
    fn the_tally_splits_the_fights_by_act_and_the_score_by_term() {
        let content = content();
        let (setup, unlocks) = run_setup(0, &Start::default());
        let start = run_create(&setup, &unlocks, &content).expect("run 0 is created");
        let file = RunFile {
            setup,
            unlocks,
            log: Vec::new(),
        };
        // No boss won and no curse: only the five sums.
        let plain = RunState {
            row: Some(3),
            phase: RunPhase::Dead,
            hp: 0,
            stats: RunStats {
                fights_won: 3,
                elites_won: 1,
                damage_dealt: 40,
                healing_done: 7,
                cards_drawn: 12,
                ..RunStats::default()
            },
            ..start.clone()
        };
        // A mini-boss and the final boss won, and a curse held: the percents add.
        let cursed = RunState {
            row: Some(8),
            phase: RunPhase::Won,
            curses: vec![content.curses[0].id.clone()],
            stats: RunStats {
                fights_won: 5,
                mini_bosses_won: 1,
                bosses_won: 1,
                damage_dealt: 90,
                healing_done: 20,
                cards_drawn: 30,
                ..RunStats::default()
            },
            ..start.clone()
        };
        let outcomes = [
            RunOutcome {
                file: file.clone(),
                end: plain.clone(),
                fights: vec![
                    in_act(0, row(RunNodeKind::Fight, 30, 35, 25, true, 10)),
                    in_act(0, row(RunNodeKind::Elite, 25, 0, 0, false, 14)),
                    in_act(0, row(RunNodeKind::Fight, 30, 35, 30, true, 8)),
                    in_act(0, row(RunNodeKind::MiniBoss, 30, 35, 20, true, 20)),
                ],
                faults: Vec::new(),
            },
            RunOutcome {
                file,
                end: cursed.clone(),
                fights: vec![
                    in_act(1, row(RunNodeKind::Fight, 20, 25, 18, true, 6)),
                    in_act(1, row(RunNodeKind::Ghost, 18, 23, 18, true, 10)),
                    in_act(1, row(RunNodeKind::MiniBoss, 18, 23, 10, true, 16)),
                ],
                faults: Vec::new(),
            },
        ];
        let sums = tally(&outcomes, &content);
        let sums_of = |fights, won, hp_lost, turns| KindTally {
            fights,
            won,
            hp_lost,
            turns,
        };
        assert_eq!(sums.acts[0], sums_of(4, 3, 40, 52));
        assert_eq!(sums.acts[1], sums_of(3, 3, 10, 32));
        assert_eq!(sums.acts[2], KindTally::default());

        let text = sums.render("header");
        let line = |name: &str| {
            text.lines()
                .find(|line| line.split_whitespace().next() == Some(name))
                .unwrap_or_else(|| panic!("no {name} line in:\n{text}"))
                .to_string()
        };
        let words = |name: &str| {
            line(name)
                .split_whitespace()
                .map(String::from)
                .collect::<Vec<_>>()
        };
        assert_eq!(words("1"), ["1", "4", "3", "75.0", "10.0", "13.0"]);
        assert_eq!(words("2"), ["2", "3", "3", "100.0", "3.3", "10.7"]);
        assert!(line("3").ends_with('-'), "{text}");
        assert!(text.contains("\nact "), "{text}");

        let plain_parts = score_parts(&plain, &content);
        assert_eq!((plain_parts.bosses, plain_parts.curses), (0, 0));
        assert_eq!(
            plain_parts.rows
                + plain_parts.wins
                + plain_parts.damage
                + plain_parts.healing
                + plain_parts.drawn,
            run_score(&plain, &content)
        );
        let cursed_parts = score_parts(&cursed, &content);
        assert!(
            cursed_parts.bosses > 0 && cursed_parts.curses > 0,
            "{cursed_parts:?}"
        );
        for (end, parts) in [(&plain, plain_parts), (&cursed, cursed_parts)] {
            assert_eq!(
                parts.terms().iter().map(|(_, n)| n).sum::<i64>(),
                run_score(end, &content)
            );
        }
        assert_eq!(sums.parts, [plain_parts, cursed_parts]);
        for (at, (label, _)) in ScoreParts::default().terms().iter().enumerate() {
            let values = [plain_parts.terms()[at].1, cursed_parts.terms()[at].1];
            assert!(
                text.contains(&format!("score from {label}: {}\n", mean_and_max(&values))),
                "{label}: {text}"
            );
        }
    }

    /// The runs, from `run-sim --from 0`, whose fights the tier bands play.
    const BAND_SEEDS: u32 = 6;
    /// The fewest tier-1 fights, in percent, the starting deck may win before the early game is no longer
    /// clearly winnable (measured at `TEST_BUDGET`: all of 24).
    const TIER_ONE_WIN_PERCENT_MIN: i64 = 75;
    /// The most final-boss fights, in percent, the starting deck may win before the boss is no longer
    /// clearly hard (measured: 2 of 24, 2 of the first 6).
    const FINAL_BOSS_WIN_PERCENT_MAX: i64 = 50;
    /// The tiers whose elite is played beside the normal fight it stands on (the cheap ones).
    const ELITE_BAND_TIERS: [usize; 2] = [1, 2];

    /// A kind of fight the bands play, each from run `n`'s start at full HP with its starting deck.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum BandFight {
        /// A normal fight at this tier, on the run's first row.
        Normal(usize),
        /// An elite at this tier, on that same node: the normal fight's seed and enemy deck.
        Elite(usize),
        /// The map's last row, the final boss's.
        FinalBoss,
    }

    impl BandFight {
        /// The tier table's name for this fight.
        fn label(self) -> String {
            match self {
                BandFight::Normal(tier) => format!("tier {tier}"),
                BandFight::Elite(tier) => format!("elite {tier}"),
                BandFight::FinalBoss => format!("tier {}", RUN_TIERS.len()),
            }
        }
    }

    /// Run `n`'s start (nothing taken, level 1), standing in a `which` fight.
    fn band_state(n: u32, which: BandFight, content: &RunContent) -> RunState {
        let (setup, unlocks) = run_setup(n, &Start::default());
        let start = run_create(&setup, &unlocks, content).expect("the run is created");
        let (tier, kind) = match which {
            BandFight::FinalBoss => {
                return RunState {
                    row: Some(start.map.len() - 1),
                    col: 0,
                    phase: RunPhase::Fight,
                    ..start
                };
            }
            BandFight::Normal(tier) => (tier, RunNodeKind::Fight),
            BandFight::Elite(tier) => (tier, RunNodeKind::Elite),
        };
        let col = start.map[0]
            .iter()
            .position(|node| node.kind == RunNodeKind::Fight)
            .unwrap_or(0);
        let mut state = run_reduce(&start, content, &RunAction::Go { col }).expect("row 0 is a fight row");
        let (row, col) = (state.row.expect("a row"), state.col);
        let node = &mut state.map[row][col];
        node.kind = kind;
        node.enemy.as_mut().expect("a fight node has an enemy").tier = tier;
        state
    }

    /// `which` on run `n`, played to its end at `TEST_BUDGET` and full skill, as the tally's row.
    fn play_band_fight(n: u32, which: BandFight, content: &RunContent) -> Result<FightRow, String> {
        let state = band_state(n, which, content);
        let action = play_fight(&state, content, AI_SKILL_MAX, TEST_BUDGET)?;
        let next = run_reduce(&state, content, &action)?;
        fight_row(&state, &action, &next)
    }

    /// Each fight the bands read, `BAND_SEEDS` times, summed as the tally's tier table sums them, in the
    /// order listed. The fights play on every core, the slowest first, and come back in input order.
    fn play_bands(content: &RunContent) -> Result<Vec<(BandFight, KindTally)>, String> {
        let mut fights = vec![BandFight::FinalBoss];
        for tier in std::iter::once(1).chain(ELITE_BAND_TIERS) {
            if !fights.contains(&BandFight::Normal(tier)) {
                fights.push(BandFight::Normal(tier));
            }
        }
        fights.extend(ELITE_BAND_TIERS.map(BandFight::Elite));
        let jobs: Vec<(BandFight, u32)> = fights
            .iter()
            .flat_map(|&which| (0..BAND_SEEDS).map(move |n| (which, n)))
            .collect();
        let rows: Vec<(BandFight, Result<FightRow, String>)> = jobs
            .into_par_iter()
            .map(|(which, n)| {
                let row = play_band_fight(n, which, content).map_err(|e| format!("run {n}, {which:?}: {e}"));
                (which, row)
            })
            .collect();
        let mut sample: Vec<(BandFight, KindTally)> = fights
            .iter()
            .map(|&which| (which, KindTally::default()))
            .collect();
        for (which, row) in rows {
            let at = sample.iter().position(|(played, _)| *played == which);
            sample[at.expect("a played fight is a listed one")].1.add(&row?);
        }
        Ok(sample)
    }

    /// The bands `sample` breaks, each named with its numbers: tier 1 won at least
    /// `TIER_ONE_WIN_PERCENT_MIN` percent, the final boss at most `FINAL_BOSS_WIN_PERCENT_MAX` percent,
    /// each elite costing more HP than the normal fight at its tier. A fight not played breaks its band.
    fn broken_bands(sample: &[(BandFight, KindTally)]) -> Vec<String> {
        let sums = |which: BandFight| {
            sample
                .iter()
                .find(|(played, _)| *played == which)
                .map(|(_, sums)| *sums)
                .unwrap_or_default()
        };
        let percent = i64::from(PERCENT);
        let mut broken = Vec::new();
        let first = sums(BandFight::Normal(1));
        if first.fights == 0 || first.won as i64 * percent < first.fights as i64 * TIER_ONE_WIN_PERCENT_MIN {
            broken.push(format!(
                "tier 1 won {} of {}, under {TIER_ONE_WIN_PERCENT_MIN}%: the early game is no longer clearly winnable",
                first.won, first.fights
            ));
        }
        let boss = sums(BandFight::FinalBoss);
        if boss.fights == 0 || boss.won as i64 * percent > boss.fights as i64 * FINAL_BOSS_WIN_PERCENT_MAX {
            broken.push(format!(
                "the final boss won {} of {}, over {FINAL_BOSS_WIN_PERCENT_MAX}%: it is no longer clearly hard",
                boss.won, boss.fights
            ));
        }
        for tier in ELITE_BAND_TIERS {
            let (normal, elite) = (sums(BandFight::Normal(tier)), sums(BandFight::Elite(tier)));
            if normal.fights == 0 || elite.fights == 0 || elite.hp_lost <= normal.hp_lost {
                broken.push(format!(
                    "elite {tier} cost {} HP over {} fights, no more than tier {tier}'s {} over {}: it is no longer harder",
                    elite.hp_lost, elite.fights, normal.hp_lost, normal.fights
                ));
            }
        }
        broken
    }

    /// `sample` as the tier table prints it.
    fn render_bands(sample: &[(BandFight, KindTally)]) -> String {
        let mut out = table_header("tier");
        for (which, sums) in sample {
            out.push_str(&table_line(&which.label(), sums));
        }
        out
    }

    #[test]
    fn the_enemy_tiers_hold_run_sims_win_rate_bands() {
        // R3586000, R3586001: `RUN_TIERS` and `RUN_ELITE_TIERS` are strawmen to tune, not to break. A
        // whole run dies by its first mini-boss at this budget, so the climb is read from the fights a
        // run meets, played one at a time from each seed's start (the same game `run-sim` plays there).
        let content = content();
        let sample = play_bands(&content).expect("the band fights end clean");
        let broken = broken_bands(&sample);
        assert!(
            broken.is_empty(),
            "the enemy tiers broke their bands (change the bands only with a deliberate retune):\n{}\n{}",
            broken.join("\n"),
            render_bands(&sample)
        );
    }

    #[test]
    fn each_band_names_the_tier_edit_that_breaks_it() {
        let sums = |fights, won, hp_lost| KindTally {
            fights,
            won,
            hp_lost,
            turns: 0,
        };
        let sound = vec![
            (BandFight::FinalBoss, sums(6, 2, 152)),
            (BandFight::Normal(1), sums(6, 6, 13)),
            (BandFight::Normal(2), sums(6, 6, 6)),
            (BandFight::Elite(1), sums(6, 6, 23)),
            (BandFight::Elite(2), sums(6, 6, 21)),
        ];
        assert_eq!(broken_bands(&sound), Vec::<String>::new());
        let with = |which: BandFight, edited: KindTally| -> Vec<(BandFight, KindTally)> {
            sound
                .iter()
                .map(|&(played, sums)| (played, if played == which { edited } else { sums }))
                .collect()
        };
        let broken = |which, edited| broken_bands(&with(which, edited));
        // The first fights brutal: 3 of 6 won is under the floor, 5 of 6 is not.
        let brutal = broken(BandFight::Normal(1), sums(6, 3, 13));
        assert_eq!(brutal.len(), 1);
        assert!(brutal[0].starts_with("tier 1 won 3 of 6"), "{brutal:?}");
        assert!(broken(BandFight::Normal(1), sums(6, 5, 13)).is_empty());
        // The final boss trivial: 6 of 6 won is over the ceiling, 3 of 6 is not.
        let trivial = broken(BandFight::FinalBoss, sums(6, 6, 20));
        assert_eq!(trivial.len(), 1);
        assert!(trivial[0].starts_with("the final boss won 6 of 6"), "{trivial:?}");
        assert!(broken(BandFight::FinalBoss, sums(6, 3, 120)).is_empty());
        // An elite no harder than its tier's normal fight: equal HP lost breaks the band.
        let elite = broken(BandFight::Elite(2), sums(6, 6, 6));
        assert_eq!(elite.len(), 1);
        assert!(elite[0].starts_with("elite 2 "), "{elite:?}");
        // A fight that was not played breaks its band.
        assert_eq!(broken_bands(&[]).len(), 2 + ELITE_BAND_TIERS.len());
    }

    /// The stub-won runs the economy bands read (a prefix of them is `BAND_SEEDS`' runs). A won run costs
    /// no AI, so there are more of them than of the AI's.
    const ECONOMY_RUNS: u32 = 24;
    /// The fights each AI run plays before it stops (the first mini-boss ends act 1's route); whole AI runs
    /// are the test's slow part, so the score and Embers bands are measured on this many.
    const ECONOMY_AI_FIGHTS: usize = 6;
    /// The fewest Coins, on average, a won run earns (§15.7) before shops are decoration (measured: mean 707
    /// over 24 runs, max 870).
    const COINS_PER_RUN_MIN: i64 = 353;
    /// The most Coins, on average, a won run earns before nothing in a shop is a choice (measured: as above).
    const COINS_PER_RUN_MAX: i64 = 1414;
    /// The fewest Tickets, on average, a won run earns (measured: mean 43, max 55).
    const TICKETS_PER_RUN_MIN: i64 = 21;
    /// The most Tickets, on average, a won run earns (measured: as above).
    const TICKETS_PER_RUN_MAX: i64 = 86;
    /// The fewest points, on average, an AI run at `TEST_BUDGET` scores (§15.10; measured: mean 470 over
    /// `BAND_SEEDS` runs of `ECONOMY_AI_FIGHTS` fights, max 1084).
    const SCORE_PER_RUN_MIN: i64 = 235;
    /// The most points, on average, an AI run at `TEST_BUDGET` scores (measured: as above).
    const SCORE_PER_RUN_MAX: i64 = 941;
    /// The fewest Embers, on average, an AI run pays (measured: mean 4, max 10).
    const EMBERS_PER_RUN_MIN: i64 = 2;
    /// The most Embers, on average, an AI run pays (measured: as above).
    const EMBERS_PER_RUN_MAX: i64 = 8;
    /// The fewest stub-won runs, in percent, that enter a shop.
    const SHOP_RUNS_MIN_PERCENT: i64 = 50;
    /// The most the cheapest shop card may cost, in percent of the Coins a run holds entering a shop.
    const CARD_PRICE_CEILING_PERCENT: i64 = 50;

    /// What the economy bands read: the stub-won runs' earnings and first shops, the AI runs' scores.
    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    struct EconomySample {
        /// The stub-won runs played.
        runs: usize,
        coins: Vec<i64>,
        tickets: Vec<i64>,
        /// Each stub-won run's first shop, for the runs that enter one.
        first_shops: Vec<ShopVisit>,
        /// Each AI run's score and the Embers it pays.
        scores: Vec<i64>,
        embers: Vec<i64>,
    }

    /// The stub-won runs' Coins, Tickets and first shops, and `BAND_SEEDS` whole AI runs at `TEST_BUDGET`
    /// and full skill (the tiers test's seeds and budget) for the score and Embers.
    fn play_economy(content: &RunContent) -> Result<EconomySample, String> {
        let mut sample = EconomySample {
            runs: ECONOMY_RUNS as usize,
            ..EconomySample::default()
        };
        for n in 0..ECONOMY_RUNS {
            let run = play_run(n, content, &Start::default(), &won, None)?;
            if !run.faults.is_empty() {
                return Err(run.faults.join("; "));
            }
            sample.coins.push(i64::from(run.end.stats.coins_earned));
            sample.tickets.push(i64::from(run.end.stats.tickets_earned));
            sample
                .first_shops
                .extend(shop_visits(&run.file, content).first().copied());
        }
        let ai: Vec<(u32, Result<RunOutcome, String>)> = (0..BAND_SEEDS)
            .into_par_iter()
            .map(|n| {
                let fight = |state: &RunState| play_fight(state, content, AI_SKILL_MAX, TEST_BUDGET);
                (
                    n,
                    play_run(n, content, &Start::default(), &fight, Some(ECONOMY_AI_FIGHTS)),
                )
            })
            .collect();
        for (n, run) in ai {
            let run = run.map_err(|e| format!("run {n}: {e}"))?;
            if !run.faults.is_empty() {
                return Err(run.faults.join("; "));
            }
            let score = run_score(&run.end, content);
            sample.scores.push(score);
            sample.embers.push(run_embers(score));
        }
        Ok(sample)
    }

    /// The middle of `values` (the upper one of two), 0 for none.
    fn median(values: &[i64]) -> i64 {
        let mut sorted = values.to_vec();
        sorted.sort_unstable();
        sorted.get(sorted.len() / 2).copied().unwrap_or(0)
    }

    /// `values`' mean, rounded down, 0 for none.
    fn mean_of(values: &[i64]) -> i64 {
        i64::try_from(values.len()).map_or(0, |n| values.iter().sum::<i64>() / n.max(1))
    }

    /// The bands `s` breaks, each named with its numbers and the edit that breaks it. A sample with no
    /// runs breaks every band.
    fn broken_economy(s: &EconomySample) -> Vec<String> {
        let percent = i64::from(PERCENT);
        let mut broken = Vec::new();
        let (coins, tickets) = (mean_of(&s.coins), mean_of(&s.tickets));
        if s.coins.is_empty() || !(COINS_PER_RUN_MIN..=COINS_PER_RUN_MAX).contains(&coins) {
            broken.push(format!(
                "coins earned per won run {coins} outside {COINS_PER_RUN_MIN}..{COINS_PER_RUN_MAX}: RUN_COINS retuned too far"
            ));
        }
        if s.tickets.is_empty() || !(TICKETS_PER_RUN_MIN..=TICKETS_PER_RUN_MAX).contains(&tickets) {
            broken.push(format!(
                "tickets earned per won run {tickets} outside {TICKETS_PER_RUN_MIN}..{TICKETS_PER_RUN_MAX}: RUN_TICKETS retuned too far"
            ));
        }
        let shops = s.first_shops.len();
        if shops as i64 * percent < s.runs as i64 * SHOP_RUNS_MIN_PERCENT || shops == 0 {
            broken.push(format!(
                "only {shops} of {} runs reach a shop, under {SHOP_RUNS_MIN_PERCENT}%: the policy or the map no longer gets to one",
                s.runs
            ));
        }
        let shop_values = |value: &dyn Fn(&ShopVisit) -> Option<i64>| -> Vec<i64> {
            s.first_shops.iter().filter_map(value).collect()
        };
        let held = median(&shop_values(&|shop| Some(i64::from(shop.coins))));
        let cheapest = median(&shop_values(&|shop| shop.cheapest_card.map(i64::from)));
        if shops == 0 || held < cheapest {
            broken.push(format!(
                "median run cannot afford the cheapest shop card by its first shop: {held} held, {cheapest} asked: RUN_CARD_PRICE or RUN_COINS"
            ));
        }
        // The cards beside the Coins held on the same visits, so the percent compares like with like.
        let priced: Vec<&ShopVisit> = s
            .first_shops
            .iter()
            .filter(|shop| shop.cheapest_card.is_some())
            .collect();
        let asked: i64 = priced
            .iter()
            .filter_map(|shop| shop.cheapest_card)
            .map(i64::from)
            .sum();
        let held_sum: i64 = priced.iter().map(|shop| i64::from(shop.coins)).sum();
        if priced.is_empty() || asked * percent > held_sum * CARD_PRICE_CEILING_PERCENT {
            broken.push(format!(
                "cheapest shop card costs {asked} for every {held_sum} Coins a run holds entering its first shop, over {CARD_PRICE_CEILING_PERCENT}%: the cards are no longer a real choice (RUN_CARD_PRICE or RUN_COINS)"
            ));
        }
        let remove = median(&shop_values(&|shop| Some(i64::from(shop.remove))));
        if shops == 0 || median(&s.coins) < cheapest + remove {
            broken.push(format!(
                "median run cannot afford a Remove plus the cheapest card over its run: {} earned, {} asked: RUN_REMOVE_PRICE, RUN_CARD_PRICE or RUN_COINS",
                median(&s.coins),
                cheapest + remove
            ));
        }
        let services = median(&shop_values(&|shop| Some(i64::from(shop.radiant + shop.buff))));
        if shops == 0 || median(&s.tickets) < services {
            broken.push(format!(
                "median run's Tickets cannot pay one Radiant and one buff: {} earned, {services} asked: RUN_TICKETS, RUN_RADIANT_TICKETS or RUN_BUFF_TICKETS",
                median(&s.tickets)
            ));
        }
        let (score, embers) = (mean_of(&s.scores), mean_of(&s.embers));
        if s.scores.is_empty() || !(SCORE_PER_RUN_MIN..=SCORE_PER_RUN_MAX).contains(&score) {
            broken.push(format!(
                "score per run {score} outside {SCORE_PER_RUN_MIN}..{SCORE_PER_RUN_MAX}: the RUN_SCORE_* row retuned too far"
            ));
        }
        if s.embers.is_empty() || !(EMBERS_PER_RUN_MIN..=EMBERS_PER_RUN_MAX).contains(&embers) {
            broken.push(format!(
                "Embers per run {embers} outside {EMBERS_PER_RUN_MIN}..{EMBERS_PER_RUN_MAX}: RUN_SCORE_PER_EMBER retuned too far"
            ));
        }
        broken
    }

    /// `s` as the economy lines print them.
    fn render_economy(s: &EconomySample) -> String {
        let shop = |value: &dyn Fn(&ShopVisit) -> Option<i64>| -> Vec<i64> {
            s.first_shops.iter().filter_map(value).collect()
        };
        format!(
            "coins earned per won run: {}\ntickets earned per won run: {}\nfirst shops: {} of {} runs\ncoins held entering the first shop: {}\ncheapest shop card: {}\nremove price: {}\nscore per AI run: {}\nEmbers per AI run: {}\n",
            mean_and_max(&s.coins),
            mean_and_max(&s.tickets),
            s.first_shops.len(),
            s.runs,
            mean_and_max(&shop(&|v| Some(i64::from(v.coins)))),
            mean_and_max(&shop(&|v| v.cheapest_card.map(i64::from))),
            mean_and_max(&shop(&|v| Some(i64::from(v.remove)))),
            mean_and_max(&s.scores),
            mean_and_max(&s.embers),
        )
    }

    #[test]
    fn the_economy_holds_run_sims_bands() {
        // The strawmen of §15.13 (Coins, Tickets, prices, the score row) are to tune, not to break.
        let content = content();
        let sample = play_economy(&content).expect("the economy runs end clean");
        let broken = broken_economy(&sample);
        assert!(
            broken.is_empty(),
            "the economy broke its bands (change the bands only with a deliberate retune):\n{}\n{}",
            broken.join("\n"),
            render_economy(&sample)
        );
    }

    #[test]
    fn each_economy_band_names_the_edit_that_breaks_it() {
        let shop = ShopVisit {
            coins: 180,
            tickets: 10,
            cheapest_card: Some(23),
            remove: 50,
            radiant: 3,
            buff: 2,
        };
        let sound = EconomySample {
            runs: 24,
            coins: vec![700; 24],
            tickets: vec![43; 24],
            first_shops: vec![shop; 22],
            scores: vec![470; 6],
            embers: vec![4; 6],
        };
        assert_eq!(broken_economy(&sound), Vec::<String>::new());
        let with_shops = |edit: &dyn Fn(&mut ShopVisit)| {
            let mut edited = sound.clone();
            edited.first_shops.iter_mut().for_each(edit);
            edited
        };
        let has = |broken: &[String], starts: &[&str]| {
            assert_eq!(broken.len(), starts.len(), "{broken:?}");
            for start in starts {
                assert!(broken.iter().any(|b| b.starts_with(start)), "{start}: {broken:?}");
            }
        };
        // RUN_CARD_PRICE tenfold: the first shop is out of reach and the card is no choice, nothing else.
        let dear = broken_economy(&with_shops(&|v| {
            v.cheapest_card = v.cheapest_card.map(|p| p * 10)
        }));
        has(
            &dear,
            &[
                "median run cannot afford the cheapest shop card by its first shop: 180 held, 230 asked: RUN_CARD_PRICE",
                "cheapest shop card costs ",
            ],
        );
        assert!(dear.iter().all(|b| b.contains("RUN_CARD_PRICE")), "{dear:?}");
        // RUN_COINS zeroed: no income cannot keep affordability, and its bands go with it; Tickets and the
        // score stay.
        let mut broke = with_shops(&|v| v.coins = 0);
        broke.coins = vec![0; 24];
        has(
            &broken_economy(&broke),
            &[
                "coins earned per won run 0 ",
                "median run cannot afford the cheapest shop card",
                "cheapest shop card costs ",
                "median run cannot afford a Remove",
            ],
        );
        // RUN_SCORE_PER_EMBER tenfold: the score is the same and its Embers fall to a tenth.
        let mut stingy = sound.clone();
        stingy.embers = vec![0; 6];
        let stingy = broken_economy(&stingy);
        has(&stingy, &["Embers per run 0 "]);
        assert!(stingy[0].contains("RUN_SCORE_PER_EMBER"), "{stingy:?}");
        // The score row tenfold at the same price: the score and the Embers both leave their bands.
        let mut rich = sound.clone();
        rich.scores = vec![4700; 6];
        rich.embers = vec![47; 6];
        has(
            &broken_economy(&rich),
            &["score per run 4700 ", "Embers per run 47 "],
        );
        // Edits inside the bands break nothing: a card half as dear again, a fifth less income.
        let mut mild = with_shops(&|v| v.cheapest_card = v.cheapest_card.map(|p| p * 3 / 2));
        mild.coins = vec![560; 24];
        assert_eq!(broken_economy(&mild), Vec::<String>::new());
        // Services out of a run's means.
        let mut poor = sound.clone();
        poor.tickets = vec![4; 24];
        has(
            &broken_economy(&poor),
            &[
                "tickets earned per won run 4 ",
                "median run's Tickets cannot pay one Radiant and one buff",
            ],
        );
        // Runs that never reach a shop, and no runs at all.
        let mut lost = sound.clone();
        lost.first_shops.truncate(5);
        assert!(broken_economy(&lost)[0].starts_with("only 5 of 24 runs reach a shop"));
        assert_eq!(broken_economy(&EconomySample::default()).len(), 9);
    }

    #[test]
    fn the_tally_prints_the_economy_lines_after_the_old_rows() {
        let content = content();
        let (n, shopped) = (0..ECONOMY_RUNS)
            .map(|n| {
                (
                    n,
                    play_run(n, &content, &Start::default(), &won, None).expect("a run"),
                )
            })
            .find(|(_, run)| !shop_visits(&run.file, &content).is_empty())
            .expect("a stub-won run enters a shop");
        assert!(shopped.faults.is_empty(), "{:?}", shopped.faults);
        let visits = shop_visits(&shopped.file, &content);
        assert!(
            visits
                .iter()
                .all(|v| v.radiant == RUN_RADIANT_TICKETS && v.buff == RUN_BUFF_TICKETS && v.remove > 0),
            "{visits:?}"
        );
        // A fresh run's empty log enters no shop.
        let (setup, unlocks) = run_setup(n, &Start::default());
        let fresh = RunFile {
            setup,
            unlocks,
            log: Vec::new(),
        };
        assert!(shop_visits(&fresh, &content).is_empty());
        let end = RunState {
            stats: RunStats {
                coins_earned: 100,
                tickets_earned: 7,
                ..RunStats::default()
            },
            ..shopped.end.clone()
        };
        let outcomes = vec![
            shopped,
            RunOutcome {
                file: fresh,
                end,
                fights: Vec::new(),
                faults: Vec::new(),
            },
        ];
        let text = tally(&outcomes, &content).render("h");
        let embers = text.find("Embers: ").expect("the Embers line");
        for line in [
            "coins earned: mean ",
            "tickets earned: ",
            "shops: ",
            "cheapest shop card: ",
            "prices: remove ",
        ] {
            assert!(text.find(line).is_some_and(|at| at > embers), "{line}: {text}");
        }
        assert!(
            text.contains("runs won: ") && text.contains("score from rows: "),
            "{text}"
        );
        assert!(
            text.contains(&format!("shops: {} visits in 1 of 2 runs\n", visits.len())),
            "{text}"
        );
    }

    #[test]
    #[ignore = "daily: DAILY_SEEDS whole runs per skill at full budget; the two run-fight tests above play at the tiny budget per pull request"]
    fn the_daily_sweep_plays_whole_runs_at_full_budget_and_prints_the_report() {
        let content = content();
        let (text, faults) = sweep(DAILY_SEEDS, 0, &DAILY_SKILLS, "full", &Start::default(), &content);
        println!("{text}");
        assert!(faults.is_empty(), "{faults:#?}");
        assert!(text.contains("runs won"), "{text}");
    }
}
