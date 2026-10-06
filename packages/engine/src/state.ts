// The state model of SPEC §10.1. Everything here is JSON: no functions, no class instances, no
// closures, so a state survives JSON.parse(JSON.stringify(state)) and a replay is exact (§9.3).

import type {
  CardDef,
  CardDefs,
  CardType,
  Enchantment,
  Keyword,
  PlayerId,
  PromptKind,
  Row,
  Selection,
  Tag,
  Tuning,
  Zone,
  ZoneRef,
} from "@jackioh/shared";
import { PLAYER_IDS } from "@jackioh/shared";
import type { GameEvent, GameOverReason } from "@jackioh/shared";
import {
  BACKROW_ZONES,
  DECK_SIZE,
  HERO_HEALTH,
  HUMAN_HANDICAP,
  LIBRARY_CAP,
  SETUP_TURN,
  UNIT_ZONES,
  type Handicap,
} from "./config";
import { registerCatalog, registeredCatalog } from "./catalog";
import type { CostRule } from "./costRules";
import { showToOwner } from "./ownLibrary";
import { freezeLastBoards } from "./subsystems/lastBoards";
import { createRng } from "./rng";

export type Phase = "setup" | "mulligan" | "start" | "main" | "end" | "over";
export type Position = "ATK" | "DEF";

export type CardInstance = {
  id: string;
  defId: string;
  owner: PlayerId;
  controller: PlayerId;
  radiant: boolean;
  zone: Zone;
  position?: Position;
  summonedTurn?: number;
  damage: number;
  buffs: { attack: number; health: number };
  grantedKeywords: Keyword[];
  vanilla: boolean;
  costMod: number;
  costOverride?: number;
  x?: number;
  embiggened?: boolean;
  counters: { plague?: number; grade?: number };
  memory: Record<string, unknown>;
  /**
   * §4.1: `attacked` and `switched` spend the turn's exertion. R636: `attacks` counts the attacks
   * declared this turn once there is a second one (absent, it is 1 when `attacked` and 0 otherwise)
   * against `combat.attacksPerTurn`, two with Windfury.
   */
  exertion: { attacked: boolean; switched: boolean; attacks?: number };
  statsOverride?: { attack: number; health: number };
  /**
   * §7: the Bread Token's radiant face prints "Armor X", where X is the same unspent-mana X its
   * stats use — so it cannot be a printed number any more than its X/X can. Set beside
   * `statsOverride` by whoever summons it, and substituted into the printed `Armor` keyword by
   * `faceOf`. Inert until the token is radiant, because only the radiant face prints Armor.
   */
  armorOverride?: number;
  returnToHandAtEndOfTurn?: boolean;
  /** Indestructible would-destroy: no Taunt for this turn (R46). */
  tauntSuppressedTurn?: number;
  /** A backrow card whose identity is public, e.g. a Field Trap that has fired (R33). */
  faceUp?: boolean;
  /**
   * R638: a backrow Trap or Field Trap both players may read while it stays armed — revealed, not
   * face-up, so it still fires. Cleared by R78's reset with the card leaving the field.
   */
  revealed?: boolean;
  /**
   * Instance id of the source whose damage instance was lethal — the hit that took this unit from
   * above 0 health to 0 or less, or a Poisonous hit — for "destroys a unit" (R42, R89). Unset while
   * no hit has killed it (`damage.creditKiller`).
   */
  lastDamagedBy?: string;
  /** Divine Shield has absorbed a hit and is gone until granted again (§6.1). */
  divineShieldSpent?: boolean;
  /** Destroyed by an effect; the next state check collects it (§4.5, §6.3 Destroy). */
  markedDestroyed?: boolean;
  /** Came back through Reborn, so it no longer has it (§4.5 step 4). */
  rebornSpent?: boolean;
  /**
   * R311: what this card's owner was shown of it as it went into their library — its definition and
   * its face — written by `ownLibrary.showToOwner` where a card goes in openly and read by `viewFor`
   * for the owner's library list (R310) and by nothing else. Absent on a library card its owner was
   * never shown (R312). A change made to the card inside the library, where nobody sees it, leaves
   * this record as it was.
   */
  knownAs?: { defId: string; radiant: boolean };
  // ---- Patch v0.2.0: what rides a card through every zone (R78's reset leaves these alone) ----
  /**
   * B3.4, R386: what Degrade, Upgrade and KY's Constant have changed on this card (`tuning.ts`). Kept
   * in every zone and through leaving the field; a copy keeps it, a Transform makes a card without
   * it, a Fuse sums it (R57, R102).
   */
  tuning?: Tuning;
  /**
   * B3.3, R385, R638: the card's Brittle count and the turn it started, which the first tick waits two
   * player-turns behind (`brittle.ts`). Kept in every zone but ticking on the field only: a card in a
   * hand or a deck holds its count, and `since` is set again as the card enters the field from one. A
   * copy never inherits it (R57), and a count that has crumbled its card (0) is spent and goes with
   * R78's reset (R441). `printed` marks a
   * count its printed Brittle started as the card entered the field, which a Vanilla switches off
   * while a given one stays (B3.3 rule 5).
   */
  brittle?: { count: number; since: number; printed?: true };
  /** B5 E39: lasting instructions riding the card through every zone (`enchantments.ts`). */
  enchantments?: Enchantment[];
  // ---- v0.2.0 instance fields, by workstream: field (B3.1, E21, E22) ----
  // ---- v0.2.0 instance fields, by workstream: play pipeline (E1, E2, E4, E5, E11, E12, E15) ----
  // ---- v0.2.0 instance fields, by workstream: activate and turn (B3.2, E3, E10, E27, E28) ----
  // ---- v0.2.0 instance fields, by workstream: damage and combat (E5, E6, E9, E35, E37) ----
  /**
   * B5 E35: this unit has gone Berserk (Classic+ #19.2 sends Classic+ #19.5 there) — a status an
   * effect sets (`effects/statuses.goBerserk`), never text, so a Vanilla keeps it; R78's reset takes
   * it off with the card leaving the field. Its own card makes the forced attacks it owes.
   */
  berserk?: true;
  // ---- v0.2.0 instance fields, by workstream: prompts and generation (E13, E16–E19, E23–E26) ----
  // ---- v0.2.0 instance fields, by workstream: Core patches (R426–R437) ----
  /**
   * R429: how many times this card has been played, the play under way included — counted at §10.5
   * step 4 (casts too, R70; a countered play never reaches it) for a card whose script asks
   * (`StaticFlags.countsPlays`, #31 KY's Math Equation) and absent on every other card. Kept in every
   * zone and through leaving the field, like `costMod` (R78's reset leaves it alone); a copy or a
   * Transform is a new card with a count of its own (R57).
   */
  timesPlayed?: number;
};

