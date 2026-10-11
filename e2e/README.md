# `e2e/` — the thirty-six specs: BUILD M8's seventeen, `18`–`28`, patch v0.2.0's `29`–`32`, the Card Almanac's `33`, the public Statistics page's `34`, the settings dialog's `35` and the Meditative play-through's `36`, plus sixteen component specs

Cypress runs against `apps/web` in `E2E=1` mode: the `/dev/hotseat` route for the local specs and
a test server with fixture accounts for the networked ones, the Rust `jackioh-server` started with
`E2E=1` (`crates/server`). BUILD M8's house rules hold
everywhere in here:

- every spec sets a seed;
- there is no fixed `cy.wait(ms)` — `support/e2e.ts` throws if a spec calls one. Waits are
  assertions: `data-animating` clearing (`cy.settled()`), an animation appearing
  (`cy.expectAnimating`), a prompt (`cy.waitForPrompt`), or a testid;
- each spec asserts what BUILD's "Key assertions" column says for its row, quoted verbatim in the
  spec's header comment.

## Layout

Moved to docs/history/e2e-readme.md.

`e2e/` is a member of the root pnpm workspace, so the root `pnpm install` installs it as CI does, and
it also carries its own `pnpm-workspace.yaml` and lockfile, so `cd e2e && pnpm install` works alone.
`e2e/tsconfig.json` type-checks without the client or the Rust crates being buildable, on purpose: the
`@jackioh/*` specifiers resolve to the client's wire layer (`apps/web/src/wire/`) as its Vite config
resolves them, and the engine is reached only through the `jackioh` binary.

## Install

```
pnpm install                              # repo root: e2e's packages, and tsx, which the replayHash task runs under
cargo build --release -p jackioh-tools    # target/release/jackioh, whose `replay` the replayHash task runs
cd e2e && pnpm install                    # on its own, as above; the postinstall fetches the Cypress binary
pnpm exec cypress verify
pnpm exec tsc -p tsconfig.json
pnpm check:fixtures                       # every deck fixture obeys L2/L3/L6 before a browser is involved
```

## Run

```
# 1. the client, in E2E mode
E2E=1 pnpm --dir apps/web dev                  # must serve http://localhost:5173

# 2. the server, in E2E mode, for the networked specs 05, 06, 09, 10, 18, 19, 20, 26, 27, 35
E2E=1 cargo run --release -p jackioh-server    # http://localhost:8787 and ws://…/ws/match (WS_PATH)

# 3. the suite
cd e2e
pnpm test:e2e                                  # headless, default browser
pnpm test:e2e:chrome                           # Chrome, as CI runs it
pnpm open                                      # interactive
pnpm exec cypress run --spec cypress/e2e/01-hotseat-full-game.cy.ts
```

Under `E2E=1` the server needs no other variable: the Supabase, database and pepper settings fall
back to fixture placeholders, the store is in memory, and `CATALOG_VERSION` defaults to the version
compiled into it. It reseeds its fixture accounts and invite codes at every boot (R144), so restart it
before re-running spec 10. CI (`.github/actions/e2e-shard`) boots one release `jackioh-server` and one
`vite preview` of the `build:e2e` client per shard, waits for `/api/catalog` and the client to answer,
then runs its shard's specs on Chrome: the smoke list (01, 06, 10, 13, 19,
`scripts/shard-specs.mjs`'s `SMOKE`) on every pull request, every spec in the daily super run.
Electron is no longer run.

Endpoints are overridable, so nothing in a spec has to change when a port moves:

```
E2E_BASE_URL=http://localhost:4173 pnpm test:e2e
pnpm test:e2e --expose wsUrl=ws://127.0.0.1:8787/ws/match --expose apiUrl=http://127.0.0.1:8787
```

`support/config.ts` lists every overridable key (`loginRoute`, `inviteRoute`, `deckbuilderRoute`,
`playRoute`, `matchRoute`, `apiUrl`, `wsUrl`, the fixture accounts and the invite codes). Cypress
16 replaced `Cypress.env()` with `expose` / `Cypress.expose()`, which is why the flag is
`--expose`.

Two runs on one machine (parallel agents, a component run beside an e2e run) must not share the
artifacts folder: Cypress empties its screenshots folder at the start of every run, so one run
deletes the other's evidence. Give each its own, and keep what is already there:

```
E2E_ARTIFACTS=artifacts/my-run E2E_KEEP_ASSETS=1 pnpm exec cypress run --spec …
```

