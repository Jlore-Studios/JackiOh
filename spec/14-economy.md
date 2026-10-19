## 14. Economy

How an account comes to own cards and cosmetics: Jlore, the cosmetics there are to own, card packs and their slot roll, pity, the token bars and the cosmetic token cycle, cosmetic packs and the Chase Shard, set tracks and XP, what play earns, daily quests, dev accounts, the ledger every grant is written to, and going live. Every gameplay card is earned by playing, and generously; the endgame is a collection of cosmetics. The model is the designer's, tracked in #1294; each of its sub-issues builds a part and writes the rulings it needs ([[§11]]), #1294's numbers go into `crates/server/src/config.rs` (exported to `apps/web/src/wire/serverConfig.ts`) together, before the parts that use them; a constant whose value #1294 does not give goes in with the sub-issue that first needs it, and `DEV_PROFILE_IDS`, an environment variable, is read with the rest of the server's environment.

Every value here is a constant named in backticks, never a number (CLAUDE.md rule 9). The server owns every balance, roll, counter, bar and grant ([[§9.1]]): the client sends intent, such as buying or opening a pack, redeeming a token or picking a chase, and renders the server's projection of its own account, deciding nothing (CLAUDE.md rule 7). An account reads its own wallet, inventory, pity counters, bars, tracks, quests, cosmetics and ledger rows, and nobody else's: what a pack held, a counter or a balance reaches no other player. Only non-token cards of the sets that ship are collected: no pack, reward or grant holds a token or a card of a set that has not shipped ([[§9.4]] L3, [[R1420]]). Real money is out of scope: nothing here is sold, and nothing is bought but with Jlore. Art cosmetics and showing an opponent's cosmetics are out of scope too: this section stops at which cosmetics an account owns. A ladder bot owns no collection ([[R1454]]) and earns nothing. Until the economy goes live ([[§14.12]]), every person's active account owns every card ([[R111]], [[R481]]), a dev account that has cleared its own aside ([[§14.10]]), and until the sub-issue that pays for practice writes its ruling, a practice game records nothing ([[R187]], [[§14.8]]).

| Constant | What it holds | Where |
| --- | --- | --- |
| `PACK_PRICE_JLORE` | The Jlore a card pack costs | [[§14.3]] |
| `PACK_SLOTS` | The slots of a card pack | [[§14.3]] |
| `PACK_UNOWNED_WEIGHTS` | How a slot's card draw leans toward the cards the account does not own | [[§14.3]] |
| `PACK_BASE_RATES` | The chance a card pack holds each rarity before pity | [[§14.4]] |
| `PACK_TARGET_RATES` | The chance a card pack holds an Epic, a Legendary or a Mythic once pity is counted in | [[§14.4]] |
| `PACK_RATE_TOLERANCE` | How far the seeded simulation's rates may land from `PACK_TARGET_RATES` | [[§14.4]] |
| `PITY_EPIC_PACKS`, `PITY_LEGENDARY_PACKS`, `PITY_MYTHIC_PACKS` | The run of card packs in a row within which that rarity always drops | [[§14.4]] |
| `CARD_TOKEN_BAR_SIZE`, `COSMETIC_TOKEN_BAR_SIZE` | The points that fill each token bar | [[§14.5]] |
| `CARD_TOKEN_BAR_POINTS_PER_DUPLICATE`, `COSMETIC_TOKEN_BAR_POINTS_PER_DUPLICATE` | The points a spent duplicate adds to each bar | [[§14.5]] |
| `CARD_TOKEN_BAR_TARGET_PACKS`, `COSMETIC_TOKEN_BAR_TARGET_PACKS` | The card packs each bar takes to fill, on average | [[§14.5]] |
| `COSMETIC_TOKEN_CYCLE` | The steps the cosmetic token bar pays, in order | [[§14.5]] |
| `COSMETIC_PACK_SLOTS`, `COSMETIC_PACK_RATES` | A cosmetic pack's slots and their odds | [[§14.6]] |
| `CHASE_SHARD_PACKS` | The cosmetic packs that fill the Chase Shard bar | [[§14.6]] |
| `CHASE_DROP_RATE` | The chance a card pack's cosmetic is a chase | [[§14.6]] |
| `SET_TRACK_LEVELS` | The levels of a set track | [[§14.7]] |
| `SET_TRACK_XP_PER_LEVEL` | The XP step of the level curve | [[§14.7]] |
| `SET_TRACK_UNLOCK_LEVEL` | The newest track's level that unlocks the older ones | [[§14.7]] |
| `JLORE_LADDER_WIN`, `JLORE_LADDER_LOSS` | The Jlore a ladder game pays | [[§14.8]] |
| `JLORE_PRACTICE_WIN`, `JLORE_PRACTICE_LOSS` | The Jlore a practice game pays | [[§14.8]] |
| `XP_PER_ACTION`, `XP_WIN_BONUS`, `XP_GAME_CAP` | A game's XP: per action, for a win, and its most | [[§14.8]] |
| `XP_PRACTICE_SCALE` | A practice game's share of the XP the same game earns on the ladder | [[§14.8]] |
| `XP_LADDER_TARGET_MEAN` | The XP a ladder game averages | [[§14.8]] |
| `PRACTICE_SEED_TTL_SECONDS` | How long a practice seed waits to be claimed | [[§14.8]] |
| `QUEST_INTERVAL_HOURS`, `QUEST_MAX_HELD` | How often a quest arrives, and how many are held | [[§14.9]] |
| `QUEST_JLORE_MIN`, `QUEST_JLORE_MAX`, `QUEST_XP_PER_JLORE` | A quest's Jlore, and its XP per Jlore | [[§14.9]] |
| `DEV_PROFILE_IDS` | The environment variable that lists the dev accounts | [[§14.10]] |
| `ECONOMY_LIVE` | The server's switch for the economy | [[§14.12]] |

