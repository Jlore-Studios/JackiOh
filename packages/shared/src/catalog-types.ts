// Card definitions and the vocabulary every package shares (SPEC §5, §6.1, §10.6).
// Script and Effect types live in engine/src/script.ts: they need GameState, which lives in the engine.

export type PlayerId = "p1" | "p2";
export const PLAYER_IDS = ["p1", "p2"] as const;

export function opponentOf(player: PlayerId): PlayerId {
  return player === "p1" ? "p2" : "p1";
}

/** §5.1 */
export type CardType = "Unit" | "Spell" | "Field Spell" | "Trap" | "Field Trap";

/** §5: tribes and tags. "Jlockeed" is #13 and #14's (R278). */
export type Tag =
  | "Human"
  | "Felinor"
  | "KY"
  | "CN"
  | "Fruit"
  | "Call to Chaos"
  | "Quickdraw"
  | "Jlockeed"
  | "Token";

/** §8: assigned by mechanical complexity; every token carries "Token". */
export type Rarity = "Common" | "Rare" | "Epic" | "Legendary" | "Mythic" | "Token";

/** §5: Core, Classic and Classic+ ship (R380); Boss and Boss-X are reserved. */
export type SetName = "Core" | "Classic" | "Classic+" | "Boss" | "Boss-X";

/** §5: 0 to 6, 100 (Ceaseless Void), X, or "A embiggen B". */
export type CardCost = number | "X" | { base: number; embiggen: number };

/**
 * A unit keyword (§6.1). Armor and Lucky carry a number; Armor sums across sources (§10.4).
 */
export type Keyword =
  | { kind: "Taunt" }
  | { kind: "Rush" }
  | { kind: "Charge" }
  | { kind: "First Strike" }
  | { kind: "Poisonous" }
  | { kind: "Lifesteal" }
  | { kind: "Reborn" }
  | { kind: "Divine Shield" }
  | { kind: "Trample" }
  | { kind: "Cleave" }
  /** R346: its damage ignores Armor (§4.4 step 2), on a unit or on a spell. */
  | { kind: "Pierce" }
  | { kind: "Indestructible" }
  | { kind: "Immutable" }
  | { kind: "Stack" }
  | { kind: "Can't attack" }
  | { kind: "Armor"; n: number }
  | { kind: "Lucky"; n: number }
  /** R383: a Field Spell, Trap or Field Trap that steps into a unit zone as a Unit (B3.1). */
  | { kind: "Animated" }
  /** R383: animated at its controller's start of turn, back in its backrow zone at their cleanup. */
  | { kind: "Animated on your turn" }
  /** R385: printed Brittle N — the count starts when the card enters the field (B3.3). */
  | { kind: "Brittle"; n: number }
  /** §4.4: a Spell its controller casts deals N more damage per hit (E6). Printed "Spell Damage +N". */
  | { kind: "Spell Damage"; n: number }
  /** E35: a Spell can't target this and doesn't affect it. */
  | { kind: "Immune to Spells" };

export type KeywordKind = Keyword["kind"];

/** Every keyword kind. R21's random pool is the narrower list in engine config. */
export const KEYWORD_KINDS = [
  "Taunt",
  "Rush",
  "Charge",
  "First Strike",
  "Poisonous",
  "Lifesteal",
  "Reborn",
  "Divine Shield",
  "Trample",
  "Cleave",
  "Pierce",
  "Indestructible",
  "Immutable",
  "Stack",
  "Can't attack",
  "Armor",
  "Lucky",
  "Animated",
  "Animated on your turn",
  "Brittle",
  "Spell Damage",
  "Immune to Spells",
] as const;

export function keywordKey(keyword: Keyword): string {
  return "n" in keyword ? `${keyword.kind} ${keyword.n}` : keyword.kind;
}

export function hasKeyword(keywords: readonly Keyword[], kind: KeywordKind): boolean {
  return keywords.some((k) => k.kind === kind);
}

/** Total Armor across every source (§10.4). */
export function armorOf(keywords: readonly Keyword[]): number {
  return keywords.reduce((sum, k) => (k.kind === "Armor" ? sum + k.n : sum), 0);
}

/** One side of a card: the base form or the radiant form (§5). Spells have no stats. */
export type CardFace = {
  attack?: number;
  health?: number;
  keywords: Keyword[];
  /**
   * The face's printed text: the base face's §8 cell, or the Radiant face's cell read by §8's
   * Conventions and written out in full (R277), so a client can print it whole and mark what differs.
   */
  text: string;
};

