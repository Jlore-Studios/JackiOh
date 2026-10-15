//! Adapters between the engine's hook types and Luau functions in the VM.
//!
//! Each adapter returns an engine hook (an `Arc<dyn Fn ...>`) that captures its `Site` and the card's
//! bytecode. When invoked, it calls into the thread-local VM, wraps the context in scoped userdata,
//! invokes the Luau function, and translates the returned value back to the engine's types.

use std::sync::Arc;

use jackioh_engine::prelude::*;
use jackioh_engine::script::{
    AttackModArgs, AttackModHook, AuraHook, ConditionContext, ConditionHook, CostArgs, CostAuraArgs,
    CostAuraHook, CostHook, DiscardGuardHook, DrawLimitHook, EchoXHook, GraveyardPlayHook, HasHook,
    HeroGuardHook, Hook, HookArgs, PlagueMultiplierHook, PreviewHook, ReplacementContext, ReplacementWhen,
    RevealEchoHook, SetStatHook, TargetCheck, TargetCheckArgs, TargetingDiscardsHook, TributeWhenHook,
    TriggerExtraHook, TriggerRun, TriggerWhen, WouldCounterArgs, WouldCounterHook,
};
use mlua::{Function, LuaSerdeExt, Table, Value};

use crate::LuauError;
use crate::Site;
use crate::api::{self, EffectCall};
use crate::numbers;
use crate::vm;

/// A hook's list answer from its walked JSON: `{}` (and no return at all)
/// reads as no entries. An empty Luau table walks as an empty map, never an
/// empty list (L7), so the typed deserialisation alone would refuse it; the
/// type here says list, and an empty map is the empty list.
fn vec_answer<T: serde::de::DeserializeOwned>(json: serde_json::Value) -> mlua::Result<Vec<T>> {
    match json {
        serde_json::Value::Null => Ok(vec![]),
        serde_json::Value::Object(record) if record.is_empty() => Ok(vec![]),
        json => serde_json::from_value(json).map_err(|e| mlua::Error::runtime(e.to_string())),
    }
}

/// Helper to get a function from a table trying exact name, camelCase and snake_case.
pub(crate) fn get_fn(table: &Table, name: &str) -> mlua::Result<Function> {
    if let Ok(f) = table.raw_get::<Function>(name) {
        return Ok(f);
    }
    let alt = if name.contains('_') {
        snake_to_camel(name)
    } else {
        camel_to_snake(name)
    };
    table.raw_get::<Function>(&*alt)
}

/// Helper to get a subtable from a table trying exact name, camelCase and snake_case.
pub(crate) fn get_table(table: &Table, name: &str) -> mlua::Result<Option<Table>> {
    if let Ok(t) = table.raw_get::<Table>(name) {
        return Ok(Some(t));
    }
    let alt = if name.contains('_') {
        snake_to_camel(name)
    } else {
        camel_to_snake(name)
    };
    if let Ok(t) = table.raw_get::<Table>(&*alt) {
        return Ok(Some(t));
    }
    Ok(None)
}

pub(crate) fn snake_to_camel(s: &str) -> String {
    let mut out = String::new();
    let mut capitalize = false;
    for c in s.chars() {
        if c == '_' {
            capitalize = true;
        } else if capitalize {
            out.extend(c.to_uppercase());
            capitalize = false;
        } else {
            out.push(c);
        }
    }
    out
}

