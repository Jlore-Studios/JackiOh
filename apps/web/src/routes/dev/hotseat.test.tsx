// `/dev/hotseat` route tests (BUILD M5-T3).
// The session tests fake-port behavior; this route tests URL, deck injection, browser rendering, and `window.__jackioh`.
// `apps/web/package.json` builds E2E in development: the handle exists there and never in production.
// The fake port proves route requests and drawing only (CLAUDE.md rule 7).

import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";

import type { Action, CardDef, CardDefs, EmoteId, PlayerId, PlayerView } from "@jackioh/shared";

import { DEFAULT_EMOTE_HAND } from "../../emotes/hand.ts";

import { testid } from "../../game/contract.ts";
import { setEnginePort } from "../../game/engine.ts";
import type { CreateGameArgs, EnginePort, EngineState } from "../../game/engine.ts";
import { baseView, emptySide } from "../../test/fixtures.ts";
import {
  DEFAULT_SEED,
  E2E_DECKS_KEY,
  HotseatRoute,
  readInjectedDecks,
  readInjectedHandicaps,
  readParams,
} from "./hotseat.tsx";

// Fake engine and catalog

type FakeState = { turn: number; active: PlayerId; pendingFor: PlayerId | null };

type Fake = {
  port: EnginePort;
  createGameArgs: CreateGameArgs[];
  /** Actions the route handed to `reduce`. */
  actions: Action[];
  viewedBy: PlayerId[];
  /** Next accepted `reduce` leaves this prompt open; consumed once. */
  pendingAfterNext: PlayerId | null;
  /** Opening prompt owner (§2.1 mulligan). */
  pendingAfterBegin: PlayerId | null;
};

function def(index: number, over: Partial<CardDef> = {}): CardDef {
  return {
    id: `core-${String(index).padStart(3, "0")}`,
    index: String(index),
    name: `Card ${index}`,
    set: "Core",
    type: "Unit",
    tags: [],
    rarity: "Common",
    token: false,
    // Non-monotonic costs ensure `first20` and `cheap20` differ.
    cost: (index * 3) % 7,
    base: { attack: 1, health: 1, keywords: [], text: "" },
    radiant: { attack: 2, health: 2, keywords: [], text: "" },
    ...over,
  };
}

function catalogOf25(): CardDefs {
  const defs: CardDef[] = [];
  for (let i = 1; i <= 25; i += 1) defs.push(def(i));
  return Object.fromEntries(defs.map((d) => [d.id, d]));
}

function makeEngine(options: { catalog?: CardDefs | "throws"; pendingAfterBegin?: PlayerId | null } = {}): Fake {
  const fake: Fake = {
    port: undefined as unknown as EnginePort,
    createGameArgs: [],
    actions: [],
    viewedBy: [],
    pendingAfterNext: null,
    pendingAfterBegin: options.pendingAfterBegin ?? null,
  };

  let live: FakeState = { turn: 0, active: "p1", pendingFor: null };
  const unwrap = (state: EngineState): FakeState => state as unknown as FakeState;
  const wrap = (state: FakeState): EngineState => state as unknown as EngineState;
  const catalog = options.catalog ?? catalogOf25();

  fake.port = {
    createGame: (args) => {
      fake.createGameArgs.push(args);
      live = { turn: 0, active: "p1", pendingFor: null };
      return wrap(live);
    },
    beginGame: (state) => {
      live = { ...unwrap(state), turn: 1, pendingFor: fake.pendingAfterBegin };
      return { state: wrap(live), events: [] };
    },
    reduce: (state, action) => {
      fake.actions.push(action);
      live = { ...unwrap(state), turn: unwrap(state).turn + 1, pendingFor: fake.pendingAfterNext };
      fake.pendingAfterNext = null;
      return { state: wrap(live), events: [] };
    },
    legalActions: () => [],
    viewFor: (state, player) => {
      fake.viewedBy.push(player);
      const held = unwrap(state);
      const other: PlayerId = player === "p1" ? "p2" : "p1";
      const view: PlayerView = baseView({
        viewer: player,
        turn: held.turn,
        active: held.active,
        you: emptySide(player),
        opponent: emptySide(other, { hand: { count: 0 } }),
        pending:
          held.pendingFor === null
            ? null
            : held.pendingFor === player
              ? { forYou: true, choiceId: "ch1", kind: "target", options: [], min: 1, max: 1, prompt: "Choose" }
              : { forYou: false, pendingFor: held.pendingFor },
      });
      return view;
    },
    hashState: (state) => `hash:${unwrap(state).turn}`,
    catalog:
      catalog === "throws"
        ? () => {
            throw new Error("no catalog is registered");
          }
        : () => catalog,
  };

  return fake;
}

