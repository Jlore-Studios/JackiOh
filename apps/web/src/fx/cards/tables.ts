// R2784000: every card's signature animation, one JSON5 table per set (Stinkyscape's in two, split at
// `stinkyscape-091`), read as text and checked against the catalog once at import, so a bad entry fails
// loudly at load naming its file and card (docs/polish/9-card-animations.md). An entry is `"none: <reason>"`,
// `{ bespoke: "<recipe>" }` (bespoke.ts), `{ setPiece: "<key>" }` (a board clear's set piece, setPieceKeys.ts,
// R2800000) or a motif per hook: `{ cast: { motif, palette?, row? } }`. The tables are bundled data keyed by
// a definition the viewer reads; a hidden play never keys one (R202).
// R2787000: a hooks entry may also carry, or be only, an `aura: { look, palette?, scope }`, the standing
// look its card wears while it holds (fx/aura/); a `zone` or `lane` aura stands in a Field card's zone.
// R2794000: a Unit's hooks entry may also carry `entrance: "<key>"`, its marquee entrance (entrances.ts)
// beside its hooks; an entrance alone stays `{ bespoke: "<key>" }`.

import JSON5 from "json5";

import type { CardDefs, SetName } from "@jackioh/shared";

import { messageOf } from "../../net/errors.ts";
import { CATALOG } from "../../wire/cards.ts";
import { isRecord } from "../../wire/json.ts";
import { isAuraFieldType, isAuraLook, isAuraScope, type AuraLook, type AuraScope } from "../aura/looks.ts";
import { isPalette, type FxPalette } from "../flavors.ts";
import { isCardHook, isMotifName, type FxCardHook, type MotifName } from "../motifs/names.ts";
import { isSetPieceKey, type SetPieceKey } from "../setPieceKeys.ts";
import { isBespokeKey, isEntranceKey, type BespokeKey } from "./bespoke.ts";
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

export type FxCardEntry =
  | { kind: "none"; reason: string }
  | { kind: "bespoke"; key: BespokeKey }
  | { kind: "setPiece"; key: SetPieceKey }
  | { kind: "hooks"; hooks: Partial<Record<FxCardHook, FxHookSpec>>; aura?: FxAuraSpec; entrance?: EntranceKey };

const HOOK_FIELDS: readonly string[] = ["motif", "palette", "row"];
const AURA_FIELDS: readonly string[] = ["look", "palette", "scope"];

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
  const spec: FxAuraSpec = { look, scope };
  if (palette !== undefined) {
    if (typeof palette !== "string" || !isPalette(palette)) fail(`unknown palette "${String(palette)}" in aura`);
    spec.palette = palette;
  }
  return spec;
}

function parseEntry(value: unknown, type: string, fail: (problem: string) => never): FxCardEntry {
  if (typeof value === "string") {
    if (!/^none: \S/.test(value)) fail(`a string entry must be "none: <reason>" (found ${JSON.stringify(value)})`);
    return { kind: "none", reason: value.slice("none: ".length) };
  }
  if (!isRecord(value)) fail(`must be "none: <reason>", { bespoke }, { setPiece } or hooks`);
  if (Object.prototype.hasOwnProperty.call(value, "bespoke")) {
    const key = value.bespoke;
    if (Object.keys(value).length !== 1) fail("a bespoke entry holds nothing else");
    if (typeof key !== "string" || !isBespokeKey(key)) fail(`unknown bespoke "${String(key)}"`);
    return { kind: "bespoke", key };
  }
  if (Object.prototype.hasOwnProperty.call(value, "setPiece")) {
    const key = value.setPiece;
    if (Object.keys(value).length !== 1) fail("a setPiece entry holds nothing else");
    if (typeof key !== "string" || !isSetPieceKey(key)) fail(`unknown set piece "${String(key)}"`);
    return { kind: "setPiece", key };
  }
  const hooks: Partial<Record<FxCardHook, FxHookSpec>> = {};
  let aura: FxAuraSpec | undefined;
  let entrance: EntranceKey | undefined;
  for (const [hook, spec] of Object.entries(value)) {
    if (hook === "aura") {
      aura = parseAura(spec, type, fail);
      continue;
    }
    if (hook === "entrance") {
      if (typeof spec !== "string" || !isEntranceKey(spec)) fail(`unknown entrance "${String(spec)}"`);
      if (type !== "Unit") fail(`an entrance is a Unit's; this card is a ${type}`);
      entrance = spec;
      continue;
    }
    if (!isCardHook(hook)) fail(`unknown hook "${hook}"`);
    hooks[hook] = parseHook(spec, hook, fail);
  }
  if (Object.keys(hooks).length === 0 && aura === undefined) fail("names no hook");
  if (hooks.cry !== undefined && type !== "Unit") fail(`cry is a Unit's play; this card is a ${type}, whose play is cast`);
  if (hooks.cast !== undefined && type === "Unit") fail("cast is a non-Unit's play; a Unit's is cry");
  return { kind: "hooks", hooks, ...(aura === undefined ? {} : { aura }), ...(entrance === undefined ? {} : { entrance }) };
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
    entries.set(id, parseEntry(value, def?.type ?? "", (problem) => failAt(id, problem)));
  }
  return entries;
}

/** Every card's entry, all eight files merged. */
export const FX_CARD_TABLE: ReadonlyMap<string, FxCardEntry> = new Map(FX_CARD_FILES.flatMap(({ file, source }) => [...parseFxCardTable(file, source)]));
