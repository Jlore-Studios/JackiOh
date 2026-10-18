// The web client's own copy of the shared wire helper (docs/v0.3.0/SURFACE.md §10.4), kept as
// TypeScript and unchanged but for its import paths; the server's port is crates/engine/src/wire/catalog_types.rs.
//
// Card definitions and the vocabulary every package shares (SPEC §5, §6.1, §10.6).
// Script and Effect types live in engine/src/script.ts: they need GameState, which lives in the engine.

export type PlayerId = "p1" | "p2";
export const PLAYER_IDS = ["p1", "p2"] as const;

export function opponentOf(player: PlayerId): PlayerId {
  return player === "p1" ? "p2" : "p1";
}

/** §5.1 */
export type CardType = "Unit" | "Spell" | "Field Spell" | "Trap" | "Field Trap";

/**
 * §5: tribes and tags. "Jlockeed" is Core #13 and #14's and the Classic+ Jlockeed cards' (R278);
 * patch v0.2.0 adds Book (every "Book of …" card), Pancake (Classic+ #12, #13 and the eight Pancake
 * tokens) and AI (the ten AI generated cards); the v0.2.x mechanics patch adds Plague (every card
 * that uses Plague Counters); patch v0.2.Y adds Catalyst (Classic+ #38 Solarius and #46 Felinor
 * Flagbearer), Prime (their Prime tokens, Classic+ #38.1 Solarius Prime and #46.1 Felinor
 * Flagbearer Prime) and Acclaimed (Classic #80 BOOM! Big Max and Classic+ #37 Wardrum).
 */
export type Tag =
  | "Human"
  | "Felinor"
  | "KY"
  | "CN"
  | "Fruit"
  | "Call to Chaos"
  | "Quickdraw"
  | "Jlockeed"
  | "Book"
  | "Pancake"
  | "AI"
  | "Plague"
  | "Catalyst"
  | "Prime"
  | "Acclaimed"
  | "Wincon"
  | "Potion"
  | "Token";

/** §8: Core's by mechanical complexity, Classic's and Classic+'s the designer's; every token carries "Token". */
export type Rarity = "Common" | "Rare" | "Epic" | "Legendary" | "Mythic" | "Token";

/** A rarity a card prints: every rarity but Token. A token's printed one is display only (B2.5). */
export type PrintedRarity = Exclude<Rarity, "Token">;

/**
 * §5: Core, Classic, Classic+ (R380) and Meditative (v0.3.5, R1420) ship; Meditative+ is in the
 * catalog and ships with the last part of its patch (issue #1173, R1178000); Boss and Boss-X are
 * reserved.
 */
export type SetName = "Core" | "Classic" | "Classic+" | "Meditative" | "Meditative+" | "Boss" | "Boss-X" | "Custom";

/**
 * The sets that ship, in catalog order. A pool that names no set draws from all of them (R380), and
 * a set the catalog holds that is not listed here is in no pool, no deck and no list a player reads
 * until it is (R1420).
 */
export const SHIPPED_SETS = ["Core", "Classic", "Classic+", "Meditative"] as const satisfies readonly SetName[];

/** R1420: whether a set ships (`SHIPPED_SETS` lists it). The engine's `set_ships`, mirrored. */
export function setShips(set: SetName): boolean {
  return (SHIPPED_SETS as readonly SetName[]).includes(set);
}

/**
 * R1371: the newest set that ships, the last entry of `SHIPPED_SETS`: Classic+ until the Meditative
 * set ships. The engine's `newest_shipped_set`, mirrored, so "More cards from the newest set" names
 * the set it leans on (R1372, R1373) without ever writing one down. Which set a deck leans on is
 * still the dealer's to resolve: the server's for All Random, the practice worker's for its random
 * deck.
 */
export function newestShippedSet(): SetName {
  const newest = SHIPPED_SETS[SHIPPED_SETS.length - 1];
  if (newest === undefined) throw new Error("SHIPPED_SETS lists no set");
  return newest;
}

/** R924000: the word the pick "newest" is written as. The engine's `LEAN_NEWEST`, mirrored. */
export const LEAN_NEWEST = "newest";

/**
 * R924000: what "More cards from" picks for a player's dealt Random deck (R1372, R1373): `"newest"`,
 * the newest set that ships when the deck is dealt (R1371), or one shipped set. No lean is the pick's
 * absence. The engine's `LeanPick`, which no generated type carries, mirrored as the string it is
 * serialised as.
 */
export type LeanPick = typeof LEAN_NEWEST | SetName;

