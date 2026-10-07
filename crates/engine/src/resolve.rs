//! Running a card's script: build a context, apply the effects it returns, then finish the card.
//! The full play pipeline of §10.5 arrives with M3; this is the part M1's draw and turn loop need.
//!
//! Port of `packages/engine/src/resolve.ts` (part 3, SURFACE §4, §6.5). TS's `EngineSink = { state,
//! events, rng }` is part 1's `script::EngineSink` (SURFACE §6.5), re-exported here under its TS path.
//! TS's cast-driver registry (`registerCastDriver`, set by `playSteps.ts` at module scope to break an
//! import cycle) goes (SURFACE §6.6): `cast_card` calls the play pipeline's driver directly.

use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::Value;

use crate::script::{Effect, EffectContext, EffectExpand, EffectPart, Hook, Memo, Script};
use crate::state::{CardInstance, GameState, find_instance, find_instance_mut};
use crate::wire::{CardType, PlayerId, Selection, ZoneName};

/// TS `export type EngineSink = { state; events; rng }` (`resolve.ts:13`), which part 1 declared in
/// `script.rs` beside the context that derefs to it (SURFACE §6.5).
pub use crate::script::EngineSink;

/// TS `HookOptions`: what a run's context is built with, beyond the card itself.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HookOptions {
    pub controller: Option<PlayerId>,
    pub targets: Option<Vec<Selection>>,
    pub modes: Option<Vec<String>>,
    pub data: Option<IndexMap<String, Value>>,
}

/// Where a played card's Cry keeps its player's mana as the play began (`EffectContext.manaBeforePlay`):
/// in the card's own data, which every continuation of the Cry carries (`prompts.resumeSelf`,
/// `work.parkWork`), so the number survives a pause and a JSON round trip as the rest of the data does.
pub const MANA_BEFORE_PLAY_KEY: &str = "__manaBeforePlay";

/// TS `makeContext(sink, self, options)`. The context borrows the sink for as long as it lives (it is
/// the sink, plus this run's facts); `self_` is copied in as the card stands now.
pub fn make_context<'b>(
    sink: &'b mut EngineSink<'_>,
    self_: Option<&CardInstance>,
    options: HookOptions,
) -> EffectContext<'b> {
    let mana_before_play = options
        .data
        .as_ref()
        .and_then(|data| data.get(MANA_BEFORE_PLAY_KEY))
        .and_then(Value::as_i64)
        .map(|mana| mana as i32);
    // R136: this script's own event window opens where the sink's list stands right now. Every
    // context is built here, so this is the one place the mark has to be taken; a resumed
    // continuation calls back through here and opens a fresh window on the action it resumes in,
    // and what its first pass summoned in an earlier action comes with it (`EffectContext.summoned`).
    let events_from = sink.events.len();
    // R174: the stay every card on the field has as this script begins.
    let exits_from = Some(crate::stays::exit_mark(sink.state));
    // R98: a run that began in the resolving zone is the resolving card's while it stays there.
    let self_resolving = if self_.is_some_and(|card| card.zone.z() == ZoneName::Resolving) {
        Some(true)
    } else {
        None
    };
    let controller = options
        .controller
        .or(self_.map(|card| card.controller))
        .unwrap_or(sink.state.active);
    let radiant = self_.is_some_and(|card| card.radiant);
    // B2.7, B3.4: the X it was played for, as Degrade and Upgrade have tuned it (`tuning.xOf`).
    let x = self_.map_or(0, crate::tuning::x_of);
    let embiggened = self_.and_then(|card| card.embiggened).unwrap_or(false);
    EffectContext {
        sink: sink.reborrow(),
        events_from,
        exits_from,
        chosen_from: None,
        event_stay: None,
        summoned: None,
        self_resolving,
        controller,
        self_: self_.cloned(),
        def_id: None,
        radiant,
        targets: options.targets.unwrap_or_default(),
        modes: options.modes.unwrap_or_default(),
        x,
        embiggened,
        data: options.data.unwrap_or_default(),
        mana_before_play,
    }
}

