//! What a script reaches (#442 L8): the module `require("@jackioh")` answers, `J`, and the `ctx` a
//! hook is called with.
//!
//! An effect is data. `J.damage(args)` returns `{ verb = "damage", args = args }` and touches
//! nothing, the hook returns a list of them, and the host walks the list (L6) and builds each one
//! with the engine's own verb (CLAUDE.md rule 5).

use jackioh_engine::Effect;
use mlua::{Error, Function, Lua, LuaSerdeExt, Table, Value};
use serde::de::DeserializeOwned;
use serde_json::Value as Json;

use crate::numbers::{self, arg};
use crate::sandbox::uncapped;
use crate::{LuauError, Site};

/// One effect a hook returned, walked but not yet built: the verb's name and its arguments.
#[derive(Clone, Debug, PartialEq)]
pub struct EffectCall {
    pub verb: String,
    pub args: Json,
}

macro_rules! verbs {
    ( $( verb!($name:ident $(, $arg:ident)?); )* ) => {
        #[allow(unused_imports)]
        use jackioh_engine::effects::*;

        pub const VERBS: &[&'static str] = &[ $( stringify!($name) ),* ];
        pub const SPECS: &[(&'static str, Option<&'static str>)] = &[
            $( $crate::api::verbs!(@spec $name $(, $arg)?) ),*
        ];

        pub fn effect_of(call: &$crate::api::EffectCall, site: $crate::Site, _bytecode: &'static [u8]) -> Result<Option<jackioh_engine::Effect>, $crate::LuauError> {
            match call.verb.as_str() {
                $( stringify!($name) => $crate::api::verbs!(@build call, site, _bytecode, $name $(, $arg)?).map(Some), )*
                _ => Ok(None),
            }
        }
    };
    (@spec $name:ident, $arg:ident) => { (stringify!($name), Some(stringify!($arg))) };
    (@spec $name:ident) => { (stringify!($name), None) };
    (@build $call:ident, $site:ident, $bytecode:ident, after_state_check, $arg:ty) => {
        $crate::api::after_check::build_after_state_check($call, $site, $bytecode)
    };
    (@build $call:ident, $site:ident, $bytecode:ident, cast_each, $arg:ty) => {
        $crate::api::cast::build_cast_each($call, $site, $bytecode)
    };
    (@build $call:ident, $site:ident, $bytecode:ident, draw_while, $arg:ty) => {
        $crate::api::draw_while::build_draw_while($call, $site, $bytecode)
    };
    (@build $call:ident, $site:ident, $bytecode:ident, for_each_card, $arg:ty) => {
        $crate::api::each::build_for_each_card($call, $site, $bytecode)
    };
    (@build $call:ident, $site:ident, $bytecode:ident, with_kill_credit, $arg:ty) => {
        $crate::api::kill_credit::build_with_kill_credit($call, $site, $bytecode)
    };
    (@build $call:ident, $site:ident, $bytecode:ident, radiant_chance, $arg:ty) => {
        $crate::api::radiant::build_radiant_chance($call, $site)
    };
    (@build $call:ident, $site:ident, $bytecode:ident, do_random_entries, $arg:ty) => {
        $crate::api::random_entries::build_do_random_entries($call, $site, $bytecode)
    };
    (@build $call:ident, $site:ident, $bytecode:ident, defense_armor_aura, $arg:ty) => {
        Err($crate::LuauError::Hook {
            site: $site,
            message: "defense_armor_aura is an aura verb, not an effect verb".to_string(),
        })
    };
    (@build $call:ident, $site:ident, $bytecode:ident, defense_armor, $arg:ty) => {
        Err($crate::LuauError::Hook {
            site: $site,
            message: "defense_armor is an aura verb, not an effect verb".to_string(),
        })
    };
    (@build $call:ident, $site:ident, $bytecode:ident, aura, $arg:ty) => {
        Err($crate::LuauError::Hook {
            site: $site,
            message: "aura is an aura verb, not an effect verb".to_string(),
        })
    };
    (@build $call:ident, $site:ident, $bytecode:ident, cloak_hand_aura, $arg:ty) => {
        Err($crate::LuauError::Hook {
            site: $site,
            message: "cloak_hand_aura is an aura verb, not an effect verb".to_string(),
        })
    };
    (@build $call:ident, $site:ident, $bytecode:ident, $name:ident, $arg:ty) => {{
        let _ = $bytecode;
        $crate::api::typed::<$arg>($call, $site).map(jackioh_engine::effects::$name)
    }};
    (@build $call:ident, $site:ident, $bytecode:ident, $name:ident) => {{
        let _ = &$site;
        let _ = $bytecode;
        Ok(jackioh_engine::effects::$name())
    }};
}
pub(crate) use verbs;

macro_rules! register_modules {
    ( $( $mod:ident ),* $(,)? ) => {
        $( pub mod $mod; )*

        pub fn all_verbs() -> Vec<&'static str> {
            let mut list = Vec::new();
            $( list.extend_from_slice($mod::VERBS); )*
            list
        }

        pub fn all_specs() -> Vec<(&'static str, Option<&'static str>)> {
            let mut list = Vec::new();
            $( list.extend_from_slice($mod::SPECS); )*
            list
        }

        pub fn effect_of(call: &EffectCall, site: Site, bytecode: &'static [u8]) -> Result<Effect, LuauError> {
            $(
                if let Some(effect) = $mod::effect_of(call, site, bytecode)? {
                    return Ok(effect);
                }
            )*
            Err(LuauError::Hook {
                site,
                message: format!("no verb {:?}", call.verb),
            })
        }
    };
}

