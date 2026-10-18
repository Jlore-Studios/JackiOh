# Issues and patches

How an issue is titled and labelled, how a patch is numbered, and how a patch that takes several
pull requests is split up. It binds people and agents alike. The night bot's own `bot:*` labels are
in [`bot/README.md`](../bot/README.md#labels), its fullsend mode `bot:fullsend` and a fullsend
parent, `bot:tree`, among them ([Fullsend](../bot/README.md#fullsend)), and Squishy's `squishy:*`
ones (the same, plus its modes `squishy:oneshot`, `squishy:split`, `squishy:split-bot` and
`squishy:fullsend`, and a split's parent, `squishy:tree`) in [its section](../bot/README.md#squishy).

Open an issue through one of the forms **New issue** lists (`.github/ISSUE_TEMPLATE/`: Patch, Micro
patch, Bug, Architecture or CI, Night bot): each starts the title, puts on the type labels and asks
for what a builder needs. Its Difficulty, Priority and Who does it dropdowns become the
`difficulty:*` and `priority:*` labels and `bot:build` (the night bot) or `human` (people), as a
person would set them by hand, and triage gives the issue its type (Task, Bug or Feature) as it
opens (`triage.yml`). "Decide later" leaves who does it to you. A pull request starts from
`.github/pull_request_template.md` ([Pull requests](#pull-requests)). `CONTRIBUTING.md` is the short
version of this page for people and agents.

## Labels

Every issue carries at least one type label:

| Label | For |
|---|---|
| `patch` | A numbered release of the game: cards, rules, the client, the server's features |
| `major version` | A `vX.Y.0` release that changes the game or the codebase broadly enough to bump the minor or major version (v0.2.0, v0.3.0, v1.0.0), and each of its parts |
| `architecture` | The repository, tooling, CI, deploys and agent setup |
| `night bot` | The night bot and Squishy themselves: `bot/`, `.harness/`, `.squishy/` and their workflows |
| `production merge` | The countdown issue `promote-production.yml` keeps open (`Merging to production in N hours`) and the pull requests that merge main into production. The workflow opens, retitles and closes them and finds the open one by this label, so leave it on, and leave its titles to the workflow. It also labels them `human` and assigns both people. Comments `/hold`, `/resume`, `/delay 3h` and `/fast-forward` steer it (`docs/deploy-cloudflare.md`, section 2.3) |

The types can combine. A major version that changes the game is `major version` and `patch`, and a
patch whose work is all tooling (the codebase pass) is `patch` and `architecture`. The
`difficulty:*` labels (`easy`, `medium`, `hard`: the weakest model tier that may build it; `hard`
is Claude Opus's alone), `human` (people do it, such as a decision or any change to `bot/`,
`.harness/` or `.github/`; the bot never queues, plans, builds or labels it), the `priority:*`
labels (the bot's pickup order) and the `bot:*` labels are separate. Never add `bot:build` while
retitling or relabelling, because it queues a build.

Every label, with its colour and description, is listed once, in `bot/harness/config.py`'s
`LABELS`: the sweep creates one the repository lacks and deletes one it retired
(`RETIRED_LABELS`), and `harness setup` also restores a changed one.

**Who does an issue: one label each.** `bot:build` hands an issue to the night bot, which queues it;
so does assigning the bot or commenting `/harness build`. `human` hands it to people: triage
assigns MaxGoetzmann and jgoetzmann, unassigns the bot and takes the bot's queue labels off, and
the bot never touches it again (#664). A suggestion the bot opened (`bot:suggestion`) is approved
the same way: queue it, and `bot:suggestion` comes off; close it to say no.

**How hard it is.** A person may set a `difficulty:*` label, and the bot never changes it.
Otherwise the night bot's planner rates the issue when it plans it, and it counts as medium until
then: **easy** only when it is small (at most 10 files and 400 lines), in one crate or app,
touches none of `SPEC.md`, the notes in `spec/`, `BUILD.md` or the shared surfaces, does no engine,
AI, catalog, card or migration work, is checked by a unit test, leaves the builder no decision, and
is blocked by nothing (`bot/harness/easy.py`); **hard** for engine rules, the AI, design across
crates, a migration, or about 40 files or more; **medium** for the rest. Triage
gives no difficulty beyond the form's answer. Every ten minutes the sweep labels what is left bare
(#735): a planned issue with no difficulty gets `difficulty:medium`, and a bot pull request its
issue's label (or medium); an issue still waiting for its plan is left to the planner. An agent
that opens an issue labels it itself (`CLAUDE.md` rule 11). Three failures of an issue's own raise it a step (`bot/README.md`, Rating,
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
| Normal patch not shipped yet | `Patch v0.3.X: <what it does>`, the X replaced by the next number once its fragment is made (see Version numbers) |
| Micro patch | `Patch v0.3.Y: <what it does>`, named when it ships (see Version numbers) |
| Revision of a shipped patch | `Patch vX.Y.Zb: <what it does>` |
| Major version | `vX.Y.0: <what it does>` |
| Part of a multi-part patch | `Patch vX.Y.Z (part n of m): <what it does>`, or `vX.Y.0 (part n of m): …` |
| Night bot | `Night bot: <…>` |
| Tooling | `CI: <…>` or `Architecture: <…>` |

"What it does" is a short phrase, such as `Patch v0.2.4: aimed random casts and the Deft keyword`.
A pull request takes the same titles ([Pull requests](#pull-requests)).
Keep a version number as it was given; #290's renames (R743) were not carried into older titles.
`ci-duration.yml` finds its open issue by its exact title, so leave that title alone. Night bot
suggestions (`bot:suggestion`) are asked for conventional titles; retitle one that arrives without.

## Version numbers

- **Numbers run in order (R743).** A card patch takes the next number after the newest card patch on
  `main`, shipped (`crates/cards/patches/patches.json`) or still pending (`pending/`): after v0.2.9
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
- **Micro or normal.** A patch is a micro patch, `Patch v0.3.Y: …`, when it is small: one fix, one
  card's numbers or text, one cosmetic or client tweak, with no new mechanic, keyword, ruling set or
  feature. Anything larger is a normal patch, `Patch v0.3.X: …`, which takes the next number (above).
  When a micro patch grows past that, retitle it `X`; when a normal one shrinks to one tweak,
  retitle it `Y`.
- **A micro patch is named when it ships (R650).** `Y` becomes the newest version in `patches.json`
  plus the next letter: a micro patch that ships after v0.2.5 is v0.2.5b, and one after v0.2.7c is
  v0.2.7d. Going live through a pending fragment it keeps its `Y` (`pending/v0.2.Y.json`) until
  `patches ship` promotes it, which names it (`crates/tools/src/patches.rs`'s `versions`). It refuses a
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
  `cargo jackioh patches <version> <date> "<title>"` writes
  `crates/cards/patches/pending/<version>.json` claiming the cards the branch changed, and after
  the branch merges `patches ship` promotes it — appending the version to `patches.json` and
  bumping the catalog version everywhere it lives (`crates/cards/README.md` §6) — and the next
  deploy reseeds the database. The PR that adds the fragment is the last part, and the patch is
  live when the promotion PR merges. A patch with no card data change is live when its last part
  merges.
- **Until then, players see nothing new.** Every earlier part leaves main playable and unchanged
  for players. Code nothing reaches yet is fine. A changed card, rule or screen waits for the last
  part. The alternative is the one v0.2.0 used: build on an integration branch named after the
  version, keep a draft PR to main open, and land the whole thing as the last part. A new set may
  instead be built on main behind the release gate (R1420), as the Meditative set was (#496,
  `docs/meditative-set.md` M3, shipped by #553) and the Meditative+ set is (#1173,
  `docs/meditative-plus-set.md` MP3, R1178000). Its cards are in the catalog but in no pool, deck, list or patch
  until its last part adds the set to `SHIPPED_SETS`, so its parts need no fragment, move no golden
  trace and can merge in any order. Work under the same tracker that players should see as soon as
  it lands is a patch of its own, titled without `(part n of m)`, and goes live when it merges.
- **After it.** Work that shouldn't hold the patch back, such as an e2e spec, an audit or a deploy
  check, can be split off as a part that merges after the version is live, provided it changes
  nothing for players. A change for players after that is a revision (`vX.Y.Zb`).
- **Moving a part.** A part deferred to another version moves under that version's tracker. Unlink
  it and link it to the new tracker, retitle it, and leave a comment saying why.

## A task over every card

An issue that asks for a pass over the whole catalog (every card, every face, every term) is where
a run most easily does less than was asked and still reports it done: #814's first pass shortened
no card, decided nothing about bold, and checked consistency with a text-similarity threshold that
could not find what it was looking for (`reviews/2026-10-14-issue-814-first-pass.md`). Such an
issue:

- **Names the deliverable that proves the pass**: a table in the pull request with a row per card
  or per group, its verdict and the reason, so a reviewer sees what was checked. "Every card
  checked" alone cannot be reviewed.
- **Gives the measure**: a length tier, a list of terms, a grouping method ("group cards by the
  verbs their scripts call", never "compare similar texts"). An example in the issue is a test
  case for the pass, not its scope.
- **Is `difficulty:hard`**: a pass over 450 entries touches about 40 files or more (CLAUDE.md
  rule 11).

A run that splits such an issue says in the tracker, part by part, what each part leaves out, so a
part that narrows the task shows the gap instead of hiding it.

## Patch notes players read

- **Players see the note, never the record (R724000, R724001).** A patch's `title`, `sources` and
  `notes` are the history's: they may name issues, rulings and commits. The Patch notes page shows
  its `player` note instead: `{ title, new, balance, changes, text, fixes }`.
- **Write it by hand in the fragment.** `cargo jackioh patches …` has no flag for it: open
  `pending/<version>.json` and fill in `player`. Running the command again (when the cards change)
  keeps the note, `patches ship` carries it onto the shipped patch, and `patches check` fails until
  it has a title and at least one line, and on a blank line, a line over 200 characters, or a
  ruling (`R656`), an issue or card number (`#88`) or a `§`.
- **The title** is a short phrase with no version, which the page shows beside it: "Gary the Gambler
  buffed", not "Patch v0.2.Y: Buff Gary the Gambler".
- **One fact per line, under its heading.**
  - `new`: the cards, tokens, keywords and tags the patch brings.
  - `balance`: a cost, stat, number or rarity, written from and to ("Hit Job costs (3), up from
    (2).").
  - `changes`: how a card or a rule plays.
  - `text`: wording only, nothing plays differently.
  - `fixes`: what was broken and now works.
- **Only what that patch changed.** Take it from the patch's snapshot diff (the cards behind its
  toggle) and its change, never from a pull request's title, and never say what the game does not
  do yet.
- **Today's words.** Deck and Tribute, "(N) Cost" and "costs (N)", Nerf and Buff (R373, R432,
  R1320): `apps/web/src/wording.test.ts` checks every note in `patches.json`.
- **A set is a release line.** A major version that brings a set adds its line to `RELEASES`
  (`apps/web/src/patches/releases.ts`), with `first` set to its first card patch, a summary and
  its technical changes. The page shows one bubble per line, newest open.
- **Technical changes.** A change players notice that ships no card data (a mode, a screen, the AI,
  the engine, sound) adds one line to the `technical` of its major version's line, the newest, in its
  own pull request, in the style of the lines there ("Background music and a main menu theme."). A
  major version's changes before its first card patch are its line's too: v0.3.0, the Rust rewrite,
  is Meditative's.

## Pull requests

- **Title.** A pull request's title becomes the subject of the squash commit it lands as, so it
  follows [Titles](#titles), usually as its issue's does. The `pr title` check
  (`.github/workflows/pr-title.yml`) fails one that doesn't and runs again when it is edited. Its
  pattern is triage's (`bot/harness/triage.py`, `CONVENTION`), plus the titles tools give their own
  pull requests (`AUTOMATED_PULL`): `patches ship: v0.3.1` (`patches-ship.yml`),
  `Promote main to production: N commit(s) up to abc1234` (`scripts/promote-production.sh`), a
  training lane's `AI gen N (improve): …`, `AI gen N (unban): …` or `AI gen N (learn): …`, and GitHub's `Revert "…"` of
  a title that passes. The bots title theirs from the builder's answer when it follows the
  convention, else from the issue (`triage.pull_title`), and merge by squash, so their branch
  commits' subjects (`bot: build pass 1 for #85`) never become one on `main`.
- **Body.** The template asks for `Closes #n` (a part says `Closes #<part>` and
  `Part of #<tracker>`), what changed, the tests run, the risks, the rulings added or changed and
  the patch version, and the gates (CLAUDE.md, Commands).
- **Review.** `.github/CODEOWNERS` asks MaxGoetzmann and jgoetzmann to review any change to
  `.github/`, `bot/`, `.harness/` or `.squishy/`, the paths the bots may not change.

## Ruling numbers

A ruling the spec does not make yet takes the lowest number free in its issue's block (CLAUDE.md
rule 3, R690000): issue N owns the thousand numbers R(N×1000) to R(N×1000+999), so issue #690 owns
R690000–R690999 and issue #900 owns R900000–R900999. A sub-issue (a fullsend part, a tracker's
part) uses its own number's block, not its parent's, and a pull request with no issue uses its own
number's; GitHub numbers issues and pull requests in one sequence, so no two blocks overlap. The
note is `spec/rulings/R<n>.md`, zero-padded to four digits at least (`R690000.md`). Branches on
different issues never take the same number, so merging `main` in never renumbers one. The numbers
taken before R690000 are sequential and below R2000: a branch that took one under the old rule
(the last row of `spec/INDEX.md` on `main` plus one) keeps it, and issue #1, whose block would
overlap them, has none.

Two branches can still take one number when they work on one issue. The one that merges second
renumbers before it merges: it merges `main` in, moves its note to the next number free in its
issue's block (the file name and its `id`), renames its proving tests (`fn r<n>_…`, `it("R<n> …")`)
and every `R<n>` and `[[R<n>]]` its change cites, and runs `cargo jackioh spec index` again for its
index row. The clash shows as a conflict on the note, and `cargo jackioh spec check` fails on
whatever the renumbering missed: two notes with one `id`, a test or a citation with no note, a link
to nothing. A conflict on `spec/INDEX.md` or `spec/11-rulings.md` alone is no clash: two branches'
rows or paragraphs met, and `cargo jackioh spec index` rewrites the index with both, renumbering
nothing. Numbers on `main` are never reused or renumbered (`spec/README.md`). Under the old rule
this happened on nearly every merge (#357 and #360 both took R744, and #360 moved to R745).

Before R690000, a tracker whose parts built side by side could hold a block of sequential numbers
for each part, so its parts never clashed with each other; a part now takes its own sub-issue's
block, and nothing new is held. A held number counts as in use: other work never takes one. Held
now:

| Tracker | Numbers | Blocks |
| --- | --- | --- |
| #496, the Meditative set | R780–R1429 | `docs/meditative-set.md` M10 |

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
