# `e2e/` — the thirty-five specs: BUILD M8's seventeen, `18`–`28`, patch v0.2.0's `29`–`32`, the Card Almanac's `33`, the public Statistics page's `34` and the settings dialog's `35`, plus twelve component specs

Cypress runs against `apps/web` in `E2E=1` mode: the `/dev/hotseat` route for the local specs and
a test server with fixture accounts for the networked ones. BUILD M8's house rules hold
everywhere in here:

- every spec sets a seed;
- there is no fixed `cy.wait(ms)` — `support/e2e.ts` throws if a spec calls one. Waits are
  assertions: `data-animating` clearing (`cy.settled()`), an animation appearing
  (`cy.expectAnimating`), a prompt (`cy.waitForPrompt`), or a testid;
- each spec asserts what BUILD's "Key assertions" column says for its row, quoted verbatim in the
  spec's header comment.

## Layout

```
e2e/
  cypress.config.ts        e2e: specPattern cypress/e2e, fixturesFolder fixtures, supportFile support/e2e.ts;
                           component: specPattern cypress/component, support/component.tsx, support/component-index.html
  cypress/e2e/*.cy.ts      the thirty-five specs (`99-online-smoke` is skipped unless enabled); CI splits them
                           over its e2e shards with scripts/shard-specs.mjs
  cypress/e2e/01-hotseat-full-game.cy.ts  BUILD M8: a seeded game played to completion through the UI with two aggro decks
  cypress/e2e/02-prompts.cy.ts  BUILD M8: each of the ten choice pickers rendered once and answered, whether it builds the `play` action or answers a `PendingChoice` (R81)
  cypress/e2e/03-trap-opponent-turn.cy.ts  BUILD M8: p2's set Sheepish fires on p1's turn when p1 plays a unit
  cypress/e2e/04-combat.cy.ts  BUILD M8: Taunt, Defense Position, First Strike and Divine Shield in combat
  cypress/e2e/05-reconnect.cy.ts  BUILD M8, networked: the same view and the same open prompt after a reload, the clock kept running
  cypress/e2e/06-room-code.cy.ts  BUILD M8, networked: a room created in the browser and joined by `wsPlayer`
  cypress/e2e/07-my-pawn-ai.cy.ts  BUILD M8: p2's My Pawn cancels p1's lethal attack, and an AI plays out p1's turn (R44)
  cypress/e2e/08-turn-cap-draw.cy.ts  BUILD M8: after the 60th player-turn, 30 each, the overlay says Draw (R2, R389)
  cypress/e2e/09-deckbuilder.cy.ts  BUILD M8, networked: the deck workshop and the queue's rules (§9.4, R250–R253)
  cypress/e2e/10-invite-gate.cy.ts  BUILD M8, networked: a pending account sees the code screen, three bad codes get one identical error, and a good code activates it
  cypress/e2e/11-radiant.cy.ts  BUILD M8: Glowy Jelly Bean on a hand card and Knockoff Temu on a field unit make them Radiant
  cypress/e2e/12-rotation-and-swaps.cy.ts  BUILD M8: Silly Silas moves every card one lane round the ring (R14), and Pocket Chaos's board swap flips the sides
  cypress/e2e/13-practice-vs-ai.cy.ts  SPEC §9.9, R187: practice against the AI on `build:e2e` with no server: the AI's mulligan and turns, a few human turns, the browser's game folding to the same hash in Node, and Concede
  cypress/e2e/14-landing-and-sign-in.cy.ts  polish 5: the landing page and the way in, every `/api` call answered by `cy.intercept`
  cypress/e2e/15-audio.cy.ts  polish 2 (SPEC §10.11): the first click unlocks audio, a unit played from hand logs its play line, mute survives a reload
  cypress/e2e/16-drag-to-play.cy.ts  polish 7, B40: drag to play and to attack in a seeded hotseat game, and drag to play turned off in the settings panel
  cypress/e2e/17-card-showcase-and-hovers.cy.ts  the opponent's played card held up for about a second (a back for a face-down set), a log line's card on hover and click, and a graveyard browsed on hover and in a dialog, on /dev/hotseat and /practice
  cypress/e2e/18-deck-workshop.cy.ts  TASK 1 (R250–R252, R255, R256): an incomplete deck saves and survives a reload, an edit made while `PUT /api/decks/:id` fails at the network is kept on the device and saved once it answers, a copied deck code imports as a new deck and a damaged one is refused with a sentence, a trio marks the cards two decks share and is ready once they share none, and the deck cap
  cypress/e2e/19-queue-modes-and-series.cy.ts  TASK 1 (R257–R264, R330–R338): Best of 1 queues the chosen deck, All Random needs no saved deck, a Conquest series between the browser and `wsPlayer` (sealed picks chosen and locked in, the picked decks, a won deck locked and a lost one picked again, the last deck picked for you, a concede loses a game, three wins with three decks end it and rate it once), and a Conquest room refusing a Best-of-1 joiner
  cypress/e2e/20-mulligan-concede-draw.cy.ts  networked, like 06: both seats mulligan at once in either order (R265–R268), Concede's confirmation, and a draw offer declined and then accepted (R36, R269), asserted on the browser's DOM and on seat 2's socket alike
  cypress/e2e/21-radiant-marks.cy.ts  the Radiant pass (SPEC §10.10, R277, R279, R280): a hand card's computed value "{n}", a reference's face beside the preview and its tooltip in the touch sheet, and a card made Radiant in hand printing its change in gold, on /dev/hotseat
  cypress/e2e/22-tutorial-lesson-one.cy.ts  SPEC §9.10: lesson 1 played to a win by doing, through the UI, what the coach asks; progress saved; the log replays with the tutorial handicap
  cypress/e2e/23-tutorial-path.cy.ts  SPEC §9.10: the lesson path (locked, open, completed), progress seeded, reloaded and corrupt, no Skip step and Exit (R314), a later lesson's fixed deal, the phone layout
  cypress/e2e/24-library-browse.cy.ts  R310–R314: on /dev/hotseat your own library (the "Deck" pile, "Your deck", R373) opens on hover, click and Enter, grouped with counts and "Order hidden", card for card what the library holds; the opponent's is a count; the other seat's opens after the hand-over; in a tutorial lesson there is no Skip step and the library opens there too
  cypress/e2e/25-overflow-animations.cy.ts  R315–R318 on /dev/hotseat: "Fatigue N" on the deck pile and the hit on the hero, "Hand full" over a hand with the burned card's face, and "Deck full" with the refused card's face and outcome, each on the seat whose action caused it and on the owner's own device, gone once the board catches up; the games are handicapped fixtures (R180), and `--expose shots=1` adds screenshots of each notice at 1280x720 and 390x844
  cypress/e2e/26-trio-codes.cy.ts  R339–R341, networked: Copy trio code on `e2e-p2`'s trio, Import trio into `e2e-p1` as three new decks and a trio the server holds card for card, and with too little room the import says how many slots it needs, the server refuses the same import, and nothing is made
  cypress/e2e/27-account-tutorial-and-email-link.cy.ts  R320–R322, networked: a lesson won on the device reaches a signed-in account and a device with empty storage finds it on the path; Hide tutorial folds the path to one focused Show tutorial button, the choice follows the account to a fresh device, and Show brings the path back everywhere (the newest choice wins); both buttons are 44 px touch targets at 390x844; and R323, R324: an emailed link's PKCE code opened on another device lands on /login, scrubbed, and says the email is confirmed (the exchange itself is unit-tested: a build:e2e bundle has no auth provider)
  cypress/e2e/28-patch-011-ui.cy.ts  v0.1.1 (issue #27, R370–R374) on /dev/hotseat and the landing page: your own face-down trap drawn with the Face down tag and its note (R371), the other seat's back labelled "Face-down trap" with its overlay and, where the view gives one, its "(N) Cost" (R370, worded by R432 since patch v0.2.0), the pile reading "Deck" (R373), Combo-Index's grade badge and face by letter, E then D (R372, fixture `28-combo-a`), and the landing fan's four real faces, a new hand on a new visit (R374); `--expose shots=1` adds 1280x720 and 390x844 shots
  cypress/e2e/29-animated-trap.cy.ts  patch v0.2.0, R383, on /dev/hotseat: a face-down Animated Field Trap (C #5 Tesla) fires on the opponent's turn when a Unit arrives on that side, hits it, then steps out of its backrow zone into its controller's unit row as a Unit in Defense Position, face-up on both seats' views, with `animated` on the board between the two
  cypress/e2e/30-activate.cy.ts  patch v0.2.0, R384, on /dev/hotseat: a card's Activate control is live exactly when `legal` lists that card's `activate`, one use plays `activated` and its effect, and a once-per-turn ability's control is greyed for the rest of the turn and live again on its controller's next turn
  cypress/e2e/31-counter-opponent-turn.cy.ts  patch v0.2.0, §10.5's announce window, on /dev/hotseat: a face-down counter (C #17 Counterspell) answers the opponent's Spell on the opponent's turn — `cardAnnounced`, the trap fires, `countered` — and the Spell goes to its owner's graveyard without resolving (no `cardPlayed`, no effect), its mana still spent, on both seats' views
  cypress/e2e/32-tribute-full-board.cy.ts  patch v0.2.0, R391, on /dev/hotseat: with all five of a seat's unit zones full, a card with a Tribute cost is playable because its own Tribute empties a zone, and it lands in the zone that Tribute emptied
  cypress/e2e/33-almanac.cy.ts  R630 on `/` and `/almanac`, against `build:e2e` with no server: signed out, the footer's "Card almanac" link (right after Patch notes) opens the almanac, read-only (no "Owned only", no "+", no draggable card), a cost chip keeps only cards of that cost, a card's detail view opens with no add action and closes, no `/api` call except the public stats block (R654) is made on the page, and Back returns to the landing page
  cypress/e2e/34-stats.cy.ts  R654 on `/` and `/stats`, against `build:e2e` with stubbed endpoints: signed out, the footer's "Stats" link opens `/stats`, the provisional banner displays live-game progress toward the gate, summary tiles, sortable cards table, search, card drill-down dialog, and players tab (with no Elo rating displayed); Back returns to the landing page
  cypress/e2e/35-settings-dialog.cy.ts  #128, #129, R633, R634 (#263): the dialog's four tabs by click and arrow key, the tab it reopens on, Reduce motion applied at once, Reset this tab and Reset all, the background-music switch kept across a reload, the Account tab signed out, and signed in as `e2e-p1` a change reaching the account (`PUT /api/settings`), a fresh device taking the account's copy (`GET`), and a failed save sent again by Try again
  cypress/component/audio-recipes.cy.tsx  polish 2: every SFX recipe rendered in Chrome's OfflineAudioContext is finite, audible and quiet after its length, impact grows with damage, and through the real mix each effect sits in its band against the shipped voice lines
  cypress/component/audio-toggle.cy.tsx   polish 2: inside .app-shell the mute toggle is a 44 px circle with a 22 px icon
  cypress/component/board-layout.cy.tsx  BUILD M5-T1's pixel acceptance: the fixture view with 10 units, 10 backrow cards and a stacked pile has no layout overflow at 1280x720 and 390x844
  cypress/component/card-faces.cy.tsx  polish 6, B15 and B21: every face's name and rules text fit their boxes in a real layout engine, a clamp only past 260 characters and never under the reading floor
  cypress/component/deckbuilder-layout.cy.tsx  B39/B29/B38 on the deck workshop (`DeckWorkshop`, a full deck open): no overflow at 390x844 and 1280x720, two pool columns on the phone, two whole pool rows at 1280x720, the first pool row on a phone's first screen, the verdict in the sidebar, every pool keyword bold in its rules box's ink (#85), and an off-screen pool card fitted only once it nears the screen (#263)
  cypress/component/emotes-layout.cy.tsx  #219 on the fixture board: the picker's emoji drawn at least 36 px square, every voice label at least 11 px, every item a 44 px touch target on a touch-sized screen, both menus and a voice line's bubble kept on the screen at six viewports
  cypress/component/fx-layer.cy.tsx  polish 1: the effects layer's real device pixel ratio (B28), the board shake (B35), and B40's browser half: it takes no click, the canvas draws, the board still fits
  cypress/component/keyword-visuals.cy.tsx  R438 (#40): the keyword visuals on a board minion as Chrome paints them
  cypress/component/landing-and-code-field.cy.tsx  polish 5, B39: at 360, 390, 768 and 1280 px neither the landing page nor the invite code field overflows, and every CTA and the code input are on screen
  cypress/component/mobile-ux.cy.tsx  polish 7: the glow colours from the computed box-shadow and the mobile layout only a real layout engine can measure
  cypress/component/practice-table.cy.tsx  the practice table at the viewports practice is played on, on the M5-T1 fixture board with a hand of 4, 7 and 10 cards
  cypress/component/radiant-marks.cy.tsx  the Radiant pass: the gold mark's weight, underline and contrast on both backgrounds, a reference's tooltip in the detail view, and a computed value inside its rules box
  fixtures/decks/*.json    scenario decks, named for the spec that uses them; a deck may carry the seat's
                           `handicap` (R180: deckSize, manaBonus, manaCap, extraOpeningCards,
                           extraDrawsPerTurn, heroHealth?), and then holds its deckSize cards (R184)
  support/
    commands.ts            seedGame, playCard, attack, answerPrompt, endTurn (+ the waiting
                           helpers, signIn/visitAs, installLoadout and the saved-deck helpers,
                           freeAccount/concedeAs, dragCardToDeck)
    testids.ts             every selector the suite uses, in one file
    ux.ts                  polish 7: the pointer-drag gesture (spec 16) and the drag, glow and
                           settings selectors (specs 16 and 35)
    tutorial.ts            specs 22 and 23: the lesson URL, seeded progress, the practice and
                           tutorial dev handles, and the driver that follows the coach by clicking
    config.ts              routes, endpoints, fixture accounts, the session key, timeouts,
                           SPEC constants
    cards.ts               SPEC §8 index -> name -> catalog id for the three sets (`core-NNN`,
                           `classic-NNN`, `classicplus-NNN`), and the 49 Tokens
    types.ts               structural subsets of the engine types; `window.__jackioh`
    e2e.ts                 support file; enforces the no-fixed-wait rule
    component.tsx          the component specs' support file (`cy.mount`); not e2e.ts, whose
                           commands need `window.__jackioh`
    component-index.html   the page the component specs mount into
    tasks/
      index.ts             registers the node tasks
      wsPlayer.ts          `cy.task("wsPlayer")`: the second player, driven from Node (specs 05, 06, 19), and
                           the one-task connect-and-concede `cy.concedeAs` uses
      replay.ts            `cy.task("replayHash")`: fold the recorded log (and its handicaps: spec 13's, spec 25's) outside the browser
      replay-runner.ts     runs under the repo's tsx; imports packages/* to fold the log
      lessons.ts           `cy.task("tutorialLessons")`: the lessons, seeds and decks as apps/web states them
      lessons-runner.ts    runs under the repo's tsx; reads apps/web/src/tutorial/lessons.ts, AI_TUTORIAL and
                           the catalog's Quickdraw tag (excluded from tsconfig.json, like replay-runner.ts)
      onlineReset.ts       spec 99's cleanup, and only its: leaves both accounts outside any live match
  scripts/check-fixtures.mjs  pre-flight for the deck fixtures; needs no browser and no client
  scripts/shard-specs.mjs     `node scripts/shard-specs.mjs <k> <K>`: the specs CI's shard k of K runs,
                              balanced by measured time; a new spec lands in exactly one shard
  artifacts/               recorded logs, screenshots, videos (git-ignored)
```

