//! `cargo jackioh fused-pairs [--pair <body>+<text>] [--day N] [--pairs N]` (#2381): each card whose text
//! has a Death, a trigger, a start- or end-of-turn hook, an aura or a fused hook (Field Spells and Traps
//! too: Wormhole and Job Application are no Units) fused onto each fusable Unit, and played until the
//! board settles.
//!
//! Why: some of the worst bugs of a month were one card fused into another, and a random game meets a
//! given pair by luck. #1562 (R1121000): a fused Book of Stats and Autocomplete replayed without end in
//! one turn. #998: a fusion grew to 288 cards and a fuzz seed ran out of memory. #2243: Wormhole's text
//! on a Unit kept games from ending (R1884003), a fused Job Application at 0 health never settled
//! (R1884004) and a fused copier's targeting cost panicked (R1850000). #2356 (R2197005): Floodgates and
//! Job Application fused, and "the board did not settle". The fuzz's only deliberate fusion is one pair
//! of Units on a manufactured last board (`fuzz.rs`'s `manufactured_board`), which games read only
//! through the few cards that bring one back; this walks the pairs on purpose.
//!
//! A pair plays like this. The body, a `FUSABLE_UNITS` card (no token, no Immutable card: R23), stands
//! on seat 1's board with a Vanilla beside it on seat 2's; the text is fused onto it as R77 has it, the
//! body keeping its instance, and `fused_into` runs (R179 rebuilds the same card from its id). Then a
//! fixed script: seat 1 casts a Spell, ends the turn, seat 2 destroys the fused Unit if its Spell may
//! reach it, ends the turn, seat 1 ends the next. Then a random-policy run, seeded from the pair, of
//! `PAIR_TURNS` more turns. Every action runs under the invariant monitor (I1 to I10, as the fuzz checks
//! them) and the fuzz's growth bounds, and the pair fails on a panic, an engine cap, an invariant breach,
//! runaway growth, a prompt nobody can answer or `PAIR_STEP_BOUND` actions: a turn that never ends.
//! Both ids, the seed and the command that plays the pair alone are in the report.
//!
//! Two samples. A day's sample, `DAILY_PAIRS` of the pairs drawn from the day's number, is the daily
//! run's (`the_days_sample_of_fused_pairs_settles`, `#[ignore = "daily: …"]`): the whole walk is too
//! much compute to play each day, and a new sample each day reaches them all over time. A pull request
//! plays a fixed stride sample of `PR_PAIRS`, the pinned pairs of #2356 and R1884004, and a planted text
//! that loops on its own death, which must fail.
//!
//! The clock only picks which pairs a day plays (`today`): a pair's game comes from its ids alone, so
//! `--pair` plays a failure again on any day. A failing pair is fixed, or filed and put on `KNOWN_OPEN`
//! with its open issue; the list only shrinks, as the fuzz's `POOL_EXCLUSIONS` does.
//!
//! ```text
//! cargo jackioh fused-pairs                                    # today's sample
//! cargo jackioh fused-pairs --day 20755 --pairs 500            # another day's, another size
//! cargo jackioh fused-pairs --pairs 100000                     # every pair
//! cargo jackioh fused-pairs --pair traininggrounds-030+stinkyscape-112
//! cargo test -p jackioh-tools --bin jackioh fused_pairs                        # a pull request's
//! cargo test -p jackioh-tools --bin jackioh fused_pairs -- --ignored --nocapture   # the daily one
//! ```

use std::panic::{self, AssertUnwindSafe};
use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, bail};
use jackioh_cards::{CATALOG, register_all};
use jackioh_engine::scripts::registered_entry;
use jackioh_engine::subsystems::{self, FuseArgs};
use jackioh_engine::testkit::{
    EngineSink, InvariantMonitor, create_invariant_monitor, json, new_instance, preview_every_set, scenario,
};
use jackioh_engine::{
    AI_PLAYOUT_STEP_CAP, Action, ActionBody, AttackHealth, GameEvent, GameState, PLAYER_IDS, PlayerId, Rng,
    Script, Selection, Zone, find_instance, legal_actions, reduce, seat_to_act,
};
use rayon::prelude::*;

