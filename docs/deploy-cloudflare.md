# Hosting plan: Vercel is staging, Cloudflare is production

This is the plan and the runbook for serving `apps/web` from two hosts. It says what each host is
for, how a commit reaches each one, what the repo enforces, what has to be set by hand outside the
repo, and what to do when something goes wrong. Nothing here changes `apps/server` (Render) or the
database and auth (Supabase).

## 1. Target state

| | Vercel: staging | Cloudflare: production |
| --- | --- | --- |
| Purpose | See main as it is now; preview a branch on request | What players use |
| Builds from | `main`, plus any branch whose commit message says `[vercel]` | `production` only |
| How often | Every push to main that changes the bundle | Once a day (an open issue counts down, and anyone can hold or delay it), plus catalog bumps as soon as CI passes |
| Gate | Vercel's own build | `main`'s push-to-main CI run must have passed |
| Routing and headers | `vercel.json` (source of truth) | `apps/web/public/_redirects`, `_headers`, `wrangler.jsonc` |
| Domain | `jackioh.vercel.app` | your Cloudflare domain (to be chosen) |
| Backend | Render + Supabase (shared) | Render + Supabase (shared) |

`main` stays the only branch anyone merges into. `production` is a pointer to "the commit of main
that is live on Cloudflare". Nothing but `.github/workflows/promote-production.yml` moves it, by
merging a pull request whose head is a green commit of main, so its files always equal that commit's.

## 2. How a commit reaches production

1. A pull request merges into `main` (branch protection already requires the six CI checks).
2. Pushes to `main` trigger three things at once: Render deploys `apps/server`; Vercel deploys
   staging (unless `scripts/vercel-ignore.sh` skips it); and CI runs again on main.
3. An open issue labelled `production merge` says "Merging to production in N hours" (section 2.3).
   When its time comes, 15:00 UTC each day unless someone held or delayed it,
   `promote-production.yml` looks up the newest commit of `main` whose push-to-main CI run succeeded,
   opens a pull request from `promote/<date>-<sha>` into `production` listing the commits, and
   merges it with a merge commit. It then closes the issue as completed, with the pull request, and
   opens the next one. If `production` already has that commit, no pull request is opened and
   nothing deploys. GitHub did not start the first scheduled run (it starts this repository's
   schedules late, if at all), so the same check also runs on every green CI run on `main`: the
   first one after the issue's time merges, and the cron is only a second chance.
4. The same run's `deploy` job checks out `production`, installs the way CI does, builds
   `apps/web` with the public settings in `apps/web/.env.production`, refuses the bundle if any of
   them is missing (`apps/web/scripts/check-production-bundle.mjs`), deploys it to the `jackioh`
   Worker with `wrangler deploy`, and then fetches the live site: sign-in configured, `/decks` a 200
   with the CSP, an unknown path a 404. Cloudflare only serves the files; it builds nothing.

### 2.1 The catalog fast path, and why it exists

The web bundle compiles the catalog version in (`packages/cards` `CATALOG_VERSION`) and sends it
with every deck save and queue request. The server refuses any request whose catalog version is not
its own (SPEC §9.4, R105). Render deploys the server from `main` on every push, but Cloudflare now
deploys only once a day. Without a fast path, a merge that bumps the catalog would leave production
with a new server and a day-old client, and **every deck save and queue in production would be
refused until the next daily run**.

So the workflow also runs whenever CI completes on main, and every check starts with this one (so
a lost event is made up by the next). It promotes only if the commit changes `render.yaml`'s
`CATALOG_VERSION` compared to what `production` has, through the same kind of pull request, and **a
hold does not stop it** (the server already runs the new catalog, so holding the client back is what
breaks production). It comments on the open countdown issue and leaves its time alone. Every other
change waits for the countdown. The gap that remains is the time for CI plus a Cloudflare build
after Render has deployed (roughly 10 to 20 minutes). Section 7 discusses closing that gap.

### 2.2 The rules the promotion workflow enforces

These are all tested against a simulated repo before this change shipped:

- The candidate must be on `main` and have a successful push-to-main CI run. A commit whose CI is
  still running, failed, or was cancelled is never promoted; the daily run takes the newest green
  one instead.
- `production` only receives merges of green main commits. Before opening anything, the run checks
  that `production`'s files equal those of the main commit it last merged
  (`git diff <merge-base> production` is empty). If someone committed to `production`, the run fails
  with an error and changes nothing, and after the merge it checks that `production`'s files equal
  the candidate's. It never force-pushes.