`e2e/` is its own pnpm root (`e2e/pnpm-workspace.yaml`): the root workspace globs only
`packages/*` and `apps/*`, and nothing outside this directory should have to carry Cypress.
`e2e/tsconfig.json` type-checks without `packages/*` or `apps/*` being buildable, on purpose —
M8 is written before the client and server are finished.

## Install

```
pnpm install            # repo root: provides tsx, which the replayHash task uses
cd e2e && pnpm install  # Cypress + ws; the postinstall fetches the Cypress binary
pnpm exec cypress verify
pnpm exec tsc -p tsconfig.json
pnpm check:fixtures     # every deck fixture obeys L2/L3/L6 before a browser is involved
```

## Run

```
# 1. the client, in E2E mode
E2E=1 pnpm --dir apps/web dev                  # must serve http://localhost:5173

# 2. the server, in E2E mode, for specs 05, 06, 07(networked path), 09, 10, 18, 19, 20, 26, 27, 35
E2E=1 pnpm --dir apps/server dev               # http://localhost:8787 and ws://…/ws/match (WS_PATH)

# 3. the suite
cd e2e
pnpm test:e2e                                  # headless, default browser
pnpm test:e2e:chrome                           # M8 gate: Chrome
pnpm test:e2e:electron                         # M8 gate: Electron
pnpm open                                      # interactive
pnpm exec cypress run --spec cypress/e2e/01-hotseat-full-game.cy.ts
```

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
   `apps/server/src/match/wsServer.ts`. A handshake off that path is never upgraded, so this is
   not a preference: `support/config.ts`, `cypress.config.ts` and the `wsPlayer` task all default
   to it.

