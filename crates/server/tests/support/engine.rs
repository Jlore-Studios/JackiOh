//! The server tests' cards (← `apps/server/test/fakes/engine.ts`, SURFACE §8, §11.2).
//!
//! WHY THIS IS NOT A FAKE ENGINE. TS's server tests ran a scripted `EnginePort` (`createFakeEngine`)
//! because what M6-T4 and M7 prove is about the actor, the clock, the log and the results writer,
//! and each of those needs a *terminal state on demand*: `test-lethal` reaches `hero-death` in one
//! action, where the real engine would need a whole game of real cards to get there. The Rust
//! server has no engine port (SURFACE §11.3): it calls `jackioh_engine` directly. So the scripted
//! situations become **real cards**, scripted with the effects library and installed for the calling
//! test's thread with the testkit's override (`testkit::register_catalog`, `register_scripts`,
//! SURFACE §8) on top of the whole real catalog. Every test then runs the real reducer, the real
//! `view_for`, the real `fold` and the real `summarize_game`, which is the contract TS's fake could
//! only imitate:
//!
//!  - `reduce` is pure, returns `{ state, events, error? }` and refuses illegal actions itself;
//!  - a reused nonce returns the original events and does not advance the state (SPEC §9.3);
//!  - `view_for` shows the viewer's hand in full and the opponent's as a count (§10.8) — and, being
//!    the real one, it is also what a redaction test may be asserted against;
//!  - `fold({ seed, decks, log })` rebuilds the same state, so crash recovery is testable, and
//!    `summarize_game` folds the same way to a finished game's record (R376);
//!  - a prompt is state, answered by another action (§9.3);
//!  - the game always opens on the concurrent mulligan (R265), as every real game does: both seats'
//!    prompts open at once outside `pending`, either seat answers first, an answer is sealed until
//!    the other is in (R266), and a `timeout` answers only the timing-out seat's own mulligan (R268).
//!    `past_the_mulligans` answers both by keeping the whole hand, for a test that is not about them.
//!
//! The scripted cards, each a 0-cost Spell with Quickdraw (§6.2), so the opening deal always puts
//! them in their owner's hand (as TS's `fakeDeck(extra)` put its extras there):
//!  - `test-prompt-self`   opens a prompt for the player who played it;
//!  - `test-prompt-enemy`  opens a prompt for the other player (a trap firing on your turn, R79);
//!  - `test-lethal`        ends the match: the player who played it wins by `hero-death`;
//!  - `test-mutual-lethal` ends the match: both heroes die in the same check, a draw by
//!    `both-heroes-dead` (§2.5's second row — the one ending no other scripted
//!    card can reach, and the seventh of the reasons `api/results.rs` writes);
//!  - `test-glitch-swap`   a Glitch's swap (R677): `seat_swaps` goes up by one, `glitched` announces it;
//!  - `test-glitch-void`   a Glitch's void (R679): the game ends, no winner, reason `voided`.
//!
//! Their prompts are the engine's own (`choose_mode`): kind `mode`, where TS's fake opened kind
//! `target`. The filler `test-card-<n>` cards are 0-cost Spells that do nothing.
//!
//! Not ported: `createFakeEngine` itself (the real engine stands where it stood), `FAKE_HAND_SIZE`
//! (the real opening hands are `OPENING_DRAW`'s, three for p1 and four for p2), the fake's
//! `dealRandomDeck` (`actor::engine::deal_random_deck` is the real R258 deal). The two Glitch cards
//! force one outcome each where the engine's own Glitch (`subsystems::glitch`) draws it from the
//! match rng: each is a test-made `Effect` that writes exactly what that outcome writes (TS's fake
//! wrote the same two fields and events).

#![allow(dead_code)]

use indexmap::IndexMap;
use jackioh_engine::config::{DECK_SIZE, HERO_HEALTH, TURN_CAP_PLAYER_TURNS};
use jackioh_engine::effects::{choose_mode, lose_health};
use jackioh_engine::game_over::end_game;
use jackioh_engine::prelude::json_as;
use jackioh_engine::script::Effect;
use jackioh_engine::state::validate_deck;
use jackioh_engine::testkit::{register_catalog, register_scripts};
use jackioh_engine::wire::{GameEvent, GameOverReason, GlitchOutcome, Winner};
use jackioh_engine::{
    Action, ActionBody, CardDef, CardDefs, CardScripts, CreateGameOptions, GameState, Phase, PlayerId,
    Script, StaticFlags, begin_game, create_game, hook, mulligan_owed, reduce,
};
use serde_json::{Value, json};

