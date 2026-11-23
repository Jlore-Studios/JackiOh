// #3609: a card face is drawn again only when what it draws changes. Through `<Game>`, with every view a fresh
// parse as the wasm module and the socket hand it over (shareView.ts), a selection (cardHighlight.ts) and a
// pointer in motion (the drag and the aim, which keep the pointer out of the board's state): no card is drawn
// that did not change. `marksOf` runs once in each drawing of a face-up card, so its calls for a card count
// that card's drawings (Card.memo.test.tsx), and the engine's `face` calls are counted as Board.render.test.tsx does.

import type { ActionBody, AttackForecast, CardView, CombatForecast, PlayerView } from "@jackioh/shared";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { marksOf } from "../cards/marks.ts";
import { __resetSettingsForTests } from "../settings/index.ts";
import { withCatalog } from "../test/board.tsx";
import { baseView, card, emptySide, fullBoardView, unit } from "../test/fixtures.ts";
import type { FaceAnswer } from "../wire/generated/FaceAnswer.ts";
import type { FaceCall } from "../wire/generated/FaceCall.ts";
import { testid } from "./contract.ts";
import Game from "./Game.tsx";
import { DRAG_THRESHOLD_PX } from "./drag/model.ts";

/** Every `face` call made since the calls were last cleared, in order. */
const calls = vi.hoisted(() => [] as FaceCall[]);

vi.mock("../cards/marks.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../cards/marks.ts")>();
  return { ...actual, marksOf: vi.fn(actual.marksOf) };
});

vi.mock("../wasm/index.ts", async (importOriginal) => {
  const real = await importOriginal<typeof import("../wasm/index.ts")>();
  return {
    ...real,
    face: (request: FaceCall): FaceAnswer => {
      calls.push(request);
      return real.face(request);
    },
  };
});

/** What the wasm module and a socket hand the client: the same data, as new objects. */
function freshView(view: PlayerView): PlayerView {
  return JSON.parse(JSON.stringify(view)) as PlayerView;
}

/** How many times the card `instanceId` has been drawn since the last clear. */
function drawings(instanceId: string): number {
  return vi.mocked(marksOf).mock.calls.filter(([shown]) => (shown as { instanceId?: string } | null | undefined)?.instanceId === instanceId).length;
}

/** The instance ids of every card the board draws a face for: each unit, each of the viewer's hand cards, each face-up backrow card. */
function idsOf(view: PlayerView): string[] {
  const ids: string[] = [];
  for (const seat of [view.you, view.opponent]) {
    for (const entry of seat.units) if (entry !== null) ids.push(entry.instanceId);
    for (const entry of seat.backrow) {
      if (entry !== null && !entry.faceDown) ids.push(entry.type === "Unit" ? (entry.unit?.instanceId ?? entry.instanceId) : entry.instanceId);
    }
  }
  if (Array.isArray(view.you.hand)) for (const entry of view.you.hand) ids.push(entry.instanceId);
  return ids;
}

/** The testid each of those cards carries: a hand card's `hand-card-`, any other's `card-`. */
function testidsOf(view: PlayerView): string[] {
  const hand = Array.isArray(view.you.hand) ? view.you.hand.map((entry) => testid.handCard(entry.instanceId)) : [];
  const field = idsOf(view).filter((id) => !hand.includes(testid.handCard(id))).map((id) => testid.card(id));
  return [...field, ...hand];
}

/** The ids whose `live` face was asked for since the last clear. */
function liveIds(): string[] {
  return calls.filter((request) => request.call === "live").map((request) => request.card.instanceId);
}

function clearCounters(): void {
  vi.mocked(marksOf).mockClear();
  calls.length = 0;
}

function handOf(view: PlayerView): CardView[] {
  if (!Array.isArray(view.you.hand)) throw new Error("the fixture's own hand is cards");
  return view.you.hand;
}

beforeEach(() => {
  __resetSettingsForTests();
});

afterEach(() => {
  cleanup();
  clearCounters();
  delete (document as Partial<Document>).elementsFromPoint;
  document.documentElement.removeAttribute("data-dragging");
  localStorage.clear();
});

const onAction = (): void => undefined;

