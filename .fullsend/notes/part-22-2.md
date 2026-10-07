# Slice: part 22, chunk 2 of 3: `patches.rs` and `catalog.rs` (the jackioh CLI)
BUILDS-RUN: 0

## FILES
- `crates/tools/src/patches.rs`: ports, in TS order, `packages/cards/scripts/naming.ts` (inline
  `pub mod naming`), `patch.ts` (`pub mod patch`: the version sites), `patches-io.ts`
  (`pub mod patches_io`), `versions.ts` (`pub mod versions`) and `patches.ts` (the module's own body:
  `patch_paths`, `write_fragment`, `check_patches`, `adding_commit`, `utc_date_of`, `ship_patches`,
  `parse_fragment_args`, `main`, then `Args` + `run`), plus `pub(crate) mod js` (below). Tests at the
  bottom: `test/versions.test.ts`, `test/patches-ship.test.ts` (incl. the throwaway-git-repo
  promotion tests), `test/patches.test.ts` (R388, R743, R375 against the real
  `crates/cards/patches/`), and a few naming tests from naming.ts's own comment examples.
- `crates/tools/src/catalog.rs`: ports `packages/cards/scripts/validate-catalog.ts` (`catalog check`,
  `check_catalog(&Object) -> CatalogReport`), `scripts/catalog-version.mjs` (`catalog-version`,
  `newest_version(&Path)`), and writes `catalog loc <path>` (`count_loc`, SURFACE §7.5). Unit tests:
  the shipped catalog passes, `id_for_index`, broken-entry fixtures, `count_loc`, catalog-version.

## SURFACE
- §12 matched: `catalog::{Args, run, VersionArgs, run_version}`, `patches::{Args, run}` as part 1's
  `main.rs` calls them. `catalog` has clap subcommands `check` and `loc <path>`; `catalog-version`
  takes an optional patch-list path (as the .mjs did). `patches` takes its raw argument list
  (`trailing_var_arg`, `allow_hyphen_values`) and keeps `patches.ts`'s own parser, so
  `patches <version> [date] "<title>" [--source …] [--notes …] [--cards …]`, `patches check` and
  `patches ship` read exactly as before (the same choice part 1 made for the server's subcommands).
- Files and formats unchanged, now under `crates/cards/patches/` and `crates/cards/catalog.json`
  (`CATALOG_REL`, `PATCHES_REL`). `write_json` writes `JSON.stringify(v, null, 2)` + `\n` through a
  `<path>.<pid>.tmp` rename, as TS did.

## DEPENDS-ON
Only part 1's frozen code: `jackioh_engine::wire::{CardType, Tag, Rarity, PrintedRarity, SetName,
KEYWORD_KINDS, param_placeholders}` (with `ALL`/`as_str`), and in tests
`jackioh_cards::{catalog_json, catalog_version, CATALOG_VERSION}`. No other part's names.

## GAPS
- Nothing left out: every TS function is ported. Two TS tests changed with the rewrite (see
  Decisions): R388's "bumps the version everywhere" (two Rust sites, the TS-only sites gone) and
  R388's Render start-command test (now: `catalog-version` equals `CATALOG_VERSION`, render.yaml has
  no `startCommand`, an empty list fails naming "names no newest version").
- For part 30 (person: `.github/`): `patches-ship.yml` must call `cargo jackioh patches ship`, and
  CI's catalog step must run both `cargo jackioh catalog check` and `cargo jackioh patches check`
  (TS's `pnpm validate:catalog` ran validate-catalog then `patches check`; SURFACE §12 lists them as
  two commands, so `catalog check` does not run the fragment proof).
- For part 37 / docs: CLAUDE.md, `docs/ADDING_CARDS.md` and `docs/issues-and-patches.md` still name
  `pnpm --filter @jackioh/cards run patches …` and `validate:catalog`.
- `apps/server/.env.example` still carries `CATALOG_VERSION` but is no longer a version site (part 37
  deletes it). If part 37 keeps it, add it to `patch::VERSION_SITES`.
- The ship tests and `ship_patches` need `git` on PATH (TS's did too); CI images have it.
- Part 31: `patches.rs` carries `#![allow(dead_code)]` for the ported-but-uncalled functions
  (`naming`, `order_entry`, `rewrite_index`, `versions_at_sites` outside tests). clippy may still
  ask for small style changes (long lines; rustfmt has not run).

