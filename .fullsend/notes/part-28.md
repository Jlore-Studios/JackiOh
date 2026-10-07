# Slice: part 28, the spec graph and structural spec checks (#133)
BUILDS-RUN: 0 (no cargo, rustc, rustfmt, clippy, tsc, pnpm or vitest. Scratch Node scripts only: the converter, a JS mirror of `spec check` and of `spec index`, and main's `glossary.ts` loaded with Node's type stripping to print the glossary text, in place of the allowed vitest run)

## FILES
spec/README.md — the map, the note format, how to add a ruling
spec/01-overview.md … spec/10-engine-guide.md — SPEC.md's sections, verbatim, links `[[R<n>]]`/`[[§N.M]]` added (none in code)
spec/11-rulings.md — §11's provenance paragraphs, verbatim, plus one closing paragraph pointing at rulings/ and INDEX.md
spec/rulings/R0001.md … R0764.md — 563 notes, one per §11 row (R1–R764; R763 and R764 landed after the plan said 561)
spec/INDEX.md — what `spec index` writes (rendered by a JS mirror of `render_index`; byte-identical by construction)
spec/06-keywords.md — §6.3's "Degrade / Upgrade" row is a Degrade row and an Upgrade row
SPEC.md — a 15-line pointer
CLAUDE.md — rule 3 only
crates/tools/src/spec.rs — `spec check`, `spec index`, unit tests
crates/engine/tests/rules/rulings_config.rs — 29 `fn r<n>_…` config asserts
apps/web/src/cards/glossary.ts — rule text as reviewed data; helpers deleted; `row?` link
apps/web/src/cards/rules.test.ts — structural check against spec/06's first columns
apps/web/src/audio/spec-rows.test.ts — deleted

## SURFACE
`jackioh spec [--root <dir>] check|index` (`spec::Args { root: Option<PathBuf>, command: SpecCommand }`, `pub enum SpecCommand { Check, Index }`, `pub fn run(args: Args) -> anyhow::Result<()>`); check prints one line per problem and returns Err (exit 1).
Ruling note front matter: `id`, `title` (plain, or double-quoted where YAML needs it), `cards` (always double-quoted), `proven_in` (list).
`GlossaryEntry.row?: string`: the spec row's name where it differs from `id` ("Armor X" for Armor; 11 entries).

## DEPENDS-ON
`jackioh_engine::config::{CRY_ON_PLAY_ONLY, TURN_CAP_PLAYER_TURNS, HAND_CAP, LANE_RESTRICTED_ATTACKS, MULLIGAN_ORDER, ROTATION_RING, RANDOM_KEYWORD_POOL, FIB, fib, GENN_GREED_EXILES, CALL_TO_CHAOS_CHAIN_CAP, DRAW_OFFERS_PER_TURN, DRAW_OFFER_BLOCK_TURNS, FIENDER_STATS_MODE, ANTI_ONESHOT_CAP{.base,.radiant}, CAST_ON_DRAW_CHAIN_CAP, HERO_HEALTH, FUSE_COST_CAP, LIBRARY_CAP, AI_END_TURN_PROBABILITY, HUMAN_HANDICAP, DECK_SIZE, MAX_MANA, DIFFICULTIES, AI_DIFFICULTY, DRAWS_PER_TURN, AI_TUTORIAL, MIN_CHOSEN_X, RADIANT_FALLBACK_FACTOR}` (part 1's config.rs, not on staging when written). Structs, enums and string switches are compared through `serde_json::to_value`, i.e. they must serialise to the TS JSON (as `export_config` needs anyway).
`spec check` rule 2 reads 829 Rust proof paths that parts 9–27 write (mapped from the TS index through port-map.tsv), 11 SQL scripts part 20 copies into crates/server/tests/sql/, and apps/web/src/practice/personas.test.ts (part 21).
Every Rust test that proves a ruling must keep it as a `_`-delimited `r<n>` token in its fn or mod name (SURFACE §7.3).

## GAPS
- Both Rust files are unbuilt and unformatted (rustfmt not run; max_width 110).
- rulings_config.rs guesses config.rs's shapes (above). Fix the accessors in Wave 3 if part 1 chose others.
- Dropped with the index, not ported anywhere (the engine crate cannot see the server's config): the `serverConstant` asserts of R104, R106, R107, R108, R109, R137, R250, R252, R255, R256, R263, R330, R333, R334, R339 and R389's MATCH_CEILING_MINUTES, and the 23 source-text pins (R104, R105, R106, R110, R137, R141, R142, R145–R149). Their rows stay proved by the server tests named in `proven_in`. A server config test asserting those constants would restore the numbers (part 18 or 27).
- R107, R108, R109, R137, R141, R142 and R145–R149 had no `provenIn` in the index (source pins only). Their `proven_in` lists every test file whose `it`/`describe` title names them. R108 is proven only by queue.test.ts's two `describe`s that cite it mid-title (R166, R167), and R142 by 03_match_lifecycle.sql's `### R110: … (R142) ###` heading.
- `spec check` goes beyond SURFACE §15 in two places: an SQL heading `\echo '### R<n>: … ###'` counts as a proof (11 scripts), and a note with an empty `proven_in` fails.
- The Degrade/Upgrade split: the Upgrade row's engine cell says "As Degrade: the two verbs share the Degrade row's semantics, each in its own direction (R386)", so the 2,000-character cell is not duplicated. Those words are new; the rule words are unchanged.
- Out of my table and now stale (left alone): apps/web/src/wording.test.ts's header and CONVERTED set name `inPlayerWords`; CLAUDE.md's Commands list `pnpm rulings:coverage` and the intro still calls SPEC.md the spec; BUILD.md, REVIEW.md, docs/ADDING_CARDS.md cite rulings.test.ts and SPEC §11's table; scripts/worktree.sh names "SPEC.md §11's numbering".
- On staging the TS gates that read SPEC.md's table now fail (rulings.test.ts's completeness test, rulings-coverage.ts, conditionActive.test.ts's R195 prose block): parts 25 and 37 delete or port them.
- apps/web/src/audio/music-assets.test.ts reads assets/music/LICENSES.md for track ids plus "FluidR3" and "MIT": left (the licence names are the contract). rules.test.ts reads spec/06-keywords.md's first column and spec/05's `###` headings: structural, like spec.rs; the done-when grep misses it because the path is a constant.
- Obsidian resolves `[[R113]]` by file name only, so its graph shows the ruling links as unresolved (the files are R0113.md). SURFACE fixes both forms, so I added no `aliases`.
