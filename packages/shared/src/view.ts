// What a player is allowed to see (SPEC §10.8). The client renders this and nothing else.

import type { CardDef, CardType, Keyword, KeywordKind, PlayerId, PromptKind, Row } from "./catalog-types";
import type { GameEvent, GameOverReason } from "./events";

export type CardView = {
  instanceId: string;
  defId: string;
  radiant: boolean;
  /** Cost as it stands now (§6.3 Cost, R65); "X" cards show 0 until X is chosen. */
  cost: number;
  /**
   * R243: a Unit card's stats in its owner's hand, as they stand: its printed face (the radiant one
   * when it is Radiant, a fused card's summed one) plus the permanent buffs it has gained there
   * (§10.4 layers 1, 3 and 4 — #89 Corpse Eater feeds in hand). Set on the viewer's own hand cards
   * only; a unit on the field reads its layers off `UnitView`.
   */
  attack?: number;
  health?: number;
  /**
   * R243, R43, R151: the power a #98 Heroic Power in its owner's hand rolled as it arrived, by name.
   * Its X is the card's cost, which does not name it: four of the eight powers cost the same.
   */
  power?: string;
  /**
   * R195, §10.8: Hearthstone's yellow glow. Present, and `true`, only on the viewer's own card
   * whose printed condition holds now. Absent otherwise: never `false`, never on the opponent's
   * cards. `UnitView` and the public `BackrowView` inherit it.
   */
  conditionActive?: true;
  /**
   * R280, §10.8: what the card's formula comes to now, one entry per labelled number its script's
   * `preview` hook returns — the label the formula as the running face prints it, the value what it
   * would come to if the card resolved now. Only on a card view the viewer may read: its own hand, a
   * unit on top of its pile, a face-up backrow card, a face-down one for its controller. Absent when
   * the hook returns nothing or the card has none.
   */
  preview?: PreviewValue[];
  // ---- Patch v0.2.0 (docs/classic-sets.md B2.7, B3, B5) ----
  /**
   * B2.7: the type the card has now, set only where it differs from its definition's — a face with
   * its own type (Classic+ #22 Blood Moon's Radiant face is a Field Trap).
   */
  type?: CardType;
  /** B3.3, R385: the card's Brittle count, where the viewer may read the card and it has one. */
  brittle?: number;
  /**
   * B3.4, R386: the card's declared numbers as they stand now (its face's `params`, moved by
   * Degrade, Upgrade and KY's Constant), by key, which the client fills into the face's `{key}`s.
   */
  params?: Record<string, number>;
  /** B3.4, R386: what Degrade and Upgrade have changed on the card, where the viewer may read it. */
  tuning?: Tuning;
  /** B5 E39: the enchantments riding the card (Classic+ #14's return, #40's cast on draw). */
  enchantments?: Enchantment[];
  /**
   * B5 E38, R243: a card in the viewer's own hand, its keywords as it will carry them onto the field
   * — printed as Degrade and Upgrade left them, and those it was granted in the hand or the deck —
   * set only where they differ from its face's printed keywords. A unit on the field reads its
   * keywords off `UnitView`.
   */
  keywords?: Keyword[];
  /** R437: the marks on the card — an effect aimed at it and waiting (K-Pop Fanatic's steal). Both views. */
  marks?: CardMark[];
  /** B3.2, R384: the card's Activate abilities, on its controller's own view of it on the field. */
  activations?: ActivationView[];
  /**
   * B5 E33, R404: Classic #90 In Too Deep's open quests with their progress and the rewards on offer,
   * and the auras its quest line holds — on every view of the card on the field (a face-up Field Spell).
   */
  quest?: QuestView;
  /**
   * B5 E14, R399, R243: the Spell whose text a copier (Classic #57 Echo) has now, on the face it was
   * played on, with that definition's declared numbers as they read on this card (`params`, R386) —
   * on its owner's view of it in hand, and on both views while it resolves (its play is public). The
   * card keeps its own name, cost and type; absent when it copies nothing.
   */
  copies?: CopiedTextView;
};

/**
 * B5 E33, R404, §10.8: a quest line as the board shows it. Each open quest carries its text, its
 * progress against its goal ("1/2") and the rewards it offers; `auras` are the rewards held while the
 * card stays on the field (In Too Deep's L and M).
 */
export type QuestView = {
  open: { id: string; text: string; progress: number; goal: number; rewards: { id: string; text: string }[] }[];
  auras: { id: string; text: string }[];
};

/** B5 E14, R399: what a copier's text is now (`CardView.copies`). */
export type CopiedTextView = { defId: string; radiant: boolean; params?: Record<string, number> };

