// #3633: a queue step re-renders the board with a new `animating` map; a card none of whose own keys
// changed is not drawn again (Card.tsx, `sameCardProps`). `marksOf` runs once in each drawing of a
// face-up card, so its calls for a card count that card's drawings.

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import Board from "./Board.tsx";
import type { AnimatingMap } from "./contract.ts";
import { marksOf } from "../cards/marks.ts";
import { withCatalog } from "../test/board.tsx";
import { fullBoardView } from "../test/fixtures.ts";

vi.mock("../cards/marks.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../cards/marks.ts")>();
  return { ...actual, marksOf: vi.fn(actual.marksOf) };
});

afterEach(() => {
  cleanup();
  vi.mocked(marksOf).mockClear();
});

/** How many times the card `instanceId` has been drawn since the last clear. */
function drawings(instanceId: string): number {
  return vi.mocked(marksOf).mock.calls.filter(([shown]) => (shown as { instanceId?: string } | null | undefined)?.instanceId === instanceId).length;
}

describe("#3633 a queue step draws only what it changes", () => {
  it("#3633 a board re-rendered for a queue step draws again only the card it animates", () => {
    const view = fullBoardView();
    const onClick = (): void => undefined;
    const { rerender } = render(withCatalog(<Board view={view} animating={new Map()} onClick={onClick} />));
    expect(drawings("u1")).toBeGreaterThan(0);
    expect(drawings("u2")).toBeGreaterThan(0);
    vi.mocked(marksOf).mockClear();
    rerender(withCatalog(<Board view={view} animating={new Map([["card-u1", "damage"]]) as AnimatingMap} onClick={onClick} />));
    expect(drawings("u1")).toBe(1);
    expect(drawings("u2")).toBe(0);
    expect(drawings("c11")).toBe(0);
  });
});
