// The game log's wording for events the view redacted (SPEC §10.10, R97, R177). The log is built
// from the event payload alone, so a redacted payload must not read as a real statement.

import { CATALOG } from "@jackioh/cards";
import type { GameEvent, PlayerView } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { INSPECT_CLOSE, INSPECT_HOVER, INSPECT_SHEET, closeInspect } from "../cards/index.ts";
import { HOVER_DELAY_MS } from "../cards/inspect/index.ts";
import Log, { LOG_CARD_TESTID, keptLineFor } from "./Log.tsx";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import { LOG_HISTORY_LIMIT } from "./config.ts";
import { testid } from "./contract.ts";
import { EMPTY_LOG_HISTORY, LOG_GAP_TEXT, advanceLogHistory } from "./useLogHistory.ts";
import { CHINESE } from "../cards/chinese.ts";
import { baseView, emptySide, fullBoardView, unit, withEvents } from "../test/fixtures.ts";

afterEach(() => {
  cleanup();
  closeInspect();
  vi.useRealTimers();
});

describe("Log: redacted events", () => {
  it("R177 says a hidden card was buffed rather than '+0/+0', and names a public buff in full", () => {
    const view = fullBoardView();
    const mine = view.you.units.find((unit) => unit !== null)?.instanceId ?? "";
    const logged = withEvents(view, [
      { type: "buffed", instanceId: "hidden", attack: 0, health: 0 },
      { type: "buffed", instanceId: mine, attack: 2, health: 1 },
    ]);

    render(<Log view={logged} />);

    const lines = [...screen.getByTestId(testid.log).querySelectorAll(".log-line")].map((li) => li.textContent);
    expect(lines[0]).toBe("A hidden card was buffed");
    expect(lines[1]).toMatch(/gained \+2\/\+1$/);
    expect(lines.join("\n")).not.toContain("+0/+0");
  });
});

