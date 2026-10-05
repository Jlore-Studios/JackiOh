// The event union (SPEC §10.3). Every visible state change emits one, and BUILD M5-T4 animates each type.
// Payloads carry ids and numbers only, so an event list serializes and replays exactly.

import type { CardType, Keyword, PlayerId, PromptKind, Row, Zone } from "./catalog-types";

export type GameEvent =
  /**
   * `formerId` (R227): the id the card had until this moment, set only when the play put it
   * face-down into a backrow, which gives it a fresh id. Its controller's client finds the hand card
   * by it; a view that hides the card hides this too (R97).
   */
  | {
      type: "cardPlayed";
      player: PlayerId;
      instanceId: string;
      defId: string;
      costPaid: number;
      x?: number;
      embiggened?: boolean;
      formerId?: string;
      /** B5 E11, R454: the card was played from its player's graveyard, not the hand. Public. */
      from?: "graveyard";
      /**
       * R119: the permanents that arrived on the field during this play before §10.5 step 4
       * announced it — a tributed unit's Death at step 2 (#22's copies) — which do not answer it, as
       * `cardResolved`'s field says for step 7. Engine bookkeeping: a view never forwards it.
       */
      arrivedDuring?: string[];
      /**
       * R174, R212: the field's departures as the play was announced (the engine's exit mark), so a
       * response the loop hands this event later judges the played card's stay from the moment the
       * event happened. Engine bookkeeping: a view never forwards it.
       */
      exitsFrom?: number;
    }
  /**
   * §10.5 step 7: the card has finished resolving — after its Cry and any Echo repeats, and after a
   * Spell has reached the graveyard or exile. R17 keys the post-resolution traps on this moment
   * (Bear Honeypot, Unstable Clone Machine, Unlicensed Experimentation), as against Sheepish, which
   * fires at step 4 and costs the card its Cry. `permanent` says whether the card is still in play,
   * which is R61's distinction for Unlicensed Experimentation.
   *
   * `costPaid` repeats `cardPlayed`'s number — the mana actually charged after every modifier, R65's
   * X and embiggen prices included, and 0 for a cast (R70). It is carried rather than looked up
   * because R89 is the hazard: a trigger answering this event must find what it needs on the event,
   * since the instance may have been reset between the two moments (#60 reads "costing 1 or less").
   *
   * `radiant` is the face that resolved, for the same reason: #33 Unstable Clone Machine copies the
   * played card with its radiant flag (R34, R57), and by step 7 the card may have ceased to exist —
   * #41 Sheepish transforms a played Unit at step 4 — so a trigger cannot look it up. It is the
   * card's flag at step 7 when it still exists, and the flag it was played with otherwise. A view
   * that hides the card hides this too (R97).
   */
  | {
      type: "cardResolved";
      player: PlayerId;
      instanceId: string;
      defId: string;
      permanent: boolean;
      costPaid: number;
      radiant?: boolean;
      /**
       * R119: the permanents that arrived on the field while this play resolved, whatever put them
       * there — a Recruit by its Cry (#98), #95's backrow, a Reborn body its own Cry brought back, a
       * unit a trap answering the play summoned — which do not answer it, as the played card does not
       * answer its own play. Engine bookkeeping: a view never forwards it.
       */
      arrivedDuring?: string[];
      /**
       * R174, R212: the field's departures as step 7 emitted this (the engine's exit mark). A cast's
       * `cardResolved` waits for the loop of the effect that cast it (R70), and the rest of that
       * effect's list, and the state check after it, can take the card off the field and Reborn put
       * a new body back before the event reaches a response — which judges the card's stay from
       * here, not from the dispatch, so that body is not "it". Engine bookkeeping: a view never
       * forwards it.
       */
      exitsFrom?: number;
    }
  | {
      type: "summoned";
      player: PlayerId;
      instanceId: string;
      defId: string;
      row: Row;
      lane: number;
      /** `formerId` (R227): as on `cardPlayed`, when this summon put an existing card face-down. */
      formerId?: string;
      /** R119: on a played card's step-4 `summoned`, as on its `cardPlayed`. A view never forwards it. */
      arrivedDuring?: string[];
      /** R174, R212: on a played card's step-4 `summoned`, as on its `cardPlayed`. A view never forwards it. */
      exitsFrom?: number;
    }
  | { type: "damage"; sourceId: string | null; targetId: string; amount: number; combat: boolean }
  | { type: "healthLost"; player: PlayerId; amount: number }
  | { type: "healed"; targetId: string; amount: number }
  | { type: "divineShieldLost"; instanceId: string }
  /** R89: what the card was as it died — the layers' attack and max health, and who killed it. */
  | {
      type: "destroyed";
      instanceId: string;
      defId: string;
      owner: PlayerId;
      /**
       * The player who controlled it as it died. R172: a stolen unit dies as its controller's, who is
       * its current owner too since patch v0.2.1 (R669) — Classic #14 Shadowstep's "your Units" reads this.
       */
      controller: PlayerId;
      attack: number;
      maxHealth: number;
      killerId: string | null;
      /** R89: set when the unit died on its Radiant face (C+ #12.8 Frostspatula's memory, R409). */
      radiant?: true;
    }
  | { type: "enteredGraveyard"; instanceId: string; defId: string; owner: PlayerId }
  | { type: "exiled"; instanceId: string; defId: string; owner: PlayerId }
  | { type: "bounced"; instanceId: string; defId: string; owner: PlayerId }
  /**
   * §2.4, R4: a card drawn or added to a full hand, burned on its way in. It is the hand cap's only
   * event, so it is what "hand full" means on the board (R317). An ordinary card's `enteredGraveyard`
   * follows it; a unit-token card ceases to exist instead (R11) and has none.
   */
  | { type: "burned"; instanceId: string; defId: string; owner: PlayerId }
  /**
   * §2.4, R3, R315: a draw from an empty library that fatigues. `count` is the owner's fatigue count
   * after this draw (the Nth) and `amount` is the hit it deals before Armor (`FATIGUE_DAMAGE(count)`).
   * The `damage` instance on the hero follows it, R240's zero-damage report when Armor takes the
   * whole hit. A draw #75 Infinite Reserves replaces emits none. Public: it names no card.
   */
  | { type: "fatigue"; player: PlayerId; count: number; amount: number }
  /**
   * §2.4, R80, R316: a card refused by a full library. `outcome` says what became of it: a card the
   * effect was creating is `notCreated` (it never existed, #33's and #90's copies), an existing card
   * goes to its owner's `graveyard` (its `enteredGraveyard` follows), and an existing unit-token card
   * has `ceased` to exist (R11). `player` owns the library. `instanceId` and `defId` follow R97.
   */
  | {
      type: "libraryOverflow";
      player: PlayerId;
      instanceId: string;
      defId: string;
      outcome: LibraryOverflowOutcome;
      /**
       * R316: set when the refused card is Radiant (#33's Radiant face makes every copy Radiant, a
       * Radiant CN-Virus copies Radiant), so the board shows the face it would have had. It is the
       * card's, so a view that hides the card hides this too (R97).
       */
      radiant?: true;
      /**
       * R316: the card a `notCreated` copy was a copy of (#33 copies whatever its controller plays,
       * a Trap set face-down included; #90.1 copies itself), so a view judges the copy that was never
       * made by that card, and a face-down trap's copy does not name it (R97). Engine bookkeeping: a
       * view never forwards it.
       */
      copyOf?: string;
    }
  | { type: "discarded"; instanceId: string; defId: string; owner: PlayerId }
  /**
   * `turnDraw` (B5 E4, R457): this draw's number among `player`'s draws this turn, whoever's turn it
   * is (1 for the first; a fatigue draw counts, a limited one does not). Public: the hand count and
   * the fatigue count already say as much. Absent during setup, which is no player's turn (§2.1), so
   * the opening deal says nothing of which draw a Quickdraw card replaced (R225).
   */
  | {
      type: "drawn";
      player: PlayerId;
      instanceId: string;
      defId: string;
      turnDraw?: number;
      /**
       * B5 E33: this draw took the last card of the drawer's own library — Classic #90's quest 9, "a
       * draw of yours takes the last card of your deck". Present, and `true`, only then. Public: the
       * library count already says as much.
       */
      emptied?: true;
    }
  | { type: "addedToHand"; player: PlayerId; instanceId: string; defId: string }
  | { type: "shuffledIn"; player: PlayerId; instanceId: string; defId: string; position: number }
  | { type: "buffed"; instanceId: string; attack: number; health: number }
  | {
      type: "keywordGranted";
      instanceId: string;
      keyword: Keyword;
      /**
       * R46: set when the unit loses the keyword instead — the Taunt an Indestructible unit's
       * knock-down takes for the rest of the turn, which a unit already in Attack Position would
       * otherwise lose with no event (§10.3, R91). Absent on a grant.
       */
      lost?: true;
    }
  /**
   * `brittle` is B3.3's count (R385). `placed` (B5 E19) is how many Plague Tokens one placement put
   * on the card — set on a placement only, so "whenever Plague Tokens are placed on this" answers the
   * placement once however many tokens it placed, and never a removal.
   */
  | {
      type: "counterChanged";
      instanceId: string;
      counter: "plague" | "grade" | "brittle";
      value: number;
      placed?: number;
    }
  /**
   * R177: `hiddenFrom` is set on a change made to a card in a library — both players, who could not
   * read it there (§3) — so a view keeps the event hidden from them for good, even once the card
   * reads openly. The view uses it and never forwards it.
   */
  | { type: "costChanged"; instanceId: string; cost: number; hiddenFrom?: PlayerId[] }
  | { type: "modifierChanged"; player: PlayerId; modifierId: string; added: boolean }
  | { type: "radiantSet"; instanceId: string; defId: string; zone: Zone }
  /**
   * R177: `hiddenFrom` names the players who could not read the old card where it ceased to exist —
   * both of them for a library card, the other player for a face-down trap (R33) — so a view keeps
   * it hidden from them for good, even after its replacement reaches a public pile. Absent when the
   * old card was public. The view uses it and never forwards it.
   */
  | {
      type: "transformed";
      instanceId: string;
      fromDefId: string;
      toDefId: string;
      newInstanceId: string;
      hiddenFrom?: PlayerId[];
    }
  | { type: "fused"; instanceIds: string[]; resultInstanceId: string; defId: string }
  | { type: "positionSwitched"; instanceId: string; position: "ATK" | "DEF" }
  /**
   * `formerId` (R227): set when the move put the card face-down with a fresh id (C+ #35's restore,
   * R419), as on `summoned`; a view that hides the card hides this too (R97).
   */
  | { type: "controlChanged"; instanceId: string; controller: PlayerId; row: Row; lane: number; formerId?: string }
  | { type: "rotated"; direction: "left" | "right" }
  | { type: "swapped"; what: "health" | "board" | "library" }
  | { type: "locked"; player: PlayerId; row: Row; lane: number }
  /**
   * R154: `row` and `lane` say which zone flipped, so a client can point at it without being told
   * which card it was. `instanceId` and `defId` follow §10.8's redaction (R97) — the controller
   * reads them, the other player reads the sentinel — and a face-down trap is given no instance id
   * in the view at all, so without the lane the opponent's side has nothing to animate on.
   */
  | {
      type: "trapFired";
      instanceId: string;
      defId: string;
      controller: PlayerId;
      row: Row;
      lane: number;
    }
  | { type: "attackDeclared"; attackerId: string; targetId: string; forced: boolean }
  | { type: "attackCancelled"; attackerId: string; targetId: string; byInstanceId: string }
  | { type: "manaChanged"; player: PlayerId; current: number; max: number }
  | { type: "turnStarted"; player: PlayerId; turn: number }
  | { type: "turnEnded"; player: PlayerId; turn: number; unspentMana: number }
  | { type: "turnAutoEnded"; player: PlayerId; turn: number }
  | { type: "promptOpened"; player: PlayerId; choiceId: string; kind: PromptKind }
  | { type: "promptAnswered"; player: PlayerId; choiceId: string }
  | { type: "drawOffered"; player: PlayerId }
  | { type: "drawAnswered"; player: PlayerId; accept: boolean }
  | { type: "gameOver"; winner: PlayerId | "draw"; reason: GameOverReason }
  // -------------------------------------------------------------------------------------------
  // Patch v0.2.0 (docs/classic-sets.md B3, B5). Each has a BUILD M5-T4 row in the client's
  // `ANIMATIONS` and a `SOUND_CUES` row, and follows R97 in `viewFor` like every event above.
  // -------------------------------------------------------------------------------------------
  /**
   * B5 E1: a play or cast has been paid for and is about to move (§10.5 between steps 3 and 4). The
   * window it opens is where a Counter answers. `cardType` is the type it is played as (a face's own
   * type, B2.7); `targets` names what the play declared, a hero as `hero-<player>`. A card being set
   * face-down shows the other player only the zone it is going to (`row`, `lane`), as `cardPlayed`
   * would (R97, R227).
   */
  | {
      type: "cardAnnounced";
      player: PlayerId;
      instanceId: string;
      defId: string;
      cardType: CardType;
      costPaid: number;
      targets: string[];
      row?: Row;
      lane?: number;
      faceDown?: true;
    }
  /** B5 E1: an announced play was cancelled. `to` is where the card went (a steal sends it to a hand, E2). */
  | {
      type: "countered";
      player: PlayerId;
      instanceId: string;
      defId: string;
      byInstanceId: string | null;
      to: "graveyard" | "exile" | "hand" | "gone";
    }
  /**
   * B5 E2, E16, R466: a card changed owner as it moved to the thief's hand. `zone` is where it was
   * taken from. A viewer reads the card if they could read it where it was taken from — the hand's
   * holder, everyone for a public pile or a face-up zone, the controller of a face-down zone, nobody
   * for a library — or can read it where it is now (R97). `readableFrom` names the first set, written
   * as the card is taken; the view uses it and never forwards it.
   */
  | {
      type: "stolen";
      instanceId: string;
      defId: string;
      from: PlayerId;
      to: PlayerId;
      zone: "hand" | "library" | "resolving" | "graveyard" | "exile" | "field";
      readableFrom?: PlayerId[];
    }
  /** B5 E20: a Locked zone opened again. */
  | { type: "unlocked"; player: PlayerId; row: Row; lane: number }
  /** B3.2, R384: a card's Activate ability was used. `ability` names it (`"activate"` when it has one). */
  | { type: "activated"; player: PlayerId; instanceId: string; defId: string; ability: string }
  /**
   * B3.1, R383: a backrow card stepped into a unit zone as a Unit. `carried` (R446): it was a Unit a
   * carrier held (Classic+ #33 Ivory Tower), stepping down because its zone no longer carries it — the
   * same move, from a backrow zone to a unit zone without leaving the field.
   */
  | {
      type: "animated";
      player: PlayerId;
      instanceId: string;
      defId: string;
      backrowLane: number;
      unitLane: number;
      carried?: true;
    }
  /** B3.1, R383: an "Animated on your turn" card went back to its backrow zone. */
  | { type: "deanimated"; player: PlayerId; instanceId: string; defId: string; unitLane: number; backrowLane: number }
  /** B3.3, R385, R638: a Brittle count on the field reached 0 and the card was destroyed. */
  | { type: "crumbled"; instanceId: string; defId: string; owner: PlayerId; zone: "field" }
  /**
   * B3.4, R386: one Degrade or Upgrade change. `hiddenFrom` (R177) names the players who could not
   * read the card where it changed — both for a library card, the other player for a hand card — so a
   * view keeps it hidden from them for good. The view uses it and never forwards it.
   */
  | { type: "degraded"; instanceId: string; defId: string; change: TuningChange; hiddenFrom?: PlayerId[] }
  | { type: "upgraded"; instanceId: string; defId: string; change: TuningChange; hiddenFrom?: PlayerId[] }
  /**
   * B3.4, Classic+ #41 KY's Constant: one of a card's numbers set outright (`key` as `numbersOn` names
   * it: "cost", "attack", "health", a numbered keyword, a declared number's key). `hiddenFrom` as on
   * `degraded`: the view uses it and never forwards it.
   */
  | { type: "numberChanged"; instanceId: string; defId: string; key: string; value: number; hiddenFrom?: PlayerId[] }
  /** B5 E9: a damage instance, an attack or a chosen target moved to a new one. Ids as `damage` writes them. */
  | {
      type: "redirected";
      what: "damage" | "attack" | "target";
      fromId: string;
      toId: string;
      byInstanceId: string | null;
    }
  /** B5 E7: a hero's health was set — not damage, not a heal (R18's lose health is the nearest rule). */
  | { type: "healthSet"; player: PlayerId; health: number; sourceId: string | null }
  /** Classic #90 (E33): a quest's count moved, or a quest was completed. */
  | { type: "questProgressed"; player: PlayerId; instanceId: string; quest: string; progress: number; goal: number }
  | { type: "questCompleted"; player: PlayerId; instanceId: string; quest: string }
  /** Classic+ #35 (E29): the board went back `turnsAgo` turns on the named sides. */
  | { type: "rolledBack"; player: PlayerId; turnsAgo: number; sides: PlayerId[] }
  /** R436: Call to Chaos names the effects it rolled, to both players, in the order they resolve. */
  | { type: "chaosRolled"; player: PlayerId; instanceId: string; defId: string; effects: string[] }
  /** B5 E22: a card left the field and came back into the same zone at once (R78's reset, no Cry, no Death). */
  | { type: "flickered"; player: PlayerId; instanceId: string; defId: string; row: Row; lane: number }
  /** B5 E3: a draw that a draw limit stopped — no card moved, no fatigue. Public: it names no card. */
  | { type: "drawLimited"; player: PlayerId }
  /** B5 E10: an effect ended `player`'s turn (the turn's own `turnEnded` follows). */
  | { type: "turnCutShort"; player: PlayerId; byInstanceId: string | null }
  /**
   * R437: a card gained or lost a mark — a pending effect aimed at it, shown on it in both views
   * (#50 K-Pop Fanatic's steal is `"steal"`, purple). `color` is a key the client maps to a colour.
   */
  | { type: "marked"; instanceId: string; mark: string; color: string; added: boolean };