/**
 * B3.4, R386: the lasting changes Degrade, Upgrade and KY's Constant made to one card, kept in every
 * zone and through leaving the field (R78 does not reset it). The cost change is the card's
 * `costMod`, not a field here. `x` holds a step per numbered keyword or X ("Armor", "Echo",
 * "Activate", "Brittle", "Spell Damage", "Tribute", "Lucky", "X"), `numbers` a step per declared
 * number (`CardDef.params` key), `set` a declared number or numbered keyword set outright (KY's
 * Constant's "to 3"), which wins over the steps.
 */
export type Tuning = {
  attack?: number;
  health?: number;
  addKeywords?: Keyword[];
  removeKeywords?: KeywordKind[];
  x?: Record<string, number>;
  numbers?: Record<string, number>;
  set?: Record<string, number>;
};

/**
 * B5 E39: a lasting instruction that rides a card through every zone. `returnAfterResolve` is Classic+
 * #14 Forever&'s "After this resolves, return it to your hand. This can't cost less than (floor)";
 * `castOnDraw` and `targetEnemies` are Classic+ #40 Appropriations' "They have Cast on draw and aim
 * at enemies when they harm and at your side when they help".
 */
export type Enchantment =
  | { kind: "returnAfterResolve"; floor: number }
  | { kind: "castOnDraw" }
  | { kind: "targetEnemies" };

/** R437: a mark on a card, and the colour key the client draws it with ("purple"). */
export type CardMark = { mark: string; color: string };

/**
 * B3.2, R384: one Activate ability as its controller's client needs it. `usesLeft` is how many more
 * times it may be used this turn, null for Activate ♾️ (bounded only by `ACTIVATE_UNLIMITED_CAP`).
 * `usable` is `legalActions`' answer now; `reason` says why not when it is false.
 */
export type ActivationView = {
  ability: string;
  label: string;
  usesLeft: number | null;
  usable: boolean;
  reason?: string;
};

/**
 * R280: one number a card's formula comes to now, and the formula it is ("+1 per card in your exile").
 * `display` is how the value prints when the text names it by a word rather than a numeral: #93
 * Combo-Index's grade 3 prints as its letter, "C" (R372). A client prints `display` when present and
 * the number otherwise, and never works one out from the other.
 *
 * `ids` is the set of cards the value counts, by instance id, when the formula is a set of cards
 * rather than a number alone: Classic+ #44 Simplicity Audit's and #45 Complexity Audit's Radiant
 * "highlight targets", the permanents the card would exile now (docs/classic-sets.md C+ #44). A
 * client marks those cards on the board; the hook never names a card its controller may not read.
 */
export type PreviewValue = { label: string; value: number; display?: string; ids?: string[] };

export type UnitView = CardView & {
  owner: PlayerId;
  controller: PlayerId;
  attack: number;
  maxHealth: number;
  health: number;
  keywords: Keyword[];
  armor: number;
  position: "ATK" | "DEF";
  counters: { plague?: number; grade?: number };
  /** Cards under this one in a Stack pile are face-down and dormant (§3.2). */
  buried: number;
  canAct: boolean;
  /**
   * R243, §6.3 Vanilla, R115: the unit's text is gone — its printed keywords and every script, the
   * ones its definition still names included — so a client shows none of it. Absent otherwise.
   */
  vanilla?: true;
  /**
   * B3.1, R383: a Field Spell, Trap or Field Trap standing in a unit zone as a Unit. `home` is the
   * backrow lane an "Animated on your turn" card returns to at its controller's cleanup, when it has
   * one (that zone is reserved for it meanwhile, `SideView.reserved`).
   */
  animated?: { home?: number };
  /** B5 E35: the unit has gone Berserk (a status, lost when it leaves the field). Absent otherwise. */
  berserk?: true;
};

/**
 * A backrow zone: a public card, a face-down trap, or empty (§3, §10.8). A public card names its
 * `owner` and `controller` like a `UnitView` does, because R33 keys readability on the controller:
 * after a steal (#36 radiant, #49), a board swap (#87) or a rotation (#52) the card sits in a
 * backrow that is not its controller's, and the view says so rather than leaving the client to
 * track `controlChanged` out of band. A face-down zone stays a marker: §10.8 grants the
 * non-controller the fact that something is there and what it costs (R351), and nothing else.
 */
