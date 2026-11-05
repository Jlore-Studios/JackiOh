# `@jackioh/admin`: the dev dashboard

The dev dashboard of issue #3074: one page, at its own hostname, where a dev account sees and changes
its own rating, reads every row of the leaderboard (bots included), posts and takes down news, reads the
audit trail of every admin action, and uses everything the web's Dev tools panel offers
(`apps/web/src/routes/DevTools.tsx`). It is a separate Vite + React app with its own Cloudflare Worker;
the player site (`apps/web`) does not link to it.

## Three locks

Each is enough on its own to keep a player out; the last is the one that decides.

1. **Cloudflare Access on the hostname.** `admin.jackioh.com` and `admin-staging.jackioh.com` are behind
   an Access application whose policy lists the devs' emails, so nobody else gets as far as the page.
   `infra/cloudflare.tf` owns both hostnames and their Access applications; this app's `wrangler.jsonc`
   names no route or custom domain, and has no `workers.dev` or preview address, either of which would
   be a way around Access.
2. **The account's own sign-in**, against its Supabase project (`src/auth.ts`): email and password, or a
   code mailed to the address (so an account that signs in with Google alone can sign in here too), then
   the authenticator app's code when the account has one. On a strict server (`strict` in
   `GET /api/admin/me`, production's rules) the server requires the session to be `aal2`, so a dev
   account turns two-step sign-in on at `/account` on the site first.
3. **The server's own checks on every route**: the bearer token, the account in `DEV_PROFILE_IDS`, and on
   a strict server the session's `aal2`. A 403 here shows the server's message and Sign out, nothing else.

## The one rule

CLAUDE.md rule 7: **the dashboard sends intent and shows the server's answers.** Every button sends what
was typed (an amount or a rating as `Number(text)`, a title and summary as typed); the server checks it,
and a refusal's `message` (`{ "error": { "code", "message" } }`) is shown to the dev word for word in the
status line (`role="status"`). Every number shown is one the server sent. Rule 9: the dashboard restates
no server number; its timeouts and renewal margins are the server's, from the generated
`apps/web/src/wire/serverConfig.ts`, and the set tracks it offers are `SHIPPED_SETS`
(`apps/web/src/wire/catalog.ts`), the list the web's Dev tools read. Those two modules, both data with no
imports, are all it takes from `apps/web` (`src/wire.ts`). Its own numbers are named constants.

Titles, summaries, usernames and audit entries are React text, never markup: no source uses
`dangerouslySetInnerHTML`, `innerHTML`, `outerHTML` or a document write (`src/no-raw-html.test.ts`).

## What it does

One page, in sections (`src/Dashboard.tsx`), one request at a time; each answered request reads every
section again.

