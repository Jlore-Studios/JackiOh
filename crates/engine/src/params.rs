//! Declared numbers (docs/classic-sets.md B3.4 rule 5, R386): the numbers on a card that Degrade,
//! Upgrade and KY's Constant may move, beyond cost, stats and numbered keywords.
//!
//! A card declares them in the catalog (`CardDef.params`) and writes them into its texts as `{key}`;
//! a script reads `param(ctx, key)`, the face's printed value moved by the instance's tuning and held
//! inside its bounds. `view_for` (`params_view`) and a `preview` hook (R280) read the same function, so
//! the text, the view and the resolution cannot disagree. `param_in_reach` says which numbers a tune
//! may reach (`tunedOn`, R749, R1431; a power's, #98 Heroic Power's, R1430).
//!
//! A fused card (R77, R102) runs each ingredient's text against its own declaration (`work::PART_KEY`'s
//! path); its own numbers are its ingredients' (the first of each key) under one tuning record. A
//! number a card never declared panics (SURFACE §4.4.9); a digest id's ingredients are read off its
//! definition in `state.transient_defs` (SURFACE §6.6: no digest table).

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
use crate::wire::{Param, ParamBetter, ParamTunedOn, Tuning, ZoneName};

/// B3.4 rule 5: the numbers a definition declares. A catalog card's are its `params`; a fused
/// definition's (R77, R102) are its ingredients', the first declaration of each key kept.
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

pub fn param_decl_of(state: &GameState, def_id: &str, key: &str) -> Option<Param> {
    params_of(state, def_id)
        .into_iter()
        .find(|param| param.key == key)
}

/// B3.4 rule 5: how far one Degrade or Upgrade moves a declared number: the `step` it declares, else
/// `PARAM_DEFAULT_STEP` read off the face's printed value.
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
    // Rounded half up (SURFACE §4.4.4).
    let quarter = (f64::from(size) / f64::from(PARAM_DEFAULT_STEP.large_divisor) + 0.5).floor() as i32;
    quarter.max(1)
}

/// B3.4 rule 5, R386: a declared number's floor — its `min`, else "an amount never drops below 1",
/// or its printed value where that is lower (a number printed at 0 is never taken below it).
fn param_min(param: &Param, printed: i32) -> i32 {
    param.min.unwrap_or_else(|| TUNE_MIN_AMOUNT.min(printed))
}

fn param_max(param: &Param) -> Option<i32> {
    param.max
}

/// R749, R1431: whether a number may be tuned on the face `radiant` names — either face, unless its
/// `tunedOn` names the one face that prints it.
fn tuned_on_face(param: &Param, radiant: bool) -> bool {
    match param.tuned_on {
        None => true,
        Some(ParamTunedOn::Radiant) => radiant,
        Some(ParamTunedOn::Base) => !radiant,
    }
}

/// R1430: whether the card has the power `power` names now: an Activate ability it has (`abilities_of`)
/// with that id, or `<name>#n` on a fused card (R102).
fn has_power(state: &GameState, card: &CardInstance, power: &str) -> bool {
    crate::subsystems::activate::abilities_of(state, card)
        .iter()
        .any(|decl| decl.id.split('#').next() == Some(power))
}

/// B3.4 rule 3's Number row and KY's Constant: whether a tune may reach the declared number `param` on
/// `card` now: not on the face a one-face number is not tuned on (`tunedOn`, R749, R1431), and a power's
/// number (`Param.power`) only while the card has that power (R1430), keeping its tuning meanwhile.
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
/// R749, R1431: a one-face number reads its printed value on the other face; a number of a power the
/// card lacks still reads its tuning (R1430), `param_in_reach` keeping the draws off it.
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

/// B3.4 rule 5, R386: the value of the declared number `key` on `instance` now, for `view_for`
/// (`CardView.params`) and a `preview` hook (R280). Panics for a key the definition does not declare:
/// a card text that reads a number it never declared is a catalog error, not a 0.
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

/// R386: a number a card's static flag stands for (#73's `cap`, #64's `giftLimit`, #79's `echoGain`,
/// #84's `armor` and `paidArmor`), which the engine reads rather than a script: `flag` moved by as much
/// as tunes moved the declared number `key` off its printed value, so on a fused card (R102), whose flag
/// may be its ingredients' sum, the one tuning record moves it too. No `key`: `flag` as it is.
pub fn declared_or(state: &GameState, card: &CardInstance, key: &str, flag: i32) -> i32 {
    let Some(param) = param_decl_of(state, &card.def_id, key) else {
        return flag;
    };
    let printed = value_with(&param, card.radiant, None, None);
    flag + param_value(state, Some(card), key, ParamValueOptions::default()) - printed
}

