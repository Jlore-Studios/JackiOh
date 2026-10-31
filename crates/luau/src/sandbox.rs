//! The sandbox (#442 L5): Luau runtime environment for card scripts. Standard libraries
//! restricted with globals removed (L6, L7), module access limited to `@jackioh`,
//! interrupt budget `LUAU_HOOK_INTERRUPTS`, and memory cap `LUAU_MEMORY_BYTES` (#843).

use mlua::{Error, Lua, LuaOptions, StdLib, Table, Value, VmState};

use crate::Site;
use crate::api;
use crate::config::{CUSTOM_HOOK_INTERRUPTS, LUAU_HOOK_DEPTH, LUAU_HOOK_INTERRUPTS, LUAU_MEMORY_BYTES};
use crate::lint::MATH_ALLOWED;

/// Base library globals removed (L5): output, environments, dynamic loading, gc, metatables, xpcall.
pub const REMOVED_GLOBALS: [&str; 9] = [
    "print",
    "getfenv",
    "setfenv",
    "loadstring",
    "load",
    "collectgarbage",
    "gcinfo",
    "newproxy",
    "xpcall",
];

/// Libraries completely absent from the sandbox.
pub const ABSENT_LIBRARIES: [&str; 7] = ["os", "io", "debug", "coroutine", "buffer", "vector", "integer"];

/// Base library iterators removed (L7).
pub const REMOVED_ITERATORS: [&str; 2] = ["pairs", "next"];

/// `table` functions removed (L7): unstable or hash-ordered.
pub const REMOVED_TABLE: [&str; 2] = ["sort", "foreach"];

/// `math` functions removed (L5, L6): random draws must use engine `Rng` (CLAUDE.md rule 4).
pub const REMOVED_MATH: [&str; 3] = ["random", "randomseed", "noise"];

/// Builtins disabled in Luau's compiler to prevent fastcalls bypassing removed globals.
pub const DISABLED_BUILTINS: [&str; 54] = [
    "vector.create",
    "vector.magnitude",
    "vector.normalize",
    "vector.cross",
    "vector.dot",
    "vector.floor",
    "vector.ceil",
    "vector.abs",
    "vector.sign",
    "vector.clamp",
    "vector.min",
    "vector.max",
    "vector.lerp",
    "buffer.readi8",
    "buffer.readu8",
    "buffer.writeu8",
    "buffer.readi16",
    "buffer.readu16",
    "buffer.writeu16",
    "buffer.readi32",
    "buffer.readu32",
    "buffer.writeu32",
    "buffer.readf32",
    "buffer.writef32",
    "buffer.readf64",
    "buffer.writef64",
    "buffer.readinteger",
    "buffer.writeinteger",
    "math.acos",
    "math.asin",
    "math.atan2",
    "math.atan",
    "math.cosh",
    "math.cos",
    "math.deg",
    "math.exp",
    "math.fmod",
    "math.frexp",
    "math.ldexp",
    "math.log10",
    "math.log",
    "math.modf",
    "math.pow",
    "math.rad",
    "math.sinh",
    "math.sin",
    "math.sqrt",
    "math.tanh",
    "math.tan",
    "math.round",
    "math.lerp",
    "math.isnan",
    "math.isinf",
    "math.isfinite",
];

/// The one name `require` answers.
pub const MODULE: &str = "@jackioh";

/// Sandboxed Luau and read-only global table.
pub struct Sandbox {
    pub env: Table,
    pub lua: Lua,
}

/// Interrupt budget for in-flight hook calls (L5).
pub struct Budget {
    pub calls: Vec<Site>,
    pub left: u32,
}

impl Budget {
    /// No call in flight.
    fn new() -> Budget {
        Budget {
            calls: Vec::new(),
            left: LUAU_HOOK_INTERRUPTS,
        }
    }

    /// A call begins: outermost call starts full budget (R915004 for custom cards).
    pub fn enter(&mut self, site: Site) {
        if self.calls.is_empty() {
            self.left = budget_of(site);
        }
        self.calls.push(site);
    }

    /// The innermost call ends.
    pub fn leave(&mut self) {
        self.calls.pop();
    }

    /// Whether the calls in flight have nothing left.
    pub fn spent(&self) -> bool {
        self.left == 0
    }
}

/// Scoped VM memory cap guard (#843).
pub(crate) struct MemoryCap<'l> {
    lua: &'l Lua,
    previous: usize,
}

