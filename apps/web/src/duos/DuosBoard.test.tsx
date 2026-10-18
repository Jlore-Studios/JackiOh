// #936 S8 B61–B64: `DuosBoard`, the co-op board. It renders `Game.tsx` for the seat's own duel and
// wraps the board's moves as `{kind: "duel", body}`; a life strip shows own HP (`duos-own-hp`) and the
// pool (`duos-pool`) for both teams, the own bar giving way to the pool once spent; the pass control
// (`duos-pass`) sends `{kind: "pass", instanceId}` only when `legal` holds it; the portal
// (`duos-portal`) flips the board to the teammate's duel read-only; the teammate strip
// (`duos-teammate-strip`) always shows their HP, mana, hand count and whether they have ended; and the
// ping wheel (`duos-ping-wheel`) offers the five pings, a received one marking its card
// (`duos-ping-marker`) for PING_SHOW_MS.
//
// The fixtures are `DuosView`s built from the repo's own `PlayerView` fixtures (`test/fixtures.ts`).
// The client renders them and decides nothing (CLAUDE.md rule 7).

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi, type Mock } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { PlayerView } from "@jackioh/shared";
import { PING_SHOW_MS } from "@jackioh/server-config";

import { closeInspect } from "../cards/index.ts";
import { CatalogContext, lookupFromDefs } from "../game/catalog.ts";
import { testid } from "../game/contract.ts";
import type { PingKind, PingTarget } from "../net/duos.ts";
import { __resetSettingsForTests } from "../settings/store.ts";
import { baseView, card, emptySide, unit } from "../test/fixtures.ts";
import { must, showsNumber } from "../test/stubServer.ts";
import type { DuosActionBody } from "../wire/generated/DuosActionBody.ts";
import type { DuosView } from "../wire/generated/DuosView.ts";
import { DuosChromeContext, type DuosChrome } from "./context.ts";
import { DuosBoard, type DuosBoardProps } from "./DuosBoard.tsx";

type ReceivedPing = DuosBoardProps["pings"][number];

const LOOKUP = lookupFromDefs(CATALOG);
const PAST_ANY_INSPECT_DELAY_MS = 2_000;

const ID = {
  portal: "duos-portal",
  pass: "duos-pass",
  pool: "duos-pool",
  ownHp: "duos-own-hp",
  teammateStrip: "duos-teammate-strip",
  pingWheel: "duos-ping-wheel",
  pingMarker: "duos-ping-marker",
  round: "duos-round",
  ping: (kind: PingKind): string => `duos-ping-${kind}`,
} as const;

const PING_KINDS: readonly PingKind[] = ["thumbsUp", "thumbsDown", "portal", "careful", "killThis"];

// Distinct numbers, so each one shown can be told apart.
const OWN_HP = 14;
const TEAMMATE_HP = 11;
const ACROSS_HP = 9; // B1, across from A1 in duel 0
const B2_HP = 8;
const TEAM_POOL = 17;
const ENEMY_POOL = 13;
const POOL_CAP = 20;
const TEAMMATE_MANA = 2;
const TEAMMATE_HAND = 3;
const PASSES_LEFT = 8;
const PASS_COST = 1;

// ---------------------------------------------------------------------------------------------
// fixtures
// ---------------------------------------------------------------------------------------------

type Life = { pool: number; poolCap: number; spent: boolean };

function hero(health: number, duos: Life) {
  return { health, armor: 0, powers: [], power: null, duos };
}

/** A1's own duel (duel 0) from A1's seat: two hand cards, B1's unit in lane index 2. */
function ownDuel(over: Partial<PlayerView> = {}, spent = false): PlayerView {
  return baseView({
    viewer: "p1",
    active: "p1",
    turn: 5,
    autoEndTurn: false,
    you: emptySide("p1", {
      hero: hero(spent ? TEAM_POOL : OWN_HP, { pool: TEAM_POOL, poolCap: POOL_CAP, spent }),
      mana: { current: 5, max: 5 },
      hand: [card({ instanceId: "h1", defId: "core-002", cost: 1 }), card({ instanceId: "h2", defId: "core-019", cost: 3 })],
    }),
    opponent: emptySide("p2", {
      hero: hero(ACROSS_HP, { pool: ENEMY_POOL, poolCap: POOL_CAP, spent: false }),
      hand: { count: 4 },
      units: [null, null, unit("p2", { instanceId: "e1", defId: "core-008" }), null, null],
    }),
    ...over,
  });
}