/**
 * B3.4, R386: what one Degrade or Upgrade application changed. `cost` is a `costMod` step; `stats`
 * the attack and health it moved (negative for a Degrade); `keyword` one keyword added or removed;
 * `x` a numbered keyword's or an X's step (`key` names it: "Armor", "Echo", "Activate", "X", …);
 * `number` a declared number's step (`key` is the catalog `params` key). `delta` is how far the value
 * moved. `none` is R440's cue on a card someone may not read that nothing could change (Immutable, or
 * no change applies), so the events over a hidden pile number the applications, never the changes.
 */
export type TuningChange =
  | { kind: "cost"; delta: number }
  | { kind: "stats"; attack: number; health: number }
  | { kind: "keyword"; keyword: Keyword; added: boolean }
  | { kind: "x"; key: string; delta: number }
  | { kind: "number"; key: string; delta: number }
  | { kind: "none" };

export type GameEventType = GameEvent["type"];

/** R316: what became of a card R80's full library refused (`libraryOverflow.outcome`). */
export type LibraryOverflowOutcome = "notCreated" | "graveyard" | "ceased";

export type GameOverReason =
  | "hero-death"
  | "both-heroes-dead"
  | "concede"
  | "draw-accepted"
  | "turn-cap"
  | "disconnect"
  | "match-ceiling";