// Mounting

function at(search: string): void {
  window.history.replaceState({}, "", `/dev/hotseat${search}`);
}

/** Render the route and wait for the engine promise the route resolves in an effect. */
async function mount(search = "?seed=42&a=first20&b=cheap20"): Promise<void> {
  at(search);
  await act(async () => {
    render(<HotseatRoute />);
  });
}

afterEach(() => {
  // `globals: false`, so @testing-library/react registers no cleanup.
  cleanup();
  setEnginePort(null);
  delete window.__jackioh;
  delete window.__jackiohE2E;
  window.localStorage.clear();
  at("");
  vi.unstubAllEnvs();
  vi.resetModules();
});

// URL

describe("readParams", () => {
  it("reads ?seed=&a=&b= as BUILD M5-T3 writes them", () => {
    expect(readParams("?seed=42&a=first20&b=cheap20")).toEqual({ seed: "42", a: "first20", b: "cheap20" });
  });

  it("falls back to seed 42 and the default deck for a missing or empty parameter", () => {
    expect(readParams("")).toEqual({ seed: DEFAULT_SEED, a: "first20", b: "first20" });
    expect(readParams("?seed=&a=&b=")).toEqual({ seed: DEFAULT_SEED, a: "first20", b: "first20" });
    expect(readParams("?seed=7")).toEqual({ seed: "7", a: "first20", b: "first20" });
  });
});

describe("readInjectedDecks (ASSUMPTION A1: cy.seedGame's fixture decks)", () => {
  it("reads the window handle, then the localStorage copy that survives a reload", () => {
    window.__jackiohE2E = { decks: { fixture: ["core-001", "core-002"] } };
    expect(readInjectedDecks()).toEqual({ fixture: ["core-001", "core-002"] });

    delete window.__jackiohE2E;
    window.localStorage.setItem(E2E_DECKS_KEY, JSON.stringify({ decks: { stored: ["core-003"] } }));
    expect(readInjectedDecks()).toEqual({ stored: ["core-003"] });
  });

  it("is undefined with no injection, and drops anything that is not a list of card ids", () => {
    expect(readInjectedDecks()).toBeUndefined();

    window.localStorage.setItem(E2E_DECKS_KEY, "not json");
    expect(readInjectedDecks()).toBeUndefined();

    window.__jackiohE2E = { decks: { good: ["core-001"], notAList: "core-002", notStrings: [1, 2] } } as never;
    expect(readInjectedDecks()).toEqual({ good: ["core-001"] });
  });
});

/** R180 fixture: spec 25's 4-card fatigue library. */
const FOUR_CARDS = { deckSize: 4, manaBonus: 0, manaCap: 4, extraOpeningCards: 0, extraDrawsPerTurn: 0 };