/** A2's duel (duel 1) from A2's own seat: their three hand cards, B2's unit in lane index 0. */
function teammateDuel(): PlayerView {
  return baseView({
    viewer: "p1",
    active: "p1",
    turn: 5,
    autoEndTurn: false,
    you: emptySide("p1", {
      hero: hero(TEAMMATE_HP, { pool: TEAM_POOL, poolCap: POOL_CAP, spent: false }),
      mana: { current: TEAMMATE_MANA, max: 6 },
      hand: [
        card({ instanceId: "t1", defId: "core-055", cost: 0 }),
        card({ instanceId: "t2", defId: "core-005", cost: 2 }),
        card({ instanceId: "t3", defId: "core-007", cost: 4 }),
      ],
    }),
    opponent: emptySide("p2", {
      hero: hero(B2_HP, { pool: ENEMY_POOL, poolCap: POOL_CAP, spent: false }),
      hand: { count: 5 },
      units: [unit("p2", { instanceId: "e2", defId: "core-013" }), null, null, null, null],
    }),
  });
}

type ViewOptions = {
  seat?: DuosView["seat"];
  spent?: boolean;
  ended?: [boolean, boolean];
  roundSeats?: DuosView["roundSeats"];
};

/** A1's DuosView in round 3, unless told otherwise. */
function duosView(options: ViewOptions = {}): DuosView {
  const spent = options.spent ?? false;
  const ownSide = { own: spent ? 0 : OWN_HP, spent };
  const teammateSide = { own: TEAMMATE_HP, spent: false };
  const a2 = options.seat === "a2";
  return {
    seat: options.seat ?? "a1",
    own: ownDuel({}, spent),
    teammate: teammateDuel(),
    team: {
      team: "a",
      pool: TEAM_POOL,
      poolCap: POOL_CAP,
      sides: a2 ? [teammateSide, ownSide] : [ownSide, teammateSide],
      ended: options.ended ?? [false, false],
      passesLeft: a2 ? [10, PASSES_LEFT] : [PASSES_LEFT, 10],
      passCost: PASS_COST,
    },
    enemy: {
      team: "b",
      pool: ENEMY_POOL,
      poolCap: POOL_CAP,
      sides: a2
        ? [
            { own: B2_HP, spent: false },
            { own: ACROSS_HP, spent: false },
          ]
        : [
            { own: ACROSS_HP, spent: false },
            { own: B2_HP, spent: false },
          ],
      ended: [false, false],
      passesLeft: [0, 0],
      passCost: PASS_COST,
    },
    round: 3,
    roundSeats: options.roundSeats ?? ["a1", "a2"],
    concedeOfferedBy: null,
    result: null,
  };
}

/** Team B's win by concede: a1's team (team A) lost. */
const CONCEDED: DuosView["result"] = { outcome: "win", winner: "b", reason: "concede" };

const END_TURN: DuosActionBody = { kind: "duel", body: { type: "endTurn" } };
const PASS_H1: DuosActionBody = { kind: "pass", instanceId: "h1" };

// ---------------------------------------------------------------------------------------------
// rendering and reading the board
// ---------------------------------------------------------------------------------------------

type Board = {
  onAction: Mock<(body: DuosActionBody) => void>;
  onPing: Mock<(ping: PingKind, target: PingTarget) => void>;
  rerender: (over: Partial<DuosBoardProps>) => void;
};

