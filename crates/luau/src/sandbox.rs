//! The sandbox (#442 L5): the Luau a card script runs in. Five libraries (string, table, math, bit32
//! and utf8), with the base library's ways out of a pure function taken away, a `require` that answers
//! only the host's own module, Luau's own sandbox (`Lua::sandbox(true)`: every library and the
//! globals read-only) and an interrupt that stops a hook after `LUAU_HOOK_INTERRUPTS` interrupts.

use mlua::{Error, Lua, LuaOptions, StdLib, Table, Value, VmState};

use crate::Site;
use crate::api;
use crate::config::LUAU_HOOK_INTERRUPTS;

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

/// `math`'s functions a script never sees: every random draw goes through the engine's `Rng`
/// (CLAUDE.md rule 4), and `noise` is a float.
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

/// What a running hook may still spend (L5): the interrupts left before it is stopped, and the hook
/// to name when it is. The VM keeps a stack of them, one per hook call in flight, so a nested call
/// spends its own.
pub struct Budget {
    pub site: Site,
    pub left: u32,
}

impl Budget {
    pub fn new(site: Site) -> Budget {
        Budget {
            site,
            left: LUAU_HOOK_INTERRUPTS,
        }
    }
}

/// A Luau with the sandbox above, the host's module behind `require`, and the interrupt.
pub fn new_sandbox() -> mlua::Result<Sandbox> {
    let lua = Lua::new_with(
        StdLib::STRING | StdLib::TABLE | StdLib::MATH | StdLib::BIT | StdLib::UTF8,
        LuaOptions::default(),
    )?;
    let env = lua.globals();
    for name in REMOVED_GLOBALS.iter().chain(&ABSENT_LIBRARIES) {
        env.raw_set(*name, Value::Nil)?;
    }
    let math: Table = env.raw_get("math")?;
    for name in REMOVED_MATH {
        math.raw_set(name, Value::Nil)?;
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
    lua.set_app_data(Vec::<Budget>::new());
    lua.set_interrupt(interrupt);
    Ok(Sandbox { env, lua })
}

/// L5's cap: the hook on top of the budget stack spends one interrupt, and is stopped, naming its
/// card, face and hook, once it has none left. Outside a hook call nothing is counted.
fn interrupt(lua: &Lua) -> mlua::Result<VmState> {
    let Some(mut budgets) = lua.app_data_mut::<Vec<Budget>>() else {
        return Ok(VmState::Continue);
    };
    let Some(budget) = budgets.last_mut() else {
        return Ok(VmState::Continue);
    };
    if budget.left == 0 {
        return Err(Error::runtime(format!(
            "{}: stopped after {LUAU_HOOK_INTERRUPTS} interrupts (L5)",
            budget.site
        )));
    }
    budget.left -= 1;
    Ok(VmState::Continue)
}
