## 11. Open rules questions and recommended rulings

Every place this spec decided something the source left open is listed here; each row is a config value or a named test in the engine. Rows marked decide change gameplay materially and are still open: [[R1]], [[R4]], [[R5]], [[R14]], [[R26]] and [[R39]] ([[R2]] was decided by patch v0.2.0, [[R389]]). The spec reviews of 2026-09-17 added [[R58]] to [[R90]]: [[R58]]–[[R75]] in the first round, [[R76]]–[[R81]] in the second, [[R82]] and the refinements to [[R33]], [[R43]], [[R61]], [[R64]], [[R73]], [[R76]], [[R77]], [[R79]], [[R80]] and [[R81]] in the third and fourth, and [[R83]]–[[R90]] in the milestone reviews of the same day, along with edits to [[§2.5]], [[§3]], [[§4]], [[§4.4]], [[§4.5]], [[§5.1]], [[§6.3]], [[§8]], [[§9.5]], [[§10.1]], [[§10.2]], [[§10.6]], [[§10.7]] and [[§10.8]]: the designer chose [[R59]], [[R60]], [[R61]] and the Transmogulate pool in [[R35]], and the rest follow the source text or Hearthstone.

**[[R91]] to [[R170]] were added on 2026-09-18**, and this paragraph did not name them until it was corrected — which is why the provenance is now stated as a rule rather than a list: every row from [[R91]] on was added by the milestone work or by a review of it, in the same change as the code it describes and the test that pins it (`fn r<n>_…` in Rust, `it("R<n> …")` in TypeScript), and `cargo jackioh spec check` fails if either is missing. The sections they edit, beyond those above, are [[§4.3]] ([[R93]]'s First Strike, stated attacker-only against [[§6.1]] until Part A r5 caught it), [[§9.1]], [[§9.3]], [[§9.4]], [[§9.5]], [[§10.3]], [[§10.4]], [[§10.5]], [[§10.9]] ([[R102]], [[R113]], [[R116]], [[R126]] and [[R153]] all needed `resume`, `delayed` and `setStat`, which its `Script` listing omitted) and [[§10.8]] ([[R168]], [[R169]]). None of these is a designer choice: they are rulings the code had to make to run, written down after the fact where the spec was silent, and where one contradicted working code — [[R93]], [[R119]] — the code was right and the text was the defect.

**[[R171]] on are the polish pass of 2026-09-22** (`docs/polish/`), under the same rule. Each task was given a block of numbers so the branches could be built at once, and a block need not be used up, so the table has gaps: [[R171]]–[[R179]] and, past the blocks, [[R209]]–[[R226]] and [[R240]]–[[R243]] are the edge-case hunt (`docs/polish/4-edge-cases.md`), whose rounds filled its own block first. The sections it edits are [[§4.1]] ([[R171]]), [[§4.4]] ([[R240]]), [[§8]] #64's Engine cell ([[R213]]), [[§10.1]], [[§10.5]] and [[§10.7]].

**[[R244]] on are features added after the polish pass**, under the same rule. [[R244]] and [[R245]] are The Coin ([[§2.1]], [[§2.3]], [[§5]], [[§7]], [[§9.9]]), which the user asked for on 2026-09-24.

**[[R247]] is the live-cards change of 2026-09-24**, from the block [[R247]]–R249: what a card face shows in play against what the collection prints ([[§10.10]]). It edits [[§8]] #82's Engine cell ([[R247]]) and [[§10.10]].

**[[R250]] to [[R264]] are the decks-and-modes change of 2026-09-24**, which the user asked for: named decks and trios in place of the one three-deck loadout, deck codes and autosave ([[§2.6]], [[§9.1]], [[§9.4]]), and three queue modes with the Best-of-3 series ([[§9.5]], [[§9.8]], [[§9.9]]). Where the brief left a choice open — what a drawn game in a series counts for, how a series is rated, what happens when nobody picks — the row says so and gives the reason.

**[[R265]] on are the concurrent mulligan and the draw offer of 2026-09-24**, from the block [[R265]]–R274, which the user asked for: both players mulligan at the same time ([[R265]]–[[R268]]), a draw offer's lifetime ([[R269]]), and the server's own nonces, which the review of the mulligan clock found a client could pre-empt ([[R270]]). They edit [[§2.1]], [[§2.5]], [[§9.5]], [[§9.9]], [[§10.1]], [[§10.2]], [[§10.6]] and [[§10.8]].

**[[R275]] on are the Radiant pass of 2026-09-24**, from the block [[R275]]–R289, which the user asked for the same day: every Radiant face held to one power standard and the cards below it raised (`docs/radiant-audit.md`), the Radiant face printed in full with what differs from the base marked, the Jlockeed tag, the reference map a card's text links through, and the numbers a formula comes to now. It edits [[§5]], [[§5.2]], [[§6.1]], [[§7]], [[§8]] (27 rows' Radiant or Stats cells, #13 and #14's tags, and the Engine cells of #33, #38, #50, #67, #80 and #96), [[§10.8]], [[§10.9]], [[§10.10]], [[R67]] and [[R74]].

**[[R290]] on are the tutorial of 2026-09-24**, from the block [[R290]]–R309: the tutorial opponent's handicap ([[R290]]), the lessons ([[R291]]), the coach ([[R292]]), lessons that are won by following it ([[R293]]) and progress kept on the device ([[R294]]). They add [[§9.10]] and a sentence to [[§9.9]].

**[[R310]] on are the library list and the tutorial without Skip step of 2026-09-25**, from the block [[R310]]–[[R314]], which the user asked for that day: a player may look through what is left in their own library, never its order ([[R310]]–[[R313]]), and the tutorial loses its Skip step ([[R314]]). They edit [[§3]], [[§9.1]], [[§9.10]], [[§10.1]], [[§10.8]], [[§10.10]] and [[R292]].

**[[R315]] on are the overflow animations of 2026-09-25**, from the block [[R315]]–[[R319]], which the user asked for: an animation when a hero takes fatigue damage from an empty library, when a full hand burns a card, and when a full library turns one away. Nothing reported a fatigue draw apart from its hit, or a card [[R80]] refused, so two events are new, `fatigue` ([[R315]]) and `libraryOverflow` ([[R316]]); a full hand already had `burned`, and [[R317]] records what each player reads of it. [[R318]] is what the board shows for the three and [[R319]] what the speakers play. They edit [[§2.4]], [[§10.3]] and [[§10.10]].

**[[R320]] on are the account additions of 2026-09-25**, from the block [[R320]]–R329, which the user asked for the same day: a signed-in player's tutorial progress and hidden lesson path kept on their account as well as on the device ([[R320]]–[[R322]]), and emailed links that carry a PKCE code instead of tokens ([[R323]], [[R324]]). They edit [[§9.2]], [[§9.4]] and [[§9.10]].

**[[R330]] on are Conquest and trio codes of 2026-09-25**, from the block [[R330]]–R344, which the user asked for: the trio mode became Conquest, a win with each deck of a trio, with a sealed pick of an unwon deck before every game ([[R330]]–[[R338]]), in place of [[R259]]–[[R261]]'s first to two wins with each deck played once; and a trio travels with its three decks as a code, imported whole or not at all ([[R339]]–[[R341]]). [[R259]], [[R260]] and [[R261]] keep their numbers and now say what of them still stands; [[R262]] to [[R264]] stand as written. They edit [[§2.6]], [[§9.1]], [[§9.4]], [[§9.5]], [[§9.8]], [[R253]], [[R257]] and [[R264]].

**[[R345]] is the automatic-turn-end setting of 2026-09-25**, from the block [[R345]]–[[R349]], which the user asked for: a setting that turns [[R82]]'s automatic turn end off. It edits [[R82]]'s row, [[§10.2]]'s action list and [[§10.8]].

**[[R346]] on are patch v0.1.1 (issue #27)**, from the block [[R346]]–R359 ([[R345]]'s block left [[R346]]–[[R349]] unused), which the game's designer asked for on 2026-09-26: where the issue and this spec disagreed, the issue won, and this spec was changed to match. They are the engine and mechanics half of the patch: Pierce ([[R346]]), Indestructible without Taunt ([[R347]]), X of at least 1 ([[R348]]), the Radiant fallback ([[R349]]), CN-Virus's copies at the end of the turn ([[R350]]), face-down Traps' cost ([[R351]]), Heroic Power's Stitching ([[R352]]), the Ghoul Token ([[R353]]) and Zao Gao ([[R354]]). They edit [[§1]], [[§2.2]], [[§2.3]], [[§2.4]], [[§3]], [[§4.1]], [[§4.2]], [[§4.4]], [[§5]], [[§5.1]], [[§5.2]], [[§6.1]], [[§6.3]], [[§7]], [[§8]] (#44, #74, #80, #90, #90.1 and #98), [[§9.7]], [[§10.4]], [[§10.6]], [[§10.8]], [[§10.9]], [[R16]], [[R21]], [[R46]], [[R103]], [[R221]] and [[R275]].

**[[R360]] on are the card patches of patch v0.1.1 (issue #27, 2026-09-26)**, from the block [[R360]]–R369, which the game's designer set. The designer's words and numbers are the cards' now — [[§8]]'s rows for #1, #8, #20, #25, #30, #42, #46, #50 (renamed K-Pop Fanatic), #53, #55, #56, #62, #77, #78, #81, #83, #84, #85, #86, #87, #92 and #99 carry them, and docs/radiant-audit.md records each patched card against [[R275]], where the designer's word stands — and these rows are what the new texts leave open: whose side a Lava Golem paid for with the opponent's units lands on ([[R360]]), which unit "Miss" Mrow takes ([[R361]]), what the Radiant Felinor Fiender's "twice" doubles ([[R362]]), which costs Professor Curvature's "(4)+ Cost" reaches ([[R363]]), what a Refresh is ([[R364]]), where Transmogulate's hand fits [[R35]] ([[R365]]), and the words every card's text now uses ([[R366]]). They edit [[§2.3]], [[§6.1]]'s card columns, [[§6.2]], [[§6.3]] (Tribute, Cost and a new Refresh row), [[§8]], [[§10.4]], [[R8]], [[R23]], [[R35]], [[R39]], [[R48]], [[R64]], [[R65]], [[R102]] and [[R217]].

**[[R370]] on are the v0.1.1 patch's presentation and wording of 2026-09-26** (issue #27, from the block [[R370]]–R379), which the game's designer asked for: a face-down trap's cost on its back ([[R370]]), your own face-down trap marked as one the other player cannot see ([[R371]]), Combo-Index read by its letter grade ([[R372]]), "Deck" and "Tribute" for the rules' library and Sacrifice wherever a player reads them ([[R373]]), and a landing page that deals a random hand ([[R374]]). They edit [[§6]]'s introduction, [[§8]] #93's Engine cell, [[§10.8]], [[§10.10]], [[R313]] and [[R318]].

**[[R375]] is the first build of the card patch history (issue #39, 2026-09-30)**, which patch v0.2.0's [[R388]] and [[R507]] replaced when the two met on main; the row stays so issue #39's versions keep their record.

**[[R376]] on are card win-rate tracking (issue #48, "Patch v0.2.5", 2026-10-01)**, the next numbers free, which the game's designer asked for: the record every finished game leaves ([[R376]]), each card's win rates by match type, patch and pilot, with four breakdowns and the played delta ([[R377]]), and the AI's development runs, kept apart from live data ([[R378]]). They add [[§9.11]].

**[[R380]] on are patch v0.2.0**, from the design brief `docs/classic-sets.md` and issue #40, which the game's designer asked for on 2026-09-30: the Classic and Classic+ sets, the new keywords (Animated, Activate, Brittle, Degrade and Upgrade) and the mechanics the new cards need, the Core card patches and the cosmetic items. [[R380]]–[[R425]] are the brief's proposed rulings, in its order; [[R426]]–[[R439]] are the issue's card patches ([[R426]]–[[R431]]), its house style ([[R432]]) and its presentation items ([[R433]]–[[R439]]). Where the brief asked the designer a question, its default answer stands, except four that the patch's plan answered otherwise: Classic #55 is kept as Book of Wildfire and Classic #72 is renamed Grand Counterspell ([[R381]]), the turn cap is 60 player-turns under a 120-minute match ceiling ([[R389]]), and Brittle's first tick is at t + 2 ([[R385]]). The workstreams that build the patch each took a block for the rulings they find — the engine [[R440]]–R479, the catalog, patch history and server [[R480]]–R499, the client [[R500]]–R519, Classic #1–#45 [[R520]]–R539, Classic #46–#90 [[R540]]–R559, Classic+ #1–#39 [[R560]]–R579, Classic+ #40–#78 and the AI cards [[R580]]–R599, the AI [[R600]]–[[R609]], and QA and end to end [[R610]]–R629 — their rows are ported here as each workstream lands them, and a block need not be used up, so the table may have gaps. They edit [[§1]], [[§2.2]]–[[§2.6]], [[§3]], [[§3.1]], [[§3.2]], [[§4.1]], [[§4.2]], [[§4.4]], [[§4.5]], [[§5]], [[§5.1]], [[§5.2]], [[§5.3]], [[§6.1]]–[[§6.3]], [[§7]], [[§8]] (with the new [[§8.6]] Classic and [[§8.7]] Classic+), [[§9.4]], [[§9.5]], [[§9.7]], [[§9.9]] and [[§10.1]]–[[§10.11]]; they rewrite [[R1]], [[R2]], [[R12]], [[R13]], [[R17]], [[R25]], [[R28]], [[R35]], [[R41]], [[R43]], [[R57]], [[R59]], [[R65]], [[R67]], [[R73]], [[R78]], [[R79]], [[R87]], [[R105]], [[R184]], [[R185]], [[R186]], [[R201]], [[R255]], [[R278]], [[R279]], [[R310]], [[R339]], [[R366]] and [[R370]], and edit [[R5]], [[R275]] (its named exceptions), [[R16]], [[R23]], [[R44]], [[R53]], [[R62]], [[R70]], [[R77]], [[R101]], [[R118]], [[R119]], [[R153]], [[R173]], [[R174]], [[R175]], [[R204]], [[R212]], [[R213]], [[R215]], [[R222]], [[R244]], [[R245]], [[R280]], [[R283]], [[R316]], [[R350]] and [[R363]], whose words the patch made untrue.

**[[R603]] on are the ranked ladder (issue #49, "Patch v0.2.6", 2026-10-02)**, the next numbers free, which the game's designer asked for: a hidden Glicko-2 rating ([[R603]]), the ranked match type as the only thing that moves it ([[R604]]), the Grape tiers ([[R605]]), the gentle convergence and the win-streak bonus ([[R606]]), the tier floors and season peaks ([[R607]]), Jlorious as the top 100 Mythic players by rating ([[R608]]), seasons on the minor version with a soft reset ([[R609]]), the bots' own ratings ([[R610]]), the record every rated game leaves ([[R611]]), and the reads, which never show the rating ([[R612]]). They add [[§9.12]].

**[[R630]] is the Card Almanac (issue #54, "Patch v0.2.9", 2026-10-02)**: a public page that browses every card, tokens included, through the deck builder's own browse pane, read-only. It takes the first number past patch v0.2.0's blocks, since rows already in flight on other branches hold the numbers after [[R602]]. It edits [[§10.10]].

**[[R631]] is the music (issue #51, "Patch v0.2.7", 2026-10-03)**: a main menu theme, and on the board
music that follows each player's own game, under four stations. It takes the next number after
[[R630]]. It edits [[§10.11]].

**[[R633]] and [[R634]] are the game settings kept on the account (issue #129, 2026-10-03)**: an active
account's settings follow it to another device, as its tutorial progress does ([[R320]], [[R321]]). They take
the next numbers after [[R632]]. They edit [[§9.1]], [[§9.4]] and [[§10.11]].

**[[R636]] to [[R638]] are the keyword rules of the v0.2.X patch's first part (issue #123, 2026-10-03)**: Windfury,
Temporary, and Brittle ticking on the field only. They take the next numbers after [[R634]], [[R635]] being held
at the time for the card patch history's pending fragments (issue #63), which landed as [[R646]]. They edit [[§2.2]], [[§4.1]], [[§6.1]] and [[§6.3]].

**[[R639]] is the player statistics and the homescreen's rotation of the v0.2.X patch's third part (issue
#125, 2026-10-03)**: a tally the device keeps of its player's games, shown on the homescreen, and a fan
that draws on every set once there is enough of it. It takes the next number after [[R638]]. It edits [[§9.11]]
and [[§10.10]].

**[[R641]] to [[R645]] are the emotes of patch v0.2.X (issue #75)**: hero portraits a deck carries, the
five voice lines and five emoji each can send, the limit both ends enforce, the mutes, and the AI's
emote personas — all cosmetic, none of it ever reaching `PlayerView`, the action log, a replay hash
or a game record. They take the next numbers after [[R640]]. They edit [[§9.4]], [[§9.5]], [[§9.9]], [[§10.10]] and [[§10.11]].

**[[R646]] is pending card-patch fragments and promotion in ship order (issue #63, 2026-10-03)**:
branches claim catalog changes without editing history; a promotion records the adding commit in
the new patch's `commits` list with `reconstructed: false`, then snapshots that commit and advances
the current catalog version. It takes the next number after [[R645]] and edits [[§10.10]].

**[[R653]] is patch v0.2.3 (issue #113, 2026-10-03)**, which the game's designer asked for: eighteen Field
Spells that could come to life gain Animated with a unit face ([[§8]]'s rows say which), and C+ #33
Ivory Tower's text becomes "The first Unit you stack onto this is fused into it". [[R653]] settles what the
issue left to Hearthstone's semantics: when the stacked Unit is fused in, what is fused when an answer
changed it, and how long "the first" lasts. It takes the next number after [[R652]] ([[R651]] and [[R652]] having
been taken by patch v0.2.2's Joro and Blade Storm rulings while this branch was in flight), and the
patch edits [[§3.2]], [[§5.1]], [[§6.1]], [[§7]], [[§8]] (C+ #5, C+ #24, C+ #33, T-AI-6 and the eighteen cards) and [[R5]],
[[R13]], [[R78]], [[R383]], [[R408]], [[R418]], [[R446]], [[R447]] and [[R588]]. Undone by patch v0.2.5 (issue #218), except Ivory Tower's fusion, which stays; the patch also adds [[R657]].

**[[R654]] is the public card and player statistics page of the v0.2.X patch (issue #131, 2026-10-03)**:
the publication gate (`PUBLIC_STATS_MIN_LIVE_GAMES`, 1000 live ranked and unranked games, tutorial excluded)
under which AI development games pad the card win-rate figures, and at and above which public statistics
strictly use live games only and AI games are ignored; the source labelling and provisional progress banner;
the minimum per-row sample threshold (`CARD_STATS_MIN_SAMPLE`, 20 games) below which win rates show
"not enough games" rather than a percentage; the public player summaries with player privacy opt-out;
and the server cache TTLs (`CARD_STATS_CACHE_TTL_SECONDS`, 300 s; `PLAYER_STATS_CACHE_TTL_SECONDS`, 60 s).
It takes the next number after [[R653]]. It edits [[§9.11]].

**[[R655]] is the card sound effects of issue #192 (2026-10-04)**: a card's hooks may play a sound effect
beside or instead of a voice line, a Unit picked up to attack plays its attack hook, and every card's
sounds are one hand-edited file. It takes the next number after [[R654]]. It edits [[§10.11]].

**[[R656]] is patch v0.2.4 (issue #181, 2026-10-04)**: aimed random casts, where every target declaration
says whether it harms or helps and a cast that targets enemies aims each pick by it, and the Deft
keyword, which replaces Deft Duelist's flag ([[R49]]) and joins the random keyword pool ([[R21]]). It takes the
next number after [[R655]] ([[R654]] and [[R655]] having been taken by the statistics page and the card sound
hooks while this branch was in flight), and the patch edits [[§4.1]], [[§6.1]], [[§6.3]], [[§8]] (#45, C+ #17,
C+ #38.1, C+ #40) and [[R21]], [[R49]], [[R452]] and [[R636]].

**[[R657]] is the Animated stat fallback of patch v0.2.5 (issue #218, 2026-10-04)**: an Animated card
with no printed stats fights as a 0/1. It takes the next number after [[R656]] ([[R654]], [[R655]] and [[R656]]
having been taken by the statistics page, the card sound hooks and the aimed random casts while this
branch was in flight). It edits [[§6.1]].

**[[R661]] is patch v0.2.Y's statistics made less central (issue #255, 2026-10-05)**: the landing page
loses its Stats link, the statistics page and the card detail view stop showing where the figures
come from and how the publication gate works, and the Play online lobby shows the player's own rank
with a link to the leaderboard. It takes the next number after [[R660]]. It edits [[§9.11]], [[§9.12]] and [[R654]].

**[[R669]] is the sound and feel polish of patch v0.2.X (issue #259, part 2 of the polish backlog #146,
2026-10-04)**: a sting for every card the viewer can read as it is played, the effects panned by lane,
ducked under voice lines and given a light shared reverb, and a short vibration on a phone for a drop,
a hit and the viewer's turn start. Presentation only. It takes [[R669]], the number assigned to it while
several branches were in flight. It edits [[§10.11]].

**[[R704]] is the homescreen's rotation from the first visit (issue #200, "Patch v0.2.X", 2026-10-04)**:
the hand swaps one card at a time on every device, among Core's cards until [[R639]]'s threshold opens
every set, and a swap is a fizzle and an apparition rather than a cut. It takes the next number
after [[R703]]. It edits [[§10.10]], [[R374]] and [[R639]].

**[[R739]] to [[R742]] are patch v0.2.9 (issue #44, the rarity pass, 2026-10-05)**, which the game's designer asked for: the rarity criteria, families and consistency rules ([[§8]], [[R739]]), fifteen rarity changes across the three sets, Traps that reveal instead of activating ([[R741]]), Plague Counters instead of Plague Tokens ([[R740]]), Crop Dusting's designer Radiant face, and Pocket Chaos at (4) with its priced gift ([[R742]]). It edits [[§6.3]], [[§8]], [[§10.11]], [[R65]], [[R78]], [[R395]], [[R400]], [[R403]], [[R454]], [[R602]] and [[R631]].

**[[R745]] is the game log's history (issue #350, 2026-10-06)**: the log keeps the whole game rather than the view's last events. It takes the next number after [[R744]]. It edits [[§10.10]].

**[[R763]] is the fuzz gate's hidden-information invariant (issue #348, 2026-10-06)**: the one leak it found, a fired trap named to its controller after it went back into a library. It takes the next number after [[R762]]. It edits [[§10.8]], [[R97]] and [[R154]].

**[[R764]] is a Glitch's reset and boards leaving no public trace (issue #348, 2026-10-06)**: the second leak the invariant's review found, in the events a Glitch leaves behind. It takes the next number after [[R763]]. It edits [[R97]], [[R676]] and [[R678]].

**[[R765]] is the menus' way back into a game (issues #475, #476 and #478, 2026-10-08)**: a practice game's Save and leave and Leave without saving, the practice menu's banner that resumes a saved game, a queue pairing that takes the player to the game from anywhere in the client, and the banner on the main and practice menus while their online game is live. It takes [[R765]], the number assigned to it while several branches were in flight. It edits [[§9.5]], [[§9.9]] and [[R668]].

**[[R766]] is a card reaching a graveyard or exile as its printed card, its price included (issue #473, 2026-10-08)**: the owner's decision after part 40's sweep of record found C+ #54 Book of Books' (0) following a Book of Stats into the graveyard, where C #90 In Too Deep's reward L replayed it for free until the action cap. It takes [[R766]], the number assigned to it while several branches were in flight. It edits [[§3]], [[§6.1]], [[§8.2]] (#31), [[§10.1]], [[R78]], [[R215]], [[R385]], [[R386]], [[R429]] and [[R742]].

**[[R767]] is a room shared as a link (issue #432, 2026-10-08)**: the room ticket's Copy invite link, and the lobby that opens on one with its join form filled in. It takes the next number after [[R766]]. It edits [[§9.5]].

**[[R768]] is what a replay shows (issue #508, part 1 of #430, 2026-10-08)**: a seat's view at any step of a finished game, refused on another catalog version or a final-hash mismatch. It takes the next number after [[R767]]. It edits [[§9.3]].

**[[R1420]] to [[R1424]] are the foundation of the Meditative set (issue #496, 2026-10-08)**: the set is in the catalog and built card by card on `main` before it ships, so its cards are in no pool, deck, list, patch or server table until the last part of its patch lists it in `SHIPPED_SETS` ([[R1420]]); a Prime pool holds the Prime tokens ([[R1421]]); a pool may ask for any one of several tags ([[R1422]]); control can be given to the other player ([[R1423]]); and the tribal tags are named ([[R1424]]). They take the top block of the numbers held for the set (R1420–R1429; `docs/issues-and-patches.md`, Ruling numbers), so the next free number is above every block, and the set's own rulings take the blocks below it, one per part (`docs/meditative-set.md` M10). They edit [[§5]], [[§5.1]], [[§6.3]], [[§9.4]], [[R380]] and [[R382]].

The table's rows are the ruling notes in `rulings/`, one per row (`rulings/R0195.md` is [[R195]]), each with its ruling, the cards and sections it affects, and the tests that prove it. [INDEX.md](INDEX.md) lists them all, one line each.
