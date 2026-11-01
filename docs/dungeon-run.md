# Dungeon runs: the plan

2026-11-01 · written from the owner's chat of the same day and two rounds of the owner's answers (Jack
Goetzmann, with Maximoose Goetzmann)

> **Status: the plan of record, not yet the spec.** SPEC is still the only source of rules (CLAUDE.md):
> the mode gets its own spec section and rulings when it is built. A name marked _placeholder_ is to be
> replaced, and a number marked _strawman_ is to be tuned. What is still open is in
> [DR13](#dr13-open-questions). M1, the run's core, is built in `crates/run` (its README is the contract).

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
- [DR14. Stage 1, the Sandbox: the fullsend tree](#dr14-stage-1-the-sandbox-the-fullsend-tree)

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
  never touches the server. You can make custom cards and play with them, and they join the run's card
  pools. It earns nothing, and its stats and leaderboards stay on the device.
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
| Rare | a Rare or lower |
| Epic | an Epic or lower |
| Legendary | a Legendary or lower |
| Mythic | a Mythic or lower |
| Anything | any card, a boss card you extracted included |

The slots get better with the starting deck's level, bought with the permanent currency: Rare slots become
Epic slots, Common slots give way to Anything slots, and so on. The deck never grows past 10. A strawman
ladder, to be tuned (`RUN_SLOT_LADDER`):

| Level | Common | Rare | Epic | Legendary | Mythic | Anything |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 5 | 4 | 0 | 0 | 0 | 1 |
| 2 | 4 | 4 | 1 | 0 | 0 | 1 |
| 3 | 3 | 4 | 2 | 0 | 0 | 1 |
| 4 | 2 | 3 | 3 | 1 | 0 | 1 |
| 5 | 1 | 2 | 3 | 2 | 1 | 1 |
| 6 | 0 | 2 | 3 | 2 | 1 | 2 |

The cards a starting deck may hold are about 20 unlocked at the start (the simplest Training Grounds
Commons and Rares, placeholders until the designer's list) and every card extracted since.

**Artifacts.** At most 3 of them, worth at most 8 points together. An artifact's tier is its strength and
its price:

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
- **Campfire:** one of: heal, make a card Radiant, remove 2 cards, or buff a card (the Buff keyword; a
  shop buffs one for Tickets too).
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

**Score**, built to make killing bosses pay: points for each row of the map reached, for each fight won
(more for an elite, a ghost, a mini-boss and the final boss), and for the damage dealt, healing done and
cards drawn in the fights; then times 1.5 for each mini-boss beaten, times 2 for the final boss, and 10%
more for each curse point. The weights are strawmen in `crates/run/src/config.rs`. Time and turns are
reported beside the score, not in it.

**Leaderboards:** the deepest floor, the most curse points, the top score and the fastest win, with a timer
on screen. Sandbox boards stay on the device; Campaign boards hold checked runs.

**Stats** kept for every run: nodes visited; fights, elites and bosses won; damage taken and healed; each
currency earned and spent; cards kept, bought, removed, upgraded and buffed; packs skipped; artifacts
taken and upgraded; blessings taken; events seen; chests opened; the time and the turns played.

## DR10. The extended campaign

After the final boss, the run goes on into the next world, and the worlds never end.

- Every enemy carries a prestige artifact, the designer's and deliberately busted, so each world is
  harder than the last.
- You keep all your artifacts and your curses. HP resets.
- Your deck becomes 10 cards again: 5 you choose from the run's deck and 5 of the 10 you started with.

## DR11. Where it lives

- **The run's rules: pure Rust,** seeded like the engine. The map, the states, the packs, extraction, the
  slots, curses, the score and the stats. A run folds back from its seed and its log, so the server can
  check a Campaign run by replaying it, as it checks practice games. It is a new pure crate, `crates/run`
  (CLAUDE.md rule 4).
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
| M1 | The run's core, offline: seeds, the map, the states, the packs, extraction, the slots, artifact points, curse tiers, the score and the stats, on placeholder content. **Built:** `crates/run` | — |
| M2 | Run fights: the carried HP, Looming and Overwhelming Presence, Radiant and buffed run cards in a game's setup | M1 |
| M3 | The Sandbox screen: the map, the tiles, the packs, the artifact tray, the timer, local leaderboards | M1, M2 |
| M4 | Artifacts in fights (Combat) and on the map (Economic) | M2, M3 |
| M5 | The tiles in full: shop, campfire, events, chests, shrines | M3 |
| M6 | The Campaign: the account's unlocks, Embers and levels, checked runs, leaderboards | M1 |
| M7 | Ghosts: snapshots and friends | M6 |
| M8 | The extended campaign | M5 |
| M9 | The designer's content: cards, bosses, artifacts, events, gods | M4 |

## DR13. Open questions

1. **Copies in a run deck.** A run deck holds one copy of each card for now, as every JackiOh deck does.
   With 3 cards kept a fight, should a pack offer a card you already have, for a second copy?
2. **The Buff:** how much one campfire or shop buff gives (+1/+1?), and whether a card takes it more than
   once.
3. **The starting 20 cards and the starter artifacts:** the designer's lists (placeholders until then).
4. **The prestige artifacts:** the designer's list.
5. **Ghost snapshots:** when one is taken (at a death, at a mini-boss, mid-game online) and how friends'
   are shared.
6. **Tuning:** the slot ladder, the prices, the enemies' health and the score's weights.

## DR14. Stage 1, the Sandbox: the fullsend tree

The first stage the night bot builds: the dungeon run playable offline, in the Sandbox, on the cards
that ship, from the start screen to the final boss. It is one `bot:fullsend` tree whose parts are
opened by hand (`bot/README.md`, Fullsend: "An issue split already"). Every part is built blind on
the tree's integration branch and reconciled once, so the parts meet only at the surface below,
which is frozen: a part that needs a name from another part uses it exactly as written here, without
waiting for that part, and the reconcile settles any seam.

### Surface (frozen)

**The engine (part A).** A seat's deck can arrive with some cards already upgraded:

```rust
// crates/engine/src/wire/, beside CreateGameOptions' other setup types; serde camelCase
pub struct DeckMark { pub index: usize, pub radiant: bool, pub upgrades: i32 }
// CreateGameOptions gains (JSON "deckMarks"), and ReplayInput gains the same field, so a fold replays it:
pub deck_marks: Option<PerPlayerOpt<Vec<DeckMark>>>,
```

At setup, before the opening draw, the card at `index` of that seat's deck list (as given to
`createGame`, before the shuffle) is dealt on its Radiant face when `radiant`, and has the Buff keyword
(the engine's Upgrade, `effects::upgrade`) applied `upgrades` times, its random changes drawn from the
game's `Rng`. A game created without `deckMarks` stores nothing and hashes as before.

**The run (part B).** `crates/run` gains, every type exported to TypeScript under a `ts` feature
(`apps/web/src/wire/generated/`, re-exported by `apps/web/src/wire/run.ts`, imported as `@jackioh/run`):

```rust
pub struct RunFightSetup {
    pub seed: String,               // the fight's game seed: "{run seed}-{row}-{col}"
    pub human_deck: Vec<String>,    // the run deck, in order
    pub human_handicap: Handicap,   // HUMAN_HANDICAP, deck_size = the deck's length, hero_health = the run's HP
    pub human_marks: Vec<DeckMark>, // each Radiant run card, and each card's buffs as upgrades
    pub enemy_deck: Vec<String>,
    pub enemy_handicap: Handicap,   // Easy's; Medium's with Looming Presence; Hard's with Overwhelming Presence;
                                    // deck_size = the deck's length, hero_health = the enemy's
    pub enemy: RunEnemy,
}
pub fn run_fight_setup(state: &RunState, content: &RunContent) -> Option<RunFightSetup>; // None outside RunPhase::Fight
pub fn run_fight_report(game: &ReplayInput, human: PlayerId) -> Result<RunAction, String>; // a RunAction::FightOver
pub fn run_catalog_content(catalog: &CardDefs, custom: &[CustomCard]) -> RunContent;
pub fn run_level_cost(level: usize) -> Option<i64>; // the Embers from `level` to the next; None at the top
// config.rs: RUN_STARTER_CARDS: [&str; 20], RUN_LEVEL_EMBERS
```

**The browser's bindings (part C).** One binding per function, JSON in and out, a refusal as
`{ "error": "…" }`, and in `apps/web/src/wasm/index.ts`:

```ts
type RunResult = { state: RunState } | { error: string };
export function runContent(customCards: CustomCard[]): RunContent; // run_catalog_content over the compiled-in catalog
export function runCheckSetup(setup: RunSetup, unlocks: RunUnlocks, content: RunContent): string | null;
export function runCreate(setup: RunSetup, unlocks: RunUnlocks, content: RunContent): RunResult;
export function runReduce(state: RunState, content: RunContent, action: RunAction): RunResult;
export function runFold(file: RunFile, content: RunContent): RunResult;
export function runFightSetup(state: RunState, content: RunContent): RunFightSetup | null;
export function runFightReport(game: ReplayInput, human: PlayerId): { action: RunAction } | { error: string };
export function runScore(state: RunState, content: RunContent): number;
export function runEmbers(score: number): number;
export function runMaxHp(state: RunState, content: RunContent): number;
export function runLevelCost(level: number): number | null;
export function runSeed(source: string): string;
export function runDailySeed(date: string, n: number): string;
```

**The page's store (part D),** `apps/web/src/dungeon/store.ts`:

```ts
export type DungeonMode = "sandbox" | "campaign";
export type DungeonProgress = { unlocks: RunUnlocks; embers: number };
export type RunRecord = {
  seed: string; daily: boolean; mode: DungeonMode; curses: string[]; cursePoints: number;
  rowReached: number; won: boolean; score: number; embers: number; timeMs: number; endedAt: string;
  stats: RunStats;
};
export type Dungeon = {
  content: RunContent;
  progress: DungeonProgress;
  run: RunState | null;
  startedAt: number | null;               // Date.now() when the run started, for the timer
  start(setup: RunSetup): string | null;  // the refusal, or null
  act(action: RunAction): string | null;  // the refusal, or null
  abandon(): void;
  buyLevel(): string | null;
};
export function useDungeon(): Dungeon;
export function readRunHistory(): RunRecord[];
```

**The screens.** `apps/web/src/dungeon/DungeonPage.tsx` (part D) renders `StartScreen` (D) while
`run` is null, `FightScreen` (E) in `RunPhase::Fight`, and `RunScreen` (F) in every other phase:

```ts
// apps/web/src/dungeon/fight/FightScreen.tsx (E)
export function FightScreen(props: { dungeon: Dungeon }): ReactElement;
// apps/web/src/dungeon/map/RunScreen.tsx (F)
export function RunScreen(props: { dungeon: Dungeon }): ReactElement;
// apps/web/src/dungeon/stats/DungeonStats.tsx (G), the page at /dungeon/stats
export function DungeonStats(): ReactElement;
```

**The practice worker (part E).** `PracticeStartConfig` gains `run?: RunFightSetup`: when set, its
decks, handicaps, deck marks and seed replace `deck`, `difficulty`'s handicap and the dealt AI deck.
A new request `{ type: "runReport" }` answers `{ type: "runReport", action: RunAction }` or
`{ type: "runReport", error: string }` from `runFightReport` over the finished game. `Game` gains an
optional `heroAside?: { own?: ReactNode; enemy?: ReactNode }`, drawn beside each hero.

Each part keeps its test ids in its own directory's `testids.ts`, prefixed `dungeon-`. Words go
through `t()`; only part D edits `apps/web/src/i18n/zh-CN.json`.

### Behaviors

Each is proved by a test that names it in a comment (`// B7`).

- **B1** A deck mark with `radiant` deals that card on its Radiant face, from setup on.
- **B2** A deck mark with `upgrades: n` applies the Buff keyword (Upgrade) n times to that card at setup;
  a fold of the same setup and log reaches the same hash.
- **B3** A game created without `deckMarks` hashes and replays as before: the hotseat fixture still folds
  to its hash.
- **B4** A mark whose index is past its seat's deck refuses `createGame` with a reason.
- **B5** `run_catalog_content` pools every non-token card of the sets that ship and the custom cards
  given, and no boss card while the Boss set has none.
- **B6** `RUN_STARTER_CARDS` holds 20 Training Grounds Commons and Rares, each in the catalog.
- **B7** `run_fight_setup` answers only in a fight; the human's handicap holds the run's HP and the deck's
  length; the marks hold each Radiant card and each card's buffs.
- **B8** The enemy's handicap is Easy's, Medium's with Looming Presence or Hard's with Overwhelming
  Presence, with its deck's length and its hero health.
- **B9** A fight's seed is the run's seed, row and column, so one run meets one deal.
- **B10** `run_fight_report` folds the finished game and answers `FightOver`: the winner, the human's final
  health, the damage dealt to the enemy's hero and units (`damage` events), the healing done on the
  human's side (`healed` events), the cards the human drew and the turns.
- **B11** `run_level_cost` gives each level's Embers and nothing past the ladder's top.
- **B12** Every run binding answers JSON in and out with refusals as `{ error }`, through the typed
  functions in `apps/web/src/wasm/index.ts`, in both browser modules.
- **B13** `/dungeon` opens the Dungeon Run screen; the Play hub and the command menu link it.
- **B14** The screen offers the Sandbox and the Campaign; the Campaign shows, disabled, as coming with
  accounts.
- **B15** The seed is the daily run's (today's date and the day's run number), a random one or one typed
  (6 capital letters, refused otherwise).
- **B16** The starting deck builder shows the level's slots, offers only unlocked cards, refuses what
  `runCheckSetup` refuses with its reason, and shows the artifacts' points out of 8 and at most 3.
- **B17** Curse tiers can be stacked, their points shown.
- **B18** Embers buy the next level at `runLevelCost`.
- **B19** A run in progress survives a reload: the store keeps its `RunFile` on the device and folds it
  back.
- **B20** A finished run adds what it extracted to the unlocks, its Embers to the balance and a
  `RunRecord` to the history.
- **B21** In the Sandbox, the custom cards the creator saved join the pool and the fights.
- **B22** A Fight node starts a practice game from `runFightSetup`, the AI playing the enemy, its name
  shown.
- **B23** Both sides' artifacts show in a tray beside their hero; a button opens each one's name and text.
- **B24** A timer shows the run's time on screen.
- **B25** When the game ends, the page asks the worker for `runReport` and sends its `FightOver` to the run.
- **B26** A lost fight ends the run, and the run-over screen follows.
- **B27** The map shows the whole current act, each node's kind, each boss's preseeded artifact and where
  the player stands; only the next row's nodes can be chosen.
- **B28** The reward screen offers the artifact (take or leave) before the four packs; the pack screen
  keeps exactly `keep` cards, or none with a Skip passive.
- **B29** Extraction: a card after a mini-boss, an artifact after the final boss, the boss's own refused.
- **B30** Shop, campfire, event and shrine screens send their actions and show the run's refusals; Coins,
  Tickets and HP are always on screen.
- **B31** The run-over screen shows the score, the Embers it pays, what was extracted and the stats.
- **B32** Leaderboards on the device rank the deepest row, the most curse points, the top score and the
  fastest win; each entry opens that run's stats.
- **B33** `spec/15-dungeon-run.md` states the mode's rules as this plan decided them, `spec/README.md`
  maps it, and `cargo jackioh spec check` passes.

### Out of scope

The Campaign (accounts' unlocks and Embers, checked runs, Jlore rewards, online leaderboards); ghosts
from real players; Combat artifacts and curses acting inside a fight; the extended campaign; the
designer's content; an e2e spec; and CI holding the run's generated types to the Rust, which needs a
person to add `-p jackioh-run --features ts` to `ci.yml` (the bot cannot edit `.github/`).

### The parts

| Part | Title | Owns | Behaviors | Difficulty |
| --- | --- | --- | --- | --- |
| A | A deck's Radiant and buffed cards at a game's setup | `crates/engine/`, its ruling in its own block, the engine's regenerated `apps/web/src/wire/generated/` files | B1–B4 | hard |
| B | The run plays real cards: catalog content, fight setup and fight report | `crates/run/`, `crates/tools/tests/acceptance.rs`, `CLAUDE.md`, `apps/web/src/wire/run.ts`, the run's generated types, `apps/web/tsconfig.json`, `apps/web/vite.config.ts` | B5–B11 | hard |
| C | The run in the browser's WebAssembly modules | `crates/wasm/`, the root `Cargo.toml`, `scripts/build-wasm.sh`, `apps/web/src/wasm/` | B12 | medium |
| D | The Dungeon Run screen: route, start, unlocks and the device store | `apps/web/src/dungeon/` (but `fight/`, `map/`, `stats/`), `main.tsx`, `routes/chunks.tsx`, `net/navigate.ts`, both `vercel.json`, `routes/play.tsx`, `routes/commandEntries.ts`, `i18n/zh-CN.json` | B13–B21 | medium |
| E | The run's fights: practice games, the artifact tray and the timer | `apps/web/src/dungeon/fight/`, `apps/web/src/practice/`, `game/Game.tsx`, `game/Board.tsx`, `game/Hero.tsx` | B22–B26 | hard |
| F | The map, the tiles and the run's end | `apps/web/src/dungeon/map/` | B27–B31 | medium |
| G | The run's leaderboards and stats on the device | `apps/web/src/dungeon/stats/` | B32 | medium |
| H | The dungeon run's spec section | `spec/15-dungeon-run.md`, `spec/README.md` | B33 | medium |

No part waits for another: each reads the others only through the surface above.

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
