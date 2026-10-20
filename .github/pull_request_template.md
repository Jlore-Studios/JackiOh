<!--
Title: docs/issues-and-patches.md, Titles. `Patch: <what it does>`, or `Patch (part n of m): …` for a
part. No version: `patches ship` names a card change's version when it ships and retitles this pull
request. It becomes the squash commit's subject, and the `pr title` check holds it to the convention.
-->

Closes #
<!-- A part of a multi-part patch: `Closes #<part>` and `Part of #<tracker>`, never `Closes #<tracker>`. -->

## What changed

<!-- What and why, by area. Name the files a reviewer should read first. -->

## Tests run

<!-- The commands you ran and what they printed. -->

## Risks

<!-- What could break, for players or for the bots, and what you did about it. "None" is an answer. -->

## Rulings and patch version

- R-rows added or changed: <!-- `spec/rulings/R<nnnn>.md` (the next free number, CLAUDE.md rule 3), or "none" -->
- Fragment: <!-- `pending/<slug>.json` and what it claims (cards, `--plays`, `--rule`), or "none" (no card plays differently). The classifier's labels (`kind:*`, `tier:*`) say what it does; its version is named when it ships. -->

## Gates

<!-- Tick what you ran; strike through what does not apply. CLAUDE.md, Commands, has the full list. -->

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace --features jackioh-engine/testkit,jackioh-engine/ts` (commit any diff it writes to `apps/web/src/wire/`)
- [ ] `cargo jackioh spec check` (a ruling added or changed: `cargo jackioh spec index` first)
- [ ] `cargo jackioh catalog check` and `cargo jackioh patches check` (card data changed)
- [ ] `cargo jackioh classify --base origin/main --claims` (a card script, the engine or the AI changed: `--parent-bin` with a build of `origin/main` when it asks)
- [ ] `pnpm --dir apps/web typecheck`, `pnpm --dir apps/web test` and `pnpm exec eslint apps/web e2e` (the client or the e2e specs changed)
- [ ] `cd bot && python3 -m unittest discover -s tests -t .` (`bot/`, `.harness/`, `.squishy/` or the bots' workflows changed)