/// What `param` reads off its context: a script's `EffectContext` or any read-only hook argument that
/// carries a state, a card and a face.
pub trait ParamContext {
    fn param_state(&self) -> &GameState;
    /// The running card as it stands now, or `None` once it has ceased to exist.
    fn param_self(&self) -> Option<&CardInstance>;
    fn param_radiant(&self) -> bool;
    /// The run's `data`, absent on a hook's arguments.
    fn param_data(&self) -> Option<&IndexMap<String, Value>> {
        None
    }
    /// The running text's definition, absent on a hook's arguments.
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

/// MD-D4, R1120: an `attack_mods` hook reads its card's declared numbers like any pure-read hook.
impl ParamContext for crate::script::AttackModArgs<'_> {
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

/// B3.4 rule 5: what a card script reads in place of a literal (`param(ctx, "damage")` for "Deal
/// {damage} damage"): the running card's number on the running face, off the running text's declaration
/// (a fused card's ingredient, `work::PART_KEY`; B5 E14, R546: `ctx.def_id`'s, as a Classic #57 Echo's
/// copied Spell). A card that has ceased to exist (R127) reads its definition's printed value.
pub fn param<C: ParamContext + ?Sized>(ctx: &C, key: &str) -> i32 {
    param_read(ctx, key, None)
}

/// R2657006: `param`, or `printed` when the definition the card folds under declares no `key` — a
/// number a card declares since a later patch, read by a script that a replay of an older version runs
/// with that version's definition (R767000), which printed `printed` as a plain number. A card that
/// declares it reads it exactly as `param` does.
pub fn param_or<C: ParamContext + ?Sized>(ctx: &C, key: &str, printed: i32) -> i32 {
    param_read(ctx, key, Some(printed))
}

fn param_read<C: ParamContext + ?Sized>(ctx: &C, key: &str, printed: Option<i32>) -> i32 {
    let state = ctx.param_state();
    let self_ = ctx.param_self();
    // R1850000: a copier with no running text named reads the numbers of the text it copied, on the
    // face it copied.
    let copied = if ctx.param_def_id().is_none() {
        self_.and_then(crate::subsystems::unit_text::unit_text_of)
    } else {
        None
    };
    let radiant = copied
        .as_ref()
        .map_or_else(|| ctx.param_radiant(), |copy| copy.radiant);
    let mut def_id: String = match ctx
        .param_def_id()
        .map(str::to_string)
        .or_else(|| copied.map(|copy| copy.def_id))
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
    if let Some(printed) = printed
        && param_decl_of(state, &def_id, key).is_none()
    {
        return printed;
    }
    param_value(
        state,
        self_,
        key,
        ParamValueOptions {
            def_id: Some(def_id),
            radiant: Some(radiant),
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

/// B3.4 rule 3's "Number" row: the declared numbers in reach (`param_in_reach`), but for one only the
/// card's Cry reads (`Param.cry`) on the face it shows on the field (R1748000, R2261001), that one step
/// in `direction` would change (the way `better` says for an Upgrade, the other for a Degrade), with how
/// far each would move, in declaration order. A number at the bound it would move past is not one.
pub fn steppable_params(
    state: &GameState,
    instance: &CardInstance,
    change: TuneDirection,
) -> Vec<SteppableParam> {
    steppable_params_times(state, instance, change, crate::config::TUNE_MULTIPLIER_NONE)
}

/// R1160 (Meditative #84 Volatility, ME-TUNEMULT): `steppable_params` moved `times` steps at once.
/// Each step is one unit in the row's direction; the first step that would not move the value stops
/// the walk, so what a bound refuses is lost and only what moved is recorded.
pub fn steppable_params_times(
    state: &GameState,
    instance: &CardInstance,
    change: TuneDirection,
    times: i32,
) -> Vec<SteppableParam> {
    params_of(state, &instance.def_id)
        .into_iter()
        .filter(|param| param_in_reach(state, instance, param))
        // R1748000: on the field the card's Cry has resolved, so a number only it reads moves nothing,
        // on the face it shows (R2261001).
        .filter(|param| !(param.cry_only_on(instance.radiant) && instance.zone.z() == ZoneName::Field))
        .filter_map(|param| {
            let toward_better = if change == TuneDirection::Upgrade { 1 } else { -1 };
            let unit = if param.better == ParamBetter::Up {
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
            let mut value = now;
            let mut taken = 0;
            for k in 1..=times.max(1) {
                let next = value_with(&param, instance.radiant, tuning, Some(recorded + unit * k));
                if next == value {
                    break;
                }
                value = next;
                taken = k;
            }
            if taken == 0 {
                None
            } else {
                Some(SteppableParam {
                    param,
                    steps: unit * taken,
                    delta: value - now,
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

/// Classic+ #41 KY's Constant: set a declared number outright (`tuning.set`), spending the steps
/// recorded before it; a later Degrade or Upgrade steps from it.
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
