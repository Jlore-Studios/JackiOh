//! Scripted cards for server tests (SURFACE §8, §11.2, §11.3, M6-T4, M7).
//! Uses real engine scripts under testkit override to produce terminal states on demand:
//!  - Nonce idempotence (SPEC §9.3), hand redaction (§10.8), fold recovery (R376).
//!  - Mulligan sequencing (R265, R266, R268).
//!  - Cards: `test-hears-emotes` (MD-D29, R1127), `test-prompt-enemy` (R79, §6.2),
//!    `test-mutual-lethal` (§2.5), `test-glitch-swap` (R677), `test-glitch-void` (R679),
//!    `test-flee` (R1843002, R1843004), and `deal_random_deck` (R258).

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use indexmap::IndexMap;
use jackioh_engine::config::{DECK_SIZE, HERO_HEALTH};
use jackioh_engine::effects::{add_to_hand, choose_mode, flee, lose_health};
use jackioh_engine::game_over::end_game;
use jackioh_engine::prelude::json_as;
use jackioh_engine::script::Effect;
use jackioh_engine::state::validate_deck;
use jackioh_engine::testkit::{register_catalog, register_scripts};
use jackioh_engine::win_rates::win_rate_leader;
use jackioh_engine::wire::{GameEvent, GameOverReason, GlitchOutcome, Winner};
use jackioh_engine::{
    Action, ActionBody, CardDef, CardDefs, CardScripts, CreateGameOptions, GameState, Hook, Phase, PlayerId,
    Script, StaticFlags, begin_game, create_game, hook, mulligan_owed, reduce,
};
use serde_json::json;

/// Scripted card IDs: self prompt, enemy prompt (R79), emote listener (MD-D29, R1127),
/// lethal, mutual lethal draw (§2.5), glitch swap (R677), glitch void (R679), flee (R1843002).
pub const TEST_PROMPT_SELF: &str = "test-prompt-self";
pub const TEST_PROMPT_ENEMY: &str = "test-prompt-enemy";
pub const TEST_HEARS_EMOTES: &str = "test-hears-emotes";
pub const TEST_LETHAL: &str = "test-lethal";
pub const TEST_MUTUAL_LETHAL: &str = "test-mutual-lethal";
pub const TEST_GLITCH_SWAP: &str = "test-glitch-swap";
pub const TEST_GLITCH_VOID: &str = "test-glitch-void";
pub const TEST_FLEE: &str = "test-flee";

/// Panicking scripted card constants (R1253000).
pub const TEST_PANICS: &str = "test-panics";
pub const TEST_PANICS_AT_TURN_END: &str = "test-panics-at-turn-end";
pub const PANIC_TEXT: &str = "test-panics: the settle loop reached its cap";

/// The filler cards' id prefix: `test-card-0`, `test-card-1`, …
pub const FILLER_PREFIX: &str = "test-card-";

/// The options a scripted prompt offers; answering either resumes at `ANSWERED_STEP`, which does
/// nothing more.
pub const PROMPT_OPTIONS: [&str; 2] = ["keep", "pass"];
/// The resume step a scripted prompt's answer re-enters.
pub const ANSWERED_STEP: &str = "answered";

/// Health reduction multiple bypassing armor (R18).
const LETHAL_MULTIPLE: i32 = 100;
const FIRST_TEST_INDEX: i32 = 9001;

/// One 0-cost Spell of the test catalog.
fn test_def(id: &str, index: i32, text: &str) -> CardDef {
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{id} (server tests)"),
        "set": "Core",
        "type": "Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": { "keywords": [], "text": text },
        "radiant": { "keywords": [], "text": text },
    }))
}

