# Dungeon runs: the plan

2026-11-01 · written from the owner's chat of the same day and two rounds of the owner's answers (Jack
Goetzmann, with Maximoose Goetzmann)

> **Status: the plan of record, not yet the spec.** SPEC is still the only source of rules (CLAUDE.md):
> the mode gets its own spec section and rulings when it is built. A name marked _placeholder_ is to be
> replaced, and a number marked _strawman_ is to be tuned. What is still open is in
> [DR13](#dr13-open-questions).

## Contents

- [DR0. The mode in one page](#dr0-the-mode-in-one-page)
- [DR1. Sandbox and Campaign](#dr1-sandbox-and-campaign)
- [DR2. Seeds](#dr2-seeds)
- [DR3. The starting deck](#dr3-the-starting-deck)
- [DR4. The map](#dr4-the-map)
- [DR5. The fights](#dr5-the-fights)
- [DR6. After a fight](#dr6-after-a-fight)
- [DR7. The tiles](#dr7-the-tiles)
- [DR8. Curses](#dr8-curses)
- [DR9. Score, currencies and leaderboards](#dr9-score-currencies-and-leaderboards)
- [DR10. The extended campaign](#dr10-the-extended-campaign)
- [DR11. Where it lives](#dr11-where-it-lives)
- [DR12. Build order](#dr12-build-order)
- [DR13. Open questions](#dr13-open-questions)

## DR0. The mode in one page

A single-player run in the style of Slay the Spire, played against the AI.

1. You pick a starting deck of 10 cards and up to 8 points of artifacts.
2. You climb a seeded map of three acts, about 16 fights: normal enemies, elites, ghosts, two mini-bosses
   and the final boss, with shops, campfires, events, chests and shrines between them.
3. Your HP carries from fight to fight, and one lost fight ends the run.
4. After every fight you open one of four packs and keep 3 of its cards, so the deck grows. Shops,
   events, campfires and artifacts thin it.
5. Beating a mini-boss lets you extract a card, and beating the final boss lets you extract an artifact.
   What you extract leaves the run at once and is unlocked for every future starting deck.
6. The run's score pays a permanent currency, which makes your starting slots and artifacts better.
7. The extended campaign carries on past the final boss into harder worlds.

## DR1. Sandbox and Campaign

The dungeon run screen offers two ways to play:

- **Sandbox.** Offline, no account, open to anyone. A run plays in the browser as a practice game does and
  never touches the server. Custom cards are allowed and many limits are lifted, for fun. It earns nothing,
  and its stats and leaderboards stay on the device.
- **Campaign.** Signed in. The account holds the unlocks, the permanent currency and the slots' levels. A
  finished run is checked by the server before it pays, as practice games already are (§14.8, R1303000),
  and its stats feed the leaderboards. The server is involved at a run's start and end, and for ghosts, not
  during it. The Campaign is how the mode rewards making an account.

An extracted card goes into a Campaign starting deck only if the account owns it (§14's collection).

## DR2. Seeds

- Every run has a seed of 6 letters, and the seed deals everything: the map, the enemies and their decks,
  the ghosts, the events, the shops, the packs and every drop. Two players who make the same choices on the
  same seed see the same things. Their starting decks may differ.
- A seed can be shared: "I did this on this hard map, try it."
- **The daily run:** a seed hashed from the date. The day's later runs append a number to it, so everyone's
  second run of the day is the same too.
- Any other run draws a random seed.

## DR3. The starting deck

**Cards.** Always 10 cards, in 10 slots. A slot takes cards up to its rarity:

| Slot | Takes |
| --- | --- |
| Common | a Common |
| Rare | a Common or a Rare |
| Epic | a Common, a Rare or an Epic |
| Anything | any card |

The slots get better with the starting deck's level, bought with the permanent currency: Rare slots become
Epic slots, Common slots become Anything slots, and so on. The deck never grows past 10. A strawman ladder,
to be tuned (DR13):

| Level | Common | Rare | Epic | Anything |
| --- | --- | --- | --- | --- |
| 1 | 5 | 4 | 0 | 1 |
| 2 | 4 | 4 | 1 | 1 |
| 3 | 3 | 4 | 2 | 1 |
| 4 | 2 | 3 | 3 | 2 |
| 5 | 1 | 2 | 4 | 3 |

The cards a starting deck may hold are about 20 unlocked at the start (the simplest Training Grounds
Commons and Rares, placeholders until the designer's list) and every card extracted since.

**Artifacts.** Up to 8 points of them. An artifact's tier is its strength and its price:

| Tier | Points |
| --- | --- |
| Bronze | 1 |
| Silver | 2 |
| Gold | 3 |
| Platinum | 4 |
| Diamond | 5 |

A few artifacts start unlocked. Final bosses unlock the rest, one extraction at a time.

## DR4. The map

- Three acts. Act 1 ends at mini-boss 1, act 2 at mini-boss 2 and act 3 at the final boss, about 16 fights
  in all.
- A branching map: you pick your path row by row and see the whole act ahead, elites included.
- The node kinds: Fight, Elite, Ghost, Mini-boss, Final boss, Shop, Campfire, Event, Chest and Shrine.
- A campfire always stands on the row before each boss.

## DR5. The fights

- Each fight is a practice game against the AI. Your HP carries in, and one lost fight ends the run.
- **An enemy's strength is an artifact it carries.** _Looming Presence_ gives the Medium AI's resources
  (+1 mana, +1 opening card) and _Overwhelming Presence_ the Hard AI's (+1 mana up to 7, 2 draws a turn),
  both through the handicaps practice already uses (R180). Elites and bosses carry them.
- **Enemy decks** are random for now.
- **Ghosts** are the decks of real players who died around the same point of a run, plausibly what a
  player would bring and never boss cards. They come from friends' runs, and a seed can pre-seed some.
  Until snapshots exist, a ghost's deck is random. Later, snapshots are taken of signed-in players' decks
  and positions as they play.
- **Mini-bosses** carry strong artifacts and always drop one, which any roll can also give.
- **The final boss** drops its own preseeded artifact, Unobtainable: nothing else gives it, and it cannot
  be extracted.

Strawman enemy hero health: normal 20, elite 30, mini-boss 40, final boss 50.

## DR6. After a fight

1. **Currencies:** Coins and Tickets (DR9).
2. **An artifact:** a 10% chance after a normal fight, 25% after an elite, always after a mini-boss (one of
   its strong ones) and after the final boss (its own).
3. **Four packs:** the beaten enemy's deck, a random one, one of a set and one of a type, tag or rarity.
   You open one and keep 3 of its cards. Artifacts let you skip a pack or keep more.
4. **Extraction:** after a mini-boss, one card of your deck; after the final boss, one artifact you hold,
   never the boss's own. It leaves the run at once and is unlocked forever.

## DR7. The tiles

- **Shop:** cards and artifacts for Coins; upgrades for Tickets; removing a card.
- **Campfire:** one of: heal, make a card Radiant, remove 2 cards, or buff a card.
- **Event:** a short story with choices. Placeholders for now.
- **Chest:** Coins, Tickets, sometimes an artifact.
- **Shrine** (a monastery or a statue): a god offers a blessing for the rest of the run.

## DR8. Curses

Curses are tiers, chosen at the start of a run, like Slay the Spire's Ascension levels. Each is worth
points, as Hades' Heat is, and stacking more of them is the challenge. Curse points raise the score, and so
the permanent currency a run pays.

## DR9. Score, currencies and leaderboards

**Currencies.** Code names each by a plain id and shows its name through translations, so the web and a
future Roblox version can call them differently.

| Id | Placeholder name | Where | What it does |
| --- | --- | --- | --- |
| `coins` | Coins | inside a run | buys cards and artifacts at shops |
| `tickets` | Tickets | inside a run | upgrades artifacts and cards |
| `embers` | Embers | the account (the device in Sandbox) | raises the starting deck's level, upgrades artifacts, buys buffs |
| (Jlore) | Jlore | the account | unchanged: the cosmetic currency (§14) |

Embers are earned only by playing: a run pays them from its score.

**Score**, a strawman: points for each floor reached, each elite and boss beaten and for beating the final
boss, times (100 + the curse points)%. Turn count and time are reported beside it, not in it.

**Leaderboards:** the deepest floor, the most curse points, the top score and the fastest win, with a timer
on screen. Sandbox boards stay on the device; Campaign boards hold checked runs.

**Stats** kept for every run: nodes visited; fights, elites and bosses won; damage taken and healed; each
currency earned and spent; cards kept, bought, removed, upgraded and buffed; packs skipped; artifacts
taken and upgraded; blessings taken; events seen; chests opened; the time and the turns played.

## DR10. The extended campaign

After the final boss, the run goes on into the next world, where every enemy is harder. You keep all your
artifacts. Your deck becomes 10 cards again: 5 you choose from the run's deck and 5 of the 10 you started
with.

## DR11. Where it lives

- **The run's rules: pure Rust,** seeded like the engine. The map, the states, the packs, extraction, the
  slots, curses, the score and the stats. A run folds back from its seed and its log, so the server can
  check a Campaign run by replaying it, as it checks practice games. It goes in a new pure crate,
  `crates/run`, or a module of the engine beside `duos`, which already wraps plain games (DR13).
- **The fights:** practice games whose setup the run writes: the carried HP, the enemy's handicap and its
  deck, and the run's Radiant and buffed cards (a new setup field in the engine).
- **The client:** the dungeon run screen (Sandbox, Campaign), the map, the tiles, the packs, the artifact
  tray beside the hero, the timer and the leaderboards. Sandbox runs in the practice worker.
- **The server (Campaign only):** the account's unlocks, Embers and levels; checking a finished run;
  ghost snapshots; the leaderboards.

## DR12. Build order

Each line is one issue; the first playable run is M1 to M3, in the Sandbox.

| # | Issue | After |
| --- | --- | --- |
| M1 | The run's core, offline: seeds, the map, the states, the packs, extraction, the slots, artifact points, curse tiers, the score and the stats, on placeholder content | — |
| M2 | Run fights: the carried HP, Looming and Overwhelming Presence, Radiant and buffed run cards in a game's setup | M1 |
| M3 | The Sandbox screen: the map, the tiles, the packs, the artifact tray, the timer, local leaderboards | M1, M2 |
| M4 | Artifacts in fights (Combat) and on the map (Economic) | M2, M3 |
| M5 | The tiles in full: shop, campfire, events, chests, shrines | M3 |
| M6 | The Campaign: the account's unlocks, Embers and levels, checked runs, leaderboards | M1 |
| M7 | Ghosts: snapshots and friends | M6 |
| M8 | The extended campaign | M5 |
| M9 | The designer's content: cards, bosses, artifacts, events, gods | M4 |

## DR13. Open questions

1. **The slots at level 1** and how the ladder climbs (DR3's table is a strawman).
2. **Artifacts:** 8 points and also at most 3 artifacts, or 8 points alone?
3. **Sandbox limits:** which are lifted? Every card and artifact unlocked? Does the Sandbox keep its own
   unlocks on the device, apart from the Campaign's?
4. **"Buff a card"** at a campfire: what buff? +1/+1, a keyword, something else? Is that what Tickets'
   "enchantment" upgrades are?
5. **The score:** what each part is worth.
6. **The extended campaign:** does HP reset between worlds? Do curses stay? Do the worlds go on forever? Do
   they look different?
7. **The mode's name.** _Dungeon Run_ until then.
8. **Where the code goes:** a new crate, `crates/run`, or a module of the engine.

## Appendix: what the codebase already gives

| What the mode needs | What exists | Where |
| --- | --- | --- |
| A fight against the computer, with no server | Practice: the engine and the AI in a Web Worker; a game replays from `(seed, decks, handicaps, log)` | §9.9, R187, `apps/web/src/practice/` |
| Harder enemies | The handicap per seat: deck size, extra mana, extra opening cards, extra draws; Medium and Hard are only handicaps | R180, `Handicap` in `crates/engine/src/config.rs` |
| HP that carries from fight to fight | `Handicap.hero_health` starts a hero at a set health | R290 |
| A deck that is not 25 cards | `Handicap.deck_size` and `Handicap.deck_copies` | R180, R2433001 |
| Themed random enemy decks | The AI's deck draw is curve- and tag-aware and can lean on a set | §9.9, R1370 |
| Boss cards | Two reserved single-player sets, Boss and Boss-X, in `SetName` and `SINGLE_PLAYER_SETS` | R1846001 |
| Upgrading a card | Every card has a Radiant face | §5 |
| A lasting change stamped on one card | Enchantments that ride a card instance (E39) | §10.1 |
| Paying the account for a single-player game | Practice seeds and claims: the server folds the log before it pays | R1303000, §14.8 |
| Friends | Accounts, friends and Play together | §9.4, §9.5 |
| An AI that pilots any deck | `crates/ai`, the same search at every tier | §9.9 |