/**
 * B5 E12, R452: one cast being driven that makes its caster's choices at random (`random`), narrows
 * its target picks to enemies when one is legal (`targetEnemies`), or both. `casts` counts the casts
 * a random cast's resolution has made in all, itself included, against RANDOM_CAST_CHAIN_CAP.
 */
export type CastMode = {
  instanceId: string;
  player: PlayerId;
  random: boolean;
  targetEnemies: boolean;
  casts: number;
};

/** A unit zone holds a Stack pile, top card first (§3.2). */
export type Pile = CardInstance[];

/** B3.1 rule 6: an animated "Animated on your turn" card's backrow zone, held for its return (SPEC §10.1). */
export type HomeZone = { instanceId: string; zone: ZoneRef };

export type ModifierExpiry =
  | { until: "thisTurn"; turn: number }
  /** Lasts through that player's next turn; `fromTurn` is the turn it was created on (R48). */
  | { until: "nextTurnOf"; player: PlayerId; fromTurn: number }
  | { until: "used" }
  | { until: "never" };

export type PlayerModifier = {
  id: string;
  expiry: ModifierExpiry;
} & (
  /** `minCurrentCost`: R48, R363 — only a card whose cost is then this or more (#77, "Cost (4)+"). */
  | { kind: "costDiscount"; amount: number; onlyType?: "Spell"; minCurrentCost?: number; oncePerTurn?: boolean }
  | { kind: "echoNextSpell"; amount: number; sourceId?: string }
  | { kind: "radiantFirstCheapCard"; maxCost: number; usedTurn?: number }
  | { kind: "comboDraw"; amount: number }
  | { kind: "quickstrikerDamage" }
  // ---- v0.2.0 modifier kinds, by workstream: play pipeline (E15 costs, E39 stamping, Devil's Pact) ----
  // play pipeline B (E15, E39; R455). A price rule on this player's cards (`costRules.ts`): Classic #2's
  // "your next Trap or Field Spell costs (2) less" or "costs (0)" (until used, spent by the play it
  // priced), AI Alignment Tax's "(1) more during their next turn" (R48's `nextTurnOf`).
  | { kind: "costRule"; rule: CostRule }
  /** Classic+ #14 Forever&: the next Spell its player plays gains this enchantment (E39), until used. */
  | { kind: "enchantNextSpell"; enchantment: Enchantment }
  /**
   * Classic #23 Devil's Pact, R449: each card this player plays (a cast included, R70) is replaced at
   * §10.5 step 3 by a new instance of `defId`, Radiant when `radiant` says so, which resolves as that
   * play (`playSteps.replacePlayedCard`).
   */
  | { kind: "replacePlays"; defId: string; radiant: boolean }
  // ---- v0.2.0 modifier kinds, by workstream: activate and turn (E28 rest of the game) ----
  /**
   * B5 E10, R456: this player's turn ends once `actionsLeft` more main-phase actions of theirs have
   * resolved. 0 is "your turn ends" — as soon as what is resolving now has resolved (Classic+ #26's
   * "End your turn", the AI card Rate Limit) — and 1 is "you may take one more action, then your turn
   * ends" (Classic+ #26 Radiant). `byInstanceId` is the card whose effect it is. Expiry `thisTurn`:
   * ending the turn any other way ends it too (`reduce.ts` counts the actions and ends the turn).
   */
  | { kind: "turnEnds"; actionsLeft: number; byInstanceId: string | null }
  /**
   * B5 E28, R458: "For the rest of the game: at the start of your turn, …" (Classic+ #52). `resume`
   * re-enters the card's step at each start of this player's turn, in R62's delayed-effect stage
   * among the delayed effects in creation order (`seq`); `ranTurn` is the turn it last ran, so a
   * prompt that pauses the stage never runs it twice. `label` is its badge (R169), the card's own
   * words. Expiry `never`; several stack, each its own modifier.
   */
  | { kind: "startOfTurnEffect"; seq: number; resume: Resume; label: string; ranTurn?: number }
  // ---- v0.2.0 modifier kinds, by workstream: damage and combat (E8 heal into damage) ----
  /**
   * B5 E8: a heal of X on one of this player's enemies — the enemy hero or an enemy unit — deals X
   * Pierce damage to it instead, from the converting card (Classic+ #22 Blood Moon's base face, "for
   * the rest of this turn"). `converterId` rather than `sourceId`: the converter is a Trap already in
   * its graveyard, and `endOrphanedModifiers` ends a `sourceId` modifier whose card left the field.
   */
  | { kind: "healToDamage"; converterId: string }
  /**
   * R757: #98's Armor Up — Armor on this player's hero, added to `damage.heroArmorOf`'s total from the
   * moment it is gained until the cleanup that ends the opponent's next turn takes it off (expiry
   * `nextTurnOf` the opponent), so it is gone when this player's next turn begins.
   */
  | { kind: "heroArmor"; amount: number }
);

