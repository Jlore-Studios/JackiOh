# The AI training lanes

Two lanes that keep improving JackiOh's AI (`crates/ai/`, SPEC §9.9), each a Devin session after
another on the training box, each gated by a fixed number of games against the AI on `main`. They
succeed #55's ladder (`ladder/`). The design is docs/v0.3.0/README.md §8; the interfaces are
docs/v0.3.0/SURFACE.md §14.

| Lane | Goal | Standing prompt |
|---|---|---|
| `improve` | a stronger AI: beat its predecessor in 85 of 100 games and random in 90 of 100 | [`improve.md`](improve.md) |
| `unban` | an AI that plays well with fewer shadow-banned cards (R186): strictly fewer bans than its predecessor, while beating it in 75 of 100 and random in 90 of 100 | [`unban.md`](unban.md) |

## How a lane runs

The training box (an always-on EC2 `m7i.xlarge`, set up by #306's part 39) runs two systemd services,
`jackioh-train@improve` and `jackioh-train@unban`, each as its own Linux user (`agent-train-improve`,
`agent-train-unban`) with its own checkout, Devin login and GitHub token. Each runs
[`loop.sh`](loop.sh) `<lane>` forever (`Restart=always`):

1. Fetch `main`, reset the lane's branch `ai/<lane>` to it, build `jackioh` from it and keep that
   binary as `~/parent-jackioh`. The parent is always the AI on `main`.
2. Run one Devin session on the lane's standing prompt
   (`devin -p --prompt-file training/<lane>.md --model "$DEVIN_MODEL" --permission-mode dangerous
   --respect-workspace-trust false --export ~/logs/<lane>-<t>.json`). Devin changes the AI, measures
   it with dry runs of the gate, and commits only once a real run of the gate has passed. A session
   ends after a promotion or after four hours.
3. When the session ends with a promotion commit, rebase it on `main`. If `main`'s AI, engine or
   cards changed meanwhile, the parent changed, so the promotion is dropped (kept as a local
   `ai/<lane>-stale-<t>` branch and noted in `attempts.md`) and the cycle starts again at 1.
   Otherwise the loop re-checks it against the parent (`cargo jackioh promote --verify`), pushes
   `ai/<lane>` and opens a pull request titled `AI gen <N> (<lane>): <what changed>` with the gate's
   report as its body, with auto-merge (squash) on.
4. Sleep 60 seconds and repeat. While a pull request from `ai/<lane>` is open, the loop waits for it.

A lane never merges anything itself: CI does, through branch protection, once its checks pass.

## What a lane may change

Only `crates/ai/**` and `training/history/**`. A pull request from an `ai/*` branch that touches
anything else is refused by CI, and so is one whose `crates/ai/generation.json` is not `main`'s
plus one (part 30's `training-gate` job). That job builds `main`'s `jackioh` and the branch's, and
runs `promote --lane <lane> --verify` again on the same seeds: a lane's own claim is never trusted.
Branch protection's "require branches to be up to date" makes a lane whose parent moved run again.

The engine (`crates/engine`), the cards (`crates/cards`), every test outside `crates/ai`, the gate
(`crates/tools/src/promote.rs`, `arena.rs`) and its thresholds (`TRAINING_*` in
`crates/engine/src/config.rs`) are never a lane's to change.

## The gate

`cargo jackioh promote --lane <lane> --parent-bin <parent> [--dry-run] [--verify]` plays
`TRAINING_GAMES` (100) games against SPEC §10.7's random policy and 100 against the parent:

- game `k`'s seed is `<lane>:<tree>:<k>`, where `<tree>` is `git rev-parse HEAD:crates/ai/src`, so
  the seeds name the AI being measured and committing `generation.json` does not move them; the
  same seeds are played against both opponents;
- the candidate sits p1 on odd games and p2 on even ones;
- each seat's deck is `build_ai_deck` over all three sets minus its own AI's shadow ban, with no
  handicap (Easy both). The random seat is dealt the candidate's ban list, as the quality gates
  deal it;
- the AI plays at `AI_GATE_BUDGET`, the browser's own budget, so the gate measures the AI that ships.

| Lane | vs random | vs parent | Shadow bans |
|---|---|---|---|
| `improve` | ≥ 90 of 100 (`TRAINING_IMPROVE.vs_random`) | ≥ 85 of 100 (`TRAINING_IMPROVE.vs_parent`) | — |
| `unban` | ≥ 90 of 100 (`TRAINING_UNBAN.vs_random`) | ≥ 75 of 100 (`TRAINING_UNBAN.vs_parent`) | strictly fewer than the parent's |

A draw is not a win, and neither is a game that ended without a result. Exit 0 on a pass, 1 on a
failed gate. The report (Markdown) goes to stdout.

- `--dry-run` measures and writes nothing. It runs on a working copy with uncommitted changes in
  `crates/ai/src` (on HEAD's seeds, and says so).
- Without a flag, a pass writes `crates/ai/generation.json` and appends the same object to
  `training/history/<lane>.jsonl`. It refuses to run while `crates/ai/src` has uncommitted changes,
  since the seeds come from the committed tree: commit the change first, then run it, then amend the
  two files into that commit.
- `--verify` (CI, and `loop.sh` before it pushes) recomputes and compares every field of the
  branch's `generation.json` except `date` and `parent`.

The games run on rayon's threads (`RAYON_NUM_THREADS` is respected); no result depends on the
thread count.

### The arena and the agent

- `cargo jackioh arena --a self|random|bin:<path> --b self|random|bin:<path> --games N --seed <base>
  [--out <dir>]` plays games `1..=N` with seeds `<base>:<n>`, agent `a` on p1 in odd games. It is
  the free-form version of the gate, for experiments on other seeds than the gate's. Per game it
  prints duel.ts's line (who won, why, the hash, whether the log replays, decision times) and a
  tally at the end.
- `cargo jackioh agent` is this build's AI speaking JSON lines on stdin and stdout (`info`,
  `decide`, `quit`; SURFACE §14.1). `bin:<path>` spawns `<path> agent`. The referee holds the true
  state and sends each agent only `redact(state, seat)` (R185). The agent's answers name its rng
  cursor, so `self` and `bin:<the same build>` play identical games.

## Where the numbers go

| What | Where |
|---|---|
| Every game the arena plays (a promotion run or an experiment): one `GameRecord` (R376) per line, `source: "dev"`, `mode: "random"` | `~/training-out/<lane>/<date>.jsonl` (`$JACKIOH_TRAINING_OUT`; `arena` without it prints to stdout) |
| The same records in Postgres, loaded once per finished day when the box has `DATABASE_URL` | `jackioh-server stats-import`, read back with `jackioh-server stats-cards --source=dev` |
| A promotion's gate numbers | `training/history/<lane>.jsonl` and `crates/ai/generation.json`, committed with it |
| What each Devin session tried | `~/training-out/<lane>/attempts.md` |
| The gate's report for each pull request | `~/training-out/<lane>/promote-<t>.md` |
| The loop's log, and each Devin session's export | `~/logs/<lane>.log`, `~/logs/<lane>-<t>.json` |

## Stopping, starting and watching a lane

```sh
sudo systemctl stop jackioh-train@improve      # stop the lane now (its Devin session ends with it)
sudo systemctl disable jackioh-train@improve   # and keep it stopped across reboots
sudo systemctl enable --now jackioh-train@improve
systemctl status jackioh-train@unban
tail -f ~agent-train-unban/logs/unban.log
```

To refuse a promotion, close its pull request: the loop waits while one is open, and starts again
from `main` once it is closed. A lane's thresholds change only by a pull request from a person to
`crates/engine/src/config.rs`.

## Reading the history

`crates/ai/generation.json` is the AI on `main`. `training/history/<lane>.jsonl` holds one line per
promotion of that lane, oldest first:

```json
{"generation":8,"lane":"improve","parent":"<main commit>","tree":"<crates/ai/src tree>","vsRandom":"93/100","vsParent":"87/100","shadowBan":11,"parentShadowBan":11,"date":"2026-11-02"}
```

- `generation`: the parent's plus one. Generations count across both lanes: they share `main`.
- `parent`: the `main` commit the parent was built from; `tree`: the AI's source tree, which names
  the seeds (`<lane>:<tree>:1` … `:100`), so anyone can re-run the gate:
  `git checkout <parent> && cargo build --release -p jackioh-tools && cp target/release/jackioh /tmp/parent`,
  then on the promotion's commit `cargo jackioh promote --lane <lane> --parent-bin /tmp/parent --verify`.
- `vsRandom`, `vsParent`: the candidate's wins out of `TRAINING_GAMES`.
- `shadowBan`, `parentShadowBan`: how many cards each AI keeps out of its own decks (R186).

```sh
jq -r '[.generation, .date, .vsRandom, .vsParent, .shadowBan] | @tsv' training/history/*.jsonl
git log --oneline -- crates/ai/generation.json   # every generation's pull request
```

Generation 0 is the port of the last TypeScript AI (`packages/ai` at `91cc43c`, which won 94 of 100
against random, 35 of 50 against greedy and 47 of 50 as Hard against Easy). Its record says
`"lane": "port"` until part 40 measures it in Rust; the histories start empty.