pub(crate) fn camel_to_snake(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_uppercase() {
            out.push('_');
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// Helper to convert a returned Luau value to EffectCalls.
fn to_calls(returned: &Value, site: Site) -> mlua::Result<Vec<EffectCall>> {
    let json = numbers::to_json(returned, site).map_err(|e| mlua::Error::runtime(e.to_string()))?;
    api::effect_calls(&json, site).map_err(|e| mlua::Error::runtime(e.to_string()))
}

/// Turns a list of effect calls into engine Effects. A custom card's that the host refuses fails the
/// hook (R915004); any other card's panics, naming it.
fn build_effects(calls: &[EffectCall], site: Site, bytecode: &'static [u8]) -> Vec<Effect> {
    let mut effects = Vec::with_capacity(calls.len());
    for call in calls {
        match api::effect_of(call, site, bytecode) {
            Ok(effect) => effects.push(effect),
            Err(err) => return failed(site, &err),
        }
    }
    effects
}

/// A hook that failed: a custom card's resolves `scriptFailed` and nothing else (R915004, SPEC §12.5);
/// any other card's panics with its error, which names its card, face and hook (an engine hook has
/// no error to return).
fn failed(site: Site, err: &dyn std::fmt::Display) -> Vec<Effect> {
    if crate::custom::is_custom_site(site) {
        return vec![crate::custom::failure(site, &err.to_string())];
    }
    panic!("{err}")
}

/// `face[name]` as a function: a missing key reads as nil, never as an error.
pub fn face_function(face: &Table, name: &str) -> mlua::Result<Function> {
    match face.raw_get::<Value>(name) {
        Ok(val) => val.as_function().cloned().ok_or_else(|| {
            if val.is_nil() {
                mlua::Error::runtime(format!("{name}: no such face function"))
            } else {
                mlua::Error::runtime(format!("{name} is of type {}, not a function", val.type_name()))
            }
        }),
        Err(_) => Err(mlua::Error::runtime(format!("{name}: no such face function"))),
    }
}

/// Whether `module[face][name]` is a function: what a closure verb's `hook` must name.
/// Checked when the verb builds, so a misspelled name fails the card's own test with an
/// error naming the card, face and function.
pub fn check_named_hook(site: Site, bytecode: &'static [u8], name: &str) -> Result<(), LuauError> {
    vm::with_vm(site, bytecode, |_lua, face| {
        face_function(&face, name).map(|_| ())
    })
}

/// Runs a closure verb's named face function as `face[name](ctx, data)` and builds the
/// effect list it returns: the card's chunk re-run through the thread-local VM, the
/// function indexed out of its face and called in a `lua.scope` with the scoped `ctx`,
/// the way part 4 builds every hook. The closure captures only the card, the face, the
/// name, the data and the site: nothing a script sets survives the call (L8).
/// A failure names the card, face and function; the caller panics with it, since an
/// engine hook has no error to return.
pub fn call_named_effects(
    site: Site,
    bytecode: &'static [u8],
    name: &str,
    ctx: &mut EffectContext<'_>,
    data: &serde_json::Value,
) -> Result<Vec<Effect>, LuauError> {
    let calls = vm::with_vm(site, bytecode, |lua, face| {
        let func = face_function(&face, name)?;
        crate::reads::scoped(lua, ctx, |ud| {
            let arg = lua.to_value(data)?;
            let returned: Value = func.call((ud, arg))?;
            to_calls(&returned, site)
        })
    })?;
    calls
        .iter()
        .map(|call| api::effect_of(call, site, bytecode))
        .collect()
}

/// Runs `with_kill_credit`'s named `pairs` function as `face[name](ctx, data)` and reads its
/// answer as kill credits: a list of `{ victimId, toId }` records. A failure names the card, face
/// and function; the caller panics with it, since an engine hook has no error to return.
pub fn call_named_kill_pairs(
    site: Site,
    bytecode: &'static [u8],
    name: &str,
    ctx: &mut EffectContext<'_>,
    data: &serde_json::Value,
) -> Result<Vec<jackioh_engine::KillCredit>, LuauError> {
    vm::with_vm(site, bytecode, |lua, face| {
        let func = face_function(&face, name)?;
        crate::reads::scoped(lua, ctx, |ud| {
            let arg = lua.to_value(data)?;
            let returned: Value = func.call((ud, arg))?;
            let json = numbers::to_json(&returned, site).map_err(|e| mlua::Error::runtime(e.to_string()))?;
            let json = match json {
                serde_json::Value::Null => serde_json::Value::Array(Vec::new()),
                serde_json::Value::Object(record) if record.is_empty() => {
                    serde_json::Value::Array(Vec::new())
                }
                json => json,
            };
            serde_json::from_value(json).map_err(|e| mlua::Error::runtime(e.to_string()))
        })
    })
}

/// Runs `with_kill_credit`'s named `then` function as `face[name](pair, data)` and builds the
/// effect list it returns. A failure names the card, face and function; the caller panics with
/// it, since an engine hook has no error to return.
pub fn call_named_kill_then(
    site: Site,
    bytecode: &'static [u8],
    name: &str,
    pair: &jackioh_engine::KillCredit,
    data: &serde_json::Value,
) -> Result<Vec<Effect>, LuauError> {
    let calls = vm::with_vm(site, bytecode, |lua, face| {
        let func = face_function(&face, name)?;
        let pair = lua.to_value(pair)?;
        let arg = lua.to_value(data)?;
        let returned: Value = func.call((pair, arg))?;
        to_calls(&returned, site)
    })?;
    calls
        .iter()
        .map(|call| api::effect_of(call, site, bytecode))
        .collect()
}

/// Runs a closure verb's named face function as `face[name](id, data)` — `for_each_card`'s
/// `each`, which takes no context — and builds the effect list it returns. `data` is what the
/// verb's `data` field carried (#464 gap 2); a function written before it ignores the second
/// argument. A failure names the card, face and function; the caller panics with it, since an
/// engine hook has no error to return.
pub fn call_named_each(
    site: Site,
    bytecode: &'static [u8],
    name: &str,
    id: &str,
    data: &serde_json::Value,
) -> Result<Vec<Effect>, LuauError> {
    let calls = vm::with_vm(site, bytecode, |lua, face| {
        let func = face_function(&face, name)?;
        let data = lua.to_value(data)?;
        let returned: Value = func.call((id.to_string(), data))?;
        to_calls(&returned, site)
    })?;
    calls
        .iter()
        .map(|call| api::effect_of(call, site, bytecode))
        .collect()
}

/// Adapts a direct effect-list hook (`cry`, `death`, `start_of_turn`, etc.). One function value
/// held by two hook keys — or by both faces (#464 gaps 10 and 14) — shares this one adapter.
/// An adapter both faces share (`across_faces`) reads its function from the RUNNING face's table,
/// by `ctx.radiant`, so a named sub-hook resolves where the run runs, while `param` reads off
/// `ctx` either way. Any other runs its own face's function, as a Rust face runs its own closure:
/// a Radiant ingredient's text fused into a base host runs Radiant with `ctx.radiant` false.
pub fn adapt_hook(site: Site, bytecode: &'static [u8], across_faces: bool) -> Hook {
    Arc::new(move |ctx: &mut EffectContext<'_>| {
        let running = if !across_faces {
            site.face
        } else if ctx.radiant {
            crate::Face::Radiant
        } else {
            crate::Face::Base
        };
        // The run's site: the function and everything it names resolve on the running face.
        // `with_vm` falls back to the base table when the module holds no table for the run
        // (`radiant = "base"`).
        let runtime = Site {
            face: running,
            ..site
        };
        match vm::with_vm(runtime, bytecode, |lua, face| {
            let func = get_fn(&face, runtime.hook)?;
            crate::reads::scoped(lua, ctx, |ud| {
                let returned: Value = func.call(ud)?;
                to_calls(&returned, runtime)
            })
        }) {
            Ok(calls) => build_effects(&calls, runtime, bytecode),
            Err(err) => failed(runtime, &err),
        }
    })
}

/// Adapts an aura hook.
pub fn adapt_aura_hook(site: Site, bytecode: &'static [u8]) -> AuraHook {
    Arc::new(
        move |args: HookArgs<'_>| match vm::call_aura_hook(site, bytecode, args) {
            Ok(entries) => entries,
            // R915004: a custom card's aura that fails gives nothing.
            Err(_) if crate::custom::is_custom_site(site) => Vec::new(),
            Err(err) => panic!("{err}"),
        },
    )
}

