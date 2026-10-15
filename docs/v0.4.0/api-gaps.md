# v0.4.0 Luau card-API gaps

> **Superseded for official cards by v0.5.0 ([#902](https://github.com/Jlore-Studios/JackiOh/issues/902)):** every official card is a Rust script again ([README.md](README.md) §8). This list is kept as the record of the port, and for the Luau host custom cards use.

Part 6 of 29 (#448): what `cargo jackioh luau surface` lists (all 447 catalog
cards, Meditative included; 45 hook types, 176 verbs, 151 readers, 4 pools, 2
unclassified names) minus what the Luau API binds — and, for each leftover, why it
waits and which cards stay in Rust because of it. The batches read this to know
what they can port and what they must leave.

Regenerate the input with `cargo jackioh luau surface --json` (printed, never
committed). Surface names are Rust names; the Luau spelling mangles `::` to `_`
(`subsystems::activation_paid` is `ctx:subsystems_activation_paid(...)`).

Part 28 (#464) fixed fifteen gaps (the last section). #442's reconcile brought
the file up to date with batches 11–19 (parts 19–27), which ran on the fixed
API: every card they ported is off the lists below, and the gaps they found are
"Open gaps the batches found after part 28". Each status was checked against
`crates/luau/src/` by grep. The counts (hooks, verbs, readers, cards per name)
are still part 6's: `luau surface` was not re-run for this update. Which
cards stay in Rust, and why, is `README.md` §4; this file is the API's side.

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

`on_play_hook` is bound though surface lists no card using it. Since part 6 the
merge of `main` (v0.3.7's patches, `README.md` §7) replaced the `faceDownPlay`
hook, gone from the engine, with `revealEcho` (`vm::HOOKS`;
`hooks.rs`, `adapt_reveal_echo_hook`).

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
  text comes from the catalog. Bound as `ctx:fill_params(id, face, values)`
  (`reads.rs`), and both cards now run from Luau on it: Spiteful Stab labels
  its `preview` with it, Jlockheed's Permanent Defense Contract its
  `for_rest_of_game` entry (batches 18 and 12).
- `remember` (11 cards): bound as the `J.remember` verb; surface lists calls as
  readers.

### True gaps — these cards stay in Rust

- `crate::card_def` at load time — **open**, narrowed to three cards:
  classic-082 Sheeople, meditative-088 The True Sheep, meditative-089 Jlarna.
  Part 6 listed 14; ten have since been ported on the covers: core-031 (its
  formula read per preview call with `ctx:def_of(ID)`), core-070 and
  classicplus-052 (`ctx:fill_params`), core-100, classicplus-064 and
  meditative-074 (`J.cost(ID)`), classicplus-075 (names its pack's id directly,
  batch 5), and core-062, core-080 and classicplus-076. classicplus-t-ai-06
  reads the def only for its labels, which Luau can read (batch 15); it stays
  Rust on `field_spells_doomed` (next section). Inside a hook the def is
  `ctx:def_of(id)` (`reads.rs`, on every context). What has no Luau form is a
  def read as the script loads: the three cards declare `staticFlags` from the
  catalog's params (`tributeWorth` from `worth`: 2 and 3 for Sheeople, 5 and
  500 for The True Sheep; Jlarna's `creditLine` and `creditInstalments`). A
  face's `staticFlags` is data read at load, with no context, and `J.query` or
  `J.cost` at module level reads the registered catalog, empty until
  `register_all()`, which `scripts_of()` does not need (batch 19:
  `crates/cards/tests/cross/registry.rs`, `has_no_duplicate_def_id`). A
  literal would restate the catalog and drift under a tune (rule 9). Wanted
  (batch 19): a catalog-def read that does not depend on the registered
  catalog, or `staticFlags` read from a context.
- `left_field_since_resolved` — **fixed** in part 28 (#464 gap 9):
  `ctx:left_field_since_resolved(event)`, taking the trigger's event
  (`reads.rs`; `crates/luau/tests/api.rs`,
  `gap5_6_7_9_12_readers_events_config_chance_and_nil`). core-041 Sheepish
  ported on it (batch 12).
- `roll_weighted` — **open**, meditative-054 Money Machine. Deliberately
  unbound (`reads.rs` says so where it would be): its table is `&[RollOdds]`
  holding `&'static str`, which cannot cross without leaking, and every random
  draw goes through the state's rng (`J.query`/`J.pool`); a weighted table needs
  a data shape first. Batch 19 adds the other half: `MONEY_MACHINE_ODDS`
  (`crates/engine/src/config.rs`) is not in `J.config`, which holds `i32`s only,
  so a script cannot even spell the roll without restating the odds. Wanted: a
  weighted-roll data shape, or the odds in `J.config`.
- `jackioh_engine::work::part_memory_key` — **no card waits**; still unbound.
  Its one card user was classic-007 InfiniScepter (part 6 wrote "Chain of
  Thought", which is classicplus-t-ai-04 and never called it). Batch 13 ported
  InfiniScepter reading its held Spell with `ctx:recalled(key)`, the running
  ingredient's own key on a fused card (R102): the engine's `recalled`
  (`crates/engine/src/query.rs`) does the `part_memory_key` lookup itself.
- `subsystems::call_to_chaos` — **open**, meditative-095 (batch 15). The Core
  and Classic+ editions (core-095, classicplus-073) are glue to the same
  function and stay Rust under L14; the Meditative edition is the same glue
  with `CHAOS_MED_EFFECTS` and waits only on the binding.
- `subsystems::cane_return`, `subsystems::cane_strike` — **open**,
  meditative-071 Pareto Optimality (batch 17): its effect is both, on the
  remembered Cane. Both are effect builders (`subsystems/pareto.rs`); only
  `subsystems_pareto_judgement_of` of that module is bound.
- `subsystems::night_market_script` — **open**, meditative-042 CN Flea Market
  (batch 18): the whole card is it (the stall roll, the yuan, the `market`
  prompt's `resume` steps).
- The subsystem helpers above are one-method bindings each; they wait for a
  change to `crates/luau` (the batches could not make one) that adds the method,
  the stub line and the probe row, following `subsystems_activation_paid`.
  None is bound today: `grep -rn "call_to_chaos\|cane_\|night_market"
  crates/luau/src` finds nothing.

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
  All four users run from Luau.

## Open gaps the batches found after part 28

The batches could not edit this file or `crates/luau`, so they wrote these in
their notes (folded into `README.md` §4–§6; most from batches 11–19, one also
from batch 7); each is checked here against the code and is still **open**.
Every card named stays in Rust on it (PORTING.md §5's first reason). With the
true gaps above, they account for all 23 of the 28 non-L14 Rust cards that are
kept for a missing API; the other five are under "Not API gaps" below.

- **The action's events before the script's window** — classic-014
  Shadowstep (batch 13), classicplus-012-8 Frostspatula (batches 7 and 11).
  `ctx:events()` answers `ctx.sink.events[ctx.events_from..]` only (`reads.rs`,
  gap 6), and no other reader reaches the action's events. Shadowstep's
  trigger reads the whole §4.5 pass it answers (the unbroken run of
  `destroyed` events around its own), and Frostspatula's Death reads the kills
  made in the pass of its own death; both lie before the window, which a
  trigger opens when its run begins (batch 13). Wanted: a reader for the
  action's events before the window, or for the pass a trigger answers.
- **A named `where` for `choose_target_where`** — classic-032 Felinor Feelings
  (batch 13). `ChooseTargetWhereArgs.where_` is `#[serde(skip)]`
  (`crates/engine/src/effects/choose_where.rs`) and `api/choose_where.rs` binds
  the engine's args as they are, so a Luau `where` is dropped and the Radiant
  face would offer every enemy permanent. The lanes are read after the Cry's
  own summon, so a precomputed set will not do. Wanted: a named predicate, as
  `aura_filter` is for the aura.
- **A `cast_new` definition read as the cast applies** — classicplus-037
  Wardrum (batch 12). `CastNewDef::Read` is `#[serde(skip)]`
  (`crates/engine/src/effects/cast.rs`); `J.cast_new` takes only a fixed `def`
  (an id or `{ defId, radiant }`), so a Luau Wardrum would fix every copy's
  definition when the hook runs, where R86 reads each as it is cast. Wanted: a
  by-name `def` (`def = "cast_def"`, called with the context as the cast
  applies). classic-007 InfiniScepter ported around it, since its hook's list
  is that one effect (batch 13).
- **A generic aura that names one instance, a tag or carried Units** —
  classic-069 Plague Charger (its own attack only; batch 12), classic-042
  Transmutable Toxins (one entry per Plague-carrying unit, for that unit alone;
  batch 13), classicplus-012-5 Anti-Waffle Shell (the ids `active_units_of`
  answers, carried Units in the backrow included; batch 19), classicplus-046
  Felinor Flagbearer and classicplus-046-1 Felinor Flagbearer Prime ("your
  other Felinors": not the source, granted tags counted through `tags_of`,
  R923; batch 19). The data filter (`AuraFilter`, `api/aura.rs`) has `side`,
  `rows`, `defId` and `exceptDefId` only, and a named `aura_filter` is called
  with the candidate unit alone (`api/mod.rs`, `generic_aura_entry`): no
  source, no `data`, no state. A `defId` filter would lend one Charger's bonus
  to every other; `rows` either misses a carried Unit or takes every backrow
  card. Wanted: an instance and a tag field on the filter, or the source and
  `data` handed to the predicate as `after_state_check` hands its hook `data`
  (batches 12, 13, 19). Mind the bench ("Not API gaps", below) either way.
- **A backrow `is_locked`** — meditative-027 Clip-Farming Lawyer (batch 16),
  meditative-098 Showdown (batch 18). `ctx:is_locked(player, lane)` fixes the
  row to Units (`reads.rs`), and no reader answers a backrow zone's Lock
  (`open_zones` cannot tell one either: a Locked zone takes moves, R688). The
  Lawyer asks about either row; Showdown locks only the zones, both rows, not
  already Locked (R945). `ctx:card_at` has the same Units-only shape, but the
  ports read `permanents_on_field` instead and no card waits on it.
- **Snapshot reads of `keywords_of` and `base_stats_of`** — meditative-065
  Keymaster Keenus, meditative-094 Shrinking Felinor (batch 16). Both readers
  resolve the id to the live instance (`reads.rs`, `find_instance`), which in a
  Death hook is the reset graveyard card (R78); Keenus grants the keywords the
  card had as it died, and Shrinking Felinor reads the `statsOverride` the
  graveyard copy has lost (R1181). Trial ports failed 1 of 9 and 3 of 8 tests.
  Wanted: the snapshot read gap 9 gave `killer_of` (an id naming the running
  card reads the hook's own snapshot).
- **A memory record's keys** — meditative-060 Eschews (batch 16). Its
  `tributeWhen` sums every `deaths@…` key of its memory (a fused card's parts
  remember apart, R102). The lint refuses `pairs`, `next` and any generic `for`
  not over `ipairs` (L7, `lint.rs`), and the only memory reader is
  `ctx:recalled(key)`. Wanted: a reader for a memory's keys, or a sum over a key
  prefix.
- **Non-`i32` engine constants in `J.config`** — meditative-087 Tatches the
  Totem (batch 16): its deck trigger tests tags against `TRIBAL_TAGS`
  (`&[Tag]`, `crates/engine/src/config.rs`, R1424). `J.config` is `CONFIG_I32`
  (`crates/luau/src/config.rs`), `i32`s only, and five literal tags would
  restate the constant (rule 9). Money Machine's `MONEY_MACHINE_ODDS` is the
  same gap (`roll_weighted`, above).
- **`field_spells_doomed` on the condition context** — classicplus-t-ai-06
  Datacenter Fire (batches 9 and 15). Its `preview` runs on a
  `ConditionContext` (`hooks.rs`, `adapt_preview_hook`); the reader is
  registered in `register_ctx` only, outside `shared_readers!` (`reads.rs`), so
  gap 3 did not bring it there. Wanted: the reader on the condition context
  too. An AI token: `luau diff` cannot force it, so its own tests would prove
  the port.
- **`effective_cost` over an instance table** — classic-036 Burn (batch 15),
  one of its two divergences ("Not API gaps", below). An Echo copying Burn runs
  `conditionMet` with `self_` as the copied text face (cost 0); the Rust reads
  `effective_cost` off that instance, while `ctx:effective_cost(id)` finds the
  live Echo (cost 1) (`reads.rs`). Wanted: an `effective_cost` that takes an
  instance table, or a self-snapshot form as `killer_of_self`.

### The typed module lags the bindings — no card waits

`crates/cards/luau/jackioh.luau` (generated, `crates/luau/tests/export_api.rs`)
declares less than runs, so strict scripts cast through `any`:

- `J.pool(name: string): {string}`, where the binding takes the excluded id and
  a query and answers definitions (`api/mod.rs`, `pool_catalog`); batch 13's
  Transmogulate called it through `any`. `J.query` is typed `{string}` too and
  answers `catalog::query`'s definitions.
- `ConditionContext` lists a subset of the readers `shared_readers!` binds on
  it; batch 15 called `active_units_of`, `permanents_on_field`,
  `plague_multiplier_of`, `unit_view` and `card_type_of` through `any`.
- `J.TRAP_TYPES` is typed `{string}`, not `{CardType}`, so Zoomerbin Oomen
  (core-067) casts it.

## Not API gaps: divergences and the bench

Five of the 28 stay Rust for PORTING.md §5's second or third reason; their
rows in `README.md` §4 say why. classic-036 Burn and meditative-022 Mind Games
diverged under `luau diff` (a Radiant ingredient in a base fusion ran its base
`cry`; the reconcile fixed the host, `script.rs`'s direct hooks, and
neither was ported again; Burn's other divergence is the `effective_cost` gap
above). classic-088 Siphon Squad, core-014 Jlockeed's Weapons and core-065-1
Spikey Pillow break `luau bench` with their auras; a cheaper aura call in
`crates/luau` would let them port.

## Added since part 28 (the reconcile)

Added by #442's reconcile, with `main` merged in (`README.md` §7):
`J.list(t)` (`api/mod.rs`: an empty table crosses as `[]` in untyped data);
`ctx:immutable_bars` (`reads.rs`; Classic #5 Tesla); `J.cloak_hand_aura`
(`api/aura.rs`; Meditative #45, #45.1, #99); the `revealEcho` hook (above); and
the face fix: only an adapter both faces share picks its face by `ctx.radiant`
(`script.rs`'s direct hooks; since #871 only a hook both faces hold the function
under shares one).

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
   Not quite every: a reader registered in `register_ctx` outside the macro
   stays on the effect context, as `field_spells_doomed` does (Datacenter
   Fire, "Open gaps" above).
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
   Only that window: the events before it are still out of reach (Shadowstep,
   Frostspatula, "Open gaps" above).
7. **`J.config`** (`J.config.CHAIN_OF_THOUGHT_REPEATS`; same test). The engine's
   config constants card scripts use, generated into `jackioh.luau` from the
   same source as `engineConfig.ts`, so they cannot drift (CLAUDE.md rule 9).
   Example: `repeats = J.config.CHAIN_OF_THOUGHT_REPEATS`.
   A script needing a constant not listed there adds it to `CONFIG_I32`
   (`crates/luau/src/config.rs`) — never a literal. `i32`s only: a table
   constant (`TRIBAL_TAGS`, `MONEY_MACHINE_ODDS`) has no place there yet
   ("Open gaps" above).
8. **A generic aura** (`J.aura({ mod = ..., applies = ... })`, `api/aura.rs`;
   `gap8_generic_aura_entries`). A stat `mod` and an `applies` filter declared
   as data: either a `CardQuery`-shaped filter table, evaluated in Rust with no
   Luau call per stat read, or a named `aura_filter` predicate called per
   candidate unit. Example: `J.aura({ mod = { attack = 2 }, applies = { side = "self" } })`.
   Hold it to L13 with `luau bench`; a card that breaks the budget stays Rust
   (§5's third reason). The filter table is `AuraFilter` (`side`, `rows`,
   `defId`, `exceptDefId`). core-046 Suppressive Aura and core-058 Rush Token
   Farm port on it (batch 19); Jlockeed's Weapons and Spikey Pillow broke the
   bench on it, and five auras need what it cannot say ("Open gaps" above).
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
   The snapshot read is `killer_of`'s alone: `keywords_of` and `base_stats_of`
   still read the live instance ("Open gaps" above).
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
    adapter — only the named resolution follows the run. The faces share an
    adapter only under one hook key, since an adapter looks its function up
    under the key it was built for (#871): a function under `cry` on the base
    face and only under `death` on the Radiant face gets an adapter a face, each
    running its own face's function
    (`one_function_under_another_hook_on_each_face_runs_on_both_faces`).
15. **`.luaurc`'s alias** (`crates/cards/src/scripts/.luaurc` maps `jackioh`
    to `"../../luau/jackioh"`, no extension). luau-analyze 0.740 refuses the
    extension as `Unknown require` under `--mode=strict`; without it the CLI
    resolves the alias, so strict runs check the files in place — no copies
    with a relative require.
