# `.harness/`: the night bot's switches

The bot reads these files from `main` and never changes them: `.harness/` is one of its
forbidden paths, so a change it makes here is reverted before review and refused at delivery.

| File | What it does |
|---|---|
| `config.json` | Every knob but the subscriptions: turn caps, time budget, review rounds, self-check rounds (`max_self_check_rounds`), auto-merge, the checks it runs (`"machine": false` leaves one to CI when the run is on the bot's machine), the required CI checks, the suggestion cap, the quiet check, and the easy rule's limits (`easy`: `max_files`, `max_lines`, `off_limits`, `generated`; `bot/harness/easy.py`) |
| `providers.json` | The subscriptions it may spend (Claude accounts, Codex, agy, Muse, Devin): each one's CLI, model, effort and tier (`weak`, `medium` or `strong`), any `extra_models` it runs (Sonnet, medium, on every Claude account, which switches between it and Opus; `takes_over` makes one a stand-in for another subscription), `fix_effort` for fix passes, `self_check`, its login (a secret's name, or `machine`), the runner its model job runs on (`runs_on`), hours and limits, how many run at once (`max_parallel`, `machine_parallel`, and `plan_lanes` for the planning lane), the usage order (`priority`) and each tier's model order (`tiers`). A Claude account without its secret set sits out; `enabled: false` turns any one off ([Subscriptions](../bot/README.md#subscriptions), [the machine](../bot/machine/README.md)) |
| `trust.txt` | Who may command the bot, and at which level |
| `HALT` | Absent normally. Commit a file here (any content) to stop every model call at once; delete it to allow them again. `/harness halt` and `/harness start` are the everyday switch; this file is the one nobody but a committer can lift |

See [`bot/README.md`](../bot/README.md) for how the bot works.
