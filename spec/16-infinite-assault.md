## 16. The Infinite Assault

The Infinite Assault is a daily single-player gauntlet against the AI: every UTC day deals 20 levels, each one game, the Assault growing stronger at every level and bringing a boss, a Commander, on levels 5, 10, 15 and 20. Everyone who plays on the same day and catalog version meets the same enemies, decks and upgrades. Win a level to reach the next; lose one and the attempt ends; beating level 20 repels the Assault for the day. Tomorrow it is new.

Every level is an ordinary practice game ([[§9.9]], [[R187]]) whose setup the Assault writes: the enemy's deck, its handicap ([[R180]]) and its deck marks ([[R2702000]]). No card and no game rule knows about the Assault, as none knows about a dungeon run ([[§15]]), and sections two to eight hold inside each fight. A game without the Assault's setup hashes as before.

It is a practice mode: it plays in the browser's practice worker, needs no account, records nothing on the server and earns nothing ([[R4582000]]); its fights are kept on the device for replay as practice games are ([[R5125000]]). Its enemies' cards are the Infinite Assault set ([[§8.13]]), a single-player set, open since the mode's release (issue #4597, [[R4577000]]), whose cards an Assault enemy's seat holds by naming the set ([[R4597001]]).

The rules are pure Rust in `crates/run`'s module `assault`, beside the dungeon run, sharing its seeds, its `Rng` use and its fight report, and `crates/run/README.md` is the contract. Every draw goes through the engine's `Rng` from the day's seed ([[§10.7]]). The plan is `docs/infinite-assault.md` (tracker #4576); where it and this section disagree, this section wins.

Every value is a constant of `crates/run/src/config.rs`, and every one is a strawman to be tuned against the Assault's simulator ([[§16.9]]).

| Constant | What it holds | Where |
| --- | --- | --- |
| `ASSAULT_SET` | The set the Infinite Assault deals: Infinite Assault ([[R4577000]]) | [[§16.4]], [[R4579001]] |
| `ASSAULT_COMMANDER_TAG` | The tag a Commander's cards carry: `Commander` ([[R4577001]]) | [[§16.4]], [[R4579001]] |
| `ASSAULT_SEED_PREFIX` | The prefix of a day's seed, `assault-` | [[§16.1]], [[R4579002]] |
| `ASSAULT_LEVELS` | The levels a day: 20 | [[§16.2]], [[R4579000]] |
| `ASSAULT_COMMANDER_LEVELS` | The Commander levels, ranks I to IV in order: 5, 10, 15, 20 | [[§16.4]], [[R4579000]] |
| `ASSAULT_HISTORY_DAYS` | The days of records the device keeps: 30 | [[§16.6]], [[R4581001]] |
| `ASSAULT_BASE_HANDICAP` | The resources every level starts from: Easy's ([[§9.9]]) | [[§16.2]], [[R4579000]] |
| `ASSAULT_HERO_HEALTH` | A normal level's hero health, `(base, per level)`: 20 + the level | [[§16.2]], [[R4579000]] |
| `ASSAULT_COMMANDER_HEALTH` | A Commander level's hero health, `(base, per level)`: 40 + five times the level (30 + twice the level until issue #4783, 30 + five times until the release) | [[§16.2]], [[R4579000]], [[R4597002]] |
| `ASSAULT_COMMANDER_UPGRADES` | The upgrades a Commander level takes after its card step: 2 (1 until the release) | [[§16.2]], [[R4597002]] |
| `ASSAULT_FEWER_OPENING` | The cards fewer in the opening draw on the first levels: 1 ([[R3582000]]) | [[§16.2]], [[R4579000]] |
| `ASSAULT_FEWER_OPENING_LEVELS` | The last level dealing fewer opening cards: 3 | [[§16.2]], [[R4579000]] |
| `ASSAULT_DECK_SIZE` | The deck's size before Bigger Dataset: `DECK_SIZE` (25) | [[§16.2]], [[R4579000]] |
| `ASSAULT_START_SLOTS` | Level 1's deck as Common, Rare, Epic, Legendary and Mythic slots: 19, 6, 0, 0, 0 | [[§16.2]], [[R4579000]] |
| `ASSAULT_CARD_STEP` | The slots a card step promotes: 4 | [[§16.2]], [[R4579000]] |
| `ASSAULT_RARITY_CEILING` | The rarest slot a level may hold, each `(from level, rarest slot)`: Rare from 1, Epic from 4, Legendary from 8, Mythic from 14 | [[§16.2]], [[R4579000]] |
| `ASSAULT_CARD_STEP_PERCENT` | The chance, in percent, that a normal level's escalation is a card step: 50 | [[§16.2]], [[R4579000]] |
| `ASSAULT_DOCTRINE_SHARE` | The percent of a deck's slots dealt from its doctrine: 60 | [[§16.4]], [[R4579001]] |
| `ASSAULT_UPGRADES` | The upgrades, each `(kind, per stack, most stacks, from level, weight)` | [[§16.3]], [[R4579000]] |