export type DelayedEffect = {
  id: string;
  /** Whose script scheduled it, for R68's creation order. */
  seq: number;
  owner: PlayerId;
  at: { phase: "start" | "end"; player: PlayerId };
  /**
   * B5 E27, R458: the first turn number whose boundary may run it — "at the end of your *next* turn"
   * (Classic #37 Radiant) is made with the current turn plus one, so the end of the turn it was made
   * on passes it by. Absent: the next such boundary.
   */
  notBefore?: number;
  /** A serializable continuation: script id, hook name, captured data (§10.6). */
  resume: Resume;
  /**
   * R174: the instance this effect is aimed at, when it is aimed at one on the field (#50 Kpop
   * Fanatic's chosen permanent). The entry is dropped the moment that card leaves the field
   * (`zones.moveToZone`), so a card that comes back — bounced and replayed, or a Reborn body — is a
   * new arrival the effect never chose, and R76's "fizzles if the target has left the field" holds.
   */
  watch?: string;
};

export type Resume = {
  defId: string;
  hook: string;
  /** A named step, so a continuation reads as the script wrote it (§10.6). */
  step: string;
  radiant: boolean;
  /** The instance the script belongs to, when it still exists. */
  instanceId?: string;
  data: Record<string, unknown>;
};

/**
 * One paused step of an engine sequence (§9.3: "mid-action choices are state, not callbacks").
 * A work item never holds effects — those are closures — it names the continuation to re-enter,
 * so a state with paused work survives JSON and replays exactly (`src/work.ts`).
 */
/** §10.3: an emitted event waiting for the trigger loop to dispatch it. */
export type DispatchItem = {
  id: string;
  seq: number;
  event: GameEvent;
};

/**
 * R30: a spell's pending Echo repeats. A repeat waits here while a prompt from the first
 * resolution is still open, so the sequence survives the pause (§10.6, `src/work.ts`).
 */
export type EchoItem = {
  id: string;
  seq: number;
  instanceId: string;
  controller: PlayerId;
  /** Repeats still owed to this instance. */
  remaining: number;
};

export type WorkItem = {
  id: string;
  /** Creation order, so the queue is deterministic (R68). */
  seq: number;
  owner: PlayerId;
  resume: Resume;
};

export type PromptOption = {
  key: string;
  label: string;
  selection: Selection;
  /** B5 E18: what this option counts against a `pick` prompt's `budget` (R65's cost where it lies). */
  cost?: number;
  /** A face the option shows: the card it offers is Radiant, or a definition is offered Radiant. */
  radiant?: true;
};

export type PendingChoice = {
  id: string;
  playerId: PlayerId;
  kind: PromptKind;
  prompt: string;
  options: PromptOption[];
  min: number;
  max: number;
  /**
   * B5 E18: a `pick` prompt's budget — the most its picked options' `cost`s may add up to (Classic
   * #44's "a total cost of (5) or less"). Absent on every other prompt, so a state without one hashes
   * as it did before the field existed.
   */
  budget?: number;
  resume: Resume;
};

/**
 * §2.1 step 3, R265: one seat's mulligan while the mulligans are open. `prompt` is the seat's own
 * prompt, whose options are its opening hand; `keep` is its sealed answer — the ids it keeps — and
 * null until it answers. Nothing reads an answer before both seats have given one (R266).
 */
export type MulliganSeat = { prompt: PendingChoice; keep: string[] | null };

export type QueuedTrigger = {
  id: string;
  seq: number;
  instanceId: string;
  hook: string;
  resume: Resume;
};

/**
 * §4.2 step 4 and R44: the attack whose trap window is open — the moment between a declaration,
 * which has already spent the attacker's exertion, and the damage of step 5. `src/combat.ts` opens
 * it, a trap that fires inside it closes it with `effects/combat.cancelAttack` (§6.3 "Cancel an
 * attack"), and step 5 reads it back to find out whether there is still a combat to resolve.
 *
 * Ids and flags only, like the rest of §10.1: no instances and no closures, so the field survives
 * `cloneState`'s JSON round trip. That is not decoration here — My Pawn's window hands the rest of
 * the turn to the AI policy, which drives `reduce`, which clones, so by the time step 5 runs every
 * `CardInstance` the declaration was built from is a different object and only the ids still name
 * the same cards.
 */
export type DeclaredAttack = {
  /**
   * This declaration, told apart from one opened inside its own window (R44's AI turn takes
   * actions of its own). Deterministic, from `nextSeq`, so a replay mints the same ids.
   */
  id: string;
  /** The attacker's instance id. */
  attackerId: string;
  /** §4.2 step 2's two possibilities: an enemy unit's instance id, or `hero-<player>`. */
  targetId: string;
  /** Set by `cancelAttack` inside the window, so step 5 resolves no combat (§6.3, R44). */
  cancelled: boolean;
  /** R220: the player who declared it, whose unit the attacker must still be at step 5. */
  by?: PlayerId;
  /** R220, R174: the field's departures when it was declared (`stays.exitMark`). */
  exitsFrom?: number;
};

export type TurnLog = {
  playedIds: string[];
  cardsPlayed: number;
  unspentAtEnd?: number;
  /**
   * The cost each play this turn actually paid (R56), in play order beside `playedIds`, a cast's 0
   * included (R70). #64 Gifted Program's "the first card costing 1 or less you play each turn" is
   * the player's count, not the card's (R213). Optional so a log written without it reads as no
   * plays; `startTurn` rebuilds the log, which clears it.
   */
  costsPaid?: number[];
  /**
   * B5 E4: this turn's plays by the type each was played as (B2.7), casts included (R70), countered
   * plays never — Classic+ #37 Wardrum counts Spells, Field Spells and Traps. `startTurn` rebuilds
   * the log for both players, which clears it as it clears the rest of "this turn".
   */
  playedByType?: Partial<Record<CardType, number>>;
};