/// Adapts a cost hook.
pub fn adapt_cost_hook(site: Site, bytecode: &'static [u8]) -> CostHook {
    Arc::new(move |args: CostArgs<'_>| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let func = get_fn(&face, "cost")?;
            crate::reads::scoped(lua, args, |ud| {
                let val: Value = func.call(ud)?;
                numbers::to_i32(&val, site).map_err(|e| mlua::Error::runtime(e.to_string()))
            })
        }) {
            Ok(n) => n,
            Err(err) => panic!("{err}"),
        }
    })
}

/// Adapts a condition hook (`conditionMet` or `canActivate`).
pub fn adapt_condition_hook(site: Site, bytecode: &'static [u8]) -> ConditionHook {
    Arc::new(move |ctx: ConditionContext<'_>| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let func = get_fn(&face, site.hook)?;
            crate::reads::scoped(lua, ctx, |ud| {
                let val: Value = func.call(ud)?;
                Ok(val.as_boolean().unwrap_or(false))
            })
        }) {
            Ok(b) => b,
            Err(err) => panic!("{err}"),
        }
    })
}

/// Adapts a preview hook.
pub fn adapt_preview_hook(site: Site, bytecode: &'static [u8]) -> PreviewHook {
    Arc::new(move |ctx: ConditionContext<'_>| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let func = get_fn(&face, "preview")?;
            crate::reads::scoped(lua, ctx, |ud| {
                let val: Value = func.call(ud)?;
                let json = numbers::to_json(&val, site).map_err(|e| mlua::Error::runtime(e.to_string()))?;
                vec_answer(json)
            })
        }) {
            Ok(previews) => previews,
            Err(err) => panic!("{err}"),
        }
    })
}