export type BackrowView =
  | (CardView & {
      faceDown: false;
      type: CardType;
      /**
       * `grade` is #93 Combo-Index's counter, 1..6; `gradeLetter` is the letter that number is,
       * E..S, which the engine names so a client prints it rather than working it out (R372).
       * `plague` is the card's Plague Tokens (§6.3, B5 E19), absent at none.
       */
      counters: { grade?: number; gradeLetter?: string; plague?: number };
      owner: PlayerId;
      controller: PlayerId;
      /**
       * R351, R371: present, and `true`, on the controller's own view of a Trap or Field Trap that
       * is still face-down: the controller reads the card (R33), and the other player sees only its
       * back. Absent on every public card and on a Field Trap that has fired.
       */
      unrevealed?: true;
      /**
       * R243, §6.3 Vanilla: the backrow card's text is gone — a client stamps it as it stamps a
       * vanilla unit. Absent otherwise.
       */
      vanilla?: true;
      /** B5 E21: face-down, dormant cards beneath this one in a backrow pile (§3.2). Absent for none. */
      buried?: number;
    })
  | {
      faceDown: true;
      /**
       * R351, R370: a face-down Trap shows its cost to both players, the number its controller's own
       * view shows (§6.3 Cost, R65). Always set by `viewFor`; optional so a client draws a back with
       * or without it (a view built before the patch, a test fixture).
       */
      cost?: number;
      /**
       * B5 E19, R471: the Plague Tokens on the face-down card. Tokens are public wherever they sit, so
       * both players see the count on the card's back; the card stays hidden. Absent at none.
       */
      plague?: number;
      /** B5 E21: how many dormant cards lie beneath it in a backrow pile — a count, never an identity. */
      buried?: number;
      /**
       * R437: the marks the face-down card carries — an effect aimed at it that waits (#50's
       * pending steal) — which the player who may not read it sees on its back (R33).
       */
      marks?: CardMark[];
    }
  | null;

/** A Heroic Power on the field (§8 #98, R43), as the client needs it to act. */
export type HeroPowerView = {
  /** #98's instance, so `activatePower {instanceId}` is built from the view alone (§10.2). */
  instanceId: string;
  defId: string;
  name: string;
  /** "Once per turn, spend X" (§8 #98): the X, which is also the card's cost (R43, R65). */
  x: number;
  usedThisTurn: boolean;
};

export type HeroView = {
  health: number;
  /**
   * The Armor §4.4 step 2 subtracts from each hit on this hero, as a computed total and not a
   * stored field: whatever is written on the hero plus every backrow card granting it (#84 Going
   * Long), which R124 adds up. So it drops back when a granting card leaves the backrow, exactly
   * like a unit's `armor` above.
   */
  armor: number;
  /**
   * Every Heroic Power this player controls, in board order. Usually none or one, but #36 radiant
   * and #49 steal a backrow permanent and a Field Spell is one (§3), so a player holding their own
   * #98 can come to control the opponent's as well — and each is separately once-per-turn (R43).
   */
  powers: HeroPowerView[];
  /** `powers[0] ?? null`: the one a single-button hero panel shows. */
  power: HeroPowerView | null;
};

/**
 * One player-level modifier (§10.1 `PlayerState.mods`) as the hero panel shows it.
 *
 * `id` is the `PlayerModifier.id` that §10.3's `modifierChanged` event already names on both
 * seats, so the badge an animation plays on is the badge the view carries. `label` is a short
 * caption built from the modifier's own kind and numbers — and its timing while R48 keeps it
 * dormant — and it is the *whole* of what a modifier reveals: never `sourceId`, never the card
 * that installed it, so nothing that could name a face-down card rides out on a badge.
 */
export type ModifierView = {
  id: string;
  label: string;
};

/**
 * R310: one kind of card left in the viewer's own library: a definition, the face it went in with
 * (R311) and how many such cards are there. No instance id and no position, so nothing in it can
 * say where a card lies.
 */
export type LibraryEntryView = { defId: string; radiant: boolean; count: number };

/**
 * R310–R312: the viewer's own library as a list without order. `cards` holds what the viewer was
 * shown of each card as it went in, one entry per definition and face, sorted by printed cost, then
 * name, then id, base face first (R310): an order that depends on the cards alone, never on where
 * they lie. `unknown` counts the cards the viewer was never shown (R312: a library Pocket Chaos
 * swapped in, Transmogulate's picks), which a client draws as backs. The two add up to
 * `libraryCount`.
 */
export type LibraryView = { cards: LibraryEntryView[]; unknown: number };