export type PlayerState = {
  hero: { health: number; armor: number };
  mana: { current: number; max: number; nextTurnMod: number; permMod: number };
  hand: CardInstance[];
  library: CardInstance[];
  graveyard: CardInstance[];
  exile: CardInstance[];
  /** Cards mid-resolution: a Spell sits here between its play and its graveyard (§10.5). */
  resolving: CardInstance[];
  units: (Pile | null)[];
  backrow: (CardInstance | null)[];
  locks: { units: boolean[]; backrow: boolean[] };
  mods: PlayerModifier[];
  turnLog: TurnLog;
  drawOffer: { offeredTurn?: number; blockedUntil?: number };
  fatigueCount: number;
  /** Turns this player has started, for the mana refresh (§2.3). */
  turnsStarted: number;
  /** My Pawn: the AI policy plays out the rest of this turn (R44). */
  aiTurn: boolean;
  /**
   * R180: this seat's handicap. Absent means HUMAN_HANDICAP, and createGame never stores one equal
   * to it, so a game without handicaps hashes exactly as it did before this field existed.
   */
  handicap?: Handicap;
  /**
   * R345: `false` once this player has turned R82's automatic turn end off (`setAutoEndTurn`).
   * Absent means on, and turning it back on deletes the field, so a game in which nobody touched
   * the setting hashes exactly as it did before this field existed.
   */
  autoEndTurn?: false;
  // ---- v0.2.0 player fields, by workstream: field (B3.1, E20, E21, E22) ----
  /**
   * B5 E21: the dormant cards beneath each backrow zone's top card, top first, by lane (index lane −
   * 1). The top stays in `backrow` and is the one card that acts there (§3.2, R13); these are face-down
   * and not on the field for effects (`zones.isBuried`). Absent while no backrow zone holds a pile, so a
   * game that never builds one hashes as it did before the field existed.
   */
  backrowPiles?: CardInstance[][];
  /**
   * B5 E21, R446: the Unit a carrier in each backrow zone holds, by lane — a Unit played on top of a
   * backrow card whose static flag lets one (Classic+ #33 Ivory Tower). It stands in that backrow
   * zone (its `zone.row` is "backrow"), is a Unit for every rule (`zones.activeUnitsOf`), and can
   * neither attack nor be attacked (`zones.isCarried`). Absent while nothing is carried.
   */
  carried?: (CardInstance | null)[];
  // ---- v0.2.0 player fields, by workstream: play pipeline (E4 play counters, E11) ----
  /**
   * B5 E4, R451: what this player's plays leave for the rest of the game, never reset
   * (`playCounts.ts`): `playedByTag`, their plays by tag (Classic+ #64's Fruit, AI Scaling Law's AI),
   * casts included (R70) and countered plays never; `lastFaceUpPlay`, the last face-up card they
   * played (AI Autocomplete). Absent until their first play, so a game without one hashes as it did
   * before this field existed.
   */
  gameLog?: GameLog;
  // ---- v0.2.0 player fields, by workstream: activate and turn (E3, E4 draw counts, E10) ----
  /**
   * B5 E3, E4, R457: how many draws this player has made on turn `turn`, whoever's turn it is — a
   * fatigue draw included, a draw a limit stopped not. A count kept for an earlier turn reads as 0,
   * so it resets where the turn log does without anything clearing it (`draw.drawsThisTurn`).
   */
  draws?: { turn: number; count: number };
  // ---- v0.2.0 player fields, by workstream: damage and combat (E5–E9, E35) ----
  // ---- v0.2.0 player fields, by workstream: Core patches and cosmetics (R433, R434) ----
};

