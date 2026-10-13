# v0.4.0 Part 13: Classic+ Cry cards, second half, and the AI tokens (26 cards)

Notes for issue #455 (v0.4.0 part 13 of 29): 25 Classic+ cards and tokens
ported from Rust `script()` to Luau, 1 staying in Rust with its reason, and
the proofs.

## 1. Cards ported (25)

All run from `crates/cards/src/scripts/classic_plus/<file>.luau`; the `.rs`
keeps `pub const ID`, the header comment and `mod tests` unchanged (L16).
Every effect keeps the Rust order; declared numbers stay `ctx:param` reads
(R386); AI cards declare no params (B8), so their numbers live in the file
as named constants (rule 9).

| Card | Luau shape |
|---|---|
| classicplus-036 Conjure Bones | `shuffle_into` of Bone Storms; `count`/`paidCount` via `ctx:embiggened()`; per-face `radiant` flag |
| classicplus-038-1 Solarius Prime | `cast_random` over `type = "Spell"`, `count = ctx:param("casts")`, `targetEnemies`; per-face `radiant` flag |
| classicplus-043 AI Slop | `count < 2` → `add_random_from_catalog` (costOverride 0) else `fuse_generated`; `{ tags = { "AI" }, token = true }` query; `radiant = "base"` |
| classicplus-053 Book of Tokens | Lucky roll loop (`rng_next_int` + `lucky_of` + `luck_of`, keep max, R987); `summon` of Rush tokens; per-face `radiant` flag |
| classicplus-054 Book of Books | `add_random_from_catalog` Book query, `count`/`costOverride` params, `temporary`; per-face `radiant` flag |
| classicplus-055 Book of Greed | Legendary/Mythic query, `count` param; per-face `radiant` flag |
| classicplus-056 Book of Pain | `discard_random` on enemy, `discards` param; `radiant = "base"` |
| classicplus-057 Book of Stats | help-aimed (`aim = "help"`, R656) target; `buff` with param amount; `radiant = "base"` |
| classicplus-058 Fruit Basket | `tags = { "Fruit" }` query is the Fruit pool, Grapes included (R382) |
| classicplus-059 All Purpose Apple | `summon` Rush token, `heal` selfHero (hoisted `J.TargetSpec`), `damage` chosen — in that order |
| classicplus-065-1 Rotten Grape | `lose_health` self, `loss` param; `radiant = "base"` |
| classicplus-065-2 Normal Grape | `damage_enemy_or_heal_friend` + N `draw_priced` with `costMod` discount; `radiant = "base"` |
| classicplus-065-3 Large Grape | same with `costOverride` setCost; `radiant = "base"` |
| classicplus-065-4 Golden Grape | `set_radiant` on the pick, radiant also on neighbours (`adjacent_to` on field, index neighbours in hand) |
| classicplus-065-5 Mythic Grape | `replace_hand_with_random` Mythic query, `costOverride` param, `radiant = if radiant then true else nil` |
| classicplus-065 Two Grapes | `add_rolled_grapes` with `grapes` param (the `roll_grape` pool reader, R382) |
| classicplus-066 Vine of Grapes | same verb, its own `grapes` param |
| classicplus-067 Pear | loop of `summon_random` (`Unit`, cost 1, Common) over the `units` param |
| classicplus-069 Buff Billy | `upgrade` self `ctx:x()` times, radiant `RADIANT_UPGRADES_PER_X * ctx:x()` |
| classicplus-071 Book of Buff | help-aimed target incl. hand; `upgrade` chosen, `times` param; `radiant = "base"` |
| classicplus-072 Book of Nerf | `degrade` chosen; one shared `cry` fn, radiant targets add own hand (R275) |
| classicplus-075 J-lease J-Jungle EX-plorer | `shuffle_into` of the `classicplus-075-1` pack id string, `packs` param |
| classicplus-t-ai-03 Hallucination | `add_library_copies` of enemy, 1 / 2 copies, `BRITTLE = 2` |
| classicplus-t-ai-05 Autocomplete | `last_face_up_played` of the opponent, copy by defId/face (+ `chinese`, R1300), costOverride 0 radiant |
| classicplus-t-ai-07 Alignment Tax | `add_cost_rule` on enemy, `theirNextTurn`, `TAX = { base = 1, radiant = 2 }` |

Test-only changes (no assertion changed): every `super::script()` now reads
the registry (`crate::scripts_of()[super::ID].clone()`), plus
`use std::sync::Arc` in c065_2, c065_3 and c071 where the `ptr_eq`
assertions now need it; `use` lines for the removed `script()` bodies
narrowed or dropped.

## 2. Cards staying Rust (1)

