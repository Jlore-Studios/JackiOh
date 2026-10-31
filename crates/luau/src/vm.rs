//! One VM per thread (#442 L8). Thread builds its sandbox on first hook call and caches compiled chunks.
//! Nested calls draw on in-flight budget up to `LUAU_HOOK_DEPTH` deep (L5).

use std::cell::Cell;
use std::sync::Arc;

use indexmap::IndexMap;
use jackioh_engine::{CardScripts, EffectContext};
use mlua::chunk::ChunkMode;
use mlua::{Function, Lua, Table, Value};

use crate::api::{self, EffectCall};
use crate::config::{LUAU_HOOK_DEPTH, LUAU_MEMORY_COLLECT_BYTES};
use crate::numbers;
use crate::sandbox::{
    Budget, MemoryCap, Sandbox, new_sandbox, out_of_memory, ran_out_of_memory, stopped, too_deep,
};
use crate::{Face, LuauError, Site};

/// Hooks a face may declare (`start_of_game` is Rust only, L14).
pub const HOOKS: [&str; 31] = [
    "cry",
    "death",
    "delayed",
    "startOfTurn",
    "endOfTurn",
    "onPlayHook",
    "whenAttacks",
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
    "revealEcho",
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

/// Named sub-hooks a face may declare beside its hooks (Classic #39).
pub const SUB_HOOKS: [&str; 8] = [
    "after_check",
    "draw_more",
    "each_cards",
    "each_card",
    "kill_pairs",
    "kill_then",
    "aura_filter",
    "admits",
];

/// Whether a face key is a named sub-hook.
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

/// Thread VM state holding chunk cache, kit environment (#913), and custom scripts (R915002).
pub(crate) struct Vm {
    pub(crate) chunks: Table,
    pub(crate) kit_env: Table,
    pub(crate) custom: Cell<IndexMap<String, Arc<CardScripts>>>,
    pub(crate) sandbox: Sandbox,
}

// CLAUDE.md rule 4 exception (#442 L8): thread-local VM caching only bytecode chunks, no game state.
thread_local! {
    pub(crate) static VM: Vm = Vm::new();
}

impl Vm {
    fn new() -> Vm {
        let sandbox = new_sandbox().expect("the Luau sandbox builds");
        crate::reads::register_types(&sandbox.lua).expect("the readers register");
        let chunks = sandbox.lua.create_table().expect("the chunk cache builds");
        let kit_env = kit_env(&sandbox).expect("the card kit loads");
        Vm {
            chunks,
            kit_env,
            custom: Cell::new(IndexMap::new()),
            sandbox,
        }
    }

    /// Card chunk loaded from bytecode. Custom cards run in kit environment.
    fn chunk(&self, card: &str, bytecode: &[u8]) -> mlua::Result<Function> {
        if let Some(chunk) = self.chunks.raw_get::<Option<Function>>(card)? {
            return Ok(chunk);
        }
        let env = if jackioh_engine::custom::is_custom_id(card) {
            self.kit_env.clone()
        } else {
            self.sandbox.env.clone()
        };
        let chunk = self
            .sandbox
            .lua
            .load(bytecode)
            .set_mode(ChunkMode::Binary)
            .set_name(format!("={card}"))
            .set_environment(env)
            .into_function()?;
        self.chunks.raw_set(card, &chunk)?;
        Ok(chunk)
    }

    /// The card's module, built by running its chunk on every call (L8).
    fn module(&self, card: &str, bytecode: &[u8]) -> mlua::Result<Table> {
        self.chunk(card, bytecode)?.call::<Table>(())
    }
}

/// #913: card kit environment exposing `Card` and globals with sandboxed `require`.
fn kit_env(sandbox: &Sandbox) -> mlua::Result<Table> {
    let lua = &sandbox.lua;
    let bytecode = crate::compiler()
        .compile(crate::custom::KIT_SOURCE)
        .map_err(|error| mlua::Error::runtime(format!("the card kit does not compile: {error}")))?;
    let kit: Table = lua
        .load(bytecode)
        .set_mode(ChunkMode::Binary)
        .set_name("=kit")
        .set_environment(sandbox.env.clone())
        .call(())?;
    let env = lua.create_table()?;
    env.raw_set("Card", kit.raw_get::<Function>("Card")?)?;
    env.raw_set(
        "require",
        lua.create_function(|_, _: Value| -> mlua::Result<()> {
            Err(mlua::Error::runtime(
                "a custom card's script requires nothing: the card kit is all it needs",
            ))
        })?,
    )?;
    let meta = lua.create_table()?;
    meta.raw_set("__index", sandbox.env.clone())?;
    env.set_metatable(Some(meta))?;
    env.set_readonly(true);
    Ok(env)
}

/// R915002: this thread's scripts for a custom card's chunk name, if built.
pub(crate) fn cached_custom(name: &str) -> Option<Arc<CardScripts>> {
    VM.with(|vm| {
        let map = vm.custom.take();
        let found = map.get(name).cloned();
        vm.custom.set(map);
        found
    })
}

/// R915002: keep a custom card's scripts on this thread.
pub(crate) fn cache_custom(name: String, scripts: Arc<CardScripts>) {
    VM.with(|vm| {
        let mut map = vm.custom.take();
        map.insert(name, scripts);
        vm.custom.set(map);
    })
}

/// #913: validates that a custom card compiled chunk loads as a card module.
pub(crate) fn check_custom_module(name: &str, bytecode: &[u8]) -> Result<(), String> {
    VM.with(|vm| {
        let site = Site {
            card: "custom-check",
            face: Face::Base,
            hook: "load",
        };
        let budget = BudgetGuard::enter(&vm.sandbox.lua, site);
        let returned = budget
            .run(site, || vm.chunk(name, bytecode)?.call::<Value>(()))
            .map_err(|error| error.to_string())?;
        let shaped = match &returned {
            Value::Table(module) => matches!(module.raw_get::<Value>(Face::Base.key()), Ok(Value::Table(_))),
            _ => false,
        };
        if shaped {
            Ok(())
        } else {
            Err("the script must return Card { … }".to_string())
        }
    })
}

/// Guard tracking in-flight hook interrupt budget and memory limits.
struct BudgetGuard<'l> {
    lua: &'l Lua,
    _cap: MemoryCap<'l>,
}

