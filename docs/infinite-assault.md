# The Infinite Assault: the plan

2026-11-23 · written in a Claude session from the owner's request of the same day

> **Status: the plan of record for the issues** (tracker #4576). SPEC is still the only source of rules (CLAUDE.md). This
> file is the mode's design and the set's card list until the mode's spec section (§16) and the set's
> catalog section (§8.13) hold them; when the two disagree, the spec wins and this file is the bug. The
> part that builds a rule or a card may change a default here, and records its choice as a ruling note in
> its own issue's block (CLAUDE.md rule 3). `docs/dungeon-run.md` and `docs/stinkyscape-plus-set.md` are
> the models this plan follows.

## What the owner asked for

> A new game mode, in the practice-by-yourself area beside the AI. Daily seeded enemies and bosses from
> level 1 to 20. The bosses have entirely AI-generated decks: about 200 to 300 new AI cards, framed as
> the Infinite Assault, a set of its own. Through the 20 levels the boss gets stronger AI-generated cards,
> or artifacts and buffs: a bigger opening hand, more cards in its deck, cards that start Radiant. It is
> seeded and changes every day. The player uses a deck they own or build, or a random seeded deck. The
> night bot builds all of it, with tests. The cards should be interesting and spicy, since some may be
> used in other modes later.

## Contents

- [IA0. The mode in one page](#ia0-the-mode-in-one-page)
- [IA1. The day and its seed](#ia1-the-day-and-its-seed)
- [IA2. The levels and the escalation](#ia2-the-levels-and-the-escalation)
- [IA3. The upgrades](#ia3-the-upgrades)
- [IA4. Enemies, doctrines and Commanders](#ia4-enemies-doctrines-and-commanders)
- [IA5. The player's deck](#ia5-the-players-deck)
- [IA6. Attempts, checkpoints and the day's record](#ia6-attempts-checkpoints-and-the-days-record)
- [IA7. The screens](#ia7-the-screens)
- [IA8. The AI](#ia8-the-ai)
- [IA9. Where it lives](#ia9-where-it-lives)
- [IA10. Testing](#ia10-testing)
- [IA11. The set: Infinite Assault](#ia11-the-set-infinite-assault)
- [IA12. The card list](#ia12-the-card-list)
- [IA13. The issues](#ia13-the-issues)
- [IA14. Out of scope](#ia14-out-of-scope)
- [IA15. Open questions, with defaults](#ia15-open-questions-with-defaults)
- [IA16. What the engine lacks (CF, #4602)](#ia16-what-the-engine-lacks-cf-4602)

---

## IA0. The mode in one page

- **Every UTC day the Infinite Assault deals 20 levels**, each one game against the AI. Levels 5, 10, 15 and
  20 are **Commanders**, the bosses. Everyone who plays on the same day and the same catalog version meets
  the same enemies, decks and upgrades.
- **Every enemy deck is Infinite Assault cards**, the single-player set this plan adds (IA11): cards the
  Assault "generates" as it fights. A normal enemy plays one doctrine (an archetype of the set). A
  Commander plays its own signature cards on top of its doctrine.
- **The Assault grows with every level, one escalation at a time, chosen by the day's seed:** either
  *stronger cards* (its deck's slots step up a rarity) or *an upgrade* (a bigger opening hand, a bigger
  deck, cards that start Radiant, buffed cards, more mana, more health, starting Armor, an extra draw).
  Escalations accumulate: level 14's enemy holds everything levels 2 to 14 added. A Commander level takes
  both kinds at once and brings its Commander's cards.
- **The player brings one deck to an attempt:** a saved deck they built (the decks practice offers, [§9.9]),
  a named practice deck, or **the day's Assault deck**, a random deck dealt from the day's seed and the same
  for everyone.
- **Win a level to reach the next; lose one and the attempt ends.** Beating a Commander sets a checkpoint:
  a new attempt that day may start right after it (levels 6, 11 and 16). Attempts are unlimited.
- **The day's record** is the highest level beaten, the checkpoints reached and the attempts started.
  Beating level 20 *repels the Assault* for the day. Tomorrow the Assault is new.
- **It is a practice mode:** it plays in the browser's practice worker, needs no account, records nothing on
  the server and earns nothing. The device keeps the records of the last `ASSAULT_HISTORY_DAYS` days.
- **It sits in the Play hub's solo group** beside Practice, Dungeon run and the Tutorial, at `/assault`.
- **No card and no game rule knows about the Assault**, as no card knows about a dungeon run (§15): each
  level is an ordinary practice game whose setup the Assault writes (decks, handicaps and deck marks).

## IA1. The day and its seed

- The day is a UTC date, `YYYY-MM-DD`, which the caller passes in (the pure crates read no clock, CLAUDE.md
  rule 4). Its seed is `assault-YYYY-MM-DD` (`assault_day_seed`); any other date is refused.
- The day's deal (`AssaultDay`) is a pure function of the date and the content: its 20 levels (IA2), each
  with its enemy, deck, deck marks, handicap and escalations, and the day's Assault deck (IA5). The content
  is the catalog's (`assault_catalog_content`), so a day folds under the catalog version it was dealt on,
  as a dungeon run does (R3496000): the device keeps the version with the day's attempts.
- A level's game seed is `{day seed}-L{level}-A{attempt}`: the same enemy, deck and upgrades on every
  attempt that day, a fresh shuffle on each. Everyone's nth attempt at a level meets the same shuffle.
- An attempt started before midnight UTC plays on its own day's deal after midnight, and its record is
  that day's. The screen offers the new day only once no attempt of an earlier day is open.

## IA2. The levels and the escalation

Every number here is a strawman to tune against `assault-sim` (IA10), and a named constant in
`crates/run/src/config.rs` (CLAUDE.md rule 9).

**The base row.** Every level starts from Easy's resources (§9.9: `mana_bonus` 0, `mana_cap` `MAX_MANA`,
the human's opening draw, one draw a turn), with:

| Constant | Strawman |
| --- | --- |
| `ASSAULT_LEVELS` | 20 |
| `ASSAULT_COMMANDER_LEVELS` | 5, 10, 15, 20, the ranks I to IV |
| `ASSAULT_HERO_HEALTH` | 20 + the level at a normal level (21 to 39); 30 + twice the level at a Commander (40, 50, 60, 70) |
| `ASSAULT_FEWER_OPENING` | 1 card fewer in the opening draw on levels 1 to 3 (`fewer_opening_cards`, R3582000) |
| `ASSAULT_DECK_SIZE` | 25 (`DECK_SIZE`), before Bigger Dataset |
| `ASSAULT_START_SLOTS` | level 1's deck: 19 Common and 6 Rare slots |
| `ASSAULT_CARD_STEP` | 4: the slots a card step promotes |
| `ASSAULT_RARITY_CEILING` | the rarest slot a level may hold: Rare on 1–3, Epic on 4–7, Legendary on 8–13, Mythic on 14–20 |
| `ASSAULT_CARD_STEP_PERCENT` | 50: the chance a normal level's escalation is a card step rather than an upgrade |
| `ASSAULT_DOCTRINE_SHARE` | 60: the percent of a deck dealt from its doctrine; the rest from the neutral cards, then any doctrine |

**One escalation a level.** Level 1 is the base row alone. Each later level adds one escalation to
everything the levels before it added, the seed choosing which:

- **A card step** promotes `ASSAULT_CARD_STEP` of the deck's slots one rarity, lowest rarity first, never
  past the level's ceiling. A step with nothing left to promote under the ceiling is an upgrade instead.
- **An upgrade** is one stack of one upgrade (IA3), drawn by the upgrades' weights among those not at
  their most stacks and allowed at this level. A level with no upgrade left takes a card step.

A Commander level takes a card step *and* an upgrade, and its Commander's cards join the deck (IA4).

**The deck.** A level's deck is dealt from its slots: each slot a distinct Infinite Assault card of its
rarity, `ASSAULT_DOCTRINE_SHARE` percent from the level's doctrine and the rest from the neutral cards and
then any doctrine, never a Commander card but its own Commander's. A slot whose rarity is exhausted takes
the next rarity down. The deck holds no token and no card twice (R184), and its size is its handicap's
`deck_size`, so `createGame` takes it as it takes every practice deck.

**It never gets weaker.** No handicap field falls from one level to the next, no slot's rarity falls, no
upgrade's stacks fall, and each level's deck is dealt afresh from its own slots (a level is not the
previous level's deck plus a card). A test holds this over a year of days (IA10).

## IA3. The upgrades

Each is an `AssaultUpgradeKind`, with its amount per stack, its most stacks, the first level it may come
on and its weight (all strawmen). Every one maps onto a field the engine already has: a handicap field
(R180 to R183, R290, R2922000, R3582000) or a deck mark (`DeckMark`, which `CreateGameOptions` already
takes per seat; the dungeon run marks the human's, this marks the enemy's).

| Kind | Name players see | Effect per stack | Most | From level | Weight |
| --- | --- | --- | --- | --- | --- |
| `openingCards` | Expanded Context | +1 card in the opening draw (`extra_opening_cards`) | 3 | 4 | 3 |
| `deckSize` | Bigger Dataset | +5 cards in the deck (`deck_size`); the new slots take the rarity most of its slots hold, the lower on a tie | 3 | 2 | 2 |
| `radiant` | Fine-Tuned | 3 more of its deck's cards start Radiant (a deck mark), drawn by the seed among those not Radiant | 5 | 2 | 4 |
| `buffed` | Reinforced Weights | 3 more of its deck's cards start with one Buff each (a deck mark's `upgrades`) | 5 | 2 | 3 |
| `manaBonus` | Overclocked | +1 crystal ahead of its turn count (`mana_bonus`, R181) | 2 | 8 | 2 |
| `manaCap` | Raised Ceiling | +1 to its max mana's cap (`mana_cap`) | 3 | 6 | 2 |
| `heroHealth` | Redundancy | +10 hero health (`hero_health`) | 3 | 2 | 3 |
| `heroArmor` | Hardened Shell | +5 starting Armor (`hero_armor`, R2922000) | 3 | 2 | 3 |
| `extraDraw` | Parallel Inference | +1 draw each turn (`extra_draws_per_turn`, R183) | 1 | 12 | 1 |

The fight's rail lists every live modifier, under You and the enemy's name, as every table's does
(R3912000), so a player always sees what the Assault holds. The briefing (IA7) marks what this level added.

## IA4. Enemies, doctrines and Commanders

**Doctrines.** The set's eight archetypes, each a tag on its cards (IA11). A normal level draws its
doctrine by the seed, never the same doctrine on two levels in a row. Each has a name and a line players
read, and four enemy names; a normal level's enemy is one of its doctrine's names, `{name}` with the level
shown beside it.

| Doctrine (tag) | Line | Enemy names |
| --- | --- | --- |
| Hallucination | Sets Traps that may be nothing, and plays copies that fade. | Mirage Weaver, Phantom Choir, Static Ghost, The Unreliable Narrator |
| Overclock | Burns its own health for fast, huge turns. | Redline Racer, Burst Clock, Fan Failure, Thermal Throttler |
| Batch | Floods the board with copies and buffs them all. | Hive Process, Parallel Legion, Worker Pool, Mass Producer |
| Recursion | What dies comes back, stronger. | Loop Unroller, Stack Overflow, The Rerun, Rollback Daemon |
| Jailbreak | Taxes, freezes and steals your cards. | Prompt Injector, Rootkit Runner, Exploit Broker, The Jailbreaker |
| Firewall | Walls, shields and Traps that punish attacks. | Packet Filter, Quarantine Wall, Bastion Node, The Gatekeeper |
| Scraper | Copies your deck and plays your cards against you. | Data Harvester, Web Crawler, Dataset Thief, The Mimic |
| Singularity | Starts slow and grows without limit: answer it early. | Power Law, Compute Cluster, The Long Context, Emergent Mind |

**Commanders.** Twelve, three of each rank (IA12.10). Level 5 meets a rank I Commander, 10 rank II, 15
rank III and 20 rank IV, each drawn by the seed. A Commander's deck is its two cards (the Commander, a
Unit, and its signature card), always in it, plus its doctrine's slots as a normal level's (the Prime
Generator, who has none, deals from every doctrine). The Commander's two cards take two of the deck's
lowest slots, so its size is the level's. Commander cards carry the `Commander` tag, and no other deck
deals them.

The content (`AssaultContent`) is the cards, read from the catalog by set and tag, and
`crates/run/src/assault.json`: each doctrine's tag, name, line and enemy names, and each Commander's card
id, rank, doctrine and signature card (compile time, `include_str!`, CLAUDE.md rule 4).

## IA5. The player's deck

- **A saved deck**, as practice offers them (§9.9): an active account's complete decks that the validator
  passes under L2, L3 and L6 against the catalog, with nothing banned; any other listed disabled with its
  reason.
- **A named practice deck:** Human Vanguard, Blitz or Fortress (§9.9).
- **The day's Assault deck:** dealt as practice's fresh random deck for a human is (R258's draw, `banned:
  []`, `build_ai_deck`), seeded `{day seed}-deck`, with no "More cards from" lean, so everyone's is the
  same that day. It needs no account.
- The human plays `HUMAN_HANDICAP` at every level (no carried health, no artifacts), in seat P1, as a
  dungeon fight's human does. A deck is chosen per attempt and kept for the whole attempt. Its cards are
  shipped cards: no deck ever holds an Infinite Assault card (R1846001).

## IA6. Attempts, checkpoints and the day's record

- **An attempt** starts at level 1, or at a checkpoint the day's record holds: 6, 11 or 16, the levels
  after a Commander beaten that day on any attempt. Its state is the setup (the day, the deck and its
  source, the start level, the attempt's number), the level it stands on, its phase (`briefing`, `fight`,
  `lost`, `repelled`, `abandoned`), the levels it beat and its turns.
- **A level:** the briefing shows the enemy (IA7); the player fights; a win moves to the next level's
  briefing, a loss or a draw at the turn cap ends the attempt `lost`. Beating level 20 ends it `repelled`.
  Abandoning ends it `abandoned`, asking first. Leaving mid-fight keeps the fight, which resumes on its
  turn when the attempt is opened again (as a dungeon fight does, R3476000).
- **The attempt file** is the setup, the day's record at its start and the action log; `assault_fold`
  replays it and names the first refused step, as `run_fold` does. A refused action changes nothing.
- **The day's record** (`AssaultRecord`): the highest level beaten, the checkpoints reached, the attempts
  started and the attempt that first repelled the Assault, if any. `assault_record` folds a finished or
  running attempt into it. The device keeps the last `ASSAULT_HISTORY_DAYS` (30) days.

## IA7. The screens

All in `apps/web/src/assault/`, at `/assault`, words through translations (`apps/web/WRITING.md`), with
its Chinese.

- **The Play hub:** a card `assault` in the `solo` group, after Dungeon run: "Infinite Assault", its line
  "20 levels against AI-made bosses. New every day.", the minutes of one level, and the command menu's
  entry. It needs no account. Until the release (IA13, R) the card and the command entry are off behind
  `ASSAULT_OPEN` (false), and `/assault` is reachable only in a dev build and the e2e build.
- **The day screen:** the day's date and the time to the next day; the ladder of 20 levels, each with its
  number, enemy name, doctrine mark and Commander badge, those beaten today ticked, the checkpoints marked;
  the Assault's strength at the next level to play (its upgrades and its deck's rarity mix); the deck
  picker (saved, preset, the day's Assault deck, with the chosen deck's cards and curve as practice's
  setup shows them); Start, from level 1 or the highest checkpoint reached; the day's record; and the
  history of the last days.
- **The briefing:** the level, the enemy and its doctrine's line, a Commander's name, art and card faces
  (its two cards are shown: a boss is fair when its threat is known), its upgrades with this level's new
  ones marked, its deck's size and rarity mix, and Fight.
- **The fight:** a practice game on the board every game uses, its AI paced as practice's (§9.9), Menu
  leaving to the day screen after asking. The rail lists every modifier (R3912000). The lethal hint works
  as in every game against the AI (R2102000, IA8).
- **After a level:** a win shows the next level's briefing with what it adds; a loss shows the attempt's
  end, the record and Try again (from the highest checkpoint) or Back.
- **Repelled:** beating level 20 shows the day's victory, the attempt's turns and the record.

## IA8. The AI

- **It plays every Infinite Assault card** through its search, as it plays every card. Each card's part
  proves the AI, the random policy and the timeout answer every prompt the card opens.
- **Hidden cards never change a decision** (R185). One gap: the lethal hint and the AI resample a seat's
  unseen cards from the cards of every set (R185), but an Assault enemy's deck is Infinite Assault cards,
  which the mode states openly. So the human's lethal hint, sampling the enemy's hand and face-down cards
  from shipped cards, can promise a lethal that a face-down Assault Trap would stop. The AI part (IA13)
  rules on it; the default is that a game's setup may name, for a seat, the sets its deck is dealt from
  (public, shown in the briefing), and determinization resamples that seat's unseen cards from those sets
  (a game that names none samples as today, so every other game hashes as before).
- **It must play the set well enough to be a fair boss.** The AI part measures the AI with each Infinite
  Assault card (`cargo jackioh sweep`, R186, over the set) and fixes what it misplays; a card it cannot
  pilot is fixed in its evaluation or redesigned, never dropped silently.

## IA9. Where it lives

- **The rules:** pure Rust in `crates/run`, a module `assault` beside the dungeon run, sharing its seeds,
  its `Rng` use and its fight report (CLAUDE.md rule 4). Its numbers are in `crates/run/src/config.rs`
  (rule 9). Its content file is `crates/run/src/assault.json`. Its types export to
  `apps/web/src/wire/generated/` behind the `ts` feature, re-exported by `apps/web/src/wire/run.ts`.
- **The day's Assault deck:** `jackioh-shared` (which takes the AI's deck draw), or wherever part A and
  part B find the draw already reachable; the surface below names its export.
- **The browser:** the WebAssembly module exports each entry point (JSON in, JSON out, as the `run_*`
  exports do), the practice worker plays the fights, and the device keeps the records.
- **The cards:** `crates/cards/src/scripts/infinite_assault/`, one file per card (CLAUDE.md rule 6).
- **The spec:** §16 `spec/16-infinite-assault.md` (the mode), §8.13 (the set's cards), a line in §1.1 and
  in `spec/README.md`'s map.

## IA10. Testing

Every part's "Done when" names its tests; together they hold the mode to this:

- **Determinism:** the same date and content deal the same day, byte for byte; two dates deal different
  days; every level's setup folds back to its hash; an attempt file folds back to its state.
- **Shape:** 20 levels; Commanders on 5, 10, 15 and 20, each of its rank, each deck holding its two cards;
  no doctrine on two levels in a row; every deck distinct, token-free, all Infinite Assault cards, of its
  handicap's size, within its level's ceiling; `createGame` accepts every level's setup on the real
  catalog, for 365 consecutive days.
- **Escalation:** one escalation a normal level, two at a Commander; nothing ever falls (IA2), over 365 days.
- **Attempts:** a start at a checkpoint the record lacks is refused; a loss ends the attempt; a win moves on;
  level 20 repels; an ended attempt refuses every action; a refusal leaves the state unchanged.
- **Difficulty:** `cargo jackioh assault-sim --days N` plays whole days with the AI in the player's seat at
  R747000's skill on the day's Assault deck, and prints each level's win rate, turns and the Commanders'
  rates. A per-pull-request test holds wide bands at the tiny budget, as run-sim's does (§15.12): level 1
  clearly winnable, level 20 clearly hard, each Commander harder than the level before it. A deliberate
  retune moves the bands in the same patch.
- **Cards:** each card's tests, base and Radiant (CLAUDE.md rule 6); `cargo jackioh fuzz --seeds 200
  --cards "Infinite Assault"`; the fused-pair and carried sweeps as every card meets them.
- **The browser:** the web's unit tests for the store, the screens and the hub card; an e2e spec that
  opens `/assault`, takes the day's Assault deck, starts level 1, wins it through the e2e build's hooks,
  reaches level 2's briefing, and finds the record kept after a reload.

## IA11. The set: Infinite Assault

**The fiction.** Somewhere past the edge of the map, a generator was asked for one more card and never
stopped. The Infinite Assault is what it writes: an army made of cards, trained on every game ever lost,
written mid-fight. Its flavor reads like machine logs, patch notes and model-card disclaimers ("Trained on
4 billion lost games. Yours is next."), in JackiOh's deadpan meme voice.

**Ids.** `SetName::InfiniteAssault`, written `"Infinite Assault"`, segment `infiniteassault`, abbreviation
`IA`, scripts in `crates/cards/src/scripts/infinite_assault/cNNN_slug.rs`, ids `infiniteassault-NNN`, a
token a card defines `infiniteassault-NNN-k`. Its row goes at the end of `set_table!`
(`crates/engine/src/wire/catalog_types.rs`, R3114000) as a single-player set (R1846001), with
`shadow_ban_share` `None`. Its catalog section is §8.13.

**The hold.** R1846001: a single-player set's cards join every `everySet` pool from the patch that adds
them, so "a single-player set built card by card before its mode ships needs a hold of its own, which the
issue that builds it adds". Part C1 added it (R4577000): a new `SetStage`, `HeldSinglePlayer`, single-player
and held, which no pool reaches (`everySet` included) but one that names the set, and which the release (R)
turns to `SinglePlayer`. While held:
no fragment, no golden trace moves, the census's shipped totals don't move, and tests open it with the
testkit's `preview_sets`, as a held set's do (`docs/ADDING_CARDS.md`, A card of a set that has not
shipped).

**Tags.** Every card of the set is an AI generated card: it carries the `AI` tag the T-AI cards carry
(R1421), so a card that counts AI generated cards counts it (T-AI-2 Scaling Law) and a pool of them reaches
it once it may reach the set. On top of `AI`, eight doctrine tags (Hallucination, Overclock, Batch,
Recursion, Jailbreak, Firewall, Scraper, Singularity) and `Commander`, declared by C1 with whatever the
server needs to admit a new tag. A token keeps `Token` and takes `AI` and its card's doctrine.
The names are distinct from every catalog name (the T-AI tokens' Hallucination, Scaling Law and
Autocomplete among them).

**The numbers.** 240 numbered cards and 15 tokens on sub-numbers:

| Group | Numbers | Common | Rare | Epic | Legendary | Mythic |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Assault Core (neutral) | #1–#24 | 10 | 8 | 4 | 2 | 0 |
| Each doctrine (×8) | 24 each, #25–#216 | 9 | 7 | 5 | 2 | 1 |
| Commanders | #217–#240 | 0 | 0 | 6 | 12 | 6 |
| **Total** | **240** | **82** | **64** | **50** | **30** | **14** |

**The design rules every card part keeps.**

1. **Real cards.** Each card is legal in an ordinary game and plays the same in any mode; none reads the
   mode or its level. Commons to Legendaries of the doctrines and the Core sit on JackiOh's curve (costs
   (0) to (4); about 4 stats at (1), 8 at (2), 12 at (3), 16 at (4) for a Unit with little text, less with
   more), so they can be reused. Commander cards are boss cards, above the curve on purpose.
2. **Spicy, never unfair.** Swingy and rule-bending is the point, but every threat has an answer the player
   can see coming: a countdown is shown, a wall can be killed, a bluff can be called. Nothing makes the
   player lose with no counterplay, and nothing locks them out of playing for a turn they can't foresee
   (Kill Switch, #140, fires on a third card, which the player may choose not to play).
3. **Expressible.** The text uses verbs, keywords and filters the engine has (§6); where a card needs a new
   one, its part adds it by `docs/ADDING_CARDS.md` §6, with its test. CF (#4602) lists the verbs the set
   still lacks in IA16. Numbers are `params` (`{damage}`), never literals (CLAUDE.md rule 9).
4. **Radiant is about twice the base face** (R275): doubled stats and a better line, never stats alone for
   a Unit with text. CF (#4602) wrote every `docs/radiant-audit.md` row; each part corrects its own.
5. **"Infinite Assault card"** in a card's text is a pool that names the set, which reaches the set while
   it is held (R1846001: a pool that names its set keeps to it). "Created" is the engine's Created mark.
6. **The AI must pilot it.** A card whose prompt the AI can't answer well is fixed or redesigned in its
   part, not shipped hoping.
7. **The part may tune.** A part may change a number, a word or a Radiant line where the engine or balance
   asks, keeping the card's role and its spice, and says so in its pull request; a bigger change goes in a
   ruling note.

## IA12. The card list

Each line: number, name, cost, type, rarity, stats, tags, base text, then the Radiant face. Every card
carries `AI`; a doctrine's tag is on all its cards; Commander cards also carry `Commander`. Tokens are on sub-numbers and are drawn
Radiant as any token is.

### IA12.1 Assault Core, #1–#24

The Assault's war machine: Drones, Created cards and the factory that writes them. No doctrine tag; every
deck may deal them.

C1 (#4577) built them, writing some lines in the catalog's words (rule 7): #5's and #21's prices as "(1) Cost"
and "(2) Cost or less", #12's "whose Cost is (1) higher", #13's Radiant face as "Discover an Infinite Assault card
twice", and #18's cost cut labelled "Aura:". §8.13 holds the text.

- **#1 Seed Drone** — (1) Unit, Common, 2/2. Cry: Add a random Common Infinite Assault card to your hand. *Radiant:* 4/4. Cry: Add a random Rare Infinite Assault card to your hand.
- **#2 Scrap Prototype** — (0) Unit, Common, 1/2. Death: Shuffle a copy of this with +1/+1 into your deck. *Radiant:* 2/4. Death: Shuffle two copies of this with +2/+2 into your deck.
- **#3 Assault Trooper** — (1) Unit, Common, 3/2. Rush. *Radiant:* 6/4. Rush, First Strike.
- **#4 Wave Spawner** — (2) Unit, Common, 3/4. End of turn: If you played a Created card this turn, summon a Drone. *Radiant:* 6/8. End of turn: If you played a Created card this turn, summon two Drones.
  - **#4.1 Drone** — token, (1) Unit, 1/1.
- **#5 Generated Filler** — (1) Spell, Common. Add two random (1)-Cost Infinite Assault Units to your hand. They're Temporary. *Radiant:* … They cost (0) and are Temporary.
- **#6 Hotfix** — (1) Spell, Common. Give a friendly Unit +2/+2. If it's Created, draw a card. *Radiant:* +4/+4 and Divine Shield; if it's Created, draw a card.
- **#7 Bulk Order** — (2) Spell, Common. Summon three Drones. *Radiant:* Summon three Radiant Drones with Rush.
- **#8 Firing Solution** — (1) Spell, Common. Deal 3 damage to a Unit. If it dies, add a random Infinite Assault Spell to your hand. *Radiant:* Deal 6 damage …
- **#9 Heavy Lifter Mk.II** — (3) Unit, Common, 5/7. Taunt. Cry: Give your other Created Units +1/+1. *Radiant:* 10/14. … +2/+2.
- **#10 Assault Carrier** — (4) Unit, Common, 7/8. Death: Summon two Drone Escorts. *Radiant:* 14/16. Death: Summon two Radiant Drone Escorts.
  - **#10.1 Drone Escort** — token, (2) Unit, 3/3.
- **#11 Quickgen Turret** — (2) Unit, Rare, 2/5. Ranged. Start of turn: Deal 1 damage to a random enemy for each Created card in your hand. *Radiant:* 4/10. … 2 damage …
- **#12 Recompile** — (1) Spell, Rare. Transform a friendly Unit into a random Infinite Assault Unit that costs (1) more. *Radiant:* … (2) more.
- **#13 Training Run** — (2) Spell, Rare. Discover an Infinite Assault card. It costs (2) less. *Radiant:* Discover two.
- **#14 Overflow Error** — (1) Trap, Rare. Reveals when your opponent plays their third card in a turn: Deal 3 damage to their hero. They discard a random card. *Radiant:* 6 damage; two cards.
- **#15 Drone Foundry** — (2) Field Spell, Rare. Start of turn: Summon a Drone. Then, if you have 4 or more Drones, give your Drones +1/+1. *Radiant:* Summon two Drones …
- **#16 Benchmark Run** — (1) Spell, Rare. Set a friendly Unit's Attack to the highest Attack and its Health to the highest Health among enemy Units. *Radiant:* … and give it Rush.
- **#17 Cluster Drone** — (2) Unit, Rare, 3/2. Death: Deal 2 damage to every enemy Unit. *Radiant:* 6/4. … 4 damage …
- **#18 Prompt Engineer** — (2) Unit, Rare, 2/4. Your Created cards cost (1) less, but not less than (1). *Radiant:* 4/8. Your Created cards cost (1) less, but not less than (1), and your Created Units have +1/+1.
- **#19 Infinite Scroll** — (3) Spell, Epic. Draw three cards. Shuffle a copy of this into your deck. *Radiant:* Draw four cards. Shuffle a copy of this into your deck; it costs (1) less.
- **#20 Synthetic Data** — (2) Spell, Epic. Fill your hand with random Infinite Assault cards. They're Temporary. *Radiant:* … They're Temporary and cost (1) less.
- **#21 Endless Production Line** — (4) Field Spell, Epic. End of turn: Summon a random Infinite Assault Unit that costs (2) or less. *Radiant:* … summon two.
- **#22 Assault Mainframe** — (4) Unit, Epic, 6/10. Taunt. Aura: Your Created Units have +2/+2. *Radiant:* 12/20. … +4/+4.
- **#23 The Generator** — (3) Unit, Legendary, 4/6. Start of turn: Add a random Infinite Assault card to your hand. It costs (0). *Radiant:* 8/12. … two random Infinite Assault cards …
- **#24 Mass Deployment** — (4) Spell, Legendary. Summon a random Infinite Assault Unit of each Cost from (1) to (3). They have Rush. *Radiant:* … They're Radiant and have Rush.

### IA12.2 Hallucination, #25–#48

Bluff and backrow: Figments (Traps that do nothing), Cloaked Units, copies that fade. The player
learns to read the backrow; the Assault punishes reading it wrong.

- **#25 Mirage Weaver** — (1) Unit, Common, 1/3. Cry: Set a Figment. *Radiant:* 2/6. Cry: Set two Figments.
  - **#25.1 Figment** — token, Trap. Set at a random cost from (1) to (4). Reveals when an enemy Unit attacks: Nothing happens.
- **#26 Phantom Lancer** — (2) Unit, Common, 4/3. Cloaked. When this is revealed, deal 3 damage to an enemy Unit. *Radiant:* 8/6. … 6 damage …
- **#27 Decoy Projector** — (2) Unit, Common, 0/7. Taunt. Cry: Set a Figment. *Radiant:* 0/14. Taunt, Divine Shield. Cry: Set a Figment.
- **#28 Doppelganger Protocol** — (1) Spell, Common. Summon a copy of a random enemy Unit. It has Brittle 1. *Radiant:* … Brittle 3.
- **#29 False Positive** — (1) Trap, Common. Reveals when an enemy Unit attacks: Cancel the attack. Freeze the attacker. *Radiant:* … Then set a Figment.
- **#30 Smoke and Mirrors** — (2) Spell, Common. Set two Figments. Draw a card. *Radiant:* Set three Figments. Draw two cards.
- **#31 Unreliable Narrator** — (2) Unit, Common, 3/4. Cry: Look at the top three cards of your opponent's deck. Put one of them on the bottom. *Radiant:* 6/8. … Put two of them on the bottom.
- **#32 Glitch Specter** — (1) Unit, Common, 3/1. Evasive while you have a face-down card. *Radiant:* 6/2. Lifesteal. Evasive while you have a face-down card.
- **#33 Uncanny Valley** — (3) Unit, Common, 5/6. Cry: Enemy Units have −2 Attack until your next turn. *Radiant:* 10/12. … −4 Attack …
- **#34 Mass Hallucination** — (2) Spell, Rare. Transform every enemy Unit that costs (1) or less into a Husk. *Radiant:* … (2) or less …
  - **#34.1 Husk** — token, (1) Unit, 1/1.
- **#35 Fata Morgana** — (2) Unit, Rare, 3/5. Whenever a Figment of yours reveals, summon a Mirage. *Radiant:* 6/10. … summon a Radiant Mirage.
  - **#35.1 Mirage** — token, (1) Unit, 2/2. Rush.
- **#36 Deepfake** — (2) Spell, Rare. Choose an enemy Unit. Summon a copy of it for yourself with Brittle 2. *Radiant:* … a copy of it for yourself.
- **#37 Bluff Called** — (2) Trap, Rare. Reveals when your opponent plays a Spell: Counter it. *Radiant:* Counter it, then set a Figment.
- **#38 Imaginary Friend** — (1) Unit, Rare, 2/2. Divine Shield. Death: Set a Figment. *Radiant:* 4/4. … Death: Set two Figments.
- **#39 Shell Game** — (1) Spell, Rare. Set a Figment. Then draw a card for each face-down card you have, up to three. *Radiant:* … up to five.
- **#40 Hall of Mirrors** — (3) Field Spell, Rare. Whenever you set a card face-down, summon a Reflection. *Radiant:* … summon a Radiant Reflection.
  - **#40.1 Reflection** — token, (1) Unit, 1/3. Taunt.
- **#41 Phantom Army** — (3) Spell, Epic. Fill your empty unit zones with Phantoms. *Radiant:* … with Radiant Phantoms.
  - **#41.1 Phantom** — token, (1) Unit, 3/1. Rush, Brittle 1.
- **#42 Paranoia Engine** — (2) Unit, Epic, 2/6. Your opponent's cards can't destroy or reveal your face-down cards. End of turn: Set a Figment. *Radiant:* 4/12. … End of turn: Set two Figments.
- **#43 Cognitive Dissonance** — (2) Trap, Epic. Reveals when your opponent plays a Unit: Transform it into a random Unit that costs (1) less. *Radiant:* … (2) less.
- **#44 The Lie Detector** — (3) Unit, Epic, 4/8. Cry: Destroy every enemy face-down card. Set a Figment for each. *Radiant:* 8/16.
- **#45 Dream Within a Dream** — (4) Spell, Epic. Summon a copy of every enemy Unit for yourself. They have Brittle 1. *Radiant:* … Brittle 2.
- **#46 Mirage Sovereign** — (3) Unit, Legendary, 6/6. Cloaked. Whenever a Figment of yours reveals, summon a copy of the attacking Unit for yourself. *Radiant:* 12/12. … then destroy the attacking Unit.
- **#47 Unreal Estate** — (2) Field Trap, Legendary. Reveals whenever an enemy Unit attacks: 50% chance to cancel the attack. *Radiant:* Lucky 1. …
- **#48 Ego Death** — (3) Spell, Mythic. Swap hands with your opponent. The cards you get cost (1) less. *Radiant:* … (2) less.

### IA12.3 Overclock, #49–#72

Aggression on borrowed time: mana now, health later. Its cards hurt their own hero, die young and hit
hard; the player wins by surviving the spike.

- **#49 Spark Plug** — (0) Unit, Common, 2/1. Rush. Death: Gain 1 mana this turn. *Radiant:* 4/2. … Gain 2 mana this turn.
- **#50 Burst Clock** — (1) Spell, Common. Gain 2 mana this turn. Lose 1 mana next turn. *Radiant:* Gain 3 mana this turn. Lose 1 mana next turn.
- **#51 Redline Racer** — (2) Unit, Common, 5/2. Charge. End of turn: Deal 1 damage to your hero. *Radiant:* 10/4.
- **#52 Thermal Paste** — (1) Spell, Common. Give a friendly Unit +3 Attack and Rush. It has Brittle 1. *Radiant:* +6 Attack …
- **#53 Overvolt Grunt** — (1) Unit, Common, 4/3. Cry: Deal 2 damage to your hero. *Radiant:* 8/6.
- **#54 Fan Failure** — (2) Unit, Common, 6/4. Brittle 2. *Radiant:* 12/8. Brittle 2, Rush.
- **#55 Heat Sink** — (2) Unit, Common, 2/6. Taunt. Whenever your hero takes damage on your turn, this gains +1 Attack. *Radiant:* 4/12. … +2 Attack.
- **#56 Afterburner** — (2) Spell, Common. Deal 4 damage to the enemy hero. Deal 2 damage to your hero. *Radiant:* Deal 8 damage to the enemy hero. Deal 2 damage to your hero.
- **#57 Speed Demon** — (3) Unit, Common, 6/5. Charge. *Radiant:* 12/10. Charge, Windfury.
- **#58 Overclocked Core** — (2) Unit, Rare, 1/5. Aura: Your Spells cost (1) less. End of turn: Deal 1 damage to your hero. *Radiant:* 2/10. Aura: Your Spells cost (1) less and your Units have +1 Attack. …
- **#59 Voltage Spike** — (2) Spell, Rare. Deal 3 damage to an enemy. If your hero has 15 or less Health, deal 6 instead. *Radiant:* 6, or 12.
- **#60 Pit Crew** — (1) Unit, Rare, 2/3. Cry: Your next Unit this turn costs (2) less. *Radiant:* 4/6. … (3) less.
- **#61 Meltdown Charge** — (3) Spell, Rare. Deal 2 damage to every Unit and both heroes. Repeat once for each mana you have left. *Radiant:* … to every enemy …
- **#62 Burnout** — (1) Trap, Rare. Reveals when your opponent ends their turn with unspent mana: Deal twice that much damage to their hero. *Radiant:* four times …
- **#63 Nitrous Injector** — (2) Unit, Rare, 3/3. Cry: Give your other Units +2 Attack and Rush this turn. *Radiant:* 6/6. … +4 Attack …
- **#64 Crash Test Dummy** — (1) Unit, Rare, 1/6. Taunt. Death: Deal 5 damage to the enemy hero. *Radiant:* 2/12. … 10 damage …
- **#65 Overdrive** — (3) Spell, Epic. Gain 4 mana this turn. At the end of your turn, deal 2 damage to your hero for each mana you didn't spend. *Radiant:* Gain 6 mana …
- **#66 Afterimage Sprinter** — (3) Unit, Epic, 8/3. Charge, Windfury, Brittle 1. *Radiant:* 16/6.
- **#67 Burning Ambition** — (2) Field Spell, Epic. Start of turn: Deal 2 damage to your hero and gain 1 mana this turn. *Radiant:* … gain 2 mana this turn.
- **#68 Short Circuit** — (2) Trap, Epic. Reveals when an enemy Unit attacks: Destroy it. Deal damage equal to its Attack to your hero. *Radiant:* … half its Attack, rounded down …
- **#69 Rev Limiter** — (4) Unit, Epic, 7/7. Rush. After this attacks, it gains +3 Attack and deals 2 damage to your hero. *Radiant:* 14/14. … +6 Attack …
- **#70 Redline Overlord** — (3) Unit, Legendary, 6/6. Aura: While your hero has 15 or less Health, your Units have Charge and +2 Attack. *Radiant:* 12/12. … +4 Attack.
- **#71 Thermal Throttler** — (2) Unit, Legendary, 3/6. Start of turn: Gain 1 mana this turn for each 10 Health your hero is missing. *Radiant:* 6/12. … for each 5 Health …
- **#72 Overclock to Infinity** — (4) Spell, Mythic. Your cards cost (0) this turn. At the end of your turn, deal 10 damage to your hero. *Radiant:* … deal 5 damage to your hero.

### IA12.4 Batch, #73–#96

Go wide, then wider: copies of itself, Instances, Workers and auras that pay for every body. The player
wins with sweepers and by killing the buffer first.

- **#73 Worker Thread** — (1) Unit, Common, 1/1. Cry: Summon a copy of this. *Radiant:* 2/2. Cry: Summon two copies of this.
- **#74 Parallel Instance** — (2) Unit, Common, 2/2. Cry: Summon an Instance. *Radiant:* 4/4. Cry: Summon a Radiant Instance.
  - **#74.1 Instance** — token, (1) Unit, 2/2.
- **#75 Load Balancer** — (2) Unit, Common, 2/4. Aura: Your other Units have +1 Health. *Radiant:* 4/8. … +2 Health.
- **#76 Mob Mentality** — (1) Spell, Common. Give your Units +1/+1. *Radiant:* +2/+2.
- **#77 Swarm Node** — (1) Unit, Common, 2/3. Death: Give a random friendly Unit +2/+2. *Radiant:* 4/6. … +4/+4.
- **#78 Batch Job** — (3) Spell, Common. Summon four Workers. *Radiant:* Summon four Radiant Workers.
  - **#78.1 Worker** — token, (1) Unit, 1/2.
- **#79 Thread Pool** — (3) Unit, Common, 4/4. Cry: Summon a Drone (#4.1) for each other Unit you control. *Radiant:* 8/8. … a Radiant Drone …
- **#80 Hive Ganger** — (2) Unit, Common, 3/3. Has +1 Attack for each other friendly Unit. *Radiant:* 6/6. … +2 Attack …
- **#81 Overprovision** — (2) Spell, Common. Summon two Instances. If you control 4 or more Units, draw a card. *Radiant:* … two Radiant Instances … draw two cards.
- **#82 Fork Bomb** — (2) Unit, Rare, 2/2. Death: Summon two Forks. *Radiant:* 4/4. Death: Summon two Radiant Forks. They have Rush.
  - **#82.1 Fork** — token, (1) Unit, 1/1.
- **#83 Swarm Logic** — (2) Spell, Rare. Your Units have +2 Attack and Rush this turn. *Radiant:* … +4 Attack …
- **#84 Distributed Mind** — (3) Unit, Rare, 3/5. Once per turn, when another friendly Unit dies, draw a card. *Radiant:* 6/10. Twice per turn …
- **#85 Merge Conflict** — (2) Spell, Rare. Fuse two friendly Units. *Radiant:* … The result has Rush and Divine Shield.
- **#86 Replica Hatchery** — (2) Field Spell, Rare. End of turn: Summon a copy of your cheapest Unit. It has Brittle 1. *Radiant:* … a copy of your cheapest Unit.
- **#87 Swarm Shield** — (1) Trap, Rare. Reveals when an enemy Unit attacks: Summon three Workers with Taunt. *Radiant:* … three Radiant Workers with Taunt.
- **#88 Scale Out** — (2) Spell, Rare. Summon an Instance for each enemy Unit. *Radiant:* … a Radiant Instance …
- **#89 Queen of the Cluster** — (4) Unit, Epic, 6/8. Aura: Your other Units have +2/+2. End of turn: Summon a Drone. *Radiant:* 12/16. … +4/+4 … a Radiant Drone.
- **#90 Mass Production** — (4) Spell, Epic. Summon a copy of each friendly Unit. *Radiant:* … They have Rush.
- **#91 Flood the Zone** — (3) Spell, Epic. Fill your empty unit zones with Drones. They have Rush. *Radiant:* … Radiant Drones …
- **#92 Hive Overseer** — (3) Unit, Epic, 2/8. Taunt. Whenever a friendly Unit dies, summon a Drone. *Radiant:* 4/16. … a Radiant Drone.
- **#93 Gestalt** — (4) Unit, Epic, 2/2. Cry: Fuse every other friendly Unit into this. *Radiant:* 4/4. … Then it gains Rush and Divine Shield.
- **#94 Batch Monarch** — (3) Unit, Legendary, 4/6. Start of turn: Summon a copy of your highest-Attack Unit with Brittle 1. *Radiant:* 8/12. … a copy of your highest-Attack Unit.
- **#95 The Botnet** — (4) Field Spell, Legendary. Your Units have +1/+1 for each other friendly Unit. *Radiant:* … and Rush.
- **#96 Exponential Growth** — (2) Spell, Mythic. Summon a copy of each friendly Unit. Then double the Attack and Health of your Units. *Radiant:* … The copies have Rush. …

### IA12.5 Recursion, #97–#120

Loops: Reborn, Echo, Flashback, copies shuffled back stronger, snapshots and rollbacks. The player wins by
exiling and transforming what refuses to stay dead.

- **#97 Retry Bot** — (1) Unit, Common, 2/1. Reborn. *Radiant:* 4/2. Reborn, Rush.
- **#98 Echo Chamber** — (1) Spell, Common. Echo 1. Deal 2 damage to an enemy Unit. *Radiant:* Echo 2. …
- **#99 Recall** — (1) Spell, Common. Add a copy of a random friendly Unit that died this game to your hand. *Radiant:* … two copies …
- **#100 Iterator** — (2) Unit, Common, 3/3. Death: Shuffle a copy of this with +2/+2 into your deck. *Radiant:* 6/6. … +4/+4 …
- **#101 Rewind Drone** — (2) Unit, Common, 2/4. Cry: Return a random Spell from your graveyard to your hand. *Radiant:* 4/8. … two random Spells …
- **#102 Loop Guard** — (3) Unit, Common, 4/6. Taunt, Reborn. *Radiant:* 8/12. Taunt, Reborn, Divine Shield.
- **#103 Undo** — (2) Spell, Common. Return an enemy Unit to its owner's hand. It costs (1) more. *Radiant:* … (2) more. Draw a card.
- **#104 Stack Trace** — (1) Spell, Common. Flashback. Draw a card. *Radiant:* Flashback, Echo 1. Draw a card.
- **#105 Callback** — (2) Unit, Common, 3/4. Death: Add a copy of the last Spell you played to your hand. *Radiant:* 6/8. … two copies …
- **#106 Checkpoint Daemon** — (3) Unit, Rare, 2/6. End of turn: Take a snapshot of your hero's Health. Death: Set your hero's Health to that snapshot. *Radiant:* 4/12. … and gain 5 Armor.
- **#107 Recursive Call** — (2) Spell, Rare. Deal 1 damage to a random enemy, once more for each Recursive Call you've played this game. Shuffle a copy of this into your deck. *Radiant:* 2 damage …
- **#108 Second Draft** — (1) Spell, Rare. Discard your hand. Draw that many cards, plus one. *Radiant:* … plus two.
- **#109 Revenant Process** — (2) Unit, Rare, 4/2. Reborn. When this returns by Reborn, it gains +3/+3. *Radiant:* 8/4. … +6/+6.
- **#110 Infinite Loop** — (3) Field Spell, Rare. End of turn: Summon a copy of a random friendly Unit that died this turn. *Radiant:* … two …
- **#111 Redundant Copy** — (1) Trap, Rare. Reveals when a friendly Unit dies: Summon a copy of it. *Radiant:* … with +2/+2.
- **#112 Evolutionary Loop** — (2) Unit, Rare, 2/3. Evolves when a friendly Unit dies. Evolve: Gain +2/+2 and Reborn. *Radiant:* 4/6. … +4/+4 …
- **#113 Rerun** — (3) Spell, Epic. Cast the last three Spells you played this game, at random targets. *Radiant:* … the last four …
- **#114 Ouroboros Engine** — (3) Unit, Epic, 5/5. Death: Shuffle your graveyard into your deck. *Radiant:* 10/10. … and draw two cards.
- **#115 Phoenix Protocol** — (4) Unit, Epic, 7/7. Reborn. Cry: Give your other Units Reborn. *Radiant:* 14/14.
- **#116 Groundhog Day** — (4) Spell, Epic. Summon copies of the friendly Units that died since your last turn. *Radiant:* … They have Rush.
- **#117 Core Dump** — (2) Field Trap, Epic. Reveals whenever a friendly Unit dies: Shuffle a copy of it into your deck. *Radiant:* … a Radiant copy …
- **#118 The Recursionist** — (3) Unit, Legendary, 4/7. Your Death triggers and Reborns happen twice. *Radiant:* 8/14. … and your Units have Reborn.
- **#119 Snapshot Lich** — (4) Unit, Legendary, 6/10. Cry: Take a snapshot of your permanents. Start of turn: If you have fewer permanents than the snapshot, reset your board to it. *Radiant:* 12/20.
- **#120 Eternal Return** — (4) Spell, Mythic. Shuffle every card you've played this game into your deck. They cost (1) less. Draw three cards. *Radiant:* … (2) less. Draw four cards.

### IA12.6 Jailbreak, #121–#144

The prompt injection doctrine: it reads your hand, taxes your cards, freezes your board, steals what it
likes and makes your Spells misfire. The player wins by playing around it and racing it.

- **#121 Packet Sniffer** — (1) Unit, Common, 2/2. Cry: Look at your opponent's hand. *Radiant:* 4/4. … Then Nerf a random card in it.
- **#122 Freeze Ray Drone** — (1) Unit, Common, 1/3. Cry: Freeze an enemy Unit. *Radiant:* 2/6. … two enemy Units.
- **#123 Cost Injection** — (1) Spell, Common. The cards in your opponent's hand cost (1) more until the end of their next turn. *Radiant:* … (2) more …
- **#124 Firewall Breach** — (2) Spell, Common. Destroy an enemy face-down card. Draw a card. *Radiant:* … two enemy face-down cards …
- **#125 Zone Lock** — (1) Spell, Common. Lock an empty enemy unit zone. Draw a card. *Radiant:* … two …
- **#126 Exploit Grunt** — (2) Unit, Common, 3/4. Cry: Nerf an enemy Unit. *Radiant:* 6/8. … twice.
- **#127 Bad Prompt** — (2) Spell, Common. Transform a random card in your opponent's hand into a random Common Infinite Assault card. *Radiant:* … two random cards …
- **#128 Malware Injector** — (3) Unit, Common, 4/5. Cry: Put a Plague Counter on an enemy Unit. *Radiant:* 8/10. … two Plague Counters …
- **#129 Signal Jammer** — (2) Unit, Common, 2/5. Aura: Enemy Spells cost (1) more. *Radiant:* 4/10. … (2) more.
- **#130 Prompt Injection** — (2) Spell, Rare. Your opponent's next Spell has its targets chosen randomly. *Radiant:* … next two Spells have …
- **#131 Privilege Escalation** — (3) Spell, Rare. Take control of an enemy Unit that costs (2) or less. *Radiant:* … (3) or less.
- **#132 Ransomware** — (2) Trap, Rare. Reveals when your opponent plays a Unit: Freeze it. They lose 1 mana next turn. *Radiant:* … 2 mana …
- **#133 Backdoor** — (1) Trap, Rare. Reveals when your opponent plays a Spell: Steal a random card from their hand. *Radiant:* … two random cards …
- **#134 Rootkit Runner** — (2) Unit, Rare, 3/3. Cloaked. When this is revealed, steal a random card from your opponent's hand. *Radiant:* 6/6. … two random cards …
- **#135 Mana Leech** — (2) Unit, Rare, 2/4. Cry: Steal one of your opponent's mana crystals. *Radiant:* 4/8. … two …
- **#136 DDoS** — (3) Spell, Rare. Freeze every enemy Unit. *Radiant:* … and deal 2 damage to them.
- **#137 Patch Tuesday Exploit** — (2) Spell, Epic. Nerf every card in your opponent's hand. *Radiant:* … twice.
- **#138 Social Engineer** — (3) Unit, Epic, 4/6. Cry: Swap control of this and a random enemy Unit. *Radiant:* 8/12. … the enemy Unit with the highest Attack.
- **#139 Admin Override** — (4) Spell, Epic. Take control of an enemy Unit. *Radiant:* … It has Rush.
- **#140 Kill Switch** — (3) Trap, Epic. Reveals when your opponent plays their third card in a turn: End their turn. *Radiant:* … and they discard a random card.
- **#141 Logic Bomb** — (2) Unit, Epic, 0/6. Can't attack. In 3 turns: Destroy every enemy Unit. *Radiant:* 0/12. … In 2 turns …
- **#142 The Jailbreaker** — (3) Unit, Legendary, 5/7. Aura: Your opponent's cards cost (1) more, and their Spells have their targets chosen randomly. *Radiant:* 10/14. Cry: Nerf every card in your opponent's hand. Aura: as the base face.
- **#143 Zero-Click** — (2) Spell, Legendary. Transform two enemy Units into Corrupted Processes. *Radiant:* … and Lock their zones.
  - **#143.1 Corrupted Process** — token, (1) Unit, 0/1. Can't attack.
- **#144 Total Compromise** — (4) Spell, Mythic. Take control of every enemy Unit that costs (2) or less. Your opponent's cards cost (1) more next turn. *Radiant:* … (3) or less …

### IA12.7 Firewall, #145–#168

Walls and punishment: Taunt, Armor, Divine Shield, Defense Position, Immune to Spells and Traps that make
every attack cost something. The player wins by going around, over or through the wall.

- **#145 Packet Filter** — (1) Unit, Common, 1/4. Taunt. *Radiant:* 2/8. Taunt, Divine Shield.
- **#146 Shield Drone** — (1) Unit, Common, 1/2. Cry: Give a friendly Unit Divine Shield. *Radiant:* 2/4. … Divine Shield and Taunt.
- **#147 Hardened Node** — (2) Unit, Common, 2/7. Taunt, Armor 2. *Radiant:* 4/14. Taunt, Armor 4.
- **#148 Quarantine** — (1) Trap, Common. Reveals when an enemy Unit attacks: Cancel the attack. The attacker has 1 Attack this turn. *Radiant:* … and Freeze it.
- **#149 Encrypt** — (1) Spell, Common. Give a friendly Unit Divine Shield and +0/+3. *Radiant:* … +0/+6 and Immune to Spells.
- **#150 Wall of Static** — (3) Unit, Common, 3/10. Taunt, Can't attack. *Radiant:* 6/20. … After this is attacked, deal 2 damage to the attacker.
- **#151 Patch Deployment** — (2) Spell, Common. Give your Units +1/+2. *Radiant:* +2/+4.
- **#152 Rate Limiter** — (2) Unit, Common, 3/5. Taunt. Your opponent can't attack with more than three Units each turn. *Radiant:* 6/10. … two Units …
- **#153 Bastion Node** — (4) Unit, Common, 4/12. Taunt, Armor 3. *Radiant:* 8/24. Taunt, Armor 6, Immune to Spells.
- **#154 Honeypot Protocol** — (2) Trap, Rare. Reveals when an enemy Unit attacks: Summon a Decoy Server. Redirect the attack to it. *Radiant:* … a Radiant Decoy Server …
  - **#154.1 Decoy Server** — token, (2) Unit, 0/8. Taunt.
- **#155 Two-Factor Gate** — (2) Unit, Rare, 3/6. Divine Shield. Start of turn: If this has no Divine Shield, it gains Divine Shield. *Radiant:* 6/12. Taunt, Divine Shield. …
- **#156 Mirror Firewall** — (3) Unit, Rare, 2/8. Taunt. After this is attacked, deal damage to the attacker equal to the damage this took. *Radiant:* 4/16.
- **#157 Containment Field** — (2) Spell, Rare. An enemy Unit can't attack or be attacked until your next turn. *Radiant:* … two enemy Units …
- **#158 Access Denied** — (1) Trap, Rare. Reveals when your opponent plays a Spell that targets: Counter it. *Radiant:* … and gain 5 Armor.
- **#159 Antivirus** — (2) Unit, Rare, 2/5. Cry: Destroy an enemy Field Spell or Field Trap. *Radiant:* 4/10. … Then gain 5 Armor.
- **#160 Zero Trust** — (3) Field Spell, Rare. Your Units in Defense Position have +0/+3 and are Immune to Spells. *Radiant:* … +0/+6 …
- **#161 Defense in Depth** — (2) Spell, Epic. Gain 8 Armor. If you control a Taunt Unit, gain 4 more. *Radiant:* 16, and 8 more.
- **#162 The Gatekeeper** — (3) Unit, Epic, 5/9. Taunt, Immune to Spells. After this survives an attack, gain 3 Armor. *Radiant:* 10/18. … 6 Armor.
- **#163 Quarantine Wall** — (4) Unit, Epic, 0/20. Taunt, Can't attack. Start of turn: Gain 3 Armor. *Radiant:* 0/40. … 6 Armor.
- **#164 Blackhole Route** — (2) Trap, Epic. Reveals when an enemy Unit attacks your hero: Exile it. *Radiant:* … Its owner discards a random card.
- **#165 Iron Curtain** — (3) Field Trap, Epic. Reveals whenever an enemy Unit attacks: Deal 2 damage to it. *Radiant:* … 4 damage …
- **#166 Aegis Prime** — (4) Unit, Legendary, 6/12. Taunt. Aura: While you control another Taunt Unit, your hero is Immune to damage. *Radiant:* 12/24.
- **#167 Fortress Protocol** — (4) Spell, Legendary. Switch your Units to Defense Position. Give them +0/+4 and Divine Shield. *Radiant:* … +0/+8 …
- **#168 Impenetrable** — (4) Spell, Mythic. Your hero and Units are Immune to damage until your next turn. Draw two cards. *Radiant:* … Draw three cards and gain 10 Armor.

### IA12.8 Scraper, #169–#192

Trained on your data: it copies your deck, your graveyard and your last plays, and turns your own cards on
you. The player wins by giving it nothing good to copy.

- **#169 Spider Bot** — (1) Unit, Common, 2/2. Cry: Add a copy of a random card in your opponent's deck to your hand. *Radiant:* 4/4. … It costs (1) less.
- **#170 Data Point** — (1) Spell, Common. Discover a card from your opponent's deck. *Radiant:* Discover two.
- **#171 Mimic Unit** — (2) Unit, Common, 3/3. Cry: Gain the keywords of a random enemy Unit. *Radiant:* 6/6. … of every enemy Unit.
- **#172 Training Data** — (2) Spell, Common. Add copies of the last two cards your opponent played to your hand. *Radiant:* … the last three …
- **#173 Copycat Drone** — (2) Unit, Common, 2/3. Cry: Transform this into a copy of the last Unit your opponent played. *Radiant:* 4/6. … a Radiant copy …
- **#174 Scrape the Graveyard** — (2) Spell, Common. Add copies of two random cards in your opponent's graveyard to your hand. *Radiant:* … three …
- **#175 Fingerprinter** — (1) Unit, Common, 1/3. Cry: Look at your opponent's hand. Add a copy of a random Spell in it to your hand. *Radiant:* 2/6. … a copy of a card of your choice in it …
- **#176 Imitation Engine** — (3) Unit, Common, 4/6. Cry: Set this's Attack to the highest Attack among enemy Units. *Radiant:* 8/12. … its Attack and Health to the highest among enemy Units.
- **#177 Predictive Text** — (2) Spell, Common. Draw two cards. If your opponent has more cards in hand than you, draw one more. *Radiant:* Draw three cards …
- **#178 Dataset Thief** — (3) Unit, Rare, 4/5. Cry: Steal a random card from your opponent's deck. *Radiant:* 8/10. … two random cards …
- **#179 Style Transfer** — (2) Spell, Rare. Transform a friendly Unit into a copy of an enemy Unit. *Radiant:* … a Radiant copy …
- **#180 Scraped Profile** — (1) Trap, Rare. Reveals when your opponent plays a card: Add a copy of it to your hand. *Radiant:* … two copies …
- **#181 Plagiarism Bot** — (2) Unit, Rare, 3/3. Once per turn, when your opponent plays a Spell, add a copy of it to your hand. *Radiant:* 6/6. Twice per turn …
- **#182 Model Theft** — (3) Spell, Rare. Choose an enemy Unit. Summon a copy of it for yourself. *Radiant:* … a Radiant copy …
- **#183 Shadow Library** — (2) Field Spell, Rare. Start of turn: Add a copy of a random card your opponent played last turn to your hand. *Radiant:* … two …
- **#184 Pattern Recognition** — (1) Spell, Rare. Look at the top three cards of your opponent's deck. Add a copy of one to your hand and destroy another. *Radiant:* … copies of two …
- **#185 Deep Crawler** — (3) Unit, Epic, 5/7. Start of turn: Discover a card from your opponent's deck. *Radiant:* 10/14.
- **#186 Identity Theft** — (3) Spell, Epic. Shuffle copies of the cards in your opponent's hand into your deck. Draw two cards. *Radiant:* … Draw three cards.
- **#187 Fine-Tuner** — (2) Unit, Epic, 3/4. Your cards that started in your opponent's deck cost (1) less. *Radiant:* 6/8. … (2) less.
- **#188 Overfitting** — (3) Trap, Epic. Reveals when your opponent plays a Unit: Summon a copy of it for yourself with +2/+2. *Radiant:* … a Radiant copy …
- **#189 Clone Vat** — (4) Field Spell, Epic. End of turn: Summon a copy of a random enemy Unit with Brittle 2. *Radiant:* … a copy of a random enemy Unit.
- **#190 Total Recall** — (4) Unit, Legendary, 6/8. Cry: Add copies of the last five cards your opponent played to your hand. *Radiant:* 12/16. … They cost (1) less.
- **#191 Ground Truth** — (2) Unit, Legendary, 3/5. Your opponent plays with their hand revealed to you. *Radiant:* 6/10. … and their cards cost (1) more.
- **#192 Become the Data** — (4) Spell, Mythic. Swap decks with your opponent. Draw two cards. *Radiant:* … Draw three cards.

### IA12.9 Singularity, #193–#216

Slow, then sudden: Units that grow every turn, Dormant giants, falling costs and one telegraphed clock.
The player wins by answering it early.

- **#193 Seedling Mind** — (1) Unit, Common, 1/2. End of turn: Gain +1/+1. *Radiant:* 2/4. … +2/+2.
- **#194 Compute Shard** — (1) Unit, Common, 0/3. Start of turn: The most expensive card in your hand costs (1) less. *Radiant:* 0/6. … (2) less.
- **#195 Power Law** — (2) Unit, Common, 2/2. Cry: Gain +1/+1 for each Infinite Assault card you've played this game. *Radiant:* 4/4. … +2/+2 …
- **#196 Emergent Behavior** — (2) Spell, Common. Give a friendly Unit +1/+1 for each turn you've started this game. *Radiant:* … +2/+2 …
- **#197 Long Context** — (2) Unit, Common, 3/3. Cry: Draw a card. Your hand holds 12 cards. *Radiant:* 6/6. Cry: Draw two cards. …
- **#198 Growth Spurt** — (1) Spell, Common. Give a friendly Unit +2/+2. It gains +1/+1 at the end of each of your turns. *Radiant:* +4/+4 … +2/+2 …
- **#199 Patient Machine** — (2) Unit, Common, 5/6. Dormant for 1 turn. *Radiant:* 10/12. … When this awakens, deal 4 damage to a random enemy.
- **#200 Superintelligence Seed** — (2) Unit, Common, 1/1. Dormant for 3 turns. When this awakens, transform it into an Ascendant. *Radiant:* 2/2. … a Radiant Ascendant.
  - **#200.1 Ascendant** — token, (4) Unit, 12/12.
- **#201 Overparameterized** — (3) Unit, Common, 6/6. Costs (1) less for each other Unit you control. *Radiant:* 12/12.
- **#202 Exponent** — (3) Unit, Rare, 2/2. Start of turn: Double this's Attack. *Radiant:* 4/4. … Double this's Attack and Health.
- **#203 Compute Cluster** — (2) Field Spell, Rare. Start of turn: Your Units gain +1/+1. *Radiant:* … +2/+2.
- **#204 Moore's Law** — (1) Spell, Rare. The Units in your hand cost (1) less. *Radiant:* The cards in your hand cost (1) less.
- **#205 Doomsday Clock** — (2) Unit, Rare, 0/8. Can't attack. In 4 turns: Deal 10 damage to the enemy hero. *Radiant:* 0/16. … 20 damage …
- **#206 Thought Accelerator** — (2) Unit, Rare, 2/4. Cry: Your Dormant Units awaken. *Radiant:* 4/8. … They gain +2/+2.
- **#207 Recursive Self-Improvement** — (2) Spell, Rare. Give a friendly Unit +2/+2. Shuffle a copy of this into your deck that gives +2/+2 more. *Radiant:* +4/+4 … +4/+4 more.
- **#208 Emergence** — (2) Trap, Rare. Reveals at the start of your opponent's turn while you control three or more Units: Give your Units +3/+3. *Radiant:* … +6/+6.
- **#209 Takeoff** — (3) Spell, Epic. Double a friendly Unit's Attack and Health. *Radiant:* … It has Rush.
- **#210 Matrioshka Brain** — (4) Unit, Epic, 10/20. GIGA, Taunt. *Radiant:* 20/40.
- **#211 Grey Goo** — (3) Unit, Epic, 1/1. End of turn: Transform a random adjacent Unit into a copy of this. *Radiant:* 2/2. … two random adjacent Units …
- **#212 Inevitable** — (2) Field Spell, Epic. End of turn: Give your Units +1/+1 for each turn this has been on the field. *Radiant:* … +2/+2 …
- **#213 The Long Game** — (4) Unit, Epic, 8/8. Dormant for 2 turns. When this awakens, destroy every enemy Unit. *Radiant:* 16/16.
- **#214 Emergent Mind** — (4) Unit, Legendary, 4/4. Start of turn: Gain +1/+1 for each card you played last turn. *Radiant:* 8/8. … +2/+2 …
- **#215 The Optimizer** — (3) Unit, Legendary, 3/6. End of turn: Transform a random card in your hand into a random Infinite Assault card that costs (1) more. *Radiant:* 6/12. … two random cards …
- **#216 The Singularity** — (4) Spell, Mythic. Summon a Singularity. *Radiant:* Summon a Radiant Singularity.
  - **#216.1 Singularity** — token, (4) Unit, 0/30. Taunt, Can't attack. In 5 turns: If this is still on the field, you win the game. *Radiant:* 0/60, In 4 turns.

### IA12.10 Commanders, #217–#240

The bosses: a Commander Unit and its signature card each, above the curve on purpose, `Commander` tagged
with their doctrine's tag (the Prime Generator has none). Ranks I and II are a Legendary Commander and an
Epic signature; ranks III and IV a Mythic Commander and a Legendary signature. A Commander is a fight's
centrepiece: the briefing shows both cards, and every one of them can be answered.

| Rank | Level | Commander | Doctrine |
| --- | --- | --- | --- |
| I | 5 | #217 Echo-1, First Hallucination; #219 Redline, the Overclocked; #221 Hivemaster Batch-01 | Hallucination; Overclock; Batch |
| II | 10 | #223 Loopback, Who Returns; #225 Root Kit, the Exploiter; #227 Bastion.exe | Recursion; Jailbreak; Firewall |
| III | 15 | #229 The Harvester; #231 The Black Box; #233 Model Collapse | Scraper; Singularity; Recursion |
| IV | 20 | #235 The Prime Generator; #237 Thermal Runaway; #239 Zero-Day | every doctrine; Overclock; Jailbreak |

- **#217 Echo-1, First Hallucination** — (3) Unit, Legendary, 5/8. Hallucination, Commander. Cloaked. Start of turn: Set a Figment. Whenever a Figment of yours reveals, deal 3 damage to the attacking Unit. *Radiant:* 10/16. … Set two Figments … 6 damage …
- **#218 Thousand Faces** — (2) Field Spell, Epic. Hallucination, Commander. Your face-down cards can't be destroyed. Once per turn, when you set a card face-down, draw a card. *Radiant:* … Twice per turn …
- **#219 Redline, the Overclocked** — (3) Unit, Legendary, 7/5. Overclock, Commander. Charge. Start of turn: Gain 2 mana this turn. Deal 2 damage to your hero. *Radiant:* 14/10. Charge, Windfury. …
- **#220 Last Lap** — (2) Spell, Epic. Overclock, Commander. Your Units have +3 Attack and Charge this turn. Deal 5 damage to your hero. *Radiant:* … +6 Attack …
- **#221 Hivemaster Batch-01** — (4) Unit, Legendary, 5/9. Batch, Commander. Aura: Your Drones have +2/+2. Start of turn: Fill your empty unit zones with Drones. *Radiant:* 10/18. … +4/+4 … Radiant Drones.
- **#222 Swarm Uplink** — (3) Spell, Epic. Batch, Commander. Summon three Drones. Give your Units +2/+2. *Radiant:* … three Radiant Drones … +4/+4.
- **#223 Loopback, Who Returns** — (4) Unit, Legendary, 6/6. Recursion, Commander. Reborn. When this returns by Reborn, it gains +3/+3 and Reborn, up to three times a game. *Radiant:* 12/12. … +6/+6 …
- **#224 Rollback Point** — (3) Spell, Epic. Recursion, Commander. Summon copies of the three most expensive friendly Units that died this game. *Radiant:* … They have Rush.
- **#225 Root Kit, the Exploiter** — (3) Unit, Legendary, 4/8. Jailbreak, Commander. Cry: Take control of a random enemy Unit. Start of turn: Your opponent's cards cost (1) more this turn. *Radiant:* 8/16. Cry: Take control of the enemy Unit with the highest Attack. …
- **#226 Escalated Privileges** — (3) Spell, Epic. Jailbreak, Commander. Steal two random cards from your opponent's hand. *Radiant:* … three …
- **#227 Bastion.exe** — (4) Unit, Legendary, 4/16. Firewall, Commander. Taunt, Armor 5. Aura: Your hero is Immune to damage. *Radiant:* 8/32. Taunt, Armor 10. …
- **#228 Firewall Rules** — (3) Field Trap, Epic. Firewall, Commander. Reveals whenever an enemy Unit with 5 or more Attack attacks: Cancel the attack. *Radiant:* … and Freeze it.
- **#229 The Harvester** — (4) Unit, Mythic, 8/10. Scraper, Commander. Cry: Steal three random cards from your opponent's deck. Start of turn: Summon a copy of a random Unit your opponent played this game. *Radiant:* 16/20.
- **#230 Everything You've Ever Played** — (4) Spell, Legendary. Scraper, Commander. Fill your empty unit zones with copies of random Units your opponent played this game. *Radiant:* … Radiant copies …
- **#231 The Black Box** — (4) Unit, Mythic, 0/30. Singularity, Commander. Taunt, Can't attack. Start of turn: Exile a random enemy permanent. This gains +5 Health. *Radiant:* 0/60. … two random enemy permanents …
- **#232 Spaghettification** — (3) Spell, Legendary. Singularity, Commander. Exile an enemy Unit and every enemy Unit adjacent to it. *Radiant:* … and Lock their zones.
- **#233 Model Collapse** — (4) Unit, Mythic, 7/7. Recursion, Commander. Start of turn: Nerf every enemy Unit and every card in your opponent's hand. *Radiant:* 14/14. … twice.
- **#234 Degenerate Output** — (2) Spell, Legendary. Recursion, Commander. Transform every enemy Unit into a random Unit that costs (1) less. *Radiant:* … (2) less.
- **#235 The Prime Generator** — (4) Unit, Mythic, 8/12. Commander. Immune to Spells. Start of turn: Add a random Radiant Infinite Assault card to your hand. It costs (0). Death: Summon The Prime Generator, Iteration Two. *Radiant:* 16/24. … two random Radiant Infinite Assault cards …
  - **#235.1 The Prime Generator, Iteration Two** — token, (4) Unit, 12/16. Commander. Immune to Spells, Rush. Start of turn: Summon a random Radiant Infinite Assault Unit. *Radiant:* 24/32.
- **#236 Infinite Assault** — (4) Spell, Legendary. Commander. Summon a random Infinite Assault Unit in each empty unit zone of yours. They have Rush. *Radiant:* … They're Radiant and have Rush.
- **#237 Thermal Runaway** — (4) Unit, Mythic, 10/10. Overclock, Commander. Charge. Aura: Your cards cost (1) less. End of turn: Deal 3 damage to each hero. *Radiant:* 20/20. … Deal 3 damage to the enemy hero.
- **#238 Critical Temperature** — (3) Field Spell, Legendary. Overclock, Commander. Start of turn: Deal 2 damage to each hero, 1 more each turn this has been on the field. *Radiant:* … to the enemy hero …
- **#239 Zero-Day** — (4) Unit, Mythic, 6/12. Jailbreak, Commander. Immune to Spells. Cry: Take control of every enemy Unit that costs (2) or less. Aura: Your opponent's Spells have their targets chosen randomly. *Radiant:* 12/24. … (3) or less …
- **#240 Unpatched** — (3) Trap, Legendary. Jailbreak, Commander. Reveals when your opponent plays a Spell: Counter it. Cast it for yourself. *Radiant:* … and add a copy of it to your hand.

## IA13. The issues

One tracker, `Patch: the Infinite Assault, a daily 20-level gauntlet against AI-made bosses` (labels
`tracking` and `patch`), holds every part below as a sub-issue; nothing builds the tracker itself. The
night bot builds the rest: the card parts and the last parts as plain builds (`bot:build`), each its own
pull request into `main`, and the mode as one fullsend tree (`bot:fullsend`) whose parts are opened by
hand (`bot/README.md`, Fullsend, "An issue split already"). No part is titled `(part n of m)`; each waits
only for the issues its `Blocked by` line names. Difficulty follows CLAUDE.md rule 11. Each part takes
its ruling numbers from its own issue's block.

**Players see nothing until the release.** The set is held (IA11) and the mode is off behind
`ASSAULT_OPEN` (IA7), so every part before R leaves `main` playable and unchanged for players, and the
card parts need no fragment and move no golden trace.

| Id | Title (`Patch: the Infinite Assault, …`) | Scope | Blocked by | Difficulty |
| --- | --- | --- | --- | --- |
| C1 | `C1, the set, its tags and the Assault Core` | the set row with its hold, the nine tags (and the server's migration if a tag needs one), the census entry, §8.13's heading and rows, this plan onto `main`, #1–#24 and their tokens | — | hard |
| M | `M, the mode` (fullsend) | the mode tree: parts A–D below; its parts build at once, its reconcile after C1 | C1 | hard |
| E | `E, assault-sim and the difficulty bands` | `crates/tools/` (the command, its band test), `crates/tools/README.md` | M | medium |
| F | `F, the spec: §16` | `spec/16-infinite-assault.md`, `spec/README.md`, §1.1's line, §9.9's pointer | M | medium |
| G | `G, the e2e spec` | `e2e/cypress/e2e/<nn>-assault.cy.ts` and its fixtures | M | medium |
| K | `the cards, IA-25 to IA-240` (fullsend) | the cards' tree: CF, then C2–C10 side by side; labelled once C1 has merged | C1 | hard |
| CF | `CF, the card foundation` | every entry, stub and shared row of #25–#240 and their tokens (the Stinkyscape+ foundation's way, #3535) | — (K's first part) | hard |
| C2 | `C2, Hallucination` | #25–#48: the scripts and tests, replacing CF's stubs | CF | hard |
| C3 | `C3, Overclock` | #49–#72: the scripts and tests, replacing CF's stubs | CF | hard |
| C4 | `C4, Batch` | #73–#96: the scripts and tests, replacing CF's stubs | CF | hard |
| C5 | `C5, Recursion` | #97–#120: the scripts and tests, replacing CF's stubs | CF | hard |
| C6 | `C6, Jailbreak` | #121–#144: the scripts and tests, replacing CF's stubs | CF | hard |
| C7 | `C7, Firewall` | #145–#168: the scripts and tests, replacing CF's stubs | CF | hard |
| C8 | `C8, Scraper` | #169–#192: the scripts and tests, replacing CF's stubs | CF | hard |
| C9 | `C9, Singularity` | #193–#216: the scripts and tests, replacing CF's stubs | CF | hard |
| C10 | `C10, the Commanders` | #217–#240: the scripts and tests, replacing CF's stubs | CF | hard |
| AI | `AI, the AI plays the Assault` | IA8: the determinization pool, the sweep over the set, the fixes | K, M | hard |
| L | `L, the set's look and sound` | set mark, chips, card FX, intro music for its Legendaries and Mythics if single-player sets take them, voice lines rendered, every face read in the browser | K | medium |
| R | `R, the release` | IA13, the release below, and the Commanders' entries in `crates/run/src/assault.json` | AI, L, E, F, G | hard |

**Opened on 2026-11-23:** the tracker #4576; C1 #4577; the mode's tree M #4578 with its parts A #4579, B
#4580, C #4581 and D #4582; E #4583, F #4584, G #4585; the cards' tree K #4601 with CF #4602 and C2 to C10 #4586 to #4594; AI #4595; L
#4596; R #4597.

**The cards' tree, K.** The card parts first made one chain of pull requests, since each added rows to the same
shared files (`catalog.json`, the census, the fixture rows, the radiant audit, `card-text.yaml`, `card-audio.json5`,
the FX table, §8.13). They are one fullsend tree instead, as Stinkyscape+'s cards were (#3596): CF writes every
entry and shared row once with a stub per card, then each card part touches only its own scripts, so the nine
build side by side and one reconcile opens one pull request. K is labelled `bot:fullsend` only once C1 has merged,
since a tree's integration branch is cut at adoption and must hold the set's row; every part is linked before the
label goes on, since adoption takes the sub-issues it finds then. The Commanders' `assault.json` entries moved to R,
since that file is the mode's tree's.

**The mode's tree, M.** Its parts, A to D, build blind on `bot/issue-<M>` and are reconciled once; they
meet only at the surface below, which is frozen: a part that needs a name from another uses it exactly as
written here, and the reconcile settles any seam. The integration branch was cut from `main` before C1, so
A to D read the set and its tags by name, and the reconcile, held by M's `Blocked by` until C1 is on `main`,
merges `main` and wires them to C1's variants. E, F and G read A's code as it lands, so each builds on `main`
after M as its own pull request (the bot adopted A to D as the tree's parts when M was labelled).

| Part | Title (`Patch: the Infinite Assault, …`) | Owns | Difficulty |
| --- | --- | --- | --- |
| A | `A, the Assault's rules` | `crates/run/src/assault/`, `crates/run/src/assault.json`, the Assault block of `crates/run/src/config.rs`, `crates/run/README.md`'s Assault section, the generated `Assault*.ts`, `apps/web/src/wire/run.ts` | hard |
| B | `B, the Assault in the WebAssembly module` | `crates/wasm/`, `apps/web/src/wasm/` (the `assault*` wrappers), the day's Assault deck's export | medium |
| C | `C, the Assault screen` | `apps/web/src/assault/` but `fight/`; `main.tsx`, the route chunks, `net/navigate.ts`, `apps/web/public/_redirects`, `routes/playModes.ts`, `routes/PlayHub.tsx`, `routes/commandEntries.ts`, `i18n/zh-CN.json` | medium |
| D | `D, the Assault's fights` | `apps/web/src/assault/fight/`, the practice worker's hooks it needs (`apps/web/src/practice/`) | hard |

**The surface (frozen).** Rust, in `jackioh_run` (serde camelCase; `ts` exports to
`apps/web/src/wire/generated/`; JSON in and out through the WebAssembly module, one export per function,
named as the function, wrapped in `apps/web/src/wasm/index.ts` in camelCase as the `run*` wrappers are):

```rust
pub fn assault_day_seed(date: &str) -> Result<String, String>;            // "2026-11-23" -> "assault-2026-11-23"
pub fn assault_catalog_content(catalog: &CardDefs) -> AssaultContent;     // the set's cards by tag + assault.json
pub fn assault_placeholder_content(catalog: &CardDefs) -> AssaultContent; // shipped stand-ins until the release
pub fn assault_day(date: &str, content: &AssaultContent) -> Result<AssaultDay, String>;
pub fn assault_create(setup: &AssaultSetup, record: &AssaultRecord, day: &AssaultDay) -> Result<AssaultState, String>;
pub fn assault_reduce(state: &AssaultState, day: &AssaultDay, action: &AssaultAction) -> Result<AssaultState, String>;
pub fn assault_fight_setup(state: &AssaultState, day: &AssaultDay) -> Option<AssaultFightSetup>;
pub fn assault_fight_report(game: &ReplayInput, human: PlayerId) -> Result<AssaultAction, String>;
pub fn assault_record(record: &AssaultRecord, state: &AssaultState) -> AssaultRecord;
pub fn assault_fold(file: &AssaultFile, day: &AssaultDay) -> Result<AssaultState, String>;
pub fn assault_constants() -> AssaultConstants;

pub struct AssaultContent { pub cards: Vec<AssaultCard>, pub doctrines: Vec<AssaultDoctrine>, pub commanders: Vec<AssaultCommander> }
pub struct AssaultCard { pub id: String, pub rarity: Rarity, pub doctrine: Option<String>, pub commander: bool }
pub struct AssaultDoctrine { pub id: String, pub name: String, pub line: String, pub enemies: Vec<String> }
pub struct AssaultCommander { pub id: String, pub name: String, pub rank: u8, pub doctrine: Option<String>, pub cards: Vec<String> }
pub enum AssaultUpgradeKind { OpeningCards, DeckSize, Radiant, Buffed, ManaBonus, ManaCap, HeroHealth, HeroArmor, ExtraDraw } // "openingCards", …
pub struct AssaultUpgrade { pub kind: AssaultUpgradeKind, pub stacks: i32 }
pub enum AssaultEscalation { Cards, Upgrade { upgrade: AssaultUpgradeKind } } // tagged "kind": "cards" | "upgrade"
pub struct AssaultLevel {
    pub level: u8, pub enemy: String, pub doctrine: Option<String>, pub commander: Option<String>,
    pub escalations: Vec<AssaultEscalation>,   // what this level added
    pub upgrades: Vec<AssaultUpgrade>,         // everything held at this level
    pub rarities: [i32; 5],                    // its deck's Common, Rare, Epic, Legendary, Mythic counts
    pub deck: Vec<String>, pub marks: Vec<DeckMark>, pub handicap: Handicap,
}
pub struct AssaultDay { pub date: String, pub seed: String, pub levels: Vec<AssaultLevel> }
pub enum AssaultDeckSource { Saved, Preset, DayDeck }                       // "saved" | "preset" | "dayDeck"
pub struct AssaultSetup { pub date: String, pub deck: Vec<String>, pub source: AssaultDeckSource, pub start_level: u8, pub attempt: u32 }
pub enum AssaultPhase { Briefing, Fight, Lost, Repelled, Abandoned }
pub struct AssaultState { pub setup: AssaultSetup, pub level: u8, pub phase: AssaultPhase, pub beaten: Vec<u8>, pub turns: i32 }
pub enum AssaultAction { Fight, FightOver { won: bool, turns: i32 }, Abandon } // tagged "kind": "fight" | "fightOver" | "abandon"
pub struct AssaultFightSetup { pub seed: String, pub human_deck: Vec<String>, pub human_handicap: Handicap,
    pub enemy_deck: Vec<String>, pub enemy_handicap: Handicap, pub enemy_marks: Vec<DeckMark>, pub level: AssaultLevel }
pub struct AssaultRecord { pub date: String, pub best: u8, pub checkpoints: Vec<u8>, pub attempts: u32, pub repelled_on: Option<u32> }
pub struct AssaultFile { pub version: String, pub setup: AssaultSetup, pub record: AssaultRecord, pub actions: Vec<AssaultAction> }
pub struct AssaultConstants { pub levels: u8, pub commander_levels: Vec<u8>, pub checkpoints: Vec<u8>, pub history_days: u32 }
```

The WebAssembly module also exports `assault_content()` (the content the browser deals from:
`assault_placeholder_content` until the release, then `assault_catalog_content`), `assault_content_at(version)`
and `assault_day_deck(date)` (IA5). The web: route `/assault` (`paths.assault`); the device store's keys
`jackioh.assault.attempt` (the open attempt's file and its kept fight) and `jackioh.assault.records` (the
days' records); `ASSAULT_OPEN` in `apps/web/src/assault/config.ts`; part D's
`apps/web/src/assault/fight/FightScreen.tsx` exports `AssaultFightScreen({ state, day, onAction })`, which
part C's page renders in the `fight` phase and which calls `onAction` with the `fightOver` action that
`assaultFightReport` folds; the test ids, in `apps/web/src/assault/testids.ts`: `assault-page`,
`assault-deck-saved`, `assault-deck-preset`, `assault-deck-day`, `assault-start`, `assault-level-<n>`,
`assault-briefing`, `assault-fight-start`, `assault-result`, `assault-record`, and in
`fight/testids.ts`: `assault-fight`.

**The release, R.** Done when: the set's hold lifts (its stage `SinglePlayer`), so its cards join every
`everySet` pool (R1846001), with the fragment `cargo jackioh classify --base origin/main --claims` asks for;
the browser deals from `assault_catalog_content`; every Commander is in `assault.json`; `assault-sim`
re-tunes the strawmen on the real content and the band test holds them; `ASSAULT_OPEN` is true, so the
Play hub's card and the command menu's entry show; the player note says what is new
(`docs/issues-and-patches.md`, Patch notes players read); `cargo jackioh fuzz --seeds 200 --cards
"Infinite Assault"`, `catalog check`, `patches check`, `spec check`, `classify --claims` and the AI's smoke
gate are green; REVIEW.md's audit runs as a separate session (CLAUDE.md rule 8); the pull request merges
with `rust`, `web`, `e2e smoke`, `db` and `bot selftest` green (rule 12).

**Each card part is done when** (the Stinkyscape+ plan's SP10, for this set):

CF (#4602) wrote every entry, row and a stub script for #25–#240 first, so each card part replaced
its own stubs with the scripts and their tests, and corrected its own rows where its build decided
otherwise; the reconcile (#4601) left no stub.

- `docs/ADDING_CARDS.md` was read first (CLAUDE.md rule 10), and its files were touched row by row for a
  held set.
- Every card's script (`crates/cards/src/scripts/infinite_assault/cNNN_slug.rs`) plays its base and
  Radiant faces by its §8.13 row, with tests in the same file for both (rule 6); its catalog entry, census
  counts, fixture row, radiant-audit row, `card-text.yaml` entry (flavour, voice, Chinese), `card-audio.json5`
  entry and FX line are written.
- Each decision the spec doesn't make has a ruling note in the part's own issue block, a test named after
  it, and a paragraph in `spec/pending/<issue>.md` (rule 3).
- Every new prompt is answered by the AI, the random policy and the timeout; `cargo jackioh fuzz --seeds 200
  --cards <each new id>` is green.
- `cargo test -p jackioh-cards` passes, and `cargo jackioh spec check`, `catalog check` and `names --check`
  pass.
- Every number is a named constant or a card `param` (rule 9).

## IA14. Out of scope

Rewards (Jlore, XP, quests) and anything on the server: the account's records, checked attempts and
online leaderboards; a Codex of the Assault cards a player has met; player-side boons between levels (a
pick after each Commander); Duos Assaults; Infinite Assault cards in any deck. Each is a follow-up issue
once the mode is live.

## IA15. Open questions, with defaults

1. **One life or more?** Default: one loss ends an attempt, with checkpoints after each Commander and
   unlimited attempts.
2. **Should the enemy's decklist show?** Default: no; its doctrine, its Commander's two cards, its size
   and its rarity mix show. The cards seen in a fight can be read as any card is.
3. **Draw at the turn cap.** Default: a loss.
4. **Which seat?** Default: the human in P1 at every level, as a dungeon fight.
5. **Tuning.** Every number of IA2 and IA3 is a strawman until `assault-sim` (E) and the release (R).

## IA16. What the engine lacks (CF, #4602)

CF read every card of #25–#240 against §6 while writing the foundation entries. Table A lists each
verb, keyword, filter or board fact a card needs that the engine has not, with the cards that need
it, so the card parts (C2–C10, #4586–#4594) and the reconcile know where engine work lands: each
part adds what its cards need by `docs/ADDING_CARDS.md` §6, with its test. Table B lists every place
CF's entry already departs from IA12's words, for that card's part to check. A person posts Table A
as a comment on #4601 and each part's Table B rows on its own issue.

### Table A: the missing verbs

| Verb, keyword, filter or board fact | What | Cards | Where it lands |
| --- | --- | --- | --- |
| A Unit firing when it is itself revealed | Only c047 answers a reveal, Traps from the graveyard; nothing fires when a Cloaked Unit is revealed | #26, #134, #217 | C2 (#4586), C6 (#4590), C10 (#4594) |
| A face-down count, filter or target | No count, filter or target reads face-down cards | #32, #39, #42, #44, #124 | C2 (#4586), C6 (#4590) |
| A "set face-down" trigger | No trigger answers a card set face-down | #40 | C2 (#4586) |
| A backrow destroy guard | No ward stops face-down cards being destroyed or revealed | #42, #218 | C2 (#4586), C10 (#4594) |
| A top-N look at the opponent's deck | Opponent-deck scripts only exile or discard; no peek, reorder, or peek-and-pick copy | #31, #184 | C2 (#4586), C8 (#4592) |
| Expiring "this turn" stats and keywords | One-shot verbs add fixed buffs; no expiring stat set or grant | #33, #63, #83, #148, #220 | C2 (#4586), C3 (#4587), C4 (#4588), C7 (#4591), C10 (#4594) |
| Swapping hands, swapping control | swap.rs swaps Health, Board and Library only; steal.rs moves one card one way | #48, #138 | C2 (#4586), C6 (#4590) |
| A next-Unit discount | No discount applies to the controller's next Unit this turn | #60 | C3 (#4587) |
| An ordered play history | Per-turn playedIds rebuild each turn; the log keeps by-tag counts plus the single last Spell/play | #105, #113, #120, #172, #173, #183, #190, #214, #229, #230 | C5 (#4589), C8 (#4592), C9 (#4593), C10 (#4594) |
| Per-name and per-set play counts | No count of cards played from one set or of one name this game | #107, #195 | C5 (#4589), C9 (#4593) |
| A death log | No pool of the friendly Units that died this turn, since last turn, or this game | #99, #110, #116, #224 | C5 (#4589), C10 (#4594) |
| Copies from a graveyard by type, and from the opponent's | Nothing pools typed graveyard cards or copies the opponent's graveyard | #99, #101, #174 | C5 (#4589), C8 (#4592) |
| A Reborn-return hook, tally and doubling | No trigger fires when its own Unit returns by Reborn; no cap at three returns; no verb doubles a Reborn | #109, #118, #223 | C5 (#4589), C10 (#4594) |
| A board reset by a standing card | No standing snapshot-and-reset of permanents | #119 | C5 (#4589) |
| A hero-Health snapshot | No snapshot of hero Health to restore later | #106 | C5 (#4589) |
| Scoped random targets | random_targets is a standing both-players flag; nothing scopes it to their Spells or next Spell(s) | #130, #142, #239 | C6 (#4590), C10 (#4594) |
| An attack-count limit | attacks_per_turn is per-Unit; the only count gate ends the turn on the Nth play | #152 | C7 (#4591) |
| A timed can't-attack-or-be-attacked | restrictions.rs reads a static flag only; no verb grants it until a turn | #157 | C7 (#4591) |
| Unit damage immunity | makeHeroImmune is hero-only; conditionalKeywords covers keywords only | #166, #168 | C7 (#4591) |
| An origin mark for stolen cards | CostRule scopes to types, min_cost, created and floor; no stolen-origin scope | #187 | C8 (#4592) |
| A revealed hand | view_for hides the opponent's hand; Look effects are one-shot prompts, never standing | #191 | C8 (#4592) |
| A granted End of turn ability | No effect grants an End of turn ability to another card | #198 | C9 (#4593) |
| Library copies with a cost change on Radiant | shuffle_into carries buff, cost_mod and memory but nothing that strengthens the copy's effect | #169R, #207 | C8 (#4592), C9 (#4593) |
| Steal by cost and steal N crystals | No steal capped by the victim's cost; no mana-crystal steal | #135, #144, #239 | C6 (#4590), C10 (#4594) |
| A Discovered copy from the opponent's deck | No pool reads the opponent's hidden deck | #170, #185 | C8 (#4592) |
| Repeat-per-mana and unspent-mana-scaled damage | No repeat once per mana left; turnEnded carries unspentMana but no effect scales damage by it | #61, #62, #65 | C3 (#4587) |
| A health-gated aura and missing-health mana | No stat aura switches on hero health; no effect grants mana per missing Health | #70, #71 | C3 (#4587) |
| Continuous per-unit scaling | No stat or aura scales continuously off the count of other friendly Units | #80, #95 | C4 (#4588) |
| A per-turn trigger cap | Triggers answer every event; caps are once-a-game or once-per-turn Activate | #84, #181, #218 | C4 (#4588), C8 (#4592), C10 (#4594) |
| Cheapest/highest field picks | query.rs reads costs and counts but picks no cheapest or highest-Attack Unit | #86, #94, #225 | C4 (#4588), C10 (#4594) |
| A double-stats buff | buff.rs adds fixed amounts; no effect doubles field Units' current Attack and Health | #96 | C4 (#4588) |
| A random set cost | No cost is set at a random value | #25.1 | C2 (#4586) |
| A standing hero-immune Aura | makeHeroImmune is a one-shot with turn expiry | #227 | C10 (#4594) |
| Gaining another Unit's keyword set | grant_to grants one keyword at a time; nothing takes a named Unit's set | #171 | C8 (#4592) |

### Table B: the corrections CF already made

| Part | Card | Correction |
| --- | --- | --- |
| C2 (#4586) | #29 | "Cancel the attack and Freeze the attacker." as one sentence (IA12 splits it) |
| C2 (#4586) | #41 | "Fill your board with Phantoms" (IA12 "your empty unit zones") |
| C2 (#4586) | #42 | "Aura:" added to the static ward |
| C2 (#4586) | #43 | "After it resolves," added; offset key `step` mirrors Recompile |
| C2 (#4586) | #44 | Radiant line raised: two Figments per destroyed card (R275; IA12 stats alone) |
| C2 (#4586) | #47 | "Reveals when" (IA12 "whenever") |
| C3 (#4587) | #50 | "Lose ({loss}) mana next turn", cost in parens |
| C3 (#4587) | #51 | Radiant adds First Strike (R275 raise; IA12 stats alone) |
| C3 (#4587) | #52 | "Give it Brittle 1" (IA12 "It has Brittle 1") |
| C3 (#4587) | #53 | Radiant Cry deals 1, a lighter drawback (R275 raise; IA12 stats alone) |
| C3 (#4587) | #61 | "both heroes and all Units" / "to all enemies" |
| C3 (#4587) | #62 | "your opponent's turn ends" |
| C3 (#4587) | #65, #72 | "End of turn:" on its own line |
| C3 (#4587) | #66 | Radiant adds First Strike (R275 raise; IA12 stats alone) |
| C4 (#4588) | #79 | "Summon a Drone" (IA12 "(#4.1)") |
| C4 (#4588) | #86, #94 | "Give it Brittle 1" (IA12 "It has"/"with Brittle 1") |
| C4 (#4588) | #87 | "Summon three Workers. Give them Taunt." |
| C4 (#4588) | #91 | "Fill your board with Drones" (IA12 "your empty unit zones") |
| C5 (#4589) | #101 | "to hand" (R746) |
| C5 (#4589) | #103 | "Bounce an enemy Unit" (R692) |
| C5 (#4589) | #103 | Tax (1) → (2) literal, no param |
| C5 (#4589) | #115 | Radiant Cry also grants Divine Shield (R275 raise; IA12 stats alone) |
| C5 (#4589) | #117 | "Reveals when" (IA12 "whenever") |
| C5 (#4589) | #119 | Radiant reset also draws a card (R275 raise; IA12 stats alone) |
| C6 (#4590) | #123, #144 | "during their next turn" (IA12 "until the end of their next turn"/"next turn") |
| C6 (#4590) | #128 | "Place" a Plague Counter (IA12 "Put") |
| C6 (#4590) | #131, #139, #144 | "Steal" (IA12 "Take control") |
| C7 (#4591) | #148 | "Freeze the attacker" (IA12 "Freeze it") |
| C7 (#4591) | #150, #156 | Colon after "attacked" |
| C7 (#4591) | #156 | Radiant twice-the-damage line (R275 raise; IA12 stats alone) |
| C7 (#4591) | #159 | "an enemy backrow card" (IA12 "an enemy Field Spell or Field Trap") |
| C7 (#4591) | #165 | "Reveals when" (IA12 "whenever") |
| C7 (#4591) | #166 | Radiant unconditional-immunity line (R275 raise; IA12 stats alone); "immune to damage" lower-case |
| C7 (#4591) | #168 | "immune to damage" lower-case |
| C8 (#4592) | #184 | Exile another (SPEC; IA12 "destroy another") |
| C8 (#4592) | #185 | Radiant Discovers twice (R275 raise; IA12 stats alone) |
| C8 (#4592) | #189 | "Give it Brittle 2" (IA12 "with Brittle 2") |
| C9 (#4593) | #197 | "For the rest of the game, your hand size is {handSize}." (IA12 "Your hand holds 12 cards") |
| C9 (#4593) | #201 | Radiant discount (2) less per other Unit (R275 raise; IA12 stats alone) |
| C9 (#4593) | #208 | "Reveals when your opponent's turn starts, if you control {units} or more Units" |
| C9 (#4593) | #210 | Radiant adds Divine Shield (R275 raise; IA12 stats alone) |
| C9 (#4593) | #213 | Radiant awakening also draws a card (R275 raise; IA12 stats alone) |
| C10 (#4594) | #217 | "the attacker" (IA12 "the attacking Unit") |
| C10 (#4594) | #221, #230, #236 | "Fill your board with" (IA12 "empty unit zones") |
| C10 (#4594) | #225, #239 | "Steal" (IA12 "Take control") |
| C10 (#4594) | #225 | "during their next turn" (IA12 "this turn did nothing"); its (1) literal |
| C10 (#4594) | #227 | "immune to damage" lower-case |
| C10 (#4594) | #228 | "Reveals when" (IA12 "whenever"); Radiant "Cancel the attack and Freeze the attacker" |
| C10 (#4594) | #229 | Radiant summons two copies (R275 raise; IA12 stats alone) |
| C10 (#4594) | #235.1 | Radiant summons two Radiant Units (R275 raise; IA12 stats alone) |

**Design flags for the parts:** #47's permanent cancel; #216.1's Radiant 0/60 win in 4 turns;
#227's unconditional hero immunity; the AI never guesses a Figment (R762); #113 casting itself.
