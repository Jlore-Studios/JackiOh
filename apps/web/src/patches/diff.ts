// Patch card diffs (brief B4.2; R388) compare snapshots, unknown fields included so no change is lost.
// Text uses `fillParams` and R277 word diffs (R432; B2.7, B2.5, B3.4; R279; E36; R507). Public catalog
// data only (CLAUDE.md rule 7).

import { fillParams, keywordKey, type CardCost, type CardDef, type CardDefs, type CardFace, type Param } from "@jackioh/shared";

import { powerTitle } from "../cards/inPlay.ts";
import { wordDiff, type TextRange } from "../cards/radiantDiff.ts";

export type FaceKey = "base" | "radiant";

export type ValueChange = {
  readonly kind: "value";
  readonly field: string;
  readonly label: string;
  readonly before: string;
  readonly after: string;
};

export type TextChange = {
  readonly kind: "text";
  readonly field: "base.text" | "radiant.text";
  readonly label: string;
  readonly before: string;
  readonly after: string;
  readonly added: readonly TextRange[];
  readonly removed: readonly TextRange[];
};

export type FieldChange = ValueChange | TextChange;

export type CardDelta =
  | { readonly kind: "added"; readonly id: string; readonly def: CardDef }
  | { readonly kind: "removed"; readonly id: string; readonly def: CardDef }
  | {
      readonly kind: "changed";
      readonly id: string;
      readonly before: CardDef;
      readonly def: CardDef;
      readonly changes: readonly FieldChange[];
    };

export const NONE = "none";
export const NOT_RECORDED = "not recorded";

/** How the diff reads one `CardDef` key (#2607). A `printed` key and a `data` one (R507: no face prints
 * it) each have a `FIELD_LABEL` entry and a line of their own in `fieldChanges`, under the key's own
 * name, the name `patches ship` records (`changed_fields`, R388); a `face` is diffed field by field
 * (`base.text`, `radiant.stats`); an `ignored` key is never a change. */
type KeyRole = "printed" | "data" | "face" | "ignored";

/** Every `CardDef` key and its role, so a key `CardDef` gains with no entry here, or an entry for a key
 * it lost, fails the web's typecheck (#2607), as a printed or data key with no `FIELD_LABEL` does. R507:
 * fields without printed faces are listed by name. R1748001 and R1748002's flags are the Buff's data,
 * and so is the Core card a Training Grounds copy named (`snapshotOf`, in the catalogs shipped from
 * v0.5.5 to v0.5.7; R2511001). A Resource's prices print: its face prints its Credits (R2521009). */
const CARD_KEYS = {
  // The card itself: a delta's `id`, never one of its changes.
  id: "ignored",
  index: "printed",
  name: "printed",
  set: "printed",
  type: "printed",
  tags: "printed",
  rarity: "printed",
  printedRarity: "printed",
  // R1437000: display only and in no snapshot, so a change to it alone is no card change.
  flavor: "ignored",
  token: "printed",
  cost: "printed",
  refs: "data",
  params: "data",
  dealsDamage: "data",
  usesPlague: "data",
  resource: "printed",
  loc: "data",
  radiantFallback: "printed",
  // A Fuse's and a crafter's own (R179, R882), on a transient definition no snapshot holds.
  ingredients: "data",
  craft: "data",
  snapshotOf: "data",
  base: "face",
  radiant: "face",
} as const satisfies Record<keyof CardDef, KeyRole>;

type CardKey = keyof typeof CARD_KEYS;
/** The keys with a line of their own: every printed and data one. */
type LabeledKey = { [K in CardKey]: (typeof CARD_KEYS)[K] extends "printed" | "data" ? K : never }[CardKey];
/** What `fieldChanges` diffs on each face, under `<face>.<field>`. */
type FaceField = `${FaceKey}.${"type" | "stats" | "keywords" | "text" | "grants"}`;

export const FIELD_LABEL = {
  name: "Name",
  cost: "Cost",
  type: "Type",
  "base.type": "Base type",
  "radiant.type": "Radiant type",
  rarity: "Rarity",
  printedRarity: "Printed rarity",
  tags: "Tags",
  "base.stats": "Stats",
  "radiant.stats": "Radiant stats",
  "base.keywords": "Keywords",
  "radiant.keywords": "Radiant keywords",
  "base.text": "Text",
  "radiant.text": "Radiant text",
  "base.grants": "Granted ability",
  "radiant.grants": "Radiant granted ability",
  params: "Tunable numbers",
  refs: "Cards it names",
  loc: "Lines of code",
  index: "Number",
  set: "Set",
  token: "Token",
  radiantFallback: "Radiant form",
  dealsDamage: "Deals damage",
  usesPlague: "Uses its Plague Counters",
  snapshotOf: "Copy of",
  resource: "Resource prices",
  ingredients: "Ingredients",
  craft: "Recipe",
} as const satisfies Record<LabeledKey | FaceField, string>;

