// R2097000: a deck holding a card the newest patch changed says so, until it is saved under it.

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import type { CardDef, Flavor } from "@jackioh/shared";

import { DECK_AUTOSAVE_DEBOUNCE_MS } from "@jackioh/server-config";
import { PatchSourceProvider } from "../../patches/context.tsx";
import { sourceFromData, type Patch } from "../../patches/source.ts";
import { patchTestid } from "../../patches/testids.ts";
import DeckWorkshop from "./DeckWorkshop.tsx";
import { CATALOG_VERSION, fixtureCardId, fixtureCardName, fixtureCatalog, fixtureCollection, fixtureDefs } from "./fixtures.ts";
import {
  TEST_PROFILE,
  decksResponse,
  fakeDeckServer,
  manualClock,
  memoryStorage,
  savedDeck,
  type ManualClock,
} from "./testkit.ts";
import {
  DECK_EDITOR,
  DECK_LIST,
  DECK_NAME_INPUT,
  DECK_PATCH_BADGE,
  DECK_PATCH_CARDS,
  SYNC_STATUS,
  deckPatchCardId,
} from "./testids.ts";

const COST = fixtureCardId(1);
const TEXT = fixtureCardId(2);
const FLAVOR = fixtureCardId(3);
const KEPT = fixtureCardId(4);

const OLD = "test-catalog-0";

const PATCHES: Patch[] = [
  { version: OLD, date: "2026-09-01", title: "Older", source: "Test", notes: "Test", changes: [] },
  {
    version: CATALOG_VERSION,
    date: "2026-10-01",
    title: "Newest",
    source: "Test",
    notes: "Test",
    changes: [
      { id: COST, name: fixtureCardName(1), kind: "changed", fields: ["cost"] },
      { id: TEXT, name: fixtureCardName(2), kind: "changed", fields: ["base.text"] },
      { id: FLAVOR, name: fixtureCardName(3), kind: "changed", fields: ["flavor"] },
    ],
  },
];

function withFlavor(id: string, flavor: Flavor): CardDef {
  const def = fixtureDefs()[id];
  if (def === undefined) throw new Error(`no ${id}`);
  return { ...def, flavor };
}

const SOURCE = sourceFromData({
  patches: PATCHES,
  index: {},
  snapshots: {
    [OLD]: { ...fixtureDefs(), [FLAVOR]: withFlavor(FLAVOR, "Fire") },
    [CATALOG_VERSION]: {
      ...fixtureDefs(),
      [COST]: { ...withFlavor(COST, "Fire"), cost: 1 },
      [TEXT]: { ...withFlavor(TEXT, "Fire"), base: { attack: 1, health: 1, keywords: [], text: "Draw 1." } },
      [FLAVOR]: withFlavor(FLAVOR, "Dark"),
    },
  },
});

const catalog = fixtureCatalog();
const collection = fixtureCollection();

type Mounted = {
  server: ReturnType<typeof fakeDeckServer>;
  tick: (ms: number) => Promise<void>;
};

function mount(): Mounted {
  const server = fakeDeckServer();
  const decks = [
    savedDeck("d-stale", "Stale", [COST, TEXT, FLAVOR, KEPT], 1, OLD),
    savedDeck("d-kept", "Kept", [KEPT], 2, OLD),
    savedDeck("d-flavor", "Flavor", [FLAVOR], 3, OLD),
    savedDeck("d-resaved", "Resaved", [COST, TEXT], 4, CATALOG_VERSION),
  ];
  const data = decksResponse(decks, [], CATALOG_VERSION);
  server.seed(data);
  const storage = memoryStorage();
  const clock: ManualClock = manualClock();
  let minted = 0;
  render(
    <PatchSourceProvider source={SOURCE}>
      <DeckWorkshop
        catalog={catalog}
        collection={collection}
        data={data}
        profileId={TEST_PROFILE}
        api={server.api}
        storage={storage}
        clock={clock}
        newId={() => {
          minted += 1;
          return `new-${String(minted)}`;
        }}
      />
    </PatchSourceProvider>,
  );
  return {
    server,
    tick: async (ms) => {
      await act(async () => {
        await clock.advance(ms);
      });
    },
  };
}

