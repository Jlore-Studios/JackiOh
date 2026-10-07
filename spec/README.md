# JackiOh — Master Game Specification

2026-09-16, revision 5 of 2026-09-17, patch v0.2.0 of 2026-09-30 · @Someone

This directory is the specification, the only source of rules, cards and engine design. It was `SPEC.md`, split into notes in v0.3.0 (#306, #133) with every section and ruling id unchanged: `SPEC §6.3` is [[§6.3]] in `06-keywords.md`, and `R113` is [[R113]] in `rulings/R0113.md`. If a doc disagrees with these notes, the notes win and the doc is the bug. Open the directory in Obsidian as a vault, or read it on GitHub.

## The map

One note per top-level section, its subsections inside:

- [[§1]] [01-overview.md](01-overview.md): the game in one page, the design pillars, how to read the spec, its sources.
- [[§2]] [02-core-systems.md](02-core-systems.md): setup, the turn loop, mana, drawing and fatigue, ending the game, deckbuilding.
- [[§3]] [03-zones.md](03-zones.md): lanes and adjacency, the zones and their rules.
- [[§4]] [04-combat.md](04-combat.md): positions and exertion, declaring an attack, combat, one damage instance, deaths and the state check.
- [[§5]] [05-card-types.md](05-card-types.md): card anatomy, the card types, Radiant, the catalog's data decisions.
- [[§6]] [06-keywords.md](06-keywords.md): the keyword glossary: unit keywords, triggers and timing words, actions and verbs.
- [[§7]] [07-tokens.md](07-tokens.md): the tokens.
- [[§8]] [08-catalog.md](08-catalog.md): every card of Core, Classic and Classic+.
- [[§9]] [09-architecture.md](09-architecture.md): trust, topology, accounts, matchmaking, abuse, the practice AI, the tutorial, card statistics, the ranked ladder.
- [[§10]] [10-engine-guide.md](10-engine-guide.md): the engine: state, actions, events, layers, playing a card, prompts, randomness, the view, card scripts, the client's rendering and audio.
- [[§11]] [11-rulings.md](11-rulings.md): where the rulings came from; the rulings themselves are the notes in [rulings/](rulings/).
- [INDEX.md](INDEX.md): every ruling on one line: its id, title, note and the tests that prove it.

## Note format

A section note is the section's text as SPEC.md had it, starting at its `## N.` heading, with its `### N.M` subsections inside. Those headings are the section ids: `[[§10.3]]` links the `### 10.3` heading, `[[§8]]` the `## 8.` one.

A ruling note is `rulings/R<nnnn>.md`, the number four digits and zero-padded (`R0195.md` is R195), and holds one ruling:

```markdown
---
id: R195
title: When a card glows yellow (`conditionActive`)
cards: "§10.8, §10.9, #10, #53, #68, #71, #93, C #22, C #36, C #40"
proven_in:
  - crates/engine/tests/rules/condition_active.rs
  - crates/cards/tests/cross/condition_active.rs
---
The ruling, with every ruling it names written as a link and every section as a link.
```

- `id` is the ruling's id and matches the file name; `title` is the question it answers; `cards` lists the cards, sections and rulings it affects, as plain text.
- `proven_in` lists the files whose tests prove it. A Rust file proves R195 with a `fn` or `mod` whose name has the token `r195` between underscores (`fn r195_glows_…`, `fn r90_r81_…`); a TypeScript test under `apps/web/` or `e2e/` with an `it(` or `describe(` whose title has `R195` as a word; an SQL evidence script with a `\echo '### R195: … ###'` heading.
- Links are written `[[R113]]` for a ruling and `[[§10.3]]` for a section. Every other number in the text is plain.

## Adding a ruling

When a decision the spec does not make is needed (CLAUDE.md rule 3), follow Hearthstone semantics and, in the same change:

1. Write `rulings/R<nnnn>.md` with the next free number (the last row of [INDEX.md](INDEX.md) plus one): its front matter as above and the ruling as its text. Numbers are never reused or renumbered.
2. Name the proving test after it, `fn r<n>_…` in Rust or `it("R<n> …")` in TypeScript, and list its file in `proven_in`.
3. Say in [11-rulings.md](11-rulings.md) where the ruling came from, and edit the sections it changes, linking it.
4. Run `cargo jackioh spec index` to rewrite [INDEX.md](INDEX.md), then `cargo jackioh spec check`.

`cargo jackioh spec check` fails when a note's `id` does not match its file name or repeats another's; when a `proven_in` path is missing or holds no test named after the ruling; when an `r<n>` test name or an `R<n>` cited anywhere in `crates/`, `apps/web/src/` or `e2e/` (comments included) has no note; and when a `[[…]]` link resolves to no note or section. It reads ids, file names and test names, never the wording, so a reworded sentence fails nothing (#133).
