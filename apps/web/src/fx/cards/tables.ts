// R2784000: every card's signature animation, one JSON5 table per set (Stinkyscape's in two, split at
// `stinkyscape-091`), read as text and checked against the catalog once at import, so a bad entry fails
// loudly at load naming its file and card (docs/polish/9-card-animations.md). An entry is `"none: <reason>"`,
// `{ bespoke: "<recipe>" }` (bespoke.ts), `{ setPiece: "<key>" }` (a board clear's set piece, setPieceKeys.ts,
// R2800000) or a motif per hook: `{ cast: { motif, palette?, row? } }`. The tables are bundled data keyed by
// a definition the viewer reads; a hidden play never keys one (R202).
// R2787000: a hooks entry may also carry, or be only, an `aura: { look, palette?, scope }`, the standing
// look its card wears while it holds (fx/aura/); a `zone` or `lane` aura stands in a Field card's zone.
// R2794000: a Unit's hooks entry may also carry `entrance: "<key>"`, its marquee entrance (entrances.ts)
// beside its hooks. R3332000: every entry but a `none` may carry the frame (`FxCardFrame`) beside its kind:
// an `entrance` on a Unit's or a Field Spell's (alone, it is a hooks entry of no hook), a `takeover` look
// of its own on a Mythic's (takeover.ts; the rest take "prismatic") and `signature: true` on a
// Legendary's or a Mythic's, which must name its first beat: a Unit's entrance, a Field Spell's entrance
// or cast of its own, any other card's cast of its own (`{ bespoke }` or `{ setPiece }`). A token's
// rarity here is the one it prints (B2.5), as its entrance's (game/animations.ts `EntranceResolver`).
// R3400000: a `{ bespoke }` entry may wear an `aura` too, and a Unit's aura may stand on its own `tile`
// (its unit zone, fx/aura/plan.ts); "summit", "canvas" and "napkin" join the Mythics' own takeover looks.
// R3405000: Meditative's Mythics' twelve join them ("ascent", "workshop", "brush", "mine", "quarry",
// "pavilion", "virus", "dojo", "studyHall", "caneRack", "warRoom", "fortress").

import JSON5 from "json5";

import type { CardDefs, SetName } from "@jackioh/shared";

import { messageOf } from "../../net/errors.ts";
import { CATALOG } from "../../wire/cards.ts";
import { isRecord } from "../../wire/json.ts";
import { isAuraFieldType, isAuraLook, isAuraScope, type AuraLook, type AuraScope } from "../aura/looks.ts";
import { isPalette, type FxPalette } from "../flavors.ts";
import { isCardHook, isMotifName, type FxCardHook, type MotifName } from "../motifs/names.ts";
import { isSetPieceKey, type SetPieceKey } from "../setPieceKeys.ts";
import type { TakeoverLook } from "../types.ts";
import { isCardRecipeKey, isEntranceKey, type CardRecipeKey } from "./bespoke.ts";
import type { EntranceKey } from "./recipe.ts";
import rawClassic from "./classic.json5?raw";
import rawClassicPlus from "./classicplus.json5?raw";
import rawCore from "./core.json5?raw";
import rawMeditative from "./meditative.json5?raw";
import rawMeditativePlus from "./meditativeplus.json5?raw";
import rawStinkyscape1 from "./stinkyscape-1.json5?raw";
import rawStinkyscape2 from "./stinkyscape-2.json5?raw";
import rawTrainingGrounds from "./traininggrounds.json5?raw";

/** One table file: its name in `fx/cards/`, the set it holds and its text. */
export type FxCardFile = { file: string; set: SetName; source: string };

/** In catalog order. */
export const FX_CARD_FILES: readonly FxCardFile[] = [
  { file: "core.json5", set: "Core", source: rawCore },
  { file: "classic.json5", set: "Classic", source: rawClassic },
  { file: "classicplus.json5", set: "Classic+", source: rawClassicPlus },
  { file: "meditative.json5", set: "Meditative", source: rawMeditative },
  { file: "meditativeplus.json5", set: "Meditative+", source: rawMeditativePlus },
  { file: "stinkyscape-1.json5", set: "Stinkyscape", source: rawStinkyscape1 },
  { file: "stinkyscape-2.json5", set: "Stinkyscape", source: rawStinkyscape2 },
  { file: "traininggrounds.json5", set: "Training Grounds", source: rawTrainingGrounds },
];

