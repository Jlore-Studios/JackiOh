# v0.4.0 Part 25: Meditative batch 17 (27 cards ported, 2 stay Rust)

Notes for issue #764 (v0.4.0 part 25 of 29), built by
`docs/v0.4.0/PORTING.md`. Of the batch's 29 cards, 27 run from Luau now
and 2 stay Rust (section 2). Every test in the 29 files is unchanged
except where a test called `script()` or used a constant `script()` alone
owned (section 3).

## 1. Cards ported (27)

Each has a new `crates/cards/src/scripts/meditative/<module>.luau`. Its
`.rs` lost `script()`, the helpers and constants only `script()` used,
and the now-unused `use` lines. It keeps its header comment,
`pub const ID` and `mod tests`. Every effect keeps the Rust order;
declared numbers stay `ctx:param` reads (R386).

| Card | Hooks | Luau shape |
|---|---|---|
| `meditative-004` Juicy Kumquat Melon | `cost`, `cry` | `choices` collects the six topmost Meditative library picks into the host-made list (cleared first, so an empty pick list still crosses as `[]`); `remember` notes them under `kumquatPicked`; two `for_each_card`s draw them and (radiant) discount them in hand |
| `meditative-019` Expedition1234 | `cry`, `static_flags` | `lose_refresh_through` + `summon` of the Rift (radiant flag on the radiant face); `radiant = "base"` for the shared quickdraw flag |
| `meditative-021` API Key Fishing | `cost`, `cry`, `end_of_turn` | `give_from_hand` on a `param` chance roll (`paidChance` embiggened); end-of-turn return gated on `live_self` + `returnToHandAtEndOfTurn` / `was_played_this_turn`; `radiant = "base"` |
| `meditative-025` Blue-Eyes White Felinor | `static_flags` | `tribute = 2` both faces; the immunity and Immutable are catalog data |
| `meditative-026` Alternate Fate | `end_of_turn`, `static_flags` | `noNaturalMana` both faces; `fill_board` of Mana Wells (`radiantFor = "self"` radiant) |
| `meditative-028-1` Ancient Curse | `cry`, `static_flags` | `castOnDraw` + `damage` of `param("damage")` to own hero; `radiant = "base"` (Pierce is catalog data) |
| `meditative-030-1` Love Bomb | `cry`, `static_flags` | `castOnDraw` + uncapped `heal` of `param("heal")`, target hoisted as `J.TargetSpec`; `radiant = "base"` |
| `meditative-040` Feng Shui | `static_flags`, `preview` | `fengShui` + literal `luck = 1` (the Rust `const LUCK`, not a param); both element previews through `subsystems_feng_shui_*` readers; `radiant = "base"` |
| `meditative-052` Economic Anxiety | `attack_mods` | shared `anxiety` helper: no bonus under tribal-hate immunity (R940, so the yellow skips it too) or a shared printed tag; `param("bonus")` attack, Poisonous on the radiant face only |
| `meditative-057` Clip-Farming Critikal | `cry`, `condition_met` | Combo 1 via `played_earlier` / `played_ids_this_turn` / `find_instance`; graveyard/exile returns to hand, own field Bounces (`set_radiant` first radiant); `conditionMet` is the same read gated to hand |
| `meditative-058` Permanent Underclassman | `cry`, `grants` | `summon_random` ((1) Unit) then `for_each_card` with `data = { books }` over `summoned_so_far`, granting `bookDeath`; the granted Death adds `grant_param("books")` random Books |
| `meditative-059` Permanent Upperclassman | `cry`, `grants` | same shape over a (4) Unit, granting `fusedBookDeath`; the granted Death fuses one Book + one AI pick (`fused` hand price) |
| `meditative-072` The Banisher | `static_flags` | `exilesOnDamage`; `radiant = "base"` (Cleave is catalog data) |
| `meditative-074` Montaña Giant | `cost`, `end_of_turn` | cost off `J.cost(ID)` minus `discount` × hand size (floor 0, hand/graveyard-playable only); full-hand end of turn `set_radiant`s (base) or Buffs `param("buff")` (radiant) |
| `meditative-084` Volatility | `static_flags` | `tuneMultiplier` from `J.config` (`VOLATILITY_TUNE_MULTIPLIER` both ways base; `VOLATILITY_RADIANT_BUFF_MULTIPLIER` upgrade-only radiant) |
| `meditative-086` Mayor Medinamogger | `static_flags` | `randomTargets`; `radiant = "base"` (Lucky 1 is catalog data) |
| `meditative-091-1` Windfurious Prime | `static_flags` | `attackJoiners`; `radiant = "base"` (First Strike is catalog data) |
| `meditative-091` Windfast | `death`, `static_flags` | `tribute = 1`, `attackFromHand`, base-only `bounceAttacker` (absent on radiant via a `nil` field); Death shuffles one (Radiant) Prime |
| `meditative-093` Growing Felinor | `death`, `static_flags` | `neverDefense`; Death summons `-093-1` (radiant when radiant) |
| `meditative-093-1` Growing Felinor Sr | `death`, `static_flags` | same shape, summons `-093-2` |
| `meditative-093-2` Growing Felinor Sr Sr | `death`, `static_flags` | same shape, summons `-093-3` |
| `meditative-093-3` Growing Felinor Super Senior | `static_flags` | `neverDefense` only |
| `meditative-096-1` Journey Complete | `cry`, `static_flags` | `castOnDraw`; Cry returns each journey-noted exile card to hand, Radiant with a `discount` cost mod on the radiant face |
| `meditative-097-1` Empty Plot | `static_flags` | `stackBase` (+1 buff radiant) |
| `meditative-097-5` Bunker | `static_flags` | `tribute = 2`, `laneMultiplier = 3` |
| `meditative-097-7` The Great Wall | `cry`, `static_flags` | `tribute = 3`; Cry `lock`s all five own unit lanes |
| `meditative-097-8` Prison | `triggers`, `static_flags` | `tribute = 2`; `prison-capture` trigger on enemy Unit `cardResolved`: `param("chance")` percent roll plus Lucky rerolls, then `capture` |