`E2E_ARTIFACTS` moves the e2e and the component folders alike (the CLI's `--config
screenshotsFolder` does not reach the component block). Stop a server you started by its port
(`lsof -tiTCP:<port> -sTCP:LISTEN | xargs kill`), never by a process-name pattern, which also ends
everyone else's. Headless Chrome's default window crops a capture taller than about 633 px; pass
`--config viewportHeight=…` (or `cy.viewport`) with a browser launched at a larger `--window-size`
when a shot must show a full 1280x720, 768x1024 or 390x844 screen. `E2E_WINDOW_SIZE=W,H` does the
launch (`cypress.config.ts`, Chrome and Electron; unset, nothing changes):

```
E2E_WINDOW_SIZE=1600,1200 pnpm exec cypress run --browser chrome --spec cypress/e2e/25-overflow-animations.cy.ts --expose shots=1
```

A shot of something in motion also wants `cy.screenshot(name, { disableTimersAndAnimations: false })`:
by default Cypress jumps every CSS animation to its end for the capture, and a card that burns or
fizzles away ends invisible.

## What must be true of the app first

The suite is written against the contract BUILD M5-T1 and M5-T4 fix. Until these hold, specs fail
for contract reasons rather than rules reasons.

1. **Testids.** `zone-<side>-<row>-<lane>`, `card-<instanceId>`, `hero-<side>`,
   `hand-card-<instanceId>`, `end-turn`, `offer-draw`, `power`.
2. **`data-animating="<eventType>"`** on the element animating an event, for that event's
   duration, with `prefers-reduced-motion` collapsing durations to 0 (BUILD M5-T4).
3. **`data-prompt-kind="<kind>"`** on the open prompt modal or inline picker (M5-T4
   `promptOpened` / `promptAnswered`).
4. **`/dev/hotseat?seed=&a=&b=`** exposing `window.__jackioh = { state, dispatch, seed }` outside
   production builds (M5-T3).
5. **`E2E=1`** serving the hotseat route and a test server with fixture accounts (M8 preamble).
6. **The session key.** The client reads its access token from
   `localStorage["jackioh.e2e.session"] = { accessToken }` at boot, because spec 05 reloads
   mid-match and the session has to survive it. Already true:
   `apps/web/src/net/session.ts` reads that key alongside the one a real sign-in writes (in the tab's
   `sessionStorage`, R632).
   `cy.signIn` / `cy.visitAs` write it, and `support/config.ts` is the only place it is spelled.
7. **The socket path.** `ws://<host>/ws/match` — `WS_PATH` in
   `crates/server/src/actor/ws_server.rs`. A handshake off that path is never upgraded, so this is
   not a preference: `support/config.ts`, `cypress.config.ts` and the `wsPlayer` task all default
   to it.

## Assumptions beyond that contract

Moved to docs/history/e2e-readme.md.

Every one of these is either an ask on another team or a convention this suite invented because
BUILD does not fix it. They are all made in `support/`, never in a spec, so each has exactly one
place to change.

| # | Assumption | Where | Ask |
| --- | --- | --- | --- |
| A6 | Routes and fixtures for the non-hotseat specs: `/login`, `/invite`, `/decks`, `/play`, `/match/<id>`; `http://localhost:8787` + `ws://localhost:8787/ws/match` (the path is `WS_PATH`, not an assumption); fixture accounts `e2e-p1`, `e2e-p2` (active, own every card) and `e2e-pending` (pending, verified email); seeded invite codes — one good, one missing, one expired, one exhausted. | `support/config.ts` | `apps/web`, `crates/server` |
| A8 | The WS protocol `cy.task("wsPlayer")` speaks: `-> hello {token?,matchId?,roomCode?}`, `-> action {action:{…,nonce}}`; `<- view {view}`, `<- ack {nonce,seq}`, `<- error {code,message,nonce?}`, `<- prompt {forYou,…}`, `<- clock {now,clocks}`. Read off `crates/server/src/actor/protocol.rs`, which fixes every shape, so this is documentation now rather than an assumption. Joining a room is **not** on the socket: `POST /api/rooms/:code/join` is, and the `joinRoom` frame exists only to answer a client that tries with `error {code:"unsupported"}`. | `support/tasks/wsPlayer.ts` | — |

## What `support/` owns, so a spec does not

Beyond the five BUILD names (`seedGame`, `playCard`, `attack`, `answerPrompt`, `endTurn`) and the
waiting helpers (`settled`, `expectAnimating`, `waitForPrompt`, `noPrompt`):

| Command | What it is for |
| --- | --- |
| `signIn(account)` / `signOut()` / `visitAs(account, path)` | A10's session. `signIn` is remembered for the file, and every later `cy.visit` installs it in `onBeforeLoad`, so a `cy.reload()` mid-match keeps it. |
| `installLoadout(account, fixtureId, { deckIndex })` | A12: the account's saved decks replaced by the fixture at `deckIndex` and two padding decks, plus a trio of the three; yields their ids. |
| `savedDecks` / `clearDecks` / `saveDeck` / `saveTrio` | `GET /api/decks`; delete every deck and trio (yields the catalog version); one `PUT /api/decks/:id` or `PUT /api/trios/:id` under a client-minted id (R256), yielding it. |
| `freeAccount(account)` / `concedeAs(account, matchId)` | The E2E server keeps its state for its whole life, so a networked spec starts by taking both accounts out of any queue ticket, live match or unfinished series (a concede, or a forfeit between games, R261). `concedeAs` connects, concedes and closes in one `wsPlayer` task, because the account's own browser would reclaim the seat between two. |
| `dragCardToDeck(cardId)` | A11's whole gesture onto the open deck's `deck-drop` — `dragstart`, `dragover`, `drop`, `dragend`, one `DataTransfer` built in the app's window. |
| `handIds(player)` / `unitIds(player)` | Which instance ids might I click. Setup only: assertions belong on the DOM, which is `viewFor` (CLAUDE.md rule 7). |
| `advanceToTurn(turn)` | End turns until `turn`, R82-safe: a turn that ended by itself leaves nothing to press, only a device to hand over. |
| `{ expectAnimating, during }` on `playCard` / `attack` / `endTurn` / `switchPosition` / `usePower` | Every acting command drains `data-animating` before it returns, which closes the window BUILD M5-T4 asks three specs to look through. `expectAnimating` asserts the animation between the click and the drain, and `during` runs any other retried assertions there (spec 25's notices, a number pop), so a spec never has to hand-roll clicks to get in between. |
| `playCard(id, { visiblePart: true })` | Clicks the part of the hand card that shows instead of its centre: at 390x844 a full hand fans out and overlaps, and the centre is another card's. The point is found as a tap is hit-tested (`elementFromPoint`). |
| `seedGame({ onBeforeLoad })` | More storage for the page before it boots, after the deck injection: spec 25 installs its notice recorder there and, for its screenshots, slows the effects down (`FX_SETTINGS_KEY` in `support/config.ts`). |
| `answerPrompt(kind, { first, embiggen })` | `first: 1` takes the first option offered, which is the only way to answer a Discover (its options are rng-drawn). `embiggen: true` is R81's price as the boolean it is, not the picker's `"true"` key. |
| `keepMulligans()` | Both opening mulligans are open at once (R265). On `/dev/hotseat` the device follows the seat that still owes one, so this answers both; networked it answers this client's own, after which the picker gives way to `mulligan-waiting`. |
| `concede()` | The `concede` control only asks ("Concede this game?"); this confirms in the dialog it opens. A spec that clicks `concede` alone has conceded nothing. |
| `wsPlayer` `awaitView` `where: { mulliganOpponentReady, drawOfferBy }` | R266's "the other seat is ready" and R269's standing offer, read off seat 2's own view (`view.mulligan.opponentReady`, `view.drawOffer.by`). |

## Which spec needs which milestone

Moved to docs/history/e2e-readme.md.

| Spec | Needs |
| --- | --- |
| 23 | SPEC §9.10 on `/practice`, against `build:e2e` with no server: the lesson path and its progress in `localStorage["jackioh.tutorial.v1"]` (R294), no Skip step (R314) and Exit, a lesson's seed, decks and handicap (R290, R291) compared with `apps/web/src/tutorial/lessons.ts` through `cy.task("tutorialLessons")`, and a 390x844 smoke. Rebuild the client after a lesson changes, or the task and the bundle disagree. |

Spec 14 (`14-landing-and-sign-in.cy.ts`) never submits to the auth provider, because a `build:e2e`
bundle has no `VITE_SUPABASE_URL`. So it runs against a static preview with nothing else started:

```
pnpm --dir apps/web build:e2e
pnpm --dir apps/web exec vite preview --port 5173 --strictPort
E2E_BASE_URL=http://localhost:5173 pnpm --dir e2e exec cypress run --spec cypress/e2e/14-landing-and-sign-in.cy.ts
```