describe("readInjectedHandicaps (R180: a fixture deck's handicap, spec 25)", () => {
  it("reads each seat's handicap off the window handle, then off the localStorage copy", () => {
    window.__jackiohE2E = { decks: {}, handicaps: { p1: FOUR_CARDS } };
    expect(readInjectedHandicaps()).toEqual({ p1: FOUR_CARDS });

    delete window.__jackiohE2E;
    const tutorial = { ...FOUR_CARDS, deckSize: 12, manaCap: 3, heroHealth: 20 };
    window.localStorage.setItem(E2E_DECKS_KEY, JSON.stringify({ decks: {}, handicaps: { p2: tutorial } }));
    expect(readInjectedHandicaps()).toEqual({ p2: tutorial });
  });

  it("is undefined with no injection, no handicaps, or none shaped like one", () => {
    expect(readInjectedHandicaps()).toBeUndefined();
    window.__jackiohE2E = { decks: { fixture: ["core-001"] } };
    expect(readInjectedHandicaps()).toBeUndefined();
    window.__jackiohE2E = { decks: {}, handicaps: "p1" } as never;
    expect(readInjectedHandicaps()).toBeUndefined();
    window.__jackiohE2E = { decks: {}, handicaps: { p1: null, p2: [] } };
    expect(readInjectedHandicaps()).toBeUndefined();
  });

  it("drops a seat whose handicap is malformed and keeps the other, with only the handicap's own fields", () => {
    window.__jackiohE2E = {
      decks: {},
      handicaps: {
        p1: { deckSize: 4, manaBonus: 0, manaCap: 4, extraOpeningCards: 0 },
        p2: { ...FOUR_CARDS, deckSize: "60", heroHealth: 20 },
        p3: FOUR_CARDS,
      },
    } as never;
    expect(readInjectedHandicaps()).toBeUndefined();

    window.__jackiohE2E = {
      decks: {},
      handicaps: { p1: { ...FOUR_CARDS, heroHealth: "20" }, p2: { ...FOUR_CARDS, stray: true, deckSize: 60 } },
    } as never;
    expect(readInjectedHandicaps()).toEqual({ p2: { ...FOUR_CARDS, deckSize: 60 } });
  });

  it("leaves the numbers to the engine: a shape with an illegal value is passed on for createGame to refuse", () => {
    // R184 belongs to `validateHandicap`; the route must not duplicate it.
    window.__jackiohE2E = { decks: {}, handicaps: { p1: { ...FOUR_CARDS, deckSize: 61, manaBonus: -1 } } };
    expect(readInjectedHandicaps()).toEqual({ p1: { ...FOUR_CARDS, deckSize: 61, manaBonus: -1 } });
  });
});

// Starting the game

