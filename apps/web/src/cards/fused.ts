// What a fused definition is made of, for how its card looks (R3045000, issue #3045). The engine's Fuse
// (crates/engine/src/subsystems/fuse.rs `build_def`) unions the ingredients' tags and joins their names, so
// a card that took an Equip, or a Starship that took a piece, is named and tagged as all of them. Which
// ingredient a fused card is drawn as, and what a fused Dormant Starship is called, are the client's:
// `cards/art/fusedArt.ts` and `cards/model.ts`, `chinese.ts` and `game/Log.tsx`. The engine's names and
// tags stay as they are, since a rule can read them (R2529000). Presentation only (CLAUDE.md rule 7).

import type { CardDef } from "@jackioh/shared";

/** A card's definition by id, wherever the reader finds it (`useDefResolver`, a catalog lookup). */
export type DefOf = (id: string) => CardDef | undefined;

/** The Dormant Starship's catalog id (Stinkyscape, R1852000); `cards/fused.test.ts` holds both to the catalog's. */
export const DORMANT_STARSHIP_ID = "stinkyscape-t-dormant-starship";
export const DORMANT_STARSHIP_NAME = "Dormant Starship";

/** `/^t-\d+:/`, the head of a fused id (`catalog.rs` `fused_head_len`, R179). */
const FUSED_HEAD = /^t-\d+:/;
/** R468: a fused id past the cap spells a digest of its ingredients after its head, not their ids. */
const DIGEST_MARK = "#";
/** R469: the mark an ingredient that went in on its Radiant face carries in a readable id. */
const RADIANT_MARK = "*";
const OPEN = "(";
const CLOSE = ")";
const JOIN = "+";

/** An ingredient as a fused id spells it (R179: a fused one in parentheses, R469: a Radiant one marked), as its id. */
function bareId(spelled: string): string {
  const unmarked = spelled.endsWith(RADIANT_MARK) ? spelled.slice(0, -RADIANT_MARK.length) : spelled;
  return unmarked.startsWith(OPEN) && unmarked.endsWith(CLOSE) ? unmarked.slice(OPEN.length, -CLOSE.length) : unmarked;
}

/**
 * R179, R468: the ingredient ids a readable fused id spells, split at its top-level `+` signs; null for a
 * digest, which spells none.
 */
function spelledIngredients(id: string): string[] | null {
  const body = id.replace(FUSED_HEAD, "");
  if (body.startsWith(DIGEST_MARK)) return null;
  const spelled: string[] = [];
  let depth = 0;
  let from = 0;
  for (let at = 0; at <= body.length; at++) {
    const mark = body.charAt(at);
    if (mark === OPEN) depth++;
    else if (mark === CLOSE) depth--;
    else if (at === body.length || (mark === JOIN && depth === 0)) {
      spelled.push(bareId(body.slice(from, at)));
      from = at + JOIN.length;
    }
  }
  return spelled;
}

/**
 * The catalog cards a definition is made of, in ingredient order, a fusion of fusions walked to its
 * leaves: `[id]` for a card that is no fusion. A definition lists its ingredients (R468); one the reader
 * does not hold (the view carries the definitions its cards name, R243, not the ones inside them) is read
 * from its readable id (R179). Null when a digest hides some of them (R468).
 */
export function fusedParts(id: string, defOf: DefOf, inside: ReadonlySet<string> = new Set()): string[] | null {
  const listed = defOf(id)?.ingredients?.map((ingredient) => ingredient.defId);
  const ingredients = listed !== undefined && listed.length > 0 ? listed : FUSED_HEAD.test(id) ? spelledIngredients(id) : undefined;
  if (ingredients === undefined) return [id];
  if (ingredients === null || inside.has(id)) return null;
  const parts: string[] = [];
  for (const ingredient of ingredients) {
    const leaves = fusedParts(ingredient, defOf, new Set(inside).add(id));
    if (leaves === null) return null;
    parts.push(...leaves);
  }
  return parts;
}

/** Whether `def` is a fusion with the Dormant Starship's hull among its ingredients, in any depth. */
export function holdsDormantStarship(def: CardDef, defOf: DefOf = () => undefined): boolean {
  const parts = fusedParts(def.id, (id) => (id === def.id ? def : defOf(id)));
  return parts !== null && parts.length > 1 && parts.includes(DORMANT_STARSHIP_ID);
}