/// Apply an effect list in order. R216: once a state check inside the list has ended the game — a
/// cast on draw that killed its own hero, halfway through #5 Stockpile's "draw 2; heal your hero 2" —
/// the rest of the list does not resolve: the game is over (§2.5), and nothing happens after it.
pub fn apply_effects(effects: &[Effect], ctx: &mut EffectContext<'_>) {
    for effect in effects {
        if ctx.state.result.is_some() {
            return;
        }
        (effect.apply)(ctx);
    }
}

/// A part of a composed list (`Effect.expand`), built when the list reaches it rather than when the
/// list is made (R102). Applied on its own it builds and applies its effects in one go; inside
/// `prompts.applyResumable` it runs as a nested list a prompt can pause.
pub fn lazy_part(
    kind: &'static str,
    expand: impl Fn(&mut EffectContext<'_>, &Memo) -> EffectPart + Send + Sync + 'static,
) -> Effect {
    let expand: EffectExpand = Arc::new(expand);
    let building = Arc::clone(&expand);
    Effect {
        kind,
        apply: Arc::new(move |ctx: &mut EffectContext<'_>| {
            let part = building(ctx, &None);
            apply_effects(&part.effects, ctx);
        }),
        expand: Some(expand),
    }
}

/// TS `HookName`: the effect-list hooks `run_hook` may run by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HookName {
    Cry,
    Death,
    StartOfGame,
    StartOfTurn,
    StartOfOpponentTurn,
    EndOfTurn,
    Activate,
    OnPlayHook,
}

impl HookName {
    /// The literal, as TS names the `Script` key.
    pub fn as_str(self) -> &'static str {
        match self {
            HookName::Cry => "cry",
            HookName::Death => "death",
            HookName::StartOfGame => "startOfGame",
            HookName::StartOfTurn => "startOfTurn",
            HookName::StartOfOpponentTurn => "startOfOpponentTurn",
            HookName::EndOfTurn => "endOfTurn",
            HookName::Activate => "activate",
            HookName::OnPlayHook => "onPlayHook",
        }
    }
}

/// `script[name]`. `Script.activate` is not ported (SURFACE §7.2), so "activate" names nothing.
fn hook_of(script: &Script, name: HookName) -> Option<Hook> {
    script.hook_named(name.as_str()).cloned()
}

/// Run one of a card's hooks. `cry` is also a spell's on-resolve hook (§10.9).
///
/// This is the *non-resumable* runner: the whole effect list is applied here and now, so it is only
/// ever right on a path where no effect can open a prompt — a caller that has already established
/// there is nothing to ask. Any path whose effects may ask something uses `prompts.runHookResumable`
/// (a trigger, a play's or a cast's Cry, an activate), and a start-of-game clause, which may return a
/// Choose like any other list (§10.9), runs through `prompts.runStartOfGame` (R151, R113).
pub fn run_hook(sink: &mut EngineSink<'_>, instance: &CardInstance, name: HookName, options: HookOptions) {
    let Some(hook) = hook_of(&crate::scripts::script_of(sink.state, instance), name) else {
        return;
    };
    let mut ctx = make_context(sink, Some(instance), options);
    let effects = hook(&mut ctx);
    apply_effects(&effects, &mut ctx);
}

/// `afterward: "exile"` (R453).
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum CastAfterward {
    Exile,
}

/// How a cast is made, beyond R70's defaults (B5 E12; R452, R453):
///
/// - `random`: every choice its caster would make is made at random (`randomCast.ts`, R452).
/// - `targetEnemies`: each target pick is aimed by its declaration (R656: harm at enemies, help at
///   friends, when one is legal); a card carrying the `targetEnemies` enchantment (E39) is cast so
///   whoever casts it.
/// - `afterward: "exile"`: a Spell goes to exile rather than its graveyard once it has resolved
///   (Classic #56 Spell Tyrant's "then exile them", R453) — §10.5 step 7's landing (`echo.exileOnLanding`).
///
/// TS `HookOptions & { random?; targetEnemies?; afterward? }`: `HookOptions`' fields, then these.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CastOptions {
    pub controller: Option<PlayerId>,
    pub targets: Option<Vec<Selection>>,
    pub modes: Option<Vec<String>>,
    pub data: Option<IndexMap<String, Value>>,
    pub random: Option<bool>,
    pub target_enemies: Option<bool>,
    pub afterward: Option<CastAfterward>,
}

