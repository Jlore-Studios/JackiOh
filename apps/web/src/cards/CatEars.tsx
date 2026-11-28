// R5264500: the cat ears a granted Felinor tag puts on a card (Felinors vs Humans #64 Fiender Artist's discovered
// Unit, #68 Closeted Commentator's Evolve, #79 Post Human Aesthetist's Human): the scene kit's own "cat-ears" part
// (art/sceneKitInkCharacters.ts), standing on the card's top edge, on the board and in a hand alike. It holds still,
// takes no pointer and says nothing: the tags line already names the tag. Presentation only (CLAUDE.md rule 7): it
// reads the view's tags and the definition's printed ones, never a rule.

import { memo, useMemo, type ReactElement } from "react";

import type { Tag } from "@jackioh/shared";

import { drawKitPart, type PartPlace } from "./art/scene.ts";

/** The tag whose grant draws the ears. */
export const CAT_EARS_TAG: Tag = "Felinor";

/** The ears drawn about their own anchor: the part is about 16 wide and 8 tall at scale 1. */
const EARS_PLACE: PartPlace = { x: 0, y: 0, s: 1, flip: 1 };
const EARS_VIEW_BOX = "-9 -8.4 18 9";
/** The outline the kit's inked shapes take, as a scene's ink line is drawn. */
const EARS_INK = "#0b0d12";
const EARS_INK_WIDTH = 0.35;

/**
 * Whether a card wears granted cat ears: the view lists the Felinor tag for it (`tags`, given only where they
 * differ from its definition's, R923) and the definition it is drawn from does not print it. With no
 * definition to read, it wears none.
 */
export function wearsCatEars(viewTags: readonly Tag[] | undefined, printedTags: readonly Tag[] | undefined): boolean {
  return viewTags?.includes(CAT_EARS_TAG) === true && printedTags !== undefined && !printedTags.includes(CAT_EARS_TAG);
}

/** The ears, drawn from `seed`'s own stream (the card's instance), so one card's twitch is the same on every render. */
export const CatEars = memo(function CatEars({ seed }: { seed: string }): ReactElement | null {
  const shapes = useMemo(() => drawKitPart("cat-ears", EARS_PLACE, seed), [seed]);
  if (shapes.length === 0) return null;
  return (
    <span className="cf-cat-ears" data-granted-tag={CAT_EARS_TAG} aria-hidden="true">
      <svg viewBox={EARS_VIEW_BOX} preserveAspectRatio="xMidYMax meet" aria-hidden="true" focusable="false">
        {shapes.map((shape) =>
          shape.inked ? (
            <path key={shape.d} d={shape.d} fill={shape.fill} fillOpacity={shape.opacity} stroke={EARS_INK} strokeWidth={EARS_INK_WIDTH} strokeLinejoin="round" />
          ) : (
            <path key={shape.d} d={shape.d} fill={shape.fill} fillOpacity={shape.opacity} />
          ),
        )}
      </svg>
    </span>
  );
});
