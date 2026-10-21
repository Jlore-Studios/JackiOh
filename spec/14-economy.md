## 14. Economy

How an account comes to own cards and cosmetics: Jlore, the cosmetics there are to own, card packs and their slot roll, pity, the token bars and the cosmetic token cycle, cosmetic packs and the Chase Shard, set tracks and XP, what play earns, daily quests, dev accounts, the ledger every grant is written to, and going live. Every gameplay card is earned by playing, and generously; the endgame is a collection of cosmetics. The model is the designer's, tracked in #1294; each of its sub-issues builds a part and writes the rulings it needs ([[§11]]), #1294's numbers go into `crates/server/src/config.rs` (exported to `apps/web/src/wire/serverConfig.ts`) together, before the parts that use them; a constant whose value #1294 does not give goes in with the sub-issue that first needs it, and #1333 brought every part built before the owner's numbers in line with them ([[R1333000]], [[R1333001]]); `DEV_PROFILE_IDS`, an environment variable, is read with the rest of the server's environment.

Every value here is a constant named in backticks, never a number (CLAUDE.md rule 9). The server owns every balance, roll, counter, bar and grant ([[§9.1]]): the client sends intent, such as buying or opening a pack, redeeming a token or picking a chase, and renders the server's projection of its own account, deciding nothing (CLAUDE.md rule 7). An account reads its own wallet, inventory, pity counters, bars, tracks, quests, cosmetics and ledger rows, and nobody else's: what a pack held, a counter or a balance reaches no other player. Only non-token cards of the sets that ship are collected: no pack, reward or grant holds a token or a card of a set that has not shipped ([[§9.4]] L3, [[R1420]]). Real money is out of scope: nothing here is sold, and nothing is bought but with Jlore. Art cosmetics and showing an opponent's cosmetics are out of scope too: this section stops at which cosmetics an account owns. A ladder bot owns no collection ([[R1454]]) and earns nothing. The economy went live with #1310 ([[§14.12]]): a new account starts with the starter cards and earns the rest ([[R1310000]]), every account active before then kept every card it owned ([[R1310002]]), and a practice game records nothing but what [[R1303000]] pays ([[R187]], [[§14.8]]).

