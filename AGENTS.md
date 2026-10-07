# AGENTS.md

Shared entrypoint for every coding agent working in this repo (GPT/Codex,
Claude, Gemini, Copilot). One-liner: JackiOh is a 1v1 card game —
Hearthstone-style mana, combat and keywords on Yu-Gi-Oh-style lanes with a
hidden trap backrow.

## Source of truth

`SPEC.md` is the only source of rules, cards and engine design. It is kept as
notes in `spec/` (one per section, one per ruling; `SPEC.md` points there). If
any doc disagrees with the spec, the spec wins and the doc is the bug.

## Read first

| Read | For |
|---|---|
| `SPEC.md`, `spec/` | Rules, cards, engine design |
| `BUILD.md` | Work order, milestones, acceptance criteria, definition of done |
| `REVIEW.md` | Audit procedure |
| `CLAUDE.md` | Repo layout, rules of engagement, commands (applies to all agents, not just Claude) |
| `docs/ADDING_CARDS.md` | Before any card work: the files a card touches, templates, order, gates |
| `bot/README.md` | Night-bot workflow (issues, labels, safety) |

Crate and package contracts live in the READMEs of `crates/engine`,
`crates/cards`, `crates/ai`, `crates/server`, `crates/tools`, `crates/wasm`,
`apps/web` and `e2e`; deployment in `docs/architecture.md`.

## Rules every agent follows

The rules of engagement in `CLAUDE.md` bind every agent: work `BUILD.md` in
order, keep `crates/engine`, `crates/cards` and `crates/ai` pure (no clock,
I/O, environment, threads or OS randomness; their `clippy.toml` and CI's
`cargo clippy --workspace --all-targets -- -D warnings` enforce it), return `Effect`s from card scripts
without mutating state, send intent and render `viewFor` on the client, and
keep every number a named constant. Rulings follow `CLAUDE.md` rule 3 (a new
note in `spec/rulings/` plus a test named after it, checked by
`cargo jackioh spec check`). `CLAUDE.md` lists the commands.

## Model notes

- GPT/Codex: this file is your contract. `CLAUDE.md` holds the full rules.
- Claude: `CLAUDE.md` is your contract; this file is the shared subset.
  Skills live in `.claude/skills/` (see `.claude/skills/ponytail/SKILL.md`).
- Gemini: `GEMINI.md` points here; there is no separate Gemini contract.
- Copilot: `.github/copilot-instructions.md` points here; there is no
  separate Copilot contract.
