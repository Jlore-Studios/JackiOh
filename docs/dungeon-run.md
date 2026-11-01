# Dungeon runs: the plan (draft)

2026-11-01 · written from the owner's chat of the same day (Jack Goetzmann, with Maximoose Goetzmann)

> **Status: a draft for the designer to answer.** Nothing here is a rule. SPEC is still the only source of
> rules (CLAUDE.md), and the mode gets its own spec section and rulings once the questions in
> [DR12](#dr12-the-questions) are answered. Where the chat said nothing, the plan proposes a **default**,
> and every default is also a question in DR12. Answer by number ("Q11: B, Q36: default, …"). A bare
> "default" accepts the proposal.

## Decided so far

The owner's answers to round 1 (2026-11-01). Where they disagree with a default below, they win; the
sections below are rewritten from them once round 2 is answered.

- **Offline.** A run is played in the browser like a practice game against the AI: no account and no
  server. Its stats and leaderboards stay on the device. Later, a signed-in player's runs may be checked
  so that they earn rewards ("make an account to save your progress"), and a competitive mode may come.
- **Seeds.** Every run has a seed that deals its map, bosses and events. A seed can be shared so friends
  play the same map ("I did this on this hard map, try it").
- **Extraction.** An extracted card or artifact is unlocked for every future starting deck, forever. It
  leaves the current run at once: this run's strength traded for progress. A mini-boss gives a card
  extraction; the final boss gives an artifact extraction.
- **The starting deck is slots, not points.** Each slot takes cards up to a rarity: a Common or Rare slot
  takes a Common or a Rare, an Epic slot an Epic or lower, an Anything slot any card. About 10 slots at the
  start (8 Rare and 2 Anything, to confirm). Levels turn Rare slots into Epic ones and add Anything
  slots. The cards unlocked at the start are about 20 of the simplest Training Grounds Commons and Rares,
  placeholders until the designer's list.
- **Starting artifacts.** Up to 3, under an artifact point budget the designer will set.
- **The deck grows.** You keep 3 cards after every fight, so the deck gets big. Shops, events, campfires
  (the rest tiles) and artifacts thin it.
- **Custom cards.** A for-fun mode allows custom cards and lifts many limits. It earns nothing.
- **Curses.** Tiers like Slay the Spire's Ascension, each worth points as Hades' Heat is; stacking more of
  them is the challenge.
- **Currencies.** Robux and Tix are spent inside a run. Jlore stays the account's cosmetic currency. A new
  permanent dungeon currency (its name to come) upgrades the card slots and artifacts and buys buffs; it
  may be sold later.
- **Leaderboards.** Deepest floor reached, most curses stacked, a score, and the fastest win, with a timer
  on screen.
- **Content.** Placeholder cards and temporary bosses until the designer sends the lists.

## Contents

- [Decided so far](#decided-so-far)
- [DR0. What was asked](#dr0-what-was-asked)
- [DR1. What the codebase already gives](#dr1-what-the-codebase-already-gives)
- [DR2. The run's shape](#dr2-the-runs-shape)
- [DR3. Your deck](#dr3-your-deck)
- [DR4. The fights](#dr4-the-fights)
- [DR5. After a fight](#dr5-after-a-fight)
- [DR6. Artifacts](#dr6-artifacts)
- [DR7. The tiles](#dr7-the-tiles)
- [DR8. Ghosts](#dr8-ghosts)
- [DR9. The extended campaign (prestige)](#dr9-the-extended-campaign-prestige)
- [DR10. Where it lives](#dr10-where-it-lives)
- [DR11. Build order](#dr11-build-order)
- [DR12. The questions](#dr12-the-questions)

## DR0. What was asked

The chat, item by item:

- A single-player run in the style of Slay the Spire, roughly 16 fights from the start to the final boss.
- The fights: normal enemies, elites (about 30%), 2 mini-bosses, ghosts (snapshots of other players who
  played the same seed, "the hash", and died) and the final boss.
- The other tiles: shops, rest tiles, events (story dialog), chests, and monasteries or statues where gods
  give blessings.
- Two currencies, Robux and Tix, paid after each fight. Robux buys new enchantments and artifacts at the
  shop. Tix upgrades enchantments and artifacts.
- After each fight, a choice of 4 packs to open: the boss's, a random one, one of a set, and one of a type,
  tag or rarity. You keep 3 of its cards, up to 5.
- Artifacts: passives that sit beside your hero, seen through a button. Two kinds, Combat and Economic. At
  least 3 artifacts change the pack pick (skip it, take more).
- Artifact drops: a 10% chance after a normal fight, 25% after an elite. Mini-bosses and the final boss
  always give one, chosen ahead of time by the seed. Some boss artifacts are tagged Unobtainable.
- An extended campaign: after the final boss you prestige into the next world, keeping 5 cards of your deck,
  and every enemy gets harder.
- For now, a plan and no code (Squishy's usage was at 6%), with plenty of questions to find the intent.

How this plan reads three of the chat's words. Each reading is also a question:

- **"16 bosses long", "normal bosses":** "boss" means any fight. A run is 16 fights, and a "normal boss" is
  a normal enemy (Q1, Q26).
- **"After each boss fight you pick one of 4 packs":** after every fight you win, not only the bosses (Q26).
- **"The hash":** the run's seed, the code that makes two runs the same map with the same rolls (Q62).

## DR1. What the codebase already gives

Practice already holds most of the parts, so the mode needs less engine work than it looks:

| What the mode needs | What exists | Where |
| --- | --- | --- |
| A fight against the computer, with no server | Practice: the engine and the AI in a Web Worker; a game replays from `(seed, decks, handicaps, log)` | §9.9, R187, `apps/web/src/practice/` |
| Harder enemies | The handicap per seat: deck size, extra mana (`manaBonus`, `manaCap`), extra opening cards, extra draws. Easy, Medium and Hard differ only in it | R180, `Handicap` in `crates/engine/src/config.rs` |
| HP that carries from fight to fight | `Handicap.hero_health` starts a hero at a set health (the tutorial's 20); today it is only ever lower than 30 | R290 |
| A deck that is not 25 cards, or holds copies | `Handicap.deck_size` and `Handicap.deck_copies` | R180, R2433001 |
| Hand-built enemy decks | The tutorial's fixed decks and seeds; the three named practice decks (Human Vanguard, Blitz, Fortress) | §9.10, `apps/web/src/practice/decks.ts` |
| Themed random enemy decks | The AI's deck draw is curve- and tag-aware and can lean on a set | §9.9 Decks, R1370 |
| Boss cards | Two reserved single-player sets, Boss and Boss-X, already in `SetName` and `SINGLE_PLAYER_SETS`, never in a deck, reached only by `everySet` pools | R1846001, `crates/engine/src/wire/catalog_types.rs` |
| Upgrading a card | Every card has a Radiant face, the game's own upgraded card | §5 |
| A lasting change stamped on one card | Enchantments that ride a card instance through every zone (E39) | §10.1, `docs/classic-sets.md` E39 |
| Other players' snapshots inside your game | Last boards (C+ #29) and Glitch boards: the server hands other players' boards to a game's setup | R417, R678 |
| Paying the account for a single-player game | Practice seeds and claims: the server folds the log before it pays | R1303000, §14.8 |
| An AI that pilots any deck | `crates/ai`, the same search at every tier | §9.9 |

What is missing:

1. **The run:** a seeded map, its nodes, the currencies, the rewards, saving it and replaying it.
2. **Combat artifacts inside a fight:** something on a seat that is not a card in a zone, with triggers. A
   new engine concept.
3. **A deck with cards already upgraded:** `CreateGameOptions.decks` is a list of card ids today, so a run
   card made Radiant or enchanted at a rest tile has no way into a fight. A new setup field.
4. **Ghosts:** storing and serving snapshots by seed. New server tables.
5. **The client:** the map, the tiles' screens and the artifact tray. A new route.
6. **The content:** enemies, bosses, artifacts, events, gods and blessings. The designer's.

## DR2. The run's shape

**Default:** three acts, one map each, each ending at a boss:

| Act | Fights | Ends with |
| --- | --- | --- |
| 1 | 1 to 6 | Mini-boss 1 (fight 6) |
| 2 | 7 to 11 | Mini-boss 2 (fight 11) |
| 3 | 12 to 16 | The final boss (fight 16) |

Each act is a branching map like Slay the Spire's: rows of nodes with several paths, the whole act visible,
a rest tile always on the row before its boss. The node kinds are Fight, Elite, Ghost, Boss (mini or final),
Shop, Rest, Event, Chest, Shrine (monastery or statue) and Unknown (`?`, rolled as you step on it).

The arithmetic at these defaults:

- **Fights:** 16 = 3 boss fights + 13 others. At 30% elites, about 4 elites and 9 normal fights or ghosts.
- **Artifacts from fights:** 9 × 10% + 4 × 25% + 3 ≈ 5 a run, before chests, shops, shrines and events.
  With those, about 7 or 8 (Q45).
- **Cards:** 16 packs × 3 kept = 48 cards picked (Q11 says what that does to the deck).
- **Time:** 16 fights at 8 to 12 minutes is 2 to 3 hours, plus the map (Q6).

## DR3. Your deck

The biggest open choice (Q11). Three models:

- **A, the deck grows (Slay the Spire's).** Every card kept goes into the deck. At 3 kept per fight the deck
  ends near 60 cards unless shops remove cards. This is the model where a "skip" artifact matters most,
  since every card kept dilutes the deck.
- **B, a pool and a 25-card deck (default).** Kept cards go into the run's pool. Before each fight your deck
  is 25 cards from the pool, edited between fights. It fits the game's exact deck size and its singleton
  rule (`MAX_COPIES` is 1), and the prestige's "keep 5 cards" reads cleanly. Skipping a pack trades it for
  currency.
- **C, swap.** The deck stays 25, and every card you keep replaces one.

**Default start:** one of the three named practice decks (Human Vanguard, Blitz, Fortress), each 25 Core
cards (Q10).

## DR4. The fights

Every fight is a practice game whose setup the run writes:

- **Your seat:** a human's resources, the HP the run carried in (`hero_health`), your combat artifacts and
  your deck, upgrades included.
- **The enemy's seat:** a handicap by node kind and world, a deck, and for a boss or a ghost its own combat
  artifacts.

**Default enemy ladder**, every number a named constant of the run's config (CLAUDE.md rule 9):

| Enemy | Deck | Resources (the existing handicap levers) | Hero HP | Artifacts |
| --- | --- | --- | --- | --- |
| Normal | Themed random draw | Easy's | 20 | none |
| Elite | Hand-built | Medium's (+1 mana, +1 opening card) | 30 | none or 1 |
| Mini-boss | Hand-built, Boss cards in it | Medium's | 40 | its signature artifact |
| Final boss | Hand-built, Boss cards in it | Hard's (+1 mana up to 7, 2 draws) | 50 | 2 |
| Ghost | The snapshot's deck | A human's | 30 | the snapshot's combat artifacts |

A lost fight ends the run (Q19). The extended campaign raises every row by a step per world (DR9).

## DR5. After a fight

In this order:

1. **Currencies.** Robux and Tix, by node kind (Q35).
2. **The artifact roll.** 10% after a normal fight, 25% after an elite, always after a boss. Rolled from the
   run's seeded `Rng`, so one seed gives the same artifacts at the same nodes. A boss's artifact is known
   before the fight, shown on the map (Q46).
3. **The pack choice.** Four packs, each labelled:
   - **Boss:** cards of the deck you just beat (Q27).
   - **Random:** any card of the sets that ship.
   - **Set:** one set, named on the pack.
   - **Type, tag or rarity:** one of those, named ("Spells", "Robots", "Epic or better").

   You open one. It shows N cards and you keep 3; artifacts move that from 0 to 5 (Q31, Q32).

## DR6. Artifacts

Two kinds, and the split decides where each lives:

- **Combat artifacts** work inside a fight: they live in the engine, as a list on the seat set up when the
  game is created and replayed like a handicap. Each has one script file with hooks that return effects, as
  a card's do (CLAUDE.md rules 5 and 6). Illustrations only: "your hero starts each fight with 3 Armor",
  "the first Spell you play each fight costs (1) less".
- **Economic artifacts** work on the map: rewards, shop prices, pack sizes, skips, currencies. They live in
  the run's code and never touch a fight. Illustrations: "shop prices are 20% lower", "+5 Robux after each
  fight".

**The pack artifacts** (the chat asked for at least 3), default proposals: _Picky_ (skip a pack for Robux),
_Greedy_ (keep one more card, up to 5), _Second Look_ (open a second pack), _Wide Net_ (a pack shows 2 more
cards).

**Default levels:** Tix upgrades an artifact from I to II to III (Q41).

**Default Unobtainable:** a tag on a boss's signature artifact: it drops only from that boss, never from a
shop, a chest or a random roll (Q47).

**Default display:** a tray of icons beside the hero portrait, a button opening the list with each
artifact's text, an icon flashing when it fires; both trays public, since nothing about them is hidden (Q52).

## DR7. The tiles

Defaults, each a question in DR12:

- **Shop:** cards and artifacts for Robux; upgrades for Tix; remove a card (model A) or heal for Robux. Stock
  rolled by the seed, no restock.
- **Rest:** heal 30% of your max HP, or upgrade one card to Radiant. One or the other.
- **Event:** a short story with 2 or 3 choices, each an outcome (HP, currency, a card, an artifact, a curse).
  The designer writes the text.
- **Chest:** a free reward of Robux, Tix, cards or an artifact, in three sizes.
- **Shrine (monastery or statue):** a god offers a blessing, a buff for the rest of the run, sometimes at a
  price (HP, Robux, a curse). One blessing per god.

## DR8. Ghosts

**Default flow:**

1. When a player dies on seed S at fight k, the server stores a snapshot: their deck, their combat
   artifacts, their username and portrait.
2. A Ghost node on seed S at depth d serves the snapshot of someone who died on seed S at or near d. When
   nobody has, the node is an elite instead.
3. The AI pilots the ghost's deck at a human's resources with their combat artifacts.
4. Beating it pays Robux and lets you take one card of their deck.

A seed few people play has no ghosts, so **default seeds:** a daily seed everyone shares, which is where
ghosts gather, and a random seed for every other run, shareable as a code (Q69). Ghosts need an account and
the server; a guest's run always gets the elite.

## DR9. The extended campaign (prestige)

**Default:** chosen when the run starts, Normal (one world, ends at the final boss) or Extended. After an
Extended run's final boss:

- you keep 5 cards of your deck, chosen by you, and start the next world from a starter deck plus those 5;
- artifacts, blessings and currencies do not carry (Q71);
- every enemy's row in the DR4 ladder rises one step (Q74);
- the worlds go on without end, and a leaderboard ranks the deepest world reached (Q72, Q76).

## DR10. Where it lives

- **`crates/run` (new, pure):** the run's state, its actions and `reduce`; the seeded map; the tiles; the
  rewards and shop stock; the economic artifacts; the run's log, which folds back from `(run seed, run log,
  each fight's result)` as a game folds from its log. It uses the engine's `Rng` and the catalog query. A
  fifth pure crate changes CLAUDE.md rule 4's list (Q88).
- **The engine:** a run field on `CreateGameOptions` holding each seat's combat artifacts and the deck's
  upgraded cards, replayed like `handicaps`. A game without it hashes as before (R180's pattern). The
  artifacts' hooks; both trays in `view_for`.
- **The AI:** reads the combat artifacts (they are public) and plays with them; a bigger budget for bosses
  (Q25).
- **The client:** a `/run` route with the map and the tiles' screens; the deck builder and the pack opening
  (`apps/web/src/packs`) reused; the artifact tray beside the portrait in `Game.tsx`; the fights in the
  practice worker.
- **The server:** daily seeds; saved runs for accounts; checking a run by folding it, as practice claims
  are; the ghosts' tables; the account's rewards through the ledger (§14.11); a leaderboard.
- **The spec:** a new section for the mode, its rulings in its issue's block.

## DR11. Build order

Proposed issues. The first playable run is after M1 to M3: fights, packs and a map, with no artifacts yet.

| # | Issue | After | Difficulty |
| --- | --- | --- | --- |
| M0 | Design lock: DR12's answers written as the mode's spec section | the answers | the owner's |
| M1 | The run's core: `crates/run`, the seeded map, the nodes, the currencies, the packs, the run's log and its fold | M0 | hard |
| M2 | Run fights: the engine's run setup (carried HP, upgraded deck cards), the enemy ladder | M1 | hard |
| M3 | The client: `/run`, the map, the pack pick, the deck between fights, the run saved on the device | M1, M2 | hard |
| M4 | Artifacts: economic in the run, combat in the engine, the tray, the AI playing with them | M2, M3 | hard |
| M5 | The tiles: shop, rest, events, chests, shrines | M3 | medium |
| M6 | Content: enemies, elites, 2 mini-bosses, the final boss, the Boss set's cards, the artifacts, the events | M4, the designer's list | hard |
| M7 | The server: daily seeds, saved runs, checking a run, the account's rewards | M1 | hard |
| M8 | Ghosts | M7 | hard |
| M9 | The extended campaign | M6 | medium |

## DR12. The questions

Each has the plan's default after it. Answer by number.

### The run's shape

1. Is "16 bosses long" 16 fights, the 2 mini-bosses and the final boss among them? Or 16 tiles of any kind?
   _Default: 16 fights, about 26 tiles in all._
2. Three acts, ending at mini-boss 1, mini-boss 2 and the final boss? _Default: yes._
3. A branching map where you choose your path (Slay the Spire), a straight line, or something else?
   _Default: branching._
4. Can you see the whole act ahead, or only the next row? _Default: the whole act._
5. Elites at 30%: 30% of the fights that are not bosses, placed on the map where you can see and avoid them,
   or a surprise roll on a normal fight? _Default: on the map, visible, about 4 a run._
6. How long should a run take? 16 fights at 8 to 12 minutes is 2 to 3 hours. Should run fights be shorter
   (less enemy HP, smaller decks)? _Default: normal enemies at 20 HP, about 2 hours a run._
7. Can a run be saved and finished later? On another device? _Default: yes; on the server for accounts, on
   the device for guests; one run at a time._
8. Can you abandon a run? _Default: yes; it counts as a death (and leaves a ghost)._
9. What is the mode called? "Dungeon Run", or a JackiOh name?

### Your deck

10. How does a run start: one of the three named practice decks, one of your saved decks, a small fixed
    starter, or a draft? _Default: a named practice deck._
11. Which deck model: **A** (the deck grows, Slay the Spire), **B** (kept cards go into a pool, and your
    25-card deck is built from it between fights) or **C** (the deck stays 25, and each kept card replaces
    one)? _Default: B._
12. If A: how big is the starting deck, and can a shop remove cards? _Default: 15 cards; removal for Robux._
13. Copies: the game is singleton (one copy of each card). Can a run deck hold copies? Does a pack offer a
    card you already have? _Default: singleton; a pack never offers a card already in your pool._
14. Does a run use only the cards you own, or every card? _Default: every card of the sets that ship; the
    run is apart from your collection._
15. Does a card from a run ever reach your collection? _Default: no._

### HP and the fights

16. Does your hero's HP carry from fight to fight? What is the max? _Default: it carries; 30 max; rest tiles
    heal._
17. Who goes first in a run fight, and is The Coin dealt as usual? _Default: as in practice._
18. The mulligan as usual? _Default: yes._
19. Does losing one fight end the run? Or are there lives or a retry? _Default: it ends the run._
20. Are the existing handicap levers (more mana, more draws, more HP, deck size) the right way to make
    enemies harder? _Default: yes, as DR4's ladder._
21. Normal enemies: named characters with hand-built decks, or random decks with a theme (a set, a tag)?
    _Default: themed random decks; hand-built for elites and bosses._
22. What makes a boss a boss: more HP, cards of the Boss set, its own artifacts, a boss-only Heroic Power,
    a second phase, a rule that changes the fight? _Default: a hand-built deck with Boss cards, its
    signature artifact and more HP._
23. Are the 2 mini-bosses and the final boss fixed, or drawn from a pool each run? _Default: a pool of 3
    for each, the seed picks._
24. Who is the final boss? Is there a story behind the dungeon?
25. Should the AI think harder against you in a boss fight (a bigger search budget)? _Default: twice the
    budget for bosses._

### After a fight

26. "After each boss fight you pick one of 4 packs": after every fight, or after the bosses only?
    _Default: every fight._
27. The "boss" pack after a normal fight: the cards of the deck you just beat? _Default: yes._
28. The "random" pack: every set that ships? The Boss set too? _Default: the sets that ship._
29. The "set" pack: a set the seed rolls, or one you choose? _Default: the seed rolls it, named on the
    pack._
30. The "type, tag or rarity" pack: one of the three rolled each time? _Default: yes._
31. How many cards does a pack show? _Default: 8._
32. "Pick 3 up to 5": do you keep exactly 3, with artifacts raising it to 4 or 5? Or do you choose anywhere
    from 3 to 5? And "skip": take none and get something else instead (Robux, HP)? _Default: keep exactly
    3; Greedy raises it; Picky lets you skip for Robux._
33. Rarity odds in a run pack: the account packs' (§14.3), or the run's own, better deeper in the run and
    after an elite? _Default: the run's own, rising with depth._
34. Radiant cards in packs? _Default: a small chance, higher in elite and boss packs._
35. How much Robux and Tix does each kind of fight pay? _Default: normal 15 and 1, elite 30 and 2, boss 60
    and 5, to be tuned._

### The currencies

36. Are Robux and Tix only for the run (gone when it ends, like Slay the Spire's gold), or kept on the
    account? _Default: only for the run._
37. Robux buys cards, artifacts and enchantments; Tix upgrades enchantments and artifacts. Does Tix also
    upgrade a card to Radiant? Does Robux buy healing or card removal? _Default: yes to both._
38. Robux is Roblox's trademark, and Tix was Roblox's second currency. Fine while JackiOh is private, a risk
    if it goes public. Keep the names, or rename before launch? _Default: the code uses plain ids and
    shows the names from data, so renaming later is one line._
39. In the extended campaign, do any currencies carry into the next world? _Default: no._

### Enchantments and upgrades

40. What is an "enchantment"? (a) a card's Radiant upgrade, (b) a stone you put on one card of your deck
    (+1 Attack, Taunt …) that stays on it for the run, (c) another name for an artifact, or (d) a buff on
    your hero? _Default: (b), with the Radiant upgrade apart._
41. How many levels can an artifact or an enchantment be upgraded through? _Default: 3 (I, II, III)._
42. Rest tile: heal, or upgrade a card to Radiant, or both? _Default: one or the other._

### Artifacts

43. The split: Combat artifacts work inside a fight, Economic ones on the map (rewards, shop, currency,
    packs). Is that what you meant? _Default: yes._
44. How many artifacts at launch? _Default: 30 (15 Combat, 15 Economic), the boss ones among them._
45. Besides fights, do chests, shops, shrines and events give artifacts? _Default: yes; about 7 or 8 a run
    in all._
46. "Preseeded": each boss always drops its own signature artifact? Or the seed picks one from a pool? Do
    you see it on the map before the fight? _Default: each boss has its signature artifact, shown on the
    map._
47. "Unobtainable" means what? (a) it drops only from its boss, never from a shop, chest or random roll;
    (b) it can never leave the run; (c) a visible badge; (d) something else. _Default: (a)._
48. When an artifact drops, do you take it, or choose 1 of 3? _Default: choose 1 of 3 from a boss; take it
    otherwise._
49. Is there a cap on how many you hold? _Default: no._
50. Artifact rarities? _Default: Common, Uncommon, Rare, Boss._
51. Do enemies have artifacts too? _Default: bosses, some elites and ghosts._
52. The tray: icons always beside the portrait with a button for the details, or everything behind the
    button? Can your opponent see yours? _Default: icons beside the portrait, the button opens the list,
    both public._
53. Does an artifact flash or animate when it fires? _Default: yes._
54. Bad artifacts or curses? _Default: only from shrines and events, as a price._
55. Can you sell or drop an artifact? _Default: no._

### The tiles

56. Shop: what is on sale, how many of each, and does it restock? _Default: 5 cards, 3 artifacts, 1
    enchantment, an upgrade service, a heal; no restock._
57. Rest: as Q42. Anything else (remove a card, dig for an artifact)? _Default: heal or upgrade only._
58. Events: who writes them, and how many at launch? Characters from the sets (xxFreshJack, Shockwave447,
    the family)? _Default: 10 events; you write the text, I wire the outcomes._
59. Chests: always free, or some locked (a key), or some a trap that starts a fight? _Default: always free,
    three sizes._
60. Monasteries and statues: which gods? A pantheon of JackiOh's own? How many?
61. What is a blessing, against an artifact? A run-long buff, with a price? Choose 1 of 3? Do blessings
    stack? _Default: a run-long buff, choose 1 of 3, one per god, some with a price._

### Ghosts

62. "The hash": the run's seed, so ghosts come from players who played the same seed? _Default: yes._
63. Only players who died, or winners too? _Default: only deaths; a winner could stand in as an alternate
    final boss later._
64. What does a snapshot hold: the deck, artifacts, blessings, the HP they died with? _Default: the deck
    and combat artifacts, at full HP._
65. How strong is a ghost: a human's resources, or harder? _Default: a human's._
66. Where does a ghost appear: its own node kind, or in place of a fight? With no ghost for that seed?
    _Default: its own node; an elite when there is none._
67. What does beating a ghost pay? _Default: Robux and a card of their deck, your pick._
68. Show the ghost's username and portrait? Prefer friends' ghosts? Can a player opt out of being a
    ghost? _Default: show them, prefer friends, opt-out in settings._
69. Seeds: a daily seed everyone shares? Seeds you can share as a code? _Default: both._

### The extended campaign

70. Is it chosen when the run starts (Normal or Extended)? _Default: yes._
71. "Keep 5 cards": chosen by you, from your deck (or pool)? And artifacts, blessings, HP? _Default: you
    choose 5 cards; everything else resets._
72. How many worlds: a fixed number or endless? _Default: endless._
73. What changes between worlds: the look, the enemy pools, a set per world (a Core world, a Meditative
    world …)? _Default: each world themed by one set._
74. "All enemies get harder": by how much a world? _Default: +5 enemy HP a world, +1 mana from world 2, an
    extra draw from world 3._
75. Does a deeper world pay more (Robux, Tix, better packs)? _Default: yes._
76. A leaderboard? By what: deepest world, fastest win, the daily seed? _Default: the deepest world, and
    the daily seed's._
77. Does world 2 start from a starter deck plus the 5? _Default: yes._

### The account and the economy outside the run

78. Does a run need an account? _Default: guests can play; ghosts, saving across devices and rewards need
    an account._
79. What does a run pay the account: Jlore, XP? Under the daily cap (§14.8)? _Default: Jlore and XP by
    the depth reached, under the cap._
80. Cosmetics for beating a boss the first time (a card back, a portrait)? _Default: yes._
81. Do runs count for daily quests? _Default: yes._
82. Should the server check a run before it pays, as it checks practice games? _Default: yes._

### The content and the tone

83. Who designs the bosses, artifacts and events? Will you write a list file like the set files
    (`JackiOh_*.md`), for a set plan to be made from it? _Default: yes, a `JackiOh_DungeonRun.md`._
84. The theme: what is the dungeon? Roblox-flavored, a school, the friend group's world?
85. What is the Boss set for, and what is Boss-X? _Default: Boss holds the bosses' cards; Boss-X the
    prestige worlds' harder versions._
86. Do boss cards ever become collectible for normal play? _Default: no._
87. A co-op run, two players like Duos (§13)? _Default: later, not at launch._

### How it is built

88. The run's logic in a new pure crate, `crates/run` (CLAUDE.md rule 4's list grows to five), or inside
    the engine? _Default: a new crate._
89. Which comes first: a playable run with no artifacts (M1 to M3), or artifacts first? _Default: a
    playable run first._
90. Who builds it: the night bot and Squishy from issues, or a person? _Default: issues for the bots,
    opened once M0 is answered._