describe("Log: a line never prints an id or the sentinel (integration QA)", () => {
  function lines(): string[] {
    return [...screen.getByTestId(testid.log).querySelectorAll(".log-line")].map((li) => li.textContent ?? "");
  }

  it("R154 an opponent's trap the viewer may not read fires as a face-down trap, not as 'hidden'", () => {
    const view = fullBoardView();
    render(
      <Log
        view={withEvents(view, [
          { type: "trapFired", instanceId: "hidden", defId: "hidden", controller: "p2", row: "backrow", lane: 2 },
        ])}
      />,
    );
    expect(lines()).toEqual(["The opponent's face-down trap fired"]);
  });

  it("R370 a card set face down reads as a face-down trap, with the cost its back shows while it stands there", () => {
    const set: GameEvent = { type: "summoned", player: "p2", instanceId: "hidden", defId: "hidden", row: "backrow", lane: 3 };
    const withCost = baseView({ opponent: { ...baseView().opponent, backrow: [null, null, { faceDown: true, cost: 2 }, null, null] } });
    render(<Log view={withEvents(withCost, [set])} />);
    expect(lines()).toEqual(["The opponent's backrow lane 3: a face-down trap was set, (2) Cost"]);
    cleanup();
    // Gone from the zone (it fired), or a back with no cost: the line claims none.
    render(<Log view={withEvents(baseView(), [set])} />);
    expect(lines()).toEqual(["The opponent's backrow lane 3: a face-down trap was set"]);
    expect(lines().join("\n")).not.toContain("hidden");
  });

  it("R432 a prompt names what it asks for, and a number a card's text changed names its key in words", () => {
    const view = fullBoardView();
    const unit = view.you.units[0];
    if (unit === null || unit === undefined) throw new Error("the fixture has a unit in lane 1");
    render(
      <Log
        view={withEvents(view, [
          { type: "promptOpened", player: "p2", choiceId: "c1", kind: "pick" },
          { type: "numberChanged", instanceId: unit.instanceId, defId: unit.defId, key: "drawLimit", value: 2 },
          { type: "numberChanged", instanceId: unit.instanceId, defId: unit.defId, key: "cost", value: 1 },
        ])}
      />,
    );
    const text = lines().join("\n");
    expect(text).toContain("must choose cards to take");
    expect(text).toContain("draw limit became 2");
    expect(text).toContain("now costs (1)");
    expect(text).not.toMatch(/\(pick\)|drawLimit/);
  });

  it("names a unit that has left the board from the window's own public events, and never prints its id", () => {
    const view = fullBoardView();
    render(
      <Log
        view={withEvents(view, [
          { type: "summoned", player: "p2", instanceId: "c46", defId: "core-002", row: "units", lane: 1 },
          { type: "attackDeclared", attackerId: "c46", targetId: "hero-p1", forced: false },
          { type: "damage", sourceId: "c46", targetId: "c46", amount: 12, combat: true },
          { type: "attackDeclared", attackerId: "c99", targetId: "c98", forced: false },
        ])}
      />,
    );
    const text = lines();
    expect(text[1]).toBe("core-002 attacked your hero");
    expect(text[2]).toBe("core-002 took 12 damage in combat");
    // Nothing public ever named c99 or c98: they read as units, not as ids.
    expect(text[3]).toBe("A unit attacked a unit");
    expect(text.join("\n")).not.toMatch(/\bc\d+\b/);
  });

  it("says mana and summons in words, prints a modifier's label, and leaves out 'finished resolving'", () => {
    const view = fullBoardView();
    const modifier = view.you.modifiers[0];
    render(
      <Log
        view={withEvents(view, [
          { type: "manaChanged", player: "p1", current: 2, max: 2 },
          { type: "summoned", player: "p1", instanceId: "x1", defId: "core-015", row: "units", lane: 1 },
          { type: "cardResolved", instanceId: "x1", defId: "core-015" } as never,
          { type: "modifierChanged", player: "p1", modifierId: modifier?.id ?? "", added: true } as never,
          { type: "modifierChanged", player: "p2", modifierId: "m-gone", added: false } as never,
        ])}
      />,
    );
    const text = lines();
    expect(text[0]).toBe("You have 2/2 mana");
    expect(text[1]).toBe("core-015 entered your lane 1");
    expect(text).toHaveLength(4);
    expect(text[2]).toBe(`You gained "${modifier?.label ?? ""}"`);
    expect(text[3]).toBe("Opponent lost an effect");
  });

  it("R1000 a night market reads as a deal at the night market", () => {
    render(<Log view={withEvents(baseView(), [{ type: "promptOpened", player: "p2", choiceId: "c1", kind: "market" }])} />);
    const text = lines().join("\n");
    expect(text).toContain("a deal at the night market");
    expect(text).not.toContain("(market)");
  });

  it("ends the game in words", () => {
    render(<Log view={withEvents(fullBoardView(), [{ type: "gameOver", winner: "p2", reason: "concede" }])} />);
    expect(lines()).toEqual(["Opponent won. You conceded."]);
  });
});

