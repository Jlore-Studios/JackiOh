# Contributing to JackiOh

For people and agents alike. This is a pointer page: the rules live in the files it names.

## Read first

- `CLAUDE.md`: the rules of engagement (numbered; code cites them, "CLAUDE.md rule 7"), the
  commands, CI and the architecture. They bind every agent, not only Claude. `AGENTS.md` is the
  shared entry point for other agents.
- `SPEC.md` and the notes in `spec/`: the only source of rules, cards and engine design.
- `docs/ADDING_CARDS.md`, before any card work.
- [`docs/issues-and-patches.md`](docs/issues-and-patches.md): how issues and pull requests are
  titled and labelled, how patches are numbered, and how a patch split over several pull requests
  goes live.

## Issues

Open one through a form (**New issue**): Patch, Bug, Architecture or CI, Night bot. Each titles it `Patch: `.
Each starts the title, puts on the type labels and sets the issue type (Task, Bug or Feature). When
it is ready, label it `human` (people do it) or `bot:build` (the night bot does), or pick one in
the form's Who does it. Work on `.github/`, `bot/`, `.harness/` or `.squishy/` is `human`: the bots
may not change those paths.

## Titles

Issues and pull requests share them (`docs/issues-and-patches.md`, Titles):

| Kind | Title |
|---|---|
| Any change | `Patch: <what it does>` |
| Part of a change split over several issues | `Patch (part n of m): <…>` |
| A shipped card change | `Patch v<version>: <…>`, given by `patches-ship.yml` once it ships |

No title names a version before it ships: `patches ship` names a card change's version from what it
does to the cards (the classifier, R1365000) and retitles its issue and pull request. The `pr title`
check fails a pull request whose title breaks these, the older prefixes (`Patch v0.3.X`, `Patch
v0.3.Y`, `vX.Y.0`, `Night bot`, `CI`, `Architecture`) included since the title retcon (#1372). Tools' own
pull requests (`patches ship: …`, `Promote main to production: …`, a training lane's `AI gen N
(lane): …`) pass as they are.

## Pull requests and commits

- Fill in the template: `Closes #n` (or `Part of #<tracker>`), what changed, the tests you ran,
  the risks, the rulings and what the fragment claims, and the gates from `CLAUDE.md`, Commands.
- The title becomes the squash commit's subject on `main`. Commits on a branch say what they change
  in a short subject, with the why in the body when it isn't obvious. Put `[vercel]` in a branch
  commit's message for a Vercel preview of it (`docs/architecture.md` §4.2).
- A change to a card goes live through a pending fragment (`cargo jackioh patches "<title>"`, with
  `--plays <ids>` for a card whose script or engine rule changed how it plays); CI's `rust
  (classify)` checks the claims and labels the pull request with its kind and tier. The version rules
  are in `docs/issues-and-patches.md`, Version numbers.

## Ruling numbers

A new ruling takes the lowest number free in its issue's block (`CLAUDE.md` rule 3): issue N
owns R(N×1000) to R(N×1000+999), so issue #690's first is R690000, and a pull request with no
issue uses its own number. Name it in your pull request. If another open pull request on the same
issue took the same number, the one that merges second renumbers its note, its tests and its index
row first, within the block; `cargo jackioh spec check` catches what it misses
(`docs/issues-and-patches.md`, Ruling numbers).
