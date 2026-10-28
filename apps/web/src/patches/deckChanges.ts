// R2097000: what the newest shipped patch changed of a deck. A deck whose latest save predates
// the newest patch wears a badge when it holds a card that patch changed; the badge opens exactly
// those cards, each with its change. Presentation only (CLAUDE.md rule 7).

import { patchCards, versionBefore } from "./history.ts";
import type { CardDelta } from "./diff.ts";
import type { Patch, PatchSource, Snapshot } from "./source.ts";

export type ChangedCard = Extract<CardDelta, { kind: "changed" }>;

export type NewestChanges = { readonly version: string; readonly cards: ReadonlyMap<string, ChangedCard> };

/** The newest patch's changed cards, printed or only in unprinted data, each once, in patch order. */
export function newestChanges(patch: Patch, after: Snapshot, before: Snapshot | null): NewestChanges {
  const { changed, dataOnly } = patchCards(patch, after, before);
  return { version: patch.version, cards: new Map([...changed, ...dataOnly].map((delta) => [delta.id, delta])) };
}

export type DeckRef = { readonly cards: readonly string[]; readonly catalogVersion?: string };

/** The newest patch's changed cards this deck holds, each once, in patch order. */
export function deckChanges(deck: DeckRef, changes: NewestChanges): ChangedCard[] {
  if (deck.catalogVersion === changes.version) return [];
  const held = new Set(deck.cards);
  return [...changes.cards.values()].filter((delta) => held.has(delta.id));
}

/**
 * Loads the newest patch's changes, but only while that patch is the catalog the builder shows:
 * the badge describes only the patch the builder's catalog is.
 */
export async function loadNewestChanges(source: PatchSource, catalogVersion: string): Promise<NewestChanges | null> {
  const patches = await source.patches();
  const newest = patches.at(-1);
  if (newest === undefined || newest.version !== catalogVersion) return null;
  const beforeVersion = versionBefore(patches, newest.version);
  const [after, before] = await Promise.all([
    source.snapshot(newest.version),
    beforeVersion === null ? Promise.resolve(null) : source.snapshot(beforeVersion),
  ]);
  if (after === null) return null;
  return newestChanges(newest, after, before);
}