use crate::agent::panic_message;
use crate::fuzz::{FUSABLE_UNITS, acting_card, growth_breach, is_immutable, weigh};

/// The random-policy turns a pair plays after its fixed script.
const PAIR_TURNS: i32 = 6;

/// The actions one pair may take: `PAIR_TURNS` turns of the AI's playout cap each. It fires only on a
/// turn that never ends, like R1884003's loop; a game that ends sooner never reaches it.
const PAIR_STEP_BOUND: usize = PAIR_TURNS as usize * AI_PLAYOUT_STEP_CAP;

/// The pairs a day's sample plays (3.6% of them at the catalog of #2381): the whole walk is too much
/// compute for a day.
const DAILY_PAIRS: usize = 2_000;

/// The seconds in a day, which `today` counts the clock in.
const SECONDS_PER_DAY: u64 = 86_400;

/// What a body printed at 0 health stands with, the way an X Unit is summoned with one: without it Classic+
/// #69 Buff Billy and the Dormant Starship die in the setup, before anything is fused onto them.
const STANDING_STATS: AttackHealth = AttackHealth { attack: 1, health: 1 };

/// Seat 1's Spell in the fixed script: Stockpile, which takes no target.
const P1_SPELL: &str = "core-005";

/// Seat 2's Spell in the fixed script: Hit Job, "Destroy a Unit", cast at the fused Unit.
const P2_SPELL: &str = "core-016";

/// The card on seat 2's board and in both hands: Core's Vanilla, with no text of its own.
const VANILLA: &str = "core-008";

/// What both libraries hold, so a draw has a card to take.
const LIBRARY: [&str; 4] = ["core-008", "core-011", "core-019", "core-005"];

/// A pair the sweep knows fails, and the open issue that owns its fix. An entry names that issue, and
/// the fix removes the entry: the list only shrinks, as the fuzz's `POOL_EXCLUSIONS` does. The pair
/// leaves `all_pairs`, so no sample plays it.
pub struct KnownOpen {
    /// The Unit the text is fused onto.
    pub body: &'static str,
    /// The card whose text is fused.
    pub text: &'static str,
    /// The open issue that owns the fix.
    pub issue: u32,
    /// What the pair does.
    pub why: &'static str,
}

/// Every pair known to fail, each with its open issue.
pub const KNOWN_OPEN: &[KnownOpen] = &[];

/// Whether `script` has a text that a fusion carries onto a Unit: a Death, a trigger, a start- or
/// end-of-turn hook, an aura or a fused hook.
fn has_fused_text(script: &Script) -> bool {
    script.death.is_some()
        || script.start_of_turn.is_some()
        || script.end_of_turn.is_some()
        || script.aura.is_some()
        || script.fused_into.is_some()
        || !script.triggers.is_empty()
}

/// The cards whose text a pair fuses: every card with a fused text on either face, a Spell, a Trap, a
/// Field Spell or a token included, and no Immutable one (R23). Sorted by catalog id.
pub static FUSED_TEXTS: LazyLock<Vec<String>> = LazyLock::new(|| {
    register_all();
    let mut texts: Vec<String> = CATALOG
        .iter()
        .filter(|(_, def)| !is_immutable(def))
        .filter(|(id, _)| {
            registered_entry(id)
                .is_some_and(|entry| has_fused_text(&entry.base) || has_fused_text(&entry.radiant))
        })
        .map(|(id, _)| id.clone())
        .collect();
    texts.sort();
    texts
});

/// Every pair, body-major, but the `KNOWN_OPEN` ones.
fn all_pairs() -> Vec<(String, String)> {
    FUSABLE_UNITS
        .iter()
        .flat_map(|body| FUSED_TEXTS.iter().map(move |text| (body.clone(), text.clone())))
        .filter(|(body, text)| {
            !KNOWN_OPEN
                .iter()
                .any(|open| open.body == body && open.text == text)
        })
        .collect()
}

