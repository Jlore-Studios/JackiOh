## 5. Card anatomy and card types

A card is a definition in the catalog plus an instance in a game; the definition carries both the base form and the Radiant form, and the instance carries a `radiant` flag.

Every card carries the fields below; the catalog stores the base and Radiant forms under one id, and the Radiant form always has the same cost.

| Field | Values | Catalog field |
| --- | --- | --- |
| Cost | 0 to 6, 10 (C+ #64 Mulch Muncher), 100 (Ceaseless Void), X, or "A embiggen B" | `cost: number or 'X' or {base, embiggen}` |
| Type | Unit, Spell, Field Spell, Trap, Field Trap; a face may carry its own type, which is the card's while that face is up (C+ #22 Blood Moon's Radiant face is a Field Trap, [[§5.2]]) | `type`, `base.type?`, `radiant.type?` |
| Tribes and tags | Human, Felinor, KY, CN, Fruit, Call to Chaos, Quickdraw, Jlockeed, Book, Pancake, AI, Plague, Catalyst, Prime, Acclaimed, Token (Jlockeed: [[R278]]; Book on every "Book of …" card, Pancake on C+ #12, #13 and the eight Pancake tokens, AI on the ten AI generated cards, Plague on every card that uses Plague Counters; patch v0.2.Y (#322) adds Catalyst on C+ #38 Solarius and C+ #46 Felinor Flagbearer, Prime on the two tokens their Deaths shuffle in, C+ #38.1 Solarius Prime and C+ #46.1 Felinor Flagbearer Prime, and Acclaimed on C #80 BOOM! Big Max and C+ #37 Wardrum; no rule and no pool reads these three) | `tags: string[]` |
| Rarity | Common, Rare, Epic, Legendary, Mythic; Token for every token. A Classic+ token may print a rarity on its frame, a display field for the card frame and the summon sting that no pool ever reads | `rarity`, `printedRarity?` |
| Set | Core, Classic, Classic+; Meditative, in the catalog and shipping with the last part of its patch ([[R1420]]); Boss, Boss-X reserved | `set` |
| Id | `core-043`, `classic-043`, `classicplus-043`, `meditative-043`; a token a card defines `core-051-1`, `classicplus-012-1`, `meditative-039-1`; a shared token `core-t-rush`, `classicplus-t-ai-01` … `-10`. The set part has no hyphen inside it, so an id splits one way | `id` |
| Index | 1 to 100 in Core, 1 to 90 in Classic, 1 to 78 in Classic+, 1 to 99 in Meditative; tokens N.1 (Core) or N.k (Classic+, C+ #12.1–#12.8; Meditative, #39.1–#39.5) when card N defines them, T-name when shared (Rush, Sheep, Felinor, Bread), dealt by a rule (Coin), a card of its own (Ghoul, [[R353]]), or made by two cards (T-AI-1 … T-AI-10). An index is unique only within its set, so every lookup by index names the set, `(set, index)`, or uses the id | `index: string` |
| Stats | Attack/Health, base and radiant; a Unit may print them in X (C+ #69 Buff Billy's 3X/3X), summoned with the X it was played for through `statsOverride`, X at least 1 ([[R348]]); an Animated Field Spell, Trap or Field Trap prints the stats of its unit face ([[R383]]) | `base.stats`, `radiant.stats`, `base.xStats?`, `radiant.xStats?` |
| Text | keywords + scripted effects, base and radiant; a number Degrade, Upgrade or C+ #41 KY's Constant may tune is written `{key}`, or `{key|singular|plural}` for a count and the words that agree with it, and declared in `params` ([[R386]], [[R482]]); a client prints a face's text only through `fillParams`, which fills it from the printed or the instance's current values | `base.script`, `radiant.script` |
| Tunable numbers | `{ key, base, radiant, better: "up" or "down", step?, min?, max? }` per number; the view carries each instance's current value and a script reads `param(ctx, key)`; the ten AI generated cards declare none, their numbers being printed ([[R386]], [[R482]]) | `params?` |
| Lines of code | the non-blank, non-comment lines of the card's script file, imports excluded, recorded in the catalog (since v0.3.0 frozen as it stands, and counted by `cargo jackioh catalog loc` for a card added later); a fused card's is its ingredients' sum. It is public card data (the collection's inspect overlay prints it, while a match hides it, [[R693]]), C #48 Hired Shrimp and C+ #44, #45 read it, and it is part of the card's patch history ([[R388]]), so a refactor that moves it is a balance change | `loc` |

### 5.1 Types

- Unit: a permanent in a unit zone with stats; does combat.
- Spell: one-shot. Resolves, then goes to the graveyard, or to exile when it says "exile this". Spells with "End of turn: add this back to your hand" are flagged `returnToHandAtEndOfTurn` when played and return from the graveyard at the end of that turn, as graveyard triggers ([[R68]]).
- Field Spell: a permanent in the backrow with a lasting effect. May have a Cry (Anti-oneshot Armor), start/end-of-turn triggers, or an activated ability (Heroic Power, and any Activate, [[§6.2]]). A Field Spell's unlabelled one-time text is its Cry, after which it stays in the backrow with no further text (C+ #12.7, [[R408]]). An Animated Field Spell animates as it enters the field ([[§6.1]], [[R383]]).
- Trap: paid for and placed face-down in the backrow. Fires automatically the moment its condition is met, on either player's turn, then goes to the graveyard — unless it is Animated, when it animates as the last step of its firing and, with no open unit zone, stays face-up in its zone as a Field Trap does ([[R383]]). Only its controller sees its identity before it fires; the other player sees a face-down card and its cost, even if they own it ([[R33]], [[R351]]).
- Field Trap: a Trap that stays after firing and can fire again.
- Permanent = anything occupying a unit or backrow zone.
- Token: generated only when a card names it, or a rule does (The Coin, [[§2.1]]). Random pools ("a random card", "Discover a (2) Cost card") never include Token-tagged cards, with two exceptions the designer wrote ([[R382]]): a Fruit pool is the non-token Fruit cards plus the five Grapes (C+ #65.1–#65.5), which any Fruit card may generate, and C+ #23 Dropshipping's pool "(including tokens)" takes every token of every set.
- Pools reach every set that ships ([[R380]], [[R1420]]): a pool whose card does not name a set draws from Core, Classic and Classic+ alike, Hearthstone's reading of "a random card", and from a set the catalog holds before it ships (Meditative, until the last part of its patch) only once it does; a pool that names that set, or names its cards, reaches them. A Prime pool holds the Prime tokens and an AI pool the ten AI generated cards ([[R1421]]), as a Fruit pool holds the Grapes. A card that names a set keeps to it: #82 KY's Trial and #97 Zephyrs stay Core, and C+ #27, #73, #73.1 and #75.1 name Classic or Classic+.
- A card never generates itself ([[R387]]): every way a card makes a card from a pool — added to a hand, shuffled in, summoned, Discovered, cast, transformed into, fused in, or put in another card's place — excludes the generating card's own definition, by its id (indices repeat across sets), and a fused card excludes each of its ingredients' definitions. The one exception is a card that names a pool holding itself: Call to Chaos's "cast a random Call to Chaos" draws from the Call to Chaos tag, which holds every edition alike, #95, C+ #73 and Meditative #95 once its set ships ([[R1240]], [[R1420]]), and C+ #73's deck replacement names the same pool. A copy is not generation: "summon a copy of this unit" (#12), "fill your board with copies of this" (C+ #46.1), Echo repeats and C+ #14 Forever& are unaffected.

Rarity, tribe and set are pure filter tags. The tribes are Human, Felinor, KY, CN and Jlockeed ([[R1424]]). The catalog needs one query function, `catalog.query({type, cost, costRange, tags, notTags, anyTags, rarity, set, excludeDefId, withTokens})`, that every random-generation and Discover effect uses: `set` absent is every set that ships ([[R380]], [[R1420]]), `anyTags` asks for at least one of its tags where `tags` asks for all ([[R1422]]), `excludeDefId` is [[R387]]'s self-exclusion by id, and `withTokens` lets tokens into the pool (C+ #23's, [[R382]]). Nothing looks a card up by index alone: an index lookup names its set, `defByIndex(set, index)`.

### 5.2 Radiant

The source defines Radiant only as "upgraded versions of normal cards". Rulings that make it implementable ([[R74]]):

- Radiant is a boolean on an instance, never a separate card id. Making a card Radiant sets it; nothing in any set un-sets it.
- In hand or library: cost unchanged, stats and text swap to the radiant form, and a face with its own type swaps the card's type with it: a C+ #22 Blood Moon made Radiant in hand is a Field Trap there, which pools and filters then read.
- On the field (Radiant Saintess, Knockoff Temu Glowy Jelly Bean, radiant GIGA Glowy Jelly Bean, Snom Bunny Mind Control, radiant K-Pop Fanatic's steal): the base-stat layer swaps immediately, damage taken and buffs are kept, newly gained keywords apply at once, ongoing triggers use the radiant text from then on, and Cry does not re-fire.
- A copy of a Radiant card is Radiant. A card an effect generates "Radiant" is Radiant. Tokens can be Radiant (Radiant CN-Virus, Radiant Reminisce).
- Every card has a Radiant face, so making any card Radiant changes it ([[R276]]). The five the source left without one (Quickstriker, Zao Gao, Combo-Fodder, Chaos Golem and My Pawn) were given one by the Radiant pass of 2026-09-24, and the unit tokens have theirs in [[§7]]. The Classic and Classic+ cards whose source faces were identical or missing were given one by patch v0.2.0 (C #63 Crop Dusting, C #86 Genn, C+ #63 Fruit Tree, C+ #75 J-lease J-Jungle EX-plorer), as `docs/radiant-audit.md` records.
- A Unit that prints no Radiant form of its own (the Ghoul Token, [[§7]]) is, made Radiant, its base face with its attack and health doubled: the same keywords and text, and a summon's X/X doubled with it ([[R349]]). The catalog marks such a card `radiantFallback` and prints that fallback as its Radiant face. A fused or crafted card is not one of these: [[R77]] builds its Radiant form from its ingredients' Radiant forms, an ingredient without one lending its fallback.
- A Radiant face is about twice its base face ([[R275]]): a Unit's attack and health are each at least double its base face's, and its effect is 100–150% stronger — 2 to 2.5 times the amount, a broader scope (one target to all, one side to a choice), or an added rider or keyword; a Spell, Field Spell, Trap or Field Trap scales its effect the same way and may add a draw or a tangential rider instead. `docs/radiant-audit.md` records every card against it, and names the few Classic and Classic+ faces where the designer's word stands against the standard ([[R275]]).
- A Radiant face prints its whole text, [[§8]]'s Radiant cell read by [[§8]]'s Conventions and written out (the catalog's `radiant.text`), and a client marks the words that differ from the base face's text ([[§10.10]], [[R277]]).

### 5.3 Catalog data decisions

Places where the source card list was inconsistent, with the value the catalog uses; section 11 records them together as [[R75]], and #94's reading as [[R26]].

| Card | Issue in source | Catalog value used here |
| --- | --- | --- |
| #2 Bigot | No set listed | Core |
| #15 Me and Mr Token, #16 Hit Job | Radiant separator is `~~` | Treated as `~~~` |
| #12, #31, #51 | Header typos: tag "Felinors", name "KY’s, …" | Tag Felinor; names as in section 8 |
| #68 Twisted Sorcerer | Listed as "Spell, Unit" with stats | Unit |
| #90.1 | Named "CN-Viral Injection" like #90 | Named CN-Virus |
| #94 Genn's Greed | "exile all cost cards" is garbled | Exile every card with an odd Cost (Genn Greymane reference) |
| #29 GIGA Glowy Jelly Bean | Cost 6 exceeds max mana 4 | Kept at 6; only castable with mana gain |
| #15, #34, #66, #86, #99, #100 | No card type in the header | Unit, Spell, Unit, Unit, Spell, Unit, from stats and text |

Classic and Classic+ (patch v0.2.0) have their own, recorded under [[R381]] (names) and the rows cited:

| Card | Issue in source | Catalog value used here |
| --- | --- | --- |
| Classic+ #11, #25, #28, #42 (second of each), #47, #75 (second) | Five numbers used twice, and two tokens numbered as cards (#47, and #75's second, [[R381]]) | Jogg's Box C+ #47, Brother Lar C+ #76 and Brother Ping C+ #76.1, Anti-Softlock C+ #77, Claude's Datacenter C+ #78, Felinor Flagbearer Prime C+ #46.1, the J-lease Pack C+ #75.1: the first card under a number keeps it ([[§8.7]]) |
| C #16, C #55 | Two cards named Book of Flame, one text | C #16 is Book of Flame, the one C #23 and C #29 name; C #55 is **Book of Wildfire**, a distinct name so nothing names it by accident ([[R381]]) |
| C #17, C #72 | Two cards named Counterspell | C #72 is **Grand Counterspell** ([[R381]]) |
| C #10 Exile, C #36 Burn, C #57 Echo, C #30 Recycle | Names that are also rules words | Kept; [[R279]]'s proof treats a rules word as a reference only where `refs` lists the card ([[R381]]) |
| C+ #48, #51, #52 | "Jlockheed", where Core's #13 and #14 are "Jlockeed" | Names as the designer spelled them; one tag, `Jlockeed`, on all six ([[R278]]): Core's two, Classic #4 Palantir, and the three named left |
| C #78 Mutate Spell, C+ #60 Doctors Orders, C+ #74 Twice Forward One Step Backwards | Typed Spell, "Unit, Spell" and Trap | Field Spell ([[R402]]), Field Spell ([[R421]]), Field Trap ([[R425]]), as Core #68's type was settled |
| C+ #12.6 Frozen Wastes | Spell whose text has no label | The text is a Cry ([[R408]]) |
| C+ #12.7 Legion of the Hungry | Field Spell whose text has no label | The text is a Cry ([[R408]]) |
| C #69 Plague Charger | Radiant separator is `~~` | Treated as `~~~` |
| C #70 Book of Plague | Tag written "book" | Book |
| C #67 Felinor Feeler, C #32 Felinor Feelings, C+ #39 Book Worm, C+ #60, C+ #64 Mulch Muncher | Tags that look unintended (a Human "Felinor Feeler", no Book or Fruit tag) | Kept as written |
| C #1, C #7, C #63, C #86, C+ #17, C+ #63, C+ #66, C+ #75 | A Radiant face that drops a clause, is missing, repeats the base face, or copies another card (C+ #17's base face too, which repeated C+ #16's: 6 keywords on both faces) | The faces [[§8.6]] and [[§8.7]] print (the audit's reasons, `docs/radiant-audit.md`) |
| C #23 "are this turn", C #36 "at 4", C #42 "a 2", C #68 "cost can't", C #78, C #81 and C #84 "Active", C #90 "work of stats", C+ #11 "Year", C+ #12.3 "deck Deck", C+ #14 "Then next", C+ #17 "Conjure Tush Token++", C+ #41 "number number", C+ #65 and C+ #66 "Graphes" and "roles", C+ #73.1 "Firststrike" | Typos | Read as meant: "this turn", "4", "2", "can't", Activate, "worth of stats", "Your", "deck", "The next", Conjure Rush Token++, "a number", Grapes and "rolls", First Strike |
| C+ #38.1 Solarius Prime | Named "Solarius-Prime", hyphenated, where C+ #46.1 Felinor Flagbearer Prime is spaced | Patch v0.2.Y (#322) names both Prime tokens spaced, their card's name then "Prime", and C+ #38's Death names it that way. [[R279]]'s naming rule then reads C+ #38's own name inside "Solarius Prime", so C+ #38's `refs` list C+ #38 beside C+ #38.1, as C+ #46's list C+ #46 beside C+ #46.1 |
