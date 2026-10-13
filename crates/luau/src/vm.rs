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

/// The hooks a face may declare. `start_of_game` is not among them: only cards that
/// stay in Rust use it (L14), so a face naming it fails to load like any other
/// unknown key, instead of loading and never firing.
pub const HOOKS: [&str; 30] = [
    "cry",
    "death",
    "delayed",
    "startOfTurn",
    "endOfTurn",
    "onPlayHook",
    "afterAttack",
    "startOfOpponentTurn",
    "aura",
    "cost",
    "entersHand",
    "setStat",
    "conditionMet",
    "preview",
    "costAura",
    "graveyardPlay",
    "faceDownPlay",
    "targetingDiscards",
    "drawLimit",
    "discardGuard",
    "heroGuard",
    "attackMods",
    "conditionalKeywords",
    "afterAttacked",
    "plagueMultiplier",
    "echoX",
    "tributeWhen",
    "wouldCounter",
    "turnHookExtra",
    "cryDeathExtra",
];

/// Named sub-hooks a face may declare beside its hooks: the functions closure-taking
/// effect verbs call back by name (`after_state_check`, `draw_while`, `for_each_card`,
/// `with_kill_credit`, and the generic aura's `applies` predicate).
/// A second use of one verb in the same face appends `_2`, `_3`, … (`each_cards_2`),
/// since one face holds two `for_each_card`s with different closures (Classic #39).
/// Anything else unknown is still a load error, so a misspelled hook stays one.
pub const SUB_HOOKS: [&str; 7] = [
    "after_check",
    "draw_more",
    "each_cards",
    "each_card",
    "kill_pairs",
    "kill_then",
    "aura_filter",
];

/// Whether a face key is a named sub-hook: one of `SUB_HOOKS`, optionally with a
/// `_<n>` suffix for a second use of the verb in the same face.
pub fn is_sub_hook(name: &str) -> bool {
    SUB_HOOKS.iter().any(|base| {
        *base == name
            || name
                .strip_prefix(format!("{base}_").as_str())
                .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()))
    })
}

/// Declarations a face may declare: target choices, modes, static flags, activations, triggers, etc.
pub const DECLARATIONS: [&str; 19] = [
    "targets",
    "modes",
    "secretModes",
    "secret_modes",
    "staticFlags",
    "static_flags",
    "activations",
    "triggers",
    "handTriggers",
    "hand_triggers",
    "deckTriggers",
    "deck_triggers",
    "graveyardTriggers",
    "graveyard_triggers",
    "replacements",
    "targetChecks",
    "target_checks",
    "resume",
    "grants",
];

#[derive(Clone, Debug, Default)]
pub struct FaceDecl {
    pub targets: Vec<jackioh_engine::TargetDecl>,
    pub modes: Vec<jackioh_engine::ModeDecl>,
    pub static_flags: Option<jackioh_engine::StaticFlags>,
    pub hooks: Vec<&'static str>,
}

#[derive(Clone, Debug)]
pub enum RadiantDecl {
    Base,
    Face(Box<FaceDecl>),
}

#[derive(Clone, Debug)]
pub struct CardDecl {
    pub base: FaceDecl,
    pub radiant: RadiantDecl,
}

/// A thread's VM: the chunk cache, then the sandbox, which drops last.
pub(crate) struct Vm {
    /// Card id to its loaded chunk, in a Lua table, so the cache needs no interior mutability.
    pub(crate) chunks: Table,
    pub(crate) sandbox: Sandbox,
}

// CLAUDE.md rule 4's one exception in this crate (#442 L8): a VM per thread, which keeps only the
// loaded chunks between calls. It holds no game state: every hook call runs its chunk afresh, so what a
// hook returns depends only on its context, on any thread.
thread_local! {
    pub(crate) static VM: Vm = Vm::new();
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

pub(crate) fn with_module<T>(
    card: &'static str,
    bytecode: &[u8],
    f: impl FnOnce(&Lua, Table) -> Result<T, LuauError>,
) -> Result<T, LuauError> {
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
        f(&vm.sandbox.lua, module)
    })
}

pub(crate) fn with_vm<T>(
    site: Site,
    bytecode: &[u8],
    f: impl FnOnce(&Lua, Table) -> mlua::Result<T>,
) -> Result<T, LuauError> {
    VM.with(|vm| {
        let returned = {
            let budget = BudgetGuard::enter(&vm.sandbox.lua, site);
            budget.run(site, || {
                let module = vm.module(site.card, bytecode)?;
                let face: Table = module.raw_get(site.face.key())?;
                f(&vm.sandbox.lua, face)
            })
        };
        returned.map_err(|error| LuauError::Hook {
            site,
            message: error.to_string(),
        })
    })
}

