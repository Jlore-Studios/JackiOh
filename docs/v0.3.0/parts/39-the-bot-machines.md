# v0.3.0 (part 39 of 40): the bot machines: six normal lanes and two training lanes

Part 39 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 5 (training lanes) |
| Starts after | part 38 |
| Branch | a pull request to `main` |
| Builder | a person (or a session a person runs and watches) |
| Notes | a comment on this issue when done |

Unlabelled on purpose: #306 holds labels until a person says the run is ready. The Plan below is between the night bot's plan markers, so if a person later queues this issue for a bot, it builds from this plan without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Rewrite the bot machines' setup for six normal lanes and two always-on AI training lanes (#306): the night box keeps six harness lanes; a new training box runs `jackioh-train@improve` and `jackioh-train@unban` as systemd services that never stop and never power the box off. The harness's training-box reservation, which those lanes replace, is deleted.

**Who builds it: a person**, or a Claude Code session a person runs and watches, because it changes `.github/`, `.harness/`, `.squishy/` or `bot/`, which no bot may change (CLAUDE.md, The night bot). It is not a fullsend part: build and test normally.

### 2. Files to touch

| File | new/change/delete | What |
|---|---|---|
| `.harness/providers.json` | change | `machine_parallel: 6` (l.3); remove the `devin-train` provider (l.193-209) and its id from `priority` (l.6) |
| `bot/harness/plan.py` | change | delete `training_ids`, the training branch of `machine_cap`/`machine_full` (l.481-506) and its docstring |
| `bot/harness/status.py, bot/harness/dashboard.py` | change | "n of 6 on the night box"; drop the training-box line (l.23-37; l.290-348) |
| `bot/harness/providers.py` | change | `only_labels` parsing and `takes_item`'s training rule go (l.272-279, l.510-514) if nothing else uses them |
| `bot/tests/test_basics.py, test_providers.py, test_dashboard.py` | change | l.39 (10, 6); the devin-train cases go; the dashboard text |
| `bot/machine/setup.sh` | change | a `--training` mode: no idle-stop timer, users `agent-train-improve` and `agent-train-unban`, Rust 1.97 + `gh` for them, `training/loop.sh` as `jackioh-train@.service` (Restart=always, RestartSec=60, `Environment=DEVIN_MODEL=swe-2-max`, `RAYON_NUM_THREADS=2`, `JACKIOH_TRAINING_OUT=%h/training-out/%i`), `systemctl enable --now jackioh-train@improve jackioh-train@unban` |
| `bot/machine/README.md` | change | the training box: always on, its two services, cost, how to stop a lane |
| `bot/README.md, CLAUDE.md (The night bot)` | change | "up to ten items … at most six on the night box"; the training lanes outside the harness, `training` label gone |
| `the training box (EC2 m7i.xlarge, 4 vCPU, 100 GB gp3, us-east-2, SSM only, tag Name=jackioh-train-box)` | build | `bot/machine/on-machine.sh` with TAG=jackioh-train-box runs `setup.sh --training`; log in Devin as both users (`devin auth login --force-manual-token-flow`); put a fine-grained GitHub token (contents and pull requests on this repository) in each user's `gh auth login` |


### 3. Steps

1. Change the harness and its tests; `cd bot && python3 -m unittest discover -s tests -t .` green.
2. Change `setup.sh` and the READMEs.
3. Build the training box and start both services; check `systemctl status`, a reboot (`sudo reboot`) and that both come back.
4. Check one loop iteration end to end on each lane (`journalctl -u jackioh-train@improve -f`): the parent binary builds, Devin starts, `~/training-out/<lane>/` gets GameRecords.

### 4. Tests

- bot selftest green; both services survive a reboot (V27).

### 5. Done when

- [ ] Night box: 6 lanes in the harness; training box: 2 services always running; #306 told the monthly cost.

### 6. Risks

- Devin's SWE-2 is free on the CLI only until 2026-10-16; `DEVIN_MODEL` is the knob.
- Devin has been off in the harness since 2026-10-05 because its CLI stopped answering (CLAUDE.md, The night bot). Before step 4, run `devin --version` and a one-line prompt as each training user. If either still fails, leave the services enabled (systemd retries every 60 s, which costs nothing) and say so on #306; the lanes start on their own once Devin answers.
- An always-on m7i.xlarge costs about $150 a month.

<!-- /jackioh-bot:plan -->
