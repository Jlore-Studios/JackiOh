// The card patch history (SPEC §10.10, R375): when each card was created and every change to it
// since, read off the patch list and the catalog snapshots in `packages/cards/patches/`.
//
// `patches.json` lists every patch, oldest first; `snapshots/<version>.json` is catalog.json exactly
// as that version left it; and `changes.json` is `patchCards` below applied to the two, so the
// client can count a card's versions and list a patch's cards without loading any snapshot.
// `scripts/patches.ts` writes all three. A card's history is its creation entry, then one entry per
// version in which its catalog entry changed, with what changed: each field as a "before" and an
// "after" string, and each text whole, which the client word-diffs.
//
// Before v0.1.0-r3 a Radiant text was SPEC §8's Radiant cell as the catalog stored it, shorthand
// (#44 True Strike's was "9"); that pass wrote every one out in full (R277). An entry says which
// kind each Radiant text is, so the client can label shorthand and say "written out in full"
// rather than word-diff a shorthand against the text that replaced it.
//
// Pure and sync (CLAUDE.md rule 4): the caller loads the snapshots (the client with a dynamic
// import, so the main bundle does not carry them; the tests and the script with fs) and hands them
// in. The history is presentation only: nothing reads it but the client, and no game, replay or
// server reads a snapshot (CLAUDE.md rule 7).

import { keywordKey, type CardCost, type CardDef, type CardDefs, type CardFace, type Keyword } from "@jackioh/shared";

/** An issue or a pull request of the repository that a patch came from. */
export type PatchSource = { readonly kind: "issue" | "pr"; readonly number: number };

/** One entry of `patches.json`. */
export type Patch = {
  readonly version: string;
  /** The day it shipped, YYYY-MM-DD. */
  readonly date: string;
  readonly title: string;
  /**
   * The commits that made it, oldest first, as full ids. Its snapshot is catalog.json as the last
   * one left it. Empty while the patch has not shipped.
   */
  readonly commits: readonly string[];
  readonly sources: readonly PatchSource[];
  /** R375: a label given later to a state git recorded, never a name the version had at the time. */
  readonly reconstructed: boolean;
  readonly notes: string;
};

/** The catalog as each version left it, by version. */
export type Snapshots = Readonly<Record<string, CardDefs>>;

/** The version whose snapshot first writes every Radiant text out in full (R277); shorthand before it. */
export const RADIANT_TEXT_IN_FULL_SINCE = "v0.1.0-r3";

/**
 * The fields a history entry names, in the order it reports them. A card's two texts come between
 * them as `TextChange`s, each after its own face's stats and keywords; any other field, one this
 * list has no name for (`index`, `set`, `radiantFallback`, a face's field the texts and stats do
 * not cover), follows under its own key, so no change to an entry goes unreported.
 */
export const HISTORY_FIELDS = [
  "name",
  "cost",
  "type",
  "rarity",
  "tags",
  "stats",
  "keywords",
  "radiantStats",
  "radiantKeywords",
  "refs",
] as const;

export type HistoryField = (typeof HISTORY_FIELDS)[number];

/** A field that changed: its value in the version before and in this one, as the history prints them. */
export type FieldChange = {
  readonly kind: "field";
  /** A `HistoryField`, or the key of a field the list has no name for (`"index"`, `"base.loc"`). */
  readonly field: string;
  readonly before: string;
  readonly after: string;
};

/** A face's text that changed, both versions whole. */
export type TextChange = {
  readonly kind: "text";
  readonly face: "base" | "radiant";
  readonly before: string;
  readonly after: string;
  /** Whether `before` is SPEC §8's shorthand rather than the whole text. Always false on a base face. */
  readonly beforeShorthand: boolean;
  /** Whether `after` is. Always false on a base face. */
  readonly afterShorthand: boolean;
};

export type CardChange = FieldChange | TextChange;

/** One version in a card's history. */
export type HistoryEntry = {
  readonly patch: Patch;
  /** Created: the card enters the catalog. Changed: its entry differs from the version before. Removed: it leaves. */
  readonly kind: "created" | "changed" | "removed";
  /** The card as this version left it; for a removal, as it stood before. */
  readonly def: CardDef;
  /** Whether `def.radiant.text` is SPEC §8's shorthand (a version before `RADIANT_TEXT_IN_FULL_SINCE`). */
  readonly radiantShorthand: boolean;
  /** What changed, in `HISTORY_FIELDS` order; empty unless the entry is a change. */
  readonly changes: readonly CardChange[];
};

/** The cards one version created, changed and removed, each in its snapshot's order. */
export type PatchCards = {
  readonly created: readonly string[];
  readonly changed: readonly string[];
  readonly removed: readonly string[];
};

