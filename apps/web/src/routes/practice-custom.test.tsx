// `/practice` with custom cards (SPEC §12.6, §12.10; R915005, R915006, R915009; #921): a saved deck
// that names custom cards starts with them, one naming a card the account no longer has does not,
// `?try=<id>` deals a random deck around the card, and no such game records card statistics. R962000:
// a sandbox game started with one offers it in its panel.

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Mock } from "vitest";

import { DECK_NAME_MAX_LENGTH, MAX_SAVED_DECKS, MAX_SAVED_TRIOS } from "@jackioh/server-config";
import { opponentOf } from "@jackioh/shared";
import type { CardDefs, CustomCard, PlayerId } from "@jackioh/shared";

import type { DecksResponse } from "../net/api.ts";
import type { CustomCardsResponse } from "../net/customCards.ts";
import type { Account } from "../net/gate.ts";
import { tryPath } from "../net/navigate.ts";
import { PRACTICE_PACING_FAST, PRACTICE_SETUP_KEY } from "../practice/config.ts";
import type { PracticeHost } from "../practice/host.ts";
import type { PracticeRequestBody, PracticeResponse, PracticeSnapshot, PracticeStartConfig } from "../practice/protocol.ts";
import { readPracticeResume } from "../practice/resume.ts";
import { practiceTestid } from "../practice/testids.ts";
import { readPlayerStats, resetPlayerStats } from "../stats/store.ts";
import { baseView, emptySide } from "../test/fixtures.ts";
import PracticeRoute, { readPracticeParams } from "./practice.tsx";
import type { PracticeRouteProps } from "./practice.tsx";

/** A 1-cost 1/2 Unit with no script, as the account keeps it: with the creator's builder model. */
const IMP: CustomCard = {
  id: "custom-ember-imp",
  version: 2,
  frame: { name: "Ember Imp", type: "Unit", cost: 1, rarity: "Common", attack: 1, health: 2, text: "" },
  source: "",
  builder: { blocks: [] },
};

/** The card as a game carries it (R915006): the builder's model is dropped. */
const IMP_IN_GAME: CustomCard = { id: IMP.id, version: IMP.version, frame: IMP.frame, source: IMP.source };

const CORE = Array.from({ length: 19 }, (_, i) => `core-${String(i + 1).padStart(3, "0")}`);
const IMP_DECK = [...CORE, IMP.id];

const FAKE_DEFS: CardDefs = Object.fromEntries(
  CORE.map((id) => [
    id,
    {
      id,
      index: id.slice(5),
      name: `Card ${id}`,
      set: "Core",
      type: "Unit",
      tags: [],
      rarity: "Common",
      token: false,
      cost: 2,
      base: { attack: 1, health: 1, keywords: [], text: "" },
      radiant: { attack: 2, health: 2, keywords: [], text: "" },
    } as unknown as CardDefs[string],
  ]),
);

const ACTIVE: Account = {
  kind: "ready",
  token: "tok-1",
  me: {
    profile: { id: "u1", status: "active" },
    needsInviteCode: false,
    emailVerified: true,
    currentMatchId: null,
    email: "player@example.com",
  },
};

function decksResponse(decks: readonly { name: string; cards: readonly string[] }[]): DecksResponse {
  return {
    catalogVersion: "v1",
    decks: decks.map((deck, i) => ({
      id: `deck-${String(i + 1)}`,
      name: deck.name,
      cards: [...deck.cards],
      catalogVersion: "v1",
      portrait: null,
      createdAt: i,
      updatedAt: i,
    })),
    trios: [],
    limits: { decks: MAX_SAVED_DECKS, trios: MAX_SAVED_TRIOS, nameLength: DECK_NAME_MAX_LENGTH },
  };
}

function customResponse(cards: CustomCard[]): CustomCardsResponse {
  return { cards, limits: { cards: 50 } };
}

