# v0.4.0 Part 28: the API gaps and three Classic ports (#464)

Notes for issue #464 (v0.4.0 part 28 of 29). Re-scoped on 2026-10-13 (#442,
L27): this part lands §2's gaps 14 and 15 (gaps 1–13 were built on the
`bot/issue-464` WIP), ports `classic-026`, `classic-030` and `classic-035`,
and updates `docs/v0.4.0/api-gaps.md`, `PORTING.md` §5 and the readers docs so
the nine batches still to build (#461, #462, #463, #761–#766) port onto the
fixed API. The other held-back cards and the §4 contracts are the batches'
and the reconcile's now — untouched here.

## 1. The gaps

Gaps 1–13 (empty-table round trip, `for_each_card` data, the shared reader
table, `ctx:turn()`, `given_lucky_on`, the events window, `J.config`, the
generic aura, `left_field_since_resolved` / self-snapshot `killer_of` /
`chancePercent` / kill `pairs`+`then`, the shared adapter within one face,
the ungated typed-module check, `None`→`nil`, `ctx:data()`) were built on the
WIP with fixture tests (`crates/luau/tests/api.rs`, `gap*`) green. This run
finished the last two:

- **Gap 14 — one adapter across both faces.** Gap 10's cache lived in
  `build_script`, so each face built its own adapter. It lives in `load_card`
  now (`crates/luau/src/script.rs`), so a function value both faces hold gets
  one `Arc` (`classic-082`, `classicplus-047`, whose faces' params differ).
  The shared function reads `param` off the running face (`ctx`), not the
  adapter's `Site`, so one adapter serves both. Fixture
  `crates/luau/tests/fixtures/gap_shared_faces.luau` (separate face tables,
  one `death`) and `gap14_one_function_across_both_faces_shares_its_adapter`.
  A runtime-face lookup in the adapter was rejected: with `radiant = "base"`
  there is no radiant table to look in.
- **Gap 15 — `.luaurc`'s alias.** `crates/cards/src/scripts/.luaurc` maps
  `jackioh` to `"../../luau/jackioh"`, no extension: luau-analyze 0.740
  refuses the extension as `Unknown require` under `--mode=strict`, which had
  silenced every batch's strict analysis (777 cascading errors). The CLI
  resolves the extensionless alias, so strict runs check the files in place.
  `crates/luau/README.md` and `PORTING.md` no longer say the CLI resolves no
  aliases.

Strict analysis (Luau 0.740 release binary, fetched for the run) over this
part's three card files is clean — and it caught three generator bugs on the
way there, all fixed in `crates/luau/tests/export_api.rs` with `jackioh.luau`
regenerated:

- a swallowed newline that commented out `export type ConditionContext`;
- `GameCounters`, `AuraArgs`, `AuraFilter` and `AuraApplies` named but never
  registered (`Unknown type`);
- `PlayerModifierSpec` emitted as `{ expiry } & (arms)`, a shape strict checks
  no literal against: the emitter now distributes the base table's fields into
  each table variant (falling back to the intersection when a variant is not a
  table — the file's only intersection).

## 2. The three ports

Each has a new `.luau`; its `.rs` lost `script()` and keeps `ID` and its
tests unchanged.

| Card | Hooks | Notes |
|---|---|---|
| `classic-026` Rapid Draw | `cry` (`draw`, `for_each_card` with `each_cards` / `each_card`, `discard_random`); `radiant = "base"` | The WIP named the sub-hooks `whole_hand` / `each_discard`, which are not in `vm::SUB_HOOKS`: `load_card` refused the card and `register_all()` panicked every card test in the crate. Renamed to `each_cards` / `each_card` (#442, L27); an empty hand answers `{}` (gap 1) |
| `classic-030` Recycle | `cry` (two `for_each_card`s with `data`: shuffle, then the Radiant discount); two face tables | The graveyard's ids are read once in the Cry and carried in `data` (gap 2); the discount half matches them in the library |
| `classic-035` Prep | `cry` (`add_player_modifier`); one shared local, two face tables | `expiry = { ["until"] = "thisTurn", turn = ctx:turn().number }` (gap 4); the discount is `param("discount")` off the running face |

No port stayed Rust.

## 3. Docs for the batches

- `docs/v0.4.0/api-gaps.md`: the `with_kill_credit` and
  `left_field_since_resolved` entries now bind, and a new section gives each
  of the 15 gaps its Luau form with one line of example.
- `docs/v0.4.0/PORTING.md` §5: the first stay-Rust reason no longer holds for
  the 15 fixed gaps.
- `crates/luau/README.md`: the `.luaurc` paragraph corrected.

## 4. Checks

- `cargo test -p jackioh-cards --features jackioh-engine/testkit` (full crate):
  PENDING.
- `cargo test -p jackioh-luau` (gap fixtures, export regen): PENDING.
- `cargo jackioh luau diff --parent-bin <base> --cards classic-026,classic-030,classic-035`: PENDING.
- `cargo clippy -p jackioh-luau -p jackioh-cards -p jackioh-tools --all-targets -- -D warnings`: PENDING.
- `cargo fmt --check`: clean. `cargo jackioh spec check`: PENDING.
- `luau-analyze --mode=strict` over the three `.luau` files: clean.
- `golden check`, `fuzz --seeds 200`, `luau bench`: PENDING (bench covers the
  generic aura path; no aura card ports here).