| Constant | What it holds | Where |
| --- | --- | --- |
| `PACK_PRICE_JLORE` | The Jlore a card pack costs | [[§14.3]] |
| `PACK_SLOTS` | The slots of a card pack | [[§14.3]] |
| `PACK_UNOWNED_WEIGHTS` | How a slot's card draw leans toward the cards the account does not own | [[§14.3]] |
| `TIER_WEIGHT_TOTAL` | The fixed total every tier weight table is out of | [[§14.3]] |
| `CARD_TIER_WEIGHTS` | A card pack slot's weight for each card rarity above Common, the fallback | [[§14.3]] |
| `COSMETIC_TIER_WEIGHTS` | A duplicate's and a cosmetic pack slot's weight for each cosmetic rarity above Common, the fallback | [[§14.3]] |
| `PACK_FLOORED_SLOT`, `PACK_FLOOR_RARITY`, `COSMETIC_PACK_FLOOR_RARITY` | The slot of each pack that is floored, and the floor of a card pack's and of a cosmetic pack's | [[§14.3]] |
| `PITY_EPIC_PACKS`, `PITY_LEGENDARY_PACKS`, `PITY_MYTHIC_PACKS` | The run of card packs in a row within which that rarity always drops | [[§14.4]] |
| `CARD_TOKEN_BAR_SIZE`, `COSMETIC_TOKEN_BAR_SIZE` | The points that fill each token bar | [[§14.5]] |
| `CARD_TOKEN_BAR_POINTS_PER_DUPLICATE`, `COSMETIC_TOKEN_BAR_POINTS_PER_DUPLICATE` | The points a spent duplicate adds to each bar | [[§14.5]] |
| `CARD_TOKEN_BAR_TARGET_PACKS`, `COSMETIC_TOKEN_BAR_TARGET_PACKS` | The card packs each bar takes to fill, on average | [[§14.5]] |
| `COSMETIC_TOKEN_CYCLE` | The steps the cosmetic token bar pays, in order | [[§14.5]] |
| `COSMETIC_PACK_SLOTS` | A cosmetic pack's slots | [[§14.6]] |
| `CHASE_SHARD_PACKS` | The cosmetic packs that fill the Chase Shard bar | [[§14.6]] |
| `SET_TRACK_LEVELS` | The levels of a set track | [[§14.7]] |
| `SET_TRACK_XP_PER_LEVEL` | The XP step of the level curve | [[§14.7]] |
| `SET_TRACK_UNLOCK_LEVEL` | The newest track's level that unlocks the older ones | [[§14.7]] |
| `JLORE_LADDER_WIN`, `JLORE_LADDER_LOSS` | The Jlore a ladder game pays | [[§14.8]] |
| `JLORE_PRACTICE_WIN`, `JLORE_PRACTICE_LOSS` | The Jlore a practice game pays | [[§14.8]] |
| `JLORE_GAMES_DAILY_CAP` | The most Jlore an account's games pay it in a UTC day | [[§14.8]] |
| `XP_PER_ACTION`, `XP_WIN_BONUS`, `XP_GAME_CAP` | A game's XP: per action, for a win, and its most | [[§14.8]] |
| `XP_PRACTICE_SCALE` | A practice game's share of the XP the same game earns on the ladder | [[§14.8]] |
| `XP_LADDER_TARGET_MEAN` | The XP a ladder game averages | [[§14.8]] |
| `PRACTICE_SEED_TTL_SECONDS` | How long a practice seed waits to be claimed | [[§14.8]] |
| `PRACTICE_SEEDS_OPEN_MAX` | How many unclaimed practice seeds an account holds at once | [[§14.8]] |
| `PRACTICE_VERIFY_CONCURRENCY` | How many practice claims the server checks at once | [[§14.8]] |
| `QUEST_INTERVAL_HOURS`, `QUEST_MAX_HELD` | How often a quest arrives, and how many are held | [[§14.9]] |
| `QUEST_JLORE_MIN`, `QUEST_JLORE_MAX`, `QUEST_XP_PER_JLORE` | A quest's Jlore, and its XP per Jlore | [[§14.9]] |
| `DEV_PROFILE_IDS` | The environment variable that lists the dev accounts | [[§14.10]] |
| `ECONOMY_LIVE` | The server's switch for the economy, on since going live | [[§14.12]] |
| `STARTER_CARDS` | The cards a new account starts with | [[§14.12]] |

### 14.1 Jlore

Jlore is the soft currency: a whole-number balance in each account's wallet, never below zero. It is earned only by playing: games ([[§14.8]]), daily quests ([[§14.9]]) and set track levels ([[§14.7]]), besides what a dev account grants itself ([[§14.10]]). It is never sold, and no money becomes Jlore. It buys card packs ([[§14.3]]). A spend the wallet cannot cover is refused and changes nothing.

### 14.2 Cosmetics

A cosmetic is an entry of the cosmetics registry, one data file the server and the client both read: its id, its kind, its name, its rarity, on the cosmetics' own scale from Common to Mythic, whether it is its kind's default and whether it is a chase. A per-card cosmetic (a foil, a finish, a border, a full-art face) is owned for one card; an account cosmetic (a board, a field, a card back) is owned for the whole account. Apart from the starter cosmetics, to own a cosmetic is to hold it in the ledger ([[§14.11]]). A player's cosmetics show on their own side only: showing an opponent's needs the frame that would carry them, which is out of scope here, and the client never infers what an opponent owns.

