# v0.3.0 (part 38 of 40): cutover: staging into main, deploys and the bots' settings

Part 38 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 4 (cutover) |
| Starts after | part 37, part 30 |
| Branch | `staging` → `main` |
| Builder | a person (or a session a person runs and watches) |
| Notes | a comment on this issue when done |

Unlabelled on purpose: #306 holds labels until a person says the run is ready. The Plan below is between the night bot's plan markers, so if a person later queues this issue for a bot, it builds from this plan without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Land `staging` on `main` and make `main` (the staging environment: Vercel for the web, Render for the server, one Supabase) run the Rust: the cutover pull request, the Render service on Docker, the Vercel build, branch protection's required checks, and the bots' settings that hard-code the TypeScript toolchain.

**How you work.** Normal engineering: build and test as you go; a person does the steps that need dashboard access.

### 2. Files to touch

| File | new/change/delete | What |
|---|---|---|
| `.harness/config.json, .squishy/config.json` | change | `install`, `gates` (cargo fmt/clippy/test + spec/catalog checks + web vitest --changed), `required_checks` (part 30's names), `easy.off_limits` and `generated` (crates paths), `review_paths` (Cargo files) |
| `bot/harness/easy.py, bot/harness/work.py, bot/tests/*` | change | the pnpm/vitest hard-codes research D §1.6 lists |
| `.github/workflows/bot-night.yml, squishy-run.yml` | change | Rust toolchain setup beside pnpm |
| `branch protection on main` | settings | required checks = part 30's names |
| `Render service jackioh-server` | settings | Blueprint sync to render.yaml's runtime: docker; secrets unchanged |
| `Vercel project` | settings | the installCommand from vercel.json; nothing else |


### 3. Steps

1. Merge `main` into `staging` one last time; green CI on staging.
2. Open the pull request `staging` → `main` titled `v0.3.0: the Rust rewrite` (body: README §1, the V table with each one's evidence, the cull report). Merge it with a merge commit (not squash: the parts' history stays).
3. Within the same hour: branch protection's required checks; `.harness/config.json` and `.squishy/config.json`; the bot code and workflows above (a person's pull request).
4. Watch `deploy-watch.yml` see `/api/catalog` serve the version on the merged commit; play one online match on Vercel against Render; resume one practice save made before the cutover (proves §5.2).
5. Post the outcome on #306; close parts 1–38.

### 4. Tests

- deploy-watch green; one online match and one resumed practice save, by hand.

### 5. Done when

- [ ] `main` is the Rust; Render and Vercel serve it; the bots' gates pass on their next run.

### 6. Risks

- If Render fails to boot the image, Render keeps the previous deploy; fix forward on main.

<!-- /jackioh-bot:plan -->