describe("Log: a line about a card opens that card", () => {
  const lookup = lookupFromDefs(CATALOG);
  const nameOf = (defId: string): string => CATALOG[defId]?.name ?? defId;

  function renderLog(view: PlayerView): void {
    render(
      <CatalogContext.Provider value={lookup}>
        <Log view={view} />
      </CatalogContext.Provider>,
    );
  }

  function cardLines(): HTMLElement[] {
    return within(screen.getByTestId(testid.log)).queryAllByTestId(LOG_CARD_TESTID);
  }

  const PLAY: GameEvent = { type: "cardPlayed", player: "p2", instanceId: "c7", defId: "core-032", costPaid: 2 };

  it("hovering the line with a mouse shows the card's face after the hover delay; leaving hides it", () => {
    vi.useFakeTimers();
    renderLog(withEvents(baseView(), [PLAY]));
    const [line] = cardLines();
    expect(line).toBeDefined();
    if (line === undefined) return;
    expect(line.tagName).toBe("BUTTON");
    expect(line).toHaveAttribute("data-def-id", "core-032");
    expect(line).toHaveTextContent(`Opponent played ${nameOf("core-032")} for 2`);

    fireEvent.pointerEnter(line, { pointerType: "mouse" });
    act(() => {
      vi.advanceTimersByTime(HOVER_DELAY_MS - 1);
    });
    expect(screen.queryByTestId(INSPECT_HOVER)).toBeNull();
    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(screen.getByTestId(INSPECT_HOVER)).toHaveTextContent(nameOf("core-032"));
    fireEvent.pointerLeave(line, { pointerType: "mouse" });
    expect(screen.queryByTestId(INSPECT_HOVER)).toBeNull();
  });

  it("a click (a tap, Enter or Space on the button) opens the card in the sheet; Escape closes it and gives focus back", () => {
    renderLog(withEvents(baseView(), [PLAY]));
    const [line] = cardLines();
    if (line === undefined) throw new Error("no card line");
    line.focus();
    fireEvent.click(line);
    const sheet = screen.getByTestId(INSPECT_SHEET);
    expect(sheet).toHaveAttribute("role", "dialog");
    expect(sheet).toHaveTextContent(nameOf("core-032"));
    expect(screen.getByTestId(INSPECT_CLOSE)).toHaveFocus();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByTestId(INSPECT_SHEET)).toBeNull();
    expect(line).toHaveFocus();
  });

  it("R97 a line that names no card opens nothing: the sentinel, a hero, a draw, a mana change", () => {
    renderLog(
      withEvents(baseView(), [
        { type: "cardPlayed", player: "p2", instanceId: "hidden", defId: "hidden", costPaid: 1 },
        { type: "summoned", player: "p2", instanceId: "hidden", defId: "hidden", row: "backrow", lane: 3 },
        { type: "damage", sourceId: null, targetId: "hero-p1", amount: 3, combat: false },
        // Even the viewer's own draw, whose card it may read: the log never names a draw.
        { type: "drawn", player: "p1", instanceId: "c1", defId: "core-011" },
        { type: "manaChanged", player: "p1", current: 2, max: 2 },
        { type: "trapFired", instanceId: "hidden", defId: "hidden", controller: "p2", row: "backrow", lane: 2 },
      ] as GameEvent[]),
    );
    expect(cardLines()).toHaveLength(0);
    expect(screen.getByTestId(testid.log).innerHTML).not.toMatch(/core-\d+/);
  });

  it("names a unit that has left the board from the window's own events, as the line's words do", () => {
    renderLog(
      withEvents(baseView(), [
        { type: "summoned", player: "p2", instanceId: "c46", defId: "core-002", row: "units", lane: 1 },
        { type: "attackDeclared", attackerId: "c46", targetId: "hero-p1", forced: false },
        { type: "attackDeclared", attackerId: "c99", targetId: "c98", forced: false },
      ]),
    );
    const lines = cardLines();
    expect(lines.map((line) => line.getAttribute("data-def-id"))).toEqual(["core-002", "core-002"]);
    // "A unit attacked a unit" names nothing, so it opens nothing.
    expect(screen.getByText("A unit attacked a unit").querySelector(`[data-testid="${LOG_CARD_TESTID}"]`)).toBeNull();
  });

  it("a line about a card on the board opens the face the board shows it with, and a radiantSet the radiant one", () => {
    vi.useFakeTimers();
    const view = fullBoardView();
    const radiantUnit = view.you.units.find((u) => u !== null && u.radiant);
    if (radiantUnit === undefined || radiantUnit === null) throw new Error("the fixture has a radiant unit");
    renderLog(
      withEvents(view, [
        { type: "buffed", instanceId: radiantUnit.instanceId, attack: 1, health: 1 },
        {
          type: "radiantSet",
          instanceId: "c5",
          defId: "core-005",
          zone: { z: "graveyard", player: "p1" },
        } as GameEvent,
      ]),
    );
    const [buffed, radiant] = cardLines();
    if (buffed === undefined || radiant === undefined) throw new Error("two card lines");
    fireEvent.pointerEnter(buffed, { pointerType: "mouse" });
    act(() => {
      vi.advanceTimersByTime(HOVER_DELAY_MS);
    });
    expect(screen.getByTestId(INSPECT_HOVER).querySelector(".cf")).toHaveAttribute("data-radiant-face", "true");
    fireEvent.pointerLeave(buffed, { pointerType: "mouse" });
    fireEvent.pointerEnter(radiant, { pointerType: "mouse" });
    act(() => {
      vi.advanceTimersByTime(HOVER_DELAY_MS);
    });
    expect(screen.getByTestId(INSPECT_HOVER)).toHaveTextContent(nameOf("core-005"));
    expect(screen.getByTestId(INSPECT_HOVER).querySelector(".cf")).toHaveAttribute("data-radiant-face", "true");
  });

  it("R1301 the log names a Chinese unit by its Chinese name", () => {
    const chinese = CHINESE["core-002"]?.name ?? "";
    const view = baseView({
      opponent: emptySide("p2", {
        units: [unit("p2", { instanceId: "u9", defId: "core-002", chinese: true }), unit("p2", { instanceId: "u8", defId: "core-002" }), null, null, null],
      }),
    });
    renderLog(
      withEvents(view, [
        // A translation gets no line of its own: the card's face says it.
        { type: "translated", instanceId: "u9" },
        { type: "damage", sourceId: null, targetId: "u9", amount: 2, combat: false },
        { type: "damage", sourceId: null, targetId: "u8", amount: 1, combat: false },
      ]),
    );
    const lines = cardLines();
    expect(lines.map((line) => line.textContent)).toEqual([`${chinese} took 2 damage`, `${nameOf("core-002")} took 1 damage`]);
    expect(chinese).not.toBe("");
    // The line opens the card as the board draws it: in Chinese.
    fireEvent.click(lines[0] ?? document.body);
    expect(screen.getByTestId(INSPECT_SHEET)).toHaveTextContent(chinese);
    expect(screen.getByTestId(INSPECT_SHEET)).not.toHaveTextContent(nameOf("core-002"));
  });
});

