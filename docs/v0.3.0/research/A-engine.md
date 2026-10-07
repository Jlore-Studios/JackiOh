# Research A — engine, cards, shared, validator inventory for the v0.3.0 Rust port

Repo: `/home/user/JackiOh` at `91cc43c` (main, 2026-10-06, catalog `v0.2.11`). Read-only survey; every
number below comes from `wc -l`, `grep` or a small node script over the tree. Paths are relative to the
repo root unless absolute.

Headline numbers

| thing | files | lines |
| --- | --- | --- |
| `packages/engine/src` (all `.ts`) | 147 | 36,999 |
| `packages/engine/test` (`*.test.ts` + fixtures) | 169 tests + 30 fixtures = 199 | 68,173 (tests 62,390, fixtures 5,783) |
| `packages/cards/src/scripts` (card + token scripts, excl. `_generated.ts`) | 318 | 16,181 |
| `packages/cards/src` other (`index.ts`, `catalog-data.ts`, `query.ts`, `flavour.ts`, `kyTestBank.ts`, `_generated.ts`) | 6 | 1,043 |
| `packages/cards/test` (`*.test.ts`) | 366 (+ `_harness.ts` 1,149, `_invariants.ts` 258, `_glow.ts` 35, `globalSetup.ts` 36, 1 JSON fixture) | 90,068 in tests |
| `packages/cards/catalog.json` | 1 | 221,758 bytes, 318 entries |
| `packages/shared/src` | 9 | 2,477 |
| `packages/validator/src` | 2 | 540 |
| `it(`/`test(` calls | engine 2,705; cards 5,174; shared 74; validator 40; ai 313 | |

---

## 1. ENGINE SOURCE MAP

### 1.0 How the engine is wired (read before clustering)

* Root barrel `packages/engine/src/index.ts` (70 lines) does `export *` from 47 modules (~750 exported
  declarations) plus `export * as effects` (199 values / 50 types) and `export * as subsystems` (~187
  functions/consts). Package exports: `"."`, `"./effects"`, `"./config"` (`packages/engine/package.json`).
* At the **cluster** level everything imports everything. At the **file** level, counting only value
  imports (`import type` excluded, re-exports included), the graph is almost a DAG: only three
  multi-file strongly connected components exist:
  1. `state.ts` ↔ `zones.ts` ↔ `preview.ts` ↔ `subsystems/lastBoards.ts`
  2. `damage.ts` ↔ `replacements.ts`
  3. `reduce.ts` ↔ `subsystems/aiPolicy.ts` ↔ `subsystems/heroPower.ts` ↔ `subsystems/callToChaos.ts` ↔
     `effects/index.ts` ↔ `effects/cast.ts` ↔ `effects/combat.ts` ↔ `effects/rounds.ts`
  The DAG shape is bought with **module-scope registration hooks** that invert the dependency at
  runtime (see §8): `registerWorkHandler` ×16, `registerDefaultWorkHandler`, `registerPromptAnswerer` ×4,
  `registerTargetingHooks`, `registerCastDriver`, `registerDeclarationCheck`, `registerGraveyardRedirect`.
  So the runtime call graph is fully cyclic (work items call back into play steps, combat, turn, …).
* Topological value-import layers (L0 = imports nothing local), useful as a port order:
  - L0: `announce config castOnDrawNow drawComplete enchantments gameOver killCredit marks rng script stays`
  - L1: `catalog scripts tuning` — L2: `faces ownLibrary work subsystems/copiedText subsystems/lastBoards timesPlayed`
  - L3: `brittleCount params playCounts preview state` — L4: `randomCast resolve zones`
  - L5: `carriers costRules graveyardPlay layers modifiers plague prompts effects/each subsystems/audit subsystems/callToChaos`
  - L6: `animated instanceView mana restrictions` — L7: `damage echo numbers targeting effects/targets`
  - L8: 27 files (most simple `effects/*`, `playChoices`, `subsystems/lethal`, `subsystems/quests`)
  - L9: `cryTrigger query replacements effects/{brittle,buff,chooseWhere,enchant,perks}`
  - L10: `condition stateCheck effects/{coins,cry,summon,tune}` — L11: `draw traps effects/{afterCheck,destroy,summonThis}`
  - L12: `brittle ownership subsystems/fuse effects/{addToHand,draw,drawWhile,libraryCopies,randomPicks,rounds,shuffleCard,shuffleInto,shuffleRandom}`
  - L13: `effects/{fruit,give,lastBoard,move}` — L14: `targetingPoint temporary effects/{datacenter,delay,fuse}`
  - L15: `effects/transform subsystems/activate subsystems/twiceForward` — L16: `bookSwap subsystems/callToChaosPlus subsystems/heroPower`
  - L17: `triggers` — L18: `combat counterWarning subsystems/aiPolicy` — L19: `effects/{combat,position,steal,swap} subsystems/boardHistory subsystems/rotation`
  - L20: `turn effects/rotate` — L21: `setup` — L22: `subsystems/glitch` — L23: `playSteps` — L24: `reduce subsystems/scorer`
  - L25: `gameSummary replay subsystems/perfectHand` — L26: `effects/index` — L27: `subsystems/{comboIndex,kyTest,papaya}` — L28: `viewFor subsystems/index` — L29: `index`

### 1.1 Proposed port clusters (9), sizes and cross-cluster value imports

| cluster | files | lines | value imports INTO other clusters (file names) |
| --- | --- | --- | --- |
| **C1 model + registries** (state, rng, config, catalog, script contract, instance data readers) | 20 | 3,917 | C4: `work` (from `params.ts`); C9: `lastBoards` (from `state.ts`). Everything else in C1 is a leaf. |
| **C2 board** (zones, layers, query, numbers, field statuses) | 12 | 2,568 | C1: announce, brittleCount, catalog, config, faces, ownLibrary, params, scripts, state, stays, tuning · C4: resolve, work · C5: draw, mana, playChoices · C7: preview · C8: destroy (brittle.ts), move (temporary.ts), transform (bookSwap.ts) · C9: lethal (query.ts) |
| **C3 damage / combat / replacements / targeting** | 5 | 2,392 | C1: catalog, config, faces, killCredit, scripts, state, stays · C2: layers, restrictions, zones · C4: modifiers, prompts, resolve, stateCheck, traps, triggers, work · C5: mana · C8: flicker (replacements), move + summon (targetingPoint) |
| **C4 resolution loop** (work/resume, prompts, triggers, traps, state check, modifiers, echo) | 10 | 5,077 | C1: announce, catalog, config, drawComplete, faces, marks, script, scripts, state, stays, tuning · C2: animated, bookSwap, carriers, layers, plague, zones · C3: replacements · C5: mana, playChoices · C6: gameOver · C7: preview · C8: delay (triggers.ts) · C9: copiedText, quests |
| **C5 play pipeline** (§10.5 steps, play choices, mana/cost, draw) | 7 | 4,821 | C1: announce, catalog, config, drawComplete, enchantments, faces, ownLibrary, params, scripts, state, stays, timesPlayed · C2: animated, layers, query, restrictions, zones · C3: damage, targeting, targetingPoint · C4: echo, modifiers, prompts, randomCast, resolve, stateCheck, triggers, work · C9: copiedText, glitch |
| **C6 turn / setup / reduce / replay** | 6 | 2,306 | C1: catalog, config, ownLibrary, rng, scripts, state · C2: animated, brittle, query, temporary, zones · C3: combat · C4: modifiers, prompts, stateCheck, traps, triggers, work · C5: draw, mana, playChoices, playSteps · C8: delay · C9: activate, aiPolicy, boardHistory, fuse, glitch, heroPower |
| **C7 view** (viewFor, instance view, preview, glow, counter warning) | 5 | 1,459 | C1: announce, brittleCount, catalog, enchantments, faces, marks, ownLibrary, params, state, tuning · C2: animated, layers, plague, query, zones · C3: combat, damage · C4: echo, triggers · C5: costRules, mana, playChoices · C6: setup, turn · C9: activate, comboIndex, copiedText, fuse, heroPower, quests |
| **C8 effects verbs** (`effects/*`) | 61 | 8,639 | C1: announce, brittleCount, catalog, config, enchantments, faces, killCredit, marks, ownLibrary, params, state, stays, tuning · C2: animated, layers, numbers, ownership, plague, query, restrictions, temporary, zones · C3: combat, damage · C4: cryTrigger, echo, modifiers, prompts, randomCast, resolve, stateCheck, triggers, work · C5: costRules, draw, mana · C7: preview · C9: aiPolicy, callToChaos, fuse, lastBoards, rotation (+ barrel re-exports perfectHand, boardHistory, glitch) |
| **C9 subsystems** (`subsystems/*`, card-specific machinery) | 21 | 5,820 | C1: brittleCount, catalog, config, enchantments, faces, params, rng, script, scripts, state, stays, tuning · C2: layers, zones · C3: combat, damage, targeting, targetingPoint · C4: prompts, resolve, stateCheck, triggers, work · C5: draw, mana, playChoices, playSteps · C6: gameOver, reduce, setup · C7: preview · C8: addToHand, brittle, destroy, fuse, index, move, summon, targets, transform, tune |

Notes for the plan author: `preview.ts` (C7) is imported from C2/C4/C8/C9 and sits at L3 — it belongs with
C1/C2 in a port order. `query.ts` (C2) pulls `subsystems/lethal`. `temporary.ts`/`brittle.ts`/`bookSwap.ts`
(C2) depend on effect verbs, so they port after C8's `move`/`destroy`/`transform`. `C9` is effectively
"last": it depends on every other cluster, and `subsystems/scorer.ts` drives `playSteps.runPlaySteps` +
`triggers.settle` on cloned states.

### 1.2 File tables (line count, first line of the header comment)

#### C1 model + registries — 3,917 lines
| file | lines | responsibility |
| --- | --- | --- |
| `state.ts` | 1012 | §10.1 state model (all JSON), `createGame`, `cloneState`, `newInstance`, `findInstance`, `validateDeck`, handicap helpers |
| `rng.ts` | 86 | xmur3 → mulberry32, random-access by cursor; `int/pick/shuffle/coin/chance/lucky` |
| `config.ts` | 469 | every rules constant (BUILD §2), incl. "decide" rulings R1, R2, R4, R5, R14, R26, R39 |
| `catalog.ts` | 344 | process-global catalog registry, `defOf`/`findDef` (transient defs win), §5.1 `query`, fused-id parsing/digests, Grapes, Glitch odds |
| `scripts.ts` | 130 | process-global script registry, `scriptOf`/`flagsOf` (Vanilla → empty), fused ingredient price records |
| `script.ts` | 577 | the card-script contract: `EffectContext`, `Effect`, `Script`, `TriggerDef`, `StaticFlags`, `ActivationDecl`, hooks |
| `faces.ts` | 31 | a card's running face (base/radiant, face type) |
| `enchantments.ts` | 58 | enchantments riding an instance (B5 E39) |
| `tuning.ts` | 200 | Degrade/Upgrade lasting changes readers (R386) |
| `params.ts` | 193 | declared tunable numbers (`param`, `stepParam`) |
| `brittleCount.ts` | 98 | Brittle X count readers/writers |
| `stays.ts` | 285 | a card's stay in a zone, field-exit marks (R174, R212) |
| `announce.ts` | 81 | announce-window record (R448) |
| `marks.ts` | 92 | R437 card marks for pending effects |
| `killCredit.ts` | 23 | kill credited to another unit (R42, R412) |
| `castOnDrawNow.ts` | 13 | is a card being cast on draw now |
| `drawComplete.ts` | 42 | "draw complete" point of cast-on-draw (R58) |
| `timesPlayed.ts` | 24 | R429 times played |
| `ownLibrary.ts` | 89 | what a player may know of their own library (R310–R312) |
| `index.ts` | 70 | package barrel (`export *` ×47, `effects`/`subsystems` namespaces) |

#### C2 board — 2,568 lines
| file | lines | responsibility |
| --- | --- | --- |
| `zones.ts` | 874 | zones, lanes, adjacency, rotation rings, locks, Stack piles; the only zone movers (`placeOnField`, `moveToZone`, `removeFromAnyZone`) |
| `layers.ts` | 324 | §10.4 stat/keyword layers, computed on read (`unitView`, `computeLayers`, `auraMods`) |
| `query.ts` | 348 | read-only board facts for card scripts (`heroOf`, `zoneCards`, …) |
| `numbers.ts` | 193 | "a number on a card" for Degrade/Upgrade/KY's Constant |
| `animated.ts` | 225 | Animated / "Animated on your turn" (R383) |
| `carriers.ts` | 73 | backrow card carrying a Unit (R446) |
| `restrictions.ts` | 152 | what a card may not be done to (E35), attack bars |
| `plague.ts` | 107 | Plague Counters on a permanent (R471) |
| `ownership.ts` | 112 | a card changing owner (E2, E16) |
| `brittle.ts` | 92 | Brittle tick and crumble |
| `temporary.ts` | 33 | Temporary keyword discard (R637) |
| `bookSwap.ts` | 35 | Classic #55 Book of Wildfire swap trigger (R671) |

