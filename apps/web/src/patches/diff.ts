// Patch card diffs (brief B4.2; R388) compare snapshots, including unknown fields so no change is lost.
// Text uses `fillParams` and R277 word diffs (R432; B2.7, B2.5, B3.4; R279; E36; R507).
// They read public catalog data only (CLAUDE.md rule 7).

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
  params: "Tunable numbers",
  refs: "Cards it names",
  loc: "Lines of code",
  index: "Number",
  set: "Set",
  token: "Token",
  radiantFallback: "Radiant form",
} as const;

/** R507: fields without printed faces are listed by name. */
const DATA_ONLY_FIELDS: ReadonlySet<string> = new Set(["loc", "refs", "params"]);

const KNOWN_KEYS: ReadonlySet<string> = new Set([
  "id",
  "index",
  "name",
  "set",
  "type",
  "tags",
  "rarity",
  "printedRarity",
  "token",
  "cost",
  "refs",
  "params",
  "loc",
  "radiantFallback",
  "base",
  "radiant",
]);

export function costText(cost: CardCost): string {
  if (typeof cost === "number") return `(${String(cost)}) Cost`;
  if (cost === "X") return "(X) Cost";
  return `(${String(cost.base)}) Cost, embiggen (${String(cost.embiggen)})`;
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
  parts.push(param.better === "up" ? "more is better" : "less is better");
  return parts.join(", ");
}

function paramsText(params: readonly Param[] | undefined): string {
  return params === undefined || params.length === 0 ? NONE : params.map(paramText).join("; ");
}

function refsText(refs: readonly string[] | undefined, defs: CardDefs | undefined): string {
  return listText((refs ?? []).map((id) => defs?.[id]?.name ?? id));
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