/// Adapts a setStat hook.
pub fn adapt_set_stat_hook(site: Site, bytecode: &'static [u8]) -> SetStatHook {
    Arc::new(move |args: HookArgs<'_>| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let func = get_fn(&face, "setStat")?;
            crate::reads::scoped(lua, args, |ud| {
                let val: Value = func.call(ud)?;
                let json = numbers::to_json(&val, site).map_err(|e| mlua::Error::runtime(e.to_string()))?;
                serde_json::from_value(json).map_err(|e| mlua::Error::runtime(e.to_string()))
            })
        }) {
            Ok(stat) => stat,
            Err(err) => panic!("{err}"),
        }
    })
}

/// Adapts a costAura hook.
pub fn adapt_cost_aura_hook(site: Site, bytecode: &'static [u8]) -> CostAuraHook {
    Arc::new(move |args: CostAuraArgs<'_>| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let func = get_fn(&face, "costAura")?;
            crate::reads::scoped(lua, args, |ud| {
                let val: Value = func.call(ud)?;
                let json = numbers::to_json(&val, site).map_err(|e| mlua::Error::runtime(e.to_string()))?;
                vec_answer(json)
            })
        }) {
            Ok(rules) => rules,
            Err(err) => panic!("{err}"),
        }
    })
}

/// Adapts a graveyardPlay hook.
pub fn adapt_graveyard_play_hook(site: Site, bytecode: &'static [u8]) -> GraveyardPlayHook {
    Arc::new(move |args: CostAuraArgs<'_>| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let func = get_fn(&face, "graveyardPlay")?;
            crate::reads::scoped(lua, args, |ud| {
                let val: Value = func.call(ud)?;
                let json = numbers::to_json(&val, site).map_err(|e| mlua::Error::runtime(e.to_string()))?;
                vec_answer(json)
            })
        }) {
            Ok(perms) => perms,
            Err(err) => panic!("{err}"),
        }
    })
}

/// Adapts a revealEcho hook (R1045): the Echo this card adds to each Spell its controller reveals.
pub fn adapt_reveal_echo_hook(site: Site, bytecode: &'static [u8]) -> RevealEchoHook {
    Arc::new(move |args: HookArgs<'_>| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let func = get_fn(&face, "revealEcho")?;
            crate::reads::scoped(lua, args, |ud| {
                let val: Value = func.call(ud)?;
                numbers::to_i32(&val, site).map_err(|e| mlua::Error::runtime(e.to_string()))
            })
        }) {
            Ok(n) => n,
            Err(err) => panic!("{err}"),
        }
    })
}

