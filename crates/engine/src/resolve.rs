//! Running a card's script (SURFACE §4): build a context, apply the effects it returns, then finish
//! the card. `cast_card` enters §10.5's play pipeline (`play_steps.rs`) directly (SURFACE §6.6).

use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::Value;

use crate::script::{Effect, EffectContext, EffectExpand, EffectPart, Hook, Memo, Script};
use crate::state::{CardInstance, GameState, find_instance, find_instance_mut};
use crate::wire::{CardType, PlayerId, Selection, ZoneName};

/// Declared in `script.rs` beside the context that derefs to it (SURFACE §6.5).
pub use crate::script::EngineSink;

/// What a run's context is built with, beyond the card itself.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HookOptions {
    pub controller: Option<PlayerId>,
    pub targets: Option<Vec<Selection>>,
    pub modes: Option<Vec<String>>,
    pub data: Option<IndexMap<String, Value>>,
}

/// Where a played card's Cry keeps its player's mana as the play began: the card's own data, which
/// every continuation of the Cry carries, so it survives a pause and a JSON round trip.
pub const MANA_BEFORE_PLAY_KEY: &str = "__manaBeforePlay";

/// The context borrows the sink for as long as it lives (it is the sink, plus this run's facts);
/// `self_` is copied in as the card stands now.
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
    // R136: this script's own event window opens where the sink's list stands now. Every context,
    // a resumed continuation's too, is built here, so this is the one place the mark is taken.
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
    // R1850000: the face of the text the card runs, a copier's copied one.
    let radiant = self_.is_some_and(|card| crate::subsystems::unit_text::text_def_of(card).1);
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

/// Apply an effect list in order. R216: once a state check inside the list has ended the game (a cast
/// on draw that killed its own hero), the rest of the list does not resolve (§2.5).
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

/// The effect-list hooks `run_hook` may run by name.
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
    /// The `Script` key it names.
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

/// `Script.activate` is no hook here (SURFACE §7.2), so "activate" names nothing.
fn hook_of(script: &Script, name: HookName) -> Option<Hook> {
    script.hook_named(name.as_str()).cloned()
}

/// Run one of a card's hooks. `cry` is also a spell's on-resolve hook (§10.9).
///
/// The *non-resumable* runner: the whole list is applied now, so it is right only where no effect can
/// open a prompt. A path whose effects may ask uses `prompts.runHookResumable` (a trigger, a Cry, an
/// activate), and a start-of-game clause, which may Choose (§10.9), `prompts.runStartOfGame` (R151, R113).
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum CastAfterward {
    Exile,
}

/// How a cast is made, beyond R70's defaults (B5 E12; R452, R453):
///
/// - `random`: every choice its caster would make is made at random (R452).
/// - `targetEnemies`: each target pick is aimed by its declaration (R656: harm at enemies, help at
///   friends, when one is legal); a card carrying the `targetEnemies` enchantment (E39) is cast so.
/// - `afterward: "exile"`: a Spell lands in exile rather than its graveyard (Classic #56 Spell
///   Tyrant's "then exile them", R453), at §10.5 step 7's landing (`echo.exileOnLanding`).
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CastOptions {
    pub controller: Option<PlayerId>,
    pub targets: Option<Vec<Selection>>,
    pub modes: Option<Vec<String>>,
    pub data: Option<IndexMap<String, Value>>,
    pub random: Option<bool>,
    pub target_enemies: Option<bool>,
    /// R1850004: its target picks are its caster's own side alone.
    pub friends_only: Option<bool>,
    pub afterward: Option<CastAfterward>,
    /// MD-B22, R946: an aimed cast takes a named legal pick for its first target declaration.
    pub aim_at: Option<String>,
}

