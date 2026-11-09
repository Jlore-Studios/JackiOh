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
//! `run_fight_report`. Every other choice (a column, the artifact, a pack and what to
//! keep, a shop's buys and upgrades, a campfire, an event, a blessing) follows a seeded policy.
//!
//! For each skill it prints, by kind of fight and again by act, how many were played and won, the HP they
//! cost and their turns, then the HP entering each boss, the rows reached, the runs won, the score by term
//! (rows, wins, damage, healing, cards drawn, and what the boss percents and the curse percent add, each
//! mean and max: only a breakdown shows whether stalling pays), the score and the Embers. The AI plays
//! the player's seat better or worse than a person would, so the table is a guide to tuning the strawmen
//! (§15.13), not a target. A lost fight is data; an unclean fight, a dead end of the policy
//! or a run file that does not fold to the run's end is a fault, which exits 1 and names the command
//! that plays its run again.
//!
//! The policy draws from a stream of its own, never the run's `rng_cursor`, so the run's log still
//! `run_fold`s to the same end; nothing reads a clock, and each run's results are kept in seed order, so
//! the output is the same on any thread count.
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
    RUN_ACT_FIGHTS, RUN_SCORE_CARD_DRAWN, RUN_SCORE_DAMAGE, RUN_SCORE_FIGHT, RUN_SCORE_HEALING,
    RUN_SCORE_ROW, RUN_SLOT_LADDER, RUN_STARTER_CARDS, RUN_STARTER_SIZE,
};
use jackioh_run::{
    RunAction, RunContent, RunFightSetup, RunFile, RunNodeKind, RunPassiveKind, RunPhase, RunSetup, RunState,
    RunUnlocks, run_catalog_content, run_check_setup, run_create, run_embers, run_fight_report,
    run_fight_setup, run_fold, run_max_hp, run_node, run_reduce, run_score, run_seed,
};
use rayon::prelude::*;

use crate::arena::TEST_BUDGET;

/// The steps one run may take before it is a fault: a whole run with every fight won takes about a
/// hundred, so this fires only on a policy that never moves on.
const MAX_STEPS: usize = 2000;

/// A campfire heals first when the run's HP is below this percent of its max; otherwise it upgrades a
/// card.
const HEAL_BELOW_PERCENT: i32 = 60;

/// The kinds of fight the table breaks down, each with its row's name (the kind's serialized name).
const KINDS: [(RunNodeKind, &str); 5] = [
    (RunNodeKind::Fight, "fight"),
    (RunNodeKind::Elite, "elite"),
    (RunNodeKind::Ghost, "ghost"),
    (RunNodeKind::MiniBoss, "miniBoss"),
    (RunNodeKind::Boss, "boss"),
];

/// A run's bosses: one at the end of each act (§15.5), the last the final boss.
const BOSSES: usize = RUN_ACT_FIGHTS.len();

/// A run's acts (§15.5): one table row each.
const ACTS: usize = RUN_ACT_FIGHTS.len();

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

