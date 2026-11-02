# Hosting: everything on Cloudflare, staging and production

This is the plan and the runbook for running JackiOh on Cloudflare: what runs where, how a commit
reaches each site, what the repo enforces, what a person sets outside the repo, and what to do when
something goes wrong. Render and Vercel are gone. The database and sign-in stay on Supabase.

## 1. Target state

| | Staging | Production |
| --- | --- | --- |
| Purpose | The team sees main as it is now | What players use |
| Address | https://staging.jackioh.com, behind Cloudflare Access: only the team signs in | https://jackioh.com |
| Worker | `jackioh-staging` (`wrangler.jsonc` `env.staging`) | `jackioh` (`wrangler.jsonc`'s top level) |
| Builds from | `main`, after main's push-to-main CI run passes (`deploy-staging.yml`) | `production`, once a day by the countdown issue (`promote-production.yml`) |
| Game server | one `jackioh-staging-server` container | one `jackioh-server` container |
| Database and sign-in | Supabase JackiOh-Staging, `exmjdaswedxhnzmpzqrq` (the original project), where the development lanes file too | Supabase JackiOh-Production, `fuzpvmjdejvbjcarypyi`: live games only (section 8) |
| Routing and headers | `apps/web/public/_redirects` and `_headers`, `wrangler.jsonc` | the same |

Each site is one Worker at one origin (`apps/edge/README.md`). Every path but `/api/*` and `/ws/*` is
a static file from the web build, served without running any Worker code. Those two prefixes go to
the Worker, which hands them to one container running `crates/server/Dockerfile`'s image. The
container sleeps after ten minutes with no request and no open socket, and the next request starts
it again. A daily cron wakes it.

Each deploy ships the web build and the server together, so a site's client and server are never
from different commits, and production's server no longer runs a day ahead of its client.

`main` stays the only branch anyone merges into. `production` is a pointer to "the commit of main
that is live on jackioh.com". Nothing but `.github/workflows/promote-production.yml` moves it, by
merging a pull request whose head is a green commit of main, so its files always equal that commit's.

## 2. How a commit reaches each site

1. A pull request merges into `main` through the merge queue (branch protection requires the five
   CI checks).
2. CI runs again on the push to main. When it passes, `deploy-staging.yml` checks out that commit
   (or main's newest green commit, if a newer one already passed, so staging never goes backwards),
   builds the web with `apps/web/.env.staging`, checks the bundle, and runs `wrangler deploy --env
   staging`, which builds the server's image on the runner and deploys the Worker, the static files
   and the container together. It then polls `https://staging.jackioh.com/api/catalog` (through
   Access, with a service token) until the server reports the commit in `x-deployed-commit`.
3. The open issue labeled `production merge` counts down to 10:00 AM Central (section 2.1). When its
   time comes, `promote-production.yml` looks up the newest commit of `main` whose push-to-main CI run
   succeeded, opens a pull request from `promote/<date>-<sha>` into `production` listing the
   commits, and merges it with a merge commit. GitHub starts this repository's schedules late, if at
   all, so the same check also runs on every green CI run on `main`: the first one after the time
   merges, and the cron is only a second chance.
4. The same run's `deploy` job checks out `production`, builds the web with
   `apps/web/.env.production`, refuses the bundle if a public setting is missing
   (`apps/web/scripts/check-production-bundle.mjs`), runs `wrangler deploy`, and then checks the live
   site: sign-in configured, `/decks` a 200 with the CSP, an unknown path a 404, and
   `https://jackioh.com/api/catalog` reporting the commit.

The first request after a deploy finds the old server still running (a container keeps the image it
started with), so the Worker stops it and starts the new one: every live match rebuilds from its
seed and log when its sockets return (SPEC §9.5), as after any restart.

### 2.1 The countdown issue: hold, delay and fast-forward

One issue labeled `production merge` stays open for good. Its title reads "Merging to production in
23 hours (2026-10-05 10:00 AM CT)", and every check brings the hours up to date. The first line of
its body is a hidden state line; leave it alone. Under the countdown it lists every pull request on
`main` that is not on production yet, with its push-to-main CI (passed, failed, running) and where it
stands for production (live, the next merge, waits for CI, held).

Each catalog version is also a GitHub release (`.github/workflows/releases.yml`): a pre-release, "on
staging", while only `main` holds it, and a release once a merge brings it to `production`, the newest
one Latest.

Anyone with write access comments on the issue:

| Comment | Effect |
| --- | --- |
| `/hold` (a reason after it, if you like) | Nothing merges until `/resume`. The title says "on hold". |
| `/resume` (or `/release`, `/unhold`) | Lifts the hold. If the time has passed, it merges at once. |
| `/delay 3h`, `/delay 2d` | Moves the merge later by that long, at most 168 hours at a time. |
| `/fast-forward` | Merges now, past a hold. |

Each command is acknowledged with a 👍 (😕 when refused or not understood). A command may name
its target first (`/hold production`; `prod` and `cloudflare` mean the same); one that names
`staging` or `vercel` is refused with a 😕 and changes nothing. Staging takes no command: it deploys
every green commit of main. `scripts/promote-production.sh` holds the rules, the
hour (`RELEASE_HOUR`, `RELEASE_TZ`) and the label (`RELEASE_LABEL`).

### 2.2 The rules the promotion enforces

- The candidate must be on `main` and have a successful push-to-main CI run.
- `production` only receives merges of green main commits: if someone committed to `production`,
  the run fails and changes nothing. It never force-pushes.
- Promoting an older commit than `production` already has is a no-op, not a rollback.
- It moves `production` with the night bot's `BOT_GITHUB_TOKEN` (which has the `workflow` scope:
  GitHub refuses the built-in token a push that changes `.github/workflows`). The issue and its
  comments stay on `GITHUB_TOKEN`, whose writes start no workflow. Nothing loops: a pull request's
  CI never starts the promotion, and on a push to `production` only `releases.yml` runs, which writes
  nothing but GitHub releases.

## 3. What a person sets outside the repo

| Where | What | Used by |
| --- | --- | --- |
| Cloudflare | the Workers Paid plan (Containers need it) | both Workers |
| Cloudflare | each Worker's secrets: `SUPABASE_SECRET_KEY`, `DATABASE_URL`, `CODE_PEPPER`, and optionally `CREATE_ISSUES_TOKEN`, `DEV_PROFILE_IDS` (`apps/edge/README.md`, Settings) | the servers |
| Cloudflare Zero Trust | the team's organisation, and a service token for staging's deploy check | Access |
| GitHub secret | `CLOUDFLARE_API_TOKEN`: the Edit Cloudflare Workers template, plus Containers if wrangler asks for it | both deploy jobs |
| GitHub secrets | `CF_ACCESS_CLIENT_ID`, `CF_ACCESS_CLIENT_SECRET`: the service token's pair | staging's live check |
| GitHub secret | `BOT_GITHUB_TOKEN` (already set) | moving `production` |
| `infra/terraform.tfvars` (git ignores it) | `staging_team_emails`: who Access lets into staging | `terraform apply` |

Everything else is in the repo: the Workers, their settings that are not secrets (`vars`), the
container's size and placement in `wrangler.jsonc`; the custom domains, Access and Supabase's
redirect URLs in `infra/` (`infra/README.md`). **Never** put `SUPABASE_SECRET_KEY`, `DATABASE_URL`
or `CODE_PEPPER` in the web build or in `wrangler.jsonc`: anything the web build reads is readable by
every visitor, and the repository is public.

## 4. Moving from Render and Vercel, in order

Production keeps serving from Render until step 6 verifies the new server, so do these in order.

### Step 1: Hold production, then merge

Comment `/hold moving to Cloudflare` on the countdown issue, so no promotion runs before
production's secrets exist. Then merge the pull request. Its first green CI run on main starts
`deploy-staging.yml`, which creates `jackioh-staging`. That first deploy's server cannot start until
step 3 gives it its secrets, so its live check fails once: expected.

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
workflow).

