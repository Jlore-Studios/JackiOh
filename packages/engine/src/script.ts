// The card-script contract (SPEC §10.9). A card file returns Effect[] from the effects library and
// never touches state itself (CLAUDE.md rule 5); the engine applies the effects.

import type { GameEvent, Keyword, ModeDecl, PlayerId, PreviewValue, Selection, Tag, TargetDecl } from "@jackioh/shared";
import type { CostAura, CostAuraArgs } from "./costRules";
import type { GraveyardPlayPermission } from "./graveyardPlay";
import type { Rng } from "./rng";
import type { CardInstance, GameState, PlayRecord } from "./state";
import type { EventStay } from "./stays";
import type { QuestBook } from "./subsystems/quests";

export type EffectContext = {
  state: GameState;
  rng: Rng;
  /** Effects append here; reduce returns the list (§10.3). */
  events: GameEvent[];
  /**
   * R136: where *this script's* events begin in `events`. The array is the whole action's sink, so
   * a card that asks "what did I just do" — #60 Bear Honeypot's "they attack it", and the same
   * shape in #24, #31, #33, #38 — must read `events.slice(eventsFrom)` and never the earlier
   * entries, or a second copy of a card, or a trap firing mid-action, feeds its condition. Set once
   * where the context is built (`resolve.makeContext`), so a step re-entered after a prompt opens a
   * new window on the action it resumes in; what the list did before the pause is `summoned`.
   *
   * Optional only because `packages/engine/test/pauses.test.ts` (line 227) hand-builds a context
   * literal instead of calling `makeContext`, and a test is not this task's to edit. Every engine
   * path builds its context through `makeContext`, which always sets it; absent it reads as 0 —
   * the whole action, the pre-R136 reading — and each reader spells that default out at the point
   * it slices. Once that literal is allowed to change this becomes required.
   */
  eventsFrom: number;
  /**
   * R174: the field's departures when this script's run began (`stays.exitMark`), so an effect later
   * in the list can tell a card an earlier one took off the field from the card that stood there
   * when the run began — across a prompt too, since a paused list resumes with the mark it began
   * with (`work.PausedStep.exitsFrom`). Set by `makeContext`; absent reads as "now".
   */
  exitsFrom?: number;
  /**
   * R174, §10.6: the field's departures when `targets` were chosen, where that is later than the
   * run began — the answer to this run's own prompt, picked as the prompt offered the board. A card
   * the list took off the field before it asked, and that stood there again when the prompt offered
   * it (a Reborn body, R83), is picked on that new stay, and the answered step's effect lands on it.
   * Absent reads as `exitsFrom`: a play's declared targets were chosen as its run began.
   */
  chosenFrom?: number;
  /**
   * R174, R212: the cards the event a queued trigger answers names, and the field's departures when
   * that event happened (`stays.eventStayOf`). The loop hands the trigger its event some time
   * later, so a card the event names is judged from then: a trigger that reads the played unit's id
   * off its `cardPlayed` does not land on the Reborn body an earlier trigger on the same event made
   * (R59). Every other card the run aims at is judged from `exitsFrom`, when the run began. Carried
   * across a pause with the run's other marks. Absent for any run that is not a queued trigger's.
   */
  eventStay?: EventStay;
  /**
   * R136: the units this script's run summoned in the actions before a prompt split it. The window
   * `eventsFrom` opens is the action's own event list, and a list the answer continues resumes in a
   * later action, so what its head summoned is carried here (`work.PausedStep.summoned`,
   * `work.RunMarks`). Absent for a run that has not paused.
   */
  summoned?: readonly string[];
  /**
   * R98: the card running the script sat in the resolving zone as the run began (§10.5 step 4) — a
   * Spell resolving, or a permanent that found no zone. The run is that card's while it stays there,
   * so a continuation re-entered once the card has left it — the Spell's own list put it back in its
   * owner's hand before it asked — resumes with no self (`prompts.runResume`). Carried across a pause
   * with the run's other marks (`work.RunMarks`, `work.PausedStep`). Absent for any other card.
   */
  selfResolving?: boolean;
  /** Who is resolving this: the controller of `self`, or the player who cast the card. */
  controller: PlayerId;
  /** The instance whose script is running, when it still exists. */
  self: CardInstance | null;
  /**
   * R127: the definition whose script is running, set where a continuation is re-entered
   * (`prompts.runResume`), because `self` is null once the card has ceased to exist and a step that
   * asks again must still name its script. Absent elsewhere, where `self` names it.
   */
  defId?: string;
  /** Whether the radiant text is the one running (§5.2). */
  radiant: boolean;
  targets: Selection[];
  modes: string[];
  x: number;
  embiggened: boolean;
  /** Captured data from a Resume, for chained steps (§10.6). */
  data: Record<string, unknown>;
  /**
   * play pipeline B (Classic #22 Mid Runner: "If you had 4 or more mana when you played this"): the
   * current mana of the player who played or cast the card as the play began — at §10.5 step 1, before
   * step 2 paid, or as a cast began — on the played card's own Cry and every continuation of it
   * (`resolve.MANA_BEFORE_PLAY_KEY`, which rides the card's data across a pause). Absent anywhere else.
   */
  manaBeforePlay?: number;
};