function renderBoard(over: Partial<DuosBoardProps> = {}, chrome: DuosChrome | null = null): Board {
  const onAction = vi.fn<(body: DuosActionBody) => void>();
  const onPing = vi.fn<(ping: PingKind, target: PingTarget) => void>();
  const base: DuosBoardProps = { view: duosView(), legal: [], onAction, pings: [], onPing, error: null };
  const tree = (props: DuosBoardProps) => (
    <CatalogContext.Provider value={LOOKUP}>
      <DuosChromeContext.Provider value={chrome}>
        <DuosBoard {...props} />
      </DuosChromeContext.Provider>
    </CatalogContext.Provider>
  );
  let current: DuosBoardProps = { ...base, ...over };
  const result = render(tree(current));
  return {
    onAction,
    onPing,
    rerender: (next) => {
      current = { ...current, ...next };
      result.rerender(tree(current));
    },
  };
}

function sent(board: Board): DuosActionBody[] {
  return board.onAction.mock.calls.map(([body]) => body);
}

function passesSent(board: Board): DuosActionBody[] {
  return sent(board).filter((body) => body.kind === "pass");
}

function texts(id: string): string[] {
  return screen.queryAllByTestId(id).map((element) => `${element.textContent ?? ""} ${element.getAttribute("aria-label") ?? ""} ${element.getAttribute("title") ?? ""}`);
}

function anyShows(id: string, n: number): boolean {
  return texts(id).some((text) => showsNumber(text, n));
}

function usable(element: HTMLElement | null): boolean {
  if (element === null) return false;
  if ((element as HTMLButtonElement).disabled === true) return false;
  return element.getAttribute("aria-disabled") !== "true";
}

function flipPortal(): void {
  fireEvent.click(screen.getByTestId(ID.portal));
  act(() => {
    vi.advanceTimersByTime(0);
  });
}

/**
 * Bring up a control that lives "on a hovered/inspected card": a mouse hover, then a touch long-press,
 * then a right-click, then a plain click, stopping as soon as the control is on the page.
 */
function reveal(target: HTMLElement, id: string): HTMLElement | null {
  const now = (): HTMLElement | null => screen.queryAllByTestId(id)[0] ?? null;
  if (now() !== null) return now();
  fireEvent.pointerEnter(target, { pointerType: "mouse", clientX: 10, clientY: 10 });
  fireEvent.mouseEnter(target);
  act(() => {
    vi.advanceTimersByTime(PAST_ANY_INSPECT_DELAY_MS);
  });
  if (now() !== null) return now();
  fireEvent.pointerLeave(target, { pointerType: "mouse" });
  fireEvent.pointerDown(target, { pointerType: "touch", clientX: 10, clientY: 10, pointerId: 1, isPrimary: true, button: 0 });
  act(() => {
    vi.advanceTimersByTime(PAST_ANY_INSPECT_DELAY_MS);
  });
  fireEvent.pointerUp(target, { pointerType: "touch", clientX: 10, clientY: 10, pointerId: 1, isPrimary: true, button: 0 });
  if (now() !== null) return now();
  fireEvent.contextMenu(target);
  act(() => {
    vi.advanceTimersByTime(PAST_ANY_INSPECT_DELAY_MS);
  });
  if (now() !== null) return now();
  fireEvent.click(target);
  act(() => {
    vi.advanceTimersByTime(PAST_ANY_INSPECT_DELAY_MS);
  });
  return now();
}

/** If jsdom lacks `PointerEvent`, testing-library drops `pointerType`; keep it (as inspect.test.tsx does). */
function ensurePointerEvent(): void {
  const win = document.defaultView;
  if (win === null) return;
  const probe = typeof win.PointerEvent === "function" ? new win.PointerEvent("pointerdown", { pointerType: "touch" }) : null;
  if (probe !== null && probe.pointerType === "touch") return;
  class TestPointerEvent extends win.MouseEvent {
    readonly pointerType: string;
    readonly pointerId: number;
    readonly isPrimary: boolean;
    constructor(type: string, init: PointerEventInit = {}) {
      super(type, init);
      this.pointerType = init.pointerType ?? "";
      this.pointerId = init.pointerId ?? 1;
      this.isPrimary = init.isPrimary ?? true;
    }
  }
  for (const target of [win, globalThis]) {
    Object.defineProperty(target, "PointerEvent", { value: TestPointerEvent, configurable: true, writable: true });
  }
}

