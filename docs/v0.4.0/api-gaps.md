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
  do not. They are closures over the watched kills (a live `CardInstance` in,
  per-pair effects out), and the by-name pattern the other closure verbs use
  (`after_state_check`, `draw_while`, `for_each_card`: a hook name plus data,
  resolved from the card's own chunk when the verb builds) needs a second design
  pass with the batch that ports its card: the pair records cross the boundary as
  data. Until then the Luau verb watches nothing and `during` still applies.
  Stays in Rust: Classic+ #19.2 Jungle Loser (which also needs
  `random_attack_targets`, bound).

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
- `left_field_since_resolved` — core-041 Sheepish. Takes the trigger event; no
  event-parameter reader is bound.
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
