//! `cargo jackioh carried [--catalog V] [--card id] [--json]` (#4382, R4382000): every card's era on
//! every older catalog this build carries (R767000), played and folded back as a replay pinned to it.
//! A replay of a game from an older patch folds under that patch's cards and the scripts it kept, and
//! nothing else plays them: a current script reading a number the old definition never declared
//! panicked there (R4382000), and a panic in the browser's module is a trap the page cannot catch.
//!
//! An era (`jackioh_cards::card_eras`) is a run of carried versions over which a card's definition and
//! its covering `KEPT_SCRIPTS` row stay the same, played on its first. Each plays a fixed script under
//! `with_shipped`: the card in seat 1's hand with `SWEEP_MANA` mana, seat 1 laying a filler Unit for
//! each one its Tribute takes (one at least) and seat 2 a Vanilla, the first play of it the engine
//! lists, its prompts answered by the random policy, a turn ended on each side so its start- and
//! end-of-turn hooks run, Hit Job aimed at it while it stands as a Unit so its Death runs, then
//! `ERA_TURNS` random-policy turns. Both seats' `view_for` is read after every step, as the hosts'
//! replay shows each one (§9.5). The log is then folded as a replay pinned to the version, by
//! `cargo jackioh replay`'s own path, and must reach the live hash.
//!
//! An era fails on a panic, `ERA_STEP_BOUND` actions, a listed action the engine refuses, a stall or a
//! fold that reaches another hash, and its report names the version, the card, the stage (`setup`,
//! `play`, `prompt`, `turn`, `death`, `random`, `fold`) and what it found, with the command that plays
//! that era again. The `coverage:` line counts the eras whose card the script never played; it never
//! fails. No invariant monitor runs: kept scripts did what today's invariants forbid
//! (`crates/cards/tests/cross/invariants.rs`, I10 on v0.3.16's). Play and fold run the same script, so
//! a kept row one version early cannot show here.
//!
//! The whole sweep runs daily; a pull request's `fuzz` gate plays every era of the cards it changed
//! (`play_changed`), up to `CHANGED_ERAS_MAX`.
//!
//! ```text
//! cargo jackioh carried                                       # every era of every carried catalog
//! cargo jackioh carried --catalog v0.4.5                      # the eras that start on one version
//! cargo jackioh carried --card classic-039                    # one card's eras
//! cargo jackioh carried --catalog v0.4.5 --card classic-039   # one card on one version
//! cargo test -p jackioh-tools --bin jackioh carried                            # a pull request's
//! cargo test -p jackioh-tools --bin jackioh carried -- --ignored --nocapture   # the daily one
//! ```

use std::cell::Cell;
use std::collections::BTreeSet;
use std::panic::{self, AssertUnwindSafe};
use std::time::Instant;

use anyhow::bail;
use jackioh_cards::versions::VERSIONS_KEY;
use jackioh_cards::{CATALOG, card_eras, carried_versions, register_all};
use jackioh_engine::config::{HUMAN_HANDICAP, OPENING_DRAW, VERSIONS_SCHEME};
use jackioh_engine::replay::{ReplayInput, fold, hash_state};
use jackioh_engine::subsystems;
use jackioh_engine::testkit::{preview_every_set, shipped_override};
use jackioh_engine::{
    AI_PLAYOUT_STEP_CAP, Action, ActionBody, CardDef, GameState, Handicap, PLAYER_IDS, PerPlayerOpt,
    PlayerId, Rng, Selection, find_instance, legal_actions, made_by_a_rule, mulligan_owed, reduce,
    seat_to_act, shipped_catalog, tribute_cost_of, view_for, with_shipped,
};
use rayon::prelude::*;
use serde_json::{Value, json};

use crate::agent::panic_message;
use crate::classify::family;
use crate::golden::carrier_of;

/// The turns the fixed script spans: seat 1's and seat 2's first, which lay their fillers, then the
/// card's turn and the turn Hit Job is aimed at it.
const SCRIPT_TURNS: i32 = 4;

/// The random-policy turns an era plays after its fixed script.
const ERA_TURNS: i32 = 4;

/// The actions one era may take: the AI's playout cap for each turn of the script and the random
/// ones, so it fires only on a turn that never ends.
const ERA_STEP_BOUND: usize = (SCRIPT_TURNS + ERA_TURNS) as usize * AI_PLAYOUT_STEP_CAP;