export type GameState = {
  seed: string;
  rngCursor: number;
  /** Player-turn counter, 1-based, capped by TURN_CAP_PLAYER_TURNS (§2.5, R2). */
  turn: number;
  active: PlayerId;
  phase: Phase;
  players: Record<PlayerId, PlayerState>;
  pending: PendingChoice | null;
  triggerQueue: QueuedTrigger[];
  /** The attack whose trap window is open, between declaration and damage (§4.2 step 4, R44). */
  declaredAttack: DeclaredAttack | null;
  /** Paused sequences waiting to continue, in order (§9.3, §10.6). */
  work: WorkItem[];
  /**
   * R113: how many items the *current* pause cascade has parked. A scope parks its remainder at
   * this index and advances it, so one cascade lands innermost-first, and taking an item resets it
   * to 0 so the next cascade is inserted ahead of everything still owed. Neither a plain queue nor
   * a plain stack is correct: a Cry's parked tail must run before the play steps that follow it,
   * while a prompt opened *inside* that tail must run before both.
   */
  workCursor: number;
  /** Echo repeats owed but not yet resolved (R30). */
  echoQueue: EchoItem[];
  /** Events emitted but not yet dispatched to triggers (§10.3). */
  dispatch: DispatchItem[];
  delayed: DelayedEffect[];
  /** Ceaseless Void's four game counters (R55). */
  counters: { drawn: number; played: number; destroyed: number; exiled: number };
  transientDefs: Record<string, CardDef>;
  /** Zones a dying Reborn unit holds until it returns (R64). */
  reserved: { player: PlayerId; row: Row; lane: number }[];
  /** Players whose mulligan has resolved, in seat order (§2.1, R265). */
  mulliganed: PlayerId[];
  /**
   * §2.1 step 3, R265: both seats' mulligans, open at once. Present only while they are open — the
   * opening deal done, `pending` null, phase `mulligan` — and gone the moment the second answer
   * resolves them, so a state past its mulligan hashes as it did before this field existed.
   * `pending` stays the one prompt §10.1 allows; the two mulligans are the one sealed-bid step.
   */
  mulligan?: Record<PlayerId, MulliganSeat>;
  result: null | { winner: PlayerId | "draw"; reason: GameOverReason };
  /** Next instance/choice/trigger id, so ids are deterministic under replay. */
  nextId: number;
  /** Monotonic sequence for R68's creation order. */
  nextSeq: number;
  /** Nonce dedupe: the events each already-applied action produced (§9.3). */
  applied: { nonce: string; events: GameEvent[] }[];
  /**
   * R217: how many cards the cast-on-draw chain that is running has cast, draws made by its casts
   * included, so R58's cap bounds the whole chain. Present only while a chain runs (a pause inside
   * one keeps it here for the answer), and gone once the draw that began it has finished.
   */
  castChain?: number;
  /**
   * R174: the field's departures, counted, and each card's latest (`stays.ts`). An effect aimed at a
   * card on the field is aimed at that stay, and a sequence a prompt splits resumes in a later
   * action whose event list does not hold what happened before the pause, so "has this card left
   * the field since" is read off this record, which survives the pause. Absent until a card first
   * leaves the field.
   */
  fieldExits?: FieldExits;
  // ---- v0.2.0 game fields, by workstream: field (B3.1 home zones, E21) ----
  /**
   * B3.1 rule 6, R383: the backrow zone each animated "Animated on your turn" card goes back to at
   * its controller's cleanup, held for it meanwhile as R64 holds a dying Reborn unit's zone
   * (`zones.isReserved` reads both). Absent while no such card is animated.
   */
  homes?: HomeZone[];
  // ---- v0.2.0 game fields, by workstream: play pipeline (E1 announce, E4 last plays, E12) ----
  /**
   * play pipeline B (E12, R452): the casts being driven right now that change how choices are made —
   * a random cast (every choice its caster makes answered from the rng) or a cast that targets
   * enemies when it can — innermost last. Present only while such a cast's steps run
   * (`randomCast.withCastMode`), so a state at rest, a paused one included, never carries it.
   */
  castsResolving?: CastMode[];
  /**
   * B5 E1, R448: the plays whose announce window is open, innermost last (a cast a responder makes
   * announces inside the window it answers). Present only while one is open (`announce.ts`).
   */
  announcing?: AnnounceRecord[];
  /** B5 E4: the last Spell anyone played (Classic #57 Echo), overwritten by the next, never cleared. */
  lastSpell?: PlayRecord;
  /**
   * R58: the cards a cast-on-draw draw is casting, by the id each was drawn under, whose `drawn` is
   * held from every dispatch until that cast has resolved — the draw's "complete" point
   * (`drawComplete.ts`). Present only while one is held.
   */
  heldDraws?: string[];
  // ---- v0.2.0 game fields, by workstream: activate and turn (E10) ----
  // ---- v0.2.0 game fields, by workstream: damage and combat (E5) ----
  // ---- v0.2.0 game fields, by workstream: prompts and generation (E17, E18, E26) ----
  // ---- v0.2.0 game fields, by workstream: Core patches and cosmetics (R433) ----
  /**
   * R437: the marks cards carry for an effect aimed at them that is still to come — #50 K-Pop
   * Fanatic's pending steal on its target — one entry per mark, tied to the delayed effect that
   * made it (`marks.ts`). `viewFor` puts them on the card in both views; the mark goes when its
   * effect resolves or fizzles, or is dropped because its card left the field (R174), and a
   * `marked` event says so each way. Absent when no card is marked, so a game without marks hashes
   * as it did before the field existed.
   */
  marks?: MarkRecord[];
  // ---- v0.2.0 game fields, by workstream: cards-plus-c (E30) ----
  /**
   * R417: each seat's last board, a `createGame` input frozen into the match and never written again
   * (`subsystems/lastBoards`). Only a seat with one has a key, and a game with none has no field, so
   * it hashes as it did before. `viewFor` never sends it.
   */
  lastBoards?: Partial<Record<PlayerId, LastBoardEntry[]>>;
  /**
   * C+ #35 Rollback, R419: the field as each of the last BOARD_HISTORY_DEPTH turns began, oldest first
   * (`subsystems/boardHistory.ts`). Never in a view (§10.8). Absent until the first turn starts.
   */
  boardHistory?: BoardSnapshot[];
  // ---- the Glitch Easter egg (issue #170; R673–R679) ----
  /** R673: how many "… in the System" cards either player has played. Absent at 0, so a match without one hashes as before. */
  systemPlays?: number;
  /** R676: the decks and dealt seats `createGame` began with, which a reset deals again. Never in a view. */
  opening?: OpeningRecord;
  /** R676: how many times Glitch has reset the match; keys the reset's id numbering. Absent until the first. */
  resets?: number;
  /** R676: a reset Glitch owes, done once the action that drew it has settled (`subsystems/glitch`). */
  resetOwed?: true;
  /** R677: how many times Glitch has swapped the seats; odd means each account now holds the other seat. */
  seatSwaps?: number;
  /**
   * R678: the two boards of other players' games a Glitch may put on the field, a `createGame` input
   * frozen like `lastBoards` (R417, R564). Never in a view.
   */
  glitchBoards?: Partial<Record<PlayerId, LastBoardEntry[]>>;
};

/** R676: what a Glitch reset deals again — `createGame`'s decks and dealt seats. */
export type OpeningRecord = { decks: [string[], string[]]; dealt?: PlayerId[] };

/** R417: one card of a last board — the card and its face, never stats, buffs or damage. */
export type LastBoardEntry = { defId: string; radiant: boolean };

/** R417: each seat's last board in seat order, as `createGame` and `replay.fold` take them. */
export type LastBoardInput = readonly [readonly LastBoardEntry[], readonly LastBoardEntry[]];

/** R437: one mark on one card, while the delayed effect `delayedId` waits (`marks.ts`). */
export type MarkRecord = { instanceId: string; mark: string; color: string; delayedId: string };

/** R419: one side of the field as a turn began — its zones' cards whole, its Locks, the homes held then. */
export type SideSnapshot = Pick<PlayerState, "units" | "backrow" | "backrowPiles" | "carried" | "locks"> & { homes?: HomeZone[] };

/** R419: the field at the start of player-turn `turn` (`subsystems/boardHistory.ts`). */
export type BoardSnapshot = { turn: number; sides: Record<PlayerId, SideSnapshot> };

