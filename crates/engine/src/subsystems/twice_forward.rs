//! C+ #74 Twice Forward One Step Backwards (SPEC §8.7 row 74, R425): an Animated Field Trap that counts
//! the opponent's plays from the moment it is set and, on every `plays`-th one, once that card has
//! resolved, fuses a copy of it into itself (R831011, in place of R589's card itself) and gains Brittle.
//!
//! The count is the card's own (`memory.plays`, §10.1), surviving the Fuse that keeps this instance
//! (R77; not a `remember` note, so `work::reroot_remembered` never re-roots it). The trap's predicate
//! keeps it, with `&mut`: §10.3 offers a trap each event once (`traps::fire_trap`, R99), and only the
//! predicate runs without firing the card face-up (R33). It counts plays (`cardPlayed`, §10.5 step 4),
//! not resolutions, as a cast resolves before the play that cast it (R70).
//! maxi-tail: a trap predicate that writes its card's own counter; a watch-without-firing trigger kind in
//! traps.rs is the upgrade path if a second card ever needs one.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::brittle_count::start_brittle_on_field;
use crate::effects::animate::{AnimateArgs, animate};
use crate::effects::brittle::gain_brittle;
use crate::effects::fuse::fuse_cards;
use crate::params::param;
use crate::prelude::json_as;
use crate::script::{Effect, EffectContext, TriggerDef};
use crate::state::{CardInstance, GameState, find_instance, find_instance_mut};
use crate::wire::{GameEvent, GameEventType, ZoneName};

/// §10.1: where the card keeps the opponent's plays since it was set (R425).
pub const TWICE_FORWARD_PLAYS_KEY: &str = "plays";
/// §10.1: the plays an even count named, still to resolve (a play's cast resolves before it, R70).
const FUSE_ON_KEY: &str = "fuseOn";

/// The declared numbers the text reads (`params`, R386): every N plays, and the Brittle each fuse gains.
const EVERY: &str = "plays";
const GAIN: &str = "brittleGain";

/// The fields of a `cardResolved` this module reads.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Resolved {
    instance_id: String,
    def_id: String,
}

/// R425: the opponent's plays counted on this card so far — 0 before its first.
pub fn twice_forward_plays(card: &CardInstance) -> i32 {
    match card.memory.get(TWICE_FORWARD_PLAYS_KEY).and_then(Value::as_f64) {
        Some(value) if value.is_finite() && value > 0.0 => value.trunc() as i32,
        _ => 0,
    }
}

