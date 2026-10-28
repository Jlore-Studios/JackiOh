// #2266: the pool's cards are memoized. A filter, a sort, an ownership change or a card the deck takes
// re-renders only the cards it changes (a face renders once per card render, which is what these
// count), every card calls the screen's latest handlers though its own never change, and a pool
// longer than POOL_FIRST_CARDS draws that many first and the rest in a render of its own. The
// viewer's picks are PoolGrid.test.tsx's (R1956000), the Almanac's route is almanac.test.tsx's.

import type { CatalogSnapshot, Collection } from "@jackioh/validator";
import { cleanup, fireEvent, render } from "@testing-library/react";
import { Profiler, createElement, type ReactElement } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";

import PoolGrid, { POOL_FIRST_CARDS } from "./PoolGrid.tsx";
import { addPoolId } from "./testids.ts";

/** How many times each card's face has rendered, by card id. */
const faceRenders = vi.hoisted(() => new Map<string, number>());

vi.mock("../../cards/CardFace.tsx", async (importOriginal) => {
  const real = await importOriginal<typeof import("../../cards/CardFace.tsx")>();
  return {
    ...real,
    CardFace: (props: Parameters<typeof real.CardFace>[0]) => {
      faceRenders.set(props.face.defId, (faceRenders.get(props.face.defId) ?? 0) + 1);
      return createElement(real.CardFace, props);
    },
  };
});

const CATALOG_HERE: CatalogSnapshot = { version: "test", cards: CATALOG };
const IDS = ["core-002", "core-004", "core-008"];

function noop(): void {}

/** The deck editor's half of the grid: handlers built anew on each call, as a screen's are. */
function deckProps(collection: Collection | null, inDeck: string[], onAdd: (cardId: string) => void = noop) {
  return {
    deckName: "Deck",
    collection,
    inDeck: new Set(inDeck),
    holders: new Map(),
    refusedCardId: null,
    onAdd: (cardId: string) => {
      onAdd(cardId);
    },
    onDragStart: () => {},
    onDragEnd: () => {},
  };
}

/** The cards whose face rendered since the counts were last cleared. */
function rendered(): string[] {
  return [...faceRenders.keys()].sort();
}

function readOnly(ids: readonly string[]): ReactElement {
  return <PoolGrid ids={ids} catalog={CATALOG_HERE} onInspect={() => {}} />;
}

afterEach(() => {
  cleanup();
  faceRenders.clear();
});

describe("#2266 the pool's cards are memoized", () => {
  it("a filter or a sort re-renders no card that stays, and mounts only the cards it brings back", () => {
    const { rerender } = render(readOnly(IDS));
    expect(rendered()).toEqual([...IDS].sort());
    faceRenders.clear();

    // A narrower pool with a new handler: no card that stays draws again.
    rerender(readOnly(["core-002", "core-008"]));
    expect(rendered()).toEqual([]);

    // A new order draws the cards that stay as they were; the one it brings back mounts.
    rerender(readOnly(["core-008", "core-004", "core-002"]));
    expect(rendered()).toEqual(["core-004"]);
  });

  it("a card the deck takes, or one the collection stops owning, re-renders alone, and every card calls the latest handlers", () => {
    const ownAll: Collection = { "core-002": 1, "core-004": 1, "core-008": 1 };
    const first = vi.fn();
    const latest = vi.fn();
    const grid = (props: ReturnType<typeof deckProps>): ReactElement => (
      <PoolGrid ids={IDS} catalog={CATALOG_HERE} onInspect={() => {}} {...props} />
    );
    const { rerender, getByTestId } = render(grid(deckProps(ownAll, [], first)));
    faceRenders.clear();

    rerender(grid(deckProps(ownAll, ["core-004"], latest)));
    expect(rendered()).toEqual(["core-004"]);
    faceRenders.clear();

    rerender(grid(deckProps({ ...ownAll, "core-008": 0 }, ["core-004"], latest)));
    expect(rendered()).toEqual(["core-008"]);

    // The cards never got the handlers the grid was first given: they call the screen's latest.
    fireEvent.click(getByTestId(addPoolId("core-002")));
    expect(first).not.toHaveBeenCalled();
    expect(latest).toHaveBeenCalledExactlyOnceWith("core-002");
  });

  it("a pool longer than POOL_FIRST_CARDS draws that many first and the rest in the next render; one that grows keeps what it drew until then, and one that shrinks is drawn at once", () => {
    const ids = Object.keys(CATALOG).slice(0, POOL_FIRST_CARDS + 12);
    expect(ids.length).toBeGreaterThan(POOL_FIRST_CARDS);
    let drawn: number[] = [];
    const grid = (shown: readonly string[]): ReactElement => (
      <Profiler
        id="pool"
        onRender={() => {
          drawn.push(document.querySelectorAll(".db-card").length);
        }}
      >
        {readOnly(shown)}
      </Profiler>
    );

    const { rerender } = render(grid(ids));
    expect(drawn).toEqual([POOL_FIRST_CARDS, ids.length]);

    drawn = [];
    rerender(grid(ids.slice(0, 2)));
    expect(new Set(drawn)).toEqual(new Set([2]));

    drawn = [];
    rerender(grid(ids));
    expect(drawn).toEqual([2, ids.length]);
  });
});