/** The first card of Stinkyscape's second file. */
export const FX_STINKYSCAPE_SPLIT = "stinkyscape-091";

/** A motif on one hook. `palette` is a flavor or `"<Flavor>/<sub-look>"` (the card's own look by default); `row` defaults to `replace`. */
export type FxHookSpec = { motif: MotifName; palette?: FxPalette; row?: "keep" | "replace" };

/** R2787000: a card's standing look. `palette` is a flavor or `"<Flavor>/<sub-look>"` (the card's own look by default). */
export type FxAuraSpec = { look: AuraLook; palette?: FxPalette; scope: AuraScope };

/** R3332000: a takeover look a Mythic may name as its own; every other Mythic takes the shared "prismatic". */
export type OwnTakeoverLook = Exclude<TakeoverLook, "prismatic">;

/** R3332000: every look `OwnTakeoverLook` holds; a missing one fails `typecheck`. */
export const OWN_TAKEOVER_LOOKS: Readonly<Record<OwnTakeoverLook, true>> = {
  void: true,
  pawn: true,
  genie: true,
  ai: true,
  summit: true,
  canvas: true,
  napkin: true,
  // R3405000: Meditative's Mythics'.
  ascent: true,
  workshop: true,
  brush: true,
  mine: true,
  quarry: true,
  pavilion: true,
  virus: true,
  dojo: true,
  studyHall: true,
  caneRack: true,
  warRoom: true,
  fortress: true,
};

function isOwnTakeoverLook(look: string): look is OwnTakeoverLook {
  return Object.prototype.hasOwnProperty.call(OWN_TAKEOVER_LOOKS, look);
}

/**
 * R3332000: what an entry may carry beside its kind: the entrance its card lands with in place of its
 * rarity's (a Unit's or a Field Spell's), the takeover look a Mythic's takeover wears, and whether the
 * entry is the card's signature (a Legendary's or a Mythic's, its first beat named).
 */
export type FxCardFrame = { entrance?: EntranceKey; takeover?: OwnTakeoverLook; signature?: true };

export type FxCardEntry =
  | { kind: "none"; reason: string }
  | ({ kind: "bespoke"; key: CardRecipeKey; aura?: FxAuraSpec } & FxCardFrame)
  | ({ kind: "setPiece"; key: SetPieceKey } & FxCardFrame)
  | ({ kind: "hooks"; hooks: Partial<Record<FxCardHook, FxHookSpec>>; aura?: FxAuraSpec } & FxCardFrame);

/** The catalog facts an entry is checked against: the card's type and its rarity (a token's printed one). */
type CardFacts = { type: string; rarity: string };

const HOOK_FIELDS: readonly string[] = ["motif", "palette", "row"];
const AURA_FIELDS: readonly string[] = ["look", "palette", "scope"];
const FRAME_FIELDS: readonly string[] = ["entrance", "takeover", "signature"];
/** R3332000: the cards that land on the field face up as they are played, so an entrance can draw them landing. */
const ENTRANCE_TYPES: readonly string[] = ["Unit", "Field Spell"];

function parseHook(value: unknown, hook: FxCardHook, fail: (problem: string) => never): FxHookSpec {
  if (!isRecord(value)) fail(`${hook} must be { motif, palette?, row? }`);
  for (const field of Object.keys(value)) {
    if (!HOOK_FIELDS.includes(field)) fail(`unknown field "${field}" in ${hook}`);
  }
  const { motif, palette, row } = value;
  if (typeof motif !== "string" || !isMotifName(motif)) fail(`unknown motif "${String(motif)}" in ${hook}`);
  const spec: FxHookSpec = { motif };
  if (palette !== undefined) {
    if (typeof palette !== "string" || !isPalette(palette)) fail(`unknown palette "${String(palette)}" in ${hook}`);
    spec.palette = palette;
  }
  if (row !== undefined) {
    if (row !== "keep" && row !== "replace") fail(`row in ${hook} must be "keep" or "replace"`);
    spec.row = row;
  }
  return spec;
}