describe("#3609 a fresh view draws only the cards that changed", () => {
  it("#3609 a fresh view with every card unchanged draws no card", () => {
    const first = freshView(fullBoardView({ trades: { cost: 0, left: 1 } }));
    const { rerender } = render(withCatalog(<Game view={first} legal={[]} onAction={onAction} />));
    const ids = idsOf(first);
    expect(ids.length).toBeGreaterThan(10);
    for (const id of ids) expect(drawings(id), `${id} is drawn the first time`).toBeGreaterThan(0);

    clearCounters();
    rerender(withCatalog(<Game view={freshView(first)} legal={[]} onAction={onAction} />));

    for (const id of ids) expect(drawings(id), `${id} drawn again`).toBe(0);
    // The piles list their cards' faces anew on every board render (Board.tsx, `browseEntries`); a card on the board or in the hand asks for none.
    const shown = new Set(ids);
    expect(liveIds().filter((id) => shown.has(id))).toEqual([]);
  });

  it("#3609 a fresh view that changes one minion's attack draws that minion and no other card", () => {
    const first = freshView(fullBoardView({ trades: { cost: 0, left: 1 } }));
    const { rerender } = render(withCatalog(<Game view={first} legal={[]} onAction={onAction} />));
    const next = freshView(first);
    const target = next.you.units[1];
    if (target === null || target === undefined) throw new Error("the fixture has a unit in lane 2");
    target.attack += 1;
    const changed = target.instanceId;

    clearCounters();
    rerender(withCatalog(<Game view={next} legal={[]} onAction={onAction} />));

    expect(drawings(changed)).toBeGreaterThanOrEqual(1);
    for (const id of idsOf(next).filter((other) => other !== changed)) expect(drawings(id), `${id} drawn again`).toBe(0);
    const shown = new Set(idsOf(next));
    expect(liveIds().filter((id) => shown.has(id))).toEqual([changed]);
    expect(screen.getByTestId(testid.card(changed))).toBeInTheDocument();
  });

  it("#3609 a fresh view with a hand card played draws no card that stayed", () => {
    const first = freshView(fullBoardView({ trades: { cost: 0, left: 1 } }));
    const { rerender } = render(withCatalog(<Game view={first} legal={[]} onAction={onAction} />));
    const next = freshView(first);
    const played = handOf(next).shift();
    if (played === undefined) throw new Error("the fixture's own hand has a card");

    clearCounters();
    rerender(withCatalog(<Game view={next} legal={[]} onAction={onAction} />));

    expect(screen.queryByTestId(testid.handCard(played.instanceId))).toBeNull();
    for (const id of idsOf(next)) expect(drawings(id), `${id} drawn again`).toBe(0);
  });
});

/** h1 plays in lanes 3 and 4; s1 plays; u1 attacks e1 or the hero; u2 only switches (test/ux/drag-layer.test.tsx). */
function dragView(): PlayerView {
  return baseView({
    you: emptySide("p1", {
      hand: [
        card({ instanceId: "h1", defId: "core-008", cost: 3 }),
        card({ instanceId: "s1", defId: "core-005", cost: 1 }),
        card({ instanceId: "n1", defId: "core-019", cost: 2 }),
      ],
      units: [unit("p1", { instanceId: "u1" }), unit("p1", { instanceId: "u2" }), null, null, null],
    }),
    opponent: emptySide("p2", {
      hand: { count: 3 },
      units: [unit("p2", { instanceId: "e1" }), null, null, null, null],
    }),
  });
}

const LEGAL: readonly ActionBody[] = [
  { type: "play", instanceId: "h1", zone: { row: "units", lane: 3 } },
  { type: "play", instanceId: "h1", zone: { row: "units", lane: 4 } },
  { type: "play", instanceId: "s1" },
  { type: "attack", attackerId: "u1", targetId: "e1" },
  { type: "attack", attackerId: "u1", targetId: "hero-p2" },
  { type: "switchPosition", instanceId: "u2" },
  { type: "endTurn" },
  { type: "offerDraw" },
  { type: "concede" },
];

const BLOW: AttackForecast = { attackerId: "u1", targetId: "e1", dealt: 2, taken: 1, kills: false, survives: true, pops: false, attackerSize: 1, targetSize: 1 };
const FORECAST: CombatForecast = { attacks: [BLOW], lethal: false };

const el = (id: string): HTMLElement => screen.getByTestId(id);

