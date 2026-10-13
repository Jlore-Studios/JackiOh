# v0.4.0 Part 9: Core Cry cards, first half (22 cards)

Notes for issue #451 (v0.4.0 part 9 of 29): 20 Core cards ported from Rust
`script()` to Luau, 2 staying in Rust with reasons, and the proofs.

## 1. Cards ported (20)

All run from `crates/cards/src/scripts/core/<file>.luau`; the `.rs` keeps
`pub const ID`, the header comment and `mod tests` unchanged (L16). Every
effect keeps the Rust order; declared numbers stay `ctx:param` reads (R386).

| Card | Luau shape |
|---|---|
| core-004 Gary the Gambler | `flip_coins` + `flip_coin_keyword`, keywords hoisted as `J.Keyword`; `radiant = "base"` |
| core-005 Stockpile | `draw` + `heal` (hero target hoisted as `J.TargetSpec`, parity_heal pattern); `radiant = "base"` |
| core-008 Mr. Vanilla | `base = {}`, `radiant = "base"` (printed keywords only) |
| core-011 Tempo Timmy | `base = {}`, `radiant = "base"` |
| core-012 Duplicating Felinors | `summon_copy({ of = { of = "self" }, player = "self" })`; `radiant = "base"` |
| core-015 Me and Mr Token | base one `summon` of `core-t-rush`; radiant numeric-for over `param("tokens")` |
| core-016 Hit Job | `destroy` (+ `destroy_adjacent_to` radiant); shared `targets` table |
| core-020 Pointmaster | `base = {}`, `radiant = "base"` |
| core-021 Hinder | `staticFlags = { castOnDraw = true }` both faces; `next_turn_mana` (negated param) + `discard_random` base only |
| core-025 4-mana 7/7 | `base = {}`, `radiant = "base"` |
| core-026 Glowy Jelly Bean | `hand` targets (1 / 2 picks); `set_radiant` per chosen index 0..picks |
| core-028 Knockoff Temu Glowy Jelly Bean | one `set_radiant_random` over hand+library+field; `radiant = "base"` |
| core-029 GIGA Glowy Jelly Bean | base hand ids via `zone_cards`; radiant adds `active_units_of` ids plus own backrow via `permanents_on_field` filtered to own backrow (`zone.player`/`zone.row`), same units-then-backrow order as the Rust |
| core-034 Collateral Damage | base `exile` + enemy `exile_random_from_library`; radiant `exile_adjacent_to` first, then the base two |
| core-036 Magic Jammed | `lock` (chosen zone) then `destroy` / `steal` |
| core-043 Big Felinor | `destroy_all` with `notTags = { "Felinor" }`, side any / enemy |
| core-044 True Strike | `damage` (chosen, param amount, `ignoreArmor`) then `exile` of self; `radiant = "base"` |
| core-045 Deft Duelist | `base = {}`, `radiant = "base"` (printed keywords only) |
| core-047 Fig of Life | `target` declaration with `aim = "help"`; `heal` with hoisted target; `radiant = "base"` |
| core-049 Snom Bunny Mind Control | `steal` (+ `set_radiant` radiant); shared `targets` table |

Test-only changes (no assertion changed): the 7 `super::script` uses now
read the registry (`crate::scripts_of()[ID].clone()`, `HINDER` for c021,
`super::super::ID` for c049); `use super::{ID, script}` narrowed to
`use super::ID`, c021's `use super::script` removed.

## 2. Cards staying Rust (2)

- **core-035 Lunar Eclipse**: its discount is `add_player_modifier` with
  `expiry = { until = "thisTurn", turn = ctx.state.turn }`, and
  `ModifierExpiry::ThisTurn` requires `turn: i32` (no default,
  `crates/engine/src/state.rs`). `EffectContext` exposes no turn reader
  (full method list in `crates/cards/luau/jackioh.luau` has only
  `*_this_turn` counters, and `crates/luau/src/reads.rs` binds none), so
  the Luau cannot stamp the expiry. Missing API, PORTING.md §5 reason 1;
  the batch does not extend the API.