/// Each seat's mana bonus and cap (R181): every turn starts with this much, past `MAX_MANA`, so a card
/// that costs up to it is played on the script's turn with mana to spare.
const SWEEP_MANA: i32 = 10;

/// The most eras a pull request plays for the cards it changed (#4382). A card has three eras on
/// average, so a change of a few cards plays them all and a wide one a stride sample. The whole sweep,
/// 2,563 eras of 799 cards over 88 versions, took 23.8 s on four cores as the daily test (`cargo test`,
/// 9 ms an era) and 20.1 s from the release build (`cargo jackioh carried`), so the cap costs about 3 s.
const CHANGED_ERAS_MAX: usize = 300;

/// The eras the `coverage:` line names, the first of those whose card was never played.
const COVERAGE_SHOWN: usize = 20;

/// Core's Vanilla, with no text: seat 2's Unit, and seat 1's first filler unless the card is it.
const VANILLA: &str = "core-008";

/// Hit Job, "Destroy a Unit": seat 2 casts it at the card while it stands as a Unit, so its Death runs.
const HIT_JOB: &str = "core-016";

/// What fills both decks past the card and Hit Job, in this order: plain Units every carried catalog
/// holds, laid by seat 1 to pay the card's Tribute.
const FILLERS: [&str; 4] = [VANILLA, "core-011", "core-019", "core-020"];

/// One card on a run of carried catalogs over which its definition and kept script stay the same,
/// played on the first of them; or on the one version `--catalog` names.
#[derive(Clone, Debug, PartialEq)]
pub struct Era {
    /// The version it is played on, its era's first.
    pub version: String,
    /// Its era's last version, which plays as `version` does.
    pub through: String,
    pub card: String,
}

impl Era {
    /// `card` on `version` alone.
    fn one(version: &str, card: &str) -> Era {
        Era {
            version: version.to_string(),
            through: version.to_string(),
            card: card.to_string(),
        }
    }

    /// The version and the card, and the era's last version when it spans more than one.
    fn name(&self) -> String {
        if self.through == self.version {
            return format!("{} {}", self.version, self.card);
        }
        format!(
            "{} {} (its era through {})",
            self.version, self.card, self.through
        )
    }
}

/// Whether the sweep plays `def`: a card a deck may hold, so no token and no card only a rule makes
/// (R674, R1857001). A token runs when its carrier makes it.
fn sweepable(def: &CardDef) -> bool {
    !def.is_token() && !made_by_a_rule(def)
}

/// `card`'s eras, oldest first.
fn eras_of(card: &str) -> Vec<Era> {
    card_eras(card)
        .into_iter()
        .map(|(first, last)| Era {
            version: first.to_string(),
            through: last.to_string(),
            card: card.to_string(),
        })
        .collect()
}

/// Every sweepable card's eras, in catalog order. Builds no catalog.
fn all_eras() -> Vec<Era> {
    register_all();
    CATALOG
        .iter()
        .filter(|(_, def)| sweepable(def))
        .flat_map(|(id, _)| eras_of(id))
        .collect()
}

/// The eras of every card in a family of `changed` (a token its own family, any other card itself and
/// its parts, R1365000), a token's through the card whose `refs` name it (R279), in `all_eras`' order.
fn eras_holding(changed: &[String]) -> Vec<Era> {
    let families: BTreeSet<String> = changed
        .iter()
        .map(|id| match CATALOG.get(id) {
            Some(def) if def.is_token() => {
                carrier_of(id, def).map_or_else(|_| family(id), |carrier| family(&carrier))
            }
            _ => family(id),
        })
        .collect();
    if families.is_empty() {
        return Vec::new();
    }
    all_eras()
        .into_iter()
        .filter(|era| families.contains(&family(&era.card)))
        .collect()
}

/// `eras` as they are, or every `k`th of them when more than `max`, the fewest `k` that leaves at most `max`.
fn at_most(eras: Vec<Era>, max: usize) -> Vec<Era> {
    if eras.len() <= max {
        return eras;
    }
    let stride = eras.len().div_ceil(max.max(1));
    eras.into_iter().step_by(stride).collect()
}

/// One era that failed.
#[derive(Debug)]
pub struct EraFailure {
    era: Era,
    stage: &'static str,
    message: String,
}