#### C3 damage / combat / replacements / targeting — 2,392 lines
| file | lines | responsibility |
| --- | --- | --- |
| `damage.ts` | 427 | one damage instance: §4.4's ten steps, heal, lose health |
| `combat.ts` | 1043 | positions, exertion, attack validation, combat resolution, forced attacks, after-attack |
| `replacements.ts` | 584 | replacement windows (E5, E8, E9; R460) |
| `targeting.ts` | 166 | who may be targeted and what targeting costs (R450) |
| `targetingPoint.ts` | 172 | what happens the moment a card is targeted (redirects, discards) |

#### C4 resolution loop — 5,077 lines
| file | lines | responsibility |
| --- | --- | --- |
| `work.ts` | 593 | resumable-work queue (R113 cursor), `PausedStep`, handler registry, drain |
| `resolve.ts` | 206 | build `EffectContext`, apply effects, `runHook`, cast driver |
| `prompts.ts` | 974 | `state.pending`, 15 prompt kinds, resume plans, `answerPrompt`, `promptAnswers`, resumable list runner |
| `triggers.ts` | 853 | trigger registry, trigger queue, `settle` (§10.3 loop) in R68 order |
| `traps.ts` | 766 | trap matching and immediate resolution, trap windows |
| `stateCheck.ts` | 662 | deaths and the state check (§4.5), Reborn |
| `modifiers.ts` | 240 | player modifiers and delayed effects, expiry |
| `echo.ts` | 292 | Echo repeats (R30) |
| `cryTrigger.ts` | 304 | "Trigger a Cry" (R467) |
| `randomCast.ts` | 187 | random casts / enemy-targeting casts (R452) |

#### C5 play pipeline — 4,821 lines
| file | lines | responsibility |
| --- | --- | --- |
| `playSteps.ts` | 2059 | §10.5 eight steps as named resumable steps; play prompts answerer; cast driver |
| `playChoices.ts` | 1413 | play-time choices built and validated (R81, R90); `playActionsFor`, tribute rules |
| `mana.ts` | 223 | mana refresh, temporary mana, cost calculation (R65, R396) |
| `costRules.ts` | 248 | cost rules (E15, R455) |
| `graveyardPlay.ts` | 177 | playing from the graveyard (R454) |
| `playCounts.ts` | 69 | per-play counters (R451) |
| `draw.ts` | 632 | drawing, fatigue, hand cap, cast-on-draw chains, library cap, draw limits |

#### C6 turn / setup / reduce / replay — 2,306 lines
| file | lines | responsibility |
| --- | --- | --- |
| `reduce.ts` | 567 | `reduce`, `legalActions`, `beginGame`, `seatToAct`, timeout/auto-end-turn |
| `turn.ts` | 842 | turn loop §2.2 in R62 order, cleanup, turn cap, concede/draw offers |
| `setup.ts` | 605 | shuffle, opening draw, Quickdraw, mulligan (R265), Coin, start-of-game |
| `gameOver.ts` | 23 | `endGame` |
| `replay.ts` | 83 | `fold` (seed, decks, log → state) and `hashState` |
| `gameSummary.ts` | 186 | finished game's record for card statistics (R376) |

#### C7 view — 1,459 lines
| file | lines | responsibility |
| --- | --- | --- |
| `viewFor.ts` | 1177 | `viewFor(state, player)` — hidden-information filter (§10.8) |
| `instanceView.ts` | 62 | instance data on a card view |
| `preview.ts` | 100 | `preview` numbers (R280) |
| `condition.ts` | 70 | yellow glow `conditionActive` (R195) |
| `counterWarning.ts` | 50 | R667 counter warning on hand cards |

