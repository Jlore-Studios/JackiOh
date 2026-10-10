//! The sandbox (#442 L5): the Luau a card script runs in. Five libraries (string, table, math, bit32
//! and utf8), with the base library's ways out of a pure function taken away, a `require` that answers
//! only the host's own module, Luau's own sandbox (`Lua::sandbox(true)`: every library and the
//! globals read-only) and an interrupt that stops a hook after `LUAU_HOOK_INTERRUPTS` interrupts.
//! What the lint refuses by name (L6, L7) is taken away here too, so a key the lint cannot read
//! (`_G["pairs"]`, `math["sqrt"]`) finds nothing.

use mlua::{Error, Lua, LuaOptions, StdLib, Table, Value, VmState};

use crate::Site;
use crate::api;
use crate::config::LUAU_HOOK_INTERRUPTS;
use crate::lint::MATH_ALLOWED;

/// The base library's globals a script never sees: output, environments, loading code at run time,
/// the collector and `newproxy`'s metatables.
pub const REMOVED_GLOBALS: [&str; 8] = [
    "print",
    "getfenv",
    "setfenv",
    "loadstring",
    "load",
    "collectgarbage",
    "gcinfo",
    "newproxy",
];

/// The libraries a script never sees. None of them is opened; each is cleared too, so a later mlua
/// that opened one by default could not hand it over.
pub const ABSENT_LIBRARIES: [&str; 4] = ["os", "io", "debug", "coroutine"];

/// The base library's iterators in the hash's order (L7), which the lint refuses as `pairs` and
/// `next`.
pub const REMOVED_ITERATORS: [&str; 2] = ["pairs", "next"];

/// `table`'s functions a script never sees (L7): `sort`, which is not stable, and `foreach`, which
/// runs in the hash's order. The lint refuses both.
pub const REMOVED_TABLE: [&str; 2] = ["sort", "foreach"];

/// `math`'s functions L5 names: every random draw goes through the engine's `Rng` (CLAUDE.md
/// rule 4), and `noise` is a float. `math` keeps only `lint::MATH_ALLOWED`, so these go with every
/// other member the lint refuses (L6).
pub const REMOVED_MATH: [&str; 3] = ["random", "randomseed", "noise"];

/// The one name `require` answers.
pub const MODULE: &str = "@jackioh";

/// A sandboxed Luau and the read-only table every chunk runs in. `Lua::sandbox(true)` gives the main
/// thread a fresh table over the globals that takes writes; `env` is the globals themselves, so a
/// chunk loaded with it as its environment fails on any global write. `lua` is last, so it drops last.
pub struct Sandbox {
    pub env: Table,
    pub lua: Lua,
}

/// What the hook calls in flight may still spend (L5). The outermost call starts the budget at
/// `LUAU_HOOK_INTERRUPTS`, and every hook it runs from a reader draws on the same one, so the cap
/// bounds all the work one engine hook call does. `calls` are the calls in flight, outermost first;
/// the last is the one an interrupt names when it stops them.
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

    /// A call begins: the outermost one starts a full budget, a nested one goes on drawing it down.
    pub fn enter(&mut self, site: Site) {
        if self.calls.is_empty() {
            self.left = LUAU_HOOK_INTERRUPTS;
        }
        self.calls.push(site);
    }

    /// The innermost call ends.
    pub fn leave(&mut self) {
        self.calls.pop();
    }

    /// Whether the calls in flight have nothing left: a call that caught its stop (`pcall`) and
    /// returned is failed all the same.
    pub fn spent(&self) -> bool {
        self.left == 0
    }
}

/// The error a hook stopped at L5's cap fails with, naming its card, face and hook.
pub fn stopped(site: Site) -> String {
    format!("{site}: stopped after {LUAU_HOOK_INTERRUPTS} interrupts (L5)")
}

/// A Luau with the sandbox above, the host's module behind `require`, and the interrupt. Rust panics
/// are not Lua errors here (`catch_rust_panics(false)`): `pcall` and `xpcall` raise a panic again,
/// so a hook cannot catch a nested hook's failure and go on.
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
    Ok(Sandbox { env, lua })
}

/// L5's cap: the calls in flight spend one interrupt, and the innermost is stopped, naming its card,
/// face and hook, once they have none left. Outside a hook call nothing is counted.
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
