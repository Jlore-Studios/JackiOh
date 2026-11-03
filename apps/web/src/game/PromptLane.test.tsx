// #2757: a Choose one of lanes (Showdown, Meditative #98: "Choose an empty lane"; R81, R831008) is answered on the
// board. Each lane on offer is lit by one oblong over its four zones, and a click, Enter or Space on any of them
// picks it. Driven through `<Game/>` as a player drives it; every action expected is a body `legal` lists
// (CLAUDE.md rule 7). The lanes' words are MODE_TEXT's (modeText.ts).

import { CATALOG } from "@jackioh/cards";
import type { ActionBody, PlayerId, PlayerView } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { closeInspect } from "../cards/index.ts";
import { __resetSettingsForTests } from "../settings/index.ts";
import { baseView, card, emptySide, faceUpBackrow, pendingFor, unit } from "../test/fixtures.ts";
import { aimTargets } from "./aim/aim.ts";
import { highlightFor, isLanePick, laneZoneTestids, type Interaction } from "./actions.ts";
import Board from "./Board.tsx";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import { LANES, NO_HIGHLIGHT } from "./contract.ts";
import Game from "./Game.tsx";
import { LANE_PICK_OUTSET_PX, LANE_TIP_TESTID, laneOblongTestid } from "./LanePick.tsx";
import { modeText } from "./modeText.ts";
import Prompt from "./Prompt.tsx";

const lookup = lookupFromDefs(CATALOG);
const SHOWDOWN = "meditative-098";
const LIT = [1, 3, 4];

const playOf = (modes: readonly string[]): ActionBody => ({ type: "play", instanceId: "h1", modes: [...modes] });
/** The plays Showdown lists with lanes 1, 3 and 4 empty (R831008). */
const PLAYS: ActionBody[] = LIT.map((lane) => playOf([`lane-${String(lane)}`]));

const el = (id: string): HTMLElement => screen.getByTestId(id);
const oblongs = (): string[] =>
  [...document.querySelectorAll<HTMLElement>(".lane-pick-oblong")].map((oblong) => oblong.getAttribute("data-lane") ?? "");

/**
 * Showdown in hand at cost 1; lanes 2 (your unit `u2`) and 5 (the opponent's backrow) are full, so lanes
 * 1, 3 and 4 are the empty ones. `viewer` p2 is the same board seen from the second seat.
 */
function showdownView(over: { radiant?: boolean; viewer?: PlayerId } = {}): PlayerView {
  const me = over.viewer ?? "p1";
  const them: PlayerId = me === "p1" ? "p2" : "p1";
  return baseView({
    viewer: me,
    active: me,
    you: emptySide(me, {
      hand: [card({ instanceId: "h1", defId: SHOWDOWN, radiant: over.radiant === true, cost: 1 })],
      units: [null, unit(me, { instanceId: "u2" }), null, null, null],
    }),
    opponent: emptySide(them, { hand: { count: 3 }, backrow: [null, null, null, null, faceUpBackrow(them)] }),
  });
}

function renderGame(legal: readonly ActionBody[], view: PlayerView = showdownView()) {
  const onAction = vi.fn();
  const utils = render(
    <CatalogContext.Provider value={lookup}>
      <Game view={view} legal={legal} onAction={onAction} />
    </CatalogContext.Provider>,
  );
  return { ...utils, onAction };
}

/** Starts the play: a click on Showdown in hand. */
function startPlay(legal: readonly ActionBody[] = PLAYS, view: PlayerView = showdownView()) {
  const out = renderGame(legal, view);
  fireEvent.click(el("hand-card-h1"));
  return out;
}

// The rows of the board, top to bottom, as Board.tsx stacks them.
const ROWS = ["opponent-backrow", "opponent-units", "you-units", "you-backrow"];
const ZONE_WIDTH = 80;
const ZONE_HEIGHT = 50;
const LANE_PITCH = 100;
const ROW_PITCH = 60;

function rect(left: number, top: number, width: number, height: number): DOMRect {
  return { left, top, width, height, right: left + width, bottom: top + height, x: left, y: top, toJSON: () => ({}) };
}