beforeAll(() => {
  ensurePointerEvent();
});

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  cleanup();
  closeInspect();
  vi.useRealTimers();
  window.localStorage.clear();
  __resetSettingsForTests();
});

// ---------------------------------------------------------------------------------------------
// B61: the own duel's board and the life strip
// ---------------------------------------------------------------------------------------------

describe("B61 the own duel and the life strip", () => {
  it("B61 renders the board for the seat's own duel: its own hand, not the teammate's", () => {
    renderBoard();
    expect(screen.getByTestId("game")).toBeInTheDocument();
    expect(screen.getByTestId(testid.handCard("h1"))).toBeInTheDocument();
    expect(screen.getByTestId(testid.handCard("h2"))).toBeInTheDocument();
    expect(screen.queryByTestId(testid.handCard("t1"))).toBeNull();
  });

  it("B61 the board's moves go out as {kind: \"duel\", body}", () => {
    const board = renderBoard({ legal: [END_TURN] });
    const endTurn = screen.getByTestId(testid.endTurn);
    expect(endTurn).not.toBeDisabled();
    fireEvent.click(endTurn);
    expect(board.onAction).toHaveBeenCalledWith(END_TURN);
  });

  it("B61 a move legal does not hold is not offered: End turn stays disabled and sends nothing", () => {
    const board = renderBoard({ legal: [PASS_H1] });
    const endTurn = screen.getByTestId(testid.endTurn);
    expect(endTurn).toBeDisabled();
    fireEvent.click(endTurn);
    expect(sent(board).some((body) => body.kind === "duel" && body.body.type === "endTurn")).toBe(false);
  });

  it("B61 the life strip shows the seat's own HP and its team's pool", () => {
    renderBoard();
    expect(anyShows(ID.ownHp, OWN_HP), `own HP ${String(OWN_HP)} in ${JSON.stringify(texts(ID.ownHp))}`).toBe(true);
    expect(anyShows(ID.pool, TEAM_POOL), `team pool in ${JSON.stringify(texts(ID.pool))}`).toBe(true);
  });

  it("B61 the life strip shows the enemy team's pool and own HP too", () => {
    renderBoard();
    expect(anyShows(ID.pool, ENEMY_POOL), `enemy pool in ${JSON.stringify(texts(ID.pool))}`).toBe(true);
    expect(anyShows(ID.ownHp, ACROSS_HP), `the enemy across's own HP in ${JSON.stringify(texts(ID.ownHp))}`).toBe(true);
  });

  it("B61 once the seat is spent its own bar gives way to the pool: no own-HP bar reads 0", () => {
    renderBoard({ view: duosView({ spent: true }) });
    for (const text of texts(ID.ownHp)) {
      expect(showsNumber(text, 0), `a spent seat's own bar still drawn: ${text}`).toBe(false);
    }
    expect(anyShows(ID.pool, TEAM_POOL)).toBe(true);
  });

  it("B61 seat a2's board shows its own HP and its team's pool as well", () => {
    renderBoard({ view: duosView({ seat: "a2" }) });
    expect(anyShows(ID.ownHp, OWN_HP)).toBe(true);
    expect(anyShows(ID.pool, TEAM_POOL)).toBe(true);
  });
});

// ---------------------------------------------------------------------------------------------
// B62: the pass control
// ---------------------------------------------------------------------------------------------