- **Two scales.** A card's rarity is the catalog's ([[§5]]). A cosmetic's rarity is a scale of its own, Common, Rare, Epic, Legendary or Mythic, never tied to the card it goes on: a duplicate's cosmetic ([[§14.3]]), a token ([[§14.5]]) and a cosmetic pack's slot ([[§14.6]]) each read the cosmetic's own rarity. The two are kept as separate types, so no code can take one for the other ([[R1333000]]).
- **Starter cosmetics.** Each kind's default and the starter items of the card and deck slots are owned by every person's account, on every card, before and after going live.
- **Chase cosmetics** are the Legendary and Mythic ones, the chase tier, and the registry's chase mark agrees with the rarity: it is set exactly on them. No token pays for one ([[§14.5]]). One drops when a duplicate's cosmetic roll ([[§14.3]]) or a cosmetic pack's slot lands on its tier, and a full Chase Shard bar buys one ([[§14.6]]). The only Legendary cosmetics today are starters, so no chase can be earned yet ([[R1333000]]).
- **Full-art faces.** The full art of the hero-portrait cards ([[R1017001]]), six today, is a normal per-card cosmetic and **Epic**, whatever its card's rarity ([[R1313000]]), not a chase: it drops from packs like any other Epic cosmetic, and an Epic cosmetic token buys it while a Rare token does not ([[§14.5]]). Since the economy went live, a player's own copy prints its full art when they own it and their Full-art cards setting is on; every other viewer of it, an opponent, a watcher or a replay's viewer, sees its normal face until another player's ownership can be shown. Nobody owned one when the economy went live ([[§14.12]], [[R1310002]]). The landing page's fan and the Almanac keep showing full art, and practice, hotseat and every end-to-end test but the one that walks the economy treat every cosmetic as owned ([[R1310004]]).

### 14.3 Card packs and the slot roll

There is a card pack for each set that ships ([[R1420]]) and a global card pack, whose pool is every set that ships. Each costs `PACK_PRICE_JLORE` and holds `PACK_SLOTS` slots. Buying one debits the wallet and adds the pack, unopened, to the account's inventory, in one transaction. Packs also come from set track rewards ([[§14.7]]). Opening a pack rolls every slot and grants what each holds, its pity counters and bars included, in one transaction or not at all.

Every drop of the economy, a card pack slot's, a duplicate's cosmetic and a cosmetic pack slot's ([[§14.6]]), is a **tiered roll** of two draws ([[R1339000]]). The first is the tier: one integer draw below `TIER_WEIGHT_TOTAL` against a table that sets a weight for each tier above the lowest. The lowest tier is the fallback and takes whatever weight is left, `TIER_WEIGHT_TOTAL` less the rest, computed and never stored, so raising one tier takes from the fallback alone. A tier's chance is its weight over `TIER_WEIGHT_TOTAL`: the table is the published odds. A floor is a fallback too: a slot that must be at least some tier rolls the same table, and any result below the floor becomes the floor tier, the tiers above it keeping their own odds. The second draw picks the item within the tier, as the roll's rules say. Every roll goes through one pure function, from the set tiers' weights, the fallback, an optional floor and a draw to a tier, which refuses set weights summing past `TIER_WEIGHT_TOTAL`; each roll's draws are seeded and logged with its opening ([[§14.11]], [[R1339002]]).

Each slot of a card pack rolls a rarity, one of [[§5]]'s from Common to Mythic, on `CARD_TIER_WEIGHTS`, Common its fallback. One slot, `PACK_FLOORED_SLOT`, the Rare-or-better slot, is floored at `PACK_FLOOR_RARITY`, so every pack holds a Rare or better. Pity then forces a rarity on top of the rolled slots when a counter runs out ([[§14.4]]). The slot then draws a card of its rarity from the pack's pool, the non-token cards of its set or, for the global pack, of every set that ships, weighted toward the cards the account does not own by `PACK_UNOWNED_WEIGHTS`. A rarity the pool lacks falls to the highest rarity below it that the pool holds, down to the slot's fallback (the floor, for the floored slot), then to the nearest one above, and below the floor last; the slot keeps the rarity it rolled ([[R1301001]]). What the slot grants follows the dupe chain:

- **A new card**, one the account owns fewer than `MAX_COPIES` of, unlocks: the account is granted a copy.
- **A duplicate**, a card the account already owns `MAX_COPIES` of, the most a deck may hold and never more than the account owns ([[§9.4]] L3, L5), makes its own tiered roll on `COSMETIC_TIER_WEIGHTS` for a cosmetic rarity, on the cosmetics' own scale whatever the card's rarity ([[§14.2]], [[R1301002]]), and becomes a per-card cosmetic of that rarity for that card, one the account does not own on it; it falls to no other rarity. A Legendary or Mythic one is a chase ([[R1333000]]).
- **A spent duplicate**, a duplicate whose card has no cosmetic of the rarity it rolled left for the account to own, adds points to both token bars ([[§14.5]]).