/**
 * One state change from the effects library. Effects are built by engine code, so a card file
 * composing them stays pure.
 */
export type Effect = {
  readonly kind: string;
  apply: (ctx: EffectContext) => void;
  /**
   * A part of a composed list, built when the list reaches it: a fused hook runs each ingredient's
   * list in turn (R77, R102), and a later ingredient's list reads the board the earlier ones left
   * (#68's threshold after Reno's heal, #22's meal after #100's exile), which a list built all at
   * once cannot. `prompts.applyResumable` runs the part it builds as a nested list, so a prompt
   * inside it pauses the part and everything after it, and the pause records where it stood
   * (`work.PausedStep.part`) — so the part is built again on resume, and only the part the pause
   * stood in. `memo` is what the first build must hand every rebuild so the part is the same one:
   * #95's roll, which must not be rolled again (R87). A caller that only calls `apply` gets the
   * part built and applied in one go, as `resolve.lazyPart` writes it.
   */
  readonly expand?: (ctx: EffectContext, memo: unknown) => EffectPart;
};

/** What a part of a composed list builds (`Effect.expand`): its effects, and what a rebuild reads. */
export type EffectPart = { effects: readonly Effect[]; memo?: unknown };

export type Hook = (ctx: EffectContext) => Effect[];

/** A trigger a card registers while it is in a given zone (§10.3). */
export type TriggerDef = {
  id: string;
  /** Which events wake it. */
  on: GameEvent["type"][];
  /**
   * R99: the trigger's condition, kept out of `run` so a trap can decline an event without being
   * spent — R61 makes an empty effect list mean "fired and did nothing". Absent, the `on` match
   * alone arms it, so a trap with no predicate answers every event it names on either side.
   */
  when?: (ctx: EffectContext & { event: GameEvent }) => boolean;
  /** Reads the event and the state; returns the effects to queue, or none. */
  run: (ctx: EffectContext & { event: GameEvent }) => Effect[];
};

export type StatMod = {
  attack?: number;
  maxHealth?: number;
  keywords?: Keyword[];
  /**
   * §10.4 layer 5: an aura that SETS attack to a value (Classic #88 Siphon Squad's Radiant "Enemy Units
   * have 0 Attack"), applied after every other layer; with several, the last in aura order holds.
   */
  setAttack?: number;
};

/** An aura contributes stat and keyword layers while its card is in play (§10.4 layer 5). */
export type AuraHook = (ctx: { state: GameState; self: CardInstance; radiant: boolean }) => {
  /** Which units the aura touches. */
  applies: (unit: CardInstance) => boolean;
  mod: StatMod;
}[];

