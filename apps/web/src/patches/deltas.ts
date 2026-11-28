// R5268000: a shipped snapshot is the fold of its version's delta (`crates/cards/patches/deltas/<version>.json`)
// and every one before it in patches.json's order, its cards in `order.json`'s order. Pure, and no
// `import.meta.glob` of its own, so the build plugin (`static-pages.ts`) and the GitHub-releases
// script (`scripts/github-releases.ts`, under `tsx`) fold the same history `source.ts` does.

import type { CardDef, CardDefs } from "@jackioh/shared";
import { isRecord } from "../wire/json.ts";

/** A shipped catalog: the entries by card id. */
export type Snapshot = CardDefs;

/** What one version changed from the one before it: the entries it set, whole, and the ids it removed. */
export type Delta = { readonly set: Snapshot; readonly unset: readonly string[] };

export function readDelta(value: unknown, version: string): Delta {
  if (!isRecord(value) || !isRecord(value.set) || (value.unset !== undefined && !Array.isArray(value.unset))) {
    throw new Error(`deltas/${version}.json is not a delta`);
  }
  return { set: value.set as Record<string, CardDef>, unset: (value.unset as string[] | undefined) ?? [] };
}

export function readOrder(value: unknown): readonly string[] {
  if (!Array.isArray(value) || !value.every((id) => typeof id === "string")) throw new Error("order.json is not a list of card ids");
  return value as string[];
}

/** `before` with `delta` applied, its cards in `order`'s order. */
export function applyDelta(before: Snapshot, delta: Delta, order: readonly string[], version: string): Snapshot {
  const placed = new Set(order);
  for (const id of Object.keys(delta.set)) {
    if (!placed.has(id)) throw new Error(`deltas/${version}.json sets ${id}, which has no place in order.json`);
  }
  const removed = new Set(delta.unset);
  const next: Record<string, CardDef> = {};
  for (const id of order) {
    const entry = delta.set[id] ?? (removed.has(id) ? undefined : before[id]);
    if (entry !== undefined) next[id] = entry;
  }
  return next;
}

/**
 * A synchronous reader of every shipped snapshot, each folded once. `read(stem)` is the parsed file
 * `<stem>.json` under crates/cards/patches/ (`patches`, `order`, `deltas/<version>`), or undefined
 * when there is none. A version patches.json does not list, or one with no delta, has no snapshot.
 */
export function snapshotReader(read: (stem: string) => unknown): (version: string) => Snapshot | null {
  const folded = new Map<string, Snapshot>();
  let versions: readonly string[] | undefined;
  let order: readonly string[] | undefined;
  return (version) => {
    versions ??= ((read("patches") as { version: string }[] | undefined) ?? []).map((patch) => patch.version);
    const at = versions.indexOf(version);
    if (at < 0) return null;
    let before: Snapshot = {};
    for (const shipped of versions.slice(0, at + 1)) {
      const known = folded.get(shipped);
      if (known !== undefined) {
        before = known;
        continue;
      }
      const delta = read(`deltas/${shipped}`);
      if (delta === undefined) return null;
      order ??= readOrder(read("order"));
      before = applyDelta(before, readDelta(delta, shipped), order, shipped);
      folded.set(shipped, before);
    }
    return before;
  };
}
