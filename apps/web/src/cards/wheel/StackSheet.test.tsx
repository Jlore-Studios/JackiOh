// A Stack pile opened as a wheel (cards/wheel/StackSheet.tsx): the top with its face, every buried
// card as a back, and a Close that reports.

import { CATALOG } from "@jackioh/cards";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { INSPECT_CLOSE } from "../inspect/testids.ts";
import { faceModel } from "../model.ts";
import { wheelTestid } from "./CardWheel.tsx";
import StackSheet, { STACK_NOTES, stackItems, stackTestid } from "./StackSheet.tsx";

afterEach(cleanup);

function topFace() {
  const def = CATALOG["core-004"];
  if (def === undefined) throw new Error("the catalog has no core-004");
  return faceModel({ defId: "core-004", def, radiant: false });
}

describe("stackItems", () => {
  it("holds the top up over one back per buried card, naming only the top", () => {
    const items = stackItems(topFace(), "Top", 2);
    expect(items.map((item) => [item.face === null ? "back" : "face", item.note])).toEqual([
      ["face", STACK_NOTES.top],
      ["back", "Buried"],
      ["back", STACK_NOTES.bottom],
    ]);
    expect(items[0]?.name).toBe("Top");
    expect(items[1]?.name).toBe("Unknown card");
  });
});

describe("StackSheet", () => {
  it("shows the pile's cards on a wheel and closes on Close", () => {
    const onClose = vi.fn();
    render(<StackSheet title="Stack pile" top={topFace()} topName="Top" buried={2} onClose={onClose} />);
    expect(screen.getByTestId(stackTestid.sheet)).toBeDefined();
    expect(screen.getByTestId(stackTestid.title)).toHaveTextContent("Stack pile");
    expect(screen.getByTestId(stackTestid.count)).toHaveTextContent("3 cards");
    expect(screen.getByTestId(wheelTestid.position)).toHaveTextContent("1 of 3");
    fireEvent.click(screen.getByTestId(INSPECT_CLOSE));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("draws a back on top when the pile names nothing", () => {
    render(<StackSheet title="Stack pile" top={null} topName="Unknown card" buried={1} onClose={() => {}} />);
    const primary = screen.getAllByTestId(wheelTestid.card).find((element) => element.getAttribute("data-offset") === "0");
    expect(primary?.getAttribute("data-face")).toBe("back");
  });
});
