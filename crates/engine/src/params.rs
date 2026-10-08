//! Declared numbers (docs/classic-sets.md B3.4 rule 5, R386): the numbers on a card that Degrade,
//! Upgrade and KY's Constant may move, beyond cost, stats and numbered keywords.
//!
//! A card declares them in the catalog (`CardDef.params`, one `Param` per number, with its printed
//! value on each face) and writes them into its texts as `{key}`. A card script never writes the
//! literal: it reads `param(ctx, key)`, which is the number as it stands on the instance now — its
//! face's printed value moved by the instance's tuning (`tuning.numbers`, a step count, and
//! `tuning.set`, KY's Constant's "to 3") and held inside the number's bounds. `view_for` carries the
//! same values on the card's view (`CardView.params`, `params_view`) for the client to fill the `{key}`s
//! with, and a `preview` hook (R280) reads `param_value`, so the text, the view and the resolution read
//! one function and cannot disagree.
//!
//! Which numbers a Degrade, an Upgrade or KY's Constant may reach is `param_in_reach`: a number only
//! one face prints is tuned on that face alone and reads its printed value on the other (`tunedOn`,
//! R749 for the Radiant face, R1426 for the base face), and a number that belongs to a power
//! (`Param.power`, #98 Heroic Power's) only while the card has that power (R1425), keeping its
//! tuning while it has another.
//!
//! A fused card (R77, R102) runs each ingredient's text, and each text reads its own declaration: the
//! running part's ingredient definition (`work::PART_KEY`'s path) names which. The fused card's own
//! declared numbers are its ingredients' — the first declaration of each key — and its tuning is one
//! record, so a step on "damage" moves every ingredient's "damage".
//!
//! Port of `packages/engine/src/params.ts`. TS's `param(ctx, key)` took any object with `state`,
//! `self`, `radiant` and optionally `data` and `defId` — a script context, a hook's
//! `{ state, self, radiant }`, a condition context — so the Rust `param` takes any `ParamContext`.
//! TS threw on a number a card never declared; Rust panics with the same message (SURFACE §4.4.9).
//! A fused id's ingredients are read here as `catalog.ts`'s `fusedIdParts` read them, with a digest
//! id's list taken off its definition in `state.transient_defs` (SURFACE §6.6: no digest table).

use indexmap::IndexMap;
use serde_json::Value;

use crate::catalog::def_of;
use crate::config::{PARAM_DEFAULT_STEP, TUNE_MIN_AMOUNT};
use crate::effects::tune::TuneDirection;
use crate::script::{
    ConditionContext, CostArgs, EffectContext, HookArgs, ReplacementContext, TargetCheckArgs,
    WouldCounterArgs,
};
use crate::state::{CardInstance, GameState};
use crate::tuning::{add_step, tidy_tuning, tuning_of};
use crate::wire::{Param, ParamBetter, ParamTunedOn, Tuning};

/// B3.4 rule 5: the numbers a definition declares. A catalog card's are its `params`; a fused
/// definition's (R77, R102) are its ingredients', the first declaration of each key kept. Empty for a
/// card that declares none.
pub fn params_of(state: &GameState, def_id: &str) -> Vec<Param> {
    let def = def_of(Some(state), def_id);
    if let Some(params) = def.params.as_ref() {
        return params.clone();
    }
    let Some(parts) = crate::catalog::fused_id_parts(Some(state), def_id) else {
        return Vec::new();
    };
    let mut out: Vec<Param> = Vec::new();
    for part in parts {
        for param in params_of(state, &part) {
            if !out.iter().any(|held| held.key == param.key) {
                out.push(param);
            }
        }
    }
    out
}

/// The declaration of one key, or `None` when the definition declares no such number.
pub fn param_decl_of(state: &GameState, def_id: &str, key: &str) -> Option<Param> {
    params_of(state, def_id)
        .into_iter()
        .find(|param| param.key == key)
}

/// B3.4 rule 5: how far one Degrade or Upgrade moves a declared number — the `step` it declares, else
/// 1 for a number up to 5, 2 for 6 to 12, and a quarter of it above that (rounded), all read off the
/// face's printed value.
pub fn param_step(param: &Param, printed: i32) -> i32 {
    if let Some(step) = param.step {
        return step.max(1);
    }
    let size = printed.abs();
    if size <= PARAM_DEFAULT_STEP.small.up_to {
        return PARAM_DEFAULT_STEP.small.step;
    }
    if size <= PARAM_DEFAULT_STEP.medium.up_to {
        return PARAM_DEFAULT_STEP.medium.step;
    }
    // `Math.round` rounds .5 up (SURFACE §4.4.4).
    let quarter = (f64::from(size) / f64::from(PARAM_DEFAULT_STEP.large_divisor) + 0.5).floor() as i32;
    quarter.max(1)
}