export type StaticFlags = {
  /** Plays itself on draw, then draws again (§6.2, R58). */
  castOnDraw?: boolean;
  /** Starts in the opening hand instead of a draw (§6.2). */
  quickdraw?: boolean;
  /** Replaces an empty-library draw with a Rush Token card (#75). */
  infiniteReserves?: boolean;
  /** Cannot switch to Defense Position (#65.1). */
  neverDefense?: boolean;
  /** R49: two exertions, so one attack plus one switch in a turn (#45 Deft Duelist). */
  deftDuelist?: boolean;
  /** R30: this card's own Echo, so its play resolves this many extra times. */
  echo?: number;
  /** R30, R209: the Echo this permanent's rider gives the next Spell, read off its face now (#79). */
  echoGrant?: number;
  /**
   * #38: while on the field, its controller's cards gain "Combo X: deal X damage to the enemy hero"
   * (radiant 2X, dealt as one hit, R281). A number is how many times the card grants it: a card
   * fused from two Quickstrikers carries both texts (R102), and `true` is once. The multiple of X
   * each grant deals is not the flag's: it is `QUICKSTRIKER_COMBO_MULTIPLE` in `config.ts`, picked by
   * the granting instance's own face, as #84's Armor is (`HERO_ARMOR`), so a base and a Radiant
   * Quickstriker side by side deal X and then 2X.
   */
  quickstriker?: boolean | number;
  /**
   * #64 Gifted Program: while on the field, the first card costing this much or less its controller
   * plays each turn becomes Radiant as it is played (§10.5 step 3, R56, R213).
   */
  giftedProgram?: number;
  /** Tribute cost in units, Sheep Tokens counting 2 (§6.3). */
  tribute?: number;
  /**
   * §3.2, §7: what this unit counts toward a Tribute while it is on the field — the Sheep Token's
   * "worth 2 Tributes" (3 on its radiant face). Absent is 1. It is the face's text, so a Vanilla
   * unit is worth 1, and a fused card takes the larger of its ingredients' (R102).
   */
  tributeWorth?: number;
  /** R101: only a card that says so may pay its Tribute with the opponent's units (§8 #55). */
  tributeEnemies?: boolean;
  /**
   * R360: a play whose Tribute took any of the opponent's units summons this card for the opponent
   * (§8 #55's base face: "If opposing Units are used, summon for your opponent").
   */
  enemyTributeHandsOver?: boolean;
  /** Anti-oneshot Armor: caps each hit on this player's hero at ANTI_ONESHOT_CAP (§4.4 step 3). */
  antiOneshot?: boolean;
  /**
   * #84 Going Long: while this card is in a backrow it gives that hero Armor from `HERO_ARMOR`,
   * picked by the instance's own `radiant` and `embiggened`, and §4.4 step 2 subtracts it. R124:
   * several sources add up, so this is a layer and not a value — a card carrying its own numbers
   * would put rules constants in a card file, which BUILD §2 keeps in `config.ts`. A card fused from
   * two carries both grants (R102), so a fused face may hold a count.
   */
  heroArmor?: boolean | number;
  /**
   * R429 (Core patches, v0.2.0): §10.5 step 4 counts each play of this card on its instance
   * (`CardInstance.timesPlayed`, `timesPlayed.ts`) — #31 KY's Math Equation's "times played".
   */
  countsPlays?: boolean;
  // ---- v0.2.0 static flags, by workstream: instance data (B2.7, B3.3, B3.4, E38, E39) ----
  // ---- v0.2.0 static flags, by workstream: field (B3.1, E20, E21, E22) ----
  /**
   * B5 E21, R446: "A Unit may be played on top of this" (Classic+ #33 Ivory Tower). While this backrow
   * card acts in its zone, a Unit its controller plays may name that zone and stand on it: a Unit for
   * every rule that can neither attack nor be attacked, with this card still acting beneath it
   * (`zones.carrierZonesFor`, `zones.isCarried`).
   */
  carrier?: boolean;
  // ---- v0.2.0 static flags, by workstream: play pipeline (E1, E2, E5 targeting, E11, E12, E15) ----
  /**
   * Classic+ #68 Organic Produce: while on the field, every card its controller plays carrying one of
   * these tags becomes Radiant as it is played (§10.5 step 3) — R213's Gifted Program rule by tag, on
   * every such play rather than the first cheap one (`playChoices.playMadeRadiant`).
   */
  radiantPlaysTagged?: Tag[];
  /**
   * B5 E14, Classic #57 Echo, R399, R545–R547: "This has the text of the last Spell either player
   * played". The card's text is the copied Spell's face (`subsystems/copiedText.ts`): its declared
   * choices, its resolution and prompt continuations, its Echo X and Cast on draw, its `preview` and
   * `conditionMet`. The card keeps its own name, cost, type and tags, and its own other flags (Echo).
   */
  copiesLastSpell?: boolean;
  // ---- v0.2.0 static flags, by workstream: activate and turn (E3 draw limit, E10) ----
  // ---- v0.2.0 static flags, by workstream: damage and combat (E5, E6, E8, E35) ----
  /** B5 E35: no attack may be made on this unit, declared or forced (Classic+ #51 J15 Fighter). */
  cantBeAttacked?: boolean;
  /**
   * B5 E35: only a unit standing in this unit's lane may attack it, declared or forced (Classic+
   * #19.1 Top Loser); a Taunt on it binds only the attackers that may reach it (§4.2 step 3).
   */
  attackedOnlyFromLane?: boolean;
  /** B5 E35: this unit neither attacks nor is attacked, declared or forced. */
  cantAttackOrBeAttacked?: boolean;
  /** B5 E35: "This can't go Berserk" (Classic+ #19.5's Radiant face). */
  neverBerserk?: boolean;
  /**
   * B5 E8: while this card acts on the field — face-up, when it is a Trap or Field Trap — a heal of X
   * on one of its controller's enemies deals X Pierce damage to it instead, from this card (Classic+
   * #22 Blood Moon's Radiant Field Trap, "From now on").
   */
  healToDamage?: boolean;
  // ---- v0.2.0 static flags, by workstream: prompts and generation (E19, E26) ----
};

