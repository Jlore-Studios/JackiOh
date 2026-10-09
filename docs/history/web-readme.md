# History: `apps/web/README.md`

Text moved word for word out of `apps/web/README.md` by #603, under the heading it sat beneath there. That heading is still in `apps/web/README.md`, with a line pointing here.

## Layout

```
src/
  main.tsx              entry and the pathname switch
  index.css             reset and design tokens
  cards/                card faces, procedural art, inspect and card settings (docs/polish/6-cards.md). A face is
                        the card in play or the card as printed (SPEC §10.10): `faceModel` with `inPlay` reads
                        the view's own facts (a hand Unit's stats, a unit's keywords and Vanilla mark, a #98's
                        rolled power, R243, what a formula comes to now, R280) and inPlay.ts's words (#98's
                        power, ??? for Call to Chaos); with no `inPlay` it is the collection's printed card.
                        The inspect overlays in play show the printed text beside a face wherever the two
                        differ (inspect/Printed.tsx), and a card's flavour line and artist credit from
                        `@jackioh/cards/flavour.json` under the glossary (flavour.ts, inspect/Flavour.tsx,
                        R660). Real art follows art/ART.md, which art/convention.test.ts holds
                        public/art/ and art/manifest.ts to. RulesText draws every face's text with its marks: a
                        Radiant face's changes in gold (radiantDiff.ts, R277), the cards its `refs` name as
                        references (refs.ts, CardRef.tsx, refContext.tsx, R279; the hover preview's
                        "Mentions" column is inspect/References.tsx), and "{n}" values (R280). A card's marks
                        (R437, #50's pending steal) are read through marks.ts (`marksOf`, `markEventOf`, the
                        colour key → palette and mark → words tables) and drawn by CardMarks.tsx (marks.css):
                        a corruption aura in the mark's colours and a badge with its words, still under
                        reduced motion; the board's Card.tsx mounts it on units and face-up backrow cards
  game/
    engine.ts           the EnginePort: hotseat's only seam onto the engine, its functions the WASM
                        module's (../wasm/index.ts), synchronous once loadWasm() has run
    contract.ts         data-testid vocabulary, ClickTarget, Highlight, BoardProps
    catalog.ts          card names and rules text (see the §10.8 finding below), and `MatchCardsContext`: the
                        match-made definitions the view carries (`PlayerView.defs`, a Fuse's, R243) and each field
                        Heroic Power's rolled power, which Board, Prompt and DragLayer provide from their view
    Board.tsx Zone.tsx Card.tsx Hand.tsx Hero.tsx Backrow.tsx Log.tsx   M5-T1; a graveyard or exile pile that
                        holds cards (public on both seats, §10.8) opens its cards on hover and in a dialog on a
                        click (cards/inspect/CardList.tsx), and so does your own library, from the list without
                        order the view carries for it (`SideView.ownLibrary`, R310–R313): grouped with counts,
                        "Order hidden", unknown cards as backs; the opponent's library is a count. Players read
                        the rules' library as the Deck ("Deck", "Your deck") and Sacrifice as Tribute (R373;
                        src/wording.test.ts refuses the old words in any string a player can read). A log line
                        that names a card opens that card. A face-down backrow card is a back wearing the cost
                        the view gives it, whose hover and sheet say "Face-down trap" and its "Cost (N)"
                        (R370, cards/inspect/FaceDown.tsx); your own face-down trap, `unrevealed` in the view,
                        is its face under a dashed frame, a veil and a "Face down" tag with a struck-through
                        eye (R371, facedown.css, cards/faceDown.ts for the words); a grade badge prints the
                        letter the view names (R372). Every card with something to inspect takes keyboard focus,
                        and I, the context-menu key or Shift+F10 opens its sheet, as on the deck builder's tiles
                        (cards/inspect/keys.ts, #258); Enter and Space still play or pick only a legal one
    actions.ts Prompt.tsx                                               M5-T2
    ActivateControl.tsx activate.css   R384, R510: the Activate control on a card the viewer controls (see
                        "Patch v0.2.0 at the table")
    hotseat.ts decks.ts                                                 M5-T3
    animations.ts                                                       M5-T4
    useOsReducedMotion.ts  the OS reduced-motion query followed live: Game.tsx rebuilds the runner when it flips (#258)
    spent.ts spent.css  the "can't act yet" cue (#258): a unit of the player acting now (main phase, no prompt)
                        whose view says `canAct` false is dimmed and wears a "Zz" badge, in words for a screen
                        reader; drawn state, never permission (rule 7)
    Game.tsx            board + prompts + animation runner + effects layer + audio + drag layer + showcase, wired together
    ConfirmConcede.tsx  "Concede this game?": the Concede control only asks (see "Three flows at the table")
    DrawOffer.tsx       the draw offer's notices and the answering seat's Accept / Decline (same section)
    notices.css         the look of both: over the board, never taking height from it
    showcase/           the opponent's play held up beside the field for about a second (SHOWCASE_HOLD_MS over the
                        effects speed): plan.ts picks the opponent's `cardPlayed` out of the redacted events,
                        per viewer, and a card the view hides (R97, R227) is a back with "Opponent set a card".
                        A cast on draw is held up on both seats, longer, under a "Cast on draw!" ribbon, out of
                        its Deck pile, as the runner reaches it (R502); a Call to Chaos roll is said in words and,
                        where the effects layer draws nothing, shown still (ChaosBanner.tsx, R436).
                        Click-through, never on `data-animating`; `data-showcase` holds practice's AI while it is up
    OverflowNotices.tsx overflow.css   §2.4's overflows on the board's own elements (R318): "Fatigue N" and
                        "Deck full" (with the refused card) inside the deck pile, "Hand full" (with the burned
                        card) over a hand, a face or a back by R97. Read off the runner's entries like the damage
                        pops, up from their entry's start until the board catches up; animations.css moves them
    faces.ts            the face in play of a card the view lists or names (the board, a prompt, the showcase,
                        a log line, a pile): as it stands where the view lists it, else its definition
    inspectable.css     the look of what can be looked into: a browsable pile and a log line that names a card
    board.css prompt.css  layout and look: the game screen budgeted to the viewport (the
                        route's bar and the board share its height, and the cards are sized
                        off the board's with `cqh`), the board grid (a play area and sidebar
                        on desktops and landscape tablets, one column on portrait tablets and
                        phones held upright, sideways on phones held landscape), the playmat,
                        the hand fan, 44px touch targets, safe areas, the log behind a toggle
                        on phones, the prompt's bottom sheet on phones, and small pickers
                        docked clear of the field
    glow.ts             data-glow / data-condition-active helpers: green from
                        Highlight.glow, yellow from the view's conditionActive; and
                        data-countered-on-play, R667's Plague Chalice warning from the view's
                        counteredOnPlay
    highlights.css      the green and yellow glow colours, imported after board.css
    countered.css       R667: the warning's green bubbling film and badge, still under reduced motion
    drag/               drag to play: pointer events for mouse and touch, the targeting
                        arrow and reticle, and a dropped card held where it landed until
                        the board shows the play; a build lifted again from its picks, a
                        backrow card dropped on the board, a prompt option dragged out of its
                        panel (R658, OptionDrag.tsx); click-click keeps working in every mode
  audio/                sound (SPEC §10.11); index.ts is the barrel Game.tsx imports, appAudio.ts
                        the page-wide unlock and UI ticks main.tsx holds, mix.ts the buses and limiter
    engine.ts sfx.ts unlock.ts settings.ts   lazy AudioContext and buses, procedural SFX, gesture unlock, the settings store
    cues.ts director.ts useGameAudio.ts      SOUND_CUES (a total map over GameEventType) and the runner-synced director
    AudioToggle.tsx AudioControls.tsx        the mute button (in the board's control bar) and the full panel
    useVoiceSpeaking.ts                      the engine's `speaking()`, which Game marks as data-speaking
    usePickupSound.ts                        a Unit picked up to attack plays its attack hook (R655)
    card-audio.json5 voiceData.ts            every card's sounds (voices, effects, hooks; R655), hand-edited, and its parser
    voice-manifest.json                      the generated hash and size of each rendered line
    music.ts musicScene.ts                   the music player (bar-line crossfades, the turn mix, focus, a card's intro on
                                             top with the duck under it) and menu vs board (R631, R1350, R1351)
    musicDirector.ts musicPlan.ts            a board's music from the viewer's own view, and the priority stack
    musicData.ts music-manifest.json         the rendered tracks (loop points, tempo) and music-cards.json, the
    music-cards.json                         Mythic themes, shared Legendary entrance themes, station switches and
                                             every Legendary's and Mythic's intro (R1352) by card id
  fx/                   the effects layer (docs/polish/1-animations.md; SPEC §10.10, R200–R202)
    types.ts constants.ts   the cue contract and every FX number
    settings.ts         effects speed, intensity and motion (localStorage, jackioh.fx.v1)
    cues.ts memory.ts   the planner: an entry's events → cues, pure (and the killing blow a game over replays)
    stage.ts            stage cues, pure: a stand-in for a moved card, a hidden card, an aimed lunge (B46–B48)
    rng.ts presets.ts sprites.ts particles.ts canvasFx.ts surface.ts loop.ts shake.ts   the canvas engine
    anchors.ts          anchor → viewport box at fire time (a hand: its cards); the board shake sink
    director.ts         one frame loop: fires cues, steps and draws, expires DOM and stage effects
    dom.ts fx.css       DOM flourishes (splats, rays, banners, ghosts, stand-ins) and their keyframes
    castOnDraw.ts       R502: which cardPlayed is a cast on draw, read off the order of the redacted events
    cardFx.ts           R502: the cast on draw's burst out of the Deck pile, and CARD_FX, one table from a card
                        to its signature recipe (#21 Hinder's mana crack, #27 Blood Ridden's blood drain,
                        C+ #24 Crushing Walls' spiked walls)
    entrances.ts        R670: the marquee Legendary and Mythic Units' own entrances, keyed in CARD_FX, which
                        replace the rarity entrance on their summon into a unit zone
    manaMarks.ts        R502: the crystals the next refresh will not fill, read off the view's rider badge and
                        marked on the board's trays (drawn in every mode: it is information)
    chaos.ts brand.ts   R436: Call to Chaos's effect names and slot-machine reveal; R437: a mark's brand
    shield.ts           R1363: the shield Armor flashes up, small over a hit it took half or more of, full
                        over one it took whole (`damageAbsorbed`)
    build.ts            the small cue builders the v0.2.0 recipes share
    FxLayer.tsx         the overlay Game mounts after the board; listens to the runner's signals, and reads
                        the newest view (`latest`) for a number no event carries
  settings/             the settings store (localStorage, in try/catch) and the panel the
                        gear opens from the game's control bar and the nav
    slots.ts controls.tsx   the other tasks' controls the panel mounts (effects speed and
                        intensity, animated foil, the audio panel), each with its reset
    tabs.ts             the dialog's sections are tabs (Gameplay, Visuals, Audio); the tab used
                        last is kept on the device (jackioh.settings.tab)
    groups.ts accountSync.ts   an active account's copy of the settings (R633,
                        R634): the four stores as groups and the sync that takes the newer side of each
                        and sends changes up (GET/PUT /api/settings), with no status UI since #303:
                        the sync runs silently
  stats/                the device's player statistics (R639): `track.ts` reads a game's log off the views the
                        board is handed (only what the viewer was shown), `useGameStats.ts` adds the finished
                        game to the totals `store.ts` keeps in localStorage (jackioh.stats.v1, in try/catch),
                        `PlayerStatsCard.tsx` is "Your table" on the homescreen, and `config.ts` holds the
                        numbers (the rotation threshold, interval and swap length, the card weights)
  routes/dev/hotseat.tsx  the dev hotseat route
  routes/patch-notes.tsx  /patch-notes: every card patch and the cards it touched (patches/PatchNotes.tsx,
                        R388, R507); the site footer (routes/SiteFooter.tsx) links it
  routes/almanac.tsx    /almanac: the public Card Almanac (R630), every card with tokens, read-only through
                        the deck builder's browse pane (game/deckbuilder/CardBrowser.tsx) and the bundled
                        catalog, no API call; the site footer links it beside Patch notes
  routes/stats.tsx      /stats: the public card and player statistics page (R654), sortable cards table with
                        confidence floor, card drill-down, public player aggregates, and a provisional banner
                        that names no data source and none of the gate's workings (R661); the site footer
                        links it, and the landing page's calls to action do not
  wasm/index.ts         the only caller of the WASM module: loadWasm() (main.tsx and the practice worker
                        await it before anything else), loadWasmSync(bytes) for jsdom, and one typed
                        function per binding; pkg/ is scripts/build-wasm.sh's output, gitignored
  wire/                 what the @jackioh/* imports resolve to (vite.config.ts, vitest.config.ts and
                        tsconfig.json alias them): generated/ (the Rust wire types, ts-rs), engineConfig.ts
                        and serverConfig.ts (the Rust constants the client reads), all three written by
                        `cargo test` and never edited; engine.ts, ai.ts, validator.ts and cards.ts (typed
                        functions over ../wasm and the catalog); rng.ts and the hand-kept helpers
                        (codes.ts, emotes.ts, aim.ts, catalog.ts, stats.ts)
  test/
    setup.ts            jsdom matchers, a matchMedia stub, and the WASM module loaded synchronously
    fixtures.ts         fixture PlayerViews; every test renders one of these
scripts/
  gen-voice.mjs         renders card-audio.json5's lines to public/audio/voice/<card-id>-<hook>.m4a
  gen-music.mjs         renders the scores in music/tracks.mjs to public/audio/music/<track>.m4a (R631)
  music/                the composition toolkit (theory.mjs, compose.mjs, midi.mjs) and every score
  font-fallbacks.py     the metric-matched fallbacks' size-adjust and ascent/descent overrides for
                        src/fonts.css, from the font files (by hand; needs fontTools and brotli)
```

## `PlayerView` gaps found while building M5 (SPEC §10.8)

`CardView` is `{ instanceId, defId, radiant, cost }`. BUILD M5-T1 requires a card's **name** on
its face, and a picker needs its type and rules text, so a `PlayerView` alone cannot draw a
card. The catalog is public information (§5.1; §9.4 checks a `catalogVersion` on both sides), so
`game/catalog.ts` holds a lookup behind a React context — but the view does not supply it, and
with no catalog loaded every card renders its `defId` rather than a guessed name. Either
`viewFor` should carry the names of the cards it reveals, or §10.8 should say that the client
loads the catalog separately and pins it to `catalogVersion`.
