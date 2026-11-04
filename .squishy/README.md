# `.squishy/`: Squishy's switches

Squishy (`@squishy-squooby`, #60) is the night bot's harness run as a second bot: every Squishy
workflow sets `HARNESS_HOME=.squishy`, and the harness reads these files from `main` instead of
`.harness/`. Like `.harness/`, this is a path neither bot may change.

| File | What it does |
|---|---|
| `config.json` | The same knobs as `.harness/config.json` (the same merge gates, `review_paths`, `migration_paths` and `lock_files`, and the same `decision_wait_hours`, 24, #2986, and `decision_assignees`, both people, #3009), with Squishy's account, no suggestions, no quiet check and no hold for an owner (`hold_minutes: 0`: its one account has no other to hold an item from), no after-merge spot-check (`spot_check_every: 0`), no fullsend adoption of a queued parent and no part its reconcile sends back (`fullsend_adopt: false`, `fullsend_send_backs: 0`), no own review run after a build (`own_review_run: false`: its build reviews itself in the same run), no usage history (`usage_history: false`, and its identity's `usage_branch` is empty: only the night bot's loop records one, [The usage history](../bot/README.md#the-usage-history)), and its `identity`: its name, its slash command (`/squishy`), its label and branch prefixes (`squishy:`, `squishy/`), its state and journal branches and no usage branch, its workflow (`squishy-run.yml`), its marker, its modes (`oneshot`, `split`, `split-bot`, `fullsend`) and the other bot whose issues it leaves alone |
| `providers.json` | One subscription, `claude-squishy`: Squishy's own Claude Max account (`CLAUDE_CODE_OAUTH_TOKEN_SQUISHY`), Opus for every role, on GitHub's runners |
| `HALT` | Absent normally. Commit a file here to stop Squishy's model calls; `/squishy halt` and `/squishy start` are the everyday switch |

Its trust list is the night bot's, `.harness/trust.txt`. See [`bot/README.md`](../bot/README.md#squishy).