/// B3.4 rule 5, R386: a declared number's floor — its `min`, else "an amount never drops below 1",
/// or its printed value where that is lower (a number printed at 0 is never taken below it).
fn param_min(param: &Param, printed: i32) -> i32 {
    param.min.unwrap_or_else(|| TUNE_MIN_AMOUNT.min(printed))
}

/// A declared number's ceiling; `None` is TS's `Number.POSITIVE_INFINITY`.
fn param_max(param: &Param) -> Option<i32> {
    param.max
}

/// R749, R1426: whether a number may be tuned on the face `radiant` names — either face, unless its
/// `tunedOn` names the one face that prints it.
fn tuned_on_face(param: &Param, radiant: bool) -> bool {
    match param.tuned_on {
        None => true,
        Some(ParamTunedOn::Radiant) => radiant,
        Some(ParamTunedOn::Base) => !radiant,
    }
}

/// R1425: whether the card has the power `power` names now — one of the Activate abilities it has
/// (`abilities_of`, which leaves out an ability `ActivationDecl.has` says it lacks), its id that name,
/// or `<name>#n` on a fused card (R102).
fn has_power(state: &GameState, card: &CardInstance, power: &str) -> bool {
    crate::subsystems::activate::abilities_of(state, card)
        .iter()
        .any(|decl| decl.id.split('#').next() == Some(power))
}

/// B3.4 rule 3's Number row and KY's Constant: whether a Degrade, an Upgrade or KY's Constant may reach
/// the declared number `param` on `card` now. A number tuned on one face only (`tunedOn`) is out of
/// reach on the other, where it reads its printed value whatever its tuning (R749, R1426). A number
/// that belongs to a power (`Param.power`) is in reach only while the card has that power (R1425):
/// another power's number keeps the tuning it has, which counts again once the card has that power
/// back, but nothing draws it meanwhile. Every other declared number is in reach.
pub fn param_in_reach(state: &GameState, card: &CardInstance, param: &Param) -> bool {
    if !tuned_on_face(param, card.radiant) {
        return false;
    }
    match param.power.as_deref() {
        None => true,
        Some(power) => has_power(state, card, power),
    }
}

/// A declared number's value on a face, as `tuning` moves it: the one formula every reader shares.
/// R749, R1426: a number tuned on one face only reads its printed value on the other face, whatever
/// its tuning, so `steppable_params` finds no step there. A number of a power the card does not have
/// now still reads its tuning (R1425): `param_in_reach` is what keeps the draws off it.
fn value_with(param: &Param, radiant: bool, tuning: Option<&Tuning>, steps: Option<i32>) -> i32 {
    let printed = if radiant { param.radiant } else { param.base };
    if !tuned_on_face(param, radiant) {
        return printed;
    }
    let set = tuning
        .and_then(|t| t.set.as_ref())
        .and_then(|set| set.get(&param.key))
        .copied();
    let count = steps
        .or_else(|| {
            tuning
                .and_then(|t| t.numbers.as_ref())
                .and_then(|numbers| numbers.get(&param.key))
                .copied()
        })
        .unwrap_or(0);
    if set.is_none() && count == 0 {
        return printed;
    }
    let raw = set.unwrap_or(printed) + count * param_step(param, printed);
    let floored = param_min(param, printed).max(raw);
    match param_max(param) {
        Some(max) => max.min(floored),
        None => floored,
    }
}

/// `param_value`'s options: the definition whose declaration to read when it is not the instance's
/// own (an ingredient of a fused card, `param`), and the face, the instance's own by default.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParamValueOptions {
    pub def_id: Option<String>,
    pub radiant: Option<bool>,
}

/// B3.4 rule 5, R386: the value of the declared number `key` on `instance` now — pure, for `view_for`
/// (`CardView.params`) and for a `preview` hook (R280). `def_id` names the definition whose declaration
/// to read when it is not the instance's own (an ingredient of a fused card, `param`); `radiant` the
/// face, the instance's own by default. Panics for a key the definition does not declare: a card text
/// that reads a number it never declared is a catalog error, not a 0.
pub fn param_value(
    state: &GameState,
    instance: Option<&CardInstance>,
    key: &str,
    options: ParamValueOptions,
) -> i32 {
    let def_id = match options
        .def_id
        .or_else(|| instance.map(|card| card.def_id.clone()))
    {
        Some(def_id) => def_id,
        None => panic!("param \"{key}\": no card to read it on (B3.4 rule 5)"),
    };
    let Some(param) = param_decl_of(state, &def_id, key) else {
        panic!("{def_id} declares no number \"{key}\" (B3.4 rule 5, CardDef.params)");
    };
    let radiant = options
        .radiant
        .or_else(|| instance.map(|card| card.radiant))
        .unwrap_or(false);
    value_with(
        &param,
        radiant,
        instance.and_then(|card| card.tuning.as_ref()),
        None,
    )
}

