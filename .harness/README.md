# `.harness/`: the night bot's switches

The bot reads these files from `main` and never changes them: `.harness/` is one of its
forbidden paths, so a change it makes here is reverted before review and refused at delivery.

| File | What it does |
|---|---|
| `config.json` | Every knob but the subscriptions: turn caps, time budget, review rounds, auto-merge, the checks it runs, the required CI checks, the suggestion cap, the quiet check |
| `providers.json` | The subscriptions it may spend (Claude accounts, Codex, Gemini, Muse): each one's CLI, model, secret name, hours and limits, how many run at once and in which order. A subscription without its secret set sits out ([Subscriptions](../bot/README.md#subscriptions)) |
| `trust.txt` | Who may command the bot, and at which level |
| `HALT` | Absent normally. Commit a file here (any content) to stop every model call at once; delete it to allow them again. `/harness halt` and `/harness start` are the everyday switch; this file is the one nobody but a committer can lift |

See [`bot/README.md`](../bot/README.md) for how the bot works.