/** R195, R280: where `viewFor` is asking about a card. */
export type ConditionZone = "hand" | "field";

/**
 * R195, R280, §10.9: the argument of the two read-only hooks `viewFor` asks, the Hearthstone "yellow
 * glow" predicate (`conditionMet`) and the number a formula comes to now (`preview`). A hook is a
 * PURE READ — it never writes, never draws from `rng` (the context carries none), never returns
 * effects — and must agree with what the card's own resolution would do if it resolved now.
 */
export type ConditionContext = {
  state: GameState;
  self: CardInstance;
  /**
   * The card's controller. R195 only ever asks about the viewer's own cards, so there this is the
   * viewer; R280 asks about any card the viewer may read, the other seat's public ones included, so
   * there it is the card's controller and never the viewer as such.
   */
  controller: PlayerId;
  /** Whether the Radiant face is the one running (§5.2). */
  radiant: boolean;
  /** "hand": as if played now. "field": as the card on the field reads it now. */
  zone: ConditionZone;
  /** `state.active === controller`, so a card file never reads `state.active` itself. */
  yourTurn: boolean;
};

export type ConditionHook = (ctx: ConditionContext) => boolean;

/**
 * R280, §10.9: the labelled numbers a card's formula comes to now — #31's Fib(cost+1), #70's sum
 * over missing health and exile. Each `label` is the formula as the running face prints it, an
 * exact substring of that face's catalog text (the client prints the value in braces right after
 * the label's first occurrence, §10.10), and each `value` what it would come to if the card resolved
 * now. The hook is asked with the same context `conditionMet` is (the running face, the card's
 * controller, the zone, `yourTurn`) and is a PURE READ, built on the same function the card's own
 * resolution computes the number with, so the two cannot disagree. It reads only what the card's
 * controller may read (§9.1) — a hero's health, a pile's size, the plays this turn, its own cost and
 * counters — never a library's contents or order or a hidden hand, because `viewFor` shows the
 * result to every viewer who may read the card, the other seat included (`preview.ts`). An empty
 * list is no preview at all.
 */
export type PreviewHook = (ctx: ConditionContext) => PreviewValue[];