## Assumptions beyond that contract

Every one of these is either an ask on another team or a convention this suite invented because
BUILD does not fix it. They are all made in `support/`, never in a spec, so each has exactly one
place to change.

| # | Assumption | Where | Ask |
| --- | --- | --- | --- |
| A1 | In E2E mode the hotseat route resolves `a=`/`b=` from `window.__jackiohE2E = { seed, decks, handicaps? }` (also mirrored into `localStorage["jackioh.e2e.decks"]`, so a reload keeps it) before falling back to built-in dev decks. `seedGame` injects it in `onBeforeLoad`, with fixture A's `handicap` as `handicaps.p1` and fixture B's as `handicaps.p2` (R180); the route drops a seat's handicap that is not shaped like one, hands the rest to `createGame` (which validates the numbers, R184, and whose refusal it prints), and reports them on `window.__jackioh.handicaps` for `cy.replayCheck`. Development builds only, like the rest of the injection. | `support/commands.ts`, `support/types.ts` | `apps/web` |
| A2 | `window.__jackioh` also carries `log: Action[]` (every action reduced, in order, with its nonce) and `decks: [string[], string[]]`. Only spec 01 needs them: they are the "recorded actions" BUILD M8 folds for the replay hash. | `support/types.ts`, `cy.replayCheck` | `apps/web` |
| A3 | `<side>` in `zone-<side>-…` and `hero-<side>` is view-relative, `you` \| `opponent`, matching `PlayerView.you` / `.opponent`. Lanes are 1..5, as `packages/engine/src/zones.ts` numbers them (not an assumption — the engine fixes it). | `support/testids.ts` | `apps/web` |
| A4 | Prompt internals: one option per `PendingOption.key` at `prompt-option-<key>`, a `prompt-submit` confirm button for multi-select kinds, and `prompt-x` for the numeric input of `x`/`embiggen`. A chosen option is marked `aria-pressed="true"` (or `data-selected="true"`), which is how `keepMulligans` knows it does not have to click it again. `answerPrompt` prefers these and falls back to the board testids for pickers M5-T2 renders on the board (target, zone, tribute). | `support/testids.ts`, `support/commands.ts` | `apps/web` |
| A5 | Chrome not in BUILD's list: `result-overlay`, `turn-banner`, `seat-switch`, `graveyard-count-<side>`, `exile-count-<side>`, `library-count-<side>`, `hand-count-<side>`, `mana-<side>` + `.mana-crystal`, `switch-<instanceId>`, and `data-legal="true|false"` for the highlighting M5-T2 describes. (`.damage-pop`, `.heal-pop`, `.loss-pop`, `.radiant`, `data-locked` are BUILD M5-T4's own.) Plus R169's badges inside `modifiers-<side>`: `.modifier-badge` carrying `data-modifier-id`, and `data-count` on the list itself — client vocabulary rather than BUILD's, and the list is rendered on both seats even when empty. Plus the shown stats as attributes — `data-attack`, `data-health`, `data-max-health`, `data-armor`, `data-keyword`, `data-position`, `data-radiant` — because M5-T4's `buffed` row is "shown stats equal the view" and a reformat of `{health}/{maxHealth}` must not break a spec; and the regions the animation table targets — `hand-<side>`, `library-<side>`, `graveyard-<side>`, `exile-<side>`, `modifiers-<side>`, `backrow-<side>`, `board`, `game`, `concede`, `log`, `action-error`, `draw-toast`, `prompt-modal`, `prompt-scrim`. Every one of those is a name `apps/web` already renders (`animTestid`, `contract.ts`), so this row documents them rather than asking for them. | `support/testids.ts` | `apps/web` |
| A6 | Routes and fixtures for the non-hotseat specs: `/login`, `/invite`, `/decks`, `/play`, `/match/<id>`; `http://localhost:8787` + `ws://localhost:8787/ws/match` (the path is `WS_PATH`, not an assumption); fixture accounts `e2e-p1`, `e2e-p2` (active, own every card) and `e2e-pending` (pending, verified email); seeded invite codes — one good, one missing, one expired, one exhausted. | `support/config.ts` | `apps/web`, `apps/server` |
| A7 | Catalog ids are the set (`core`, `classic`, `classicplus`) + `-` + SPEC §8 index zero-padded to three digits; a Token takes its parent's index with a `.k` suffix (`core-065-1`, `classicplus-012-1`) or a name (`core-t-sheep`, `classicplus-t-ai-01`). An index alone repeats across sets, so a fixture names cards by id. Confirmed against `packages/cards/catalog.json`, so this is documentation rather than an assumption. `CARD_NAMES` stays the 268 deckable cards of the three sets and `TOKEN_NAMES` the 49 Tokens, because `asDeck` checks a fixture against `CARD_NAMES` being exactly the deckable set (L3). | `support/cards.ts` | — |
| A8 | The WS protocol `cy.task("wsPlayer")` speaks: `-> hello {token?,matchId?,roomCode?}`, `-> action {action:{…,nonce}}`; `<- view {view}`, `<- ack {nonce,seq}`, `<- error {code,message,nonce?}`, `<- prompt {forYou,…}`, `<- clock {now,clocks}`. Read off `apps/server/src/match/protocol.ts`, which fixes every shape, so this is documentation now rather than an assumption. Joining a room is **not** on the socket: `POST /api/rooms/:code/join` is, and the `joinRoom` frame exists only to answer a client that tries with `error {code:"unsupported"}`. | `support/tasks/wsPlayer.ts` | — |
| A9 | Fixture deck shape: `{ id, spec, description, cards, handicap? }` with `cards` holding exactly `DECK_SIZE` (20) distinct non-Token catalog ids, of any set (one format, R380) — or exactly `handicap.deckSize` when the fixture carries a handicap (R184) — and `id` equal to the filename. A `handicap` is five non-negative integers with 1 <= `deckSize` <= `LIBRARY_CAP` (60) and an optional positive `heroHealth`. `seedGame` and `pnpm check:fixtures` check all of it before visiting, so a bad fixture fails with a readable message instead of an engine throw. | `support/commands.ts`, `scripts/check-fixtures.mjs` | — |
| A10 | A session is `localStorage["jackioh.e2e.session"] = {accessToken}`, installed in `cy.visit`'s `onBeforeLoad` so the first boot already has it. `cy.signIn(account)` remembers an account and every later visit carries it; `cy.visitAs(account, path)` is the two together. Confirmed against `apps/web/src/net/session.ts`, which reads that key. | `support/config.ts`, `support/commands.ts` | — |
| A11 | Deck workshop testids, which BUILD names none of: `workshop` (`data-view`), `sync-status` (`data-state="saved\|saving\|offline\|error"`), `deck-list` / `deck-row-<deckId>` (`data-count`, `data-status`, `data-unsynced`), `deck-new`, `deck-cap` / `deck-cap-reason`, the trio rail's `trio-list` / `trio-row-<trioId>` (`data-ready`) / `trio-new` / `trio-cap`, the deck editor's `deck-editor` (`data-deck`), `deck-name-input`, `deck-count`, `deck-drop`, `deck-card-<cardId>` (`data-conflict`, `data-conflict-with`), `deck-status`, `deck-save-error`, `deck-copy-code` / `deck-code-output`, `deck-compare-select` (`trio:<id>`, `deck:<id>`, `none`), `deck-verdict` and `trio-verdict` (`data-ready`) around `loadout-errors` / `loadout-error-<rule>` (`data-rule`, `data-source`, `data-deck`, `data-card`), the trio editor's `trio-slot-<n>`, `trio-open-<n>`, `trio-card-<slot>-<cardId>`, and the import panel's `deck-import-*`; the pool's `card-pool-<cardId>` (`data-in-deck`, `data-unavailable`, `data-held-by`) and `db-add-<cardId>`; a drag carrying the catalog id on `application/x-jackioh-card` plus `text/plain`. The workshop replaced the three-deck loadout editor, so the per-deck `deck-tab-<n>` family and `loadout-save` are gone. These mirror `apps/web/src/game/deckbuilder/testids.ts` name for name; keep the two files identical. | `support/testids.ts`, `support/commands.ts` | — |
| A12 | A one-deck scenario fixture is padded into a trio R253's Best of 3 accepts: L1 wants exactly 3 decks and L4 wants them disjoint, so the other two are the next `DECK_SIZE * 2` Core ids the fixture did not use — inside L5, because R111 grants one copy of every non-token card. `cy.installLoadout` deletes every deck and trio the account has (`GET /api/decks`, then `DELETE` each), saves the three as "Deck 1".."Deck 3" with one `PUT /api/decks/:id` each — the fixture at `deckIndex` — under client-minted ids in ascending order, so `GET /api/decks`'s oldest-first order (ties on id) is the save order even inside one millisecond, and a legacy `{ deckIndex: n }` body (R257) names deck `n`; then one `PUT /api/trios/:id` ("E2E trio") of the three in slot order. It yields `{ deckIds, trioId, decks, catalogVersion }`. | `support/commands.ts` | — |
| A13 | Invite-screen testids, which BUILD names none of either: `invite-code-input`, `invite-submit`, `invite-error`, `invite-paused`, `invite-not-needed`. Like A11 these are documentation rather than an ask — `apps/web/src/routes/invite.tsx` exports and renders all five already; nothing under `e2e/` had ever named them, which is why spec 10's "code screen shown" was a URL redirect and not a screen. Keep the two files identical. | `support/testids.ts` | — |
| A16 | Lobby, series-screen and series-banner testids: `play-mode-<bo1\|bo3\|random>`, `play-deck-select`, `play-trio-select`, `play-choice-verdict`, `play-queue`, `play-status`, `play-create-room`, `play-room-code`, `play-room-mode`, …; `series-screen`, `series-score`, `series-pick-<slot>`, `series-deck-<slot>`, `series-opponent-deck-<slot>`, `series-opponent-status`, `series-game-<n>`, `series-result`; `series-banner`, `series-banner-continue`, `series-banner-result`. Like A11 and A13, documentation rather than an ask: they mirror `playTestid` (`routes/play.tsx`), `seriesTestid` (`routes/series.tsx`) and `seriesBannerTestid` (`routes/SeriesBanner.tsx`). Keep the files identical. | `support/testids.ts` | — |
| A18 | The overflow notices (R318): `pile-notice-<side>` inside `library-<side>` (`data-kind="fatigue"` reading "Fatigue N", or `"libraryFull"` reading "Deck full" (R373) around `overflow-card-<side>` with `data-face="face\|back"` and `data-outcome="notCreated\|graveyard\|ceased"`), and `burn-notice-<side>` inside `hand-<side>` ("Hand full" around `burn-card-<side>`, `data-face`); each carries `data-playing="true"` while its entry runs and stays until the board shows the next view. Documentation, like A11: `noticeTestid` in `apps/web/src/game/OverflowNotices.tsx` renders them. | `support/testids.ts` | — |
| A17 | The workshop's device mirror (R256): `localStorage["jackioh.decks.v1.<profileId>"] = { v: 1, decks: [{ item, dirty }], trios, deletedDecks, deletedTrios }` (`mirrorKey` in `apps/web/src/game/deckbuilder/sync.ts`). Spec 09 writes drafts a save would refuse into it, because a restored draft is the only way a browser meets L3 and L6 (D3/D4 refuse them at save, R250). | `support/config.ts` (`deckMirrorKey`) | — |
| A19 | The v0.1.1 patch's board marks (R370–R372): a back in a backrow carries `data-face-down="true"`, the label "Face-down trap[, (N) Cost]" (R432's wording since patch v0.2.0) and, where the view gives it a cost, `data-facedown-cost` and a `.facedown-cost` gem; its hover or long-press opens `inspect-face-down` (`data-mode="hover\|sheet"`, with `inspect-face-down-cost`). Your own face-down trap's root carries `data-unrevealed="true"` and holds `unrevealed-<instanceId>`, the "Face down" tag; its preview and sheet hold `inspect-note`. A grade badge is `[data-counter="grade"]` with `data-grade` and `data-grade-letter`. Documentation, like A11: `testid.unrevealed` (`apps/web/src/game/contract.ts`) and `apps/web/src/cards/inspect/testids.ts` render them. | `support/testids.ts` | — |
| A20 | Patch v0.2.0's board, for specs 29–32 (`apps/web/README.md`, "Patch v0.2.0 at the table"): a card's Activate control is an element of its own, live exactly when `legal` lists that card's `activate` (Heroic Power's `power` is one such control); an animated card is a unit-zone card like any other, its `card-<instanceId>` inside `zone-<side>-units-<lane>`, and an "Animated on your turn" card's home backrow zone carries `data-reserved` while the card is away (R383); the new prompts carry `data-prompt-kind` `number`, `answer`, `cell`, `reward` or `pick`, with one `prompt-option-<key>` per option as A4 says; and `cardAnnounced`, `countered` and `animated` mark their elements with `data-animating` like every event (M5-T4). The Activate control's testid is the client's to name; `support/testids.ts` mirrors it name for name, as A11 does. | `support/testids.ts` | `apps/web` |
| A21 | The Card Almanac's testids (R630): `almanac` on the page's root, `site-footer`, `site-footer-almanac` and `site-footer-patch-notes` in the footer, and `nav-back` for a screen's Back. The page renders the deck builder's browse pane, so its pool, chips and detail view carry A14's names. Documentation, like A11: `almanacTestid` (`apps/web/src/routes/almanac.tsx`), `siteFooterTestid` (`routes/SiteFooter.tsx`) and `navTestid` (`routes/nav.tsx`) render them. | `support/testids.ts` | — |
| A22 | The public Statistics screen's testids (R654): `stats-screen`, `stats-tab-cards`, `stats-tab-players`, `stats-summary-tiles`, `stats-summary-total-games`, `stats-summary-live-games`, `stats-summary-source`, `stats-summary-best-card`, `stats-summary-worst-card`, `stats-provisional-banner`, `stats-fallback-toggle`, `stats-search-input`, `stats-cards-table`, `stats-players-table`, `stats-drill-down-modal`, `stats-drill-down-close`, and the footer's `site-footer-stats`. Documentation, like A11: `statsTestid` (`apps/web/src/stats/testids.ts`) and `siteFooterTestid` (`routes/SiteFooter.tsx`) render them. | `support/testids.ts` | — |

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