- **core-042 Eugenics**: `radiant_chance` takes `chance: f64` (0.3 / 0.4),
  but every hook return crosses through `numbers::walk`, which refuses any
  non-integral number (`number_to_i32`; the `bad_numbers` fixture proves a
  1.5 amount fails the hook). It also reads `given_lucky_on` (granted-only
  Lucky, D14), which has no Luau reader (`lucky_on` folds printed, tuned
  and given Lucky together and differs under tuning). Missing API,
  PORTING.md §5 reason 1.

## 3. Proofs

Parent oracle: `git worktree add ../base HEAD` (c65d6d55c; the 22 cards
still Rust there, verified) + `cargo build --release -p jackioh-tools
--manifest-path ../base/Cargo.toml` (7m01s). Current binary built the
same way (6m57s). Worktree removed after the runs.

- Own + cross tests: `cargo test -p jackioh-cards --lib` 5860 passed,
  0 failed; `cargo test -p jackioh-cards --test cards` 752 passed,
  0 failed. `cargo fmt --check` clean, `cargo clippy -p jackioh-cards
  --all-targets -- -D warnings` clean, `cargo jackioh spec check`
  (805 rulings, 1710 proofs: ok), `catalog check` and `patches check`
  clean.
- `luau diff`: 22 forced cards do not fit a 20-card deck, so two runs of
  11 cards × 200 games (seeds 1–200) instead of one run of 22 — every
  card still forced into both decks of 200 games:
  - `core-004,…,core-026` (11): recorded 200 games, 21,424 steps,
    0 mismatches.
  - `core-028,…,core-049` (11): recorded 200 games, 14,339 steps,
    0 mismatches.
  - (core-035 and core-042 ride along as Rust on both sides.)
- `golden check`: 240 of 240 games replayed identically (22,713 steps),
  0 diverged. `fuzz --seeds 200`: 200 passed, 0 failed.
- `luau bench --parent-bin ../base/target/release/jackioh`
  (linux x86_64, 2 threads, Xeon Platinum 8488C), within every budget:

  | Number | Parent | This build | Ratio | Budget |
  |---|---|---|---|---|
  | `fuzz --seeds 200` wall | 21,773.298 ms | 21,720.969 ms | ×1.00 | ≤ 2× |
  | Slowest `gate` decision (scaled) | 229.054 ms | 235.203 ms | ×1.03 | ≤ 1.5× and < 1500 ms |
  | Golden step | 1.757 ms | 1.754 ms | ×1.00 | ≤ 2× |

- Strict analysis (`luau-analyze --mode=strict`, Luau 0.740 at
  `/tmp/luau740/luau-analyze`; per PORTING.md step 7, copies staged in
  one directory with the `@jackioh` require rewritten to a relative
  one — the only difference from the committed files): exit 0 over all
  20 files. 15 files report nothing; the 5 empty cards (c008, c011,
  c020, c025, c045) each report one `ImportUnused` lint warning for the
  template header `local J = require(...)`.

## 4. Decisions the plan did not settle

- **c035 and c042 stay Rust** (§2 above). The plan's premise ("only cry
  and declared targets") does not hold for them: c035 stamps the current
  turn into its discount expiry (no turn reader exists) and c042 needs a
  fractional `chance` (the L6 walk refuses it) plus a granted-only Lucky
  reader. Rejected: inventing the turn/chance values, and extending
  `crates/luau` (the plan forbids it). Per step 7 their `.rs` files are
  untouched and no `.luau` was written.
- **Empty faces spell `radiant = "base"`**, including the five
  keyword-only cards (`base = {}`). PORTING.md names that spelling for
  faces that act alike; two empty faces act alike.
- **Identical `targets` shared as one module-level table** (c016, c034,
  c036, c049) rather than duplicated per face. It is read-only
  declaration data (deserialized, never mutated), so L8 is unaffected.
- **Uniform `local J = require("@jackioh")` header kept in the five
  empty files**, accepting five exit-0 `ImportUnused` lint notes, per
  PORTING.md §2 ("start every file with …"). Rejected: dropping the
  line or renaming to `_J` (breaks the template every other batch
  follows).
- **`luau diff` split 11+11** (deck holds 20, the batch has 22): two
  full 200-game runs, same seeds, every card forced in its run.
- **Test `script()` calls read `crate::scripts_of()[ID].clone()`**
  (`HINDER` for c021, `super::super::ID` for c049): the registry build
  of the Luau scripts, so the declaration/emptiness assertions now prove
  the Luau output. No assertion changed.