function parseAura(value: unknown, type: string, fail: (problem: string) => never): FxAuraSpec {
  if (!isRecord(value)) fail("aura must be { look, palette?, scope }");
  for (const field of Object.keys(value)) {
    if (!AURA_FIELDS.includes(field)) fail(`unknown field "${field}" in aura`);
  }
  const { look, palette, scope } = value;
  if (typeof look !== "string" || !isAuraLook(look)) fail(`unknown aura look "${String(look)}"`);
  if (typeof scope !== "string" || !isAuraScope(scope)) fail(`unknown aura scope "${String(scope)}"`);
  if ((scope === "zone" || scope === "lane") && !isAuraFieldType(type)) {
    fail(`a ${scope} aura stands in a zone; this card is a ${type}, whose aura is side or board`);
  }
  // R3400000: a tile is a Unit's own unit zone.
  if (scope === "tile" && type !== "Unit") fail(`a tile aura stands on a Unit's own tile; this card is a ${type}`);
  const spec: FxAuraSpec = { look, scope };
  if (palette !== undefined) {
    if (typeof palette !== "string" || !isPalette(palette)) fail(`unknown palette "${String(palette)}" in aura`);
    spec.palette = palette;
  }
  return spec;
}

function has(value: Record<string, unknown>, key: string): boolean {
  return Object.prototype.hasOwnProperty.call(value, key);
}

/** R3332000: the frame fields `value` names, each checked against the card; an absent one stays absent. */
function parseFrame(value: Record<string, unknown>, card: CardFacts, fail: (problem: string) => never): FxCardFrame {
  const frame: FxCardFrame = {};
  if (has(value, "entrance")) {
    const key = value.entrance;
    if (typeof key !== "string" || !isEntranceKey(key)) fail(`unknown entrance "${String(key)}"`);
    if (!ENTRANCE_TYPES.includes(card.type)) fail(`an entrance is a Unit's or a Field Spell's; this card is a ${card.type}`);
    frame.entrance = key;
  }
  if (has(value, "takeover")) {
    const look = value.takeover;
    if (typeof look !== "string" || !isOwnTakeoverLook(look)) fail(`unknown takeover "${String(look)}"`);
    if (card.rarity !== "Mythic") fail(`a takeover is a Mythic's; this card is ${card.rarity}`);
    frame.takeover = look;
  }
  if (has(value, "signature")) {
    if (value.signature !== true) fail("signature must be true");
    if (card.rarity !== "Legendary" && card.rarity !== "Mythic") fail(`a signature is a Legendary's or a Mythic's; this card is ${card.rarity}`);
    frame.signature = true;
  }
  return frame;
}

/**
 * R3332000: a signature names its first beat: a Unit's entrance; a Field Spell's entrance or a cast of
 * its own; any other card's cast of its own, a `{ bespoke }` recipe or a `{ setPiece }`.
 */
function checkSignature(frame: FxCardFrame, kind: "bespoke" | "setPiece" | "hooks", type: string, fail: (problem: string) => never): void {
  if (frame.signature !== true) return;
  const ownCast = kind === "bespoke" || kind === "setPiece";
  if (type === "Unit") {
    if (frame.entrance === undefined) fail("a Unit's signature names its entrance");
  } else if (type === "Field Spell") {
    if (frame.entrance === undefined && !ownCast) fail("a Field Spell's signature names its entrance or a { bespoke } or { setPiece } cast of its own");
  } else if (!ownCast) {
    fail(`a ${type}'s signature is a { bespoke } or { setPiece } cast of its own`);
  }
}

/** A `{ bespoke }` or `{ setPiece }` entry's fields: its key and the frame, nothing else (R3400000: a bespoke entry's aura besides). */
function onlyKeyAndFrame(value: Record<string, unknown>, kind: "bespoke" | "setPiece", fail: (problem: string) => never): void {
  for (const field of Object.keys(value)) {
    if (field === kind || FRAME_FIELDS.includes(field) || (kind === "bespoke" && field === "aura")) continue;
    fail(kind === "bespoke" ? "a bespoke entry holds nothing else but its frame and its aura" : `a ${kind} entry holds nothing else but its frame`);
  }
}