- **Account**: the username and profile id, whether the server is strict, whether two-step sign-in is
  on, and Sign out (which forgets the tab's session and revokes it, `scope=local`).
- **Holdings and Dev tools** (`src/Holdings.tsx`): the account's Jlore (`GET /api/wallet`) and held
  quests (`GET /api/quests`), then the Dev tools with exactly `apps/web/src/net/api.ts`'s bodies: XP to a
  shipped set's track, Jlore, Add all cosmetics and cards, each clear, both token bars, give a quest,
  complete a held quest.
- **Rating** (`src/Rating.tsx`): both ladders' rating, deviation, volatility, the hand-set flag and the
  rating the games had earned; Set (`PUT /api/admin/rating`) and, when hand-set, Reset to earned
  (`POST /api/admin/rating/reset`). On a strict server a hand-set rating is off the public ladder until
  reset.
- **Leaderboard** (`src/Leaderboard.tsx`): every row of either ladder, people and bots marked, a bot's
  strength and persona beside its row (read-only), hand-set ratings marked.
- **News posts** (`src/News.tsx`): post (`POST /api/admin/news`), every post newest first, Take down
  (`DELETE /api/admin/news/{id}`). A post pays no reward; a reward goes in `crates/server/news.json` by
  pull request.
- **Audit** (`src/Audit.tsx`): every admin action, newest first, its request and the state before and
  after as `JSON.stringify` text.

## Sign-in and the session

`src/auth.ts` talks to Supabase as `apps/web/src/net/auth.ts` does: `grant_type=password`, or
`/auth/v1/otp` (`create_user: false`) then `/auth/v1/verify` (type `email`). When the session's `aal`
claim (read from the token, never verified) is `aal1` and the user object (`GET /auth/v1/user`) has a
verified TOTP factor, it asks for the authenticator code (`/auth/v1/factors/{id}/challenge`, then
`/verify`) and keeps only the `aal2` session that returns. The provider's own words are never shown.

The session lives in this tab's `sessionStorage` (`jackioh.admin.session`), never `localStorage`
(`src/session.ts`). It is renewed `AUTH_SESSION_REFRESH_MARGIN_SECONDS` before it expires, and before any
request made inside that margin; a refused renewal ends it and says so. Then `GET /api/admin/me` decides:
200 opens the dashboard, 403 shows the server's message and Sign out, 401 returns to sign-in with the
server's message.

**Development and e2e builds only** (`MODE` is not `production`): at boot, a
`localStorage["jackioh.e2e.session"]` holding `{ "accessToken": "…" }` is used as the session and
sign-in is skipped, as `apps/web/src/net/session.ts` does for the e2e suite's fixture accounts. Vite
folds the `MODE` test away in a production build, and the key with it: `build.test.ts` builds both and
checks.

## The Worker

`worker/index.ts`, one Worker per environment (`wrangler.jsonc`):

| | Production | Staging |
| --- | --- | --- |
| Worker | `jackioh-admin` (the config's top level) | `jackioh-admin-staging` (`env.staging`) |
| Address | https://admin.jackioh.com, behind Access | https://admin-staging.jackioh.com, behind Access |
| `SITE` service binding | `jackioh`, the site's Worker | `jackioh-staging` |
| Supabase project (`.env.*`) | JackiOh-Production | JackiOh-Staging |

Every path but `/api/*` is a static file from `apps/admin/dist` (`not_found_handling:
single-page-application`). `/api/*` runs the Worker (`run_worker_first`), which hands to
`env.SITE.fetch(request)` only these (`worker/routes.ts`, `forwards(method, pathname)`):

- `GET /api/auth/me`
- any method under `/api/admin/`
- any method under `/api/dev/`
- `GET /api/quests`, `GET /api/wallet`

and answers everything else under `/api/` 404 `{ "error": { "code": "not_found", … } }` without
forwarding it. The site's Worker then hands the request to the game server as it does the site's own, so
the page and its API share one origin and need no CORS.

`public/_headers` sets a strict CSP (`connect-src` is `'self'` and the build's own Supabase origin, which
`headers.ts`, a small Vite plugin, writes in at build time from `VITE_SUPABASE_URL`), `frame-ancestors
'none'`, `X-Frame-Options: DENY`, `Referrer-Policy: no-referrer` and `X-Robots-Tag: noindex`.

## Commands

From the repository root, after `pnpm install`:

```
pnpm --dir apps/admin dev                 # http://localhost:5174, /api proxied to http://localhost:8787
pnpm --dir apps/admin build               # production's bundle into apps/admin/dist (.env.production)
pnpm --dir apps/admin build:staging       # staging's (.env.staging over .env.production, MODE stays production)
pnpm --dir apps/admin build:e2e           # a development-mode bundle, which reads the e2e session
pnpm --dir apps/admin exec vite preview   # serves dist on http://localhost:5174, /api proxied to :8787
pnpm --dir apps/admin typecheck           # the page (tsconfig.json) and the Worker (worker/tsconfig.json)
pnpm exec vitest run --project admin      # every test below
pnpm exec eslint apps/admin
```

Locally the API is the e2e server, `E2E=1 cargo run --release -p jackioh-server` on :8787, whose fixture
accounts the e2e suite signs in through `jackioh.e2e.session`; so
`pnpm --dir apps/admin build:e2e && pnpm --dir apps/admin exec vite preview` serves a dashboard the e2e
suite can drive at http://localhost:5174 (the fixture account must be in that server's
`DEV_PROFILE_IDS`).

### Tests

- `src/auth.test.ts`: password, mailed code, the two-step code, renewal and sign-out against a fake `fetch`.
- `src/api.test.ts`: headers, bodies, the error shape, the timeout, and every route's request.
- `src/session.test.ts`: `sessionStorage` only, the e2e session in development and not in production,
  renewal ahead of expiry.
- `src/App.test.tsx`: each way to sign in, the two-step step, the 403, 401 and fault screens, the e2e boot.
- `src/Dashboard.test.tsx`: every section from canned answers, and every button's request.
- `src/no-raw-html.test.ts`, `src/env.test.ts` (the two `.env` files hold only the web's same two values).
- `worker/index.test.ts`: the allow-list, and that a refused path is never forwarded.
- `wrangler.test.ts`, `headers.test.ts`, `build.test.ts`: the config, the CSP, and real production and
  e2e builds.

### Deploying

With `CLOUDFLARE_API_TOKEN` exported, and the bundle of that environment built into `apps/admin/dist`
(`build` for production, `build:staging` for staging), deploy with apps/edge's wrangler:

```
pnpm --dir apps/edge exec wrangler deploy --config ../admin/wrangler.jsonc --env=""
pnpm --dir apps/edge exec wrangler deploy --config ../admin/wrangler.jsonc --env staging
```

`--env=""` names the top level, production, explicitly. The site's Worker (`jackioh`, `jackioh-staging`)
must already exist: the service binding names it.

## Test ids

Every control and read value has a `data-testid` from `src/testids.ts`'s `testids`, which the e2e suite
drives: `admin-sign-in-*` (`email`, `password`, `submit`, `method-password`, `method-code`, `send-code`,
`code`, `verify-code`, `totp`, `totp-submit`, `cancel`, `notice`, `error`), `admin-forbidden`,
`admin-sign-out`, `admin-status` (with `data-state`), and per section `admin-account-*`, `admin-xp-*`,
`admin-jlore-*`, `admin-fill`, `admin-bar-<bar>`, `admin-clear-<part>`, `admin-quest-*`,
`admin-rating-{value,deviation,volatility,hand-set,earned,input,set,reset}-<ladder>`, `admin-ladder-<ladder>`,
`admin-leaderboard-row` (with `data-profile-id`, `data-kind`, `data-hand-set`, `data-you`), `admin-news-*`
(`admin-news-row` with `data-post-id`, `data-taken-down`) and `admin-audit-row` (with `data-action-id`).

## Left to a person

- The Access applications' policies: the devs' emails in `infra/terraform.tfvars`, then `terraform apply`
  (`infra/README.md`), which also makes both hostnames.
- Each dev account's profile id in the server's `DEV_PROFILE_IDS` secret (`apps/edge/README.md`), and
  two-step sign-in turned on for it at `/account` before it uses production's dashboard.
- The first deploys above, and promoting the dashboard to production with the rest of a release.
