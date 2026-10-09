//! What a script reaches (#442 L8): the module `require("@jackioh")` answers, `J`, and the `ctx` a
//! hook is called with. Only enough is bound to test the host; part 6 binds the rest.
//!
//! An effect is data. `J.damage(args)` returns `{ verb = "damage", args = args }` and touches
//! nothing, the hook returns a list of them, and the host walks the list (L6) and builds each one
//! with the engine's own verb (CLAUDE.md rule 5). A reader is a method of `ctx` (`ctx:hero_of(p)`),
//! since `ctx` is the only way to the state, and only for as long as the hook call lasts.

use jackioh_engine::effects::{self, DamageEffectArgs, DestroyArgs};
use jackioh_engine::{Effect, EffectContext, PlayerId, query, scripts};
use mlua::{Error, Function, Lua, LuaSerdeExt, Table, UserDataMethods, UserDataRegistry, Value};
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

/// The effect verbs `J` binds, each named as the engine's (`jackioh_engine::effects`).
pub const VERBS: [&str; 2] = ["destroy", "damage"];

/// The module, read-only: one function per verb, and `J.div` and `J.rem`.
pub fn module(lua: &Lua) -> mlua::Result<Table> {
    let module = lua.create_table()?;
    for verb in VERBS {
        let call = lua.create_function(move |lua, args: Value| {
            let call = lua.create_table()?;
            call.raw_set("verb", verb)?;
            call.raw_set("args", args)?;
            Ok(call)
        })?;
        module.raw_set(verb, call)?;
    }
    module.raw_set("div", integer_function(lua, "div", numbers::div)?)?;
    module.raw_set("rem", integer_function(lua, "rem", numbers::rem)?)?;
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

/// A hook's walked return as its effect calls: a list of `{ verb, args }`, or nothing (`nil` or an
/// empty table).
pub fn effect_calls(value: &Json, site: Site) -> Result<Vec<EffectCall>, LuauError> {
    let refused = |message: String| LuauError::Hook { site, message };
    match value {
        Json::Null => Ok(vec![]),
        Json::Object(record) if record.is_empty() => Ok(vec![]),
        Json::Array(items) => items
            .iter()
            .map(|item| {
                let call = item
                    .as_object()
                    .filter(|record| record.keys().all(|key| key == "verb" || key == "args"));
                match call.and_then(|record| record.get("verb")) {
                    Some(Json::String(verb)) => Ok(EffectCall {
                        verb: verb.clone(),
                        args: item.get("args").cloned().unwrap_or(Json::Null),
                    }),
                    _ => Err(refused(format!("{item} is not an effect"))),
                }
            })
            .collect(),
        other => Err(refused(format!("a hook returns a list of effects, not {other}"))),
    }
}

/// The engine's effect for one call.
pub fn effect_of(call: &EffectCall, site: Site) -> Result<Effect, LuauError> {
    match call.verb.as_str() {
        "destroy" => typed::<DestroyArgs>(call, site).map(effects::destroy),
        "damage" => typed::<DamageEffectArgs>(call, site).map(effects::damage),
        verb => Err(LuauError::Hook {
            site,
            message: format!("no verb {verb:?}"),
        }),
    }
}

/// A call's arguments as the verb's own argument type.
fn typed<T: DeserializeOwned>(call: &EffectCall, site: Site) -> Result<T, LuauError> {
    serde_json::from_value(call.args.clone()).map_err(|error| LuauError::Hook {
        site,
        message: format!("{}: {error}", call.verb),
    })
}

/// `ctx`'s methods: the readers a hook calls on its context.
pub fn register_ctx(registry: &mut UserDataRegistry<&mut EffectContext<'_>>) {
    registry.add_method("controller", |lua, ctx, ()| lua.to_value(&ctx.controller));
    registry.add_method("hero_of", |lua, ctx, player: Value| {
        lua.to_value(&query::hero_of(ctx.state, arg::<PlayerId>(player)?))
    });
    // Another card's hook, run on this context: the kinds of the effects it returns. A stand-in for
    // part 6's readers that run a script (a reader of what a card would do), so the host's nesting is
    // tested before any of them exists.
    registry.add_method_mut(
        "effects_of",
        |lua, ctx, (card, face, hook): (Value, Value, Value)| {
            let (card, face, hook) = (arg::<String>(card)?, arg::<String>(face)?, arg::<String>(hook)?);
            let scripts = scripts::scripts_ref(ctx.state, &card);
            let script = match face.as_str() {
                "base" => &scripts.base,
                "radiant" => &scripts.radiant,
                other => return Err(Error::runtime(format!("{other:?} is not base or radiant"))),
            };
            let Some(run) = script.hook_named(&hook).cloned() else {
                return Err(Error::runtime(format!("{card} {face} has no {hook}")));
            };
            let kinds: Vec<&str> = run(ctx).iter().map(|effect| effect.kind).collect();
            lua.to_value(&kinds)
        },
    );
}
