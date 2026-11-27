# #4595's sweep of the Infinite Assault: the id lists

The report is `training/history/assault-sweep-2026-11-26.md`. This directory keeps its id lists. The raw
per-card lines (pass 1, pass 2 and the re-measure) are not committed; they are on the branch
`claude/issue-4595-assault-ai-2`, under `sweep-partial-4595/`.

- Base: `main` at `c0b49712` (#4885); cards 001–171 of pass 1 on `c739a07d` (#4872's merge).
- Machine: 4 cores, Intel(R) Xeon(R) Processor @ 2.10GHz; lanes 4 (pass 1), 3 (pass 2).
- `at-risk.txt`: the 139 cards pass 1 put at risk, each swept in pass 2.
- `banned.txt`: the 8 cards the report's `SHADOW_BAN entries` name.
- `fixed.txt`: the 8 cards swept again on the fixed AI (the report's Re-measure): every banned card.
