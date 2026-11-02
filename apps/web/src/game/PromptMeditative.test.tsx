// #617 (MN09): the play-time choices the Meditative set adds read on any screen. A list of five or
// more keeps its title, so the card being played is read from its name in it (a short Choose one is
// bare cards alone, #2738), which #163 Swear Loyalities' five factions show; and #98 Showdown's five
// lanes say which lane each one is. #2757: Showdown's lanes are no list but the lanes on the board.

import type { ActionBody, PlayerView } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { INSPECT_HOVER, closeInspect } from "../cards/index.ts";
import { LONG_PRESS_MS } from "../cards/inspect/index.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import Game from "./Game.tsx";
import Prompt from "./Prompt.tsx";
import { modeText } from "./modeText.ts";
import type { Interaction } from "./actions.ts";
import { baseView, card, emptySide } from "../test/fixtures.ts";

afterEach(() => {
  cleanup();
  closeInspect();
  vi.useRealTimers();
});

function playView(defId: string, radiant = false): PlayerView {
  return baseView({ you: emptySide("p1", { hand: [card({ instanceId: "h1", defId, radiant })] }) });
}

function renderChoice(defId: string, modes: readonly string[], radiant = false) {
  const onAction = vi.fn();
  const candidates = modes.map((mode): ActionBody => ({ type: "play", instanceId: "h1", modes: [mode] }));
  const interaction: Interaction = { stage: "playing", instanceId: "h1", candidates, picked: {} };
  render(
    <CatalogContext.Provider value={lookupFromDefs(CATALOG)}>
      <Prompt view={playView(defId, radiant)} interaction={interaction} legal={candidates} onAction={onAction} />
    </CatalogContext.Provider>,
  );
  return onAction;
}

const LANES = ["lane-1", "lane-2", "lane-3", "lane-4", "lane-5"];
const FACTIONS = ["Vanguard Coalition", "ETHICS", "Golden Accord", "Pegasus", "Star-jumper Core"];

describe("#617 the Meditative play-time choices", () => {
  it("#617 Showdown's lanes are named by number and by where they sit, on both faces", () => {
    expect(LANES.map((lane) => modeText("meditative-098", lane).label)).toEqual(["Lane 1", "Lane 2", "Lane 3", "Lane 4", "Lane 5"]);
    expect(modeText("meditative-098", "lane-1").detail).toContain("on the left");
    expect(modeText("meditative-098", "lane-5").detail).toContain("on the right");
    expect(modeText("meditative-098", "lane-3", true).detail).toContain("Book of Buff");
    expect(modeText("meditative-098", "lane-3").detail).not.toContain("Book of Buff");
  });

  it("#617 Showdown's lanes answer with the lane's own option string, from the board (#2757)", () => {
    const onAction = vi.fn();
    const legal = LANES.map((mode): ActionBody => ({ type: "play", instanceId: "h1", modes: [mode] }));
    render(
      <CatalogContext.Provider value={lookupFromDefs(CATALOG)}>
        <Game view={playView("meditative-098")} legal={legal} onAction={onAction} />
      </CatalogContext.Provider>,
    );
    fireEvent.click(screen.getByTestId("hand-card-h1"));
    expect(screen.getByTestId("lane-pick-2").getAttribute("aria-label")).toContain("Lane 2");
    fireEvent.click(screen.getByTestId("zone-you-units-2"));
    expect(onAction).toHaveBeenCalledWith({ type: "play", instanceId: "h1", modes: ["lane-2"] });
  });

  it("#617 R2617000 a touch held on the card's name over a list of choices (Swear Loyalities' factions) shows its preview and picks nothing", () => {
    vi.useFakeTimers();
    const onAction = renderChoice("stinkyscape-163", FACTIONS);
    const source = screen.getByTestId("prompt-title-source");
    expect(source).toHaveTextContent(CATALOG["stinkyscape-163"]?.name ?? "stinkyscape-163");
    fireEvent.pointerDown(source, { pointerType: "touch", pointerId: 1, clientX: 10, clientY: 10 });
    act(() => {
      vi.advanceTimersByTime(LONG_PRESS_MS);
    });
    expect(screen.getByTestId(INSPECT_HOVER)).toHaveTextContent(CATALOG["stinkyscape-163"]?.name ?? "stinkyscape-163");
    fireEvent.pointerUp(source, { pointerType: "touch", pointerId: 1 });
    expect(screen.queryByTestId(INSPECT_HOVER)).toBeNull();
    fireEvent.click(source);
    expect(onAction).not.toHaveBeenCalled();
  });
});
