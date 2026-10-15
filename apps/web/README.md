# `@jackioh/web`

The JackiOh client: React 19 + Vite. It renders a `PlayerView` and nothing else. The engine, the AI
and the deck validator it runs locally are the Rust crates compiled to WebAssembly (`crates/wasm`,
[The engine as WebAssembly](#the-engine-as-webassembly) below).

## The one rule

CLAUDE.md rule 7 and SPEC §10.8: **the client sends intent and renders `viewFor`. It never
enforces rules and never sees hidden information.** In practice:

- Every component's only input is a `PlayerView` from `@jackioh/shared`. A component that wants
  something the view does not carry is a finding against §10.8, not a reason to reach past it.
- Legality comes from the engine's `legalActions`, never from the client. `game/actions.ts`
  filters that array; it contains no cost check, no keyword check and no zone check. Anything
  whose `data-testid` is not in the returned `Highlight.legal` renders greyed out and fires
  nothing on click.
- `EngineState` is opaque (`game/engine.ts`). The client passes it back to the port and cannot
  read a field off it, which is how "never sees hidden information" is enforced by the compiler
  rather than by discipline. Under it is the WASM module's `GameState` JSON, which no component
  reads.
- Choices split two ways per SPEC §11 R81: zone, X, embiggen, Tribute and a card's declared
  targets and modes are built **inline into the `play` action**; everything decided during
  resolution is a `PendingChoice` answered with an `answer` action. `game/Prompt.tsx` renders
  both with the same pickers.

The web fonts (`src/fonts.css`, imported first by `index.css`): Alegreya for display
(`--font-display`, which `--tavern-serif` names) and Inter for text (`--font-text`), both OFL,
variable over weight, Latin subset, self-hosted in `public/fonts` with their licences. Each is
`font-display: swap` behind a metric-matched local fallback, so text measures the same before and
after a face lands, and `cards/fit.ts` refits every mounted box once `document.fonts` reports
`loadingdone`. A font file's name carries its version, since `/fonts/*` is cached for a year
(`public/_headers` and `vercel.json`, held equal by `net/cloudflare-config.test.ts`).

## Three flows at the table

None of them is a rule (CLAUDE.md rule 7): each reads the view, takes its moves from `legal`, and
sends intent. The testids are `contract.ts`'s `testid`, and e2e reads the same strings.

**The mulligan is both seats' at once** (SPEC §2.1, R265–R268). Each seat answers its own, in
either order; an answer is sealed until the other is in, and the hand does not change until then.
The view says so: a seat that still owes sees its own `pending` mulligan, one that has answered sees
`pending: { forYou: false, pendingFor }`, and `view.mulligan = { youReady, opponentReady, kept? }` is
present for exactly that window.

- The picker (`Prompt.tsx`) keeps its testids (`prompt-modal` with `data-prompt-kind="mulligan"`,
  `prompt-option-<id>`, `prompt-submit`); its confirm reads "Ready". Under the count,
  `mulligan-opponent-status` (`data-ready="true|false"`) says "Opponent is choosing…" or holds
  `mulligan-opponent-ready`, "Opponent is ready".
- After Ready, until the game starts: `mulligan-waiting`, "Waiting for your opponent…", with the
  hand and each card stamped Keep or Redraw from `kept` (`mulligan-waiting-card-<id>`,
  `data-verdict`; `data-returning` counts the cards going back). It has no `data-prompt-kind`, so no
  picker helper takes it for a question.
- Practice: the AI owes its mulligan from the first snapshot and answers it after
  `promptAnswerMs` (controller.ts), never after the human; the human's Ready goes out whenever it is
  pressed, queued behind an AI step already in flight.
- Hotseat: the device follows the view's `pending.pendingFor`, so after one seat's Ready it goes to
  the seat that still owes one, whichever answered first (p2 may answer first after a manual
  hand-over). Turn 1 is then handed over with the button, as every turn is.
- Online, the window has one clock (R268): the server reports its deadline as the frame's
  `promptDeadline` and as each seat's `clockMs`, and `Clock.tsx` (with `mulligan` set by the match
  route for exactly the window) shows it on both sides, the seat that is ready included, as
  `data-kind="mulligan"` over `MULLIGAN_CLOCK_MS`, with no turn clock.

**Concede asks first** (`ConfirmConcede.tsx`, mounted by `Game.tsx`, so every mode has it). The
`concede` control opens `concede-dialog`, a modal `alertdialog` "Concede this game?"; only
`concede-confirm` ("Concede") sends `{ type: "concede" }`. `concede-cancel` ("Keep playing") has the
focus on open and is what Escape and a click outside do; Tab stays in the dialog, and the focus goes
back to the control when it closes. A hotseat hand-over or the game's end drops the question.

**A draw offer is a question with two answers** (SPEC §2.5, R36, R269). Only the active player
offers, and `view.drawOffer = { by }` stands on both seats until the offer is answered or lapses at
the end of the offerer's turn. `DrawOffer.tsx` draws one element, `draw-toast` (the element the
animation table's `drawOffered` and `drawAnswered` rows play on), inside an always-present
`aria-live` region across the top of the screen:

- the offerer: `draw-offer-status`, "Draw offered — waiting for reply";
- the other seat: `draw-offer` (a `region`, not a modal), "Your opponent offers a draw", with
  `draw-accept` and `draw-decline`, live exactly when `legal` lists the matching `answerDraw` (the
  engine withholds it while a prompt is open) and sending that listed body;
- afterwards, `draw-outcome` with `data-outcome`: "You declined the draw" / "Your opponent declined
  the draw" for the rest of that turn, "Draw accepted" (the result reads "Game drawn by
  agreement."), and "The draw offer expired" through the turn after an unanswered offer lapsed.
- the offerer, after a decline: while `view.drawOfferBlockedTurns` is set (the viewer's own block, R36, never the opponent's), the `offer-draw` control reads "Draw blocked (N)", with "Offer draw blocked (N of your turns left)" as its tooltip; it is dark because `legal` lists no `offerDraw`, never because of the count.
- Sound (`audio/cues.ts`): the answering seat hears an urgent notify (a doorbell, `urgent: true`);
  the offerer hears `cancel` on a decline, and an acceptance is sounded by the `gameOver` that
  follows. That is online: in hotseat the offer and the answer each hand the device over, and a
  hand-over plays nothing (SPEC §10.11), so there the notice itself is the whole of it.
- Hotseat: an offer from the seat holding the device hands it to the other seat to answer, and the
  answer hands it back to the player whose turn it is (`hotseat.ts`). Only the offer moves it: if
  the players pass the device back unanswered with the seat switch, the offerer plays on and the
  offer lapses with the turn (R269). Practice hides the Offer draw
  control (SPEC §9.9, R188); an offer made anyway is declined at once and reads as declined.

## Patch v0.2.0 at the table

**Activate** (SPEC §6.2, R384, R510). A card whose view lists `activations` (the engine gives them to
its controller's own view of a card acting on the field) wears one control per ability on the card
itself (`ActivateControl.tsx`): a lightning glyph and a badge with the uses left this turn ("∞" for
Activate ♾️), the ability's words (its `{key}`s filled from the card's `params`) as tooltip and
accessible name. It is `activate-<instanceId>` with its badge `activate-uses-<instanceId>`, the
ability named after the id (`activate-<instanceId>-<ability>`) only on a card listing several
(`namedAbility`, `contract.ts`). It is live exactly when `legal` lists that card's `activate`;
otherwise it is greyed and its tooltip gives the view's `reason`. A press reports
`{ on: "activate" }`, and `actions.ts` builds the activation exactly as a play's choices are built: one
listed body is sent at once; several wait for a target clicked on the board (or dragged to from the
control, or from a card of yours that has nothing to attack and one ability), a Tribute, or a mode
in the inline picker. Heroic Power is built the same way: `power` (the first power) and
`power-<instanceId>` (any further one) report the power's activation: the `activate` naming the
power the card rolled, which is all `legal` lists for it since R752 (an old `activatePower` still
matches). The control flashes (`data-flash="activated"`, a static ring under
reduced motion) while the `activated` row plays on its card.

**A play's payments** (B5 E5, E11, E19). Plays that differ by `discards` (Classic #89's targeting
cost) ask for them after the target, in the `hand` picker; plays that differ by `plague` (Classic
#74) ask how many Plague Counters pay, in a `number` picker of chips with "Pay in mana only" first. A
Tribute and its zone are narrowed together as whole candidates (R391), so a Tribute onto its own
zone is picked by clicking the unit or its zone.

**Plays from the graveyard** (B5 E11). While `legal` lists a `play` for a card in the viewer's
graveyard, that pile glows and its sheet (`cards/inspect/CardList.tsx`) puts "Play"
(`pile-play-<instanceId>`) under the card; it reports `{ on: "graveyard" }`, closes the sheet, and the
play is built as a hand card's.

## Commands

```
pnpm --dir apps/web dev          # vite dev server on :5173
pnpm --dir apps/web build        # production build
pnpm --dir apps/web build:e2e    # the build e2e drives: development mode, so window.__jackioh stays
pnpm --dir apps/web typecheck    # tsc -p apps/web/tsconfig.json
pnpm --dir apps/web test         # vitest (jsdom)
```

`dev`, `build`, `build:e2e` and `test` first run `scripts/build-wasm.sh` (their `pre` scripts), which
needs the Rust toolchain `rust-toolchain.toml` pins; `typecheck` reads the module's generated `.d.ts`,
so run one of them (or the script) once first.

The dev hotseat route is `/dev/hotseat?seed=42&a=first20&b=first20`. It runs `reduce` in the
browser and exposes `window.__jackioh = { state, dispatch, seed, … }` whenever
`import.meta.env.MODE !== "production"`, which is what the Cypress specs drive (BUILD M5-T3).

To look at the client rather than test it (a layout, a card face, the phone view, a board mid-game),
an agent drives a headless browser with `sh scripts/browse.sh` (#811): Microsoft's Playwright CLI,
installed on first use into `.cache/` and named in no manifest, so no one else pays for it.
`.claude/skills/ui-walkthrough/SKILL.md` is the walkthrough: serving the client, the routes, setting a
board up through `window.__jackioh`, waiting for animations, the viewports and the screenshots. It is
for work on what a player sees, and nothing else.

Replays (SPEC §9.3, §9.9, R768, R768000) open from three places: `/replays` (`routes/replays.tsx`) lists
the practice games kept on the device and, signed in, the account's online games a page at a time; the
practice result dialog and an online match's result panel offer "Watch the replay" (step 0) and "Share the
last turn" (the last step, its turn's GIF already being made). A practice result reads the kept game from
the worker's final snapshot (`replay`) and asks its pages through the practice controller's queue; a
match result loads its first page to learn the replay's length (`OpenedReplay`, `openOnlineMatchReplay`).
The GIF code still loads on demand, never in the main bundle.
A finished co-op game's result panel offers "Watch the replay" too (§13.9): `duos/DuosReplay.tsx` reads
`GET /api/duos/matches/:matchId/replay` once and steps the seat's `DuosView`s on `DuosBoard`'s replay mode,
with the player's controls, a round standing for a turn; it is never in `/replays`.

```
src/replay/
  source.ts       ReplaySource (a total and pages of steps), ReplayFailure and the sentence each reason shows
  sources.ts      the two sources: the practice worker's `replay` requests, the server's replay routes
  steps.ts        the pages fetched so far, by step; load, prefetch, and the turn moves' search; a clip's ends
  ReplayPlayer.tsx  Game on a step's view with no legal action (`replay`), the controls in its notice slot;
                  `start` and `shareOnOpen` for a result screen's two buttons (R768000)
  OpenedReplay.tsx  a source loaded first (a finished match's first page), then the player
  ShareGif.tsx    "Share as GIF" (#513, R513000): the clip, the shape, progress, cancel, Share or Download
  gif/            what it loads on demand, never in the main bundle: plan.ts (a clip's views to frames, a pure
                  function on a fixed frame clock), raster.ts and raster.worker.ts (the frames drawn on an
                  OffscreenCanvas, in a worker where it can), encoder.ts (GIF bytes by hand), share.ts, export.ts
  config.ts replay.css
```

## Decks, the lobby and the series

The screens an account uses between games (SPEC §9.4, §9.5, R250–R264, R330–R341). None of them
decides a rule: every verdict they show comes from `@jackioh/validator` (the Rust validator through
WASM), the module the server runs at queue, and the server's refusal, when it comes, is shown in its own words (rule 7).

```
routes/decks.tsx        /decks: loads GET /api/decks, the catalog and the collection, hands them to the workshop
game/deckbuilder/       the deck workshop: up to ten named decks and five trios (R250, R252), the pool
                        browser (CardBrowser.tsx, which the Card Almanac renders read-only too, R630),
                        the trio editor that marks every card two of its decks share (R251),
                        deck codes (deckCode.ts, R255), trio codes (trioCode.ts, built on deckCode.ts's
                        parts, R339) with Copy trio code in the trio editor and Import trio
                        (TrioImportPanel.tsx, R340), and autosave with a local mirror of unsaved edits
                        (sync.ts, R256); a trio import is one POST /api/trios/import, never the autosave (R341)
routes/play.tsx         /play: the mode picker (Best of 1, Conquest, All Random, R257, R330), the deck or
                        trio choice with the validator's verdict as UX, the queue and the room code; it
                        waits on /api/auth/me's currentMatchId and currentSeriesId; it shows the player's
                        own rank and links /leaderboard (R661); a queue it joined is remembered for the
                        tab, so the player may leave while they wait and it picks the wait up again (R765);
                        a room's ticket shares it as an invite link, and a lobby opened on one fills in
                        its join form (R767); a queue the server no longer holds ends the wait with a
                        rejoin message (R897000)
net/roomLink.ts         R767: the room link (/play?room=…&mode=…): built for the share sheet or the
                        clipboard, read once off the address bar into this tab, through a sign-in, for
                        the lobby
net/liveGame.ts         R765: the player's online game, read off /api/auth/me: the queue this tab joined,
                        which main.tsx's useQueueFollow follows on every screen but /play ("Match found!",
                        then the game), and useLiveGame, the live match or series the menus' banner offers;
                        an ended queue is forgotten after QUEUE_ENDED_AFTER_READS reads (R897000)
routes/GameBanner.tsx   R765: the one banner atop a menu while there is a game to go back to: a saved
                        practice game (Resume, on /practice) and the live online game (Rejoin, on / and
                        /practice); game-banner.css draws it as the tavern's notice
routes/series.tsx       /series/:id: a Conquest series (R330–R336): score, both sides' won (locked) decks,
                        the history, forfeit between games, the result
routes/SeriesPicker.tsx the deck-selection phase before each game, laid out as the mulligan (R331–R333,
                        R338): choose a deck that has not won, lock it in, wait sealed; one clock
routes/SeriesBanner.tsx the board's banner for a series game, each side's won decks as pips, and its
                        "Continue" once the game is over
routes/leaderboard.tsx  /leaderboard: the global ranked ladder (R608, R612) — Jlorious #1–#100, the
                        Grape tiers, the Raisins — and the caller's own standing; rank/rank.ts says
                        a visible rank in a player's words for this screen, the account and the board
routes/lobby.css        the lobby's, the series screen's and its picker's tavern look, and the banner's pips
```

A practice game offers the account's complete saved decks by name, read from `GET /api/decks`.

## Play together and Duos

Play together (SPEC §9.4, §9.5) and Duos co-op (SPEC §13) are screens over the server's answers and
views, and decide nothing: a friend's status, whether they may be challenged or watched, a chat
message's mask and every refusal come from the server, shown in its own words, and a Duos board's
legality comes from the `legal` array its socket sends (rule 7). Every screen here needs a signed-in,
active account, and asks for one the way `/play` does.

```
routes (net/navigate.ts paths, and both vercel.json route tables):
  /together              together/TogetherRoute.tsx: friends with status dots, the requests in and out
                         (accept, decline, cancel), the blocked players (unblock), the add form by
                         name#tag; per friend Challenge (online and idle), Watch (when `watchable`),
                         Chat (always), Remove and Block; "Shared with me"; the co-op panel
  /watch/:matchId        together/WatchRoute.tsx: a friend's seat over /ws/watch, Game on a view with no
                         legal action, SPECTATE_DELAY_MS behind
  /duos/:matchId         duos/DuosRoute.tsx: the networked co-op board over /ws/duos
  /dev/duos              duos/DevDuosRoute.tsx: a four-seat hotseat Duos game over the WASM Duos
                         bindings, with a seat switcher (development only, as /dev/hotseat)
net/social.ts            fetch wrappers for every Play together route, typed (Friend, FriendRequest,
                         Challenge, ChatMessage, SharedReplay, PlayerRef); errors are ApiRequestError
net/duos.ts              fetch wrappers for the co-op routes, and the ping types (PingKind, PingTarget)
together/ChatPanel.tsx   a conversation, polled every CHAT_POLL_MS while open
together/InMatchChat.tsx the match board rail's chat toggle: online friends, each opening ChatPanel; the
                         friends list is read only while its list is open
together/config.ts       the social reads' pace: the friends list every FRIENDS_POLL_MS (a heartbeat,
                         PRESENCE_HEARTBEAT_MS), the challenges and unread counts every SOCIAL_POLL_MS
together/ChallengeToast.tsx  mounted once in the shell; polls GET /api/challenges while signed in and
                         toasts an incoming challenge anywhere: Accept opens /play with the room code
                         filled in (R767), Decline declines
together/presence.ts     usePresenceHeartbeat: POST /api/presence every PRESENCE_HEARTBEAT_MS
duos/CoopPanel.tsx       create a co-op room (difficulty, a deck or a random one), show its code,
                         invite a friend by sending the code in chat, join by code; both players go to
                         /duos/:matchId once the guest joins
duos/DuosBoard.tsx       Game.tsx for the own duel, the life strip (both teams' own HP and pools, a
                         spent seat's bar giving way to its pool), the pass control, the portal, the
                         teammate strip and the ping wheel
duos/DuosReplay.tsx      a finished co-op game's replay (#965): the server's steps read once, stepped on
                         DuosBoard's `replay` mode (nothing live) with the replay player's controls
```

The landing page offers Play together beside Play online. The replays list and the replay player offer
"Share with a friend". The match board's rail says how many friends are watching while any are
(`match-watchers`, from the match socket's `watchers {count}` frame, which `game/net.ts` keeps as the
snapshot's `watchers`). On the Duos board:

- the pass control on an inspected hand card, or a hand card dragged onto the portal, sends `Pass`
  only when `legal` holds it, with the pass cost and the passes left beside it (SPEC §13.5);
- the portal flips the board to the teammate's duel, read-only (`legal` empty, a no-op `onAction`),
  where cards can be hovered and inspected; the teammate strip always shows their own HP, mana, hand
  count and whether they have ended, and "Waiting for your teammate" shows once the seat has ended;
- the ping wheel on a hovered or inspected card offers the five pings; a received ping plays its
  sound, marks its card for `PING_SHOW_MS` on either view and flashes the portal when the card is on
  the other view (SPEC §13.10). A Duos board deals no emote hand and draws no aim arrow.

The Duos WASM bindings (`duos_create`, `duos_begin`, `duos_reduce`, `duos_legal_actions`,
`duos_view_for`, JSON in and out like the 1v1 ones) are reached only through `src/wasm/index.ts`, for
`/dev/duos`; a networked co-op game's state stays on the server. Only the normal module carries them,
and only the dev server's build (`predev`'s `JACKIOH_WASM_DUOS=1`, crates/wasm's `duos` feature): in
any other build each throws the reason.

Test ids, mirrored in `e2e/support/testids.ts`: `landing-play-together`, `together-add-input`,
`together-add-submit`, `together-friend-row`, `together-request-accept`, `together-request-decline`,
`together-challenge`, `together-watch`, `together-chat-open`, `chat-input`, `chat-send`,
`chat-message`, `coop-create`, `coop-difficulty`, `coop-code`, `coop-join-input`, `coop-join-submit`,
`duos-portal`, `duos-pass`, `duos-pool`, `duos-own-hp`, `duos-teammate-strip`, `duos-ping-wheel`,
`duos-ping-<kind>` and `duos-ping-marker`.

## Practice

`/practice` (SPEC §9.9, R187) is a game against the AI with no account and no server. The engine
and the AI (`crates/engine`, `crates/ai`, as WebAssembly) run in a Web Worker, which stands where SPEC §9.1 puts the server, so
rule 7 holds exactly as it does online:

```
src/practice/
  protocol.ts         the request/response shapes: everything the page ever learns about a game
  core.ts             the ONLY practice file that imports @jackioh/engine, @jackioh/cards or
                      @jackioh/ai; holds the GameState, runs both seats and the AI
  practice.worker.ts  the worker entry: core.handle per message
  host.ts             a module Worker in a browser, the in-thread core in jsdom; one answer per
                      request, strictly in order
  controller.ts       one request in flight, and the pacing loop that plays the AI's turn one
                      visible action at a time (think indicator, gaps from config.ts)
  config.ts testids.ts PracticeSetup.tsx ThinkIndicator.tsx PracticeLeave.tsx PracticeResult.tsx practice.css
  decks.ts            random, the three named practice decks (hand-built lists), the account's saved decks by name
  lastBoard.ts        the human's last practice board (C+ #29, R417, R508) in localStorage
                      `jackioh.practice.lastBoard`, try/catch
  saveStore.ts        the worker's save of a free game in progress (R668): config, log, AI cursor,
                      catalog version, hash, in IndexedDB `jackioh.practice` (memory in jsdom), and the last
                      PRACTICE_REPLAYS_KEPT finished free games (R768)
  resume.ts           the page's half (R668): only the setup it chose, in localStorage
                      `jackioh.practice.game`, try/catch, marked `saved` once left with Save and leave (R765)
  DeckPreview.tsx     the chosen deck's name, identity, mana curve and cards, before Start
  ModifierList.tsx    every live R169 modifier in full, one tap from the HUD
  sandbox.ts SandboxPanel.tsx sandbox.css
                      a sandbox game's dev tools (R923000–R923003): the card search, the cards the
                      human can name, and the panel that sends each tool as a `sandbox` act
routes/practice.tsx   the route: the tutorial path, setup, HUD, and Game.tsx unchanged inside the worker's catalog
```

- The page holds snapshots, never a state: `{ view, legal, aiToAct, error }`, where `view` is
  `viewFor(state, human)` and `legal` is `legalActions(state, human)`. The AI seat's hand and
  library never cross the worker boundary. A finished free game's snapshot adds `lastBoard`, the
  engine's `lastBoardFor(state, human)`; the controller keeps it on the device and sends it with the
  next start, as the human's seat's last board (R508). The AI's seat and a lesson never have one.
- A free game in progress survives a reload (R668). After every answer the core writes its save
  to the worker's store, which never crosses to the page (the log names the AI's hidden cards); the
  controller remembers the setup when the game is dealt and forgets it when the game ends, and the
  route forgets it when the player leaves. A visit with no parameter that starts a game sends
  `{ type: "resume", config }`, and the core folds the log back under the same nonces to the saved
  hash, with the AI's stream at its saved cursor, or answers `failed` and the route shows the setup.
  A lesson is never kept, so only a lesson in progress asks before the page unloads.
- Keeping a free game is the player's choice when they leave it (R765). "Leave this game?" offers
  Save and leave beside Leave without saving: the first marks the remembered setup `saved`, and the
  next `/practice` shows the setup with a banner at its top (`routes/GameBanner.tsx`) whose Resume
  sends the same `resume`; the second forgets it. A reload while the player is in the game still
  resumes it at once, and starting any game from the setup or the lesson path ends the saved one.
  The same lobby shows the live online game's banner (Rejoin) for a signed-in player in a match.
- A finished free game is kept for its replay (R768): when it ends the worker adds its config, log,
  catalog version, final hash and a summary to the store, the newest PRACTICE_REPLAYS_KEPT of them, but
  not a lesson or a game Glitch voided (R679). `{ type: "replays" }` answers the summaries, newest
  first, and folds none of them: each has `unavailable` set to `earlier_patch` when it was played on
  another catalog, or to `rules_changed` once opening it found that it no longer folds to its hash;
  `{ type: "replay", game, from, count }` answers a page of `{ step, turn, view }` in the seat the
  human played at each step (R677), and `total`, from the fold the worker holds of each of the last
  PRACTICE_REPLAYS_CACHED games opened. Both are refused while a game is in progress, a game that is
  unavailable is refused with its reason, and neither ever carries the log, the seed, the decks or a
  state. Nothing on screen sends them yet.
- The one exception is `debug`, which carries the raw state, the log, the decks, the handicaps and
  the dealt seats for spec 13's replay check. The core answers it only when `MODE !== "production"`,
  and the route sets `window.__jackiohPractice` under the same condition, like `window.__jackioh`.
- A practice game replays exactly from `(seed, decks, handicaps, lastBoards, dealt, sandbox, log)`: nonces are `h<n>` for the
  human and `a<n>` for the AI, counted over accepted actions only, and the AI draws from its own
  stream (`${seed}:ai`), never the match rng.
- A sandbox game (R923000, the setup's Sandbox box) is created with `sandbox: true`, the only game
  whose engine takes the `sandbox` action. The panel's tools are the human's acts, logged as `h<n>`;
  its "Hold the AI's turn" is one more `controller.setHold` reason. It records no statistics, brings
  and leaves no last board, and resumes only as a sandbox.
- The random deck is dealt, not built (R433): `createGame` is told the human's seat was dealt, so
  the human's deck pile lists only what they have been shown of it and a card back for the rest. A
  preset, a saved deck and a lesson's fixed deck are listed in full; the AI's seat is never dealt.
- URL params (all optional): `?seed=`, `?difficulty=easy|medium|hard`, `?deck=random|preset:<id>`
  (both together start a game at once), `?seat=p1|p2`, and `?pace=fast` for e2e, which a
  production build ignores.
- `vite.config.ts` sets `worker: { format: "es" }` for the module worker.
- The setup previews decks from the catalog a short-lived worker sends (`{ type: "catalog" }`), so
  the page still bundles no card data; an autostarted game (`?difficulty=&deck=`) asks for none.
- The AI's next step waits while anything on the board carries `data-animating`, while any
  element on the page carries `data-speaking` (Game marks its root while a voice line holds the
  audio engine's channel; a mark that is never cleared holds the AI for at most
  `PRACTICE_VOICE_HOLD_MAX_MS`), and while the showcase holds up the card the AI has just played
  (`data-showcase`, at most `PRACTICE_SHOWCASE_HOLD_MAX_MS`). `?pace=fast` waits for neither the
  voice nor the showcase. The controller's general form is `setHold(reason, held)`. The settings panel's "Reduce motion" gives the reduced pacing,
  as the media query does.
- A lesson in progress asks before a reload or a closed tab ends it (`beforeunload`), and the HUD's
  Menu leaves for the landing page, asking first (Save and leave, or Leave without saving) while the
  game is on.

## The tutorial

The practice lobby opens with the tutorial (SPEC §9.10, R290–R294, R320–R322): four lessons on a path, each
a practice game with two fixed decks, a fixed seed and seat, and the tutorial handicap on the AI
seat (`AI_TUTORIAL`, the tier below Easy). A lesson is a practice config whose `lesson` names it
(`tutorial/start.ts`), so the same worker, controller and board play it; the worker reads the
decks from `tutorial/lessons/<id>.ts` and nothing else the page sends changes them (R291).

```
src/tutorial/
  lessons.ts lessons/*.ts   each lesson as data: title, mechanics, seed, seat, both decks, retry tip
  scripts/*.ts              each lesson's coach script: steps in order and reactive tips (page only)
  coach.ts                  the coach machine, pure: coachObserve per snapshot, ack, display (R292, R314)
  steps.ts targets.ts       reads of the view and legal actions, step factories, anchors → board testids
  tracker.ts                feeds every controller snapshot to the coach and holds the AI for `holdAi`
  Coach.tsx layout.ts       the ring, and the bubble placed clear of what it points at (a panel above the board on phones)
  TutorialPath.tsx          the lesson path at the top of the lobby (locked / open / done), with Hide / Show
                            tutorial (R322; its look in path-visibility.css)
  TutorialHud.tsx TutorialResult.tsx   the lesson's HUD (step counter, Exit tutorial) and its result dialog
  progress.ts               completed lessons and the Hide/Show choice in localStorage `jackioh.tutorial.v1`,
                            try/catch (R294), and R321's merge with another copy (a union, the newest choice)
  accountSync.ts            an active account's copy (`GET`/`PUT /api/tutorial`, R320) kept level with the
                            device's (R321); the practice route mounts it
  harness.ts                test support only: a lesson played through the real core by a policy (R293)
  config.ts testids.ts devHandle.ts   the numbers, the testids (mirrored in e2e), `window.__jackiohTutorial`
```

- The coach reads the snapshot the page already holds (`view`, `legal`, `aiToAct`) and nothing
  else, and never sends an action (rule 7). A step points at a board element by its testid and
  completes when the view shows it done. There is no Skip step, and nothing needs one (R314): only
  a tip or an `info` step holds the AI, and each shows "Got it"; an action step never holds it; a
  step expires after `TUTORIAL_STEP_TURNS_MAX` of the player's own turns (the lesson's `final` step
  ends with the game); and Exit tutorial is in the HUD throughout. Nothing strands a lesson.
- The bubble adopts a new step only once the board has caught up (no `data-animating`), so it never
  points at a card the board has not drawn; a step or tip with `holdAi` holds the AI through
  `setHold("coach", …)` until its "Got it".
- `?lesson=<id>` starts a lesson at once (`&pace=fast` works as for practice); `?seed=` and
  `?seat=` never override a lesson's own. `window.__jackiohTutorial` (dev builds only) exposes the
  coach's display and the action its current step asks for, which spec 22 performs through the UI.
- `scripts/lesson-deal.ts <lessonId> [seed | --scan …]` prints a lesson's deal, which is how a
  lesson's seed is picked: `pnpm --dir apps/web exec tsx scripts/lesson-deal.ts basics`, after
  `sh scripts/build-wasm.sh` has written `src/wasm/pkg` (it loads the module from disk through
  `src/wasm/`, as `src/test/setup.ts` does).
- Progress lives on the device and, for an active account, on the account too (R320, R321). The
  device's copy is the one the page renders; `accountSync.ts` reads the account's once per visit,
  merges it in (the union of completed lessons, the newest Hide/Show choice by the time it was
  made), sends up what the account lacks, and after that sends each change as it happens. A request
  that fails changes nothing on screen and throws nothing; the next visit catches the account up.
  Signed out, pending or banned, nothing is sent at all.
- Hide tutorial (R322) sits in the path's header while a lesson is still to do and folds the path to
  one Show tutorial button in its place; focus moves to the button that undoes the press. The choice
  is stored with the progress, so it holds on the next visit and on the account; a finished path
  folds to its header by itself and offers no Hide.

## Regenerating the music

The music is composed as code (R631): `scripts/music/tracks.mjs` holds every track's score, built
on the toolkit in `compose.mjs` and `theory.mjs`. That means keys, tempos, progressions,
arrangements, and melodies on one shared motif. `scripts/gen-music.mjs` turns each score into MIDI,
renders it with FluidSynth and the FluidR3 GM SoundFont, and encodes it with ffmpeg to stereo AAC at
44.1 kHz and 80 kbps. The tracks are committed with `src/audio/music-manifest.json`, so CI never
renders. After editing a score, run `pnpm --filter @jackioh/web gen:music` on a machine with
`fluidsynth`, `ffmpeg` and the SoundFont (Debian or Ubuntu: `apt install fluidsynth
fluid-soundfont-gm ffmpeg`); with any of them missing it exits 2.

- It renders only the tracks whose input hash changed: the MIDI, the render and post settings, and
  the encoding.
- It deletes orphan files, rewrites the manifest, and fails if the set passes `MUSIC_BUDGET_BYTES`.
- `--only <id>` renders one track, and `--force` renders everything.
- A looping track's file holds its intro, one pass of the body, and a short tail. The loop points
  live in the manifest. The renderer crossfades into the audio before the loop's start, ending 0.1 s
  before the loop's end, and copies the loop's start from there to the file's end. So the jump is
  sample-exact even for a decoder that keeps AAC's 48 ms of encoder priming.
- `node apps/web/scripts/gen-music.mjs --check` needs no renderer, runs anywhere, and is what
  `music-assets.test.ts` calls.
- Record a new track's source in `assets/music/LICENSES.md`. A station switch or a card's intro is an entry in
  `src/audio/music-cards.json`, and the music system needs no change for it. No card plays a theme (R822000).
- A card's intro (R1352) is a score in `tracks.mjs`'s `INTRO_CARDS`: the card's facts as the catalog printed them and
  its `tune`, from which `introSpec` derives the rest (key, tempo, meter, motif and answer from the id; instruments,
  accompaniment and modes from the family; the set's glint; a Mythic's shimmer). It renders as a sting of a few bars
  with a one-second faded tail (`tail`, `post.fadeOutS`), its loudness measured over the file it keeps
  (`post.loudnessOverFile`); a track without those fields renders exactly as before. A Legendary or Mythic added to a
  set that ships needs its `INTRO_CARDS` row, its `intro` in `music-cards.json`, a render and a licence row:
  `music-assets.test.ts` refuses a shipped one without them.

## Regenerating the voice lines

Every card's sounds are in `src/audio/card-audio.json5` (R655), a JSON5 file edited by hand, whose
header explains it: a `voices` bank, an `effects` bank (a procedural recipe from `sfx.ts` at its own
`pitch` and `gain`, which renders nothing), and `cards`, in catalog order with each card's name in a
comment beside its id, where each hook (`play`, `attack`, `death`, `cast`, `trigger`; `CARD_HOOKS` in
`constants.ts`) gives a voice line (`voice` and `text`), an `effect`, or both — a `trigger` (R1088)
gives a voice and a non-empty `lines` list instead, one of which plays when the runner starts the
`summoned` entry the card caused. `voiceData.ts` checks it
at load and names the path of any mistake. The voice files are generated from its lines and
committed, so CI never runs a synthesizer. Each voice names its backend (R501): Core's voices are
macOS `say` (the default), and the Classic and Classic+ voices are `backend: "sapi"`, Windows SAPI
(`System.Speech`, the "Microsoft David Desktop" and "Microsoft Zira Desktop" voices) shaped by
`ffmpeg`: trimmed, pitched by the voice's `semitones`, coloured by its `filter` chain, peak-normalised
and encoded to the same mono AAC at 22050 Hz and about 32 kbps. After editing a line or a voice, run
`pnpm --filter @jackioh/web gen:voice` on a Mac for `say` voices, or on Windows or WSL (it finds
`/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe`) with `ffmpeg` and `ffprobe` on the
`PATH` for SAPI voices; with neither it exits 2. It renders only the keys whose input hash changed
and whose backend this machine has, reports a stale key of the other backend as needing it, deletes
orphan files, rewrites `src/audio/voice-manifest.json`, and fails if the set passes
`VOICE_BUDGET_BYTES` (12 MiB since R1448; 6 MiB from R501) or a line runs past 4 s. `--only <defId>` limits it to one card
and `--force` renders everything this machine can again (legacy `say` voices such as Fred are not
byte-deterministic, so expect a large diff). Commit the manifest together with `public/audio/voice/`.
With neither machine at hand, push a commit whose message holds `[gen:voice]` (or run the `voice`
workflow by hand): GitHub's macOS runner renders the `say` lines and its Windows runner the SAPI ones,
each committing what it rendered back to the branch, and a Linux job runs `--check` on the result
(`.github/workflows/voice.yml`, R1448).
`node apps/web/scripts/gen-voice.mjs --check` needs no synthesizer, runs on any OS and is what the
asset test calls; `--catalog <file>` checks the lines against another catalog. A line must stay
flavour text: `voice-lines.test.ts` enforces the word limits and bans rules words. At runtime a file
missing from the manifest falls back to the browser's `speechSynthesis`, with the voice's `web`
pitch and rate.

## The engine as WebAssembly

Since v0.3.0 the engine, the card scripts, the AI and the validator are Rust (`crates/`), and the
client runs them as one WebAssembly module (`crates/wasm`, its README the binding list). JSON
strings go in and out; the state is an opaque JSON object the client hands back on every call.

- `scripts/build-wasm.sh` builds the module into `src/wasm/pkg/` (gitignored). It adds the
  `wasm32-unknown-unknown` target and downloads the pinned `wasm-bindgen` CLI only when they are
  missing, and cargo rebuilds only what changed. The package's `pre` scripts run it, so `dev`,
  `build`, `build:e2e` and `test` always load a fresh module.
- `src/wasm/index.ts` is the module's only caller. `main.tsx` starts `loadWasm()` at boot, and its
  `EngineGate` holds every screen that calls the engine until it resolves (#778; the deck builder
  calls the validator synchronously), the practice worker awaits it before its first message, and
  `test/setup.ts` loads it synchronously for jsdom.
- `src/wire/` keeps every import the client had when these packages were TypeScript working
  unchanged: `@jackioh/shared` is the generated wire types plus hand-kept helpers, `@jackioh/engine`
  and `@jackioh/ai` are typed functions over the module, `@jackioh/engine/config` and
  `@jackioh/server-config` are the generated Rust constants, `@jackioh/validator` the validator,
  `@jackioh/cards` the catalog and its version (read from `crates/cards/patches/patches.json`), and
  `@jackioh/cards/catalog.json`, `flavour.json`, `chinese.json` and `chinese-terms.json` the files in
  `crates/cards`.
- The generated files (`src/wire/generated/`, `engineConfig.ts`, `serverConfig.ts`) are written by
  `cargo test --workspace --features jackioh-engine/testkit,jackioh-engine/ts`; CI fails when they
  differ from what is committed, so a Rust change that moves the wire commits them in the same change.
- Three files hold a game's state, and nothing else imports the engine's game functions: `game/engine.ts`
  (hotseat's port), `practice/core.ts` (the practice worker's game, and the only caller of the AI's
  `decide`) and `duos/DevDuosRoute.tsx` (the Duos dev hotseat's `DuosState`, opaque, through the WASM
  Duos bindings, which only the normal module carries), with `tutorial/harness.ts` for tests.
- **The Luau module** (#919, SPEC §12.10, R915009) is a second build of the engine with the card kit,
  for custom cards: `scripts/build-wasm.sh` writes it into `src/wasm/pkg/luau/`
  (`crates/wasm/README.md`). Only `src/wasm/luau.ts` imports it, and only `src/wasm/index.ts` imports
  that, dynamically: `useLuauModule(on)` switches the bindings every engine call goes through, which
  the practice worker does per request (`PracticeCore.needsLuau`), and `checkCustomCard(card)` loads it
  for the creator without switching. A normal game never loads it (`wasm/luau.test.ts` holds that no
  other file imports it), and the normal module refuses a game with custom cards.

## Custom cards (SPEC §12)

- **The creator**, `/creator` (`routes/creator.tsx`, `creator/`): the account's custom cards, read and
  saved through `net/customCards.ts` (`/api/custom-cards`, R915008), and an editor for one. The frame is
  `creator/FrameForm.tsx`; the script is written in the no-code builder (`creator/builder.ts` is its
  vocabulary and the kit Luau it writes, `BuilderView.tsx` its blocks) or the code view (`CodeView.tsx`,
  highlighted by `highlight.ts`). `creator/draft.ts` is the editor's state and the pure steps between
  the views (R915010). The frame's issues come from the validator (`validateCustomCard`), the script's
  from the Luau module's `checkCustomCard`, and the preview draws `customDef`'s definition.
- **The deck builder** puts the account's custom cards (`creator/useCustomCards.ts`) in its catalog
  under a "Custom" set, owned once each; a deck naming one is marked practice only by the validator's
  L7 (R915005), and the play lobby offers it disabled.
- **Practice** starts a saved deck with its custom cards on the config (`customCards`), and
  `/practice?try=<id>` (`tryPath`) deals a random deck around one card. Such a game runs in the Luau
  module, records no card statistics, and keeps its custom cards in its save, its replay and its
  debug snapshot (R915006). A script that fails shows on the board (`game/ScriptFailedNotice.tsx`) and
  in the log.

## Internationalisation

The client supports multiple reader languages (`src/i18n/`). Adding another language means adding one dictionary file (`src/i18n/<tag>.json`), its card table pair beside the catalog (`crates/cards/`), and one line in `LOCALES` (`src/i18n/i18n.ts`). The English strings remain inline wrapped in `t()`, and missing translations fall back cleanly to English.
