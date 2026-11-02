// #936 S8 B61–B64: `DuosBoard`, the co-op board. The fixtures are `DuosView`s built from the repo's own
// `PlayerView` fixtures (`test/fixtures.ts`); the client renders them and decides nothing (CLAUDE.md rule 7).

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, onTestFinished, vi, type Mock } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { GameEvent, PlayerView } from "@jackioh/shared";
import { PING_SHOW_MS } from "@jackioh/server-config";

import { closeInspect } from "../cards/index.ts";
import { createSoundDirector } from "../audio/director.ts";
import { CatalogContext, lookupFromDefs } from "../game/catalog.ts";
import { testid } from "../game/contract.ts";
import { DRAG_THRESHOLD_PX } from "../game/drag/model.ts";
import { modifierListTestid } from "../game/ModifierList.tsx";
import type { PingKind, PingTarget } from "../net/duos.ts";
import { __resetSettingsForTests } from "../settings/store.ts";
import { baseView, card, emptySide, opponentModifiers, unit, withEvents, yourModifiers } from "../test/fixtures.ts";
import { must, showsNumber } from "../test/stubServer.ts";
import type { DuosActionBody } from "../wire/generated/DuosActionBody.ts";
import type { DuosView } from "../wire/generated/DuosView.ts";
import { DUOS_BOTH_KEY } from "./config.ts";
import { DuosChromeContext, type DuosChrome, type DuosPlayAgain } from "./context.ts";
import { DuosBoard, type DuosBoardProps } from "./DuosBoard.tsx";

vi.mock("../audio/director.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../audio/director.ts")>();
  return {
    ...actual,
    createSoundDirector: vi.fn(() => ({ onView: vi.fn(), onEntryStart: vi.fn(), onIdle: vi.fn() })),
  };
});

type ReceivedPing = DuosBoardProps["pings"][number];

const LOOKUP = lookupFromDefs(CATALOG);
const PAST_ANY_INSPECT_DELAY_MS = 2_000;