describe("R745 the log keeps the whole game, not only the view's window", () => {
  /** The engine's `VIEW_EVENT_LIMIT`: how many of the newest events a view carries (R168). */
  const WINDOW = 32;

  const lost = (n: number): GameEvent => ({ type: "healthLost", player: "p1", amount: n });
  const game = (count: number): GameEvent[] => Array.from({ length: count }, (_, i) => lost(i + 1));
  const windowOf = (events: readonly GameEvent[], end: number): GameEvent[] => events.slice(Math.max(0, end - WINDOW), end);
  const said = (n: number): string => `You lost ${String(n)} health`;
  const saidRange = (from: number, to: number): string[] => Array.from({ length: to - from + 1 }, (_, i) => said(from + i));
  const lines = (): string[] => [...screen.getByTestId(testid.log).querySelectorAll(".log-line")].map((li) => li.textContent ?? "");

  it("R745 joins windows that slide past the view's last 32 events, in order, with no line shown twice", () => {
    const events = game(80);
    const { rerender } = render(<Log view={withEvents(baseView(), windowOf(events, 8))} />);
    for (let end = 16; end <= 80; end += 8) {
      rerender(<Log view={withEvents(baseView(), windowOf(events, end))} />);
    }
    expect(lines()).toEqual(saidRange(1, 80));
  });

  it("R745 keeps a line that has left the window under the card name it was shown with, and stores no instance id", () => {
    const lookup = lookupFromDefs(CATALOG);
    const name = CATALOG["core-002"]?.name ?? "core-002";
    const first: GameEvent = { type: "summoned", player: "p2", instanceId: "c46", defId: "core-002", row: "units", lane: 1 };
    const second: GameEvent = { type: "attackDeclared", attackerId: "c46", targetId: "hero-p1", forced: false };
    const filler = game(WINDOW);
    const views = [
      withEvents(baseView(), [first, second]),
      withEvents(baseView(), [first, second, ...filler.slice(0, WINDOW - 2)]),
      withEvents(baseView(), filler),
    ];
    const [one, two, three] = views;
    if (one === undefined || two === undefined || three === undefined) throw new Error("three views");
    const at = (view: PlayerView): ReactElement => (
      <CatalogContext.Provider value={lookup}>
        <Log view={view} />
      </CatalogContext.Provider>
    );
    const { rerender } = render(at(one));
    rerender(at(two));
    rerender(at(three));

    const log = screen.getByTestId(testid.log);
    const [entered, attacked] = [...log.querySelectorAll(".log-line")];
    expect(entered?.textContent).toBe(`${name} entered the opponent's lane 1`);
    expect(attacked?.textContent).toBe(`${name} attacked your hero`);
    expect(within(log).getAllByTestId(LOG_CARD_TESTID).map((line) => line.getAttribute("data-def-id"))).toEqual([
      "core-002",
      "core-002",
    ]);
    expect(log.innerHTML).not.toMatch(/\bc46\b/);

    const history = views.reduce((h, view) => advanceLogHistory(h, view, keptLineFor(view, lookup)), EMPTY_LOG_HISTORY);
    const seat = history.seats.p1;
    expect(seat?.kept.map((line) => line.text)).toEqual([`${name} entered the opponent's lane 1`, `${name} attacked your hero`]);
    expect(seat?.kept.map((line) => line.face?.defId)).toEqual(["core-002", "core-002"]);
    expect(JSON.stringify(seat?.kept)).not.toContain("c46");
    expect(JSON.stringify(seat?.window.map((entry) => entry.line))).not.toContain("c46");
  });

  it("R745 keeps one history per viewer, and a new game starts with an empty one", () => {
    const events = game(40);
    const p2View = baseView({
      viewer: "p2",
      active: "p2",
      you: emptySide("p2"),
      opponent: emptySide("p1", { hand: { count: 4 } }),
    });
    const { rerender } = render(<Log view={withEvents(baseView(), windowOf(events, 20))} />);
    expect(lines()).toEqual(saidRange(1, 20));

    rerender(<Log view={withEvents(p2View, [{ type: "healthLost", player: "p1", amount: 500 }])} />);
    expect(lines()).toEqual(["Opponent lost 500 health"]);

    rerender(<Log view={withEvents(baseView(), windowOf(events, 40))} />);
    expect(lines()).toEqual(saidRange(1, 40));
    expect(lines().join("\n")).not.toContain("500");

    // A view with a result, then one without: the next game, with nothing of the last.
    rerender(<Log view={{ ...withEvents(baseView(), windowOf(events, 40)), result: { winner: "p1", reason: "concede" } }} />);
    rerender(<Log view={withEvents(baseView(), [lost(7)])} />);
    expect(lines()).toEqual([said(7)]);
  });

  it("R745 holds at most LOG_HISTORY_LIMIT lines, dropping the oldest first", () => {
    const total = LOG_HISTORY_LIMIT + 40;
    const events = game(total);
    let end = WINDOW;
    const { rerender } = render(<Log view={withEvents(baseView(), windowOf(events, end))} />);
    while (end < total) {
      end = Math.min(end + WINDOW - 1, total);
      rerender(<Log view={withEvents(baseView(), windowOf(events, end))} />);
    }
    const read = lines();
    expect(read).toHaveLength(LOG_HISTORY_LIMIT);
    expect(read[0]).toBe(said(total - LOG_HISTORY_LIMIT + 1));
    expect(read[read.length - 1]).toBe(said(total));
  });

  it("R745 draws a divider before each turn's first line but the log's first", () => {
    render(
      <Log
        view={withEvents(baseView(), [
          { type: "turnStarted", player: "p1", turn: 3 },
          lost(1),
          { type: "turnStarted", player: "p2", turn: 4 },
          lost(2),
        ])}
      />,
    );
    const dividers = screen.getAllByRole("separator");
    expect(dividers).toHaveLength(1);
    expect(dividers[0]?.nextElementSibling?.textContent).toBe("Turn 4: Opponent");
    expect(lines()).toEqual(["Turn 3: You", said(1), "Turn 4: Opponent", said(2)]);
  });

  it("R745 says in one line where a view shares no event with the one before, and keeps what it had", () => {
    const events = game(80);
    const { rerender } = render(<Log view={withEvents(baseView(), windowOf(events, 32))} />);
    rerender(<Log view={withEvents(baseView(), windowOf(events, 80))} />);
    expect(lines()).toEqual([...saidRange(1, 32), LOG_GAP_TEXT, ...saidRange(49, 80)]);
    expect(screen.getByText(LOG_GAP_TEXT)).toHaveAttribute("data-event", "gap");
  });
});

