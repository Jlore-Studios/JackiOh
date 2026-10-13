# v0.4.0 Part 26: cards to Luau, batch 18 (Meditative prompts, modes, continuations and the rare hooks; 17 cards and 8 held back)

Notes for issue #765 (v0.4.0 part 26 of 29, batch 18, a fullsend part of #442): the 17 Meditative
cards of the plan and the 8 Core and Classic+ cards earlier batches held back (#442, L27), ported
by `docs/v0.4.0/PORTING.md` onto part 28's fixed API (#464, `docs/v0.4.0/api-gaps.md`, "Fixed in
part 28"). 22 of the 25 now run from a `.luau` file. 3 stay in Rust because the API cannot say them
(PORTING.md §5, reason 1). Base commit: `30ad28e29` (`bot/issue-442` with part 28 and batches 11,
13 and 14 landed; every card of this batch still Rust there).

## 1. Ported to Luau (22)

Each `.luau` sits beside its `.rs`. The `.rs` keeps its `//!` header, `pub const ID`, the comment
above its tests and its `#[cfg(test)] mod tests`. Its `script()`, the helpers and constants only
`script()` used, and its `use` lines are gone. Every effect keeps the Rust order. Every trigger id,
mode option, prompt text, `resume` key and `step` is the Rust one, because `legal_actions`, a paused
state and a replayed log name them. Every declared number is still a `ctx:param` read (R386), and
the engine's constants are `J.config` reads (CLAUDE.md rule 9). Where the Rust built one script for
both faces (`base.clone()`), the module says `radiant = "base"`, so the tests' `Arc::ptr_eq` checks
still hold.

| Card | Id | What it declares |
|---|---|---|
| M #8 Reach the Summit | `meditative-008` | `staticFlags` (`quickdraw`, `countsPlays`); a per-face `cost` (`times_played_of(instance)`, Radiant `max(0, level − 1)`); `cry` by level (heal; `choose_target` with `resume["ascent-damage"]`; draw; `discard_random`; a `for_each_card` over the exile picks; `add_random_from_catalog`; nothing; `win_game`); `endOfTurn` return from the graveyard on the turn it was played; `preview` "Ascent level" |
| M #17 True Craft a Card | `meditative-017` | `cry`: `choose_number` 0 to `J.config.CRAFT_MAX_COST`; `resume.cost` opens `choose_craft`, `resume.craft` mints with `craft_card`; `radiant` per face |
| M #20 Aestheticize the Game | `meditative-020` | `staticFlags.quickdraw`, three `modes`, `cry`: `alt_win` with the mode's declared threshold, then on `ctx:radiant()` a draw; `radiant = "base"` |
| M #22 Mind Games | `meditative-022` | `modes` (the three choices in `SecretChoice::ALL`'s order) and `secretModes`; `cry`: `keep_secret` linking a Fortify Mind (Radiant: `costOverride = param("fortifyCost")`), or with no mode only the Fortify Mind; `resume.reward` reads `ctx:data().secret` (gap 13), `resolve_secret`, then the reward, Defend's two `for_each_card`s carrying their numbers in `data` (gap 2) |
| M #22.1 Fortify Mind | `meditative-022-1` | `modes`, `cry`: `ctx:recalled("secret")`, `secret_of`, `guess_secret`, and on `judge(..) == "lost"` the guess's penalty; the base face's `graveyardTriggers.discardPenalty` on its own `discarded` takes all three penalties |
| M #76 Do or Die | `meditative-076` | `cry`: the opponent's hand ids, `rng_shuffle` and the first `param("marks")` (Radiant: all), one `delay` with `handWatch` and the purple steal mark; `resume.steal` reads `ctx:data().handWatch` and `give_from_hand` by `ids` |
| M #85 Playtester | `meditative-085` | `startOfTurn`: `choose_mode` (Book of Buff / Book of Nerf); `resume.book`: `param("books")` × `add_to_hand`, `radiant` per face |
| M #96 Meditative Journey | `meditative-096` | `cry`: `choose_pick` of up to `param("cards")` hand cards (Radiant then draws); `resume.journey`: one `exile` per pick by id, then `shuffle_into` Journey Complete with `memory.journey` the picks |
| M #97 Jlockheed's Evil Blueprints | `meditative-097` | `cry`: `discover_from_catalog` over the nine buildings (`withTokens`); `resume.picked`: `add_to_hand`, `radiant` per face |
| M #1 Disruptive Disruptor | `meditative-001` | `discardGuard` answering `{ "self" }` on both faces; the Radiant face adds `cry`: `draw`, `gain_mana` |
| M #37 CN in a bottle | `meditative-037` | `entersHand`: `transform_random` of itself (`radiant`, `chinese`, `arrives`; Radiant also `costOverride = 0`); no `cry` on either face |
| M #45.1 Knowledge Breaker Prime | `meditative-045-1` | `cry`: `param("traps")` × `summon_random` over `J.TRAP_TYPES` (Radiant ones Radiant), then `destroy_all` (Radiant `exile_all`) over every other CN or KY permanent; `faceDownPlay` `{ units = true }` |
| M #45 Knowledge Breaker | `meditative-045` | `cry`: `degrade` over every other CN or KY card on fields, hands and decks, `param("times")`; `death`: `shuffle_into` the Prime; `faceDownPlay` `{ units = true }` |
| M #99 Paranoia | `meditative-099` | `cry`: `draw`; `faceDownPlay` `{ spells = true }` (Radiant also `echo = param("echo")`) |
| Core #10 Rapid Replenish | `core-010` | `cry` and `conditionMet` through `played_earlier` on the effect and condition contexts (gap 3); `radiant = "base"` |
| Core #31 KY's Math Equation | `core-031` | `staticFlags` (`countsPlays`, `returnKeepsPrice`), `targets`, `cry` (`fib` of the plays), `endOfTurn` return with the climb (`return_price_of`, `set_cost_mod`), `preview` labelled with the face's own "Fib(…)" read off `ctx:def_of(ID)`; `radiant = "base"` |
| Core #60 Bear Honeypot | `core-060` | a `cardResolved` trigger per face (`when` plus the `run` guard, `open_zones`, `costPaid` against `param("costLimit")`), two summons or a `fill_board` and the `forced_attacks`; `conditionMet` through `open_zones` (gap 3) |
| Core #70 Spiteful Stab | `core-070` | `targets`, `cry`, `preview` labelled `ctx:fill_params(ID, face, ctx:params_view())` (gap 3), `conditionMet`; the scaling reads `J.config.HERO_HEALTH` and `J.div` |
| Core #85 Unlicensed Experimentation | `core-085` | a `cardResolved` trigger per face (`when` and `run`, `fuse_cards` with `pick` random / all) and `conditionMet` through `fusable_permanents_of` (gap 3) |
| Core #100 Ceaseless Void | `core-100` | `cost`: `J.cost(ID)` less `param("discount")` × the four `ctx:counters()` (gap 3), floored at 0; `cry`: `exile_all`; `conditionMet`: `ctx:play_cost()` within the unspent mana; `radiant = "base"` |
| T-sheep Sheep Token | `core-t-sheep` | `staticFlags.tributeWorth` of `J.config.SHEEP_TRIBUTE_VALUE` / `RADIANT_SHEEP_TRIBUTE_VALUE` (gap 7) |
| C+ #64 Mulch Muncher | `classicplus-064` | `cost`: `J.cost(ID)` outside a play, else less `param("discount")` × `played_this_game_with_tag(.., "Fruit")` (gap 3); `radiant = "base"` |

Five translations are not word for word. Each gives the same answers:

- **Meditative Journey reads its picks by probing `instance_of`**, as Classic #11 Mind Melt and #44
  Back from the GY do: there is no `targets` reader. `{ of = "chosen", index = i }` answers each
  picked card and nil past the end. The picks are hand cards, which no departure from the field and
  no Spell immunity (`unaffected_by` reads the field only) can hide, so the probe finds every pick,
  in order. The exiles still name each card by id, as the Rust does.
- **Reach the Summit's Ascent 4 passes its ids outright** (`cards = ids`), read in the hook as the
  Rust closure captured them. An empty pool leaves `cards` out, which the verb reads as no cards: an
  empty table would cross as a record (`api/each.rs`).
- **Unlicensed Experimentation walks `permanents_on_field(controller)`** where the Rust walks the
  controller's unit and backrow slots through `card_at`. The reader walks the same slots in the same
  order first and then the other side's, so the port keeps the cards whose zone names the
  controller's side (the zone a field card holds is its slot, `zones::slot_of`). The Luau `card_at`
  reads the unit row only.
- **Mulch Muncher's graveyard test reads the card by id** (`ctx:playable_from_graveyard(card.id)`),
  where the Rust hands `playable_from_graveyard` the instance the cost hook got. That function finds
  the card in its graveyard by id (`in_own_graveyard`: "a price is often read off a copy of the card
  with a probe's X"), and a copy differs only in X and embiggen, so the two answer alike.
- **KY's Math Equation reads its formula per preview call** (`ctx:def_of(ID)`), where the Rust read
  it once as the script was built from `crate::card_def(ID)`. Both are the catalog's text for
  `core-031`.

## 2. Kept in Rust (3)

Each keeps its `.rs` exactly as the base commit has it and has no `.luau`.

| Card | Id | Why |
|---|---|---|
| M #42 CN Flea Market | `meditative-042` | Missing API: the whole card is `subsystems::night_market_script` (the stall roll, the yuan and the `market` prompt's `resume` steps), which `api-gaps.md` lists as unbound; the plan forbids adding it from a batch |
| M #88 The True Sheep | `meditative-088` | Missing API (`crate::card_def`, as `api-gaps.md` lists for this card): its `staticFlags.tributeWorth` is the catalog's declared `worth` (5, Radiant 500), read off the def when the script is built. `staticFlags` is load-time data, no `J` function reads a def's params, and the flag's value counts on its own for a fused card (`fuse.rs` takes the larger worth), so only a literal could stand in for it, which rule 9 refuses |
| M #98 Showdown | `meditative-098` | Missing API: its Cry locks only the zones of the four other lanes, both rows, that are not already Locked, and stores those for the unlock (MD-B21, R945). The Luau `is_locked(player, lane)` reads the unit row only, and nothing reads a backrow lock, so the port cannot tell which backrow zones to leave alone |

These belong in `docs/v0.4.0/api-gaps.md`: `subsystems::night_market_script` is listed there
already; the catalog def at load time (for a declared number in `staticFlags`) and a backrow
`is_locked` are new. This part does not edit that file (the plan names only the card files and
these notes, and the batches build side by side); #442's reconcile folds them in.

## 3. The tests

No test asserts anything new or anything less: the diff removes no line with `assert` and adds none,
and the three Rust cards' files are untouched. In the 22 ported files (all their tests, as many as at
the base commit) the edits are imports and the scripts' source, made because `script()` and the
module's `use` lines are gone:

- `script()` in a test is now `crate::scripts_of()[ID].clone()`, the registry `build.rs` generates
  (M #1, M #20, M #97, Core #100, the Sheep Token; `super::ID` in C+ #64 and `super::super::ID` in
  Core #70). M #37 calls `script()` inline in four assertions and Core #31 imported it, so each tests
  module declares a `fn script()` reading the registry, and their assertion lines stay byte for byte.
- Constants the removed script declared and the tests used now live in the tests module, with their
  doc comments: `BUFF_BOOK`, `NERF_BOOK`, `BUFF`, `NERF` (M #85), `HEALTH_MODE`, `GRAVEYARD_MODE`,
  `BOARD_MODE` (M #20), `FORTIFY_MIND` (M #22), `JOURNEY_COMPLETE`, `JOURNEY_KEY` (M #96) and
  `BUILDINGS` (M #97, which no other file read).
- Names the tests reached through `use super::*` and the removed `use jackioh_engine::prelude::*`
  are imported directly: `cast` (M #8), `cast_new` (M #37) and `std::sync::Arc` (Core #100, for its
  `Arc::ptr_eq` checks, which still hold under `radiant = "base"`). `use super::*;` is dropped where
  it no longer brings anything in (M #8, Core #85).

## 4. Proofs

PROOFS_PENDING

## 5. Decisions the plan did not settle

- **The parent was built in a scratch worktree at the base commit** (`git worktree add --detach
  ../jackioh-wt/base 30ad28e29`, release `jackioh-tools`, removed afterwards), per PORTING.md step 1.
  Rejected: building it in this worktree, which would have meant building before any edit and copying
  the binary out.
- **Three cards stay Rust (section 2) rather than ported with a literal or a near miss.** The True
  Sheep with a literal 5 / 500 would equal the Rust today and drift from the catalog with the first
  patch that moves its worth; Showdown without the backrow check would unlock backrow zones another
  card had Locked.
- **Shared hook functions across faces.** Where both faces run the same body (Paranoia's and Fortify
  Mind's `cry`, Mind Games' `reward`, Reach the Summit's `cry`, `endOfTurn` and `preview`, the
  `conditionMet` of Bear Honeypot and Unlicensed Experimentation), the Luau names one local under
  both faces, whether the Rust built one closure per face or shared one `Arc`. The loader shares one
  adapter for a direct hook (gap 14) and builds one per face for a condition, preview or `resume`
  entry; no test of these cards compares the two faces' hooks, and the per-face reads stay per face
  (`param`, `ctx:radiant()`).
- **Named sub-hooks sit on both faces** wherever a hook both faces run calls `for_each_card` by name
  (Mind Games, Fortify Mind, Reach the Summit), since the run resolves names on the running face.
- **`api-gaps.md` is not edited here** (section 2), as the other batches did not.