/// R70: "a cast is free and counts as a play for every rule that counts or reacts to plays, with
/// cost paid 0", fires the card's script, and "a cast Spell does use Twinspell's Echo". So a cast IS
/// §10.5's pipeline, entered after the two steps a cast skips — step 1 has nothing to validate, since
/// the effect chose the card, and step 2 pays nothing — and run through the same named steps as a
/// play from hand: step 3's Gifted Program hook (#64), step 4's placement, counters and `cardPlayed`
/// with the Echo gained as it is played (R178), step 5's Quickstriker (#38) and /fullsend Combo draw
/// (#78) before the card's own script, step 6's repeats and step 7's landing and `cardResolved`.
///
/// Its own path used to stop short of that: it skipped steps 3 and 5's granted parts outright, and
/// owed steps 6 and 7 to `state.work` even when nothing had paused (against R117), so a cast-on-draw
/// Spell's Echo repeat resolved after the draw had already repeated (§2.4). The one difference from a
/// play is where the resolution loop runs: a cast happens inside some other effect — §2.4's draw,
/// #95's recursion — so it settles nothing itself. Its step 4 is still a window, as a play's is (R70,
/// R17): every event so far reaches the traps there (`triggers.dispatchPending`), while the other
/// triggers they wake wait for that effect's loop; and §2.4's chain runs the state check after each
/// cast-on-draw cast (§4.5, R59).
///
/// Used by Cast on draw (§2.4), by Call to Chaos, and by the cast verbs of B5 E12 (`effects/cast.ts`):
/// a card from a graveyard or any pile, a new card of a named definition, a random catalog card.
///
/// The play pipeline (`play_steps.rs`, which owns §10.5) is called directly: TS reached it through a
/// driver `playSteps.ts` registered at module scope (`registerCastDriver`), which Rust needs not.
pub fn cast_card(sink: &mut EngineSink<'_>, instance: &CardInstance, options: CastOptions) {
    crate::play_steps::cast_through_pipeline(sink, instance, options);
}

/// §5.1 and R155: "Spells with 'End of turn: add this back to your hand' are flagged
/// `returnToHandAtEndOfTurn` when played and return from the graveyard at the end of that turn".
/// This is that flag, written at the one moment §10.5 describes — step 7, as the Spell lands in the
/// graveyard — so the card carries its own answer and nothing has to infer it later.
///
/// The three conditions are step 7's own sentence, in order. It must be a Spell: a Unit with an
/// `endOfTurn` hook (#13 Jlockeed Shredder-10) that was played and died on the turn it was played is
/// in the graveyard too, and it must not return from there (R153). Its resolving face must declare
/// an end-of-turn return, which for a Spell is exactly an `endOfTurn` hook — #23 Reoccurring Dream,
/// #24 Efficiency Dividend and #31 KY's Math Equation are the only three in Core, and the only
/// `endOfTurn` a Spell can have, since R153 gives a graveyard no other hook. And it must have
/// reached the **graveyard**: a Spell that says "exile this on play" (#39 Recycling Initiative) is
/// not in the graveyard when step 7 is done, so it is never flagged and never comes back.
///
/// `turn.cleanup` clears it at the end of that turn, which is what makes the flag mean "this turn"
/// rather than "for ever" — a flagged card that returns to hand and is later discarded into the
/// graveyard must stay there (R153).
pub fn flag_return_to_hand_at_end_of_turn(state: &mut GameState, instance_id: &str) {
    let Some(card) = find_instance(state, instance_id) else {
        return;
    };
    if card.zone.z() != ZoneName::Graveyard {
        return;
    }
    if crate::faces::card_type_of(state, card) != CardType::Spell {
        return;
    }
    if crate::scripts::script_of(state, card).end_of_turn.is_none() {
        return;
    }
    if let Some(card) = find_instance_mut(state, instance_id) {
        card.return_to_hand_at_end_of_turn = Some(true);
    }
}