### 16.1 The day and its seed

- **The day** is a UTC date written `YYYY-MM-DD`, which the caller passes in: the pure crates read no clock (CLAUDE.md rule 4). Its seed is `ASSAULT_SEED_PREFIX` and the date (`assault-2026-11-24`, `assault_day_seed`); any other shape, and any date no Gregorian calendar holds, is refused ([[R4579002]]).
- **The day's deal** (`assault_day`, an `AssaultDay`) is a pure function of the date and the content ([[§16.4]]): one `Rng` from the day's seed deals, in order, each rank's Commander, then each level's escalations, doctrine, enemy, handicap, deck and marks. The same date and content deal the same day byte for byte, and two dates deal different days.
- **The content folds under its catalog version**, as a dungeon run's does ([[R3496000]]): the device keeps the version with the attempt, and the WebAssembly module answers the content of a carried version (`assault_content_at`) or none.
- **A level's game seed** is `{day seed}-L{level}-A{attempt}` (`assault-2026-11-24-L7-A2`): the same enemy, deck and upgrades on every attempt that day, a fresh shuffle on each, and everyone's nth attempt at a level meets the same shuffle ([[R4579002]]).
- **Midnight never redeals an attempt in progress.** An open attempt folds on the UTC date and catalog version in its file; the new day is offered once it ends or is abandoned, and every attempt after that, Try again's included, is dealt on the current day ([[R4581000]]).

### 16.2 The levels and the escalation

**The base row.** Every level starts from `ASSAULT_BASE_HANDICAP`, Easy's resources ([[§9.9]], [[R180]]), with:

- hero health `ASSAULT_HERO_HEALTH` on a normal level (21 to 39) and `ASSAULT_COMMANDER_HEALTH` on a Commander's (65, 90, 115, 140), plus Redundancy's stacks ([[R290]]);
- `ASSAULT_FEWER_OPENING` card fewer in the opening draw on levels 1 to `ASSAULT_FEWER_OPENING_LEVELS` (`fewer_opening_cards`, [[R3582000]]);
- a deck of `ASSAULT_DECK_SIZE` cards plus Bigger Dataset's, dealt from slots of each rarity, level 1's being `ASSAULT_START_SLOTS`.

**One escalation a level** ([[R4579000]]). Level 1 is the base row alone. Each later level adds one escalation to everything the levels before it added (`AssaultLevel.escalations` holds what it added, `upgrades` everything held):

- **A card step** promotes `ASSAULT_CARD_STEP` of the deck's slots one rarity each, lowest rarity first, never past the level's `ASSAULT_RARITY_CEILING`.
- **An upgrade** is one stack of one upgrade ([[§16.3]]), drawn by the upgrades' weights among those allowed at the level that hold fewer than their most stacks.