/**
 * Every type in the union, for the animation-table test (BUILD M5-T4).
 *
 * `satisfies readonly GameEventType[]` below is only a SUBSET check — it rejects a member that is
 * not a `GameEventType` and says nothing about one that is missing. `GameEventTypesAreExhaustive`
 * underneath the array closes that direction, and `test/events.test.ts` closes it again at runtime
 * by reading the union out of this file's own source, so a new event cannot slip past `vitest run`.
 */
export const GAME_EVENT_TYPES = [
  "cardPlayed",
  "cardResolved",
  "summoned",
  "damage",
  "healthLost",
  "healed",
  "divineShieldLost",
  "destroyed",
  "enteredGraveyard",
  "exiled",
  "bounced",
  "burned",
  "fatigue",
  "libraryOverflow",
  "discarded",
  "drawn",
  "addedToHand",
  "shuffledIn",
  "buffed",
  "keywordGranted",
  "counterChanged",
  "costChanged",
  "modifierChanged",
  "radiantSet",
  "transformed",
  "fused",
  "positionSwitched",
  "controlChanged",
  "rotated",
  "swapped",
  "locked",
  "trapFired",
  "attackDeclared",
  "attackCancelled",
  "manaChanged",
  "turnStarted",
  "turnEnded",
  "turnAutoEnded",
  "promptOpened",
  "promptAnswered",
  "drawOffered",
  "drawAnswered",
  "gameOver",
  "cardAnnounced",
  "countered",
  "stolen",
  "unlocked",
  "activated",
  "animated",
  "deanimated",
  "crumbled",
  "degraded",
  "upgraded",
  "numberChanged",
  "redirected",
  "healthSet",
  "questProgressed",
  "questCompleted",
  "rolledBack",
  "chaosRolled",
  "flickered",
  "drawLimited",
  "turnCutShort",
  "marked",
] as const satisfies readonly GameEventType[];

/**
 * The other half of the check: a `GameEvent` member missing from `GAME_EVENT_TYPES` is a COMPILE
 * ERROR, and the error names it — `Type '"newThing"' does not satisfy the constraint 'never'`.
 *
 * `Exclude` leaves exactly the members the array forgot; `never` is the only type that satisfies
 * the constraint, so an empty difference compiles and a non-empty one does not. Nothing is emitted:
 * this is a type alias, so the assertion costs no runtime bytes.
 */
type NoneMissing<T extends never> = T;

export type GameEventTypesAreExhaustive = NoneMissing<
  Exclude<GameEventType, (typeof GAME_EVENT_TYPES)[number]>
>;