#### C8 effects verbs — 8,639 lines (61 files)
| file | lines | responsibility |
| --- | --- | --- |
| `effects/index.ts` | 382 | the barrel: whole card-script verb vocabulary |
| `effects/targets.ts` | 250 | `TargetSpec`, `BoardScope`, `PlayerSpec`, `resolveTarget`, `cardsInScope` |
| `effects/choose.ts` | 753 | Choose one / target / hand / Discover / cell / number / pick / reward prompts |
| `effects/tune.ts` | 569 | Degrade, Upgrade, KY's Constant |
| `effects/summon.ts` | 553 | Summon, Recruit, fill board |
| `effects/fuse.ts` | 450 | Fuse as a verb |
| `effects/move.ts` | 398 | Exile, Bounce, Discard, Counter |
| `effects/transform.ts` | 319 | Transform, Vanilla |
| `effects/delay.ts` | 311 | delayed effects |
| `effects/combat.ts` | 295 | forced attack, cancel attack, AI turn |
| `effects/swap.ts` | 270 | Swap (#87) |
| `effects/radiant.ts` | 257 | Make Radiant |
| `effects/plague.ts` | 232 | Plague verbs (+ placement prompt answerer) |
| `effects/cast.ts` | 221 | Cast from anywhere, cost rules |
| `effects/fruit.ts` | 195 | Classic+ Fruit verbs |
| `effects/addToHand.ts` | 186 | add to hand |
| `effects/buff.ts` | 184 | buffs and keyword grants |
| `effects/locks.ts` | 145 | Lock variants |
| `effects/cardScope.ts` | 128 | card scopes across zones |
| `effects/give.ts` | 125 | cards between players' piles |
| `effects/steal.ts` | 118 | Steal |
| `effects/coins.ts` | 114 | coin flips |
| `effects/lastBoard.ts` | 104 | last-board verbs (R417) |
| `effects/destroy.ts` | 104 | Destroy, Sacrifice |
| `effects/datacenter.ts` | 103 | AI generated cards' verbs |
| `effects/counters.ts` | 100 | counters and locks |
| `effects/damage.ts` | 96 | Deal damage |
| `effects/library.ts` | 94 | exile out of a library |
| `effects/afterCheck.ts` | 80 | "…, then …" after deaths |
| `effects/cost.ts` | 80 | cost mods on an instance |
| `effects/rounds.ts` | 69 | damage in rounds |
| `effects/shuffleInto.ts` | 69 | shuffle into library |
| `effects/brittle.ts` | 68 | Brittle verbs |
| `effects/flicker.ts` | 67 | Flicker |
| `effects/chooseWhere.ts` | 65 | narrowed target prompt |
| `effects/mana.ts` | 63 | mana effects |
| `effects/draw.ts` | 59 | Draw |
| `effects/killCredit.ts` | 49 | kill credit |
| `effects/handExile.ts` | 49 | exile at random from hand |
| `effects/randomPicks.ts` | 49 | random picks (#90 rewards) |
| `effects/shuffleCard.ts` | 48 | shuffle an existing card |
| `effects/statuses.ts` | 48 | Berserk, attack again |
| `effects/enchant.ts` | 47 | enchant |
| `effects/heal.ts` | 47 | Heal |
| `effects/split.ts` | 47 | split damage |
| `effects/health.ts` | 44 | set health, heal→damage |
| `effects/turnEnd.ts` | 44 | end turn from effect |
| `effects/perks.ts` | 43 | small Classic+ gifts |
| `effects/playerMods.ts` | 43 | player modifiers |
| `effects/libraryCopies.ts` | 41 | T-AI-3 Hallucination |
| `effects/cry.ts` | 41 | Trigger a Cry verb |
| `effects/memory.ts` | 38 | remember on instance |
| `effects/shuffleRandom.ts` | 38 | shuffle random catalog cards |
| `effects/position.ts` | 35 | switch position as effect (R20) |
| `effects/animate.ts` | 34 | Animate |
| `effects/each.ts` | 32 | for each card |
| `effects/drawWhile.ts` | 31 | draw until |
| `effects/reveal.ts` | 27 | Reveal |
| `effects/summonThis.ts` | 21 | summon this from hand/deck |
| `effects/loseHealth.ts` | 14 | lose health |
| `effects/rotate.ts` | 53 | Rotate verb |

#### C9 subsystems — 5,820 lines (21 files)
| file | lines | responsibility |
| --- | --- | --- |
| `subsystems/fuse.ts` | 1171 | Fuse / Craft a Card: transient defs, fused scripts rebuilt from ids, `syncFusedScripts`, digest ids (R468) |
| `subsystems/heroPower.ts` | 590 | Heroic Power: 13 powers as Activate abilities (R752–R761) |
| `subsystems/activate.ts` | 568 | Activate abilities (R384), `activateAbility`, `activateActionsFor` |
| `subsystems/quests.ts` | 544 | Classic #90 In Too Deep quest machinery |
| `subsystems/scorer.ts` | 503 | Zephyrs scorer (R29) with dry-run plays on cloned states |
| `subsystems/callToChaos.ts` | 394 | Call to Chaos table and capped recursion |
| `subsystems/comboIndex.ts` | 266 | Combo-Index grades |
| `subsystems/aiPolicy.ts` | 205 | random-legal-action policy (§10.7, R44), `chooseAction`, My Pawn's AI turn |
| `subsystems/rotation.ts` | 197 | Silly Silas ring rotation |
| `subsystems/boardHistory.ts` | 194 | board snapshots for Rollback (R419) |
| `subsystems/lethal.ts` | 172 | projected attack damage for My Pawn |
| `subsystems/papaya.ts` | 141 | KY's Papaya curve targeting (fractions, gcd) |
| `subsystems/callToChaosPlus.ts` | 139 | C+ #73 Call to Chaos table |
| `subsystems/copiedText.ts` | 136 | Classic #57 Echo copied text |
| `subsystems/glitch.ts` | 132 | Glitch easter egg (reset, seat swap, boards, void) |
| `subsystems/twiceForward.ts` | 120 | C+ #74 |
| `subsystems/kyTest.ts` | 110 | KY's Test question bank |
| `subsystems/lastBoards.ts` | 106 | last boards setup input (R417) |
| `subsystems/perfectHand.ts` | 49 | perfect-hand scorer (C+ #27) |
| `subsystems/index.ts` | 43 | subsystems barrel |
| `subsystems/audit.ts` | 40 | Simplicity/Complexity Audit — reads catalog `loc` (see §8) |

---

## 2. CORE TYPES

### 2.1 `GameState` (`packages/engine/src/state.ts:450-581`)

All fields are JSON. Optional fields are **absent** (never `undefined`/`null`) when unused, and that absence
is load-bearing for the hash (comments: "so a game without … hashes as it did before the field existed").

| line | field | TS type |
| --- | --- | --- |
| 451 | `seed` | `string` |
| 452 | `rngCursor` | `number` (draws taken from the match rng) |
| 454 | `turn` | `number` (player-turn counter, 0 = setup) |
| 455 | `active` | `PlayerId` (`"p1" \| "p2"`) |
| 456 | `phase` | `"setup" \| "mulligan" \| "start" \| "main" \| "end" \| "over"` |
| 457 | `players` | `Record<PlayerId, PlayerState>` |
| 458 | `pending` | `PendingChoice \| null` |
| 459 | `triggerQueue` | `QueuedTrigger[]` (`{id, seq, instanceId, hook, resume}`, :327) |
| 461 | `declaredAttack` | `DeclaredAttack \| null` (`{id, attackerId, targetId, cancelled, by?, exitsFrom?}`, :347) |
| 463 | `work` | `WorkItem[]` (`{id, seq, owner, resume}`, :285) |
| 471 | `workCursor` | `number` (R113 insertion cursor) |
| 473 | `echoQueue` | `EchoItem[]` (`{id, seq, instanceId, controller, remaining}`, :276) |
| 475 | `dispatch` | `DispatchItem[]` (`{id, seq, event: GameEvent}`, :266) |
| 476 | `delayed` | `DelayedEffect[]` (`{id, seq, owner, at:{phase,player}, notBefore?, resume, watch?}`, :226) |
| 478 | `counters` | `{drawn, played, destroyed, exiled}` |
| 479 | `transientDefs` | `Record<string, CardDef>` (fused/crafted defs) |
| 481 | `reserved` | `{player, row, lane}[]` |
| 483 | `mulliganed` | `PlayerId[]` |
| 490 | `mulligan?` | `Record<PlayerId, MulliganSeat>` (`{prompt: PendingChoice, keep: string[] \| null}`) |
| 491 | `result` | `null \| {winner: PlayerId \| "draw", reason: GameOverReason}` |
| 493 | `nextId` | `number` (instance ids `c<n>`, prompt/trigger ids) |
| 495 | `nextSeq` | `number` (R68 creation order) |
| 497 | `applied` | `{nonce, events: GameEvent[]}[]` (nonce dedupe, ≤ `NONCE_HISTORY` 64; **excluded from hash**, but `viewFor` reads its events) |
| 503 | `castChain?` | `number` |
| 511 | `fieldExits?` | `{count, last: Record<string, number>, uncovered?: Record<string, {resumed, reported?, movedOn?}>}` |
| 518 | `homes?` | `HomeZone[]` |
| 526 | `castsResolving?` | `CastMode[]` |
| 531 | `announcing?` | `AnnounceRecord[]` (`{instanceId, player, faceDown?: true, countered?: true}`) |
| 533 | `lastSpell?` | `PlayRecord` (`{defId, radiant}`) |
| 539 | `heldDraws?` | `string[]` |
| 552 | `marks?` | `MarkRecord[]` |
| 559 | `lastBoards?` | `Partial<Record<PlayerId, LastBoardEntry[]>>` |
| 564 | `boardHistory?` | `BoardSnapshot[]` (whole `CardInstance` copies) |
| 567 | `systemPlays?` | `number` |
| 569 | `opening?` | `{decks: [string[], string[]], dealt?: PlayerId[]}` (**excluded from hash**) |
| 571 | `resets?` | `number` |
| 573 | `resetOwed?` | `true` |
| 575 | `seatSwaps?` | `number` |
| 580 | `glitchBoards?` | `Partial<Record<PlayerId, LastBoardEntry[]>>` |

### 2.2 `PlayerState` (`state.ts:384-448`)

`hero {health, armor}` · `mana {current, max, nextTurnMod, permMod}` · `hand`, `library`, `graveyard`, `exile`,
`resolving: CardInstance[]` · `units: (Pile|null)[]` (Pile = `CardInstance[]`, top first; 5 lanes) ·
`backrow: (CardInstance|null)[]` · `locks {units: boolean[], backrow: boolean[]}` · `mods: PlayerModifier[]` ·
`turnLog {playedIds, cardsPlayed, unspentAtEnd?, costsPaid?, playedByType?}` · `drawOffer {offeredTurn?, blockedUntil?}` ·
`fatigueCount` · `turnsStarted` · `aiTurn: boolean` · `handicap?: Handicap` (stored only when ≠ human) ·
`autoEndTurn?: false` (absent = on) · `backrowPiles?: CardInstance[][]` · `carried?: (CardInstance|null)[]` ·
`gameLog? {playedByTag, lastFaceUpPlay?}` · `draws? {turn, count}`.

`PlayerModifier` (`state.ts:170-224`) = `{id, expiry}` & one of 12 kinds: `costDiscount`, `echoNextSpell`,
`radiantFirstCheapCard`, `comboDraw`, `quickstrikerDamage`, `costRule`, `enchantNextSpell`, `replacePlays`,
`turnEnds`, `startOfTurnEffect` (holds a `Resume`), `healToDamage`, `heroArmor`. `ModifierExpiry` (:163) =
`thisTurn{turn}` | `nextTurnOf{player, fromTurn}` | `used` | `never`.

### 2.3 `CardInstance` (`state.ts:40-142`)

Required: `id` (`c<n>`), `defId`, `owner`, `controller`, `radiant`, `zone: Zone`, `damage`, `buffs {attack, health}`,
`grantedKeywords: Keyword[]`, `vanilla`, `costMod`, `counters {plague?, grade?}`, `memory: Record<string, unknown>`,
`exertion {attacked, switched, attacks?}`.
Optional: `position?`, `summonedTurn?`, `costOverride?`, `x?`, `embiggened?: boolean`, `statsOverride?`,
`armorOverride?`, `returnToHandAtEndOfTurn?: boolean`, `tauntSuppressedTurn?`, `faceUp?: boolean`,
`revealed?: boolean`, `lastDamagedBy?`, `divineShieldSpent?: boolean`, `markedDestroyed?: boolean`,
`rebornSpent?: boolean`, `knownAs? {defId, radiant}`, `tuning?: Tuning`, `brittle? {count, since, printed?: true}`,
`enchantments?: Enchantment[]`, `berserk?: true`, `timesPlayed?`.
Several optional booleans can be **present-and-false** (e.g. `faceUp: false`, see `_harness.ts` header) — a
port needs tri-state `Option<bool>` with identical presence to hash identically.

`Zone` (`packages/shared/src/catalog-types.ts:339`): `{z: "hand"|"library"|"graveyard"|"exile", player}` |
`{z: "field", player, row, lane}` | `{z: "resolving", player}` | `{z: "gone", player}`.

### 2.4 Actions (`packages/shared/src/actions.ts`, 97 lines) — 15 variants

`ActionBody` (:16-67): `mulligan{keep}`, `play{instanceId, zone?, x?, embiggen?, tributes?, targets?, modes?, plague?{from,tokens}}`,
`attack{attackerId, targetId}`, `switchPosition{instanceId}`, `activate{instanceId, ability?, targets?, modes?, tributes?}`,
`activatePower{instanceId, targets?}` (legacy alias, see §8), `answer{choiceId, selection: Selection[]}`, `offerDraw`,
`answerDraw{accept}`, `concede`, `endTurn`, `setAutoEndTurn{enabled}`, server-only `timeout`, `disconnectExpired{player}`,
`ceilingReached`. `Action = ActionBody & {playerId, nonce}` (:69). Const lists `NON_ACTIVE_ACTION_TYPES` (:79),
`PROMPT_OPEN_ACTION_TYPES` (:90). `Selection` (:9) = `instance{instanceId}` | `hero{player}` |
`zone{player,row,lane}` | `mode{option}` | `none`.

### 2.5 Events (`packages/shared/src/events.ts`, 498 lines) — 65 variants

Union at :6, exhaustive list `GAME_EVENT_TYPES` at :418 (compile-time exhaustiveness check :496):
cardPlayed, cardResolved, summoned, damage, healthLost, healed, divineShieldLost, destroyed, enteredGraveyard,
exiled, bounced, burned, fatigue, libraryOverflow, discarded, drawn, addedToHand, shuffledIn, buffed,
keywordGranted, counterChanged, costChanged, modifierChanged, radiantSet, transformed, fused, positionSwitched,
controlChanged, rotated, swapped, locked, trapFired, attackDeclared, attackCancelled, manaChanged, turnStarted,
turnEnded, turnAutoEnded, promptOpened, promptAnswered, drawOffered, drawAnswered, gameOver, cardAnnounced,
countered, stolen, unlocked, activated, animated, deanimated, crumbled, degraded, upgraded, numberChanged,
redirected, healthSet, questProgressed, questCompleted, rolledBack, chaosRolled, flickered, drawLimited,
turnCutShort, marked, glitched.
Also `TuningChange` (:386, 6 kinds), `LibraryOverflowOutcome` (:397), `GameOverReason` (:399: hero-death,
both-heroes-dead, concede, draw-accepted, turn-cap, disconnect, match-ceiling, voided).
Events carry engine-bookkeeping optional fields a view strips (e.g. `cardPlayed.arrivedDuring`, `exitsFrom`,
`formerId`). Events are stored in state (`dispatch[].event`, Resume `data.event`, `applied[].events`), so their
exact shape feeds the hash.

### 2.6 Prompts (`PendingChoice`, `state.ts:303-318`; kinds `catalog-types.ts:319`)

`PendingChoice {id, playerId, kind: PromptKind, prompt: string, options: PromptOption[], min, max, budget?, resume: Resume}`;
`PromptOption {key, label, selection: Selection, cost?, radiant?: true}` (:293).
15 `PromptKind`s (`prompts.ts:70` `PROMPT_KINDS`): discover, target, mode, mulligan, hand, zone, tribute,
direction, x, embiggen, number, answer, cell, reward, pick.
Prompt answerers registered by hook (`prompts.ts:427` map): `"play"` (`playSteps.ts:2059`),
`"plague:placement"` (`effects/plague.ts:232`), `"fuse:onto"` (`effects/fuse.ts:450`), `"@triggerCry"`
(`cryTrigger.ts:304`); everything else re-enters a card script step.
Data control keys inside `Resume.data`: `PROMPT_OWNER_KEY "__owner"`, `ANSWER_KEY "__answerKey"`, `SELF_KEY "__self"`
(`prompts.ts:62-141`).
`legalActions` answers for a prompt: `promptAnswers` (`prompts.ts:556`) enumerates option combinations
size `min..max` in index order, capped at `MAX_PROMPT_ANSWERS` 256; `pick` prompts use `pickAnswers`
(:591, wrap-around sets first, dedup by sorted index key).

### 2.7 Resume / work records (`state.ts:249-258`, `work.ts`)

`Resume {defId, hook: string, step: string, radiant, instanceId?, data: Record<string, unknown>}`;
`WorkItem {id, seq, owner, resume}`; `WorkPlan = Resume & {owner}` (`work.ts:66`); `PausedStep` (`work.ts:75-103`:
`from, targets, modes, part?, memo?, exitsFrom?, chosenFrom?, summoned?, resolving?, eventStay?`) stored at
`data.__paused`; `RunMarks` at `data.__run`.

`resume.hook` values a work item can carry:

| hook | owner module (handler registration) |
| --- | --- |
| `"@setup"` (steps deal, quickdraw, mulligan, startOfGame, suspended) | `setup.ts:135, :471` |
| `"@deaths"` (step hooks) | `stateCheck.ts:161, :467` |
| `"@trapWindow"` (step window) | `traps.ts:115, :766` |
| `"@trapFiring"` | `traps.ts:356, :433` |
| `"play"` (`PLAY_WORK_KIND`) | `playSteps.ts:167, :1894` |
| `"@drawChain"` (chain), `"@drawCount"` (draws) | `draw.ts:303/338, :631/632` |
| `"@attackWindow"` (combat), `"@forcedRun"`, `"@forcedRandom"`, `"@afterAttack"` | `combat.ts:406/771/866/916; :592/813/888/1043` |
| `"@startOfTurn"` (brittle, animate, delayed, settle, triggers, main) | `turn.ts:223, :487` |
| `"@endOfTurn"` (triggers, window, delayed, cleanup, next) | `turn.ts:545, :783` |
| `"@activate"` | `subsystems/activate.ts:380, :504` |
| `"@aiTurn"` | `subsystems/aiPolicy.ts:165, :205` |
| default: a card's own step — `Script` hook names (`cry`, `death`, `delayed`, `startOfTurn`, `endOfTurn`, `resume` + `step` key into `Script.resume`, …), a `TriggerDef.id`, or `activation:<id>` | `prompts.ts:912` (`registerDefaultWorkHandler`), lookup `work.scriptStepFor` (`work.ts:496`) |

Delayed-effect hooks (in `DelayedEffect.resume`): `"delayed"`, `"@delayedDestroy"` (steps scope/unit),
`"@delayedDiscardHand"` (`effects/delay.ts:164-171, :275-281`).
Other `data` keys (all must round-trip byte-identically): `__paused`, `__run`, `__part`, `__partDepth`,
`__remembered` (`work.ts`), `event` (`work.ts:453`), `__play` (`playSteps.ts`), `__manaBeforePlay` (`resolve.ts:27`),
`__stacked` (`zones.ts:144`), `__stranded` (`carriers.ts`), `__castOnDraw` (`drawComplete.ts`),
`__declarationSlices` (`playChoices.ts`), `__ingredients` (`scripts.ts`), `__copiedText`, `__activation`,
`@suspendedCast`, `@killCredit`, `replaced`, `pass`, `attack`, `cry`, `run`, `power`, `picks`, `quest`, `plays`,
`fuseOn`, `papayaPoints`, `chaosChain`, `numberOf`, `activations` (the last group partly live in `CardInstance.memory`).

### 2.8 Script contract (`packages/engine/src/script.ts`, 577 lines)

* `EffectContext` (:12-96): `state, rng, events, eventsFrom, exitsFrom?, chosenFrom?, eventStay?, summoned?,
  selfResolving?, controller, self: CardInstance|null, defId?, radiant, targets: Selection[], modes: string[],
  x, embiggened, data: Record<string, unknown>, manaBeforePlay?`.
* `Effect` (:102-117) = `{ kind: string; apply(ctx): void; expand?(ctx, memo): EffectPart }` — **a closure object**,
  never in state. `EffectPart = {effects, memo?}` (:120). `Hook = (ctx) => Effect[]` (:122).
* `TriggerDef` (:125) `{id, on: GameEventType[], when?(ctx & {event}) => boolean, run(ctx & {event}) => Effect[]}`.
* `StatMod` (:139), `AuraHook` (:151), `ConditionContext` (:280), `ConditionHook`, `PreviewHook` (:312).
* `StaticFlags` (:157-269): castOnDraw, quickdraw, infiniteReserves, neverDefense, deftDuelist (legacy), echo,
  echoGrant, quickstriker (bool|number), giftedProgram, tribute, tributeWorth, tributeEnemies,
  enemyTributeHandsOver, antiOneshot, heroArmor (bool|number), countsPlays, carrier, fusesCarried,
  radiantPlaysTagged, copiesLastSpell, cantBeAttacked, attackedOnlyFromLane, cantAttackOrBeAttacked,
  neverBerserk, healToDamage.
* `Script` (:314-471) fields and signatures:
  - `cost?({state, instance}) => number`
  - `cry?, death?, startOfGame?, delayed?, startOfTurn?, endOfTurn?, activate?, onPlayHook?, afterAttack?,
    startOfOpponentTurn?: Hook`
  - `resume?: Record<string, Hook>`
  - `setStat?({state, self, radiant}) => {attack?, maxHealth?}`
  - `aura?: AuraHook`
  - `triggers?, handTriggers?, deckTriggers?, graveyardTriggers?: TriggerDef[]`
  - `staticFlags?: StaticFlags`; `targets?: TargetDecl[]`; `modes?: ModeDecl[]`
  - `conditionMet?: ConditionHook`; `preview?: PreviewHook`
  - `activations?: ActivationDecl[]` (:482: `{id, label, uses: number|"unlimited", cost?{mana?, discardRandom?, tribute?, tributeSelf?, tributeExcludesSelf?}, targets?, modes?, canActivate?, has?, run}`)
  - `targetChecks?: Record<string, TargetCheck>`
  - `costAura?(CostAuraArgs) => CostAura[]`; `graveyardPlay?(CostAuraArgs) => GraveyardPlayPermission[]`
  - `targetingDiscards?`, `plagueMultiplier?` (`({state,self,radiant}) => number`)
  - `recordsPlayAs?(...) => PlayRecord|null`; `drawLimit?: DrawLimitHook`; `replacements?: ReplacementDef[]`;
    `heroGuard?(...) => {cap?, divisor?}[]`; `conditionalKeywords?(...) => Keyword[]`; `quests?: QuestBook`;
    `tributeWhen?(...) => boolean`; `wouldCounter?({state, self, controller, player, costPaid}) => boolean`
* `CardScripts = {base: Script; radiant: Script}` (:575); `EMPTY_SCRIPT` (:577); `ACTIVATION_HOOK_PREFIX "activation:"` (:520).
* Hook usage across the 318 card files (files containing the key): cry 159, resume 28, staticFlags 23,
  targets 20, triggers 19, endOfTurn 16, startOfTurn 14, modes 13, conditionMet 13, preview 13, death 12,
  activations 8, aura 7, replacements 7, cost 5, targetChecks 5, handTriggers 3, graveyardPlay 3, drawLimit 2,
  costAura 2, and 1 each for startOfGame, delayed, setStat, targetingDiscards, recordsPlayAs, heroGuard,
  conditionalKeywords, afterAttack, plagueMultiplier, deckTriggers, graveyardTriggers, quests, tributeWhen,
  wouldCounter, startOfOpponentTurn; **0** for `activate` and `onPlayHook`.

### 2.9 Effect verbs exported by the barrel (`packages/engine/src/effects/index.ts`)

199 value exports + 50 type exports. ~188 distinct `kind: "…"` strings in `effects/`. Grouped by source file:

| file | value exports |
| --- | --- |
| targets | adjacentTo, cardsInScope, instanceOf, matchesScope, playerOf, resolveTarget, sidesOf |
| summon | fillBoard, recruit, summon, summonCopy, summonRandom, recruitAll |
| destroy | destroy, destroyAdjacentTo, destroyAll, sacrifice |
| move | EXILE_ZONE_ORDER, bounce, bounceAll, counter, discard, discardHand, discardRandom, exile, exileAdjacentTo, exileAll, exileHand, exileMatching, counterPlay |
| steal | steal, stealAll |
| transform | transform, vanilla, swapBook, transformBeneath, transformRandom |
| heal | heal |
| choose | chooseFromHand, chooseMode, chooseTarget, chosenOptions, discoverFromCatalog, discoverFromGraveyard, discoverFromLibrary, targetsInScope, ANSWER_OPTION_IDS, answeredCorrectly, chooseAnswer, chooseCell, chooseCostInHand, chooseNumber, choosePick, chooseReward, chosenCells, chosenNumber, matchesLibraryFilter |
| counters | clearPlague, lock, plague, unlock |
| mana | gainMana, nextTurnMana, refreshMana |
| damage | damage, damageAll |
| loseHealth | loseHealth |
| draw | draw, drawFromLibrary |
| addToHand | addRandomFromCatalog, addRandomFromGraveyard, addToHand |
| library | exileBottomOfLibrary, exileRandomFromLibrary |
| shuffleInto | shuffleCopiesOfSelf, shuffleInto |
| radiant | radiantChance, setRadiant, setRadiantRandom |
| position | switchAllPositions, switchPositionOf |
| cost | setCostMod, setCostOverride |
| swap | SWAP_ROWS, swap, swapBoard, swapHealth, swapLibrary |
| buff | buff, buffAllUnits, grantKeyword, grantRandomKeywords, buffCards, grantKeywordCards |
| combat | aiPlaysOutTurn, cancelAttack, forcedAttackOwnHero, forcedAttackRandom, forcedAttacks, forcedAttacksOn |
| delay | DELAYED_HOOK, THIS_TURN, delay, destroyAtNextTurnStart, discardHandAtTurnEnd, forRestOfGame |
| playerMods | addPlayerModifier |
| coins | flipCoins, flipCoinKeyword |
| fuse | fuseCards, FUSE_ONTO_HOOK, fuseGenerated, fuseOntoYourCard, fuseRandomInto |
| rotate | rotate |
| memory | remember, rememberRandom |
| each | forEachCard |
| cardScope | cardsInCardScope, matchesCardScope, readersOf, unreadableBy |
| tune | NUMBER_CARD_KEY, applicableChanges, chosenTuningNumber, degrade, discoverNumber, reachedCards, setNumber, tuneOnce, upgrade |
| brittle | gainBrittle, giveBrittle |
| enchant | enchant |
| animate | animate |
| flicker | flicker, flickerCard |
| reveal | reveal |
| locks | lockLane, lockOwnZone, lockPlayedZone, lockRandomZone, unlockAll |
| cast | addCostRule, cast, castEach, castNew, castRandom, enchantNextSpell |
| turnEnd | endTurn, endTurnAfterActions |
| health | convertHealing, setHealth |
| split | damageSplit |
| statuses | goBerserk, mayAttackAgain |
| killCredit | withKillCredit |
| give | drawFromOpponent, giveFromHand, takeFromLibrary |
| cry | hasTriggerableCry, triggerCry |
| summonThis | summonThis |
| plague | PLAGUE_PLACEMENT_HOOK, consumePlague, placePlague, placePlagueEach, placePlagueRandom, placePlagueTokens |
| handExile | exileRandomFromHand |
| shuffleCard | shuffleCardInto |
| chooseWhere | chooseTargetWhere |
| afterCheck | afterStateCheck |
| shuffleRandom | shuffleRandomFromCatalog |
| libraryCopies | addLibraryCopies |
| datacenter | destroyFieldSpellsAndHit, drawWhileCheap, fieldSpellsDoomed |
| perks | discountRandomInHand, gainHeroArmor |
| fruit | addRolledGrapes, cardThisDrawPutInHand, damageEnemyOrHealFriend, drawPriced, replaceHandWithRandom, rollGrape |
| randomPicks | buffRandomUnit, returnRandomFromGraveyard |
| drawWhile | drawWhile |
| lastBoard | LAST_BOARD_CARD_COST, LAST_BOARD_DISCOVER_OPTIONS, addFromLastBoard, addRandomFromLastBoard, discoverFromLastBoard |
| rounds | castRoundsUntilDeath, damageRoundsUntilDeath |
| ../subsystems/perfectHand | replaceHandWithPerfect |
| ../subsystems/boardHistory | rollBack |
| ../subsystems/glitch | glitch |

Types: `BoardScope, PlayerSpec, TargetSpec` (targets; `TargetSpec` = `self|selfHero|enemyHero|instance{instanceId}|chosen{index?}`,
`effects/targets.ts:15`), `RecruitFilter, StatsOverride, SummonArgs, SummonCopyArgs, SummonPlacement, RecruitSource`,
`CostFilter, ExileZone, CounterDestination`, `StealTarget`, `TransformTarget`, `HealArgs`,
`DiscoverOffer, LibraryFilter, TargetScope, CellScope, PickFilter, PileSpec`, `ZoneSpec`, `DamageAllArgs, DamageEffectArgs`,
`RadiantTarget, RadiantZone`, `SwapWhat`, `BuffAmount`, `ForcedAttackerFilter, ForcedSide, ForcedTarget`, `DelayAt`,
`CardScope, CardZone, Readers, ScopedCard`, `TuneArgs, TuneDirection, TuneRow`, `BrittleTarget`, `AnimateArgs`,
`LaneSpec, ZoneScope`, `CastDef, CastHow, CostRuleSpan`, `TakenRiders`, `FuseInto, FuseOntoPile`, `FieldSpellSide`.
Representative constructor shape (`effects/damage.ts`): `damage(args) => ({ kind: "damage", apply(ctx) { … dealDamage(ctx, …) } })`.

`EngineSink = {state, events, rng}` (`resolve.ts:13`) is what every engine mutator takes.

---

## 3. DETERMINISM HAZARDS for an exact port

### 3.1 RNG (`packages/engine/src/rng.ts`, 86 lines)

* `seedToInt` = xmur3 over `seed.charCodeAt(i)` (UTF-16 code units), `Math.imul`, `>>>`. Port with `u32`
  wrapping ops over `encode_utf16()`.
* `valueAt(seedInt, n)` = mulberry32 **random-access by cursor**: `a = (seedInt + Math.imul(n, 0x6d2b79f5)) >>> 0`,
  then `Math.imul(a ^ a>>>15, a|1)`, `a ^= a + Math.imul(a ^ a>>>7, a|61)` (the `+` is a double sum then
  ToInt32 = wrapping i32 add), `((a ^ a>>>14) >>> 0) / 4294967296` → f64 in [0,1).
* `int(n) = Math.floor(next() * n) % n` (0 for n ≤ 0); `pick`, `shuffle` = Fisher–Yates top-down
  (`j = int(i+1)` for i = len-1..1); `coin = next() < 0.5`; `chance(p) = next() < p`; `lucky(x, roll, better)`
  = 1 + trunc(x) rolls.
* The match rng is rebuilt each `reduce` from `(state.seed, state.rngCursor)` (`reduce.ts:436`) and the cursor
  written back (`reduce.ts:451`). Separate streams (never touch the cursor): `${seed}:instance-ids${stream}`
  (`state.ts:890`, deck id numbering, R223), `${seed}:instance-ids:${nextId}` (`state.ts:950`, `numberingOrder`),
  `"zephyrs-dry-run"` cursor 0 (`subsystems/scorer.ts:346`). Floats used against the rng: `AI_END_TURN_PROBABILITY = 0.1`.

### 3.2 `replay.ts` hash (`packages/engine/src/replay.ts:12-37`)

```ts
function canonical(value: unknown): string {
  if (value === null || typeof value !== "object") return JSON.stringify(value) ?? "null";
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  const entries = Object.entries(value).filter(([, v]) => v !== undefined)
    .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
    .map(([k, v]) => `${JSON.stringify(k)}:${canonical(v)}`);
  return `{${entries.join(",")}}`;
}
export function hashState(state: GameState): string {
  const { applied: _applied, opening: _opening, ...rest } = state;
  const text = canonical(rest);
  let hash = 0x811c9dc5;
  for (let i = 0; i < text.length; i += 1) { hash ^= text.charCodeAt(i); hash = Math.imul(hash, 0x01000193) >>> 0; }
  return hash.toString(16).padStart(8, "0");
}
```

* **What**: 32-bit FNV-1a over the **UTF-16 code units** of a canonical JSON of the whole `GameState` minus
  `applied` and `opening`; keys sorted by UTF-16 code-unit order; `undefined` members dropped (arrays: an
  `undefined` element prints `null`); primitives via `JSON.stringify` (JS number formatting, JS string escaping);
  output 8 lowercase hex chars.
* **Reproducible in Rust if**: (a) every field serialises to the same JSON value tree — same presence/absence of
  optional fields, `null` where TS stores `null`, present-`false` where TS writes `false`; (b) numbers print as JS
  does (all engine state numbers are expected to be integers — no non-integer write into state was found, but
  this was not verified by running; `memory`/`data` are `unknown` bags); (c) strings escaped as `JSON.stringify`
  (`\"`, `\\`, `\b \f \n \r \t`, other C0 as `\u00xx` lowercase; non-ASCII emitted raw); (d) FNV over UTF-16
  units — state strings contain non-ASCII BMP characters (prompt labels / card text carry `× − – ♾ U+FE0F ³ ²`;
  `viewFor.ts:559` writes `"−"`), so hashing UTF-8 bytes would diverge. Struct field order is irrelevant (keys are
  sorted). All map keys seen are ASCII, so byte order = UTF-16 order.
* **Consumers that pin TS hashes**: `packages/cards/test/hotseat-replay.test.ts:115` `EXPECTED_HASH = "a798906b"`
  over the committed log `packages/cards/test/fixtures/01-hotseat-full-game.json` (seed `01-hotseat`, 37 actions);
  fuzz gates (live hash vs fold hash, same process); e2e specs 01/13/22 (`cy.task("replayHash")`, browser vs Node
  fold); **practice saves on users' devices store the hash** (`apps/web/src/practice/core.ts:252, :279` — a save
  that does not fold to its stored hash is rejected); `packages/ai` (`gate.ts`, `match.ts`).

### 3.3 `cloneState` and other JSON round-trips

* `cloneState = JSON.parse(JSON.stringify(state))` (`state.ts:957`) — called by `reduce` (:434) and `beginGame`
  (:458) on every action, `scorer.ts:345, :375`, AI (`packages/ai`, 2 sites). Semantics a port must mirror:
  drops `undefined`, `-0 → 0`, `NaN/Infinity → null`.
* Other deep copies via JSON: `viewFor.ts:1164` (defs into view), `setup.ts:352`, `stateCheck.ts:221, :353, :505`
  (death snapshots), `traps.ts:417`, `tuning.ts:168`, `combat.ts:963` (attacker snapshot), `replacements.ts:334`,
  `subsystems/boardHistory.ts:32`.
* **JSON.stringify as equality / key** (insertion-order dependent, NOT canonical): `enchantments.ts:27`
  `sameEnchantment = JSON.stringify(a) === JSON.stringify(b)`; `subsystems/scorer.ts:246-247` map keys from
  `JSON.stringify(action.tributes)`/`action.zone`.

### 3.4 Sorting (18 `.sort(` sites in engine src)

JS `Array.prototype.sort` is stable; port with stable `sort_by`. String `<` compares UTF-16 code units.

| site | comparator |
| --- | --- |
| `catalog.ts:270` | §5.1 pool order: set rank, then `Number.parseFloat(index)` (tokens → +∞), then index string, then id |
| `subsystems/scorer.ts:497` (`compareScored` :483) | float `score` desc, `parseFloat(index)`, index string, id |
| `subsystems/scorer.ts:260` | rank asc, tribute desc, original index (stable ties) |
| `subsystems/perfectHand.ts:27` | score desc, def id |
| `playChoices.ts:857` | offered rank (`indexOf` of `selectionKey`, missing = `MAX_SAFE_INTEGER`), then original position |
| `triggers.ts:231` | library holders by `creationNumber(id)` (`/^c(\d+)$/` → int, else `MAX_SAFE_INTEGER`) then id |
| `effects/choose.ts:486` | library pile by `creationNumber` |
| `effects/choose.ts:257` | numeric cost keys of a `Map` |
| `effects/libraryCopies.ts:27` | defId string |
| `effects/fuse.ts:351` | name, defId, radiant, id (`compareText` code-unit) |
| `ownLibrary.ts:87` | `compareEntries` (printed cost, name, id, base before Radiant) |
| `modifiers.ts:175, :239`, `turn.ts:141` | `seq` |
| `prompts.ts:547, :604` | position / numeric indices |
| `replay.ts:18` | hash key order |
| `instanceView.ts:49` | default `.sort()` on `keywordKey` strings |

`localeCompare` appears only in comments saying it is avoided (`effects/fuse.ts:317`, `ownLibrary.ts:49`).
`Number.parseFloat` is prefix-lenient (`"12abc"` → 12); catalog indexes today are `^\d+(\.\d+)?$` or `T-…`
(17 token indexes), so no current index hits the lenient case.

### 3.5 Map / Set / object-key order

* 13 `new Map`, 61 `new Set` in engine src. Iteration order (insertion) matters at:
  `playChoices.ts:1106` and `effects/tune.ts:170` (`[...new Set(...)]` dedupe keeps first-seen order),
  `query.ts:229`, `subsystems/fuse.ts:663` (`combineObjects` key union), `ownLibrary.ts:87` (values then sorted),
  `effects/choose.ts:257` (keys then sorted), `gameSummary.ts:72`, `restrictions.ts:92` (registry values),
  `playChoices.ts:1255`. Use insertion-ordered sets/maps (IndexMap/IndexSet or Vec) in Rust.
* `Object.keys/entries/values` (25 sites): `catalog.ts:33` (`defByIndex` = first match in catalog insertion
  order), `catalog.ts:268`, `tuning.ts:127-194`, `work.ts:261/285/289` (`rerootRemembered`/`memoryOfPart` rename
  memory keys while iterating), `traps.ts:650`, `subsystems/quests.ts:164`, `subsystems/fuse.ts:336-337`
  (fused id number `t-<n>` = `Object.keys(transientDefs).length + 1`), `subsystems/fuse.ts:949`,
  `subsystems/glitch.ts:129`, `subsystems/aiPolicy.ts:104`, `subsystems/lastBoards.ts:62`, `viewFor.ts:1147, :1172`.
  JS orders integer-like keys first (ascending) — no integer-like keys were found in state maps (ids are `c<n>`,
  `t-<n>:…`, `core-…`), but `Record<string, unknown>` bags are unconstrained.
* `catalog.json` is an object keyed by id; JS keeps insertion order (`core-001` … `classicplus-t-ai-10`).

### 3.6 Numbers

* `Math.imul`: `rng.ts` (5), `replay.ts:33`, `subsystems/fuse.ts:360-370` (`fusedDigest` = cyrb53-style 2×32-bit
  over `charCodeAt`, hex-padded; names digest fused ids `t-<n>:#<hex>` when the readable id exceeds
  `FUSED_ID_CAP` 120).
* Float ops on game numbers: `damage.ts:151` `Math.ceil(afterArmor / divisor)`; `params.ts:59`
  `Math.round(size / largeDivisor)` (JS rounds .5 up; operand non-negative so equals Rust `round`);
  `graveyardPlay.ts:117` `Math.floor(price / PLAGUE_TOKEN_MANA)`; `zones.ts:28` `(lanes+1)/2`;
  `subsystems/papaya.ts:67-73` gcd and fraction reduce; `subsystems/scorer.ts:183` `stats / max(1, cost)` (float
  score, only used for ordering). `%` sites: `playChoices.ts:968/978`, `prompts.ts:623`, `zones.ts:76`,
  `effects/move.ts:162-163`, `subsystems/glitch.ts:40`, `subsystems/twiceForward.ts:94`, `reduce.ts:471` (bitmask
  `1 << i`, guarded ≤ 8 ids by `MAX_MULLIGAN_SUBSETS` 256).
* 80 `Math.trunc`, 136 `Math.max`, 48 `Math.min` — mostly defensive coercion of `unknown` data.
  `Math.min(...[])` would be `Infinity` (guarded at `damage.ts:128`, `graveyardPlay.ts:141`, `scorer.ts:239`,
  `subsystems/fuse.ts:651`). `Number.MAX_SAFE_INTEGER` used as a sentinel 5×. No `Date`, `Math.random`,
  `structuredClone`, `Symbol`, `for…in`.
* 92 `delete obj.field` statements: absent vs present matters for the hash.

### 3.7 `legalActions` order (trace parity)

`reduce.ts:492` `eachLegalAction`: (prompt open) prompt answers then `concede`; (mulligan open) 2^n keep-subsets
by bitmask (or `[all, none]` if > 256) then `concede`; else `answerDraw` true/false if offered; non-active or
non-main → `concede`; else hand cards in hand order (`playChoices.playActionsFor`, R81 crossing bounded by
`MAX_CHOICE_COMBINATIONS` 64 with a "wheel" enumeration `playChoices.ts:960-980`), graveyard plays, per active
unit (lane order) attacks (`combat.attackTargets` order) then switch, then activations for unit slots then
backrow slots (`slotsOf`), `offerDraw`, `endTurn`, `concede`. Errors are English strings; 83 test assertions
match on `.error` text (engine + cards tests).

---

## 4. CONFIG (`packages/engine/src/config.ts`, 469 lines)

* 92 `export const`, 1 `export function` (`FATIGUE_DAMAGE` is an arrow const; `fib(index)` is the function),
  3 `export type` (`Handicap`, `Difficulty`, `KyTestReward`).
* Shapes: mostly plain numbers (`DECK_SIZE 20`, `MAX_MANA 4`, `HERO_HEALTH 30`, `TURN_CAP_PLAYER_TURNS 60`,
  `HAND_CAP 10`, `UNIT_ZONES 5`, `BACKROW_ZONES 5`, `LIBRARY_CAP 60`, `NONCE_HISTORY 64`, caps…); booleans
  (`LANE_RESTRICTED_ATTACKS false`, `CRY_ON_PLAY_ONLY true`); string ids (`COIN_DEF_ID "core-t-coin"`,
  `GLITCH_DEF_ID "classic-t-glitch"`); string-literal "decide" switches (`ROTATION_RING "two-rings"`,
  `GENN_GREED_EXILES "odd"`, `FIENDER_STATS_MODE "printed-plus-sum"`, `MULLIGAN_ORDER "draw-then-shuffle"`);
  arrays (`OPENING_DRAW [3,4]`, `OPENING_COINS [0,1]`, `FIB`, `GLITCH_NUMBERS`, `RANDOM_KEYWORD_POOL`,
  `KY_TEST_EASY_MISSES`, `GLITCH_OUTCOMES`, `TUNE_HARMFUL_KEYWORDS`, `SYSTEM_CARD_DEF_IDS`, `POOL_TOKEN_TAGS`,
  `LAST_FACE_UP_SKIPPED_TAGS`); objects (`ANTI_ONESHOT_CAP {base, radiant}`, `HERO_ARMOR`,
  `QUICKSTRIKER_COMBO_MULTIPLE`, `PARAM_DEFAULT_STEP`, `HUMAN_HANDICAP`, `AI_DIFFICULTY` record of 3 handicaps,
  `AI_TUTORIAL`, `KY_TEST_REWARDS`, `KY_TEST_EASY_ADDENDS`, `GRAPE_ODDS [{defId, percent}]`, `BERSERK_MARK`);
  a float `AI_END_TURN_PROBABILITY 0.1`; a function `FATIGUE_DAMAGE(n)`.
* Numeric constants **outside** config.ts (rule 9 exceptions): `viewFor.ts:108 VIEW_EVENT_LIMIT 32`,
  `playChoices.ts:409-410 RADIANT_SHEEP_TRIBUTE_VALUE/SHEEP_TRIBUTE_VALUE`, `effects/lastBoard.ts:20,23`,
  `stateCheck.ts:474 STATE_CHECK_PASS_CAP 100`, `work.ts:69 MAX_WORK_STEPS 500`, `prompts.ts:92 MAX_PROMPT_ANSWERS 256`,
  `reduce.ts:63 MAX_MULLIGAN_SUBSETS 256`, `subsystems/lastBoards.ts:26`, `subsystems/callToChaos.ts:40-60` (8),
  `subsystems/scorer.ts:44-60, :210` (`SCORER_WEIGHTS`, `SCORER_LOW_HEALTH`, `SCORER_DRY_RUN_PLAYS`),
  `subsystems/heroPower.ts:80-98` (11), `subsystems/aiPolicy.ts:22 AI_PLAYOUT_STEP_CAP 500`,
  `subsystems/comboIndex.ts:30-38` (4), `subsystems/fuse.ts:87, :90`.
* What `apps/web/src` imports from the engine (non-test, value imports):
  - `@jackioh/engine/config`: `DECK_SIZE` (`game/decks.ts`, `practice/core.ts`, `practice/decks.ts`, `routes/play.tsx`;
    re-exported with `MAX_COPIES` by `game/deckbuilder/deckSize.ts`), `MAX_COPIES` (`game/decks.ts`),
    `DIFFICULTIES` (`practice/PracticeSetup.tsx`, `practice/Tier.tsx`, `practice/resume.ts`, `routes/practice.tsx`,
    `tutorial/TutorialResult.tsx`), `AI_DIFFICULTY` (`practice/PracticeSetup.tsx`, `practice/core.ts`),
    `AI_TUTORIAL` (`practice/core.ts`), `HUMAN_HANDICAP`, `DRAWS_PER_TURN` (`practice/PracticeSetup.tsx`),
    `TURN_CAP_PLAYER_TURNS` (`tutorial/TutorialResult.tsx`), `HERO_HEALTH` (`audio/musicDirector.ts`),
    `MAX_MANA` (`tutorial/scripts/basics.ts`), `COIN_DEF_ID` (`tutorial/scripts/traps.ts`),
    `GLITCH_DEF_ID` (`cards/glitch.ts`); types `Handicap`, `Difficulty` (`game/engine.ts` and others).
  - `@jackioh/engine` root: `game/engine.real.ts` (`import * as engine`; uses `createGame, beginGame, reduce,
    legalActions, viewFor, hashState, registeredCatalog` and `registerAll` from `@jackioh/cards`);
    `practice/core.ts` (`beginGame, createGame, createRng, hashState, lastBoardFor, legalActions, reduce,
    registeredCatalog, seatPlayedBy, viewFor`, types `GameState, Rng`, and `@jackioh/ai`);
    `tutorial/harness.ts` (`createRng`).
* Elsewhere: `packages/validator/src/config.ts:14` re-exports `DECK_SIZE, MAX_COPIES`; `packages/ai/src` uses 38
  engine values (`unitView`, `legalActions`, `createRng`, `activeUnitsOf`, `cloneState`, `reduce`, `fold`,
  `hashState`, `subsystems.*`, …) and reads/rewrites raw `GameState` objects for redaction; `apps/server/src`
  has 19 `@jackioh/engine` + 4 `/config` imports (engine through `src/match/engine.ts` port; `fold` used for
  recovery in `src/match/actor.ts:132`, `src/match/registry.ts:136`).

---

## 5. CARD SCRIPTS (`packages/cards`)

### 5.1 Layout and counts

* `src/scripts/` (core, 105 numbered files incl. tokens like `093-1-combo-fodder.ts`, + 6 shared tokens
  `t-bread t-coin t-felinor t-ghoul t-rush t-sheep`), `src/scripts/classic/` (91, incl. `t-glitch-glitch.ts`),
  `src/scripts/classic-plus/` (116, incl. `t-ai-01…t-ai-10`) = **318** = 268 cards + 50 tokens; plus `_generated.ts` (654).
* Lines: total 16,181; root 7,447, classic 4,865, classic-plus 3,869. min 13, p25 29, **median 41**, p75 62,
  p90 89, max 277, mean 50.9; 201 files ≤ 50 lines, 302 ≤ 100, 316 ≤ 200.
* Top 10: `051-kys-private-tutor.ts` 277, `classic/090-in-too-deep.ts` 255, `050-k-pop-fanatic.ts` 184,
  `085-unlicensed-experimentation.ts` 176, `022-carnivorous-cube.ts` 159, `083-transmogulate.ts` 146,
  `classic/008-pickle.ts` 144, `039-recycling-initiative.ts` 141, `060-bear-honeypot.ts` 134,
  `031-kys-math-equation.ts` 134 (then `096-my-pawn.ts` 130, `018-bread-and-butter.ts` 126).
* File contract: `export const def = cardDef("<catalog id>")`, `export const base: Script`, `export const radiant: Script`.

Small script verbatim (`src/scripts/classic-plus/011-anime-armor.ts`, 14 lines):

```ts
// C+ #11 Anime Armor (SPEC §8.7 row 11). (2) Unit, Rare, 4/4 → 8/8 (Radiant: Reborn).
// Aura: while it acts on the field its controller's hero takes at most {cap} from each damage instance
// (§4.4 step 3, E6's per-hit cap: the lowest cap wins beside Anti-oneshot Armor's). Lose health (R18)
// and Set health are not damage and are not capped.

import { param, type Script } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-011");

export const base: Script = { heroGuard: (args) => [{ cap: param(args, "cap") }] };

/** The Radiant face adds Reborn and doubles the stats: catalog data only. */
export const radiant: Script = base;
```

Typical targeted script (`src/scripts/002-bigot.ts`, code part):

```ts
import type { Script } from "@jackioh/engine";
import { destroy, destroyAll } from "@jackioh/engine/effects";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-002");

export const base: Script = {
  targets: [
    { kind: "target", min: 1, max: 1, filter: { side: "enemy", of: ["unit"], notTags: ["Human"] } },
  ],
  cry: () => [destroy({ target: { of: "chosen" } })],
};

export const radiant: Script = {
  cry: () => [destroyAll({ side: "enemy", rows: ["units"], notTags: ["Human"] })],
};
```

Large script, trimmed (`src/scripts/051-kys-private-tutor.ts`, 277 lines; ~95 lines of header comment, code below):

```ts
import { defOf, effectiveCost, isUnitToken, zoneCards, type CardInstance } from "@jackioh/engine";
import { addToHand, chooseMode, chosenOptions, discoverFromLibrary } from "@jackioh/engine/effects";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-051");
const NOTEBOOK = "core-051-1";
const REVEAL_COUNT = 3;
const TYPE_OPTIONS = ["Spell", "Unit", "Field Spell", "Trap"] as const;
const BRACKET_OPTIONS = ["0-1", "2", "3", "4+"] as const;
const TYPE_KEY = "type";
// … typesFor / rangeFor / matchesType (defOf) / matchesBracket (effectiveCost) / libraryCards (zoneCards) …
const startStep: Hook = (ctx) => {
  const library = libraryCards(ctx);
  const options = TYPE_OPTIONS.filter((option) => library.some((card) => matchesType(ctx, card, option)));
  if (options.length === 0) return [addToHand({ defId: NOTEBOOK })];
  return [chooseMode({ options: [...options], step: "bracket", prompt: "Choose a card type" })];
};
const bracketStep: Hook = (ctx) => {
  const type = typeOptionOf(chosenOptions(ctx)[0]);
  if (type === null) return [];
  // … filter brackets with a match …
  return [chooseMode({ options: [...options], step: "reveal", data: { [TYPE_KEY]: type },
                       prompt: `Choose a cost bracket for ${type}` })];
};
const revealStep: Hook = (ctx) => {
  // … read ctx.data[TYPE_KEY] and the chosen bracket …
  return [discoverFromLibrary({ step: "take", count: REVEAL_COUNT, player: "self",
    filter: { type: typesFor(type), costRange: rangeFor(bracket) },
    prompt: "Reveal 3 cards from your deck; choose one to add to your hand" })];
};
const takeStep: Hook = () => [addToHand({ instance: { of: "chosen" } })];
const steps: Record<string, Hook> = { bracket: bracketStep, reveal: revealStep, take: takeStep };
export const base: Script = { cry: startStep, resume: steps };
const RADIANT_FLAGS: StaticFlags = { echo: 1 };
export const radiant: Script = { cry: startStep, resume: steps, staticFlags: RADIANT_FLAGS };
```

(Prompt continuation = named `step` string + `data`, re-entered through `Script.resume[step]`.)

### 5.2 Registry mechanism

* `packages/cards/scripts/gen-registry.ts` writes `src/scripts/_generated.ts`: one `import * as mNNN_slug` per
  script, in SPEC §5 order, and `SCRIPT_MODULES` array. Regenerated by `pnpm --filter @jackioh/cards gen`,
  typecheck, and the vitest `globalSetup` (`test/globalSetup.ts`).
* `src/index.ts` (103 lines): `buildRegistry(SCRIPT_MODULES)` keyed by `mod.def.id` (throws on unknown id or
  duplicate); `CARDS`; frozen `SCRIPTS: Record<id, {base, radiant}>`; `registerAll()` →
  `registerCatalog(CATALOG, CATALOG_VERSION)` + `registerScripts(SCRIPTS)` (idempotent by identity). Engine side:
  `catalog.ts:16-21` (`let registered`, `version`), `scripts.ts:7-15` (`let registered`; missing id →
  `EMPTY_SCRIPT`).
* `src/catalog-data.ts` (44 lines): the only reader of `catalog.json`; `CATALOG`, `CATALOG_VERSION = "v0.2.11"`,
  `CATALOG_IDS` (insertion order), `cardDef(id)` (throws), `cardDefByIndex(set, index)`.
* `src/query.ts` (110): typed wrapper over engine `query`/`queryCost` (`catalog.query`, `catalog.pool`,
  `catalog.cost`, `catalog.trapTypes`, …). `src/kyTestBank.ts` (108), `src/flavour.ts` (24, + `flavour.json`).
* Engine catalog side (`packages/engine/src/catalog.ts`): `registerCatalog`, `registeredCatalog`, `defByIndex`,
  `catalogVersion`, `findDef/defOf(state|null, id)` (transient defs first), `queryCost`, `CatalogQueryArgs`
  (:75), `query` (:266 sorted total order), `selfDefIds`, `excludingDefId`, fused-id parsing `fusedIdParts`/
  `fusedIdSpecs`/`isDigestId`, process-global `digestIngredients` (:181), `rollGrape`, `pickGenerated`,
  `glitchOrNot`.

### 5.3 What card files import

* From `@jackioh/engine` (74 values): `param` (125 files), `defOf` 20, `zoneCards` 19, `zoneCount` 16,
  `subsystems` 15, `findInstance` 14, `activeUnitsOf` 13, `recalled` 10, `cardAt` 10, `plagueOn` 8, `slotOf` 7,
  `slotsOf` 6, `effectiveCost` 6, `costNow` 6, `unspentManaOf` 4, `RESUME_HOOK` 4, `heroOf` 4, `cardTypeOf` 4,
  `unitView` 4, `wasPlayedThisTurn` 3, `cardsPlayedThisTurn` 3, `unitHas` 3, `attackTargetOf` 3,
  `permanentsOnField` 3, then 1–2 each: `UNIT_ZONES, printedCost, afterAttackOf, playedIdsThisTurn, queryCost,
  replacementOf, firstFreeZone, numberedSum, playedThisGameWithTag, playedEarlier, fib, timesPlayedOf,
  leftFieldSinceResolved, isUnitToken, openZones, defByIndex, numberingOrder, fusablePermanentsOf, killerOf,
  dormantUnitsOf, statsWithBuffs, isXCost, lethalAttackersOf, playCost, ownCost, GLITCH_NUMBERS, midlaneLanesOf,
  maxManaOf, BOOK_SWAP_TRIGGER, plagueMultiplierOf, LIBRARY_CAP, fillBoardZones, randomAttackTargets, isBerserk,
  isLocked, summonedSoFar, isCarried, cardKeywords, isCastOnDraw, carriedAt, stackedOnto, ROLLBACK_MAX_TURNS,
  playedThisTurnOfType, numbersOn, numberedKeywordsOn, playableFromGraveyard, CHAIN_OF_THOUGHT_REPEATS,
  lastFaceUpPlayed, RADIANT_SHEEP_TRIBUTE_VALUE, SHEEP_TRIBUTE_VALUE`.
* Engine types: `Script` 318, `EffectContext` 99, `Effect` 73, `CardInstance` 32, `GameState` 27, `TriggerDef` 19,
  `Hook` 16, `TrapTrigger` 15, `ConditionContext` 12, `ActivationDecl` 8, `AuraHook` 7, `CatalogQueryArgs` 3,
  `StaticFlags`, `TargetCheck`, `CostAuraArgs` 2, `DrawLimitHook`, `StatMod`, `KillCredit`, `PreviewHook` 1.
* `subsystems.*` used by cards: callToChaos 4, auditTargets 4, twiceForwardTrigger, questRewardOf, linesOfCode,
  isLethal, holdQuestAura, heldQuestAuras, activationPaid, QuestDef, CHAOS_PLUS_EFFECTS (2 each), twiceForwardPlays,
  topThree, startGrade, rollPower, questDefOf, powerAbilities, papayaBegin, papayaAnswered, openQuest, kyTestScript,
  isTerminalGrade, heroPower, gradeRises, gradeOf, gradeName, defendingHero, copiedTextOf, comboIndexEndOfTurn,
  QuestBook, POWER_RESUME, PAPAYA_STEP, FUSE_MIN_INGREDIENTS (1 each).
* Effects barrel: 168 distinct names used; top: draw 34, forEachCard 31, damage 27, summon 22, chosenOptions 21,
  addToHand 19, exile 19, addRandomFromCatalog 18, heal 17, destroy 11, gainMana 9, discardRandom 9, remember 9,
  cardsInScope 9.
* Documented read surface for cards: `packages/cards/README.md:205-227` (heroOf, zoneCards, zoneCount,
  cardsPlayedThisTurn, unspentManaOf, playedIdsThisTurn, playedEarlier, wasPlayedThisTurn, activeUnitsOf /
  dormantUnitsOf / cardAt / slotsOf / slotOf, faceOf / statsWithBuffs / unitView, defOf / printedCost /
  effectiveCost / queryCost, findInstance, instanceOf, recalled, killerOf, afterAttackOf, param, ownCost, costNow,
  maxManaOf, subsystems.activationPaid). Actual use (above) is wider than the documented table.

### 5.4 `catalog.json`

* 221,758 bytes; top level is an object keyed by catalog id, 318 entries in order `core-001` … `classicplus-t-ai-10`.
  Sets: Core 100 + 11 tokens, Classic 90 + 1 token, Classic+ 78 + 38 tokens.
* Entry keys (count of 318): `id, index, name, set, type, tags, rarity, token, cost, loc, base, radiant` (all);
  `params` 127; `refs` 41; `printedRarity` 28; `radiantFallback` 1. `cost` is number (309), string `"X"` (5), or
  `{base, embiggen}` (4). Face = `{type?, attack?, health?, xStats?, keywords: Keyword[], text}`.
* Example (`classicplus-011`):
  `{"id":"classicplus-011","index":"11","name":"Anime Armor","set":"Classic+","type":"Unit","tags":[],"rarity":"Rare","token":false,"cost":2,"params":[{"key":"cap","base":1,"radiant":1,"better":"down","step":1,"min":1}],"loc":3,"base":{"attack":4,"health":4,"keywords":[],"text":"Aura: Your hero can't take more than {cap} damage at a time."},"radiant":{"attack":8,"health":8,"keywords":[{"kind":"Reborn"}],"text":"Reborn\nAura: Your hero can't take more than {cap} damage at a time."}}`
* Names are pure ASCII; texts contain non-ASCII BMP characters (× − – ♾ U+FE0F ³ ²).
* `loc` = non-blank non-comment non-import lines of the card's **TypeScript** script file, written by
  `packages/cards/scripts/gen-loc.ts` and held current by `test/loc.test.ts` — and it is **gameplay data** (see §8).
* Patch history: `packages/cards/patches/` (snapshots, `pending/<version>.json` fragments; tooling in
  `packages/cards/scripts/patches*.ts`).

---

## 6. TESTS

### 6.1 `packages/engine/test` — 169 test files (62,390 lines, 2,705 `it`/`test`) + 30 fixtures (5,783 lines)

| group | files | lines | its | files |
| --- | --- | --- | --- | --- |
| effects verbs | 50 | 14,393 | 631 | `effects-*.test.ts` (after-check, animate, boardwide, brittle, buff, cardScope, cast-chaos, cast, choose-where, choose, combat, core, cost, counters, cry, damage, datacenter, delay, destroy, drawWhile, each, enchant, flicker, fruit, give, hand-exile, heal, health, library, locks, move, perks, plague-random-cast, plague, plus-c, radiant, random, reveal, shuffle-card, split, statuses, steal, summon-copies, summon, summonThis, swap, targets, transform, tune, turnEnd) |
| rulings index + rulings-a/b/c | 4 | 10,171 | 699 | `rulings.test.ts` (4,565 lines, 563 its = one per SPEC §11 row via `provenIn(n, file)`, 552 delegations, some into `apps/`, `packages/ai`, `packages/cards`, `packages/shared`), `rulings-a` 1,385/40, `rulings-b` 1,682/42, `rulings-c` 2,539/54 |
| subsystems | 21 | 8,301 | 352 | activate, aiPolicy, audit, boardHistory, callToChaos, callToChaosPlus, comboIndex, copied-text, corePatches, fuse-registry, fuse-variants, fuse, glitch, heroPower, kyTest, lastBoards, papaya, perfectHand, quests, scorer, twiceForward |
| view / layers / instance data / field | 25 | 7,953 | 297 | animated, backrow-piles, brittle, conditionActive, control-change(.property), destroyed-face, faces, glow-facts, instance-data, layers, library-copies, params, pools, preview-ids, preview, query, recruit-variants, rotation, rounds, shuffle-random, temporary, transform-variants, view-marks, viewFor |
| prompts / work / triggers / traps / state check | 13 | 5,670 | 138 | death-pause, delayed-kinds, modifiers, pauses, prompt-kinds, prompts, start-of-opponent-turn, statecheck, stays, trap-cardresolved, trap-window-pause, trigger-zones, triggers |
| turn / setup / reduce / state / replay / handicap | 21 | 5,532 | 225 | auto-end-turn, config, endgame, game-summary, generation-replay, handicap (1,303/66), hotseat.smoke, lint-ban, mulligan-concurrent, ownLibrary, reduce, replay-scripted, replay, rng, setup-aside, setup, state, turn-cap, turn-wiring, turn, zones |
| combat / damage / targeting / replacements | 18 | 5,253 | 191 | after-attack, backrow-death, carried-damage, combat-positions, combat-resolution, combat-validation, combat.property, damage-pipeline, damage, kill-credit, lethal, replacements, restrictions, self-tribute, targeting, tribute-zones, tribute, windfury |
| play pipeline / mana / costs / draw | 17 | 5,117 | 172 | announce, cost-rules, counterWarning, draw-complete, draw-limit, draw-pause, draw, echo, graveyard-play, mana-before-play, mana, overflow-events, play-pipeline-b-replay, play-step3, playChoices-filters, playChoices, playCounts |

Fixtures (`packages/engine/test/fixtures/`): test-only catalogs + scripts per topic (`prompts.ts` 649,
`damage-combat.ts` 510, `generation.ts` 428, `combat.ts` 408, `activate.ts` 393, `playPipelineB.ts` 384,
`instanceData.ts` 361, `playPipelineA.ts` 352, `field.ts` 301, `turn.ts` 285, `quests.ts` 266, `scripts.ts` 253,
`copiedText.ts` 136, `promptHarness.ts` 125, `harness.ts` 107, …, `rng-child.ts` 9, `lint/*.ts` 3 lint probes).
Engine tests register fixture scripts with `registerScripts`/`registerCatalog`, build states by hand and call
internal mutators directly (not only `reduce`).

### 6.2 `packages/cards/test` — 366 test files (90,068 lines, 5,174 `it`/`test`)

* Per-card tests: 315 files (67,930 lines, 4,470 its), one per script except `090-1-cn-virus`, `093-1-combo-fodder`,
  `095-1-chaos-golem` (covered by their parents). Median 191 lines, min 43, max 971 (`093-combo-index.test.ts`;
  then `classic/090-in-too-deep` 782, `095-call-to-chaos` 738, `050-k-pop-fanatic` 737, `classic-plus/035-rollback` 715).
  Root 108 files, `classic/` 91 (22,211 lines), `classic-plus/` 116 (20,663 lines).
* Non-card tests (51 files): rules suites (31 files, 16,196 lines, 475 its): after-resolution, combat-windows,
  condition-active, control-change(-carry), costs-and-mana, deaths-and-reborn, echo-and-exile, forced-attacks,
  fused-hooks, fused-nested-resume, fused-target-checks, fuse-registry, game-over, hand-returns,
  hidden-information (1,474), lasting-effects, my-pawn, paused-sequences (2,175), play-choices, plays-and-casts,
  preview (1,383), re-entry, resolving-face, setup-and-mulligan, stacks-and-reborn, tributes, trigger-stays,
  turn-clock-and-legality, turn-stages, vanilla-and-positions, …; data/registry/patches (14 files, 4,582 lines,
  214 its): `_harness.test`, catalog, card-text, flavour, loc, params, references, registry, radiant-standard,
  versions, patches, patches-ship, query, pools-and-randomness; gates (6 files, 1,360 lines, 15 its): fuzz,
  fuzz-handicap, hotseat-replay, invariants, game-summary, self-generation.
* Card tests import the engine directly: 284 files `from "@jackioh/engine"` (165 distinct values; top:
  `stepParam` 133, `legalActions` 69, `reduce` 63, `createRng` 53, `subsystems` 42, `hashState` 38,
  `effectiveCost` 28, `defOf` 22, `newInstance` 19, `query` 17, `HAND_CAP` 16, `registerScripts` 16,
  `makeContext` 15, `registeredScripts` 15, `beginGame`/`createGame`/`applyEffects` 14, `fold` 11, …), 31 from
  `/effects`, 2 from `/config`.

### 6.3 `test/_harness.ts` (1,149 lines) — `scenario()` API

* Exports: `scenario(opts)` (:1147), `DEFAULT_SEED = "jackioh-harness"`, `DEFAULT_TURN = 9`; types `CardRef`,
  `DefRef`, `CostSetup`, `PileSetup`, `FieldEntry`, `FieldSetup`, `BackrowSetup`, `SideSetup`, `PlayOptions`,
  `ActivateOptions`, `ScenarioOptions`, `ZoneName`, `PileName`, `Scenario`, `UnitView` (re-export).
* `ScenarioOptions {seed?, p1?: SideSetup, p2?: SideSetup, turn?, active?, lastBoards?, glitchBoards?}`;
  `SideSetup {hand?, field?, backrow?, library?, graveyard?, exile?, health?, mana?, armor?}`; field entries
  `{def|defId, radiant?, row?, lane?, stack?, position?, damage?, counters?, faceUp?, statsOverride?, costMod?, costOverride?}`.
* `Scenario` methods: getters `state`, `events`, `lastEvents`; actions `play(card, {zone?, row?, x?, embiggen?,
  targets?, modes?, tributes?})`, `attack(attacker, target|"hero")`, `answer(selection)`, `endTurn()`,
  `startTurn()`, `switchPosition(card)`, `activate(card, {targets?, ability?, modes?, tributes?})` (sends
  `activatePower` when none of ability/modes/tributes is given); reads `view(player?)`, `unit(p, lane)`,
  `backrow(p, lane)`, `hand(p?)`, `pile(p, zone)`, `card(ref)`, `stats(card)`; assertions `expectInZone`,
  `expectStats`, `expectEvents`, `expectHealth`, `expectMana`.
* It builds states with engine internals, not `reduce` only: `createGame`, `newInstance`, `placeOnField`
  (`zones.ts:328`), `moveToZone` (`zones.ts:692`), `removeFromAnyZone` (`zones.ts:497`), `createInHand`
  (`setup.ts:601`), `refreshMana`, `stateCheck`, `startTurn`, `declareAttack`, `answerPrompt`, `showToOwner`,
  `unitView`, `viewFor`, `query`, `registeredCatalog`, `cardAt`, `findInstance`, `defOf`. Card references resolve
  by catalog id, §5 index, or name. Typical card test (`test/002-bigot.test.ts`, 62 lines): 5 `it`s, each
  `scenario({...}) → s.play(...) → s.expectInZone(...)`.
* `_invariants.ts` (258): I1–I4 monitor (summoning sickness/exertion, R171) used by fuzz. `_glow.ts` (35).

### 6.4 Fuzz gates

* `test/fuzz.test.ts` (555 lines, 2 its): seeds 1–1000 under `pnpm fuzz` (1–100 under `pnpm test`; env
  `JACKIOH_FUZZ_FROM`, `JACKIOH_FUZZ_SEEDS`). Deck pool = all non-token catalog ids sorted (`FUZZ_POOL`,
  `POOL_EXCLUSIONS` empty); decks from `createRng("jackioh-fuzz-decks-<seed>").shuffle(pool)` first/next 20;
  game seed `"jackioh-fuzz-<seed>"`; policy = `subsystems.chooseAction(state, player, policyRng)` with
  `policyRng = createRng("jackioh-fuzz-policy-<seed>")`, mulligan order from `createRng("jackioh-fuzz-policy-order-<seed>")`;
  nonce `fuzz-<n>`; `reduce` called without an rng. Asserts: no throw, termination (reason ∈ hero-death /
  both-heroes-dead / turn-cap, phase over, turn ≤ cap, ≤ `TURN_CAP_PLAYER_TURNS × AI_PLAYOUT_STEP_CAP` actions),
  every `legalActions` pick accepted, invariants each step, **replay equality** (`fold({seed, decks, log})` has no
  errors and `hashState(fold) === hashState(live)`), failure lists every card in both decks.
* `test/fuzz-handicap.test.ts` (154 lines, 2 its): same with one seat at Medium/Hard handicap rotating by seed;
  fold includes handicaps.
* `test/hotseat-replay.test.ts` (213 lines, 6 its): folds `fixtures/01-hotseat-full-game.json`
  (`{seed: "01-hotseat", decks, log}` 37 actions, Core-only decks) to the literal `EXPECTED_HASH "a798906b"`; also
  checks a shorter/other fold differs and compares with `e2e/artifacts/` when present.
* These give ready trace oracles: any (seed, decks, log) from the fuzz generator can be recorded from TS with
  per-step `legalActions`, events and `hashState`.

### 6.5 Totals

engine 2,705 + cards 5,174 = **7,879** `it`/`test` calls (plus shared 74, validator 40, ai 313).

---

## 7. VALIDATOR + SHARED

### 7.1 `packages/validator` (src 540 lines; tests 2 files, 554 lines + 247-line fixture, 40 its)

* `src/index.ts` (526): `LOADOUT_DECKS = 3`, `TRIO_DECKS`, types `CardId`, `LoadoutDeck`, `CatalogSnapshot`,
  `Collection`, `LoadoutInput`, `LoadoutRule` (L1–L6), `LoadoutError`, `LoadoutResult`, `DeckInput`,
  `TrioConflict`, `DraftRule` (D1–D5), `TrioDraftRule` (T1–T3), `DraftIssue`, `NameLimits`, `DeckDraftInput`,
  `TrioDraftInput`, `ImportRoomInput`, `ImportRoom`; functions `validateLoadout` (:119), `validateTrio`
  (alias :124), `validateDeck` (:138), `trioConflicts` (:292), `normalizeName` (:359), `checkDeckDraft` (:405),
  `checkTrioDraft` (:450), `checkImportRoom` (:501).
* `src/config.ts` (14): re-exports `DECK_SIZE, MAX_COPIES` from `@jackioh/engine/config` (comment: "Collapse
  this file … once the engine …").
* Consumers: `apps/server/src/api/{loadout-validator,catalog,decks,ports}.ts`; `apps/web/src/net/api.ts`,
  `apps/web/src/routes/{almanac,decks,play}.tsx`, 17 files in `apps/web/src/game/deckbuilder/`.

### 7.2 `packages/shared` (src 2,477 lines; tests 4 files 1,505 lines, 74 its)

| file | lines | exports | engine-relevant? |
| --- | --- | --- | --- |
| `catalog-types.ts` | 406 | `PlayerId`, `PLAYER_IDS`, `opponentOf`, `CardType`, `Tag` (16), `Rarity`, `PrintedRarity`, `SetName`, `SHIPPED_SETS`, `CardCost`, `Keyword` (25 kinds), `KeywordKind`, `KEYWORD_KINDS`, `keywordKey`, `hasKeyword`, `armorOf`, `CardFace`, `Param`, `PARAM_PLACEHOLDER` (regex), `paramPlaceholders`, `fillParams`, `CardDef`, `FusedIngredient`, `CardDefs`, `CatalogQuery`, `PromptKind`, `Row`, `Zone`, `ZoneRef`, `TargetFilter`, `TargetDecl`, `ModeDecl` | yes |
| `actions.ts` | 97 | `ZoneChoice`, `Selection`, `ActionBody` (15), `Action`, `DistributiveOmit`, `ActionInput`, `ActionType`, `NON_ACTIVE_ACTION_TYPES`, `PROMPT_OPEN_ACTION_TYPES` | yes |
| `events.ts` | 498 | `GameEvent` (65), `TuningChange`, `GameEventType`, `LibraryOverflowOutcome`, `GameOverReason`, `GAME_EVENT_TYPES`, `GameEventTypesAreExhaustive` | yes |
| `view.ts` | 439 | `CardView`, `QuestView`, `CopiedTextView`, `Tuning`, `Enchantment` (4 kinds), `CardMark`, `ActivationView`, `PreviewValue`, `UnitView`, `BackrowView`, `HeroPowerView`, `HeroView`, `ModifierView`, `LibraryEntryView`, `LibraryView`, `SideView`, `PendingView`, `PendingOption`, `MulliganView`, `PlayerView` | yes (viewFor output; web renders it) |
| `stats.ts` | 442 | `GAME_SOURCES`, `GAME_MODES`, `PILOTS`, `GAME_OVER_REASONS`, `SeatSummary`, `GameSummary`, `GameRecord`, filters, `cardStats`, `winRate`, `playedDelta`, formatters, `parseGameRecord(Lines)` | partly (engine `gameSummary.ts` uses `SeatSummary`/`GameSummary`; rest is server/AI tooling) |
| `codes.ts` | 358 | invite/deck code formatting and parsing | no (client/server) |
| `emotes.ts` | 139 | emote/portrait ids, cooldown gate | no |
| `aim.ts` | 90 | drag-aim wire type and parser | no |
| `index.ts` | 8 | `export *` of the above | |

The engine imports ~50 shared symbols (most used: `PlayerId` 67 files, `opponentOf` 31, `PLAYER_IDS` 26,
`GameEvent` 23, `CardType` 15, `Selection` 14, `Row` 14, `hasKeyword` 11).

---

## 8. EXCEPTIONS / SPECIAL CASES a rewrite could delete

| # | what | where |
| --- | --- | --- |
| 1 | `activatePower` action, a legacy alias of `activate` kept "so old logs replay"; routed by naming the rolled power's ability | `packages/shared/src/actions.ts:49`; `packages/engine/src/reduce.ts:26, :132-155, :190-194, :318`; `subsystems/heroPower.ts:14, :552`; `subsystems/activate.ts:28`; `apps/server/src/match/protocol.ts:104, :411-419`; `packages/ai/src/candidates.ts:89-170`; harness `ActivateOptions` sends it (`packages/cards/test/_harness.ts` docs at :285-291) |
| 2 | `StaticFlags.deftDuelist` "Legacy" — Deft became a keyword in v0.2.4; no engine src reads the flag (engine test fixtures still set it) | `script.ts:166-170`; `test/fixtures/combat.ts:277-399` |
| 3 | `EffectContext.eventsFrom` comment says it is optional only because `test/pauses.test.ts:227` hand-builds a context; the field is already required — stale | `script.ts:17-31` |
| 4 | Type aliases kept for readability: `TrapTrigger = TriggerDef`, `TributeFlags = StaticFlags & {tributeEnemies?}` | `traps.ts:66-71`; `playChoices.ts:388-393` |
| 5 | `Script.activate` hook and `HookName "activate"` have no reader in engine src and no card uses them (superseded by `activations[]`); `onPlayHook` is used by the engine but by no shipped card (only a rulings-c fixture) | `script.ts:334-335`; `resolve.ts:84-92`; `playSteps.ts:651-664` |
| 6 | `registerAttackBar` registry: declared, iterated, never registered by any module | `restrictions.ts:78-95` |
| 7 | DI registration hooks that exist to break TS import cycles (module side effects at import time): `registerWorkHandler` ×16, `registerDefaultWorkHandler` (`prompts.ts:912`), `registerPromptAnswerer` ×4, `registerTargetingHooks` (`targetingPoint.ts:168` → `prompts.ts:524`), `registerCastDriver` (`playSteps.ts:1964` → `resolve.ts:143`), `registerDeclarationCheck` (`combat.ts:524` → `traps.ts:444`), `registerGraveyardRedirect` (`replacements.ts:532` → `zones.ts:651`) | see §2.7 table |
| 8 | Process-global mutable state: catalog registry (`catalog.ts:16-17`), script registry (`scripts.ts:7`), fused digest table (`catalog.ts:181`), `syncFusedScripts` re-registering fused scripts from `state.transientDefs` on every `reduce`/`legalActions`/`viewFor` entry (`subsystems/fuse.ts:948`, called `reduce.ts:399, :493`), reentrancy counters `replacements.ts:383 let converting`, `subsystems/scorer.ts:225 let dryRunning` | |
| 9 | Fused-card machinery that rebuilds closures from ids: `Effect.expand`/`memo`/`lazyPart`, `PausedStep.part/memo`, `__part/__partDepth/__remembered` keys, `rerootRemembered`/`memoryOfPart`, `INGREDIENTS_KEY` price records, `combineObjects` merging `Script` objects key by key | `script.ts:102-120`; `work.ts:205-300`; `scripts.ts:39-130`; `subsystems/fuse.ts` (1,171 lines) |
| 10 | "hashes as it did before the field existed" shapes: fields stored only when non-default to keep old hashes — `handicap` only if ≠ human (`state.ts:904-919`), `heroHealth` only if ≠ 30, `autoEndTurn?: false` (absent = on; `reduce.ts:220-221`), `budget?`, `mulligan?`, `marks?`, `lastBoards?`, `systemPlays?`, `gameLog?`, `backrowPiles?`, `carried?`, … | `state.ts:313-316, :405-447, :486-581`; `announce.ts`, `tuning.ts`, `timesPlayed.ts` |
| 11 | Defensive re-parsing of `Record<string, unknown>` payloads "because it came through JSON" (~58 `typeof`/`Array.isArray` guards, ~136 `data.*`/`memory.*` reads): `work.ts:117-150` (`runMarksOf`, `eventStayIn`), `traps.ts:633-650`, `stateCheck.ts:234`, `scripts.ts:55-68`, `subsystems/quests.ts:158-170`, `draw.ts:375`, `targeting.ts:32`, `timesPlayed.ts:17`, `subsystems/callToChaos.ts:83`, `subsystems/twiceForward.ts:41` | typed enums would remove them |
| 12 | Harness dead fallbacks for "not-yet-wired" reduce cases (`"combat arrives with M2"`, `"prompts arrive with M3"`) — reduce has wired both for a long time | `packages/cards/test/_harness.ts:193-200, :901, :933` |
| 13 | Duplicated helpers: `creationNumber` (`triggers.ts:217`, `effects/choose.ts:464`); `compareText` (`effects/fuse.ts:317`, `ownLibrary.ts:49`); `indexRank` (`catalog.ts:245`, `subsystems/scorer.ts:473`); `validateDeck` exists twice with different semantics (`state.ts:770` vs `packages/validator/src/index.ts:138`); `cards/src/query.ts` is a typed re-wrapper of `engine/catalog.ts` `query` | |
| 14 | Dynamic-import / lazy-binding tricks: server `apps/server/src/match/engine.ts:163-200` (`import(specifier)` from a variable so bundlers do not follow, `EngineUnavailableError`, `setEnginePort`); web `apps/web/src/game/engine.ts:51-80` (`REQUIRED_ENGINE_EXPORTS`, lazy `engine.real.ts`); `packages/cards/src/index.ts:19-27` import-order comment for a runtime cycle; `_generated.ts` type-only cycle | |
| 15 | Rule-9 exceptions (numbers outside `config.ts`): see §4 list (heroPower 11, callToChaos 8, scorer weights, comboIndex 4, …) | |
| 16 | `loc` (lines of TypeScript per card script) is gameplay data: C+ #44 Simplicity Audit / #45 Complexity Audit (`subsystems/audit.ts:9-36`, `linesOfCode`) and Classic #48 Hired Shrimp (`src/scripts/classic/048-hired-shrimp.ts:17`) compare it; generated by `packages/cards/scripts/gen-loc.ts` from the TS file and asserted by `test/loc.test.ts`; also in patch snapshots. A Rust port changes every count unless `loc` is frozen as data | |
| 17 | Cards reach engine internals beyond the documented read surface (74 values incl. `numberingOrder`, `RESUME_HOOK`, `summonedSoFar`, `fillBoardZones`); card tests reach 165 engine values incl. `makeContext`, `applyEffects`, `registerScripts`, `newInstance`, `placeOnField`, `moveToZone`, `lockZone`, `scheduleDelayed`, `settle` | §5.3, §6.2 |
| 18 | `reduce` mutates its *input* via `syncFusedScripts(state)` before cloning (global registry side effect) | `reduce.ts:399` |
| 19 | `timeout` answer path draws from the match rng with `legalActions` order (`reduce.ts:276-277`); `answerForLockedOut` / `endDueTurns` loops guarded by `TURN_CAP_PLAYER_TURNS` | `reduce.ts:252-372` |

---

## 9. PERFORMANCE hot spots (issue #70)

Issue #70 ("v0.3.0 (part 1 of 2): the engine and AI are 1.6–2.5× slower", closed 2026-10-05, parent #80
"rewrite the server in Go and the engine and AI in Rust") measured 3.1 → 4.9 ms/action (Core decks) and
7.5 ms/action (all sets) for `reduce` in random-policy games; `gate-hard-easy` 6m41s → 17m02s. Profile items and
their current locations (no caching or memoisation exists in engine src today — no `cache`/`memo`/`WeakMap` hits):

| hot spot (#70) | location | note |
| --- | --- | --- |
| `cloneState` 13.6% self | `packages/engine/src/state.ts:957` (`JSON.parse(JSON.stringify(state))`), called per action at `reduce.ts:434`, `beginGame` `reduce.ts:458`, per dry-run play `subsystems/scorer.ts:345`, `:375`; AI `packages/ai/src` (2 sites) | state grew with v0.2.0 (`boardHistory` holds whole instance copies, `applied` 64 event lists) |
| `slotsOf` / `activeUnitsOf` ~12% self | `zones.ts:39` (already de-`Array.from`'d per #188 comment), `zones.ts:770` | called from aura/layer computation for every unit read |
| `computeLayers` / `auraMods` / `unitView` ~20% inclusive | `layers.ts:214` (`computeLayers`), `:146` (`auraMods` iterates `auraSources` :128 and runs every aura hook per unit), `:190` (`unitView`), `:310` (`statsWithBuffs`) | recomputed on every read, never stored (by design §10.4) |
| `perfectHand` / `scorer.dryRun` ~19% inclusive | `subsystems/scorer.ts:322` (`dryRun`: clones `base` per candidate play and runs `runPlaySteps` + `settle`), `:371` (`dryRunBase`), `:492` (`rank`, scores every non-token Core def), `:501` (`topThree`); `subsystems/perfectHand.ts:19` (`rankPerfectHand`) | scales with catalog size |
| (also) `autoEndDue` after every action | `reduce.ts:381-389` uses the `eachLegalAction` generator to stop at the first non-end action (#188: listing all plays was ~1/3 of a long gate game) | |

---

### Appendix: useful anchors

* Entry points: `reduce` `reduce.ts:398`, `legalActions` `:484`, `beginGame` `:457`, `seatToAct` `:83`,
  `createGame` `state.ts:833`, `fold` / `hashState` `replay.ts:60 / :27`, `viewFor` `viewFor.ts:1128`,
  `settle` `triggers.ts:806`, `drainWork` `work.ts:580`, `answerPrompt` `prompts.ts:460`, `runPlaySteps` `playSteps.ts:1974`,
  `declareAttack` `combat.ts:616`, `chooseAction` `subsystems/aiPolicy.ts:50`.
* SPEC §11 has 561 R-rows (R1…R761 with gaps); `pnpm rulings:coverage` (`packages/engine/scripts/rulings-coverage.ts`)
  scans `packages/engine/test`, `packages/cards/test` for tests and `packages/` + `apps/` `.ts/.tsx/.sql` for
  cited ids — a `crates/` tree is outside its scan today.