/// `size` of the pairs, in the order `day` shuffles them to: one day's sample, the same on every call.
fn daily_sample(day: u64, size: usize) -> Vec<(String, String)> {
    let mut rng = Rng::new(&format!("jackioh-fused-pairs-day-{day}"), 0);
    rng.shuffle(&all_pairs()).into_iter().take(size).collect()
}

/// Today's number: the days since the epoch. The one reading of the clock, which only picks the
/// pairs a day plays.
fn today() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() / SECONDS_PER_DAY)
}

/// The seed of the pair's game, from its ids alone.
fn pair_seed(body: &str, text: &str) -> String {
    format!("jackioh-fused-pair-{body}+{text}")
}

/// A stage a pair failed in, and what it found.
type Broke = (&'static str, String);

/// One pair that failed.
#[derive(Debug)]
pub struct PairFailure {
    body: String,
    text: String,
    stage: &'static str,
    message: String,
}

impl PairFailure {
    /// Both ids, the seed, the stage and what it found, and the command that plays the pair alone.
    fn report(&self) -> String {
        format!(
            "fused pair {} + {} (seed \"{}\"): {}: {}\n  reproduce: cargo jackioh fused-pairs --pair {}+{}",
            self.body,
            self.text,
            pair_seed(&self.body, &self.text),
            self.stage,
            self.message,
            self.body,
            self.text,
        )
    }
}

/// R77: `text` fused onto the card `body_id` on the field, which keeps its instance, and its
/// `fused_into` runs. The fused card's id, or `None` when the fusion is refused.
fn fuse_onto(state: &mut GameState, body_id: &str, text: &str) -> Option<String> {
    let body = find_instance(state, body_id)?.clone();
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let made = {
        let text_card = new_instance(
            &mut *state,
            text,
            body.controller,
            Zone::Gone {
                player: body.controller,
            },
        );
        let mut sink = EngineSink::new(&mut *state, &mut events, &mut rng);
        subsystems::fuse(
            &mut sink,
            FuseArgs {
                ingredients: vec![body.clone(), text_card],
                target: Some(body),
                ..FuseArgs::default()
            },
        )
    };
    state.rng_cursor = rng.cursor();
    made.map(|card| card.id)
}

/// One pair's game in progress: its state, the invariant monitor beside it, the policy's stream and the
/// actions taken.
struct PairRun {
    state: GameState,
    monitor: InvariantMonitor,
    policy: Rng,
    actions: usize,
}

impl PairRun {
    /// Takes `body` as `seat`, under every check: the step bound, the monitor before and after, the
    /// engine's own refusal and the growth bounds.
    fn step(&mut self, seat: PlayerId, body: ActionBody) -> Result<(), Broke> {
        if self.actions >= PAIR_STEP_BOUND {
            return Err((
                "step bound",
                format!(
                    "{PAIR_STEP_BOUND} actions taken and the game not over (turn {}, phase \"{}\")",
                    self.state.turn, self.state.phase
                ),
            ));
        }
        if let Some(first) = self.monitor.before(&self.state, seat, &body).first() {
            return Err(("invariant", first.clone()));
        }
        let card = acting_card(&self.state, &body);
        let action = Action::new(body, seat, format!("fused-pair-{}", self.actions));
        let action_type = action.action_type().as_str().to_string();
        let result = reduce(&self.state, &action);
        if let Some(error) = &result.error {
            return Err(("play", format!("reduce refused \"{action_type}\": {error}")));
        }
        self.state = result.state;
        self.actions += 1;
        let mut broken = self.monitor.after(&result.events, &self.state);
        broken.extend(self.monitor.hidden(&self.state));
        broken.extend(self.monitor.unlisted(&self.state));
        if let Some(first) = broken.first() {
            return Err(("invariant", first.clone()));
        }
        let weights = weigh(&result.events, &self.state);
        match growth_breach(&weights, &self.state, self.actions, &action_type, card.as_deref()) {
            Some(message) => Err(("growth", message)),
            None => Ok(()),
        }
    }

    /// The seat the game waits on.
    fn seat(&self) -> PlayerId {
        seat_to_act(&self.state).unwrap_or(self.state.active)
    }

    /// The policy's action for the seat the game waits on; a stall when it has none.
    fn chosen(&mut self) -> Result<(PlayerId, ActionBody), Broke> {
        let seat = self.seat();
        match subsystems::choose_action(&self.state, seat, &mut self.policy) {
            Some(body) => Ok((seat, body)),
            None => Err((
                "stall",
                format!(
                    "no action for {seat} while the game is live (turn {}, phase \"{}\")",
                    self.state.turn, self.state.phase
                ),
            )),
        }
    }

    /// Answers every open prompt with the policy's choice.
    fn settle_prompts(&mut self) -> Result<(), Broke> {
        while self.state.result.is_none() && self.state.pending.is_some() {
            let (seat, body) = self.chosen()?;
            self.step(seat, body)?;
        }
        Ok(())
    }

    /// `seat` casts `spell` at nothing, or at the fused Unit `fused_id`: the first play the engine lists
    /// that does; nothing when it lists none (the Spell is not in hand, or its text reaches no such target).
    fn cast(&mut self, seat: PlayerId, spell: &str, fused_id: &str) -> Result<(), Broke> {
        let chosen = legal_actions(&self.state, seat).into_iter().find(|action| match action {
            ActionBody::Play { instance_id, targets, .. } => {
                find_instance(&self.state, instance_id).is_some_and(|card| card.def_id == spell)
                    && targets.as_deref().unwrap_or_default().iter().all(
                        |target| matches!(target, Selection::Instance { instance_id } if instance_id == fused_id),
                    )
            }
            _ => false,
        });
        match chosen {
            Some(body) => self.step(seat, body),
            None => Ok(()),
        }
    }
}

/// Plays `text` fused onto `body`: its fixed script, then `PAIR_TURNS` random-policy turns. `prepare`
/// changes the state before the fusion (a test plants a card there). The actions taken.
fn play_pair_with(body: &str, text: &str, prepare: fn(&mut GameState)) -> Result<usize, PairFailure> {
    register_all();
    // R1420: the fuzz pool holds the sets that have not shipped, so this thread previews every set.
    let _preview = preview_every_set();
    let failed = |(stage, message): Broke| PairFailure {
        body: body.to_string(),
        text: text.to_string(),
        stage,
        message,
    };
    let seed = pair_seed(body, text);
    let caught = panic::catch_unwind(AssertUnwindSafe(|| -> Result<usize, Broke> {
        let standing = CATALOG.get(body).and_then(|def| def.base.health).unwrap_or(1) <= 0;
        let entry = if standing {
            json!({ "def": body, "statsOverride": STANDING_STATS })
        } else {
            json!(body)
        };
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "field": [entry], "hand": [P1_SPELL, VANILLA], "library": LIBRARY },
            "p2": { "field": [VANILLA], "hand": [P2_SPELL, VANILLA], "library": LIBRARY },
        }));
        prepare(s.state_mut());
        let body_id = s.unit_at(PlayerId::P1, 1).id;
        let fused_id = fuse_onto(s.state_mut(), &body_id, text)
            .ok_or(("fuse", "subsystems.fuse refused the pair".to_string()))?;

        // The monitor starts with nothing entered, so the cards the setup placed are entered here, the
        // way crates/cards/tests/cross/invariants.rs seeds it.
        let mut state = s.state().clone();
        let turn = state.turn;
        let mut on_field: Vec<String> = Vec::new();
        for player in PLAYER_IDS {
            let side = &mut state.players[player];
            for card in side
                .units
                .iter_mut()
                .flatten()
                .flatten()
                .chain(side.backrow.iter_mut().flatten())
            {
                card.summoned_turn = Some(turn);
                on_field.push(card.id.clone());
            }
        }
        let mut monitor = create_invariant_monitor(&state);
        for id in &on_field {
            monitor.enter(id);
        }
        let mut run = PairRun {
            state,
            monitor,
            policy: Rng::new(&format!("{seed}-policy"), 0),
            actions: 0,
        };

        let script = [
            (PlayerId::P1, Some(P1_SPELL)),
            (PlayerId::P1, None),
            (PlayerId::P2, Some(P2_SPELL)),
            (PlayerId::P2, None),
            (PlayerId::P1, None),
        ];
        for (seat, spell) in script {
            run.settle_prompts()?;
            if run.state.result.is_some() || seat_to_act(&run.state) != Some(seat) {
                continue;
            }
            match spell {
                Some(spell) => run.cast(seat, spell, &fused_id)?,
                None => run.step(seat, ActionBody::EndTurn)?,
            }
        }
        let until = run.state.turn + PAIR_TURNS;
        while run.state.result.is_none() && run.state.turn < until {
            let (seat, body) = run.chosen()?;
            run.step(seat, body)?;
        }
        Ok(run.actions)
    }));
    match caught {
        Ok(played) => played.map_err(failed),
        Err(payload) => Err(failed(("panic", panic_message(&*payload)))),
    }
}

