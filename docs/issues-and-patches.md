# Issues and patches

How an issue is titled and labelled, how a patch is numbered, and how a patch that takes several
pull requests is split up. It binds people and agents alike. The night bot's own `bot:*` labels are
in [`bot/README.md`](../bot/README.md#labels), and Squishy's `squishy:*` ones (the same, plus its
modes `squishy:oneshot`, `squishy:split`, `squishy:split-bot` and a split's parent, `squishy:tree`)
in [its section](../bot/README.md#squishy).

## Labels

Every issue carries at least one type label:

| Label | For |
|---|---|
| `patch` | A numbered release of the game: cards, rules, the client, the server's features |
| `major version` | A `vX.Y.0` release that changes the game or the codebase broadly enough to bump the minor or major version (v0.2.0, v0.3.0, v1.0.0), and each of its parts |
| `architecture` | The repository, tooling, CI, deploys and agent setup |
| `night bot` | The night bot and Squishy themselves: `bot/`, `.harness/`, `.squishy/` and their workflows |
| `Info` | For the record, nothing to build: migrations, statistics, the night bot's status |
| `production merge` | The countdown issue `promote-production.yml` keeps open (`Merging to production in N hours`) and the pull requests that merge main into production. The workflow opens, retitles and closes them and finds the open one by this label, so leave it on, and leave its titles to the workflow. It also labels them `human` and assigns both people. Comments `/hold`, `/resume`, `/delay 3h` and `/fast-forward` steer it (`docs/deploy-cloudflare.md`, section 2.3) |

The types can combine. A major version that changes the game is `major version` and `patch`, and a
patch whose work is all tooling (the codebase pass) is `patch` and `architecture`. The
`difficulty:*` labels (`easy`, `medium`, `hard`: the weakest model tier that may build it; `hard`
is Claude Opus's alone), `human` (people do it, such as a decision or any change to `bot/`,
`.harness/` or `.github/`; the bot never queues, plans, builds or labels it), the `priority:*`
labels (the bot's pickup order), the `method:*` labels and the `bot:*` labels are separate. Never
add `bot:build` while retitling or relabelling, because it queues a build.

**Who does an issue: the method labels.** An issue is triaged only once a person labels it
`method:manual` or `method:use-bot` (#307). Two minutes after the label goes on, so a person can
set a difficulty and a priority first, triage reads it again: `method:manual` adds `human` and
assigns MaxGoetzmann and jgoetzmann (unassigning the bot); `method:use-bot` adds `bot:build`, a
priority and its type labels, and assigns the bot (unassigning both people). An issue labelled
`human` never goes to the bot. A suggestion the bot opened (`bot:suggestion`) is approved with
`bot:approved`, which triage treats as `method:use-bot`; the bot builds no suggestion without it.

**How hard it is.** A person may set a `difficulty:*` label, and the bot never changes it.
Otherwise the night bot's planner rates the issue when it plans it, and it counts as medium until
then: **easy** only when it is small (at most 10 files and 400 lines), in one package, touches
none of `SPEC.md`, `BUILD.md`, `packages/engine/test/rulings.test.ts` or the shared surfaces,
does no engine, AI, catalog, card or migration work, is checked by a unit test, leaves the builder
no decision, and is blocked by nothing (`bot/harness/easy.py`); **hard** for engine rules, the AI,
design across packages, a migration, or about 40 files or more; **medium** for the rest. Triage
gives no difficulty. Three failures of an issue's own raise it a step (`bot/README.md`, Rating,
strikes and the step up).

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
| Normal patch not shipped yet | `Patch v0.2.X: <what it does>`, the X replaced by the next number once its fragment is made (see Version numbers) |
| Micro patch | `Patch v0.2.Y: <what it does>`, named when it ships (see Version numbers) |
| Revision of a shipped patch | `Patch vX.Y.Zb: <what it does>` |
| Major version | `vX.Y.0: <what it does>` |
| Part of a multi-part patch | `Patch vX.Y.Z (part n of m): <what it does>`, or `vX.Y.0 (part n of m): …` |
| Night bot | `Night bot: <…>` |
| Tooling | `CI: <…>` or `Architecture: <…>` |

"What it does" is a short phrase, such as `Patch v0.2.4: aimed random casts and the Deft keyword`.
Keep a version number as it was given; #290's renames (R743) were not carried into older titles.
`ci-duration.yml` finds its open issue by its exact title, so leave that title alone. Night bot
suggestions (`bot:suggestion`) arrive with plain titles, so retitle one when you accept it.

## Version numbers

- **Numbers run in order (R743).** A card patch takes the next number after the newest card patch on
  `main`, shipped (`packages/cards/patches/patches.json`) or still pending (`pending/`): after v0.2.9
  with nothing pending, a normal patch (`Patch v0.2.X: …`) is v0.2.10, and a micro patch
  (`Patch v0.2.Y: …`) becomes v0.2.9b when it ships (below). Name the pending fragment that number
  when the branch makes it. Two normal patches in flight at once cannot both take it: the one that
  merges second renames its fragment to the number after, or `patches ship` would ship it as a
  revision letter of the first. A patch that changes no card data ships no version and takes no
  number, so it keeps its `X` or `Y` title and leaves no gap. The order is still `patches.json`'s,
  and nothing compares version strings (R105, R388). #290 renamed the history this way: v0.2.4,
  v0.2.5, v0.2.10, v0.2.11, v0.2.12, v0.2.13, v0.2.14, v0.2.14b, v0.2.16, v0.2.16b and v0.2.17 are
  now v0.2.1 to v0.2.7, v0.2.7b, v0.2.8, v0.2.8b and v0.2.9, and #88's pending v0.2.15 is v0.2.10;
  commit messages and older titles keep the old names.
- **A shipped version never reopens.** A follow-up to it takes the same number plus a letter:
  `vX.Y.Zb`, then `c`, then `d`. This replaces the old `-rN` suffix: #85 renamed the patch history's
  v0.1.0-r1, -r2 and -r3 to v0.1.0b, v0.1.0c and v0.1.0d.
- **Micro or normal.** A patch is a micro patch, `Patch v0.2.Y: …`, when it is small: one fix, one
  card's numbers or text, one cosmetic or client tweak, with no new mechanic, keyword, ruling set or
  feature. Anything larger is a normal patch, `Patch v0.2.X: …`, which takes the next number (above).
  When a micro patch grows past that, retitle it `X`; when a normal one shrinks to one tweak,
  retitle it `Y`.
- **A micro patch is named when it ships (R650).** `Y` becomes the newest version in `patches.json`
  plus the next letter: a micro patch that ships after v0.2.5 is v0.2.5b, and one after v0.2.7c is
  v0.2.7d. Going live through a pending fragment it keeps its `Y` (`pending/v0.2.Y.json`) until
  `patches ship` promotes it, which names it (`packages/cards/scripts/versions.ts`). It refuses a
  `v0.2.Y` when the newest version is not a v0.2 one. A micro patch with no card data change bumps
  nothing and keeps its `Y` title.

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
- **Going live.** A change to card data goes live through a pending fragment:
  `pnpm --filter @jackioh/cards run patches <version> <date> "<title>"` writes
  `packages/cards/patches/pending/<version>.json` claiming the cards the branch changed, and after
  the branch merges `patches ship` promotes it — appending the version to `patches.json` and
  bumping `CATALOG_VERSION` everywhere it lives (`packages/cards/README.md` §8) — and the next
  deploy reseeds the database. The PR that adds the fragment is the last part, and the patch is
  live when the promotion PR merges. A patch with no card data change is live when its last part
  merges.
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