/// R386: a number a card's static flag stands for (#73 Anti-oneshot Armor's `cap`, #64 Gifted
/// Program's `giftLimit`, #79 Twinspell's `echoGain`, #84 Going Long's `armor` and `paidArmor`),
/// which the engine reads rather than a script: `flag`, the flag's own value, moved by as much as
/// Degrade, Upgrade and KY's Constant have moved the card's declared number `key` off its printed
/// value. On a card of its own the flag is that printed value, so this is the number as the card
/// stands; on a fused card (R102), whose flag may be its ingredients' sum, the one tuning record moves
/// it as it moves every ingredient's number. A card that declares no `key` reads `flag` as it is.
pub fn declared_or(state: &GameState, card: &CardInstance, key: &str, flag: i32) -> i32 {
    let Some(param) = param_decl_of(state, &card.def_id, key) else {
        return flag;
    };
    let printed = value_with(&param, card.radiant, None, None);
    flag + param_value(state, Some(card), key, ParamValueOptions::default()) - printed
}

/// What `param` reads off its context (TS's structural `{ state, self, radiant, data?, defId? }`):
/// implemented for a script's `EffectContext` and for every read-only hook argument that carries a
/// state, a card and a face.
pub trait ParamContext {
    fn param_state(&self) -> &GameState;
    /// TS `ctx.self`: the running card as it stands now, or `None` once it has ceased to exist.
    fn param_self(&self) -> Option<&CardInstance>;
    fn param_radiant(&self) -> bool;
    /// TS `ctx.data`, absent on a hook's arguments.
    fn param_data(&self) -> Option<&IndexMap<String, Value>> {
        None
    }
    /// TS `ctx.defId`, absent on a hook's arguments.
    fn param_def_id(&self) -> Option<&str> {
        None
    }
}

impl ParamContext for EffectContext<'_> {
    fn param_state(&self) -> &GameState {
        &*self.sink.state
    }
    fn param_self(&self) -> Option<&CardInstance> {
        self.live_self()
    }
    fn param_radiant(&self) -> bool {
        self.radiant
    }
    fn param_data(&self) -> Option<&IndexMap<String, Value>> {
        Some(&self.data)
    }
    fn param_def_id(&self) -> Option<&str> {
        self.def_id.as_deref()
    }
}

impl ParamContext for HookArgs<'_> {
    fn param_state(&self) -> &GameState {
        self.state
    }
    fn param_self(&self) -> Option<&CardInstance> {
        Some(self.self_)
    }
    fn param_radiant(&self) -> bool {
        self.radiant
    }
}

impl ParamContext for ConditionContext<'_> {
    fn param_state(&self) -> &GameState {
        self.state
    }
    fn param_self(&self) -> Option<&CardInstance> {
        Some(self.self_)
    }
    fn param_radiant(&self) -> bool {
        self.radiant
    }
}

impl ParamContext for TargetCheckArgs<'_> {
    fn param_state(&self) -> &GameState {
        self.state
    }
    fn param_self(&self) -> Option<&CardInstance> {
        Some(self.self_)
    }
    fn param_radiant(&self) -> bool {
        self.radiant
    }
}

impl ParamContext for ReplacementContext<'_> {
    fn param_state(&self) -> &GameState {
        self.state
    }
    fn param_self(&self) -> Option<&CardInstance> {
        Some(self.self_)
    }
    fn param_radiant(&self) -> bool {
        self.radiant
    }
}

impl ParamContext for WouldCounterArgs<'_> {
    fn param_state(&self) -> &GameState {
        self.state
    }
    fn param_self(&self) -> Option<&CardInstance> {
        Some(self.self_)
    }
    fn param_radiant(&self) -> bool {
        self.self_.radiant
    }
}

impl ParamContext for CostArgs<'_> {
    fn param_state(&self) -> &GameState {
        self.state
    }
    fn param_self(&self) -> Option<&CardInstance> {
        Some(self.instance)
    }
    fn param_radiant(&self) -> bool {
        self.instance.radiant
    }
}

