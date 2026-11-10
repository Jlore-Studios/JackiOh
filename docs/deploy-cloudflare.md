# Hosting: everything on Cloudflare, staging and production

This is the plan and the runbook for running JackiOh on Cloudflare: what runs where, how a commit
reaches each site, what the repo enforces, what a person sets outside the repo, and what to do when
something goes wrong. Render and Vercel are gone. The database and sign-in stay on Supabase.

## 1. Target state

| | Staging | Production |
| --- | --- | --- |
| Purpose | The team sees main as it was at most two hours ago | What players use |
| Address | https://staging.jackioh.com, behind Cloudflare Access: only the team signs in | https://jackioh.com |
| Worker | `jackioh-staging` (`wrangler.jsonc` `env.staging`) | `jackioh` (`wrangler.jsonc`'s top level) |
| Builds from | `staging`, each time it moves (`deploy-staging.yml`): every two hours, main's newest commit whose push-to-main CI run passed (`promote-staging.yml`) | `production`, once a day: the commit of `staging` the daily super run passed, or sooner on `/promote` (`promote-production.yml`) |
| Game server | one `jackioh-staging-server` container | one `jackioh-server` container |
| Database and sign-in | Supabase JackiOh-Staging, `exmjdaswedxhnzmpzqrq` (the original project) | Supabase JackiOh-Production, `fuzpvmjdejvbjcarypyi` (section 8) |
| Routing and headers | `apps/web/public/_redirects` and `_headers`, `wrangler.jsonc` | the same |
| Dev dashboard (R3074000) | https://admin-staging.jackioh.com, Worker `jackioh-admin-staging`, behind Access for the devs; `DEV_STRICT` off | https://admin.jackioh.com, Worker `jackioh-admin`, behind Access for the devs; `DEV_STRICT` on, so its routes need the two-step code |

Each site is one Worker at one origin (`apps/edge/README.md`). Every path but `/api/*` and `/ws/*` is
a static file from the web build, served without running any Worker code, but one: the engine's
WebAssembly module, stored brotli-compressed (`/assets/*.wasm-*.br`, #3183), which the Worker serves
from the `ASSETS` binding with its `Content-Encoding`, so the browser decodes it as it downloads. Those
two prefixes go to the Worker, which hands them to one container running `crates/server/Dockerfile`'s image. The
container sleeps after ten minutes with no request and no open socket, and the next request starts
it again. A daily cron wakes it.

Each deploy ships the web build and the server together, so a site's client and server are never
from different commits, and production's server no longer runs a day ahead of its client.

`main` stays the only branch anyone merges into. Two branches follow it, one a site (#3275, the
owners' call of 2026-11-05):

| Branch | Site | Moves | Gate |
| --- | --- | --- | --- |
| `main` | none | every merge, through the merge queue | the five required checks |
| `staging` | staging.jackioh.com | every two hours, by fast-forward to a snapshot of `main` (`promote-staging.yml`) | that commit's push-to-main CI run passed |
| `production` | jackioh.com | once a day, to the commit of `staging` the daily super run tested (`promote-production.yml`) | the super run passed every job on that commit |

`staging` is a pointer to "the commit of main that is live on staging.jackioh.com", and it only ever
moves forward along main, so every commit on it is a commit of main. `production` is a pointer to
"the commit of main that is live on jackioh.com". Nothing but `promote-staging.yml` moves `staging`,
and nothing but `promote-production.yml` moves `production`, by merging a pull request whose head is
a commit of `staging` that the super run passed, on `staging` or on the pull request, so its files
always equal that commit's. main moves many times an hour; `staging` holds still between moves, so
the daily super run tests one commit, and the night bot fixes one commit, instead of chasing main.

## 2. How a commit reaches each site

1. A pull request merges into `main` through the merge queue (branch protection requires the five
   CI checks).
2. CI runs again on the push to main, for every piece the queue's build of that same commit skipped:
   a piece it already passed there is not run twice (#3564, `.github/ci/main_scope.py`), and
   anything it cannot tell runs everything. Every two hours `promote-staging.yml`
   (`scripts/promote-staging.sh`) fast-forwards `staging` to main's newest commit whose push-to-main
   CI run passed; it never moves it backwards. GitHub fires this repository's schedules late, so each
   green push-to-main CI run wakes it as well, and it moves `staging` then once the last move is two
   hours old (`STAGING_EVERY_MINUTES`). It pushes with `BOT_GITHUB_TOKEN`, whose push starts
   `deploy-staging.yml`, which checks out that commit, builds the web with `apps/web/.env.staging`,
   checks the bundle, and runs `wrangler deploy --env staging`, which deploys the Worker, the static files and the container together. The server's
   image is `scripts/server-image.sh`'s: named by what it is built from, so it is pushed to
   Cloudflare's registry once (from CI's `deploy-image`, or built on CI's layer cache) and a commit
   that changes none of it reuses it; the WASM modules likewise come from GitHub's cache when
   nothing under `crates/` changed. It then polls `https://staging.jackioh.com/api/catalog`
   (through Access, with a service token) until the server reports the commit in
   `x-deployed-commit`.
3. Once a day the super run (`super.yml`, its `schedule`) runs on `staging`: a schedule always runs on
   the default branch, so its one job there starts the run on `staging`
   (`gh workflow run super.yml --ref staging`), whose commit is then the one it tests. It runs every
   fuzz seed, the full AI gates, golden, the daily tests, coverage, every e2e spec, the db rehearsal
   and the notebooks. When every job passed, its completion wakes `promote-production.yml`, which
   promotes the commit it tested (the daily promotion, section 2.1): a pull request from
   `promote/<date>-<sha>` into `production` listing the commits, opened with the Actions token so that
   super does not run on it a second time, and merged at once with a merge commit. When a job failed,
   super's report opens "CI: daily super run failed" for the night bot as `priority:high`, which the
   bot takes before everything else it has queued (`GateTitles`, `bot/go/internal/queue`), and
   production waits for the next run on `staging` that passes. The fix lands on `main` and reaches
   `staging` within two hours. A promotion or a deploy that fails opens "CI: production promotion
   failed" for Squishy, also `priority:high`; a staging move that fails opens "CI: staging promotion
   failed" for Squishy.
4. Sooner, by hand: a `/promote` on the open issue labeled `production merge` takes `staging`'s commit
   (or the commit of `staging` named) and opens the same pull request,
   with the bot's token, so the super run runs on it. When every job passed, `promote-production.yml`
   merges it. When one failed, it says so on the issue and the pull request waits, and super's report
   opens "CI: super run failed on a production candidate" for the night bot, as `priority:high` and
   `difficulty:hard`.
5. The merging run's `deploy` job checks out `production`, builds the web with
   `apps/web/.env.production`, refuses the bundle if a public setting is missing
   (`apps/web/scripts/check-production-bundle.mjs`), runs `wrangler deploy`, and then checks the live
   site: sign-in configured, `/decks` a 200 with the CSP, an unknown path a 404, and
   `https://jackioh.com/api/catalog` reporting the commit.

The first request after a deploy finds the old server still running (a container keeps the image it
started with), so the Worker stops it and starts the new one: every live match rebuilds from its
seed and log when its sockets return (SPEC §9.5), as after any restart.

### 2.1 The production issue: the daily promotion, promote, skip-super and cancel

Production promotes itself once a day, from the daily super run on `staging` (the owners' call of
2026-11-03, on `main` until #3275 moved it to `staging`; it was by hand only from 2026-11-02, #2750):
each run on `staging` that passes every job is promoted once, and a red one promotes nothing until
the night bot's fix, once `staging` has it, lets the next one pass. One
issue labeled `production merge` is open at a time. Its title says where production stands
("Production: commits on main are not live yet", "the super run is testing 1a2b3c4 (#2901)", "the
super run failed on 1a2b3c4 (#2901)", "everything on main is live"); the count of what waits is in
the body, since a title that carried it was retitled with every commit of `main`. Once the issue is
a week old (`ROTATE_DAYS`), a fresh one takes over with its state, and the old one is retitled with
the days it covered ("Production, 2026-10-21 to 2026-11-04 (continued in #3101)") and closed with a
link to the new one, so the thread never grows long; the closed ones are the history. A command left
on a closed one is still carried out, and answered on the open one. The first line of its body is
a hidden state line; leave it alone. Under it the issue says which commit of `main` each site serves
(production from its branch, staging from the newest of its GitHub deployments that succeeded) and
what production does next, then lists every pull request on `main` that is not on production yet,
with where it stands on staging (live, deploying, the next move of `staging`, or its checks running
or failed) and on production (being tested in the open promotion, the next promotion once it is on
`staging`, after staging, or its checks running or failed), and ends with how both sites move and the commands. Its state line also keeps
the last super run on `staging` it promoted from (`daily`), so a `/cancel` of a daily promotion stays
cancelled until the next day's run.

Each catalog version is also a GitHub release (`.github/workflows/releases.yml`): a pre-release, "on
staging", while only `main` holds it, and a release once a merge brings it to `production`, the newest
one Latest.

Anyone with write access comments on the issue:

| Comment | Effect |
| --- | --- |
| `/promote` (or `/fast-forward`) | Opens the promotion pull request of `staging`'s commit into `production`. The super run tests it; it merges and deploys once every job passes. |
| `/promote <sha>` | The same, for that commit of `staging`. A newer commit of `main` must reach `staging` first (Actions -> promote staging -> Run workflow). |
| `/skip-super <reason>` | The emergency way: merges the open pull request, or `staging`'s commit, now, without the super run. The reason is required; the issue and the pull request record who, why and which commit. |
| `/cancel` | Closes the open pull request. Nothing merges. |

Each command is acknowledged with a 👍 (😕 when refused) and a reply saying what came of it. A newer
`/promote`, or the daily promotion of a newer commit, closes the open pull request as superseded; the
daily promotion of a commit an open `/promote` already carries leaves it to its own super run. After a
red super run the pull request stays open: re-running the run's failed jobs (the run's page, Re-run
failed jobs) merges it once they pass, and the next daily run that passes, or a `/promote` once the
fix is on `staging`, takes the newer commit instead. `/hold`, `/resume` and `/delay`, from the old daily
countdown, are answered that there is nothing to hold: production follows the super run. To stop the
daily promotion for a while, disable `promote-production.yml` (Actions -> promote production -> the
menu -> Disable workflow), which stops `/promote` too.
Actions -> promote production -> Run workflow does the same as `/promote` for its `sha`, or as
`/skip-super` when its `skip_super_reason` is filled; its `redeploy` deploys production as it is.
Staging takes no command: `promote-staging.yml` moves it every two hours, and Actions -> promote
staging -> Run workflow moves it now (to its `sha`, or main's newest green commit).
`scripts/promote-production.sh` holds the rules and the label (`RELEASE_LABEL`), and
`scripts/promote-staging.sh` staging's.

### 2.2 The rules the promotion enforces

- `staging` only moves forward along `main`, to a commit whose push-to-main CI run passed: a
  candidate it already has is a no-op, and a `staging` that holds anything `main` does not fails the
  run. Its ruleset (`infra/github.tf`) refuses deletion and force pushes.
- The production candidate must be on `staging` and have a successful push-to-main CI run, and a
  super run in which every job passed, on `staging` (the daily promotion) or on its promotion pull
  request, unless a person merged it with `/skip-super` and a reason.
- `production` only receives merges of commits of `staging`: if someone committed to `production`,
  the run fails and changes nothing. It never force-pushes.
- Promoting an older commit than `production` already has is a no-op, not a rollback.
- It moves `production`, and `promote-staging.yml` moves `staging`, with the night bot's
  `BOT_GITHUB_TOKEN` (which has the `workflow` scope: GitHub refuses the built-in token a push that
  changes `.github/workflows`, and a push the built-in token makes starts no workflow, so
  `deploy-staging.yml` would never hear of it). The issue and its
  comments stay on `GITHUB_TOKEN`, whose writes start no workflow. Nothing loops: a pull request's
  CI never starts the promotion, and on a push to `production` only `releases.yml` runs, which writes
  nothing but GitHub releases.

## 3. What a person sets outside the repo

| Where | What | Used by |
| --- | --- | --- |
| Cloudflare | the Workers Paid plan (Containers need it) | both Workers |
| Cloudflare | each Worker's secrets: `SUPABASE_SECRET_KEY`, `DATABASE_URL`, `CODE_PEPPER`, and optionally `CREATE_ISSUES_TOKEN`, `DEV_PROFILE_IDS`, `ALERT_RESEND_API_KEY`, `ALERT_EMAILS` (`apps/edge/README.md`, Settings) | the servers |
| Resend | a sending-only API key for jackioh.com, the Worker secret `ALERT_RESEND_API_KEY`, beside the sign-in mail's (section 8) | the server's alert emails (`crates/server/README.md`, Alerts) |
| Cloudflare Zero Trust | the team's organisation, and a service token for staging's deploy check | Access |
| GitHub secret | `CLOUDFLARE_API_TOKEN`: the Edit Cloudflare Workers template, plus Containers if wrangler asks for it | both deploy jobs |
| GitHub secrets | `CF_ACCESS_CLIENT_ID`, `CF_ACCESS_CLIENT_SECRET`: the service token's pair | staging's live check |
| GitHub secret | `CLOUDFLARE_BILLING_TOKEN`: a custom token with the account's Billing: Read alone | `cf-spend.yml` |
| GitHub secret | `BOT_GITHUB_TOKEN` (already set) | moving `production` |
| GitHub secret | `CODE_PEPPER`: both Workers' pepper, the one copy that can be read back (a Worker's secret cannot), kept from Render's on 2026-11-03 | a person setting it on a new Worker, or `mint-code` |
| `infra/terraform.tfvars` (git ignores it) | `staging_team_emails`: who Access lets into staging | `terraform apply` |
| `infra/terraform.tfvars` (git ignores it) | `admin_emails`: who Access lets into the dev dashboard, the dev accounts' addresses (R3074000, `apps/admin/README.md`) | `terraform apply` |

Everything else is in the repo: the Workers, their settings that are not secrets (`vars`), the
container's size and placement in `wrangler.jsonc`; the custom domains, Access and Supabase's
redirect URLs in `infra/` (`infra/README.md`). **Never** put `SUPABASE_SECRET_KEY`, `DATABASE_URL`
or `CODE_PEPPER` in the web build or in `wrangler.jsonc`: anything the web build reads is readable by
every visitor, and the repository is public.

## 4. Moving from Render and Vercel, in order

Production keeps serving from Render until step 6 verifies the new server, so do these in order.

**Actions -> cutover** (`.github/workflows/cutover.yml`) ran steps 3 and 6's secrets, the copy and
the dropping of staging's keep-alive, one `step` a run, so no secret passed through a person: each
value went from Render's `jackioh-server` or a repository secret straight to where it was used. Those
steps read Render, and left the file with it (step 7; they are in its history); what still runs there
is `check-production`, `production-auth`, `email`, `stats-logins` and `dev-stats`. For production it
reads `PRODUCTION_SUPABASE_SECRET_KEY` and `PRODUCTION_DATABASE_URL` (JackiOh-Production's secret key
and session pooler URI). The URI may keep Supabase's `[YOUR-PASSWORD]`, with the password itself in
`PRODUCTION_DB_PASSWORD`, which, when set, replaces any password the URI holds; the workflow
percent-encodes it. Step `check-production` writes nothing and says whether those secrets are a secret
key and an address that reaches JackiOh-Production. The commands below are what it does, for doing it
by hand.

### Step 1: Merge

Production moved only on a `/promote` during the move (section 2.1), so nothing promoted it before
its secrets existed. (Before 2026-11-02 production merged by itself once a day, and this step held it
with `/hold`; since 2026-11-03 the daily promotion moves it again.) Merge the pull request. Its first green CI run on main
starts `deploy-staging.yml`, which creates `jackioh-staging`. That first deploy's server cannot start
until step 3 gives it its secrets, so its live check fails once: expected.

### Step 2: Cloudflare account

1. Workers & Pages -> Plans: the **Workers Paid** plan.
2. The deploy token: My Profile -> API Tokens. The existing `CLOUDFLARE_API_TOKEN` was made from the
   **Edit Cloudflare Workers** template. If a deploy says the token may not manage containers, edit
   the token and add the account's Containers permission (Edit).
3. Workers & Pages -> `jackioh` -> Settings -> Build: no Git repository connected (Cloudflare's own
   builds stay off: GitHub Actions builds and deploys).

### Step 3: The servers' secrets

For each Worker (`--env staging`, then `--env=""` for production), from `apps/edge`:

```
pnpm exec wrangler secret put SUPABASE_SECRET_KEY --config ../../wrangler.jsonc --env staging
pnpm exec wrangler secret put DATABASE_URL --config ../../wrangler.jsonc --env staging
pnpm exec wrangler secret put CODE_PEPPER --config ../../wrangler.jsonc --env staging
```

Staging's values are the ones Render's `jackioh-server` holds now (Environment), since staging keeps
the project production used (JackiOh-Staging, section 8), with one change: `DATABASE_URL` is
Supabase's **session pooler** URI (Supabase -> Connect -> Session pooler), which works over IPv4. The
direct host is IPv6-only, and a container's outbound IPv6 is not something to depend on. Copy
`DEV_PROFILE_IDS` if Render has it. Production's are set in step 6, from its own project. A secret reaches a container when it
starts, so after setting them run **deploy staging** by hand (Actions -> deploy staging -> Run
workflow). Actions -> cutover, step `staging-secrets`, does all of this once deploy staging has
created the Worker.

### Step 4: Staging's address and Access

Done on 2026-11-02 through Cloudflare's API, in this order, so staging was behind Access from its
first request:

1. Zero Trust, team `jackioh` (`jackioh.cloudflareaccess.com`), the Free plan (up to 50 users), with
   One-time PIN as the sign-in.
2. The Access application `JackiOh staging` on `staging.jackioh.com` and its two policies, as
   `infra/cloudflare.tf` declares them, the team's being the two owners' addresses.
3. `staging.jackioh.com` as `jackioh-staging`'s custom domain. `infra/cloudflare.tf`'s `import`
   blocks adopt all four at step 6's apply, whose `infra/terraform.tfvars` must list the same
   addresses (`staging_team_emails`), or the apply changes the policy to the file's.
4. The service token `JackiOh staging deploy check` (a year), whose pair is the repository secrets
   `CF_ACCESS_CLIENT_ID` and `CF_ACCESS_CLIENT_SECRET`, for deploy-staging.yml's live check.

To let someone else into staging, add their address to the policy `JackiOh staging: the team` (Zero
Trust -> Access -> Policies) and to `staging_team_emails`.

Until step 6 production signs in on the project that becomes staging's, and `infra/supabase.tf`
would move that project's Site URL to staging. So add `https://staging.jackioh.com/login` to that
project's Redirect URLs (Actions -> cutover, step `production-auth` does it; or by hand,
Authentication -> URL Configuration), leaving its Site URL and `https://jackioh.com/login` as they
are; step 6's full apply puts it to the file.

### Step 5: Verify staging

Replace `$SITE` with `https://staging.jackioh.com`. A browser asks for an Access sign-in first.

1. Open `$SITE` signed out of Access: Cloudflare's sign-in page, not the game.
2. `/practice` loads, `/decks` loaded directly and reloaded stays on the deck screen, `/loginx` is a
   404, and DevTools -> Console shows no CSP violations.
3. Sign in. DevTools -> Network: the API calls go to `$SITE/api/...` and the socket to
   `wss://staging.jackioh.com/ws/match`.
4. Request a password reset: the email's link opens staging.
5. Save a deck, queue, and play a match in two browsers. Reload one mid-match: it comes back into
   the same position.
6. Calibrate `TRUSTED_PROXY_HOPS` (`docs/architecture.md` §10, step 8): Workers & Pages ->
   `jackioh-staging` -> Logs, find `api.forwarded_for` after a request of your own. If `fewestEntries`
   is 2, set both environments' `TRUSTED_PROXY_HOPS` to `"2"` in `wrangler.jsonc`.
7. Leave it idle for 15 minutes, then load `/decks`: it waits a few seconds while the server starts.
   Note how long; that is the cold start players see.

### Step 6: Production

1. Step 3's secrets for `--env=""`, from JackiOh-Production (Project Settings -> API Keys, and its
   own session pooler URI), with the `CODE_PEPPER` and `DEV_PROFILE_IDS` Render holds: the copy
   keeps every invite code's hash and every account's id, so the same pepper and ids still fit.
   Actions -> cutover, step `production-secrets`.
2. Copy the data into JackiOh-Production (section 8, The copy), straight before the next item: Render
   writes to the old project until the redeploy, so anything a player saves between the two stays
   on staging. Actions -> cutover, step `drop-ping` first, then `copy` with `dry_run` and without.
3. Actions -> promote production -> Run workflow, with **redeploy** ticked. This deploys `production`
   as it is to the `jackioh` Worker and its container, and checks the live site.
4. Verify as in step 5 against `https://jackioh.com` (no Access). The site and the server now share
   an origin; nothing calls `jackioh-server.onrender.com` any more. Sign in with an account made
   before the copy: its password works, its collection and decks are there.
5. `terraform -chdir=infra plan`, then `apply`, untargeted: the first full one (`infra/README.md`, The
   first import). Only now may staging's project lose `https://jackioh.com` as its Site URL.
6. From here on production moves as section 2.1 says: `/promote` on the production issue.

### Step 7: Turn Render and Vercel off

Done on 2026-11-03, the owners' call (CLAUDE.md rule 13), straight after step 6 rather than a day
later, so Render would stop billing:

1. Actions -> cutover, step `after-cutover`, once `https://jackioh.com/api/catalog` answered with an
   `x-deployed-commit`: suspended Render's `jackioh-server` and gave staging's project its own Site URL
   and Redirect URLs. Step `delete-render` then kept Render's `CODE_PEPPER`, sealed to a key a person
   held and opened straight into the repository secret `CODE_PEPPER` (section 3), and deleted the
   Blueprint and the service. The owners deleted the Render workspace's card and the Vercel project.
2. The repository secrets `RENDER_DEPLOY_HOOK_URL`, `RENDER_API_KEY` and `VERCEL_DEPLOY_HOOK_URL` are
   deleted, and cutover's steps that read Render (`staging-secrets`, `drop-ping`,
   `production-secrets`, `copy`, `after-cutover`, `delete-render`) left `cutover.yml`;
   `check-production`, `production-auth`, `email`, `stats-logins` and `dev-stats` still run.

## 5. Day-to-day operation

- **Normal day:** merge to main as usual. Staging takes it within two hours of its CI passing;
  production takes the commit of `staging` the daily super run passed, the same day (section 2.1). A red daily
  run goes to the night bot as `priority:high`, and production waits for the next one that passes; a
  promotion or deploy that fails goes to Squishy.
- **Ship to production sooner:** move `staging` (Actions -> promote staging -> Run workflow) if the
  change is not on it yet, then `/promote` on the production issue. The super run tests the pull
  request (an hour or so) and it merges and deploys once every job passed. A red job goes to the night
  bot as `priority:high`; re-run the failed jobs once it is fixed, or `/promote` again for the newer
  commit.
- **Ship now (hotfix):** merge the fix, wait for its CI on main, run promote staging, then `/promote`. If the super run is
  red for a reason the fix does not touch and it cannot wait, `/skip-super <why>` merges it at once,
  and the issue records who and why.
- **Catalog bump:** nothing special. Client and server ship together, on each site's own schedule.
- **Roll back:** revert the change on main, wait for its CI, and promote it as a hotfix (above):
  the deploy rebuilds the web and the server from that commit, and the first request restarts the
  server on it. Workers & Pages -> the Worker -> Deployments -> Rollback is quicker for the Worker
  and its static files, but check in the deployment's details that it took the container's image
  back too before relying on it. Either way, nobody `/promote`s until the fix has merged, and the
  next daily promotion deploys main as it stands: if the revert cannot be on main before the next
  daily super run passes, disable `promote-production.yml` until it is (section 2.1).
- **Change a secret:** `wrangler secret put` (section 4, step 3), then redeploy that site (Run
  workflow on deploy staging, or promote production with **redeploy**).
- **Move staging by hand:** Actions -> promote staging -> Run workflow. With no `sha` it moves
  `staging` to main's newest green commit now, whatever the two-hour pace; a `sha` given must be on
  main and have a green push-to-main CI run. Its push deploys staging.
- **Deploy staging by hand:** Actions -> deploy staging -> Run workflow. With no `sha` it deploys
  `staging`'s commit again (after a secret changed, say); a `sha` given must be on `staging`.
- **Staging stopped moving:** the issue "CI: staging promotion failed" names the run. "is not an
  ancestor of" means someone pushed to `staging` by hand: an admin resets it to a green commit of
  main with `git push --force origin <sha>:staging` (temporarily allowing force pushes in the
  `staging` ruleset; `terraform -chdir=infra apply` puts the rule back).
- **A dev account (SPEC §14.10):** the `DEV_PROFILE_IDS` secret of that site's Worker, each
  developer's profile id comma-separated (from `/account`: "Manage sign-in & account", then "Show
  account secret ID"); then redeploy. A malformed id refuses the boot, and the Worker's logs name it.
- **A deploy job fails:** the run's log names the step: the build, the bundle check (a missing
  `VITE_` setting), the deploy (the token, or Docker building the image), or the live check (the
  server did not report the commit in time: Workers & Pages -> the Worker -> Logs shows the server's
  boot, and a failed migration there). Fix it on `main`; for production, then run promote
  production with **redeploy**.
- **An alert:** the server opens `Patch: alert <event> on <site>` (labelled `human`) and emails
  `ALERT_EMAILS` the details (`crates/server/README.md`, Alerts). Read the event in the Worker's logs
  (Workers & Pages -> the Worker -> Logs), and close the issue once it is handled, so the next one emails
  again. To turn the emails on or change who gets them, `wrangler secret put ALERT_RESEND_API_KEY` and
  `ALERT_EMAILS` on that Worker, then redeploy it.
- **The server will not start:** its boot runs `release` (migrate, seed the catalog) and exits on
  any failure; with one container there is no older instance to keep serving. The Worker's logs
  show the error. CI's `db` job boots the image against Postgres before any merge that changes the
  migrations or the image, so this should be rare. Roll back (above) while fixing it.
- **A promote run fails with "refusing to allow a GitHub App to create or update workflow":** the
  run had no `BOT_GITHUB_TOKEN`, or the token lost its `workflow` scope or expired. Renew it
  (`bot/README.md`, "Setting it up"), then `/promote` again.
- **A promote run fails with "production has diverged":** someone committed to `production` by
  hand. Compare with `git log --oneline --graph origin/main origin/production`. An admin can reset it
  with `git push --force origin <green-main-sha>:production` (temporarily allowing force pushes in
  the `production` ruleset; `terraform -chdir=infra apply` puts the rule back, `infra/README.md`).
- **The production issue is missing or doubled:** the next run of promote production (any green
  push to `main` starts one) opens one when none is open and closes all but the oldest.
- **A merged super run did not merge its pull request:** the run that merges is promote
  production's, woken by super's completion; GitHub keeps only one waiting run in its concurrency
  group, so a wake-up can be dropped. Any later run (a comment, a green push to `main`, or Run
  workflow) checks the open pull request's super run and merges it.
- **Scheduled runs stop:** GitHub disables scheduled workflows in a repository with no activity for
  60 days. The daily super run is one: re-enable it in the Actions tab. A promotion's super run is
  not scheduled, so it runs regardless.

## 6. Cost

The Workers Paid plan is $5 a month. Static files cost nothing; the engine's module is one Worker
request per download of it, which a browser then keeps a year. A container is billed for memory
and disk while it is awake, and for CPU while it works: about $7 a month on top of the plan for one
`basic` container awake all month, far less when it sleeps, and the plan's included usage covers
the first hours. Staging's container sleeps whenever the team is away. Supabase is the Pro plan, $25
a month for the organization (jlore-studios), whose compute credit covers one project's smallest
instance; the second project's instance is billed on top, and each project's disk past 8 GB
(`db-size.yml` warns before then). GitHub Actions (a public repository) costs nothing. The levers are `SLEEP_AFTER`
(`apps/edge/src/index.ts`, five minutes) and `instance_type` (`wrangler.jsonc`); a signed-in tab keeps its
site's server awake through its presence beat and its social socket until it has been hidden for
`HIDDEN_QUIET_MS` (two minutes, `apps/web/src/together/config.ts`, R3576000), so a tab left in the
background lets the server sleep about seven minutes later.

Cloudflare has no spend cap, so two things watch the usage bill (the plan's $5 aside). The account's
budget alerts (Notifications -> Budget Alert: one at $1 and the auto-created one at $10) email the
owner when the period's projected usage spend crosses them. `cf-spend.yml` reads the same figures
once a day and opens a `human` issue once the period's spend, or its projection, reaches its
`ALERT_DOLLARS` ($15, about both containers awake all month); it needs `CLOUDFLARE_BILLING_TOKEN`
(section 3) and says so, green, until it is set.

## 7. Known gaps

- **An open tab can run an older client.** A deploy ships both halves together, but a browser that
  loaded the page before it keeps that client until it reloads, so a change to the API or the match
  protocol must keep accepting the previous client for a while (as `ws_server.rs` keeps the
  previous client's `?token=` upgrade, R1379000).
- **One container is one point of failure,** as Render's one instance was (`docs/architecture.md`
  §4.1), and now without Render's "keep the old instance until the new one is healthy".
- **Cold starts.** The first request after ten idle minutes waits for the container and the
  server's boot, up to `START_WAIT_MS` (`apps/edge/src/index.ts`). A wake with nothing new reads the
  migration ledger once and writes the catalog in one statement that rewrites no unchanged row
  (#2926), a handful of round trips where it was about 1,100. Timed from the logs on 2026-11-04: the
  catalog step took 0.5 s on staging with that change and 24.5 s on production without it, which
  outran the library's 20-second wait and answered the first visitor with a 500.
- **A deploy whose image is new still waits for its rollout.** Since 2026-11-03 neither deploy
  compiles the server when the registry or CI has its image (`scripts/server-image.sh`), and a
  promotion of a commit staging already ran pushes nothing. The registry keeps the newest ten tags
  (and whatever a site runs), so a Worker rolled back further than that finds no image: promote a
  revert instead.
- **No built file may reach Cloudflare Workers' 25 MiB cap on one asset.** `wrangler deploy` stops
  at the first one ("Asset too large"), as staging's deploys did on 2026-11-05, when the engine's
  module reached 25.5 MiB. The module now ships brotli-compressed (about 2.5 MiB, #3183), and
  `apps/web/scripts/check-assets.mjs` refuses any file at or over `ASSET_REFUSED_BYTES` (a MiB under
  the cap) and any uncompressed `.wasm`: before both deploys (`check-production-bundle.mjs`) and on
  every pull request the web's jobs run on (`src/net/check-assets.test.ts`, over CI's built client).
- **Containers are in beta** on Cloudflare: limits and behaviour can change; read their changelog
  when something surprises you.

## 8. Production's own database

Each site has a Supabase project of its own, both in the jlore-studios organization, in `us-east-2`
(Ohio) beside the containers:

| | Staging | Production |
| --- | --- | --- |
| Project | JackiOh-Staging, `exmjdaswedxhnzmpzqrq`: the original project, which production used until the move | JackiOh-Production, `fuzpvmjdejvbjcarypyi`, made on 2026-11-01 |
| Named in | `apps/web/.env.staging`, `wrangler.jsonc` `env.staging`, `infra/supabase.tf` | `apps/web/.env.production`, `wrangler.jsonc`'s top level, `infra/supabase.tf` |
| Card statistics | the development lanes' (`gather.yml`, `learn.yml`'s import, through `STATS_IMPORT_DATABASE_URL`) and staging's own games | the same development lanes' (through `PRODUCTION_STATS_IMPORT_DATABASE_URL`) and players' games (R1933001) |
| Read by | `db-size.yml` through `STATS_IMPORT_DATABASE_URL` | `winrates-refresh.yml` and `db-size.yml` through `STATS_DATABASE_URL`, the read-only login `stats_reader` |

`apps/web/src/net/env-production.test.ts` fails when the two builds name one project, or when a
build's project is not its own server's `SUPABASE_URL`.

**Setting up JackiOh-Production,** once, before section 4's step 6: Authentication as
`docs/architecture.md` §10 steps 1 and 2 say (email confirmations, SMTP, the email templates, MFA,
and each OAuth provider wanted, whose callback is now this project's); the Site URL and Redirect URLs
are `infra/supabase.tf`'s. Settings live per project, so none comes over with the data. Actions ->
cutover, step `production-auth` (the repository secret `SUPABASE_ACCESS_TOKEN`, a personal access
token), copies staging's to production, first with `dry_run` to list the changes. What the API does
not hand back, SMTP's password and each provider's client secret among them, it leaves out whole and
names in the run's warnings: those are set by hand. The new
project has its own signing keys, so every player signs in once more after the switch, with the same
password. The project's two statistics logins, `stats_importer` and `stats_reader` (read-only), came
with the copy; Actions -> cutover, step `stats-logins`, gives each a password of its own and prints
their addresses sealed to a key a person holds, who opens them into `PRODUCTION_STATS_IMPORT_DATABASE_URL`
and `STATS_DATABASE_URL` (done on 2026-11-03).

**Email.** Both projects send their sign-in mail (confirmations, password resets, invites) through
Resend, from `JackiOh <no-reply@jackioh.com>`, at most 30 an hour each: Supabase's own mailer reaches
only the project's team, two an hour. Resend's records in the jackioh.com zone, and a DMARC policy
that only reports, are `infra/cloudflare.tf`'s. Actions -> cutover, step `email` (the repository
secrets `RESEND_API_KEY`, a Full access key, and `SUPABASE_ACCESS_TOKEN`) sets both projects' SMTP to
a sending-only key it makes for the domain, and deletes the one before: run it again to rotate the
key. Resend's free plan sends 100 a day; its dashboard (Emails) shows each one sent and bounced.

**The copy,** at section 4's step 6, with nobody promoting production meanwhile: Supabase's own way
between projects, the CLI's `db dump` (it runs `pg_dump` in Docker) from staging and `psql` into the
empty production project, both through their session pooler URIs as `postgres`, held in a file outside
the repository (`chmod 600`) and never pasted anywhere. Actions -> cutover, step `copy`, did it all,
refusing a production project that was not empty and rolling the whole copy back when the counts
differed (it read staging's address from Render, so it left the file with Render; it is in the file's
history):

```
supabase db dump --db-url "$STAGING_DB_URL" -f roles.sql --role-only
supabase db dump --db-url "$STAGING_DB_URL" -f schema.sql
supabase db dump --db-url "$STAGING_DB_URL" -f data.sql --use-copy --data-only \
  -x public.game_records -x public.card_totals -x public.game_totals -x public.record_totals \
  -x public.game_record_batches
psql "$PRODUCTION_DB_URL" --single-transaction -v ON_ERROR_STOP=1 -f roles.sql -f schema.sql \
  -c 'set session_replication_role = replica' -f data.sql
```

The five tables left out hold the card statistics; only their live rows come over, `\copy (select *
from public.<table> where source = 'live')` out of staging and `\copy public.<table> from` into
production for `game_records`, `card_totals`, `game_totals` and `record_totals`. `game_record_batches`
holds only the development imports' batches, so none of it comes. Then Advisors -> Security on the new
project (row-level security is on for every `public` table, as on staging), and the counts of
`profiles`, `matches` and `auth.users` on both sides, which must agree.

**Production's figures count the development games too** (R1933001, the owner's decision of
2026-11-03, which replaced R1933000's live games alone). §9.11's publication gate pads a patch's
public figures with development games until it has `PUBLIC_STATS_MIN_LIVE_GAMES` live games, and with
fewer than 100 players online at once a patch seldom gets there before the next ships. The copy left
the development rows behind (R1933000 was in force); Actions -> cutover, step `dev-stats`, then
replaced production's development rows with staging's, and the development lanes have filed into
both since. Run `dev-stats` again whenever the two may have drifted, such as after an import that
reached staging's and failed on production's and was not rerun.