A normal level's kind is the seed's, a card step at `ASSAULT_CARD_STEP_PERCENT`; a card step with nothing left to promote under the ceiling is an upgrade instead, and a level with no upgrade left takes a card step. A Commander level takes three escalations ([[R4597002]]): a card step (or an upgrade when no slot can move) and then `ASSAULT_COMMANDER_UPGRADES` upgrades (each a card step when no upgrade is left), and its Commander's cards join its deck ([[§16.4]]).

**It never gets weaker** ([[R4579000]]). No handicap field but health and `fewer_opening_cards` falls from one level to the next; `fewer_opening_cards` drops off after level 3; no upgrade's stacks fall; and the count of slots at or above each rarity never falls (`AssaultLevel.rarities` holds the slots). Health is compared within a kind: a Commander's base is a peak, so the level after it goes back to the normal base and keeps every Redundancy stack. Each level's deck is dealt afresh from its own slots, never the previous level's deck plus a card.

### 16.3 The upgrades

Each upgrade is an `AssaultUpgradeKind`, written in camelCase on the wire, with its row of `ASSAULT_UPGRADES`: its amount per stack, its most stacks, the first level it may come on and its weight. Every one maps onto a field the engine already has, a handicap field of the enemy's seat or one of its deck marks ([[R2702000]]), which `CreateGameOptions` takes per seat.

| Players see | Kind | Effect per stack | Most | From level | Weight |
| --- | --- | --- | --- | --- | --- |
| Expanded Context | `openingCards` | +1 card in the opening draw (`extra_opening_cards`, [[R182]]) | 3 | 4 | 3 |
| Bigger Dataset | `deckSize` | +5 cards in the deck (`deck_size`, [[R184]]); the new slots join the rarity holding the most slots, the lower on a tie | 3 | 2 | 2 |
| Fine-Tuned | `radiant` | 3 more of its deck's cards start Radiant (a deck mark) | 5 | 2 | 4 |
| Reinforced Weights | `buffed` | 3 more of its deck's cards start with one Buff each (a deck mark's `upgrades`) | 5 | 2 | 3 |
| Overclocked | `manaBonus` | +1 crystal ahead of its turn count (`mana_bonus`, [[R181]]) | 2 | 8 | 2 |
| Raised Ceiling | `manaCap` | +1 to its max mana's cap (`mana_cap`) | 3 | 6 | 2 |
| Redundancy | `heroHealth` | +10 hero health (`hero_health`, [[R290]]) | 3 | 2 | 3 |
| Hardened Shell | `heroArmor` | +5 starting Armor (`hero_armor`, [[R2922000]]) | 3 | 2 | 3 |
| Parallel Inference | `extraDraw` | +1 draw each turn (`extra_draws_per_turn`, [[R183]]) | 1 | 12 | 1 |

Fine-Tuned's and Reinforced Weights' cards are each drawn by their own shuffle of the deck's positions, so a card may be both ([[R4579001]]). The fight shows the enemy's upgrades by name, stacks and effect, and players read them as Boosts ([[R4582002]]).

### 16.4 Enemies, doctrines and Commanders

**The content** (`AssaultContent`) is what a day deals from: the set's cards, each with its rarity, its doctrine and whether it is a Commander's, and `crates/run/src/assault.json`'s doctrines and Commanders, read at compile time (CLAUDE.md rule 4). `assault_catalog_content` reads the cards from the catalog by name ([[R4579001]]): the cards of `ASSAULT_SET`, never a token; a card's doctrine is the first of the file's doctrines whose id one of its tags names; a card is a Commander's when it carries `ASSAULT_COMMANDER_TAG`; and a Commander is dealt only when every one of its cards is a Commander card there.

**Doctrines.** The set's eight archetypes, each a tag on its cards ([[R4577001]]): Hallucination, Overclock, Batch, Recursion, Jailbreak, Firewall, Scraper and Singularity. Each has a name, a line players read and four enemy names, in the file. A normal level draws its doctrine by the seed, never the previous level's and never the next level's Commander's, and its enemy is one of its doctrine's names.