register_modules! {
    add_to_hand,
    after_check,
    animate,
    aura,
    brittle,
    buff,
    cast,
    choose,
    choose_where,
    coins,
    combat,
    cost,
    counters,
    craft,
    cry,
    damage,
    datacenter,
    delay,
    destroy,
    draw,
    draw_while,
    each,
    enchant,
    flicker,
    fruit,
    fuse,
    give,
    grant_tag,
    hand_exile,
    heal,
    health,
    jade,
    kill_credit,
    last_board,
    library,
    library_copies,
    locks,
    lose_health,
    mana,
    memory,
    move_,
    perks,
    plague,
    player_mods,
    position,
    radiant,
    random_entries,
    random_picks,
    reveal,
    rotate,
    rounds,
    secret,
    shuffle_card,
    shuffle_into,
    shuffle_random,
    split,
    statuses,
    steal,
    subsystems,
    summon,
    summon_this,
    swap,
    transform,
    translate,
    tune,
    turn_end,
    turn_hooks,
    turns,
    win,
}

/// A call's arguments as the verb's own argument type.
pub fn typed<T: DeserializeOwned>(call: &EffectCall, site: Site) -> Result<T, LuauError> {
    match serde_json::from_value::<T>(call.args.clone()) {
        Ok(val) => Ok(val),
        Err(err) => {
            // Risk #2: Luau empty tables `{}` serialize as `Json::Object({})`. If the target type
            // expects a sequence (e.g. `Vec<T>`), retry with empty objects coerced to empty arrays.
            let coerced = numbers::coerce_empty_objects(&call.args);
            serde_json::from_value::<T>(coerced).map_err(|_| LuauError::Hook {
                site,
                message: format!("{}: {err}", call.verb),
            })
        }
    }
}

