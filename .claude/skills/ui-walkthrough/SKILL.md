---
name: ui-walkthrough
description: >
  Look at the JackiOh web client in a headless browser: open a route, set up a game, click
  through it and screenshot it to see. Use only when the work is what a player sees: apps/web
  components, CSS and layout, card faces and art, animations and effects, the phone layout, or
  when asked to look at, screenshot or play the UI. Not for engine rules, card scripts, the AI,
  the server, the bot, CI or docs, nor to prove what a test already proves: booting the client
  takes minutes and every screenshot costs context.
allowed-tools: Bash(sh scripts/browse.sh:*)
---

# Walking through the UI

`sh scripts/browse.sh <command>` drives a headless Chromium through Microsoft's Playwright CLI, one
shell command at a time, and the browser stays open between commands. The script's header says what
it installs (once, into `.cache/`) and which browser it picks (#811).

If the task is not about what the client shows, stop here: nothing below is worth its minutes.

## 1. Serve the client

```
CYPRESS_INSTALL_BINARY=0 pnpm install   # once per checkout; the variable skips Cypress's 800 MB binary
pnpm --dir apps/web dev                  # http://localhost:5173; its predev builds the WASM engine first (about 2.5 minutes cold)
```

Run the dev server in the background and wait until `curl -s -o /dev/null http://localhost:5173/`
succeeds. An edit under `apps/web/src` hot-reloads the page, which starts a hotseat game over at
turn 0: set the board up again after an edit.

## 2. Pick a route

| Route | What it shows |
|---|---|
| `/dev/hotseat?seed=42&a=first20&b=cheap20` | A two-seat game run in the page; `a` and `b` are `DECK_IDS` (`apps/web/src/game/decks.ts`). Dev builds only |
| `/practice` | The tutorial lessons and a game against the AI, with no server |
| `/almanac` | Every card face and token, searchable: art, frames, rules text |
| `/`, `/patch-notes`, `/stats`, `/leaderboard` | The landing and the public pages |

The signed-in routes (`/decks`, `/play`, `/match/<id>`) need the E2E server and a session; `e2e/README.md`
covers both. Stay on the routes above unless the task is about those pages.

## 3. Drive it

```
sh scripts/browse.sh open "http://localhost:5173/dev/hotseat?seed=42"
sh scripts/browse.sh find "End turn"                   # the matching elements and their refs (e12)
sh scripts/browse.sh click e12                         # or a locator: click "getByTestId('end-turn')"
sh scripts/browse.sh hover e31                         # a hand card's ref: its hover preview
sh scripts/browse.sh eval "() => document.title" --raw
sh scripts/browse.sh resize 390 844
sh scripts/browse.sh screenshot --filename=board.png   # or `screenshot e12` for one element
sh scripts/browse.sh close
```

- **Refs.** `find "<text>"` answers with a few hundred bytes; a bare `snapshot` prints the whole
  page (about 2,000 tokens on a board), and `snapshot --filename=page.yml` writes it to a file
  instead. A ref is good until the page changes: find it again after an action.
- **Files.** The CLI runs in `.cache/browse/` (gitignored), so a relative `--filename` lands there and
  its own snapshots and logs in `.cache/browse/out/`. Read `.cache/browse/board.png` to look at it.
  Never copy a screenshot into the repository.
- **Test ids** (`e2e/README.md`, What must be true of the app first): `zone-<side>-<row>-<lane>`,
  `card-<instanceId>`, `hero-<side>`, `hand-card-<instanceId>`, `end-turn`, `offer-draw`, `power`;
  an open prompt carries `data-prompt-kind`.
- **Two walkthroughs on one machine** each take a session: `sh scripts/browse.sh -s=<name> open …`,
  and the same `-s=<name>` on every later command.

## 4. Set up a board

On `/dev/hotseat`, `window.__jackioh` is the dev handle: `legal()` lists the legal actions of the
seat holding the device, `dispatch({ ...action, playerId })` plays one, `setSeat(seat)` hands the
device over, `view()` is that seat's `PlayerView`. This keeps both opening hands and plays the first
legal card:

```
sh scripts/browse.sh eval "async () => {
  for (let i = 0; i < 100 && !window.__jackioh; i++) await new Promise((r) => setTimeout(r, 100));
  const j = window.__jackioh;
  for (const seat of ['p1', 'p2']) {
    j.setSeat(seat);
    const keep = j.legal().filter((a) => a.type === 'mulligan').sort((a, b) => b.keep.length - a.keep.length)[0];
    if (keep) j.dispatch({ ...keep, playerId: seat });
  }
  j.setSeat('p1');
  const play = j.legal().find((a) => a.type === 'play');
  if (play) j.dispatch({ ...play, playerId: 'p1' });
  return JSON.stringify(play);
}" --raw
```

The handle's `state` is the whole game, hidden cards included: use it to reach a board, never to
judge one. What the player sees is the page, which renders `viewFor` (CLAUDE.md rule 7).

## 5. Wait, then look

- **Animations.** An event's animation marks its element `data-animating` until it ends. Wait for
  none to be left before a still:

  ```
  sh scripts/browse.sh eval "async () => { await new Promise((r) => setTimeout(r, 50)); while (document.querySelector('[data-animating]')) await new Promise((r) => setTimeout(r, 50)); }"
  ```

  `sh scripts/browse.sh set-reduced-motion reduce` cuts the animations short as a player's reduced
  motion does (BUILD M5-T4): a card's play settles in under a fifth of a second instead of over one.
  To see an animation mid-flight, leave it on and screenshot straight after the action.
- **Viewports.** It opens at 1280x720. BUILD M5-T1 also holds the board at 390x844, and the e2e
  specs shoot 768x1024 too: `resize <w> <h>`. Resizing changes the page's size only; it does not
  emulate a phone's touch or pixel density.
- **Cost.** A 1280x720 screenshot is about 1,200 tokens. Shoot what the task needs: one element
  (`screenshot <ref>`) for a card face, and `find` instead of a picture when the question is what is
  on the page rather than how it looks.
- **What it cannot show.** No sound (the browser is muted), no GPU (canvas and WebGL render in
  software), and stills only.

## 6. Clean up

```
sh scripts/browse.sh close
lsof -tiTCP:5173 -sTCP:LISTEN | xargs kill      # the dev server you started, by its port, never by a process name
```