impl EraFailure {
    /// The version, the card, the stage and what it found, and the command that plays the era alone.
    fn report(&self) -> String {
        format!(
            "carried {}: {}: {}\n  reproduce: cargo jackioh carried --catalog {} --card {}",
            self.era.name(),
            self.stage,
            self.message,
            self.era.version,
            self.era.card,
        )
    }

    fn json(&self) -> Value {
        json!({
            "version": self.era.version,
            "through": self.era.through,
            "card": self.era.card,
            "stage": self.stage,
            "message": self.message,
        })
    }
}

/// What an era that passed did: the actions it took, and whether its card was played.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Played {
    actions: usize,
    played: bool,
}

/// The seed of an era's game, from its version and card alone.
fn era_seed(version: &str, card: &str) -> String {
    format!("jackioh-carried-{version}-{card}")
}

/// An era's setup, as a replay takes it: seat 1's deck the card and every filler that is not it, seat
/// 2's Hit Job and the fillers its opening draw (§2.1) leaves room for, each seat drawing its whole deck
/// (R182's extra opening cards) so the card starts in hand, and each on `SWEEP_MANA`.
fn era_setup(version: &str, card: &str) -> ReplayInput {
    let deck = |first: &str| -> Vec<String> {
        std::iter::once(first)
            .chain(FILLERS.into_iter().filter(|filler| *filler != first))
            .map(str::to_string)
            .collect()
    };
    let p2_draw = usize::try_from(OPENING_DRAW[1]).unwrap_or_default();
    let decks: (Vec<String>, Vec<String>) = (deck(card), deck(HIT_JOB).into_iter().take(p2_draw).collect());
    let handicap = |deck: &[String], seat: usize| {
        let size = i32::try_from(deck.len()).unwrap_or_default();
        Handicap {
            deck_size: size,
            mana_bonus: SWEEP_MANA,
            mana_cap: SWEEP_MANA,
            extra_opening_cards: (size - OPENING_DRAW[seat]).max(0),
            ..HUMAN_HANDICAP
        }
    };
    ReplayInput {
        seed: era_seed(version, card),
        handicaps: Some(PerPlayerOpt {
            p1: Some(handicap(&decks.0, 0)),
            p2: Some(handicap(&decks.1, 1)),
        }),
        decks,
        log: Vec::new(),
        ..ReplayInput::default()
    }
}

/// One era's game in progress: its state, the actions taken and the policy's stream.
struct EraRun {
    state: GameState,
    log: Vec<Action>,
    policy: Rng,
}

impl EraRun {
    /// Takes `body` as `seat`: refused past `ERA_STEP_BOUND` or when the engine refuses it; then both
    /// seats' views read, as a replay's host reads them for each step (§9.5).
    fn step(&mut self, seat: PlayerId, body: ActionBody) -> Result<(), String> {
        if self.log.len() >= ERA_STEP_BOUND {
            return Err(format!(
                "{ERA_STEP_BOUND} actions taken and the game not over (turn {}, phase \"{}\")",
                self.state.turn, self.state.phase
            ));
        }
        let action = Action::new(body, seat, format!("carried-{}", self.log.len()));
        let result = reduce(&self.state, &action);
        if let Some(error) = &result.error {
            return Err(format!(
                "reduce refused \"{}\": {error}",
                action.action_type().as_str()
            ));
        }
        self.state = result.state;
        self.log.push(action);
        self.view();
        Ok(())
    }

    /// Both seats' views of the state now.
    fn view(&self) {
        for seat in PLAYER_IDS {
            view_for(&self.state, seat);
        }
    }

    /// The seat the game waits on.
    fn seat(&self) -> PlayerId {
        seat_to_act(&self.state).unwrap_or(self.state.active)
    }

    /// The policy's action for the seat the game waits on; a stall when it has none.
    fn chosen(&mut self) -> Result<(PlayerId, ActionBody), String> {
        let seat = self.seat();
        match subsystems::choose_action(&self.state, seat, &mut self.policy) {
            Some(body) => Ok((seat, body)),
            None => Err(format!(
                "stall: no action for {seat} while the game is live (turn {}, phase \"{}\")",
                self.state.turn, self.state.phase
            )),
        }
    }

    /// Answers every open prompt with the policy's choice.
    fn settle_prompts(&mut self) -> Result<(), String> {
        while self.state.result.is_none() && self.state.pending.is_some() {
            let (seat, body) = self.chosen()?;
            self.step(seat, body)?;
        }
        Ok(())
    }

