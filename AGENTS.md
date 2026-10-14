# AGENTS.md

Entry point for every coding agent in this repo (Codex, Gemini, Copilot, Muse, Devin, Claude). JackiOh is a 1v1
card game: Hearthstone-style mana, combat and keywords on Yu-Gi-Oh-style lanes with a hidden trap backrow.

- **`CLAUDE.md` is the contract for every agent, not only Claude.** Read it first: the numbered rules of
  engagement (code cites them, "CLAUDE.md rule 7"), the commands, CI, the architecture and the bots' limits.
- **`SPEC.md`** (the notes in `spec/`) is the only source of rules, cards and engine design. On any conflict
  the spec wins and the other doc is the bug.
- **Issues, pull requests and commits** follow `CONTRIBUTING.md` (its short form) and
  `docs/issues-and-patches.md` (in full): titles, labels, the pull request template, patch and ruling
  numbers. Never add `bot:build` while relabelling: it queues a build.
- **Paths the bots may not change:** `.github/`, `bot/`, `.harness/`, `.squishy/`, `.claude/` (the full list
  is `forbidden_paths` in `.harness/config.json`).
- **Skills** are in `.claude/skills/` (`maxi-tail`, `ui-walkthrough`, `fullsend`). An agent without skill
  support may read a `SKILL.md` as plain instructions. `GEMINI.md` and `.github/copilot-instructions.md`
  point here.
