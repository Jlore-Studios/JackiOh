## 13. Duos

Duos is co-op: two players, team A, against an AI team, team B, in two ordinary 1v1 games side by side, the duels, whose teams share their life and pass cards to each other.

Every card sees a 1v1 game: you and the player across from you. Only three things cross between the duels: a team's shared life ([[§13.4]]), passing ([[§13.5]]) and the round, which ends both duels' turns together ([[§13.3]]). Nobody plays on one combined board, and every player has full control of their own. Sections 2 to 8 hold inside each duel wherever this section says nothing else, and nothing here reaches a 1v1 game: a 1v1 state carries no Duos field, so every 1v1 replay, golden trace and fuzz seed hashes exactly as before.

Duos needs a signed-in, active account, as Play online does ([[§9.4]]). It runs on the server, starts from the Play together page ([[§9.5]], [[§13.9]]) and is unranked ([[§9.12]]). Out of scope: two humans against two humans, ranked Duos, Duos-only cards, watching a Duos game, Duos games in the recent games list (a co-op match has its own replay, [[§13.9]]), and emotes and aim arrows in Duos ([[§13.10]]).

### 13.1 Seats and pairing

Four seats in two teams, paired for the whole game:

| Seat | Team | Duel | In its duel | Teammate | Across |
| --- | --- | --- | --- | --- | --- |
| A1 | A, the humans | 0 | Player 1 | A2 | B1 |
| A2 | A, the humans | 1 | Player 1 | A1 | B2 |
| B1 | B, the AI | 0 | Player 2 | B2 | A1 |
| B2 | B, the AI | 1 | Player 2 | B1 | A2 |

So team A goes first and team B second in both duels. Each seat has its own 20-card deck, mana, hand, library, graveyard, exile, hero and zones, exactly as a 1v1 player has ([[§3]]).

Card words resolve inside your own duel. "Opponent", "enemy" and "the other player" are the player across from you; "each player", "both players", "all Units", "the field" and every other word for the game's players or cards mean your duel's two players and their cards; Silly Silas's rings ([[§3.1]]) are your duel's ten unit zones and ten backrow zones; a steal ([[§6.3]], [[R12]]) takes from the player across; and Pocket Chaos's swaps ([[R73]]) are with the player across, its health swap as [[§13.4]] says. Your teammate is not a player of your duel: no card targets, counts, damages, heals, reads or copies anything of theirs, and nothing in the catalog reaches the other duel.

### 13.2 Setup

A co-op match ([[§13.9]]) makes its two duels from its seed: duel 0 is a 1v1 game seeded `{seed}:duel0` with A1's and B1's decks, duel 1 one seeded `{seed}:duel1` with A2's and B2's, and each is set up as [[§2.1]] sets up a game. A human seat brings a saved deck that passes the Best of 1 rules ([[§9.4]]: L2, L3, L5 and L6, and L7, so a deck with a custom card plays no co-op game, [[R915005]]) or is dealt a random deck as All Random deals one ([[R258]]), leaning on the newest set when its player asks ([[R1372]]); an AI seat's deck is dealt by the AI's deck-builder at its tier's deck size ([[§9.9]], [[R184]]). Each duel hero starts at its seat's own HP, and each team at its pool ([[§13.4]]), instead of at 30 health.

- Opening hands go by team: team A's seats, each its duel's Player 1, draw 3, and team B's draw 4 ([[§2.1]] step 2).
- All four mulligans are open at once, [[R265]]'s way: each seat answers its own when it likes, an answer is sealed ([[R266]]), and a duel's two resolve, its Player 1's first, once both are in. Each of team B's seats is then dealt The Coin ([[R244]]), and each duel resolves its start-of-game effects ([[§2.1]] step 4).
- The four mulligans share one clock, `DUOS_MULLIGAN_CLOCK_MS` (45 s); a seat still owing when it runs out keeps its whole hand, as [[R268]] says: the AI's own mulligan ([[§13.7]]) never answers for a person, only for a seat past its disconnect grace, which the AI plays outright ([[§13.8]]).
- No seat takes a main-phase action until all four mulligans have resolved, and the round counter stays at 0 meanwhile. Then round 1 opens, with A1 and A2 to act.