- **classicplus-t-ai-04 Chain of Thought**: its Cry is one
  `draw_while_cheap` with `repeats = CHAIN_OF_THOUGHT_REPEATS`, the engine's
  named constant (`crates/engine` config). The Luau API binds the verb but no
  reader for that constant, so the Luau would have to restate 4 — breaking
  rule 9 (every number a named constant). Missing API, PORTING.md §5 reason
  1; the batch does not extend the API. Its `.rs` is untouched and no
  `.luau` was written.

## 3. Proofs

Parent oracle: `git worktree add --detach ../base-455 c65d6d55c` (the 26
cards still Rust there, verified) + `cargo build --release -p jackioh-tools
--manifest-path ../base/Cargo.toml` (6m58s). Current binary: the same build
in this worktree (6m49s, via `cargo jackioh`). Worktree removed after the
runs.

- Own + cross tests: `cargo test -p jackioh-cards --lib` 5860 passed,
  0 failed; `cargo test -p jackioh-cards --test cards` 752 passed,
  0 failed (covers `query.rs`, which pins R382's Fruit pool with the Grapes
  in it). The 26 cards' own tests: 101 + 193 = 294 passed, 0 failed.
  `cargo test -p jackioh-ai` 325 passed, 0 failed (T-AI-5's redaction
  behaviour). `cargo fmt --check` clean, `cargo clippy -p jackioh-cards
  --all-targets -- -D warnings` clean, `cargo jackioh spec check`
  (805 rulings, 1710 proofs: ok), `catalog check` and `patches check`
  clean.
- `luau diff --parent-bin ../base-455/target/release/jackioh --cards
  <the 17 forceable ids>`: recorded 200 games (seeds 1–200, 18,245 steps),
  0 mismatches. The Grapes and T-AI-3/5/7 are not directly forceable (no
  refs, R279): they ride along through 058 (Fruit pool), 065/066 (rolled
  Grapes) and 043 (AI pool), which are forced into both decks.
- `golden check`: 240 of 240 games replayed identically (22,713 steps),
  0 diverged. No separate `fuzz` run: the harness leaves whole suites
  (`cargo jackioh fuzz` included) to CI, which runs 200 fuzz seeds on the
  pull request; the diff and bench below cover the same ground here.
- `luau bench --parent-bin ../base-455/target/release/jackioh`
  (linux x86_64, 2 threads, Xeon Platinum 8488C), within every budget:

  | Number | Parent | This build | Ratio | Budget |
  |---|---|---|---|---|
  | `fuzz --seeds 200` wall | 21,706.489 ms | 22,164.521 ms | ×1.02 | ≤ 2× |
  | Slowest `gate` decision (scaled) | 225.975 ms | 235.495 ms | ×1.04 | ≤ 1.5× and < 1500 ms |
  | Golden step | 1.773 ms | 1.774 ms | ×1.00 | ≤ 2× |

- Strict analysis (`luau-analyze --mode=strict`, Luau 0.740 at
  `/tmp/luau740/luau-analyze`; per PORTING.md step 7, copies staged in
  one directory with the `@jackioh` require rewritten to a relative
  one — the only difference from the committed files): exit 0 over all
  25 files, no output.

## 4. Decisions the plan did not settle

- **t_ai_04 stays Rust** (§2 above). Rejected: restating `repeats = 4` in
  Luau (rule 9 forbids it) and binding a new API reader (the plan forbids
  touching `crates/luau`).
- **c075 writes the token id string.** Its Rust read
  `crate::card_def("classicplus-075-1").id` is the id itself, so the Luau
  names `"classicplus-075-1"` directly (`PACK`). This is the cover
  `api-gaps.md` foresees for the `crate::card_def` gap ("each batch judges
  per card whether the covers suffice"); no gap entry changes.
- **Reader-None checks spell `type(x) ~= "table"`.** A reader's `None`
  crosses into Luau as a null userdata, never nil, so `== nil` is always
  false; the explicit type check is the only correct spelling (c053
  `live_self`, c065_4 `instance_of`, t_ai_05 `last_face_up_played`).
- **Optional `radiant` flags spell `if radiant then true else nil`.**
  Omitting the field is Rust's `None` (base faces summon/add base copies);
  `false` would cross as a value. Used by 065-5, 065, 066, 067, 075.
- **Shared `cry` across faces (c072)** rather than two identical
  closures. It is read-only declaration-time code, so L8 (no module-level
  state) is unaffected — the same reasoning as part-09's shared `targets`
  tables.
- **No separate `fuzz --seeds 200` run** (plan step 6): the harness for
  this run leaves whole suites to CI. `luau diff` (200 recorded games),
  `golden check` and the `fuzz --seeds 200` wall measurement inside
  `luau bench` exercise the same paths here.