/** R419: every card one side of a snapshot holds. */
export function sideSnapshotInstances(side: SideSnapshot): CardInstance[] {
  return [
    ...side.units.flatMap((pile) => pile ?? []),
    ...side.backrow.flatMap((card) => (card === null ? [] : [card])),
    ...(side.backrowPiles ?? []).flat(),
    ...(side.carried ?? []).flatMap((card) => (card === null ? [] : [card])),
  ];
}

/** R227, R419: a card that took a fresh id is still the card the history recorded, so the history follows it. */
export function renameInBoardHistory(state: GameState, from: string, to: string): void {
  for (const snapshot of state.boardHistory ?? []) {
    for (const player of PLAYER_IDS) {
      for (const card of sideSnapshotInstances(snapshot.sides[player])) if (card.id === from) card.id = to;
      for (const home of snapshot.sides[player].homes ?? []) if (home.instanceId === from) home.instanceId = to;
    }
  }
}

/**
 * R174: `count` departures so far; `last` maps a card to the departure that was its latest.
 * `uncovered` (R212, §3.2) maps a card that has left the top of a Stack pile — died, bounced,
 * exiled, stolen, fused away — to the note of that removal (`stays.noteUncovered`), so the events
 * reporting its leaving, and every event before them, are not answered by the card it uncovered.
 */
export type FieldExits = { count: number; last: Record<string, number>; uncovered?: Record<string, UncoveredNote> };

/**
 * R212, §3.2: `resumed` is the dormant card that became its pile's top when the card left.
 * `reported`: the loop has dispatched a report of that removal (`stays.noteReported`). `movedOn`: the
 * card that left has moved zones again since (`stays.noteMoved`). The note goes once both are true.
 */
export type UncoveredNote = { resumed: string; reported?: boolean; movedOn?: boolean };

// ---- v0.2.0 play pipeline A (E1 announce, E4 records) ----

/**
 * B5 E1, R448: one play or cast between its announce and §10.5 step 4. `faceDown` is a card that
 * will be set face-down (a Trap or Field Trap): while it waits in the resolving zone only `player`
 * reads it (`viewFor`). `countered` is set by the Counter that cancelled it (`effects/move.counterPlay`).
 */
export type AnnounceRecord = {
  instanceId: string;
  player: PlayerId;
  faceDown?: true;
  countered?: true;
};

/** B5 E4: a card as a play record names it — the definition and the face it was played with. */
export type PlayRecord = { defId: string; radiant: boolean };

/** B5 E4: a face-up play's record, with the type it was played as (B2.7). */
export type FaceUpRecord = PlayRecord & { type: CardType };

/** B5 E4, R451: a player's per-game play record (`PlayerState.gameLog`). */
export type GameLog = { playedByTag: Partial<Record<Tag, number>>; lastFaceUpPlay?: FaceUpRecord };

function emptyRow<T>(size: number): (T | null)[] {
  return Array.from({ length: size }, () => null);
}

function emptyLocks(size: number): boolean[] {
  return Array.from({ length: size }, () => false);
}

export function createPlayerState(): PlayerState {
  return {
    hero: { health: HERO_HEALTH, armor: 0 },
    mana: { current: 0, max: 0, nextTurnMod: 0, permMod: 0 },
    hand: [],
    library: [],
    graveyard: [],
    exile: [],
    resolving: [],
    units: emptyRow<Pile>(UNIT_ZONES),
    backrow: emptyRow<CardInstance>(BACKROW_ZONES),
    locks: { units: emptyLocks(UNIT_ZONES), backrow: emptyLocks(BACKROW_ZONES) },
    mods: [],
    turnLog: { playedIds: [], cardsPlayed: 0 },
    drawOffer: {},
    fatigueCount: 0,
    turnsStarted: 0,
    aiTurn: false,
  };
}

export type CreateGameOptions = {
  seed: string;
  decks: [string[], string[]];
  /** Registers the catalog for this process; omit when it is already registered. */
  catalog?: CardDefs;
  /** R180: per-seat handicaps. An omitted seat, or one equal to HUMAN_HANDICAP, stores nothing. */
  handicaps?: Partial<Record<PlayerId, Handicap>>;
  /**
   * R433: the seats whose deck their player was dealt rather than built — All Random's (R258),
   * practice's fresh random deck. Their starting library is written no record of what its owner
   * was shown (R311), so its cards list as unknown until they leave it (R312). Setup, not an
   * action: a replay passes the same list (`replay.ReplayInput.dealt`). Omitted, every deck was built.
   */
  dealt?: readonly PlayerId[];
  /**
   * R417: each seat's last board (C+ #29). Setup, not an action: a replay passes the same boards
   * (`replay.ReplayInput.lastBoards`). Omitted, both are empty (hotseat, practice, a first game).
   */
  lastBoards?: LastBoardInput;
  /**
   * R678: the boards of two other players' games a Glitch may put on the field, one per seat. Setup,
   * not an action, frozen as `lastBoards` is (R564): a replay passes the same boards
   * (`replay.ReplayInput.glitchBoards`). Omitted, a Glitch's boards outcome leaves both fields empty.
   */
  glitchBoards?: LastBoardInput;
};

/** The five fields of a handicap, in §9.9's order, so every reader walks the same list. */
const HANDICAP_FIELDS = [
  "deckSize",
  "manaBonus",
  "manaCap",
  "extraOpeningCards",
  "extraDrawsPerTurn",
] as const satisfies readonly (keyof Handicap)[];

/** R180: the handicap a seat plays with. A seat that stores none has this spec's resources. */
export function handicapOf(side: PlayerState): Handicap {
  return side.handicap ?? HUMAN_HANDICAP;
}

/** R290: the health a seat's hero starts with — its handicap's `heroHealth`, else §2's HERO_HEALTH. */
export function startingHeroHealth(handicap: Handicap | undefined): number {
  return handicap?.heroHealth ?? HERO_HEALTH;
}