/// The plays an even count named that have not resolved yet, read back defensively (JSON).
fn owed(card: &CardInstance) -> Vec<String> {
    match card.memory.get(FUSE_ON_KEY) {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|id| id.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

/// §10.5 step 7: an opponent's play or cast (R70) has resolved; a countered card never is (R448).
fn opponents_play(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<Resolved> {
    match event {
        GameEvent::CardResolved {
            player,
            instance_id,
            def_id,
            ..
        } if *player != ctx.controller => Some(Resolved {
            instance_id: instance_id.clone(),
            def_id: def_id.clone(),
        }),
        _ => None,
    }
}

/// R781002: the played card if it is still in play or in a graveyard — the one place an Immutable
/// card bars the trap from copying it (R781001).
fn still_there<'s>(state: &'s GameState, play: &Resolved) -> Option<&'s CardInstance> {
    let card = find_instance(state, &play.instance_id)?;
    matches!(card.zone.z(), ZoneName::Field | ZoneName::Graveyard).then_some(card)
}

/// R687: turn the card face-up and start the printed Brittle its face-down arrival never started, so no
/// Brittle ever sits on an unrevealed card.
fn reveal_self(state: &mut GameState, self_id: &str) {
    let Some(live) = find_instance_mut(state, self_id) else {
        return;
    };
    live.face_up = Some(true);
    let mut card = live.clone();
    // Started on a copy of the face-up card, the count is written back by id.
    start_brittle_on_field(state, &mut card, false);
    if let Some(live) = find_instance_mut(state, self_id) {
        live.brittle = card.brittle;
    }
}

fn remember(state: &mut GameState, self_id: &str, key: &str, value: Value) {
    if let Some(card) = find_instance_mut(state, self_id) {
        card.memory.insert(key.to_string(), value);
    }
}

/// The trigger's `when`: count the opponent's play, or admit the resolution of a play an even count
/// named (R425, R99), which always fires as a copy is fused (R831011).
fn watch(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    let Some(self_) = ctx.live_self().cloned() else {
        return false;
    };
    if let GameEvent::CardPlayed {
        player, instance_id, ..
    } = event
        && *player != ctx.controller
    {
        let plays = twice_forward_plays(&self_) + 1;
        remember(ctx.sink.state, &self_.id, TWICE_FORWARD_PLAYS_KEY, json!(plays));
        // A zero step fuses nothing (and `%` by zero would panic).
        let every = param(ctx, EVERY);
        if every != 0 && plays % every == 0 {
            let mut fuse_on = owed(&self_);
            fuse_on.push(instance_id.clone());
            remember(ctx.sink.state, &self_.id, FUSE_ON_KEY, json!(fuse_on));
        }
        return false;
    }
    let Some(play) = opponents_play(ctx, event) else {
        return false;
    };
    let fuse_on = owed(&self_);
    if !fuse_on.contains(&play.instance_id) {
        return false;
    }
    let rest: Vec<String> = fuse_on.into_iter().filter(|id| *id != play.instance_id).collect();
    remember(ctx.sink.state, &self_.id, FUSE_ON_KEY, json!(rest));
    true
}

/// The trigger's `run`: reveal the card, then fuse a copy, gain Brittle and animate.
fn fire(radiant_copy: bool, ctx: &mut EffectContext<'_>, event: &GameEvent) -> Vec<Effect> {
    let Some(self_) = ctx.live_self().cloned() else {
        return Vec::new();
    };
    let Some(play) = opponents_play(ctx, event) else {
        return Vec::new();
    };
    // R781001: no copy of an Immutable card in play is fused in; the trap still fires and gains Brittle.
    let shielded = still_there(ctx.sink.state, &play)
        .is_some_and(|card| crate::restrictions::immutable_bars(ctx.sink.state, Some(&self_), card));
    // R687: firing turned it face-up; its printed Brittle starts now, before the gain lands on it.
    reveal_self(ctx.sink.state, &self_.id);
    let gain = gain_brittle(json_as(json!({ "instanceId": self_.id, "n": param(ctx, GAIN) })));
    // §6 Animated, R831011: the last step of its firing; a card already a Unit stays where it is.
    let animated = animate(AnimateArgs::default());
    if shielded {
        return vec![gain, animated];
    }
    // R831011: a copy of the played card's definition; the card itself stays where it is.
    let fused = fuse_cards(json_as(json!({
        "defIds": [play.def_id],
        "targetInstanceId": self_.id,
        "radiantIngredients": radiant_copy,
    })));
    vec![fused, gain, animated]
}

/// `twice_forward_trigger`'s argument, data a card may write as a JSON literal.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TwiceForwardArgs {
    pub radiant_copy: bool,
}

/// R425: the trigger both faces carry. `radiant_copy` is the Radiant face's "a Radiant copy of the
/// last", where the base face fuses a plain copy (R831011).
pub fn twice_forward_trigger(args: TwiceForwardArgs) -> TriggerDef {
    let radiant_copy = args.radiant_copy;
    TriggerDef {
        id: "twice-forward".to_string(),
        on: vec![GameEventType::CardPlayed, GameEventType::CardResolved],
        when: Some(Arc::new(move |ctx: &mut EffectContext<'_>, event: &GameEvent| {
            watch(ctx, event)
        })),
        run: Arc::new(move |ctx: &mut EffectContext<'_>, event: &GameEvent| fire(radiant_copy, ctx, event)),
        even_if_gone: false,
    }
}
