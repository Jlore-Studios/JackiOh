# JackiOh — Master Game Specification

The specification now lives in [`spec/`](spec/README.md), as one note per section and one per ruling (v0.3.0, #306, #133). This file is a pointer, so that links to it still land somewhere.

Every id is unchanged, so every citation of the old file still holds:

- `SPEC §N` and `SPEC §N.M` are the `## N.` and `### N.M` headings of `spec/NN-<slug>.md`: §2 is [`spec/02-core-systems.md`](spec/02-core-systems.md), §6.3 is in [`spec/06-keywords.md`](spec/06-keywords.md), §10.8 in [`spec/10-engine-guide.md`](spec/10-engine-guide.md). [`spec/README.md`](spec/README.md) maps every section to its note.
- `R<n>`, a row of the old §11 table, is the note `spec/rulings/R<nnnn>.md`, four digits and zero-padded: R113 is [`spec/rulings/R0113.md`](spec/rulings/R0113.md). Its front matter names the tests that prove it (`proven_in`).
- §11's prose, where each block of rulings came from, is [`spec/11-rulings.md`](spec/11-rulings.md).

To find a ruling, look its id up in [`spec/INDEX.md`](spec/INDEX.md) (id, title, note and proving tests, one line each), or open `spec/rulings/R<nnnn>.md` directly. To find the rulings about a topic, search the notes: `grep -l "Cry" spec/rulings/*.md`.

To add a ruling, follow [`spec/README.md`](spec/README.md#adding-a-ruling) (CLAUDE.md rule 3): a new note with the next free number, a test named after it, then `cargo jackioh spec index` and `cargo jackioh spec check`.

The rules of precedence are unchanged: the spec is the only source of rules, cards and engine design, and a doc that disagrees with it is the bug.
