// R1017000: a face that prints less than its whole text has the whole text on its left in the hover
// preview, under it in the touch sheet, and at reading size in the detail view; a face that prints
// its whole text has no such panel (issue #1017, Extended.tsx).

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import PoolGrid from "../../game/deckbuilder/PoolGrid.tsx";
import { poolCardId } from "../../game/deckbuilder/testids.ts";
import { faceModel, type FaceModel } from "../model.ts";
import { ASCENT_LABEL, SUMMIT_DEF_ID } from "../shortText.ts";
import { CardDetail } from "./CardDetail.tsx";
import { HOVER_DELAY_MS } from "./constants.ts";
import { HoverPreview } from "./HoverPreview.tsx";
import { InspectSheet } from "./InspectSheet.tsx";
import { closeInspect } from "./store.ts";
import { INSPECT_DETAIL, INSPECT_EXTENDED, INSPECT_FACE, INSPECT_HOVER, INSPECT_SHEET } from "./testids.ts";

const ANCHOR = { left: 600, top: 200, width: 100, height: 140 };

function def(id: string): CardDef {
  const found = CATALOG[id];
  if (found === undefined) throw new Error(`no ${id}`);
  return found;
}

function collection(id: string, radiant = false): FaceModel {
  return faceModel({ defId: id, def: def(id), radiant });
}

afterEach(() => {
  closeInspect();
  cleanup();
  vi.useRealTimers();
});

describe("R1017000 the extended description", () => {
  it("R1017000 a hovered Fortify Mind prints its short form, and its whole text stands on the face's left", () => {
    const face = collection("meditative-022-1");
    render(<HoverPreview face={face} anchor={ANCHOR} />);
    const hover = screen.getByTestId(INSPECT_HOVER);
    const extended = within(hover).getByTestId(INSPECT_EXTENDED);
    expect(extended.textContent).toContain(face.text.full.split("\n").at(-1) ?? "");
    expect(extended.textContent).toContain("Defend:");
    // The face prints the header and "…", and not the penalties.
    const rules = within(within(hover).getByTestId(INSPECT_FACE)).getByText(/Guess their Mind Games/);
    expect(rules.closest(".card-text")?.textContent).toContain("…");
    expect(rules.closest(".card-text")?.textContent).not.toContain("Defend:");
    // On the face's left: the text and the face are one group, the text first.
    const group = extended.parentElement;
    expect(group?.className).toBe("inspect-facegroup");
    expect(group?.firstElementChild).toBe(extended);
    expect(group?.lastElementChild).toBe(within(hover).getByTestId(INSPECT_FACE));
  });

  it("R1017000 a face that prints its whole text has no extended description", () => {
    render(<HoverPreview face={collection("core-052", true)} anchor={ANCHOR} />);
    expect(screen.queryByTestId(INSPECT_EXTENDED)).toBeNull();
  });

  it("R1017000 a clamped face prints its whole text and still has it beside the face, whole", () => {
    const face = collection("meditative-049-2");
    render(<HoverPreview face={face} anchor={ANCHOR} />);
    expect(screen.getByTestId(INSPECT_EXTENDED).textContent).toContain("Death:");
  });

  it("R1017000 R280 a Summit in a game previews its next Ascent line, the whole ladder on its left", () => {
    const face = faceModel({
      defId: SUMMIT_DEF_ID,
      def: def(SUMMIT_DEF_ID),
      radiant: false,
      inPlay: { preview: [{ label: ASCENT_LABEL, value: 3 }] },
    });
    render(<HoverPreview face={face} anchor={ANCHOR} />);
    const rules = screen.getByTestId(INSPECT_FACE).querySelector(".card-text");
    expect(rules?.textContent).toContain("Ascent 3:");
    expect(rules?.textContent).not.toContain("Ascent 4:");
    const extended = screen.getByTestId(INSPECT_EXTENDED);
    expect(extended.textContent).toContain("Ascent 0:");
    expect(extended.textContent).toContain("Ascent 10: Win the game.");
  });

  it("R1017000 the touch sheet has the whole text under the face", () => {
    render(<InspectSheet face={collection("meditative-022")} onClose={() => undefined} />);
    expect(within(screen.getByTestId(INSPECT_SHEET)).getByTestId(INSPECT_EXTENDED).textContent).toContain("Greed:");
  });

  it("R1017000 the detail view prints a short-form card's rules at reading size on every screen", () => {
    render(<CardDetail def={def("core-095")} onClose={() => undefined} />);
    const rules = screen.getByTestId(INSPECT_DETAIL).querySelector(".inspect-rules");
    expect(rules?.getAttribute("data-long")).toBe("true");
    expect(rules?.textContent).toContain("summon a Chaos Golem");
  });

  it("R1017000 in the Almanac's and the deck builder's grid, resting on a card that prints less opens its preview, and only then", () => {
    vi.useFakeTimers();
    const ids = ["meditative-022-1", "core-052"];
    render(<PoolGrid ids={ids} catalog={{ version: "test", cards: CATALOG }} onInspect={() => undefined} />);
    const rest = (id: string): void => {
      fireEvent.pointerEnter(screen.getByTestId(poolCardId(id)), { pointerType: "mouse", clientX: 40, clientY: 40, pointerId: 1, isPrimary: true });
      act(() => {
        vi.advanceTimersByTime(HOVER_DELAY_MS * 2);
      });
    };
    rest("core-052");
    expect(screen.queryByTestId(INSPECT_HOVER)).toBeNull();
    rest("meditative-022-1");
    expect(within(screen.getByTestId(INSPECT_HOVER)).getByTestId(INSPECT_EXTENDED).textContent).toContain("Defend:");
  });
});
