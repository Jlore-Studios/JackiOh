# `jackioh-tools`

The `jackioh` binary: every development and CI tool that is not the server. Run it as
`cargo jackioh <command>` (`.cargo/config.toml`'s alias for `cargo run --release -p jackioh-tools --`),
or build it once with `cargo build --release -p jackioh-tools` and run `target/release/jackioh`, as
CI does. It reads and writes files under the repository it was built from (`--root` on `spec`
overrides that).

Exit status: 0 on success, 1 on a failed check, 2 on bad usage. Output goes to stdout; nothing is
interactive. Tooling may read the clock and the disk; the crates it drives stay pure (CLAUDE.md
rule 4), and it hands them the clock as a callback where they need one.

| Command | Module | Does |
| --- | --- | --- |
| `fuzz [--from N] [--seeds N] [--handicap]` | `fuzz.rs` | random-policy games (spec §10.7) from every non-token card of every set, the I1–I4 invariants checked each step and every game folded back to its live hash; seeds 1–1,000 by default, `--handicap` rotates one seat through Medium and Hard. CI plays `--seeds 200`, the daily run 10,000 of each |
| `replay` | `replay.rs` | reads `{seed, decks, log, handicaps?, dealt?, lastBoards?, glitchBoards?}` on stdin and prints `{"hash", "errors"}`: e2e specs 01 and 13 check the browser's hash with it |
| `trace [matchup] [n] [full]` | `trace.rs` | one AI gate game, turn by turn |
| `catalog check` | `catalog.rs` | `crates/cards/catalog.json`'s shape and census, set by set (spec §5, §6.1, §7, §8): 370 cards and 80 tokens, the rarity counts, the tag counts, the patch v0.2.0 fields |
| `catalog loc <script path>` | `catalog.rs` | a new card script's `loc` (its non-blank, non-comment, non-`use` lines above `#[cfg(test)]`); prints, writes nothing |
| `catalog-version` | `catalog.rs` | the newest version in `crates/cards/patches/patches.json` |
| `patches <version> [date] "<title>" [--source …] [--notes …] [--cards …]` | `patches.rs` | claim a card patch as `crates/cards/patches/pending/<version>.json` (R388, R646) |
| `patches check`, `patches ship` | `patches.rs` | every catalog change claimed by exactly one fragment; after a merge, promote the fragments in ship order and bump the catalog version |
| `gate [--full] [--shard k/K] [--out dir]`, `gate merge [dir]` | `gate.rs` | the AI's three quality gates and its perf gate (spec §9.9, `crates/ai/README.md`) |
| `sweep [ids…] [--json] [--pass2 files] [--report files]` | `sweep.rs` | R186's two-pass shadow-ban sweep; prints the `SHADOW_BAN` and `SHADOW_WATCH` rows to copy into `crates/ai/src/shadow_ban.rs` |
| `stats [--games N] [--from N] [--patch v] [--series s] [--out file]` | `stats.rs` | R378's development run for the card statistics |
| `stats report <file>… [--source live\|dev\|all] [--mode m] [--patch v] [--pilot p] [--card id] [--json]` | `stats.rs` | R377's card win rates over record files (one `GameRecord` per line: `stats --out`, `jackioh-server stats-export`, the training lanes' `~/training-out/<lane>/*.jsonl`) with no database; `stats-cards`' flags and defaults (live games unless `--source` says otherwise), its table or its `--json`; a line that is not a record is an error naming its file and line. The figures `analysis/` computes over the same files are held to these |
| `golden check`, `golden bless` | `golden.rs` | replay `crates/engine/tests/golden/games.jsonl` (the traces recorded from the TypeScript engine) and the hotseat fixture; `bless` rewrites them, only after an intended rules change |
| `spec check`, `spec index` | `spec.rs` | the spec graph's structural checks (CLAUDE.md rule 3), and `spec/INDEX.md` rewritten from the ruling notes |
| `arena --a … --b … --games N --seed s [--out dir]` | `arena.rs` | two agents (`self`, `random`, `bin:<path>`) play N games; one game record per line |
| `agent` | `agent.rs` | this build's AI over JSON lines on stdin and stdout (`info`, `decide`, `quit`) |
| `promote --lane improve\|unban --parent-bin <path> [--dry-run] [--verify]` | `promote.rs` | a training lane's promotion gate (`training/README.md`) |

```
cargo jackioh --help
cargo jackioh catalog check && cargo jackioh patches check && cargo jackioh spec check
cargo jackioh fuzz --seeds 200
cargo jackioh fuzz --from 700 --seeds 1        # replay the one seed a failure names
cargo jackioh golden check
cargo test -p jackioh-tools                    # the tools' own tests
```

`spec check` holds four rules: every ruling note's `id` matches its file name and is unique; every
`proven_in` path exists and holds a proof (a Rust `fn` or `mod` whose name has the `_`-delimited token
`r<n>`, a TypeScript `it(`, `test(` or `describe(` whose title has `R<n>` as a word, or an SQL
script's `\echo '### R<n>: … ###'` heading); every `r<n>` test token and every `R<n>` cited in
`crates/`, `apps/web/src/` and `e2e/` has a note; every `[[…]]` link in `spec/` resolves. It reads
ids, file names and test names, never wording.
