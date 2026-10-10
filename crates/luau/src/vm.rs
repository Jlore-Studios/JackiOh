//! One VM per thread (#442 L8). Each thread builds its sandbox the first time it calls a hook and
//! keeps it, with each card's compiled chunk cached by card id. A hook call runs its card's chunk
//! afresh, so the module, and every module-level local in it, starts new on every call, then calls
//! `module[face][hook]` with `ctx`, a userdata that lives for that call only. A hook may call a reader
//! that runs another card's hook: the thread's VM and `Lua::scope` both take a nested call, which
//! draws on the budget of the call it is nested in, up to `LUAU_HOOK_DEPTH` calls deep (L5).

use jackioh_engine::EffectContext;
use mlua::chunk::ChunkMode;
use mlua::{Function, Lua, Table, Value};

use crate::api::{self, EffectCall};
use crate::config::LUAU_HOOK_DEPTH;
use crate::numbers;
use crate::sandbox::{Budget, Sandbox, new_sandbox, stopped, too_deep};
use crate::{Face, LuauError, Site};

/// The hooks a face may declare: the effect-list hooks `Script::hook_named` answers, in its order.
pub const HOOKS: [&str; 9] = [
    "cry",
    "death",
    "startOfGame",
    "delayed",
    "startOfTurn",
    "endOfTurn",
    "onPlayHook",
    "afterAttack",
    "startOfOpponentTurn",
];

/// A thread's VM: the chunk cache, then the sandbox, which drops last.
struct Vm {
    /// Card id to its loaded chunk, in a Lua table, so the cache needs no interior mutability.
    chunks: Table,
    sandbox: Sandbox,
}

// CLAUDE.md rule 4's one exception in this crate (#442 L8): a VM per thread, which keeps only the
// loaded chunks between calls. It holds no game state: every hook call runs its chunk afresh, so what a
// hook returns depends only on its context, on any thread.
thread_local! {
    static VM: Vm = Vm::new();
}

impl Vm {
    fn new() -> Vm {
        let sandbox = new_sandbox().expect("the Luau sandbox builds");
        let chunks = sandbox.lua.create_table().expect("the chunk cache builds");
        Vm { chunks, sandbox }
    }

    /// The card's chunk, loaded from its bytecode the first time this thread asks.
    fn chunk(&self, card: &str, bytecode: &[u8]) -> mlua::Result<Function> {
        if let Some(chunk) = self.chunks.raw_get::<Option<Function>>(card)? {
            return Ok(chunk);
        }
        let chunk = self
            .sandbox
            .lua
            .load(bytecode)
            .set_mode(ChunkMode::Binary)
            .set_name(format!("={card}"))
            .set_environment(self.sandbox.env.clone())
            .into_function()?;
        self.chunks.raw_set(card, &chunk)?;
        Ok(chunk)
    }

    /// The card's module, built by running its chunk: on every call (L8).
    fn module(&self, card: &str, bytecode: &[u8]) -> mlua::Result<Table> {
        self.chunk(card, bytecode)?.call::<Table>(())
    }
}

/// A hook call in flight on the interrupt's budget, left when the call ends, a panic included.
struct BudgetGuard<'l>(&'l Lua);

impl<'l> BudgetGuard<'l> {
    fn enter(lua: &'l Lua, site: Site) -> BudgetGuard<'l> {
        if let Some(mut budget) = lua.app_data_mut::<Budget>() {
            budget.enter(site);
        }
        BudgetGuard(lua)
    }

    /// What `call` returns, unless the call is nested deeper than `LUAU_HOOK_DEPTH`, which it is
    /// never made, or the budget ran out during it: a hook that caught its stop with `pcall` and
    /// returned is stopped all the same.
    fn run<T>(&self, site: Site, call: impl FnOnce() -> mlua::Result<T>) -> mlua::Result<T> {
        let depth = self
            .0
            .app_data_ref::<Budget>()
            .map_or(0, |budget| budget.calls.len());
        if depth > LUAU_HOOK_DEPTH {
            return Err(mlua::Error::runtime(too_deep(site)));
        }
        let result = call();
        let spent = self
            .0
            .app_data_ref::<Budget>()
            .is_some_and(|budget| budget.spent());
        if spent && result.is_ok() {
            return Err(mlua::Error::runtime(stopped(site)));
        }
        result
    }
}

impl Drop for BudgetGuard<'_> {
    fn drop(&mut self) {
        if let Some(mut budget) = self.0.app_data_mut::<Budget>() {
            budget.leave();
        }
    }
}