- Asking it to promote an older commit than `production` already has is a no-op, not a rollback.
- The first promotion creates `production`, by pushing the candidate: there is no pull request to
  open against a branch that does not exist.
- The pull request lists the catalog version and every commit that goes live, and the run summary
  repeats it.

### 2.3 The countdown issue: hold and delay

Exactly one issue labelled `production merge` is open at a time. Its title reads "Merging to
production in 23 hours (2026-10-05 15:00 UTC)" when it is opened and again whenever a command changes
it, so the hours are right at that moment and the date always is. The first line of its body is a
hidden state line (the time, whether it is held, the last comment read); leave it alone.

Anyone with write access comments on the issue:

| Comment | Effect |
| --- | --- |
| `/hold` (a reason after it, if you like) | Nothing merges until `/resume`. The title says "on hold". |
| `/resume` (or `/release`, `/unhold`) | Lifts the hold. If the time has passed, it merges at once. |
| `/delay 3h`, `/delay 2d` | Moves the merge later by that long. At most 168 hours at a time; use `/hold` for longer. If the time has already passed, it counts from the comment. |

A command is a line that starts with the slash, in any case. Each is acknowledged with a 👍 (😕 when
it was refused: the commenter has no write access, or it is not understood), and a comment wakes the
workflow at once, so the answer takes a minute rather than an hour. Commands apply once, in the order
they were written; editing a comment later changes nothing.

The check merges when the issue's time has passed and it is not held. It runs on every green CI run
on `main`, on every command comment, on a manual run and, if GitHub starts it, from the hourly cron
(minute 7). GitHub has not started this repository's scheduled runs, so in practice the first green
push after the time is what merges: a merge comes a little after its time, never before. When it merges, the issue is closed as completed with a link to the pull
request, and the next one is opened for the first 15:00 UTC at least 12 hours away. A manual run
(section 5) merges at once, hold or no hold, and starts the countdown over.

The label is the custom tag: the workflow finds the open issue by it, and it is on every promotion
pull request too. To rename it, change `RELEASE_LABEL` in `scripts/promote-production.sh` and the
label named in the job's `if` in `promote-production.yml`; `promote-production.test.ts` fails until
they agree. The hour (`RELEASE_HOUR_UTC`) and the other numbers are at the top of the script.

It moves `production` with the built-in `GITHUB_TOKEN` (permission `contents: write`); a push made
with that token starts no other workflow, so it cannot loop. Its `deploy` job uses one secret,
`CLOUDFLARE_API_TOKEN`.

## 3. What is in this change

- `wrangler.jsonc`: serves `apps/web/dist` as static assets on Cloudflare Workers. There is no
  Worker script, so there are no runtime variables and no secrets.
- `apps/web/public/_redirects`: the paths that serve the app, copied from `vercel.json`. Every other
  path gets `public/404.html` with a 404 status.
- `apps/web/public/_headers`: the CSP and security headers, copied from `vercel.json`.
- `apps/web/src/net/cloudflare-config.test.ts`: fails CI if the three files above drift from
  `vercel.json`.
- `.github/workflows/promote-production.yml`: when the promotion runs (section 2), and
  `scripts/promote-production.sh`: what it does. `apps/web/src/net/promote-production.test.ts` runs
  the script against a throwaway repository and a stand-in for `gh`.