/// Every card the server tests add to the real catalog: the five scripted cards, then the
/// `DECK_SIZE` fillers `fake_deck` deals from.
pub fn test_card_defs() -> Vec<CardDef> {
    let mut defs = vec![
        test_def(
            TEST_PROMPT_SELF,
            FIRST_TEST_INDEX,
            "Quickdraw. Choose one: keep or pass.",
        ),
        json_as(json!({
            "id": TEST_HEARS_EMOTES,
            "index": (FIRST_TEST_INDEX + 4).to_string(),
            "name": format!("{TEST_HEARS_EMOTES} (server tests)"),
            "set": "Core",
            "type": "Field Spell",
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 0,
            "base": { "keywords": [], "text": "Quickdraw. Hears emotes." },
            "radiant": { "keywords": [], "text": "Quickdraw. Hears emotes." },
        })),
        test_def(
            TEST_PROMPT_ENEMY,
            FIRST_TEST_INDEX + 1,
            "Quickdraw. Your opponent chooses: keep or pass.",
        ),
        test_def(
            TEST_LETHAL,
            FIRST_TEST_INDEX + 2,
            "Quickdraw. Your opponent loses the game.",
        ),
        test_def(
            TEST_MUTUAL_LETHAL,
            FIRST_TEST_INDEX + 3,
            "Quickdraw. Both heroes die.",
        ),
    ];
    let fillers = FIRST_TEST_INDEX + defs.len() as i32;
    for n in 0..DECK_SIZE {
        defs.push(test_def(
            &format!("{FILLER_PREFIX}{n}"),
            fillers + n,
            "Does nothing.",
        ));
    }
    // After the fillers, so the indexes above stay where they were.
    let glitches = fillers + DECK_SIZE;
    defs.push(test_def(TEST_GLITCH_SWAP, glitches, "Quickdraw. Glitch: swap."));
    defs.push(test_def(
        TEST_GLITCH_VOID,
        glitches + 1,
        "Quickdraw. Glitch: void.",
    ));
    // After the Glitches, so every index above stays where it was.
    defs.push(test_def(
        TEST_FLEE,
        glitches + 2,
        "Quickdraw. Escape the universe and win the game; your opponent draws.",
    ));
    defs
}

/// §6.2: "Starts in the opening hand instead of a draw".
fn quickdraw() -> Option<StaticFlags> {
    Some(StaticFlags {
        quickdraw: Some(true),
        ..StaticFlags::default()
    })
}

/// A scripted prompt (`choose_mode`, §6.3), answered by `by` (`"self"` or `"enemy"`).
fn prompt_script(by: &'static str) -> Script {
    let mut resume = IndexMap::new();
    resume.insert(ANSWERED_STEP, hook(|_ctx| Vec::new()));
    Script {
        static_flags: quickdraw(),
        cry: Some(hook(move |_ctx| {
            vec![choose_mode(json_as(json!({
                "options": PROMPT_OPTIONS,
                "step": ANSWERED_STEP,
                "prompt": "pick one",
                "by": by,
            })))]
        })),
        resume,
        ..Script::default()
    }
}

/// A Glitch forced to one outcome (`subsystems::glitch::glitch`'s swap or void arm, without its
/// draw): the public `glitched` event, then the outcome's write.
fn glitch_script(outcome: GlitchOutcome) -> Script {
    Script {
        static_flags: quickdraw(),
        cry: Some(hook(move |_ctx| {
            vec![Effect::new("test-glitch", move |ctx| {
                ctx.sink.events.push(GameEvent::Glitched {
                    player: ctx.controller,
                    outcome,
                });
                match outcome {
                    GlitchOutcome::Swap => {
                        ctx.sink.state.seat_swaps = Some(ctx.sink.state.seat_swaps.unwrap_or(0) + 1);
                    }
                    GlitchOutcome::Void => end_game(&mut ctx.sink, Winner::Draw, GameOverReason::Voided),
                    GlitchOutcome::Reset | GlitchOutcome::Boards => {
                        unreachable!("the server tests force swap and void only")
                    }
                }
            })]
        })),
        ..Script::default()
    }
}