### 13.3 Rounds and turns

A round is one team turn. Its round seats are the seats whose turn each duel is on as the round opens, usually both seats of one team, and they act at the same time: each plays its own duel's main phase, and the server applies their actions one at a time, in the order they arrive. Round 1's round seats are A1 and A2.

- Inside a duel the turn loop is [[§2.2]]'s, run as in a 1v1 game. A seat takes its duel's main-phase actions only while it is that duel's round seat and has not ended its turn.
- Each round seat ends its own turn. Its `endTurn` marks it ended and changes nothing in its duel yet. Once both round seats have ended, the round closes: duel 0's turn ends ([[§2.2]]'s end-of-turn steps, then the next turn's start, its draw included), then duel 1's, and the next round opens with each duel's new active player as that duel's round seat.
- A seat that has ended has no main-phase action left, but it still answers a prompt it holds and still receives passes ([[§13.5]]).
- A duel whose turn ends by itself, through an effect ([[R456]]), counts its round seat as ended, and that duel's new active player waits for the round to close before it acts.
- An extra turn ([[R846]]) keeps that duel's next round seat on the same player, so for a round the two duels' round seats can belong to different teams, each acting in its own duel. Nothing waits on a seat that cannot act: each duel's active player is its round seat.
- [[R82]]'s automatic turn end is the round's to apply: a round seat whose only legal actions in its duel are ending the turn, conceding and draw offers, and which has no pass it may make ([[§13.5]]), is ended for it. A card it could still pass keeps it in the round.
- Each duel's turn counter moves once a round, so [[§2.5]]'s turn cap (`TURN_CAP_PLAYER_TURNS`, 60) counts rounds, which are team turns: 30 for each team.
- A prompt belongs to the seat whose duel opened it, and one duel's prompt never holds up the other duel.
- No draw is offered or answered in Duos, and a seat's 1v1 concede is refused: a team concedes together ([[§13.8]]).
- The round's own turn ends reach the duels under nonces of the round's own, in a namespace no seat's action may carry (`duos:`; an action whose nonce starts with it is refused, as [[R270]] keeps the server's `srv-` from a client). So no action can take the nonce a round's close will use and turn that turn end into the duel's replay of an action it has already applied, which would leave the duel's turn where it is and the round never closing. A duel that has already applied a round's nonce refuses that turn end, which is tried again after the next step, rather than taking it as done.

The round has one clock, `DUOS_ROUND_CLOCK_MS` (90 s), in place of [[§9.5]]'s turn clock: when it runs out, every round seat that has not ended has its open prompts answered by the AI ([[§13.7]]) and its turn ended. A prompt held by a seat that is not a round seat has its own clock, `DUOS_PROMPT_CLOCK_MS` (30 s), which on expiry answers that prompt alone, as [[R79]]'s prompt clock does in a 1v1 game. Both are server config values.

### 13.4 Life

A team's life is in two layers: each seat's own HP and the team's shared pool. A seat starts with its team's `own_health` as its duel hero's health, and the team's pool starts at its `pool_health`, which is also the pool's cap. The humans' team rules are `DUOS_HUMAN_TEAM`: `DUOS_OWN_HEALTH` (20) own HP for each seat, a `DUOS_POOL_HEALTH` (20) pool, and each seat passing up to `DUOS_PASS_CAP` (10) cards a turn at `DUOS_PASS_COST` (1) mana each ([[§13.5]]); the AI team's rules depend on its tier ([[§13.7]]).

- Damage, once Armor has taken its part of the hit ([[§4.4]]), and life loss ([[R18]]), which no Armor stops, come off the seat's own HP first, then off the pool, the excess carrying over. Fatigue is damage and is taken the same way, by the seat that drew.
- Own HP that reaches 0 or less is spent for the rest of the game: what went past 0 comes off the pool, and from then on the hero's health is the pool. Every word for a hero's health reads own HP while the seat is not spent and the pool once it is.
- While a seat is spent, damage and life loss on its hero come off the pool, and healing on it goes to the pool, never above the pool's cap. While it is not spent, healing works as in a 1v1 game, with no cap, and never reaches the pool.
- Both duels always agree on a team's pool: after every step the pool is copied to the teammate's duel, and to the teammate's hero's health when the teammate is spent.
- [[§4.4]] step 4a's lethal judge counts all of a seat's life: a hit is lethal only when it would leave the seat's own HP and its team's pool together, the pool alone once the seat is spent, at 0 or less.
- An effect that sets a hero's health sets own HP while the seat is not spent (a value of 0 or less spends it, as damage would) and sets the pool once it is, never above the pool's cap.
- A health swap (Pocket Chaos, [[R73]]) swaps the two heroes' own HP while neither hero is spent, and does nothing once either is.