/** R180: whether a handicap is exactly a human's, which is what `createGame` declines to store. */
function isHumanHandicap(handicap: Handicap): boolean {
  return (
    HANDICAP_FIELDS.every((field) => handicap[field] === HUMAN_HANDICAP[field]) &&
    startingHeroHealth(handicap) === HERO_HEALTH
  );
}

/**
 * R180, R184: a handicap is five non-negative integers, and its deck size is one a library can
 * hold (R80's LIBRARY_CAP). Throws naming the seat and the field, as `validateDeck` does.
 */
export function validateHandicap(handicap: Handicap, label: string): void {
  for (const field of HANDICAP_FIELDS) {
    const value: unknown = handicap[field];
    if (typeof value !== "number" || !Number.isInteger(value) || value < 0) {
      throw new Error(`${label}: handicap ${field} must be a non-negative integer (R180), got ${String(value)}`);
    }
  }
  if (handicap.deckSize < 1 || handicap.deckSize > LIBRARY_CAP) {
    throw new Error(
      `${label}: handicap deckSize must be between 1 and ${LIBRARY_CAP} (R184), got ${handicap.deckSize}`,
    );
  }
  // R290: optional, and when given a hero that starts alive.
  const heroHealth: unknown = handicap.heroHealth;
  if (heroHealth !== undefined && (typeof heroHealth !== "number" || !Number.isInteger(heroHealth) || heroHealth < 1)) {
    throw new Error(`${label}: handicap heroHealth must be a positive integer (R290), got ${String(heroHealth)}`);
  }
}

/**
 * §2.6 and §9.4 L2, L3, L6: the rules a deck must satisfy before a game exists. `size` is the
 * seat's handicap deck size (R184) and defaults to DECK_SIZE, whose message is §2.6's own; any
 * other size is the handicap's, and the message says so.
 */
export function validateDeck(
  deck: readonly string[],
  catalog: CardDefs,
  label: string,
  size: number = DECK_SIZE,
): void {
  if (deck.length !== size) {
    if (size === DECK_SIZE) {
      throw new Error(`${label}: deck must hold exactly ${DECK_SIZE} cards (§2.6 L2), got ${deck.length}`);
    }
    throw new Error(`${label}: deck must hold exactly ${size} cards (its handicap, R184), got ${deck.length}`);
  }
  const seen = new Set<string>();
  for (const defId of deck) {
    const def = catalog[defId];
    if (def === undefined) {
      throw new Error(`${label}: "${defId}" is not in the catalog (§9.4 L6)`);
    }
    if (seen.has(defId)) {
      throw new Error(`${label}: "${defId}" appears twice; no duplicate card ids (§2.6 L3)`);
    }
    seen.add(defId);
    if (def.token || def.tags.includes("Token")) {
      throw new Error(`${label}: "${defId}" is a Token card and cannot be in a deck (§2.6 L3)`);
    }
  }
}

export function newInstance(
  state: Pick<GameState, "nextId">,
  defId: string,
  owner: PlayerId,
  zone: Zone,
): CardInstance {
  const instance: CardInstance = {
    id: `c${state.nextId}`,
    defId,
    owner,
    controller: owner,
    radiant: false,
    zone,
    damage: 0,
    buffs: { attack: 0, health: 0 },
    grantedKeywords: [],
    vanilla: false,
    costMod: 0,
    counters: {},
    memory: {},
    exertion: { attacked: false, switched: false },
  };
  state.nextId += 1;
  return instance;
}

/**
 * A game in phase `setup`: libraries hold the decks in list order, and `setup.ts` (M1-T5)
 * shuffles them with the match rng and deals the opening hands.
 *
 * R180: each seat's handicap is validated first, so a bad deck size is named as the handicap's
 * fault; then each deck is checked against its seat's deck size (R184). A handicap is stored on the
 * seat only when it differs from a human's, so a game with none — or with Easy's, which *is* a
 * human's — carries no `handicap` key and hashes and replays exactly as before the field existed.
 */
export function createGame(options: CreateGameOptions): GameState {
  return buildGame(options, 1, "");
}

/**
 * R676: the new game a Glitch reset deals — `createGame`'s, with ids numbered on from `nextId` in an
 * order drawn from a stream of this reset's own (R223), so no id the old game showed names a card of
 * the new one.
 */
export function createGameForReset(options: CreateGameOptions, at: { nextId: number; resets: number }): GameState {
  return buildGame(options, at.nextId, `:reset-${at.resets}`);
}