| Spec | Needs |
| --- | --- |
| 01, 02, 03, 04, 11, 12 | M4 (the real catalog and card scripts) + M5 (board, prompts, hotseat loop, animations). 01 additionally needs `window.__jackioh.log`/`.decks` (A2) and the repo's `tsx` for the replay fold. |
| 05, 06 | M6 (server, match actor, WS protocol) and, for the clock assertion in 05, M7-T1. |
| 07 | M4 (card #96) + M5. The AI turn runs inside `reduce`, so no server is needed. |
| 08 | M4 + M5 only: it ends turns until the cap. Since patch v0.2.0 the cap is 60 player-turns, 30 each (R2, R389), and two do-nothing 20-card decks fatigue out before it, at player-turn 48 (SPEC §2.5), so 08's decks never fatigue: both seats play #75 Infinite Reserves, whose draws from an empty deck give a Rush Token card instead of fatigue (a game seeded near the cap would serve as well). It asserts R2's arithmetic at 30 turns each, the Draw on the overlay after the 60th player-turn, and both heroes still at `HERO_HEALTH`, which is what makes it the cap's draw and not two heroes dying together. |
| 09 | M6-T3 (validator, deck and queue endpoints) and TASK 1's deck workshop. L1, L2, L4 and L5 are shown in the workshop's verdict and in `POST /api/queue`'s 422; L3 and L6 cannot reach the queue (D3/D4 refuse them at save, R250), so they are shown on a draft restored from the device mirror (A17), with the save's refusal and the queue's "no longer saved". |
| 10 | M6-T1 (auth, invite gate) and the code screen. |
| 13 | Polish 3 (SPEC §9.9): `/practice`, the practice worker and `packages/ai`, against `build:e2e` with no server. The replay check passes the game's handicaps to the fold (R180, R187). |
| 14 | A built client only (`pnpm build:e2e`, then `vite preview`): no server and no auth provider. Every `${apiUrl}/api/*` call is a `cy.intercept` stub, sessions are seeded under `jackioh.e2e.session` in `onBeforeLoad`, and emailed links are visited as `/login#…`. It covers the landing page, the segmented code field, rate-limit feedback, emailed-link handling, the reset screen and the gate's exits (`docs/polish/5-sign-in.md`). |
| 15 | Polish 2 (SPEC §10.11): the audio layer on `/dev/hotseat`, M4 + M5, no server. Chrome for the audio context; it asserts the voice request in `window.__jackiohAudio`'s log, never the sound. |
| 16 | Polish 7 (§10.8, R195): drag to play on `/dev/hotseat` with spec 04's decks and seed, M4 + M5, no server. The gestures are real pointer events from `support/ux.ts`, and the settings panel turns drag to play off. |
| 17 | M4 + M5 and polish 3 (`/practice`), against `build:e2e` with no server: the opponent's-play showcase, the log's card lines and the pile browser (§10.8, §10.10, R97, R202, R227), with the selectors in `support/testids.ts` block A15. It uses spec 01's and spec 03's decks and seeds, and plays `/practice` at normal pacing, because `?pace=fast` releases the AI without waiting for the showcase. How long the showcase stood is read off a MutationObserver recorder in the page, never off a fixed wait. |
| 18 | TASK 1 (R250–R252, R255, R256): the `E2E=1` server and a `build:e2e` client. "Unreachable" is `cy.intercept` failing `PUT /api/decks/*` at the network; a second, pass-through intercept is "the server answers again", and the workshop's own retry (`DECK_AUTOSAVE_RETRY_SECONDS`) does the rest. |
| 19 | TASK 1 (R257–R264, R330–R338): the `E2E=1` server and a `build:e2e` client. The browser (`e2e-p1`) queues and picks through `/play` and `/series/<id>` (choose a deck, lock it in); `e2e-p2` queues, picks and joins over HTTP (every body seeded, R143) and plays over `wsPlayer`. The Conquest test plays three games, seat two conceding each. Its four tests keep the two accounts' ratings level (two rated wins and two losses each), so re-runs against one server keep pairing at once; the pairing waits still allow for §9.5's full window widening. |
| 20 | M6 + M7-T1 and a `build:e2e` client, like 05 and 06: a room-code match per case with seat 2 on `cy.task("wsPlayer")`, spec 06's decks, and one seed per case; the selectors are `support/testids.ts` block A16. The draw offer's sound is asserted in `window.__jackiohAudio`'s log, as spec 15 asserts its voice lines. |
| 22 | SPEC §9.10 (the tutorial) on `/practice`, against `build:e2e` with no server. It follows the coach through lesson 1 by clicking what `window.__jackiohTutorial.suggested` names (never dispatching), with reduced motion so every view is drawn as it arrives, and folds the log with the tutorial handicap (R290). It names no card or step, so the lesson's content may change under it. |
| 23 | SPEC §9.10 on `/practice`, against `build:e2e` with no server: the lesson path and its progress in `localStorage["jackioh.tutorial.v1"]` (R294), no Skip step (R314) and Exit, a lesson's seed, decks and handicap (R290, R291) compared with `apps/web/src/tutorial/lessons.ts` through `cy.task("tutorialLessons")`, and a 390x844 smoke. Rebuild the client after a lesson changes, or the task and the bundle disagree. |
| 24 | R310–R314 on `/dev/hotseat` and `/practice?lesson=basics`, against `build:e2e` with no server: your library's list (the engine's `SideView.ownLibrary`) read back card for card against `window.__jackioh.state`, the opponent's library closed, and no Skip step in a lesson. |
| 25 | R315–R318 on `/dev/hotseat`, against `build:e2e` with no server: the board's overflow notices (A18) in three handicapped games (fixture `handicap`, A1, A9). Each notice is asserted with a retried `should` inside the action's `during`, then gone after `cy.settled()`; a MutationObserver in the page records every state a notice was drawn in, which is how "Fatigue 2" is shown to come before "Fatigue 3". The screenshot pass (`--expose shots=1`) runs the three games again at 1280x720 and 390x844 at half effects speed. |
| 26 | R339–R341: the `E2E=1` server and a `build:e2e` client. The code is read off the trio editor's read-only field, not the clipboard, which a Cypress browser may refuse. `e2e-p1` is cleared before each test, so the import is the only thing that could have made what `GET /api/decks` lists. |
| 27 | SPEC §9.10's account copy (R320–R322): the `E2E=1` server and a `build:e2e` client, as `e2e-p1`. The server's store lives as long as the server and tutorial progress only grows, so every assertion is "contains", and each test first makes an explicit Show through `PUT /api/tutorial`, the newest choice until the page makes its own. The page's `GET` and `PUT /api/tutorial` are spied on with `cy.intercept` and awaited, never answered by the spec. The emailed-link block is filled in with the PKCE sign-in; until then that case is covered by the web client's unit tests. |
| 28 | v0.1.1 (issue #27, R370–R374) on `/dev/hotseat` and `/`, against `build:e2e` with no server: spec 03's decks and seed for the face-down trap, `28-combo-a` and seed `28-combo-16` for Combo-Index, and the landing page with `Math.random` replaced in `onBeforeLoad` by a seeded source, one per visit. A back's cost is asserted where the view gives one, since the engine's half of the patch carries it, in R432's words ("(N) Cost"); the screenshot pass turns the effects layer off (`FX_SETTINGS_KEY`) and waits for the showcase to pass before each shot. |
| 29, 30, 31, 32 | BUILD M9 (patch v0.2.0): the engine's new keywords and systems (Animated R383, Activate R384, Counter in §10.5's announce window, the Tribute zone R391), the Classic and Classic+ cards each spec deals, and the client's layers for them (A20), on `/dev/hotseat` against `build:e2e` with no server. Each deals its cards through fixtures (A9: non-token ids of any set) with its own seed. 29 and 31 each turn on a card that acts on the other seat's turn, so the device is handed over to the seat whose turn it is and the effect is asserted on both seats' views. |
| 33 | R630, the Card Almanac, against `build:e2e` with no server, like 28's landing case: every `/api/*` call is answered 401 by `cy.intercept` and recorded, so the spec can show the almanac made nothing except the public stats block (R654, `/api/stats/*`). No game is started; the seed names the scenario (BUILD M8). |
| 34 | R654, the public Statistics page (/stats), against `build:e2e` with stubbed `/api/stats/*` endpoints: signed out, the footer's "Stats" link opens /stats, provisional banner shows live-game progress below the gate, cards table filters and searches, card drill-down modal opens, players tab lists public player records without Elo, and Back returns to the landing page. |

Spec 14 (`14-landing-and-sign-in.cy.ts`) never submits to the auth provider, because a `build:e2e`
bundle has no `VITE_SUPABASE_URL`. So it runs against a static preview with nothing else started:

```
pnpm build:e2e
pnpm --dir apps/web exec vite preview --port 5173 --strictPort
E2E_BASE_URL=http://localhost:5173 pnpm --dir e2e exec cypress run --spec cypress/e2e/14-landing-and-sign-in.cy.ts
```

## What the root still needs (not changed from here)

`e2e/` deliberately touches nothing outside itself. Three edits belong to whoever owns the root:

1. `package.json` → `"test:e2e": "pnpm --dir e2e test:e2e"`, which is the command CLAUDE.md
   documents.
2. `package.json` → add `tsc -p e2e/tsconfig.json` to the `typecheck` script, so the specs are
   type-checked by `pnpm typecheck`.
3. CI → run the suite on Chrome and on Electron (the M8 gate), after starting `apps/web` and
   `apps/server` in `E2E=1` mode; a nightly job re-runs spec 01 over 20 seeds (BUILD §4).
