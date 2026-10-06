# `.squishy/`: Squishy's switches

Squishy (`@squishy-squooby`, #60) is the night bot's harness run as a second bot: every Squishy
workflow sets `HARNESS_HOME=.squishy`, and the harness reads these files from `main` instead of
`.harness/`. Like `.harness/`, this is a path neither bot may change.

| File | What it does |
|---|---|
| `config.json` | The same knobs as `.harness/config.json`, with Squishy's account, no suggestions and no quiet check, and its `identity`: its name, its slash command (`/squishy`), its label and branch prefixes (`squishy:`, `squishy/`), its state and journal branches, its workflow (`squishy-run.yml`), its marker, its modes (`oneshot`, `split`, `split-bot`) and the other bot whose issues it leaves alone |
| `providers.json` | One subscription, `claude-squishy`: Squishy's own Claude Max account (`CLAUDE_CODE_OAUTH_TOKEN_SQUISHY`), Opus for every role, on GitHub's runners |
| `HALT` | Absent normally. Commit a file here to stop Squishy's model calls; `/squishy halt` and `/squishy start` are the everyday switch |

Its trust list is the night bot's, `.harness/trust.txt`. See [`bot/README.md`](../bot/README.md#squishy).