function parseEntry(value: unknown, card: CardFacts, fail: (problem: string) => never): FxCardEntry {
  const { type } = card;
  if (typeof value === "string") {
    if (!/^none: \S/.test(value)) fail(`a string entry must be "none: <reason>" (found ${JSON.stringify(value)})`);
    return { kind: "none", reason: value.slice("none: ".length) };
  }
  if (!isRecord(value)) fail(`must be "none: <reason>", { bespoke }, { setPiece } or hooks`);
  if (has(value, "bespoke")) {
    const key = value.bespoke;
    onlyKeyAndFrame(value, "bespoke", fail);
    if (typeof key !== "string" || !isCardRecipeKey(key)) fail(`unknown bespoke "${String(key)}"`);
    const aura = has(value, "aura") ? parseAura(value.aura, type, fail) : undefined;
    const frame = parseFrame(value, card, fail);
    checkSignature(frame, "bespoke", type, fail);
    return { kind: "bespoke", key, ...(aura === undefined ? {} : { aura }), ...frame };
  }
  if (has(value, "setPiece")) {
    const key = value.setPiece;
    onlyKeyAndFrame(value, "setPiece", fail);
    if (typeof key !== "string" || !isSetPieceKey(key)) fail(`unknown set piece "${String(key)}"`);
    const frame = parseFrame(value, card, fail);
    checkSignature(frame, "setPiece", type, fail);
    return { kind: "setPiece", key, ...frame };
  }
  const hooks: Partial<Record<FxCardHook, FxHookSpec>> = {};
  let aura: FxAuraSpec | undefined;
  for (const [hook, spec] of Object.entries(value)) {
    if (FRAME_FIELDS.includes(hook)) continue;
    if (hook === "aura") {
      aura = parseAura(spec, type, fail);
      continue;
    }
    if (!isCardHook(hook)) fail(`unknown hook "${hook}"`);
    hooks[hook] = parseHook(spec, hook, fail);
  }
  const frame = parseFrame(value, card, fail);
  if (Object.keys(hooks).length === 0 && aura === undefined && frame.entrance === undefined) fail("names no hook");
  if (hooks.cry !== undefined && type !== "Unit") fail(`cry is a Unit's play; this card is a ${type}, whose play is cast`);
  if (hooks.cast !== undefined && type === "Unit") fail("cast is a non-Unit's play; a Unit's is cry");
  checkSignature(frame, "hooks", type, fail);
  return { kind: "hooks", hooks, ...(aura === undefined ? {} : { aura }), ...frame };
}

/**
 * The entries of one table file, checked against `catalog`; throws
 * Error("fx/cards/<file>: <id>: <problem>"). That each file holds exactly its set's cards in catalog
 * order, one line each under its name, `tables.test.ts` checks.
 */
export function parseFxCardTable(file: string, source: string, catalog: Readonly<CardDefs> = CATALOG): ReadonlyMap<string, FxCardEntry> {
  const failAt = (id: string, problem: string): never => {
    throw new Error(`fx/cards/${file}: ${id}: ${problem}`);
  };
  let raw: unknown;
  try {
    raw = JSON5.parse(source);
  } catch (err) {
    failAt("(syntax)", messageOf(err));
  }
  if (!isRecord(raw)) failAt("(root)", "must be an object of card ids");
  const entries = new Map<string, FxCardEntry>();
  for (const [id, value] of Object.entries(raw as Record<string, unknown>)) {
    const def = Object.prototype.hasOwnProperty.call(catalog, id) ? catalog[id] : undefined;
    if (def === undefined) failAt(id, "not a card in the catalog");
    const card: CardFacts = { type: def?.type ?? "", rarity: def?.printedRarity ?? def?.rarity ?? "" };
    entries.set(id, parseEntry(value, card, (problem) => failAt(id, problem)));
  }
  return entries;
}

/** R3332000: whether `entry` is its card's signature (`signature: true`, its first beat named). */
export function isSignature(entry: FxCardEntry | undefined): boolean {
  return entry !== undefined && entry.kind !== "none" && entry.signature === true;
}

/** Every card's entry, all eight files merged. */
export const FX_CARD_TABLE: ReadonlyMap<string, FxCardEntry> = new Map(FX_CARD_FILES.flatMap(({ file, source }) => [...parseFxCardTable(file, source)]));
