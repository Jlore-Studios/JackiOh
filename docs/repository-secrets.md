# Repository secrets and variables

Every secret and variable this repository's workflows read (Settings -> Secrets and variables ->
Actions on `jlore-studios/jackioh`): what each is and which workflows read it. **Read this before
asking a person to add a secret.** A secret listed here is one the workflows already use, so it has
been set unless its row says otherwise. Ask a person only for a secret this page does not list, or
one whose row says it is missing or expired.

A secret's value cannot be read back, only replaced, and no session or bot can list them. To tell
whether one is set, read the newest run of a workflow that reads it: each workflow with an optional
secret says when it is missing (`::notice::<NAME> is not set …`) and otherwise uses it. `GITHUB_TOKEN`
is GitHub's own and is not listed. The Workers' own secrets (`SUPABASE_SECRET_KEY`, `DATABASE_URL`,
`CODE_PEPPER` and the rest) live in Cloudflare, not here: [deploy-cloudflare.md](deploy-cloudflare.md)
section 3.

`bot/tests/test_repository_secrets.py` (the bot selftest, on every pull request) holds the tables
below to the workflows. A workflow that names a secret or variable missing from them fails it, and so
does a row that no workflow reads and that does not say so. So a change that adds a secret adds its
row in the same pull request.

## GitHub tokens

| Secret | What it is | Read by |
| --- | --- | --- |
| `BOT_GITHUB_TOKEN` | A classic token of `@jgoetzmann-bot` (`public_repo`, `workflow`). A pull request or push made with it starts CI, which one made with `GITHUB_TOKEN` never does. It also moves `_production`. [bot/README.md](../bot/README.md), Setting it up | bot-night, bot-commands, bot-status, patches-ship, spec-fold, winrates-refresh, promote-staging, promote-production, releases |
| `SQUISHY_GITHUB_TOKEN` | A classic token of `@squishy-squooby`, Squishy's account ([bot/README.md](../bot/README.md), Squishy) | squishy-run, squishy-commands, bot-status |
| `LEARN_LANE_TOKEN` | A fine-grained token that writes contents and pull requests on this repository alone, for the shadow-ban sweep's pull request (R1678000): one opened with a run's own token would start no CI. Named for the retired learn lane, which used it first. It expires on the date it was given, so renew it before then; without it, or once it has expired, the sweep fails at its push and leaves its record as the run's artifact (`record.patch`) | sweep |

## Model subscriptions

The bots' logins, one secret per subscription ([bot/README.md](../bot/README.md), Subscriptions and
Setting up each subscription). A run's plan picks one by name, so the workflows also read them as
`secrets[<name>]`.

| Secret | What it is | Read by |
| --- | --- | --- |
| `CLAUDE_CODE_OAUTH_TOKEN` | `claude-1`'s token, from `claude setup-token` | bot-night, bot-commands, bot-status, claude-wake, squishy-run, squishy-commands, hygiene |
| `CLAUDE_CODE_OAUTH_TOKEN_2` to `CLAUDE_CODE_OAUTH_TOKEN_8` | `claude-2` to `claude-8`'s tokens, the same way | bot-night, bot-commands, bot-status, claude-wake, squishy-run, squishy-commands |
| `CLAUDE_CODE_OAUTH_TOKEN_SQUISHY` | Squishy's own Claude subscription | bot-night, bot-commands, bot-status, claude-wake, squishy-run, squishy-commands |
| `CODEX_AUTH_JSON` | The whole of `~/.codex/auth.json` after `codex login` | bot-night, bot-commands, bot-status, claude-wake, squishy-run, squishy-commands |
| `MUSE_AUTH` | `~/.config/muse/auth.json` after `muse login`, or a Meta API key | bot-night, bot-commands, bot-status, claude-wake, squishy-run, squishy-commands |

## Databases

Supabase's two projects, JackiOh-Production and JackiOh-Staging ([deploy-cloudflare.md](deploy-cloudflare.md)
section 8). Every address is a session pooler URI (Supabase -> Connect -> Session pooler), since
GitHub's runners have no IPv6.

| Secret | What it is | Read by |
| --- | --- | --- |
| `STATS_DATABASE_URL` | Production's read-only statistics login, `stats_reader` (R1933001): the card statistics' totals, which `jackioh-server stats-cards --window` reads. Set: winrates-refresh's run 43600686946 (2026-11-14) read production through it | winrates-refresh, db-size (production) |
| `STATS_IMPORT_DATABASE_URL` | Staging's statistics importer, `stats_importer`, which the development lanes file their games through. cutover also takes its pooler host for production's addresses | gather, db-size (staging), cutover |
| `PRODUCTION_STATS_IMPORT_DATABASE_URL` | Production's `stats_importer`, set on 2026-11-03 by cutover's `stats-logins` (R1933001) | gather |
| `PRODUCTION_DATABASE_URL` | JackiOh-Production's session pooler URI for its owner role, as Supabase shows it | cutover |
| `PRODUCTION_DB_PASSWORD` | That role's password, which cutover percent-encodes into the URI | cutover |
| `PRODUCTION_SUPABASE_SECRET_KEY` | JackiOh-Production's secret API key | cutover (`check-production`) |
| `SUPABASE_ACCESS_TOKEN` | A Supabase personal access token, for the Management API (both projects' sign-in settings and mail) | cutover |

## Cloudflare and mail

| Secret | What it is | Read by |
| --- | --- | --- |
| `CLOUDFLARE_API_TOKEN` | The Edit Cloudflare Workers template, plus Containers, for `wrangler deploy` | deploy-staging, promote-production |
| `CF_ACCESS_CLIENT_ID` | The Access service token's id, so a check can reach staging behind Access | deploy-staging, cutover |
| `CF_ACCESS_CLIENT_SECRET` | That service token's secret | deploy-staging, cutover |
| `CLOUDFLARE_BILLING_TOKEN` | A custom token with the account's Billing: Read alone | cf-spend |
| `RESEND_API_KEY` | A Resend Full access key, to add jackioh.com and make the sending keys | cutover (`email`) |

## Kept here, read by no workflow

| Secret | What it is | Read by |
| --- | --- | --- |
| `CODE_PEPPER` | The Workers' pepper, its one copy that can be read back, since a Worker's secret cannot ([deploy-cloudflare.md](deploy-cloudflare.md) section 3) | no workflow: a person setting it on a new Worker |

## Variables

| Variable | What it is | Read by |
| --- | --- | --- |
| `CI_GATE_ENVIRONMENT` | `ci-runners`, the environment the runner gate holds CI in ([bot/README.md](../bot/README.md), The runner gate). Deleting it turns the gate off | ci, super |
| `GATHER_ON_PATCH` | `false` turns the gather lane off | gather |
