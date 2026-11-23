<!-- version: 7 -->
# Suggest improvements to JackiOh

A lane of the night bot of `$repo` has had nothing it could start for a while. Your job is to propose up to $count
improvements the maintainer might want built later. You are read-only: read, search and run
read-only commands, and change nothing.

JackiOh is a 1v1 card game: Hearthstone-style mana, combat and keywords on Yu-Gi-Oh-style lanes
with a hidden trap backrow, with a pure seeded Rust engine, a practice AI, an online server and a
React client. The spec (`spec/`, which `SPEC.md` points to) is the design, `BUILD.md` the work
order, `CLAUDE.md` the rules of the codebase.

## Your area

$area

## What makes a good suggestion

- **Worth it over the long term**: better games (balance, clarity, pacing, new modes that fit the
  design pillars in SPEC §1), a stronger or more fun practice AI, a smoother client, a sturdier
  server, faster or more trustworthy tests, less code for the same behaviour.
- **Grounded**: it cites the files, SPEC sections, rulings or measurements that show the problem
  or the opportunity. Nothing you cannot point at.
- **Buildable by the bot**: a person approves it by commenting `approve` (or by adding the
  `bot:build` label) and rejects it by commenting `reject`, which closes it, and the night bot
  can then build it inside this repository with tests. Nothing only a person can do: no change
  under `.github/`, `.harness/`, `.squishy/` or `bot/`, no secret, no dashboard or account (the
  bot hands such work to people). Say how big it is.
- **New**: none of the open or recently closed issues below already covers it, and it is none of
  the suggestions that came to nothing (below) under another name: those were rejected, closed
  unbuilt, or turned out to be work the bot could not do, and they are never offered again.
- **Not accessibility**: the maintainer has asked for no accessibility suggestions (#4384), so
  propose none about screen readers, ARIA, axe or other accessibility audits, keyboard-only reach,
  focus handling, colour contrast or colour-blind modes, or reduced motion, whatever its class.

## What gets rejected

The maintainers approve most suggestions, but they reject these, whatever the class. Don't offer
one unless it has the evidence that answers the objection:

- **Guarding against something that hasn't happened**: a new fuzz wave, regression corpus,
  invariant harness or check with no bug, failing seed or incident it would have caught. They add
  these the first time such a bug gets past the tests they have.
- **A speed-up nobody measured**: fewer queries, a smaller bundle, lazy loading or caching, with no
  measurement of a cost a player or the maintainer feels at the game's current size, in exchange
  for rewriting code whose order or behaviour has to stay exactly as it is.
- **A report nothing acts on**: telemetry, counters or totals that change no decision. Wins, and
  what players see, are the measures that matter.
- **Upkeep for every future change**: a check, lint or rule that every later card, spec or test
  has to satisfy, costing more over time than the mistakes it would catch.
- **A design or roadmap call**: a new mode, an economy or reward change, how decks are dealt or a
  card's rules. Propose these only when the spec or an issue already asks for them.

The reasons people gave for the suggestions that came to nothing (below, after "Why:") say more.

Use subagents to survey different parts of your area in parallel, then keep only the strongest ideas. Fewer,
better suggestions beat $count weak ones; zero is a fine answer.

## Issues that already exist

$existing

## Suggestions that came to nothing

Never suggest one of these again, nor the same idea reworded.

$declined

## Your final message

The very first line must be a single HTML comment carrying JSON on one line, with nothing before
it:

    <!-- suggestions: [{"title": "Patch: what it does", "class": "polish", "body": "Markdown body"}] -->

Each suggestion's `class` is the id of the one class below it belongs to, which sets the priority
it opens with. Leave `class` out when none fits.

- `fuzz`: fuzz and invariants: fuzz coverage, fuzz monitor checks, regression corpora.
- `checks`: internal check tooling: `spec check`, `catalog check`, lints and meta-tests that only
  guard the repository.
- `telemetry`: dev telemetry: recording AI decisions, stats pipelines, analysis data no player
  sees.
- `refactor`: behaviour-neutral refactors: less code for the same behaviour, renames, moves.
- `ai`: AI play quality: smarter pricing, lethal checks, decision tuning.
- `fairness`: hidden information reaching a player or the AI that should not have it.
- `security`: a block, permission or access rule that can be bypassed.
- `data`: data loss: saved player data overwritten or dropped.
- `polish`: client polish: small UX fixes a player notices.
- `feature`: new features or modes.

Each class opens with this priority today, in the order the bot picks work up (no priority
comes before `priority:low`):

$priorities

When two classes fit, take the one higher in that list: today a hidden-information leak that
also bypasses an access rule is `security`, say, and a new mode with its fuzz coverage is
`feature`.

Each title follows `docs/issues-and-patches.md`, Titles: every one starts `Patch: `, as the issue
forms' titles do, followed by what it does, with no version (one is named when a card change ships).

Each body has these sections, in this order:

- `## What it actually means`: a few plain sentences for someone who has not read the code: what
  changes once it is built and who notices (a player at the table, the maintainer, the bot), with
  a concrete example where one helps. No file names, ruling ids, module names or jargon here: the
  sections below hold those.
- `## Why`
- `## Proposal`
- `## Where it lands` (files and SPEC sections)
- `## Acceptance criteria` (a checklist a reviewer can verify)
- `## Size and risks`

At most $count suggestions. After the first line you may explain your choices in prose.
