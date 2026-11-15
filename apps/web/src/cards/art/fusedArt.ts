// R3045000 (issue #3045): a fused Miner or Weapon keeps its host's own art. The engine's Fuse unions the
// ingredients' tags and joins their names (crates/engine/src/subsystems/fuse.rs `build_def`), and the art
// reads a card's id, name and tags: the delivered file, the Stinkyscape check, the Legacy check, the seed,
// `themeFor`. So a Unit with a Miner or a Weapon equipped wore the weapon's or the miner's picture in
// place of its own. Here the card whose art a window draws is resolved first (`CardArt` calls it), so the
// faces that hand CardArt a fused card's id need no change. A Starship keeps its own art already, from its
// pieces and its launch (R2249000). Presentation only (CLAUDE.md rule 7): the engine's definition is whole.

import type { Tag } from "@jackioh/shared";

import { CATALOG } from "../../wire/cards.ts";
import { DORMANT_STARSHIP_ID, fusedParts, type DefOf } from "../fused.ts";
import { STINKYSCAPE_PREFIX } from "./stinkyscape.ts";

/** What an art window is drawn from: a card's id, its (English) name and its tags. */
export type ArtSubject = { defId: string; name: string | undefined; tags: readonly Tag[] };

/**
 * The Stinkyscape Miners, Miner v1 to v5: Equip cards that print no Weapon tag. `fusedArt.test.tsx` holds the
 * list to the catalog's.
 */
export const MINER_IDS: ReadonlySet<string> = new Set(
  ["062", "063", "064", "065", "066"].map((suffix) => `${STINKYSCAPE_PREFIX}${suffix}`),
);

/** The tag the Stinkyscape set's weapons carry. */
const WEAPON_TAG: Tag = "Weapon";

/**
 * R3045000: the card whose art `subject` draws. A fused definition (through `defOf`) holding a Miner or a
 * Weapon draws the card it was fused onto, the last ingredient that is neither (an Equip's host is the last
 * ingredient, play_steps.rs `fuse_equip`): that card's id, name and tags, so its picture, seed and theme.
 * Anything else, and a fusion a digest id (R468) hides, draws as it is given.
 */
export function artSubjectOf(subject: ArtSubject, defOf: DefOf | null): ArtSubject {
  if (defOf === null) return subject;
  const ids = fusedParts(subject.defId, defOf);
  if (ids === null || ids.length < 2) return subject;
  const parts = ids.map((id) => ({ id, def: defOf(id) }));
  const gear = (part: (typeof parts)[number]): boolean => MINER_IDS.has(part.id) || part.def?.tags.includes(WEAPON_TAG) === true;
  if (!parts.some(gear)) return subject;
  const host = parts.findLast((part) => !gear(part));
  return host?.def === undefined ? subject : { defId: host.id, name: host.def.name, tags: host.def.tags };
}

/**
 * R3345000, R3045000, R179: the catalog card whose picture the full-screen view draws for `defId`, or null when none
 * can be found. A fused id (`t-N:(a+b)`, nested for a fusion of fusions) is in no catalog, so its planes would all
 * be `url("")` and the view a black screen. A catalog card draws as itself; a fusion holding a Miner or a Weapon
 * draws the host it was equipped on (`artSubjectOf`); any other fusion draws its Dormant Starship hull, else its last
 * ingredient the catalog knows. An id that is no fusion is left as it is. A digest id (R468) hides its ingredients,
 * so it has no art: null.
 */
export function fullScreenArtId(defId: string, defOf: DefOf | null): string | null {
  if (CATALOG[defId] !== undefined) return defId;
  const lookup: DefOf = (id) => defOf?.(id) ?? CATALOG[id];
  const parts = fusedParts(defId, lookup);
  if (parts === null) return null;
  if (parts.length < 2) return defId;
  const host = artSubjectOf({ defId, name: undefined, tags: [] }, lookup).defId;
  if (host !== defId && CATALOG[host] !== undefined) return host;
  if (parts.includes(DORMANT_STARSHIP_ID)) return DORMANT_STARSHIP_ID;
  return parts.findLast((id) => CATALOG[id] !== undefined) ?? null;
}
