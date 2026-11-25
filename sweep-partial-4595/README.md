# #4595's sweep, pass 1, partial (not for main)

`cargo jackioh sweep --json <ids>` (pass 1, R186) over the Infinite Assault cards, run on
`claude/issue-4595-assault-ai` after #4872's fix of IA #32 Glitch Specter (AI generation as of main at
#4872's merge). Each card has four lines: base and Radiant faces at easy and hard, 8 seeds each
(`sweep:<tier>:<id>:<n>`, fixed seeds, so a rerun of a card repeats it).

- `pass1-partial.jsonl`: 171 cards done (684 lines). No errors since the fix; flags are `neverPlayed`
  (45 cards) and `timeout` (IA #29, #123, #149, one or two raw slow decisions each).
- `remaining.txt`: the 69 ids still to sweep.

Resume: `cargo build --release -p jackioh-tools` (with `CARGO_INCREMENTAL=0`; the incremental caches
fill a cloud container's disk), then shard `remaining.txt` over the cores:
`./target/release/jackioh sweep --json <ids…> > p1_<k>.jsonl`; then
`jackioh sweep --pass2 pass1-partial.jsonl,p1_0.jsonl,… ` (sliced likewise) and
`jackioh sweep --report <pass-1 files> <pass-2 files>`.