/** What the history prints for an absent value or an empty list. */
const NONE = "none";

/** A card as one version holds it, with that version's catalog, which names the cards it refers to. */
type Side = { readonly def: CardDef; readonly defs: CardDefs; readonly shorthand: boolean };

/** A field's value, compared, and the words it prints as. */
type Shown = { readonly value: unknown; readonly text: string };

/** The top-level and face fields the named fields and the texts cover; every other key is reported by name. */
const NAMED_KEYS: ReadonlySet<string> = new Set(["id", "name", "cost", "type", "rarity", "tags", "refs", "base", "radiant"]);
const NAMED_FACE_KEYS: ReadonlySet<string> = new Set(["attack", "health", "keywords", "text"]);

/** A value as a string that is the same for equal JSON, whatever the order of an object's keys. */
function canonical(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value !== null && typeof value === "object") {
    const keys = Object.keys(value)
      .filter((key) => (value as Record<string, unknown>)[key] !== undefined)
      .sort();
    return `{${keys.map((key) => `${JSON.stringify(key)}:${canonical((value as Record<string, unknown>)[key])}`).join(",")}}`;
  }
  return value === undefined ? "undefined" : JSON.stringify(value);
}

function listed(items: readonly string[]): string {
  return items.length === 0 ? NONE : items.join(", ");
}

/** A printed cost: a number, "X", or an embiggen card's base price and its bigger one. */
function costText(cost: CardCost): string {
  if (typeof cost === "number") return String(cost);
  if (cost === "X") return cost;
  return `${String(cost.base)} (Paid ${String(cost.embiggen)})`;
}

function statsOf(face: CardFace): Shown {
  const value = [face.attack, face.health];
  if (face.attack === undefined && face.health === undefined) return { value, text: NONE };
  return { value, text: `${String(face.attack ?? NONE)}/${String(face.health ?? NONE)}` };
}

function keywordsOf(keywords: readonly Keyword[]): Shown {
  return { value: keywords, text: listed(keywords.map(keywordKey)) };
}

/** R279's `refs`, by the names that version gave the cards they point at. */
function refsOf(side: Side): Shown {
  const refs = side.def.refs ?? [];
  return { value: refs, text: listed(refs.map((id) => side.defs[id]?.name ?? id)) };
}

function jsonOf(value: unknown): Shown {
  return { value, text: value === undefined ? NONE : JSON.stringify(value) };
}

const NAMED: Readonly<Record<HistoryField, (side: Side) => Shown>> = {
  name: ({ def }) => ({ value: def.name, text: def.name }),
  cost: ({ def }) => ({ value: def.cost, text: costText(def.cost) }),
  type: ({ def }) => ({ value: def.type, text: def.type }),
  rarity: ({ def }) => ({ value: def.rarity, text: def.rarity }),
  tags: ({ def }) => ({ value: def.tags, text: listed(def.tags) }),
  stats: ({ def }) => statsOf(def.base),
  keywords: ({ def }) => keywordsOf(def.base.keywords),
  radiantStats: ({ def }) => statsOf(def.radiant),
  radiantKeywords: ({ def }) => keywordsOf(def.radiant.keywords),
  refs: refsOf,
};

/** The keys of two records the named fields do not cover, in order. */
function otherKeys(a: object, b: object, named: ReadonlySet<string>): string[] {
  return [...new Set([...Object.keys(a), ...Object.keys(b)])].filter((key) => !named.has(key)).sort();
}

/** Everything that differs between one card's entry in two versions, in `HISTORY_FIELDS` order. */
function cardChanges(before: Side, after: Side): CardChange[] {
  const changes: CardChange[] = [];
  const field = (name: string, read: (side: Side) => Shown): void => {
    const a = read(before);
    const b = read(after);
    if (canonical(a.value) !== canonical(b.value)) changes.push({ kind: "field", field: name, before: a.text, after: b.text });
  };
  const text = (face: "base" | "radiant"): void => {
    const a = before.def[face].text;
    const b = after.def[face].text;
    if (a === b) return;
    const radiant = face === "radiant";
    changes.push({ kind: "text", face, before: a, after: b, beforeShorthand: radiant && before.shorthand, afterShorthand: radiant && after.shorthand });
  };

  for (const name of ["name", "cost", "type", "rarity", "tags", "stats", "keywords"] as const) field(name, NAMED[name]);
  text("base");
  for (const name of ["radiantStats", "radiantKeywords"] as const) field(name, NAMED[name]);
  text("radiant");
  field("refs", NAMED.refs);
  for (const key of otherKeys(before.def, after.def, NAMED_KEYS)) {
    field(key, ({ def }) => jsonOf((def as unknown as Record<string, unknown>)[key]));
  }
  for (const face of ["base", "radiant"] as const) {
    for (const key of otherKeys(before.def[face], after.def[face], NAMED_FACE_KEYS)) {
      field(`${face}.${key}`, ({ def }) => jsonOf((def[face] as unknown as Record<string, unknown>)[key]));
    }
  }
  return changes;
}