/// The module, read-only: one function per verb, and `J.div`, `J.rem`, `J.sort_by`, `J.query`, etc.
pub fn module(lua: &Lua) -> mlua::Result<Table> {
    let module = lua.create_table()?;
    // Every verb is data: `J.damage(args)` returns `{ verb, args }` and touches
    // nothing. Closure-taking verbs take the *name* of a face function (`hook`,
    // `cards`, `each`), resolved by name when the verb builds — a function itself
    // cannot cross into Rust (L6), and the walk below refuses one that tries. Each is a Luau
    // closure over its verb's name: a call into Rust, and a table built there under the memory cap,
    // cost a hook call more than the table built in Luau does.
    let verb_function: Function = lua
        .load("local verb = ...\nreturn function(args) return { verb = verb, args = args } end\n")
        .set_name("=@jackioh")
        .into_function()?;
    for &verb in &all_verbs() {
        module.raw_set(verb, verb_function.call::<Function>(verb)?)?;
    }
    module.raw_set("div", integer_function(lua, "div", numbers::div)?)?;
    module.raw_set("rem", integer_function(lua, "rem", numbers::rem)?)?;
    module.raw_set("sort_by", lua.create_function(sort_by)?)?;
    module.raw_set("list", lua.create_function(list)?)?;
    module.raw_set("query", lua.create_function(query_catalog)?)?;
    module.raw_set("pool", lua.create_function(pool_catalog)?)?;
    module.raw_set("cost", lua.create_function(cost_catalog)?)?;

    let trap_types = lua.create_table()?;
    trap_types.raw_set(1, "Trap")?;
    trap_types.raw_set(2, "Field Trap")?;
    // Read-only for the same reason as `J.config` below.
    trap_types.set_readonly(true);
    module.raw_set("TRAP_TYPES", trap_types)?;

    // `J.config`: the engine's config constants card scripts use, read from the same source as
    // `engineConfig.ts` (#464 gap 7). A script reads `J.config.HERO_HEALTH`, never `30`.
    let config = lua.create_table()?;
    for (name, value) in crate::config::CONFIG_I32 {
        config.raw_set(*name, *value)?;
    }
    // Read-only like `J` itself: the table lives as long as the thread's VM, so a write to it
    // would survive the call (L8).
    config.set_readonly(true);
    module.raw_set("config", config)?;

    module.set_readonly(true);
    Ok(module)
}

/// `J.div` or `J.rem`: two `i32`s in, one out, or an error where Rust's operator would panic.
fn integer_function(lua: &Lua, name: &'static str, f: fn(i32, i32) -> Option<i32>) -> mlua::Result<Function> {
    lua.create_function(move |_, (a, b): (Value, Value)| {
        let (a, b) = (arg::<i32>(a)?, arg::<i32>(b)?);
        f(a, b).ok_or_else(|| Error::runtime(format!("J.{name}({a}, {b}) has no i32 answer")))
    })
}

/// `J.list(t)`: `t`, marked as a list with mlua's array metatable, so that empty it crosses as `[]`
/// rather than `{}` where the Rust value is untyped (a prompt's or a memory's `data`), as an empty
/// sequence mlua serialized from Rust does (`numbers::walk_table`).
fn list(lua: &Lua, table: Table) -> mlua::Result<Table> {
    table.set_metatable(Some(lua.array_metatable()))?;
    Ok(table)
}

/// `J.sort_by(array, key_fn)`: a stable sort done in Rust (L7).
fn sort_by(lua: &Lua, (array, key_fn): (Table, Function)) -> mlua::Result<Table> {
    let len = array.raw_len();
    let mut items = Vec::with_capacity(len);
    for i in 1..=len {
        let val: Value = array.raw_get(i)?;
        let key: Value = key_fn.call(val.clone())?;
        items.push((i, val, key));
    }
    items.sort_by(|a, b| compare_luau_values(&a.2, &b.2));
    let out = lua.create_table_with_capacity(len, 0)?;
    for (idx, (_, val, _)) in items.into_iter().enumerate() {
        out.raw_set(idx + 1, val)?;
    }
    Ok(out)
}

fn compare_luau_values(a: &Value, b: &Value) -> std::cmp::Ordering {
    match (a, b) {
        (Value::Integer(x), Value::Integer(y)) => x.cmp(y),
        (Value::Number(x), Value::Number(y)) => x.total_cmp(y),
        (Value::Integer(x), Value::Number(y)) => (*x as f64).total_cmp(y),
        (Value::Number(x), Value::Integer(y)) => x.total_cmp(&(*y as f64)),
        (Value::String(x), Value::String(y)) => x.as_bytes().as_ref().cmp(y.as_bytes().as_ref()),
        (Value::Boolean(x), Value::Boolean(y)) => x.cmp(y),
        _ => std::cmp::Ordering::Equal,
    }
}