/// The hooks a card's module declares, by face and then in `HOOKS` order. The module is a table with
/// a `base` table, a `radiant` table or both (the same table for a card whose faces act alike), each
/// mapping hook names to functions; anything else is refused. Running the module spends a budget
/// named for a `load` hook on the base face.
pub fn declared_hooks(card: &'static str, bytecode: &[u8]) -> Result<Vec<(Face, &'static str)>, LuauError> {
    let refused = |message: String| LuauError::Load {
        card: card.to_string(),
        message,
    };
    VM.with(|vm| {
        let module = {
            let site = Site {
                card,
                face: Face::Base,
                hook: "load",
            };
            let budget = BudgetGuard::enter(&vm.sandbox.lua, site);
            budget
                .run(site, || vm.module(card, bytecode))
                .map_err(|error| refused(error.to_string()))?
        };
        let mut hooks = Vec::new();
        for pair in module.pairs::<String, Value>() {
            let (key, value) = pair.map_err(|error| refused(error.to_string()))?;
            let face = match key.as_str() {
                "base" => Face::Base,
                "radiant" => Face::Radiant,
                _ => {
                    return Err(refused(format!(
                        "the module declares {key:?}, which is not base or radiant"
                    )));
                }
            };
            let Value::Table(declared) = value else {
                return Err(refused(format!(
                    "{key} is of type {}, not a table of hooks",
                    value.type_name()
                )));
            };
            for pair in declared.pairs::<String, Value>() {
                let (name, value) = pair.map_err(|error| refused(error.to_string()))?;
                let Some(hook) = HOOKS.iter().find(|hook| **hook == name) else {
                    return Err(refused(format!("{key} declares {name:?}, which is not a hook")));
                };
                if !value.is_function() {
                    return Err(refused(format!(
                        "{key} {name} is of type {}, not a function",
                        value.type_name()
                    )));
                }
                hooks.push((face, *hook));
            }
        }
        hooks.sort_by_key(|(face, hook)| (*face, HOOKS.iter().position(|name| name == hook)));
        Ok(hooks)
    })
}

/// Calls one hook on `ctx` and returns the effects it asks for, walked (L6) but not yet built.
pub fn call_hook(
    site: Site,
    bytecode: &[u8],
    ctx: &mut EffectContext<'_>,
) -> Result<Vec<EffectCall>, LuauError> {
    VM.with(|vm| {
        let returned = {
            let budget = BudgetGuard::enter(&vm.sandbox.lua, site);
            budget.run(site, || run(vm, site, bytecode, ctx))
        };
        let returned = returned.map_err(|error| LuauError::Hook {
            site,
            message: error.to_string(),
        })?;
        api::effect_calls(&numbers::to_json(&returned, site)?, site)
    })
}

/// The module built afresh, its hook found and called with a `ctx` scoped to this call.
fn run(vm: &Vm, site: Site, bytecode: &[u8], ctx: &mut EffectContext<'_>) -> mlua::Result<Value> {
    let module = vm.module(site.card, bytecode)?;
    let face: Table = module.raw_get(site.face.key())?;
    let hook: Function = face.raw_get(site.hook)?;
    vm.sandbox.lua.scope(|scope| {
        let ctx = scope.create_any_userdata(ctx, api::register_ctx)?;
        hook.call::<Value>(ctx)
    })
}