`None`-answering readers (`find_instance`, `live_self`, `self_`) hand
Luau `nil` since part 28, so plain `== nil` / `if x` tests apply
(`type(x) == "table"` still correct where used).

Test-only changes (no assertion changed): where a test called `script()`
it now reads the registry (`crate::scripts_of()[ID].clone()`); constants
only `script()` used moved into `mod tests` (`RIFT_ID`, `YOUR_LAST` /
`THEIR_LAST`, three `NEXT`s, `PRIME`, `JOURNEY_KEY`, `BUFFS`,
`TRIBUTE_COST`); c058/c059's tests re-import
`jackioh_engine::prelude::*` lost with the parent's `use` lines.

## 2. Cards staying Rust (2)

Each has its `.rs` exactly as at the base commit and no `.luau`.
PORTING.md §5 reason 1 (missing API, `docs/v0.4.0/api-gaps.md` "True
gaps") in both cases; the batch does not extend `crates/luau`.

- **meditative-071 Pareto Optimality**: its effect is
  `subsystems::cane_strike` + `subsystems::cane_return` on the remembered
  Cane. Neither is bound (api-gaps: "one-method bindings each; they wait
  for a batch that needs them").
- **meditative-089 Jlarna**: its static flags carry the catalog's declared
  numbers, read via `crate::card_def(ID)` at script build
  (`creditLine`, `creditInstalments` from `params`). A bare "give me this
  id's def" has no Luau form (api-gaps); a literal would duplicate catalog
  data and stop following tuning. `J.cost` / `J.query` / `def_by_index`
  do not cover flag params.

## 3. Proofs

Parent oracle: the merge-base tree at `03771fb3a` (this branch's merge of
`origin/bot/issue-442`), which holds every part landed before this one
and all 29 batch cards still Rust — built as a release `jackioh` in a
detached worktree, removed afterwards. (The batch's nominal base
`5e292a8fb` predates #461's landing: diffing against it flags 3
mismatches from #461's Classic ports, identical on both sides of this
batch; `03771fb3a` isolates this batch.)

- `cargo test -p jackioh-cards --lib` over the 27 modules: 310 passed, 0
  failed (neighbouring-module filters catch a few extra `c097_*` files).
- `cargo jackioh luau diff --parent-bin <parent>`: two runs (a 20-card
  deck cannot hold all 27 at once), seeds 1–200 each —
  - run A (`meditative-004,019,021,025,026,028-1,030-1,040,052,057,058,059,071,072`;
    tokens `-028-1`/`-030-1` forced through their makers): 15,035 steps,
    `golden check: 200 of 200 games replayed identically`,
    `luau diff: 0 mismatches`;
  - run B (`meditative-074,084,086,089,091-1,091,093-1,093,096-1,097-1,097-5,097-7,097-8`;
    tokens forced through their makers): 22,785 steps, 200 of 200,
    0 mismatches.
  - `-093-2`/`-093-3` cannot be forced (no deckable card's `refs` name
    them; R279): they enter through forced `-093-1`'s Death chain, as
    part-14 did for its maker-forced tokens. Their own unchanged tests
    prove them directly.
- `cargo jackioh golden check`: 226 of 240 replayed identically; the same
  14 seeds diverge on the parent oracle byte-for-byte (pre-existing, from
  other batches on this tree — this batch adds none).
- `cargo jackioh fuzz --seeds 200`: 0 panics, 0 over the bound or
  stalled, 0 replay mismatches, 0 invariant violations; 200 hero deaths.
- `cargo jackioh luau bench --parent-bin <parent>`: **blocked, pre-existing.**
  The tool's third measurement replays the golden file on the parent
  first, and the parent (`03771fb3a`) does not replay it clean (the same
  14 divergences section 3 names), so the tool exits before measuring.
  Measured the tool's first two numbers by hand on both release binaries,
  same machine, one run after the other (linux x86_64):
  `fuzz --seeds 200` wall 32.15 s parent / 32.86 s this build (×1.02,
  budget 2×); gate slowest decision 116.2 ms parent / 114.6 ms this
  build (×0.99, budget 1.5×, and far under the absolute cap). The third
  number (golden step) needs a clean replay, which no build on this tree
  has until the reconcile; the identical-seeds result above shows this
  batch contributes nothing to it.
- `luau-analyze --mode=strict`: **not run**. This box has no Luau
  toolchain (`which luau-analyze` empty; `luau0-src` ships no `Analysis`
  sources, nothing downloadable). The new files follow the conventions
  that kept the pilots strict-clean: `--!strict`, annotated contexts,
  hoisted `J.TargetSpec` / `J.AddToHandArgs` locals for union-typed verb
  arguments, `ipairs` only, `J.div`/`J.rem` (no `/` or `^` operators),
  `J.sort_by` where ordering matters. The first run with the toolchain
  should check them.

## 4. Decisions the plan did not settle

- **Oracle at `03771fb3a`, not `5e292a8fb`.** The harness merged
  `origin/bot/issue-442` (#461) into this branch after the batch was
  drawn. Diffing the batch against the nominal base attributes #461's
  Classic-port behaviour to this batch (3 mismatches in run A, all in
  games whose decks draw #461 cards, all absent against `03771fb3a`).
  Rejected: keeping `5e292a8fb` and explaining the 3 away — the proof
  must be 0 mismatches, not 0-mine.
- **Two `luau diff` runs instead of one.** 27 forceable cards (22 with
  their makers) do not fit a 20-card deck; split 14/13 along the issue's
  table order. Rejected: trimming the card list — every ported card is
  forced in exactly one run.
- **c040's `luck = 1` is a literal**, mirroring the Rust `const LUCK`
  (not a `param` read): a Buff must never raise the multiplier's own
  source, and the comment cites R987. Same for c097_7's tribute and the
  other fixed flags.
- **`api-gaps.md` is not edited here**, so the parallel batches don't
  conflict on it. The gaps in section 2 are for the reconcile to fold in.
