// #3478: hovering the hand redraws no card's face, and a face is built once per change. The
// engine's `face` is counted (but still answered, so the board draws as ever): crossing the whole
// hand performs no new call, a one-card view change recomputes only that card's face, and a locale
// switch recomputes every shown face. The #991 rings still appear on hover (Board.test.tsx pins
// their cases; here they are pinned against the call count).

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { CardView, PlayerView, PreviewValue } from "@jackioh/shared";

import { resetLocaleForTests, setLocale } from "../i18n/i18n.ts";
import { withCatalog } from "../test/board.tsx";
import { fullBoardView } from "../test/fixtures.ts";
import type { FaceAnswer } from "../wire/generated/FaceAnswer.ts";
import type { FaceCall } from "../wire/generated/FaceCall.ts";
import Board from "./Board.tsx";
import { testid } from "./contract.ts";

/** Every `face` call the board made since the calls were last cleared, in order. */
const calls = vi.hoisted(() => [] as FaceCall[]);

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

function handOf(view: PlayerView): CardView[] {
  const hand = view.you.hand;
  if (!Array.isArray(hand)) throw new Error("the fixture's own hand is cards");
  return hand;
}

/** The instance ids of the `live` calls among `seen`. */
function liveIds(seen: readonly FaceCall[]): string[] {
  return seen
    .filter((request) => request.call === "live")
    .map((request) => request.card.instanceId);
}

/** The instance ids of every unit and face-up backrow card on the field. */
function fieldIds(view: PlayerView): string[] {
  const ids: string[] = [];
  for (const seat of [view.you, view.opponent]) {
    for (const unit of seat.units) {
      if (unit !== null) ids.push(unit.instanceId);
    }
    for (const entry of seat.backrow) {
      if (entry !== null && !entry.faceDown) {
        ids.push(entry.instanceId);
        if (entry.type === "Unit" && entry.unit !== undefined && entry.unit !== null) {
          ids.push(entry.unit.instanceId);
        }
      }
    }
  }
  return ids;
}

/** The sorted testids of every element the board marks `data-counted` (#991). */
function ringed(container: HTMLElement): string[] {
  return Array.from(container.querySelectorAll('[data-counted="true"]'))
    .map((element) => element.getAttribute("data-testid") ?? "")
    .sort();
}

/** The fixture with the first hand card's preview naming two units, as Board.test.tsx's #991 row does. */
function previewView(): { view: PlayerView; handId: string; a: string; b: string } {
  const view = fullBoardView();
  const hand = handOf(view);
  const first = hand[0];
  if (first === undefined) throw new Error("the fixture's own hand has a card");
  const a = view.opponent.units[0]?.instanceId;
  const b = view.you.units[1]?.instanceId;
  if (a === undefined || b === undefined) throw new Error("the fixture's units are there");
  const preview: PreviewValue[] = [{ label: "all permanents", value: 2, ids: [a, b] }];
  return {
    view: { ...view, you: { ...view.you, hand: [{ ...first, preview }, ...hand.slice(1)] } },
    handId: first.instanceId,
    a,
    b,
  };
}

afterEach(() => {
  cleanup();
  calls.length = 0;
  resetLocaleForTests();
});

describe("#3478 hovering the hand redraws no card's face", () => {
  it("crossing the whole hand performs no new face calls and still rings the preview's cards", () => {
    const { view, handId, a, b } = previewView();
    const { container } = render(withCatalog(<Board view={view} />));
    const base = calls.length;
    expect(base).toBeGreaterThan(0);

    for (const card of handOf(view)) {
      const handCard = screen.getByTestId(testid.handCard(card.instanceId));
      fireEvent.pointerEnter(handCard);
      fireEvent.pointerLeave(handCard);
    }
    expect(calls.length).toBe(base);

    const previewed = screen.getByTestId(testid.handCard(handId));
    fireEvent.pointerEnter(previewed);
    expect(ringed(container)).toEqual([testid.card(a), testid.card(b)].sort());
    expect(calls.length).toBe(base);

    fireEvent.pointerLeave(previewed);
    expect(ringed(container)).toEqual([]);
    expect(calls.length).toBe(base);
  });

  it("a new view recomputes only the changed card's face", () => {
    const view = fullBoardView();
    const { rerender } = render(withCatalog(<Board view={view} />));
    calls.length = 0;

    const hand = handOf(view);
    const one = hand[1];
    if (one === undefined) throw new Error("the fixture's own hand has a second card");
    const changed: CardView = { ...one, cost: (one.cost ?? 0) + 1 };
    const next: PlayerView = {
      ...view,
      you: { ...view.you, hand: hand.map((card) => (card === one ? changed : card)) },
    };
    rerender(withCatalog(<Board view={next} />));

    const seen = new Set(liveIds(calls));
    expect(seen.has(changed.instanceId)).toBe(true);
    const shown = new Set([...handOf(next).map((card) => card.instanceId), ...fieldIds(next)]);
    expect([...seen].filter((id) => shown.has(id)).sort()).toEqual([changed.instanceId]);
  });

  it("a locale switch recomputes every shown face", async () => {
    const view = fullBoardView();
    render(withCatalog(<Board view={view} />));
    const before = new Set(liveIds(calls));
    const shown = new Set([...handOf(view).map((card) => card.instanceId), ...fieldIds(view)]);
    const shownBefore = [...shown].filter((id) => before.has(id));
    expect(shownBefore.length).toBeGreaterThan(0);
    calls.length = 0;

    await act(async () => {
      await setLocale("zh-CN");
    });

    const after = new Set(liveIds(calls));
    for (const id of shownBefore) expect(after.has(id)).toBe(true);
  });
});
