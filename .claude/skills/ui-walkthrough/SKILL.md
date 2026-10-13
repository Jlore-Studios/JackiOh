---
name: ui-walkthrough
description: >
  Look at the JackiOh web client in a headless browser: open a route, set up a game, click
  through it and screenshot it to see. Use only when the work is what a player sees: apps/web
  components, CSS and layout, card faces and art, animations and effects, the phone layout, or
  when asked to look at, screenshot or play the UI. Not for engine rules, card scripts, the AI,
  the server, the bot, CI or docs, nor to prove what a test already proves: booting the client
  takes minutes and every screenshot costs context.
---

# Walking through the UI

`sh scripts/browse.sh <command>` drives a headless Chromium through Microsoft's Playwright CLI, one
shell command at a time, and the browser stays open between commands. The script's header says what
it installs (once, into `.cache/`) and which browser it picks (#811).

If the task is not about what the client shows, stop here: nothing below is worth its minutes.

## 1. Serve the client

```
CYPRESS_INSTALL_BINARY=0 pnpm install           # once per checkout; the variable skips Cypress's 800 MB binary
pnpm --dir apps/web dev --port 5173 --strictPort  # its predev builds the WASM engine first (about 2.5 minutes cold)
```

Run the dev server in the background and wait until `curl -s -o /dev/null http://localhost:5173/`
succeeds. `--strictPort` makes it fail rather than move to another port when 5173 is taken (another
worktree's server, say): then pick a free port and use it everywhere below, or the walkthrough
looks at someone else's checkout. An edit under `apps/web/src` hot-reloads the page, which starts a
hotseat game over at turn 0: set the board up again after an edit.

## 2. Pick a route

| Route | What it shows |
|---|---|
| `/dev/hotseat?seed=42&a=first20&b=cheap20` | A two-seat game run in the page; `a` and `b` are `DECK_IDS` (`apps/web/src/game/decks.ts`). Dev builds only |
| `/practice` | The tutorial lessons and a game against the AI, with no server |
| `/almanac` | Every card face and token, searchable: art, frames, rules text |
| `/`, `/patch-notes` | The landing and the patch notes |

The rest needs the E2E server (`/stats` renders empty without it) or a signed-in session
(`/leaderboard`, `/decks`, `/play`, `/match/<id>` send you to `/login`); `e2e/README.md` covers both.
Stay on the routes above unless the task is about those pages.

## 3. Open it and set up a board

```
sh scripts/browse.sh open "http://localhost:5173/dev/hotseat?seed=42"
```

`open` once per walkthrough: on an open session it starts the browser again, and the board, the
size and the emulation go with it. Move with `goto <url>` after that. Leaving a practice game or a
lesson in progress raises a leave-page dialog that blocks every later command until
`sh scripts/browse.sh dialog-accept`.

On `/dev/hotseat`, `window.__jackioh` is the dev handle (`apps/web/src/routes/dev/hotseat.tsx`,
`HotseatDevHandle`): `legal()` lists the legal actions of the seat holding the device,
`dispatch({ ...action, playerId })` plays one, `setSeat(seat)` hands the device over, `view()` is
that seat's `PlayerView`. A game opens on both mulligans, so nothing else is clickable yet; this
keeps both opening hands and plays the first legal card:

```
sh scripts/browse.sh eval "async () => {
  const until = Date.now() + 10000;
  while (!window.__jackioh && Date.now() < until) await new Promise((r) => setTimeout(r, 100));
  const j = window.__jackioh;
  for (const seat of ['p1', 'p2']) {
    j.setSeat(seat);
    const keep = j.legal().filter((a) => a.type === 'mulligan').sort((a, b) => b.keep.length - a.keep.length)[0];
    if (keep) j.dispatch({ ...keep, playerId: seat });
  }
  j.setSeat('p1');
  const play = j.legal().find((a) => a.type === 'play');
  if (play) j.dispatch({ ...play, playerId: 'p1' });
  return play;
}" --raw
```

The handle's `state` is the whole game, hidden cards included: use it to reach a board, never to
judge one. What the player sees is the page, which renders `viewFor` (CLAUDE.md rule 7).

On `/practice` there is no `__jackioh`: start a lesson by its test id
(`click "getByTestId('tutorial-start-basics')"`), then answer the coach and the mulligan through the
page. `window.__jackiohPractice` (`view`, `thinking`) and `window.__jackiohTutorial` (`display`,
`suggested`, the coach's next action) say where the game is.

## 4. Drive it

```
sh scripts/browse.sh find "End turn"                   # the matching elements and their refs (e12, f2e42)
sh scripts/browse.sh click e12                         # or a locator: click "getByTestId('end-turn')"
sh scripts/browse.sh eval "() => [...document.querySelectorAll('[data-testid^=\"hand-card-\"]')].map((e) => e.dataset.testid)" --raw
sh scripts/browse.sh hover "getByTestId('hand-card-c8')"            # a card in hand: its preview opens
sh scripts/browse.sh screenshot "getByTestId('inspect-face')" --filename=fix-face.png   # that preview, full size
sh scripts/browse.sh resize 390 844
sh scripts/browse.sh screenshot --filename=fix-board.png
```

- **Test ids** are `apps/web/src/game/contract.ts`'s (`end-turn`, `hand-card-<instanceId>`,
  `card-<instanceId>`, `zone-<side>-<row>-<lane>`, …) and `apps/web/src/cards/inspect/testids.ts`'s
  for the preview; an open prompt carries `data-prompt-kind`. Hand cards have no accessible name,
  so `find "<card name>"` misses them: list them by test id as above.
- **Output size.** `find "<text>"` answers in a few hundred bytes on a board, and a bare `snapshot`
  in about 2,000 tokens. On `/almanac` both are huge (a bare `snapshot` is about 70,000 tokens, and
  `find "Unit"` matches 470 elements): find an exact card name, or add `--filename=<file>` to write
  the result to a file. `find --regex "<pattern>"` takes a pattern. A ref is good until the page
  changes: find it again after an action.
- **Files.** The CLI runs in `.cache/browse/` (gitignored), so a relative `--filename` lands there,
  and the paths it prints (`./fix-board.png`, `out/page-….yml`) are relative to it: read
  `.cache/browse/fix-board.png`. Name files after your task, since other walkthroughs in this
  checkout share the folder. Never write a screenshot into the repository.
- **Sessions.** Two walkthroughs on one machine each take a session: `-s=<name>` on every command,
  `close` included (`sh scripts/browse.sh -s=<name> close`). Never `close-all` or `kill-all`, which
  end everyone's.

## 5. Wait, then look

- **Animations.** An event's animation marks its element `data-animating` until it ends, and a voice
  line holding the channel marks the board `data-speaking`. Wait for both to clear before a still,
  with a deadline, since a stuck animation may be the very bug:

  ```
  sh scripts/browse.sh eval "async () => { const until = Date.now() + 10000; await new Promise((r) => setTimeout(r, 50)); while (document.querySelector('[data-animating], [data-speaking]') && Date.now() < until) await new Promise((r) => setTimeout(r, 50)); return document.querySelector('[data-animating], [data-speaking]') ? 'still animating' : 'settled'; }" --raw
  ```

  `sh scripts/browse.sh set-reduced-motion reduce` cuts the animations short as a player's reduced
  motion does (BUILD M5-T4): a card's play settles in under a fifth of a second instead of over one.
  To see an animation mid-flight, leave it on and screenshot straight after the action.
- **Viewports.** It opens at 1280x720. BUILD M5-T1 also holds the board at 390x844, and the e2e
  specs shoot 768x1024 too: `resize <w> <h>`. Resizing changes the page's size only: no touch, and a
  pixel density of 1. To see what a phone does, open as one instead:
  `open "--device=iPhone 13" <url>` (390 wide, density 3, touch; `--device=iPhone SE` is 320 wide,
  `--device=Pixel 7` 412). The `=` and the name's capitals matter: `--device "iphone 13"` silently
  opens a desktop. Judge text size by `getComputedStyle(element).fontSize` through `eval`, and
  whether a label fits by its `scrollWidth` against its `clientWidth`, not by eye.
- **Cost.** A 1280x720 screenshot is about 1,200 tokens. Shoot what the task needs: one element
  (`screenshot <ref or locator>`) for a card face, and `find` instead of a picture when the question
  is what is on the page rather than how it looks.
- **What it cannot show.** No sound (the browser is muted), no GPU (canvas and WebGL render in
  software), and an agent sees stills.

## 6. Clean up

```
sh scripts/browse.sh close        # with your -s=<name>, if you took one
```

Stop the dev server you started (its background job, or the pid listening on your port), and only
that one.
