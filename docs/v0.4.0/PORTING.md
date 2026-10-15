# Porting a Rust card script to Luau (v0.4.0)

> **Superseded for official cards by v0.5.0 ([#902](https://github.com/Jlore-Studios/JackiOh/issues/902)):** every official card is a Rust script again ([README.md](README.md) §8). This guide is kept as the record of the port, and for the Luau host custom cards use.

How a batch (parts 9–27) replaces its cards' Rust scripts with Luau, and how
it proves the result equal and fast enough. The host, the sandbox and every
binding this guide names are part 6's (`crates/luau`, `crates/cards/luau/jackioh.luau`);
this guide is checked against them, and when this guide and the code disagree
the code wins and this guide gets fixed. The proofs are part 7's
(`cargo jackioh luau diff`, `cargo jackioh luau bench`, `golden record`,
`golden check --file`).

## 1. The batch procedure, in order

1. **Build the parent from the batch's base commit.** The base commit is the
   batch's parent on `bot/issue-442`: the commit where the batch's cards are
   still Rust. From the batch worktree:
   `git worktree add ../base HEAD && cargo build --release -p jackioh-tools --manifest-path ../base/Cargo.toml`.
   The parent binary is `../base/target/release/jackioh`: the oracle. It has
   `golden check --file` (every batch starts after part 7, so its base
   commit has it).
2. **For each card, write its `.luau` and remove its `script()` in the same
   change.** The Luau source lives beside the Rust file:
   `crates/cards/src/scripts/<set>/<module>.luau`. `crates/cards/build.rs`
   allows exactly one script source per card: a `.luau` file or a Rust
   `pub fn script()`, never both, never neither. The `.rs` file keeps
   `pub const ID` and its `#[cfg(test)] mod tests` unchanged. Section 2 is
   the translation; section 3 works the pilots line by line.
3. **Run the cards' own tests.** They live in the same `.rs` file and run
   unchanged against the Luau script: base and radiant behaviour per the
   BUILD M4-T4 table (M9's for Classic and Classic+).
4. **`cargo jackioh luau diff --parent-bin ../base/target/release/jackioh --cards <ids>`.**
   This plays 200 games with §10.7's random policy, every batch card forced
   into both decks (a token is forced through the deckable card whose `refs`
   name it, R279), records them on this build in the golden-trace format
   (state, both views, events and the legal set at every step), then has the
   parent replay them with `golden check --file`. Any mismatch prints seed,
   step, which hash (`s`, `v-p1`, `v-p2`, `e`, `l`, `refused`, `end`) and
   the action, and the exit is 1. While iterating, `--games N --seed BASE`
   plays fewer (seeds `BASE..BASE+N-1`, within 1–240); Done needs the full
   200. `--traces <path>` replays a recorded file instead of recording one.
   The views are compared, not only the state hash: R280's previews and
   R195's yellow glow live in the views.
5. **`cargo jackioh golden check`** (the whole file) and
   **`cargo jackioh fuzz --seeds 200`**: no divergence, no failure.
6. **`cargo jackioh luau bench --parent-bin ../base/target/release/jackioh`.**
   The performance gate (L13): fuzz `--seeds 200` wall time, the `gate` perf
   gate's slowest decision (scaled, as `gate` reports it) and a golden step
   natively, each measured on the parent and on this build in one run
   (about 3½ minutes). Exit 1 when the slowest decision is over 1.5× the
   parent's (and never at or over `AI_GATE.max_decision_ms`), or when either
   other number is over 2× the parent's (`BENCH_BUDGETS` in
   `crates/tools/src/luau.rs`). `docs/v0.4.0/perf.json` is the branch-point
   witness for these numbers, not the comparator: the gate always compares
   two binaries on the same machine in the same run.
7. **`luau-analyze --mode=strict`** (Luau 0.740, the version `mlua` builds)
   clean over every new `.luau` file, checked in place: the checked-in
   `.luaurc` (`crates/cards/src/scripts/.luaurc`) aliases `@jackioh` at the
   committed module, which the CLI resolves (the mapping drops the `.luau`
   extension, which luau-analyze 0.740 refuses as `Unknown require`,
   #464 gap 15).
8. **Record the cards that stay Rust** in `docs/v0.4.0/README.md`'s table of
   the cards kept in Rust (section 4 there): one row each, with its id, its
   name and its reason (section 5). The `luau diff` / `fuzz` / `bench` /
   strict results go in the pull request.
9. **Push.** The harness lands the batch on `bot/issue-442`.

## 2. The translation rules

Checked against part 6's landed API (`crates/luau/src/`, `jackioh.luau`,
`crates/luau/tests/api.rs` with its fixtures). Fix this guide, not the API.

**Shape.** A module returns a table with a `base` table, a `radiant` table,
or both. A card whose faces act alike spells it `radiant = "base"`; a card
with nothing to script (printed keywords only) spells `base = {}`. Any
other face key, at either level, is a load error, so a misspelled hook
stays one (`vm::HOOKS`, `vm::SUB_HOOKS`, `vm::DECLARATIONS`). Start every
file with `--!strict` and `local J = require("@jackioh")`.

**`json!({…})` is a table literal with the same keys.** Verb arguments keep
their serde names (camelCase): `json!({ "to": { "of": "enemyHero" },
"amount": 3 })` is `{ to = { of = "enemyHero" }, amount = 3 }`.
**`..Script::default()` is leaving the field out.** An empty face is `{}`.

**Hooks take what their Rust type takes and return what it returns (L9).**
Effect hooks (`cry`, `death`, `delayed`, `startOfTurn`, `endOfTurn`,
`afterAttack`, …) take `ctx: J.EffectContext` and return a list of effect
calls (or `nil`, or `{}`). Condition hooks (`conditionMet`) take
`ctx: J.ConditionContext` and return a boolean. Value hooks take `HookArgs`
(`aura`, `setStat`, `cost`, `echoX`, `tributeWhen`, …) and return their
value's shape. Sub-hooks for closure-taking verbs are named functions on
the same face: `after_state_check` calls back `after_check` plus data,
`draw_while` calls back `draw_more`, `for_each_card` calls back
`each_cards` / `each_card`; a second use of one verb in the same face
appends `_2`, `_3`, … (`each_cards_2`, Classic #39). One local named under
two hooks — two hooks of one face, or the same hook of both faces — builds one
adapter shared by an `Arc` (#464 gaps 10 and 14); the run resolves names on
the running face. Under different hooks of the two faces it builds one
adapter a face (#871). Two textually separate literals stay two adapters even
with identical bodies: Luau reuses one closure object for every evaluation
of an upvalue-free literal, so a factory called twice is one value
(`core-t-coin` spells its faces out). Triggers and
replacements read through their event (`ReplacementContext.event`).
`crates/luau/src/hooks.rs` is the full table of adapters.

**Integers only (L6).** Every number that crosses into Rust must be
integral, finite and inside `i32`, or the hook fails naming card, face,
hook and value. Write Rust's `/` and `%` as `J.div` and `J.rem`: they
truncate toward zero (`J.div(-7, 2)` is `-3`). Luau's own `//` and `%`
floor instead (`-7 // 2` is `-4`) and silently mean something else. The
lint refuses the `/` and `^` operators (and `/=`, `^=`) and every `math`
member but `floor`, `ceil`, `max`, `min`, `abs`, `clamp` and `sign`
(`math.pi` and `math.huge` included); the sandbox removes the rest,
including `math.random`. Annotate hook contexts
(`ctx: J.EffectContext`, `ctx: J.ConditionContext`, …): an unannotated
parameter degrades what the readers answer. A verb whose argument is a
union (several table shapes, `HealArgs`) cannot narrow an inline literal
either: hoist the inner table into an annotated local
(`local target: J.TargetSpec = { of = "selfHero" }`).

**`ipairs` only, `J.sort_by` (L7).** The lint refuses `pairs`, `next`,
`table.sort`, `table.foreach` and a generic `for … in` over anything but
`ipairs(…)`; the sandbox removes the library functions among them. Records
cross with their keys sorted, so iteration order never leaks into an
answer. To order a list, build it with `ipairs` and sort it with
`J.sort_by` (a stable sort done in Rust).

**No module-level state (L8).** Every hook call runs the card's chunk
afresh, so the module and every module-level local start new on every
call: a counter a hook increments reads the same on every call. Keep no
game state in locals; read it from `ctx`.

**`param` and `remember` as in Rust.** `ctx:param(key)` reads the card's
declared number (`armor`, …), so a Nerf or Buff moves the Luau card exactly
as it moves the Rust one. `J.remember` is the `remember` verb (surface
lists its calls as readers); it is an effect like any other, in order.

**Keep effect order exactly.** A hook returns one list; the host builds it
in order. Two effects swapped is a divergence `luau diff` will find.

## 3. Worked examples

Each pair below is the landed code: the Rust the card had before its port
(the `.rs` at the batch's base commit) beside the `.luau` that replaced it.
Every rule of section 2 shows up in at least one of them.

### Pilot 1: Core #1 Big D-fender (`core-001`)

One aura hook reading a declared number; the Radiant face reuses base. The
Rust:

```rust
fn defense_armor_aura() -> AuraHook {
    aura_hook(|args: HookArgs<'_>| {
        let n = param(&args, "armor");
        let controller = args.self_.controller;
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| {
                unit.controller == controller
                    && matches!(unit.zone, Zone::Field { row: Row::Units, .. })
                    && unit.position.unwrap_or(Position::Atk) == Position::Def
            }),
            mod_: StatMod {
                keywords: Some(vec![Keyword::Armor { n }]),
                ..StatMod::default()
            },
        }]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        aura: Some(defense_armor_aura()),
        ..Script::default()
    };
    // The same aura: the Radiant face's 4 is its declared `armor`, read off the face that is up.
    let radiant = base.clone();
    CardScripts { base, radiant }
}
```

The Luau (`crates/cards/src/scripts/core/c001_big_d_fender.luau`), in full:

```luau
--!strict
local J = require("@jackioh")

local base = {
	aura = function(ctx)
		return {
			J.defense_armor_aura({ armor = ctx:param("armor") }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
```

What changed: `base.clone()` for the Radiant face is `radiant = "base"`;
the aura's applicant predicate (controller, units row, Defense position)
is inside the engine's `defense_armor_aura` verb, so the script passes only
the declared `armor` via `ctx:param("armor")`; `..Script::default()` is
nothing. The Radiant face's 4 is the same declared number on the other
face, which is why one table serves both.

### Pilot 2: Core #2 Bigot (`core-002`)

A targeted Cry on base, a board-wide Cry on radiant, and a `targets`
declaration. The Rust:

```rust
pub fn script() -> CardScripts {
    let base = Script {
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "enemy", "of": ["unit"], "notTags": ["Human"] }),
        )],
        cry: Some(hook(|_ctx| vec![destroy(json_as(json!({ "target": { "of": "chosen" }})))])),
        ..Script::default()
    };
    let radiant = Script {
        cry: Some(hook(|_ctx| {
            vec![destroy_all(json_as(json!({
                "side": "enemy",
                "rows": ["units"],
                "notTags": ["Human"]
            })))]
        })),
        ..Script::default()
    };
    CardScripts { base, radiant }
}
```

The Luau (`crates/cards/src/scripts/core/c002_bigot.luau`), in full:

```luau
--!strict
local J = require("@jackioh")

local base = {
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = {
				side = "enemy",
				of = { "unit" },
				notTags = { "Human" },
			},
		},
	},
	cry = function(ctx)
		return {
			J.destroy({ target = { of = "chosen" } }),
		}
	end,
}

local radiant = {
	cry = function(ctx)
		return {
			J.destroy_all({
				side = "enemy",
				rows = { "units" },
				notTags = { "Human" },
			}),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
```

What changed: `TargetDecl::target(1, 1, json!(…))` is a `targets` table
with the same `side` / `of` / `notTags` keys; each `json!` is a table
literal; the radiant face declares no `targets` at all, exactly as the
Rust declares none. The pick travels in the `play` action either way (R81):
a board with no legal enemy non-Human still allows the play and the Cry
fizzles.

### Pilot 3: Core #3 Right-house defender (`core-003`)

Printed keywords need no script; the Radiant face adds one Death hook. The
Rust:

```rust
pub fn script() -> CardScripts {
    /* Printed keywords only (§10.4); an empty Script is the whole base card. */
    let base = Script::default();
    let radiant = Script {
        death: Some(hook(|_ctx| vec![summon(json_as(json!({ "defId": "core-003", "player": "self" })))])),
        ..Script::default()
    };
    CardScripts { base, radiant }
}
```

The Luau (`crates/cards/src/scripts/core/c003_right_house_defender.luau`),
in full:

```luau
--!strict
local J = require("@jackioh")

local radiant = {
	death = function(ctx)
		return {
			J.summon({
				defId = "core-003",
				player = "self",
			}),
		}
	end,
}

return {
	base = {},
	radiant = radiant,
}
```

What changed: `Script::default()` with nothing set is `base = {}`. Never
re-grant a printed keyword from a script: Divine Shield, Reborn and Taunt
stay in the catalog, and the summon is a new base face (no `radiant`
flag), so the chain ends. Death fires on both deaths of the Reborn body
(R8); the reserved zone (R64) is the engine's business, not the script's.

### Fixture: one effect (`parity_damage`)

The smallest shape a port can take. The Rust
(`crates/luau/tests/api.rs`, `parity_damage`):

```rust
let rust = Script {
    cry: Some(hook(|_| {
        vec![damage(json_as(
            json!({ "to": { "of": "enemyHero" }, "amount": 3 }),
        ))]
    })),
    ..Default::default()
};
```

The Luau (`crates/luau/tests/fixtures/parity_damage.luau`):

```luau
--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx)
		return {
			J.damage({ to = { of = "enemyHero" }, amount = 3 }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
```

`damage(json_as(json!(…)))` is `J.damage({…})`, nothing more.

### Fixture: effect order (`parity_heal`)

Two effects, and the order is the behaviour. The Rust (`parity_heal`):

```rust
let rust = Script {
    cry: Some(hook(|_| {
        vec![
            damage(json_as(json!({ "to": { "of": "selfHero" }, "amount": 5 }))),
            heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": 3 }))),
        ]
    })),
    ..Default::default()
};
```

The Luau (`crates/luau/tests/fixtures/parity_heal.luau`):

```luau
--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx: J.EffectContext)
		local target: J.TargetSpec = { of = "selfHero" }
		return {
			J.damage({ to = target, amount = 5 }),
			J.heal({ target = target, amount = 3 }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
```

Note the annotated context and the hoisted `target: J.TargetSpec`: `heal`
takes a union-typed argument, so an inline literal would not narrow under
strict. Damage first, heal second, exactly as the Rust lists them.

### Fixture: closures by name (`parity_for_each`)

The one pattern with no Rust-shaped twin: verbs that take a closure take a
hook name plus data instead (`after_state_check` → `after_check`,
`draw_while` → `draw_more`, `for_each_card` → `each_cards` / `each_card`),
and the host re-runs the chunk to call `module[face][hook]` when the engine
reaches it. Two uses of one verb in one face append `_2`. The Luau
(`crates/luau/tests/fixtures/parity_for_each.luau`):

```luau
--!strict
local J = require("@jackioh")

-- One damage to every enemy unit, twice, dealt through two `for_each_card`s in one
-- hook with different named functions: a second use appends `_2`. The lists come
-- from a reader, the blows from a function of the id.
local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.for_each_card({
				cards = "each_cards",
				each = "each_card",
			}),
			J.for_each_card({
				cards = "each_cards_2",
				each = "each_card_2",
			}),
		}
	end,
	each_cards = function(ctx: J.EffectContext)
		local ids = {}
		for _, unit in ipairs(ctx:active_units_of(ctx:opponent_of(ctx:controller()))) do
			ids[#ids + 1] = unit.id
		end
		return ids
	end,
	each_card = function(id: string)
		return { J.damage({ to = { of = "instance", instanceId = id }, amount = 1 }) }
	end,
	each_cards_2 = function(ctx: J.EffectContext)
		local ids = {}
		for _, unit in ipairs(ctx:active_units_of(ctx:opponent_of(ctx:controller()))) do
			ids[#ids + 1] = unit.id
		end
		return ids
	end,
	each_card_2 = function(id: string)
		return { J.damage({ to = { of = "instance", instanceId = id }, amount = 1 }) }
	end,
}

return {
	base = base,
	radiant = "base",
}
```

The reader loop is the `ipairs` pattern: collect ids with `ipairs`, never
`pairs`. A name the face does not declare fails the card's own test naming
card, face and function.

## 4. The checks, and what a failure means

- `luau diff` failing names seed, step, which hash and the action. `s` is
  the state hash; `v-p1` / `v-p2` are the two views (previews, glow);
  `e` is the step's events; `l` is the legal set the action was chosen
  from; `refused` is the engine refusing the recorded action; `end` is the
  ending. Start from the named step's action and walk the hook it ran.
- `golden check --file` on a recorded file replays it without a parent:
  use it to tell a bad recording from a bad opponent.
- `fuzz --seeds 200` failing names the seed: replay it with
  `fuzz --from <seed> --seeds 1`.
- `luau bench` failing names which number is over which ratio. A slowest
  decision at or over `AI_GATE.max_decision_ms` fails whatever the ratio
  says: the browser's budget is absolute.

## 5. When a card stays Rust

A card stays Rust for one of three reasons, and only those:

1. **A missing API function**, listed in `docs/v0.4.0/api-gaps.md` with the
   cards that wait on it. If the batch's card needs a name the API does not
   bind, it stays Rust; the batch does not extend the API.
2. **A divergence the batch cannot fix.** The Luau is faithful and the
   parent still disagrees, for a reason outside the card (engine behaviour
   the port cannot express). Say what was tried.
3. **A card that breaks `luau bench`.** Its hooks are correct and too slow.

In all three cases: the card gets its `.rs` back from the base commit
(`git show <base>:crates/cards/src/scripts/<set>/<module>.rs`), loses its
`.luau`, and `docs/v0.4.0/README.md`'s table of the cards kept in Rust says
which cards stayed and why, one row each. A card that stays Rust for any other reason is a batch that is not
done.

**Since part 28 (#464), the first reason no longer holds for the fifteen gaps
`docs/v0.4.0/api-gaps.md` lists as fixed**, each with its Luau form and a
fixture test: the empty-table round trip, `for_each_card`'s `data`, the shared
reader table on every context, `ctx:turn()`, `given_lucky_on`, the action's
events reader, `J.config`, the generic aura, `left_field_since_resolved`, the
self-snapshot `killer_of`, integer-unit `radiant_chance`,
`with_kill_credit`'s `pairs`/`then`, the shared adapter within one face and
across both faces, the ungated typed-module check, `nil` for absent answers,
`ctx:data()`, and the extensionless `.luaurc` alias. A card held back on one
of those ports onto the fixed API; only a gap still listed as waiting, a
divergence, or a broken bench keeps it in Rust.