    /// One scripted step, after every open prompt is answered: `seat` takes the first action the engine
    /// lists that `pick` accepts, when the game is live and waits on `seat`. Whether it took one.
    fn scripted(
        &mut self,
        seat: PlayerId,
        pick: impl Fn(&GameState, &ActionBody) -> bool,
    ) -> Result<bool, String> {
        self.settle_prompts()?;
        if self.state.result.is_some() || seat_to_act(&self.state) != Some(seat) {
            return Ok(false);
        }
        let listed = legal_actions(&self.state, seat)
            .into_iter()
            .find(|action| pick(&self.state, action));
        match listed {
            Some(body) => self.step(seat, body).map(|()| true),
            None => Ok(false),
        }
    }

    /// `seat` plays its first card of `def_id` as the engine first lists it, with any targets, or with
    /// targets that are all the card `target` when one is named. Whether it played.
    fn play_first(&mut self, seat: PlayerId, def_id: &str, target: Option<&str>) -> Result<bool, String> {
        self.scripted(seat, |state, action| match action {
            ActionBody::Play {
                instance_id,
                targets,
                ..
            } => {
                find_instance(state, instance_id).is_some_and(|card| card.def_id == def_id)
                    && target.is_none_or(|aimed| {
                        targets.as_deref().is_some_and(|targets| {
                            !targets.is_empty()
                                && targets.iter().all(|selected| {
                                    matches!(selected, Selection::Instance { instance_id } if instance_id == aimed)
                                })
                        })
                    })
            }
            _ => false,
        })
    }

    /// `seat` ends its turn, when the engine lists it.
    fn end_turn(&mut self, seat: PlayerId) -> Result<bool, String> {
        self.scripted(seat, |_, action| matches!(action, ActionBody::EndTurn))
    }

    /// Every open mulligan answered keeping the whole hand.
    fn keep_all(&mut self) -> Result<(), String> {
        for seat in mulligan_owed(&self.state) {
            let kept = legal_actions(&self.state, seat)
                .into_iter()
                .filter_map(|action| match action {
                    ActionBody::Mulligan { keep } => Some(keep),
                    _ => None,
                })
                .max_by_key(Vec::len);
            if let Some(keep) = kept {
                self.step(seat, ActionBody::Mulligan { keep })?;
            }
        }
        Ok(())
    }

    /// The instance id of seat 1's first card of `def_id` in hand.
    fn in_hand(&self, def_id: &str) -> Option<String> {
        self.state.players[PlayerId::P1]
            .hand
            .iter()
            .find(|card| card.def_id == def_id)
            .map(|card| card.id.clone())
    }
}

/// An era's game, under the carried catalog the caller set: the fixed script, then `ERA_TURNS` random
/// turns, `stage` naming the part in flight. The log, the live hash and whether the card was played.
fn play_game(
    input: &ReplayInput,
    card: &str,
    stage: &Cell<&'static str>,
) -> Result<(Vec<Action>, String, bool), String> {
    let mut run = EraRun {
        state: fold(input).state,
        log: Vec::new(),
        policy: Rng::new(&format!("{}-policy", input.seed), 0),
    };
    run.view();
    run.keep_all()?;
    // Seat 1 lays as many fillers as the card's Tribute takes, and one when it takes none.
    let tribute = run
        .in_hand(card)
        .and_then(|id| find_instance(&run.state, &id).map(|instance| tribute_cost_of(&run.state, instance)))
        .unwrap_or_default();
    let laid = usize::try_from(tribute.max(1)).unwrap_or(1);
    for filler in input.decks.0.iter().skip(1).take(laid) {
        run.play_first(PlayerId::P1, filler, None)?;
    }
    run.end_turn(PlayerId::P1)?;
    run.play_first(PlayerId::P2, VANILLA, None)?;
    run.end_turn(PlayerId::P2)?;

    stage.set("play");
    run.settle_prompts()?;
    let instance = run.in_hand(card);
    let played = run.play_first(PlayerId::P1, card, None)?;
    stage.set("prompt");
    run.settle_prompts()?;
    stage.set("turn");
    run.end_turn(PlayerId::P1)?;
    run.settle_prompts()?;
    stage.set("death");
    if let Some(instance) = &instance {
        run.play_first(PlayerId::P2, HIT_JOB, Some(instance))?;
    }
    run.settle_prompts()?;
    stage.set("turn");
    run.end_turn(PlayerId::P2)?;
    run.settle_prompts()?;

    stage.set("random");
    let until = run.state.turn + ERA_TURNS;
    while run.state.result.is_none() && run.state.turn < until {
        let (seat, body) = run.chosen()?;
        run.step(seat, body)?;
    }
    let live = hash_state(&run.state);
    Ok((run.log, live, played))
}

