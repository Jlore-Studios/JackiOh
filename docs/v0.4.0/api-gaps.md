# v0.4.0 Luau card-API gaps

Part 6 of 29 (#448): what `cargo jackioh luau surface` lists (all 447 catalog
cards, Meditative included; 45 hook types, 176 verbs, 151 readers, 4 pools, 2
unclassified names) minus what the Luau API binds — and, for each leftover, why it
waits and which cards stay in Rust because of it. The batches read this to know
what they can port and what they must leave.

Regenerate the input with `cargo jackioh luau surface --json` (printed, never
committed). Surface names are Rust names; the Luau spelling mangles `::` to `_`
(`subsystems::activation_paid` is `ctx:subsystems_activation_paid(...)`).

## Verbs: no gaps

All 176 effect verbs are bound, one Luau function each (`crates/luau/src/api/`,
one file per `effects/` module), including the two names surface leaves
unclassified (`replace_hand_with_perfect`, Classic+ #27; `roll_back`, Classic+
#35). One verb is half-bound:

- `with_kill_credit` — `killer`, `transfer` and `during` bind; `pairs` and `then`
  bind since part 28 (#464 gap 9), by name with data, the `after_state_check`
  pattern: `J.with_kill_credit({ killer = ..., pairs = "kill_pairs",
  then = "kill_then", data = { ... }, during = { ... } })`. The named `pairs`
  runs as `face[pairs](ctx, data)` and answers `{ victimId, toId }` records;
  the named `then` runs as `face[then](pair, data)` (`crates/luau/tests/api.rs`,
  `gap9_kill_credit_pairs_and_then_by_name`). The pair records cross the
  boundary as data, so the Luau verb watches exactly what the Rust one does.

## Hooks: no gaps outside L14

42 of 45 hook types have adapters (`crates/luau/src/hooks.rs`) and tests
(`crates/luau/tests/api.rs`, one per family). The three without are L14's: only
cards that stay in Rust use them, and the loader rejects all three — `quests`
and `records_play_as` were never valid face keys, and `start_of_game` was
removed from them in part 6 rather than loading and never firing:

- `quests` (Classic #90 In Too Deep), `records_play_as`, `start_of_game`.

`on_play_hook` is bound though surface lists no card using it.

## Readers: bound, covered otherwise, or gaps

Most of the 151 readers are `ctx` methods (`crates/luau/src/reads.rs`), `J`
functions (`J.query`, `J.pool`, `J.cost`, `J.div`, `J.rem`, `J.sort_by`,
`J.TRAP_TYPES`) or unnecessary in Luau. The leftovers, by kind:

### Declarations, not readers — portable today

These construct `Script` pieces in Rust; in Luau they are face-table
declarations (`crates/luau/src/script.rs` reads them as data, `vm::HOOKS` and
`vm::DECLARATIONS` list the keys). No card waits on them.

- `hook` (346 cards: every `cry`, `death`, …), `condition_hook` (37),
  `cost_hook` (4), `aura_hook` (10), `attack_mod_hook` (meditative-052),
  `would_counter_hook` (classic-087), `read_hook` (19: `has` hooks),
  `target_check` (8: `targetChecks`), `replacement_when` / `replacement_of`
  (2 each: `replacements`, read through `ReplacementContext.event`),
  `book_swap_trigger` (classic-055: a fixed `TriggerDef` constructor — the port
  writes the trigger table literally).

### Unnecessary in Luau — no card waits

- `json_as` (362 cards): tables are JSON-native in Luau; there is nothing to
  convert.
- `fill_params` (core-070, classicplus-052): card-text templating; a script's
  text comes from the catalog, so a Luau card never renders it. Bound anyway as
  `ctx:fill_params(...)` for previews.
- `remember` (11 cards): bound as the `J.remember` verb; surface lists calls as
  readers.

### True gaps — these cards stay in Rust

- `crate::card_def` — the catalog def by id, 14 cards: core-031, core-062,
  core-070, core-080, core-100, classic-082, classicplus-052, classicplus-064,
  classicplus-075, classicplus-076, classicplus-t-ai-06, meditative-074,
  meditative-088, meditative-089. Partly covered (`J.cost(id)` for costs,
  `J.query` for filtered defs, `ctx:def_by_index` for index lookups), but a bare
  "give me this id's def" has no Luau form; each batch judges per card whether
  the covers suffice.
- `left_field_since_resolved` — bound since part 28 (#464 gap 9) as
  `ctx:left_field_since_resolved(event)`, taking the trigger's event
  (`crates/luau/tests/api.rs`, `gap5_6_7_9_12_readers_events_config_chance_and_nil`).
- `roll_weighted` — meditative-054. Deliberately unbound: its table is
  `&[RollOdds]` holding `&'static str`, which cannot cross without leaking, and
  every random draw goes through the state's rng (`J.query`/`J.pool`); a weighted
  table needs a data shape first.
- `jackioh_engine::work::part_memory_key` — classic-007 Chain of Thought. Reads
  the engine-internal fused-part path out of `ctx.data`; no Luau form yet.
- `subsystems::call_to_chaos` — meditative-095.
- `subsystems::cane_return`, `subsystems::cane_strike` — meditative-071.
- `subsystems::night_market_script` — meditative-042.
- The subsystem helpers above are one-method bindings each; they wait for a
  batch that needs them to add the method, the stub line and the probe row,
  following `subsystems_activation_paid`.

### Only L14, test or tool users — no ported card waits

`subsystems::audit_targets`, `subsystems::combo_index::*` (6),
`subsystems::copied_text_of`, `subsystems::defending_hero`,
`subsystems::held_quest_auras`, `subsystems::hold_quest_aura`,
`subsystems::is_lethal`, `subsystems::ky_test_script`,
`subsystems::lines_of_code`, `subsystems::open_quest`,
`subsystems::papaya_begin`, `subsystems::power_abilities`,
`subsystems::quest_def_of`, `subsystems::quest_reward_of`,
`subsystems::roll_power`, `subsystems::top_three`,
`subsystems::twice_forward_plays`, `subsystems::twice_forward_trigger`. Surface
attributes each to zero ported cards; every user stays in Rust under L14
(core-093, core-096–099, classic-090, classicplus-042/044/045/062/074,
classic-057) or is a test script.

## Pools: no gaps

- `CardQuery` (core-067) is `J.query`; `pool` (core-083) is `J.pool`;
  `TRAP_TYPES` is `J.TRAP_TYPES`; `luck_based` (meditative-101 Gachaholic) is
  `J.query({ luckBased = true })` (`CatalogQueryArgs.luck_based`, R1444).

## Fixed in part 28 (#464): each gap's Luau form

The batches held the cards named below back on these gaps; part 28 fixed each
with a fixture test (`crates/luau/tests/api.rs`, `gap*`). Port onto these
forms — `PORTING.md` §5's first reason no longer holds for them.

1. **The empty-table round trip** (`crates/luau/src/numbers.rs`, `api/each.rs`;
   `gap1_empty_tables_round_trip`). A Rust empty `Vec` crossing into Luau and
   back, or a script answering `{}` for a list, arrives as a list: the walk
   honours mlua's array metatable, and a `cards` function answering `{}` names
   no cards instead of panicking. Example: `each_cards = function(ctx: J.EffectContext) if ctx:zone_count(me, "hand") > n then return {} end ... end`.
2. **`for_each_card` with `data`** (`api/each.rs`; `gap2_for_each_card_data`).
   The verb field `data` reaches the named `cards` and `each` functions as
   their second argument, the way `after_state_check` passes it:
   `J.for_each_card({ cards = "each_cards", each = "each_card", data = { amount = 2 } })`.
   Example: `each_card = function(id: string, data: any) return { J.damage({ to = { of = "instance", instanceId = id }, amount = data.amount }) } end`.
3. **One reader table on every context** (`reads.rs`, `shared_readers!`;
   `gap3_shared_readers_on_condition_preview_and_cost`). The condition context,
   the preview context, the replacement and hook-args contexts and `CostArgs`
   carry every reader the effect context has. Example:
   `conditionMet = function(ctx: J.ConditionContext) return ctx:played_earlier(ctx:controller(), ctx:self_()) > 0 end`.
4. **The turn reader** (`ctx:turn()`; same test). Answers the turn number and
   the active player as `{ number, active }`. Example: `local t = ctx:turn(); if t.number >= 7 and t.active == ctx:controller() then ... end`.
5. **`given_lucky_on`** (`ctx:given_lucky_on(card)`;
   `gap5_6_7_9_12_readers_events_config_chance_and_nil`). Granted-only Lucky
   (R1438); takes an id string or an instance table. Example:
   `if ctx:given_lucky_on(ctx:live_self()) > 0 then ... end`. `cargo jackioh
   luau surface` counts a bare function name passed as a value, so
   `map_or(0, given_lucky_on)` is reported.
6. **The action's events reader** (`ctx:events()`; same test). This script's
   own window onto the action's events (R136), `events[events_from..]`;
   `ctx:events_from()` names its start. Example: `for _, e in ipairs(ctx:events()) do ... end`.
7. **`J.config`** (`J.config.CHAIN_OF_THOUGHT_REPEATS`; same test). The engine's
   config constants card scripts use, generated into `jackioh.luau` from the
   same source as `engineConfig.ts`, so they cannot drift (CLAUDE.md rule 9).
   Example: `repeats = J.config.CHAIN_OF_THOUGHT_REPEATS`.
   A script needing a constant not listed there adds it to `CONFIG_I32`
   (`crates/luau/src/config.rs`) — never a literal.
8. **A generic aura** (`J.aura({ mod = ..., applies = ... })`, `api/aura.rs`;
   `gap8_generic_aura_entries`). A stat `mod` and an `applies` filter declared
   as data: either a `CardQuery`-shaped filter table, evaluated in Rust with no
   Luau call per stat read, or a named `aura_filter` predicate called per
   candidate unit. Example: `J.aura({ mod = { attack = 2 }, applies = { side = "self" } })`.
   Hold it to L13 with `luau bench`; a card that breaks the budget stays Rust
   (§5's third reason).
9. **`left_field_since_resolved`** (`ctx:left_field_since_resolved(event)`);
   **a self-snapshot `killer_of`** (`ctx:killer_of(id)` reads the hook's own
   snapshot when the id names the running card — in a Death hook the live
   instance is the reset graveyard card — and `ctx:killer_of_self()` always
   does; `gap9_killer_of_self_reads_the_snapshot`); **`radiant_chance`** in an
   integer unit (`J.radiant_chance({ zone = ..., chancePercent = 30 })`,
   converted with `f64::from(percent) / 100.0`, the Rust literal's own
   arithmetic, so the rng draw is identical); **`with_kill_credit`'s `pairs`
   and `then`** by name with data (Verbs, above;
   `gap9_kill_credit_pairs_and_then_by_name`). Examples:
   `ctx:left_field_since_resolved(event)` (the trigger's event),
   `ctx:killer_of_self()`,
   `J.radiant_chance({ zone = "hand", chancePercent = 30 })`.
10. **One adapter for one function under two hooks** (the loader shares one
    `Arc`; `gap10_one_function_under_two_hooks_shares_its_adapter`). Write the
    one local and name it twice in one face. Example:
    `local function tick(ctx: J.EffectContext) ... end; base = { startOfTurn = tick, endOfTurn = tick }`.
11. **The ungated typed-module check**: `crates/luau/tests/export_api.rs` runs
    under the workspace's ordinary `cargo test` features, not only under
    `jackioh-luau`'s own `ts` feature — `jackioh.luau` is reproduced by the
    workspace test run.
12. **Readers that answer `None` hand Luau `nil`**, not mlua's truthy `null`
    (`reads.rs`, `to_lua`; same test as 5). Test an absent answer with
    `== nil`, never `type(x) == "table"` (which stays false for `nil`, but a
    bare truthiness check on the old `null` was true).
13. **`ctx:data()`**, a `resume` entry's own data (§10.6). Example:
    `local target = ctx:data().targetId`.
14. **One adapter across both faces too** (the cache lives in `load_card`, so
    both faces draw from it;
    `gap14_one_function_across_both_faces_shares_its_adapter`,
    `gap14_shared_hook_resolves_named_hooks_on_the_running_face`). Two faces'
    hooks holding one function value — faces whose params differ — share one
    `Arc`. The shared adapter serves each run from the RUNNING face's table,
    so a named sub-hook resolves where the run runs (one `cry` with per-face
    `each_card` fires each face's own), and `param` reads off `ctx` either
    way. Example:
    `local function death(ctx: J.EffectContext) ... end; return { base = { death = death }, radiant = { death = death } }`.
    Two traps: Luau reuses one closure object for every evaluation of an
    upvalue-free literal, so one factory called twice is one value — faces
    that must keep their own adapters (`!ptr_eq`) spell their hooks out as
    separate literals (or close over a per-face upvalue), as `core-t-coin`
    does; and a shared hook with per-face named sub-hooks still shares the
    adapter — only the named resolution follows the run.
15. **`.luaurc`'s alias** (`crates/cards/src/scripts/.luaurc` maps `jackioh`
    to `"../../luau/jackioh"`, no extension). luau-analyze 0.740 refuses the
    extension as `Unknown require` under `--mode=strict`; without it the CLI
    resolves the alias, so strict runs check the files in place — no copies
    with a relative require.
