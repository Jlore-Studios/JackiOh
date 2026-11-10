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
  nothing on click (a hand card the view gives a `playReason` shows that reason, R979000).
- `EngineState` is opaque (`game/engine.ts`). The client passes it back to the port and cannot
  read a field off it, which is how "never sees hidden information" is enforced by the compiler
  rather than by discipline. Under it is the WASM module's `GameState` JSON, which no component
  reads.
- Choices split two ways per SPEC §11 R81: zone, X, embiggen, Tribute and a card's declared
  targets and modes are built **inline into the `play` action**; everything decided during
  resolution is a `PendingChoice` answered with an `answer` action. `game/Prompt.tsx` renders
  both with the same pickers. A choice of `DISCOVER_OPTION_LIMIT` (4) options or fewer that name no
  card draws each option as a choice card (`game/ChoiceCard.tsx`, #2646): art, its label and what it
  does, which is prompt chrome and no card, so it opens no inspect overlay and reaches no zone or log.
  A live card picker (`data-prompt-layout="cards"`) is its cards alone over the dimmed board (`data-bare`, #2738): no panel, title or count, and no `prompt-submit` when one tap answers.
  The other seat's is its backs (`game/OpponentChoice.tsx`, R2739000) when the view's `pending` carries `optionCount`: that many of the opponent's card backs, laid out as the bare picker over the waiting notice's lighter dim and in the middle of the screen at every width (a phone docks the chooser's picker to its bottom edge), with no words or control and "Your opponent is choosing…" as the dialog's name; without it, the notice. Online, the back the opponent's pointer or focus is on is outlined in their color, still (R2739001): an engine card picker but the mulligan sends its option's index as it changes and null as it leaves, folds or closes (`game/choiceHover.ts`, `net.ts`'s `choiceHover` frame), which the server relays to the opponent alone and drops when the count is not shown; a play's own picker, a replay's, hotseat and practice send none.
  A Choose one of lanes (`lane-<n>`, #2757) opens no list: one oblong a lane (`game/LanePick.tsx`), answered from its zones as R514's cells are.

The arena (`src/game/arena/`, R3348000, R3348001): a living playmat that is only ever the viewer's own, and that
no rule reads. `SandSurface.tsx` answers a press on any tile with a press-in and a ripple in its half's playmat
family and that family's sound, harder on a legal tile and with a haptic on touch, and never stops the press. Each
of the ten playmats keeps two corner props (`CornerProps.tsx`, drawn by `PropArt.tsx`): a tap sounds and moves
one, `CORNER_ESCALATE_TAPS` taps in a row give its payoff, each tap rolls `CORNER_RARE_CHANCE` from the client's
own random source (`arenaRandom`, never the game's RNG) and four mats hide a tap sequence. The layer takes a mat id
and nothing else: it imports nothing from `net/`, `wire/` or `replay/`, sends and logs nothing, and answers quietly
while the animation queue drains. A prop that would cover a tile, a hand, a seat, the controls or the log, or leave
the screen, is not drawn (`fit.ts`); a phone 840 high or more gives the field a strip of mat above and below its
lanes for them (`arena.css`), a shorter one has none to give. Every number is in `arena/config.ts`, and `arena.css` states the same times
(`arena/arenaCss.test.ts`).

The web fonts (`src/fonts.css`, imported first by `index.css`): Alegreya for display
(`--font-display`, which `--tavern-serif` names) and Inter for text (`--font-text`), both OFL,
variable over weight, Latin subset, self-hosted in `public/fonts` with their licenses. Each is
`font-display: swap` behind a metric-matched local fallback, so text measures the same before and
after a face lands, and `cards/fit.ts` refits every mounted box once `document.fonts` reports
`loadingdone`. A font file's name carries its version, since `/fonts/*` is cached for a year
(`public/_headers`, held by `net/cloudflare-config.test.ts`). A number that changes in place or stands in a column (the clock, a count, a price, a wallet, a bar, a rank, a stats table) takes `font-variant-numeric: tabular-nums`; headings and dialog and toast titles balance their lines and paragraphs wrap pretty (`index.css`, `text-wrap-style`); a card face takes neither, since `cards/fit.ts` measures its text (`src/typography.test.ts`).

The logo is `public/brand`: the mark (two fanned cards, a gold J), the wordmark beside it or under it,
the stacked logo and the mark alone each on the tavern's wood, and opaque app icons, as SVGs, which a screen shows (smaller, and sharp at any size). Their PNGs are in
the repository's `photos/logo`, which no build reads. `scripts/brand-logo.py` draws them all, outlining
the wordmark from the self-hosted Alegreya, so no SVG needs a font; run it by hand and commit what it
writes. The logo's shadows are stacked strokes, not SVG filters: WebKit draws whatever a filter touches
at 1x, so an `feDropShadow` blurred the whole mark on a phone (the wood variants' grain and glow, which
no screen shows, still use filters).

The colors are one palette (#2105): `src/palette.css`, imported by `index.css` after the fonts. A board stylesheet (`index.css`, `buttons.css`, and `src/game/`'s but the deck builder's and `table.css`) names a palette color, or `color-mix()`es one, and never states a color of its own. Meaning stays in tokens that name palette colors: `--you`, `--opponent`, `--legal`, `--selected` and `--radiant` in `index.css`, the glows in `game/highlights.css`. `src/palette.test.ts` checks this, and holds every other stylesheet to its count in `UNMIGRATED`.

The text is one type scale (#2107): `src/typescale.css`, imported by `index.css` after the palette, ten sizes from `--type-3xs` (9px) to `--type-4xl` (30px), the board's body being `--type-md` (13px). Each but `--type-md` has an em form, `--type-<step>-em`, the step over 13px, for text that scales with its box (the board, a card), where `--type-md`'s is `inherit`. The faces are `fonts.css`'s four families: `--font-display`, `--font-text`, `--font-mono` and `--font-signature`, the self-hosted Dancing Script (OFL) a Mythic full art signs its card's name in (R2655003), and beside it the sets' signature hands, `--font-signature-<face>` (R2715002: Kaushan Script, Pinyon Script, Mr Dafoe, Nanum Brush Script, Sedgwick Ave Display and Permanent Marker, each self-hosted with its licence, `cards/signatureFaces.ts` naming which set or card signs in which). A board stylesheet gives every `font-size` a step (a `clamp()` or `max()` may hold a viewport or container term beside one), every `font-family` a family or `inherit`, and `font` only `inherit`. `src/typescale.test.ts` checks this, holds every other stylesheet to its count in `UNSCALED`, and keeps `src/game/`'s components from setting a size or a face inline.

The surfaces are one token scale (#2109): `src/surfaces.css`, imported by `index.css` after the type scale, names seven corner radii, four neutral shadow depths and twenty-one stacking levels, each layer holding its level's current number so stacking order is unchanged. A board stylesheet gives every `border-radius` a radius, every neutral `box-shadow` a shadow (glows and selection rings keep their own), every `z-index` a layer, and keeps a background gradient only where it means something: card faces, the main button's `--primary-face*` and the table. Everything else is flat. `src/surfaces.test.ts` checks this, and keeps `src/game/`'s components from setting a radius, a shadow, a layer or a gradient inline. Every other stylesheet, the menus' among them, is held to its count of raw radii, shadows, layers and gradients in `UNTOKENED_SURFACES`: a sheet that moves values onto the tokens lowers its number, and none gains one (the dungeon's sheets state none).

The motion is one vocabulary (#2110): `src/motion.css`, imported by `index.css` after the surfaces, names four durations (`--motion-instant` 80ms, `--motion-fast` 150ms, `--motion-base` 250ms, `--motion-slow` 400ms) and five curves (`--ease-standard`, `--ease-out`, `--ease-in`, `--ease-in-out`, `--ease-overshoot`). A board stylesheet gives every transition and animation a curve (or `linear`, or `steps()`; a `@keyframes` stop spells a curve's value out) and every duration a token, keeping its `* var(--anim-scale)`. These keep their own times: BUILD's (the event table in `game/animations.css`, the `activated` and `turnEnded` rows), R701's wobbles, the table's tap marks, loops, staggers, holds, the clock's 200ms repaint, and any time a `var()` feeds in. `src/motion.test.ts` checks this and lists each sheet's own times in `KEEP`. Every other stylesheet, the menus' among them, is held to its count of raw durations and curves in `UNTOKENED_MOTION` on the same terms.

The spacing is one scale (#3225): `src/spacing.css`, imported by `index.css` after the motion, names eleven lengths, `--space-1` (2px) to `--space-11` (40px): 2px apart to 12px, where the board's own lengths sit, then 16, 20, 24, 32 and 40px. A board stylesheet gives every `gap`, `padding` and `margin` (and the `--gap` they read) a step, `0`, `auto`, or a `calc()`, `clamp()` or `max()` holding one, and may keep an `em` or `%` term (it scales with its box), a viewport or container term and `env()`, never a px or rem length of its own. A length that is no step is composed of steps (`calc(var(--space-9) + var(--space-2))`), and a negative one is a step times -1. `src/spacing.test.ts` checks this and holds every other stylesheet to its count of px and rem lengths in `UNMIGRATED`; a sheet that loses some lowers its number.

The breakpoints are one named set (#3227), `src/breakpoints.ts`'s `BREAKPOINT_PX`. Each is the widest (or tallest) window of the smaller layout: `(max-width: N)` holds up to N and the layout above starts at `(min-width: N+1)`. They are `phone` 600 px (a phone held upright), `narrow` 760 px (a small tablet upright), `medium` 899 px (a tablet upright), `tablet` 1024 px (the desktop starts at 1025) and, for height, `short` 500 px (a phone on its side). Touch is `(pointer: coarse)`. A sheet spells the number out (CSS reads no custom property in `@media`), and a script builds its query with `upTo()`, `over()` and `PHONE_SHEET_QUERY`. `src/breakpoints.test.ts` holds both, its `KEEP` listing each other value with its reason.

Every menu is one screen (R1581000): at 1280x720 and 390x844 the page is no taller or wider than the
window, and what a menu holds past its room scrolls in its one region, never in the page, with its
top (Back, the gear, the title) staying put. A menu's root takes `one-screen` (`index.css`, which
makes the shell the window's height); its region takes `one-screen__body`; a box between the two
that must shrink (a tavern panel) takes `one-screen__frame`, and a wrapper that stacks its children
as the shell does takes `one-screen__stack`. Under 501 px tall (a phone on its side) the page
scrolls as it did. The deck builder and the almanac do the same on a phone (`deckbuilder.css`,
`workshop.css`, `almanac.css`), as they already did on a desktop. No type shrinks to make it fit.
The tavern home alone may be two screens tall (#1576). e2e spec `46` visits every menu at both
sizes and fails when the page is taller or wider than its allowance.

On a phone every control takes a tap across `--hit-min` (44 px) each way (R2122000). Under
`(pointer: coarse), (max-width: 600px)`, `index.css` gives each button, link, `role="button"` and
`tabindex="0"` shorter than that a transparent `::before` centered on it; one as tall but narrower
takes its band in its own stylesheet, since a band over a tall control's box would take the taps of
what it holds. The rule has zero specificity, so a control's own `::before` or `position` wins: the
switch draws none, and a browsable pile's band is its `::after`. An input or a select draws no
`::before`, so a small one takes `min-height: var(--hit-min)` in its own stylesheet. A checkbox or a
radio is tapped through its labels. Neighbours nearer than that split the gap. e2e spec `47`
measures the board and every menu of `46` at 390x844.

A phone highlights nothing but the chat (#3260). `index.css` sets `-webkit-tap-highlight-color: transparent`
on `html` at every pointer, since only touch browsers draw a tap flash, and under `(pointer: coarse)` sets
`user-select: none` and `-webkit-touch-callout: none` there, so a narrow desktop window still selects text.
Fields (`input`, `textarea`, `contenteditable`) take `user-select: text` back, since iOS will not type into
or put a caret in one that cannot be selected. `.chat-panel` (`together/together.css`) restores all three,
so a message can be pressed, selected and copied. The two-step sign-in's key keeps its `user-select: all`,
since it has no copy button. No stylesheet sets the tap highlight back to `initial` outside `.chat-panel`
(inherited, it would bring the flash back on every child); `src/highlight.test.ts` checks it.

The top bar is one skin (#3189). BackLink's `.screen-nav` and the landing's `.landing-corner` draw their
Back or Account link, Friends, language button and settings gear with one rule, `routes/top-bar.css`,
worn by the class `top-bar-button` (`--round` for the icon buttons, a 44 px circle; `--text` for a word in
a pill). A new top-bar control adds that class and never a copy of the skin; the Jlore pill is drawn alike
in `economy/jlore-pill.css`. The nav carries the three brass colors itself (`auth/tavern.css`), since
the replays and the deck screens are not `.tavern`, and the tavern's bare-button size rule and buttons.css's
link-button skip `top-bar-button` through a `:where(:not())` that adds no weight. The board's control bar
(`placement="game"`) draws its own buttons.

The buttons are four roles (#3219), each one skin in `src/buttons.css`, imported by `index.css` after the
motion and drawn the same on every screen, in the tavern or not, on a menu or the board: `button-primary`
(the one way on, `--primary-face`'s gold; the result panel's first action takes it by its place),
`button-secondary` (on a `<button>` or an `<a>`), `button-danger` (`--danger`'s red, for the act that
cannot be taken back: Concede, Delete account, Leave game, Leave fight) and `link-button` (a quiet text
action). A `<button>` that names no role looks like the secondary. A control keeps only its own layout
(size, margin, place), and the tavern only sizes its buttons. `src/buttons.test.ts` checks this: it draws
each role inside and outside `.tavern`, finds no other sheet restating a role's skin or that of a one-off
it replaced, and holds the button skins that remain no role's (the top bar's, card-like tiles, pressed
toggles) to each sheet's count in `UNROLED`.

The dialogs are one component (#3220, R3220000): `Dialog` in `src/game/Dialog.tsx`, drawn by
`src/game/dialog.css`. A caller gives it its `role` (`dialog` or `alertdialog`), its `h2` `title`, an
optional `description` (the panel's accessible description), `onClose`, its `actions` (each a label, a
role among `primary`, `secondary` and `danger`, an `onClick` and a testid), its `testid`, and optionally a
`className` that restyles the panel only, `data-*` attributes, a `header` above the title, `children`
under the line and a `footer` under the row. An `alertdialog` takes a `cancel` too, the safe choice, which
is its close and draws no ✕; a `dialog` draws a ✕ named "Close" at its top right (`dialog-close`). The row
runs in one order: the safe choice first, holding the focus, then the actions as given with a `danger` one
moved last, at the right; three answers put the first on a row of its own, and at 360 px and under the
row stacks in the same order. Its scrim is one color at `--z-dialog` (`surfaces.css`), and its keys are
`useModalKeys`'s (`together/modalKeys.ts`), whose third argument, the control to focus, also gives the
focus back to the opener on close unless the caller has moved it: Escape, Back (a layer) and a click on
the scrim close it (not one whose press began in the panel, a selection dragged out, R3221000), and Tab
stays inside. `ConfirmConcede`, `ConfirmLeaveMatch`, `PracticeLeave`,
`ConfirmLeaveFight` and `PracticeResult` draw through it; `PracticeResult` keeps its look through
`practice-result`, and its ✕, Escape, Back and a click outside view the board.

Since #3221 (R3221000) the menus' dialogs draw through it too: `SettingsPanel`, the friends panel
(`FriendsButton`), `ChatPanel`, `ChallengeDialog`, `TokenOffer`'s and the shop's chase question, the
account's delete question, `/stats`'s card breakdown, `DungeonStats`'s run details, the dungeon's Your run
panel (`RunPanel`), `FieldGuide`, `PatchNotes`'s card, `ItemPreview`, `CardCosmeticsView`, `LooksScreen`,
`DevButton`'s panel, `CommandMenu` and `TutorialResult`. A caller may also give it an `id`, its own
`closeTestid` and `closeLabel` for the ✕, a `scrimTestid`, `focus` (what takes the focus on open, else the first answer,
else the ✕), `panelRef`, a `title` with markup of its own, and answers that are `disabled` or `busy`
(`aria-busy`), the `cancel` disabled too while a request is out. `sheet` makes it a sheet: centred on a
wide screen, at the foot of an upright phone and down the right edge of a phone on its side (Settings,
Friends, Chat). `DialogClose` is the ✕ alone, for an overlay that keeps its own body: the inspect
overlay's `CardDetail` and its sheets (`InspectSheet`, `CardListSheet`, `FaceDownSheet`, the Stack's
`StackSheet`), `HeroInspect` and the pack scene take it. `useModalKeys` and `useModalOverlay` act on Escape and
Tab only while theirs is the top layer (`useBackLayer` returns whether it is, and opens its layer in a
layout effect, so an Escape read as soon as the overlay is drawn finds it on top), so a dialog opened over
another closes first. The inspect overlay, portalled after every dialog, draws at `--z-dialog` while it
is the top layer (`useTopBackLayer`, as state) and, under a layer opened after it (a result that ends the
game over a card's detail), carries `data-under` and comes down to `--z-toast`, under every dialog, so the
dialog seen is the one that takes the keys. `useModalKeys` leaves Escape to an open tooltip inside (R279, R2098000), and its
Tab skips a hidden panel and a control at `tabindex="-1"`. `src/dialogs.test.ts` holds the sources to
it: no `"Escape"` comparison outside the shared hooks and its named exceptions, no scrim outside
`dialog.css` that paints itself or takes a raw layer, one ✕ glyph, no button labelled a text Close, and
the inline confirms' safe choice first.

The top bar is also one layout (#3218, R3218000), the same sheet's. BackLink's slots run Back, the line
(`title`, then its children as the extras), a flexible spacer, then the end group: the Jlore pill, Friends,
language, the gear, so Back is the leftmost control and the gear the rightmost. On a phone (600 px, the touch
rule's) the line takes a row under Back and the end group. Only `routes/top-bar.css` places those controls:
a screen passes its title and extras to BackLink and never wraps the bar in a flex header or sets a margin on
a bar control, and the replay player's Back is BackLink's too (`onPress`, `testId`). e2e spec `46` measures it.

A menu names itself with one `PageTitle` (#3223, R3223000): its only `h1`, in `--font-display` at `--type-3xl`
(`routes/PageTitle.tsx`, `routes/page-title.css`). A menu passes its name as BackLink's `title`, so it sits in the bar's
title slot, and never draws an `h1` of its own (`routes/page-title.test.ts` counts them); the two prompts that
replace a menu and have no bar (the username's and the consent's) draw a `PageTitle` in their panel. The brand is
`Wordmark` (`routes/Wordmark.tsx`), not a heading. A section inside a screen is an `h2` of class `section-heading`
from the same sheet. No other sheet sets a font-size on an `h1`.

The dungeon map's node icons are drawn SVGs (`dungeon/map/NodeIcon.tsx`): single color in `currentColor` on a
24-unit grid, colored by `data-kind` in `dungeon/map/run.css` from the palette. No system emoji appears
anywhere under `dungeon/`, held by `dungeon/noEmoji.test.ts`; add a node kind's glyph there, never a
character.

A menu panel waiting on a read draws a skeleton, `skeleton/Skeleton.tsx` (R2115000), in one of four
shapes: a card `grid`, list `rows`, `table` rows or a generic `panel`, each a fixed number of blocks
and never a count from the read. The sentence it printed stays as a visually hidden `role="status"`
line with the same testid, and the blocks are `aria-hidden`. The blocks shimmer only under
`data-motion="full"` (the OS's preference and the settings panel's both stop it) and sit inside a
root that clips them, so the one-screen rule holds. Over them the blocks' layer draws the JackiOh
mark, still, in a dark halo (`skeleton.css`), but in a shell panel, which the logo heads. `main.tsx`'s `Loading` takes `skeleton` for the
same drawing. A route that waits before it can draw its screen draws that screen's own frame around the
skeleton, not a plain page: `/decks` does so through `game/deckbuilder/WorkshopFrame.tsx`, which the loaded
workshop draws too, so the bar (Back, the title, the gear) and the background stay put and only the body changes
when the read lands (R3228000). Still text: the deck builder's win-rate line and the chat's and the replays' "show
more" buttons (content or a button is already there), the practice start screen's shuffle and the
boards' "Joining …" panels, and, each for its reason:

- A button's or field's own busy word ("Checking…" and "Starting…" on a button, "Show older games" and "Show
  earlier messages" while they load, the username field's inline "Checking…"): a label or a field's status,
  not a wait for a panel.
- The deck history popover's "Loading…" and "No versions kept yet": the popover sizes itself to its words,
  which a full-width skeleton or empty state would stretch.
- One line inside a panel that already has content: the account's "No finished matches yet" and "No season
  best yet" (the figures above say it), the leaderboard's "No Jlorious players yet this season" (one section
  among the tiers, whose height spec 46 measures), the stats' "No … yet" sub-lines, the trio editor's slot
  "No cards yet." and the chat's "No messages yet" row.
- An option's text: "No other decks yet" and "No friends to invite yet".
- A card face's "No pieces yet", and the accessibility statement's and the invite page's sentences, which are
  prose of the page and not a panel's empty.

A wait shows only once it outlasts a moment (R2108000): `SKELETON_DELAY_MS` (300 ms) passes before
the skeleton's blocks or the shell's waiting panel appears, and a fast read never flashes either.
`useShownAfterDelay` sets `data-shown` on the skeleton and, through `shown`, on `ShellPanel`. It is
a timer because Reduce motion zeroes CSS delays. The blocks are brass. Every `ShellPanel` (a wait, a
refusal, the 404) is headed by the stacked logo, the `h1` whose name is the site's, on the tavern's
wood (`auth/tavern.css`); both are drawn only once the panel shows, so a short wait flashes no wood. An empty menu panel draws
`empty/EmptyState.tsx`: the tankard, the words with the old testid, and an optional `action` (`to`
for a link, `onPress` for a button). The ruling lists what stays text.

The 404 is a small game (R3119000, #3119). `main.tsx`'s `NotFound` keeps its words, its `not-found` and
`not-found-home` testids and its shell panel, and adds `notfound/NotFoundPlay.tsx`: `GlitchPath` names the path in
Glitch's glyphs (`cards/glitch.ts`), seeded by the path, settling one character at a time while the element's text
stays the path; `NotFoundPlay` draws the lost card `notfound/lost.ts` picks by a hash of the path, which says its
voice line through the audio engine when tapped, beside three face-down `CardBack`s that turn over into
`ChoiceCard`s for Play, Practice and the Almanac (a tap turns one over and the next follows it; a mouse over it or
keyboard focus turns it first). Under Reduce motion all lie dealt, face up and settled. The page Cloudflare serves
for a path nothing routes, `404.html`, is the client itself (R3127000, #3127): `static-pages.ts` `notFoundHtml`
writes it from the built `index.html` (entry script, `noindex`, the 404 title, no canonical link), so the client
starts and draws that `NotFound` with the real path. Its body, which shows with no script and until the client
replaces `#root`, is a fallback in the app's look (`shellTestid.notFoundStatic`: the tavern board, the stacked logo,
the button home) whose hand is three placeholder `CardFace` Spells, Error 4, Not 0 and Found 4, that deal in, fan out,
turn over from the brand's card back and float in CSS alone (`notfound/notfound.css` `.nf-hand`, still under Reduce
motion). `handCardHtml` writes their markup from the art builders `CardArt` calls, and `static-pages.test.ts` holds it
to `CardFace`'s own render. No `public/404.html` ships beside it.

A menu opens on its last answer (R2125000): `net/readCache.ts` keeps, in tab memory only, the last answer of each
menu `GET`, keyed by path (query included) and account (the token's `sub`, so a renewal keeps it). The kept
reads are `keptPaths` in `net/api.ts` (collection, cosmetics, quests, packs, wallet, tracks, tokens, cosmetic
packs, decks, leaderboard), `cardStatsPath(options)` for one statistics filter's table (public) and
`FRIENDS_PATH` in `net/social.ts`. A screen starts its state from `cachedRead` (never an effect's dependency),
reads again and takes the new answer; a read that fails behind a kept answer keeps it and says nothing, so a
skeleton and an error show on a first visit only. A write's answer folds into its entry (`buyPack`, `openPack`,
`openAllPacks` (its last opening's), `openCosmeticPack`, `buyChase`, both token redemptions, the decks' and trios'
save, delete, sync (each item that landed) and import, a friend accepted or removed), so a revisit never shows
the old value first. `clearSession` empties the cache and the
first answer kept for another account does too; a reload starts empty. Never kept: a match's, a series',
a duos room's, a watch's or a replay's reads and anything that is not a `GET` (`getFriends({ keep: false })`
for the watch and the replay share). `/decks` opens the workshop on the kept decks and reopens it, under a new
`key`, when the decks that land differ, since its store is made once.

A kept answer carries the time its read finished (R3206000): one younger than `READ_CACHE_FRESH_MS`
(`net/readCache.ts`, 45 s) is *fresh*, and `keptGet` returns it and sends no read behind it. The gate skips its
`/api/auth/me` read the same way (`freshRead`, for an active account that owes nothing; the renewal timer and the
401 path are as they were), and the wallet pill reads through `getWallet`, so a navigation inside the window sends
nothing. An older answer is drawn and read again as above. A fold of a write's answer keeps the age it found,
`expireRead` ends freshness and keeps the value (a quest falling due), and a gate or a poll that only finished a
read keeps it with `keepRead`, which is no write. `apiRequest` shares a `GET` of one path and token while it is out
(one fetch, on its own controller that the last caller to abort aborts); a write is never shared. `writeEpoch`
counts every spend that settles (it sends an `Idempotency-Key`) and every `rememberRead`, fold, `forgetRead` and
`clearReadCache`: a read asked after it moved never joins an earlier one, and one that finished after it moved is
not kept. A write that changes a kept read without carrying it drops it (`forgetRead`; `forgetPaidReads` for a
game's paid result, which `announceWallet()` with no balance calls), and a dev tool's success empties the cache.

A move between two menus cross-fades (R2117000): `net/navigate.ts`'s `showMove` runs it inside `document.startViewTransition`, the render flushed with `flushSync`. A move into or out of a board's route (`isBoardPath`), off a board on screen, within one screen, under Reduce motion (`settings/motion.ts`) or without the API is drawn at once; a move the leave guard holds (R1754000) waits for it.

An `<a>` to a screen of the app (`href={paths.…}` or an address starting with `/`) carries `onClick={followInApp(…)}` (`routes/nav.tsx`), which moves in place and leaves a modified or middle click to the browser, or opens a new tab (`target="_blank"`); a bare one reloads the site, emptying the read cache and reloading the engine. `src/in-app-links.test.ts` holds it, with the links that load the page on purpose (`FULL_LOADS`) and why.

**Back closes the top overlay** (R2127000): `net/backLayers.ts` keeps the open sheets and dialogs as layers, newest on top, through `useBackLayer`, which `useModalOverlay` (`cards/inspect/store.ts`), `useModalKeys` (`together/modalKeys.ts`; `Dialog` in `game/Dialog.tsx`, the board's confirms, the practice result and, since #3221 (R3221000), `SettingsPanel`, `ChatPanel`, the Collection's `CardCosmeticsView`, `/stats`'s card breakdown and the menus' other dialogs, through it) and, through `BackLayer` mounted with their panels, the match's `Marketplace`, the deck builder's `DeckHistory`, `DeckCosmeticsPicker` and `PortraitPicker`, the home screen's `QuestsBookmark` and `NewsBookmark`, the match's `ModifierList` and the hero's `EmoteMenu` and `MuteMenu` (not drawn `inline` in hero inspect, which owns them) call with their own close. Where the browser has `CloseWatcher` the top layer holds one, and Escape is left to the layers' own keys; elsewhere the first layer pushes one history entry copying the address, spent when a layer closes another way, never piled up and stepped back off on Forward. `navigate.ts`'s `onPop` asks `popBackLayer` before the leave guard and the view transition, and its `go` lets `moveOverBackEntry` move over the entry. A prompt, a play being built and the mulligan are no layers, so Back leaves the screen over them as before.

**The command menu** (R2118000): Ctrl+K (⌘K) opens a modal menu on every menu screen —
`routes/CommandMenu.tsx` (the shortcut and the dialog, `commandMenuTestid` and
`commandEntryTestid`), `routes/commandEntries.ts` (the actions, the pages and the Almanac
shelf's cards, each a `navigate` move) and `routes/command-menu.css` (no animation). It is never
mounted on a board route, nor over a board sharing a menu's path, which `game/boardShown.ts`
marks; New deck goes through `newDeckPath` (`net/navigate.ts`), which the workshop makes and
opens, and a card opens its inspect detail at `/almanac?card=<id>`. Presentation only (rule 7).

**Shortcut hints** (R2126000): the shortcuts the client has, shown and never added.
`shortcuts/shortcuts.ts` (no React) names each once in `SHORTCUTS` and writes it: `shortcutText`
(⌘⇧Z on an Apple device, Ctrl+Shift+Z elsewhere, by user agent), `ariaKeyshortcuts`
("Control+Z Meta+Z") and `shortcutTitle` (a tooltip's "Undo (Ctrl+Z)"), with `touchOnly` and
`subscribeTouchOnly` for `TOUCH_ONLY_QUERY`. `shortcuts/Kbd.tsx` draws them: `useShortcutHints`,
`Kbd` (one keycap) and `ShortcutHint` (a sentence around some, `t("Focus the card, then press {keys} to inspect")`).
They show in the hover preview of a card (`keyHint` on `useInspectTrigger`, "Focus the card, then press I to inspect"),
in the Undo, Redo and replay step and turn buttons' tooltips, in the command menu (Enter on the
selected entry, a legend of its keys) and in the site footer (Ctrl/⌘+K). A touch-only device
(`(hover: none) and (pointer: coarse)`) shows no hint, though `aria-keyshortcuts` stays. A widget's
own keys (Enter, Space, Escape in a dialog) are unhinted, and no hint adds a key:
`shortcuts/shortcuts.test.tsx`'s `BOARD_KEY_HANDLERS` freezes the key handlers of the board's sources,
so a new one fails it (there is no End-turn key). Presentation only (rule 7).

**Card depth** (R3344000, R3344001): a face that is Epic or rarer, or prints full art, draws its picture as four stacked planes
by the roles its painter tags its layers with (`cards/art/pictures.ts` `PLANE_OF_ROLE`: the background far, the props
in the middle, the subject near, what lies over it, its light and its motes nearest), and tilts toward the pointer up to
`CARD_TILT_MAX_DEG`, the planes shifting by their depth (`CARD_DEPTH_STEP_PX`) under a glare that lies opposite the tilt
(`cards/depth.ts`, `cards/tilt.ts`, `cards/depth.css`). A Legendary or Mythic face in the viewer's own hand and in inspect
lets its subject break the window's top edge. A delivered art file (no layers), a Starship in play and a Legacy card (one
picture), a restyled picture and Immersive, Out of frame and Extended art (layers of their own) stay as they were, and the
Reduce motion setting, the system's and the fx intensity off draw the flat picture with its foil still. `CardFace`'s `tilt`
says where a face listens: `"host"` its parent (a card in hand, on the board, in the Collection), `"window"` the whole
window's pointer (the hover preview, which takes none; the pointer is read against the box of the card the preview is
anchored to, `tiltAnchor`), `"drag"` that and a touch drag on the face (inspect, where the
sheet's own swipe does not start on it); the card views also tilt with the phone's orientation once "Tilt with my phone" is
on (`deviceTilt`, a device-only card setting; iOS asks leave once, inside the switch's tap). One face tilts at a time and
none while the animation queue drains (`useTiltHold`, `game/Game.tsx`). The face's rotation is the one 3D thing: `.cf` is
an isolated group, which draws what it holds flat, so the planes move by `translate`. Presentation only (rule 7).

## Three flows at the table

None of them is a rule (CLAUDE.md rule 7): each reads the view, takes its moves from `legal`, and
sends intent. The testids are `contract.ts`'s `testid`, and e2e reads the same strings.

**The mulligan is both seats' at once** (SPEC §2.1, R265–R268). Each seat answers its own, in
either order; an answer is sealed until the other is in, and the hand does not change until then.
The view says so: a seat that still owes sees its own `pending` mulligan, one that has answered sees
`pending: { forYou: false, pendingFor }`, and `view.mulligan = { youReady, opponentReady, kept? }` is
present for exactly that window.

- The picker (`Prompt.tsx`) keeps its testids (`prompt-modal` with `data-prompt-kind="mulligan"`,
  `prompt-option-<id>`, `prompt-submit`); its confirm reads "Ready". Under the cards,
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

**Concede asks first** (`ConfirmConcede.tsx`, drawn through `Dialog.tsx` and mounted by `Game.tsx`, so every mode has it). The
`concede` control opens `concede-dialog`, a modal `alertdialog` "Concede this game?"; only
`concede-confirm` ("Concede") sends `{ type: "concede" }`. `concede-cancel` ("Keep playing") has the
focus on open and is what Escape, Back and a click outside do; Tab stays in the dialog, and the focus goes
back to the control when it closes. A hotseat hand-over or the game's end drops the question. A dungeon fight passes its own `concedeBody` saying the run ends (R2972000).

**One press is sent once** (R3060000). `Game.tsx`'s `dispatch` remembers the last `endTurn`, `answer` or
`mulligan` it sent and the view it was sent against, and drops an equal body against the same view, so a
double click on End turn, a double tap on a one-tap choice or a double press of the mulligan's Ready is one
intent (after a card that gave an extra turn the second End turn was legal and ended the extra turn unseen).
A new view or an `error` that gains text releases it; the error line clearing does not. Everything else, the
board click and drag paths included, goes straight to `onAction`. Hotseat's views are new objects on every
render, so it never blocks.

**Card motion** (#3347). Cards move like cards, all client-side presentation keyed on public events and the
player's own input (CLAUDE.md rule 7); the engine sends nothing new. A card played from the hand flies on an
arc to its tile (the effects layer's stand-in, `fx/stage.ts`), starting where a drag let go
(`game/drag/landing.ts` `dropPointOfTestid`) and turned toward the tile (`--fx-rot`), inside the play row's own
time. A deck riffles once per action, after its last `shuffledIn` or `traded` (`game/animations.ts`
`riffleEntries`, `RIFFLE_MS`, drawn by `Board.tsx`'s `pile-riffle` leaves); a mulligan's returned cards fly from
their hand into it. The opening hand (a game whose first view is in the mulligan phase, `game/opening.ts`) is
dealt a card at a time from the deck, `DEAL_STAGGER_MS` apart, the opponent's as backs; a tap ends it and goes
on to what it was on. The fan springs where it reflows on `translate` (`game/handSpring.ts`,
`--ease-overshoot`), and a hovered card parts its neighbours. A dragged card's ghost leans with the pointer's
speed up to `DRAG_TILT_MAX_DEG` and settles flat, the tile under it lifts (`data-drag-over`), and a card let go
over nothing springs back to the hand (`game/drag/tilt.ts`). Pick-up, drag-over, drop, return, deal and riffle
each have a soft sound (`audio/sfx.ts`), and lifting a card buzzes a phone (`haptics/`). Speed scales it all
(R201); `game/cardMotion.ts` gives the level: reduced motion is `still` (every move snaps to its end, no flight, no
deal), effects off is `calm` (the sounds stay, the flourishes go), the rest `full`. `Game.tsx` writes it as
`data-card-motion` and board.css reads it.

**Back in a live online match asks first** (R2479000). While `/match/<id>` shows a view with no result on a socket neither voided nor refused, `routes/match.tsx` holds `guardLeaving`: every way off opens `ConfirmLeaveMatch.tsx`'s `leave-match-dialog`, naming `DISCONNECT_GRACE_SECONDS`. `leave-match-stay` has the focus and is what Escape, Back and a click outside do; `leave-match-leave` makes the held move. A result or leaving the route releases the guard. The co-op board asks too (R2973000, §13.8): DuosRoute.tsx holds the same guard while its view is live, and its line names the teammate playing on and the AI taking the seat after DISCONNECT_GRACE_SECONDS.

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
  offer lapses with the turn (R269). Practice and a dungeon fight draw no Offer draw
  control (their `Game`'s `showOfferDraw={false}`, SPEC §9.9, R188; practice.css hides it on every other
  `.practice-table` board); an offer made anyway is declined at once and reads as declined.

## Patch v0.2.0 at the table

**Activate** (SPEC §6.2, R384, R510, R1751000). A card whose view lists `activations` (the engine gives
them to its controller's own view of a card acting on the field) wears one control on the card itself
(`ActivateControl.tsx`), however many abilities it lists: a lightning glyph and a badge with the uses
left this turn ("∞" for Activate ♾️; on a card listing several, the sum of its usable abilities'), the
ability's words (its `{key}`s filled from the card's `params`) as tooltip and accessible name. It is
`activate-<instanceId>` with its badge `activate-uses-<instanceId>`; each ability counts its own uses and
cooldown (R1751000), and `namedAbility` (`contract.ts`) is kept only for the ability a press names. It is
live exactly when `legal` lists that card's `activate`; otherwise it is greyed and its tooltip gives the
view's `reason`. A press reports `{ on: "activate" }`, and `actions.ts` builds the activation exactly as a
play's choices are built: one listed body is sent at once, except on a card listing several abilities,
which asks which first (the `ability` need, a Discover of choice cards in `Prompt.tsx` showing each
ability's words and uses left, even when only one is legal) and leaves the card's other abilities usable
the same turn; the rest wait for a target clicked on the board (or dragged to from the
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

**The Trade** (SPEC §6.1, §10.10, R1838000). A hand card of the viewer's that `legal` lists a `trade`
for, or whose view carries a `tradeReason`, gets the Duos pass control's bar when pointed at
(`TradeControl.tsx` over `cardBar.ts`, which `duos/CardActions.tsx` shares): `trade-bar` holding
`trade-<instanceId>`, "Trade" with the Trades left from the view's `trades`. It is live
exactly when `legal` lists that `trade` and sends that body; otherwise it is greyed with the engine's
reason as its tooltip. On a Duos board the button rides the card bar beside the pass control, and
Game's own bar is off (`tradeBar={false}`).

**Plays from the graveyard** (B5 E11). While `legal` lists a `play` for a card in the viewer's
graveyard, that pile glows and its sheet (`cards/inspect/CardList.tsx`) puts "Play"
(`pile-play-<instanceId>`) under the card; it reports `{ on: "graveyard" }`, closes the sheet, and the
play is built as a hand card's.

**Credits** (issue #1840, R1840005). `Hero.tsx` draws each seat's `marketCredits` from the view alone,
as a pill in the hero's badge row beside the Jade Counter's (`market-credits-<side>`, on both seats),
and nothing while the view carries none; a count of 0 still shows. A Resource is a hand card since
#2250 (R2250000), so the hand draws it and there is no Resource tray. A Resource's face prints
`Credits: <min>–<max>` under its text, its face's `resource` prices (the face model's `creditsLine`, jackioh-shared, R2521009).

**The Marketplace** (issue #1853, R1853003). Once the view carries `marketplace`, `Marketplace.tsx` puts
a toggle (`marketplace-button`) in the board's control bar (the pill at the bottom left where the board has
its sidebar; #2645) that opens its panel (`marketplace-panel`), upward from the bar's top edge: the viewer's
Credits, the turn the offering re-rolls on, each lot on its face with the price and the copies left
the view gives and its "Buy" (`market-buy-<offer>`), and a "Sell" (`market-sell-<instanceId>`) for each
sale the view lists, tray Resources then hand cards such as Spice (R1861000). Each is live exactly when `legal` lists its `marketBuy` or
`marketSell` and sends that body; otherwise it is greyed, a lot's tooltip the engine's `reason`.

**The match menu** (#2645). On a phone (`game/phoneBar.ts`, board.css's two phone layouts) the control bar
keeps to one row: End turn, the Marketplace toggle while the view carries a marketplace, the log toggle,
⋯ (`match-menu`) and the gear. ⋯'s menu (`match-menu-panel`, `game/MatchMenu.tsx`) holds Offer draw,
Concede and Friends, each a full-width 44px row with its whole label; it decides nothing, so Concede still
only asks (`ConfirmConcede`) and Offer draw keeps its words, ⋯ wearing a dot while a declined offer blocks
it. A press on a row, ⋯, Escape and Back close the menu, and the menu and the shop are never open together.
Tablets and desktops keep the buttons in the bar.

**The phone board's top strip and field badges** (#2932). Only on a phone held upright (`@media
(max-width: 600px)`, `usePhonePortrait` in `game/phoneBar.ts`); wide screens and a phone held sideways
are unchanged. A route that has a rail (a match's, a replay's, a watch's: `← Back` and the compact rank
line) passes `phoneRailTop` to `Game`, which then draws the rail as its own first child
(`board-rail--top`, `data-rail-top` on `.game`) beside the turn banner's pill, in the row the pill already
took, instead of in the control bar: the rank line is one line (its words and the crests' pip rows are
hidden visually, the crests and the names carrying it), and the control bar keeps only End turn, the log,
⋯ and the gear. The Duos board keeps its own bar. A field card keeps a card shape (width over height
between `--field-ratio-min` 0.72 and `--field-ratio-max` 0.86) and grows by height; the seats are one
fixed, thinner size (`--hero-h`, `--tray-chip-h` and `--tray-channel`), and that height is the field's.
A face-up field card's Activate control and keyword marks leave its face for `game/FieldBadges.tsx`, a
stack standing in the top-left inner corner of the card's own zone (#3257) as a sibling of the card, as
the switch is, so DEF's rotation never turns it. It never leaves the zone, so no mark reads as a
neighbour's and a press on a neighbour's ⟳ is never this unit's Activate, and it stays above the card's
name plate (`max-height`): the marks that do not fit fold into the face's `+N`, which `FieldBadges.tsx`
measures. It may cover the card's art and its cost, and the cost gem is hidden while it does; it never
covers the name, the attack or the health, nor the zone's own switch (⟳, top-right). A DEF card is
turned a quarter, which puts its name, attack and health in a strip down its left, under that corner, so
over a DEF card the stack starts at the same corner and stops above the turned card's box: the same
control, the named marks that fit in the band above it and a `+N` for the rest (`FieldBadges.tsx`
measures again when a card's position changes). The Activate control is never dropped, and nor is the
`+N` while the unit has a mark to draw (#3315), so a DEF unit's keywords never vanish with no sign.
Where the band above the turned card is shorter than they are (about 4px at 390x664, 18 to 28px at the
other phones) they overlap the turned card's top-left, its attack gem and the start of its name, by the
difference. The control's tap band (up to
44px) reaches down from the control only, never out of the zone; the marks let a tap through. An empty
zone's placeholder (`.zone::before`) is sized to the box a field card takes there less a 2px inset, so
the unit's picture (#3341: its art runs up to the frame) and the backrow's outline keep a card's shape.

**Hand hints** (#2267). The viewer's own hand cards can wear a row of small pips top-left,
under the cost gem. `game/HandHints.tsx` draws the row as a child of each card's root
(`Card.tsx` `hintRow`); `game/handHints.css` styles it. The hints are one list, `HAND_HINTS`
in `game/handHints.ts`. Each is `{ id, icon, applies(card, view, face) }`, reads only the
`CardView`, the `PlayerView` and the card's face the way `Card.tsx` reads it, and returns null
or `{ n?, dimmed?, label }`, its label through `t()`.
No row when no hint applies. No pointer event. Never on the opponent's backs, the revealed
hand, the board or the menus. Adding a hint is one entry plus its icon (`cards/icons.tsx`).
The list ships the Tradable hint (#2269): a small swap-mark pip on each of the viewer's own
hand cards with the Tradable keyword, dimmed once the turn's Trade is spent. It also ships
`tribute` (#2268, R2268000): the Tributes a play of the card asks now, the view's `tribute`,
which the engine sets on the viewer's own hand cards only and never at 0.

## Commands

```
pnpm --dir apps/web dev            # vite dev server on :5173
pnpm --dir apps/web build          # production build, from .env.production
pnpm --dir apps/web build:staging  # staging's: the same build with .env.staging's values over it
pnpm --dir apps/web build:e2e      # the build e2e drives: development mode, so window.__jackioh stays
pnpm --dir apps/web typecheck      # tsc -p apps/web/tsconfig.json
pnpm --dir apps/web test           # vitest (jsdom)
```

`dev`, `build`, `build:staging`, `build:e2e` and `test` first run `scripts/build-wasm.sh` (their `pre`
scripts), which needs the Rust toolchain `rust-toolchain.toml` pins; `typecheck` reads the module's
generated `.d.ts`, so run one of them (or the script) once first.

Both deployed builds are `production` mode, so neither serves the dev-only routes; `build:staging`
puts `.env.staging` in the build's environment (`node --env-file`), where Vite never overwrites a
variable. Cloudflare serves `dist/` (wrangler.jsonc): `public/_redirects` routes the screens,
`public/_headers` sends the security headers, and the build fills its CSP's `connect-src` with its
own hosts (`static-pages.ts` `connectSources`): `'self'`, its `VITE_SUPABASE_URL` and `wss://` on
its own host, since the server shares the site's origin. Beyond those the CSP lets in only Google
Identity Services, Google's own sign-in button on `/login` (R2983000): its script
(`https://accounts.google.com/gsi/client`) and, under `https://accounts.google.com/gsi/`, its frames,
styles and requests (`connect-src` names it beside Supabase). `scripts/check-production-bundle.mjs`
(`--staging` for staging's) refuses a bundle that would deploy without its settings or that CSP.

The tests share their support in `src/test/`: `setup.ts` patches jsdom's gaps (`matchMedia`,
`scrollTo` and a `PointerEvent` that keeps `pointerType`) and loads the engine before any test file;
`fixtures.ts` holds the fixture `PlayerView`s; `stubServer.ts` the stubbed `fetch`, `must` and sign-in;
`settle.ts` the one wait for the page to settle, whose depth (`ticks`, `macrotasks`) and fake-timer
mode (`advance`, `runAllTimers`) are named options; `history.ts` the browser's Back and Forward
(`pressBack`, `pressForward`), which need real timers; `seedSearch.ts` `seedWhere`, which finds a
test's seed at run time instead of pinning one a card change re-deals (#2569); and `board.tsx` `withCatalog` and `renderBoard`,
the board mounted inside the shipped catalog. `src/test/helpers.test.ts` fails any test file that
defines its own `settle`, `withCatalog`, `renderBoard` or `ensurePointerEvent` (#2215).

`scripts/github-releases.ts` prints every shipped version's GitHub release as JSON (`src/patches/github.ts`:
the Patch notes page as Markdown, a pre-release while only staging has the version), which
`.github/workflows/releases.yml` publishes. It loads the WebAssembly module from `src/wasm/pkg` (a card's
words are diffed by the face model, #3161), so run `sh scripts/build-wasm.sh` first, as the workflow does
(#3601); `src/patches/github.test.ts` runs it in a process of its own. To read the newest one's notes:
`pnpm --dir apps/web exec tsx scripts/github-releases.ts | jq -r '.releases[-1].body'`.

The dev hotseat route is `/dev/hotseat?seed=42&a=first20&b=first20`. It runs `reduce` in the
browser and exposes `window.__jackioh = { state, dispatch, seed, … }` whenever
`import.meta.env.MODE !== "production"`, which is what the Cypress specs drive (BUILD M5-T3).

`/dev/fx` (`routes/dev/fx.tsx`, development only, as `/dev/hotseat`) is the card-animation gallery
(R2784000, `docs/polish/9-card-animations.md`): over the fixture board it plays any drawn motif
(`fx/motifs/`) in any flavor's palette or sub-look (#2792) at any gallery scale, and every entry of its set's
effects table (`fx/cards/<set>.json5`, #2793, #2796, #2798, #2799, R2794000) other than `none` (`galleryPlayOf`): a cast,
a cry, a card recipe or Flourish (over its fixture, `RECIPE_FIXTURES` in `fx/motifs/gallery.ts`), a set piece or an aura
plays as the card's play, a death, trigger or activate motif from its hook's `hookResolving` marker (R2797000), a Field
card's aura stands in the first backrow zone, and a marquee Unit's entrance plays as it lands. It also lists every card
whose entry is its signature by set (R3332000), each played over the board its entrance reads (Felinor Fiender's over
three Felinors, R3332001), a signature Unit standing in its entrance's zone wearing its tile aura (Classic+'s, R3399001);
Stinkyscape part two's 23 (R3407000) each play on their module's own fixture (`fx/cards/stinkyscape/`), a hook's marker
naming the card it plays. It calls no engine.

To look at the client rather than test it (a layout, a card face, the phone view, a board mid-game),
an agent drives a headless browser with `sh scripts/browse.sh` (#811): Microsoft's Playwright CLI,
installed on first use into `.cache/` and named in no manifest, so no one else pays for it.
`.claude/skills/ui-walkthrough/SKILL.md` is the walkthrough: serving the client, the routes, setting a
board up through `window.__jackioh`, waiting for animations, the viewports and the screenshots. It is
for work on what a player sees, and nothing else.

Replays (SPEC §9.3, §9.9, R768, R768000) open from three places: `/replays` (`routes/replays.tsx`) lists
the practice games kept on the device and, signed in, the account's online games a page at a time (an account
that owes the age check or the terms meets the consent screen there first, R970000); the
practice result dialog and an online match's result panel offer "Watch the replay" (step 0) and "Share the
last turn" (the last step, its turn's GIF already being made). A practice result reads the kept game from
the worker's final snapshot (`replay`) and asks its pages through the practice controller's queue; a
match result loads its first page to learn the replay's length (`OpenedReplay`, `openOnlineMatchReplay`).
The GIF code still loads on demand, never in the main bundle.
Each `/replays` row that can be watched also offers "Download JSON" (R1141000): every step's view through the
row's own source, the row's facts and the build's catalog version, no account named, made in memory and saved
to the browser's downloads, for a bug report.
A finished co-op game's result panel offers "Watch the replay" too (§13.9): `duos/DuosReplay.tsx` reads
`GET /api/duos/matches/:matchId/replay` once and steps the seat's `DuosView`s on `DuosBoard`'s replay mode,
with the player's controls, a round standing for a turn; it is never in `/replays`.
The panel takes the focus on its first way on as it comes in; its "View the board" folds it to a "Result" button on the board's rail (`duos-result-reopen`) that opens it again, the replay door and the XP reveal with it (#1755).
A loaded co-op replay offers "Download JSON" too (R1242000, `duos/download.ts`): the seat's views as one file, made in the page, which `/replays` opens again from its Open replay JSON.

```
src/replay/
  source.ts       ReplaySource (a total and pages of steps), `memoryReplaySource` over verified steps, ReplayFailure
                  and the sentence each reason shows
  sources.ts      the two sources: the practice worker's `replay` requests, the server's replay routes
  steps.ts        the pages fetched so far, by step; load, prefetch, and the turn moves' search; a clip's ends
  ReplayPlayer.tsx  Game on a step's view with no legal action (`replay`), the seat's own open prompt read-only and folded (`replayPrompt`, R2211000), the controls in its notice slot;
                  `start` and `shareOnOpen` for a result screen's two buttons (R768000)
  ReplayToolbar.tsx  the controls both replays share (#2025): step and turn buttons, Play, the scrubber (the board
                  moves when it is let go), the readout and an actions slot; `replayTestid`, `isTyping`
  OpenedReplay.tsx  a source loaded first (a finished match's first page), then the player
  ShareGif.tsx    "Share as GIF" (#513, R513000): the clip, the shape, progress, cancel, Share or Download
  download.ts     "Download JSON" (#1141, R1141000): every step of a source and the row's facts as one file, and
                  `parseReplayJsonFile` (`readReplayJsonFile`, then `replayStepsOf`) opens one again (#1160);
                  the download both it and the GIF's share.ts hand a file to
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
routes/decks.tsx        /decks: loads GET /api/decks and the collection, hands them and the shipped catalog to the workshop; Back leads to the Collection hub (R1579000);
                        takes the gate's account and reads none of its own (R3207003)
game/deckbuilder/       the deck workshop: up to ten named decks and five trios (R250, R252), the pool
                        browser (CardBrowser.tsx, which the Card Almanac renders read-only too, R630, its
                        grid mounting only the rows of its 700 cards near the view, poolWindow.ts, #2334),
                        the trio editor that marks every card two of its decks share (R251),
                        deck codes (deckCode.ts, R255), trio codes (trioCode.ts, built on deckCode.ts's
                        parts, R339) with Copy trio code in the trio editor and Import trio
                        (TrioImportPanel.tsx, R340), Duplicate on each deck's row and in its editor (a
                        new deck with its cards, portrait and picks, named "<name> (copy)", #2096), and
                        autosave with a local mirror of unsaved edits (sync.ts, R256): one POST
                        /api/decks/sync a pass of at most `DECK_SYNC_ITEMS_MAX` items, decks, then trios,
                        then trio and deck deletions, each item judged alone (R3212001); a trio import is
                        one POST /api/trios/import, never the autosave (R341); the open deck's History
                        popover in the editor's head (DeckHistory.tsx, R2100000): the versions the server
                        kept, newest first, each with its date and the cards in and out since the one
                        before (deckHistory.ts), and Restore on all but the newest, one POST that the
                        server keeps as a new version, never the autosave; unsaved edits go first, and a
                        deck whose edit has not landed restores nothing;
                        Undo and Redo of the open deck's edits, each one step sent as any edit (history.ts, #2099);
                        a deck holding a card the newest patch changed wears a badge listing them until saved
                        under it (DeckPatchBadge.tsx, R2097000)
routes/play.tsx         /play, the Play hub (R1578000): one screen that does not scroll, online play first and
                        largest, Practice, Play together and Replays as three smaller links (R1646000)
                        beside it, and for a visitor with no account (`token` null) a Sign in in the online
                        panel and no read at all; the online panel is the mode picker (Best of 1, Conquest, All Random, R257, R330), the deck or
                        trio choice with the validator's verdict as UX, the queue and the room code; it
                        waits on /api/auth/me's currentMatchId and currentSeriesId; it shows the player's
                        own rank on the ladder of the mode picked and links that ladder's /leaderboard
                        (R661, R2778000); a queue it joined is remembered for the
                        tab, so the player may leave while they wait and it picks the wait up again (R765);
                        a room's ticket shares it as an invite link, and a lobby opened on one fills in
                        its join form (R767); a queue the server no longer holds ends the wait with a
                        rejoin message (R897000); it takes the gate's account (`me`) and sends no
                        /api/auth/me read at mount for a fresh one that names no game, reads the queue readout on
                        mount and then every QUEUE_POPULATION_POLL_SECONDS while it waits, its account
                        read keeping SERIES_POLL_SECONDS for the ticket (R3207000, R3207003)
net/roomLink.ts         R767: the room link (/play?room=…&mode=…): built for the share sheet or the
                        clipboard, read once off the address bar into this tab, through a sign-in, for
                        the lobby
net/spending.ts         R2378000: whether one of the tab's spends is out, from its press until its answer or
                        its failure (`net/api.ts`'s spends run through `whileSpending`); the Shop's Buy and
                        the pack panel's Open, Open all, sealed pack, Reveal all and Next pack and the
                        cosmetic pack panel's Open and chase Buy (R2683000) wait on it, Buy, Open and Open
                        all turning economy/spend.css's wheel, and a 429 keeps its press's key
net/liveGame.ts         R765: the player's online game, read off /api/auth/me: the queue this tab joined,
                        which main.tsx's useQueueFollow follows on every screen but /play ("Match found!",
                        then the game), and useLiveGame, the live match, series or co-op game the menus'
                        banner offers (R1566000), read every LIVE_GAME_POLL_SECONDS (R3207001); an ended
                        queue is forgotten after QUEUE_ENDED_AFTER_READS reads (R897000)
routes/GameBanner.tsx   R765: the one banner atop a menu while there is a game to go back to: a saved
                        practice game (Resume, on /practice) and the live online or co-op game (Rejoin,
                        on / and /practice); game-banner.css draws it as the tavern's notice
routes/series.tsx       /series/:id: a Conquest series (R330–R336): score, both sides' won (locked) decks,
                        the history, forfeit between games, the result
routes/SeriesPicker.tsx the deck-selection phase before each game, laid out as the mulligan (R331–R333,
                        R338): choose a deck that has not won, lock it in, wait sealed; one clock
routes/SeriesBanner.tsx the board's banner for a series game, each side's won decks as pips, and its
                        "Continue" once the game is over; while the match socket is open it takes the
                        socket's `series` frames and reads nothing after the result (R3213000)
routes/leaderboard.tsx  /leaderboard: the global ranked ladder (R608, R612) — Jlorious #1–#100, the
                        Grape tiers, the Raisins — and the caller's own standing, on Constructed or All
                        Random by its switch (`?ladder=random`, R2778000); rank/rank.ts says
                        a visible rank in a player's words and rank/RankCrest.tsx draws it as a crest
                        (#1650), for this screen, the account, Play and the board; rank/StreakFlame.tsx
                        draws a ranked win streak as a flame on the account and a ranked match's
                        line (R2101000)
routes/collection.tsx   /collection (#1307, #1579, R1579000): the hub of three doors, Decks (/decks) as a
                        link, Cards and Cosmetics opened in place by
                        `?section=` (net/navigate.ts `useSearch`) so the browser's Back and Forward move
                        between them and the hub; the page never scrolls, a section's body does. Cards is
                        the account's cards per shipped set, wallet, set tracks and quests, each panel
                        reading for itself and saying why when its read fails, each track a filling bar
                        (economy/FillBar.tsx); Cosmetics is where one is equipped (and a pack reveal's Equip,
                        collection/EquipButton.tsx, R1955000): the playmat, backdrop and card back as preview
                        tiles (collection/AccountCosmeticTiles.tsx), on the same store and `cosmetics` settings
                        group as before (R633), with no list of what the account owns (R2731000) and no
                        defaults of every card, so a card with no pick
                        of its own wears its kind's default (R1953001); a card of the Cards section opens large
                        wearing its own picks
                        (collection/CardCosmeticsView.tsx, R1655000), over a strip of the foils, finishes
                        and borders it may wear drawn as live previews and, R3015007, its variants as ladders
                        of their levels (collection/VariantLadders.tsx, `cardLadders`: the Default first,
                        each rung owned, next or locked and naming what it needs; a full-art card's retired
                        tiers are listed nowhere, the levels they map to standing for them), which equip on
                        that card alone, one level a card (`writeCardLevel`), kept on the device
                        (`jackioh.cosmetics.cards.v1`, cards/cosmetics.ts) and worn where the ledger says
                        the account owns them (cards/ownedFullArt.ts); while ownership decides, the tiles
                        and the deck builder's slot picker offer what the account owns, and an unowned
                        stored pick draws as the default (R1833000); holding a tile, a strip item, a rung or a
                        card, or its inspect key, opens it full screen (collection/ItemPreview.tsx,
                        R2730000), a grid card as the card, in hand and on the board with a Radiant
                        toggle (collection/LookViews.tsx, R3016000), and a mouse resting on a grid card,
                        or the keyboard's focus, opens the same views beside it (collection/LookHover.tsx);
                        Equip or buy on either opens the card's equip and buy screen
                        (collection/LooksScreen.tsx, R3016001): its ladders, Equip on an owned level, Buy on
                        the next with a held token or the full Chase Shard bar after a question, and what a
                        later level needs; a rung hovered there or in the card view shows that level.
                        Only there does a locked item a held token covers offer the token (of a
                        ladder, its next rung alone, R3015004)
                        (collection/TokenOffer.tsx): a Rare, Epic or Legendary cosmetic token through
                        POST /api/tokens/redeem-cosmetic (R1953001, R2728000), or a card token through
                        POST /api/tokens/redeem-card. The offer asks first (R2480000), and ownership or
                        the collection is read again. An owned card's view also shows its proficiency
                        (collection/ProficiencyPanel.tsx, §14.14, R3013001): its level, its plays to the
                        next on serverConfig.ts's curve and a picker of its stamps, a locked one with the
                        level it needs; the account's copy is cards/stamps.ts's, loaded and each pick sent
                        by cards/proficiencySync.ts (GET /api/proficiency, PUT /api/proficiency/stamp), and
                        the faces wear the pick (R3013002);
                        collection/
                        holds the panels (OwnedCards, WalletPanel, TracksPanel), their
                        model and CosmeticLook.tsx, a cosmetic's look in the reveals and the shop's rows;
                        every one renders the server's answers and decides nothing (rule 7)
routes/QuestsPanel.tsx  /collection's Cards section's quests (§14.9) from GET /api/quests: each held quest's progress and
                        reward, and the countdown to the next; moved from /account (#1307), and Replace on each, behind
                        a confirm, while a replace is left today (R2210000). The landing's bookmark
                        shares `useQuests` and `QuestsView` (R2035000)
routes/NewsPanel.tsx    the landing's news (§14.13, #2875) from GET /api/news: the live entries, newest first (R2875001),
                        each with its reward in words (R2875003) and Claim, POST /api/news/:id/claim, a spend sent
                        once (R2875000); the bookmark's count is the entries the server says are unclaimed (R2875002)
routes/shop.tsx         /shop (#1577, #1580, R1580000): the four panels, each folding the answer of a spend (a buy's, an
                        opening's) that it shows, and the page reading the rest once, when the opening scene closes
                        (R3211000), and at its foot one "Odds & pity" link to /purchases (R1665000); the scene's close
                        has the open screen and the next read the owned cosmetics again; the page never scrolls, its body does
routes/purchases.tsx    /purchases (#1665, R1665000): the Purchases disclaimer, public, linked from the site
                        footer (routes/SiteFooter.tsx) and the foot of /shop; with a session, the odds tables
                        (card packs, duplicates with the global roll's column (R1661001), cosmetic packs) GET
                        /api/packs and GET /api/cosmetic-packs state, and the pity table with the account's
                        counters, shown as the server answers them (a zero rate as a dash); a visitor sees the
                        disclaimer and a link to sign in
economy/                #1651: the economy's shared pieces: `wallet.ts`, the balance the Jlore pill shows
                        (GET /api/wallet, or the balance a later answer carries, `announceWallet`; never a
                        client sum); `JlorePill.tsx`, the pill in routes/nav.tsx's top bar and the
                        landing's, linking /shop (R1651000); `FillBar.tsx`, the bar that fills from before
                        to after, bursts when full and restarts with the overflow, each fill on its own
                        step's size where the answer gives one (R2728000); `RewardReveal.tsx`, a
                        filled bar's reward flown out one at a time; `OpeningBars.tsx`, the opening's
                        token bars pinned at the top of the screen, each slot's points (`slotPoints`)
                        flying from its card into them and a full bar pulsing before its reward
                        (R1953000, R1999000); `XpReveal.tsx` and `xp.ts`, a finished game's tracks,
                        level by level with each level's rewards, and its quests, from the server's
                        `economy` answer (the match and Duos results, a paid practice game's); `bars.ts`
                        the pure stages; Reduce motion (the setting or the system's) shows every end state
                        at once, and the effects' intensity off flies and pulses nothing; `spend.css`, the
                        loading wheel a spend button turns while one of the tab's spends is out (R2378000)
routes/lobby.css        the lobby's, the series screen's and its picker's tavern look, and the banner's pips
routes/DevTools.tsx     R1300000: a dev account's own tools on /account (XP, Jlore, fill, the clears, give or
                        complete a quest, §14.9), shown
                        when /api/auth/me says isDev; the server decides every grant, and the panel shows
                        its answer or refusal; /account's "Manage sign-in & account" shows every account
                        its account secret ID, hidden until asked for, with a copy button
packs/                  #1308: the card packs on /shop (R1580000, SPEC §14.3, §14.4; `PackOpening.tsx`, imported
                        by routes/shop.tsx): the packs held, each a physical pack (`PackObject.tsx`, #1663: a
                        foil wrapper with crimped seals, its set's emblem and colors, a sheen and a tilt
                        that follow the pointer, a press and a tear, the shelf and the scene drawing the
                        same object), and the scene that opens one (`POST /api/packs/open`; Open all is one
                        `POST /api/packs/open-all` of up to `PACKS_OPEN_ALL_MAX`, shown a pack at a time with
                        nothing more sent, R3212000), its token bars
                        pinned at the top from the answer and played once every card shows
                        (economy/OpeningBars.tsx, R1953000); the pity counters and the odds are on
                        /purchases, not here (R1665000); the scene: a tear or swipe, the best rarity tinting the
                        glow, cards turned one at a time (Epic and above on a hold), what each became, a
                        stinger and fx per rarity, skip-all, batch opening, Reduce motion showing the results
                        at once, and a new cosmetic's Equip (R1955000); the server decided every slot before anything animates (`packs.ts` holds the
                        pure model and the test ids, `fixtures.ts` the fixture responses, `packs.css` the look)
shop/                   #1309: spending what the economy pays, on /shop (R1580000) beside the opening
                        (`Shop.tsx`, `TokenPicker.tsx`, `CosmeticPacks.tsx`, imported by
                        routes/shop.tsx): the Jlore shop buys set and global packs at
                        `GET /api/packs`' price (`POST /api/packs/buy`), each shelved as a PackObject that
                        flies into the held count when bought, the button disabled with the shortfall shown
                        when the wallet (`GET /api/wallet`) is short; the token picker shows both token
                        bars as filling bars and the cycle's next step (`GET /api/tokens`),
                        the tokens held, with a link to the Collection, where each is spent from a locked
                        item's preview (R1953001, R2730000); the
                        cosmetic packs open through a simple reveal (`POST
                        /api/cosmetic-packs/open`), with the Chase Shard bar (`GET /api/cosmetic-packs`)
                        with a new cosmetic's Equip (R1955000), filling from the opening's `shardBefore` and, only when full, its reward moment: the
                        Legendary-or-Mythic choice (`POST /api/cosmetic-packs/chase`), whose Buy asks
                        inline first, "Buy Galaxy? This spends your full Chase Shard bar.", Cancel
                        focused, Escape answering Cancel, and sends once on Confirm (R2683000); every
                        refusal shows the server's reason through `t()` (R711000) and changes nothing; a buy or an opening
                        that lands hands the page its answer (`onChanged`), which folds it into the panels that show it
                        (`bought`, `opened`, `opening`), and the page reads what no answer carries once, when the
                        opening scene closes (`onClosed`, `settled`, `revision`) (R3211000) (`shop.ts` holds the
                        pure model and the test ids, `fixtures.ts` the fixture responses, `shop.css` the look)
```

A practice game offers the account's complete saved decks by name, read from `GET /api/decks`.

**A deletion is undone, not asked** (R2119000, `undo/`). Delete deck, Delete trio and
Remove on a friend's row take the item off the screen at once and show a toast, "Deck
deleted · Undo" and the like, for `UNDO_WINDOW_MS` (`undo/hold.ts`'s `useUndoHold`, drawn by
`UndoToast.tsx`: a status that takes no focus, inside the safe-area insets, with no slide under Reduce
motion). The request waits for the window to close and goes at once when a second deletion takes the
toast, when the page hides (the pass's sync as a `keepalive` request, `net/api.ts`'s `SendOptions`) or when the screen
goes away; the workshop also sends it before a new deck or trio, a duplicate or an import, since the item
counts against the server's caps until then.
A deletion within `UNDO_REPEAT_MS` of the one taken is a double click's second press: `hold` refuses it
(returns `false`) and every caller leaves the item in place and stays where it is (R3059000).
Undo puts the item back and sends nothing. A refusal puts it back in the server's words; a deck or
trio whose deletion could not reach the server stays deleted and is retried (R256). While a deck's or
trio's deletion is held, `sync.ts`'s `hide` keeps it out of every list, every trio slot and every save,
and `unhide` (Undo) brings it and its trio slots back. Block and Concede still ask.

## Play together and Duos

Play together (SPEC §9.4, §9.5) and Duos co-op (SPEC §13) are screens over the server's answers and
views, and decide nothing: a friend's status, whether they may be challenged or watched, a chat
message's mask and every refusal come from the server, shown in its own words, and a Duos board's
legality comes from the `legal` array its socket sends (rule 7). Every screen here needs a signed-in,
active account, and asks for one the way `/play` does.

```
routes (net/navigate.ts paths, and public/_redirects):
  /together              together/TogetherRoute.tsx: friends with status dots, the requests in and out
                         (accept, decline, cancel), the blocked players (unblock), the add form by
                         name#tag; per friend Challenge (online and idle), Watch (when `watchable`),
                         Chat (always), Remove and Block; "Shared with me"; the co-op panel
  /watch/:matchId        together/WatchRoute.tsx: a friend's seat over /ws/watch, Game on a view with no
                         legal action, SPECTATE_DELAY_MS behind; its turn clock counts down silently from each frame's
                         `clockMs` (together/WatchClock.tsx); Reconnect now once its own reopens run out;
                         with &coop=1 (a friend whose `duos` is true) the co-op seat's views on DuosBoard,
                         drawn as a co-op replay step is (R2881000); the watcher's emotes from the
                         sidelines (R2990000–R2990002): the socket's `spectatorHand` is the hand the
                         sideline strip's Send an emote offers while it is open, a pick goes out as
                         `{type:"emote"}` and shows at once as You, and the other watchers' relays show by name
  /duos/:matchId         duos/DuosRoute.tsx: the networked co-op board over /ws/duos; a watcher's emote
                         (`spectatorEmote`) shows in its sideline strip, as on /match/:id
sideline/                R2990000, R2990004: the sideline strip (SidelineStrip.tsx, sideline.css), the watchers'
                         emotes at the screen's inline end (along its top on a phone), each with its watcher's
                         name, on /match/:id, /duos/:matchId and /watch/:matchId; useSideline.ts over
                         session.ts (a watcher's gate at the server's SPECTATOR_EMOTE_* numbers, R2990002, one
                         show per watcher), wire.ts (the frames' checks); its Mute lasts the match and the
                         Audio tab's Mute spectator emotes mutes every match's from its start
  /dev/duos              duos/DevDuosRoute.tsx: a four-seat hotseat Duos game over the WASM Duos
                         bindings, with a seat switcher (development only, as /dev/hotseat)
net/social.ts            fetch wrappers for every Play together route, typed (Friend, FriendRequest,
                         Challenge, ChatMessage, SharedReplay, PlayerRef); errors are ApiRequestError
net/duos.ts              fetch wrappers for the co-op routes, and the ping types (PingKind, PingTarget)
together/ChatPanel.tsx   a conversation, polled every CHAT_POLL_MS while open and the social socket is
                         down, and read on its friend's chat push while it is live (R3214000); a message holding
                         this site's room link draws a Join button that navigates in-app (R2094000);
                         a send shows at once, sends go one at a time, a failed one offers Retry and
                         Discard (R2120000)
net/roomLink.ts          R767's room link (/play?room=…&mode=…) and R2094000's co-op link
                         (/together?coop=…): built for the share sheet or the clipboard, read once off
                         the address bar into this tab, through a sign-in, for the lobby or the co-op panel
together/FriendsButton.tsx  the Friends button at the top right of every signed-in screen (the nav, the
                         landing and a board's controls) with its badge (incoming requests, challenges
                         and unread chats), and its panel: the player's name#tag, the add form, the
                         requests in, the friends online first with Challenge, Watch and Chat (the rows
                         of FriendRows.tsx, shared with /together) and a link to /together; the friends
                         list is read only while the panel is open, ChatPanel and ChallengeDialog open
                         from it without leaving the screen
together/socialPoll.ts   the one SOCIAL_POLL_MS read per tab (challenges and unread counts, one
                         GET /api/social, R3210000) the toast,
                         the button and /together share, and the badge's count (paused while the tab is
                         hidden unless a challenge of the player's is open, R3205000); it holds the tab's
                         social socket and sends no read while it is live, and hands its chat and
                         friends pushes to onChatPush and onFriendsPush (R3214000); a tab hidden for
                         HIDDEN_QUIET_MS lets the socket go and opens it again when it shows (R3576000)
together/socialSocket.ts  the tab's one /ws/social socket (R3214000): the token as a subprotocol
                         (R1379000), the server's social read and chat and friends nudges pushed over it,
                         the match socket's backoff and wake probe (R1441), a 4401 reopened with a renewed
                         token and a refusal final for its token (R1115000), a socket replaced once the
                         session holds another token, its frames never shown; off under vitest
                         unless a test installs a factory (test/fakeSocket.ts)
together/config.ts       the social reads' pace: the friends list every FRIENDS_POLL_MS (a heartbeat,
                         PRESENCE_HEARTBEAT_MS), the challenges and unread counts every SOCIAL_POLL_MS;
                         both paused while the tab is hidden; HIDDEN_QUIET_MS, how long a hidden tab
                         waits before it goes quiet (R3576000)
together/ChallengeToast.tsx  mounted once in the shell; reads the challenges (socialPoll.ts's GET /api/social) while signed in and
                         toasts an incoming challenge anywhere with a deck picker (the last deck used for
                         its mode): Accept joins the room with it and goes to the game (R1831000), a
                         refused join offers Open in lobby, /play with the room code filled in (R767);
                         Decline declines
together/presence.ts     usePresenceHeartbeat: POST /api/presence once PRESENCE_HEARTBEAT_MS passes with
                         no signed-in request (onSignedInRequest, R3210000), none while the tab has
                         been hidden for HIDDEN_QUIET_MS (R3576000)
duos/CoopPanel.tsx       create a co-op room (difficulty, a deck or a random one), show its code,
                         offer its link, invite a friend by sending the link in chat, join by code or by
                         a co-op link that fills the join form and focuses Join (R2094000); both players
                         go to /duos/:matchId once the guest joins
duos/DuosBoard.tsx       Game.tsx for the own duel, the life strip (both teams' own HP and pools, a
                         spent seat's bar giving way to its pool), the pass control, the portal, the
                         Both boards toggle (R2095000), the teammate strip and the ping wheel
duos/DuosReplay.tsx      a finished co-op game's replay (#965): the server's steps read once, stepped on
                         DuosBoard's `replay` mode (nothing live) with the replay toolbar (`replay/ReplayToolbar.tsx`)
duos/download.ts        a finished co-op replay's "Download JSON" (R1242000): the seat's views as one
                         file, and `parseDuosReplayJson` opening one again from `/replays`
```

The landing page offers three doors — Play (to the `/play` hub, R1578000), Collection and Shop (#1577) — with a quiet quests bookmark under them for an active account on a live economy (R2035000) and beside it the news' (R2875002), a red count of its unclaimed entries, none with no live entry (R2875001), and no door to Play together, which the hub links and which lives at `/together` behind the gate. The replays list and the replay player offer
"Share with a friend". The match board's rail says how many friends are watching while any are
(`match-watchers`, from the match socket's `watchers {count}` frame, which `game/net.ts` keeps as the
snapshot's `watchers`). While any modifier is live on either side, the rail carries `game/ModifierList.tsx`'s chip (`modifier-list`, its count in `data-count`), which opens every label in full under You and Opponent; on a phone the badges beside each hero are cut short, and each is a button (`game/ModifierBadge.tsx`) whose tap opens its own whole label beside it (`modifier-full-<id>`, #2321) and whose second tap, a tap elsewhere or focus leaving it closes it, with no key handler (R2126000). Practice and the lessons show the same chip in their HUD, under You and AI, the Duos board shows it for the seat's own duel, and the watch screen (the newest view) and the replay player (the step on the board) show it in their rail. A finished ladder match whose `view` frame says the daily Jlore cap held back the account's Jlore (`jloreCapped`, R1483000, kept by `game/net.ts` as the snapshot's `jloreCapped`, false when absent) shows "Daily Jlore reached" in its result panel (`result-daily-jlore`); the page works nothing out, and a seat is only ever told its own. The same frame carries what the result's write paid the seat (`economy`, kept by `game/net.ts`: each set track's XP before and after, each level it reached with its rewards, each quest it completed), which the result panel plays once it has come in (`economy/XpReveal.tsx` under `game/Result.tsx`'s `OnceRevealed`) and tells the Jlore pill of; the Duos result plays its own the same way. On the Duos board:

- the pass control on an inspected hand card sends `Pass` only when `legal` holds it, with the pass
  cost and the passes left beside it (SPEC §13.5); it is the only way to pass, and a hand card
  dragged onto the portal or the teammate's board passes nothing (#2619);
- the portal flips the board to the teammate's duel, read-only (`legal` empty, a no-op `onAction`),
  where cards can be hovered and inspected; the teammate strip always shows their own HP, mana, hand
  count and whether they have ended, and "Waiting for your teammate" shows once the seat has ended;
- the Both boards toggle (`duos-both`, `aria-pressed`) shows the teammate's duel beside the own one
  (R2095000): the second board is the portal's read-only board in `duos-board-teammate` next to
  `duos-board-own`, silent so it feeds no audio and draws no effects layer, with pings and inspection working on it and a hand
  card dragged onto it doing nothing. The choice is kept per device in
  `localStorage` (off until pressed); the portal stays mounted and CSS hides it on a wide
  screen while both boards show, and below 600px the toggle hides and the board falls back to one
  with the portal flipping: the rail rides whichever board the portal names, so the one drawn
  always carries it;
- the ping wheel on a hovered or inspected card offers the five pings; a received ping plays its
  sound, marks its card for `PING_SHOW_MS` on either view and flashes the portal when the card is on
  the other view (SPEC §13.10). A Duos board deals no emote hand and draws no aim arrow.
- on a live game's result, Play again (`duos-result-again`, R2092000) sends this seat's offer
  (`POST /api/duos/matches/:matchId/again`) with the device's "More cards from" pick, and
  `duos-result-again-status` reads `GET` every `SERIES_POLL_SECONDS`: waiting, the teammate wanting
  to play again, the teammate gone (the button hidden), or a refusal in the server's words. An
  answer naming the new match takes both people to `/duos/:matchId`. A replay and `/dev/duos` show
  no button.

The Duos WASM bindings (`duos_create`, `duos_begin`, `duos_reduce`, `duos_legal_actions`,
`duos_view_for`, JSON in and out like the 1v1 ones) are reached only through `src/wasm/index.ts`, for
`/dev/duos`; a networked co-op game's state stays on the server. Only the normal module carries them,
and only the dev server's build (`predev`'s `JACKIOH_WASM_DUOS=1`, crates/wasm's `duos` feature): in
any other build each throws the reason.

Test ids, mirrored in `e2e/support/testids.ts`: `together-add-input`,
`together-add-submit`, `together-friend-row`, `together-request-accept`, `together-request-decline`,
`together-challenge`, `together-watch`, `together-chat-open`, `friends-button`, `friends-badge`,
`friends-panel`, `challenge-toast-accept`, `chat-input`, `chat-send`, `chat-message`, `chat-join`, `coop-create`, `coop-difficulty`, `coop-code`, `duos-coop-link`, `coop-join-input`, `coop-join-submit`,
`duos-portal`, `duos-both`, `duos-board-own`, `duos-board-teammate`, `duos-pass`, `duos-pool`,
`duos-own-hp`, `duos-teammate-strip`, `duos-ping-wheel`, `duos-ping-<kind>`, `duos-ping-marker`,
`sideline-strip`, `sideline-emote`, `sideline-menu` and `sideline-mute`.

## Practice

`/practice` (SPEC §9.9, R187) is a game against the AI with no account and no server. The practice
session itself is Rust, `crates/shared`'s `host/practice` (#3099): the engine, the AI and that machine
(as WebAssembly) run in a Web Worker, which stands where SPEC §9.1 puts the server, so
rule 7 holds exactly as it does online. The worker's driver feeds the machine's events and carries out its
outputs; it decides no rule (how a game is dealt, the nonces and seats, what the AI may take, the save, the kept
games and the claim are the machine's, and the server's claim check steps the same machine natively):

```
src/practice/
  protocol.ts         the request/response shapes: everything the page ever learns about a game
  driver.ts           the ONLY practice file that holds the game (the machine's `PracticeHost`, with the
                      GameState inside it) and calls the AI's `decide` and `lethalHint`: it feeds the
                      machine its events through `wasm/index.ts`'s `practiceCall` and carries out the
                      outputs (the save store, the AI's search and the lethal check under their wall-clock
                      caps, the public catalog, a preset's cards and a lesson's decks put into the config)
  practice.worker.ts  the worker entry: driver.handle per message
  host.ts             a module Worker in a browser, the in-thread driver in jsdom; one answer per
                      request, strictly in order
  controller.ts       one request in flight, and the pacing loop that plays the AI's turn one
                      visible action at a time (think indicator, gaps from config.ts)
  aiTable.ts          the hooks that pace the AI at a table (controller state, board busy, the voice and
                      showcase holds), shared by the route and a dungeon fight's screen
  config.ts testids.ts PracticeSetup.tsx ThinkIndicator.tsx PracticeLeave.tsx PracticeResult.tsx practice.css
  decks.ts            random, the three named practice decks (hand-built lists), the account's saved decks by name
  lastBoard.ts        the human's last practice board (C+ #29, R417, R508) in localStorage
                      `jackioh.practice.lastBoard`, try/catch
  saveStore.ts        the worker's save of a free game in progress (R668): config, log, AI cursor,
                      catalog version, hash, in IndexedDB `jackioh.practice` (memory in jsdom), and the last
                      PRACTICE_REPLAYS_KEPT finished free games (R768)
  resume.ts           the page's half (R668): only the setup it chose, in localStorage
                      `jackioh.practice.game`, try/catch, marked `saved` once left with Save and leave (R765)
  todaysDeal.ts       today's deal (R2882000): the UTC day's seed daily-YYYY-MM-DD and seat, on a random deck
  DeckPreview.tsx     the chosen deck's name, identity, mana curve and cards, before Start
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
- In every game against the AI (each tier, a sandbox game, a lesson) the snapshot adds
  `lethal: true` while the human has a sure win this turn by what they can see (R2102000, R2291000):
  the machine asks (a `lethal` output) and the driver runs `lethalHint(state, human, deadlineMs)`, which the Rust AI answers from the human's own
  redaction, so the AI's hidden cards never change it, under a wall-clock cap of its own
  (`PRACTICE_LETHAL_HINT_CLOCK_MS`; past it, no hint). The route passes it to `Game`'s `lethalHint`
  while the Lethal hint setting is on, and the board shows it (`data-lethal-hint` on the game, which
  outlines the enemy hero and End turn, and the `lethal-hint` line "You have lethal") once it shows
  the newest view, never on a replay. Online and hotseat boards never set it.
- Every snapshot adds `forecast` (R3477000), the engine's `combatForecast(state, human)` (R3100000): what
  each of the human's legal attacks would do, as the board shows it, and whether the attackers that reach the
  enemy hero finish it. It reads nothing the human cannot see, and the tutorial's coach names its attacks by it
  (`tutorial/advice.ts`), so the client does no combat maths. Outside a lesson the board draws it too: while
  an attacker is selected or dragged, each legal target with an entry wears a label (`game/AttackOutcome.tsx`,
  `outcome-<target testid>`) with the damage dealt, the damage taken back, a skull on whichever dies and the
  struck-through shield mark for a broken Divine Shield; the Show attack outcomes setting (`attackOutcomes`,
  on by default) hides it. A seat's 1v1 and Duos `view` frames carry the same field (`net.ts`,
  `duos/socket.ts`), and a watcher's never does.
- A claim's answer (`net/practice.ts`) carries `jlore` as the server paid it and `jloreCapped` (R1483000): when the daily Jlore cap held some back, the result dialog's line keeps the paid amounts and adds "Daily Jlore reached" (`practice/earning.ts`). It also carries the `tracks` the game moved and the `quests` it completed, which the result dialog plays (`economy/XpReveal.tsx`). A seed the server refuses (the account holds `PRACTICE_SEEDS_OPEN_MAX` open ones) leaves the game played as before, earning nothing.
- A question sequence's 60 seconds (Meditative+ #20 KY's Competition, R1563006) are the machine's: an answer carries `timers`
  (`PracticeTimer`, a delay in ms from the answer), the controller keeps each as a timeout and sends `timerFired` when it is
  due, and the machine answers Stop for the human. The prompt's countdown (`game/sequenceClock.ts`) only shows the seconds left.
- A game on a server seed decides under `PRACTICE_AI_CLOCK_MS` like every practice game: the server folds a claim's log and re-runs no AI move (R2560000, superseding R2208000).
- A free game in progress survives a reload (R668). After every answer the machine's `save` output is written
  to the worker's store by the driver, which never crosses to the page (the log names the AI's hidden cards); the
  controller remembers the setup when the game is dealt and forgets it when the game ends, and the
  route forgets it when the player leaves. A visit with no parameter that starts a game sends
  `{ type: "resume", config }`, and the machine folds the log back under the same nonces to the saved
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
  voice nor the showcase. A dungeon fight's `FightScreen` holds the AI the same way, through
  `practice/aiTable.ts`; it has no `?pace=fast`. The controller's general form is `setHold(reason, held)`. The settings panel's "Reduce motion" gives the reduced pacing,
  as the media query does.
- A lesson in progress asks before a reload or a closed tab ends it (`beforeunload`), and the HUD's
  Menu leaves for the landing page, asking first (Save and leave, or Leave without saving) while the
  game is on.

## The tutorial

The practice lobby opens with the tutorial (SPEC §9.10, R290–R294, R320–R322, R2205000–R2205003,
R2434000–R2434003): five lessons on a path, each about one thing and each a practice game with two fixed
decks (the human's Training Grounds cards, the AI's Core cards), a fixed seed and seat, the tutorial
handicap on the AI seat (`AI_TUTORIAL`, the tier below Easy), and `TUTORIAL_HUMAN` on the human's, with
the pool hold on both in lessons 1–4 (`lessonHandicaps`). Lessons 2–4 had other ids before #2434:
`EARLIER_LESSON_IDS` reads `spells`, `traps` and `advanced` as the lesson in their place (R2434000). A lesson is a practice config whose `lesson` names it
(`tutorial/start.ts`), so the same worker, controller and board play it; the worker reads the
decks from `tutorial/lessons/<id>.ts` and nothing else the page sends changes them (R291).

```
src/tutorial/
  lessons.ts lessons/*.ts   each lesson as data: title, its Field Guide entries, seed, seat, both decks, pool hold, retry tip
  scripts/*.ts              each lesson's coach script: its moments, built with scripts/kit.ts (page only);
                            scripts/testkit.ts's lessonSuite is every lesson's test
  coach.ts                  the coach machine, pure: coachObserve per snapshot, ack, hint, display (R292, R314,
                            R2205000: moments, discoveries, escalating hints)
  discoveries.ts            the Field Guide's entries: each mechanic, its rule, and how the view shows it found
  steps.ts targets.ts       reads of the view and legal actions, step factories, anchors → board testids
  advice.ts                 the coach's own line (the next play, trade, hit or End turn), picked among the
                            engine's combat forecast of each legal attack (R3100000)
  tracker.ts                feeds every controller snapshot to the coach and holds the AI for `holdAi`
  Coach.tsx layout.ts       the ring round what it points at, and the bubble docked to a corner, never the middle of the
                            board (R2434003; a panel above the board on phones)
  TutorialPath.tsx          the lesson path at the top of the lobby (locked / open / done), with Hide / Show
                            tutorial (R322; its look in path-visibility.css)
  TutorialHud.tsx TutorialResult.tsx   the lesson's HUD (step counter, Field Guide, Exit tutorial) and its result
                            dialog, which names what the game did not show found (R314)
  FieldGuide.tsx            the Field Guide's button and dialog, from the HUD and the path's header (R2205002)
  progress.ts               completed lessons, the Hide/Show choice, and the Field Guide's found entries and the
                            lessons lost by a concede (the device's alone, R2205002, R2434002) in localStorage
                            `jackioh.tutorial.v1`, try/catch (R294),
                            and R321's merge with another copy (a union, the newest choice)
  accountSync.ts            an active account's copy (`GET`/`PUT /api/tutorial`, R320) kept level with the
                            device's (R321); the practice route mounts it
  harness.ts                test support only: a lesson played through the real core by a policy: coach,
                            coach-passive, autopilot, random or reckless (R293, R2205001)
  config.ts testids.ts devHandle.ts   the numbers, the testids (mirrored in e2e), `window.__jackiohTutorial`
```

- The coach reads the snapshot the page already holds (`view`, `legal`, `aiToAct`, and a lesson's
  engine `forecast` of its attacks, R3100000) and nothing else, and never sends an action (rule 7).
  `advice.ts` names the coach's own line by that forecast and computes no damage, Armor or shield itself. A step points at a board element by its testid and
  completes when the view shows it done. There is no Skip step, and nothing needs one (R314): only
  a tip or an `info` step holds the AI, and each shows "Got it"; an action step never holds it; a
  step expires after `TUTORIAL_STEP_TURNS_MAX` of the player's own turns (the lesson's `final` step
  ends with the game); and Exit tutorial is in the HUD throughout. Nothing strands a lesson.
- A `moment` step (R2205000) asks a question with nothing pointed at, and is done once the view
  shows the player found what it sets up (`discoveries.ts`, read from the fresh events, the view
  and the public catalog). Its hint escalates a level per own turn it stays unfound, or per Hint
  press: the nudge, then the pointed hint with its ring; past `TUTORIAL_MOMENT_TURNS_MAX` it
  retires. It never holds the AI. Each entry found for the first time shows a discovery bubble with
  Got it, unless a later lesson holds it (R2434001), and the tracker hands it to the device's Field
  Guide (`markFound`).
- A concede is the one way to lose a lesson (R2434002): the result says "You conceded. That counts
  as a loss.", and the path marks the lesson lost until it is won.
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
  A Legendary's or Mythic's intro is also its play sting (R3332002): with dynamic music off or the music at zero, its
  first `MUSIC_STING_S` play on the effects bus instead, and the shared `entrance` sting plays only for one with no intro.
- A card's intro (R1352) is a score in `tracks.mjs`'s `INTRO_CARDS`: the card's facts as the catalog printed them and
  its `tune`, from which `introSpec` derives the rest (key, tempo, meter, motif and answer from the id; instruments,
  accompaniment and modes from the family; the set's glint; a Mythic's shimmer). R2722000 makes it an orchestral
  fanfare: a riser into the first downbeat, a Mythic's own motif (brass and choir, picked by its id), the melody under
  brass, tremolo strings, trombone and an orchestra hit, and an ending held to the bar line, all FluidR3's (no other
  SoundFont). A Legendary's file runs at most 9 s and every Mythic's longer, to 12 s at most. It renders as a sting
  with a 1.2 s faded tail (`tail`, `post.fadeOutS`), its loudness measured over the file it keeps
  (`post.loudnessOverFile`); a track without those fields renders exactly as before. A Legendary or Mythic added to a
  set that ships needs its `INTRO_CARDS` row, its `intro` in `music-cards.json`, a render and a license row:
  `music-assets.test.ts` refuses a shipped one without them.

## Regenerating the voice lines

Every card's sounds are in `src/audio/card-audio.json5` (R655), a JSON5 file edited by hand, whose
header explains it: a `voices` bank, an `effects` bank (a procedural recipe from `sfx.ts` at its own
`pitch` and `gain`, which renders nothing), and `cards`, in catalog order with each card's name in a
comment beside its id, where each hook (`play`, `attack`, `death`, `cast`, `trigger`; `CARD_HOOKS` in
`constants.ts`) gives a voice line (its `voice`), an `effect`, or both — a `trigger` (R1088) gives
a voice for a non-empty list of lines, one of which plays when the runner starts the `summoned`
entry the card caused, and an `emotes` section gives each portrait's voice (R644). The words the
lines speak are not in it: they are each card's `voice` and the portraits' `emotes` in
`crates/cards/card-text.yaml` (#2498), which `cargo test -p jackioh-tools --test export_card_text`
writes into `src/audio/voice-text.json`. `voiceData.ts` joins the two (`withVoiceText`), checks them
at load and names the path of any mistake. The voice files are generated from its lines and
committed, so CI never runs a synthesizer. Each voice names its backend (R501): Core's voices are
macOS `say` (the default), and the Classic and Classic+ voices are `backend: "sapi"`, Windows SAPI
(`System.Speech`, the "Microsoft David Desktop" and "Microsoft Zira Desktop" voices) shaped by
`ffmpeg`: trimmed, pitched by the voice's `semitones`, colored by its `filter` chain, peak-normalized
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
  `test/setup.ts` loads it synchronously for jsdom. A card's face is worked out by the module (#3106),
  so a screen that does not wait for it (the landing, the shop, `/replays`) draws a face only through
  `useEngineReady` or `WhenEngineReady` (`src/wasm/`), the way the landing's fan, "Your table" showcase and
  the shop's pack reveal do; `routes/engine-gate.test.tsx` holds the landing to it with a played game's stats.
- `src/routes/chunks.tsx` is the one map `main.tsx` draws its screens from (#2116, R2116000):
  `watchRouteIntent` listens for pointer, focus and touch intent on links and `data-route`
  buttons and prefetches each route's code once per page load, and the Practice, Play and match
  screens also call `loadWasm()`; Save-Data turns it all off, a production build has no dev
  chunk, and a screen whose code is already in shows no fallback.
- `src/game/currentCatalog.ts` is the current catalog every screen with no game of its own reads (SPEC
  §9.4, R163, #1378): `currentCatalog()`, the module's registered definitions and compiled-in version,
  and `currentCatalogLookup()`, the board's names over them, each built once. No screen asks
  `GET /api/catalog`; `game/currentCatalog.test.ts` fails a client file that names it.
- `src/wire/` keeps every import the client had when these packages were TypeScript working
  unchanged: `@jackioh/shared` is the generated wire types plus hand-kept helpers, `@jackioh/engine`
  and `@jackioh/ai` are typed functions over the module, `@jackioh/engine/config` and
  `@jackioh/server-config` are the generated Rust constants, `@jackioh/validator` the validator,
  `@jackioh/cards` the catalog and its version (read from `crates/cards/patches/patches.json`), and
  `@jackioh/cards/catalog.json`, `flavour.json`, `chinese.json` and `chinese-terms.json` the files in
  `crates/cards` (the second and third generated from its `card-text.yaml`, #2498, each with a `"//"`
  entry that `flavour.ts` and `chinese.ts` drop).
- The generated files (`src/wire/generated/`, `engineConfig.ts`, `serverConfig.ts`; among the first,
  `generated/cosmetics.ts` is the server's cosmetics registry, the card slots, the deck slots and the full arts, R1295000, R1297000, R1313000) are written by
  `cargo test --workspace --features jackioh-engine/testkit,jackioh-engine/ts`; CI fails when they
  differ from what is committed, so a Rust change that moves the wire commits them in the same change.
- Three files hold a game's state, and nothing else imports the engine's game functions: `game/engine.ts`
  (hotseat's port), `practice/driver.ts` (the practice worker's game, and the only caller of the AI's
  `decide`) and `duos/DevDuosRoute.tsx` (the Duos dev hotseat's `DuosState`, opaque, through the WASM
  Duos bindings, which only the normal module carries), with `tutorial/harness.ts` for tests.

## Player-facing words

Every word the client shows a player follows [`WRITING.md`](WRITING.md), the client's voice guide.
`src/writing.test.ts` holds what a machine can check of it: no word from its list in a sentence the
client shows (a string or JSX text in a source that `src/i18n/zh-CN.json` translates), no curly
apostrophe there or in a dictionary key, and no straight double quote there. Its glossary sets
sentence case, one name per page, one verb per action, and bans arrow characters in translations,
held by guards against Title Case labels outside their allowlist.

## Internationalisation

The client supports multiple reader languages (`src/i18n/`). Adding another language means adding one dictionary file (`src/i18n/<tag>.json`), its card table pair beside the catalog (`crates/cards/`), and one line in `LOCALES` (`src/i18n/i18n.ts`). What a card face prints is `jackioh-shared`'s (#3106), in the reader's words: its face and glossary strings are a dictionary there (`crates/shared/data/<tag>.json`, read through `Locale` in `crates/shared/src/face/`), so a language adds that file and its `Locale` variant too, and `cards/model.ts` passes the reader's tag with every call. The client draws the spans the face model returns (`cards/RulesText.tsx`) and words none of them. The game log's lines are `jackioh-shared`'s too (#3108, `crates/shared/src/log/`, with its sentences in the same dictionary file): `game/logLines.ts` asks the module for a line for each event of a view, and `game/Log.tsx` draws them and keeps the history of the lines it was shown (R745). The English strings remain inline wrapped in `t()`, and missing translations fall back cleanly to English. That fallback is for the engine's and the server's sentences: every literal sentence the client wraps in `t()` (a string, a template with no substitutions, or either branch of `t(cond ? "a" : "b")`) has an entry in `src/i18n/zh-CN.json`, an identity entry when it reads as written, and `src/i18n/i18n.test.ts` (R711000) fails on one that has none, naming its file and line.

The language button (`src/i18n/LanguageButton.tsx`) stays only until the device has settled on its language (R1916000): `localStorage["jackioh.locale.settledVisits"]` counts the boots, every language change and the settings' reset set it back to 0, and from the third unchanged boot the button leaves the top bar and the board while settings' Language row stays.