describe("B62 the pass control", () => {
  it("B62 on a hand card legal lets pass, duos-pass sends {kind: \"pass\", instanceId}", () => {
    const board = renderBoard({ legal: [PASS_H1, END_TURN] });
    const pass = must(reveal(screen.getByTestId(testid.handCard("h1")), ID.pass), "the pass control on h1");
    expect(usable(pass)).toBe(true);
    fireEvent.click(pass);
    expect(board.onAction).toHaveBeenCalledWith(PASS_H1);
    expect(passesSent(board)).toEqual([PASS_H1]);
  });

  it("B62 the pass control shows the pass's cost and the passes left", () => {
    renderBoard({ legal: [PASS_H1] });
    const pass = must(reveal(screen.getByTestId(testid.handCard("h1")), ID.pass), "the pass control on h1");
    const scope = pass.parentElement ?? pass;
    const words = `${scope.textContent ?? ""} ${pass.getAttribute("aria-label") ?? ""} ${pass.getAttribute("title") ?? ""}`;
    expect(showsNumber(words, PASSES_LEFT), `passes left in "${words}"`).toBe(true);
    expect(showsNumber(words, PASS_COST), `the cost in "${words}"`).toBe(true);
  });

  it("B62 a hand card legal does not let pass sends no pass for it", () => {
    const board = renderBoard({ legal: [PASS_H1] });
    const pass = reveal(screen.getByTestId(testid.handCard("h2")), ID.pass);
    if (pass !== null && usable(pass)) fireEvent.click(pass);
    expect(passesSent(board).some((body) => body.kind === "pass" && body.instanceId === "h2")).toBe(false);
  });

  it("B62 with no pass in legal (ended, out of passes or mana) the control sends nothing", () => {
    const board = renderBoard({ legal: [END_TURN] });
    const pass = reveal(screen.getByTestId(testid.handCard("h1")), ID.pass);
    if (pass !== null) {
      expect(usable(pass)).toBe(false);
      fireEvent.click(pass);
    }
    expect(passesSent(board)).toEqual([]);
  });

  it("B62 a pass legal names for a card not in the hand is not drawn on another card", () => {
    const board = renderBoard({ legal: [{ kind: "pass", instanceId: "h9" }] });
    for (const id of ["h1", "h2"]) {
      const pass = reveal(screen.getByTestId(testid.handCard(id)), ID.pass);
      if (pass !== null && usable(pass)) fireEvent.click(pass);
      closeInspect();
    }
    expect(passesSent(board).filter((body) => body.kind === "pass" && body.instanceId !== "h9")).toEqual([]);
  });

  it("B62 with legal empty no pass is offered on any hand card", () => {
    const board = renderBoard({ legal: [] });
    for (const id of ["h1", "h2"]) {
      const pass = reveal(screen.getByTestId(testid.handCard(id)), ID.pass);
      if (pass !== null) {
        expect(usable(pass), id).toBe(false);
        fireEvent.click(pass);
      }
      closeInspect();
    }
    expect(passesSent(board)).toEqual([]);
  });
});

// ---------------------------------------------------------------------------------------------
// B63: the portal, the teammate strip, waiting
// ---------------------------------------------------------------------------------------------