### Step 4: Staging's address and Access

1. Zero Trust (the dashboard's Zero Trust section): pick a team name and the Free plan (up to 50
   users). Settings -> Authentication: One-time PIN is on by default; that is the sign-in.
2. Access -> Service Auth -> Create Service Token, named `deploy-staging`. Save its Client ID and
   Client Secret as the GitHub repository secrets `CF_ACCESS_CLIENT_ID` and `CF_ACCESS_CLIENT_SECRET`
   (the secret is shown once).
3. Write `infra/terraform.tfvars` with the team's addresses, `staging_team_emails = ["...", "..."]`,
   then `terraform -chdir=infra plan` and `apply`, both with
   `-target=cloudflare_workers_custom_domain.staging_jackioh_com
   -target=cloudflare_zero_trust_access_application.staging` (`infra/README.md`; the Cloudflare token
   there needs Access: Apps and Policies, Edit). It adds `staging.jackioh.com` to `jackioh-staging`,
   and the Access application and its two policies. The rest waits for step 6: until then production
   signs in on the project that becomes staging's, and `infra/supabase.tf` would move that project's
   Site URL to staging. So add `https://staging.jackioh.com/login` to that project's Redirect URLs by
   hand (Authentication -> URL Configuration), leaving its Site URL and `https://jackioh.com/login`
   as they are; step 6's full apply puts it to the file.

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
2. Copy the data into JackiOh-Production (section 8, The copy), straight before the next item: Render
   writes to the old project until the redeploy, so anything a player saves between the two stays
   on staging.