### 14.1 Jlore

Jlore is the soft currency: a whole-number balance in each account's wallet, never below zero. It is earned only by playing: games ([[§14.8]]), daily quests ([[§14.9]]) and set track levels ([[§14.7]]), besides what a dev account grants itself ([[§14.10]]). It is never sold, and no money becomes Jlore. It buys card packs ([[§14.3]]). A spend the wallet cannot cover is refused and changes nothing.

### 14.2 Cosmetics

A cosmetic is an entry of the cosmetics registry, one data file the server and the client both read: its id, its kind, its name, its rarity, one of [[§5]]'s from Common to Mythic, whether it is its kind's default and whether it is a chase. A per-card cosmetic (a foil, a finish, a border, a full-art face) is owned for one card; an account cosmetic (a board, a field, a card back) is owned for the whole account. Apart from the starter cosmetics, to own a cosmetic is to hold it in the ledger ([[§14.11]]). A player's cosmetics show on their own side only: showing an opponent's needs the frame that would carry them, which is out of scope here, and the client never infers what an opponent owns.

- **Starter cosmetics.** Each kind's default and the starter items of the card and deck slots are owned by every person's account, on every card, before and after going live.
- **Chase cosmetics** are marked in the registry. No token pays for one; the Chase Shard bar buys one, and one can drop by luck ([[§14.6]]).
- **Full-art faces.** The full art of the hero-portrait cards ([[R1017001]]), six today, is a normal per-card cosmetic at its card's rarity, not a chase, so it drops from packs like any other, and a cosmetic token of its rarity buys it when its card is Rare or Epic; a Common card's full art comes from packs alone, since the token cycle pays only Rare and Epic tokens ([[§14.5]]). Once the economy is live, a player's own copy prints its full art when they own it and their Full-art cards setting is on; every other viewer of it, an opponent, a watcher or a replay's viewer, sees its normal face until another player's ownership can be shown. Nobody owns one when the economy goes live ([[§14.12]]). The landing page's fan and the Almanac keep showing full art, and practice and hotseat treat every cosmetic as owned ([[§14.12]]).

### 14.3 Card packs and the slot roll

There is a card pack for each set that ships ([[R1420]]) and a global card pack, whose pool is every set that ships. Each costs `PACK_PRICE_JLORE` and holds `PACK_SLOTS` slots. Buying one debits the wallet and adds the pack, unopened, to the account's inventory, in one transaction. Packs also come from set track rewards ([[§14.7]]). Opening a pack rolls every slot and grants what each holds, its pity counters and bars included, in one transaction or not at all.