/// Plays `text` fused onto `body` as the sweep does: the actions taken, or why the pair failed.
fn play_pair(body: &str, text: &str) -> Result<usize, PairFailure> {
    play_pair_with(body, text, |_| {})
}

/// Plays every pair, across the cores: the failures in the order of `pairs` and the actions taken.
fn play_pairs(pairs: &[(String, String)]) -> (Vec<PairFailure>, usize) {
    register_all();
    let played: Vec<Result<usize, PairFailure>> = pairs
        .par_iter()
        .map(|(body, text)| play_pair(body, text))
        .collect();
    let mut failures = Vec::new();
    let mut actions = 0;
    for result in played {
        match result {
            Ok(taken) => actions += taken,
            Err(failure) => failures.push(failure),
        }
    }
    (failures, actions)
}

/// Every failure's report, one after another.
fn report(failures: &[PairFailure]) -> String {
    failures
        .iter()
        .map(PairFailure::report)
        .collect::<Vec<_>>()
        .join("\n")
}

/// `cargo jackioh fused-pairs [--pair <body>+<text>] [--day N] [--pairs N]` (#2381).
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Play this one pair alone, `<body>+<text>`, the command a failure's report names.
    #[arg(long, conflicts_with_all = ["day", "pairs"])]
    pub pair: Option<String>,
    /// The day whose sample is drawn, as a number of days since the epoch: today's by default. The
    /// same day draws the same pairs.
    #[arg(long)]
    pub day: Option<u64>,
    /// How many pairs the sample holds: `DAILY_PAIRS` by default. A number past the pairs there are
    /// plays every one.
    #[arg(long)]
    pub pairs: Option<usize>,
}