/// The scripted cards' scripts: both faces the same.
fn test_card_scripts() -> Vec<(String, Script)> {
    let lethal = HERO_HEALTH * LETHAL_MULTIPLE;
    let mut scripts = vec![
        (TEST_PROMPT_SELF.to_string(), prompt_script("self")),
        (TEST_PROMPT_ENEMY.to_string(), prompt_script("enemy")),
        (
            TEST_HEARS_EMOTES.to_string(),
            Script {
                static_flags: Some(StaticFlags {
                    quickdraw: Some(true),
                    hears_emotes: Some(true),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            },
        ),
        (
            TEST_LETHAL.to_string(),
            Script {
                static_flags: quickdraw(),
                cry: Some(hook(move |_ctx| {
                    vec![lose_health(json_as(
                        json!({ "player": "enemy", "amount": lethal }),
                    ))]
                })),
                ..Script::default()
            },
        ),
        (
            // §2.5, §4.5: Both heroes at 0 or less in same check is a draw.
            TEST_MUTUAL_LETHAL.to_string(),
            Script {
                static_flags: quickdraw(),
                cry: Some(hook(move |_ctx| {
                    vec![
                        lose_health(json_as(json!({ "player": "self", "amount": lethal }))),
                        lose_health(json_as(json!({ "player": "enemy", "amount": lethal }))),
                    ]
                })),
                ..Script::default()
            },
        ),
    ];
    for n in 0..DECK_SIZE {
        scripts.push((format!("{FILLER_PREFIX}{n}"), Script::default()));
    }
    scripts.push((TEST_GLITCH_SWAP.to_string(), glitch_script(GlitchOutcome::Swap)));
    scripts.push((TEST_GLITCH_VOID.to_string(), glitch_script(GlitchOutcome::Void)));
    scripts.push((
        TEST_FLEE.to_string(),
        Script {
            static_flags: quickdraw(),
            cry: Some(hook(|_ctx| vec![flee(Default::default())])),
            ..Script::default()
        },
    ));
    scripts
}

/// The whole real catalog plus the test cards.
pub fn test_catalog() -> CardDefs {
    let mut defs: CardDefs = jackioh_cards::CATALOG.clone();
    for def in test_card_defs() {
        defs.insert(def.id.clone(), def);
    }
    defs
}

/// Every real card's scripts plus the test cards'.
pub fn test_scripts() -> IndexMap<String, CardScripts> {
    let mut scripts = jackioh_cards::scripts_of();
    for (id, script) in test_card_scripts() {
        scripts.insert(
            id,
            CardScripts {
                base: script.clone(),
                radiant: script,
            },
        );
    }
    scripts
}

/// Installs test catalog and scripts under testkit override (SURFACE §8).
pub fn install_test_cards() {
    jackioh_cards::register_all();
    register_catalog(test_catalog());
    register_scripts(test_scripts());
}

/// #1035: a 0-cost Quickdraw Spell whose play, while its `FoldGate` is armed, parks the fold that
/// replays it. It is not in `install_test_cards`'s catalog: `install_gated_test_cards` adds it.
pub const TEST_FOLD_GATE: &str = "test-fold-gate";

/// #1035: how long a parked fold waits to be released. Bounded, so a fold that holds the runtime
/// (the probe cannot run to release it) ends its test on order and does not hang it.
pub const FOLD_GATE_WAIT: Duration = Duration::from_secs(5);

/// #1035, #2213: parks fold until released, recording step order and pass count.
pub struct FoldGate {
    armed: AtomicBool,
    passes: AtomicUsize,
    parked: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    release: Mutex<mpsc::Receiver<()>>,
    order: Mutex<Vec<&'static str>>,
}

impl FoldGate {
    /// A disarmed gate, the sender that releases it and the receiver that says it has parked.
    pub fn new() -> (
        Arc<FoldGate>,
        mpsc::Sender<()>,
        tokio::sync::oneshot::Receiver<()>,
    ) {
        let (release, released) = mpsc::channel();
        let (parked, parks) = tokio::sync::oneshot::channel();
        let gate = Arc::new(FoldGate {
            armed: AtomicBool::new(false),
            passes: AtomicUsize::new(0),
            parked: Mutex::new(Some(parked)),
            release: Mutex::new(released),
            order: Mutex::new(Vec::new()),
        });
        (gate, release, parks)
    }

    /// The next play of `TEST_FOLD_GATE` parks, once.
    pub fn arm(&self) {
        self.armed.store(true, Ordering::SeqCst);
    }

    pub fn note(&self, step: &'static str) {
        self.order
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(step);
    }

    pub fn order(&self) -> Vec<&'static str> {
        self.order
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// #2213: how many plays of `TEST_FOLD_GATE` have run, armed or not: one for each fold of a log that plays it.
    pub fn passes(&self) -> usize {
        self.passes.load(Ordering::SeqCst)
    }

    fn pass(&self) {
        self.passes.fetch_add(1, Ordering::SeqCst);
        if !self.armed.swap(false, Ordering::SeqCst) {
            return;
        }
        let parked = self
            .parked
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        if let Some(parked) = parked {
            let _ = parked.send(());
        }
        let release = self
            .release
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _ = release.recv_timeout(FOLD_GATE_WAIT);
        drop(release);
        self.note("fold");
    }
}

/// #1035: `install_test_cards`' cards plus `TEST_FOLD_GATE`, whose play parks its fold while `gate` is
/// armed.
pub fn install_gated_test_cards(gate: Arc<FoldGate>) {
    jackioh_cards::register_all();
    let mut defs = test_catalog();
    let index = FIRST_TEST_INDEX + test_card_defs().len() as i32;
    defs.insert(
        TEST_FOLD_GATE.to_string(),
        test_def(TEST_FOLD_GATE, index, "Quickdraw. Parks a fold."),
    );
    register_catalog(defs);
    let script = Script {
        static_flags: quickdraw(),
        cry: Some(hook(move |_ctx| {
            gate.pass();
            Vec::new()
        })),
        ..Script::default()
    };
    let mut scripts = test_scripts();
    scripts.insert(
        TEST_FOLD_GATE.to_string(),
        CardScripts {
            base: script.clone(),
            radiant: script,
        },
    );
    register_scripts(scripts);
}

/// R1253000: a hook that counts its run in `runs` and panics.
fn panicking(runs: Arc<AtomicUsize>) -> Hook {
    hook(move |_ctx| {
        runs.fetch_add(1, Ordering::SeqCst);
        panic!("{PANIC_TEXT}")
    })
}

/// R1253000: `install_test_cards`' cards plus `TEST_PANICS` (its play panics) and
/// `TEST_PANICS_AT_TURN_END` (the end of its controller's turn panics), each run counted in `runs`.
pub fn install_panicking_test_cards(runs: Arc<AtomicUsize>) {
    jackioh_cards::register_all();
    let mut defs = test_catalog();
    // Past `TEST_FOLD_GATE`'s index, so the two never meet.
    let index = FIRST_TEST_INDEX + test_card_defs().len() as i32 + 1;
    defs.insert(
        TEST_PANICS.to_string(),
        test_def(TEST_PANICS, index, "Quickdraw. Panics."),
    );
    defs.insert(
        TEST_PANICS_AT_TURN_END.to_string(),
        json_as(json!({
            "id": TEST_PANICS_AT_TURN_END,
            "index": (index + 1).to_string(),
            "name": format!("{TEST_PANICS_AT_TURN_END} (server tests)"),
            "set": "Core",
            "type": "Field Spell",
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 0,
            "base": { "keywords": [], "text": "Quickdraw. End of turn: panics." },
            "radiant": { "keywords": [], "text": "Quickdraw. End of turn: panics." },
        })),
    );
    register_catalog(defs);
    let mut scripts = test_scripts();
    for (id, script) in [
        (
            TEST_PANICS,
            Script {
                static_flags: quickdraw(),
                cry: Some(panicking(Arc::clone(&runs))),
                ..Script::default()
            },
        ),
        (
            TEST_PANICS_AT_TURN_END,
            Script {
                static_flags: quickdraw(),
                end_of_turn: Some(panicking(runs)),
                ..Script::default()
            },
        ),
    ] {
        scripts.insert(
            id.to_string(),
            CardScripts {
                base: script.clone(),
                radiant: script,
            },
        );
    }
    register_scripts(scripts);
}

/// R1266000, R1329000: Cry adds card with highest win rate (#50 CN Tech).
pub const TEST_TECH: &str = "test-tech";

/// `test_catalog` plus `TEST_TECH`, past `TEST_PANICS_AT_TURN_END`'s index.
pub fn tech_test_catalog() -> CardDefs {
    let mut defs = test_catalog();
    let index = FIRST_TEST_INDEX + test_card_defs().len() as i32 + 3;
    defs.insert(
        TEST_TECH.to_string(),
        test_def(
            TEST_TECH,
            index,
            "Quickdraw. Add the card with the highest win rate to your hand.",
        ),
    );
    defs
}

/// R1266000, R1329000: `install_test_cards`' cards plus `TEST_TECH`, which reads the win-rate table
/// the fold it runs in reads (`win_rate_leader`).
pub fn install_tech_test_cards() {
    jackioh_cards::register_all();
    register_catalog(tech_test_catalog());
    let script = Script {
        static_flags: quickdraw(),
        cry: Some(hook(|_ctx| {
            win_rate_leader(TEST_TECH)
                .map_or_else(Vec::new, |id| vec![add_to_hand(json_as(json!({ "defId": id })))])
        })),
        ..Script::default()
    };
    let mut scripts = test_scripts();
    scripts.insert(
        TEST_TECH.to_string(),
        CardScripts {
            base: script.clone(),
            radiant: script,
        },
    );
    register_scripts(scripts);
}

/// R1253000: the Cry of each card panics when p1 plays it, counted in `runs`.
pub fn install_panicking_plays(cards: &[String], runs: Arc<AtomicUsize>) {
    jackioh_cards::register_all();
    let mut scripts = jackioh_cards::scripts_of();
    for id in cards {
        let Some(card) = scripts.get_mut(id) else {
            continue;
        };
        for face in [&mut card.base, &mut card.radiant] {
            let printed = face.cry.clone();
            let runs = Arc::clone(&runs);
            face.cry = Some(hook(move |ctx| {
                if ctx.controller != PlayerId::P1 {
                    return printed.as_ref().map_or_else(Vec::new, |cry| cry(ctx));
                }
                runs.fetch_add(1, Ordering::SeqCst);
                panic!("{PANIC_TEXT}")
            }));
        }
    }
    register_scripts(scripts);
}

/// A `DECK_SIZE`-card deck of test ids, with `extra` scripted cards first; each of them has
/// Quickdraw, so the opening deal puts them in the hand (up to the hand's size, R225).
pub fn fake_deck(extra: &[&str]) -> Vec<String> {
    let mut deck: Vec<String> = extra.iter().map(|id| id.to_string()).collect();
    let fillers = (DECK_SIZE as usize).saturating_sub(extra.len());
    for n in 0..fillers {
        deck.push(format!("{FILLER_PREFIX}{n}"));
    }
    deck
}

/// The real catalog's dealable cards (no token), in id order: the pool `decks_the_engine_accepts`
/// slices.
pub fn real_pool() -> Vec<String> {
    let mut pool: Vec<String> = jackioh_cards::CATALOG
        .values()
        .filter(|def| !def.token && !def.tags.iter().any(|tag| tag.as_str() == "Token"))
        .map(|def| def.id.clone())
        .collect();
    pool.sort();
    pool
}

/// Every playable catalog id (no token), in `catalog.json` order.
pub fn playable() -> Vec<String> {
    jackioh_cards::register_all();
    jackioh_cards::CATALOG
        .iter()
        .filter(|(_, def)| {
            !def.token
                && !serde_json::to_value(&def.tags)
                    .expect("tags")
                    .as_array()
                    .is_some_and(|tags| tags.contains(&json!("Token")))
        })
        .map(|(id, _)| id.clone())
        .collect()
}

/// A created, not yet begun, game on two decks (the test catalog's, which holds the real one).
pub fn new_game(seed: &str, decks: (Vec<String>, Vec<String>)) -> GameState {
    create_game(&CreateGameOptions {
        seed: seed.to_string(),
        decks,
        ..CreateGameOptions::default()
    })
}

/// A begun game: `begin_game(create_game(...))`, panicking on a refusal.
pub fn begun_game(seed: &str, decks: (Vec<String>, Vec<String>)) -> GameState {
    let begun = begin_game(&new_game(seed, decks));
    if let Some(error) = begun.error {
        panic!("begin_game refused the test game: {error}");
    }
    begun.state
}

/// Two legal, disjoint decks for the real engine playing real §8 cards.
/// Deck size is sized dynamically by growing slices until accepted (BUILD §2).
pub fn decks_the_engine_accepts(pool: &[String], seed: &str) -> (GameState, (Vec<String>, Vec<String>)) {
    let catalog = test_catalog();
    let mut refusals: Vec<String> = Vec::new();
    let mut size = 1;
    while size * 2 <= pool.len() {
        let decks = (pool[..size].to_vec(), pool[size..size * 2].to_vec());
        let checked = validate_deck(&decks.0, &catalog, "p1", DECK_SIZE)
            .and_then(|()| validate_deck(&decks.1, &catalog, "p2", DECK_SIZE));
        match checked {
            Ok(()) => return (new_game(seed, decks.clone()), decks),
            Err(error) => {
                let message = error.to_string();
                if !refusals.contains(&message) {
                    refusals.push(message);
                }
            }
        }
        size += 1;
    }
    panic!(
        "the real engine refused every deck size built from the catalog:\n  {}",
        refusals.join("\n  ")
    );
}

/// `decks_the_engine_accepts`, narrowed to decks opening straight onto both mulligans (§2.1, R265).
/// Slides second deck until deal asks nothing from start-of-game clauses (R224, R635, R151).
pub fn decks_that_open_on_the_mulligans(pool: &[String], seed: &str) -> (Vec<String>, Vec<String>) {
    let (_, (first, _)) = decks_the_engine_accepts(pool, seed);
    let size = first.len();
    let mut from = size;
    while from + size <= pool.len() {
        let decks = (first.clone(), pool[from..from + size].to_vec());
        let state = begun_game(seed, decks.clone());
        if state.phase == Phase::Mulligan && mulligan_owed(&state).len() == 2 {
            return decks;
        }
        from += 1;
    }
    panic!(
        "under seed {seed} no second deck lets the deal open on both mulligans: p1's own deal asks first (R224)"
    );
}

/// The instance ids of a seat's hand, in hand order.
pub fn hand_ids(state: &GameState, seat: PlayerId) -> Vec<String> {
    state.players[seat]
        .hand
        .iter()
        .map(|card| card.id.clone())
        .collect()
}

/// The instance id of the first card of `def_id` in a seat's hand, or `None`.
pub fn hand_card(state: &GameState, seat: PlayerId, def_id: &str) -> Option<String> {
    state.players[seat]
        .hand
        .iter()
        .find(|card| card.def_id == def_id)
        .map(|card| card.id.clone())
}

/// R265: the mulligan that keeps a seat's whole opening hand.
pub fn keep_whole_hand(state: &GameState, seat: PlayerId, nonce: &str) -> Action {
    Action::new(
        ActionBody::Mulligan {
            keep: hand_ids(state, seat),
        },
        seat,
        nonce,
    )
}

/// Both mulligans answered by keeping the whole hand, p1 first: the state at the start of turn 1,
/// for a test that is not about the mulligan.
pub fn past_the_mulligans(state: &GameState) -> GameState {
    let mut state = state.clone();
    for (seat, nonce) in [
        (PlayerId::P1, "test-mulligan-p1"),
        (PlayerId::P2, "test-mulligan-p2"),
    ] {
        if !mulligan_owed(&state).contains(&seat) {
            continue;
        }
        let answered = reduce(&state, &keep_whole_hand(&state, seat, nonce));
        if let Some(error) = answered.error {
            panic!("the {seat:?} mulligan was refused: {error}");
        }
        state = answered.state;
    }
    state
}
