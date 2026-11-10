## 15. Dungeon runs

A dungeon run is a single-player run against the AI, in roguelike-deckbuilder style: a starting deck, a seeded map of fights and tiles, HP that carries from fight to fight, one lost fight ending the run, packs growing the deck, bosses letting the player extract for future runs, and the score paying Embers.

Every fight is an ordinary practice game ([[§9.9]], [[R187]]) whose setup the run writes ([[§15.6]]). No card or game rule knows about a run, and sections two to eight hold inside each fight. A game without a run's setup hashes as before.

The rules are pure Rust in `crates/run`, and its `README.md` is the contract. Every draw goes through the engine's `Rng` from the run's seed and cursor ([[§10.7]]).

The content (`RunContent`: pool, starting cards, enemies, artifacts, blessings, events, curses) is passed in. Until the designer's lists land it is placeholders (`run_placeholder_content()`, `crates/run/src/placeholder.json`).

Every value is a constant of `crates/run/src/config.rs`. One the plan marks a strawman is to be tuned ([[§15.13]]).

| Constant | What it holds | Where |
| --- | --- | --- |
| `RUN_SEED_LENGTH` | The letters of a seed | [[§15.2]] |
| `RUN_ACTS` | The map's acts, each ending at its boss's row: a mini-boss's, then the final boss's for the last | [[§15.5]] |
| `RUN_ACT_ROWS` | The rows of nodes in each act before its boss's row; a strawman | [[§15.5]] |
| `RUN_MAP_COLUMNS` | The columns of the grid the map's rows lie on; a strawman | [[§15.5]] |
| `RUN_MAP_PATHS` | The seeded walks up each act, each from its own column of the act's first row; a strawman | [[§15.5]] |
| `RUN_ROUTE_FIGHTS_MIN` | The fewest fights on any route from a start to the final boss, the bosses counted | [[§15.5]] |
| `RUN_ROUTE_FIGHTS_MAX` | The most fights on any route from a start to the final boss, the bosses counted | [[§15.5]] |
| `RUN_ROUTE_TILES_MIN` | The fewest tiles on any route from a start to the final boss | [[§15.5]] |
| `RUN_ELITE_PERCENT` | The chance, in percent, that a fight node of the map is an elite | [[§15.5]] |
| `RUN_GHOST_PERCENT` | The chance, in percent, that a fight node that is no elite is a ghost | [[§15.5]] |
| `RUN_TILE_WEIGHTS` | The weights of a tile node's kind | [[§15.5]] |
| `RUN_STARTER_SIZE` | The cards of a starting deck, at every level | [[§15.3]] |
| `RUN_SLOT_LADDER` | The starting deck's slots at each level, from level 1, as counts of Common, Rare, Epic, Legendary, Mythic and Anything slots, each row holding `RUN_STARTER_SIZE`; a strawman | [[§15.3]] |
| `RUN_STARTER_CARDS` | The cards any starting deck may hold before anything is extracted; placeholders until the designer's list, the Training Grounds Commons and Rares with the fewest script lines, in catalog order, the first `RUN_STARTER_SIZE` fitting level 1's slots | [[§15.3]] |
| `RUN_PRESET_DECKS` | The named starting decks the start screen offers; placeholders | [[§15.3]] |
| `RUN_LEVEL_EMBERS` | The Embers each step of `RUN_SLOT_LADDER` costs from the level before it; a strawman | [[§15.3]], [[§15.10]] |
| `RUN_STARTER_ARTIFACTS_MAX` | The most artifacts a run starts with | [[§15.3]], [[§15.4]] |
| `RUN_STARTER_ARTIFACT_POINTS` | The most points of artifacts a run starts with | [[§15.3]], [[§15.4]] |
| `RUN_TIER_POINTS` | An artifact's points by its tier | [[§15.3]], [[§15.4]] |
| `RUN_MAX_HP` | The health a run starts with, before wins, campfires, events and passives raise it | [[§15.4]] |
| `RUN_ENEMY_HEALTH` | Each enemy's hero health by its kind of fight; a strawman | [[§15.6]] |
| `RUN_LOOMING_PRESENCE` | The enemy artifact that gives its carrier the Medium AI's resources in a fight | [[§15.6]] |
| `RUN_OVERWHELMING_PRESENCE` | The enemy artifact that gives its carrier the Hard AI's resources in a fight, outranking Looming Presence when an enemy carries both | [[§15.6]] |
| `RUN_ENEMY_DECK_SIZE` | An enemy's deck: its own cards first and the rest dealt from the pool | [[§15.6]] |
| `RUN_MINI_BOSS_DROP_TIER` | The lowest tier of a mini-boss's drop when its content names none | [[§15.6]] |
| `RUN_ARTIFACT_PERCENT` | The chance, in percent, that a won fight offers an artifact; a boss's drop is preseeded | [[§15.7]] |
| `RUN_COINS` | The Coins a won fight pays; a strawman | [[§15.7]] |
| `RUN_TICKETS` | The Tickets a won fight pays; a strawman | [[§15.7]] |
| `RUN_PACK_SIZE` | The cards an opened pack shows, before passives add more | [[§15.7]] |
| `RUN_KEEP` | The cards of an opened pack the player keeps, before passives add more | [[§15.7]] |
| `RUN_KEEP_MAX` | The most cards of a pack anyone keeps | [[§15.7]] |
| `RUN_DECK_MAX` | The most cards a run deck holds, the engine's `LIBRARY_CAP` ([[R2701000]]) | [[§15.7]] |
| `RUN_WIN_MAX_HP` | The max HP every won fight adds for the rest of the run, before refilling to the new max | [[§15.6]] |
| `RUN_CAMPFIRE_MAX_HP` | The max HP a campfire's rest adds for the rest of the run | [[§15.8]] |
| `RUN_CAMPFIRE_REMOVE` | The cards a campfire removes | [[§15.8]] |
| `RUN_SHOP_CARDS` | The cards a shop sells | [[§15.8]] |
| `RUN_SHOP_ARTIFACTS` | The artifacts a shop sells | [[§15.8]] |
| `RUN_CARD_PRICE` | A shop card's Coin price by its rarity; a strawman | [[§15.8]] |
| `RUN_ARTIFACT_PRICE` | A shop artifact's Coin price by its tier; a strawman | [[§15.8]] |
| `RUN_REMOVE_PRICE` | The shop's Coin price for removing a card | [[§15.8]] |
| `RUN_RADIANT_TICKETS` | The shop's Ticket price for making a card Radiant, which a campfire does free | [[§15.8]] |
| `RUN_BUFF_TICKETS` | The shop's Ticket price for buffing a card, which a campfire does free | [[§15.8]] |
| `RUN_LEVEL_TICKETS` | The Tickets an artifact's next level costs, times the level it is at | [[§15.4]], [[§15.8]] |
| `RUN_ARTIFACT_LEVEL_MAX` | An artifact's highest level | [[§15.4]], [[§15.8]] |
| `RUN_CHEST_COINS` | The Coins a chest holds | [[§15.8]] |
| `RUN_CHEST_TICKETS` | The Tickets a chest holds | [[§15.8]] |
| `RUN_CHEST_ARTIFACT_PERCENT` | The chance, in percent, that a chest also holds an artifact | [[§15.8]] |
| `RUN_SHRINE_OFFERS` | The blessings a shrine offers to choose from | [[§15.8]] |
| `RUN_SCORE_ROW` | The score for each row of the map reached; a strawman | [[§15.10]] |
| `RUN_SCORE_FIGHT` | The score for each fight won, by its kind; a strawman | [[§15.10]] |
| `RUN_SCORE_DAMAGE` | The score for each point of damage the fights' reports dealt; a strawman | [[§15.10]] |
| `RUN_SCORE_HEALING` | The score for each point of healing the fights' reports did; a strawman | [[§15.10]] |
| `RUN_SCORE_CARD_DRAWN` | The score for each card the fights' reports drew; a strawman | [[§15.10]] |
| `RUN_SCORE_MINI_BOSS_PERCENT` | Each mini-boss beaten multiplies the score by this percent | [[§15.10]] |
| `RUN_SCORE_FINAL_BOSS_PERCENT` | Beating the final boss multiplies the score by this percent | [[§15.10]] |
| `RUN_SCORE_CURSE_POINT_PERCENT` | Each curse point adds this percent to the score; a strawman | [[§15.9]], [[§15.10]] |
| `RUN_SCORE_PER_EMBER` | The score one Ember is worth; a strawman | [[§15.10]] |