/// Opens a prompt for the player who played it.
pub const TEST_PROMPT_SELF: &str = "test-prompt-self";
/// Opens a prompt for the other player (a trap firing on your turn, R79).
pub const TEST_PROMPT_ENEMY: &str = "test-prompt-enemy";
/// The player who played it wins by `hero-death`.
pub const TEST_LETHAL: &str = "test-lethal";
/// Both heroes die in the same check: a draw by `both-heroes-dead` (§2.5).
pub const TEST_MUTUAL_LETHAL: &str = "test-mutual-lethal";
/// A Glitch's swap (R677): the accounts now play each other's seat.
pub const TEST_GLITCH_SWAP: &str = "test-glitch-swap";
/// A Glitch's void (R679): the game ends with no winner, reason `voided`.
pub const TEST_GLITCH_VOID: &str = "test-glitch-void";

/// Mirrors `TURN_CAP_PLAYER_TURNS` (§2.5) so a test can reach the `turn-cap` ending; the real
/// engine's own cap, not a copy of it.
pub const FAKE_TURN_CAP: i32 = TURN_CAP_PLAYER_TURNS;

/// The filler cards' id prefix: `test-card-0`, `test-card-1`, …
pub const FILLER_PREFIX: &str = "test-card-";

/// The options a scripted prompt offers; answering either resumes at `ANSWERED_STEP`, which does
/// nothing more.
pub const PROMPT_OPTIONS: [&str; 2] = ["keep", "pass"];
/// The resume step a scripted prompt's answer re-enters.
pub const ANSWERED_STEP: &str = "answered";

/// How far past a hero's whole health a lethal card's loss goes, so armor, a handicap's extra
/// health or a heal along the way cannot leave the hero standing. Losing health is not damage, so
/// no Armor and no hero cap reduce it (R18).
const LETHAL_MULTIPLE: i32 = 100;

/// The scripted cards' printed indexes: far past every real catalog index, so nothing collides.
const FIRST_TEST_INDEX: i32 = 9001;

/// One 0-cost Spell of the test catalog, written as the TS fixture literal.
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

/// Every card the server tests add to the real catalog: the four scripted cards, then the
/// `DECK_SIZE` fillers `fake_deck` deals from.
pub fn test_card_defs() -> Vec<CardDef> {
    let mut defs = vec![
        test_def(
            TEST_PROMPT_SELF,
            FIRST_TEST_INDEX,
            "Quickdraw. Choose one: keep or pass.",
        ),
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
            // §2.5: "Both heroes at 0 or less in the same check" is a draw, not a win for whoever
            // struck. Both losses land inside one effect list, and the state check (§4.5) runs only
            // once the play has resolved, so it finds both heroes dead at once.
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

/// Installs the test catalog and scripts for the calling thread (the testkit's override, SURFACE
/// §8). `#[tokio::test]` runs a current-thread runtime, so the actor tasks a test spawns see them too.
/// The real catalog is registered for the process first, for anything that reads it without the
/// override.
pub fn install_test_cards() {
    jackioh_cards::register_all();
    register_catalog(test_catalog());
    register_scripts(test_scripts());
}

/// A small, stable string hash for scripted deals (FNV-1a over UTF-16 code units, as TS's
/// `charCodeAt` loop); not a random source.
pub fn seed_hash(seed: &str) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for unit in seed.encode_utf16() {
        hash ^= u32::from(unit);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
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

/// Two legal, disjoint decks for the real engine — the counterpart of `fake_deck` for the tests
/// that play real §8 cards (`engine_real.rs`, and the real-card blocks of `match_actor.rs` and
/// `recovery.rs`).
///
/// The deck size is not written here: BUILD §2 keeps `DECK_SIZE` in the engine's config, nothing
/// restates it, so the size is whatever the engine accepts: the slices grow until `validate_deck`
/// stops objecting. That loop is also an assertion. With the card catalog missing *every* size is
/// refused, so that failure surfaces here as "the real engine refused every deck size", with the
/// engine's own sentences attached, rather than as a shapeless panic inside whatever called this.
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

/// `decks_the_engine_accepts`, narrowed to the decks whose opening deal, under `seed`, opens straight
/// onto both mulligans (§2.1, R265) — what a test about the window both seats share needs from its
/// very first frame.
///
/// A deal that asks its seat something makes setup wait for the answer before it opens the mulligans
/// (R224). No cast-on-draw card is dealt (R635), so no cast asks; a start-of-game clause as its card
/// arrives in a hand still can (R151). Which cards the deal draws is the seed's to say, so whether a
/// deal asks is read off the real engine, not off card text. The first deck stays the pool's first
/// slice; the second slides along the pool a card at a time until the deal asks nothing, so a seed
/// whose deal already opened on the mulligans keeps exactly the decks `decks_the_engine_accepts` gives it.
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

/// The instance id of the first card of `def_id` in the viewer's own hand of a `view` frame's
/// `PlayerView` JSON (`view.you.hand`), or `None`.
pub fn hand_card_in_view(view: &Value, def_id: &str) -> Option<String> {
    view["you"]["hand"]
        .as_array()?
        .iter()
        .find(|card| card["defId"] == def_id)
        .and_then(|card| card["instanceId"].as_str())
        .map(str::to_string)
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
/// for a test that is not about the mulligan (TS's fake opened there unless asked not to).
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
