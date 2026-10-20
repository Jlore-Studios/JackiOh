# Issues and patches

How an issue is titled and labelled, how a patch is numbered, and how a patch that takes several
pull requests is split up. It binds people and agents alike. The night bot's own `bot:*` labels are
in [`bot/README.md`](../bot/README.md#labels), its fullsend mode `bot:fullsend` and a fullsend
parent, `bot:tree`, among them ([Fullsend](../bot/README.md#fullsend)), and Squishy's `squishy:*`
ones (the same, plus its modes `squishy:oneshot`, `squishy:split`, `squishy:split-bot` and
`squishy:fullsend`, and a split's parent, `squishy:tree`) in [its section](../bot/README.md#squishy).

Open an issue through one of the forms **New issue** lists (`.github/ISSUE_TEMPLATE/`: Patch, Bug,
Architecture or CI, Night bot): each starts the title (`Patch: `), puts on the type labels and asks
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

A pull request also gets its **kind** and, for a card change, its **tier** from the classifier
(`cargo jackioh classify`, R1365000), which CI's `rust (classify)` puts on and keeps up to date with
every push (#1370). Nothing else sets them, and no label overrides the classifier:

| Label | For |
|---|---|
| `kind:cards` | It changes how a card plays (its data, its script, or an engine rule its games meet) or its words |
| `kind:ai` | It changes the practice and ladder AI's decisions |
| `kind:app` | It changes the client or the server, what players see and use |
| `kind:internal` | CI, the bots, tooling, docs, tests or a refactor |
| `tier:major` | A set ships: the next `v0.Z.0` |
| `tier:patch` | 10 or more cards play differently, or a rule every game meets: the next `v0.Z.X` |
| `tier:micro` | Fewer cards, or their words alone: the next letter |

The type labels stay for routing (the night bot's sweeps and the forms read them), though a `Patch:`
title names none.

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
| Any change | `Patch: <what it does>` |
| A shipped card change | `Patch v<version>: <what it does>`, which `patches-ship.yml` gives its issue and its pull request once the version ships |
| Part of a change split over several issues | `Patch (part n of m): <what it does>`, a sub-issue of its tracker |
| A tool's own pull request | `patches ship: v0.3.18`, `Promote main to production: …`, `AI gen N (lane): …`, `Revert "…"` |

"What it does" is a short phrase, such as `Patch: aimed random casts and the Deft keyword`. No title
names a version before it ships: the version follows from what the change does to the cards
([Version numbers](#version-numbers)), and the issue and the pull request are retitled
`Patch v0.3.18: aimed random casts and the Deft keyword` when it ships. A change that ships no card
data never takes a version and keeps its `Patch:` title. Its kind (Cards, AI, App, Internal) is the
classifier's label on its pull request, never its title's.

A pull request takes the same titles ([Pull requests](#pull-requests)). The older prefixes
(`Patch v0.3.X: …`, `Patch v0.3.Y: …`, `vX.Y.0: …`, `Night bot: …`, `CI: …`, `Architecture: …`) still
pass the title check until the title retcon (#1372) has retitled every thread; open new threads with
`Patch: `. `ci-duration.yml` finds its open issue by its exact title, so leave that title alone, and
leave the `production merge` threads' titles to their workflow. Night bot suggestions
(`bot:suggestion`) are asked for conventional titles; retitle one that arrives without.

## Version numbers

A version is `v0.Z.X` plus a letter, and it moves exactly when the cards a player compares across it
do (#1360, R1365000, R1366000). Nothing guesses it: when a patch ships, `patches ship` reads what it
does to the cards through the classifier and names it:

- **Major, `v0.(Z+1).0`**: a set reaches `SHIPPED_SETS` (R1420). Nothing else makes one: no title,
  flag or label. v1.0.0 waits until the owner asks for it.
- **Patch, `v0.Z.(X+1)`**: 10 or more card families play differently (`PATCH_MIN_CARDS`), or a rule
  every game meets changed.
- **Micro, the newest version with the next letter** (after v0.3.18, v0.3.18b, then v0.3.18c):
  anything smaller, and any change to wording alone however many cards it touches.

A family is a card with its tokens. A card plays differently when its catalog entry changes a field
other than its wording (`name`, a face's `text`, `refs`, `printedRarity`, `index`), when it is added
or removed, or when its script or an engine rule changes how its games go, which the classifier's
second stage confirms against the base build. The patch records its `tier` and `plays` (the families)
in `patches.json`, so readers never parse a version, and the order of patches is still
`patches.json`'s, never a comparison of version strings (R105, R388).

- **A pull request's claims are checked.** A card change goes live through a pending fragment
  ([A patch that takes several pull requests](#a-patch-that-takes-several-pull-requests), Going
  live). CI's `rust (classify)` fails a pull request when a card plays differently and no fragment
  claims it, or when a fragment claims a play or a rule the base build does not show (R1370000), and
  `patches check` holds every catalog change to exactly one claim (R646).
- **A shipped version never reopens.** Every later change takes a later name.
- **Codebase projects take no version.** `docs/v0.3.0/` (the Rust rewrite) and `docs/v0.4.0/` (the
  Luau scripts) keep their folder names, which code comments cite, but they name codebase projects,
  not catalog versions.
- **Older names.** #290 renamed the history once (R743): v0.2.4, v0.2.5, v0.2.10, v0.2.11, v0.2.12,
  v0.2.13, v0.2.14, v0.2.14b, v0.2.16, v0.2.16b and v0.2.17 are v0.2.1 to v0.2.7, v0.2.7b, v0.2.8,
  v0.2.8b and v0.2.9, and #88's v0.2.15 is v0.2.10; #85 renamed v0.1.0-r1, -r2 and -r3 to v0.1.0b,
  v0.1.0c and v0.1.0d. Commit messages and older titles keep the names they were written with.

## Version retcon

#1371 (part 7 of #1360, R1371000) renamed every shipped version to what the classifier (R1365000)
would have called it: a major where a set first ships, a patch where `PATCH_MIN_CARDS` or more card
families play differently, a micro for anything smaller or for wording alone. `cargo jackioh patches
retcon` computed the map from the snapshots and, from the Rust scripts on, the scripts each patch's own
commits changed, and renamed every name the repository and the database hold. Docs, commit messages,
code comments and the older issue and pull request titles keep the old names; translate them with this
table. Old and new names overlap (the old v0.3.4 is the new v0.2.7b, while the new v0.3.4 is the old
v0.3.12), so read a bare version in an old text as old. A file without the `versions` marker
(`crates/cards/patches/aliases.json` holds the map its readers use) names the old way too.

| Old | New | Tier | Families |
|---|---|---|---:|
| v0.1.0 | v0.1.0 | major | 101 |
| v0.1.0b | v0.1.0b | micro | 3 |
| v0.1.0c | v0.1.0c | micro | 1 |
| v0.1.0d | v0.1.0d | micro | 9 |
| v0.1.1 | v0.1.1 | patch | 19 |
| v0.2.0 | v0.2.0 | major | 270 |
| v0.2.1 | v0.2.0b | micro | 0 |
| v0.2.2 | v0.2.1 | patch | 20 |
| v0.2.3 | v0.2.2 | patch | 19 |
| v0.2.4 | v0.2.2b | micro | 3 |
| v0.2.5 | v0.2.3 | patch | 17 |
| v0.2.6 | v0.2.3b | micro | 1 |
| v0.2.7 | v0.2.3c | micro | 2 |
| v0.2.7b | v0.2.3d | micro | 6 |
| v0.2.8 | v0.2.3e | micro | 1 |
| v0.2.8b | v0.2.3f | micro | 1 |
| v0.2.9 | v0.2.4 | patch | 17 |
| v0.2.10 | v0.2.5 | patch | 37 |
| v0.2.10b | v0.2.5b | micro | 4 |
| v0.2.10c | v0.2.5c | micro | 0 |
| v0.2.10d | v0.2.5d | micro | 2 |
| v0.2.11 | v0.2.5e | micro | 1 |
| v0.3.1 | v0.2.5f | micro | 1 |
| v0.3.2 | v0.2.6 | patch | 75 |
| v0.3.3 | v0.2.7 | patch | 13 |
| v0.3.4 | v0.2.7b | micro | 0 |
| v0.3.5 | v0.3.0 | major | 102 |
| v0.3.6 | v0.3.0b | micro | 0 |
| v0.3.7 | v0.3.0c | micro | 7 |
| v0.3.7b | v0.3.1 | patch | 30 |
| v0.3.8 | v0.3.2 | patch | 19 |
| v0.3.9 | v0.3.2b | micro | 1 |
| v0.3.10 | v0.3.3 | patch | 60 |
| v0.3.10b | v0.3.3b | micro | 4 |
| v0.3.11 | v0.3.3c | micro | 0 |
| v0.3.12 | v0.3.4 | patch | 12 |
| v0.3.13 | v0.3.4b | micro | 7 |
| v0.3.14 | v0.3.5 | patch | 13 |
| v0.3.15 | v0.4.0 | major | 27 |
| v0.3.16 | v0.4.0b | micro | 4 |
| v0.3.17 | v0.4.0c | micro | 3 |

## A patch that takes several pull requests

- **Tracker.** One issue, `Patch: <…>`, holds the scope and a checklist of the parts, one line per
  part issue.
- **Parts.** Each part is its own issue, `Patch (part n of m): <…>`, labelled like its
  tracker: `patch`, or `major version` with `patch` if the part changes the game. Add
  `architecture` to a part that is all tooling. Link each part as a sub-issue of the tracker:

  ```
  gh api repos/jgoetzmann/JackiOh/issues/<tracker>/sub_issues \
    -F sub_issue_id=$(gh api repos/jgoetzmann/JackiOh/issues/<part> --jq .id)
  ```

  n is the order the parts are meant to merge in. When a part is added or moved, update m on the
  rest. The night bot holds a part until the parts before it, the same parent's sub-issues, have
  closed (`bot/harness/queue.py`, `waits_for`), so a part that is not linked waits for nothing.
- **Pull requests.** A part's PR says `Closes #<part>` and `Part of #<tracker>`, never
  `Closes #<tracker>`. Close the tracker by hand once its last sub-issue is closed.
- **Going live.** A change to a card goes live through a pending fragment:
  `cargo jackioh patches "<title>"` writes `crates/cards/patches/pending/<slug>.json`, named after
  the title, claiming the catalog cards the branch changed. A card whose script or engine rule
  changed how it plays, with no catalog change, is claimed with `--plays <ids>`, and a rule every
  game meets with `--rule`, as `cargo jackioh classify --base origin/main --parent-bin <main's
  jackioh>` names them (R1366000). After the branch merges `patches ship` promotes it, naming the
  version from its tier, appending it to `patches.json` and bumping the catalog version everywhere
  it lives (`crates/cards/README.md` §6), and the next deploy reseeds the database. The PR that adds the fragment is the last part, and the patch is
  live when the promotion PR merges. A patch with no card data change is live when its last part
  merges.
- **Until then, players see nothing new.** Every earlier part leaves main playable and unchanged
  for players. Code nothing reaches yet is fine. A changed card, rule or screen waits for the last
  part. The alternative is the one v0.2.0 used: build on an integration branch named after the
  version, keep a draft PR to main open, and land the whole thing as the last part. A new set may
  instead be built on main behind the release gate (R1420), as the Meditative set was (#496,
  `docs/meditative-set.md` M3, shipped by #553) and the Meditative+ set was (#1173,
  `docs/meditative-plus-set.md` MP3, R1178000, shipped by #1187). Its cards are in the catalog but in no pool, deck, list or patch
  until its last part adds the set to `SHIPPED_SETS`, so its parts need no fragment, move no golden
  trace and can merge in any order. Work under the same tracker that players should see as soon as
  it lands is a patch of its own, titled without `(part n of m)`, and goes live when it merges.
- **After it.** Work that shouldn't hold the patch back, such as an e2e spec, an audit or a deploy
  check, can be split off as a part that merges after the version is live, provided it changes
  nothing for players. A change for players after that is a patch of its own.
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
- **Is `difficulty:hard`**: a pass over 480 entries touches about 40 files or more (CLAUDE.md
  rule 11).

A run that splits such an issue says in the tracker, part by part, what each part leaves out, so a
part that narrows the task shows the gap instead of hiding it.

## Patch notes players read

- **Players see the note, never the record (R724000, R724001).** A patch's `title`, `sources` and
  `notes` are the history's: they may name issues, rulings and commits. The Patch notes page shows
  its `player` note instead: `{ title, new, balance, changes, text, fixes }`.
- **Write it by hand in the fragment.** `cargo jackioh patches …` has no flag for it: open
  `pending/<slug>.json` and fill in `player`. Running the command again (when the cards change)
  keeps the note, `patches ship` carries it onto the shipped patch, and `patches check` fails until
  it has a title and at least one line, and on a blank line, a line over 200 characters, or a
  ruling (`R656`), an issue or card number (`#88`) or a `§`.
- **The title** is a short phrase with no version, which the page shows beside it: "Gary the Gambler
  buffed", not "Patch: Buff Gary the Gambler".
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
  (`apps/web/src/patches/releases.ts`), with a summary and its technical changes. The line opens at
  its major, the n-th major the n-th line (R1369000), so no version is written there. The page shows
  one bubble per line, newest open.
- **Technical changes.** A change players notice that ships no card data (a mode, a screen, the AI,
  the engine, sound) adds one line to the `technical` of its major version's line, the newest, in its
  own pull request, in the style of the lines there ("Background music and a main menu theme."). A
  major version's changes before its first card patch are its line's too: v0.3.0, the Rust rewrite,
  is Meditative's.
- **The technical log.** A pull request that ships no card data can carry one player line in its
  body, under a `## Player note` heading (or as `Player note: …`), written by these rules.
  `cargo jackioh patches log <base>..<head>` logs each App, AI and Internal commit of the range on the
  newest shipped X in `crates/cards/patches/technical.json`, with that line (R1369000); the page shows
  the App and AI lines under their X, and Internal ones stay in the record.

## Win rates in a balance patch

A balance patch's `notes` that cite a card's win rate quote `cargo jackioh stats compare`, never two plain
`stats` runs' rows (R1284000): about 150 of 2,000 All Random games hold a given card, and #1149's own notes
put that noise at five points. Play the card into one seat's deck in every game on a worktree of the base
commit (`scripts/worktree.sh`), `cargo jackioh stats --force <id> --games N --out before.jsonl`, and the same
with `--out after.jsonl` on the branch; then `cargo jackioh stats compare --before before.jsonl --after
after.jsonl --card <id>`. Quote both rates and the paired difference with their intervals, and the line about
`SHADOW_BAN` or `SHADOW_WATCH` if it prints one. A forced run's file is never `stats-import`ed: it is not a
sample of All Random. The base build needs the flag, so the first such comparison is the one after #1284 lands.

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
  what the fragment claims (no version: it is named when it ships), and the gates (CLAUDE.md,
  Commands).
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
