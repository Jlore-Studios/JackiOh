# apps/edge: the Worker in front of the site and its game server

One Cloudflare Worker per environment serves the whole site at one origin (`wrangler.jsonc`):

| | Production | Staging |
| --- | --- | --- |
| Worker | `jackioh` (the config's top level) | `jackioh-staging` (`env.staging`) |
| Address | https://jackioh.com, and `jackioh.<account>.workers.dev` | https://staging.jackioh.com, behind Cloudflare Access; no workers.dev or preview address |
| Deployed by | `promote-production.yml`, from `production` | `deploy-staging.yml`, from `main` once its CI passes |
| Container | `jackioh-server` | `jackioh-staging-server` |

Every path but `/api/*`, `/ws/*` and the engine's module is a static file from `apps/web/dist`, served by
Cloudflare without running this code (`assets.run_worker_first`), so page loads cost nothing. The module
(`/assets/*.wasm-*.br`, #3183) is stored brotli-compressed, and this Worker serves the stored file from
the `ASSETS` binding with `Content-Encoding: br`, `Content-Type: application/wasm` and
`encodeBody: "manual"` (`src/routing.ts` `precompressedModule`), so the browser decodes it natively and
compiles it while it downloads. A `_headers` rule could not do it: Cloudflare's asset Worker applies
`_headers` to a response it built without `encodeBody: "manual"`, so the runtime would compress the
stored file again. It costs one Worker request per download of the module, which a browser then keeps
for a year (`MODULE_CACHE_CONTROL`). Those two prefixes reach
`src/index.ts`, which hands them to the `Server` Durable Object, which runs one container of
`crates/server/Dockerfile`'s image (`jackioh-server release`: migrate, seed the catalog, serve). The
custom domains and Access are `infra/cloudflare.tf`'s.

## How it behaves

- **One instance.** The server holds every live match, queue and series in memory, so every request
  reaches the same container (`INSTANCE`, `max_instances: 1`). It is placed in eastern North
  America (`constraints.regions`, `locationHint`), beside Supabase's us-east-2.
- **It sleeps.** After `SLEEP_AFTER` (5 minutes) with no request, the container is stopped (SIGTERM,
  so the server drains), and the next request starts it again; that request waits for the boot, up
  to `START_WAIT_MS` (60 seconds, where the library's own wait is 20 and a slower boot's first caller
  got a 500). An open WebSocket counts as a request in flight, so a live match, a watcher or a co-op room keeps it
  awake, and a signed-in tab keeps it awake with its presence beat (`PRESENCE_HEARTBEAT_MS`, 45 s).
  Memory and disk are billed only while it is awake.
- **A deploy reaches the running server.** A container keeps the image it started with, so the
  first request after a deploy asks the running server's `/healthz` for its `x-deployed-commit`; if
  that is not the Worker's `DEPLOYED_COMMIT`, it stops the old server (giving it `RETIRE_WAIT_MS` to
  drain) and the request starts the new one. Live matches rebuild from their seed and log when
  their sockets return (SPEC §9.5).
- **Once a day** (`triggers.crons`) the Worker requests `/readyz`, which wakes the server: its hourly
  loops run, and the Supabase project sees a query, so a quiet week does not pause a free project.
- **The caller's address.** The Worker replaces `X-Forwarded-For` with the address Cloudflare saw
  (`CF-Connecting-IP`), and `TRUSTED_PROXY_HOPS` is 1 (R190). Calibrate it once after the first
  deploy: the server's `api.forwarded_for` log line shows the entries it received; if the platform
  appends one of its own, it is 2.

## Settings

The server's settings that are not secrets are `vars` in `wrangler.jsonc`, one set per environment
(`SERVER_VARS` in `src/routing.ts`). The secrets are set once per Worker and never in git
(`SERVER_SECRETS`; `crates/server/README.md` says what each is):

```
cd apps/edge
pnpm exec wrangler secret put SUPABASE_SECRET_KEY --config ../../wrangler.jsonc --env=""
pnpm exec wrangler secret put DATABASE_URL --config ../../wrangler.jsonc --env=""
pnpm exec wrangler secret put CODE_PEPPER --config ../../wrangler.jsonc --env=""
```

and the same three with `--env staging`. `CREATE_ISSUES_TOKEN`, `DEV_PROFILE_IDS`, `ALERT_RESEND_API_KEY`
and `ALERT_EMAILS` are optional. The last two email the server's alerts (#3595, `crates/server/README.md`,
Alerts): a Resend API key with Sending access for jackioh.com, and the addresses, comma-separated, kept a
secret since the repository is public.
`DATABASE_URL` is Supabase's pooler URI (Connect -> Session pooler), which works over IPv4; the
direct database host is IPv6 only. A container reads its settings when it starts, so after changing
one, deploy again (or wait for the next sleep).

## Commands

From the repository root, with `CLOUDFLARE_API_TOKEN` exported, Docker running (wrangler builds the
image) and the web built into `apps/web/dist`:

```
pnpm --dir apps/edge exec wrangler deploy --config ../../wrangler.jsonc --env="" --var DEPLOYED_COMMIT:$(git rev-parse HEAD)
pnpm --dir apps/edge exec wrangler deploy --config ../../wrangler.jsonc --env staging --var DEPLOYED_COMMIT:$(git rev-parse HEAD)
pnpm exec vitest run --project edge     # src/*.test.ts: the routing and wrangler.jsonc
pnpm --dir apps/edge typecheck
```

The workflows run the deploys; a person runs them only to set things up. `--env=""` names the top
level, production, explicitly.