/// R70: "a cast is free and counts as a play for every rule that counts or reacts to plays, with
/// cost paid 0", fires the card's script, and "a cast Spell does use Twinspell's Echo". So a cast IS
/// §10.5's pipeline past steps 1 and 2 (the effect chose the card; nothing is paid): step 3's Gifted
/// Program hook (#64), step 4's placement and `cardPlayed` with its Echo (R178), step 5's Quickstriker
/// (#38) and Combo draw (#78), then steps 6 and 7, owed to `state.work` only if something paused
/// (R117), so a cast-on-draw Spell's Echo repeat resolves before the draw repeats (§2.4).
///
/// A cast happens inside some other effect (§2.4's draw, #95's recursion), so it settles nothing
/// itself, but its step 4 is a window (R70, R17): the traps answer there (`triggers.dispatchPending`),
/// and §2.4's chain runs the state check after each cast-on-draw cast (§4.5, R59).
pub fn cast_card(sink: &mut EngineSink<'_>, instance: &CardInstance, options: CastOptions) {
    crate::play_steps::cast_through_pipeline(sink, instance, options);
}

/// §5.1 and R155: "Spells with 'End of turn: add this back to your hand' are flagged
/// `returnToHandAtEndOfTurn` when played and return from the graveyard at the end of that turn".
/// The flag is written at §10.5 step 7, as the Spell lands, on that step's three conditions in order:
/// a Spell (a Unit with an `endOfTurn` hook, #13 Jlockeed Shredder-10, that died on its turn must not
/// return, R153); an `endOfTurn` hook on its resolving face (#23, #24 and #31 in Core); and in the
/// **graveyard** (#39 Recycling Initiative exiles itself on play, so it never comes back).
///
/// `turn.cleanup` clears it at the end of that turn, so a flagged card that returns to hand and is
/// later discarded into the graveyard stays there (R153).
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

/// R429, R766 (issues #557, #572): the climb a Spell whose return keeps its price
/// (`StaticFlags.returnKeepsPrice`, #31 KY's Math Equation) has had from its own returns and nothing
/// else, beside the same amount in its `costMod`. R766 takes it off in the graveyard, so step 7 writes
/// it again on the card it lands (`note_return_price`) for its end-of-turn return to read. Engine
/// bookkeeping in `memory`, as R178's `@exileOnLanding` is: it lives as long as R155's flag beside it
/// (`zones::remove_from_any_zone`, `turn::clear_return_flags`), and a card that reaches a graveyard or
/// exile any other way loses it with the rest of its memory (R78).
pub const RETURN_PRICE_KEY: &str = "@returnPrice";

/// R429, R766: note `price`, the climb the card carried into its play, on a card step 7 has just
/// flagged for its end-of-turn return (R155) and whose script asks; any other card gets no note, so
/// R766's reset is all there is for it. Nothing is written for a price of 0, so a game whose #31 never
/// climbs hashes as it would without the note.
pub fn note_return_price(state: &mut GameState, instance_id: &str, price: i32) {
    if price == 0 {
        return;
    }
    let Some(card) = find_instance(state, instance_id) else {
        return;
    };
    if card.return_to_hand_at_end_of_turn != Some(true) {
        return;
    }
    if crate::scripts::script_of(state, card).flags().return_keeps_price != Some(true) {
        return;
    }
    if let Some(card) = find_instance_mut(state, instance_id) {
        card.memory
            .insert(RETURN_PRICE_KEY.to_string(), Value::from(price));
    }
}

/// R429, R766 (issue #572): add `amount` to the climb noted on `card`, as its own return prices it in
/// the hand it returns to (`setCostMod`'s `returnPrice`).
pub fn add_return_price(card: &mut CardInstance, amount: i32) {
    let price = crate::query::return_price_of(card) + amount;
    if price == 0 {
        forget_return_price(card);
    } else {
        card.memory
            .insert(RETURN_PRICE_KEY.to_string(), Value::from(price));
    }
}

pub fn forget_return_price(card: &mut CardInstance) {
    card.memory.shift_remove(RETURN_PRICE_KEY);
}