fn query_catalog(lua: &Lua, args: Value) -> mlua::Result<Value> {
    // Built with the memory cap lifted, as a reader's answer is (`uncapped`).
    uncapped(lua, || {
        let json: Json =
            serde_json::from_value(arg::<Json>(args)?).map_err(|e| Error::runtime(e.to_string()))?;
        let query_args: jackioh_engine::CatalogQueryArgs =
            serde_json::from_value(json).map_err(|e| Error::runtime(e.to_string()))?;
        let defs = jackioh_engine::catalog::query(&query_args);
        lua.to_value(&defs)
    })
}

fn pool_catalog(lua: &Lua, (own_id, args): (String, Value)) -> mlua::Result<Value> {
    uncapped(lua, || {
        let json: Json =
            serde_json::from_value(arg::<Json>(args)?).map_err(|e| Error::runtime(e.to_string()))?;
        let query_args: jackioh_engine::CatalogQueryArgs =
            serde_json::from_value(json).map_err(|e| Error::runtime(e.to_string()))?;
        let defs = jackioh_engine::catalog::query(&jackioh_engine::catalog::excluding_def_id(
            None,
            &query_args,
            Some(&own_id),
        ));
        lua.to_value(&defs)
    })
}

fn cost_catalog(_: &Lua, def_or_id: Value) -> mlua::Result<i32> {
    match def_or_id {
        Value::String(s) => {
            let s_str = s.to_str()?;
            if let Some(def) = jackioh_engine::catalog::find_def(None, &s_str) {
                Ok(jackioh_engine::catalog::query_cost(def))
            } else {
                Ok(0)
            }
        }
        Value::Table(t) => {
            let id: String = t.raw_get("id")?;
            if let Some(def) = jackioh_engine::catalog::find_def(None, &id) {
                Ok(jackioh_engine::catalog::query_cost(def))
            } else {
                Ok(0)
            }
        }
        _ => Err(Error::runtime("cost expects string id or CardDef table")),
    }
}

/// A hook's walked return as its effect calls: a list of `{ verb, args }`, or nothing (`nil` or an
/// empty table). An error names what was wrong by its kind and place, never by the value itself.
pub fn effect_calls(value: &Json, site: Site) -> Result<Vec<EffectCall>, LuauError> {
    let refused = |message: String| LuauError::Hook { site, message };
    match value {
        Json::Null => Ok(vec![]),
        Json::Object(record) if record.is_empty() => Ok(vec![]),
        Json::Array(items) => items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let call = item
                    .as_object()
                    .filter(|record| record.keys().all(|key| key == "verb" || key == "args"));
                match call.and_then(|record| record.get("verb")) {
                    Some(Json::String(verb)) => Ok(EffectCall {
                        verb: verb.clone(),
                        args: item.get("args").cloned().unwrap_or(Json::Null),
                    }),
                    _ => Err(refused(format!(
                        "effect {} is {}, not a {{ verb, args }} record",
                        index + 1,
                        kind(item)
                    ))),
                }
            })
            .collect(),
        other => Err(refused(format!(
            "a hook returns a list of effects, not {}",
            kind(other)
        ))),
    }
}

/// What a walked value is, for an error.
pub(crate) fn kind(value: &Json) -> &'static str {
    match value {
        Json::Null => "nil",
        Json::Bool(_) => "a boolean",
        Json::Number(_) => "a number",
        Json::String(_) => "a string",
        Json::Array(_) => "a list",
        Json::Object(_) => "a record",
    }
}