Each slot rolls a rarity, one of [[§5]]'s from Common to Mythic, through the pity math ([[§14.4]]). One slot of every card pack, the Rare-or-better slot, never rolls Common, so every pack holds a Rare or better. The slot then rolls a card of that rarity from the pack's pool, the non-token cards of its set or, for the global pack, of every set that ships, weighted toward the cards the account does not own by `PACK_UNOWNED_WEIGHTS`. What the slot grants follows the dupe chain:

- **A new card**, one the account owns fewer than `MAX_COPIES` of, unlocks: the account is granted a copy.
- **A duplicate**, a card the account already owns `MAX_COPIES` of, the most a deck may hold and never more than the account owns ([[§9.4]] L3, L5), becomes a per-card cosmetic of the same rarity for that card ([[§14.2]]), one the account does not own on it; it is a chase only by luck ([[§14.6]]).
- **A spent duplicate**, a duplicate whose card has no cosmetic of that rarity left for the account to own, chases aside, adds points to both token bars ([[§14.5]]).

### 14.4 Pity

Pity keeps a counter for each of Epic, Legendary and Mythic; whether an account keeps one set of counters for all its card packs or one per kind of pack is the pity math's sub-issue's to rule ([[R1298000]]). A counter counts card packs, not slots: a pack with a slot that rolls the counter's rarity resets it, whatever that slot then grants, and a pack with none adds to it. A counter resets only when its own rarity drops: a Mythic leaves the Epic and Legendary counters running, and a Legendary the Epic one. The counters are the account's to see, with each rarity's odds.

A rarity's threshold (`PITY_EPIC_PACKS`, `PITY_LEGENDARY_PACKS`, `PITY_MYTHIC_PACKS`) is the longest an account waits for it: of any `PITY_EPIC_PACKS` card packs in a row that one Epic counter counts, at least one holds an Epic, the last of them when none before it did, and likewise for Legendary and Mythic. No counter ever passes its threshold, and `PITY_MYTHIC_PACKS` is the longest wait of all.

Before pity, a card pack holds each rarity at its base rate (`PACK_BASE_RATES`); with pity counted in, it holds an Epic, a Legendary and a Mythic at the effective rates `PACK_TARGET_RATES`. Both are chances per pack, not per slot. The thresholds are fixed and the base rates are tuned: a seeded simulation of the pity math over many packs must land within `PACK_RATE_TOLERANCE` of every target. The pity math is pure, from the counters and a draw to the rarities and the next counters, so the pack roll calls it with its own seeded, logged draws ([[§14.11]]).

### 14.5 The token bars and the cosmetic token cycle

Spent duplicates ([[§14.3]]) fill two bars: each adds `CARD_TOKEN_BAR_POINTS_PER_DUPLICATE` to the card token bar and `COSMETIC_TOKEN_BAR_POINTS_PER_DUPLICATE` to the cosmetic token bar. A bar that reaches its size (`CARD_TOKEN_BAR_SIZE`, `COSMETIC_TOKEN_BAR_SIZE`) pays and keeps the overflow, in the opening's own transaction. The points per duplicate are set so that, on a collection that is mostly owned, each bar fills once every `CARD_TOKEN_BAR_TARGET_PACKS` or `COSMETIC_TOKEN_BAR_TARGET_PACKS` card packs on average, by the same seeded simulation as the pity math's rates ([[§14.4]]), which states the collection it assumes.

- **The card token bar** pays a card token. A card token unlocks any new card ([[§14.3]]) of a set that ships that the account chooses.
- **The cosmetic token bar** pays the next step of `COSMETIC_TOKEN_CYCLE`: Rare, Epic, Rare, Epic, Rare, Cosmetic Pack, and then the same again. The account keeps its place in the cycle. A Rare or Epic step is a cosmetic token of that rarity, which unlocks any cosmetic of that rarity that the account chooses and does not own: a per-card cosmetic for a card it owns, or an account cosmetic, never a chase. A Cosmetic Pack step adds an unopened cosmetic pack to the inventory ([[§14.6]]).

Redeeming a token is one transaction. A choice the token cannot pay for, a card or cosmetic already owned, a cosmetic of another rarity or a chase, is refused and spends nothing.

