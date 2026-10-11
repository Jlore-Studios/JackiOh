// ME-SECRET's badges (Meditative MB05, R860): one badge per secret a seat holds, naming the
// choice only where the view carries it. Presentation only (CLAUDE.md rule 7): whether the choice
// is there is the engine's `viewFor` to decide.

import type { PlayerView } from "@jackioh/shared";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import Board from "./Board.tsx";
import { fullBoardView } from "../test/fixtures.ts";

afterEach(() => {
  cleanup();
});

function secreted(): PlayerView {
  const base = fullBoardView();
  return {
    ...base,
    you: { ...base.you, secrets: [{ id: "secret-1", choice: "greed" }] },
    opponent: { ...base.opponent, secrets: [{ id: "secret-2" }] },
  };
}

describe("secrets badges", () => {
  it("R860 the owner's badge names the choice, the opponent's says only Secret", () => {
    render(<Board view={secreted()} />);

    const mine = screen.getByTestId("secrets-you");
    const badges = [...mine.querySelectorAll(".secret-badge")];
    expect(badges.map((badge) => badge.textContent)).toEqual(["Secret: Greed"]);
    expect(badges.map((badge) => badge.getAttribute("data-secret-id"))).toEqual(["secret-1"]);
    expect(badges.map((badge) => badge.getAttribute("data-choice"))).toEqual(["greed"]);

    const theirs = screen.getByTestId("secrets-opponent");
    const foe = [...theirs.querySelectorAll(".secret-badge")];
    expect(foe.map((badge) => badge.textContent)).toEqual(["Secret"]);
    expect(foe.map((badge) => badge.getAttribute("data-secret-id"))).toEqual(["secret-2"]);
    expect(foe.map((badge) => badge.getAttribute("data-choice"))).toEqual([null]);
  });

  it("R860 a side with no secret draws no badge", () => {
    render(<Board view={fullBoardView()} />);
    expect(screen.queryByTestId("secrets-you")).toBeNull();
    expect(screen.queryByTestId("secrets-opponent")).toBeNull();
  });
});