/// The era's game as a stored replay: its setup and log, pinned to `version` by `catalogVersion`, with
/// today's `versions` marker so the name is read as it is (R2532000), not through `aliases.json`.
fn game_json(input: &ReplayInput, log: &[Action], version: &str) -> Value {
    let mut game = serde_json::to_value(ReplayInput {
        log: log.to_vec(),
        ..input.clone()
    })
    .unwrap_or_else(|error| panic!("an era's replay input does not serialize: {error}"));
    game["catalogVersion"] = json!(version);
    game[VERSIONS_KEY] = json!(VERSIONS_SCHEME);
    game
}

/// Folds `game` as `cargo jackioh replay` does (R767000): it must fold with no refused action to the
/// hash `live` it reached.
fn fold_check(game: Value, live: &str) -> Result<(), String> {
    let answer =
        crate::replay::replay_json(game).map_err(|error| format!("the replay is refused: {error:#}"))?;
    let hash = answer["hash"].as_str().unwrap_or_default();
    let errors = &answer["errors"];
    if hash != live || errors.as_array().is_none_or(|errors| !errors.is_empty()) {
        return Err(format!(
            "the replay pinned to it folds to hash {hash}, the live game reached {live}; refused actions {errors}"
        ));
    }
    Ok(())
}

/// Plays one era: its card on the carried catalog of its version (the testkit's, while a test plants
/// some), then its log folded back. An era whose catalog lacks the card, or holds it as no sweepable
/// card, passes having played nothing.
fn play_era(era: &Era) -> Result<Played, EraFailure> {
    register_all();
    // R1420: a carried catalog may hold a set that has not shipped, so this thread previews every set.
    let _preview = preview_every_set();
    let (version, card) = (era.version.as_str(), era.card.as_str());
    let stage = Cell::new("setup");
    let failed = |stage: &'static str, message: String| EraFailure {
        era: era.clone(),
        stage,
        message,
    };
    let Some(shipped) = shipped_catalog(version) else {
        return Err(failed(
            "setup",
            "this build does not carry the catalog".to_string(),
        ));
    };
    if !shipped.defs().get(card).is_some_and(sweepable) {
        return Ok(Played {
            actions: 0,
            played: false,
        });
    }
    let input = era_setup(version, card);
    let caught = panic::catch_unwind(AssertUnwindSafe(|| -> Result<Played, String> {
        let (log, live, played) = with_shipped(Some(shipped), || play_game(&input, card, &stage))?;
        stage.set("fold");
        fold_check(game_json(&input, &log, version), &live)?;
        Ok(Played {
            actions: log.len(),
            played,
        })
    }));
    match caught {
        Ok(result) => result.map_err(|message| failed(stage.get(), message)),
        Err(payload) => Err(failed(
            stage.get(),
            format!("panic: {}", panic_message(&*payload)),
        )),
    }
}

/// Plays every era, in `eras`' order: across the cores, or on this thread while a test planted its
/// catalogs, which live in this thread's testkit registry that rayon's workers cannot see.
fn play_eras(eras: &[Era]) -> Vec<Result<Played, EraFailure>> {
    register_all();
    if shipped_override().is_some() {
        return eras.iter().map(play_era).collect();
    }
    eras.par_iter().map(play_era).collect()
}

/// What a set of eras came to.
#[derive(Debug, Default)]
struct Tally {
    failures: Vec<EraFailure>,
    /// The eras that passed without playing their card.
    unplayed: Vec<Era>,
    actions: usize,
}

/// Each era's result, `results` in `eras`' order.
fn tally(eras: &[Era], results: Vec<Result<Played, EraFailure>>) -> Tally {
    let mut tally = Tally::default();
    for (era, result) in eras.iter().zip(results) {
        match result {
            Ok(played) => {
                tally.actions += played.actions;
                if !played.played {
                    tally.unplayed.push(era.clone());
                }
            }
            Err(failure) => tally.failures.push(failure),
        }
    }
    tally
}