function snapshotFor(human: PlayerId, over: boolean): PracticeSnapshot {
  const ai = opponentOf(human);
  return {
    view: baseView({
      viewer: human,
      turn: 1,
      active: human,
      phase: "main",
      you: emptySide(human),
      opponent: emptySide(ai, { hand: { count: 4 } }),
      ...(over ? { result: { winner: ai, reason: "concede" } } : {}),
    }),
    legal: over ? [] : [{ type: "endTurn" }, { type: "concede" }],
    aiToAct: false,
    error: null,
  };
}

/** A host that starts every game and ends it on a concede. */
function fakeHost(): { factory: Mock<() => PracticeHost>; starts: () => PracticeStartConfig[]; requests: PracticeRequestBody[] } {
  const requests: PracticeRequestBody[] = [];
  let id = 0;
  let human: PlayerId = "p1";
  const host: PracticeHost = {
    request(body) {
      requests.push(body);
      id += 1;
      if (body.type === "start" || body.type === "resume") {
        human = body.config.humanSeat;
        const started: PracticeResponse = { id, type: "started", snapshot: snapshotFor(human, false), defs: {}, aiSeat: opponentOf(human) };
        return Promise.resolve(started);
      }
      if (body.type === "catalog") return Promise.resolve({ id, type: "catalog", defs: FAKE_DEFS });
      const over = body.type === "act" && body.action.type === "concede";
      return Promise.resolve({ id, type: "snapshot", snapshot: snapshotFor(human, over) });
    },
    dispose() {},
  };
  return {
    factory: vi.fn(() => host),
    starts: () => requests.flatMap((body) => (body.type === "start" ? [body.config] : [])),
    requests,
  };
}

function renderRoute(host: ReturnType<typeof fakeHost>, props: Partial<PracticeRouteProps> = {}): void {
  render(
    <PracticeRoute
      hostFactory={host.factory}
      pacing={PRACTICE_PACING_FAST}
      account={ACTIVE}
      loadDecks={vi.fn(() => Promise.resolve(decksResponse([{ name: "Imps", cards: IMP_DECK }])))}
      loadCustomCards={vi.fn(() => Promise.resolve(customResponse([IMP])))}
      {...props}
    />,
  );
}

function visit(search: string): void {
  window.history.replaceState(null, "", `/practice${search}`);
}

async function concede(): Promise<void> {
  fireEvent.click(await screen.findByTestId("concede"));
  fireEvent.click(screen.getByTestId("concede-confirm"));
  await screen.findByTestId(practiceTestid.result);
}

beforeEach(() => {
  visit("");
  window.localStorage.clear();
  resetPlayerStats();
  vi.stubGlobal("fetch", vi.fn(() => Promise.reject(new Error("practice needs no server"))));
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.localStorage.clear();
  resetPlayerStats();
  window.history.replaceState(null, "", "/");
});