/// Turns aura effect calls into `AuraEntry`s.
pub fn aura_entries<'a>(
    calls: &[EffectCall],
    site: Site,
    bytecode: &'static [u8],
    args: jackioh_engine::script::HookArgs<'a>,
) -> Result<Vec<jackioh_engine::script::AuraEntry<'a>>, LuauError> {
    let mut entries = Vec::new();
    for call in calls {
        match call.verb.as_str() {
            "defense_armor_aura" | "defense_armor" => {
                let n = match &call.args {
                    Json::Object(obj) => obj
                        .get("armor")
                        .or_else(|| obj.get("n"))
                        .and_then(|v| v.as_i64())
                        .and_then(|v| i32::try_from(v).ok())
                        .unwrap_or(0),
                    Json::Number(num) => num.as_i64().and_then(|v| i32::try_from(v).ok()).unwrap_or(0),
                    _ => 0,
                };
                let controller = args.self_.controller;
                entries.push(jackioh_engine::script::AuraEntry {
                    applies: Box::new(move |unit: &jackioh_engine::CardInstance| {
                        unit.controller == controller
                            && matches!(
                                unit.zone,
                                jackioh_engine::Zone::Field {
                                    row: jackioh_engine::Row::Units,
                                    ..
                                }
                            )
                            && unit.position.unwrap_or(jackioh_engine::Position::Atk)
                                == jackioh_engine::Position::Def
                    }),
                    mod_: jackioh_engine::script::StatMod {
                        keywords: Some(vec![jackioh_engine::Keyword::Armor { n }]),
                        ..Default::default()
                    },
                });
            }
            "aura" => {
                let aura: crate::api::aura::AuraArgs =
                    serde_json::from_value(call.args.clone()).map_err(|err| LuauError::Hook {
                        site,
                        message: format!("aura: {err}"),
                    })?;
                entries.push(generic_aura_entry(site, bytecode, args, aura)?);
            }
            "cloak_hand_aura" => {
                let cloak: crate::api::aura::CloakHandAuraArgs = serde_json::from_value(call.args.clone())
                    .map_err(|err| LuauError::Hook {
                        site,
                        message: format!("cloak_hand_aura: {err}"),
                    })?;
                entries.extend(jackioh_engine::alt_play::cloak_hand_aura(cloak.type_)(args));
            }
            verb => {
                return Err(LuauError::Hook {
                    site,
                    message: format!("no aura verb {verb:?}"),
                });
            }
        }
    }
    Ok(entries)
}

/// One generic aura entry (#464 gap 8): the declared stat `mod` with an `applies` filter, either
/// data evaluated in Rust or a named face predicate called per candidate unit.
fn generic_aura_entry<'a>(
    site: Site,
    bytecode: &'static [u8],
    args: jackioh_engine::script::HookArgs<'a>,
    aura: crate::api::aura::AuraArgs,
) -> Result<jackioh_engine::script::AuraEntry<'a>, LuauError> {
    use jackioh_engine::effects::ScopeSide;
    let controller = args.self_.controller;
    let applies: Box<dyn Fn(&jackioh_engine::CardInstance) -> bool + 'a> = match aura.applies {
        crate::api::aura::AuraApplies::Filter(filter) => {
            let side = filter.side.unwrap_or(ScopeSide::SelfSide);
            let rows = filter.rows.unwrap_or_else(|| vec![jackioh_engine::Row::Units]);
            let def_id = filter.def_id;
            let except_def_id = filter.except_def_id;
            Box::new(move |unit: &jackioh_engine::CardInstance| {
                let side_ok = match side {
                    ScopeSide::Any => true,
                    ScopeSide::SelfSide => unit.controller == controller,
                    ScopeSide::Enemy => unit.controller != controller,
                };
                let row_ok = matches!(
                    unit.zone,
                    jackioh_engine::Zone::Field { row, .. } if rows.contains(&row)
                );
                let def_ok = def_id.as_deref().is_none_or(|def| unit.def_id == def);
                let except_ok = except_def_id.as_deref().is_none_or(|def| unit.def_id != def);
                side_ok && row_ok && def_ok && except_ok
            })
        }
        crate::api::aura::AuraApplies::Predicate(name) => {
            crate::hooks::check_named_hook(site, bytecode, &name)?;
            Box::new(move |unit: &jackioh_engine::CardInstance| {
                match crate::vm::with_vm(site, bytecode, |lua, face| {
                    let func = crate::hooks::face_function(&face, name.as_str())?;
                    let unit = lua.to_value(unit)?;
                    let answered: Value = func.call(unit)?;
                    Ok(answered.as_boolean().unwrap_or(false))
                }) {
                    Ok(applies) => applies,
                    Err(error) => panic!("{error}"),
                }
            })
        }
    };
    Ok(jackioh_engine::script::AuraEntry {
        applies,
        mod_: aura.mod_,
    })
}