/// Every failure's report, one after another.
fn report(failures: &[EraFailure]) -> String {
    failures
        .iter()
        .map(EraFailure::report)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The `coverage:` line: how many of `of` eras never played their card, the first `COVERAGE_SHOWN` named.
fn coverage(unplayed: &[Era], of: usize) -> String {
    let shown: Vec<String> = unplayed.iter().take(COVERAGE_SHOWN).map(Era::name).collect();
    let more = unplayed.len().saturating_sub(COVERAGE_SHOWN);
    let tail = if more > 0 {
        format!(", … (+{more} more)")
    } else {
        String::new()
    };
    format!(
        "coverage: {} of {of} eras never played their card (no legal play of it){}{}{tail}",
        unplayed.len(),
        if shown.is_empty() { "" } else { ": " },
        shown.join(", "),
    )
}

/// Plays `eras`, the changed cards' (`total` of them before the cap): prints how many it played, or that
/// there were none, and, when an era failed, its report, and fails.
fn play_changed_eras(eras: Vec<Era>, total: usize) -> anyhow::Result<()> {
    if eras.is_empty() {
        println!("changed cards (carried catalogs): none, no changed card has a carried era");
        return Ok(());
    }
    let tally = tally(&eras, play_eras(&eras));
    println!(
        "changed cards (carried catalogs): played {} of {total} eras ({} actions), {} never played their card, {} failed",
        eras.len(),
        tally.actions,
        tally.unplayed.len(),
        tally.failures.len()
    );
    if !tally.failures.is_empty() {
        println!("{}", report(&tally.failures));
        bail!(
            "{} of {} changed carried eras failed",
            tally.failures.len(),
            eras.len()
        );
    }
    Ok(())
}

/// Plays every carried era of the cards a pull request changed (#4382), up to `CHANGED_ERAS_MAX`, for
/// the bare `fuzz` gate; fails when one did.
pub(crate) fn play_changed(changed: &[String]) -> anyhow::Result<()> {
    register_all();
    let holding = eras_holding(changed);
    let total = holding.len();
    play_changed_eras(at_most(holding, CHANGED_ERAS_MAX), total)
}

/// `cargo jackioh carried [--catalog V] [--card id] [--json]` (#4382).
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Play only this carried catalog: the eras that start on it, or with `--card` that card on it, the
    /// command a failure's report names.
    #[arg(long)]
    pub catalog: Option<String>,
    /// Play only this card's eras, or with `--catalog` that one.
    #[arg(long)]
    pub card: Option<String>,
    /// Print the result as one JSON object: `eras`, `played`, `unplayed`, `actions` and every failure.
    #[arg(long)]
    pub json: bool,
}

/// The eras `args` names: every era, those starting on `--catalog`, `--card`'s, or the one pair.
fn named_eras(args: &Args) -> anyhow::Result<Vec<Era>> {
    if let Some(version) = &args.catalog
        && !carried_versions().contains(&version.as_str())
    {
        bail!("--catalog {version}: this build does not carry it as an older catalog (R767000)");
    }
    if let Some(card) = &args.card
        && !CATALOG.get(card).is_some_and(sweepable)
    {
        bail!("--card {card}: no card of the catalog a deck may hold");
    }
    Ok(match (&args.catalog, &args.card) {
        (None, None) => all_eras(),
        (Some(version), None) => all_eras()
            .into_iter()
            .filter(|era| era.version == *version)
            .collect(),
        (None, Some(card)) => eras_of(card),
        (Some(version), Some(card)) => {
            if !jackioh_cards::catalog_defs_at(version).is_some_and(|defs| defs.contains_key(card)) {
                bail!("--catalog {version} holds no {card}");
            }
            vec![Era::one(version, card)]
        }
    })
}