afterEach(() => {
  cleanup();
});

describe("R2097000 the deck's patch badge", () => {
  it("R2097000 a deck holding changed cards wears the count on its row; one holding none wears nothing", async () => {
    mount();
    await screen.findAllByTestId(DECK_PATCH_BADGE);
    const badges = within(screen.getByTestId(DECK_LIST)).getAllByTestId(DECK_PATCH_BADGE);
    expect(badges.map((badge) => badge.getAttribute("data-deck"))).toEqual(["d-stale"]);
    expect(badges[0]?.getAttribute("data-count")).toBe("2");
    expect(badges[0]?.getAttribute("data-version")).toBe(CATALOG_VERSION);
    expect(badges[0]).toHaveTextContent(`2 cards changed in ${CATALOG_VERSION}`);
  });

  it("R2097000 the open deck's editor wears the same badge", async () => {
    mount();
    await screen.findAllByTestId(DECK_PATCH_BADGE);
    const badge = within(screen.getByTestId(DECK_EDITOR)).getByTestId(DECK_PATCH_BADGE);
    expect(badge.getAttribute("data-deck")).toBe("d-stale");
    expect(badge.getAttribute("data-count")).toBe("2");
  });

  it("R2097000 the badge lists exactly the deck's changed cards, each with its change", async () => {
    mount();
    await screen.findAllByTestId(DECK_PATCH_BADGE);
    const badge = within(screen.getByTestId(DECK_EDITOR)).getByTestId(DECK_PATCH_BADGE);
    fireEvent.click(badge);
    expect(badge).toHaveAttribute("aria-expanded", "true");
    const list = screen.getByTestId(DECK_PATCH_CARDS);
    expect([...list.querySelectorAll(":scope > li")].map((item) => item.getAttribute("data-card"))).toEqual([COST, TEXT]);
    const cost = screen.getByTestId(deckPatchCardId(COST));
    expect(cost).toHaveTextContent(fixtureCardName(1));
    expect(within(cost).getAllByTestId(patchTestid.change).map((change) => change.getAttribute("data-field"))).toEqual(["cost"]);
    const text = screen.getByTestId(deckPatchCardId(TEXT));
    expect(within(text).getAllByTestId(patchTestid.change).map((change) => change.getAttribute("data-field"))).toEqual(["base.text"]);
    expect(screen.queryByTestId(deckPatchCardId(FLAVOR))).toBeNull();
    expect(screen.queryByTestId(deckPatchCardId(KEPT))).toBeNull();
  });

  it("R2097000 a flavor-only change counts for nothing", async () => {
    mount();
    await screen.findAllByTestId(DECK_PATCH_BADGE);
    const decks = screen.queryAllByTestId(DECK_PATCH_BADGE).map((badge) => badge.getAttribute("data-deck"));
    expect(decks).not.toContain("d-flavor");
    expect(decks.filter((deck) => deck === "d-stale")).toHaveLength(2);
  });

  it("R2097000 the badge goes once the deck is saved under the newest patch", async () => {
    const { server, tick } = mount();
    await screen.findAllByTestId(DECK_PATCH_BADGE);
    fireEvent.change(screen.getByTestId(DECK_NAME_INPUT), { target: { value: "Stale, seen" } });
    await tick(DECK_AUTOSAVE_DEBOUNCE_MS);
    expect(screen.queryAllByTestId(DECK_PATCH_BADGE).map((badge) => badge.getAttribute("data-deck"))).not.toContain("d-stale");
    expect(server.decks.get("d-stale")?.catalogVersion).toBe(CATALOG_VERSION);
    expect(screen.getByTestId(SYNC_STATUS)).toHaveAttribute("data-state", "saved");
  });
});