/** jsdom has no layout: lane n's zones stand at x = 100 n, each row 60 below the last. */
function fakeLayout(): void {
  vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(function (this: Element) {
    const match = /^zone-(you|opponent)-(units|backrow)-(\d+)$/.exec(this.getAttribute("data-testid") ?? "");
    if (match === null) return rect(0, 0, 0, 0);
    const row = ROWS.indexOf(`${match[1] ?? ""}-${match[2] ?? ""}`);
    return rect(LANE_PITCH * Number(match[3]), ROW_PITCH * row, ZONE_WIDTH, ZONE_HEIGHT);
  });
}

/** The box of lane `lane`'s four zones, pushed out by the oblong's outset. */
function laneBox(lane: number) {
  const out = LANE_PICK_OUTSET_PX;
  return {
    left: `${LANE_PITCH * lane - out}px`,
    top: `${-out}px`,
    width: `${ZONE_WIDTH + 2 * out}px`,
    height: `${ROW_PITCH * (ROWS.length - 1) + ZONE_HEIGHT + 2 * out}px`,
  };
}

const focus = (id: string): void => {
  act(() => {
    el(id).focus();
  });
};

beforeEach(() => {
  __resetSettingsForTests();
  fakeLayout();
});

afterEach(() => {
  cleanup();
  closeInspect();
  vi.restoreAllMocks();
  __resetSettingsForTests();
});