No cosmetic roll, a duplicate's or a cosmetic pack's ([[§14.6]]), touches the pity counters: they read the slots' card rarities alone ([[§14.4]]).

### 14.4 Pity

Pity keeps a counter for each of Epic, Legendary and Mythic; whether an account keeps one set of counters for all its card packs or one per kind of pack is the pity math's sub-issue's to rule ([[R1298000]]). A counter counts card packs, not slots: a pack with a slot that rolls the counter's rarity resets it, whatever that slot then grants, and a pack with none adds to it. A counter resets only when its own rarity drops: a Mythic leaves the Epic and Legendary counters running, and a Legendary the Epic one. The counters are the account's to see, with each rarity's odds: the odds shown before opening, on the pack screen, the cosmetic packs panel and in every odds `GET`, are the tables of [[§14.3]] and [[§14.6]] as percentages the server computes, the fallback included, never a rate the client computes, and the pack screen adds a line that pity guarantees each of the three within its threshold ([[R1339000]]).

A rarity's threshold (`PITY_EPIC_PACKS`, `PITY_LEGENDARY_PACKS`, `PITY_MYTHIC_PACKS`) is the longest an account waits for it: of any `PITY_EPIC_PACKS` card packs in a row that one Epic counter counts, at least one holds an Epic, the last of them when none before it did, and likewise for Legendary and Mythic. No counter ever passes its threshold, and `PITY_MYTHIC_PACKS` is the longest wait of all.

Pity sits on top of the tiered roll and changes no table ([[R1339001]]): the table is exactly what one roll does, and a counter only forces a rarity when it runs out. A pack in which a counter would reach its threshold, and none of whose slots rolled that rarity, forces one slot to it, Mythic first, then Legendary, then Epic, each into the first slot not yet forced, the floored slot first, that is below it and whose loss leaves the pack every Epic, Legendary and Mythic it rolled, or else into one whose rarity another slot also holds. Before pity, a card pack holds a rarity of weight w in 1 − (1 − w/`TIER_WEIGHT_TOTAL`)^`PACK_SLOTS` of packs, a chance per pack the floored slot leaves alone, since it lifts only Commons; with pity, a little more. A seeded simulation over many packs reports both rates, checks that each rate with pity is at least the one before it and that no counter passes its threshold, and nothing is tuned to it. Pity is pure, from the counters and the slots' rolled rarities to the slots after it and the next counters, and takes no draw, so the pack roll calls it on its own seeded, logged draws ([[§14.11]]).

### 14.5 The token bars and the cosmetic token cycle

Spent duplicates ([[§14.3]]) fill two bars: each adds `CARD_TOKEN_BAR_POINTS_PER_DUPLICATE` to the card token bar and `COSMETIC_TOKEN_BAR_POINTS_PER_DUPLICATE` to the cosmetic token bar. A bar that reaches its size (`CARD_TOKEN_BAR_SIZE`, `COSMETIC_TOKEN_BAR_SIZE`) pays and keeps the overflow, in the opening's own transaction. The points per duplicate are set so that, on a collection that is mostly owned, each bar fills once every `CARD_TOKEN_BAR_TARGET_PACKS` or `COSMETIC_TOKEN_BAR_TARGET_PACKS` card packs on average, by the seeded simulation of [[§14.4]], which runs on the tables of [[§14.3]] and states the collection it assumes.

- **The card token bar** pays a card token. A card token unlocks any new card ([[§14.3]]) of a set that ships that the account chooses.
- **The cosmetic token bar** pays the next step of `COSMETIC_TOKEN_CYCLE`: Rare, Epic, Rare, Epic, Rare, Cosmetic Pack, and then the same again. The account keeps its place in the cycle. A Rare or Epic step is a cosmetic token of that tier, which unlocks any cosmetic at its tier or below on the cosmetics' own scale ([[§14.2]]) that the account chooses and does not own: a Rare token a Common or Rare one, an Epic token a Common, Rare or Epic one, a full art included. It buys a per-card cosmetic for a card the account owns, or an account cosmetic. No token reaches Legendary or Mythic, so none pays for a chase ([[R1305003]]). A Cosmetic Pack step adds an unopened cosmetic pack to the inventory ([[§14.6]]).