/// Adapts a targetingDiscards hook.
pub fn adapt_targeting_discards_hook(site: Site, bytecode: &'static [u8]) -> TargetingDiscardsHook {
    Arc::new(move |args: HookArgs<'_>| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let func = get_fn(&face, "targetingDiscards")?;
            crate::reads::scoped(lua, args, |ud| {
                let val: Value = func.call(ud)?;
                numbers::to_i32(&val, site).map_err(|e| mlua::Error::runtime(e.to_string()))
            })
        }) {
            Ok(n) => n,
            Err(err) => panic!("{err}"),
        }
    })
}

/// Adapts a drawLimit hook.
pub fn adapt_draw_limit_hook(site: Site, bytecode: &'static [u8]) -> DrawLimitHook {
    Arc::new(move |args: HookArgs<'_>| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let func = get_fn(&face, "drawLimit")?;
            crate::reads::scoped(lua, args, |ud| {
                let val: Value = func.call(ud)?;
                let json = numbers::to_json(&val, site).map_err(|e| mlua::Error::runtime(e.to_string()))?;
                vec_answer(json)
            })
        }) {
            Ok(limits) => limits,
            Err(err) => panic!("{err}"),
        }
    })
}

/// Adapts a discardGuard hook.
pub fn adapt_discard_guard_hook(site: Site, bytecode: &'static [u8]) -> DiscardGuardHook {
    Arc::new(move |args: HookArgs<'_>| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let func = get_fn(&face, "discardGuard")?;
            crate::reads::scoped(lua, args, |ud| {
                let val: Value = func.call(ud)?;
                let json = numbers::to_json(&val, site).map_err(|e| mlua::Error::runtime(e.to_string()))?;
                vec_answer(json)
            })
        }) {
            Ok(guards) => guards,
            Err(err) => panic!("{err}"),
        }
    })
}

/// Adapts a heroGuard hook.
pub fn adapt_hero_guard_hook(site: Site, bytecode: &'static [u8]) -> HeroGuardHook {
    Arc::new(move |args: HookArgs<'_>| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let func = get_fn(&face, "heroGuard")?;
            crate::reads::scoped(lua, args, |ud| {
                let val: Value = func.call(ud)?;
                let json = numbers::to_json(&val, site).map_err(|e| mlua::Error::runtime(e.to_string()))?;
                vec_answer(json)
            })
        }) {
            Ok(guards) => guards,
            Err(err) => panic!("{err}"),
        }
    })
}

/// Adapts an attackMods hook.
pub fn adapt_attack_mod_hook(site: Site, bytecode: &'static [u8]) -> AttackModHook {
    Arc::new(move |args: AttackModArgs<'_>| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let func = get_fn(&face, "attackMods")?;
            crate::reads::scoped(lua, args, |ud| {
                let val: Value = func.call(ud)?;
                let json = numbers::to_json(&val, site).map_err(|e| mlua::Error::runtime(e.to_string()))?;
                serde_json::from_value(json).map_err(|e| mlua::Error::runtime(e.to_string()))
            })
        }) {
            Ok(m) => m,
            Err(err) => panic!("{err}"),
        }
    })
}

/// Adapts a conditionalKeywords hook.
pub fn adapt_conditional_keywords_hook(site: Site, bytecode: &'static [u8]) -> ConditionalKeywordsHook {
    Arc::new(move |args: HookArgs<'_>| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let func = get_fn(&face, "conditionalKeywords")?;
            crate::reads::scoped(lua, args, |ud| {
                let val: Value = func.call(ud)?;
                let json = numbers::to_json(&val, site).map_err(|e| mlua::Error::runtime(e.to_string()))?;
                vec_answer(json)
            })
        }) {
            Ok(kws) => kws,
            Err(err) => panic!("{err}"),
        }
    })
}

/// Adapts a plagueMultiplier hook.
pub fn adapt_plague_multiplier_hook(site: Site, bytecode: &'static [u8]) -> PlagueMultiplierHook {
    Arc::new(move |args: HookArgs<'_>| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let func = get_fn(&face, "plagueMultiplier")?;
            crate::reads::scoped(lua, args, |ud| {
                let val: Value = func.call(ud)?;
                numbers::to_i32(&val, site).map_err(|e| mlua::Error::runtime(e.to_string()))
            })
        }) {
            Ok(n) => n,
            Err(err) => panic!("{err}"),
        }
    })
}