/** R924000: the picker's choices after "none", in its order: "newest", then every set that ships. */
export function leanPicks(): LeanPick[] {
  return [LEAN_NEWEST, ...SHIPPED_SETS];
}

/** R924000: whether a value is a pick the server and the practice worker read: "newest" or a set that ships. */
export function isLeanPick(value: unknown): value is LeanPick {
  return typeof value === "string" && (leanPicks() as string[]).includes(value);
}

/** R924000: the set a pick leans on, asked when the deck is dealt: "newest" follows `newestShippedSet`. */
export function leanSetOf(pick: LeanPick): SetName {
  return pick === LEAN_NEWEST ? newestShippedSet() : pick;
}

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
  | { kind: "Immune to Spells" }
  /** MD-B1, R940: harmful effects that pick cards by a tribal tag can't target or affect this. */
  | { kind: "Immune to tribal tag based hate" }
  /** R636: a Unit may attack twice each turn. */
  | { kind: "Windfury" }
  /** R637: a card discarded from its owner's hand at the end of their turn. Not temporary mana (§2.3). */
  | { kind: "Temporary" }
  /** A Unit may attack and switch position in the same turn (R49). */
  | { kind: "Deft" }
  /** R1220: no Tribute cost may take this card, and every Sacrifice of it does nothing. */
  | { kind: "Untributable" }
  /** R1086: a Unit may be played onto one of its controller's Units, resolving on top of it before fusing into it (ME-EQUIP). */
  | { kind: "Equip" }
  /** #783, R783000: an attacker that takes no strike back, unless the defender is Ranged too. */
  | { kind: "Ranged" }
  /** #783, R783002: only Flying or Ranged attackers may attack it, unless it is in Defense Position. */
  | { kind: "Flying" }
  /** #783, R783004: may be set face-down as a Trap (`faceDown: "cloaked"`), revealed by its Activate. */
  | { kind: "Cloaked" }
  /** #783, R783010: Locks its zone as it arrives as a Unit, or as it is revealed as a Trap. */
  | { kind: "Crater" }
  /** #783, R783012: may be played from its owner's graveyard once, at its cost. */
  | { kind: "Flashback" };

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
  "Immune to tribal tag based hate",
  "Windfury",
  "Temporary",
  "Deft",
  "Untributable",
  "Equip",
  "Ranged",
  "Flying",
  "Cloaked",
  "Crater",
  "Flashback",
] as const;

export function keywordKey(keyword: Keyword): string {
  return "n" in keyword ? `${keyword.kind} ${keyword.n}` : keyword.kind;
}

export function hasKeyword(keywords: readonly Keyword[], kind: KeywordKind): boolean {
  return keywords.some((k) => k.kind === kind);
}

/**
 * One side of a card: the base form or the radiant form (§5). Spells have no stats; an Animated
 * backrow card (B3.1) prints the attack and health of the Unit it becomes.
 */
export type CardFace = {
  /**
   * B2.7: the face's own type, when it differs from the card's (Classic+ #22 Blood Moon's Radiant
   * face is a Field Trap). The card's type is its running face's (§5.2). Absent: the card's `type`.
   */
  type?: CardType;
  attack?: number;
  health?: number;
  /**
   * B2.7: "[3X/3X]" stats (Classic+ #69 Buff Billy): the Unit is summoned with `statsOverride` of
   * these multiples of the X it was played for. The printed `attack`/`health` are then 0/0, as the
   * Ghoul Token's are.
   */
  xStats?: { attack: number; health: number };
  keywords: Keyword[];
  /**
   * The face's printed text: the base face's §8 cell, or the Radiant face's cell read by §8's
   * Conventions and written out in full (R277), so a client can print it whole and mark what differs.
   * A tunable number (`CardDef.params`, B3.4) is written `{key}`, filled in by `fillParams`.
   */
  text: string;
};

/**
 * B3.4 rule 5: a number on a card that Degrade, Upgrade and KY's Constant may move. The face texts
 * write it as `{key}`; the view carries an instance's current values; scripts read `param(ctx, key)`.
 */
export type Param = {
  /** The name the texts write as `{key}`, unique within the card. */
  key: string;
  /** Its printed value on the base face. */
  base: number;
  /** Its printed value on the Radiant face. */
  radiant: number;
  /** Which way is better for the card's controller: an Upgrade moves it this way, a Degrade the other. */
  better: "up" | "down";
  /** How far one Degrade or Upgrade moves it (B3.4: 1 up to 5, 2 for 6–12, a quarter above). */
  step?: number;
  /** It never goes below this (an amount never drops below 1). */
  min?: number;
  /** It never goes above this (100 for a percentage). */
  max?: number;
  /**
   * R749, R1431: the one face a Degrade, an Upgrade or KY's Constant may move it on, for a number only
   * that face prints; on the other face it always reads its printed value. Absent, both faces.
   */
  tunedOn?: "radiant" | "base";
  /**
   * R1430: the power the number belongs to, the id of the card's Activate ability that is that power
   * (#98's stored power name). A Degrade, an Upgrade or KY's Constant reaches it only while the card
   * has that power; meanwhile it keeps its tuning. Absent, the card's number whatever its power.
   */
  power?: string;
};