### 14.6 Cosmetic packs and the Chase Shard

A cosmetic pack is earned only, for now: it is the last step of the cosmetic token cycle ([[§14.5]]), and nothing sells one. Opening one rolls `COSMETIC_PACK_SLOTS` slots of cosmetics, not cards, at the odds `COSMETIC_PACK_RATES`, through the same seeded, logged roll as a card pack ([[§14.11]]). A slot whose cosmetic is for a new card ([[§14.3]]) also grants a copy of the card.

The Chase Shard bar counts the cosmetic packs the account opens. At `CHASE_SHARD_PACKS` it is full: it buys a chase cosmetic the account chooses and does not own, and empties. Chases also drop from card packs, by luck, at `CHASE_DROP_RATE`: a duplicate's cosmetic ([[§14.3]]) may be a chase.

### 14.7 Set tracks and XP

Each set that ships has a track ([[R1420]]) of `SET_TRACK_LEVELS` levels. A track starts at its first level with no XP, and going from level n to the next costs `SET_TRACK_XP_PER_LEVEL` × n XP: the cost rises with every level and is never capped. The XP of games ([[§14.8]]) and quests ([[§14.9]]) goes to every active track, each gaining all of it; a dev account's own grant goes to the track it picks ([[§14.10]]). The newest set's track is active from the start, and an older set's once it unlocks: the older tracks unlock one at a time once the newest set's track reaches `SET_TRACK_UNLOCK_LEVEL`. Which older track unlocks next, and what unlocks each after the first, the sub-issue that builds the tracks rules.

Each level pays rewards, cards, Jlore and card packs, from a reward table per set kept as data. A level pays once, as ledger rows, when its track reaches it ([[§14.11]]). A track does not pay out every card of its set; pack luck covers the rest ([[§14.3]]).

### 14.8 Earning

Games earn Jlore and XP for the people who play them, never for a ladder bot. A ladder game, a ranked match ([[§9.12]]) that is not a game of a Conquest series, pays its winner `JLORE_LADDER_WIN` and its loser `JLORE_LADDER_LOSS`. A practice game ([[§9.9]]) pays `JLORE_PRACTICE_WIN` for a win and `JLORE_PRACTICE_LOSS` for a loss. Whether a game of a ranked Conquest series, an unranked game or a co-op game ([[§13]]) earns, and what a draw, a concede or a game the reaper ends pays, the sub-issues that build earning rule.

XP stands in for time played: `XP_PER_ACTION` for each action the player took, and `XP_WIN_BONUS` more for a win, the game's total never above `XP_GAME_CAP`, so a loop of empty actions cannot farm it. A practice game earns the XP the same game would earn on the ladder, times `XP_PRACTICE_SCALE`. The amounts are tuned so a ladder game averages `XP_LADDER_TARGET_MEAN`.

Every payout goes through one reward hook on the server, which takes a finished game's summary (the account, the mode, the result and the actions taken), pays it, and moves the account's quests ([[§14.9]]).

- **On the ladder** the server ran the match, so it counts the actions from its own log and pays in the transaction that records the result; the client reports nothing.
- **In practice** the game runs in the browser ([[§9.9]]), so the server takes nothing on the client's word. It issues the game's seed and remembers it with the account, the AI's difficulty and the deck. At the end the client sends the action log; the server folds it ([[§9.3]]) to the claimed result and checks that every move of the AI's seat is the one the AI makes from that seed and difficulty. Only a game that passes pays; a seed pays once and expires unclaimed after `PRACTICE_SEED_TTL_SECONDS`. Which practice games pay is that sub-issue's to rule too, among them a sandbox game, whose spawns ([[§9.9]]) a replay check passes, a tutorial lesson on its fixed seed ([[§9.10]]) and a game with custom cards ([[§12]]). Until that sub-issue writes its ruling, [[R187]] stands: a practice game records nothing.

### 14.9 Daily quests