impl<'l> BudgetGuard<'l> {
    fn enter(lua: &'l Lua, site: Site) -> BudgetGuard<'l> {
        if let Some(mut budget) = lua.app_data_mut::<Budget>() {
            budget.enter(site);
        }
        BudgetGuard {
            lua,
            _cap: MemoryCap::hold(lua),
        }
    }

    /// Executes `call` within `LUAU_HOOK_DEPTH` and collects garbage if exceeding threshold.
    fn run<T>(&self, site: Site, call: impl FnOnce() -> mlua::Result<T>) -> mlua::Result<T> {
        let depth = self
            .lua
            .app_data_ref::<Budget>()
            .map_or(0, |budget| budget.calls.len());
        if depth > LUAU_HOOK_DEPTH {
            return Err(mlua::Error::runtime(too_deep(site)));
        }
        if depth == 1 && self.lua.used_memory() > LUAU_MEMORY_COLLECT_BYTES {
            self.lua.gc_collect()?;
        }
        let result = call();
        if result.as_ref().is_err_and(ran_out_of_memory) {
            return Err(mlua::Error::runtime(out_of_memory(site)));
        }
        let spent = self
            .lua
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
        if let Some(mut budget) = self.lua.app_data_mut::<Budget>() {
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
                // Shared adapter serves each run from its own face's table (#464 gap 14).
                let face: Table = match module.raw_get::<Value>(site.face.key()) {
                    Ok(Value::Table(table)) => table,
                    _ => module.raw_get(Face::Base.key())?,
                };
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
    crate::reads::scoped(&vm.sandbox.lua, ctx, |ctx| hook.call::<Value>(ctx))
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
    crate::reads::scoped(&vm.sandbox.lua, args, |ctx| hook.call::<Value>(ctx))
}
