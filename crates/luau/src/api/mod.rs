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

        pub fn effect_of(call: &$crate::api::EffectCall, site: $crate::Site) -> Result<Option<jackioh_engine::Effect>, $crate::LuauError> {
            match call.verb.as_str() {
                $( stringify!($name) => $crate::api::verbs!(@build call, site, $name $(, $arg)?).map(Some), )*
                _ => Ok(None),
            }
        }
    };
    (@spec $name:ident, $arg:ident) => { (stringify!($name), Some(stringify!($arg))) };
    (@spec $name:ident) => { (stringify!($name), None) };
    (@build $call:ident, $site:ident, after_state_check, $arg:ty) => {
        $crate::api::after_check::build_after_state_check($call, $site)
    };
    (@build $call:ident, $site:ident, cast_each, $arg:ty) => {
        $crate::api::cast::build_cast_each($call, $site)
    };
    (@build $call:ident, $site:ident, draw_while, $arg:ty) => {
        $crate::api::draw_while::build_draw_while($call, $site)
    };
    (@build $call:ident, $site:ident, for_each_card, $arg:ty) => {
        $crate::api::each::build_for_each_card($call, $site)
    };
    (@build $call:ident, $site:ident, with_kill_credit, $arg:ty) => {
        $crate::api::kill_credit::build_with_kill_credit($call, $site)
    };
    (@build $call:ident, $site:ident, do_random_entries, $arg:ty) => {
        $crate::api::random_entries::build_do_random_entries($call, $site)
    };
    (@build $call:ident, $site:ident, defense_armor_aura, $arg:ty) => {
        Err($crate::LuauError::Hook {
            site: $site,
            message: "defense_armor_aura is an aura verb, not an effect verb".to_string(),
        })
    };
    (@build $call:ident, $site:ident, defense_armor, $arg:ty) => {
        Err($crate::LuauError::Hook {
            site: $site,
            message: "defense_armor is an aura verb, not an effect verb".to_string(),
        })
    };
    (@build $call:ident, $site:ident, $name:ident, $arg:ty) => {
        $crate::api::typed::<$arg>($call, $site).map(jackioh_engine::effects::$name)
    };
    (@build $call:ident, $site:ident, $name:ident) => {{
        let _ = &$site;
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

        pub fn effect_of(call: &EffectCall, site: Site) -> Result<Effect, LuauError> {
            $(
                if let Some(effect) = $mod::effect_of(call, site)? {
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
            let coerced = coerce_empty_objects(&call.args);
            serde_json::from_value::<T>(coerced).map_err(|_| LuauError::Hook {
                site,
                message: format!("{}: {err}", call.verb),
            })
        }
    }
}

fn coerce_empty_objects(value: &Json) -> Json {
    match value {
        Json::Object(map) if map.is_empty() => Json::Array(Vec::new()),
        Json::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (k, v) in map {
                new_map.insert(k.clone(), coerce_empty_objects(v));
            }
            Json::Object(new_map)
        }
        Json::Array(items) => Json::Array(items.iter().map(coerce_empty_objects).collect()),
        other => other.clone(),
    }
}

/// The module, read-only: one function per verb, and `J.div`, `J.rem`, `J.sort_by`, `J.query`, etc.
pub fn module(lua: &Lua) -> mlua::Result<Table> {
    let module = lua.create_table()?;
    for &verb in &all_verbs() {
        if verb == "after_state_check" {
            let call = lua.create_function(move |lua, callback: Function| {
                let id = after_check::register_callback(lua, callback)?;
                let call = lua.create_table()?;
                call.raw_set("verb", "after_state_check")?;
                let args = lua.create_table()?;
                args.raw_set("callbackId", id)?;
                call.raw_set("args", args)?;
                Ok(call)
            })?;
            module.raw_set(verb, call)?;
        } else if verb == "draw_while" {
            let call = lua.create_function(move |lua, args_val: Value| {
                let call = lua.create_table()?;
                call.raw_set("verb", "draw_while")?;
                match &args_val {
                    Value::Function(cb) => {
                        let id = after_check::register_callback(lua, cb.clone())?;
                        let args = lua.create_table()?;
                        args.raw_set("callbackId", id)?;
                        call.raw_set("args", args)?;
                    }
                    Value::Table(tbl) => {
                        if let Ok(cb) = tbl.raw_get::<Function>("more") {
                            let id = after_check::register_callback(lua, cb)?;
                            tbl.raw_set("callbackId", id)?;
                        }
                        call.raw_set("args", tbl)?;
                    }
                    _ => {
                        call.raw_set("args", args_val)?;
                    }
                }
                Ok(call)
            })?;
            module.raw_set(verb, call)?;
        } else if verb == "for_each_card" {
            let call = lua.create_function(move |lua, args_val: Value| {
                let call = lua.create_table()?;
                call.raw_set("verb", "for_each_card")?;
                if let Value::Table(tbl) = &args_val {
                    if let Ok(cb) = tbl.raw_get::<Function>("cards") {
                        let id = after_check::register_callback(lua, cb)?;
                        tbl.raw_set("cardsCallbackId", id)?;
                    }
                    if let Ok(cb) = tbl.raw_get::<Function>("each") {
                        let id = after_check::register_callback(lua, cb)?;
                        tbl.raw_set("eachCallbackId", id)?;
                    }
                }
                call.raw_set("args", args_val)?;
                Ok(call)
            })?;
            module.raw_set(verb, call)?;
        } else {
            let call = lua.create_function(move |lua, args: Value| {
                let call = lua.create_table()?;
                call.raw_set("verb", verb)?;
                call.raw_set("args", args)?;
                Ok(call)
            })?;
            module.raw_set(verb, call)?;
        }
    }
    module.raw_set("div", integer_function(lua, "div", numbers::div)?)?;
    module.raw_set("rem", integer_function(lua, "rem", numbers::rem)?)?;
    module.raw_set("sort_by", lua.create_function(sort_by)?)?;
    module.raw_set("query", lua.create_function(query_catalog)?)?;
    module.raw_set("pool", lua.create_function(pool_catalog)?)?;
    module.raw_set("cost", lua.create_function(cost_catalog)?)?;

    let trap_types = lua.create_table()?;
    trap_types.raw_set(1, "Trap")?;
    trap_types.raw_set(2, "Field Trap")?;
    module.raw_set("TRAP_TYPES", trap_types)?;

    module.set_readonly(true);
    Ok(module)
}

/// `J.div` or `J.rem`: two `i32`s in, one out, or an error where Rust's operator would panic.
fn integer_function(
    lua: &Lua,
    name: &'static str,
    f: fn(i32, i32) -> Option<i32>,
) -> mlua::Result<Function> {
    lua.create_function(move |_, (a, b): (Value, Value)| {
        let (a, b) = (arg::<i32>(a)?, arg::<i32>(b)?);
        f(a, b).ok_or_else(|| Error::runtime(format!("J.{name}({a}, {b}) has no i32 answer")))
    })
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
    let json: Json = serde_json::from_value(arg::<Json>(args)?).map_err(|e| Error::runtime(e.to_string()))?;
    let query_args: jackioh_engine::CatalogQueryArgs =
        serde_json::from_value(json).map_err(|e| Error::runtime(e.to_string()))?;
    let defs = jackioh_engine::catalog::query(&query_args);
    lua.to_value(&defs)
}

fn pool_catalog(lua: &Lua, (own_id, args): (String, Value)) -> mlua::Result<Value> {
    let json: Json = serde_json::from_value(arg::<Json>(args)?).map_err(|e| Error::runtime(e.to_string()))?;
    let query_args: jackioh_engine::CatalogQueryArgs =
        serde_json::from_value(json).map_err(|e| Error::runtime(e.to_string()))?;
    let defs = jackioh_engine::catalog::query(&jackioh_engine::catalog::excluding_def_id(
        None,
        &query_args,
        Some(&own_id),
    ));
    lua.to_value(&defs)
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
fn kind(value: &Json) -> &'static str {
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
                    Json::Number(num) => num
                        .as_i64()
                        .and_then(|v| i32::try_from(v).ok())
                        .unwrap_or(0),
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
