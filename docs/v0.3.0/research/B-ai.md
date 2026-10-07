# Research B: the practice AI, the ladder, the bot machines and Devin (for v0.3.0, issue #306)

Repository: `/home/user/JackiOh`, checkout at `91cc43c` (branch `claude/relaxed-archimedes-i5fdah`;
shallow clone of 54 commits). `origin/main` is one commit ahead at `f13cb03` (#388, a fuzz-gate change
that does not touch `packages/ai`). History older than the clone was read through the GitHub API
(`Jlore-Studios/JackiOh`). Nothing in the repository was modified.

Headline findings, for the planner:

1. **The only AI that has ever passed its gates is `packages/ai` at `91cc43c`** (CI run 37497560121,
   green, merged gates 94/100 vs random, 35/50 vs greedy, 47/50 Hard vs Easy). The ladder
   (`ladder/`) never produced a champion: `ladder/champions/1`, `champions/2`, `hall_of_fame/`,
   `history/` and `proposals/` hold only `.gitkeep`, and its phases 4-5 workflow
   (`.github/workflows/generation.yml`) was never written.
2. **The AI is not MCTS.** It is a lethal solver plus a turn-level beam search on K=3
   determinizations, 600 `reduce` calls per decision, with a rule-based one-turn opponent reply and a
   hand-weighted static evaluation. It calls about 40 engine symbols, `reduce`/`legalActions`/`unitView`
   hundreds of times per decision, so a Rust AI needs a Rust engine (or an in-process engine); a
   per-call JSON bridge to the TS engine would be far too slow.