const DATA_ONLY_FIELDS: ReadonlySet<string> = new Set(Object.entries(CARD_KEYS).flatMap(([key, role]) => (role === "data" ? [key] : [])));

const KNOWN_KEYS: ReadonlySet<string> = new Set(Object.keys(CARD_KEYS));

/** A price in R432's words, an embiggen card's included (R1175000). */
export function costText(cost: CardCost): string {
  if (typeof cost === "number") return `(${String(cost)}) Cost`;
  if (cost === "X") return "(X) Cost";
  return `(${String(cost.base)}) Cost, Embiggened (${String(cost.embiggen)})`;
}

export function statsText(face: CardFace): string {
  if (face.xStats !== undefined) return `[${String(face.xStats.attack)}X/${String(face.xStats.health)}X]`;
  if (face.attack === undefined && face.health === undefined) return "";
  return `${String(face.attack ?? 0)}/${String(face.health ?? 0)}`;
}

function listText(items: readonly string[]): string {
  return items.length === 0 ? NONE : items.join(", ");
}

function paramText(param: Param): string {
  const parts = [`${param.key} ${String(param.base)}`];
  if (param.radiant !== param.base) parts.push(`Radiant ${String(param.radiant)}`);
  if (param.step !== undefined) parts.push(`step ${String(param.step)}`);
  if (param.min !== undefined) parts.push(`at least ${String(param.min)}`);
  if (param.max !== undefined) parts.push(`at most ${String(param.max)}`);
  if (param.tunedOn === "radiant") parts.push("tuned on the Radiant face only");
  if (param.tunedOn === "base") parts.push("tuned on the base face only");
  if (param.power !== undefined) parts.push(`tuned only while the card has ${powerTitle({ name: param.power }, false)}`);
  // R1748000: a number only the card's Cry reads is out of a Nerf's and a Buff's reach on the field,
  // on the one face it names (R2261001).
  if (param.cry === true) parts.push("tuned only off the field");
  if (param.cry === "base") parts.push("tuned only off the field on the base face");
  if (param.cry === "radiant") parts.push("tuned only off the field on the Radiant face");
  parts.push(param.better === "up" ? "more is better" : "less is better");
  return parts.join(", ");
}

/** ME-GRANT: the abilities a face grants, their numbers filled in as its text's are. */
function grantsText(def: CardDef, face: FaceKey): string {
  const grants = Object.values(def[face].grants ?? {});
  if (grants.length === 0) return NONE;
  return grants.map((text) => fillParams({ ...def, base: { ...def.base, text }, radiant: { ...def.radiant, text } }, face)).join(" ");
}

function paramsText(params: readonly Param[] | undefined): string {
  return params === undefined || params.length === 0 ? NONE : params.map(paramText).join("; ");
}

function refsText(refs: readonly string[] | undefined, defs: CardDefs | undefined): string {
  return listText((refs ?? []).map((id) => defs?.[id]?.name ?? id));
}

/** The Core card a Training Grounds copy named (`snapshotOf`), by its name in the snapshot, as `refs` are. */
function copiedText(def: CardDef, defs: CardDefs | undefined): string {
  return refsText(def.snapshotOf === undefined ? [] : [def.snapshotOf], defs);
}

/** A Resource's rarity and each face's prices (R1840000, R1840001): what a sale fetches, the Credits
 * its face prints (R2521009), and what the Marketplace asks for it. */
function resourceText(resource: CardDef["resource"]): string {
  if (resource === undefined) return NONE;
  const prices = (price: typeof resource.base): string =>
    `sells for ${String(price.sellMin)}–${String(price.sellMax)} Credits, bought for ${String(price.buy)} Credits`;
  return `${resource.rarity}, ${prices(resource.base)}; Radiant ${prices(resource.radiant)}`;
}

function dataText(value: unknown): string {
  if (value === undefined) return NONE;
  return typeof value === "string" ? value : JSON.stringify(value);
}

export type DiffNames = { readonly before?: CardDefs; readonly after?: CardDefs };

export function diffCard(before: CardDef | undefined, after: CardDef | undefined, names: DiffNames = {}): CardDelta | null {
  if (before === undefined && after === undefined) return null;
  if (before === undefined) return after === undefined ? null : { kind: "added", id: after.id, def: after };
  if (after === undefined) return { kind: "removed", id: before.id, def: before };
  const changes = fieldChanges(before, after, names);
  return changes.length === 0 ? null : { kind: "changed", id: after.id, before, def: after, changes };
}