function snapshotOf(snapshots: Snapshots, version: string): CardDefs {
  const defs = snapshots[version];
  if (defs === undefined) {
    throw new Error(`no snapshot for ${version} (packages/cards/patches/snapshots/${version}.json)`);
  }
  return defs;
}

/** Whether the version at `at` stored Radiant texts as SPEC §8's shorthand. */
function shorthandAt(patches: readonly Patch[], at: number): boolean {
  const since = patches.findIndex((patch) => patch.version === RADIANT_TEXT_IN_FULL_SINCE);
  return since !== -1 && at < since;
}

/** What one version did to one card, given how the version before and this one hold it; null for nothing. */
function step(
  patch: Patch,
  previous: Side | undefined,
  current: Side | undefined,
): HistoryEntry | null {
  if (current !== undefined && previous === undefined) {
    return { patch, kind: "created", def: current.def, radiantShorthand: current.shorthand, changes: [] };
  }
  if (current === undefined && previous !== undefined) {
    return { patch, kind: "removed", def: previous.def, radiantShorthand: previous.shorthand, changes: [] };
  }
  if (current === undefined || previous === undefined) return null;
  const changes = cardChanges(previous, current);
  return changes.length === 0 ? null : { patch, kind: "changed", def: current.def, radiantShorthand: current.shorthand, changes };
}

/** Each version's catalog with whether it stored Radiant shorthand, oldest first. */
function versionsOf(patches: readonly Patch[], snapshots: Snapshots): { patch: Patch; defs: CardDefs; shorthand: boolean }[] {
  return patches.map((patch, at) => ({ patch, defs: snapshotOf(snapshots, patch.version), shorthand: shorthandAt(patches, at) }));
}

/**
 * R375: one card's history, oldest first: the version that created it, then each version in which
 * its entry changed (or that removed it), with what changed. A card no snapshot holds has none.
 * Throws when a patch has no snapshot.
 */
export function cardHistory(id: string, patches: readonly Patch[], snapshots: Snapshots): HistoryEntry[] {
  const entries: HistoryEntry[] = [];
  let previous: Side | undefined;
  for (const { patch, defs, shorthand } of versionsOf(patches, snapshots)) {
    const def = defs[id];
    const current = def === undefined ? undefined : { def, defs, shorthand };
    const entry = step(patch, previous, current);
    if (entry !== null) entries.push(entry);
    previous = current;
  }
  return entries;
}

/**
 * R375: the cards each version created, changed and removed, by version — `changes.json`. The same
 * steps as `cardHistory`, so a card is listed under exactly the versions its history has.
 */
export function patchCards(patches: readonly Patch[], snapshots: Snapshots): Record<string, PatchCards> {
  const out: Record<string, PatchCards> = {};
  let previous: { defs: CardDefs; shorthand: boolean } = { defs: {}, shorthand: false };
  for (const { patch, defs, shorthand } of versionsOf(patches, snapshots)) {
    const created: string[] = [];
    const changed: string[] = [];
    for (const [id, def] of Object.entries(defs)) {
      const before = previous.defs[id];
      const entry = step(
        patch,
        before === undefined ? undefined : { def: before, defs: previous.defs, shorthand: previous.shorthand },
        { def, defs, shorthand },
      );
      if (entry?.kind === "created") created.push(id);
      if (entry?.kind === "changed") changed.push(id);
    }
    const removed = Object.keys(previous.defs).filter((id) => defs[id] === undefined);
    out[patch.version] = { created, changed, removed };
    previous = { defs, shorthand };
  }
  return out;
}

/**
 * The versions a card's history has, oldest first, read off `changes.json` without a snapshot:
 * the count the client labels a card's History with before it loads one.
 */
export function historyVersions(id: string, patches: readonly Patch[], changes: Readonly<Record<string, PatchCards>>): string[] {
  return patches
    .filter((patch) => {
      const cards = changes[patch.version];
      return cards !== undefined && (cards.created.includes(id) || cards.changed.includes(id) || cards.removed.includes(id));
    })
    .map((patch) => patch.version);
}

/** The newest patch: the current version. Throws on an empty list. */
export function newestPatch(patches: readonly Patch[]): Patch {
  const newest = patches[patches.length - 1];
  if (newest === undefined) throw new Error("packages/cards/patches/patches.json lists no patch");
  return newest;
}