/**
 * A tunable number in a face's text (B3.4 rule 5, R482): `{key}` is the number alone ("Deal {damage}
 * damage."); `{key|singular|plural}` is the number and the words that agree with it ("Draw
 * {draw|card|cards}." prints "Draw 1 card." and "Draw 2 cards."), so a text reads right at every
 * value a Degrade or an Upgrade can move it to.
 */
export const PARAM_PLACEHOLDER = /\{([A-Za-z][A-Za-z0-9]*)(?:\|([^|{}]*)\|([^|{}]*))?\}/g;

/** Every placeholder a text writes, in order: its key and, for the agreeing form, both wordings. */
export function paramPlaceholders(text: string): { key: string; one?: string; many?: string }[] {
  return [...text.matchAll(PARAM_PLACEHOLDER)].map((match) => {
    const key = match[1] ?? "";
    const one = match[2];
    const many = match[3];
    return one === undefined || many === undefined ? { key } : { key, one, many };
  });
}

/**
 * A face's text with its placeholders filled in: from `values` when given (an instance's current
 * numbers), else from the face's printed values; `{key|singular|plural}` takes the singular wording
 * at 1 and the plural at any other value. Unknown keys are left as written. Pure, so the client, the
 * tests and R277's diff all fill a text the same way.
 */
export function fillParams(
  def: Pick<CardDef, "params" | "base" | "radiant">,
  face: "base" | "radiant",
  values?: Readonly<Record<string, number>>,
): string {
  const text = def[face].text;
  const params = def.params;
  if (params === undefined || params.length === 0) return text;
  return text.replace(PARAM_PLACEHOLDER, (whole, key: string, one?: string, many?: string) => {
    const param = params.find((p) => p.key === key);
    if (param === undefined) return whole;
    const value = values?.[key] ?? param[face];
    if (one === undefined || many === undefined) return String(value);
    return `${String(value)} ${value === 1 ? one : many}`;
  });
}

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
  /**
   * B2.5: the rarity a token prints (the Classic+ tokens the designer rated), for the card frame and
   * the summon sting only. A token's `rarity` stays "Token", so no pool ever finds one by rarity.
   */
  printedRarity?: PrintedRarity;
  token: boolean;
  cost: CardCost;
  /**
   * R279: the cards and tokens this card's text names, by id — a name in its base or Radiant text,
   * alone or plural, or a name before its parenthesis (#95's "Call to Chaos"). A client links each
   * such name to the card it names. Absent when the text names none. A fused definition's is the
   * union of its ingredients' (R102).
   */
  refs?: string[];
  /** B3.4 rule 5: the numbers on this card Degrade, Upgrade and KY's Constant may move. */
  params?: Param[];
  /**
   * E36: the non-blank, non-comment lines of this card's script file, imports excluded, written by
   * TypeScript's `gen-loc.ts` and frozen since (SURFACE §7.5). Public (the inspect overlay
   * prints it) and part of the card's patch history (B4.2). Absent while the card has no script.
   */
  loc?: number;
  /**
   * R349: this card prints no Radiant form of its own (the Ghoul Token, §7). Its `radiant` face is
   * the fallback the rule gives it — the base face with its attack and health doubled, the same
   * keywords and text — and a summon's X/X (`statsOverride`) doubles with it at runtime. Absent on
   * every card that prints a Radiant form, a fused definition included (R77 sums the ingredients'
   * Radiant forms).
   */
  radiantFallback?: true;
  /**
   * R179, R468, R469: a fused definition's ingredients, in ingredient order — the definition each
   * was, and `radiant` when it went into both of the fused forms on its Radiant face ("fuse a random
   * Radiant card"). Only a Fuse writes it. While the list is short the id spells it out too; past
   * `FUSED_ID_CAP` the id is a digest of it, and this list is what rebuilds the scripts.
   */
  ingredients?: FusedIngredient[];
  base: CardFace;
  radiant: CardFace;
};

/** R179, R469: one ingredient of a fused definition (`CardDef.ingredients`). */
export type FusedIngredient = { defId: string; radiant?: true };

export type Row = "units" | "backrow";
