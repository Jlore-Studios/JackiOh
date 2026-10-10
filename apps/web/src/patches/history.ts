// Patch history uses patches.json order, never version-string sorting (R105 as B4.2 rewrites it).

import { SHIPPED_SETS, type CardDef, type SetName } from "@jackioh/shared";

import { isGlitch } from "../cards/glitch.ts";
import { dataOnly, diffCard, type CardDelta } from "./diff.ts";
import type { HistoryIndex, Patch, Snapshot } from "./source.ts";

export function newestFirst(patches: readonly Patch[]): Patch[] {
  return [...patches].reverse();
}

export function versionBefore(patches: readonly Patch[], version: string): string | null {
  const at = patches.findIndex((patch) => patch.version === version);
  return at <= 0 ? null : (patches[at - 1]?.version ?? null);
}

export type AddedGroup = {
  readonly label: string;
  readonly set: SetName;
  readonly token: boolean;
  readonly cards: readonly CardDef[];
};

/** Patch-page cards (R507). */
export type PatchCards = {
  readonly changed: readonly Extract<CardDelta, { kind: "changed" }>[];
  readonly dataOnly: readonly Extract<CardDelta, { kind: "changed" }>[];
  readonly added: readonly AddedGroup[];
  readonly removed: readonly CardDef[];
};

function setRank(set: SetName): number {
  const at = (SHIPPED_SETS as readonly SetName[]).indexOf(set);
  return at < 0 ? SHIPPED_SETS.length : at;
}

/** R388: diff recorded cards from the preceding snapshot; omit identical cards. */
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

export function nameMatches(name: string, query: string): boolean {
  const words = query.toLowerCase().split(/\s+/).filter((word) => word.length > 0);
  const haystack = name.toLowerCase();
  return words.every((word) => haystack.includes(word));
}

export type HistoryEntry = {
  readonly patch: Patch;
  readonly delta: CardDelta;
  readonly def: CardDef;
};

export function versionsForCard(id: string, patches: readonly Patch[], index: HistoryIndex): string[] {
  const needed = new Set<string>();
  for (const version of index[id] ?? []) {
    needed.add(version);
    const before = versionBefore(patches, version);
    if (before !== null) needed.add(before);
  }
  return patches.map((patch) => patch.version).filter((version) => needed.has(version));
}

/** R388: list indexed changes newest first, skipping missing snapshots. */
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

export function unchangedSince(entries: readonly HistoryEntry[]): string | null {
  const only = entries.length === 1 ? entries[0] : undefined;
  return only !== undefined && only.delta.kind === "added" ? only.patch.version : null;
}

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