Redeeming a token is one transaction. A choice the token cannot pay for, a card or cosmetic already owned, a cosmetic above the token's tier or a chase, is refused and spends nothing.

### 14.6 Cosmetic packs and the Chase Shard

A cosmetic pack is earned only, for now: it is the last step of the cosmetic token cycle ([[§14.5]]), and nothing sells one. Opening one rolls `COSMETIC_PACK_SLOTS` slots of cosmetics, not cards, through the same seeded, logged roll as a card pack ([[§14.11]]). Each slot's cosmetic rarity is a tiered roll ([[§14.3]]) on `COSMETIC_TIER_WEIGHTS`, a duplicate's table, Common its fallback, and one slot, `PACK_FLOORED_SLOT`, is floored at `COSMETIC_PACK_FLOOR_RARITY`, so every pack holds an Epic or better ([[R1339000]]). There is no pity on cosmetic rarity, and no cosmetic roll moves a card pack's counters ([[R1306000]]).

A slot grants one cosmetic of its rarity that the account does not own, for a card or for the account. A slot whose cosmetic is for a card the account does not own, a new card ([[§14.3]]), also grants a copy of the card. A rolled tier with nothing unowned falls to the highest tier below it that has something, down to the roll's fallback tier, which for the floored slot is its floor, then to the nearest tier above, and the floored slot below its floor last, so it stays Epic or better while anything there is unowned. No Legendary or Mythic cosmetic can drop yet, so those rolls fall down today. If nothing at all is unowned, the slot gives bar points, as a spent duplicate does ([[§14.5]], [[R1306001]]).

The Chase Shard bar counts the cosmetic packs the account opens. At `CHASE_SHARD_PACKS` it is full: it buys one chase cosmetic, Legendary or Mythic, that the account chooses and does not own, and empties. A full bar holds, counting no further, until it buys, so with no chase left to buy it waits ([[R1306002]]). Chases also drop by luck: a duplicate's cosmetic roll ([[§14.3]]) or a cosmetic pack slot that lands on Legendary or Mythic gives one ([[R1333000]]).

### 14.7 Set tracks and XP

Each set that ships has a track ([[R1420]]) of `SET_TRACK_LEVELS` levels. A track starts at its first level with no XP, and going from level n to the next costs `SET_TRACK_XP_PER_LEVEL` × n XP: the cost rises with every level and is never capped. The XP of games ([[§14.8]]) and quests ([[§14.9]]) goes to every active track, each gaining all of it; a dev account's own grant goes to the track it picks ([[§14.10]]). The newest set's track is active from the start, and an older set's once it unlocks: the older tracks unlock one at a time once the newest set's track reaches `SET_TRACK_UNLOCK_LEVEL`, nearest-older first, one per payout ([[R1302003]]). A track's last level is the last it reaches; XP past it still lands ([[R1302005]]).

Each level pays rewards, cards, Jlore and card packs, from a reward table per set kept as data. A level pays once, as ledger rows, when its track reaches it ([[§14.11]]): a track's first level when its first XP lands, or, for a new account's newest track, when the account becomes active ([[R1310000]]). A track does not pay out every card of its set; pack luck covers the rest ([[§14.3]]).

### 14.8 Earning

Games earn Jlore and XP for the people who play them, never for a ladder bot. A ladder game, a ranked match ([[§9.12]]) that is not a game of a Conquest series, pays its winner `JLORE_LADDER_WIN` and its loser `JLORE_LADDER_LOSS`. A practice game ([[§9.9]]) pays `JLORE_PRACTICE_WIN` for a win and `JLORE_PRACTICE_LOSS` for a loss. The Jlore an account's games pay it, ladder and practice together, stops once the UTC day's `game` Jlore reaches `JLORE_GAMES_DAILY_CAP`, as Hearthstone's games pay gold up to a daily limit: the game that crosses the cap is paid the part that fits, the ones after it no Jlore and no ledger row, and the next UTC day pays again. A capped game still pays its XP and moves quests, and the Jlore a quest or a track level pays neither counts toward the cap nor stops at it; a game's answer says when the cap held its Jlore back ([[R1483000]]). Whether a game of a ranked Conquest series, an unranked game or a co-op game ([[§13]]) earns, the sub-issues that build earning rule.