A co-op game ends at the first of these, judged after each step. The round's close is part of the step that closed it: both duels' turn ends ([[§13.3]]) are applied, duel 0's and then duel 1's, before the game is judged, so pools emptied by either turn end are emptied in the same step, and a team whose pool one of them empties does not win because the other was not yet applied.

| Condition | Result |
| --- | --- |
| A team's pool at 0 or less after a step | That team loses (`poolEmptied`) |
| Both teams' pools at 0 or less after the same step | Draw |
| A seat holding a win at its duel's game-end point ([[R850]]) | That seat's team wins (`duelResult`) |
| A duel reaching the turn cap | Draw (`turnCap`) |
| A team's concede, offered and confirmed ([[§13.8]]) | That team loses (`concede`) |
| The hard match ceiling ([[§9.5]]), `MATCH_CEILING_MS` after the match was made ([[R1047000]]) | Draw (`ceiling`) |

A disconnect never ends a co-op game ([[§13.8]]).

### 13.5 Passing

On its own turn a seat may pass a card from its hand to its teammate's hand. A pass names the card and is legal while all of these hold:

- the seat is a round seat that has not ended its turn;
- the card is in its hand;
- it has at least its team's pass cost in mana (`DUOS_PASS_COST`, 1, for the humans);
- it has passed fewer cards this round than its team's pass cap (`DUOS_PASS_CAP`, 10);
- its teammate's hand is below that player's hand size ([[§2.4]]: 10, or the size M #79 set, [[R1143]]).

A pass pays the pass cost and moves the card from the passer's hand to the right end of the teammate's, under a new instance id, with the teammate as its owner and its controller, as a steal from a hand makes the thief its owner ([[R12]]). The card keeps its buffs, its cost modifiers and its face. It arrives at once, so a teammate who is a round seat and has not ended may play it the same round. A pass is not a draw, a play or a discard, and nothing triggers on it. Each seat's passes start again each round, and its team sees how many each of its seats has left ([[§13.6]]).

The team sees the card go across; each opposing seat sees only a card back leave one hand or arrive in the other, as the hand count its 1v1 view already shows. The client offers a pass only where the seat's legal actions hold it, on an inspected hand card or by dragging a hand card onto the portal ([[§13.6]]), with the pass cost and the passes left beside it; it checks nothing itself (CLAUDE.md rule 7).

### 13.6 Information and the view

A team shares its private information, and an opponent's hidden cards stay hidden from both teammates. Each seat's view holds:

| Part | What |
| --- | --- |
| Its own duel | `view_for` of its duel from its seat, as in a 1v1 game ([[§10.8]]) |
| Its teammate's duel | `view_for` of the teammate's duel from the teammate's seat: their hand, their face-down cards, their mana and their prompts, and their opponent as they see them |
| Both teams' life | each seat's own HP and whether it is spent, and each team's pool and its cap |
| Its own team's turn | which of its seats have ended this round, how many passes each has left, and the pass cost |
| The game | the round number, its own team's standing concede offer, and the result once there is one |

Of the other team a seat sees the life only: whether those seats have ended and how many passes they have left stay hidden. Nothing of an opponent's hidden cards appears beyond what a 1v1 `view_for` already gives that seat, and the teammate's part shows only what the teammate may see.

The client draws the seat's own duel as the 1v1 board ([[§10.10]]), with a life strip that shows both teams' own HP and pools, a spent seat's own bar giving way to its pool. A portal on the board flips it to the teammate's duel, read-only: its cards can be hovered and inspected and never played. A teammate strip always shows the teammate's own HP, mana and hand count and whether they have ended, and once the seat has ended its turn the board says it is waiting for its teammate. A Duos match deals no emote hand and relays no emote and no aim ([[§9.5]]): a team talks by pings ([[§13.10]]).

### 13.7 The AI team

One AI plays both seats of team B. Each seat plays its own duel as the practice AI plays a 1v1 game ([[§9.9]]): its mulligan and its prompts by the 1v1 decision, and its own turn by the same search, except that when the search would end the turn, the seat first makes its passes and then ends it. It counts a seat's life as [[§13.4]]'s lethal judge does, its own HP and its team's pool together, and values a seat against the life it started with ([[R964000]]).

- **Passing.** When it can pay, a seat passes a hand card it cannot play this turn and its teammate can: the teammate is a round seat that has not ended, can afford the card and has a zone for it. When passing is free, it also passes a card the teammate can afford next turn and it cannot. It passes the best cards first, and never more than its passes left or its teammate's free hand slots.
- **Information.** It decides from what its team may know ([[R185]]): its own duel through the 1v1 decision's determinization, and its teammate's own state, which the team shares ([[§13.6]]). It never reads an opponent's hidden card.
- **Determinism.** The same state and AI seed give the same decision, so a co-op game replays exactly.
- **Concedes.** The AI never concedes ([[R188]]), so an AI seat neither offers a team's concede nor confirms one.

The difficulty sets the AI team's rules ([[§13.4]]) and each AI seat's handicap, extending [[§9.9]]'s tiers:

| | Easy | Medium | Hard |
| --- | --- | --- | --- |
| Team rules (`duos_ai_team`) | `DUOS_HUMAN_TEAM`, the humans' | the humans', passing free | the humans', passing free, pool `DUOS_POOL_HEALTH_HARD` |
| Own HP per seat | 20 | 20 | 20 |
| Pool and its cap | 20 | 20 | 30 |
| Pass cost | 1 | 0 | 0 |
| Passes per seat per turn | 10 | 10 | 10 |
| Each AI seat's handicap (`AI_DIFFICULTY`, [[§9.9]]) | Easy's | Medium's | Hard's |

The humans play `DUOS_HUMAN_TEAM` at every tier. A quality gate (`cargo jackioh gate --duos`) holds the AI team to a floor: the Easy AI team must beat a team playing [[§10.7]]'s random policy in at least `DUOS_GATE_WIN_FLOOR` (80%) of `DUOS_GATE_GAMES_SMOKE` (20) games.

### 13.8 Leaving

- **A disconnect.** A human seat whose socket has been gone past the disconnect grace (`DISCONNECT_GRACE_SECONDS`, 60 s, [[§9.5]]) is played by the AI, as [[§13.7]] plays team B's seats, until its player reconnects, and its teammate is told that the AI is playing it. A team never loses to a disconnect. A move the AI decided while it held a person's seat one way (played outright past the grace, or only to answer for a clock that ran out, [[§13.3]]) is dropped if that hold changed before the move applied, and the seat is decided for again: a player who is back is never played for, whatever clock ran out meanwhile.
- **Concede.** Conceding takes both humans. A seat's concede opens its team's concede offer, which both teammates see, and the teammate's concede while the offer stands ends the game, the other team winning. Each team has an offer of its own: the other team's concede neither takes nor clears it. An offer lapses when its round ends.
- **The way back.** A player who leaves a co-op game's page may go back to it while it lasts: the menus' banner offers it, as it offers a live match ([[R765]]).

### 13.9 Co-op rooms

A co-op game starts from a room on the Play together page ([[§9.5]]).

- The host makes a room with a difficulty (Easy, Medium or Hard, [[§13.7]]) and a deck, a saved one or a random one. The room has a code of 6 characters from the room-code alphabet ([[§9.5]]) and expires `DUOS_ROOM_TTL_MS` (10 minutes) after it was made if nobody has joined; the host may close it before then. The host invites a friend by sending the code in chat ([[§9.5]]). A host holds at most `MAX_OPEN_ROOMS_PER_HOST` (3) co-op rooms nobody has joined, counted apart from their 1v1 rooms as [[R890000]] bounds those; a fourth is refused ([[R1047002]]).
- A second active account joins by the code, with its own deck or a random one, which starts the match: the host is A1, the guest A2, and the AI plays B1 and B2. Both players are taken to the game. A third account is refused as the room being full, and the host cannot join their own room.
- A co-op match holds its two people as a 1v1 match does ([[R1065000]]): no one in a 1v1 match, a Conquest series that is not over or a live co-op match makes or joins a co-op room, and a join is refused while the host is in one. While their co-op match lasts they neither queue, open or join a 1v1 room, nor are paired, nor start a rematch, and its start cancels any ticket either left open.
- A code is no key past a block ([[§9.4]]): to a player the host has blocked, or who has blocked the host, the room answers as an unknown code does, to read it and to join it. Once joined, a room answers its host and its guest alone; anyone else reads it as an unknown code (a join is still told the room is full).
- A host's saved deck is checked against the catalog when the room is made and frozen with that catalog's version. A join after a catalog change is refused as an update, as a 1v1 room's is ([[§9.4]]): the running catalog may no longer deal that deck. The match is pinned to the catalog version it is dealt from: its play, every rebuild of it and its replay read that version's cards ([[R767000]]).
- One actor per co-op match holds its state on the server, as a 1v1 match's does ([[§9.2]]). Each human seat's socket gets its view and its legal actions as it connects and fresh ones after every applied action; an illegal action is refused and changes nothing. The AI's seats act on the server whenever they owe an action, one action every `DUOS_AI_STEP_DELAY_MS` (700 ms), so the humans can follow them.
- Every applied action is appended to the match's log, and a crashed actor rebuilds its state by folding `(seed, log)` under the match's catalog version: the fold reaches the same state and the same hash as live play, and stops at the first action it refuses. A socket that holds no seat in the match is refused before anything is folded for it, and a fold runs off the server's request threads. The end writes the result once. The server's sweep folds back a live match that has no actor; one it cannot fold back is ended at its ceiling as the ceiling's draw, which frees both people ([[R1047001]]). As for a 1v1 match, the seed, the decks and the log never leave the server ([[§9.1]]). Random-policy co-op games fold back to the hashes they were played to (`cargo jackioh fuzz --duos`), as 1v1 games do.
- The socket takes a 1v1 socket's nonce rules ([[R270]]) and the round's own namespace (`duos:`, [[§13.3]]) is refused as `srv-` is; a `hello` repeated while one from the same socket is still queued is absorbed by it, as [[R835001]] has it for a 1v1 socket ([[§9.1]]).
- A co-op game is unranked: it records a result for both humans and moves no rating ([[§9.12]]).
- Either human seat may watch a finished co-op game again, its own seat's view after each step, as [[§9.5]]'s replays show a 1v1 match; an account that held no human seat gets what an unknown match gets, and a game still being played has no replay until it ends. The replay folds under the catalog version the match was played on and is refused, never cut short, when this build neither is nor carries that version, or when the log no longer folds to its end or to the result the match recorded ([[R768]], [[R767000]]). A co-op game's log is kept `MATCH_ACTION_RETENTION_DAYS` after the game ends, as a 1v1 match's is ([[§9.5]]); then the retention purge takes it and its replay is gone, while the match and its result stay. A co-op game is not in the recent games list.

### 13.10 Pings

A team talks by pings. On any card a player can see, in either duel, they may send one of five: thumbs up (`thumbsUp`), thumbs down (`thumbsDown`), the portal (`portal`, "pass this"), careful (`careful`) and kill this (`killThis`).

- A ping names its card by a public handle, as an aim does ([[R738]]): a hero, or a unit or backrow zone by its duel, side and lane, or a hand card of the sender's own team, their own or their teammate's, by its position. A card in the other team's hands is never a target.
- The server checks a ping's shape, drops one that names the other team's hand or a position past a hand's end, drops every ping past `PING_RATE_LIMIT` (5) in `PING_RATE_WINDOW_MS` (3 s) silently, and relays the rest to the teammate's socket alone. A ping never enters the action log, a replay or a view, and no game state sees it.
- A ping wheel on a hovered or inspected card offers the five. A received ping plays its own sound and marks its card for `PING_SHOW_MS` (2.5 s) on whichever view shows it, the own duel or the teammate's, and flashes the portal when that view is not the one showing.