function buildGame(options: CreateGameOptions, firstId: number, stream: string): GameState {
  if (options.catalog !== undefined) registerCatalog(options.catalog);
  const catalog = registeredCatalog();

  const handicaps: Partial<Record<PlayerId, Handicap>> = options.handicaps ?? {};
  for (const player of PLAYER_IDS) {
    const handicap = handicaps[player];
    if (handicap !== undefined) validateHandicap(handicap, player);
  }

  PLAYER_IDS.forEach((player, seat) => {
    validateDeck(options.decks[seat] ?? [], catalog, player, handicaps[player]?.deckSize ?? DECK_SIZE);
  });

  const state: GameState = {
    seed: options.seed,
    rngCursor: 0,
    turn: SETUP_TURN,
    active: "p1",
    phase: "setup",
    players: { p1: createPlayerState(), p2: createPlayerState() },
    pending: null,
    triggerQueue: [],
    declaredAttack: null,
    work: [],
    workCursor: 0,
    echoQueue: [],
    dispatch: [],
    delayed: [],
    counters: { drawn: 0, played: 0, destroyed: 0, exiled: 0 },
    transientDefs: {},
    reserved: [],
    mulliganed: [],
    result: null,
    nextId: firstId,
    nextSeq: 1,
    applied: [],
  };

  // R223: the numbers each deck's cards take are drawn in an order of the seed's own, so a card's id
  // says nothing about where it stood in the list the deck was handed over in — which the server's
  // store sorts by card id, so an id numbered in list order told the opponent how many of a deck's
  // cards sort before it, hidden ones included (§9.1, R97). The library itself is still the list in
  // order, for §2.1's shuffle, and the stream is not the match's rng, whose draws are untouched.
  const numbering = createRng(`${options.seed}${INSTANCE_ID_STREAM}${stream}`);
  PLAYER_IDS.forEach((player, seat) => {
    const deck = options.decks[seat] ?? [];
    const side = state.players[player];
    side.library = deck.map((defId) => newInstance(state, defId, player, { z: "library", player }));
    const ids = numbering.shuffle(side.library.map((card) => card.id));
    // R433: a dealt deck is not one its player built, so they know none of it yet.
    const built = options.dealt?.includes(player) !== true;
    side.library.forEach((card, at) => {
      card.id = ids[at] ?? card.id;
      // R311: a player's own deck is the first thing they know of their library.
      if (built) showToOwner(card);
    });

    // R180: a copy of the five fields and nothing else, so no stray key reaches the state or its hash.
    // R290: `heroHealth` joins them only when it moves the hero off HERO_HEALTH, so a tier that does
    // not set it stores exactly what it stored before the field existed.
    const handicap = handicaps[player];
    if (handicap !== undefined && !isHumanHandicap(handicap)) {
      const heroHealth = startingHeroHealth(handicap);
      side.handicap = {
        deckSize: handicap.deckSize,
        manaBonus: handicap.manaBonus,
        manaCap: handicap.manaCap,
        extraOpeningCards: handicap.extraOpeningCards,
        extraDrawsPerTurn: handicap.extraDrawsPerTurn,
        ...(heroHealth === HERO_HEALTH ? {} : { heroHealth }),
      };
      side.hero.health = heroHealth;
    }
  });

  // R417, R564: frozen as the match is created, minus every entry this match cannot rebuild.
  const lastBoards = freezeLastBoards(options.lastBoards, catalog);
  if (lastBoards !== undefined) state.lastBoards = lastBoards;
  // R678: the same freeze for the boards a Glitch may lay down.
  const glitchBoards = freezeLastBoards(options.glitchBoards, catalog);
  if (glitchBoards !== undefined) state.glitchBoards = glitchBoards;
  // R676: what a Glitch reset deals again.
  state.opening = {
    decks: [[...(options.decks[0] ?? [])], [...(options.decks[1] ?? [])]],
    ...(options.dealt === undefined || options.dealt.length === 0 ? {} : { dealt: [...options.dealt] }),
  };

  return state;
}

/** R223: the seed suffix of the stream `createGame` numbers the decks' cards from. */
export const INSTANCE_ID_STREAM = ":instance-ids";

/**
 * R223 mid-game: the order a batch of new cards takes its numbers in, when the batch fills a zone
 * nobody may read — #83 Transmogulate replaces a whole library, top down, and numbered in that walk
 * the new ids were one run in library order, so the first id of the next public card told its owner
 * where every library card they are later shown lies (§9.1, §10.8). The order is drawn from the
 * seed's own stream, keyed by the next id to be handed out so each batch draws its own, and the
 * match's rng is untouched.
 */
export function numberingOrder<T>(state: Pick<GameState, "seed" | "nextId">, items: readonly T[]): T[] {
  if (items.length < 2) return [...items];
  return createRng(`${state.seed}${INSTANCE_ID_STREAM}:${state.nextId}`).shuffle([...items]);
}

/**
 * A deep copy of a state. §10.1 keeps the state JSON-only, so a JSON round-trip is a faithful
 * clone and quietly enforces that invariant: anything unserializable would not survive it.
 */
export function cloneState(state: GameState): GameState {
  return JSON.parse(JSON.stringify(state)) as GameState;
}

export function activeUnits(side: PlayerState): CardInstance[] {
  return side.units.flatMap((pile) => {
    const top = pile?.[0];
    return top === undefined ? [] : [top];
  });
}

export function allZonesEmpty(side: PlayerState): boolean {
  return (
    side.units.every((pile) => pile === null) &&
    side.backrow.every((card) => card === null) &&
    (side.carried ?? []).every((card) => card === null)
  );
}

/**
 * §2.1, §2.2: whether it is `player`'s turn. Setup (`SETUP_TURN`) is no player's turn: `active`
 * names p1 there only as a placeholder until p1 takes the first turn (§2.1 step 5), so a clause a
 * card makes during setup — a cast on the opening deal or a mulligan's replacement draw (R70) — is
 * made on no turn of its controller's (R155, R241).
 */
export function isTurnOf(state: Pick<GameState, "turn" | "active">, player: PlayerId): boolean {
  return state.turn !== SETUP_TURN && state.active === player;
}

export function findInstance(state: GameState, instanceId: string): CardInstance | undefined {
  for (const player of PLAYER_IDS) {
    const side = state.players[player];
    const inPiles = side.units.flatMap((pile) => pile ?? []);
    const candidates: (CardInstance | null | undefined)[] = [
      ...side.hand,
      ...side.library,
      ...side.graveyard,
      ...side.exile,
      ...inPiles,
      ...side.backrow,
      // B5 E21: a backrow pile's dormant cards and a carrier's Unit are on the board too (R446).
      ...(side.backrowPiles ?? []).flat(),
      ...(side.carried ?? []),
      // R98: a card that asks a question mid-resolution is still itself, and §10.5 parks it here
      // between its play and its destination, so a resumed step finds `ctx.self` rather than null.
      ...side.resolving,
    ];
    const found = candidates.find((card) => card?.id === instanceId);
    if (found != null) return found;
  }
  return undefined;
}

export function rowOf(side: PlayerState, row: Row): (Pile | null)[] | (CardInstance | null)[] {
  return row === "units" ? side.units : side.backrow;
}