3. Actions -> promote production -> Run workflow, with **redeploy** ticked. This deploys `production`
   as it is to the `jackioh` Worker and its container, and checks the live site.
4. Verify as in step 5 against `https://jackioh.com` (no Access). The site and the server now share
   an origin; nothing calls `jackioh-server.onrender.com` any more. Sign in with an account made
   before the copy: its password works, its collection and decks are there.
5. `terraform -chdir=infra plan`, then `apply`, untargeted: the first full one (`infra/README.md`, The
   first import). Only now may staging's project lose `https://jackioh.com` as its Site URL.
6. Comment `/resume` on the countdown issue.

### Step 7: Turn Render and Vercel off

Only once step 6 has run cleanly for a day or so, and with the owners' say-so (CLAUDE.md rule 13):

1. Render -> `jackioh-server` -> Settings -> Suspend Web Service; after a week with nothing missing
   it, Delete Web Service, and delete the Blueprint.
2. Vercel -> the `jackioh` project -> Settings -> Git -> Disconnect, then Settings -> Delete Project.
3. GitHub -> Settings -> Secrets and variables -> Actions: delete `RENDER_DEPLOY_HOOK_URL`,
   `RENDER_API_KEY` and `VERCEL_DEPLOY_HOOK_URL`.

## 5. Day-to-day operation

- **Normal day:** merge to main as usual. Staging updates within minutes of CI passing; production
  catches up when the countdown issue's time comes.
- **Now, please:** `/fast-forward` on the countdown issue.
- **Not yet, I'm mid-change:** `/hold`, and `/resume` when done; or `/delay 6h`.
- **Ship now (hotfix):** merge the fix, wait for its CI on main, then Actions -> promote production ->
  Run workflow. It promotes the newest green commit, or the `sha` you give, whatever the hold.
- **Catalog bump:** nothing special. Client and server ship together, on each site's own schedule.
- **Roll back:** revert the change on main, wait for its CI, and promote it as a hotfix (above):
  the deploy rebuilds the web and the server from that commit, and the first request restarts the
  server on it. Workers & Pages -> the Worker -> Deployments -> Rollback is quicker for the Worker
  and its static files, but check in the deployment's details that it took the container's image
  back too before relying on it. Either way, `/hold` the countdown until the fix has merged.
- **Change a secret:** `wrangler secret put` (section 4, step 3), then redeploy that site (Run
  workflow on deploy staging, or promote production with **redeploy**).
- **Deploy staging by hand:** Actions -> deploy staging -> Run workflow. With no `sha` it deploys
  main's newest green commit; a `sha` given must be on main and have a green push-to-main CI run.
- **A dev account (SPEC §14.10):** the `DEV_PROFILE_IDS` secret of that site's Worker, each
  developer's profile id comma-separated (from `/account`: "Manage sign-in & account", then "Show
  account secret ID"); then redeploy. A malformed id refuses the boot, and the Worker's logs name it.
- **A deploy job fails:** the run's log names the step: the build, the bundle check (a missing
  `VITE_` setting), the deploy (the token, or Docker building the image), or the live check (the
  server did not report the commit in time: Workers & Pages -> the Worker -> Logs shows the server's
  boot, and a failed migration there). Fix it on `main`; for production, then run promote
  production with **redeploy**.
- **The server will not start:** its boot runs `release` (migrate, seed the catalog) and exits on
  any failure; with one container there is no older instance to keep serving. The Worker's logs
  show the error. CI's `db` job boots the image against Postgres before any merge that changes the
  migrations or the image, so this should be rare. Roll back (above) while fixing it.
- **A promote run fails with "refusing to allow a GitHub App to create or update workflow":** the
  run had no `BOT_GITHUB_TOKEN`, or the token lost its `workflow` scope or expired. Renew it
  (`bot/README.md`, "Setting it up"), then run promote production by hand.
- **A promote run fails with "production has diverged":** someone committed to `production` by
  hand. Compare with `git log --oneline --graph origin/main origin/production`. An admin can reset it
  with `git push --force origin <green-main-sha>:production` (temporarily allowing force pushes in
  the `production` ruleset; `terraform -chdir=infra apply` puts the rule back, `infra/README.md`).