/// B3.4 rule 5: what a card script reads in place of a literal — `param(ctx, "damage")` on a card
/// whose text says "Deal {damage} damage". The number is the running card's (`ctx.self`), on the face
/// that is running (`ctx.radiant`), read off the declaration of the text that is running: on a fused
/// card, the running ingredient's (`work::PART_KEY`). A script whose card has ceased to exist (R127,
/// `ctx.self` null) reads the printed value of its definition (`ctx.def_id`).
///
/// B5 E14, R546: the text that is running is named by `ctx.def_id` where the run set it — a copier
/// (Classic #57 Echo) running a copied Spell's text reads that Spell's declared numbers, on its own
/// instance — and by the instance otherwise.
pub fn param<C: ParamContext + ?Sized>(ctx: &C, key: &str) -> i32 {
    let state = ctx.param_state();
    let self_ = ctx.param_self();
    let mut def_id: String = match ctx
        .param_def_id()
        .map(str::to_string)
        .or_else(|| self_.map(|card| card.def_id.clone()))
    {
        Some(def_id) => def_id,
        None => panic!("param \"{key}\": no card to read it on (B3.4 rule 5)"),
    };
    let path = ctx.param_data().and_then(crate::work::part_path_of);
    for index in path.iter().flatten() {
        let part =
            crate::catalog::fused_id_parts(Some(state), &def_id).and_then(|parts| parts.get(*index).cloned());
        match part {
            Some(part) => def_id = part,
            None => {
                let joined: Vec<String> = path.iter().flatten().map(usize::to_string).collect();
                panic!(
                    "param \"{key}\": {def_id} has no ingredient {index} on the part path {} (R102, R468)",
                    joined.join(".")
                );
            }
        }
    }
    param_value(
        state,
        self_,
        key,
        ParamValueOptions {
            def_id: Some(def_id),
            radiant: Some(ctx.param_radiant()),
        },
    )
}

/// B3.4 rule 5, R386: every declared number on the card as it stands, by key, for `CardView.params`.
/// `None` for a card that declares none, so its view carries no key.
pub fn params_view(state: &GameState, instance: &CardInstance) -> Option<IndexMap<String, i32>> {
    let params = params_of(state, &instance.def_id);
    if params.is_empty() {
        return None;
    }
    let mut out: IndexMap<String, i32> = IndexMap::new();
    for param in &params {
        out.insert(
            param.key.clone(),
            value_with(param, instance.radiant, instance.tuning.as_ref(), None),
        );
    }
    Some(out)
}

/// One declared number a step would change (`steppable_params`): the declaration, the steps recorded
/// for it, and how far its value would move.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SteppableParam {
    pub param: Param,
    pub steps: i32,
    pub delta: i32,
}

/// B3.4 rule 3's "Number" row: the declared numbers in reach (`param_in_reach`: on their face, R749,
/// R1426, and of the card's power now, R1425) that one step in `direction` would change — up moves a
/// number the way its `better` says for an Upgrade, the other way for a Degrade — with how far each
/// would move, in declaration order. A number already at the bound it would move past is not one.
pub fn steppable_params(
    state: &GameState,
    instance: &CardInstance,
    change: TuneDirection,
) -> Vec<SteppableParam> {
    params_of(state, &instance.def_id)
        .into_iter()
        .filter(|param| param_in_reach(state, instance, param))
        .filter_map(|param| {
            let toward_better = if change == TuneDirection::Upgrade { 1 } else { -1 };
            let steps = if param.better == ParamBetter::Up {
                toward_better
            } else {
                -toward_better
            };
            let tuning = instance.tuning.as_ref();
            let now = value_with(&param, instance.radiant, tuning, None);
            let recorded = tuning
                .and_then(|t| t.numbers.as_ref())
                .and_then(|numbers| numbers.get(&param.key))
                .copied()
                .unwrap_or(0);
            let next = value_with(&param, instance.radiant, tuning, Some(recorded + steps));
            if next == now {
                None
            } else {
                Some(SteppableParam {
                    param,
                    steps,
                    delta: next - now,
                })
            }
        })
        .collect()
}

/// B3.4 rule 4: record `steps` more steps on a declared number (`tuning.numbers`).
pub fn step_param(instance: &mut CardInstance, key: &str, steps: i32) {
    let tuning = tuning_of(instance);
    tuning.numbers = Some(add_step(tuning.numbers.as_ref(), key, steps));
    tidy_tuning(instance);
}

/// Classic+ #41 KY's Constant: set a declared number outright (`tuning.set`). The steps recorded
/// before it are spent — the number is the set value now — and a later Degrade or Upgrade steps from it.
pub fn set_param(instance: &mut CardInstance, key: &str, value: i32) {
    let tuning = tuning_of(instance);
    tuning
        .set
        .get_or_insert_with(IndexMap::new)
        .insert(key.to_string(), value);
    let mut numbers = tuning.numbers.clone().unwrap_or_default();
    numbers.shift_remove(key);
    tuning.numbers = Some(numbers);
    tidy_tuning(instance);
}
