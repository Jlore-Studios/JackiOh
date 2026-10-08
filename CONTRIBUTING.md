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

Open one through a form (**New issue**): Patch, Micro patch, Bug, Architecture or CI, Night bot.
Each starts the title, puts on the type labels and sets the issue type (Task, Bug or Feature). When
it is ready, label it `method:manual` (people do it) or `method:use-bot` (the night bot does);
triage classifies it two minutes later. Work on `.github/`, `bot/`, `.harness/` or `.squishy/` is
`human`: the bots may not change those paths.

## Titles

Issues and pull requests share them (`docs/issues-and-patches.md`, Titles):

| Kind | Title |
|---|---|
| Normal patch | `Patch v0.3.X: <what it does>`, the X numbered once its pending fragment is made |
| Micro patch | `Patch v0.3.Y: <what it does>`, named when it ships |
| Part of a patch | `Patch vX.Y.Z (part n of m): <…>` |
| Major version | `vX.Y.0: <…>` |
| The bots | `Night bot: <…>` |
| Tooling | `CI: <…>` or `Architecture: <…>` |

The `pr title` check fails a pull request whose title breaks these. Tools' own pull requests
(`patches ship: …`, `Promote main to production: …`, a training lane's `AI gen N (lane): …`) pass
as they are.

## Pull requests and commits

- Fill in the template: `Closes #n` (or `Part of #<tracker>`), what changed, the tests you ran,
  the risks, the rulings and the patch version, and the gates from `CLAUDE.md`, Commands.
- The title becomes the squash commit's subject on `main`. Commits on a branch say what they change
  in a short subject, with the why in the body when it isn't obvious. Put `[vercel]` in a branch
  commit's message for a Vercel preview of it (`docs/architecture.md` §4.2).
- A change to card data goes live through a pending fragment (`cargo jackioh patches …`); the
  version rules are in `docs/issues-and-patches.md`, Version numbers.

## Ruling numbers

A new ruling takes the next free number (`CLAUDE.md` rule 3): the last row of `spec/INDEX.md` on
`main` plus one. Name it in your pull request. If another open pull request took the same number,
the one that merges second renumbers its note, its tests and its index row first;
`cargo jackioh spec check` catches what it misses (`docs/issues-and-patches.md`, Ruling numbers).
