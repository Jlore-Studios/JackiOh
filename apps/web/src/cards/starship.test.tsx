// R2249000: a Starship's face in play (#2249) wears the Starship's own art, drawn from its pieces and its
// launch, whatever definition its fusions gave it; its rules box prints only the Starship term and the
// pieces fused into it, each a reference that opens the piece's card.

import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { CardArt } from "./art/CardArt.tsx";
import { starshipSvg } from "./art/stinkyscape.ts";
import { starshipArtUri } from "./art/svg.ts";
import { CardFace } from "./CardFace.tsx";
import { faceModel } from "./model.ts";
import { CardDefsProvider } from "./refContext.tsx";
import { STARSHIP_PIECES_TESTID } from "./StarshipPieces.tsx";

afterEach(cleanup);

const HULL = "stinkyscape-t-dormant-starship";
const SHIELD = "stinkyscape-038";
const CAPSULE = "stinkyscape-043";

function def(id: string): CardDef {
  const found = CATALOG[id];
  if (found === undefined) throw new Error(`no ${id}`);
  return found;
}

function art(defId: string, starship: { pieces: number; launched: boolean } | null): HTMLElement {
  const card = def(defId);
  const { container } = render(<CardArt defId={defId} radiant={false} tags={card.tags} type={card.type} shape="oval" starship={starship} />);
  const found = container.querySelector<HTMLElement>(".cf-art");
  if (found === null) throw new Error("no art");
  return found;
}

describe("R2249000: a Starship's face", () => {
  it("R2249000 a Starship wears its own art, the same whatever definition its fusions gave it", () => {
    const hull = art(HULL, { pieces: 3, launched: false });
    expect(hull.getAttribute("data-art-family")).toBe("starship");
    const hullArt = hull.style.backgroundImage;
    cleanup();
    // Fused, its definition is not the hull's; its art stays the Starship's.
    const fused = art("core-008", { pieces: 3, launched: false });
    expect(fused.getAttribute("data-art-family")).toBe("starship");
    expect(fused.style.backgroundImage).toBe(hullArt);
    expect(hullArt).toContain(starshipArtUri(3, false, false));
  });

  it("R2249000 the art grows with its pieces and lights up once it launches", () => {
    const dormant = starshipSvg(2, false, false);
    expect(starshipSvg(5, false, false)).not.toBe(dormant);
    expect(starshipSvg(2, true, false)).not.toBe(dormant);
    expect(starshipSvg(2, false, true)).not.toBe(dormant);
    expect(starshipSvg(2, false, false)).toBe(dormant);
  });

  it("R2249000 a card that is no Starship keeps its own art", () => {
    expect(art(SHIELD, null).getAttribute("data-art-family")).not.toBe("starship");
  });

  it("R2249000 in play a Starship prints no text, only the Starship term and its pieces, each a card reference", () => {
    const pieces = [
      { defId: SHIELD, radiant: false },
      { defId: CAPSULE, radiant: true },
    ];
    const face = faceModel({
      defId: HULL,
      def: def(HULL),
      radiant: false,
      inPlay: { starship: { pieces: pieces.length, name: null, list: pieces } },
    });
    expect(face.text.full).toBe("");
    const { container } = render(
      <CardDefsProvider defs={CATALOG}>
        <CardFace face={face} layout="full" />
      </CardDefsProvider>,
    );
    expect(container.querySelector(".cf-text-base")).toBeNull();
    const list = screen.getByTestId(STARSHIP_PIECES_TESTID);
    expect(list.getAttribute("data-pieces")).toBe("2");
    expect(within(list).getByText(def(SHIELD).name)).toBeTruthy();
    expect(within(list).getByText(def(CAPSULE).name)).toBeTruthy();
    expect(list.querySelector('[data-term="Starship"]')).not.toBeNull();
    expect([...list.querySelectorAll("[data-piece]")].map((node) => node.getAttribute("data-piece"))).toEqual([SHIELD, CAPSULE]);
    // The Starship's printed text is nowhere on the face.
    expect(container.textContent).not.toContain("launch this");
  });

  it("R2249000 out of play, in the collection, the Dormant Starship prints its short text", () => {
    const face = faceModel({ defId: HULL, def: def(HULL), radiant: false });
    expect(face.text.full).toBe("Dormant until it launches. Your Starship Pieces that die fuse into this.\nActivate: Pay (3) to launch this.");
  });
});