3. **The shadow ban today is 11 Core cards** (`packages/ai/src/shadowBan.ts`), from a 2026-09-27 sweep
   over the 100 Core cards only; `SHADOW_WATCH` is `{}`. The 168 Classic/Classic+ cards were never
   swept of record (issue #65 left it undone).
4. **Two lane counts exist on AWS today:** the night box (`m7i-flex.large`, 2 vCPU) runs 10 runners
   (gpt 1, agy 1, muse 2, devin 6) capped by `machine_parallel: 7`; the training box (`m7i.xlarge`,
   `devin-train`, 1 lane) is **not built** and `devin-train` is `"enabled": false`. When it is enabled,
   `bot/harness/plan.py` reserves its lanes, giving "6 night + 1 training".
5. **Devin as a training builder conflicts with the easy rule.** Devin is the weak tier, may only build
   `difficulty:easy` items, and the easy rule (`bot/harness/easy.py`, `.harness/config.json` `easy`)
   says "Never easy: ... engine or AI work" and puts `packages/ai/src/` and `packages/engine/src/` off
   limits, max 10 files / 400 lines. "Always running" also conflicts with the 30-minute idle power-off,
   the starter that wakes the box only for queued `bot-night.yml`/`triage.yml` jobs, and the 330-minute
   job budget.

---

## 1. `packages/ai/src`: files, API, algorithm

### 1.1 Files (`wc -l`)

| File | Lines | Responsibility |
|---|---:|---|
| `packages/ai/src/index.ts` | 24 | Barrel: `export *` from every module below (every exported name is unique package-wide). |
| `packages/ai/src/types.ts` | 65 | `SearchBudget`, `AiOptions`, `DecisionReason`, `SearchStats`, `Decision`, `NodeCounter`. |
| `packages/ai/src/config.ts` | 381 | Every AI number: `AI_BUDGET`, `AI_GATE_BUDGET` (= `AI_BUDGET`), `AI_SEARCH`, `AI_EVAL`, `EvalWeights`, `GREEDY_EVAL` (frozen copy for the greedy baseline), `AI_REPLY`, `AI_MULLIGAN`, `GREEDY_MULLIGAN`, `AI_DETERMINIZE`, and the R645 emote-persona tables (`EMOTE_TRIGGERS`, `EMOTE_REPLY_KEYS`, `AI_EMOTE`, `AI_PERSONAS`, `PersonaName`, `PersonaSpec`). Exported also as `@jackioh/ai/config`. |
| `packages/ai/src/observe.ts` | 373 | The only reader of a true `GameState` (R185): `redact(state, seat)` (seed/cursor/nonce log blanked; hidden cards become `HIDDEN_DEF_ID = "ai:hidden"` placeholders; drops `boardHistory`, `opening`, `glitchBoards`, the other seat's `lastBoards`; strips answer keys), `hiddenInstanceIds`, `aiToAct(state, seat)`, `unansweredDrawOffer`. |
| `packages/ai/src/determinize.ts` | 166 | `determinize(publicState, seat, rng, {matchShownCost?})`: one concrete world. New seed `ai:<rng.int(2^31)>`; backrow placeholders sampled from Trap/Field Trap pool (minus aura traps that would change shown unit stats, R602; matching the shown cost, R762); opponent hand + library from `query({excludeDefId:["core-098"]})` minus cards the opponent has shown; own library's opponent cards likewise, then shuffled. |
| `packages/ai/src/candidates.ts` | 195 | `actionKey` (sorted-key JSON) and `candidateActions(state, seat)`: `legalActions` minus `concede`/`offerDraw`/`answerDraw`/`mulligan`, play lane variants collapsed to lowest+highest lane, then move ordering: face attacks, "kills and survives" trades, plays/`activatePower`/`activate` round-robin by source (highest cost first), prompt answers, other attacks, position switches, `endTurn` last. |
| `packages/ai/src/simulate.ts` | 212 | Node accounting and one simulated step: `createNodeCounter(limit, shouldStop)`, `createSubCounter`, `simulate` (one `reduce` with nonce `sim:<n>` = 1 node; the opponent's prompts auto-answered with `legalActions(...)[0]`, up to `AI_SEARCH.maxAutoAnswers` = 8, 1 node each), `lineStatus` (open/passed/yielded/over), `closeLine`, `staticScore` (passed turn pays `AI_EVAL.unspentMana` per unspent crystal), `terminalScore`, `searchSignature` (dedupe key). |
| `packages/ai/src/lethal.ts` | 247 | `findLethal(dets, seat, counter, limit)`: depth-first walk in move order for `lethalQuickNodes` (40) nodes, then best-first by `readyGap` trying `lethalWidth` (60) moves per expanded position, depth <= `lethalMaxDepth` (10); a line counts only if it wins on every determinization. Also `readyGap`. |
| `packages/ai/src/search.ts` | 143 | `beamSearch(det, seat, counter, budget)` (turn-level beam, returns complete lines best first) and `scoreLine` (replay a line on another determinization, end the turn, score statically or after the reply). Type `Line`. |
| `packages/ai/src/reply.ts` | 234 | The opponent's one-turn reply used to score finalists: plays a card the line itself put in its hand if its evaluation likes it, else the best static trade/face attack, else `endTurn`; never plays unseen (sampled) cards. `hiddenCardIds`, `simulateReply`, `replyScore` (scored with `next = "seat"`). |
| `packages/ai/src/evaluate.ts` | 240 | Static evaluation `evaluate(state, seat, next="enemy", w=AI_EVAL)`, plus `unitWorth`, `faceThreat`, `damagePastTaunts`, type `NextSwing`. |
| `packages/ai/src/mulligan.ts` | 21 | `mulliganKeep(state, seat, keepMaxCost=3)`: keep hand cards with `queryCost <= 3`. |
| `packages/ai/src/decide.ts` | 225 | The entry point `decide(state, seat, options)` (algorithm in 1.3). Never throws. |
| `packages/ai/src/deck.ts` | 242 | `buildAiDeck(rng, size, options)`, `AI_DECK`, `costBucket`, `curveTargets`, types `CostBucket`, `AiDeckOptions`. |
| `packages/ai/src/shadowBan.ts` | 53 | `SHADOW_BAN` (defId -> reason), `SHADOW_BAN_IDS` (sorted keys), `SHADOW_WATCH` (`{}`). |
| `packages/ai/src/baselines.ts` | 66 | `randomAction` (SPEC §10.7 policy: `subsystems.chooseAction` with `AI_SKIPPED_ACTIONS`, 10% end-turn chance `AI_END_TURN_PROBABILITY`) and `greedyAction` (one-ply on one determinization with frozen `GREEDY_EVAL`, `matchShownCost: false`). |
| `packages/ai/src/match.ts` | 234 | `playMatch(config, hooks)` (whole deterministic game between controllers `ai`/`greedy`/`random`; controller rng `${seed}:ctl:<seat>`, nonce `m<n>`; `AI_MATCH.maxActions` = 3000, not exported) and `playAiTurn` (`AI_TURN_MAX_ACTIONS` = 60). Types `SeatController`, `MatchConfig`, `MatchHooks`, `MatchRecord`, `AiTurnResult`. |
| `packages/ai/src/gate.ts` | 249 | Quality gates: `Matchup`, `AI_GATE`, `AI_TUNING_SERIES` (`"tune"`), `binomialTail`, `gateNeeded`, `gameConfig`, `runGate`, `runGateGames`, `gateShardGames`, `GateReport`. |
| `packages/ai/src/sweep.ts` | 450 | The R186/R390 shadow-ban sweep: `AI_SWEEP`, `sweepCard`, `sweepAtRisk`, `atRiskIds`, `pass2KeepOut`, `pass2Stats`, `sweepVerdict`, `sweepFlags`, `halfFlags`, `banFlags`, types `SweepFlag`, `SweepStats`, `SweepResult`, `SweepSuspect`, `SweepPass2`, `SweepVerdict`. |
| `packages/ai/src/devRun.ts` | 80 | R378 development runs: `AI_DEV_RUN` (`games: 200`, `series: "dev"`), `devGameConfig`, `devRecordId`, `devGameRecord`. |
| `packages/ai/src/personas.ts` | 362 | R645 emote personas (cosmetic; never reaches `reduce`, the log or a decision): `pickPersona`, `rollForTrigger`, `replyKeyOf`, `rollForReply`, `createEmotePersona`, types `AiEmote`, `EmotePersona`. |
| **total src** | **4,128** | (`packages/ai` incl. scripts/tests/README/package.json: 12,611) |

`packages/ai/package.json`: exports `"."` -> `./src/index.ts`, `"./config"` -> `./src/config.ts`; scripts
`arena-bridge`, `stats`, `sweep`; deps `@jackioh/cards`, `@jackioh/engine`, `@jackioh/shared`.
`src/` never imports `@jackioh/cards` (callers `registerAll()` first; `packages/ai/test/setup.ts` does it
for the test project). ESLint enforces purity (no `Math.random`, `Date`, timers, `process`, `fetch`,
`node:*`, async).

### 1.2 Public API (everything `index.ts` re-exports), and who uses it outside the package

The entry point is `decide(state: GameState, seat: PlayerId, options: AiOptions): Decision | null`
(`packages/ai/src/decide.ts:91`). `null` when `!aiToAct(state, seat)`.

```ts
type AiOptions = { rng: Rng; budget?: SearchBudget /* default AI_BUDGET */; shouldStop?: () => boolean };
type Decision = { action: ActionBody; reason: "forced"|"mulligan"|"draw-offer"|"lethal"|"prompt"|"search"|"fallback";
                  line: ActionBody[]; stats: SearchStats };
type SearchStats = { nodes; determinizations; lines; simErrors; stoppedBy: "exhausted"|"budget"|"clock"; score };
```

Per-module export list: types (6 types); config (see table); observe `HIDDEN_DEF_ID hiddenInstanceIds
redact unansweredDrawOffer aiToAct`; determinize `DeterminizeOptions determinize`; evaluate `unitWorth
faceThreat damagePastTaunts NextSwing evaluate`; candidates `actionKey candidateActions`; simulate
`LineStatus CountingNodeCounter createNodeCounter createSubCounter lineStatus simulate closeLine
staticScore terminalScore searchSignature`; lethal `readyGap findLethal`; search `Line beamSearch
scoreLine`; reply `hiddenCardIds simulateReply replyScore`; mulligan `mulliganKeep`; decide `decide`;
deck `CostBucket AiDeckOptions AI_DECK costBucket curveTargets buildAiDeck`; shadowBan `SHADOW_BAN
SHADOW_BAN_IDS SHADOW_WATCH`; baselines `randomAction greedyAction`; match `SeatController MatchConfig
MatchHooks MatchRecord AiTurnResult playMatch playAiTurn`; personas (see table); gate (see table); sweep
(see table); devRun `AI_DEV_RUN DevRunOptions devGameConfig devRecordId devGameRecord`.

External consumers (what a Rust port must keep serving or replace):

| Consumer | Imports |
|---|---|
| `apps/web/src/practice/core.ts:40` | `AI_BUDGET, aiToAct, buildAiDeck, decide, SearchBudget` (runs in the practice Web Worker; wall clock cap `PRACTICE_AI_CLOCK_MS = 1500` in `apps/web/src/practice/config.ts:65`, passed as `shouldStop`) |
| `apps/web/src/practice/emotes.ts:23-24` | `createEmotePersona, pickPersona, AiEmote, EmotePersona`; `AI_EMOTE` from `@jackioh/ai/config` |
| `apps/web/src/tutorial/harness.ts:19` | `AI_GATE_BUDGET, SearchBudget` |
| `apps/server/src/match/engine.real.ts:31` | `buildAiDeck` (All Random deals, R258, `banned: []`) |
| tests: `apps/web/src/practice/*.test.ts`, `apps/web/src/tutorial/**/*.test.ts`, `apps/web/src/routes/practice.test.tsx` | various |

### 1.3 Options and budget model

`AI_BUDGET` (`config.ts:8`), the same at every difficulty (R180), and `AI_GATE_BUDGET = AI_BUDGET`:

| Field | Value | Meaning |
|---|---:|---|
| `nodes` | 600 | `reduce` calls the whole decision may make (lethal, beam, auto-answers, replies) |
| `lethalNodes` | 150 | of those, the most the lethal solver may spend |
| `determinizations` | 3 | K sampled worlds per decision |
| `beamWidth` | 4 | open lines kept per depth |
| `rootBranching` | 20 | children at the root (endTurn always added) |
| `branching` | 6 | children per open line below the root |
| `maxDepth` | 8 | actions per planned line |
| `finalists` | 3 | first actions re-scored on the other determinizations |

`AI_SEARCH`: `zoneVariants 2`, `maxAutoAnswers 8`, `lethalMaxDepth 10`, `lethalQuickNodes 40`,
`lethalWidth 60`, `linesPerAction 2`, `probeSeed "ai:probe"`. `AI_REPLY`: `maxSteps 12`,
`reserveSteps 5`, `knownPlays 8`, `facePerDamage 1`, `chipPerDamage 0.3`.

`shouldStop` is polled by `NodeCounter.take()` before every node (`simulate.ts:17`); true sets
`stoppedBy: "clock"` and the decision answers with the best line so far. Budgets count nodes, not time,
so (state, seed, budget) -> the same `Decision`.

### 1.4 The search algorithm (`decide.ts`), step by step

Not MCTS, not RL. A deterministic, seeded, hand-tuned search:

1. `aiToAct` and `redact` are the only reads of the true state (R185).
2. An unanswered draw offer -> `{type:"answerDraw", accept:false}` (R188).
3. Mulligan (R265) -> `mulliganKeep` (cost <= 3), answered at once.
4. Forced: candidates listed on a throwaway determinization (`createRng("ai:probe")`); one candidate is
   played without search (`options.rng` untouched).
5. K = 3 determinizations drawn from `options.rng` (the only rng draws). Node counter of 600.
6. Lethal solver (`findLethal`, <= 150 nodes) must win on all 3 worlds -> reason `lethal`.
7. Beam search on determinization 0 with `remaining - reserve` nodes, where `reserve =
   min(remaining/2, finalists x (linesPerAction x reserveSteps + (K-1) x (maxDepth+1+reserveSteps)))` =
   `min(remaining/2, 114)` at the default budget. Beam: per depth expand `rootBranching`/`branching`
   candidates (+ endTurn) of each of `beamWidth` open lines, simulate each, dedupe by
   `searchSignature`, keep the best `beamWidth` by `evaluate`; lines that pass the turn/yield/end are
   scored by `staticScore`.
8. Up to `finalists x linesPerAction` = 6 best lines (<= 2 per first action) are re-scored after the
   opponent's rule-based reply (`replyScore`) on world 0; best line per first action; top 3 first
   actions re-scored with `scoreLine(..., reply=true)` on worlds 1..K-1; best mean wins.
9. Only `line[0]` is played; the caller asks again (re-plans after every action).
10. Any failure -> `fallback` (endTurn if legal, else first candidate); counted in `simErrors`.

### 1.5 Evaluation (`evaluate.ts:206`, weights `AI_EVAL` in `config.ts`)

`heroSide(seat) - heroSide(opp) + material(seat) - material(opp) - threat + pressure + closing - race`:
hero health concave (`heroHealth x sqrt(h x HERO_HEALTH)`, -1e6 at <= 0), armor 0.6/pt; units via
`unitView` (attack 1.2, health 1, armor 0.8, keyword table, Defense-Position attack x0.5, Spell Damage
0.5, Brittle discount, Animated backrow share 0.5); hand cards `1 + 0.3 x min(cost,6)` (+0.5 Radiant,
cost-mod delta 0.5); unseen opponent hand card valued as cost 2; readable backrow `1.5 + 0.8 x cost`,
unreadable enemy backrow 2; library 0.1/card up to 10; threat (enemy `faceThreat` 0.4/pt or 150 if
lethal) and pressure (0.5/pt or 20); closing weight 3 per enemy-hero damage ramping from turn 10 to
`TURN_CAP_PLAYER_TURNS` (60); race term 1 x enemy health. Win = `1_000_000 - turn`. `GREEDY_EVAL` is the
frozen copy the greedy baseline uses (so tuning `AI_EVAL` never moves the yardstick).

### 1.6 Discover / scorer

The AI has **no Discover logic and does not call the scorer**: `grep -i "discover\|scorer"
packages/ai/src` is empty. Discover-style prompts (Zephyrs, §10.7, R29) are ranked inside the engine by
`packages/engine/src/subsystems/scorer.ts` (503 lines, a dry-run of each candidate card); the AI just
sees the resulting `answer` actions in `legalActions` and searches them like any other candidate. Issue
#70 measured the scorer's `perfectHand`/`dryRun` at ~19% of `reduce` time with all three sets, i.e. it
is a big part of the per-node cost of simulation.

### 1.7 Difficulty tiers (R180-R184)

`packages/engine/src/config.ts:230` `AI_DIFFICULTY` (the AI never reads a difficulty; only the seat's
resources differ):

| Tier | deckSize | manaBonus | manaCap | extraOpeningCards | extraDrawsPerTurn |
|---|---:|---:|---:|---:|---:|
| easy (`HUMAN_HANDICAP`) | 20 | 0 | 4 | 0 | 0 |
| medium | 25 | 1 | 5 | 1 | 0 |
| hard | 30 | 1 | 7 | 1 | 1 |
| `AI_TUTORIAL` (`config.ts:243`, R290) | 12 | 0 | 3 | 0 | 0, `heroHealth: 20` |

`buildAiDeck` takes `manaCap` (curve shift + uncastable penalty) and `size` from the handicap.

### 1.8 `redact` / `determinize` (R185)

See the table rows. Key properties: two states that differ only in hidden cards redact identically;
hidden instance ids are kept (a deck position, not an identity); `determinize` never samples
`core-098` (Heroic Power, R43) into a hidden slot; `greedyAction` keeps the pre-R762 sampler
(`matchShownCost: false`). Tests: `observe.test.ts` and the `redact-*.test.ts` files.

### 1.9 Deck building (`buildAiDeck`, `deck.ts`)

`buildAiDeck(rng, size, {banned = SHADOW_BAN_IDS, include, theme, manaCap = MAX_MANA(4), boost})`:
weighted sampling without replacement of distinct non-token ids from `query()` (all sets, R380).
`AI_DECK`: curve `{0-1: .30, 2: .33, 3: .22, 4+: .15}` shifting 0.025 per crystal above 4 toward 4+,
`minUnitShare 0.45`, `themeChance 0.35`, `minThemeSize 6`, `themeBoost 4`, `themeMinShare 0.3`,
`curveBoost 3`, `curveOverflow 0.15`, `unitBoost 2.5`, `uncastable 0.05`, `costSlack 1`, `minPool 45`
(the unbanned pool must keep >= 45 cards). `banned: []` for a human's random deck / All Random.

### 1.10 `shadowBan.ts` (R186)

Format: `export const SHADOW_BAN: Readonly<Record<string, string>>` mapping defId to a reason that starts
`"<flag>: <tier>: ..."`; `SHADOW_BAN_IDS = Object.keys(SHADOW_BAN).sort()`; `SHADOW_WATCH:
Readonly<Record<string,string>> = {}`. A long header comment records the sweep of record.

**Banned now: 11 cards, all Core, all `neverPlayed`:** `core-042, core-051, core-055, core-057,
core-076, core-078, core-082, core-091, core-093, core-094, core-099`. Sweep of record: 2026-09-27, 100
Core cards, Easy and Hard, 8 seeds per card/tier, budget `AI_GATE_BUDGET`. Last code change to the file:
`d9a04ad` (2026-10-01, the R390 two-pass sweep code; the table content is from `1005c50`, patch v0.1.1,
2026-09-27). The catalog now has 268 non-token cards (core 100, classic 90, classicplus 78) and 50 tokens;
the 168 Classic/Classic+ cards have no sweep of record (issue #65's unchecked box). The ban governs AI
deck building only (it is not §9.4 L6's server ban). `ladder/README.md` notes its own "shadowban"
(use rate) is a different list.

### 1.11 `sweep.ts` (R186, R390, R600, R601)

`AI_SWEEP`: `tiers ["easy","hard"]`, `seedsPerCard 8`, `decisionMs 2000`, `maxActions 600`,
`minAffordableTurns 3`, `selfHarmDelta -40`, `minHarmPlays 4`, `seedsPerCardAtRisk 24`, `atRiskBoost 4`,
`banAffordableTurns 6`, `banHarmPlays 8`.

- **Pass 1** (`sweepCard`): per card and tier, 8 games of the AI (tier handicap, deck with the card
  forced in, other banned cards out) vs the greedy baseline (Easy, unbanned deck), seeds
  `sweep:<tier>:<id>:<n>`, AI seat alternating. Measures `errors` (throw, refused action, fallback),
  `timeouts` (decision > 2000 ms or 600 actions), `affordableTurns`, `plays`, eval delta per play.
  Flags: `error`, `timeout`, `neverPlayed` (>= 3 affordable turns, 0 plays), `selfHarm` (>= 4 plays,
  mean delta < -40).
- **At risk** (`atRiskIds`, `halfFlags`): half-strength conditions, or already banned, or on
  `SHADOW_WATCH`.
- **Pass 2** (`sweepAtRisk`): each at-risk card 24 more games per tier, seeds
  `sweep2:<tier>:<id>:<n>`, at-risk filler weight x4, `pass2KeepOut` keeps `error`/`timeout` bans out
  of filler; counts every at-risk card dealt.
- **Verdict** (`sweepVerdict`): `neverPlayed` needs 6 affordable turns and no play in pass 2 at that
  tier; `selfHarm` 8 plays (R601); `error`/`timeout` ban from forced games; suspects listed;
  `SHADOW_WATCH` entries (R600).
- Cost: pass 1 alone is 268 cards x 2 tiers x 8 = 4,288 games.

### 1.12 Stats (R376-R378)

`devRun.ts` + `scripts/stats.ts`: AI-vs-AI All Random games (both decks `buildAiDeck(createRng(
`${seed}:p1-deck`), DECK_SIZE, {banned: []})`, no handicap, `AI_BUDGET`), seeds `${series}:${n}`; each
finished game -> `GameRecord {id: "dev:<patch>:<seed>", source: "dev", mode: "random", patch, pilots:
{p1:"ai", p2:"ai"}, game: summarizeGame(...)}` (types `GameRecord`, `DEV_RECORD_ID_PREFIX`,
`cardStats`, `formatCardStats` from `@jackioh/shared`, `packages/shared/src/stats.ts`). Loaded into
Postgres by `pnpm --filter @jackioh/server stats:import <file>`, read by `stats:cards --source=dev
--patch=<v>`. This is the existing "log stats while playing" mechanism the training lanes can reuse.

### 1.13 Engine/shared symbols the AI imports (all of `packages/ai/src`)

`@jackioh/engine` (39): `AI_DIFFICULTY, ANSWER_KEY, CardInstance, DECK_SIZE, GameState, Handicap,
MAX_MANA, Rng, SETUP_WORK, UnitView, activeBrittleCount, activeUnitsOf, animatedKindOf,
announcedFaceDownTo, beginGame, canAttack, cannotAttack, cloneState, createGame, createRng,
effectiveCost, findDef, findInstance, fold, handicapOf, hasExertion, hashState, heroArmorOf,
legalActions, mulliganPromptFor, ownCost, query, queryCost, reduce, scriptsFor, seatToAct, subsystems,
summarizeGame, unclampedAttack, unitView, zoneCards`. `subsystems.*` members used: `AI_SKIPPED_ACTIONS,
abilitiesOf, chooseAction, fusedIngredients, powerAbilityOf, projectedHeroDamage, syncFusedScripts`.

`@jackioh/engine/config`: `Difficulty, HERO_HEALTH, TURN_CAP_PLAYER_TURNS`.

`@jackioh/shared`: `Action, ActionBody, CardDef, DEV_RECORD_ID_PREFIX, EmoteId, GameEvent, GameRecord,
PLAYER_IDS, PlayerId, PlayerView, SideView, Tag, emoteGate, hasKeyword, opponentOf`.

Hot path per node: `reduce`, `legalActions` (inside `candidateActions`, `simulate` auto-answers,
`scoreLine`), `unitView`/`activeUnitsOf` (inside `evaluate`, `faceThreat`), `cloneState` (inside
`reduce`, `redact`, `determinize`). Engine size for reference: `packages/engine/src` 36,999 lines;
`packages/cards/src` 17,224 lines (319 card scripts). A Rust AI is coupled to all of it.

---

## 2. `packages/ai/test` and `packages/ai/scripts`

### 2.1 Tests (35 files)

| File | Lines | Purpose |
|---|---:|---|
| `_shard.ts` | 62 | CI sharding helper: `JACKIOH_AI_GATE_SHARD=k/K`, `gamesToPlay`, `writeShard` to `JACKIOH_AI_GATE_OUT` (default `ai-gate-shards/`). |
| `_support.ts` | 212 | Shared scenario builders (real engine + catalog). |
| `setup.ts` | 7 | Vitest setupFile: `registerAll()`. |
| `activate.test.ts` | 200 | Activate abilities / Heroic Power in candidates and search (B3.2, R384). |
| `answer-key.test.ts` | 90 | R465: `redact` strips a multiple-choice answer key. |
| `arena-bridge.test.ts` | 98 | Drives `scripts/arena-bridge.ts` over stdio: one game end to end + an error reply (#55). |
| `decide.test.ts` | 403 | `decide` short-circuits: forced, not its turn, mulligan, draw offers (R188). |
| `deck.test.ts` | 275 | `buildAiDeck` (B22, B23, R184, R380, R390 boost). |
| `determinize-shown-cost.test.ts` | 105 | R762 shown-cost trap sampling. |
| `dev-run.test.ts` | 97 | R378 dev runs. |
| `evaluate.test.ts` | 223 | Evaluation rankings (B13). |
| `evaluate-v020.test.ts` | 258 | v0.2.0 terms (Brittle, Animated, Spell Damage, ...). |
| `gate-random.test.ts` | 152 | Gate B28/B31 + `gateNeeded` rule tests. |
| `gate-greedy.test.ts` | 172 | Gate B29/B31; greedy keeps frozen weights. |
| `gate-hard-easy.test.ts` | 123 | Gate B30/B31. |
| `gate-perf.test.ts` | 145 | Gate B42: decision time vs a calibrated yardstick; two wide-board states. |
| `lethal.test.ts` | 188 | The two lethal walks. |
| `match.test.ts` | 294 | `playMatch` determinism, baselines (B26, B27). |
| `match-refusal.test.ts` | 120 | `playMatch` with misbehaving controllers. |
| `observe.test.ts` | 663 | `redact`/`determinize` (R185, B9-B12). |
| `observe-instance-data.test.ts` | 43 | Hidden instance data dropped. |
| `personas.test.ts` | 779 | R645 emote personas. |
| `prompts-v020.test.ts` | 267 | New prompt kinds (number/answer/cell/reward/pick). |
| `puzzles.test.ts` | 273 | Hand-built puzzles P1-P17 (B17-B19, The Coin). |
| `redact-announce.test.ts` | 40 | R448. |
| `redact-backrow-piles.test.ts` | 49 | R447. |
| `redact-board-history.test.ts` | 30 | R419. |
| `redact-fusion.test.ts` | 51 | R77, R179. |
| `redact-last-boards.test.ts` | 25 | R417. |
| `redact-live-face-down.test.ts` | 96 | R602, R403. |
| `reply.test.ts` | 170 | Opponent reply. |
| `search.test.ts` | 337 | `candidateActions`, search contract (B14, B15). |
| `shadowBan.test.ts` | 432 | The ban table and the sweep (B24, B25, R390, R600, R601). |
| `surface.test.ts` | 319 | Building blocks vs `docs/polish/3-ai.md` §Surface. |
| `tutorial-tier.test.ts` | 240 | R290 tutorial tier vs greedy. |

`packages/ai/vitest.config.ts`: project `ai`, `include: ["test/**/*.test.ts"]`, `setupFiles:
["test/setup.ts"]`. Root `package.json:12` `"ai:gate": "JACKIOH_AI_GATE=full vitest run --project ai
gate-"`, `:13` `"ai:gate:merge": "tsx packages/ai/scripts/gate-merge.ts"`, `:14` `ai:stats`, `:15`
`ai:sweep`.

### 2.2 The quality gates (`packages/ai/src/gate.ts`, `AI_GATE`)

| Matchup | Subject | Opponent | Handicaps | Full games | Smoke (`pnpm test`) | `briefRate` | `measuredRate` | `gateNeeded` full / smoke |
|---|---|---|---|---:|---:|---:|---:|---|
| `ai-vs-random` | AI at `AI_GATE_BUDGET` | `randomAction` (§10.7) | Easy both | 100 | 20 | 0.95 | 0.945 | 91 / 17 |
| `ai-vs-greedy` | AI | `greedyAction` | Easy both | 50 | 20 | 0.70 | 0.68 | 28 / 10 |
| `hard-vs-easy` | AI on Hard | AI on Easy | Hard vs Easy | 50 | 20 | 0.80 | 0.913 | 40 / 16 |

- Seeds: `${AI_GATE.seedSeries}:${matchup}:${n}`, `seedSeries = "gate:v3"` (re-rolled from `gate:v2`
  in `54a3002`, 2026-10-03, because R635 re-dealt setup; gate:v3 first run 97/100, 41/50, 48/50).
  Subject is p1 on odd n. Decks `buildAiDeck(createRng(`${seed}:deck:${seat}`), handicap.deckSize,
  {manaCap})` with the shadow ban for both seats. Every game is folded back (`fold`) and must hash
  equal, with no rejected/thrown/fallback (B31).
- `gateNeeded(m, n)` = largest k with `P(Binomial(n, measuredRate) >= k) >= 1 - falseAlarm(0.05)`,
  capped at `ceil(briefRate x n)` ("proposed, pending the user's acceptance", SPEC §9.9).
- Only wins count; turn-cap draws are reported (`turnCapDraws`).
- Perf gate: `maxDecisionMs 1500`, `perfSmokeGames 1`, `perfFullGames 6`, `perfRepeats 3`,
  `calibrationGames 2`, `calibrationRefMs 72` (yardstick median 2026-09-23).
- Timeouts: gate tests `60_000 + GAMES x 45_000` ms (smoke 16 min; full random 76 min; full
  greedy/hard 38.5 min); perf `60_000 + GAMES x 120_000` ms. A shard keeps the whole-run timeout.
- Sharding: `JACKIOH_AI_GATE_SHARD=k/K` plays games k, k+K, ...; writes JSON to
  `JACKIOH_AI_GATE_OUT`; `pnpm ai:gate:merge <dir>` (`scripts/gate-merge.ts`) checks every game 1..N
  once and wins >= `gateNeeded`. CI (`.github/workflows/ci.yml:235-287`): job `ai-gate-shard`, matrix
  `shard: [1..12]`, `timeout-minutes: 15`, then `ai-gate` merges (required check "ai quality gates
  (full)"). `unit` runs the ai project in halves with `--exclude '**/gate-*.test.ts'`.
- Measured (CI run 37497560121 at `91cc43c`, GitHub 4-vCPU runners): 12 shards took 0:53-2:12 each
  (shard 1 vitest `Duration 95.25s`); merged: `ai-vs-random 94 wins / 0 draws of 100 (91 needed)`,
  `ai-vs-greedy 35/50 (28)`, `hard-vs-easy 47/50 (40)`.

### 2.3 Scripts (`packages/ai/scripts`, Node tooling, may do I/O)

| File | Lines | CLI | Purpose |
|---|---:|---|---|
| `arena-bridge.ts` | 153 | `pnpm --filter @jackioh/ai arena-bridge` (stdin/stdout JSON lines) | Ladder bridge: `new_game {gameId,seed,decks}`, `legal`, `observe` (`viewFor`), `act`, `result` (hash), `close`, `deck {seed,size,manaCap?}` (`buildAiDeck`, **default ban applied**), `cards` (non-token pool). No handicaps, **no command that runs `decide`**. |
| `bench.ts` | 124 | `tsx scripts/bench.ts <matchup> <from> <to> [gate\|full]`; env `SERIES`, `OVERRIDE_<CONFIG>`, `BAN=id,id`, `OTHER_UNBANNED=1`, `SUBJECT/OPPONENT=greedy\|random\|ai`, `SWAP_DECKS=1`, `PLAYED=1` | Plays tuning-series games, one JSON line each. |
| `duel.ts` | 129 | `tsx scripts/duel.ts <matchup> <from> <to>`; env `SUBJECT_<CONFIG>`, `OPPONENT_<CONFIG>` (AI_SEARCH/AI_EVAL/AI_REPLY), `SUBJECT_BUDGET`, `OPPONENT_BUDGET`, `OPPONENT=ai\|greedy\|random`, `SWAP_DECKS=1`, `SERIES` | Per-seat weights: **the existing tool for "new AI vs predecessor" on the same deals**; prints final hash per game. |
| `oracle.ts` | 72 | `tsx scripts/oracle.ts <matchup> <from> <to> [fromTurn]`; `ORACLE_NODES` (20,000) | Lethal solver on the true state for lost games (missed kill vs no kill). |
| `trace.ts` | 97 | `tsx scripts/trace.ts <matchup> <n> [gate\|full]` | Prints one game turn by turn. |
| `gate-merge.ts` | 70 | `pnpm ai:gate:merge [dir]` | Merges shard files, exit 1 on a missing/doubled game or too few wins. |
| `sweep.ts` | 272 | `pnpm ai:sweep [ids...]`; `--json ids` (pass 1 slice, JSONL); `--pass2 a.jsonl,b.jsonl [ids]`; `--report a.jsonl p2.jsonl` | Two-pass sweep; prints flagged rows and ready-made `SHADOW_BAN`/`SHADOW_WATCH` entries; writes no file (copy by hand). |
| `stats.ts` | 121 | `pnpm ai:stats [--games=N --from=N --patch=v --series=s --out=file]` | R378 dev run; JSONL records; prints card win rates. |

---

## 3. "The last successful AI"

**Conclusion: `packages/ai` at commit `91cc43c` is the last (and only) AI that has passed gates. No ladder
champion exists.**

Evidence:

- `ladder/champions/1/.gitkeep`, `ladder/champions/2/.gitkeep`, `ladder/hall_of_fame/.gitkeep`,
  `ladder/history/.gitkeep`, `ladder/proposals/.gitkeep`: all 0 bytes, the only files there. No
  `history/proposals.jsonl`.
- `.github/workflows/` holds `bot-commands.yml bot-night.yml bot-selftest.yml bot-status.yml
  ci-duration.yml ci.yml deploy-watch.yml patches-ship.yml promote-production.yml squishy-commands.yml
  squishy-run.yml triage.yml`: no `generation.yml` or card-pool workflow; `ladder/` is not run in CI
  (no workflow references it; `ladder/DECISIONS.md`: outside the pnpm workspace and `pnpm test`).
- Issue #55 closed 2026-10-05 by PR #228 (`baeba11`, "Ladder arena, audit, and proposer pass review
  findings"), phases 1-3 only; its plan says phases 4-5 are manual and were never written.
- Last commit touching `packages/ai`: `91cc43c` (2026-10-06, "The AI samples a face-down trap at the
  cost the board shows (R752) (#385)", GitHub API `list_commits path=packages/ai`). Its push CI run
  37497560121 concluded **success**, merged gates 94/100, 35/50, 47/50. The next main commit `f13cb03`
  (#388) had CI run 37521149928 fail only because `ai gates (3/12)` failed in the
  `./.github/actions/setup` step (infra), not in a game.
- AI history (GitHub API): `696fadc` 2026-09-24 "Polish 3: an AI opponent with three difficulties" (#9);
  `1005c50` v0.1.1 sweep of record; `0b06ebd`/`4bc3a19`/`d9a04ad` 2026-10-01 v0.2.0 AI (Activate,
  new prompts, all sets, two-pass sweep); `54a3002` 2026-10-03 `gate:v3`; `106831c` 2026-10-04 #205
  speed-up (#188); `91cc43c` 2026-10-06 R762 sampler.
- Recorded gate history (SPEC.md:976): Core-era full runs 98/100, 31/50, 45/50; after Radiant pass
  93/100, 35/50, 48/50; R762 smoke 20/20, 17/20, 20/20.

---

## 4. `ladder/` (issue #55, phases 1-3; Python 3.10+, `pyyaml`, `jsonschema`, `pytest`)

| Path | Lines | Bytes | Purpose |
|---|---:|---:|---|
| `ladder/README.md` | 132 | 6,531 | Layout, how to run, read results, switch providers, what phases 4-5 need |
| `ladder/DECISIONS.md` | 18 | 2,940 | Judgment calls (Python + Node bridge; raw `legal`; random bot skips concede/offerDraw/answerDraw, no end-turn bias; forfeits; decks dealt by arena; stdlib HTTP; etc.) |
| `ladder/config.yaml` | 16 | 363 | `games_per_opponent: 100`, `win_threshold_vs_champion: 75`, `win_threshold_vs_random: 90`, `max_shadowban_fraction: 0.10`, `shadowban: {min_opportunities: 20, max_use_rate: 0.05, agent_builds_decks: auto}`, `train_budget_minutes: 300`, `proposer: {...}`, `hall_of_fame_games: 20`, `balance_flag_streak: 3` |
| `ladder/pyproject.toml` | 16 | 391 | Package `jackioh-ladder` |
| `ladder/common.py` | 18 | 470 | `LADDER_ROOT`, `load_config` |
| `ladder/arena/bridge.py` | 83 | 2,818 | `Bridge`: spawns `pnpm --filter @jackioh/ai arena-bridge`, JSON lines, 60 s timeout, `BridgeError` |
| `ladder/arena/runner.py` | 187 | 7,359 | `play_game` (decks via bridge `deck`, size 20, seats via `next`, logs `legal_plays`/`played`/`specialist`, forfeit on illegal/throw), `play_series` (candidate p1 on even i) |
| `ladder/arena/types.py` | 24 | 779 | `Agent` protocol `act(obs, legal) -> Action`; `Observation` = `viewFor` JSON |
| `ladder/agents/random/agent.py` | 33 | 1,333 | `RandomAgent`: uniform over `legal` minus the 3 skipped types, `random.Random(seed)` |
| `ladder/audit/gates.py` | 63 | 2,412 | `evaluate_gates(wins_vs_champions, wins_vs_random, cand_sb, champ_sb, pool_size, config)` -> `{promoted, failures}` |
| `ladder/audit/shadowban.py` | 124 | 4,702 | Use-rate shadowban: `opportunities >= 20` and `plays/opps < 0.05`; `unobserved`; `audit_candidate` |
| `ladder/llm/client.py` | 74 | 2,808 | `LLMClient` protocol, `from_env` (`LLM_PROVIDER`, `LLM_MODEL`, `LLM_API_KEY`) |
| `ladder/llm/providers/{anthropic,openai,google,mock,base}.py` | 21/25/44/44/49 | | stdlib-HTTP providers; `mock` returns a fixed proposal |
| `ladder/proposer/propose.py` | 220 | 7,818 | Context bundle, lenient parse (`--- FILE: ---` blocks), strict schema, retries, history entry |
| `ladder/prompts/proposer.md` | 52 | 1,861 | Proposer prompt (RL / multi-context Python agents) |
| `ladder/schemas/spec.schema.json` | 68 | 2,075 | Proposal schema (`agent.kind: rl\|multi_context`, algorithms `ppo_masked\|dqn\|mcts_value\|variant`) |
| `ladder/tests/test_arena.py` | 117 | 4,239 | 7 tests; spawns the real bridge (needs `pnpm install`) |
| `ladder/tests/test_gates.py` | 72 | 2,477 | 8 tests (0/1/2 champions, zero-shadowban exception) |
| `ladder/tests/test_llm.py` | 68 | 2,409 | 7 tests (mock, loud config failures, provider confinement) |
| `ladder/tests/test_proposer.py` | 105 | 3,619 | 8 tests |
| `ladder/tests/test_shadowban.py` | 102 | 4,088 | 9 tests |
| `ladder/champions/{1,2}/`, `hall_of_fame/`, `history/`, `proposals/` | 0 | 0 | `.gitkeep` only |
| **total** | **1,775** | **67,672** | |

**Reusable as-is / with small changes for the training lanes:**
- The gate *shape* (`ladder/audit/gates.py`) and the config-driven thresholds (`ladder/config.yaml`).
  But #306's rules differ: one predecessor (not two champions), per-lane thresholds (improve: random
  90, predecessor 85; unban: random 90, predecessor 75, strictly fewer shadow-banned), no 10% pool
  fraction gate, no zero-shadowban exception. A new small evaluator is simpler than bending
  `evaluate_gates`.
- `play_series`'s seat alternation and per-decision logging pattern; `RandomAgent` as a "fully random
  AI" (note: it differs from the gate's `randomAction`, which ends the turn with 10% probability,
  `AI_END_TURN_PROBABILITY` in `packages/engine/src/config.ts`; #306 must pick one).
- The R378 stats pipeline (`devRun.ts`, `scripts/stats.ts`, `stats:import`) for "log stats while
  playing".

**Not reusable / depends on the Node bridge:**
- `ladder/arena/bridge.py`, `runner.py`, `tests/test_arena.py` need `packages/ai/scripts/arena-bridge.ts`
  (Node, `pnpm install`). Agents there see only `viewFor` (`PlayerView`), so **the shipped
  `decide`-based AI cannot be a ladder agent** without a new bridge command (it needs a `GameState` to
  `redact`). The bridge's `deck` applies `SHADOW_BAN_IDS` by default and takes no handicaps: "with all
  cards" needs `banned: []`.
- `ladder/llm/*`, `ladder/proposer/*`, `prompts/proposer.md`, `schemas/spec.schema.json`: an LLM that
  emits Python RL agents as text. Under #306 Devin edits code directly, so these are not needed.
- `ladder/audit/shadowban.py` is the use-rate definition, not R186's sweep list. #306 says "keep the
  current list of shadow banned cards", which points at `packages/ai/src/shadowBan.ts` (R186).

---

## 5. The bot machines and lane configuration

### 5.1 Files in `bot/machine` (`wc -l`)

| File | Lines | Runs | What |
|---|---:|---|---|
| `bot/machine/README.md` | 210 | - | The night box, the training box, disk, memory, cost, starter |
| `bot/machine/setup.sh` | 255 | on the box, root | apt deps, 8 GB swap, Node 24 + corepack, CLIs (`claude`, `codex`, `agy`, `muse`, `devin`), `agent-<id>` users, runner unpack + `.env`/`.path`, **idle-stop** script/timer, disk timer, CloudWatch agent. Args: provider ids (default `gpt agy muse devin`) |
| `bot/machine/clean.sh` | 100 | each agent user | Disk clean-up after every job and every 10 min |
| `bot/machine/disk-report.sh` | 18 | root | Read-only disk report |
| `bot/machine/on-machine.sh` | 67 | your computer | Runs a script on the box via SSM (`TAG` default `jackioh-night-vm`; starts it if stopped; `AWS_REGION` default `us-east-2`) |
| `bot/machine/register-runners.sh` | 65 | your computer | Registers one runner per `lanes` for every provider whose `runs_on` starts `night-vm-` (`night-vm-<id>`, `night-vm-<id>-2`, ...), systemd service under `agent-<id>` |
| `bot/machine/starter.py` | 108 | AWS Lambda | Every 5 min: if the instance is `stopped` and a `night-vm-*` job is queued in `bot-night.yml`/`triage.yml` (`DEFAULT_WORKFLOWS`, env `WORKFLOWS`) for under `MAX_WAIT` 3 h, `start_instances`. `RUNNER_LABELS` restricts to one box |
| `bot/machine/deploy-starter.sh` | 84 | your computer | Creates the Lambda + role + EventBridge `rate(5 minutes)`; env `REPO_ID`, `TAG`, `STARTER_NAME`, `STARTER_ROLE`, `RUNNER_LABELS`, `STARTER_GITHUB_TOKEN` |

Tested by `bot/tests/test_machine.py` (starter).

**Instances (no cloud-init files or Terraform in the repo; systemd units are written inline by `setup.sh`):**

| Box | Instance | Tag | Users / runners | State |
|---|---|---|---|---|
| Night box | EC2 `m7i-flex.large` (2 vCPU, 8 GB + 8 GB swap), Ubuntu 24.04, 60 GB gp3, `us-east-2`, SSM only | `Name=jackioh-night-vm` | `agent-gpt` (1), `agent-agy` (1), `agent-muse` (2: `night-vm-muse`, `-2`), `agent-devin` (6: `night-vm-devin`, `-2`..`-6`) = 10 runners | running; ~$0.096/h (~$70/month if never stopped) + ~$4.80/month disk |
| Training box | EC2 `m7i.xlarge` (4 vCPU, 16 GB), 100 GB gp3 encrypted, IMDSv2, SSM only | `Name=jackioh-train-box` | `agent-devin-train` (1: `night-vm-devin-train`), plus `ladder/.venv` | **not built** (providers.json note: on 2026-10-06 no instance, no runner, #318). README: "A bigger box (4 vCPUs) needs the paid plan." |

systemd units written by `setup.sh`: `night-vm-idle-stop.service` + `.timer` (`OnBootSec=5min`,
`OnUnitActiveSec=5min`; script `/usr/local/bin/night-vm-idle-stop`, `IDLE_MINUTES=30`, busy if
`pgrep -f Runner.Worker`, an `ssm-user` process, or cloud-init running; then `shutdown -h now`),
`night-vm-disk.service` + `.timer` (10 min), runner services (`svc.sh install`), CloudWatch agent config
`/opt/aws/amazon-cloudwatch-agent/etc/night-vm.json` (namespace `JackiOh/NightVM`).

### 5.2 Lane configuration today

`.harness/providers.json` (211 lines):

| Key | Line | Value |
|---|---:|---|
| `max_parallel` | 2 | `10` (all runs at once, GitHub-hosted + machine) |
| `machine_parallel` | 3 | `7` (runs on the AWS boxes at once) |
| `plan_lanes` | 4 | `4` (planning runs on top of `max_parallel`; `bot/README.md` text still says 2) |
| `priority` | 6 | `claude-3, claude-1, claude-4, claude-6, claude-5, muse, agy, gpt, claude-2, devin, devin-train` |
| `tiers.weak` | 8-10 | `swe-2-max` (Devin) |
| claude-1/2/3 `lanes` | 35/54/73 | `2` each, `runs_on: ubuntu-latest` (claude-4/5/6: 1 each) |
| `gpt` | 131-144 | `runs_on: night-vm-gpt`, 1 lane |
| `agy` | 145-158 | `runs_on: night-vm-agy`, 1 lane |
| `devin` | 159-177 | `runs_on: night-vm-devin`, `lanes: 6`, `tier: weak`, `self_check: true`, `easy_first: true`, `off_from: "2026-10-15"` (SWE-2 free on the CLI only through 2026-10-16) |
| `muse` | 178-192 | `runs_on: night-vm-muse`, `lanes: 2` |
| `devin-train` | 193-209 | `"enabled": false`, `runs_on: night-vm-devin-train`, lanes default 1, `tier: weak`, `self_check: true`, `roles: ["build","revise"]`, `only_labels: ["training"]`, **no `off_from`** |

`.harness/config.json` (123 lines): `call_timeout_minutes: 150` (l.20), `job_budget_minutes: 330`
(l.21), `max_self_check_rounds: 3` (l.24), `easy` rule (l.25-42: `max_files 10`, `max_lines 400`,
`off_limits` incl. `packages/engine/src/` l.33 and `packages/ai/src/` l.34), `forbidden_paths` (l.74-86:
`.github/`, `.harness/`, `.squishy/`, `bot/`, ...), `gates` run by the bot (l.106-122).

`.squishy/config.json` (112) / `.squishy/providers.json` (52): Squishy has `max_parallel: 2`,
`machine_parallel: 0`, `plan_lanes: 0`, one provider `claude-squishy` (`ubuntu-latest`, 2 lanes).
Squishy uses no AWS lanes.

**How the counts are computed** (`bot/harness/plan.py`):
- `Lanes.limit = max_parallel` counts every held run, machine or hosted (`plan.py:137-190`,
  `read_lanes` l.203). A run holds its lane until it ends.
- `training_ids(pool)` (l.481-485) = enabled machine providers with `only_labels`.
- `machine_cap` (l.488-495): training providers get all of `machine_parallel`; everyone else gets
  `machine_parallel - sum(lanes of training providers)`.
- `machine_full` (l.498-506). `bot/harness/status.py:23-37` `night_slots`, `machine_text` ("n of 6 on
  the night box, n of 1 on the training box"). `bot/harness/dashboard.py:290-348` draws the boxes.
- `providers.py:560-588`: `machine_parallel` must be 0..`max_parallel`; two providers may not share a
  machine `runs_on`; `only_labels` parsed l.510-514; `takes_item` l.272-279 (an `only_labels`
  provider takes only items carrying all its labels; nobody else takes those).

So today: with `devin-train` disabled, **up to 7 machine runs, all on the night box** (from its 10
runners). If `devin-train` were enabled: **6 night + 1 training** (README "the seventh slot").

### 5.3 Exact changes for "6 normal lanes + 2 AI training lanes, always running"

Option A (training lanes as harness subscriptions, item-driven):

| File | Key / line | Change |
|---|---|---|
| `.harness/providers.json` | l.3 `machine_parallel: 7` | `8` (6 normal + 2 training, since `machine_cap` subtracts the training lanes) |
| `.harness/providers.json` | l.2 `max_parallel: 10` | `12` if the two always-held training lanes must not eat the 10 build lanes (they count against `Lanes.limit`) |
| `.harness/providers.json` | l.193-209 `devin-train` | either `"enabled": true, "lanes": 2` (two runners on one login, like Muse) or split into two providers (e.g. `devin-improve`, `devin-unban`) each with its own `runs_on` (`night-vm-devin-improve`, `night-vm-devin-unban` — runners may not be shared) and `only_labels` (`["training","lane:improve"]` / `["training","lane:unban"]`); add `off_from`/model decision (SWE-2 pricing after 2026-10-16) |
| `.harness/providers.json` | l.6 `priority` | must list every provider id exactly once (`providers.py:565-570`) |
| `.harness/config.json` | l.25-42 `easy` | Devin (weak) may build only easy items, and the rule forbids `packages/ai/src/`/engine and caps 10 files/400 lines; a Rust AI crate path would not be in `off_limits` yet, but "engine or AI work" is "never easy" in `bot/harness/easy.py` `RULE` (l.23-48). Needs a training exemption (or a medium/strong training tier) |
| `bot/harness/plan.py` | l.481-506 | generic, but docstring says "at most six ... with one left to train on"; adjust text |
| `bot/harness/status.py` | l.23-37 | generic; text |
| `bot/harness/easy.py` | `RULE`, `plan_breach`, `change_breach` | exemption for training items if chosen |
| `bot/tests/test_basics.py` | l.39 `(10, 7)` | update |
| `bot/tests/test_providers.py` | l.42-46 `(10, 7)`, l.270-275 devin-train fields, l.623 `(6, 7)`, l.660-690 "machine keeps a slot for training" (5 devin + 1 train) | update |
| `bot/tests/test_dashboard.py` | l.135-158 "1 of 6 on the night box, 1 of 1 on the training box" | update |
| `bot/machine/README.md` | "seventh slot", training-box table, starter commands | update |
| `bot/README.md` | l.7-9, l.364, l.429, l.495-505 | update |
| `CLAUDE.md` | "The night bot": "at most seven on the bot's own AWS machines (six on the night box, one on the training box)" | update (but `CLAUDE.md` is a skip-list path) |

All of `.harness/`, `bot/`, `.github/` are `forbidden_paths` for both bots: **a person must make these
changes** (bot selftest is a required check).

"Always running" conflicts:
1. **Idle power-off**: `setup.sh` `night-vm-idle-stop` powers the box off after 30 min with no
   `Runner.Worker`/SSM session. A training loop that is not a runner job (systemd service) would be
   killed; add its process to the busy check or skip the timer on the training box.
2. **Starter**: `starter.py` starts a stopped box only for queued jobs of `bot-night.yml`/`triage.yml`
   (`DEFAULT_WORKFLOWS`) with `night-vm-*` labels, younger than 3 h; a new workflow name needs
   `WORKFLOWS`. Worst-case wake latency 5 min.
3. **Job length**: `bot-night.yml` `work` job `timeout-minutes: 350` (l.249); `job_budget_minutes: 330`;
   each model call `min(call_timeout_minutes x 60, seconds_left - 300)` (`bot/harness/work.py:434`).
   A harness training run ends after ~5.5 h; continuous play needs a chain (like `bot-status.yml`,
   which loops 5.5 h then `gh workflow run` itself) or a systemd service outside Actions. (GitHub's own
   limit for self-hosted jobs is longer, 5 days; not stated in the repo.)
4. **Item-driven queue**: the harness only runs when a queued item exists (`training` label); after
   delivery the item closes. An always-on lane needs a standing item that is re-queued, or no harness.
5. **CPU**: the night box is 2 vCPU and already ran load average 14 with three full jobs (README,
   2026-10-02); AI games are CPU-bound, so training lanes belong on the training box (4 vCPU), and two
   lanes there split 4 vCPU.
6. **Cost**: an always-on box is billed every hour (night box ~$70/month at $0.096/h; `m7i.xlarge`
   rate not documented in the repo).

---

## 6. Existing Devin integration

- **Install**: `bot/machine/setup.sh` (l.~65-82) downloads `https://static.devin.ai/cli/current/manifest.json`,
  the `x86_64-unknown-linux` bundle, checks its sha256, unpacks into `/usr/local/lib/devin/<version>`,
  symlinks `/usr/local/bin/devin`. No API key; no Devin HTTP API is used anywhere in the repo.
- **Login** (machine login, once per Linux user): `sudo -iu agent-devin devin auth login
  --force-manual-token-flow` (and `agent-devin-train` on the training box). `bot/harness/logins.py:107-111`:
  a `"machine"` login writes only its env; the CLI finds its login in the user's home.
- **Invocation**: `bot/harness/runner.py:970-1014` `DevinCli`:
  `devin -p --prompt-file <transcript>.prompt.md --model swe-2-max --permission-mode dangerous
  --respect-workspace-trust false --export <transcript>.export`. The prompt file is
  `system_append + CLI_NOTE + "---" + prompt` (`_Cli._prompt`, l.417). Final answer from stdout
  (`_devin_answer` strips ANSI and the "Welcome to Devin CLI!" line); the `--export` JSON's `steps` become
  the trail (`_devin_steps`). No usage reporting, no turn limit: bounded by the call timeout; a
  rate-limit message parks the subscription (`park_for`). Environment stripped of secrets
  (`config.child_env`).
- **Prompts**: `bot/prompts/{system,plan,build,fix,revise,review,suggest,oneshot,split}.md` filled by
  `bot/harness/prompts.py` (`string.Template`); GitHub text fenced as data (`prompts.data`).
- **Flow**: `.github/workflows/bot-night.yml` `gate -> plan -> work -> deliver`; `work` runs on
  `needs.plan.outputs.runs_on` (e.g. `night-vm-devin-3`), `permissions: contents: read`, runs `python3 -m
  harness work --plan ... --out ... --work-dir ...`; `deliver` (GitHub runner, bot token) verifies the git
  bundle and pushes / opens the PR. Devin's builds go through the self-check loop
  (`bot/harness/work.py:1380` `_self_check_loop`, `max_self_check_rounds: 3`) and then need a review run
  by a medium/strong model; merge on CI + approvals (`bot/harness/review_rule.py`).
- **Handoff/memory**: `.bot-notes.md`, the journal `<issue>.md` on `bot-journal` (read into the next
  prompt), `bot/wip/<pr>` for revisions.
- **Runners/users**: `agent-devin` with six runners `night-vm-devin`..`night-vm-devin-6`;
  `agent-devin-train` with `night-vm-devin-train` (not built). `triage.yml` runs on `night-vm-muse`
  (moved off Devin in #317).
- **For a standing training prompt**, the repo offers two patterns: (a) a long-lived `training`
  issue whose description/plan is the standing prompt, re-queued after each run, carrying state in the
  journal; or (b) outside the harness, a systemd service on the training box running
  `devin -p --prompt-file <standing prompt> --model <m> --permission-mode dangerous
  --respect-workspace-trust false` in a loop as `agent-devin-train` (no GitHub write token on the box;
  results would need a push path).

---

## 7. AI performance numbers documented

| Source | Number |
|---|---|
| `docs/polish/3-ai.md:1431` (pre-v0.2.0) | at `AI_GATE_BUDGET` "a game takes 2 to 4 s of one core on the development machine, and Hard against Easy about twice that" |
| `docs/polish/3-ai.md:1940-1946` | decision time at `AI_BUDGET`: mean 38 ms, median 16, p95 144, max 215 ms; 123 nodes mean, 387 max; Hard seat 27/15/81/157 ms |
| `docs/polish/3-ai.md:1262` | `pnpm ai:gate` "target under 6 minutes unloaded on the 10-core machine" |
| `packages/ai/src/gate.ts` | `maxDecisionMs 1500`, `calibrationRefMs 72` |
| `apps/web/src/practice/config.ts:65` | `PRACTICE_AI_CLOCK_MS = 1500` |
| Issue #70 (2026-10-01, 4-core) | `reduce` 3.1 ms/action (Core, before v0.2.0) -> 4.9 (v0.2.0, Core) -> **7.5 ms/action (all three sets)**; `gate-hard-easy.test.ts` 20 games 6 m 41 s -> **17 m 2 s**; profile: `cloneState` 13.6%, `slotsOf`/`activeUnitsOf` ~12%, layers ~20%, scorer `dryRun` ~19% |
| `.github/workflows/ci.yml:229-233` | "one hard-vs-easy game can take four to five minutes on its own (gate:v2:hard-vs-easy:12)" |
| Issue #188 plan (PR #205, 4-vCPU runner) | shard 2/12 393 s -> 181 s; game `gate:v3:hard-vs-easy:38` ~254 s -> ~115 s; other shards <= 131 s; merged 97/39/49 unchanged |
| CI run 37497560121 (`91cc43c`) | 12 gate shards 0:53-2:12 each; shard 1 vitest 95.25 s; `unit (ai 1/2)` 2:15, `(ai 2/2)` 0:57 |
| `ci.yml:98` comment | ai unit project alone took 7m28s (#172), now halves |
| Commit `75b1e42` | GIGA Glowy Jelly Bean sweep test: 267 s (sandbox) / 313 s (bot box) for two tiers; Easy 251 s for two seeds, Hard 12 s |
| `packages/ai/README.md:247` | dev run: "A game takes seconds at the browser's budget, so a run of a few hundred takes the better part of an hour on one core" |
| `bot/machine/README.md` | on GitHub's 4-vCPU runner `pnpm test` 22.6 min; on the night box lint 2 min, typecheck 4.5 min |

Derived (not in the repo): the green CI gate run spent about 12 shards x ~95 s of vitest on 4-vCPU
runners for 200 gate games plus 6 perf games, roughly 1,100-1,500 runner-seconds, i.e. on the order of
5-25 core-seconds per game on average, with long Hard-vs-Easy outliers of 1-2 minutes. A #306
generation check (100 games vs random + 100 vs predecessor, AI-vs-AI costing about twice AI-vs-random)
is therefore tens of core-minutes per generation on the 4-vCPU training box, before any training.

Threshold feasibility (binomial, n = 100): an AI at the measured 94.5% vs random passes 90/100 with
probability 0.978 (at 90%: 0.583). Against its predecessor, passing 85/100 needs a true win rate near
85%+ (P = 0.568 at 85%, 0.129 at 80%, 0.011 at 75%); 75/100 needs about 75-80% (P = 0.553 at 75%,
0.913 at 80%). For scale, Hard's full resource handicap over Easy gives only 91.3% (`measuredRate`).
A same-algorithm successor at ~50% vs its predecessor will essentially never be promoted by the
"improve" rule.