describe("R915006 a saved deck that names custom cards starts with them", () => {
  it("R915006 the setup previews the custom card from its frame and the game's config carries it, without its builder model", async () => {
    const host = fakeHost();
    renderRoute(host);
    await waitFor(() => {
      expect(Array.from((screen.getByTestId(practiceTestid.deck) as HTMLSelectElement).options).map((o) => o.value)).toContain("saved:1");
    });
    fireEvent.change(screen.getByTestId(practiceTestid.deck), { target: { value: "saved:1" } });

    const preview = await screen.findByTestId(practiceTestid.deckPreview);
    await waitFor(() => {
      expect(within(preview).getByTestId(practiceTestid.deckCard(IMP.id))).toHaveTextContent("Ember Imp");
    });
    expect(screen.queryByTestId(practiceTestid.customBlocked)).toBeNull();

    fireEvent.click(screen.getByTestId(practiceTestid.start));
    await screen.findByTestId(practiceTestid.hud);
    const [config] = host.starts();
    expect(config?.deck).toMatchObject({ kind: "saved", cards: IMP_DECK });
    expect(config?.customCards).toEqual([IMP_IN_GAME]);
    // R668: the page's own record of the game keeps them, so a reload picks it up as what it is.
    expect(readPracticeResume()?.customCards).toEqual([IMP_IN_GAME]);
  });

  it("R915005 a saved deck naming a custom card the account no longer has says so, and Start is off", async () => {
    const host = fakeHost();
    renderRoute(host, { loadCustomCards: vi.fn(() => Promise.resolve(customResponse([]))) });
    await waitFor(() => {
      expect(Array.from((screen.getByTestId(practiceTestid.deck) as HTMLSelectElement).options).map((o) => o.value)).toContain("saved:1");
    });
    fireEvent.change(screen.getByTestId(practiceTestid.deck), { target: { value: "saved:1" } });

    const blocked = await screen.findByTestId(practiceTestid.customBlocked);
    expect(blocked).toHaveTextContent("This deck names a custom card you no longer have");
    expect(screen.getByTestId(practiceTestid.start)).toBeDisabled();
    fireEvent.submit(screen.getByTestId(practiceTestid.setup));
    expect(host.starts()).toEqual([]);

    // Any other deck still starts.
    fireEvent.change(screen.getByTestId(practiceTestid.deck), { target: { value: "random" } });
    expect(screen.queryByTestId(practiceTestid.customBlocked)).toBeNull();
    fireEvent.click(screen.getByTestId(practiceTestid.start));
    await screen.findByTestId(practiceTestid.hud);
    expect(host.starts()[0]?.customCards).toBeUndefined();
  });

  it("R915005 custom cards that could not be read block only the deck that names them", async () => {
    const host = fakeHost();
    renderRoute(host, { loadCustomCards: vi.fn(() => Promise.reject(new Error("offline"))) });
    await waitFor(() => {
      expect(Array.from((screen.getByTestId(practiceTestid.deck) as HTMLSelectElement).options).map((o) => o.value)).toContain("saved:1");
    });
    fireEvent.change(screen.getByTestId(practiceTestid.deck), { target: { value: "saved:1" } });
    expect(await screen.findByTestId(practiceTestid.customBlocked)).toHaveTextContent("could not be loaded");
    expect(screen.getByTestId(practiceTestid.start)).toBeDisabled();
  });
});

describe("#921 ?try=<custom id> plays the creator's card at once", () => {
  it("R915005 tryPath names the practice route, and ?try= is read only as a custom id", () => {
    expect(tryPath(IMP.id)).toBe(`/practice?try=${IMP.id}`);
    expect(readPracticeParams(`?try=${IMP.id}`).try).toBe(IMP.id);
    expect(readPracticeParams("?try=core-001").try).toBeUndefined();
  });

  it("R915005 once the account's cards are read, a random deck around the card starts on the remembered difficulty", async () => {
    window.localStorage.setItem(PRACTICE_SETUP_KEY, JSON.stringify({ difficulty: "hard", deck: "preset:humans" }));
    visit(`?try=${IMP.id}&seed=try-seed&seat=p2`);
    let answer: (value: CustomCardsResponse) => void = () => {};
    const loadCustomCards = vi.fn(
      () =>
        new Promise<CustomCardsResponse>((resolve) => {
          answer = resolve;
        }),
    );
    const host = fakeHost();
    renderRoute(host, { loadCustomCards });

    expect(await screen.findByTestId(practiceTestid.tryLoading)).toBeInTheDocument();
    expect(screen.queryByTestId(practiceTestid.setup)).toBeNull();
    answer(customResponse([IMP]));

    await screen.findByTestId(practiceTestid.hud);
    expect(host.starts()).toEqual([
      { seed: "try-seed", difficulty: "hard", humanSeat: "p2", deck: { kind: "try", card: IMP.id }, customCards: [IMP_IN_GAME] },
    ]);
    // The remembered setup is the player's, not the try's.
    expect(JSON.parse(window.localStorage.getItem(PRACTICE_SETUP_KEY) ?? "{}")).toMatchObject({ deck: "preset:humans" });
  });

  it("R915005 a card the account does not have, or no account, shows the setup saying so", async () => {
    visit("?try=custom-gone");
    const host = fakeHost();
    renderRoute(host);
    expect(await screen.findByTestId(practiceTestid.tryMissing)).toHaveTextContent("could not be found");
    expect(screen.getByTestId(practiceTestid.setup)).toBeInTheDocument();
    expect(host.starts()).toEqual([]);
    cleanup();

    visit(`?try=${IMP.id}`);
    const anonymous = fakeHost();
    const loadCustomCards = vi.fn(() => Promise.resolve(customResponse([IMP])));
    renderRoute(anonymous, { account: { kind: "anonymous" }, loadCustomCards });
    expect(await screen.findByTestId(practiceTestid.tryMissing)).toBeInTheDocument();
    expect(loadCustomCards).not.toHaveBeenCalled();
    expect(anonymous.starts()).toEqual([]);
  });
});

