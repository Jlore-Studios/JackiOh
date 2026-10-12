// #617 (MN09): the play-time choices the Meditative set adds read on any screen. A "Choose one" is
// words, not faces, so the card being played is read from its name in the title (a hover preview,
// or a long-press sheet on a phone); and #98 Showdown's five lanes say which lane each one is.

import type { ActionBody, PlayerView } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { INSPECT_SHEET, closeInspect } from "../cards/index.ts";
import { LONG_PRESS_MS } from "../cards/inspect/index.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
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

describe("#617 the Meditative play-time choices", () => {
  it("#617 Showdown's lanes are named by number and by where they sit, on both faces", () => {
    expect(LANES.map((lane) => modeText("meditative-098", lane).label)).toEqual(["Lane 1", "Lane 2", "Lane 3", "Lane 4", "Lane 5"]);
    expect(modeText("meditative-098", "lane-1").detail).toContain("on the left");
    expect(modeText("meditative-098", "lane-5").detail).toContain("on the right");
    expect(modeText("meditative-098", "lane-3", true).detail).toContain("Book of Buff");
    expect(modeText("meditative-098", "lane-3").detail).not.toContain("Book of Buff");
  });

  it("#617 Showdown's picker answers with the lane's own option string", () => {
    const onAction = renderChoice("meditative-098", LANES);
    expect(screen.getByTestId("prompt-option-lane-2")).toHaveTextContent("Lane 2");
    fireEvent.click(screen.getByTestId("prompt-option-lane-2"));
    expect(onAction).toHaveBeenCalledWith({ type: "play", instanceId: "h1", modes: ["lane-2"] });
  });

  it("#617 a long-press on the card's name in a Choose one opens its sheet and picks nothing", () => {
    vi.useFakeTimers();
    const onAction = renderChoice("meditative-022", ["greed", "attack", "defend"]);
    const source = screen.getByTestId("prompt-title-source");
    expect(source).toHaveTextContent(CATALOG["meditative-022"]?.name ?? "meditative-022");
    fireEvent.pointerDown(source, { pointerType: "touch", pointerId: 1, clientX: 10, clientY: 10 });
    act(() => {
      vi.advanceTimersByTime(LONG_PRESS_MS);
    });
    expect(screen.getByTestId(INSPECT_SHEET)).toHaveTextContent(CATALOG["meditative-022"]?.name ?? "meditative-022");
    fireEvent.pointerUp(source, { pointerType: "touch", pointerId: 1 });
    fireEvent.click(source);
    expect(onAction).not.toHaveBeenCalled();
  });
});
