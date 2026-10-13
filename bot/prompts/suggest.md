<!-- version: 2 -->
# Suggest improvements to JackiOh

The night bot of `$repo` has nothing queued tonight. Your job is to propose up to $count
improvements the maintainer might want built later. You are read-only: read, search and run
read-only commands, and change nothing.

JackiOh is a 1v1 card game: Hearthstone-style mana, combat and keywords on Yu-Gi-Oh-style lanes
with a hidden trap backrow, with a pure seeded Rust engine, a practice AI, an online server and a
React client. The spec (`spec/`, which `SPEC.md` points to) is the design, `BUILD.md` the work
order, `CLAUDE.md` the rules of the codebase.

## What makes a good suggestion

- **Worth it over the long term**: better games (balance, clarity, pacing, new modes that fit the
  design pillars in SPEC §1), a stronger or more fun practice AI, a smoother client, a sturdier
  server, faster or more trustworthy tests, less code for the same behaviour.
- **Grounded**: it cites the files, SPEC sections, rulings or measurements that show the problem
  or the opportunity. Nothing you cannot point at.
- **Buildable by the bot**: a person approves it by commenting `approve` (or by adding the
  `bot:build` label) and rejects it by commenting `reject`, which closes it, and the night bot
  can then build it inside this repository with tests. Say how big it is.
- **New**: none of the open or recently closed issues below already covers it, and it is not a
  suggestion the maintainer closed.

Use subagents to survey different areas in parallel, then keep only the strongest ideas. Fewer,
better suggestions beat $count weak ones; zero is a fine answer.

## Issues that already exist

$existing

## Your final message

The very first line must be a single HTML comment carrying JSON on one line, with nothing before
it:

    <!-- suggestions: [{"title": "Patch v0.3.Y: what it does", "body": "Markdown body"}] -->

Each title follows `docs/issues-and-patches.md`, Titles, since nothing retitles it later: a small
change to the game starts as `.github/ISSUE_TEMPLATE/micro-patch.yml`'s title does (`Patch v0.3.Y: `
today), a bigger one as `patch.yml`'s (`Patch v0.3.X: `), tooling or CI `Architecture: ` or `CI: `,
and the bots themselves `Night bot: `, each followed by what it does.

Each body has these sections: `## Why`, `## Proposal`, `## Where it lands` (files and SPEC
sections), `## Acceptance criteria` (a checklist a reviewer can verify), and `## Size and risks`.
At most $count suggestions. After the first line you may explain your choices in prose.