const ID = {
  portal: "duos-portal",
  both: "duos-both",
  boardOwn: "duos-board-own",
  boardTeammate: "duos-board-teammate",
  pass: "duos-pass",
  pool: "duos-pool",
  ownHp: "duos-own-hp",
  teammateStrip: "duos-teammate-strip",
  pingWheel: "duos-ping-wheel",
  pingMarker: "duos-ping-marker",
  round: "duos-round",
  lifeStrip: "duos-life-strip",
  concede: "duos-concede",
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

// fixtures

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

// rendering and reading the board

type Board = {
  onAction: Mock<(body: DuosActionBody) => void>;
  onPing: Mock<(ping: PingKind, target: PingTarget) => void>;
  rerender: (over: Partial<DuosBoardProps>) => void;
};

function renderDuosBoard(over: Partial<DuosBoardProps> = {}, chrome: DuosChrome | null = null): Board {
  const onAction = vi.fn<(body: DuosActionBody) => void>();
  const onPing = vi.fn<(ping: PingKind, target: PingTarget) => void>();
  const base: DuosBoardProps = { view: duosView(), legal: [], onAction, pings: [], onPing, error: null, jloreCapped: false };
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

function toggleBoth(): void {
  fireEvent.click(screen.getByTestId(ID.both));
  act(() => {
    vi.advanceTimersByTime(0);
  });
}

/** jsdom has no `document.elementFromPoint`, which the drop reads on release to find its target:
 * stand one in that always answers `target`, and give back what restores the document. */
function pointAt(target: Element): () => void {
  const own = Object.getOwnPropertyDescriptor(document, "elementFromPoint");
  Object.defineProperty(document, "elementFromPoint", { configurable: true, value: () => target });
  return () => {
    if (own === undefined) Reflect.deleteProperty(document, "elementFromPoint");
    else Object.defineProperty(document, "elementFromPoint", own);
  };
}

/** Bring up a control that lives "on a hovered/inspected card", by whichever gesture shows it first. */
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

describe("B61 the own duel and the life strip", () => {
  it("B61 renders the board for the seat's own duel: its own hand, not the teammate's", () => {
    renderDuosBoard();
    expect(screen.getByTestId("game")).toBeInTheDocument();
    expect(screen.getByTestId(testid.handCard("h1"))).toBeInTheDocument();
    expect(screen.getByTestId(testid.handCard("h2"))).toBeInTheDocument();
    expect(screen.queryByTestId(testid.handCard("t1"))).toBeNull();
  });

  it("B61 the board's moves go out as {kind: \"duel\", body}", () => {
    const board = renderDuosBoard({ legal: [END_TURN] });
    const endTurn = screen.getByTestId(testid.endTurn);
    expect(endTurn).not.toBeDisabled();
    fireEvent.click(endTurn);
    expect(board.onAction).toHaveBeenCalledWith(END_TURN);
  });

  it("B61 a move legal does not hold is not offered: End turn stays disabled and sends nothing", () => {
    const board = renderDuosBoard({ legal: [PASS_H1] });
    const endTurn = screen.getByTestId(testid.endTurn);
    expect(endTurn).toBeDisabled();
    fireEvent.click(endTurn);
    expect(sent(board).some((body) => body.kind === "duel" && body.body.type === "endTurn")).toBe(false);
  });

  it("B61 the life strip shows the seat's own HP and its team's pool", () => {
    renderDuosBoard();
    expect(anyShows(ID.ownHp, OWN_HP), `own HP ${String(OWN_HP)} in ${JSON.stringify(texts(ID.ownHp))}`).toBe(true);
    expect(anyShows(ID.pool, TEAM_POOL), `team pool in ${JSON.stringify(texts(ID.pool))}`).toBe(true);
  });

  it("B61 the life strip shows the enemy team's pool and own HP too", () => {
    renderDuosBoard();
    expect(anyShows(ID.pool, ENEMY_POOL), `enemy pool in ${JSON.stringify(texts(ID.pool))}`).toBe(true);
    expect(anyShows(ID.ownHp, ACROSS_HP), `the enemy across's own HP in ${JSON.stringify(texts(ID.ownHp))}`).toBe(true);
  });

  it("B61 once the seat is spent its own bar gives way to the pool: no own-HP bar reads 0", () => {
    renderDuosBoard({ view: duosView({ spent: true }) });
    for (const text of texts(ID.ownHp)) {
      expect(showsNumber(text, 0), `a spent seat's own bar still drawn: ${text}`).toBe(false);
    }
    expect(anyShows(ID.pool, TEAM_POOL)).toBe(true);
  });

  it("B61 seat a2's board shows its own HP and its team's pool as well", () => {
    renderDuosBoard({ view: duosView({ seat: "a2" }) });
    expect(anyShows(ID.ownHp, OWN_HP)).toBe(true);
    expect(anyShows(ID.pool, TEAM_POOL)).toBe(true);
  });
});

// B62: the pass control
describe("B62 the pass control", () => {
  it("B62 on a hand card legal lets pass, duos-pass sends {kind: \"pass\", instanceId}", () => {
    const board = renderDuosBoard({ legal: [PASS_H1, END_TURN] });
    const pass = must(reveal(screen.getByTestId(testid.handCard("h1")), ID.pass), "the pass control on h1");
    expect(usable(pass)).toBe(true);
    fireEvent.click(pass);
    expect(board.onAction).toHaveBeenCalledWith(PASS_H1);
    expect(passesSent(board)).toEqual([PASS_H1]);
  });

  it("B62 the pass control shows the pass's cost and the passes left", () => {
    renderDuosBoard({ legal: [PASS_H1] });
    const pass = must(reveal(screen.getByTestId(testid.handCard("h1")), ID.pass), "the pass control on h1");
    const scope = pass.parentElement ?? pass;
    const words = `${scope.textContent ?? ""} ${pass.getAttribute("aria-label") ?? ""} ${pass.getAttribute("title") ?? ""}`;
    expect(showsNumber(words, PASSES_LEFT), `passes left in "${words}"`).toBe(true);
    expect(showsNumber(words, PASS_COST), `the cost in "${words}"`).toBe(true);
  });

  it("B62 a hand card legal does not let pass sends no pass for it", () => {
    const board = renderDuosBoard({ legal: [PASS_H1] });
    const pass = reveal(screen.getByTestId(testid.handCard("h2")), ID.pass);
    if (pass !== null && usable(pass)) fireEvent.click(pass);
    expect(passesSent(board).some((body) => body.kind === "pass" && body.instanceId === "h2")).toBe(false);
  });

  it("B62 with no pass in legal (ended, out of passes or mana) the control sends nothing", () => {
    const board = renderDuosBoard({ legal: [END_TURN] });
    const pass = reveal(screen.getByTestId(testid.handCard("h1")), ID.pass);
    if (pass !== null) {
      expect(usable(pass)).toBe(false);
      fireEvent.click(pass);
    }
    expect(passesSent(board)).toEqual([]);
  });

  it("B62 a pass legal names for a card not in the hand is not drawn on another card", () => {
    const board = renderDuosBoard({ legal: [{ kind: "pass", instanceId: "h9" }] });
    for (const id of ["h1", "h2"]) {
      const pass = reveal(screen.getByTestId(testid.handCard(id)), ID.pass);
      if (pass !== null && usable(pass)) fireEvent.click(pass);
      closeInspect();
    }
    expect(passesSent(board).filter((body) => body.kind === "pass" && body.instanceId !== "h9")).toEqual([]);
  });

  it("B62 with legal empty no pass is offered on any hand card", () => {
    const board = renderDuosBoard({ legal: [] });
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

// B63: the portal, the teammate strip, waiting

describe("B63 the portal", () => {
  it("B63 the portal flips the board to the teammate's duel, and back", () => {
    renderDuosBoard({ legal: [END_TURN] });
    flipPortal();
    expect(screen.getByTestId(testid.handCard("t1"))).toBeInTheDocument();
    expect(screen.queryByTestId(testid.handCard("h1"))).toBeNull();
    flipPortal();
    expect(screen.getByTestId(testid.handCard("h1"))).toBeInTheDocument();
    expect(screen.queryByTestId(testid.handCard("t1"))).toBeNull();
  });

  it("B63 flipping the portal is not a move: it sends nothing", () => {
    const board = renderDuosBoard({ legal: [END_TURN, PASS_H1] });
    const before = board.onAction.mock.calls.length;
    flipPortal();
    flipPortal();
    expect(board.onAction.mock.calls.length).toBe(before);
    expect(board.onPing).not.toHaveBeenCalled();
  });

  it("B63 the teammate's duel is read-only: no move is offered and nothing is sent", () => {
    const board = renderDuosBoard({ legal: [END_TURN, PASS_H1] });
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
    const board = renderDuosBoard({ legal: [END_TURN, PASS_H1] });
    flipPortal();
    const before = board.onAction.mock.calls.length;
    const pass = reveal(screen.getByTestId(testid.handCard("t1")), ID.pass);
    if (pass !== null && usable(pass)) fireEvent.click(pass);
    expect(board.onAction.mock.calls.length).toBe(before);
  });

  it("B63 the teammate strip shows the teammate's HP, mana and hand count, on both views", () => {
    renderDuosBoard();
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
    const board = renderDuosBoard({ view: duosView({ ended: [false, false] }) });
    const playing = screen.getByTestId(ID.teammateStrip).outerHTML;
    board.rerender({ view: duosView({ ended: [false, true] }) });
    const ended = screen.getByTestId(ID.teammateStrip).outerHTML;
    expect(ended).not.toBe(playing);
  });

  it("B63 \"Waiting for your teammate\" shows after the seat has ended", () => {
    renderDuosBoard({ view: duosView({ ended: [true, false] }) });
    expect(document.body).toHaveTextContent(/waiting for your teammate/i);
  });

  it("B63 no waiting notice while the seat has not ended", () => {
    renderDuosBoard({ view: duosView({ ended: [false, true] }) });
    expect(document.body).not.toHaveTextContent(/waiting for your teammate/i);
  });

  it("R1168001 the round line and the waiting notice follow the round seats, not the duel's active player", () => {
    // A1's round turn ended as it began: duel 0 is already on B1's turn, a turn ahead of the round,
    // while A1 is still the round seat there, ended.
    const ahead = duosView({ ended: [true, false] });
    renderDuosBoard({ view: { ...ahead, own: { ...ahead.own, active: "p2" } } });
    expect(screen.getByTestId(ID.round)).toHaveTextContent(/your team's turn/i);
    expect(document.body).toHaveTextContent(/waiting for your teammate/i);
  });

  it("R1168001 the teammate strip says the teammate is playing only while the round is theirs", () => {
    // An extra-turn round: A1 plays on in duel 0 while duel 1's round is B2's, though A2 is that
    // duel's active player for nothing it may do.
    renderDuosBoard({ view: duosView({ roundSeats: ["a1", "b2"] }) });
    expect(screen.getByTestId(ID.teammateStrip).querySelector(".duos-teammate__turn")).toHaveAttribute(
      "data-state",
      "waiting",
    );
    expect(screen.getByTestId(ID.round)).toHaveTextContent(/your team's turn/i);
  });
});

describe("B64 the ping wheel", () => {
  it("B64 no card inspected, no ping wheel", () => {
    renderDuosBoard();
    expect(screen.queryByTestId(ID.pingWheel)).toBeNull();
  });

  it("B64 the wheel on an inspected card offers exactly the five pings", () => {
    renderDuosBoard();
    const wheel = must(reveal(screen.getByTestId(testid.card("e1")), ID.pingWheel), "the ping wheel on e1");
    const offered = within(wheel)
      .queryAllByTestId(/^duos-ping-/)
      .map((element) => element.getAttribute("data-testid"))
      .filter((id) => id !== ID.pingWheel && id !== ID.pingMarker);
    expect(new Set(offered)).toEqual(new Set(PING_KINDS.map(ID.ping)));
  });

  it("B64 a ping on an enemy unit names its duel, side and 0-based lane", () => {
    const board = renderDuosBoard();
    const wheel = must(reveal(screen.getByTestId(testid.card("e1")), ID.pingWheel), "the ping wheel on e1");
    fireEvent.click(within(wheel).getByTestId(ID.ping("killThis")));
    expect(board.onPing).toHaveBeenCalledWith("killThis", { duel: 0, at: "unit", side: "p2", lane: 2 });
  });

  it("B64 a ping on the seat's own hand card names the hand side and index", () => {
    const board = renderDuosBoard();
    const wheel = must(reveal(screen.getByTestId(testid.handCard("h2")), ID.pingWheel), "the ping wheel on h2");
    fireEvent.click(within(wheel).getByTestId(ID.ping("thumbsUp")));
    expect(board.onPing).toHaveBeenCalledWith("thumbsUp", { duel: 0, at: "hand", side: "p1", index: 1 });
  });

  it("B64 seat a2's pings name duel 1, its own duel", () => {
    const board = renderDuosBoard({ view: duosView({ seat: "a2" }) });
    const wheel = must(reveal(screen.getByTestId(testid.card("e1")), ID.pingWheel), "the ping wheel on e1");
    fireEvent.click(within(wheel).getByTestId(ID.ping("careful")));
    expect(board.onPing).toHaveBeenCalledWith("careful", { duel: 1, at: "unit", side: "p2", lane: 2 });
  });

  it("B64 opening the wheel sends no ping until a kind is picked", () => {
    const board = renderDuosBoard();
    must(reveal(screen.getByTestId(testid.card("e1")), ID.pingWheel), "the ping wheel on e1");
    expect(board.onPing).not.toHaveBeenCalled();
  });

  it("B64 a ping is never an action: pinging sends nothing through onAction", () => {
    const board = renderDuosBoard({ legal: [END_TURN, PASS_H1] });
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
    renderDuosBoard();
    expect(screen.queryByTestId(ID.pingMarker)).toBeNull();
  });

  it("B64 a received ping marks its card", () => {
    renderDuosBoard({ pings: [ping(ON_E1)] });
    expect(screen.getByTestId(ID.pingMarker)).toBeInTheDocument();
  });

  it("B64 the mark stays for PING_SHOW_MS and then goes", () => {
    renderDuosBoard({ pings: [ping(ON_E1)] });
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
    renderDuosBoard({ pings: [ping(ON_E1, PING_SHOW_MS + 1_000)] });
    expect(screen.queryByTestId(ID.pingMarker)).toBeNull();
  });

  it("B64 a ping on a hand card of the seat's own marks it too", () => {
    renderDuosBoard({ pings: [ping({ duel: 0, at: "hand", side: "p1", index: 0 }, 0, "thumbsUp")] });
    expect(screen.getByTestId(ID.pingMarker)).toBeInTheDocument();
  });

  it("B64 a ping on the teammate's duel marks its card on the portal's view", () => {
    renderDuosBoard({ pings: [ping(ON_E2, 0, "killThis")] });
    flipPortal();
    expect(screen.getByTestId(testid.card("e2"))).toBeInTheDocument();
    expect(screen.getByTestId(ID.pingMarker)).toBeInTheDocument();
  });

  it("B64 a ping arriving after the board is up marks its card", () => {
    const board = renderDuosBoard();
    expect(screen.queryByTestId(ID.pingMarker)).toBeNull();
    board.rerender({ pings: [ping(ON_E1)] });
    expect(screen.getByTestId(ID.pingMarker)).toBeInTheDocument();
  });
});

// #965: the result panel's replay door, and the board's replay mode
describe("#965 the replay door and the board's replay mode", () => {
  const OVER: DuosView = { ...duosView(), result: CONCEDED };

  it("#965 the result panel offers Watch the replay when the route hangs it on the board, and it calls it", () => {
    const onWatchReplay = vi.fn<() => void>();
    renderDuosBoard({ view: OVER }, { onWatchReplay });
    fireEvent.click(screen.getByTestId("duos-result-replay"));
    expect(onWatchReplay).toHaveBeenCalledTimes(1);
  });

  it("#965 no replay door without one, as on /dev/duos", () => {
    renderDuosBoard({ view: OVER });
    expect(screen.getByTestId("duos-result")).toBeInTheDocument();
    expect(screen.queryByTestId("duos-result-replay")).toBeNull();
  });

  it("#965 replay mode offers nothing live, even with moves in legal", () => {
    const board = renderDuosBoard({
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
    renderDuosBoard({ replay: true, view: OVER }, { onWatchReplay: vi.fn<() => void>() });
    expect(screen.queryByTestId("duos-result")).toBeNull();
    expect(screen.getByTestId("duos-round")).toHaveTextContent("Your team lost");
  });

  it("#965 the portal still shows the teammate's duel in replay mode", () => {
    renderDuosBoard({ replay: true });
    flipPortal();
    expect(screen.getByTestId(testid.handCard("t1"))).toBeInTheDocument();
    expect(screen.queryByTestId(testid.handCard("h1"))).toBeNull();
  });
});

// R1483000: the result panel names the daily cap for a capped seat
describe("R1483000 the result panel names the daily cap only for a capped seat", () => {
  const OVER: DuosView = { ...duosView(), result: CONCEDED };

  it("R1483000 the result panel names the daily cap only for a capped seat", () => {
    renderDuosBoard({ view: OVER, jloreCapped: true });
    expect(screen.getByTestId("duos-result")).toBeInTheDocument();
    expect(screen.getByTestId("duos-result-jlore")).toHaveTextContent("Daily Jlore reached");
    cleanup();

    renderDuosBoard({ view: OVER, jloreCapped: false });
    expect(screen.getByTestId("duos-result")).toBeInTheDocument();
    expect(screen.queryByTestId("duos-result-jlore")).toBeNull();
  });
});

// R169: the live-modifier chip in the rail
describe("R169 the modifier list on the co-op board", () => {
  const TEAMMATE_MODIFIER = { id: "m9", label: "Your Spells cost (1) less" };

  it("R169 the chip counts the seat's own duel and opens it under You and Opponent, never AI", () => {
    const base = duosView();
    const view: DuosView = {
      ...base,
      own: {
        ...base.own,
        you: { ...base.own.you, modifiers: yourModifiers },
        opponent: { ...base.own.opponent, modifiers: opponentModifiers },
      },
      teammate: { ...base.teammate, you: { ...base.teammate.you, modifiers: [TEAMMATE_MODIFIER] } },
    };
    renderDuosBoard({ view });

    const chip = screen.getByTestId(modifierListTestid.chip);
    expect(chip, "the teammate's duel is no part of the count").toHaveAttribute("data-count", "3");

    fireEvent.click(chip);
    const panel = screen.getByTestId(modifierListTestid.panel);
    for (const modifier of [...yourModifiers, ...opponentModifiers]) {
      expect(panel).toHaveTextContent(modifier.label);
    }
    expect(panel).not.toHaveTextContent(TEAMMATE_MODIFIER.label);
    expect(Array.from(panel.querySelectorAll(".modifier-list__who"), (who) => who.textContent)).toEqual(["You", "Opponent"]);
  });

  it("R169 no chip while the own duel has no modifier, though the teammate's has one", () => {
    const base = duosView();
    const view: DuosView = {
      ...base,
      teammate: { ...base.teammate, you: { ...base.teammate.you, modifiers: [TEAMMATE_MODIFIER] } },
    };
    renderDuosBoard({ view });
    expect(screen.queryByTestId(modifierListTestid.chip)).toBeNull();
  });
});

// #1664: the result panel plays the seat's economy
describe("#1664 the result panel plays what the game paid the seat's person", () => {
  const OVER: DuosView = { ...duosView(), result: CONCEDED };
  const economy = {
    jlore: 30,
    xp: 540,
    jloreCapped: false,
    tracks: [
      {
        setId: "Core" as const,
        before: { level: 1, xp: 60, levelXp: 100, xpToNext: 40 },
        after: { level: 2, xp: 130, levelXp: 200, xpToNext: 170 },
        levels: [{ level: 2, rewards: { jlore: 25, setPacks: 2, cardTokens: 3 } }],
      },
    ],
    quests: [{ questId: "win-3", title: "Win 3 games", jlore: 80, xp: 4000 }],
  };

  it("#1664 lists the tracks and quests and tells the pill once, only for an economy", () => {
    let announced = 0;
    const onChanged = (): void => {
      announced += 1;
    };
    window.addEventListener("jackioh:wallet-changed", onChanged);
    onTestFinished(() => {
      window.removeEventListener("jackioh:wallet-changed", onChanged);
    });

    const board = renderDuosBoard({ view: OVER });
    expect(screen.getByTestId("duos-result")).toBeInTheDocument();
    expect(screen.queryByTestId("xp-reveal")).toBeNull();
    expect(announced).toBe(0);

    board.rerender({ economy });
    expect(screen.getByTestId("xp-reveal-bar-Core")).toHaveTextContent("Core track");
    expect(screen.getByTestId("xp-reveal-quest-win-3")).toHaveTextContent("Win 3 games");
    expect(announced).toBe(1);

    // Each view of the seat carries it again: the bars play on, the pill is told once.
    board.rerender({ economy: { ...economy } });
    expect(screen.getAllByTestId("xp-reveal")).toHaveLength(1);
    expect(announced).toBe(1);
  });
});

// #1755: the result panel takes the focus, folds to the rail's Result and opens again
describe("#1755 the result panel takes the focus and opens again", () => {
  const OVER: DuosView = { ...duosView(), result: CONCEDED };
  const ECONOMY = {
    jlore: 30,
    xp: 540,
    jloreCapped: false,
    tracks: [
      {
        setId: "Core" as const,
        before: { level: 1, xp: 60, levelXp: 100, xpToNext: 40 },
        after: { level: 2, xp: 130, levelXp: 200, xpToNext: 170 },
        levels: [{ level: 2, rewards: { jlore: 25, setPacks: 2, cardTokens: 3 } }],
      },
    ],
    quests: [{ questId: "win-3", title: "Win 3 games", jlore: 80, xp: 4000 }],
  };

  it("#1755 the panel takes the focus on its first way on as it comes in", () => {
    renderDuosBoard({ view: OVER }, { onWatchReplay: vi.fn<() => void>(), onLeave: vi.fn<() => void>() });
    expect(screen.getByTestId("duos-result-board")).toHaveFocus();
  });

  it("#1755 View the board still dismisses to the board, and the rail's Result takes the focus", () => {
    renderDuosBoard({ view: OVER }, { onWatchReplay: vi.fn<() => void>(), onLeave: vi.fn<() => void>() });
    fireEvent.click(screen.getByTestId("duos-result-board"));
    expect(screen.queryByTestId("duos-result")).toBeNull();
    expect(screen.getByTestId("game")).toBeInTheDocument();
    expect(screen.getByTestId("duos-result-reopen")).toHaveFocus();
  });

  it("#1755 Result opens the panel again, its replay door and XP reveal with it, and the panel takes the focus", () => {
    const onWatchReplay = vi.fn<() => void>();
    renderDuosBoard({ view: OVER, economy: ECONOMY }, { onWatchReplay });
    fireEvent.click(screen.getByTestId("duos-result-board"));
    expect(screen.queryByTestId("xp-reveal")).toBeNull();

    fireEvent.click(screen.getByTestId("duos-result-reopen"));
    expect(screen.getByTestId("duos-result")).toBeInTheDocument();
    expect(screen.queryByTestId("duos-result-reopen")).toBeNull();
    expect(screen.getByTestId("duos-result-board")).toHaveFocus();
    expect(screen.getByTestId("xp-reveal-quest-win-3")).toHaveTextContent("Win 3 games");

    fireEvent.click(screen.getByTestId("duos-result-replay"));
    expect(onWatchReplay).toHaveBeenCalledTimes(1);
  });

  it("#1755 the rail's Result is on the teammate's board too", () => {
    renderDuosBoard({ view: OVER });
    fireEvent.click(screen.getByTestId("duos-result-board"));
    flipPortal();
    expect(screen.getByTestId(testid.handCard("t1"))).toBeInTheDocument();
    fireEvent.click(screen.getByTestId("duos-result-reopen"));
    expect(screen.getByTestId("duos-result")).toBeInTheDocument();
  });

  it("#1755 no Result before the game ends, while the panel is up, or in replay mode", () => {
    renderDuosBoard();
    expect(screen.queryByTestId("duos-result-reopen")).toBeNull();
    cleanup();

    renderDuosBoard({ view: OVER });
    expect(screen.queryByTestId("duos-result-reopen")).toBeNull();
    cleanup();

    renderDuosBoard({ replay: true, view: OVER });
    expect(screen.queryByTestId("duos-result-reopen")).toBeNull();
  });
});

// R2092000: Play again on the result panel
describe("R2092000 Play again on the result panel", () => {
  const OVER: DuosView = { ...duosView(), result: CONCEDED };
  const HERE = { youOffered: false, teammateOffered: false, teammateHere: true, matchId: null };

  function again(over: Partial<DuosPlayAgain> = {}): DuosPlayAgain {
    return { status: HERE, offering: false, refusal: null, offer: vi.fn<() => void>(), ...over };
  }

  it("R2092000 the panel draws Play again beside Watch the replay when the route hangs it, and Back to Play together is no longer primary", () => {
    const offer = vi.fn<() => void>();
    renderDuosBoard({ view: OVER }, { onWatchReplay: vi.fn(), onLeave: vi.fn(), playAgain: again({ offer }) });
    const buttons = within(screen.getByTestId("duos-result")).getAllByRole("button");
    expect(buttons.map((button) => button.getAttribute("data-testid"))).toEqual([
      "duos-result-board",
      "duos-result-replay",
      "duos-result-again",
      "duos-result-back",
    ]);
    const play = screen.getByTestId("duos-result-again");
    expect(play).toHaveClass("button-primary");
    expect(play).toHaveTextContent("Play again");
    expect(screen.getByTestId("duos-result-back")).not.toHaveClass("button-primary");
    expect(screen.getByTestId("duos-result-board")).toHaveFocus();
    fireEvent.click(play);
    expect(offer).toHaveBeenCalledTimes(1);
  });

  it("R2092000 while the offer is in flight Play again is disabled and busy", () => {
    renderDuosBoard({ view: OVER }, { onWatchReplay: vi.fn(), onLeave: vi.fn(), playAgain: again({ offering: true }) });
    const play = screen.getByTestId("duos-result-again");
    expect(play).toBeDisabled();
    expect(play).toHaveAttribute("aria-busy", "true");
  });

  it("R2092000 no Play again before the server's read, and none once the teammate has left", () => {
    renderDuosBoard({ view: OVER }, { onWatchReplay: vi.fn(), onLeave: vi.fn(), playAgain: again({ status: null }) });
    expect(screen.queryByTestId("duos-result-again")).toBeNull();
    expect(screen.queryByTestId("duos-result-again-status")).toBeNull();
    cleanup();

    renderDuosBoard(
      { view: OVER },
      { onWatchReplay: vi.fn(), onLeave: vi.fn(), playAgain: again({ status: { ...HERE, teammateHere: false } }) },
    );
    expect(screen.queryByTestId("duos-result-again")).toBeNull();
    const line = screen.getByTestId("duos-result-again-status");
    expect(line).toHaveAttribute("role", "status");
    expect(line).toHaveTextContent("Your teammate has left");
  });

  it("R2092000 no Play again in the co-op replay, nor without one, as on /dev/duos", () => {
    renderDuosBoard({ replay: true, view: OVER }, { playAgain: again() });
    expect(screen.queryByTestId("duos-result-again")).toBeNull();
    expect(screen.queryByTestId("duos-result-again-status")).toBeNull();
    cleanup();

    renderDuosBoard({ view: OVER }, { onLeave: vi.fn() });
    expect(screen.getByTestId("duos-result")).toBeInTheDocument();
    expect(screen.queryByTestId("duos-result-again")).toBeNull();
    expect(screen.queryByTestId("duos-result-again-status")).toBeNull();
  });
});

// #1146: the team's result waits for the own duel's last burst
describe("#1146 the team result waits for the own duel's lethal burst", () => {
  const START: GameEvent[] = [{ type: "turnStarted", player: "p1", turn: 5 }];
  const HIT: GameEvent = { type: "damage", sourceId: null, targetId: "hero-p1", amount: 3, combat: false };
  const LETHAL: DuosView["result"] = { outcome: "win", winner: "a", reason: "duelResult" };

  it("#1146 holds the panel while the burst plays, and opens it once the board settles", async () => {
    const board = renderDuosBoard({ view: { ...duosView(), own: withEvents(ownDuel(), START) } });
    expect(screen.queryByTestId("duos-result")).toBeNull();

    board.rerender({
      view: {
        ...duosView(),
        own: withEvents(ownDuel({ result: { winner: "p1", reason: "hero-death" } }), [
          ...START,
          HIT,
          { type: "gameOver", winner: "p1", reason: "hero-death" },
        ]),
        result: LETHAL,
      },
    });
    // The board's own marks reach the panel through a MutationObserver: let its callback run.
    await act(async () => {});
    expect(screen.getByTestId("animation-queue")).toBeInTheDocument();
    expect(screen.queryByTestId("duos-result")).toBeNull();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(10_000);
    });
    expect(screen.queryByTestId("animation-queue")).toBeNull();
    expect(screen.getByTestId("duos-result")).toBeInTheDocument();
  });
});

// R2095000: Both boards, the teammate's duel beside the own one

describe("R2095000 both boards render side by side with the teammate's read-only", () => {
  it("R2095000 with the toggle on, both boards render, the teammate's inside duos-board-teammate", () => {
    renderDuosBoard({ legal: [END_TURN, PASS_H1] });
    expect(screen.getAllByTestId("game")).toHaveLength(1);
    toggleBoth();
    expect(screen.getAllByTestId("game")).toHaveLength(2);
    const own = screen.getByTestId(ID.boardOwn);
    const mate = screen.getByTestId(ID.boardTeammate);
    expect(own).toHaveClass("duos-board-own");
    expect(mate).toHaveClass("duos-board-teammate");
    expect(within(own).getByTestId(testid.handCard("h1"))).toBeInTheDocument();
    expect(within(mate).getByTestId(testid.handCard("t1"))).toBeInTheDocument();
    expect(within(own).queryByTestId(testid.handCard("t1"))).toBeNull();
    expect(within(mate).queryByTestId(testid.handCard("h1"))).toBeNull();
  });

  it("R2095000 the own board still sends duel moves and passes", () => {
    const board = renderDuosBoard({ legal: [END_TURN, PASS_H1] });
    toggleBoth();
    const own = screen.getByTestId(ID.boardOwn);
    fireEvent.click(within(own).getByTestId(testid.endTurn));
    expect(board.onAction).toHaveBeenCalledWith(END_TURN);
    const pass = must(reveal(within(own).getByTestId(testid.handCard("h1")), ID.pass), "the pass control on h1");
    fireEvent.click(pass);
    expect(board.onAction).toHaveBeenCalledWith(PASS_H1);
  });

  it("R2095000 the teammate's board sends nothing", () => {
    const board = renderDuosBoard({ legal: [END_TURN, PASS_H1] });
    toggleBoth();
    const before = board.onAction.mock.calls.length;
    const mate = screen.getByTestId(ID.boardTeammate);
    const endTurn = within(mate).queryByTestId(testid.endTurn);
    if (endTurn !== null) {
      expect(endTurn).toBeDisabled();
      fireEvent.click(endTurn);
    }
    fireEvent.click(within(mate).getByTestId(testid.handCard("t1")));
    fireEvent.click(within(mate).getByTestId(testid.card("e2")));
    act(() => {
      vi.advanceTimersByTime(PAST_ANY_INSPECT_DELAY_MS);
    });
    expect(reveal(within(mate).getByTestId(testid.handCard("t1")), ID.pass)).toBeNull();
    expect(board.onAction.mock.calls.length).toBe(before);
  });
});

describe("R2095000 the Both boards choice persists per device and starts off", () => {
  it("R2095000 the choice persists per device and starts off with nothing stored", () => {
    renderDuosBoard({ legal: [END_TURN] });
    expect(screen.getAllByTestId("game")).toHaveLength(1);
    expect(screen.getByTestId(ID.both)).toHaveAttribute("aria-pressed", "false");
    toggleBoth();
    expect(screen.getAllByTestId("game")).toHaveLength(2);
    expect(window.localStorage.getItem(DUOS_BOTH_KEY)).toBe("1");
    cleanup();

    renderDuosBoard({ legal: [END_TURN] });
    expect(screen.getAllByTestId("game")).toHaveLength(2);
    expect(screen.getByTestId(ID.both)).toHaveAttribute("aria-pressed", "true");
    window.localStorage.clear();
    cleanup();

    renderDuosBoard({ legal: [END_TURN] });
    expect(screen.getAllByTestId("game")).toHaveLength(1);
    expect(screen.getByTestId(ID.both)).toHaveAttribute("aria-pressed", "false");
  });
});

describe("R2095000 pings and passes on the teammate's board", () => {
  it("R2095000 a ping made on the teammate's board is sent and marked there", () => {
    const board = renderDuosBoard();
    toggleBoth();
    const mate = screen.getByTestId(ID.boardTeammate);
    const wheel = must(reveal(within(mate).getByTestId(testid.card("e2")), ID.pingWheel), "the ping wheel on e2");
    expect(mate.contains(wheel)).toBe(true);
    fireEvent.click(within(wheel).getByTestId(ID.ping("killThis")));
    expect(board.onPing).toHaveBeenCalledWith("killThis", { duel: 1, at: "unit", side: "p2", lane: 0 });
    board.rerender({
      pings: [{ from: "a2", ping: "killThis", target: { duel: 1, at: "unit", side: "p2", lane: 0 }, at: Date.now() }],
    });
    expect(within(mate).getByTestId(ID.pingMarker)).toBeInTheDocument();
    expect(within(screen.getByTestId(ID.boardOwn)).queryByTestId(ID.pingMarker)).toBeNull();
  });

  it("R2095000 dragging a listed pass onto the portal or the teammate's board sends nothing (#2619)", () => {
    const board = renderDuosBoard({ legal: [END_TURN, PASS_H1] });
    const dragOnto = (drop: Element): void => {
      const restore = pointAt(drop);
      try {
        const own = screen.queryByTestId(ID.boardOwn) ?? screen.getByTestId("duos-board");
        const start = within(own).getByTestId(testid.handCard("h1"));
        fireEvent.pointerDown(start, { pointerId: 3, clientX: 10, clientY: 10 });
        fireEvent.pointerUp(start, { pointerId: 3, clientX: 10 + DRAG_THRESHOLD_PX + 4, clientY: 10 });
      } finally {
        restore();
      }
    };
    dragOnto(screen.getByTestId(ID.portal));
    toggleBoth();
    dragOnto(screen.getByTestId(ID.boardTeammate));
    expect(passesSent(board)).toEqual([]);
  });
});

describe("R2095000 a drop on the teammate's board is no drop on the own board", () => {
  it("R2095000 dragging a card onto the teammate's lane plays nothing on the own board and passes nothing", () => {
    const play: DuosActionBody = {
      kind: "duel",
      body: { type: "play", instanceId: "h1", zone: { row: "units", lane: 3 } },
    };
    const board = renderDuosBoard({ legal: [END_TURN, play, PASS_H1] });
    toggleBoth();
    const mate = within(screen.getByTestId(ID.boardTeammate));
    // The teammate's board draws the own board's test ids: its lane 3 must not take the own card.
    const lane = mate.getByTestId("zone-you-units-3");
    const restore = pointAt(screen.getByTestId(ID.boardTeammate));
    document.elementsFromPoint = (() => [lane, screen.getByTestId(ID.boardTeammate)]) as Document["elementsFromPoint"];
    try {
      const start = within(screen.getByTestId(ID.boardOwn)).getByTestId(testid.handCard("h1"));
      fireEvent.pointerDown(start, { pointerId: 3, clientX: 10, clientY: 10 });
      fireEvent.pointerMove(window, { pointerId: 3, clientX: 10 + DRAG_THRESHOLD_PX + 4, clientY: 10 });
      fireEvent.pointerUp(window, { pointerId: 3, clientX: 10 + DRAG_THRESHOLD_PX + 4, clientY: 10 });
    } finally {
      delete (document as Partial<Document>).elementsFromPoint;
      restore();
    }
    expect(board.onAction).not.toHaveBeenCalledWith(PASS_H1);
    expect(board.onAction).not.toHaveBeenCalledWith(play);
  });
});

describe("R2095000 the teammate's board draws no effects of its own", () => {
  it("R2095000 only the own board mounts an effects layer, so the teammate's duel never marks the own board", () => {
    renderDuosBoard({ legal: [END_TURN, PASS_H1] });
    toggleBoth();
    expect(within(screen.getByTestId(ID.boardOwn)).getAllByTestId("fx-layer")).toHaveLength(1);
    expect(within(screen.getByTestId(ID.boardTeammate)).queryByTestId("fx-layer")).toBeNull();
  });
});

describe("R2095000 the teammate's board feeds no audio", () => {
  it("R2095000 the teammate's board feeds no audio", () => {
    vi.mocked(createSoundDirector).mockClear();
    const view = duosView();
    window.localStorage.setItem(DUOS_BOTH_KEY, "1");
    renderDuosBoard({ view, legal: [END_TURN, PASS_H1] });
    expect(screen.getAllByTestId("game")).toHaveLength(2);
    const fed = vi
      .mocked(createSoundDirector)
      .mock.results.flatMap((result) =>
        result.type === "return" ? vi.mocked(result.value.onView).mock.calls.map(([fedView]) => fedView) : [],
      );
    expect(fed.length).toBeGreaterThan(0);
    for (const fedView of fed) expect(fedView).toBe(view.own);
  });
});

describe("R2095000 the portal hides while both boards show", () => {
  it("R2095000 the portal hides while both boards show and returns when the toggle is off", () => {
    renderDuosBoard();
    expect(screen.getByTestId(ID.portal)).toBeInTheDocument();
    toggleBoth();
    // The portal stays mounted while both boards show: CSS hides it on a wide screen and shows it
    // again on a narrowed one, where it is the only flip control.
    expect(screen.getByTestId(ID.portal)).toBeInTheDocument();
    expect(screen.getAllByTestId("game")).toHaveLength(2);
    toggleBoth();
    expect(screen.getByTestId(ID.portal)).toBeInTheDocument();
    expect(screen.getAllByTestId("game")).toHaveLength(1);
  });

  it("R2095000 the portal still flips while both boards show, for a narrowed screen", () => {
    renderDuosBoard({ legal: [END_TURN, { kind: "concede" }] });
    toggleBoth();
    expect(screen.getByTestId("duos-board")).toHaveAttribute("data-showing", "own");
    // A narrowed screen draws only the showing board (duos.css), so the rail, the portal in it,
    // must ride that board: the own board's while it shows...
    expect(within(screen.getByTestId(ID.boardOwn)).getByTestId(ID.portal)).toBeInTheDocument();
    flipPortal();
    expect(screen.getByTestId("duos-board")).toHaveAttribute("data-showing", "teammate");
    expect(screen.getAllByTestId("game")).toHaveLength(2);
    // ...and the teammate's once the portal flips to it, none of it left on the hidden own board.
    const own = screen.getByTestId(ID.boardOwn);
    const mate = screen.getByTestId(ID.boardTeammate);
    const back = within(mate).getByTestId(ID.portal);
    expect(back).toHaveAttribute("aria-pressed", "true");
    expect(within(mate).getByTestId(ID.lifeStrip)).toBeInTheDocument();
    expect(within(mate).getByTestId(ID.concede)).toBeEnabled();
    expect(within(own).queryByTestId(ID.portal)).toBeNull();
    expect(within(own).queryByTestId(ID.lifeStrip)).toBeNull();
    flipPortal();
    expect(screen.getByTestId("duos-board")).toHaveAttribute("data-showing", "own");
    expect(within(screen.getByTestId(ID.boardOwn)).getByTestId(ID.portal)).toBeInTheDocument();
    expect(within(screen.getByTestId(ID.boardTeammate)).queryByTestId(ID.portal)).toBeNull();
  });

  it("R2095000 pressing Both boards while the teammate's board shows opens both on the own one", () => {
    renderDuosBoard();
    flipPortal();
    expect(screen.getByTestId("duos-board")).toHaveAttribute("data-showing", "teammate");
    toggleBoth();
    expect(screen.getByTestId("duos-board")).toHaveAttribute("data-showing", "own");
    expect(within(screen.getByTestId(ID.boardOwn)).getByTestId(ID.both)).toHaveAttribute("aria-pressed", "true");
  });

  it("R2095000 replay mode keeps the portal flip and draws no Both boards toggle", () => {
    renderDuosBoard({ replay: true });
    expect(screen.queryByTestId(ID.both)).toBeNull();
    flipPortal();
    expect(screen.getByTestId(testid.handCard("t1"))).toBeInTheDocument();
  });
});