export type CardDef = {
  /** Catalog id, e.g. "core-043"; transient defs (Fuse, Craft a Card) use "t-<n>". */
  id: string;
  /** §5: "43", token "51.1", shared token "T-rush". */
  index: string;
  name: string;
  set: SetName;
  type: CardType;
  tags: Tag[];
  rarity: Rarity;
  token: boolean;
  cost: CardCost;
  /**
   * R279: the cards and tokens this card's text names, by id — a name in its base or Radiant text,
   * alone or plural, or a name before its parenthesis (#95's "Call to Chaos"). A client links each
   * such name to the card it names. Absent when the text names none. A fused definition's is the
   * union of its ingredients' (R102).
   */
  refs?: string[];
  /**
   * R349: this card prints no Radiant form of its own (the Ghoul Token, §7). Its `radiant` face is
   * the fallback the rule gives it — the base face with its attack and health doubled, the same
   * keywords and text — and a summon's X/X (`statsOverride`) doubles with it at runtime. Absent on
   * every card that prints a Radiant form, a fused definition included (R77 sums the ingredients'
   * Radiant forms).
   */
  radiantFallback?: true;
  base: CardFace;
  radiant: CardFace;
};

export type CardDefs = Readonly<Record<string, CardDef>>;

/**
 * §5.1: the one query every random pool and Discover goes through. Every field narrows; `{}` is every
 * non-token card of every set (R380: a pool that names no set draws from all of them).
 */
export type CatalogQuery = {
  type?: CardType | CardType[];
  cost?: number;
  costRange?: { min?: number; max?: number };
  tags?: Tag[];
  notTags?: Tag[];
  rarity?: Rarity | Rarity[];
  /** A set, or several ("Classic or Classic+"). Absent is every set (R380). */
  set?: SetName | SetName[];
  /**
   * R387: never these definitions, by catalog id — a card's own id, so it never generates itself
   * (§5.1, B4.1). An index is unique only within its set, so a pool never excludes by index.
   */
  excludeDefId?: string | string[];
  /**
   * R382: tokens may come out of this pool beside the cards — Dropshipping's "(including tokens)".
   * Without it a pool holds no token, except that a Fruit pool holds the Grapes.
   */
  withTokens?: boolean;
};

/**
 * §10.6. E18 adds: `number` (a number from a fixed range, Classic #18), `answer` (one of a
 * multiple-choice problem's options, Classic+ #42), `cell` (a board cell, Classic+ #62), `reward`
 * (a completed quest's reward, Classic #90) and `pick` (a budgeted pick of several cards from a pile,
 * Classic #34 and #44). A mode prompt the other player holds is a `mode` prompt with their id.
 */
export type PromptKind =
  | "discover"
  | "target"
  | "mode"
  | "mulligan"
  | "hand"
  | "zone"
  | "tribute"
  | "direction"
  | "x"
  | "embiggen"
  | "number"
  | "answer"
  | "cell"
  | "reward"
  | "pick";

export type Row = "units" | "backrow";

/** A zone a card can sit in. Field zones name a side, a row and a lane (§3). */
export type Zone =
  | { z: "hand" | "library" | "graveyard" | "exile"; player: PlayerId }
  | { z: "field"; player: PlayerId; row: Row; lane: number }
  | { z: "resolving"; player: PlayerId }
  /** R11: a unit token that left the field, or any card that ceased to exist (R86). */
  | { z: "gone"; player: PlayerId };

export type ZoneRef = { player: PlayerId; row: Row; lane: number };

/** Which cards a declared choice may pick (§10.6, R81). */
export type TargetFilter = {
  side?: "ally" | "enemy" | "any";
  /** `graveyard`: a card in a graveyard on the named side (Classic #54's "on the field or in your graveyard"). */
  of?: ("unit" | "hero" | "backrow" | "hand" | "zone" | "graveyard")[];
  type?: CardType | CardType[];
  tags?: Tag[];
  notTags?: Tag[];
  excludeSelf?: boolean;
  /** The card's cost as R65 reads it where it is now (a hand card at its hand cost). */
  costRange?: { min?: number; max?: number };
  /** A unit with damage above 0 (Classic+ #32.1 Execute). */
  damaged?: boolean;
  /** A card with at least one Plague Token on it (Classic #78 Mutate Spell). */
  plague?: boolean;
  /**
   * The name of a predicate in the declaring script's `targetChecks`, for a filter no field above
   * can say (Classic #32's lane rule, #48's lines of code). Data, so a declaration stays JSON.
   */
  check?: string;
};

/** What a card asks for as part of its own play (R81). */
export type TargetDecl = {
  kind: Extract<PromptKind, "target" | "hand" | "zone" | "tribute">;
  min: number;
  max: number;
  filter?: TargetFilter;
  /** For Tribute: how many tributes the play costs (Sheep Tokens count 2, §6.3). */
  amount?: number;
  /**
   * The modes this declaration belongs to, when it belongs to some only: the play asks for it only
   * when one of its chosen modes is listed here, and takes nothing for it otherwise (R90). #24
   * Efficiency Dividend's target is its damage and heal modes' — "deal X damage to a target; heal a
   * target 2X" — and its mana mode names none (§8 Conventions).
   */
  forModes?: string[];
};

/**
 * A choice among fixed options that travels in the play (R81): a mode, a direction, or E18's number
 * (Classic #18's 0 to 10, whose options are the numbers themselves).
 */
export type ModeDecl = {
  kind: Extract<PromptKind, "mode" | "direction" | "number">;
  options: string[];
};
