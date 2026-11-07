// #2932: on a phone held upright a route's rail (Back and the rank line) rides the top strip, a
// direct child of `.game` beside the turn banner, and the control bar keeps one row; every other
// layout, and a route that does not opt in, keeps the rail in the control bar. The phone is
// `stubPhonePortrait`: jsdom has no media queries.

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { fullBoardView } from "../test/fixtures.ts";
import { stubPhoneBar, stubPhonePortrait } from "../test/phoneBar.ts";
import Game from "./Game.tsx";

let restore: (() => void) | null = null;

afterEach(() => {
  cleanup();
  restore?.();
  restore = null;
});

function renderGame(over: { phoneRailTop?: boolean; rail?: boolean } = {}): HTMLElement {
  render(
    <Game
      view={fullBoardView()}
      legal={[]}
      onAction={() => undefined}
      phoneRailTop={over.phoneRailTop}
      boardRail={over.rail === false ? undefined : <button type="button">← Back</button>}
    />,
  );
  return screen.getByTestId("game");
}

describe("#2932 the rail rides the top strip on a phone held upright", () => {
  it("on a portrait phone, for a route that opts in, the rail is a direct child of `.game` and the control bar has none", () => {
    restore = stubPhonePortrait(true);
    const game = renderGame({ phoneRailTop: true });
    expect(game.getAttribute("data-rail-top")).toBe("true");
    const rail = game.querySelector(":scope > .board-rail");
    expect(rail, "the rail is `.game`'s own child").not.toBeNull();
    expect(rail?.classList.contains("board-rail--top")).toBe(true);
    expect(rail?.textContent).toContain("Back");
    expect(game.querySelector(".control-bar .board-rail"), "the control bar holds no rail").toBeNull();
    expect(game.querySelectorAll(".board-rail")).toHaveLength(1);
  });

  it("the rail comes before the turn banner, and the banner stays a direct child of `.game`", () => {
    restore = stubPhonePortrait(true);
    const game = renderGame({ phoneRailTop: true });
    const children = [...game.children];
    const banner = game.querySelector(":scope > .turn-banner");
    const rail = game.querySelector(":scope > .board-rail");
    expect(banner, "the banner is still `.game`'s child").not.toBeNull();
    expect(children.indexOf(rail as Element)).toBeLessThan(children.indexOf(banner as Element));
  });

  it("a route that does not opt in keeps the rail in the control bar, on a phone too", () => {
    restore = stubPhonePortrait(true);
    const game = renderGame();
    expect(game.getAttribute("data-rail-top")).toBeNull();
    expect(game.querySelector(".control-bar > .board-rail")).not.toBeNull();
    expect(game.querySelector(":scope > .board-rail")).toBeNull();
  });

  it("off a portrait phone the rail stays in the control bar, a landscape phone's included", () => {
    restore = stubPhonePortrait(false);
    let game = renderGame({ phoneRailTop: true });
    expect(game.getAttribute("data-rail-top")).toBeNull();
    expect(game.querySelector(".control-bar > .board-rail")).not.toBeNull();
    cleanup();
    restore();
    restore = stubPhoneBar(true);
    game = renderGame({ phoneRailTop: true });
    expect(game.getAttribute("data-rail-top")).toBeNull();
    expect(game.querySelector(".control-bar > .board-rail")).not.toBeNull();
  });

  it("with no rail to show there is no strip row to fill", () => {
    restore = stubPhonePortrait(true);
    const game = renderGame({ phoneRailTop: true, rail: false });
    expect(game.getAttribute("data-rail-top")).toBeNull();
    expect(game.querySelector(".board-rail")).toBeNull();
  });
});
