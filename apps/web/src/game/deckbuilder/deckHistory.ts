// R2100000: a saved deck's kept versions, as the History popover lists them. The server keeps them
// and decides what a version is (CLAUDE.md rule 7); this module reads its answer and says, for the
// eye, what each version changed since the one before it.

import type { DeckVersion } from "../../net/api.ts";
import { isRecord } from "../../wire/json.ts";

export type VersionChange = { added: string[]; removed: string[]; renamedFrom: string | null };

function versionFrom(value: unknown): DeckVersion | null {
  if (!isRecord(value)) return null;
  const { id, name, cards, savedAt } = value;
  if (typeof id !== "string" || id.length === 0 || typeof name !== "string") return null;
  if (!Array.isArray(cards) || !cards.every((card) => typeof card === "string")) return null;
  if (typeof savedAt !== "number" || !Number.isFinite(savedAt)) return null;
  return { id, name, cards: [...cards], savedAt };
}

/** The `versions` of a server answer, in its order (newest first); a malformed entry is left out. */
export function deckVersionsFrom(value: unknown): DeckVersion[] {
  const versions = isRecord(value) && Array.isArray(value.versions) ? value.versions : [];
  return versions.flatMap((entry) => {
    const version = versionFrom(entry);
    return version === null ? [] : [version];
  });
}

/** `from`'s cards with one copy of each of `take`'s taken out, in `from`'s order. */
function without(from: readonly string[], take: readonly string[]): string[] {
  const left = new Map<string, number>();
  for (const id of take) left.set(id, (left.get(id) ?? 0) + 1);
  return from.filter((id) => {
    const count = left.get(id) ?? 0;
    if (count === 0) return true;
    left.set(id, count - 1);
    return false;
  });
}

/** What `version` changed since `older`, copy by copy; null for the oldest version kept. */
export function versionChange(version: DeckVersion, older: DeckVersion | undefined): VersionChange | null {
  if (older === undefined) return null;
  return {
    added: without(version.cards, older.cards),
    removed: without(older.cards, version.cards),
    renamedFrom: older.name === version.name ? null : older.name,
  };
}
