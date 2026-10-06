// What a patch changed in one card (brief B4.2, R388): the card as the snapshot before the patch
// held it against the card as the patch left it. Pure, so the Patch notes page, the History section
// and the tests read one answer.
//
// Every field a player can see or the catalog records is compared: the name, the cost (X and
// embiggen included, written as card text writes a cost, R432: "(3) Cost"), the type and a face's own
// type (B2.7), the rarity and a token's printed rarity (B2.5), the tags, each face's stats and
// keywords, each face's text, the tunable numbers (`params`, B3.4), the cards the text names
// (`refs`, R279), the lines of code (`loc`, E36), and the card's number, set and token flag. A
// field this module does not know is still reported, under its own key, so no change is ever lost.
//
// A face's text is compared as it prints, its `{key}` numbers filled in (`fillParams`): a v0.2.0
// text reads "Deal {damage} damage" where the patch before printed "Deal 1 damage", and the two say
// the same thing. The text diff is R277's word diff (cards/radiantDiff.ts, `wordDiff`): case aside,
// a word at a time, separators never starting or ending a mark. What the newer text has that the
// older does not is `added` (the change mark); what the older had that the newer dropped is
// `removed` (struck through where it is shown, R507).
//
// Nothing here is a rule (CLAUDE.md rule 7): it reads two public catalog snapshots.

import { fillParams, keywordKey, type CardCost, type CardDef, type CardDefs, type CardFace, type Param } from "@jackioh/shared";

import { wordDiff, type TextRange } from "../cards/radiantDiff.ts";

/** A card's two faces, as the catalog keys them. */
export type FaceKey = "base" | "radiant";

/** A field compared as one value, printed "before → after". */
export type ValueChange = {
  readonly kind: "value";
  /** "cost", "radiant.stats", …; a field this module does not know is `data:<key>`. */
  readonly field: string;
  /** What a player reads before the values: "Cost", "Radiant stats". */
  readonly label: string;
  readonly before: string;
  readonly after: string;
};

/** A face's text, word-diffed. */
export type TextChange = {
  readonly kind: "text";
  readonly field: "base.text" | "radiant.text";
  readonly label: string;
  /** The older text, its numbers filled in. */
  readonly before: string;
  /** The newer text, its numbers filled in. */
  readonly after: string;
  /** Stretches of `after` that `before` does not have. */
  readonly added: readonly TextRange[];
  /** Stretches of `before` that `after` does not have. */
  readonly removed: readonly TextRange[];
};

export type FieldChange = ValueChange | TextChange;

/** One card in one patch: added, removed, or changed with the list of what changed. */
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

/** The words for a missing list or value. */
export const NONE = "none";
/** `loc` absent: the card's lines of code were not recorded then. */
export const NOT_RECORDED = "not recorded";

/** The labels players read, by field. */
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

/**
 * Fields a face does not print: a change to these alone leaves both faces looking as they did, so
 * the Patch notes page lists such a card by name rather than as a face (R507).
 */
const DATA_ONLY_FIELDS: ReadonlySet<string> = new Set(["loc", "refs", "params"]);

/** Top-level `CardDef` keys this module compares by name; any other key is compared as data. */
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

/** R432: a cost as card text writes one: "(3) Cost", "(X) Cost", "(2) Cost, embiggen (4)". */
export function costText(cost: CardCost): string {
  if (typeof cost === "number") return `(${String(cost)}) Cost`;
  if (cost === "X") return "(X) Cost";
  return `(${String(cost.base)}) Cost, embiggen (${String(cost.embiggen)})`;
}

/** A face's stats: "2/3", "[3X/3X]" for X stats (B2.7), "" for a face with none. */
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

/** Card names for `refs`, from the snapshot each side belongs to. */
export type DiffNames = { readonly before?: CardDefs; readonly after?: CardDefs };

/**
 * R388: what changed between `before` (the card in the snapshot before the patch, absent if the
 * patch added it) and `after` (the card as the patch left it, absent if the patch removed it).
 * Null when the two are the same card, or when both are absent.
 */
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
    // B2.7: a face's own type, reported only where a face carries one, so a card's new type is said once.
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

/** Whether a change shows on a printed face (a name, a cost, a stat, a word of text…). */
export function printsOnFace(change: FieldChange): boolean {
  return !DATA_ONLY_FIELDS.has(change.field) && !change.field.startsWith("data:");
}

/** Whether a changed card's changes all lie in data the faces do not print (R507). */
export function dataOnly(delta: CardDelta): boolean {
  return delta.kind === "changed" && !delta.changes.some(printsOnFace);
}

/**
 * The face a changed card is shown by on the Patch notes page: the Radiant face when every change
 * a face prints is the Radiant face's own, else the base face (R507).
 */
export function faceShown(delta: CardDelta): FaceKey {
  if (delta.kind !== "changed") return "base";
  const printed = delta.changes.filter(printsOnFace);
  return printed.length > 0 && printed.every((change) => change.field.startsWith("radiant.")) ? "radiant" : "base";
}

/** The marked words of a text change, for tests and summaries: [added, removed]. */
export function changedWords(change: TextChange): { added: string[]; removed: string[] } {
  return {
    added: change.added.map((range) => change.after.slice(range.start, range.end)),
    removed: change.removed.map((range) => change.before.slice(range.start, range.end)),
  };
}