- **The countdown issue is missing or doubled:** the hourly check opens one when none is open and
  closes all but the oldest.
- **Scheduled runs stop:** GitHub disables scheduled workflows in a repository with no activity for
  60 days. Re-enable them in the Actions tab; every green push to `main` runs the same check anyway.

## 6. Cost

The Workers Paid plan is $5 a month. Static files cost nothing. A container is billed for memory
and disk while it is awake, and for CPU while it works: about $7 a month on top of the plan for one
`basic` container awake all month, far less when it sleeps, and the plan's included usage covers
the first hours. Staging's container sleeps whenever the team is away. Supabase is the Pro plan, $25
a month for the organization (jlore-studios), whose compute credit covers one project's smallest
instance; the second project's instance is billed on top, and each project's disk past 8 GB
(`db-size.yml` warns before then). GitHub Actions (a public repository) costs nothing. The levers are `SLEEP_AFTER`
(`apps/edge/src/index.ts`) and `instance_type` (`wrangler.jsonc`); a signed-in tab keeps its site's
server awake through its presence beat.

## 7. Known gaps

- **An open tab can run an older client.** A deploy ships both halves together, but a browser that
  loaded the page before it keeps that client until it reloads, so a change to the API or the match
  protocol must keep accepting the previous client for a while (as `ws_server.rs` keeps the
  previous client's `?token=` upgrade, R1379000).
- **One container is one point of failure,** as Render's one instance was (`docs/architecture.md`
  §4.1), and now without Render's "keep the old instance until the new one is healthy".
- **Cold starts.** The first request after ten idle minutes waits for the container and the
  server's boot (step 5, item 7 measures it).
- **Every deploy builds the image.** `wrangler deploy` runs `docker build` on a fresh runner, so a
  deploy takes as long as a cold release build of the server. Caching the build's layers between
  runs is the next improvement.
- **Containers are in beta** on Cloudflare: limits and behaviour can change; read their changelog
  when something surprises you.

## 8. Production's own database

Each site has a Supabase project of its own, both in the jlore-studios organization, in `us-east-2`
(Ohio) beside the containers:

| | Staging | Production |
| --- | --- | --- |
| Project | JackiOh-Staging, `exmjdaswedxhnzmpzqrq`: the original project, which production used until the move | JackiOh-Production, `fuzpvmjdejvbjcarypyi`, made on 2026-11-01 |
| Named in | `apps/web/.env.staging`, `wrangler.jsonc` `env.staging`, `infra/supabase.tf` | `apps/web/.env.production`, `wrangler.jsonc`'s top level, `infra/supabase.tf` |
| Card statistics | the development lanes' (`gather.yml`, `learn.yml`'s import, through `STATS_IMPORT_DATABASE_URL`) and staging's own games | live games only |
| Read by | `db-size.yml` through `STATS_IMPORT_DATABASE_URL` | `winrates-refresh.yml` and `db-size.yml` through `STATS_DATABASE_URL`, a read-only login |

`apps/web/src/net/env-production.test.ts` fails when the two builds name one project, or when a
build's project is not its own server's `SUPABASE_URL`.

**Setting up JackiOh-Production,** once, before section 4's step 6: Authentication as
`docs/architecture.md` §10 steps 1 and 2 say (email confirmations, SMTP, the email templates, MFA,
and each OAuth provider wanted, whose callback is now this project's); the Site URL and Redirect URLs
are `infra/supabase.tf`'s. Settings live per project, so none comes over with the data. The new
project has its own signing keys, so every player signs in once more after the switch, with the same
password. `STATS_DATABASE_URL` moves to a read-only login on this project, made as `training/learn.md`
makes `stats_importer` but granted only `select`.

**The copy,** at section 4's step 6 with production on `/hold`: Supabase's own way between projects,
the CLI's `db dump` (it runs `pg_dump` in Docker) from staging and `psql` into the empty production
project, both through their session pooler URIs as `postgres`, held in a file outside the repository
(`chmod 600`) and never pasted anywhere:

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

**What production's figures lose.** §9.11's publication gate pads a patch's public figures with
development games until it has `PUBLIC_STATS_MIN_LIVE_GAMES` live games. With the development lanes
filing into staging, production holds none, so its `/stats` shows live games alone from the first
game (`not enough games` until a card has `CARD_STATS_MIN_SAMPLE`), and `winrates-refresh.yml`'s
provisional refresh (R2386000) has nothing to pad with. Copying the development rows of the three
totals tables as well (leave their `-x` off) keeps the padding for the patches already gathered, and
not for later ones. Whether production should read development figures at all is the owner's call;
the gate itself is unchanged either way, and only finds no development games to pad with.