describe("R915009 a practice game with custom cards records no card statistics", () => {
  it("R915009 a custom game that ends adds nothing to the device's statistics, and Play again deals its cards again", async () => {
    visit(`?try=${IMP.id}`);
    const host = fakeHost();
    renderRoute(host);
    await screen.findByTestId(practiceTestid.hud);
    await concede();
    expect(readPlayerStats().games).toBe(0);

    fireEvent.click(screen.getByTestId(practiceTestid.playAgain));
    await waitFor(() => {
      expect(host.starts()).toHaveLength(2);
    });
    expect(host.starts()[1]).toMatchObject({ deck: { kind: "try", card: IMP.id }, customCards: [IMP_IN_GAME] });
  });

  it("R915009 the same game on official cards alone is counted, so the switch is the custom cards", async () => {
    visit("?difficulty=easy&deck=random");
    const host = fakeHost();
    renderRoute(host);
    await screen.findByTestId(practiceTestid.hud);
    await concede();
    await waitFor(() => {
      expect(readPlayerStats().games).toBe(1);
    });
  });
});

describe("R962000 a sandbox game started with custom cards", () => {
  it("R962000 the Sandbox box with a deck naming a custom card starts a sandbox carrying it, whose panel finds and spawns it", async () => {
    const host = fakeHost();
    renderRoute(host);
    await waitFor(() => {
      expect(Array.from((screen.getByTestId(practiceTestid.deck) as HTMLSelectElement).options).map((o) => o.value)).toContain("saved:1");
    });
    fireEvent.change(screen.getByTestId(practiceTestid.deck), { target: { value: "saved:1" } });
    const preview = await screen.findByTestId(practiceTestid.deckPreview);
    await waitFor(() => {
      expect(within(preview).getByTestId(practiceTestid.deckCard(IMP.id))).toHaveTextContent("Ember Imp");
    });

    fireEvent.click(screen.getByTestId(practiceTestid.sandbox));
    fireEvent.click(screen.getByTestId(practiceTestid.start));
    await screen.findByTestId(practiceTestid.sandboxPanel);
    const [config] = host.starts();
    expect(config?.sandbox).toBe(true);
    expect(config?.customCards).toEqual([IMP_IN_GAME]);

    fireEvent.change(screen.getByTestId(practiceTestid.sandboxSearch), { target: { value: "ember imp" } });
    fireEvent.click(screen.getByTestId(practiceTestid.sandboxResult(IMP.id)));
    fireEvent.click(screen.getByTestId(practiceTestid.sandboxSpawn));
    await waitFor(() => {
      expect(host.requests).toContainEqual({
        type: "act",
        action: { type: "sandbox", op: { kind: "spawn", player: config?.humanSeat, defId: IMP.id, to: { zone: "hand" } } },
      });
    });
  });
});