impl<'l> MemoryCap<'l> {
    /// The cap at `LUAU_MEMORY_BYTES`, as a hook call runs under it.
    pub(crate) fn hold(lua: &'l Lua) -> MemoryCap<'l> {
        MemoryCap::set(lua, LUAU_MEMORY_BYTES)
    }

    /// No cap (mlua's limit 0), as a reader builds its answer (`uncapped`).
    fn lift(lua: &'l Lua) -> MemoryCap<'l> {
        MemoryCap::set(lua, 0)
    }

    fn set(lua: &'l Lua, limit: usize) -> MemoryCap<'l> {
        let previous = lua.set_memory_limit(limit).unwrap_or(limit);
        MemoryCap { lua, previous }
    }
}

impl Drop for MemoryCap<'_> {
    fn drop(&mut self) {
        let _ = self.lua.set_memory_limit(self.previous);
    }
}

/// Executes `build` with memory cap lifted, restoring and checking limits afterward (#871).
pub(crate) fn uncapped<T>(lua: &Lua, build: impl FnOnce() -> mlua::Result<T>) -> mlua::Result<T> {
    let cap = MemoryCap::lift(lua);
    let answer = build()?;
    let limit = cap.previous;
    drop(cap);
    if limit > 0 && lua.used_memory() > limit {
        return Err(Error::MemoryError(format!(
            "a reader's answer left the VM past {limit} bytes"
        )));
    }
    Ok(answer)
}

/// Interrupt budget for a hook starting at `site` (R915004, L5).
pub fn budget_of(site: Site) -> u32 {
    if crate::custom::is_custom_site(site) {
        CUSTOM_HOOK_INTERRUPTS
    } else {
        LUAU_HOOK_INTERRUPTS
    }
}

/// The error a hook stopped at L5's cap fails with, naming its card, face and hook.
pub fn stopped(site: Site) -> String {
    format!("{site}: stopped after {} interrupts (L5)", budget_of(site))
}

/// The error a hook call nested deeper than `LUAU_HOOK_DEPTH` fails with, naming its card, face and hook.
pub fn too_deep(site: Site) -> String {
    format!("{site}: hook calls nested deeper than {LUAU_HOOK_DEPTH} (L5)")
}

/// The error a hook call that ran out of memory fails with, naming its card, face and hook.
pub fn out_of_memory(site: Site) -> String {
    format!("{site}: out of memory, past {LUAU_MEMORY_BYTES} bytes (L5)")
}

/// Whether `error` is an allocation past `LUAU_MEMORY_BYTES`.
pub fn ran_out_of_memory(error: &Error) -> bool {
    error
        .chain()
        .any(|layer| matches!(layer.downcast_ref::<Error>(), Some(Error::MemoryError(_))))
}

/// Creates a new sandboxed Luau state with modules, interrupt handler, and memory limits.
pub fn new_sandbox() -> mlua::Result<Sandbox> {
    let lua = Lua::new_with(
        StdLib::STRING | StdLib::TABLE | StdLib::MATH | StdLib::BIT | StdLib::UTF8,
        LuaOptions::new().catch_rust_panics(false),
    )?;
    let env = lua.globals();
    for name in REMOVED_GLOBALS
        .iter()
        .chain(&ABSENT_LIBRARIES)
        .chain(&REMOVED_ITERATORS)
    {
        env.raw_set(*name, Value::Nil)?;
    }
    let table: Table = env.raw_get("table")?;
    for name in REMOVED_TABLE {
        table.raw_set(name, Value::Nil)?;
    }
    let math: Table = env.raw_get("math")?;
    let mut members = Vec::new();
    for pair in math.pairs::<String, Value>() {
        members.push(pair?.0);
    }
    for name in members {
        if !MATH_ALLOWED.contains(&name.as_str()) {
            math.raw_set(name, Value::Nil)?;
        }
    }
    let module = api::module(&lua)?;
    let require = lua.create_function(move |_, name: String| {
        if name == MODULE {
            Ok(module.clone())
        } else {
            Err(Error::runtime(format!(
                "require answers only \"{MODULE}\", not {name:?}"
            )))
        }
    })?;
    env.raw_set("require", require)?;
    lua.sandbox(true)?;
    lua.set_app_data(Budget::new());
    lua.set_interrupt(interrupt);
    lua.set_memory_limit(LUAU_MEMORY_BYTES)?;
    Ok(Sandbox { env, lua })
}

/// L5 interrupt callback decrementing budget and aborting upon exhaustion.
fn interrupt(lua: &Lua) -> mlua::Result<VmState> {
    let Some(mut budget) = lua.app_data_mut::<Budget>() else {
        return Ok(VmState::Continue);
    };
    let Some(&site) = budget.calls.last() else {
        return Ok(VmState::Continue);
    };
    if budget.spent() {
        return Err(Error::runtime(stopped(site)));
    }
    budget.left -= 1;
    Ok(VmState::Continue)
}