export type Script = {
  /** Ceaseless Void's computed cost (R55); everything else uses the printed cost. */
  cost?: (args: { state: GameState; instance: CardInstance }) => number;
  cry?: Hook;
  death?: Hook;
  /** After the mulligan, before turn 1: only Heroic Power uses it (§6.2, R43). */
  startOfGame?: Hook;
  /** Named continuations a prompt answer re-enters (§10.6, R81). */
  resume?: Record<string, Hook>;
  /** A delayed effect this card scheduled, resolved at its R62 point. */
  delayed?: Hook;
  /** §10.4 layer 2: a card that sets its own stats from the board (#92 Felinor Fiender, R39). */
  setStat?: (args: { state: GameState; self: CardInstance; radiant: boolean }) => {
    attack?: number;
    maxHealth?: number;
  };
  startOfTurn?: Hook;
  endOfTurn?: Hook;
  aura?: AuraHook;
  triggers?: TriggerDef[];
  activate?: Hook;
  onPlayHook?: Hook;
  handTriggers?: TriggerDef[];
  staticFlags?: StaticFlags;
  /** The play-time choices this card declares (R81). */
  targets?: TargetDecl[];
  modes?: ModeDecl[];
  /** R195: the condition `viewFor` surfaces as `conditionActive` (§10.8). */
  conditionMet?: ConditionHook;
  /** R280: the numbers the card's formula comes to now, which `viewFor` surfaces as `preview` (§10.8). */
  preview?: PreviewHook;
  // ---- Patch v0.2.0 ----
  /**
   * B3.2, R384: the card's Activate abilities ("Activate:", "Activate N:", "Activate ♾️:"), used by
   * the `activate` action while the card acts on the field (`subsystems/activate.ts`).
   */
  activations?: ActivationDecl[];
  /**
   * §10.6: the named predicates a declaration's `TargetFilter.check` points at, for a filter no data
   * field can say (Classic #32's lane rule, #48's lines of code). A pure read, like `conditionMet`.
   */
  targetChecks?: Record<string, TargetCheck>;
  // ---- v0.2.0 script hooks, by workstream: instance data (B3.3, B3.4) ----
  // ---- v0.2.0 script hooks, by workstream: field (B3.1, E21, E22) ----
  // ---- v0.2.0 script hooks, by workstream: play pipeline (E1, E5 targeting, E11, E12, E15) ----
  // play pipeline B (E11, E15). Both are pure reads, like `aura`, asked of a card acting on the field
  // (the top of its pile, or its backrow card), and both return lists so a fused card carries each
  // ingredient's (R102). A hook rather than a static flag, so a Degrade or Upgrade of the card's
  // declared numbers moves what it grants (B3.4), and so a grant can hang on the card's state (Classic
  // #90 In Too Deep's reward L).
  /** E15, R455: the price rules this card lays on cards while it acts (`costRules.ts`). */
  costAura?: (args: CostAuraArgs) => CostAura[];
  /** E11, R454: the permissions this card gives its controller to play cards from their graveyard. */
  graveyardPlay?: (args: CostAuraArgs) => GraveyardPlayPermission[];
  /**
   * B5 E5, Classic #89 Paul Allen's Ghost: "to target this with anything but an attack, a player must
   * also discard N cards" — N now, read while the card is on the field (a pure read, so a Degrade or
   * Upgrade of the declared number reaches it through `param`). 0 or absent is no cost. A declared
   * target naming it carries the discards in the action; a prompt answer naming it asks for them next
   * (`targeting.ts`, `targetingPoint.ts`).
   */
  targetingDiscards?: (args: { state: GameState; self: CardInstance; radiant: boolean }) => number;
  /**
   * B5 E4, R451: what this card's play records as the card played — the last Spell played (Classic
   * #57) and its player's last face-up play. Absent records the card itself; Classic #57 Echo
   * returns the Spell it copied, and null records nothing (an Echo with nothing to copy).
   */
  recordsPlayAs?: (args: { state: GameState; self: CardInstance; radiant: boolean }) => PlayRecord | null;
  // ---- v0.2.0 script hooks, by workstream: activate and turn (E27, E28) ----
  /**
   * B5 E3, R457: the draw limits this card sets while it acts on the field ("Your opponent can't draw
   * more than 1 card each turn": Classic #4, #49). A pure read like an aura, so a card computes its
   * number (a declared, tunable one included) from its own instance. The lowest limit on a player holds.
   */
  drawLimit?: DrawLimitHook;
  // ---- v0.2.0 script hooks, by workstream: damage and combat (E5, E6, E8, E9, E35) ----
  /**
   * B5 E5, R460: the events this card changes before they happen — a lethal hit on its hero, a heal
   * on an enemy, its units' deaths, a card's way to a graveyard, a friendly unit targeted by the
   * opponent (`replacements.ts`). Declared as data and decided synchronously, never an effect list.
   * The "targeted" entry (`TargetedReplacement`) covers every targeting: attack, play, cast,
   * activation and prompt pick.
   */
  replacements?: import("./replacements").ReplacementDef[];
  /**
   * B5 E6: what this card does to hits on its controller's hero while it acts on the field — a
   * per-hit cap (the lowest of every cap wins, Classic+ #11 Anime Armor) and a divisor applied after
   * Armor (several multiply, rounded up once, Classic #75 Argusland). A PURE READ, like `aura`; a
   * list, so a fused card carries each ingredient's.
   */
  heroGuard?: (args: { state: GameState; self: CardInstance; radiant: boolean }) => { cap?: number; divisor?: number }[];
  /**
   * B5 E35: keywords the card has only while a condition holds (Classic #69 Plague Charger's First
   * Strike "while it has a Plague Token"), read in the layers with its printed keywords (§10.4), so a
   * Vanilla takes them. A PURE READ of instance data: like an aura's `applies`, it must never call
   * back into `unitView`, or the layers would recurse.
   */
  conditionalKeywords?: (args: { state: GameState; self: CardInstance; radiant: boolean }) => Keyword[];
  /**
   * "After this attacks" (Classic #13 Boots on the Ground, Classic+ #73.1 Classic Golem, Core #32
   * Prem Panther): run for the attacker once the state check that closes each of its combats has run,
   * a declared attack's or a forced one's, also when it died there — then on the snapshot it fought
   * with, as a Death hook reads its card (R78, R89). Not for an attack called off before it fought
   * (R44). `ctx.data` holds the combat's facts, read with `combat.afterAttackOf`: `{ targetId,
   * destroyedIds, survived, forced }`. A whole effect list, parkable like any (R113).
   */
  afterAttack?: Hook;
  // ---- v0.2.0 script hooks, by workstream: prompts and generation (E13, E19, E26) ----
  /**
   * B5 E19, R471: "Plague Tokens placed on this are doubled" (Classic #27 Pestilent Slime; tripled on
   * its Radiant face). What each placement onto this card is multiplied by, asked of the card as it
   * receives the placement — a pure read (a card reads its declared number here, B3.4), floored at 1.
   * A fused card's multipliers multiply (`subsystems/fuse`).
   */
  plagueMultiplier?: (args: { state: GameState; self: CardInstance; radiant: boolean }) => number;
  /**
   * B5 E26, R464: triggers this card answers while it lies in its owner's library ("While this is in
   * your deck: …", Classic+ #37 Wardrum). A library card is hidden (§9.1), so its queue entries take
   * no number (R177), and within a side they come after the hand's and before the graveyard's, in the
   * order the instances were created — never by library position (`triggers.triggerHoldersOf`).
   */
  deckTriggers?: TriggerDef[];
  /**
   * B5 E26: triggers this card answers while it lies in a graveyard ("While this is in your
   * graveyard: when one of your Traps activates, return this", Classic #47). R153's other graveyard
   * answer, the end-of-turn return, stays the `endOfTurn` hook's.
   */
  graveyardTriggers?: TriggerDef[];
  /**
   * B5 E33, R404: the card's quest tree (Classic #90 In Too Deep) — its quests, what completes each and
   * the rewards each offers, as data. `subsystems/quests.ts` keeps the count on the instance
   * (`memory.quest`), opens the first quest as the card enters the field and reports each completion
   * (`questCompleted`), which the card's own trigger answers with its rewards.
   */
  quests?: QuestBook;
  // ---- v0.2.0 script hooks, by workstream: Classic #46–#90 (card-specific, cards-classic-b) ----
  /**
   * Classic #88 Siphon Squad, R403: "When …, Tribute this" — a condition on the card's own text that
   * every state check reads (§4.5), the one right after the card arrives included; while it holds, the
   * card acting on the field (face-down too) is sacrificed (`stateCheck.ts`). A PURE READ, like `aura`.
   */
  tributeWhen?: (args: { state: GameState; self: CardInstance; radiant: boolean }) => boolean;
  /**
   * Classic #62 Living Bomb, R400: "At the start of your opponent's turn" — the `startOfTurn` hook of
   * the other side, queued right after the active player's at R62's start-of-turn trigger point, so
   * R68's order (the active player's cards, then the opponent's) holds.
   */
  startOfOpponentTurn?: Hook;
};

