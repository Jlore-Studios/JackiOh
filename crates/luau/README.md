# `jackioh-luau`

The host that runs card scripts written in Luau (#442, v0.4.0), kept as the scripting host for
custom cards (#902, #915). It holds the sandbox a script runs in, the integer boundary every value
crosses, the lint that refuses what the sandbox cannot stop, one VM per thread, and `load_card`,
which turns a compiled script into the engine's `CardScripts`.

No official card uses it: v0.4.0 ported 409 card scripts to Luau, and v0.5.0 (#902) brought them
all back to Rust, since a VM made every game slower and the browser's module bigger for nothing an
official card needs (`docs/v0.4.0/README.md` §8). No crate the game builds depends on it (#902 V4):
not the cards, the engine, the AI, the server or the tools. It stays a workspace member, built and
tested by CI, for code that is not compiled into the game. The decisions it follows are #442's
L3–L8. This file is the contract.

## Pinned versions

- `mlua` is pinned to `=0.12.2` in the workspace's `Cargo.toml`, with `default-features = false,
  features = ["luau", "serde"]`: no `send`, `async`, `luau-jit` or `module` (L3).
- Its `mlua-sys` 0.13.0 builds Luau from `luau0-src` `0.22.0+luau740`, which is **Luau 0.740**. Part
  8 pins `luau-analyze` to this version, so a bump of `mlua` is a bump of both.
- `luau0-src` compiles Luau's C++ in a build script, so a build of this crate needs a C++ compiler
  (`g++`; GitHub's runners have one).

## Purity

The crate is pure like the engine (CLAUDE.md rule 4): its `Cargo.toml` depends on the engine,
`mlua`, `serde` and `serde_json` only, and its `clippy.toml` is the engine's, byte for byte. A
script sees no clock, no I/O, no environment and no randomness.

The one exception is the VM (L8): `vm.rs`'s `thread_local!` `VM`, which keeps each thread's
sandbox and its cache of loaded chunks between calls. It is the only static in the crate. It holds
no game state, since every hook call runs its card's chunk afresh, so what a hook returns depends
only on its context, on any thread. The cache needs no `RefCell`: it is a Lua table.

## The sandbox (L5)

`sandbox::new_sandbox` builds a Luau with:

- only the `string`, `table`, `math`, `bit32` and `utf8` libraries (`buffer`, `vector` and
  `integer` are not opened);
- `os`, `io`, `debug` and `coroutine` absent, and `print`, `getfenv`, `setfenv`, `loadstring`,
  `load`, `collectgarbage`, `gcinfo` and `newproxy` removed (`REMOVED_GLOBALS`,
  `ABSENT_LIBRARIES`), along with `math.random`, `math.randomseed` and `math.noise`
  (`REMOVED_MATH`). `xpcall` is removed too: a failure raised inside its message handler becomes
  Luau's "error in error handling", which drops what it was, a nested hook's panic included, so a
  hook could catch that failure and go on. `pcall` stays (below);
- no builtin of a library it took away: Luau's compiler calls a builtin written `library.member(…)`
  straight from the bytecode (a fastcall), and folds one with constant arguments, without reading
  the global, so `vector.magnitude` would give a square root in a sandbox with no `vector`.
  `compile` uses `compiler()`, which turns off every `vector` and `buffer` builtin and every `math`
  one outside `lint::MATH_ALLOWED` (`sandbox::DISABLED_BUILTINS`, from Luau 0.740's
  `Builtins.cpp`), so such a call reads the global and finds nothing; the lint refuses the names
  `vector`, `buffer` and `integer` first (`ABSENT_LIBRARIES`). `compiler()` also holds Luau's
  optimization level at 1 (`LUAU_OPTIMIZATION_LEVEL`, mlua's default), since level 2 folds
  `math.pi`, `math.huge` and `math`'s other constants into the bytecode without reading the global,
  which no disabled builtin prevents;
- what the lint refuses by name taken away as well, so a key the lint cannot read (`_G["pairs"]`,
  `math["sqrt"]`) finds nothing: `pairs` and `next` (`REMOVED_ITERATORS`), `table.sort` and
  `table.foreach` (`REMOVED_TABLE`), and every `math` member but `lint::MATH_ALLOWED`;
- a `require` that answers only `"@jackioh"` (the module `J`, below) and fails on anything else;
- `Lua::sandbox(true)`: every library and the globals are read-only. Each chunk runs with the
  globals table itself as its environment (`Sandbox::env`), not the writable proxy the sandbox
  gives the main thread, so writing a global fails ("attempt to modify a readonly table");
- an interrupt that stops a hook after `LUAU_HOOK_INTERRUPTS` interrupts (`config.rs`), with an
  error naming the card, face and hook: `"<card> <face> <hook>: stopped after 1000000 interrupts
  (L5)"`. Luau interrupts on every loop iteration, call and return. The outermost hook call starts
  the budget (`sandbox::Budget`), and a hook it runs from a reader draws on the same one, so the cap
  bounds everything one engine hook call does, nested calls included; the error names the innermost
  call. A spent budget stays spent: a hook that catches its stop with `pcall` is stopped again at
  its next interrupt, and a call that ends with its budget spent fails all the same. Running a
  module to read its declarations spends a budget named for a `load` hook on the base face.
  Hook calls nest at most `LUAU_HOOK_DEPTH` (16) deep: the call past that fails before it runs,
  naming its card, face and hook (`"<card> <face> <hook>: hook calls nested deeper than 16 (L5)"`),
  since a hook that runs itself from a reader spends only a few interrupts a level and would
  otherwise nest until the thread's stack overflowed, which aborts the process;
- a memory cap (`Lua::set_memory_limit`, #843): the thread's VM holds at most `LUAU_MEMORY_BYTES`
  (64 MiB, `config.rs`) all told, its libraries, its chunk cache, what the calls in flight hold and
  its garbage not yet collected included, so a hook that builds one huge string or table without
  looping (`string.rep`), which spends no interrupts, is bounded too. The allocation that would pass
  the cap fails, and the hook call with it, with an error naming the card, face and hook:
  `"<card> <face> <hook>: out of memory, past 67108864 bytes (L5)"`. Every hook call, a nested one
  included, runs under the cap (`vm::BudgetGuard`). A reader, `J.query` and `J.pool` lift it while
  they build their answer (`sandbox::uncapped`), since under a cap mlua runs each value the host
  builds through a protected call, which cost a hook call about half its time; for the same reason
  `J`'s verbs are Luau closures, not Rust functions. So while a reader builds its answer the VM can
  hold more than the cap, by what the reader allocates: the state it reads bounds that, or the
  table the script hands it (`ctx:rng_shuffle`'s list, which the cap held), not anything the script
  does meanwhile. The answer is counted like everything else: a reader that leaves the VM past the
  cap fails as the allocation past it would, and the hook call with it, naming the card, face and
  hook (#871), so a hook never goes on holding more than the cap, however many answers it keeps.
  The script's own allocations stay capped, its next one past the cap after a reader returns
  included. Luau collects nothing when an allocation fails, so an outermost hook call
  that finds the VM holding more than `LUAU_MEMORY_COLLECT_BYTES` (half the cap) first collects all
  its garbage: a call that ran out of memory does not fail the next one. `pcall` catches a memory
  error as it catches any Luau error; the allocation never happened, so the cap holds;
- Rust panics that `pcall` cannot catch (`LuaOptions::catch_rust_panics(false)`): a nested hook
  that fails panics (below), and the panic goes on through the hook that called it, since mlua's
  `pcall` raises it again and runs no handler that could replace it.
  This holds while Luau's fast `pcall` is off (its `LuauFastpcall` and `LuauCompileFastpcall`
  flags, off in 0.740), since that path calls Luau's own `pcall` rather than the global mlua puts
  in its place; `pcall_does_not_catch_a_nested_hooks_failure` fails on a Luau that turns it on.

What is left is exactly these globals, which `the_sandbox_holds_exactly_its_globals` checks: the
five libraries, `_G`, `_VERSION`, `assert`, `error`, `getmetatable`, `ipairs`, `pcall`,
`rawequal`, `rawget`, `rawlen`, `rawset`, `require`, `select`, `setmetatable`, `tonumber`,
`tostring`, `type`, `typeof` and `unpack`. A later `mlua` that adds a global fails that test.

The bytecode a VM loads is the crate's own `compile`'s, built from the repository's sources:
Luau does not verify bytecode, so nothing else may be handed to `load_card`.

## Numbers (L6)

The rules count in `i32`; a Luau number is an `f64`. Every value that leaves Luau passes
`numbers::walk` before it becomes a Rust value: a hook's return, and every argument a `J` function
or a `ctx` reader reads (`numbers::arg`).

- A number crosses only when it is integral, finite and inside `i32`. A fraction, NaN, ±inf or a
  value outside `i32` is an error naming the card, face, hook and value: `"<card> <face> <hook>:
  1.5 is not an integer in i32's range (L6)"` (`LuauError::Number` for a hook's return).
- `nil` is `null`, a boolean a boolean, a string a string (UTF-8 only).
- A table whose keys are exactly `1..=n` is a list, in index order. A table whose keys are all
  strings is a record, its keys sorted. An empty table is `{}`. Mixed keys, a list with holes, a
  function, a userdata, a thread, a buffer or a vector are refused, and so is a table nested
  deeper than `LUAU_VALUE_DEPTH` (32) tables, which is how a table that holds itself ends, and a
  value of more than `LUAU_VALUE_NODES` (10,000) values and keys, all told, which is how a shallow
  table that holds another twice over at every level ends.
- `J.div(a, b)` and `J.rem(a, b)` are Rust's `/` and `%`: they truncate toward zero, and the
  remainder takes the dividend's sign (`J.div(-7, 2)` is `-3`, `J.rem(-7, 2)` is `-1`). A zero
  divisor, or `i32::MIN` by `-1`, is an error (`"J.div(1, 0) has no i32 answer"`). Luau's own `//`
  and `%` floor instead (`-7 // 2` is `-4`, `-7 % 2` is `1`): the lint lets them through, since
  they never make a fraction from integers, but a script that means Rust's arithmetic writes
  `J.div` and `J.rem`.

## Order (L7)

Nothing a script hands over may hang on Luau's hash order. Records cross with their keys sorted
and the declared hooks are read in `vm::HOOKS` order. In the source, the lint refuses what would
iterate a hash (`pairs`, `next`, `table.foreach`, a generic `for` over anything but `ipairs(…)`,
and `ipairs` anywhere but right after `in`, since a name bound over it could hand the `for` a table,
which Luau iterates in the hash's order) and `table.sort`; the sandbox removes the library
functions among them. No randomness reaches a script: every random draw is the engine's (CLAUDE.md
rule 4).

Two ways a script's answer could still differ from run to run are not closed here, and are left to
part 6's bindings and part 8's `luau-analyze`: `tostring` of a table, a function or a userdata (and
so string interpolation of one) gives its address, and a weak table (`__mode`) loses its entries
whenever the collector runs, which hangs on everything the thread's VM did before. The memory cap
(above) adds a third, which nothing closes: whether a hook that allocates close to
`LUAU_MEMORY_BYTES` runs out hangs on what the thread's VM holds besides, so one that catches the
failure with `pcall` could answer differently on another thread. No card's hook comes near the cap.

## Statelessness (L8)

There is one VM per thread, built sandboxed the first time the thread calls a hook. It caches each
card's loaded chunk by card id, so one card id has one bytecode for the life of the thread. Every
hook call runs the card's chunk again, so the module, and every module-level local in it, starts
fresh: a counter a hook increments reads the same on every call. Re-running the chunk is L8's
price: in a release build, a hook called through `load_card`'s closure (the chunk run, the hook
called, its return walked and its effects built) took about 8 µs for a hook with one effect and no
reader, and 11 µs for one with two readers and two effects (measured for #558 on a 4-core runner).
Building a sandbox, once per thread, took about 0.2 ms.

## How a hook is called

A card's module returns a table with a `base` table, a `radiant` table or both (the same table for
a card whose faces act alike, spelled `radiant = "base"`). Each maps hook names to functions; the
names are `vm::HOOKS` (every hook type but L14's `quests`, `records_play_as` and `start_of_game`,
which stay in Rust and are refused here like any other unknown key) and `vm::SUB_HOOKS`, the
named sub-hooks closure-taking verbs call back by name. Any other key, at either level, is
refused when the card loads (`LuauError::Load`), so a misspelled hook stays a load error.

```luau
local J = require("@jackioh")

return {
	base = {
		cry = function(ctx: J.EffectContext)
			local hero = ctx:hero_of(ctx:controller())
			return { J.damage({ to = { of = "enemyHero" }, amount = J.div(hero.health, 7) }) }
		end,
	},
	radiant = "base",
}
```

Annotate every hook's context (`ctx: J.EffectContext`, `ctx: J.ConditionContext`, …): an
unannotated parameter degrades what the readers answer, and the checker cannot see through it
(`#ctx:zone_cards(...)` and `unit.id` in a loop both fail without the annotation). A verb whose
argument is a union (several table shapes, `HealArgs`) cannot narrow an inline literal either:
hoist the inner table into an annotated local (`local target: J.TargetSpec = { of = "selfHero" }`).
A table's fields are invariant, so a value going into an optional field takes that field's type
(`local MARK: J.CardMark? = …` for `DestroyAtNextTurnStartArgs`' `mark`).
`luau-analyze --mode=strict` (Luau 0.740, above) is clean over the module and every card script,
and CI's `rust` check holds it so (#872). Each pure-read context's type (`J.ConditionContext`,
`J.HookArgs`, `J.CostArgs`, …) declares every reader `shared_readers!` registers on it, typed as on
`J.EffectContext`: `tests/export_api.rs` reads the macro, so a reader the host adds is typed at once.
A `.luaurc` beside the scripts aliases `@jackioh` at the committed module, which the CLI resolves
(the mapping drops the `.luau` extension, which luau-analyze 0.740 refuses as `Unknown require`,
#464 gap 15), so local strict runs check the files in place.

- `load_card(id, bytecode)` runs the module once to read which hooks each face declares, and
  fills those slots of `CardScripts` with closures that capture only the hook's `Site` (card, face
  and hook) and the `&'static` bytecode, so any thread's VM can load the chunk the first time it
  needs it. One function value named under two hooks — two hooks of one face, or one hook of
  both faces — fills both slots with one shared closure (#464 gaps 10 and 14). A closure looks its
  function up under the hook it was built for, so it is shared only where the face it runs holds
  the function under that hook (#871): a hook both faces hold the function under gets a closure
  that serves each run from the running face's table, shared with every such hook; any other
  hook's closure runs its own face's table, whatever face the context names, and is shared with
  that face's other hooks holding the function. A function the base face holds under `cry` and
  the Radiant face only under `death` gets one closure a face. No Lua value outlives a call.
- A call runs the module afresh, takes `module[face][hook]` and calls it with `ctx`, a userdata
  scoped to the call (`Lua::scope`), so a script cannot keep it.
- An effect is data. `J.<verb>(args)` returns `{ verb = "<verb>", args = args }` and touches
  nothing. The hook returns a list of them (or `nil`, or `{}`), the host walks it and builds each
  effect with the engine's own verb from `jackioh_engine::effects` (CLAUDE.md rule 5).
- A reader is a method of `ctx`: `ctx:controller()`, `ctx:hero_of(player)`. Its arguments are
  walked like anything else that crosses.
- A hook may call a reader that runs another card's hook: the thread's VM and `Lua::scope` both
  take a nested call, which spends the outer call's budget, up to `LUAU_HOOK_DEPTH` calls deep
  (`ctx:effects_of(card, face, hook)` tests it).
- A hook that fails panics with its error, which names its card, face and hook: an engine `Hook`
  has no error to return. A panic in a nested hook unwinds through the outer one, which `pcall`
  cannot stop, so the outer call panics too.

## Compiling

`compile(file, source)` lints the source and compiles it with Luau's compiler to the bytecode
`load_card` takes, with `compiler()` (the disabled builtins and optimization level 1, above).
v0.4.0's `crates/cards/build.rs` called it on every card's `.luau`; the tests call it on their
fixtures. Its
errors name the file: a lint finding as `"<file>:<line>: <rule>"`, one per line, a syntax error as
`"<file>:<line>: <Luau's message>"`.

The lint (`lint.rs`, L6, L7) is a pure function over the source: a lexer that knows Luau's
comments, strings (interpolated ones included) and operators, then a pass over the tokens. The
lexer reads the source as Luau 0.740's (`Ast/src/Lexer.cpp`) does wherever that decides what is
code: a line comment ends at a newline, a carriage return or a NUL byte; a string breaks at an
unescaped one of them (which Luau refuses, so the code after it is linted); escapes are read as
Luau reads them (`\z` and a `\` before a newline run a string on); a NUL byte ends the source;
and a number is read as Luau's `readNumber` reads it. A name after `.`, `:` or `::` is a field, a
method or a type and is never refused. It refuses:

- `pairs`, `next` (a table key spelled `next = …` included; write `["next"]`), `table.sort`,
  `table.foreach`, a generic `for … in` over anything but `ipairs(…)` (an `=` or `in` counts only
  outside brackets, so a loop variable's type annotation cannot make a `for` look numeric), and
  `ipairs` anywhere but right after `in` (`local each = ipairs`, a parameter or a key named
  `ipairs`);
- the `/` and `^` operators (and `/=`, `^=`), and every `math.` member except `floor`, `ceil`,
  `max`, `min`, `abs`, `clamp` and `sign` (`math.pi` and `math.huge` included), and `math` used
  bare;
- the names the sandbox removed, `xpcall`, `vector`, `buffer` and `integer` among them.

## What is bound

Everything a ported card could say in Rust (#448; `docs/v0.4.0/api-gaps.md` lists what was left
and which cards waited on it):

- **Effects.** One Luau function per effect verb in `crates/engine/src/effects/` (`api/`, one
  file per module, each line a `verb!` call), same name, its argument struct's fields as table
  fields named as serde names them (camelCase). An effect is data: `J.damage(args)` returns
  `{ verb = "damage", args = args }` and touches nothing.

```luau
J.damage({ to = { of = "enemyHero" }, amount = 2 })
```

- **Readers.** The read helpers (`ctx:hero_of(player)`), the effect-context readers
  (`ctx:chosen_options()`), `J.query` and `J.pool` (the only random pools) and the context's
  rng draws — arrays, never maps, wherever order matters, plus `J.sort_by` (a stable sort done
  in Rust).

```luau
local ids = {}
for _, unit in ipairs(ctx:active_units_of(ctx:opponent_of(ctx:controller()))) do
	ids[#ids + 1] = unit.id
end
```

- **Hooks.** Every hook type a ported card uses (`hooks.rs`, one adapter each): each takes what
  its Rust type takes and returns what it returns. A hook that fails panics with its error, which
  names its card, face and hook: an engine `Hook` has no error to return.

```luau
conditionMet = function(ctx: J.ConditionContext)
	return ctx:unspent_mana_of(ctx:controller()) >= 0
end,
```

- **Declarations.** Targets, modes, static flags, activations, triggers, replacements, target
  checks, resume and grants load as data (`script.rs`); `when`/`run`/`canActivate` functions
  inside them are adapted like hooks.

```luau
triggers = {
	{
		id = "t1",
		on = { "turnStarted" },
		when = function(ctx: J.EffectContext, event: J.GameEvent)
			return true
		end,
		run = function(ctx: J.EffectContext, event: J.GameEvent)
			return { J.draw({ count = 1 }) }
		end,
	},
},
```

- **Closure verbs by name.** `after_state_check`, `draw_while` and `for_each_card` take the
  *name* of a face function plus data — never a function (L6: a function cannot cross into Rust,
  and L8: nothing a script sets survives the call, so there is no registry to keep one in). The
  closure the host builds captures only the card, the face, the name, the data and the site, and
  re-runs the card's chunk to call `module[face][hook]` when the engine reaches it. A second use
  of one verb in a face appends `_2` (`each_cards_2`), and a name that resolves to nothing fails
  the card's own test with an error naming the card, face and function. `vm::SUB_HOOKS` lists
  the valid names.

```luau
cry = function(ctx: J.EffectContext)
	return {
		J.after_state_check({ hook = "after_check", data = { amount = 2 } }),
	}
end,
after_check = function(ctx: J.EffectContext, data: J.JsonValue)
	return {
		J.damage({ to = { of = "enemyHero" }, amount = data.amount }),
	}
end,
```

- **Types.** `crates/cards/luau/jackioh.luau`: the `export type`s and typed function stubs,
  generated from the Rust types through ts-rs (the engine's off-by-default `ts` feature) by
  `tests/export_api.rs` and committed; scripts `require("@jackioh")`, so strict mode checks
  every script against it. CI fails when a run changes the module without committing it.

## Tests

`tests/host.rs`, over the card-shaped modules in `tests/fixtures/*.luau` (none is a real card):
the sandbox, the interrupt's cap (nested calls included), the depth cap, the walk, `J.div` and
`J.rem`, statelessness, nesting, two threads, the loader and the lint. `tests/api.rs`: one test
per hook adapter over fixture scripts, and Luau-vs-Rust parity per verb group — the same card as
a Luau fixture and as an engine fixture, played in identical scenarios, with equal `hash_state`
and equal events. `tests/export_api.rs` (with `--features ts`) regenerates the typed module and
fails when the run changes it. `cargo test -p jackioh-luau --features ts`.