/// Plays the eras `args` names; prints how many it played, the coverage and, when an era failed, its
/// report, and fails (exit 1).
pub fn run(args: Args) -> anyhow::Result<()> {
    register_all();
    let eras = named_eras(&args)?;
    let started = Instant::now();
    let tally = tally(&eras, play_eras(&eras));
    let played = eras.len() - tally.unplayed.len() - tally.failures.len();
    if args.json {
        let unplayed: Vec<Value> = tally
            .unplayed
            .iter()
            .map(|era| json!({ "version": era.version, "through": era.through, "card": era.card }))
            .collect();
        let failures: Vec<Value> = tally.failures.iter().map(EraFailure::json).collect();
        let answer = json!({
            "eras": eras.len(),
            "played": played,
            "unplayed": unplayed,
            "actions": tally.actions,
            "failures": failures,
        });
        println!("{}", serde_json::to_string_pretty(&answer)?);
    } else {
        let cards: BTreeSet<&str> = eras.iter().map(|era| era.card.as_str()).collect();
        let versions: BTreeSet<&str> = eras.iter().map(|era| era.version.as_str()).collect();
        println!(
            "carried catalogs: {} eras of {} cards over {} versions, {played} played their card ({} actions, {:.1} s), {} failed",
            eras.len(),
            cards.len(),
            versions.len(),
            tally.actions,
            started.elapsed().as_secs_f64(),
            tally.failures.len()
        );
        println!("{}", coverage(&tally.unplayed, eras.len()));
        if !tally.failures.is_empty() {
            println!("{}", report(&tally.failures));
        }
    }
    if !tally.failures.is_empty() {
        bail!("{} of {} carried eras failed", tally.failures.len(), eras.len());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use jackioh_engine::effects::{DamageEffectArgs, TargetSpec, damage};
    use jackioh_engine::scripts::registered_entry;
    use jackioh_engine::testkit::{
        CardScripts, IndexMap, Script, ShippedCatalog, clear_overrides, hook, register_shipped,
        registered_catalog,
    };

    use super::*;

    /// R4382000: the eras whose replays panicked before the ruling, each reading a number its carried
    /// definition never declared: Big D-fender's `armor`, The True Sheep's `setCost`, Outbreak's
    /// `drawPer` and Reoccurring Dream's `chance`.
    const PANICKED: [(&str, &str); 5] = [
        ("v0.2.5f", "core-001"),
        ("v0.3.1", "meditative-088"),
        ("v0.3.2b", "meditative-088"),
        ("v0.4.5", "classic-039"),
        ("v0.4.5", "core-023"),
    ];

    /// The card the planted tests carry with a Death of their own: Core's 4-mana 7/7, which Hit Job
    /// destroys on the script's turn.
    const PLANTED: &str = "core-025";

    /// The carried catalog the planted tests register on their thread.
    const PLANTED_VERSION: &str = "v-planted";

    /// Plants `PLANTED_VERSION`: the build's definitions, and `PLANTED`'s own scripts with `death` on
    /// both faces.
    fn plant(death: fn() -> Script) {
        register_all();
        let entry = registered_entry(PLANTED).expect("the planted card has a script");
        let mut scripts = IndexMap::new();
        scripts.insert(
            PLANTED.to_string(),
            CardScripts {
                base: Script {
                    death: death().death,
                    ..entry.base.clone()
                },
                radiant: Script {
                    death: death().death,
                    ..entry.radiant.clone()
                },
            },
        );
        register_shipped(vec![ShippedCatalog::new(
            PLANTED_VERSION,
            registered_catalog().clone(),
            scripts,
        )]);
    }

    #[test]
    fn r4382000_the_four_old_replays_play_and_fold() {
        let mut problems = Vec::new();
        for (version, card) in PANICKED {
            match play_era(&Era::one(version, card)) {
                Ok(played) if !played.played => problems.push(format!("{version} {card}: never played")),
                Ok(_) => {}
                Err(failure) => problems.push(failure.report()),
            }
        }
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }

    #[test]
    fn the_script_plays_a_plain_card_and_folds_it() {
        register_all();
        let newest = *carried_versions().last().expect("a carried version");
        let played =
            play_era(&Era::one(newest, VANILLA)).unwrap_or_else(|failure| panic!("{}", failure.report()));
        assert!(played.played, "{newest}: Vanilla was never played");
        assert!(played.actions > 0, "{newest}: no action taken");
    }

    #[test]
    fn a_planted_kept_script_that_panics_fails_naming_the_version_the_card_and_the_stage() {
        plant(|| Script {
            death: Some(hook(|_| panic!("planted death"))),
            ..Script::default()
        });
        let failure = play_era(&Era::one(PLANTED_VERSION, PLANTED)).expect_err("a Death that panics passed");
        let text = failure.report();
        assert_eq!(failure.stage, "death", "{text}");
        for needle in [
            PLANTED_VERSION,
            PLANTED,
            "planted death",
            "cargo jackioh carried --catalog v-planted --card core-025",
        ] {
            assert!(text.contains(needle), "the report lacks {needle:?}: {text}");
        }
        // The changed cards' run plays it on this thread, and fails naming it.
        let error = play_changed_eras(vec![Era::one(PLANTED_VERSION, PLANTED)], 1)
            .expect_err("a changed card's failing era passed");
        let error = format!("{error:#}");
        assert!(error.contains("1 of 1 changed carried eras failed"), "{error}");
        clear_overrides();
    }

    /// The planted Death's calls: the first, in the live game, does nothing; every later one, in the
    /// fold, hits the enemy hero, so the fold reaches another hash.
    static DEATHS: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn a_fold_that_reaches_another_hash_fails_at_the_fold_stage() {
        plant(|| Script {
            death: Some(hook(|_| {
                if DEATHS.fetch_add(1, Ordering::SeqCst) == 0 {
                    return vec![];
                }
                vec![damage(DamageEffectArgs {
                    to: TargetSpec::EnemyHero,
                    amount: 1,
                    flags: Default::default(),
                })]
            })),
            ..Script::default()
        });
        let failure = play_era(&Era::one(PLANTED_VERSION, PLANTED)).expect_err("a fold that diverges passed");
        let text = failure.report();
        assert_eq!(failure.stage, "fold", "{text}");
        assert!(text.contains("folds to hash"), "{text}");
        assert!(
            DEATHS.load(Ordering::SeqCst) >= 2,
            "the Death ran {} times",
            DEATHS.load(Ordering::SeqCst)
        );
        clear_overrides();
    }

    #[test]
    fn the_eras_follow_the_cards_changes_and_the_changed_ones_stop_at_the_cap() {
        register_all();
        let carried = carried_versions();
        let outbreak = eras_holding(&["classic-039".to_string()]);
        assert!(!outbreak.is_empty());
        assert!(
            outbreak.iter().all(|era| era.card == "classic-039"),
            "{outbreak:?}"
        );
        assert!(
            outbreak.iter().any(|era| era.version == "v0.4.5"),
            "v0.4.5 runs a newer script than v0.4.4b's kept one: {outbreak:?}"
        );
        assert!(eras_holding(&[]).is_empty());
        // A token's eras are its carrier's: the Ghoul Token is Adaptive UI's (R279).
        let ghoul = eras_holding(&["core-t-ghoul".to_string()]);
        assert!(
            !ghoul.is_empty() && ghoul.iter().all(|era| era.card == "core-074"),
            "{ghoul:?}"
        );
        play_changed(&[]).expect("no changed card plays nothing");
        let all = all_eras();
        assert!(all.len() > carried.len(), "{} eras", all.len());
        assert!(all.iter().all(|era| carried.contains(&era.version.as_str())));
        let capped = at_most(all.clone(), CHANGED_ERAS_MAX);
        assert!((1..=CHANGED_ERAS_MAX).contains(&capped.len()), "{}", capped.len());
        assert_eq!(at_most(outbreak.clone(), usize::MAX), outbreak);
    }

    #[test]
    fn the_command_refuses_an_unknown_catalog_or_card() {
        let args = |catalog: Option<&str>, card: Option<&str>| Args {
            catalog: catalog.map(str::to_string),
            card: card.map(str::to_string),
            json: false,
        };
        assert!(run(args(Some("v9.9.9"), None)).is_err());
        assert!(run(args(Some(jackioh_cards::catalog_version()), None)).is_err());
        assert!(run(args(None, Some("no-such-card"))).is_err());
        assert!(
            run(args(None, Some("core-t-rush"))).is_err(),
            "a token is played by its carrier"
        );
        run(args(Some("v0.4.6"), Some("classic-039"))).expect("one era");
    }

    /// The whole sweep: 23.8 s on four cores, beside `CHANGED_ERAS_MAX`.
    #[test]
    #[ignore = "daily: every card era of every carried catalog played and folded; r4382000's pinned eras, the planted panic and divergence, and fuzz's changed cards' eras hold it per pull request"]
    fn the_whole_sweep_of_carried_eras_plays_and_folds() {
        let eras = all_eras();
        let started = Instant::now();
        let tally = tally(&eras, play_eras(&eras));
        println!(
            "{} eras, {} actions, {} failed, {:.1} s",
            eras.len(),
            tally.actions,
            tally.failures.len(),
            started.elapsed().as_secs_f64()
        );
        println!("{}", coverage(&tally.unplayed, eras.len()));
        assert!(tally.failures.is_empty(), "{}", report(&tally.failures));
    }
}
