# `.harness/`: the night bot's switches

The bot reads these files from `main` and never changes them: `.harness/` is one of its
forbidden paths, so a change it makes here is reverted before review and refused at delivery.

| File | What it does |
|---|---|
| `config.json` | Every knob: the window (21:00 to 07:00 America/Chicago), the model and effort, turn caps, time budget, review rounds, the usage stops, auto-merge, the checks it runs, the required CI checks, the suggestion cap |
| `trust.txt` | Who may command the bot, and at which level |
| `HALT` | Absent normally. Commit a file here (any content) to stop every model call at once; delete it to allow them again. `/harness halt` and `/harness start` are the everyday switch; this file is the one nobody but a committer can lift |

See [`bot/README.md`](../bot/README.md) for how the bot works.