describe("the route starts one game from the URL", () => {
  it("hands the engine the seed and the two decks the a= and b= ids resolve to", async () => {
    const fake = makeEngine();
    setEnginePort(fake.port);
    await mount("?seed=seed-7&a=first20&b=cheap20");

    expect(fake.createGameArgs).toHaveLength(1);
    const args = fake.createGameArgs[0];
    expect(args?.seed).toBe("seed-7");
    expect(args?.decks[0]).toHaveLength(20);
    expect(args?.decks[1]).toHaveLength(20);
    expect(args?.decks[0]).not.toEqual(args?.decks[1]);
    expect(args?.decks[0]?.[0]).toBe("core-001");
    expect(args?.catalog).toBe(fake.port.catalog?.());

    const bar = document.querySelector(".hotseat-bar");
    expect(bar?.textContent).toContain("seed-7");
    expect(bar?.textContent).toContain("p1");
    expect(screen.getByTestId(testid.board)).toBeInTheDocument();
  });

  it("plays the injected fixture decks Cypress parked in localStorage", async () => {
    window.localStorage.setItem(
      E2E_DECKS_KEY,
      JSON.stringify({ decks: { fixture: Array.from({ length: 20 }, (_, i) => `core-${String(i + 1).padStart(3, "0")}`) } }),
    );
    const fake = makeEngine();
    setEnginePort(fake.port);
    await mount("?seed=42&a=fixture&b=fixture");

    expect(fake.createGameArgs[0]?.decks[0]?.[19]).toBe("core-020");
  });

  it("R180 hands the injected handicaps to createGame and to the dev handle, and sizes the deck by them", async () => {
    const tiny = ["core-001", "core-002", "core-003", "core-004"];
    window.__jackiohE2E = { decks: { tiny }, handicaps: { p1: FOUR_CARDS } };
    const fake = makeEngine();
    setEnginePort(fake.port);
    await mount("?seed=42&a=tiny&b=first20");

    expect(fake.createGameArgs).toHaveLength(1);
    expect(fake.createGameArgs[0]?.handicaps).toEqual({ p1: FOUR_CARDS });
    expect(fake.createGameArgs[0]?.decks[0]).toEqual(tiny);
    expect(window.__jackioh?.handicaps, "the replay fold needs them (cy.replayCheck)").toEqual({ p1: FOUR_CARDS });
    expect(screen.getByTestId(testid.board)).toBeInTheDocument();
  });

  it("creates the game exactly as before when nothing injects a handicap", async () => {
    const fake = makeEngine();
    setEnginePort(fake.port);
    await mount();

    expect("handicaps" in (fake.createGameArgs[0] ?? {})).toBe(false);
    expect(window.__jackioh?.handicaps).toEqual({});
  });

  it("refuses a deck its seat's handicap does not size, before the engine is asked", async () => {
    // p2 has no handicap, so §2.6's 20-card rule applies.
    window.__jackiohE2E = { decks: { tiny: ["core-001", "core-002", "core-003", "core-004"] }, handicaps: { p1: FOUR_CARDS } };
    const fake = makeEngine();
    setEnginePort(fake.port);
    await mount("?seed=42&a=first20&b=tiny");

    expect(screen.queryByTestId(testid.board)).toBeNull();
    expect(fake.createGameArgs).toHaveLength(0);
    expect(screen.getByText(/its seat's handicap wants exactly 4 \(R184\)/)).toBeInTheDocument();
  });

  it("renders the engine's refusal of an illegal handicap instead of a game", async () => {
    window.__jackiohE2E = { decks: {}, handicaps: { p1: { ...FOUR_CARDS, deckSize: 20, manaBonus: -1 } } };
    const fake = makeEngine();
    const createGame = fake.port.createGame;
    fake.port.createGame = (args) => {
      if ((args.handicaps?.p1?.manaBonus ?? 0) < 0) {
        throw new Error("p1: handicap manaBonus must be a non-negative integer (R180), got -1");
      }
      return createGame(args);
    };
    setEnginePort(fake.port);
    await mount();

    expect(screen.queryByTestId(testid.board)).toBeNull();
    expect(screen.getByText(/handicap manaBonus must be a non-negative integer/)).toBeInTheDocument();
  });

  it("renders the engine's own refusal instead of a game when a deck id is unknown", async () => {
    const fake = makeEngine();
    setEnginePort(fake.port);
    await mount("?seed=42&a=aggro-please&b=first20");

    expect(screen.queryByTestId(testid.board)).toBeNull();
    expect(screen.getByText(/unknown deck "aggro-please"/)).toBeInTheDocument();
    expect(fake.createGameArgs).toHaveLength(0);
  });

  it("renders the missing exports when the engine cannot be loaded, and draws no board", async () => {
    // CLAUDE.md rule 7 / §10.8: an unavailable port shows its failure, never a stubbed game.
    // The dynamic import needs the module mock and re-import.
    vi.resetModules();
    vi.doMock("../../game/engine.ts", async () => {
      const actual = await vi.importActual<typeof import("../../game/engine.ts")>("../../game/engine.ts");
      return {
        ...actual,
        loadEnginePort: () => Promise.reject(new actual.EngineUnavailableError(["viewFor", "hashState"])),
      };
    });

    try {
      const route = await import("./hotseat.tsx");
      at("?seed=42&a=first20&b=first20");
      await act(async () => {
        render(<route.HotseatRoute />);
      });

      expect(screen.queryByTestId(testid.board)).toBeNull();
      expect(screen.getAllByText(/viewFor, hashState/).length).toBeGreaterThan(0);
      expect(screen.getByText(/waiting on M3/)).toBeInTheDocument();
    } finally {
      vi.doUnmock("../../game/engine.ts");
      vi.resetModules();
    }
  });
});

// Seat

describe("a way back (integration: every screen has one)", () => {
  it("the hotseat bar's Back goes to the landing page", async () => {
    const fake = makeEngine();
    setEnginePort(fake.port);
    window.history.replaceState(null, "", "/dev/hotseat?seed=42");
    await mount();

    fireEvent.click(screen.getByTestId("nav-back"));
    expect(window.location.pathname).toBe("/");
  });
});

describe("the seat", () => {
  it("hands the device over on the button, and re-renders viewFor for the other player", async () => {
    const fake = makeEngine();
    setEnginePort(fake.port);
    await mount();

    expect(screen.getByTestId(testid.seatSwitch)).toHaveTextContent("Hand over to p2");
    fake.viewedBy.length = 0;

    await act(async () => {
      fireEvent.click(screen.getByTestId(testid.seatSwitch));
    });

    expect(fake.viewedBy).toContain("p2");
    expect(fake.viewedBy).not.toContain("p1");
    expect(screen.getByTestId(testid.seatSwitch)).toHaveTextContent("Hand over to p1");
    expect(window.__jackioh?.seat).toBe("p2");
  });

  it("switches seats by itself when a prompt belongs to the other player", async () => {
    const fake = makeEngine();
    setEnginePort(fake.port);
    await mount();
    expect(window.__jackioh?.seat).toBe("p1");

    fake.pendingAfterNext = "p2";
    await act(async () => {
      window.__jackioh?.dispatch({ type: "endTurn" });
    });

    expect(window.__jackioh?.seat).toBe("p2");
    expect(window.__jackioh?.view().viewer).toBe("p2");
    expect(window.__jackioh?.view().pending).toMatchObject({ forYou: true });
    await waitFor(() => {
      expect(screen.getByTestId(testid.seatSwitch)).toHaveTextContent("Hand over to p1");
    });
  });

  it("follows the opening prompt before the first render (§2.1 mulligan)", async () => {
    const fake = makeEngine({ pendingAfterBegin: "p2" });
    setEnginePort(fake.port);
    await mount();

    expect(window.__jackioh?.seat).toBe("p2");
    expect(screen.getByTestId(testid.seatSwitch)).toHaveTextContent("Hand over to p1");
  });
});

// window.__jackioh

// R1341 emote hands

describe("R1341 the hotseat's emote hands", () => {
  function offered(): (string | undefined)[] {
    fireEvent.click(screen.getByTestId("hero-you"));
    const menu = screen.getByTestId("emote-menu");
    return Array.from(menu.querySelectorAll<HTMLElement>("[role=menuitem]")).map((item) => item.dataset.testid);
  }

  it("R1341 each seat's menu holds the hand the port deals it from the URL's seed", async () => {
    const fake = makeEngine();
    const asked: [string, PlayerId][] = [];
    const hands: Record<PlayerId, EmoteId[]> = {
      p1: ["greetings", "thanks", "threaten", "laugh", "wahWah", "wave", "thumbsUp", "party"],
      p2: ["wellPlayed", "oops", "thanks", "sob", "clap", "skull", "cool", "gasp"],
    };
    setEnginePort({
      ...fake.port,
      dealEmoteHand: (seed, seat) => {
        asked.push([seed, seat]);
        return hands[seat];
      },
    });
    await mount("?seed=777&a=first20&b=cheap20");

    expect(asked).toEqual([
      ["777", "p1"],
      ["777", "p2"],
    ]);
    expect(offered()).toEqual(hands.p1.map((emote) => `emote-${emote}`));
  });

  it("R1343 a port that deals no hand leaves both seats on the default hand", async () => {
    setEnginePort(makeEngine().port);
    await mount();
    expect(offered()).toEqual(DEFAULT_EMOTE_HAND.map((emote) => `emote-${emote}`));
  });
});

describe("window.__jackioh outside a production build", () => {
  it("publishes { state, dispatch, seed } plus what Cypress drives two seats with", async () => {
    const fake = makeEngine();
    setEnginePort(fake.port);
    await mount("?seed=42&a=first20&b=cheap20");

    const handle = window.__jackioh;
    expect(handle, "BUILD M5-T3: the dev handle is what every e2e spec reaches through").toBeDefined();
    if (handle === undefined) return;

    expect(handle.seed).toBe("42");
    expect(handle.seat).toBe("p1");
    expect(handle.log).toEqual([]);
    expect(handle.decks[0]).toHaveLength(20);
    expect(handle.view().viewer).toBe("p1");
    expect(handle.legal()).toEqual([]);
    expect(typeof handle.hash()).toBe("string");
    expect(handle.state).toBeDefined();
  });

  it("exposes getters, so a spec that re-reads the handle sees the current game", async () => {
    const fake = makeEngine();
    setEnginePort(fake.port);
    await mount();
    const handle = window.__jackioh;
    const before = handle?.hash();

    await act(async () => {
      handle?.dispatch({ type: "endTurn" });
    });

    expect(handle?.hash()).not.toBe(before);
    expect(handle?.log.map((action) => action.nonce)).toEqual(["n0"]);
    expect(handle?.log.map((action) => action.type)).toEqual(["endTurn"]);
    expect(fake.actions).toHaveLength(1);
  });

  it("dispatching as the other seat switches the device first, never forging the action", async () => {
    const fake = makeEngine();
    setEnginePort(fake.port);
    await mount();

    await act(async () => {
      window.__jackioh?.dispatch({ type: "endTurn", playerId: "p2" });
    });

    expect(window.__jackioh?.seat).toBe("p2");
    expect(fake.actions.map((action) => action.playerId)).toEqual(["p2"]);
  });

  it("is removed when the route unmounts, so a stale handle cannot answer a later spec", async () => {
    const fake = makeEngine();
    setEnginePort(fake.port);
    at("?seed=42&a=first20&b=first20");
    let view!: ReturnType<typeof render>;
    await act(async () => {
      view = render(<HotseatRoute />);
    });
    expect(window.__jackioh).toBeDefined();

    await act(async () => {
      view.unmount();
    });
    expect(window.__jackioh).toBeUndefined();
  });
});

describe("window.__jackioh in a production build", () => {
  /** The E2E build uses development mode for the handle; production must never publish it. */
  it("publishes nothing: MODE=production removes the handle the twelve specs drive", async () => {
    vi.stubEnv("MODE", "production");
    vi.resetModules();

    const engine = await import("../../game/engine.ts");
    const route = await import("./hotseat.tsx");
    expect(import.meta.env.MODE, "vi.stubEnv must reach import.meta.env, or this proves nothing").toBe(
      "production",
    );

    const fake = makeEngine();
    engine.setEnginePort(fake.port);
    at("?seed=42&a=first20&b=first20");
    await act(async () => {
      render(<route.HotseatRoute />);
    });

    expect(screen.getByTestId(testid.board)).toBeInTheDocument();
    expect(window.__jackioh).toBeUndefined();
    // E2E injection, including R180 handicaps, is dev-only.
    window.__jackiohE2E = { decks: { fixture: ["core-001"] }, handicaps: { p1: FOUR_CARDS } };
    expect(route.readInjectedDecks()).toBeUndefined();
    expect(route.readInjectedHandicaps()).toBeUndefined();

    engine.setEnginePort(null);
  });
});
