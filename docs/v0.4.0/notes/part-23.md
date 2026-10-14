# v0.4.0 Part 23: Meditative Cry cards, second half (23 cards and 3 held back)

Notes for issue #762 (v0.4.0 part 23 of 29, batch 15): 19 Meditative cards plus the 7
Classic/Classic+ cards earlier batches held back (#442, L27) considered for a Rust-`script()` to
Luau port. 23 port, 3 stay in Rust with their reasons below, and the proofs. Base commit:
`cb9bfbbed` (`bot/issue-442` with parts 28, 11 and 22 landed; every card of this batch still Rust
there, except batch 11's own ports).

## 1. Ported to Luau (23)

Each `.luau` sits beside its `.rs`. The `.rs` keeps its `//!` header, `pub const ID` and its
`#[cfg(test)] mod tests`. Its `script()`, the helpers and constants only `script()` used, and its
`use` lines are gone. Every effect keeps the Rust order, and every declared number is still a
`ctx:param` read (R386). Where the Rust built one script for both faces, the module says
`radiant = "base"` (Medina Outfitter).

| Card | Id | Luau shape |
|---|---|---|
| M #39 赌石 Addict | `meditative-039` | `cry`: `add_to_hand` of `meditative-039-1`, `radiant` on the Radiant face only |
| M #43 CN Jade Market | `meditative-043` | `cry`: `max(ctx:x(), 0)` (R348) `add_to_hand` Rocks, `radiant` per face |
| M #46 Conjure Intellect | `meditative-046` | `cry`: three `add_random_from_catalog` (KY, CN, Book; `excludeDefId` itself, R387), `radiant` and `costOverride = param("setCost")` when `ctx:embiggened()` |
| M #48 Tranquility | `meditative-048` | base `cry`: `end_turn` first (E10 runs the rest of the list first, R456), then `make_hero_immune`; radiant: the immunity only |
| M #50 CN Tech | `meditative-050` | `cry`: `ctx:win_rate_leader` (never itself, R387; a random non-token card when no row qualifies, R142), `radiant` per face |
| M #51 Devin Bot | `meditative-051` | `cry`: `fill_board_random` (Units with printed base 1/1, tokens included, never itself, R1080), `radiant` per face |
| M #55 Dragon Fruit | `meditative-055` | `cry`: `add_random_from_catalog` (Prime, `param("cards")`, tokens taken, R382/R1421), `radiant` per face |
| M #56 House Party | `meditative-056` | `cry`: `fill_board` with `core-003` (R64), `radiant` on the Radiant face only |
| M #62 Small Time Recruits | `meditative-062` | `cry`: `for_each_card` with `data = { radiant }` (gap 2); `each_cards` answers the deck's distinct non-X Units costing 1 (R65/R66, R1061), all if fewer, first `param("cards")` of a shuffle (R113); `each_card` is `draw_from_library` |
| M #70 I'M WILL BE YOUR DOOM | `meditative-070` | `cry`: `summon` of `meditative-070-1` under the enemy (R360, R1066); radiant adds a second, Radiant one |
| M #73 Plate Packer | `meditative-073` | one shared `cry` on both faces: `armor_on_field × param("per")` (heroes' Armor included, R1064), no-op at 0, else `buff` self; the Radiant face's 2 is its declared `per` |
| M #77 Bulk Booster | `meditative-077` | `cry`: `hand_cap_of − zone_count(hand)` (R1143) Commons added, `radiant` per face; a full hand adds nothing and draws nothing from the rng (R129) |
| M #78 Occidentless Mandate | `meditative-078` | `cry`: count your aimed non-CN permanents first (the exile is harmful, so an immune card is in neither count nor exile, MD-B1/R940), `exile_all`, then that many CN cards (`costMod = −param("discount")` on the Radiant face) |
| M #79 Touched by KY | `meditative-079` | `cry`: `set_hand_cap(param("handSize"))` first (R1143), then base one `draw`, radiant `param("draw")` separate `draw_priced` (Radiant if it reached the hand, R596) |
| M #81 Deadman's Hand | `meditative-081` | `cry`: snapshot hand order plus this card (`copyOf`, R57), then per copy `shuffle_into`, Radiant if already Radiant or (embiggened and one `chance`-in-100 roll, R1144/R311/R129); then the Radiant face's `draw` |
| M #82 Medina Outfitter | `meditative-082` | `cry`: `upgrade` (hand scope, `times = param("times")`); `radiant = "base"` |
| M #90 Spell Basket | `meditative-090` | `cry`: `add_random_from_catalog` (non-token Spells of every set, never itself, repeats allowed, R380/R1420/R387/R60), `radiant` per face |
| M #102 Catboy Maid SSR+ | `meditative-102` | `cry`: `Rng::lucky` written out (below) for `draw` then `add_jade` |
| C #43 Plague Nuke | `classic-043` | `cry`: read the tokens and the plagued non-token Units first, `destroy_all`, then `after_state_check` with `data = { mana, plagued }`: `gain_mana`, and on the Radiant face a `summon` of each plagued Unit now in a graveyard (R59, R174); `preview` labels the mana now (R280) |
| C #59 Plague Doctor | `classic-059` | `cry`: damage of every Plague Counter on the field plus the Radiant face's own placement, then the placement; `preview` the same number (R280); `radiant = "base"` |
| C+ #12.6 Frozen Wastes | `classicplus-012-6` | `cry`: `destroy_all`, then `for_each_card` with `data = { count, deck }` (gap 2): the doomed Units' count (R46) of the face's deck exiled; `preview` the count capped by the deck (R280) |
| C+ #18 Gullible Treatler | `classicplus-018` | `startOfTurn`: `sacrifice` itself when no (base: its controller's; radiant: any) backrow holds a Field Spell or Trap; `conditionMet` the same in its zone (below) |
| C+ #50 Adaptive Growth | `classicplus-050` | `cry` branches on fewer Units than the opponent; `conditionMet` is the same `fewer`, so the glow and the branch agree (R195); one `conditionMet` on both faces |

Seven translations are not word for word. Each gives the same answers:

- **Catboy Maid's Lucky rolls are `Rng::lucky` written out**, as part 22 wrote CN Peptides'
coin: one roll, then one more per Lucky, keeping the highest. The Lucky is
`ctx:lucky_on(live_self) + ctx:luck_of(controller)` (R1438, R987).
- **Gullible Treatler reads the backrow off `permanents_on_field`.** `ctx:card_at` reads the
units row only, so the check filters `permanents_on_field` to the player's backrow and tests
`card_type_of` against Field Spell, Trap and Field Trap (a Field Trap is a Trap, §8
Conventions; an animated one is a Unit, §6.1).
- **Frozen Wastes and Small Time Recruits carry their captures through `for_each_card`
`data`** (gap 2): the Rust closures captured `count`/`deck` and `radiant`; the Luau `cards`
and `each` functions take them as their second argument, as `after_state_check` passes its.
- **Plague Nuke reads the board once, before anything dies**, into `data`, exactly where the
Rust bound `mana` and `plagued` before the `destroy_all`.
- **Deadman's Hand snapshots `(id, defId, radiant)` first**, then rolls: the hand in order,
then this card. `card.radiant == true` and `ctx:self_() ~= nil` also hold where a reader
answers `None` as `nil` (gap 12).
- **Plague Doctor's number is one shared function** (`damage_now`) read by the `cry` and the
`preview`, so the previewed value and the dealt damage cannot drift.
- **The condition-context readers the type stub does not list are called through `any`.**
`active_units_of`, `permanents_on_field`, `plague_multiplier_of`, `unit_view` and
`card_type_of` are bound on every context at runtime (gap 3); the committed
`ConditionContext` stub does not declare them, so the hooks pass their context as `any`
rather than widen `crates/luau` from a batch.

## 2. Kept in Rust (3)

- **M #95 Call to Chaos (Meditative Edition) (`meditative-095`).** Glue to
`subsystems::call_to_chaos` with this edition's table (`CHAOS_MED_EFFECTS`), as Core #95 and
Classic+ #73 are (L14): no `call_to_chaos` binding exists, and PORTING.md §5 keeps glue cards
in Rust.
- **C #36 Burn (`classic-036`).** Ported, then restored: `luau diff` proved two divergences
against the Rust script. (1) A fused card with a RADIANT Burn ingredient in a base fusion:
the engine runs the ingredient's radiant `Script` with `ctx.radiant = false`, and the Luau
host picks the face table by `ctx.radiant`, so it runs `base.cry` and never draws (host bug,
gap 14; latent for every Luau card with two face functions; a batch must not touch
`crates/luau`). (2) Echo copying Burn: the copy's `conditionMet` runs with `self_` as the
text face (cost 0), while Luau `effective_cost(id)` looks the live Echo up (cost 1): no
`effective_cost` over an instance table exists. Either alone keeps it Rust.
- **T-AI-6 Datacenter Fire (`classicplus-t-ai-06`).** Its `preview` needs `field_spells_doomed`
on the condition context; the reader is bound on the effect context only (`crates/luau/src/reads.rs:709`,
outside the `shared_readers!` macro at `:868`). An AI token, so `luau diff` cannot force it
either; its own tests (unchanged) still prove it.

## 3. The tests

No assertion changed: the diff removes no line with `assert` and adds none. The test edits are
imports and the scripts' source, made because `script()` and the module's `use` lines are gone:

- `script()` in a test is now `crate::scripts_of()[ID].clone()` (C+ c018, C c043, C c059),
`crate::scripts_of()[super::ID].clone()` (C+ c050, whose tests module nests one deeper), and
the `condition_met` assertion in C+ c018 reads the registry the same way. Each file adds
`use super::ID;` where the tests name it (C c043, C c059).
- Two tests used a constant the removed script declared. Each file keeps it as a documented
`pub const` above the tests, so the test lines stay byte for byte: `PERCENT` (M c081) and
`MIN_DRAWS`, `MAX_DRAWS`, `MIN_JADE`, `MAX_JADE`, `MAX_JADE_RADIANT` (M c102).
- The other 17 stripped files change no test line at all: their tests reached everything
through their own imports.

## 4. Proofs

All of these ran on this build, in this worktree, on linux x86_64 with 2 threads (Intel Xeon
Platinum 8488C). The parent oracle is `jackioh` release-built from the base commit in a
detached worktree (sha256 `9332725d…`; this build's is `42deff44…`).

- **Card tests.** The 26 cards' own tests: lib 415 passed, cross-card (`--test cards`) 52
passed, 0 failed. The full `jackioh-cards` suite
(`cargo test -p jackioh-cards --features jackioh-engine/testkit`): lib 5860 passed, cross-card
752 passed, 0 failed. `cargo jackioh spec check` (805 rulings, 1710 proofs, 27191 citations:
ok), `catalog check` and `patches check` are clean too. `cargo fmt --check` passes on every
file this part touches; the one diff it reports, `crates/luau/tests/api.rs:1101`, is already
there at the base commit and belongs to part 28. `cargo clippy -p jackioh-cards --all-targets
-- -D warnings` reports exactly one error, the pre-existing `unused import: super::*` in
`core/c078_fullsend.rs` (batch 11's file, untouched here); nothing in this part's files.
- **`luau diff`.** A deck holds 20, so 25 forced ids ran as two full 200-game runs (seeds
1–200), each card forced into both decks of its run (part 9 and part 22 split the same way):
  - `meditative-039,-043,-046,-048,-050,-051,-055,-056,-062,-070,-073,-077,-078`: recorded 200
games (24,278 steps), 0 mismatches.
  - `meditative-079,-081,-082,-090,-095,-102,classic-036,-043,-059,classicplus-012,-018,-050`:
recorded 200 games (24,068 steps), 0 mismatches. (`meditative-095` and `classic-036` are Rust
on both builds, so they match trivially; they ride along to prove nothing else moved.)
  - Frozen Wastes (`classicplus-012-6`) is forced through Mother Pancake (`classicplus-012`):
the tool refuses a token no deckable card's `refs` names, with that exact advice (R279).
Datacenter Fire (`classicplus-t-ai-06`) has no deckable parent, so no run forces it; its own
tests prove it.
- **`golden check`**: 226 of 240 games replayed identically (21,069 steps) on this build and
on the parent, with byte-identical diverged seed sets (14, all in the base: `jackioh-fuzz-95`
step 25, `jackioh-fuzz-48` step 85, `jackioh-fuzz-handicap-216` step 44 among them, batch
11's Felinor Fuser divergence the maintainer announced). Nothing of this part's diverges:
identical sets mean the Luau ports behave exactly as the Rust oracle on all 240 games.
**`fuzz --seeds 200`**: 200 passed, 0 failed (0 panics, 0 over the action bound or stalled,
0 replay mismatches, 0 invariant violations).
- **`luau bench --parent-bin` exits 1 on this base**, before measuring: `measure()` replays
the checked-in golden file on the parent first, and the parent diverges 14 times (above), so
there is no clean golden step to time. That red is the base's, not this part's (the
maintainer's heads-up). The three timings were mirrored instead, replicating `measure()`:
`fuzz --seeds 200` wall on each binary, the `gate` perf gate's slowest decision on each, and
the golden step over freshly recorded forced traces (groups A and B, seeds 1–40, 9,870 steps,
replayed with `RAYON_NUM_THREADS=1`, clean on both builds). Within every budget:

| Number | Parent | This build | Ratio | Budget |
|---|---|---|---|---|
| `fuzz --seeds 200` wall | 37,834 ms | 38,814 ms | ×1.03 | ≤ 2× |
| Slowest `gate` decision (scaled) | 98.484 ms | 101.849 ms | ×1.03 | ≤ 1.5× and < 1500 ms |
| Golden step | 2.199 ms | 2.451 ms | ×1.11 | ≤ 2× |

- **Strict analysis was not run.** This machine has no `luau-analyze`, the vendored Luau
sources (`luau0-src`) carry no Analysis library to build one from, and fetching the 0.740
release was not permitted in this run (as in part 22). The files follow the strict idioms of
the landed batches: `--!strict`, annotated contexts, `any` only where the committed
condition-context stub lags the runtime, and every number a local or a `ctx:param` read.

## 5. Decisions the plan did not settle

- **Burn stays Rust after a real port attempt** (section 2). Both divergences came out of
`luau diff`, not review: the fused-radiant-ingredient game (seed 70 step 144) and the Echo
game (seed 63 step 65). Rejected: shaping the port around either, which would bake the host
quirk and the missing instance-table reader into a card script.
- **Datacenter Fire stays Rust on the reader, not the labels.** Its labels do come off the
catalog text, which Luau can read; what it cannot read is `field_spells_doomed` on the
condition context. Rejected: a preview that answers a different count.
- **Test constants stay above the tests as `pub const`** (`PERCENT`, the five Maid ranges),
with a doc comment naming the script that states them too. Rejected: moving them into the
tests module (part 22's choice for `CURSE`/`EFFECTS`); here the constants were already
top-level items shared by the script, so keeping them top-level keeps every test line
unchanged.
- **`luau diff` ran as 13 + 12 ids** (a deck holds 20). The stay-Rust forceable cards ride
along in group B. The unforceable token rides through its maker (012-6 through Mother
Pancake); the parentless AI token is proved by its own tests.
- **The bench numbers are a mirror, not `luau bench`.** The real gate cannot go green while
the base golden file is red; the mirror replicates its three measures and budgets exactly,
and the failure itself is recorded here rather than worked around.

