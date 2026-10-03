// The Marvel Snap-style card wheel (cards/wheel): the primary card pops up, its neighbours peek
// out at its sides, and the viewer shuffles through by buttons, scroll, tap, swipe or keyboard.

import { CATALOG } from "@jackioh/cards";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { faceModel } from "../model.ts";
import CardWheel, { wheelTestid, type WheelItem } from "./CardWheel.tsx";

function face(id: string) {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`the catalog has no ${id}`);
  return faceModel({ defId: id, def, radiant: false });
}

function items(): WheelItem[] {
  return [
    { key: "top", face: face("core-004"), name: "Top", note: "Top of pile" },
    { key: "b1", face: null, name: "Unknown card", note: "Buried" },
    { key: "b2", face: null, name: "Unknown card", note: "Bottom of pile" },
  ];
}

afterEach(cleanup);

function offsets(): (string | null)[] {
  return screen.getAllByTestId(wheelTestid.card).map((element) => element.getAttribute("data-offset"));
}

describe("CardWheel", () => {
  it("stands the top card forward, the card under it peeking right and the bottom peeking left", () => {
    render(<CardWheel items={items()} ariaLabel="Stack pile" />);
    expect(offsets()).toEqual(["0", "1", "-1"]);
    expect(screen.getByTestId(wheelTestid.position)).toHaveTextContent("1 of 3");
    expect(screen.getByTestId(wheelTestid.note)).toHaveTextContent("Top of pile");
  });

  it("moves to the next card by button, by tap, by scroll and by keyboard", () => {
    const onIndex = vi.fn();
    render(<CardWheel items={items()} ariaLabel="Stack pile" onIndex={onIndex} />);
    fireEvent.click(screen.getByTestId(wheelTestid.next));
    expect(screen.getByTestId(wheelTestid.position)).toHaveTextContent("2 of 3");
    expect(onIndex).toHaveBeenCalledWith(1);

    // Tapping the card peeking right goes deeper; ArrowLeft comes back up.
    const cards = screen.getAllByTestId(wheelTestid.card);
    fireEvent.click(cards.find((element) => element.getAttribute("data-offset") === "1") ?? cards[0]!);
    expect(screen.getByTestId(wheelTestid.position)).toHaveTextContent("3 of 3");
    fireEvent.keyDown(screen.getByTestId(wheelTestid.wheel), { key: "ArrowLeft" });
    expect(screen.getByTestId(wheelTestid.position)).toHaveTextContent("2 of 3");

    // Scrolling down goes deeper, Home returns to the top, End to the bottom.
    fireEvent.wheel(screen.getByTestId(wheelTestid.wheel), { deltaY: 100 });
    expect(screen.getByTestId(wheelTestid.position)).toHaveTextContent("3 of 3");
    fireEvent.keyDown(screen.getByTestId(wheelTestid.wheel), { key: "Home" });
    expect(screen.getByTestId(wheelTestid.position)).toHaveTextContent("1 of 3");
    fireEvent.keyDown(screen.getByTestId(wheelTestid.wheel), { key: "End" });
    expect(screen.getByTestId(wheelTestid.position)).toHaveTextContent("3 of 3");
  });

  it("wraps past the ends, so the wheel shuffles either way", () => {
    render(<CardWheel items={items()} ariaLabel="Stack pile" />);
    fireEvent.click(screen.getByTestId(wheelTestid.prev));
    expect(screen.getByTestId(wheelTestid.position)).toHaveTextContent("3 of 3");
    fireEvent.click(screen.getByTestId(wheelTestid.next));
    expect(screen.getByTestId(wheelTestid.position)).toHaveTextContent("1 of 3");
  });

  it("shows backs without naming them", () => {
    render(<CardWheel items={items()} ariaLabel="Stack pile" defaultIndex={1} />);
    expect(screen.getByTestId(wheelTestid.note)).toHaveTextContent("Buried");
    expect(document.body.textContent).not.toMatch(/core-00\d/);
  });

  it("disables both turns for a single card", () => {
    render(<CardWheel items={[{ key: "only", face: face("core-004"), name: "Top" }]} ariaLabel="Stack pile" />);
    expect(screen.getByTestId(wheelTestid.prev)).toBeDisabled();
    expect(screen.getByTestId(wheelTestid.next)).toBeDisabled();
    expect(screen.getByTestId(wheelTestid.position)).toHaveTextContent("1 of 1");
  });
});