/**
 * B3.2, R384: one Activate ability. `uses` is "Activate" (1), "Activate N" (N) or "Activate ♾️"
 * ("unlimited", bounded by `ACTIVATE_UNLIMITED_CAP`); Degrade and Upgrade move a number by the tuning
 * key "Activate" (B3.4). `cost` is what the ability pays as it is activated — mana (Heroic Power's
 * "spend (X)", which v0.2.1 moves here), a random discard (Classic #15), a Tribute of the controller's
 * units (Classic #21, the card itself allowed), or the card itself (Classic #84). `targets` and
 * `modes` travel in the action as a play's do (R81). `canActivate` is a pure read for a condition the
 * text sets (Classic #7: "that Spell" must exist). `run` is the effect.
 */
export type ActivationDecl = {
  id: string;
  label: string;
  uses: number | "unlimited";
  /**
   * R384: `tribute` counts units, one each — "Tribute a Unit" is one unit, so a Sheep Token's "worth
   * 2" does not stretch it (that worth counts only toward a play's Tribute X, §6.3). The card itself
   * may be one of them when it is a Unit. `tributeSelf` is "Tribute this", which bypasses
   * Indestructible as every Sacrifice does (§6.3).
   */
  cost?: {
    mana?: number;
    discardRandom?: number;
    tribute?: number;
    tributeSelf?: boolean;
    /**
     * A Tribute cost that may not take the card itself, even when it is a Unit (Classic #21
     * Turtinator, which cannot Tribute itself; R635).
     */
    tributeExcludesSelf?: boolean;
  };
  targets?: TargetDecl[];
  modes?: ModeDecl[];
  canActivate?: ConditionHook;
  /**
   * Whether the card has this ability now, when that depends on the instance — an ability it lacks is
   * neither listed, shown nor accepted. Patch v0.2.1's Heroic Power declares one ability per power and
   * has only the one it rolled (B3.2 rule 10). Absent: always.
   */
  has?: (args: { state: GameState; self: CardInstance; radiant: boolean }) => boolean;
  run: Hook;
};

