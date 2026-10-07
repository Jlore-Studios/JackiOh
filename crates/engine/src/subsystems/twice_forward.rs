//! C+ #74 Twice Forward One Step Backwards (SPEC §8.7 row 74, R425): a Field Trap that counts the
//! opponent's plays from the moment it is set and, on every `plays`-th one, once that card has resolved,
//! fuses it (or, on the Radiant face, a Radiant copy of it) into itself and gains Brittle.
//!
//! The count is the card's own (`memory.plays`, §10.1), so it survives JSON, a replay and the Fuse that
//! keeps this instance (R77 keeps the target's memory; nothing here is a `remember` note, so a Fuse
//! never re-roots it, `work::reroot_remembered`). It is kept by the trap trigger's own predicate: §10.3
//! offers a trap each event once (`traps::fire_trap`; one a predicate declined is never owed it again,
//! R99), and the predicate is the only part of a trap that runs without firing it — a fired Field Trap
//! is face-up from then on (R33), and this one must stay face-down until it first activates. So the
//! predicate counts each play as it is played (`cardPlayed`, §10.5 step 4), notes the card an even
//! count names (`memory.fuseOn`), and admits that card's `cardResolved` — firing the trap — only when
//! there is a card to fuse; with nothing left to fuse it reveals and gains its Brittle there (R687).
//! Counting plays, not resolutions, keeps "every second card your opponent plays" right when a play
//! casts a card that resolves before it (R70): the cast is the later play.
//! ponytail: a trap predicate that writes its card's own counter; a "watch without firing" trigger kind in
//! traps.rs is the upgrade path if a second card ever needs one.
//!
//! Port of `packages/engine/src/subsystems/twiceForward.ts`. The predicate writes the card (its memory,
//! its face, its Brittle), so its `when` takes `&mut EffectContext` — the one `TriggerDef.when` that
//! must (see `.fullsend/notes/part-08-3.md`, GAPS). TS's live `ctx.self` is the card as it stands in
//! the state, read and written here by its id.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::brittle_count::start_brittle_on_field;
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

/// `Extract<GameEvent, { type: "cardResolved" }>`: the fields of a resolved play this module reads.
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

/// §10.5 step 7: a play or cast of the opponent's that an even count named has resolved (R70: a cast
/// counts; a countered card is never played, R448, so it never counts).
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

/// R425, R589: the played card "if it still exists: a Unit on the field, a Spell in the graveyard, a trap
/// in the backrow" — on the field (either row) or in a graveyard now. Exiled (a pile nothing takes a card
/// back out of, §6.3), back in a hand or a deck, or ceased to exist, it is no card left to fuse.
fn still_there<'s>(state: &'s GameState, play: &Resolved) -> Option<&'s CardInstance> {
    let card = find_instance(state, &play.instance_id)?;
    matches!(card.zone.z(), ZoneName::Field | ZoneName::Graveyard).then_some(card)
}

/// R687: turn the card face-up, public to both players, and start the printed Brittle its face-down
/// arrival never started. Firing already turned a fusing card face-up; the nothing-left-to-fuse path
/// reveals it here, so no Brittle ever sits on an unrevealed card.
fn reveal_self(state: &mut GameState, self_id: &str) {
    let Some(live) = find_instance_mut(state, self_id) else {
        return;
    };
    live.face_up = Some(true);
    let mut card = live.clone();
    // The card as it stands, face-up now; the count `brittle_count` starts on it is written back by id.
    start_brittle_on_field(state, &mut card, false);
    if let Some(live) = find_instance_mut(state, self_id) {
        live.brittle = card.brittle;
    }
}

/// Writes one memory key on the card as it stands in the state (TS wrote through the live `ctx.self`).
fn remember(state: &mut GameState, self_id: &str, key: &str, value: Value) {
    if let Some(card) = find_instance_mut(state, self_id) {
        card.memory.insert(key.to_string(), value);
    }
}

/// TS `twiceForwardTrigger`'s `when`: count the opponent's play, or admit the resolution of a play an
/// even count named (R425, R687, R99).
fn watch(radiant_copy: bool, ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
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
        // JS `plays % 0` is NaN, never 0: a zero step fuses nothing (and Rust's `%` would panic).
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
    if radiant_copy || still_there(ctx.sink.state, &play).is_some() {
        return true;
    }
    // Nothing left to fuse: the card reveals and its Brittle starts now (R687 — no Brittle while
    // unrevealed), then the gain lands on the started count. The trap stays armed (R33).
    reveal_self(ctx.sink.state, &self_.id);
    let gain = gain_brittle(json_as(json!({ "instanceId": self_.id, "n": param(ctx, GAIN) })));
    (gain.apply)(ctx);
    false
}

/// TS `twiceForwardTrigger`'s `run`: the first fuse reveals the card, then the fuse and the gain.
fn fire(radiant_copy: bool, ctx: &mut EffectContext<'_>, event: &GameEvent) -> Vec<Effect> {
    let Some(self_) = ctx.live_self().cloned() else {
        return Vec::new();
    };
    let Some(play) = opponents_play(ctx, event) else {
        return Vec::new();
    };
    // The first fuse reveals the card (R687): firing turned it face-up, and its printed Brittle
    // starts now, before the gain lands on it.
    reveal_self(ctx.sink.state, &self_.id);
    let fused = if radiant_copy {
        fuse_cards(json_as(json!({
            "defIds": [play.def_id],
            "targetInstanceId": self_.id,
            "radiantIngredients": true,
        })))
    } else {
        fuse_cards(json_as(json!({
            "instanceIds": [play.instance_id],
            "targetInstanceId": self_.id,
        })))
    };
    vec![
        fused,
        gain_brittle(json_as(json!({ "instanceId": self_.id, "n": param(ctx, GAIN) }))),
    ]
}

/// `twice_forward_trigger`'s argument, TS's `{ radiantCopy: boolean }` (data, so a card may write the
/// literal: `json_as(json!({ "radiantCopy": true }))`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TwiceForwardArgs {
    pub radiant_copy: bool,
}

/// R425: the trigger both faces carry. `radiant_copy` is the Radiant face's "a Radiant copy of it is
/// fused into this", which leaves the played card where it is and so always has something to fuse.
pub fn twice_forward_trigger(args: TwiceForwardArgs) -> TriggerDef {
    let radiant_copy = args.radiant_copy;
    TriggerDef {
        id: "twice-forward".to_string(),
        on: vec![GameEventType::CardPlayed, GameEventType::CardResolved],
        when: Some(Arc::new(move |ctx: &mut EffectContext<'_>, event: &GameEvent| {
            watch(radiant_copy, ctx, event)
        })),
        run: Arc::new(move |ctx: &mut EffectContext<'_>, event: &GameEvent| fire(radiant_copy, ctx, event)),
    }
}