/// Adapts an echoX hook.
pub fn adapt_echo_x_hook(site: Site, bytecode: &'static [u8]) -> EchoXHook {
    Arc::new(move |args: HookArgs<'_>| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let func = get_fn(&face, "echoX")?;
            crate::reads::scoped(lua, args, |ud| {
                let val: Value = func.call(ud)?;
                numbers::to_i32(&val, site).map_err(|e| mlua::Error::runtime(e.to_string()))
            })
        }) {
            Ok(n) => n,
            Err(err) => panic!("{err}"),
        }
    })
}

/// Adapts a trigger extra multiplier hook (`turnHookExtra` or `cryDeathExtra`).
pub fn adapt_trigger_extra_hook(site: Site, bytecode: &'static [u8], name: &'static str) -> TriggerExtraHook {
    Arc::new(move |args: HookArgs<'_>| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let func = get_fn(&face, name)?;
            crate::reads::scoped(lua, args, |ud| {
                let val: Value = func.call(ud)?;
                numbers::to_i32(&val, site).map_err(|e| mlua::Error::runtime(e.to_string()))
            })
        }) {
            Ok(n) => n,
            Err(err) => panic!("{err}"),
        }
    })
}

/// Adapts a tributeWhen hook.
pub fn adapt_tribute_when_hook(site: Site, bytecode: &'static [u8]) -> TributeWhenHook {
    Arc::new(move |args: HookArgs<'_>| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let func = get_fn(&face, "tributeWhen")?;
            crate::reads::scoped(lua, args, |ud| {
                let val: Value = func.call(ud)?;
                Ok(val.as_boolean().unwrap_or(false))
            })
        }) {
            Ok(b) => b,
            Err(err) => panic!("{err}"),
        }
    })
}

/// Adapts a wouldCounter hook.
pub fn adapt_would_counter_hook(site: Site, bytecode: &'static [u8]) -> WouldCounterHook {
    Arc::new(move |args: WouldCounterArgs<'_>| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let func = get_fn(&face, "wouldCounter")?;
            crate::reads::scoped(lua, args, |ud| {
                let val: Value = func.call(ud)?;
                Ok(val.as_boolean().unwrap_or(false))
            })
        }) {
            Ok(b) => b,
            Err(err) => panic!("{err}"),
        }
    })
}

/// Adapts a targetCheck hook.
pub fn adapt_target_check(site: Site, bytecode: &'static [u8], name: &'static str) -> TargetCheck {
    Arc::new(move |args: TargetCheckArgs<'_>| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let tc_tbl = get_table(&face, "targetChecks")?
                .ok_or_else(|| mlua::Error::runtime("no targetChecks table"))?;
            let func: Function = tc_tbl.raw_get(name)?;
            crate::reads::scoped(lua, args, |ud| {
                let val: Value = func.call(ud)?;
                Ok(val.as_boolean().unwrap_or(false))
            })
        }) {
            Ok(b) => b,
            Err(err) => panic!("{err}"),
        }
    })
}

/// Adapts a trigger `when` predicate.
pub fn adapt_trigger_when(
    site: Site,
    bytecode: &'static [u8],
    key: &'static str,
    index: usize,
) -> TriggerWhen {
    Arc::new(move |ctx: &mut EffectContext<'_>, event: &GameEvent| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let trigs: Table = face.raw_get(key)?;
            let trig: Table = trigs.raw_get(index + 1)?;
            let func = get_fn(&trig, "when")?;
            crate::reads::scoped(lua, ctx, |ctx_ud| {
                let event_val = lua.to_value(event)?;
                let val: Value = func.call((ctx_ud, event_val))?;
                Ok(val.as_boolean().unwrap_or(false))
            })
        }) {
            Ok(b) => b,
            Err(err) => panic!("{err}"),
        }
    })
}

