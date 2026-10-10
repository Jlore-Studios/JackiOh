# `jackioh-luau`

The host that will run card scripts written in Luau (#442, v0.4.0). It holds the sandbox a script
runs in, the integer boundary every value crosses, the lint that refuses what the sandbox cannot
stop, one VM per thread, and `load_card`, which turns a compiled script into the engine's
`CardScripts`. **Nothing depends on it yet.** Part 4 wires it into `crates/cards` (whose `build.rs`
will call `compile`), ports the pilot cards, and adds `g++` to the server's Docker image. Until then
no build from `main` compiles it except the workspace's own (`cargo build/clippy/test --workspace`).
The decisions it follows are #442's L3–L8. This file is the contract.

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
  `vector`, `buffer` and `integer` first (`ABSENT_LIBRARIES`);
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
  module to read its declarations spends a budget named for a `load` hook on the base face;
- Rust panics that `pcall` cannot catch (`LuaOptions::catch_rust_panics(false)`): a nested hook
  that fails panics (below), and the panic goes on through the hook that called it, since mlua's
  `pcall` raises it again and runs no handler that could replace it.
  This holds while Luau's fast `pcall` is off (its `LuauFastpcall` and `LuauCompileFastpcall`
  flags, off in 0.740), since that path calls Luau's own `pcall` rather than the global mlua puts
  in its place; `pcall_does_not_catch_a_nested_hooks_failure` fails on a Luau that turns it on.

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
whenever the collector runs, which hangs on everything the thread's VM did before. Nor does any
memory cap stop a script that builds one huge string or table without looping (`string.rep`): L5's
cap counts interrupts, not bytes.

## Statelessness (L8)

There is one VM per thread, built sandboxed the first time the thread calls a hook. It caches each
card's loaded chunk by card id, so one card id has one bytecode for the life of the thread. Every
hook call runs the card's chunk again, so the module, and every module-level local in it, starts
fresh: a counter a hook increments reads the same on every call. Re-running the chunk is L8's
price: in a release build, a hook called through `load_card`'s closure (the chunk run, the hook
called, its return walked and its effects built) took about 10 µs for a hook with one effect and no
reader, and 15 µs for one with two readers and two effects (measured for #558 on a 4-core runner).
Building a sandbox, once per thread, took under half a millisecond.

## How a hook is called

A card's module returns a table with a `base` table, a `radiant` table or both (the same table for
a card whose faces act alike). Each maps hook names to functions; the names are `vm::HOOKS`, the
effect-list hooks `Script::hook_named` answers: `cry`, `death`, `startOfGame`, `delayed`,
`startOfTurn`, `endOfTurn`, `onPlayHook`, `afterAttack`, `startOfOpponentTurn`. Any other key, at
either level, is refused when the card loads (`LuauError::Load`).

```luau
local J = require("@jackioh")

return {
	base = {
		cry = function(ctx)
			local hero = ctx:hero_of(ctx:controller())
			return { J.damage({ to = { of = "enemyHero" }, amount = J.div(hero.health, 7) }) }
		end,
	},
}
```

- `load_card(id, bytecode)` runs the module once to read which hooks each face declares, and
  fills those slots of `CardScripts` with closures that capture only the hook's `Site` (card, face
  and hook) and the `&'static` bytecode, so any thread's VM can load the chunk the first time it
  needs it. No Lua value outlives a call.
- A call runs the module afresh, takes `module[face][hook]` and calls it with `ctx`, a userdata
  scoped to the call (`Lua::scope`), so a script cannot keep it.
- An effect is data. `J.<verb>(args)` returns `{ verb = "<verb>", args = args }` and touches
  nothing. The hook returns a list of them (or `nil`, or `{}`), the host walks it and builds each
  effect with the engine's own verb from `jackioh_engine::effects` (CLAUDE.md rule 5).
- A reader is a method of `ctx`: `ctx:controller()`, `ctx:hero_of(player)`. Its arguments are
  walked like anything else that crosses.
- A hook may call a reader that runs another card's hook: the thread's VM and `Lua::scope` both
  take a nested call, which spends the outer call's budget (`ctx:effects_of(card, face, hook)` tests
  it).
- A hook that fails panics with its error, which names its card, face and hook: an engine `Hook`
  has no error to return. A panic in a nested hook unwinds through the outer one, which `pcall`
  cannot stop, so the outer call panics too.

## Compiling

`compile(file, source)` lints the source and compiles it with Luau's compiler to the bytecode
`load_card` takes, with `compiler()`. `crates/cards/build.rs` will call it in part 4; until then
only the tests do.
Its errors name the file: a lint finding as `"<file>:<line>: <rule>"`, one per line, a syntax error
as `"<file>:<line>: <Luau's message>"`.

The lint (`lint.rs`, L6, L7) is a pure function over the source: a lexer that knows Luau's
comments, strings (interpolated ones included) and operators, then a pass over the tokens. The
lexer reads the source as Luau 0.740's (`Ast/src/Lexer.cpp`) does wherever that decides what is
code: a line comment ends at a newline, a carriage return or a NUL byte; a string breaks at an
unescaped one of them (which Luau refuses, so the code after it is linted); escapes are read as
Luau reads them (`\z` and a `\` before a newline run a string on); a NUL byte ends the source;
and a number is read as Luau's `readNumber` reads it. A name
after `.`, `:` or `::` is a field, a method or a type and is never refused. It refuses:

- `pairs`, `next` (a table key spelled `next = …` included; write `["next"]`), `table.sort`,
  `table.foreach`, a generic `for … in` over anything but `ipairs(…)` (an `=` or `in` counts only
  outside brackets, so a loop variable's type annotation cannot make a `for` look numeric), and
  `ipairs` anywhere but
  right after `in` (`local each = ipairs`, a parameter or a key named `ipairs`);
- the `/` and `^` operators (and `/=`, `^=`), and every `math.` member except `floor`, `ceil`,
  `max`, `min`, `abs`, `clamp` and `sign` (`math.pi` and `math.huge` included), and `math` used
  bare;
- the names the sandbox removed, `xpcall`, `vector`, `buffer` and `integer` among them.

## What is bound

Only enough to test the host: the verbs `J.destroy` and `J.damage` (`api::VERBS`, named and typed
as the engine's `destroy` and `damage`), `J.div` and `J.rem`, and the readers `ctx:controller()`,
`ctx:hero_of(player)` and `ctx:effects_of(card, face, hook)` (the kinds of the effects another
card's hook returns, standing in for the readers that nest until they exist). Part 6 binds the
rest.

## Tests

`tests/host.rs`, over the card-shaped modules in `tests/fixtures/*.luau` (none is a real card):
the sandbox, the interrupt's cap (nested calls included), the walk, `J.div` and `J.rem`,
statelessness, nesting, two threads, the loader and the lint. `cargo test -p jackioh-luau`.