/// Inspects a card's module to extract declarations and hooks for base and radiant faces.
pub fn inspect_card(card: &'static str, bytecode: &[u8]) -> Result<CardDecl, LuauError> {
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

        let mut base_decl = None;
        let mut radiant_decl = None;

        for pair in module.pairs::<String, Value>() {
            let (key, value) = pair.map_err(|error| refused(error.to_string()))?;
            match key.as_str() {
                "base" => {
                    let Value::Table(table) = value else {
                        return Err(refused(format!(
                            "base is of type {}, not a table of hooks",
                            value.type_name()
                        )));
                    };
                    base_decl = Some(read_face_decl(card, Face::Base, &table)?);
                }
                "radiant" => match value {
                    Value::String(s) => {
                        if s.to_str().is_ok_and(|str_val| str_val == "base") {
                            radiant_decl = Some(RadiantDecl::Base);
                        } else {
                            return Err(refused(format!(
                                "radiant string must be \"base\", got {:?}",
                                s.to_string_lossy()
                            )));
                        }
                    }
                    Value::Table(table) => {
                        radiant_decl = Some(RadiantDecl::Face(Box::new(read_face_decl(
                            card,
                            Face::Radiant,
                            &table,
                        )?)));
                    }
                    _ => {
                        return Err(refused(format!(
                            "radiant is of type {}, not a table of hooks",
                            value.type_name()
                        )));
                    }
                },
                _ => {
                    return Err(refused(format!(
                        "the module declares {key:?}, which is not base or radiant"
                    )));
                }
            }
        }

        let base = base_decl.ok_or_else(|| refused("the module declares no base face".to_string()))?;
        let radiant = radiant_decl.unwrap_or_else(|| RadiantDecl::Face(Box::default()));

        Ok(CardDecl { base, radiant })
    })
}

fn read_face_decl(card: &'static str, face: Face, table: &Table) -> Result<FaceDecl, LuauError> {
    let refused = |message: String| LuauError::Load {
        card: card.to_string(),
        message,
    };
    let mut decl = FaceDecl::default();

    for pair in table.pairs::<String, Value>() {
        let (name, value) = pair.map_err(|error| refused(error.to_string()))?;
        if let Some(hook) = HOOKS.iter().find(|h| {
            **h == name || crate::hooks::camel_to_snake(h) == name || crate::hooks::snake_to_camel(h) == name
        }) {
            if !value.is_function() {
                return Err(refused(format!(
                    "{} {name} is of type {}, not a function",
                    face.key(),
                    value.type_name()
                )));
            }
            decl.hooks.push(*hook);
        } else if DECLARATIONS.iter().any(|d| {
            *d == name || crate::hooks::camel_to_snake(d) == name || crate::hooks::snake_to_camel(d) == name
        }) {
            let site = Site {
                card,
                face,
                hook: "load",
            };
            let json = numbers::to_json(&value, site)?;
            match name.as_str() {
                "targets" => {
                    decl.targets = serde_json::from_value(json)
                        .map_err(|err| refused(format!("{} targets: {err}", face.key())))?;
                }
                "modes" => {
                    decl.modes = serde_json::from_value(json)
                        .map_err(|err| refused(format!("{} modes: {err}", face.key())))?;
                }
                "static_flags" | "staticFlags" => {
                    decl.static_flags = Some(
                        serde_json::from_value(json)
                            .map_err(|err| refused(format!("{} static_flags: {err}", face.key())))?,
                    );
                }
                _ => {}
            }
        } else if is_sub_hook(&name) {
            if !value.is_function() {
                return Err(refused(format!(
                    "{} {name} is of type {}, not a function",
                    face.key(),
                    value.type_name()
                )));
            }
            // A named sub-hook: called back by name, never adapted into `Script`.
        } else {
            return Err(refused(format!(
                "{} declares {name:?}, which is not a hook",
                face.key()
            )));
        }
    }

    decl.hooks
        .sort_by_key(|hook| HOOKS.iter().position(|name| name == hook));
    Ok(decl)
}

/// The hooks a card's module declares, by face and then in `HOOKS` order.
pub fn declared_hooks(card: &'static str, bytecode: &[u8]) -> Result<Vec<(Face, &'static str)>, LuauError> {
    let decl = inspect_card(card, bytecode)?;
    let mut hooks = Vec::new();
    for hook in decl.base.hooks {
        hooks.push((Face::Base, hook));
    }
    match decl.radiant {
        RadiantDecl::Face(face) => {
            for hook in face.hooks {
                hooks.push((Face::Radiant, hook));
            }
        }
        RadiantDecl::Base => {}
    }
    hooks.sort_by_key(|(face, hook)| (*face, HOOKS.iter().position(|name| name == hook)));
    Ok(hooks)
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
        let ctx = scope.create_any_userdata(ctx, crate::reads::register_ctx)?;
        hook.call::<Value>(ctx)
    })
}

/// Calls an `aura` hook on `args` and returns the aura entries it specifies.
pub fn call_aura_hook<'a>(
    site: Site,
    bytecode: &'static [u8],
    args: jackioh_engine::script::HookArgs<'a>,
) -> Result<Vec<jackioh_engine::script::AuraEntry<'a>>, LuauError> {
    VM.with(|vm| {
        let returned = {
            let budget = BudgetGuard::enter(&vm.sandbox.lua, site);
            budget.run(site, || run_aura(vm, site, bytecode, args))
        };
        let returned = returned.map_err(|error| LuauError::Hook {
            site,
            message: error.to_string(),
        })?;
        let calls = api::effect_calls(&numbers::to_json(&returned, site)?, site)?;
        api::aura_entries(&calls, site, bytecode, args)
    })
}

fn run_aura(
    vm: &Vm,
    site: Site,
    bytecode: &[u8],
    args: jackioh_engine::script::HookArgs<'_>,
) -> mlua::Result<Value> {
    let module = vm.module(site.card, bytecode)?;
    let face: Table = module.raw_get(site.face.key())?;
    let hook: Function = face.raw_get(site.hook)?;
    vm.sandbox.lua.scope(|scope| {
        let ctx = scope.create_any_userdata(args, crate::reads::register_hook_args)?;
        hook.call::<Value>(ctx)
    })
}