function fieldChanges(before: CardDef, after: CardDef, names: DiffNames): FieldChange[] {
  const out: FieldChange[] = [];
  const value = (field: keyof typeof FIELD_LABEL, was: string, now: string): void => {
    if (was !== now) out.push({ kind: "value", field, label: FIELD_LABEL[field], before: was, after: now });
  };

  value("name", before.name, after.name);
  value("cost", costText(before.cost), costText(after.cost));
  value("type", before.type, after.type);
  for (const face of ["base", "radiant"] as const) {
    // B2.7: report a card's new type only once.
    const was = before[face].type;
    const now = after[face].type;
    if (was !== now) value(`${face}.type`, was ?? before.type, now ?? after.type);
  }
  value("rarity", before.rarity, after.rarity);
  value("printedRarity", before.printedRarity ?? NONE, after.printedRarity ?? NONE);
  value("tags", listText(before.tags), listText(after.tags));
  for (const face of ["base", "radiant"] as const) {
    value(`${face}.stats`, statsText(before[face]) || NONE, statsText(after[face]) || NONE);
  }
  for (const face of ["base", "radiant"] as const) {
    value(`${face}.keywords`, listText(before[face].keywords.map(keywordKey)), listText(after[face].keywords.map(keywordKey)));
  }
  for (const face of ["base", "radiant"] as const) {
    const was = fillParams(before, face);
    const now = fillParams(after, face);
    if (was !== now) {
      const diff = wordDiff(was, now);
      out.push({
        kind: "text",
        field: `${face}.text`,
        label: FIELD_LABEL[`${face}.text`],
        before: was,
        after: now,
        added: diff.added,
        removed: diff.removed,
      });
    }
  }
  for (const face of ["base", "radiant"] as const) {
    value(`${face}.grants`, grantsText(before, face), grantsText(after, face));
  }
  const paramsBefore = paramsText(before.params);
  const paramsAfter = paramsText(after.params);
  value("params", paramsBefore, paramsAfter);
  value("refs", refsText(before.refs, names.before), refsText(after.refs, names.after));
  value("loc", before.loc === undefined ? NOT_RECORDED : String(before.loc), after.loc === undefined ? NOT_RECORDED : String(after.loc));
  value("index", `#${before.index}`, `#${after.index}`);
  value("set", before.set, after.set);
  value("token", before.token ? "yes" : "no", after.token ? "yes" : "no");
  value(
    "radiantFallback",
    before.radiantFallback === true ? "the base face, doubled" : "printed",
    after.radiantFallback === true ? "the base face, doubled" : "printed",
  );
  value("dealsDamage", before.dealsDamage === true ? "yes" : "no", after.dealsDamage === true ? "yes" : "no");
  value("usesPlague", before.usesPlague === true ? "yes" : "no", after.usesPlague === true ? "yes" : "no");
  value("snapshotOf", copiedText(before, names.before), copiedText(after, names.after));
  value("resource", resourceText(before.resource), resourceText(after.resource));
  value("ingredients", dataText(before.ingredients), dataText(after.ingredients));
  value("craft", dataText(before.craft), dataText(after.craft));

  // Anything else the catalog grows: reported under its key rather than dropped.
  const keys = new Set([...Object.keys(before), ...Object.keys(after)]);
  const loose = (def: CardDef, key: string): unknown => (def as unknown as Record<string, unknown>)[key];
  for (const key of [...keys].filter((candidate) => !KNOWN_KEYS.has(candidate)).sort()) {
    const was = dataText(loose(before, key));
    const now = dataText(loose(after, key));
    if (was !== now) out.push({ kind: "value", field: `data:${key}`, label: key, before: was, after: now });
  }
  return out;
}

export function printsOnFace(change: FieldChange): boolean {
  return !DATA_ONLY_FIELDS.has(change.field) && !change.field.startsWith("data:");
}

export function dataOnly(delta: CardDelta): boolean {
  return delta.kind === "changed" && !delta.changes.some(printsOnFace);
}

/** R507: show Radiant only when every printed change is Radiant-specific. */
export function faceShown(delta: CardDelta): FaceKey {
  if (delta.kind !== "changed") return "base";
  const printed = delta.changes.filter(printsOnFace);
  return printed.length > 0 && printed.every((change) => change.field.startsWith("radiant.")) ? "radiant" : "base";
}

export function changedWords(change: TextChange): { added: string[]; removed: string[] } {
  return {
    added: change.added.map((range) => change.after.slice(range.start, range.end)),
    removed: change.removed.map((range) => change.before.slice(range.start, range.end)),
  };
}