XP stands in for time played: `XP_PER_ACTION` for each action the player took, a move of the game and never a row the server writes itself ([[R1302004]]), and `XP_WIN_BONUS` more for a win, the game's total never above `XP_GAME_CAP`, so a loop of empty actions cannot farm it. A practice game earns the XP the same game would earn on the ladder, times `XP_PRACTICE_SCALE`, rounded down ([[R1302007]]). The amounts are tuned so a ladder game averages `XP_LADDER_TARGET_MEAN` over [[R1333001]]'s measured games, and a practice game `XP_PRACTICE_SCALE` of it. What a draw, a concede or a game the reaper ends pays, [[R1302000]], [[R1302001]] and [[R1302002]] rule.

Every payout goes through one reward hook on the server, which takes a finished game's summary (the account, the mode, the result and the actions taken), pays it, and moves the account's quests ([[§14.9]]).

- **On the ladder** the server ran the match, so it counts the actions from its own log and pays in the transaction that records the result; the client reports nothing.
- **In practice** the game runs in the browser ([[§9.9]]), so the server takes nothing on the client's word. It issues the game's seed and remembers it with the account, the AI's difficulty and the deck. At the end the client sends the action log; the server folds it ([[§9.3]]) to the claimed result and checks that every move of the AI's seat is the one the AI makes from that seed and difficulty. Only a game that passes pays; a seed pays once and expires unclaimed after `PRACTICE_SEED_TTL_SECONDS`. Only an active account's free game on a server seed pays, never a tutorial lesson ([[§9.10]]), a sandbox game ([[§9.9]]) or a game with custom cards ([[§12]]) ([[R1303000]]); a claim burns its seed, and a game resumed after a reload claims on its own ([[R1303001]]); the AI's moves are re-run at the full budget with no clock ([[R1303002]]); and a paying game brings the last board the server kept, not the device's ([[R1303003]]). The server checks at most `PRACTICE_VERIFY_CONCURRENCY` claims at once, and a claim past that waits its turn, never dropped. An account holds at most `PRACTICE_SEEDS_OPEN_MAX` unclaimed, unexpired seeds, a request for another is refused and issues nothing, so one account cannot stockpile seeds to claim in a burst ([[R1483000]]).

### 14.9 Daily quests

A quest is a goal read from a finished game's public events, such as winning games, playing spells, dealing damage with Units or playing cards of a set, and a payout of Jlore from `QUEST_JLORE_MIN` to `QUEST_JLORE_MAX`, more for a harder goal, with `QUEST_XP_PER_JLORE` XP for each Jlore of it. Quests are drawn from a pool kept as data. An account holds at most `QUEST_MAX_HELD`; while it holds fewer, a new quest arrives `QUEST_INTERVAL_HOURS` after the last one did, by the server's clock, so a player away for a long while comes back to `QUEST_MAX_HELD` waiting. Every finished game the reward hook sees ([[§14.8]]) moves the held quests' progress, and a quest that completes pays its Jlore and XP once, as ledger rows; the XP goes to every active track ([[§14.7]]). Its rhythm and top-up are [[R1304000]]'s, a game ending as a quest arrives is [[R1304001]]'s, and none arrives while `ECONOMY_LIVE` is off ([[R1304002]]).

### 14.10 Dev accounts

A dev account is one whose profile id the server's `DEV_PROFILE_IDS` environment variable lists, read at boot with the rest of the environment: a malformed id fails the boot, naming it, and none set means no dev accounts. The server decides: it tells the client whether the caller is a dev account, and every dev route checks the list itself and refuses anyone else as forbidden. The client only chooses whether to show the Dev tools panel. Any signed-in account can reveal its own profile id on the account page, hidden until asked for, so a new developer can be listed. In end-to-end mode the fixture account `e2e-econ` is a dev account too, for the test that walks the economy ([[R1310004]]).

A dev account can, for itself alone:

- give XP to a set track it picks, through the level curve ([[§14.7]]), and give Jlore;
- fill its collection with every card and every cosmetic;
- clear its cards, its cosmetics, its Jlore, its bars and pity counters, or its track progress, each apart, or all of them;
- be given a quest at once, or complete one ([[§14.9]]);
- fill its card token bar or its cosmetic token bar, so it pays at once ([[§14.5]], [[R1310003]]).

Every dev grant and clear is written as ledger rows marked `dev` ([[R1300000]]), so a dev grant can always be told apart ([[§14.11]]). No new catalog version grants a dev account its cards, nor anyone ([[R1310001]]), so a clear stays cleared; before the economy went live, [[R481]]'s grant of each new catalog version passed over a dev account for that reason ([[R1300001]]). Otherwise a dev account counts as any other does: its games go into the card statistics ([[§9.11]]) and the ladder ([[§9.12]]).

### 14.11 The ledger

The ledger today is `collection` plus the append-only `collection_grants` (`crates/server/migrations/0002_collection.sql`): every change writes both in one transaction, and no client path writes either ([[§9.4]]). Every holding of the economy follows that pattern, a current-state table beside an append-only one, readable by its own account alone. Every grant of anything (cards, Jlore, packs, tokens, cosmetics, XP) is one append-only row, written server-side in the transaction that changes the state; so is every spend, such as a pack bought or a token redeemed. A row is never edited, and is deleted only with its account ("Delete my account", [[§9.4]]): a mistaken grant is corrected by an offsetting row.

A game's Jlore row has the reason `game`, and an account's `game` rows over a UTC day are what `JLORE_GAMES_DAILY_CAP` counts, read in the transaction that writes the next one, under the account's economy lock ([[R1301003]], [[R1483000]]); no row is written for a game the cap leaves nothing for.

Every row says why. The collection's reasons are `pack`, `craft`, `reward`, `refund`, `admin`, `launch` and `dev`, a dev tool's own ([[R1300000]]); every economy ledger takes `launch` too, for the reset of going live ([[R1310002]]); a new kind of grant gets its reason from the sub-issue that adds it. A row a dev tool wrote is marked `dev` ([[§14.10]]).

Every roll is seeded on the server, and its seed and result are logged with the grant it makes, so any opening replays exactly and can be audited. Each opening's seed is drawn afresh, so no seed an account can read foretells a later roll.

### 14.12 Going live

Until the going-live sub-issue (#1310) shipped, these stood: the launch grant, a copy of every non-token card on becoming active ([[R111]]), and [[§9.4]]'s grant of each patch's new cards to every person's active account ([[R481]]). Every person's active account owned every card, a dev account that had cleared its own aside ([[§14.10]]), the server's `ECONOMY_LIVE` switch was off, and the parts of this section were built behind them without changing what a player owned: no caller of the reward hook paid while the switch was off ([[R1302006]]), and no quest arrived ([[R1304002]]). The going-live sub-issue wrote the rulings that replace them ([[R1310000]] to [[R1310004]]), edited [[§9.1]] and [[§9.4]], and turned `ECONOMY_LIVE` on, in the server and in the client, which reads it from `serverConfig.ts`. Since then:

- A new account starts with the starter cards and its tracks' first rewards ([[§14.7]]) and earns the rest as this section says. The starter cards are `STARTER_CARDS`, the Core cards of the three practice decks ([[§9.9]]), and the first rewards are the newest set's track's first level, paid as the account becomes active ([[R1310000]]). Queueing checks ownership where it already did ([[§9.4]] L5), and a deck save still checks structure alone ([[R250]]): a draft may hold a card the player does not own, and is judged when queued ([[R1310000]]).
- An existing account kept every card granted to it, so nobody lost a deck. Its economy started at zero, and the ledger recorded the move with the reason `launch` ([[R1310002]]).
- A new catalog version grants no account its cards: a new set's cards come from its track and its packs ([[R1310001]]).
- The starter cosmetics stay owned by every person's account. The full-art faces started unowned for everyone, existing accounts included ([[R1310002]]), and every cosmetic added later must be owned ([[§14.2]]).
- Practice and hotseat let a player use every card and every cosmetic whatever the ledger holds, and so do the end-to-end tests, but the one that walks the economy itself, on a fixture account of its own ([[R1310004]]), whose token bars a dev account's tool fills ([[R1310003]]).
