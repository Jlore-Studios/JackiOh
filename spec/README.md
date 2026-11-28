# JackiOh — Master Game Specification

2026-09-16, revision 5 of 2026-09-17, patch v0.2.0 of 2026-09-30 · @Someone

This directory is the specification, the only source of rules, cards and engine design. It was `SPEC.md`, split into notes in v0.3.0 (#306, #133) with every section and ruling id unchanged: `SPEC §6.3` is [[§6.3]] in `06-keywords/`, and `R113` is [[R113]] in `rulings/R0113.md`. If a doc disagrees with these notes, the notes win and the doc is the bug. Open the directory in Obsidian as a vault, or read it on GitHub.

## The map

One note per top-level section, its subsections inside. A section that outgrew one read ([[§6]], [[§8]], [[§9]], [[§10]]) keeps its opening in its note and has one note a subsection in the folder of the same name, each named for its subsection's id: `08-catalog/8.10-stinkyscape.md` is [[§8.10]], `ls spec/08-catalog` lists the folder and `spec/*/10.8-*.md` finds a subsection by its id.

- [[§1]] [01-overview.md](01-overview.md): the game in one page, the design pillars, how to read the spec, its sources, and the rules at a glance ([[§1.1]]), each line linking the section that holds it: read it first.
- [[§2]] [02-core-systems.md](02-core-systems.md): setup, the turn loop, mana, drawing and fatigue, ending the game, deckbuilding.
- [[§3]] [03-zones.md](03-zones.md): lanes and adjacency, the zones and their rules.
- [[§4]] [04-combat.md](04-combat.md): positions and exertion, declaring an attack, combat, one damage instance, deaths and the state check.
- [[§5]] [05-card-types.md](05-card-types.md): card anatomy, the card types, Radiant, the catalog's data decisions.
- [[§6]] [06-keywords.md](06-keywords.md) and [06-keywords/](06-keywords/): the keyword glossary: unit keywords, triggers and timing words, actions and verbs.
- [[§7]] [07-tokens.md](07-tokens.md): the tokens.
- [[§8]] [08-catalog.md](08-catalog.md) and [08-catalog/](08-catalog/), a note a set: every card of Core, Classic, Classic+, Meditative, Meditative+, Stinkyscape, Training Grounds, Stinkyscape+ and Felinors vs Humans ([[R1420]], [[R1178000]], [[R1857000]], [[R2194000]], [[R3535000]], [[R5222000]]), and the held single-player Infinite Assault set ([[§8.13]], [[R4577000]]; plan: `docs/infinite-assault.md`).
- [[§9]] [09-architecture.md](09-architecture.md) and [09-architecture/](09-architecture/): trust, topology, accounts and friends, matchmaking and Play together (challenges, watching a friend, shared replays, chat), abuse, the practice AI, the tutorial, card statistics, the ranked ladder.
- [[§10]] [10-engine-guide.md](10-engine-guide.md) and [10-engine-guide/](10-engine-guide/): the engine: state, actions, events, layers, playing a card, prompts, randomness, the view, card scripts, the client's rendering and audio.
- [[§11]] [11-rulings.md](11-rulings.md) and [history/](history/): where the rulings came from, the early ones in the note and each issue's since in `history/<issue>.md`. They restate the notes and decide nothing; the rulings themselves are the notes in [rulings/](rulings/).
- [[§13]] [13-duos.md](13-duos.md): Duos, two players against an AI team in two duels side by side: seats and pairing, setup, rounds, the shared life pool, passing, what each seat sees, the AI team, leaving, co-op rooms and pings.
- [[§14]] [14-economy.md](14-economy.md): the collection economy: Jlore, cosmetics, card packs and the slot roll, pity, the token bars and the cosmetic token cycle, cosmetic packs and the Chase Shard, set tracks and XP, earning, daily quests, dev accounts, the ledger, going live, card proficiency.
- [[§15]] [15-dungeon-run.md](15-dungeon-run.md): dungeon runs, the single-player mode against the AI: Sandbox and Campaign, seeds and the daily run, the starting deck, artifacts and blessings, the map, the fights, after a fight, the tiles, curses, the score, Embers and leaderboards, the extended campaign, what is built, the open questions.
- [[§16]] [16-infinite-assault.md](16-infinite-assault.md): the Infinite Assault, the daily single-player gauntlet of 20 levels against the AI: the day and its seed, the levels and the escalation, the upgrades, enemies, doctrines and Commanders, the player's deck, attempts, checkpoints and the record, the screens, the AI, what is built.
- [INDEX.md](INDEX.md): every ruling on one line: its id, title, note and what it touches (its `cards`), rewritten on `main` daily (a ruling merged since is in [rulings/](rulings/) only).
- [CARDS.md](CARDS.md): every card and token by the keywords it prints, by its tags, and by the named random pools it is in with the cards that draw from each, written from the catalog by `cargo jackioh spec index` and rewritten on `main` daily, so the notes state a list's rule and link it (`[[CARDS#jlockeed]]`) rather than list its cards (#3804).
- `pending/`, when a branch has merged since the last daily fold: the paragraphs waiting to be filed in [history/](history/), one file per issue ([Adding a ruling](#adding-a-ruling)).

## Looking things up

Read the entry a task touches and follow its links: never open a note to find an entry in it, and never search all of `spec/` for a word. No note is longer than one read returns (256 KB, which `spec check` holds) but the index, which is searched; the three largest sets' notes are over 150 KB all the same. A paragraph or a table row is one line, often 1 to 3 KB, and a fact is written in its section, in its ruling note and again in `history/`, so `grep -rn Reborn spec/` returns 208 KB where the titles of the 101 notes that mention it are 11 KB (measured 2026-11-24).

- **Which section a question belongs to**: [[§1.1]], the rules at a glance, links the section each rule is in.
- **A subsection**: of [[§6]], [[§8]], [[§9]] or [[§10]] it is a note, read whole: `spec/10-engine-guide/10.8-viewfor.md`. Of any other section, `grep -n '^#' spec/04-combat.md` gives each heading's line; read from that line to the next heading's.
- **A keyword, a verb, a token or a card**: each is one table row, and the row is the whole entry. `grep -rn '^| Dormant |' spec/06-keywords` is the keyword. A card is in its set's note by its number, `grep -n '^| 67 |' spec/08-catalog/8.3-core-51-80.md`, or in the folder by the name its row prints, `grep -rn '| Zoomerbin Oomen |' spec/08-catalog`. [CARDS.md](CARDS.md) lists the cards that print a keyword or a tag, or are in a named pool.
- **A ruling an entry links**: `[[R113]]` is `rulings/R0113.md`, short enough to read whole. Its front matter names what it touches (`cards`) and the tests that prove it (`proven_in`).
- **The past rulings on a point**, the search to make before a hard call and before a new ruling, since a point already ruled is not ruled again (#1243): list the notes that mention it by title, then open the few that bear on it.

  ```sh
  grep -rlF 'Reborn' spec/rulings | xargs grep -m1 '^title:'    # spec/rulings/R0008.md:title: Reborn and Death
  grep -rlw -- '§4.4' spec/rulings | xargs grep -m1 '^title:'   # a section, or a card as `cards` writes it: -wF 'C #22'
  ```

  [INDEX.md](INDEX.md) holds each title beside what the ruling touches, in one file and as of the last daily fold, so `grep -w '§4.4' spec/INDEX.md` is the rulings filed against a section; the notes are the whole list.
- **Why a ruling was made**: `history/<issue>.md` is where an issue's rulings came from, and a ruling numbered from its issue's block names the issue ([[R1878001]] is issue #1878's, `spec/history/1878.md`). For any other, `grep -rlw 'R177' spec/history spec/11-rulings.md spec/pending`. These restate the notes and decide nothing, so they are never where to look for a rule.

## Note format

A section note is the section's text as SPEC.md had it, starting at its `## N.` heading, with its `### N.M` subsections inside, or, for a section in a folder, each in a note of its own that starts at its `### N.M` heading. Those headings are the section ids: `[[§10.3]]` links the `### 10.3` heading, `[[§8]]` the `## 8.` one. A note is 256 KB at most, what one read returns: a section that would pass it becomes a folder, `NN-<slug>/N.M-<slug>.md` a subsection, its opening staying in `NN-<slug>.md`.

A ruling note is `rulings/R<nnnn>.md`, the number zero-padded to four digits at least (`R0195.md` is R195, `R690000.md` is R690000), and holds one ruling:

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

- `id` is the ruling's id and matches the file name; `title` is the question it answers; `cards` lists the cards, sections and rulings it affects, as plain text: a ruling as `R113`, a section as `§10.3`, a card as `#10` (Core) or after its set's name (`C #22`, `C+ #12.1`, `M #45`, `M+ #20`, `S #74`, or in full, `Meditative #40`), a bare `#<n>` after one being in the same set (`C+ #12.1–#12.8`), and an issue as `issue #781`, never a bare `#781`.
- `proven_in` lists the files whose tests prove it, and only a test that runs counts (#1718). A Rust file proves R195 with a test whose name has the token `r195` between underscores (`fn r195_glows_…`, `fn r90_r81_…`). That test is a `fn` marked `#[test]` or `#[tokio::test]` and not `#[ignore]`d except for a `daily: …` reason, a `fn` that a macro of the same file makes a test of (the store contract's `both_stores!`), or a `mod` holding such a test. A TypeScript test under `apps/web/`, `apps/edge/src/` or `e2e/` proves it with an `it(`, `test(` or `describe(` whose title has `R195` as a word, as long as no `.skip`, `.todo`, `.fixme` or `.only` turns it off, on itself or on a call around it. An SQL evidence script proves it with a `\echo '### R195: … ###'` heading.
- Links are written `[[R113]]` for a ruling and `[[§10.3]]` for a section. Every other number in the text is plain.

## Adding a ruling

When a decision the spec does not make is needed (CLAUDE.md rule 3), follow Hearthstone semantics and, in the same change:

1. Write `rulings/R<nnnn>.md` with the lowest number free in its issue's block: issue N owns R(N×1000) to R(N×1000+999), so issue #690's first ruling is R690000 ([[R690000]]; `docs/issues-and-patches.md`, Ruling numbers, covers sub-issues and clashes): its front matter as above and the ruling as its text. Numbers are never reused or renumbered.
2. Name the proving test after it, `fn r<n>_…` in Rust or `it("R<n> …")` in TypeScript, and list its file in `proven_in`.
3. Edit the sections it changes, linking it, and say where it came from in `pending/<n>.md`, `n` its issue's number (or its pull request's when it has none): one paragraph, `**[[R<n>]] <its title> (issue #<n>, <date>)**: …`, as the notes in [history/](history/) read. It says what asked for the ruling, which sections it edits and which numbers it takes, and leaves the ruling to its note: a paragraph that restates the note is the same text to keep true twice. One file holds every paragraph of its issue's branch.
4. Run `cargo jackioh spec check`. A branch leaves [history/](history/), [INDEX.md](INDEX.md) and [CARDS.md](CARDS.md) alone, since every ruling's branch writing them conflicted on nearly every merge (#1681's audit, finding 3): `.github/workflows/spec-fold.yml` runs `cargo jackioh spec fold` on `main` once a day, which files each pending paragraph under `history/<n>.md`, after what its issue has there already, deletes it from `pending/`, rewrites [INDEX.md](INDEX.md) and [CARDS.md](CARDS.md) (what `cargo jackioh spec index` does) and opens the result as a pull request that merges itself. Until then a new ruling is in [rulings/](rulings/) and not in the index yet. A paragraph already filed is edited where it stands.

`cargo jackioh spec check` fails when a note's `id` does not match its file name or repeats another's; when a `proven_in` path is missing or holds no test that runs named after the ruling; when an `r<n>` test name or an `R<n>` cited anywhere in `crates/`, `apps/web/src/`, `apps/edge/src/`, `apps/admin/src/` or `e2e/` (comments included) has no note; when a `[[…]]` link resolves to no note or section; when a note's `title` or `cards` is missing or empty, or its `cards` names a ruling, section or card that does not exist; when a file in `pending/` is not `<n>.md` or holds nothing; when a code file in those five trees still says its ruling is to be written (`proposed ruling` or `no R-row yet`, in any case; #1243); when a `(ruling)` or `Ruling:` marker in [[§2]] to [[§10]] links no ruling (`[[R<n>]]`) before its sentence ends (REVIEW A2.3, #2048); and when a note but the index is over 256 KB. It reads ids, file names, test names and the references in `cards`, and of the wording only those two phrases and the ruling markers, so a reworded sentence fails nothing while each marker keeps its link (#133).