**Commanders.** Each has a rank, I to IV, a doctrine or none, and its cards. Level 5 meets a rank I Commander, 10 rank II, 15 rank III and 20 rank IV (`ASSAULT_COMMANDER_LEVELS`), each drawn by the seed before any level is dealt. A Commander level takes its Commander's name and doctrine. A day whose content holds no Commander of a rank, or no doctrine a level may draw, is refused.

**The deck** ([[R4579001]]). A Commander's cards join its deck first, each taking the lowest slot that holds one, so the deck keeps its level's size. The remaining slots are dealt rarest first: `ASSAULT_DOCTRINE_SHARE` percent of them, chosen by the seed, read the level's doctrine (any doctrine's when it has none), then the neutral cards, then any doctrine; the others read the neutral cards, then any doctrine; and a slot whose rarity is exhausted falls to the next rarity down. No other Commander's card is dealt. Each deck holds no card twice and is its handicap's `deck_size` ([[R184]]), so `createGame` takes it as it takes every practice deck.

**The browser deals** from `assault_catalog_content` since the release, a carried version too ([[R4597000]]): the set's cards and the file's twelve Commanders, IA #217 to IA #240, three of each rank, each its Commander card and signature card. `assault_placeholder_content`, which dealt shipped stand-ins until the release (the file's `standInCommanders`, and every other deckable shipped card dealt to the next group in turn, [[R4579001]]), stays a test fixture.

### 16.5 The player's deck

- **A deck is chosen per attempt** and kept for the whole attempt (`AssaultDeckSource`): a saved deck, as practice offers them ([[§9.9]]); a named practice deck, Human Vanguard, Blitz or Fortress; or **the day's Assault deck**.
- **The day's Assault deck** (`assault_day_deck`, the WebAssembly module's) is practice's fresh random deck for a human ([[R258]]), dealt by the AI's deck builder with nothing banned and no set leaned on, from the seed `{day seed}-deck`: the same for everyone that day, and needing no account.
- **The deck is checked when the attempt starts**, against the catalog at the human's deck size and copy limit ([[R184]]): an attempt lasts one day, so a deck the catalog refuses is refused, never filtered as a dungeon run's carried deck is ([[R4579002]], [[R3496000]]). No deck the player brings holds an Infinite Assault card ([[R1846001]], [[R4577000]]).
- **The human plays `HUMAN_HANDICAP` at every level**, its deck's own size, in seat P1, with no carried health and no artifacts, as a dungeon fight's human does ([[R4579002]], [[R4582000]]).

### 16.6 Attempts, checkpoints and the record