/** The three things a card draws off the highlight, as the board writes them: its legal, selected and glow marks. */
function marked(id: string): string {
  const element = el(id);
  return ["data-legal", "data-selected", "data-glow"].map((name) => element.getAttribute(name) ?? "-").join(",");
}

const POINTER = 1;
const START = { x: 200, y: 400 };
let under: Element[] = [];

function over(...stack: Element[]): void {
  under = stack;
}

function move(x: number, y: number): void {
  fireEvent.pointerMove(window, { pointerId: POINTER, clientX: x, clientY: y });
}

function renderDragGame(forecast?: CombatForecast) {
  under = [];
  document.elementsFromPoint = (() => under) as Document["elementsFromPoint"];
  const view = freshView(dragView());
  const utils = render(withCatalog(<Game view={view} legal={LEGAL} onAction={onAction} forecast={forecast} />));
  return { ...utils, view, ids: idsOf(view) };
}

describe("#3609 a selection draws only the cards whose own state changes", () => {
  it("#3609 selecting a hand card draws only the cards whose legal, selected or glow state changes", () => {
    const { view, ids } = renderDragGame();
    const testids = testidsOf(view);
    expect(testids).toEqual(expect.arrayContaining(["card-u1", "card-u2", "card-e1", "hand-card-h1", "hand-card-s1", "hand-card-n1"]));
    const before = new Map(testids.map((id) => [id, marked(id)]));

    clearCounters();
    fireEvent.click(el("hand-card-h1"));

    const drawn = ids.filter((id) => drawings(id) > 0).map((id) => testidsOf(view).find((shown) => shown.endsWith(`-${id}`)) ?? id).sort();
    const changed = testids.filter((id) => marked(id) !== before.get(id)).sort();
    expect(changed).toContain("hand-card-h1");
    expect(drawn).toEqual(changed);
    expect(drawn.length).toBeLessThan(testids.length);
  });
});

describe("#3609 the pointer in motion draws no card", () => {
  it("#3609 moving the pointer during a drag draws no card and crosses no face call", () => {
    const { ids } = renderDragGame();
    const source = el("hand-card-h1");
    over(source);
    fireEvent.pointerDown(source, { pointerId: POINTER, button: 0, clientX: START.x, clientY: START.y });
    move(START.x, START.y - DRAG_THRESHOLD_PX - 12);
    expect(el("drag-ghost")).toHaveAttribute("data-instance-id", "h1");
    expect(document.documentElement).toHaveAttribute("data-dragging", "play");

    over();
    clearCounters();
    for (const [x, y] of [
      [220, 360],
      [260, 320],
      [320, 280],
      [380, 240],
      [440, 200],
    ] as const) {
      move(x, y);
    }

    expect(el("drag-ghost")).toHaveAttribute("data-instance-id", "h1");
    for (const id of ids) expect(drawings(id), `${id} drawn on a pointer move`).toBe(0);
    expect(calls).toEqual([]);
  });

  it("#3609 moving the pointer during an aim draws no card, until it rests on a target that marks two", () => {
    const { ids } = renderDragGame(FORECAST);
    fireEvent.click(el("card-u1"));
    expect(el("card-u1")).toHaveAttribute("data-selected", "true");

    clearCounters();
    for (const [x, y] of [
      [300, 300],
      [320, 280],
      [340, 260],
      [360, 240],
    ] as const) {
      move(x, y);
    }
    for (const id of ids) expect(drawings(id), `${id} drawn on a pointer move`).toBe(0);
    expect(calls).toEqual([]);

    over(el("card-e1"));
    move(380, 200);
    expect(el("card-e1").querySelector('[data-testid^="outcome-"]')).not.toBeNull();
    for (const id of ids.filter((other) => other !== "u1" && other !== "e1")) expect(drawings(id), `${id} drawn when the aim met a target`).toBe(0);

    clearCounters();
    move(382, 202);
    move(384, 204);
    for (const id of ids) expect(drawings(id), `${id} drawn on a move across the target`).toBe(0);

    over();
    move(300, 300);
    clearCounters();
    move(310, 310);
    move(320, 320);
    for (const id of ids) expect(drawings(id), `${id} drawn on a move off the target`).toBe(0);
    expect(calls).toEqual([]);
  });
});
