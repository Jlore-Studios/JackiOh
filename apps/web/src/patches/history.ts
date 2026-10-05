// The patch history as the two screens read it (brief B4.2, R388, R507): a patch's cards sorted
// into what the Patch notes page shows, and one card's history, newest first. Pure: the caller
// loads the snapshots (source.ts) and hands them in.
//
// Patch order is patches.json's order and nothing else (R105 as B4.2 rewrites it): "the version
// before" is the entry before in that array, never a version string compared with another.

import { SHIPPED_SETS, type CardDef, type SetName } from "@jackioh/shared";

import { isGlitch } from "../cards/glitch.ts";
import { dataOnly, diffCard, type CardDelta } from "./diff.ts";
import type { HistoryIndex, Patch, Snapshot } from "./source.ts";

/** The patches, newest first, as both screens list them. */
export function newestFirst(patches: readonly Patch[]): Patch[] {
  return [...patches].reverse();
}

/** The version of the patch before `version` in patch order; null for the first, or an unknown one. */
export function versionBefore(patches: readonly Patch[], version: string): string | null {
  const at = patches.findIndex((patch) => patch.version === version);
  return at <= 0 ? null : (patches[at - 1]?.version ?? null);
}

/** A patch's added cards of one set, cards and tokens apart (R507). */
export type AddedGroup = {
  /** "Classic+ tokens": the set, and "tokens" for the set's tokens. */
  readonly label: string;
  readonly set: SetName;
  readonly token: boolean;
  readonly cards: readonly CardDef[];
};

/** A patch's cards, as the Patch notes page shows them (R507). */
export type PatchCards = {
  /** Changed cards whose faces print the change: shown as faces. */
  readonly changed: readonly Extract<CardDelta, { kind: "changed" }>[];
  /** Changed cards whose every change lies in data a face does not print: listed by name. */
  readonly dataOnly: readonly Extract<CardDelta, { kind: "changed" }>[];
  /** Added cards, grouped by set, each set's cards before its tokens, sets in catalog order. */
  readonly added: readonly AddedGroup[];
  /** Removed cards, as the snapshot before the patch held them. */
  readonly removed: readonly CardDef[];
};

/** Catalog order of sets: the shipped ones first (SHIPPED_SETS), any other after. */
function setRank(set: SetName): number {
  const at = (SHIPPED_SETS as readonly SetName[]).indexOf(set);
  return at < 0 ? SHIPPED_SETS.length : at;
}

/**
 * R388: every card `patch` records, diffed between `before` (the snapshot of the patch before it,
 * null for the first patch) and `after` (this patch's own snapshot). A card the two snapshots hold
 * identically is left out: there is nothing to show for it.
 */
export function patchCards(patch: Patch, after: Snapshot, before: Snapshot | null): PatchCards {
  const changed: Extract<CardDelta, { kind: "changed" }>[] = [];
  const quiet: Extract<CardDelta, { kind: "changed" }>[] = [];
  const removed: CardDef[] = [];
  const groups = new Map<string, { set: SetName; token: boolean; cards: CardDef[] }>();
  for (const change of patch.changes) {
    // R674: Glitch is never listed, though a patch records it.
    if (isGlitch(change.id)) continue;
    const delta = diffCard(before?.[change.id], after[change.id], { before: before ?? {}, after });
    if (delta === null) continue;
    if (delta.kind === "changed") {
      (dataOnly(delta) ? quiet : changed).push(delta);
    } else if (delta.kind === "removed") {
      removed.push(delta.def);
    } else {
      const key = `${delta.def.set}\u0000${delta.def.token ? "token" : "card"}`;
      const group = groups.get(key) ?? { set: delta.def.set, token: delta.def.token, cards: [] };
      group.cards.push(delta.def);
      groups.set(key, group);
    }
  }
  const added = [...groups.values()]
    .sort((a, b) => setRank(a.set) - setRank(b.set) || (a.set < b.set ? -1 : a.set > b.set ? 1 : 0) || Number(a.token) - Number(b.token))
    .map((group) => ({ ...group, label: group.token ? `${group.set} tokens` : group.set }));
  return { changed, dataOnly: quiet, added, removed };
}

/** Whether a card's name matches a filter: every word of the query, case aside, in any order. */
export function nameMatches(name: string, query: string): boolean {
  const words = query.toLowerCase().split(/\s+/).filter((word) => word.length > 0);
  const haystack = name.toLowerCase();
  return words.every((word) => haystack.includes(word));
}

/** One patch in one card's history. */
export type HistoryEntry = {
  readonly patch: Patch;
  readonly delta: CardDelta;
  /** The card as this patch left it; for a removal, as it stood before the patch removed it. */
  readonly def: CardDef;
};

/** The versions a card's history reads: each version it changed in and the version before each. */
export function versionsForCard(id: string, patches: readonly Patch[], index: HistoryIndex): string[] {
  const needed = new Set<string>();
  for (const version of index[id] ?? []) {
    needed.add(version);
    const before = versionBefore(patches, version);
    if (before !== null) needed.add(before);
  }
  // In patch order, so the loads go out oldest first.
  return patches.map((patch) => patch.version).filter((version) => needed.has(version));
}

/**
 * R388: one card's history, newest first: for each version the index lists for it, the patch and
 * what that patch did to the card, diffed against the snapshot of the patch before. `snapshots`
 * holds every version `versionsForCard` names; a version whose snapshot is missing is skipped.
 */
export function cardHistory(
  id: string,
  patches: readonly Patch[],
  index: HistoryIndex,
  snapshots: ReadonlyMap<string, Snapshot | null>,
): HistoryEntry[] {
  const listed = new Set(index[id] ?? []);
  const entries: HistoryEntry[] = [];
  for (const patch of patches) {
    if (!listed.has(patch.version)) continue;
    const after = snapshots.get(patch.version) ?? null;
    if (after === null) continue;
    const beforeVersion = versionBefore(patches, patch.version);
    const before = beforeVersion === null ? null : (snapshots.get(beforeVersion) ?? null);
    const delta = diffCard(before?.[id], after[id], { before: before ?? {}, after });
    if (delta === null) continue;
    entries.push({ patch, delta, def: delta.def });
  }
  return entries.reverse();
}

/**
 * The version a card has stood unchanged since, when its whole history is the patch that added it
 * (brief B4.2: "Unchanged since v0.2.0"); null when a later patch changed it.
 */
export function unchangedSince(entries: readonly HistoryEntry[]): string | null {
  const only = entries.length === 1 ? entries[0] : undefined;
  return only !== undefined && only.delta.kind === "added" ? only.patch.version : null;
}

/**
 * The card as it stands now: from the newest version the index lists for it, or, when that version
 * removed it, from the snapshot before. The versions are read newest first, so the loader is asked
 * for as few snapshots as it takes.
 */
export async function currentDef(
  id: string,
  patches: readonly Patch[],
  index: HistoryIndex,
  snapshot: (version: string) => Promise<Snapshot | null>,
): Promise<CardDef | null> {
  const versions = [...(index[id] ?? [])].reverse();
  for (const version of versions) {
    const found = (await snapshot(version))?.[id];
    if (found !== undefined) return found;
    const before = versionBefore(patches, version);
    const earlier = before === null ? undefined : (await snapshot(before))?.[id];
    if (earlier !== undefined) return earlier;
  }
  return null;
}
