// R1827000: the hero portraits (R641) an account owns, which the deck builder's picker offers. Every
// portrait is an account cosmetic, `portrait-<id>` in the registry; `vanilla` is a starter (§14.2).
// The client reads `GET /api/cosmetics` and decides nothing (CLAUDE.md rule 7). `routes/decks.tsx`
// mounts `PortraitsOwned` while ownership decides (`FULL_ART_GATED`, R1310004).

import { createContext, createElement, useContext, type ReactElement, type ReactNode } from "react";

import { PORTRAIT_IDS, isPortraitId, type PortraitId } from "@jackioh/shared";

import { COSMETICS } from "../wire/generated/cosmetics.ts";

/** What a portrait's registry id starts with; the rest is its `PortraitId`. */
const PORTRAIT_COSMETIC_PREFIX = "portrait-";

export function portraitCosmeticId(portrait: PortraitId): string {
  return `${PORTRAIT_COSMETIC_PREFIX}${portrait}`;
}

/** The portrait a registry id names, else null: another kind's id, or a portrait this client does not know. */
export function portraitOfCosmetic(id: string): PortraitId | null {
  if (!id.startsWith(PORTRAIT_COSMETIC_PREFIX)) return null;
  const portrait = id.slice(PORTRAIT_COSMETIC_PREFIX.length);
  return isPortraitId(portrait) ? portrait : null;
}

/** The registry ids every account owns with no row: the starters. */
const STARTER_IDS: ReadonlySet<string> = new Set(COSMETICS.filter((cosmetic) => cosmetic.starter).map((cosmetic) => cosmetic.id));

/** The portraits an account owns, in roster order: the starters, and those `account` holds. */
export function ownedPortraits(account: readonly string[]): readonly PortraitId[] {
  return PORTRAIT_IDS.filter((portrait) => {
    const id = portraitCosmeticId(portrait);
    return STARTER_IDS.has(id) || account.includes(id);
  });
}

/** What a picker below a `PortraitsOwned` offers from: `"every"` portrait, or the owned ones. */
export type PortraitOwnership = "every" | readonly PortraitId[];

const PortraitOwnershipContext = createContext<PortraitOwnership>("every");

/** The pickers below it offer the portraits `owned` holds. */
export function PortraitsOwned({ owned, children }: { owned: PortraitOwnership; children?: ReactNode }): ReactElement {
  return createElement(PortraitOwnershipContext.Provider, { value: owned }, children);
}

/** The portraits a picker offers, in roster order: every one, else the owned ones and `current`, the deck's own pick, so an editor never hides what it wears. */
export function useOfferedPortraits(current: PortraitId): readonly PortraitId[] {
  const owned = useContext(PortraitOwnershipContext);
  if (owned === "every") return PORTRAIT_IDS;
  return PORTRAIT_IDS.filter((portrait) => portrait === current || owned.includes(portrait));
}