describe("#2757 a Choose one of lanes is answered on the board", () => {
  it("#2757 lanes 1, 3 and 4 empty light exactly three oblongs over their four zones, and open no cards and no list", () => {
    for (const viewer of ["p1", "p2"] as const) {
      startPlay(PLAYS, showdownView({ viewer }));
      const modal = el("prompt-modal");
      expect(modal).toHaveAttribute("data-lane-pick", "true");
      expect(modal).toHaveAttribute("data-prompt-kind", "mode");
      expect(screen.queryByTestId("prompt-cards")).toBeNull();
      expect(screen.queryByTestId("prompt-submit")).toBeNull();
      expect(document.querySelector('[data-testid^="prompt-option-"]')).toBeNull();
      expect(document.querySelector(".prompt-modes")).toBeNull();

      expect(oblongs()).toEqual(["1", "3", "4"]);
      for (const lane of LIT) {
        const style = el(laneOblongTestid(lane)).style;
        expect({ left: style.left, top: style.top, width: style.width, height: style.height }).toEqual(laneBox(lane));
      }

      // The zones take the click and the focus, but are not lit green: that is the oblong's.
      for (const lane of LIT) for (const id of laneZoneTestids(lane)) expect(el(id)).toHaveAttribute("tabindex", "0");
      expect(el("zone-you-backrow-2")).not.toHaveAttribute("tabindex");
      expect(el("zone-you-units-5")).not.toHaveAttribute("tabindex");
      expect(document.querySelector(".zone[data-glow]")).toBeNull();
      expect(modal).toHaveTextContent("Tap a highlighted lane.");

      const words = modeText(SHOWDOWN, "lane-3");
      expect(el(laneOblongTestid(3))).toHaveAttribute("aria-label", `${words.label}: ${words.detail ?? ""}`);
      expect(el(laneOblongTestid(3)).getAttribute("aria-label")).not.toContain("Book of Buff");
      cleanup();
    }

    // A lane pick is no aim (R738): nothing is declared, so nothing is sent to the opponent.
    const view = showdownView();
    const building: Interaction = { stage: "playing", instanceId: "h1", candidates: PLAYS, picked: {} };
    expect(aimTargets(view, PLAYS, building).size).toBe(0);
    expect(highlightFor(view, PLAYS, building).glow?.size ?? 0).toBe(0);
  });

  it("#2757 a click on any of a lit lane's four zones sends one play of that lane; a click on an unlit lane sends nothing", () => {
    for (const id of laneZoneTestids(3)) {
      const { onAction } = startPlay();
      fireEvent.click(el("card-u2"));
      fireEvent.click(el("zone-you-backrow-5"));
      fireEvent.click(el("zone-opponent-units-2"));
      expect(onAction).not.toHaveBeenCalled();
      expect(oblongs()).toEqual(["1", "3", "4"]);

      fireEvent.click(el(id));
      expect(onAction).toHaveBeenCalledTimes(1);
      expect(onAction).toHaveBeenCalledWith(PLAYS[1]);
      expect(oblongs()).toEqual([]);
      expect(screen.queryByTestId("prompt-modal")).toBeNull();
      cleanup();
    }
  });

  it("#2757 Enter or Space on a lit lane's zone answers, and the arrows move between lit lanes", () => {
    const first = startPlay();
    focus("zone-you-units-1");
    expect(el(laneOblongTestid(1))).toHaveAttribute("data-active", "true");
    const walk = (key: string): void => {
      fireEvent.keyDown(document.activeElement ?? document.body, { key });
    };
    walk("ArrowRight");
    expect(document.activeElement).toBe(el("zone-you-units-3"));
    walk("ArrowRight");
    expect(document.activeElement).toBe(el("zone-you-units-4"));
    walk("ArrowRight");
    expect(document.activeElement).toBe(el("zone-you-units-4"));
    walk("ArrowLeft");
    expect(document.activeElement).toBe(el("zone-you-units-3"));
    // Only the focused lane is brightened.
    expect([...document.querySelectorAll(".lane-pick-oblong[data-active]")].map((oblong) => oblong.getAttribute("data-lane"))).toEqual(["3"]);
    expect(first.onAction).not.toHaveBeenCalled();

    fireEvent.keyDown(document.activeElement ?? document.body, { key: " " });
    expect(first.onAction).toHaveBeenCalledTimes(1);
    expect(first.onAction).toHaveBeenCalledWith(PLAYS[1]);
    cleanup();

    const second = startPlay();
    fireEvent.keyDown(el("zone-opponent-backrow-4"), { key: "Enter" });
    expect(second.onAction).toHaveBeenCalledTimes(1);
    expect(second.onAction).toHaveBeenCalledWith(PLAYS[2]);
  });

  it("#2757 a lit lane brightens and shows its words while one of its zones is hovered", () => {
    startPlay();
    expect(screen.queryByTestId(LANE_TIP_TESTID)).toBeNull();
    fireEvent.pointerOver(el("zone-opponent-units-3"));
    expect(el(laneOblongTestid(3))).toHaveAttribute("data-active", "true");
    const words = modeText(SHOWDOWN, "lane-3");
    expect(el(LANE_TIP_TESTID)).toHaveTextContent(words.label);
    expect(el(LANE_TIP_TESTID)).toHaveTextContent(words.detail ?? "");
    fireEvent.pointerOver(el("zone-you-units-2"));
    expect(document.querySelector(".lane-pick-oblong[data-active]")).toBeNull();
    expect(screen.queryByTestId(LANE_TIP_TESTID)).toBeNull();
  });

  it("#2757 Cancel and Escape abandon the play", () => {
    const first = startPlay();
    fireEvent.click(el("prompt-cancel"));
    expect(screen.queryByTestId("prompt-modal")).toBeNull();
    expect(oblongs()).toEqual([]);
    expect(el("zone-you-units-1")).not.toHaveAttribute("tabindex");
    fireEvent.click(el("zone-you-units-1"));
    expect(first.onAction).not.toHaveBeenCalled();
    cleanup();

    const second = startPlay();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByTestId("prompt-modal")).toBeNull();
    expect(oblongs()).toEqual([]);
    expect(el("zone-you-units-1")).not.toHaveAttribute("tabindex");
    expect(second.onAction).not.toHaveBeenCalled();
  });

  it("#2757 the Radiant face is the same and its words carry Book of Buff", () => {
    const { onAction } = startPlay(PLAYS, showdownView({ radiant: true }));
    expect(oblongs()).toEqual(["1", "3", "4"]);
    const words = modeText(SHOWDOWN, "lane-3", true);
    expect(words.detail).toContain("Book of Buff");
    expect(el(laneOblongTestid(3))).toHaveAttribute("aria-label", `${words.label}: ${words.detail ?? ""}`);
    fireEvent.click(el("zone-you-backrow-3"));
    expect(onAction).toHaveBeenCalledTimes(1);
    expect(onAction).toHaveBeenCalledWith(PLAYS[1]);
  });

  it("#2757 an engine mode prompt of lanes is answered on the board the same way", () => {
    const answerOf = (lane: number): ActionBody => ({
      type: "answer",
      choiceId: "ch1",
      selection: [{ pick: "mode", option: `lane-${String(lane)}` }],
    });
    const options = LIT.map((lane) => ({ key: `mode:lane-${String(lane)}`, label: `lane-${String(lane)}` }));
    const view = baseView({
      you: emptySide("p1", { units: [null, unit("p1", { instanceId: "u2" }), null, null, null] }),
      opponent: emptySide("p2", { hand: { count: 3 }, backrow: [null, null, null, null, faceUpBackrow("p2")] }),
      pending: pendingFor("mode", options, { sourceDefId: SHOWDOWN }),
    });
    const { onAction } = renderGame(LIT.map(answerOf), view);

    const modal = el("prompt-modal");
    expect(modal).toHaveAttribute("data-prompt-source", "engine");
    expect(modal).toHaveAttribute("data-lane-pick", "true");
    expect(screen.queryByTestId("prompt-cancel")).toBeNull();
    expect(screen.queryByTestId("prompt-cards")).toBeNull();
    expect(oblongs()).toEqual(["1", "3", "4"]);
    const words = modeText(SHOWDOWN, "lane-4");
    expect(el(laneOblongTestid(4))).toHaveAttribute("aria-label", `${words.label}: ${words.detail ?? ""}`);

    fireEvent.click(el("zone-you-units-2"));
    expect(onAction).not.toHaveBeenCalled();
    fireEvent.click(el("zone-opponent-backrow-4"));
    expect(onAction).toHaveBeenCalledTimes(1);
    expect(onAction).toHaveBeenCalledWith(answerOf(4));
  });

  it("#2757 a Choose one that mixes a lane with another option is no lane pick", () => {
    expect(isLanePick(["lane-1", "lane-5"])).toBe(true);
    for (const options of [["lane-1", "draw"], ["lane-6"], ["lane-0"], ["lane-01"], ["lane-"], []]) {
      expect(isLanePick(options), options.join(",")).toBe(false);
    }
    expect(LANES).toEqual([1, 2, 3, 4, 5]);

    startPlay(["lane-1", "lane-3", "draw"].map((mode) => playOf([mode])));
    expect(el("prompt-modal")).toHaveAttribute("data-prompt-kind", "discover");
    expect(el("prompt-modal")).not.toHaveAttribute("data-lane-pick");
    expect(el("prompt-option-lane-1")).toBeInTheDocument();
    expect(oblongs()).toEqual([]);
    expect(el("zone-you-units-1")).not.toHaveAttribute("tabindex");
  });

  it("#2757 a replay draws the lit lanes and nothing answers", () => {
    // R2211000: the replay's prompt is drawn as offered and sends nothing.
    const onAction = vi.fn();
    const view = baseView({
      you: emptySide("p1", { units: [null, unit("p1", { instanceId: "u2" }), null, null, null] }),
      opponent: emptySide("p2", { hand: { count: 3 }, backrow: [null, null, null, null, faceUpBackrow("p2")] }),
      pending: pendingFor(
        "mode",
        LIT.map((lane) => ({ key: `mode:lane-${String(lane)}`, label: `lane-${String(lane)}` })),
        { sourceDefId: SHOWDOWN },
      ),
    });
    render(
      <CatalogContext.Provider value={lookup}>
        <Board view={view} highlight={NO_HIGHLIGHT} />
        <Prompt view={view} legal={[]} onAction={onAction} onCancel={vi.fn()} readOnly />
      </CatalogContext.Provider>,
    );
    expect(oblongs()).toEqual(["1", "3", "4"]);
    expect(document.querySelector(".prompt-board-hint")).toBeNull();
    expect(screen.queryByTestId("prompt-cancel")).toBeNull();
    expect(document.querySelector(".prompt-title")).not.toBeNull();
    fireEvent.click(el("zone-you-units-3"));
    fireEvent.keyDown(el("zone-opponent-backrow-4"), { key: "Enter" });
    expect(onAction).not.toHaveBeenCalled();
    // And it takes no hover: a replay's lanes stand still.
    fireEvent.pointerOver(el("zone-opponent-units-3"));
    expect(screen.queryByTestId(LANE_TIP_TESTID)).toBeNull();
  });
});