/// Adapts a trigger `run` hook.
pub fn adapt_trigger_run(site: Site, bytecode: &'static [u8], key: &'static str, index: usize) -> TriggerRun {
    Arc::new(move |ctx: &mut EffectContext<'_>, event: &GameEvent| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let trigs: Table = face.raw_get(key)?;
            let trig: Table = trigs.raw_get(index + 1)?;
            let func = get_fn(&trig, "run")?;
            crate::reads::scoped(lua, ctx, |ctx_ud| {
                let event_val = lua.to_value(event)?;
                let returned: Value = func.call((ctx_ud, event_val))?;
                to_calls(&returned, site)
            })
        }) {
            Ok(calls) => build_effects(&calls, site, bytecode),
            Err(err) => failed(site, &err),
        }
    })
}

/// Adapts a replacement `when` predicate.
pub fn adapt_replacement_when(site: Site, bytecode: &'static [u8], index: usize) -> ReplacementWhen {
    Arc::new(move |ctx: ReplacementContext<'_>| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let reps: Table = face.raw_get("replacements")?;
            let rep: Table = reps.raw_get(index + 1)?;
            let func = get_fn(&rep, "when")?;
            crate::reads::scoped(lua, ctx, |ud| {
                let val: Value = func.call(ud)?;
                Ok(val.as_boolean().unwrap_or(false))
            })
        }) {
            Ok(b) => b,
            Err(err) => panic!("{err}"),
        }
    })
}

/// Adapts an activation's `run` hook.
pub fn adapt_activation_run(site: Site, bytecode: &'static [u8], index: usize) -> Hook {
    Arc::new(move |ctx: &mut EffectContext<'_>| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let acts: Table = face.raw_get("activations")?;
            let act: Table = acts.raw_get(index + 1)?;
            let func = get_fn(&act, "run")?;
            crate::reads::scoped(lua, ctx, |ud| {
                let returned: Value = func.call(ud)?;
                to_calls(&returned, site)
            })
        }) {
            Ok(calls) => build_effects(&calls, site, bytecode),
            Err(err) => failed(site, &err),
        }
    })
}

/// Adapts an activation's `canActivate` hook.
pub fn adapt_activation_can_activate(site: Site, bytecode: &'static [u8], index: usize) -> ConditionHook {
    Arc::new(move |ctx: ConditionContext<'_>| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let acts: Table = face.raw_get("activations")?;
            let act: Table = acts.raw_get(index + 1)?;
            let func = get_fn(&act, "canActivate")?;
            crate::reads::scoped(lua, ctx, |ud| {
                let val: Value = func.call(ud)?;
                Ok(val.as_boolean().unwrap_or(false))
            })
        }) {
            Ok(b) => b,
            Err(err) => panic!("{err}"),
        }
    })
}

/// Adapts an activation's `has` hook.
pub fn adapt_activation_has(site: Site, bytecode: &'static [u8], index: usize) -> HasHook {
    Arc::new(move |args: HookArgs<'_>| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let acts: Table = face.raw_get("activations")?;
            let act: Table = acts.raw_get(index + 1)?;
            let func = get_fn(&act, "has")?;
            crate::reads::scoped(lua, args, |ud| {
                let val: Value = func.call(ud)?;
                Ok(val.as_boolean().unwrap_or(false))
            })
        }) {
            Ok(b) => b,
            Err(err) => panic!("{err}"),
        }
    })
}

/// Adapts a named entry in `resume` or `grants`.
pub fn adapt_sub_hook(
    site: Site,
    bytecode: &'static [u8],
    table_name: &'static str,
    key: &'static str,
) -> Hook {
    Arc::new(move |ctx: &mut EffectContext<'_>| {
        match vm::with_vm(site, bytecode, |lua, face| {
            let tbl: Table = face.raw_get(table_name)?;
            let func: Function = tbl.raw_get(key)?;
            crate::reads::scoped(lua, ctx, |ud| {
                let returned: Value = func.call(ud)?;
                to_calls(&returned, site)
            })
        }) {
            Ok(calls) => build_effects(&calls, site, bytecode),
            Err(err) => failed(site, &err),
        }
    })
}