describe("B63 the portal", () => {
  it("B63 the portal flips the board to the teammate's duel, and back", () => {
    renderBoard({ legal: [END_TURN] });
    flipPortal();
    expect(screen.getByTestId(testid.handCard("t1"))).toBeInTheDocument();
    expect(screen.queryByTestId(testid.handCard("h1"))).toBeNull();
    flipPortal();
    expect(screen.getByTestId(testid.handCard("h1"))).toBeInTheDocument();
    expect(screen.queryByTestId(testid.handCard("t1"))).toBeNull();
  });

  it("B63 flipping the portal is not a move: it sends nothing", () => {
    const board = renderBoard({ legal: [END_TURN, PASS_H1] });
    const before = board.onAction.mock.calls.length;
    flipPortal();
    flipPortal();
    expect(board.onAction.mock.calls.length).toBe(before);
    expect(board.onPing).not.toHaveBeenCalled();
  });

  it("B63 the teammate's duel is read-only: no move is offered and nothing is sent", () => {
    const board = renderBoard({ legal: [END_TURN, PASS_H1] });
    flipPortal();
    const before = board.onAction.mock.calls.length;
    const endTurn = screen.queryByTestId(testid.endTurn);
    if (endTurn !== null) {
      expect(endTurn).toBeDisabled();
      fireEvent.click(endTurn);
    }
    fireEvent.click(screen.getByTestId(testid.handCard("t1")));
    fireEvent.click(screen.getByTestId(testid.card("e2")));
    act(() => {
      vi.advanceTimersByTime(PAST_ANY_INSPECT_DELAY_MS);
    });
    expect(board.onAction.mock.calls.length).toBe(before);
  });

  it("B63 the teammate's duel offers no pass, even with passes in legal", () => {
    const board = renderBoard({ legal: [END_TURN, PASS_H1] });
    flipPortal();
    const before = board.onAction.mock.calls.length;
    const pass = reveal(screen.getByTestId(testid.handCard("t1")), ID.pass);
    if (pass !== null && usable(pass)) fireEvent.click(pass);
    expect(board.onAction.mock.calls.length).toBe(before);
  });

  it("B63 the teammate strip shows the teammate's HP, mana and hand count, on both views", () => {
    renderBoard();
    for (const view of ["own", "teammate"]) {
      const strip = screen.getByTestId(ID.teammateStrip);
      const text = strip.textContent ?? "";
      expect(showsNumber(text, TEAMMATE_HP), `${view}: HP in "${text}"`).toBe(true);
      expect(showsNumber(text, TEAMMATE_MANA), `${view}: mana in "${text}"`).toBe(true);
      expect(showsNumber(text, TEAMMATE_HAND), `${view}: hand count in "${text}"`).toBe(true);
      if (view === "own") flipPortal();
    }
  });

  it("B63 the teammate strip shows whether the teammate has ended", () => {
    const board = renderBoard({ view: duosView({ ended: [false, false] }) });
    const playing = screen.getByTestId(ID.teammateStrip).outerHTML;
    board.rerender({ view: duosView({ ended: [false, true] }) });
    const ended = screen.getByTestId(ID.teammateStrip).outerHTML;
    expect(ended).not.toBe(playing);
  });

  it("B63 \"Waiting for your teammate\" shows after the seat has ended", () => {
    renderBoard({ view: duosView({ ended: [true, false] }) });
    expect(document.body).toHaveTextContent(/waiting for your teammate/i);
  });

  it("B63 no waiting notice while the seat has not ended", () => {
    renderBoard({ view: duosView({ ended: [false, true] }) });
    expect(document.body).not.toHaveTextContent(/waiting for your teammate/i);
  });

  it("R1168001 the round line and the waiting notice follow the round seats, not the duel's active player", () => {
    // A1's round turn ended as it began: duel 0 is already on B1's turn, a turn ahead of the round,
    // while A1 is still the round seat there, ended.
    const ahead = duosView({ ended: [true, false] });
    renderBoard({ view: { ...ahead, own: { ...ahead.own, active: "p2" } } });
    expect(screen.getByTestId(ID.round)).toHaveTextContent(/your team's turn/i);
    expect(document.body).toHaveTextContent(/waiting for your teammate/i);
  });

  it("R1168001 the teammate strip says the teammate is playing only while the round is theirs", () => {
    // An extra-turn round: A1 plays on in duel 0 while duel 1's round is B2's, though A2 is that
    // duel's active player for nothing it may do.
    renderBoard({ view: duosView({ roundSeats: ["a1", "b2"] }) });
    expect(screen.getByTestId(ID.teammateStrip).querySelector(".duos-teammate__turn")).toHaveAttribute(
      "data-state",
      "waiting",
    );
    expect(screen.getByTestId(ID.round)).toHaveTextContent(/your team's turn/i);
  });
});

// ---------------------------------------------------------------------------------------------
// B64: pings
// ---------------------------------------------------------------------------------------------

describe("B64 the ping wheel", () => {
  it("B64 no card inspected, no ping wheel", () => {
    renderBoard();
    expect(screen.queryByTestId(ID.pingWheel)).toBeNull();
  });

  it("B64 the wheel on an inspected card offers exactly the five pings", () => {
    renderBoard();
    const wheel = must(reveal(screen.getByTestId(testid.card("e1")), ID.pingWheel), "the ping wheel on e1");
    const offered = within(wheel)
      .queryAllByTestId(/^duos-ping-/)
      .map((element) => element.getAttribute("data-testid"))
      .filter((id) => id !== ID.pingWheel && id !== ID.pingMarker);
    expect(new Set(offered)).toEqual(new Set(PING_KINDS.map(ID.ping)));
  });

  it("B64 a ping on an enemy unit names its duel, side and 0-based lane", () => {
    const board = renderBoard();
    const wheel = must(reveal(screen.getByTestId(testid.card("e1")), ID.pingWheel), "the ping wheel on e1");
    fireEvent.click(within(wheel).getByTestId(ID.ping("killThis")));
    expect(board.onPing).toHaveBeenCalledWith("killThis", { duel: 0, at: "unit", side: "p2", lane: 2 });
  });

  it("B64 a ping on the seat's own hand card names the hand side and index", () => {
    const board = renderBoard();
    const wheel = must(reveal(screen.getByTestId(testid.handCard("h2")), ID.pingWheel), "the ping wheel on h2");
    fireEvent.click(within(wheel).getByTestId(ID.ping("thumbsUp")));
    expect(board.onPing).toHaveBeenCalledWith("thumbsUp", { duel: 0, at: "hand", side: "p1", index: 1 });
  });

  it("B64 seat a2's pings name duel 1, its own duel", () => {
    const board = renderBoard({ view: duosView({ seat: "a2" }) });
    const wheel = must(reveal(screen.getByTestId(testid.card("e1")), ID.pingWheel), "the ping wheel on e1");
    fireEvent.click(within(wheel).getByTestId(ID.ping("careful")));
    expect(board.onPing).toHaveBeenCalledWith("careful", { duel: 1, at: "unit", side: "p2", lane: 2 });
  });

  it("B64 opening the wheel sends no ping until a kind is picked", () => {
    const board = renderBoard();
    must(reveal(screen.getByTestId(testid.card("e1")), ID.pingWheel), "the ping wheel on e1");
    expect(board.onPing).not.toHaveBeenCalled();
  });

  it("B64 a ping is never an action: pinging sends nothing through onAction", () => {
    const board = renderBoard({ legal: [END_TURN, PASS_H1] });
    const before = board.onAction.mock.calls.length;
    const wheel = must(reveal(screen.getByTestId(testid.card("e1")), ID.pingWheel), "the ping wheel on e1");
    for (const kind of PING_KINDS) {
      const button = within(wheel).queryByTestId(ID.ping(kind));
      if (button !== null) fireEvent.click(button);
    }
    expect(board.onPing).toHaveBeenCalled();
    expect(board.onAction.mock.calls.length).toBe(before);
  });
});

describe("B64 received pings", () => {
  function ping(target: PingTarget, ageMs = 0, kind: PingKind = "careful"): ReceivedPing {
    return { from: "a2", ping: kind, target, at: Date.now() - ageMs };
  }

  const ON_E1: PingTarget = { duel: 0, at: "unit", side: "p2", lane: 2 };
  const ON_E2: PingTarget = { duel: 1, at: "unit", side: "p2", lane: 0 };

  it("B64 no ping received, no marker", () => {
    renderBoard();
    expect(screen.queryByTestId(ID.pingMarker)).toBeNull();
  });

  it("B64 a received ping marks its card", () => {
    renderBoard({ pings: [ping(ON_E1)] });
    expect(screen.getByTestId(ID.pingMarker)).toBeInTheDocument();
  });

  it("B64 the mark stays for PING_SHOW_MS and then goes", () => {
    renderBoard({ pings: [ping(ON_E1)] });
    act(() => {
      vi.advanceTimersByTime(Math.floor(PING_SHOW_MS / 2));
    });
    expect(screen.getByTestId(ID.pingMarker)).toBeInTheDocument();
    act(() => {
      vi.advanceTimersByTime(PING_SHOW_MS);
    });
    expect(screen.queryByTestId(ID.pingMarker)).toBeNull();
  });

  it("B64 a ping older than PING_SHOW_MS marks nothing", () => {
    renderBoard({ pings: [ping(ON_E1, PING_SHOW_MS + 1_000)] });
    expect(screen.queryByTestId(ID.pingMarker)).toBeNull();
  });

  it("B64 a ping on a hand card of the seat's own marks it too", () => {
    renderBoard({ pings: [ping({ duel: 0, at: "hand", side: "p1", index: 0 }, 0, "thumbsUp")] });
    expect(screen.getByTestId(ID.pingMarker)).toBeInTheDocument();
  });

  it("B64 a ping on the teammate's duel marks its card on the portal's view", () => {
    renderBoard({ pings: [ping(ON_E2, 0, "killThis")] });
    flipPortal();
    expect(screen.getByTestId(testid.card("e2"))).toBeInTheDocument();
    expect(screen.getByTestId(ID.pingMarker)).toBeInTheDocument();
  });

  it("B64 a ping arriving after the board is up marks its card", () => {
    const board = renderBoard();
    expect(screen.queryByTestId(ID.pingMarker)).toBeNull();
    board.rerender({ pings: [ping(ON_E1)] });
    expect(screen.getByTestId(ID.pingMarker)).toBeInTheDocument();
  });
});

// ---------------------------------------------------------------------------------------------
// #965: the result panel's replay door, and the board's replay mode
// ---------------------------------------------------------------------------------------------

describe("#965 the replay door and the board's replay mode", () => {
  const OVER: DuosView = { ...duosView(), result: CONCEDED };

  it("#965 the result panel offers Watch the replay when the route hangs it on the board, and it calls it", () => {
    const onWatchReplay = vi.fn<() => void>();
    renderBoard({ view: OVER }, { onWatchReplay });
    fireEvent.click(screen.getByTestId("duos-result-replay"));
    expect(onWatchReplay).toHaveBeenCalledTimes(1);
  });

  it("#965 no replay door without one, as on /dev/duos", () => {
    renderBoard({ view: OVER });
    expect(screen.getByTestId("duos-result")).toBeInTheDocument();
    expect(screen.queryByTestId("duos-result-replay")).toBeNull();
  });

  it("#965 replay mode offers nothing live, even with moves in legal", () => {
    const board = renderBoard({
      replay: true,
      view: duosView({ ended: [true, false] }),
      legal: [END_TURN, PASS_H1, { kind: "concede" }],
    });
    expect(screen.getByTestId("game")).toHaveAttribute("data-replay", "true");
    const endTurn = screen.queryByTestId(testid.endTurn);
    if (endTurn !== null) {
      expect(endTurn).toBeDisabled();
      fireEvent.click(endTurn);
    }
    expect(screen.queryByTestId("duos-concede")).toBeNull();
    expect(document.body).not.toHaveTextContent(/waiting for your teammate/i);
    const h1 = screen.getByTestId(testid.handCard("h1"));
    expect(reveal(h1, ID.pingWheel)).toBeNull();
    expect(reveal(h1, ID.pass)).toBeNull();
    expect(board.onAction).not.toHaveBeenCalled();
    expect(board.onPing).not.toHaveBeenCalled();
  });

  it("#965 replay mode draws no result panel over the last step; the rail names the team's result", () => {
    renderBoard({ replay: true, view: OVER }, { onWatchReplay: vi.fn<() => void>() });
    expect(screen.queryByTestId("duos-result")).toBeNull();
    expect(screen.getByTestId("duos-round")).toHaveTextContent("Your team lost");
  });

  it("#965 the portal still shows the teammate's duel in replay mode", () => {
    renderBoard({ replay: true });
    flipPortal();
    expect(screen.getByTestId(testid.handCard("t1"))).toBeInTheDocument();
    expect(screen.queryByTestId(testid.handCard("h1"))).toBeNull();
  });
});