/**
 * R384: the `Resume.hook` an ability's own effect list runs under, `activation:<id>`, so a tail a
 * prompt parks comes back to the same ability (`work.scriptStepFor`), as a trigger's comes back by its
 * id. Distinct from every `Script` key.
 */
export const ACTIVATION_HOOK_PREFIX = "activation:";

export function activationHook(id: string): string {
  return `${ACTIVATION_HOOK_PREFIX}${id}`;
}

/**
 * R384, R102: a face's abilities with ids made unique — a card fused from two Activate cards has both
 * abilities, and the second of two that share an id is `<id>#2` — so the `activate` action, the view
 * and a resumed tail all name the same one. Order is the face's own.
 */
export function activationDecls(script: Script): ActivationDecl[] {
  const seen = new Map<string, number>();
  return (script.activations ?? []).map((decl) => {
    const count = (seen.get(decl.id) ?? 0) + 1;
    seen.set(decl.id, count);
    return count === 1 ? decl : { ...decl, id: `${decl.id}#${count}` };
  });
}

/**
 * B5 E3, R457: one draw limit a card sets. `player` is relative to the card's controller: "enemy" is
 * the opponent (Classic #4), "both" every player (Classic #49); `count` is how many draws that player
 * may make each turn, whoever's turn it is.
 */
export type DrawLimit = { player: "self" | "enemy" | "both"; count: number };

export type DrawLimitHook = (args: { state: GameState; self: CardInstance; radiant: boolean }) => DrawLimit[];

/**
 * §10.6: a card-specific target predicate (`TargetFilter.check`). `candidate` is the card a
 * declaration would offer (null for a hero or a zone), `self` the card declaring it, `player` the
 * chooser. A pure read: no writes, no rng.
 */
export type TargetCheck = (args: {
  state: GameState;
  self: CardInstance;
  player: PlayerId;
  radiant: boolean;
  candidate: CardInstance | null;
  selection: Selection;
}) => boolean;

// B5 E5, E9 (damage and combat): what `Script.replacements` holds, re-exported beside `Script` — above
// all `TargetedReplacement`, the one declaration of "a friendly unit is targeted" that an attack, a
// play, a cast, an activation and a prompt pick all answer through (`replacements.answerTargeting`).
export type {
  ReplacedEvent,
  ReplacementContext,
  ReplacementDef,
  ReplacementMoment,
  ReplacementWhere,
  TargetedReplacement,
} from "./replacements";

export type CardScripts = { base: Script; radiant: Script };

export const EMPTY_SCRIPT: Script = {};