### 15.1 Sandbox and Campaign

**Sandbox:** offline, no account. It plays in the browser's practice worker and never touches the server. It earns nothing on an account: unlocks, Embers, levels, history, stats and leaderboards stay on the device.

**Campaign:** signed in. The account holds the unlocks, Embers and levels. The server folds a finished run's file ([[§15.2]]) before it pays, as with practice claims ([[§14.8]], [[R1303000]]). Stats feed the leaderboards. The server acts at a run's start, at its end and for ghosts, never during.

The caller keeps `RunUnlocks` (the level, the extracted cards and artifacts, and in the Campaign the owned cards) and adds a finished run's extractions and Embers.

In the Campaign an extracted card enters a starting deck only if the account owns it ([[§14]]). A boss card is the exception.

The Campaign is not yet built ([[§15.12]]).

### 15.2 Seeds and the daily run

A seed is `RUN_SEED_LENGTH` capital letters A–Z; any other is refused.

The seed deals the map, the enemies and their decks, ghosts, events, shops, packs, every drop and each fight's game seed.

Runs on one seed that start alike and choose alike meet the same things. A different starting deck or different artifacts can change what packs, shops and drops deal, because none deals a card the deck holds or an artifact the run holds.

`run_seed(source)` makes a seed from any text (a random run's roll). Seeds can be shared and typed in.

Daily run: `run_daily_seed(date, n)`, so everyone's nth run of a day shares a seed.

`RunFile` is the setup, the starting unlocks and the action log. `run_fold` replays it and names the first refused step.

A run folds under the catalog version it started on: the device keeps the version with the run file, and the rest of the run plays on the content it was dealt from ([[R3496000]]). A run kept before versions were kept folds as today's ([[R3496000]]).

`run_reduce` rebuilds the `Rng` from the seed and cursor, as the engine's reduce does. A refused action changes nothing.

### 15.3 The starting deck

The deck is `RUN_STARTER_SIZE` distinct pool cards at every level: the content's starting cards (`RUN_STARTER_CARDS`, simple Training Grounds Commons and Rares, placeholders) and every card extracted since ([[§15.7]]).

Slots are Common, Rare, Epic, Legendary, Mythic and Anything. A slot takes its rarity ([[§5]]) or lower; Anything takes any card; a boss card ([[§15.6]]) fits only Anything.

The level, from 1, is a row of `RUN_SLOT_LADDER`: each slot's count, summing to `RUN_STARTER_SIZE`. A level past the ladder is refused. A deck is refused when more of its cards need some rank or better than the level has slots of that rank or better.

Embers buy the next level at `RUN_LEVEL_EMBERS`, up to the ladder's top.

Artifacts: at most `RUN_STARTER_ARTIFACTS_MAX`, distinct, worth at most `RUN_STARTER_ARTIFACT_POINTS` by `RUN_TIER_POINTS`. Each is Combat or Economic, not Unobtainable, and a starter or extracted ([[§15.4]]).

Curses: any of the content's ([[§15.9]]).

The start screen offers the named starting decks of `RUN_PRESET_DECKS` (placeholders until the designer's lists): each holds `RUN_STARTER_SIZE` distinct cards of `RUN_STARTER_CARDS` fitting level 1's slots, so any player may pick any of them from their first run, then tweak the picks in the builder.

`run_check_setup` gives the reason for a refusal. `run_create` also refuses content that lacks a normal, elite, mini-boss or final-boss enemy, or an event.

A run starts at max HP, with no Coins or Tickets, artifacts at level 1 and no card Radiant or buffed.

### 15.4 Artifacts and blessings

**Kinds:**

- Combat: works inside a fight: its fight effect changes the human's side of each fight's setup, once per level ([[§15.6]], [[R2922001]]).
- Economic: works on the map.
- Enemy: carried by enemies only (Looming Presence, Overwhelming Presence, a boss's own).
- Blessing: a shrine's, from a god.

A player's artifact has a tier, Bronze to Diamond, which sets its strength, its points (`RUN_TIER_POINTS`) and its price (`RUN_ARTIFACT_PRICE`).

Those points are what the starting deck's limits count ([[§15.3]]): at most `RUN_STARTER_ARTIFACTS_MAX` artifacts, worth at most `RUN_STARTER_ARTIFACT_POINTS`.

Artifacts start at level 1, and a shop raises them, at `RUN_LEVEL_TICKETS` times the level, up to `RUN_ARTIFACT_LEVEL_MAX` ([[§15.8]]). An artifact's map or fight effect counts once per level; a blessing's counts once.

**Effects:**

- keep more cards per pack, never past `RUN_KEEP_MAX`;
- keep none of a pack, for Coins;
- bigger packs;
- a percent off shop Coin prices, never below zero;
- Coins or Tickets per won fight;
- more max HP.

Max HP is `RUN_MAX_HP`, plus the run's own permanent changes (each won fight's `RUN_WIN_MAX_HP` ([[§15.6]], [[R3584000]]), each campfire's `RUN_CAMPFIRE_MAX_HP` ([[§15.8]]) and each event option's `max_hp` ([[§15.8]], [[R3584001]])), plus the max-HP effects. A rise heals by the rise, and HP never stays above the max.

**Fight effects** (a Combat artifact's): more cards in the opening hand, or Armor the hero starts each fight with ([[R2922000]]).

No run holds an artifact twice. Every roll (reward, chest, event, shop) deals a Combat or Economic artifact that is neither Unobtainable nor held, and a boss drop already held is replaced by another ([[§15.7]], [[R3286000]]).

Unobtainable: a final boss's own artifact. Nothing else deals it, and it can neither start a run nor be extracted.

A few artifacts start unlocked (the content's starters). Final bosses unlock the rest, one extraction at a time.

### 15.5 The map

The map is `RUN_ACTS` acts. Each act is `RUN_ACT_ROWS` rows of nodes, then its boss's row of one node: a mini-boss for each act but the last, the final boss for the last.

The rows lie on a grid `RUN_MAP_COLUMNS` wide. `RUN_MAP_PATHS` seeded walks go up each act, each from a column of its own on the act's first row. Each step goes to the column left, the same or right, never across an edge an earlier walk took. Every node a walk visits is kept with every edge it takes, so the paths split and merge ([[R3583000]]).

A boss stands on the middle column. Every node of its act's last row leads to it, and a mini-boss leads to every node of the next act's first row. Those fans are the only edges wider than a column, and every route passes both mini-bosses and the final boss.

Before the first step the player steps onto any node of the first row. After it, only a node of the next row that the node stood on leads to may be chosen; any other step is refused, the run unchanged ([[R3583000]]).

The first row's nodes are all fights, so a run opens on one. A fight node is an Elite at `RUN_ELITE_PERCENT`, else a Ghost at `RUN_GHOST_PERCENT`, else a Fight. A tile is a Shop, Campfire, Event, Chest or Shrine, by `RUN_TILE_WEIGHTS`. A row may mix fights and tiles.

Every route from a start to the final boss holds `RUN_ROUTE_FIGHTS_MIN` to `RUN_ROUTE_FIGHTS_MAX` fights, the bosses counted, and at least `RUN_ROUTE_TILES_MIN` tiles. The deal holds the bounds for every route, not on average ([[R3583001]]). A route with fewer fights is the fast way, one with more the slow way.

The whole map (its paths, enemies, decks, bosses' drops and events) is dealt at the start from the seed, and the whole of it is shown.

A run kept from before the map had paths is folded on the map its seed deals today. It ends lost at its first refused step and still banks what it had extracted ([[R3496000]]); one whose every step that map accepts plays on.

Between fights the top bar shows HP, Coins, Tickets, the deck's size, the relics held and the score, and a Your run panel lists the deck (Radiant and buffed cards marked), the relics and blessings held with their text, the curses taken with their names and text, and the seed, which can be copied.

### 15.6 The fights

**The human:** the run deck in order, with `HUMAN_HANDICAP`, its deck size the deck's length and its hero health the run's HP, always the max ([[R180]], [[R290]], [[R3584000]]). At setup, before the opening draw, each Radiant card is dealt Radiant ([[§5.2]]) and each buff is applied as one Buff ([[§6.3]]). A fight's game is built under the current catalog, as a fight started today is: a card the run holds that the catalog no longer knows is left out of the fight's deck, and stays in the run ([[R3496000]]).

**The enemy:** its deck, with Easy's handicap (`AI_DIFFICULTY`), Medium's when it carries Looming Presence and Hard's with Overwhelming Presence; elites and bosses carry them. Its deck size is its deck's length and its hero health is `RUN_ENEMY_HEALTH` by kind.

The game seed is the run's seed, row and column. A fight left unfinished waits where it stood: when the run, still on its fight, starts it again, the kept fight is resumed on its turn ([[R3476000]]). Leaving still asks first ([[R2706000]], [[R2972000]]).

Enemy decks are random for now: the enemy's own cards, then the open pool up to `RUN_ENEMY_DECK_SIZE`.

Boss cards are the single-player sets' cards ([[R1846001]]). Only a boss's deck and its pack deal them.

**Ghosts as built:** a random open-pool deck, no artifacts, the ghost health. **The second stage:** decks of real players who died near that point, never boss cards, from friends' runs, some pre-seeded, from snapshots.

A mini-boss always drops its content's artifact, or else one of `RUN_MINI_BOSS_DROP_TIER` or better. The final boss drops its own Unobtainable artifact.

**The report:** won or not, the human's final health, damage dealt to the enemy's hero and units, healing on the human's side, cards drawn, turns.

A win of any kind adds `RUN_WIN_MAX_HP` to the max for the rest of the run, then refills HP to the new max, whatever the hero's final health was ([[R3584000]]). Any other end drops HP to zero and ends the run.

**Curses and Combat artifacts** change the two handicaps after Presence ([[R2922001]]). Each curse taken changes the enemy's once: more hero health, more opening cards, or more crystals every turn, bonus and cap together ([[R181]]). Each Combat artifact held changes the human's once per level: more opening cards ([[R182]]) or starting Armor ([[R2922000]]). A negative amount counts as none, and an enemy's own artifacts other than Presence change nothing. The placeholders' numbers are strawmen ([[§15.13]]).

### 15.7 After a fight

A won fight pays in this order.

**First, health:** `RUN_WIN_MAX_HP` more max HP for the rest of the run, then HP refilled to the new max ([[R3584000]]).

**Second, currencies:** `RUN_COINS` and `RUN_TICKETS` by kind, plus the passives'.

**Third, an artifact:** a boss's drop; a drop already held, or none, is replaced by a roll at `RUN_ARTIFACT_PERCENT`, a mini-boss's replacement of `RUN_MINI_BOSS_DROP_TIER` or better ([[R3286000]]). There is none when no artifact is left to roll. It is taken or left before any pack.

**Fourth, four packs:** the beaten enemy's deck with its boss cards, a random one, a set's, and one of a card type, tag or rarity (a random pack when the pool lacks one).

- The player opens one. It shows up to `RUN_PACK_SIZE` cards plus the passives', never a card the deck holds.
- The player keeps exactly `RUN_KEEP` plus the passives', at most `RUN_KEEP_MAX`, at most what it shows and at most the room left in the deck. With a Skip passive they may keep none, for its Coins.
- The run deck holds at most `RUN_DECK_MAX` cards, the engine's library cap, so every fight starts ([[R2701000]]). A card a full deck would gain, from a pack, a shop or an event, is not gained.

**Last, extraction:** after a mini-boss, one card of the deck or none, never the last card. After the final boss, one held artifact or none, never an Unobtainable one, and the run is won.

What is extracted leaves the run at once and is unlocked for future starting decks. A run that cannot be folded at all ends lost and still banks what it had extracted of the cards and artifacts the current content knows ([[R3496000]]).

### 15.8 The tiles

**Shop:** stock rolls on entry: `RUN_SHOP_CARDS` open-pool cards the deck lacks, at `RUN_CARD_PRICE` by rarity, and up to `RUN_SHOP_ARTIFACTS` artifacts at `RUN_ARTIFACT_PRICE`. Until the player leaves, they may:

- buy each item once, a card only while the deck has room ([[R2701000]]);
- remove a card for `RUN_REMOVE_PRICE` Coins, never the last;
- make a card Radiant for `RUN_RADIANT_TICKETS`;
- buff a card for `RUN_BUFF_TICKETS`;
- level an artifact for `RUN_LEVEL_TICKETS` times its level, up to `RUN_ARTIFACT_LEVEL_MAX`.

Discounts cut Coin prices only, and a price the run cannot pay is refused.

**Campfire:** one free choice:

- rest: `RUN_CAMPFIRE_MAX_HP` more max HP for the rest of the run;
- make a card that is not Radiant Radiant;
- buff a card;
- or remove `RUN_CAMPFIRE_REMOVE` different cards, leaving at least one.

**Event** (placeholder stories): an option may change max HP ([[R3584001]]), Coins and Tickets, add a random open-pool card the deck lacks (none to a full deck, [[R2701000]]), or give an artifact. It is refused if max HP would fall below 1 or it costs more than the run holds. HP follows the max.

**Chest,** on entry: `RUN_CHEST_COINS`, `RUN_CHEST_TICKETS`, and an artifact at `RUN_CHEST_ARTIFACT_PERCENT`.

**Shrine** (a monastery or a statue): up to `RUN_SHRINE_OFFERS` blessings not held; the player takes one or none, for the rest of the run.

Abandoning at any time ends a run, lost, still scored and paid Embers under [[§15.10]]. A run whose version this build no longer carries, or whose log folds under no carried version, ends lost with no row, no score and no Embers because there is nothing to fold to score it ([[R3496000]]). The start screen asks first, and the map links back to the Play menu ([[R2972000]]). An ended run refuses every action.

### 15.9 Curses

Curses are tiers chosen at the start, like Ascension. Each is worth its points, like Hades' Heat, and stacking them is the challenge.

Each point adds `RUN_SCORE_CURSE_POINT_PERCENT` to the score.

Each also makes every fight's enemy stronger ([[§15.6]], [[R2922001]]).

### 15.10 Score, Embers and leaderboards

**Currencies,** by plain id, named through translations: `coins` and `tickets` inside a run; `embers` on the account (on the device in the Sandbox); Jlore unchanged ([[§14.1]]).

**Score,** in whole numbers rounded down:

- `RUN_SCORE_ROW` per row reached;
- `RUN_SCORE_FIGHT` per win, by kind;
- `RUN_SCORE_DAMAGE`, `RUN_SCORE_HEALING` and `RUN_SCORE_CARD_DRAWN` per point of every fight's report, a lost fight's included;
- then `RUN_SCORE_MINI_BOSS_PERCENT` percent per mini-boss beaten, then `RUN_SCORE_FINAL_BOSS_PERCENT` percent for the final boss, then the curse points' percent (`RUN_SCORE_CURSE_POINT_PERCENT`).

Time and turns are shown beside the score, not in it. A run's time is the time it was played: it counts only while the run is open and shown, so a run left and picked up later is not timed by the calendar ([[R3502000]]).

**Embers** are earned only by playing: `run_embers`, the score divided by `RUN_SCORE_PER_EMBER`, paid when a run ends. They buy levels ([[§15.3]]) at `RUN_LEVEL_EMBERS`. Spending them on upgrading artifacts or buying buffs is not built.

**Leaderboards:** deepest row, most curse points, top score and fastest win, with an on-screen timer. The Sandbox's stay on the device; the Campaign's hold checked runs.

The run-over screen shows the run's seed, and a past run's details name its curses from the run's content; an id the content no longer holds shows as itself.

**Stats:** `RunStats`'s fields, in words: nodes visited; fights, elites, ghosts, mini-bosses and bosses won; damage dealt, healing done, cards drawn and turns; damage taken and healing taken; Coins and Tickets earned and spent; cards kept, bought, removed, made Radiant and buffed; packs skipped; artifacts taken and leveled; blessings taken; events seen; chests opened.

### 15.11 The extended campaign

Not yet built: it waits for the second stage.

Past the final boss, the run goes on into endless worlds.

Every enemy carries a prestige artifact, so each world is harder.

Artifacts and curses are kept, and HP resets.

The deck goes back to `RUN_STARTER_SIZE` cards, half from the run's deck and half from the starting deck.

Today, beating the final boss ends the run, won.

### 15.12 What is built

**The first stage, the Sandbox:** `crates/run`'s core; fights that read the run (HP, Presence, Radiant and buffed cards); the Sandbox screens and device store; Economic artifacts and blessings; Combat artifacts and curses inside fights; real cards, with placeholder enemies, artifacts, blessings, events and curses.

**The second stage:** the Campaign, ghosts from real players, the extended campaign.

The designer's content replaces the placeholders.

**Measuring it:** `cargo jackioh run-sim` plays seeded whole runs with the AI in the player's seat at [[R747000]]'s skill, from level 1 with no artifact or curse unless `--level`, `--artifacts` and `--curses` set them (each refused when the content lacks it), and prints each kind of fight's and each act's win rate, HP cost and turns, the max HP at each boss, the rows reached, the fights on each run's route, the score by term (rows, wins, damage, healing, cards drawn, and what the boss percents and the curse percent add) and the Embers (#3065, #3481): a guide to tuning the strawmen, not a target.

### 15.13 Open questions

These are the plan's DR13 open questions, not decided here:

- copies (as built, one of each card, and nothing deals a card the deck holds);
- the Buff's size and whether it repeats (as built, each buff is one Buff and a card can take more);
- the starting cards and starter artifacts;
- the prestige artifacts;
- ghost snapshots;
- tuning the strawmen, against [[§15.12]]'s `cargo jackioh run-sim`.