**An attempt** (`AssaultState`) is its setup (the day, the deck and its source, the start level and the attempt's number), the level it stands on, its phase (`briefing`, `fight`, `lost`, `repelled` or `abandoned`), the levels it beat and its turns ([[R4579002]]).

- **It starts** at level 1, or at a checkpoint the day's record holds, as the attempt after the record's last. The checkpoints are each Commander level below the top plus one: 6, 11 and 16.
- **A briefing** fights. **A fight's result** (`fightOver`, its win and turns) moves a win to the next level's briefing and ends the attempt `lost` on a loss; a draw at the turn cap is a loss. A fight a Glitch voids never happened ([[R679]]): it reports nothing, and the attempt, still in its fight, deals it again fresh from the same setup ([[R4999000]]). Beating level 20 ends it `repelled`. Abandoning from a briefing or a fight ends it `abandoned`, the page asking first.
- **Every other step is refused** with the state unchanged: another pair of phase and action, a negative turn count, another day's state, and every step of an ended attempt.
- **A fight left unfinished** waits where it stood, in a slot of the worker's own, and resumes on its turn when the attempt is opened again, as a dungeon fight does ([[R4582001]], [[R3476000]]). A finished fight's result is held beside the attempt the moment it arrives, so leaving before Continue keeps it ([[R4733000]]).
- **A finished fight is kept for replay** on the device, with the practice games and under their `PRACTICE_REPLAYS_KEPT`, its setup holding the enemy's marks and sets so it folds to its hash, and its summary its level and enemy; a fight a Glitch voided is not ([[R5125000]], [[R768]]).
- **The attempt file** (`AssaultFile`) is the setup, the day's record at its start and the action log, with the catalog version it folds under; `assault_fold` replays it and names the first refused step, as a dungeon run's fold does.

**The day's record** (`AssaultRecord`) is the highest level beaten, the checkpoints reached, the attempts started and the attempt that first repelled the Assault, if any. `assault_record` folds a running or finished attempt into it idempotently. The device keeps the records of the last `ASSAULT_HISTORY_DAYS` UTC days, and reads a malformed or differently versioned attempt or record as absent, one key at a time ([[R4581001]]).

### 16.7 The screens

All in `apps/web/src/assault/`, at `/assault`, its words through translations. Not a rule of the game (CLAUDE.md rule 7): the page sends intent and renders what the worker and the WebAssembly module answer.

- **The Play hub** shows a card, Infinite Assault, in the Solo group after the dungeon run, and the command menu an entry. Both show since the release (`ASSAULT_OPEN` is true), and every build serves `/assault`.
- **The day screen:** the date and the time to the next UTC day; the ladder of 20 levels, each with its enemy, doctrine and Commander badge, those beaten today and the checkpoints marked; the Assault's strength at the next level; the deck picker; Start, from level 1 or a checkpoint reached; the day's record and the last days' history.
- **The briefing:** the level, the enemy and its doctrine's line, a Commander's name and its two cards, its upgrades with this level's new ones marked, its deck's size and rarity mix, and Fight.
- **The fight:** a practice game on the board every game uses, its AI paced as practice's, with no Offer draw ([[R188]]) and the lethal hint ([[R2102000]]). The rail names the enemy and its level beside the live modifiers ([[R3912000]]), a Commander's name stands on its hero, and its upgrades sit in a tray beside it ([[R4582002]]). Menu asks before leaving mid-fight ([[R4582001]]), and the result offers Main menu after its own ways on ([[R4742000]]), then Watch the replay, which plays the kept fight and returns to the result ([[R5125000]]).
- **After a level** a win shows the next level's briefing; a loss shows the attempt's end, the record, and Try again from the highest checkpoint reached. Beating level 20 shows the day's victory, the attempt's turns and the record.

### 16.8 The AI

- **The enemy is the practice AI** ([[§9.9]]), playing the level's deck through its search as it plays every card, and the human's lethal hint is every AI game's ([[R2102000]]).
- **Hidden cards never change a decision** ([[R185]]). An Assault enemy's deck is Infinite Assault cards, which the mode states openly, so its fight names the set for the enemy's seat, and the AI and the lethal hint resample that seat's unseen cards from it ([[R4595000]]); a game that names no set samples from the sets that ship.

### 16.9 What is built

- **Built** (issues #4578 to #4582): the day, its deal and its escalation, the attempt, the record and the fold in `crates/run` ([[R4579000]], [[R4579001]], [[R4579002]]); the WebAssembly exports, the page, its device store and the fight in the practice worker ([[R4581000]], [[R4581001]], [[R4582000]], [[R4582001]], [[R4582002]]).
- **Live** (issue #4597, the release): the set's row is `SinglePlayer` ([[R4577000]]), an Assault enemy's seat names the set and holds its cards ([[R4597001]]), the browser deals from `assault_catalog_content` with the twelve Commanders ([[R4597000]]), and `ASSAULT_OPEN` shows the Play hub's card and the command menu's entry. The AI resamples an Assault enemy's unseen cards from the sets its seat names ([[R4595000]], [[§16.8]]).
- **Replays** (issue #5125): a finished fight is kept on the device with the practice games, listed by its level and enemy, and watched again from its result ([[R5125000]]).
- **Tuning:** every constant here is a strawman, to be tuned against `cargo jackioh assault-sim`, which plays whole days with the AI in the player's seat on the day's Assault deck and holds wide bands per pull request: level 1 clearly winnable, level 20 clearly hard, each Commander harder than the level before it. A deliberate retune moves the bands in the same patch.