A quest is a goal read from a finished game's public events, such as winning games, playing spells, dealing damage with Units or playing cards of a set, and a payout of Jlore from `QUEST_JLORE_MIN` to `QUEST_JLORE_MAX`, more for a harder goal, with `QUEST_XP_PER_JLORE` XP for each Jlore of it. Quests are drawn from a pool kept as data. An account holds at most `QUEST_MAX_HELD`; while it holds fewer, a new quest arrives `QUEST_INTERVAL_HOURS` after the last one did, by the server's clock, so a player away for a long while comes back to `QUEST_MAX_HELD` waiting. Every finished game the reward hook sees ([[§14.8]]) moves the held quests' progress, and a quest that completes pays its Jlore and XP once, as ledger rows; the XP goes to every active track ([[§14.7]]).

### 14.10 Dev accounts

A dev account is one whose profile id the server's `DEV_PROFILE_IDS` environment variable lists, read at boot with the rest of the environment: a malformed id fails the boot, naming it, and none set means no dev accounts. The server decides: it tells the client whether the caller is a dev account, and every dev route checks the list itself and refuses anyone else as forbidden. The client only chooses whether to show the Dev tools panel. Any signed-in account can reveal its own profile id on the account page, hidden until asked for, so a new developer can be listed.

A dev account can, for itself alone:

- give XP to a set track it picks, through the level curve ([[§14.7]]), and give Jlore;
- fill its collection with every card and every cosmetic;
- clear its cards, its cosmetics, its Jlore, its bars and pity counters, or its track progress, each apart, or all of them;
- be given a quest at once, or complete one ([[§14.9]]).

Every dev grant and clear is written as ledger rows marked `dev` ([[R1300000]]), so a dev grant can always be told apart ([[§14.11]]). [[R481]]'s grant of each new catalog version skips a dev account, so a clear stays cleared ([[R1300001]]). Otherwise a dev account counts as any other does: its games go into the card statistics ([[§9.11]]) and the ladder ([[§9.12]]).

### 14.11 The ledger

The ledger today is `collection` plus the append-only `collection_grants` (`crates/server/migrations/0002_collection.sql`): every change writes both in one transaction, and no client path writes either ([[§9.4]]). Every holding of the economy follows that pattern, a current-state table beside an append-only one, readable by its own account alone. Every grant of anything (cards, Jlore, packs, tokens, cosmetics, XP) is one append-only row, written server-side in the transaction that changes the state; so is every spend, such as a pack bought or a token redeemed. A row is never edited, and is deleted only with its account ("Delete my account", [[§9.4]]): a mistaken grant is corrected by an offsetting row.

Every row says why. The collection's reasons are `pack`, `craft`, `reward`, `refund`, `admin`, `launch` and `dev`, a dev tool's own ([[R1300000]]); a new kind of grant gets its reason from the sub-issue that adds it. A row a dev tool wrote is marked `dev` ([[§14.10]]).

Every roll is seeded on the server, and its seed and result are logged with the grant it makes, so any opening replays exactly and can be audited. Each opening's seed is drawn afresh, so no seed an account can read foretells a later roll.

### 14.12 Going live

Until the going-live sub-issue ships, these stand: the launch grant, a copy of every non-token card on becoming active ([[R111]]), and [[§9.4]]'s grant of each patch's new cards to every person's active account ([[R481]]). Every person's active account owns every card, a dev account that has cleared its own aside ([[§14.10]]), the server's `ECONOMY_LIVE` switch is off, and the parts of this section are built behind them without changing what a player owns. The going-live sub-issue writes the ruling that replaces them, edits [[§9.1]] and [[§9.4]], and turns `ECONOMY_LIVE` on. Then:

- A new account starts with the starter cards and its tracks' first rewards ([[§14.7]]) and earns the rest as this section says; which cards are the starter cards the going-live sub-issue sets, as data. Queueing checks ownership where it already does ([[§9.4]] L5). A deck save checks structure alone ([[R250]]) until the going-live ruling says otherwise.
- An existing account keeps every card granted to it, so nobody loses a deck. Its economy starts at zero, and the ledger records the move with a reason that says it was the launch.
- A new catalog version grants no account its cards: a new set's cards come from its track and its packs.
- The starter cosmetics stay owned by every person's account. The full-art faces start unowned for everyone, existing accounts included, and every cosmetic added later must be owned ([[§14.2]]).
- Practice and hotseat let a player use every card and every cosmetic whatever the ledger holds, and so do the end-to-end tests, but the one that walks the economy itself.