- `vercel.json` and `apps/web/vercel.json`: `production` and `promote/**` added to
  `git.deploymentEnabled` as `false`, so Vercel never builds Cloudflare's branch or the head branches
  of the promotion pull requests (they would spend Vercel's daily deployment cap);
  `deploy-routes.test.ts` holds it.
- `CLAUDE.md`: the deployment paragraph names both hosts.

Two Cloudflare behaviours shaped `_redirects`, both measured under `wrangler dev` (wrangler 4.147)
against the real build. A rewrite to `/index.html` is answered with a 307 redirect to `/`, which
throws the path away, so every rewrite targets `/` instead. And a `/login` rule does not match
`/login/`, so each path is listed both ways. With those, every case in `deploy-routes.test.ts`
(screens, match and series ids, trailing slashes, and the paths that must 404) behaves the same on
Cloudflare as on Vercel, query strings survive (so emailed auth links work), and the headers are on
every response.

## 4. What you need to do

Do these in order. Steps 2 to 4 are where most mistakes would happen.

### Step 1: Merge this pull request, and allow the workflow to open pull requests

`promote-production.yml` has to be on `main` before it can run or be started by hand. It opens and
merges pull requests with the built-in token, which GitHub allows only when two repository settings
say so: Settings -> Actions -> General -> Workflow permissions -> **Allow GitHub Actions to create
and approve pull requests** (for an organization's repository, the organization's setting must allow
it first), and Settings -> General -> Pull Requests -> **Allow merge commits**. Without the first,
a run fails with "GitHub Actions is not permitted to create or approve pull requests"; without the
second, the merge is refused.

### Step 2: Create the `production` branch

GitHub -> Actions -> **promote production** -> **Run workflow** (leave `sha` empty). This creates
`production` at the newest green commit of main. Check that the branch exists before step 3, because
Cloudflare needs a branch to point at.

Then protect it: Settings -> Rules -> Rulesets -> New branch ruleset, target `production`, and enable
**Restrict deletions** and **Block force pushes**. Do **not** enable "Require a pull request" or
required status checks on it; those would block the push that creates the branch and hold back the
promotion pull requests, which no check runs on. Merging a pull request still works under these two
rules, and they make an accidental `git push -f` to production impossible.

### Step 3: Set up Cloudflare

GitHub Actions builds and deploys production (section 2, step 4), so Cloudflare needs one API token
and no build settings. Cloudflare's own Git builds failed on every commit after the first, with logs
only the dashboard shows, and built every branch, which put a red "Workers Builds: jackioh" check on
every pull request (#207). They are switched off here.

1. **Make an API token.** Cloudflare dashboard -> your profile icon -> My Profile -> API Tokens ->
   Create Token -> the **Edit Cloudflare Workers** template -> Use template. Account Resources:
   Include, your account. Zone Resources: All zones from that account (or only the zone of your
   custom domain). Continue to summary -> Create Token, and copy it (it is shown once).
2. **Give it to GitHub.** GitHub -> `Jlore-Studios/JackiOh` -> Settings -> Secrets and variables ->
   Actions -> New repository secret. Name `CLOUDFLARE_API_TOKEN`, value the token. Only
   `promote-production.yml` reads it, and only on `main` and `production`.
3. **Switch off Cloudflare's builds.** Workers & Pages -> `jackioh` -> Settings -> Build -> Git
   repository -> Disconnect. The Worker keeps serving what was deployed last; nothing builds there
   any more, so no check appears on pull requests and no failed build can leave production behind.
4. **Deploy now.** GitHub -> Actions -> promote production -> Run workflow, tick **redeploy**. The run's
   summary ends with "production is live at https://jackioh.<your-account>.workers.dev"; a red step
   names what to fix.

**Settings this does not need.** No build variables: the four public `VITE_` values are in
`apps/web/.env.production` (the publishable key too, or as a repository variable
`VITE_SUPABASE_PUBLISHABLE_KEY`), and Node, pnpm and the install come from the same setup CI uses.
The Worker's name is `jackioh` (`wrangler.jsonc`), and its account is `CLOUDFLARE_ACCOUNT_ID` in
the workflow. **Never** put `SUPABASE_SECRET_KEY`, `DATABASE_URL` or `CODE_PEPPER` in the web build
or in Cloudflare: anything the web build reads is readable by every visitor.

**Custom domain:** Worker -> Settings -> Domains & Routes -> Add -> Custom domain. Until you add one,
the site is on `jackioh.<your-account>.workers.dev`.

### Step 4: Tell the backend about the new origin

- **Render deploy hook** (once): `jackioh-server` -> Settings -> Deploy Hook -> copy the URL, and save
  it as the GitHub repository secret `RENDER_DEPLOY_HOOK_URL`. `deploy-watch.yml` then asks Render to
  deploy every push to `main` itself, so a deploy no longer depends on Render's GitHub integration,
  which stopped delivering pushes twice (after the repository moved, and again after it was
  reconnected, #224). Keep Auto-Deploy on "On Commit" too; never "After CI checks pass" while any
  check on `main` can be red for reasons that are not the server's.
- **Render** -> `jackioh-server` -> Environment -> `PUBLIC_ORIGINS`: add the Cloudflare origin,
  comma-separated, keeping `https://jackioh.vercel.app` for staging. Example:
  `https://jackioh.vercel.app,https://play.example.com`. This list gates both CORS and the WebSocket
  Origin check (R162), so a missing entry breaks matches, not just page loads. Saving restarts the
  server.
- **Supabase** -> Authentication -> URL Configuration -> **Redirect URLs**: add
  `https://<cloudflare-domain>/login`. Emailed links (sign-up confirmation, password reset) are
  built from the page's own origin (`auth.ts` `authRedirectUrl`), and Supabase silently replaces any
  `redirect_to` that is not on this list with the Site URL. Keep the Vercel entry for staging.
- **Supabase Site URL:** change it to the Cloudflare domain at cut-over (step 6), not before.
- **`TRUSTED_PROXY_HOPS` on Render does not change.** Browsers talk to Render directly; Cloudflare
  only serves the static files. It would change only if Render itself were put behind Cloudflare.

### Step 5: Verify production

Replace `$SITE` with the Cloudflare URL.

1. `curl -sI $SITE/practice` shows `200` and a `content-security-policy` header.
2. `curl -s -o /dev/null -w '%{http_code}\n' $SITE/match/abc` prints `200`, and
   `$SITE/loginx` prints `404`.
3. In a browser: load `/decks` directly (not by clicking), reload it, and confirm it stays on the
   deck screen instead of jumping to the home page.
4. Sign in. Open DevTools -> Network and confirm requests go to `jackioh-server.onrender.com`, not
   `localhost`.
5. Request a password reset and confirm the email link opens the Cloudflare domain.
6. Save a deck and queue for a match. A "stale catalog" refusal here means the bundle and server
   disagree on the catalog version (section 2.1).
7. DevTools -> Console shows no CSP violations.

### Step 6: Cut over the canonical domain

When the Cloudflare domain is final, make one change that moves every place that names the
canonical origin, which still say `jackioh.vercel.app`:

- `apps/web/src/net/navigate.ts` `SITE_ORIGIN`
- `apps/web/index.html`: `og:url`, `og:image` and the JSON-LD `url`
- `apps/web/public/robots.txt`, `public/sitemap.xml`, `public/.well-known/security.txt`
- `apps/web/src/routes/privacy.tsx`: "Vercel hosts this website" should name Cloudflare. This is
  your privacy policy, so it has to be accurate about who hosts the site.
- `apps/server/test/deploy/rehearse.sh` `PUBLIC_ORIGINS`
- `e2e/cypress/e2e/99-online-smoke.cy.ts`: the example `E2E_BASE_URL`

In the same change, add `{ "key": "X-Robots-Tag", "value": "noindex" }` to the headers in **both
`vercel.json` copies only** (not to `_headers`), so search engines stop indexing staging as a
duplicate of production. `cloudflare-config.test.ts` compares the two header lists and will need
to allow that one extra Vercel header. Then set the Supabase Site URL to the Cloudflare domain.

## 5. Day-to-day operation

- **Normal day:** merge to main as usual. Staging updates within minutes; production catches up when
  the countdown issue's time comes (15:00 UTC). To change the hour, edit `RELEASE_HOUR_UTC` in
  `scripts/promote-production.sh`; the hourly `cron` stays as it is.
- **Not yet, I'm mid-change:** comment `/hold` on the countdown issue, and `/resume` when the work is
  done; or `/delay 6h` for a known wait (section 2.3). Neither stops a catalog bump.
- **Ship now (hotfix):** merge the fix, wait for its CI on main to pass, then Actions ->
  promote production -> Run workflow. It promotes the newest green commit, or the `sha` you give, at
  once and whatever the hold, and the countdown starts over.
- **Catalog bump:** nothing to do; it promotes itself after CI passes. Watch the Actions tab for the
  promote run, then do verification step 6.
- **Roll back:** Cloudflare -> Deployments -> pick the last good one -> Rollback. This is instant
  and does not touch git. Then fix forward on main. Because the next daily run would deploy main's
  newest green commit again, disable the workflow (Actions -> promote production -> "..." -> Disable
  workflow) until the fix has merged, then re-enable it.
- **Pause production:** `/hold` on the countdown issue (a catalog bump still goes out), or disable the
  workflow to stop everything. Staging keeps updating.
- **A promote run fails with "production has diverged":** someone committed to `production` by hand
  or main was rewritten. Compare with `git log --oneline --graph origin/main origin/production`. If
  nothing on `production` needs keeping, an admin can reset it with
  `git push --force origin <green-main-sha>:production` (temporarily allowing force pushes in the
  ruleset), and the next run continues normally. The hourly check fails the same way until then.
- **A promote run fails with "could not be merged into production":** the pull request is left open
  and says why on its page (a setting, a conflict). Fix that and the next hourly check opens a new
  one; close the old one.
- **The countdown issue is missing or doubled:** the hourly check opens one when none is open and
  closes all but the newest as not planned, so closing it by hand only starts a fresh countdown for
  the next 15:00 UTC that is at least 12 hours away.
- **deploy-watch opens "Render: the live server is not serving main's catalog":** it compares the
  version and the commit the live server reports (`x-deployed-commit`, from Render's
  `RENDER_GIT_COMMIT`) with the push, and its issue says which of three things it is. The server
  runs an older commit: Render is not receiving pushes, or a deploy failed. jackioh-server, Events
  shows a "Deploy started" for every push it receives, and a failed deploy names its step in the
  log. The server runs the pushed commit but another catalog version: render.yaml's start command
  takes the version from `patches.json`, so `render.yaml`'s `CATALOG_VERSION` and that list
  disagree, or the service is not running render.yaml's start command (Settings, Build & Deploy;
  Blueprints). The server reports no commit: it predates the check, or `RENDER_GIT_COMMIT` is not
  set. The run that finds the server live closes the issue.
- **Render stopped deploying again after the reconnect (#224):** the live server stayed on the
  commit the manual sync deployed, and no push after it arrived. The `RENDER_DEPLOY_HOOK_URL` secret
  (section 4, step 4) makes `deploy-watch.yml` request each deploy itself; until it is set, Manual
  Deploy, "Deploy latest commit", is the way to catch up.
- **Render stopped deploying (the repository moved):** when the repository was transferred from
  `jgoetzmann` to `Jlore-Studios`, Render kept the last deploy it had made (Oct 2) and received no
  push after it, so a catalog change (v0.2.4) never arrived and nothing flagged it, because the
  watch then compared only the catalog version and the version had not changed. To repair it:
  jackioh-server, Settings, Build & Deploy: the repository must be `Jlore-Studios/JackiOh`, the
  branch `main`, Auto-Deploy on commit ("After CI checks pass" would wait on the red Cloudflare
  check). If the repository is not offered, an owner of the organization has to authorize Render's
  GitHub app for it. The service is Blueprint-managed, so also check Blueprints, the repository of
  the Blueprint. Then Manual Deploy, "Deploy latest commit". The `CATALOG_VERSION` in Environment is
  ignored: the start command overwrites it at every boot from `patches.json`, so a stale copy there
  cannot be served (delete it, so nobody edits it expecting an effect).
- **The deploy job fails:** the commit is on `production` but not live. The run's log names the
  step: the build, the bundle check (a missing `VITE_` setting), the deploy (the token), or the live
  check. Fix it on `main`, then Actions -> promote production -> Run workflow with **redeploy**.
- **Scheduled runs stop, or never start:** GitHub disables scheduled workflows in a repository with
  no activity for 60 days, and may start a schedule late or not at all. Re-enable it in the Actions
  tab. Nothing depends on it: every green push to `main`, a comment on the countdown issue and a
  manual run all run the same check (section 2, step 3).

## 6. What needs no secret

- Cloudflare: none. The `VITE_` settings are all public and committed, and there is no Worker
  script.
- GitHub: `CLOUDFLARE_API_TOKEN`, which the deploy job uses to upload the build. Moving `production`
  uses the built-in `GITHUB_TOKEN`, with the two repository settings in step 1. Pull requests and
  issues made with that token start no other workflow, so `ci.yml` does not run on a promotion pull
  request, which is right: its commits already passed.
- Render and Supabase: unchanged, except for the `PUBLIC_ORIGINS` value and the Redirect URL above.

## 7. Known gaps and recommended next steps

- **The server can be up to a day ahead of production's client.** The catalog fast path covers the
  catalog version, but any other change to the API or the match protocol (`WS_SUBPROTOCOL`, request
  and response shapes, engine rules the client also runs) reaches production's server immediately
  and production's client up to a day later. Until that changes, every server change must keep
  working with the previous day's client, or must be promoted by hand right after it merges.
- **Staging shares production's data.** Accounts, decks and invite codes made on the Vercel site
  are real rows in the production Supabase project, and staging matches run on the production
  server.
- **The recommended next phase** fixes both: a second Supabase project and a second Render service
  for staging (both on `main`), with the production Render service switched to the `production`
  branch (`render.yaml` `branch: production`, and `deploy-watch.yml` triggered by pushes to
  `production`). Then the production server and client ship together once a day, staging is fully
  separate, and the catalog fast path is no longer needed.
- **Checking what went live.** `deploy-watch.yml` checks Render after every push (catalog version and
  deployed commit), and the promote run's `deploy` job checks Cloudflare after every deploy (sign-in
  configured, routing, headers). Neither compares the live client's commit with `production`'s; a
  red deploy job is the alarm for the client.
