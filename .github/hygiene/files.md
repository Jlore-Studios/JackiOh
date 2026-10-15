The weekly hygiene run ([this week's run](<!-- run -->)) asks for one pull request that removes dead files
and archives nothing reads, and proves every removal. A person reviews and merges it (`person merges`), so
make each removal easy to check. The agent instructions have their own issue this week: leave
`CLAUDE.md`, `AGENTS.md` and the skills to it, except to drop a reference to a file you delete. Leave the
branches at the end to a person.

## 1. Dead files

Find tracked files that nothing references or runs, and delete them. A file is dead only when every
reader is ruled out: grep its path and its name across the tracked files, then check that no
workflow, script, `build.rs`, `include_str!`, `import.meta.glob`, test glob, fixture macro or
`spec/rulings/*` `proven_in` reads it. The last audit (#832) found these that look dead but are not:
- `bot/prompts/*.md`, loaded by name;
- Luau fixtures, loaded through a macro;
- `crates/server/tests/sql/*.sql`, run by a glob;
- the patch JSON, read through `import.meta.glob`;
- the generated wire types, checked by CI;
- `docs/radiant-audit.md` and `assets/music/LICENSES.md`, read by tests;
- the training history files;
- `bot/tests/squishy_cases.py`, run as a subprocess;
- `e2e/`'s own lockfile;
- the font licences.

Code that only its own test imports is dead with its test.

## 2. Archives

Delete records that nothing live needs: finished plans, copies of issue bodies, and old reports of
milestones that have shipped. Git history keeps them, and an agent cannot read what is not in the tree.
Keep the following:
- the review of a milestone whose gate is still open;
- anything a workflow, test or code comment cites (fix or drop the citation instead);
- `spec/`, `BUILD.md` and `REVIEW.md`;
- the designer's source notes.

## Never

- Never change `.github/`, `bot/`, `.harness/`, `.squishy/` or `.claude/`. Name what should go there in the
  pull request, for a person.
- Never delete a file to quiet a check, or weaken a check.

## Done when

The pull request body has a table of every deleted file: its path, its size, and the evidence that nothing
reads it (the commands you ran and what they printed). The fast checks pass: `cargo fmt --check`,
`cargo jackioh spec check`, the web's typecheck and eslint, the bot selftest, and the unit tests of any
package you touched. Nothing to remove? Make no change and say so in your answer, with your evidence.

## Branches (for a person, not the builder)

<!-- branches -->

---
_Opened by `.github/workflows/hygiene.yml`._