// #552 (MN09): the lines the Meditative play-through turned up, read as English. A line that opens on
// a card the viewer may not read opens with a capital; a hidden card turned into another hidden card
// says only that it changed; a craft prompt is a card to craft; and the lines about one card shown in
// Chinese name it in Chinese, whichever field the event names it by (R1301).
describe("#552 the log reads as English on the Meditative cards' events", () => {
  const lookup = lookupFromDefs(CATALOG);
  const nameOf = (defId: string): string => CATALOG[defId]?.name ?? defId;

  function lines(view: PlayerView): string[] {
    render(
      <CatalogContext.Provider value={lookup}>
        <Log view={view} />
      </CatalogContext.Provider>,
    );
    return [...screen.getByTestId(testid.log).querySelectorAll(".log-line")].map((li) => li.textContent ?? "");
  }

  it("R1320 a Nerf, a Buff and a changed number on a card the viewer may not read open with a capital", () => {
    expect(
      lines(
        withEvents(baseView(), [
          { type: "degraded", instanceId: "hidden", defId: "hidden", change: { kind: "none" } },
          { type: "upgraded", instanceId: "hidden", defId: "hidden", change: { kind: "none" } },
          { type: "numberChanged", instanceId: "hidden", defId: "hidden", key: "hidden", value: 0 },
          { type: "crumbled", instanceId: "hidden", defId: "hidden", owner: "p2", zone: "field" },
        ]),
      ),
    ).toEqual(["A card was nerfed", "A card was buffed", "A card changed", "A card crumbled"]);
  });

  it("R97 a hidden card turned into another hidden card says only that it was transformed, and a public one names both", () => {
    expect(
      lines(
        withEvents(baseView(), [
          { type: "transformed", instanceId: "hidden", fromDefId: "hidden", toDefId: "hidden", newInstanceId: "hidden" },
          { type: "transformed", instanceId: "c1", fromDefId: "meditative-037", toDefId: "core-005", newInstanceId: "c2" },
        ]),
      ),
    ).toEqual(["A hidden card was transformed", `${nameOf("meditative-037")} became ${nameOf("core-005")}`]);
  });

  it("R880 a craft prompt reads as a card to craft", () => {
    expect(lines(withEvents(baseView(), [{ type: "promptOpened", player: "p2", choiceId: "ch1", kind: "craft" }]))).toEqual([
      "Opponent must choose a card to craft",
    ]);
  });

  it("R1301 R1320 the lines about a unit shown in Chinese all name it in Chinese: its stat buff and its Buff", () => {
    const chinese = CHINESE["meditative-023"]?.name ?? "";
    const view = baseView({
      you: emptySide("p1", { units: [unit("p1", { instanceId: "u5", defId: "meditative-023", chinese: true }), null, null, null, null] }),
    });
    expect(chinese).not.toBe("");
    expect(
      lines(
        withEvents(view, [
          { type: "buffed", instanceId: "u5", attack: 2, health: 2 },
          { type: "upgraded", instanceId: "u5", defId: "meditative-023", change: { kind: "stats", attack: 1, health: 1 } },
          { type: "destroyed", instanceId: "u5", defId: "meditative-023", owner: "p1", controller: "p1", attack: 3, maxHealth: 5, killerId: null },
        ]),
      ),
    ).toEqual([`${chinese} gained +2/+2`, `${chinese} was buffed`, `${chinese} was destroyed`]);
  });
});
