# Issues and patches

How an issue is titled and labelled, how a patch is numbered, and how a patch that takes several
pull requests is split up. It binds people and agents alike. The night bot's own `bot:*` labels are
in [`bot/README.md`](../bot/README.md#labels).

## Labels

Every issue carries at least one type label:

| Label | For |
|---|---|
| `patch` | A numbered release of the game: cards, rules, the client, the server's features |
| `major version` | A `vX.Y.0` release that changes the game or the codebase broadly enough to bump the minor or major version (v0.2.0, v0.3.0, v1.0.0), and each of its parts |
| `architecture` | The repository, tooling, CI, deploys and agent setup |
| `night bot` | The night bot itself: `bot/`, `.harness/` and its workflows |

The types can combine. A major version that changes the game is `major version` and `patch`, and a
patch whose work is all tooling (v0.2.8's codebase pass) is `patch` and `architecture`. The
`difficulty:*` labels (`easy`, `medium`, `hard`: the weakest model tier that may build it; none is
medium, and `hard` is Claude Opus's alone), `human` (people do it, such as a decision or any change to `bot/`, `.harness/` or `.github/`; the bot never queues, plans, builds or labels it), the `priority:*` labels
(the bot's pickup order) and the `bot:*` labels are separate. Never add
`bot:build` while retitling or relabelling, because it queues a build.

## Issue type

Every issue also has one of the organisation's issue types, which are not labels:

| Type | For |
|---|---|
| Task | A specific piece of work: most patches, tooling, docs |
| Bug | Something that behaves wrongly: a rule, a card, a crash, a failing job |
| Feature | Something new for players or for the team: a mode, a screen, a system |

A pull request has no type.

## Titles

| Kind | Title |
|---|---|
| Patch | `Patch vX.Y.Z: <what it does>` |
| Patch whose number isn't picked yet | `Patch v0.2.X: <what it does>`, with the X replaced once it is |
| Micro patch | `Patch v0.2.Y: <what it does>`, named when it ships (see Version numbers) |
| Revision of a shipped patch | `Patch vX.Y.Zb: <what it does>` |
| Major version | `vX.Y.0: <what it does>` |
| Part of a multi-part patch | `Patch vX.Y.Z (part n of m): <what it does>`, or `vX.Y.0 (part n of m): …` |
| Night bot | `Night bot: <…>` |
| Tooling | `CI: <…>` or `Architecture: <…>` |

"What it does" is a short phrase, such as `Patch v0.2.9: a public Card Almanac`. Keep version
numbers exactly as the designer gave them. `ci-duration.yml` finds its open issue by its exact
title, so leave that title alone. Night bot suggestions (`bot:suggestion`) arrive with plain
titles, so retitle one when you accept it.

## Version numbers

- **Names are labels.** The designer picks a version before work starts, and versions may ship out
  of name order (#63). The order is `packages/cards/patches/patches.json`'s, and nothing parses or
  compares a version string (R105, R388).
- **A shipped version never reopens.** A follow-up to it takes the same number plus a letter:
  `vX.Y.Zb`, then `c`, then `d`. This replaces the old `-rN` suffix: #85 renamed the patch history's
  v0.1.0-r1, -r2 and -r3 to v0.1.0b, v0.1.0c and v0.1.0d.
- **Micro or normal.** A patch is a micro patch, `Patch v0.2.Y: …`, when it is small: one fix, one
  card's numbers or text, one cosmetic or client tweak, with no new mechanic, keyword, ruling set or
  feature. Anything larger is a normal patch, `Patch v0.2.X: …`, whose number the designer picks.
  When a micro patch grows past that, retitle it `X`; when a normal one shrinks to one tweak,
  retitle it `Y`.
- **A micro patch is named when it ships (R650).** `Y` becomes the newest version in `patches.json`
  plus the next letter: a micro patch that ships after v0.2.5 is v0.2.5b, and one after v0.2.7c is
  v0.2.7d. The patch command does it: `pnpm --filter @jackioh/cards run patch v0.2.Y "<title>" …`
  (`packages/cards/scripts/versions.ts`). It refuses a `v0.2.Y` when the newest version is not a
  v0.2 one. A micro patch with no card data change bumps nothing and keeps its `Y` title.

## A patch that takes several pull requests

- **Tracker.** One issue, `Patch vX.Y.Z: <…>` (or `vX.Y.0: <…>`), holds the scope and a checklist
  of the parts, one line per part issue.
- **Parts.** Each part is its own issue, `Patch vX.Y.Z (part n of m): <…>`, labelled like its
  tracker: `patch`, or `major version` with `patch` if the part changes the game. Add
  `architecture` to a part that is all tooling. Link each part as a sub-issue of the tracker:

  ```
  gh api repos/jgoetzmann/JackiOh/issues/<tracker>/sub_issues \
    -F sub_issue_id=$(gh api repos/jgoetzmann/JackiOh/issues/<part> --jq .id)
  ```

  n is the order the parts are meant to merge in. When a part is added or moved, update m on the
  rest.
- **Pull requests.** A part's PR says `Closes #<part>` and `Part of #<tracker>`, never
  `Closes #<tracker>`. Close the tracker by hand once its last sub-issue is closed.
- **Going live.** A change to card data goes live through
  `pnpm --filter @jackioh/cards run patch <version> "<title>" --date <YYYY-MM-DD>`. It snapshots the
  catalog, appends the version to `patches.json` and bumps `CATALOG_VERSION` everywhere it lives
  (`packages/cards/README.md` §8), and the next deploy reseeds the database. The PR that runs it is
  the last part, and the patch is live when that PR merges. A patch with no card data change is
  live when its last part merges.
- **Until then, players see nothing new.** Every earlier part leaves main playable and unchanged
  for players. Code nothing reaches yet is fine. A changed card, rule or screen waits for the last
  part. The alternative is the one v0.2.0 used: build on an integration branch named after the
  version, keep a draft PR to main open, and land the whole thing as the last part.
- **After it.** Work that shouldn't hold the patch back, such as an e2e spec, an audit or a deploy
  check, can be split off as a part that merges after the version is live, provided it changes
  nothing for players. A change for players after that is a revision (`vX.Y.Zb`).
- **Moving a part.** A part deferred to another version moves under that version's tracker. Unlink
  it and link it to the new tracker, retitle it, and leave a comment saying why.

## How v0.2.0 was split

These rules come from v0.2.0 (#40), which came before them.

- It was built on the `v0.2.0` integration branch, with a draft PR (#50), and landed in one piece
  with #71, which bumped the catalog to v0.2.0.
- The card history and Patch notes page (#39) shipped on main a day early (PR #43), and the patch
  then rebuilt it (R375). The rule above, that players see nothing new until the last part, is
  there to stop that.
- To land without waiting, the deploy (#69), the e2e specs (#66) and the Part A audit (#67) were
  split off and merged afterwards.
- Engine speed (#70) and the AI's sweep of record (#65) moved to v0.3.0 (#80), which rewrites the
  engine and AI.
- The designer's sign-off on the B9 defaults (#68) came off the tracker so it wouldn't hold the
  patch open. Any override ships as a revision, v0.2.0b.