/// Plays the day's sample, or with `--pair` one pair; prints how many it played, the known-open
/// pairs and, when a pair failed, its report, and fails (exit 1).
pub fn run(args: Args) -> anyhow::Result<()> {
    register_all();
    let sample = match &args.pair {
        Some(pair) => {
            let (body, text) = pair.split_once('+').context("--pair takes <body>+<text>")?;
            if !FUSABLE_UNITS.iter().any(|id| id == body) {
                bail!("--pair: \"{body}\" is no fusable Unit (no token, no Immutable card)");
            }
            if !FUSED_TEXTS.iter().any(|id| id == text) {
                bail!("--pair: \"{text}\" has no Death, trigger, turn hook, aura or fused hook to fuse");
            }
            vec![(body.to_string(), text.to_string())]
        }
        None => {
            let day = args.day.unwrap_or_else(today);
            println!("day {day}");
            daily_sample(day, args.pairs.unwrap_or(DAILY_PAIRS))
        }
    };
    let (failures, actions) = play_pairs(&sample);
    println!(
        "fused pairs: played {} of {} ({} bodies x {} texts), {actions} actions, {} failed",
        sample.len(),
        all_pairs().len(),
        FUSABLE_UNITS.len(),
        FUSED_TEXTS.len(),
        failures.len(),
    );
    for open in KNOWN_OPEN {
        println!(
            "known open: {}+{} (#{}: {})",
            open.body, open.text, open.issue, open.why
        );
    }
    if !failures.is_empty() {
        println!("{}", report(&failures));
        bail!("{} of {} fused pairs failed", failures.len(), sample.len());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use jackioh_engine::effects::{DestroyArgs, SummonArgs, TargetSpec, destroy, summon};
    use jackioh_engine::testkit::{
        CardDef, CardScripts, CardType, hook, json_as, register_scripts, registered_scripts,
    };

    use super::*;

    /// The pairs a pull request plays: a stride of the whole walk, which a card joining a text or a body
    /// moves, and which takes about 2.5 s on one thread.
    const PR_PAIRS: usize = 32;

    /// Every `stride`th pair of `all_pairs`, `PR_PAIRS` of them.
    fn pr_sample() -> Vec<(String, String)> {
        let pairs = all_pairs();
        let stride = (pairs.len() / PR_PAIRS).max(1);
        pairs.into_iter().step_by(stride).take(PR_PAIRS).collect()
    }

    /// The pairs that failed once: #2356 (R2197005), Floodgates and Job Application fused, found by the
    /// AI sweep `sweep:hard:traininggrounds-014:6`; and #1884 (R1884004), a fused Job Application at 0
    /// health that never settled.
    const PINNED: [(&str, &str); 2] = [
        ("traininggrounds-030", "stinkyscape-112"),
        ("meditative-047", "stinkyscape-112"),
    ];

    /// A text whose Death summons its own card back and destroys it again, planted by the test.
    const LOOPING: &str = "test-fused-pairs-loop";

    #[test]
    fn the_pull_requests_sample_of_fused_pairs_settles() {
        let sample = pr_sample();
        assert_eq!(
            sample.len(),
            PR_PAIRS,
            "the stride sample is short: {}",
            all_pairs().len()
        );
        let (failures, actions) = play_pairs(&sample);
        assert!(failures.is_empty(), "{}", report(&failures));
        assert!(actions > 0, "the sample took no action");
    }

    #[test]
    fn the_pinned_pairs_settle() {
        for (body, text) in PINNED {
            if let Err(failure) = play_pair(body, text) {
                panic!("{}", failure.report());
            }
        }
    }

    /// The planted text's Unit def, as `cross::fixture` makes one, a 1/1.
    fn planted_def() -> CardDef {
        let face = json!({ "attack": 1, "health": 1, "keywords": [], "text": LOOPING });
        json_as(json!({
            "id": LOOPING,
            "index": LOOPING,
            "name": LOOPING,
            "set": "Core",
            "type": CardType::Unit,
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 0,
            "base": face.clone(),
            "radiant": face,
        }))
    }

    /// `LOOPING`'s def in the state, as the setup of the planted pair.
    fn plant(state: &mut GameState) {
        state.transient_defs.insert(LOOPING.to_string(), planted_def());
    }

    #[test]
    fn a_planted_text_that_loops_on_its_own_death_fails_naming_both_ids_and_the_seed() {
        register_all();
        // The fixture's script goes in the testkit's thread-local registry, so this test stays on its
        // own thread: no rayon.
        let script = Script {
            death: Some(hook(|ctx| {
                let Some(me) = ctx.self_.clone() else {
                    return vec![];
                };
                vec![
                    summon(SummonArgs {
                        instance: Some(TargetSpec::Instance {
                            instance_id: me.id.clone(),
                        }),
                        ..SummonArgs::default()
                    }),
                    destroy(DestroyArgs {
                        target: TargetSpec::Instance { instance_id: me.id },
                    }),
                ]
            })),
            ..Script::default()
        };
        let mut scripts = registered_scripts();
        scripts.insert(
            LOOPING.to_string(),
            CardScripts {
                base: script.clone(),
                radiant: script,
            },
        );
        register_scripts(scripts);

        let failure =
            play_pair_with(VANILLA, LOOPING, plant).expect_err("a text that loops on its own death settled");
        assert_eq!(
            failure.stage,
            "panic",
            "the loop failed some other way: {}",
            failure.report()
        );
        let text = failure.report();
        for needle in [
            VANILLA,
            LOOPING,
            pair_seed(VANILLA, LOOPING).as_str(),
            "cargo jackioh fused-pairs --pair core-008+test-fused-pairs-loop",
        ] {
            assert!(text.contains(needle), "the report lacks {needle:?}: {text}");
        }
    }

    #[test]
    fn the_bodies_and_texts_follow_their_rules() {
        for body in ["traininggrounds-030", "meditative-047"] {
            assert!(FUSABLE_UNITS.iter().any(|id| id == body), "{body} is no body");
        }
        for id in ["core-019", "core-t-rush"] {
            assert!(!FUSABLE_UNITS.iter().any(|body| body == id), "{id} is a body");
        }
        for text in ["stinkyscape-112", "stinkyscape-139"] {
            assert!(FUSED_TEXTS.iter().any(|id| id == text), "{text} is no text");
        }
        assert!(
            !FUSED_TEXTS.iter().any(|id| id == VANILLA),
            "{VANILLA} has no text to fuse"
        );
    }

    #[test]
    fn known_open_entries_name_an_issue_and_a_pair_the_sweep_has() {
        let pairs = all_pairs();
        for (at, open) in KNOWN_OPEN.iter().enumerate() {
            let pair = format!("{}+{}", open.body, open.text);
            assert!(
                open.issue > 0 && !open.why.is_empty(),
                "{pair} names no open issue and no why"
            );
            assert!(
                FUSABLE_UNITS.iter().any(|id| id == open.body),
                "{pair}: no such body"
            );
            assert!(
                FUSED_TEXTS.iter().any(|id| id == open.text),
                "{pair}: no such text"
            );
            assert!(
                !KNOWN_OPEN[..at]
                    .iter()
                    .any(|earlier| earlier.body == open.body && earlier.text == open.text),
                "{pair} is listed twice"
            );
            assert!(
                !pairs
                    .iter()
                    .any(|(body, text)| body == open.body && text == open.text),
                "{pair} is on the list and still in the walk"
            );
        }
    }

    #[test]
    fn the_days_sample_is_its_days_alone() {
        let day = 20_000;
        let sample = daily_sample(day, 10);
        assert_eq!(
            sample,
            daily_sample(day, 10),
            "a day's sample moved between two calls"
        );
        assert_ne!(sample, daily_sample(day + 1, 10), "two days drew the same sample");
        let mut distinct = sample.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), 10, "a pair was drawn twice: {sample:?}");
    }

    #[test]
    fn the_command_plays_one_pair_and_refuses_a_malformed_one() {
        let one = Args {
            pair: Some("traininggrounds-030+stinkyscape-112".into()),
            day: None,
            pairs: None,
        };
        run(one).expect("the pinned pair");
        let malformed = Args {
            pair: Some(VANILLA.into()),
            day: None,
            pairs: None,
        };
        assert!(run(malformed).is_err(), "a --pair with no text was taken");
    }

    #[test]
    #[ignore = "daily: DAILY_PAIRS fused pairs, a new random sample each day; the pull request's stride sample, the pinned pairs and the planted loop hold the sweep per pull request"]
    fn the_days_sample_of_fused_pairs_settles() {
        let day = today();
        let sample = daily_sample(day, DAILY_PAIRS);
        let (failures, actions) = play_pairs(&sample);
        println!(
            "day {day}: played {} of {} fused pairs, {actions} actions",
            sample.len(),
            all_pairs().len()
        );
        assert!(failures.is_empty(), "day {day}:\n{}", report(&failures));
    }
}