## Decisions
- **Ordered JSON.** serde_json has no `preserve_order` (SURFACE §2), and the patch history compares
  entries byte for byte, key order included (`differing_ids`, `same_catalog`, R646's "key order
  included" test), and reports fields in key order (`changed_fields`). So `patches::js::Json` is a
  small order-keeping JSON value (`Object = IndexMap<String, Json>`, numbers `f64` as JS's) with its
  own serde `Deserialize`/`Serialize`; `js::stringify`/`stringify_pretty` are `JSON.stringify`
  (serde_json's writer over it; integral doubles print without a fraction). `Catalog =
  IndexMap<String, Object>`. `catalog.rs` uses the same type (both files are this chunk's).
- **Typed records.** `PatchEntry`, `PatchChange`, `PendingFragment`, `ShippedEntry` are serde
  structs whose field order is `patches.json`'s, so a rewrite is byte-identical. `PatchChange` is a
  struct with `kind: ChangeKind` and `fields: Option<Vec<String>>`, not a tagged enum (serde writes an
  internal tag first, which would reorder every change line). Unknown extra keys in those files are
  dropped on rewrite (TS spread kept them; none exist).
- **Version sites after the rewrite:** `crates/server/.env.example` (`^CATALOG_VERSION=.*$`, m),
  `render.yaml` (`(- key: CATALOG_VERSION\n\s+value: )\S+`) and, optional, `apps/web/.env.production`
  (`^VITE_CATALOG_VERSION=.*$`, m; skipped when absent). `catalog-data.ts` and the TS server's e2e
  default are gone (the version is compiled in, SURFACE §11.3). Regexes are matched by hand (no regex
  crate in the tools), including JS's `\s` set and line terminators.
- **SHA-1** for `git_blob_hash` is hand-written (FIPS 180-4); the tools take no hash crate. Pinned
  by TS's two vectors.
- **Errors:** tooling errors are `anyhow` with TS's message text verbatim (the tests' regexes match
  them). `patches check` failures print `patches check: <problem>` per line (exit 1); any other error
  prints `patches: <message>`, as TS's `main` did. `naming::slug_prefix_of` panics with TS's message
  (TS threw; callers pass catalog ids only).
- **TS constants that were paths** (`PATCHES_DIR`, `PATCHES_JSON`, `INDEX_JSON`, `PENDING_DIR`,
  `SHIPPED_JSON`, `REPO_ROOT`) are functions (`patches_dir()` …, `repo_root()` from
  `CARGO_MANIFEST_DIR`). Functions that defaulted `dir`/`repoRoot` take it explicitly.
- **Regex constants kept as text** for the record where TS exported one (`FRAGMENT_VERSION`), with a
  matcher function (`is_fragment_version`), as part 1 did for `PARAM_PLACEHOLDER`.
- `validate-catalog.ts`'s union lists come from the engine's `ALL` (exhaustive by construction,
  replacing TS's `Exhaustive` aliases); `EXPECTED_TAG_COUNTS` is an exhaustive `match` over `Tag`.
  TS's duplicate total-count check (before and after the loop) is kept, so a wrong total fails twice.
- `catalog loc` counts a Rust script's lines like `gen-loc.ts`'s `countLoc`: comments (nested block
  comments) removed, strings/raw strings/char literals read as such, `use` declarations (incl.
  `pub use`, multi-line) skipped, stopping at the first `#[cfg(test)]` line. It only prints.
- `naming.ts` is ported as the TS file-name convention it is (`001-big-d-fender.ts`); the Rust card
  files follow SURFACE §4.1 and `build.rs`'s `ID` lookup instead, so nothing in Rust calls it yet.
- Test temp dirs: `TempDir` under `std::env::temp_dir()` (pid + counter), removed on drop (no
  tempfile crate). Fixture repos write `crates/cards/...` paths and the Rust version sites.