export type SideView = {
  player: PlayerId;
  hero: HeroView;
  /**
   * The player modifiers on this seat, in the order they were installed. Public on BOTH seats:
   * every modifier in the Core set is installed by the Cry of a card played face-up (§10.5 step 4,
   * #35, #77, #78, #79), and `modifierChanged` is already an unredacted event for both players, so
   * the label states only what the public play already said. Nothing derived from a hidden card
   * travels with it (see `ModifierView`).
   */
  modifiers: ModifierView[];
  mana: { current: number; max: number };
  /** Full cards for the viewer; a count only for the opponent (§10.8). */
  hand: CardView[] | { count: number };
  libraryCount: number;
  /**
   * R310: the viewer's own library, as a list without order. Present on the viewer's own side only;
   * the opponent's library is `libraryCount` and nothing else (§9.1, §10.8).
   */
  ownLibrary?: LibraryView;
  graveyard: CardView[];
  exile: CardView[];
  /**
   * Cards mid-resolution: a Spell between its play and its graveyard (§10.5 step 4). Public for
   * both sides — playing a card is public — and R98 makes one still itself while it sits here, so
   * a Spell that opened a prompt can be shown on the board instead of vanishing until it lands.
   */
  resolving: CardView[];
  units: (UnitView | null)[];
  backrow: BackrowView[];
  /**
   * B5 E21, R446: the Unit standing on each backrow zone's carrier (Classic+ #33 Ivory Tower), by lane —
   * a Unit on the field, public like any, that can neither attack nor be attacked. Absent when no
   * carrier on this side holds one.
   */
  carried?: (UnitView | null)[];
  locks: { units: boolean[]; backrow: boolean[] };
  /**
   * R64: a zone held for a dying Reborn unit until it comes back. It takes no summon, exactly as a
   * Locked zone takes none, so a client that reads only `locks` would draw it open. B3.1 rule 6: the
   * backrow zone an animated "Animated on your turn" card will return to is held the same way.
   */
  reserved: { units: boolean[]; backrow: boolean[] };
  fatigueCount: number;
};

export type PendingView =
  | {
      forYou: true;
      choiceId: string;
      kind: PromptKind;
      options: PendingOption[];
      min: number;
      max: number;
      prompt: string;
      /**
       * B5 E18: a `pick` prompt's budget — the most the picked options' `cost`s may add up to
       * (Classic #44's "total cost of (5) or less"). Absent on every other prompt.
       */
      budget?: number;
    }
  | { forYou: false; pendingFor: PlayerId };

export type PendingOption = {
  /** The selection to send back in an `answer` action. */
  key: string;
  label: string;
  instanceId?: string;
  defId?: string;
  player?: PlayerId;
  row?: Row;
  lane?: number;
  /** B5 E18: what this option counts against a `pick` prompt's `budget`. */
  cost?: number;
  /** A face the option shows, when the card it names is Radiant (a Discover of Radiant cards). */
  radiant?: true;
};

/** R265, R266: the concurrent mulligan as one seat may see it. */
export type MulliganView = {
  youReady: boolean;
  opponentReady: boolean;
  /** The ids the viewer kept, once it has answered (R266). */
  kept?: string[];
};

export type PlayerView = {
  viewer: PlayerId;
  turn: number;
  active: PlayerId;
  phase: "setup" | "mulligan" | "start" | "main" | "end" | "over";
  you: SideView;
  opponent: SideView;
  pending: PendingView | null;
  /**
   * The last N events, for animation (§10.10). Redacted, not truncated: an event that names a card
   * this viewer may not read keeps its type and its animation fields and carries the sentinel
   * `"hidden"` in place of that card's `instanceId` and `defId` (R97).
   */
  events: GameEvent[];
  result: { winner: PlayerId | "draw"; reason: GameOverReason } | null;
  /** Milliseconds left on the turn clock, when the server is running one (R79). */
  clockMs: number | null;
  /**
   * §2.1 step 3, R265, R266: while both mulligans are open, whether each seat has answered, and the
   * ids the viewer itself kept once it has. Never the opponent's choice or cards (§9.1): that the
   * opponent is ready is all it shows. Absent outside that window.
   */
  mulligan?: MulliganView;
  /**
   * §2.5, R36, R269: the draw offer standing right now — made by the active player this turn and
   * not yet answered — on both seats, since the offer was public (`drawOffered`). Absent when none;
   * it disappears when the offer is answered or lapses at the end of the offerer's turn.
   */
  drawOffer?: { by: PlayerId };
  /**
   * R345: `false` when the viewer has turned R82's automatic turn end off for themselves. Absent
   * means on, the rule's default. Only ever the viewer's own preference, never the opponent's.
   */
  autoEndTurn?: false;
  /**
   * R243: the definitions of the match-made cards this view names — a Fuse's (R77), a crafted
   * card's (R102, R179) — by id. They exist only in the match, so no catalog a client holds has
   * them, and a card the view shows could not otherwise be read. Only a card the viewer may read
   * brings its definition: a hidden one's id is already the sentinel (R97). Absent when none.
   */
  defs?: Record<string, CardDef>;
};
