<!--
Title: docs/issues-and-patches.md, Titles. `Patch v0.3.X: <what it does>`, `Patch v0.3.Y: …` (micro),
`Patch vX.Y.Z (part n of m): …`, `vX.Y.0: …`, `Night bot: …`, `CI: …` or `Architecture: …`.
It becomes the squash commit's subject, and the `pr title` check holds it to the convention.
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
- Patch version: <!-- the pending fragment's version (the next number, docs/issues-and-patches.md, Version numbers), `Y` for a micro patch, or "none" (no card data change) -->

## Gates

<!-- Tick what you ran; strike through what does not apply. CLAUDE.md, Commands, has the full list. -->

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace --features jackioh-engine/testkit,jackioh-engine/ts` (commit any diff it writes to `apps/web/src/wire/`)
- [ ] `cargo jackioh spec check` (a ruling added or changed: `cargo jackioh spec index` first)
- [ ] `cargo jackioh catalog check` and `cargo jackioh patches check` (card data changed)
- [ ] `pnpm --dir apps/web typecheck`, `pnpm --dir apps/web test` and `pnpm exec eslint apps/web e2e` (the client or the e2e specs changed)
- [ ] `cd bot && python3 -m unittest discover -s tests -t .` (`bot/`, `.harness/`, `.squishy/` or the bots' workflows changed)