/// The policy's choices at a phase other than a fight, most wanted first: a random column, then the
/// first; the artifact; a random pack, then the first; a random `keep` of the pack, then its first; no
/// extraction; at a shop each unsold good in a random order, a random card made Radiant, a random card
/// Buffed, then leaving (each but leaving spends, so the shop ends); at a campfire a heal when HP is
/// below `HEAL_BELOW_PERCENT`, else a random card made Radiant, else one Buffed; a random event option,
/// then the last that costs nothing; a random blessing, then none.
fn candidates(state: &RunState, content: &RunContent, rng: &mut Rng) -> Result<Vec<RunAction>, String> {
    let deck = state.deck.len();
    Ok(match &state.phase {
        RunPhase::Map => {
            let row = state.row.map_or(0, |r| r + 1);
            let width = state.map.get(row).map_or(0, Vec::len);
            vec![
                RunAction::Go {
                    col: index(rng, width),
                },
                RunAction::Go { col: 0 },
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
            let low = state.hp * PERCENT < run_max_hp(state, content) * HEAL_BELOW_PERCENT;
            let mut out: Vec<RunAction> = low.then_some(RunAction::Heal).into_iter().collect();
            out.push(RunAction::MakeRadiant {
                index: index(rng, deck),
            });
            out.push(RunAction::Buff {
                index: index(rng, deck),
            });
            out.push(RunAction::Heal);
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
                    .rposition(|option| option.hp >= 0 && option.coins >= 0 && option.tickets >= 0)
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

/// One fight of a run: its kind, the HP entering and leaving it (0 for a lost fight), and its turns.
#[derive(Clone, Debug, PartialEq, Eq)]
struct FightRow {
    kind: RunNodeKind,
    /// 0-based, `act_of`'s.
    act: usize,
    hp_before: i32,
    hp_after: i32,
    won: bool,
    turns: i32,
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
                fights.push(FightRow {
                    kind: run_node(&state).ok_or("the fight stands on no node")?.kind,
                    act: act_of(&state),
                    hp_before: state.hp,
                    hp_after: next.hp,
                    won: !matches!(next.phase, RunPhase::Dead),
                    turns: match &action {
                        RunAction::FightOver { turns, .. } => *turns,
                        _ => 0,
                    },
                });
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

/// One group of fights' sums over the runs: a kind's or an act's.
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
        self.hp_lost += i64::from(fight.hp_before - fight.hp_after);
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
    let s = &end.stats;
    let won = |kind: RunNodeKind, n: i32| i64::from(RUN_SCORE_FIGHT.of(kind)) * i64::from(n);
    let normal = s.fights_won - s.elites_won - s.ghosts_won - s.mini_bosses_won - s.bosses_won;
    let rows = RUN_SCORE_ROW * i64::try_from(end.row.map_or(0, |r| r + 1)).unwrap_or(0);
    let wins = won(RunNodeKind::Fight, normal)
        + won(RunNodeKind::Elite, s.elites_won)
        + won(RunNodeKind::Ghost, s.ghosts_won)
        + won(RunNodeKind::MiniBoss, s.mini_bosses_won)
        + won(RunNodeKind::Boss, s.bosses_won);
    let damage = RUN_SCORE_DAMAGE * i64::from(s.damage_dealt);
    let healing = RUN_SCORE_HEALING * i64::from(s.healing_done);
    let drawn = RUN_SCORE_CARD_DRAWN * i64::from(s.cards_drawn);
    let base = rows + wins + damage + healing + drawn;
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

/// What the table prints, summed over one skill's runs.
#[derive(Debug, Default, PartialEq, Eq)]
struct Tally {
    runs: usize,
    runs_won: usize,
    /// By `KINDS`.
    kinds: [KindTally; KINDS.len()],
    /// By act, from the first.
    acts: [KindTally; ACTS],
    /// The HP entering each boss, summed, and how many runs reached it.
    boss_hp: [(i64, usize); BOSSES],
    /// Each run's rows reached.
    rows: Vec<usize>,
    /// The most rows a run's map holds.
    map_rows: usize,
    /// Each run's score at its end.
    scores: Vec<i64>,
    /// Each run's score by term.
    parts: Vec<ScoreParts>,
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
        out.map_rows = out.map_rows.max(outcome.end.map.len());
        out.scores.push(run_score(&outcome.end, content));
        out.parts.push(score_parts(&outcome.end, content));
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
        out.push_str("(HP lost is the HP entering a fight minus the HP leaving it, so a lost fight loses all of it.)\n");
        for (at, (hp, reached)) in self.boss_hp.iter().enumerate() {
            out.push_str(&format!(
                "HP entering boss {}: mean {} (n {reached})\n",
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
        out.push_str(&format!("runs won: {} of {}\n", self.runs_won, self.runs));
        for (at, (label, _)) in ScoreParts::default().terms().iter().enumerate() {
            let values: Vec<i64> = self.parts.iter().map(|parts| parts.terms()[at].1).collect();
            out.push_str(&format!("score from {label}: {}\n", mean_and_max(&values)));
        }
        let embers: Vec<i64> = self.scores.iter().map(|&score| run_embers(score)).collect();
        out.push_str(&format!("score: {}\n", mean_and_max(&self.scores)));
        out.push_str(&format!("Embers: {}\n", mean_and_max(&embers)));
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
    use jackioh_run::config::{RUN_STARTER_ARTIFACT_POINTS, RUN_STARTER_ARTIFACTS_MAX};
    use jackioh_run::{RunStats, run_artifact_points, run_curse_points};

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
            assert_eq!(
                outcome.fights.len(),
                RUN_ACT_FIGHTS.iter().sum::<usize>() + BOSSES,
                "run {n}"
            );
            assert_eq!(
                run_fold(&outcome.file, &content),
                Ok(outcome.end.clone()),
                "run {n}"
            );
        }
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
        // §15.5: an act is its fight rows, a tile row between each two, a campfire and its boss.
        let spans: Vec<usize> = RUN_ACT_FIGHTS.iter().map(|fights| 2 * fights + 1).collect();
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
            for (act, fights) in RUN_ACT_FIGHTS.iter().enumerate() {
                let played = outcome.fights.iter().filter(|fight| fight.act == act).count();
                assert_eq!(played, fights + 1, "run {n} act {act}");
            }
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
        let content = content();
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

    fn row(kind: RunNodeKind, hp_before: i32, hp_after: i32, won: bool, turns: i32) -> FightRow {
        FightRow {
            kind,
            act: 0,
            hp_before,
            hp_after,
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
                    row(RunNodeKind::Fight, 30, 25, true, 10),
                    row(RunNodeKind::Elite, 25, 0, false, 14),
                ],
                faults: Vec::new(),
            },
            RunOutcome {
                file,
                end: won.clone(),
                fights: vec![
                    row(RunNodeKind::Fight, 30, 30, true, 8),
                    row(RunNodeKind::MiniBoss, 22, 12, true, 20),
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
        assert!(text.contains("HP entering boss 1: mean 22.0 (n 1)"), "{text}");
        assert!(text.contains("HP entering boss 2: mean - (n 0)"), "{text}");
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
                row(RunNodeKind::Fight, 30, 28, true, 8),
                row(RunNodeKind::MiniBoss, 28, 20, true, 18),
                row(RunNodeKind::Elite, 20, 16, true, 12),
                row(RunNodeKind::MiniBoss, 16, 12, true, 16),
                row(RunNodeKind::Boss, 12, 5, true, 22),
            ],
            faults: Vec::new(),
        }];
        let sums = tally(&outcomes, &content);
        assert_eq!(sums.boss_hp, [(28, 1), (16, 1), (12, 1)]);
    }

    fn in_act(act: usize, row: FightRow) -> FightRow {
        FightRow { act, ..row }
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
                    in_act(0, row(RunNodeKind::Fight, 30, 25, true, 10)),
                    in_act(0, row(RunNodeKind::Elite, 25, 0, false, 14)),
                    in_act(0, row(RunNodeKind::Fight, 30, 30, true, 8)),
                    in_act(0, row(RunNodeKind::MiniBoss, 30, 20, true, 20)),
                ],
                faults: Vec::new(),
            },
            RunOutcome {
                file,
                end: cursed.clone(),
                fights: vec![
                    in_act(1, row(RunNodeKind::Fight, 20, 18, true, 6)),
                    in_act(1, row(RunNodeKind::Ghost, 18, 18, true, 10)),
                    in_act(1, row(RunNodeKind::MiniBoss, 18, 10, true, 16)),
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
