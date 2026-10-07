# Engine suspects (Wave 3)

A failure in another part's crate that traces to `crates/engine`, one entry each, appended (pull first): the failing test or command, what the engine does, what the TS does (file:line), and the part that found it.

- **part 34, `cargo jackioh spec check`** (`crates/engine/tests/rules/rulings_a.rs:113: R301 has no note in spec/rulings/`). Not a rule: the doc comment on `ra_unit` reads "a Core unit fixture whose index is the next of this file's (R301 on)", and spec check's rule 3 reads every `R<n>` cited under `crates/` (comments included) as a ruling. There is no R301 (§11 skips 295–309). The TS has no such citation: `packages/engine/test/rulings-a.test.ts:88` is `let nextIndex = 300;` with no comment. Fix (part 32's file): write the fixture index without the `R`, e.g. "(index 301 on)". One of the four problems that keep `spec check` red.
