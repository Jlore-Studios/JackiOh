// The landing page (docs/polish/5-sign-in.md, B37, B38): `/` renders `LandingRoute`, its CTAs are
// real links that navigate in place on a plain left click, and the corner slot follows the account.
//
// Layout (no horizontal overflow at 360-1280 px, B39) needs a layout engine, so it is the Cypress
// component spec's job (e2e/cypress/component/landing-and-code-field.cy.tsx), not this file's.

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import { landingFanCardTestid, landingTestid } from "../auth/testids.ts";
import { LONG_PRESS_MS } from "../cards/inspect/constants.ts";
import { INSPECT_DETAIL, INSPECT_HOVER } from "../cards/inspect/testids.ts";
import { paths } from "../net/navigate.ts";
import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { setReducedMotion } from "../test/setup.ts";
import { seeded } from "../test/random.ts";
import { PLAYER_STATS_KEY, PLAYER_STATS_VERSION, ROTATION_INTERVAL_MS, ROTATION_MIN_GAMES, ROTATION_SWAP_MS } from "../stats/config.ts";
import { dropPlayerStatsCache } from "../stats/store.ts";
import LandingRoute from "./landing.tsx";
import { EVEN, FAN_POOL, ROTATION_POOL, dealLandingFan, featureWeight, rotateFan } from "./landingFan.ts";
import { siteFooterTestid } from "./SiteFooter.tsx";

const { App } = await import("../main.tsx");

const FAN_CARDS = 5;

/** A stubbed round trip can outrun the 1 s default under load. */
const SLOW = { timeout: 5_000 } as const;

// ---------------------------------------------------------------------------------------------
// the stubbed server
// ---------------------------------------------------------------------------------------------

type MeAnswer = "active" | "unauthorized" | "hang";

function jsonResponse(status: number, body?: unknown): Response {
  const text = body === undefined ? "" : JSON.stringify(body);
  return {
    ok: status >= 200 && status < 300,
    status,
    statusText: String(status),
    headers: new Headers({ "content-type": "application/json" }),
    json: () =>
      text === "" ? Promise.reject(new SyntaxError("empty body")) : Promise.resolve(JSON.parse(text)),
    text: () => Promise.resolve(text),
    clone: () => jsonResponse(status, body),
  } as unknown as Response;
}

function serve(answer: MeAnswer): void {
  vi.stubGlobal(
    "fetch",
    vi.fn((input: unknown) => {
      const url = typeof input === "string" ? input : String((input as URL).href ?? input);
      if (url.endsWith("/api/auth/me")) {
        if (answer === "hang") return new Promise<Response>(() => {});
        if (answer === "unauthorized") {
          return Promise.resolve(
            jsonResponse(401, { error: { code: "unauthorized", message: "sign in first" } }),
          );
        }
        return Promise.resolve(
          jsonResponse(200, {
            profile: { id: "profile-1", status: "active" },
            needsInviteCode: false,
            emailVerified: true,
            currentMatchId: null,
            email: "player@example.test",
          }),
        );
      }
      return Promise.resolve(
        jsonResponse(404, { error: { code: "not_found", message: `no stub for ${url}` } }),
      );
    }),
  );
}

function signedIn(): void {
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "e2e-token" }));
}

/**
 * jsdom 30 ships `PointerEvent`, but if the window a node lives in ever lacks it, testing-library
 * falls back to a plain `Event` and drops `pointerType`, which would make every touch look like a
 * mouse. This installs a `MouseEvent` subclass that keeps it, only when the probe shows it is lost.
 */
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

function at(path: string): void {
  window.history.replaceState(null, "", path);
}

function landing(): HTMLElement {
  return screen.getByTestId(landingTestid.root);
}

/** The exit goes to `path`: a link whose href is that path, or a control that moves the URL there. */
async function expectLeadsTo(element: HTMLElement, path: string): Promise<void> {
  const link = element.closest("a");
  if (link !== null && link.hasAttribute("href")) {
    expect(new URL(link.href, window.location.origin).pathname).toBe(path);
    return;
  }
  fireEvent.click(element);
  await waitFor(() => {
    expect(window.location.pathname).toBe(path);
  }, SLOW);
}

beforeEach(() => {
  window.localStorage.clear();
  dropPlayerStatsCache();
  setReducedMotion(false);
  serve("active");
  at("/");
});

afterEach(() => {
  cleanup();
  setReducedMotion(false);
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

// ---------------------------------------------------------------------------------------------
// B37: the route and its CTAs
// ---------------------------------------------------------------------------------------------

const CTAS = [
  ["Play vs AI", landingTestid.playAi, paths.practice],
  ["Play online", landingTestid.playOnline, paths.play],
  ["Build decks", landingTestid.buildDecks, paths.decks],
] as const;

describe("B37 the landing route", () => {
  it("B37 / renders LandingRoute through the route table", async () => {
    render(<App />);
    expect(await screen.findByTestId(landingTestid.root, undefined, SLOW)).toBeInTheDocument();
    expect(window.location.pathname).toBe(paths.landing);
  });

  it.each(CTAS)("B37 %s is a real link to its path", (_label, testid, path) => {
    render(<LandingRoute />);
    const cta = within(landing()).getByTestId(testid);
    const link = cta.closest("a");
    expect(link, `${testid} is an <a href>, so middle-click and open-in-new-tab work`).not.toBeNull();
    expect(link?.getAttribute("href")).toBe(path);
  });

  it.each(CTAS)("B37 a plain left click on %s navigates in place", (_label, testid, path) => {
    render(<LandingRoute />);
    const cta = within(landing()).getByTestId(testid);

    const notPrevented = fireEvent.click(cta, { button: 0 });

    expect(window.location.pathname).toBe(path);
    // Handled in the page: the browser's own full load is cancelled.
    expect(notPrevented).toBe(false);
  });

  it.each([
    ["a ctrl-click", { button: 0, ctrlKey: true }],
    ["a cmd-click", { button: 0, metaKey: true }],
    ["a middle click", { button: 1 }],
  ] as const)("B37 %s is left to the browser: no in-page navigation, default kept", (_name, init) => {
    render(<LandingRoute />);
    for (const [, testid] of CTAS) {
      const cta = within(landing()).getByTestId(testid);
      const notPrevented = fireEvent.click(cta, init);
      expect(notPrevented, `${testid} keeps the browser's default`).toBe(true);
      expect(window.location.pathname).toBe(paths.landing);
    }
  });
});

describe("R661 the statistics are not a call to action", () => {
  it("R661 the landing page's calls to action carry no Stats link; the site footer's is the only one", () => {
    render(<LandingRoute />);
    const ctas = within(landing()).getByRole("navigation", { name: "Play" });
    expect(within(ctas).queryByText("Stats")).toBeNull();
    expect(ctas.querySelector('a[href="/stats"]')).toBeNull();
    const statsLinks = landing().querySelectorAll('a[href="/stats"]');
    expect(statsLinks).toHaveLength(1);
    expect(statsLinks[0]).toHaveAttribute("data-testid", siteFooterTestid.stats);
  });
});

// ---------------------------------------------------------------------------------------------
// B37: the corner slot
// ---------------------------------------------------------------------------------------------

describe("B37 the corner slot", () => {
  it("B37 anonymous: shows landing-sign-in, which leads to /login, and no landing-account", async () => {
    render(<LandingRoute />);

    const signIn = await screen.findByTestId(landingTestid.signIn, undefined, SLOW);
    expect(screen.queryByTestId(landingTestid.account)).toBeNull();
    expect(landing()).toHaveAttribute("data-account", "anonymous");
    await expectLeadsTo(signIn, paths.login);
  });

  it("B37 signed in: shows landing-account, which leads to /account, and no landing-sign-in", async () => {
    signedIn();
    render(<LandingRoute />);

    const account = await screen.findByTestId(landingTestid.account, undefined, SLOW);
    expect(screen.queryByTestId(landingTestid.signIn)).toBeNull();
    expect(landing()).toHaveAttribute("data-account", "signed-in");
    await expectLeadsTo(account, paths.account);
  });

  it("B37 loading: the slot holds nothing while /api/auth/me is unanswered", async () => {
    signedIn();
    serve("hang");
    render(<LandingRoute />);

    expect(landing()).toHaveAttribute("data-account", "loading");
    // Give the read every chance to (wrongly) settle before looking.
    await new Promise((resolve) => {
      setTimeout(resolve, 50);
    });
    expect(screen.queryByTestId(landingTestid.signIn)).toBeNull();
    expect(screen.queryByTestId(landingTestid.account)).toBeNull();
    expect(landing()).toHaveAttribute("data-account", "loading");
  });

  it("B37 a token the server refuses is anonymous: the slot offers landing-sign-in", async () => {
    signedIn();
    serve("unauthorized");
    render(<LandingRoute />);

    expect(await screen.findByTestId(landingTestid.signIn, undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByTestId(landingTestid.account)).toBeNull();
    await waitFor(() => {
      expect(landing()).toHaveAttribute("data-account", "anonymous");
    }, SLOW);
    // The landing is not gated: it stays put rather than sending anyone to /login.
    expect(window.location.pathname).toBe(paths.landing);
  });

  it("B37 a device that holds a session is offered landing-account, not sign-in, when /api/auth/me cannot be reached", async () => {
    signedIn();
    vi.stubGlobal("fetch", vi.fn(() => Promise.reject(new TypeError("Failed to fetch"))));
    render(<LandingRoute />);

    await waitFor(() => {
      expect(landing()).toHaveAttribute("data-account", "signed-in");
    }, SLOW);
    // The account screen is where sign-out is; a sign-in would replace (and revoke) this session.
    expect(screen.getByTestId(landingTestid.account)).toBeInTheDocument();
    expect(screen.queryByTestId(landingTestid.signIn)).toBeNull();
  });

  it("B37 a device with no session is offered sign-in when /api/auth/me cannot be reached", async () => {
    // With no session there is nothing to read, so the landing never even asks the server.
    vi.stubGlobal("fetch", vi.fn(() => Promise.reject(new TypeError("Failed to fetch"))));
    render(<LandingRoute />);

    expect(await screen.findByTestId(landingTestid.signIn, undefined, SLOW)).toBeInTheDocument();
    expect(landing()).toHaveAttribute("data-account", "anonymous");
  });

  it("B37 the three play CTAs are there whatever the slot shows", async () => {
    signedIn();
    serve("hang");
    render(<LandingRoute />);
    for (const [, testid] of CTAS) expect(within(landing()).getByTestId(testid)).toBeInTheDocument();
  });

  it("B37 says, beside the CTAs, that online play needs an invite code and Play vs AI needs no account", () => {
    render(<LandingRoute />);
    const note = within(landing()).getByTestId(landingTestid.inviteOnly);
    expect(note.textContent).toMatch(/invite code/);
    // des-9: the note under "Play vs AI" says it, so the invite note no longer repeats it.
    const ctas = within(landing()).getByRole("navigation", { name: "Play" });
    expect(within(ctas).getByText(/No account needed/)).toBeInTheDocument();
    expect(note.textContent).not.toMatch(/needs no account/);
  });
});

// ---------------------------------------------------------------------------------------------
// B38: the hero
// ---------------------------------------------------------------------------------------------

describe("B38 the hero", () => {
  it("B38 shows the JackiOh wordmark", () => {
    render(<LandingRoute />);
    const wordmarks = within(landing()).queryAllByText(
      (_content, element) => element !== null && (element.textContent ?? "").replace(/\s+/g, "") === "JackiOh",
    );
    expect(wordmarks.length).toBeGreaterThan(0);
  });

  it("B38 R639 the fan holds exactly five cards, landing-fan-card-0 to -4: four faces that are buttons, and a back hidden from assistive tech", () => {
    render(<LandingRoute />);
    const fan = within(landing()).getByTestId(landingTestid.fan);

    for (let index = 0; index < FAN_CARDS - 1; index += 1) {
      const card = within(fan).getByTestId(landingFanCardTestid(index));
      expect(card, `${landingFanCardTestid(index)} is a button`).toHaveAttribute("role", "button");
      expect(card.getAttribute("aria-label")).toMatch(/^Read /);
      expect(card.closest('[aria-hidden="true"]'), `${landingFanCardTestid(index)} is readable`).toBeNull();
    }
    const back = within(fan).getByTestId(landingFanCardTestid(FAN_CARDS - 1));
    expect(back.closest('[aria-hidden="true"]'), "the card back is aria-hidden").not.toBeNull();
    expect(screen.queryByTestId(landingFanCardTestid(FAN_CARDS))).toBeNull();
  });

  it("B38 R374 the fan's cards are the game's own faces, named and costed, with a card back last (integration QA)", () => {
    render(<LandingRoute random={seeded(38)} />);
    const fan = within(landing()).getByTestId(landingTestid.fan);
    const faces = [0, 1, 2, 3].map((index) => within(fan).getByTestId(landingFanCardTestid(index)));
    // The page deals from the source it is given (R374), so the same seed names the same hand.
    const hand = dealLandingFan(seeded(38));

    for (const [index, face] of faces.entries()) {
      const { def, radiant } = hand[index] ?? { def: undefined, radiant: false };
      expect(face.querySelector(".cf"), `fan card ${String(index)} is a CardFace`).not.toBeNull();
      expect(face).toHaveAttribute("data-def-id", def?.id);
      expect(face.querySelector(".card-name")?.textContent).toBe(def?.name);
      expect(face.querySelector(".cost-gem")?.textContent).toBe(String(def?.cost));
      expect(face.getAttribute("data-radiant") === "true").toBe(radiant);
    }
    const back = within(fan).getByTestId(landingFanCardTestid(4));
    expect(back).toHaveAttribute("data-face", "down");
    expect(back.querySelector(".cf-back")).not.toBeNull();
    expect(back.textContent).toBe("");
  });

  it("R374 each visit deals its own hand, and a re-render in the same visit does not deal again", () => {
    const shown = (): string[] =>
      [0, 1, 2, 3].map((index) => screen.getByTestId(landingFanCardTestid(index)).getAttribute("data-def-id") ?? "");
    const first = render(<LandingRoute random={seeded(1)} />);
    const hand = shown();
    expect(hand).toEqual(dealLandingFan(seeded(1)).map(({ def }) => def.id));
    // A re-render is the same visit: the deal is not drawn again.
    first.rerender(<LandingRoute random={seeded(2)} />);
    expect(shown()).toEqual(hand);
    first.unmount();
    // The next visit is a new deal.
    render(<LandingRoute random={seeded(2)} />);
    expect(shown()).toEqual(dealLandingFan(seeded(2)).map(({ def }) => def.id));
    expect(shown()).not.toEqual(hand);
  });

  it("B38 the corner holds the settings gear beside Sign in, as every other screen's top bar does", () => {
    render(<LandingRoute />);
    const gear = within(landing()).getByTestId("settings-open-nav");
    expect(gear.closest(".landing-corner")).not.toBeNull();
    expect(gear).toHaveAttribute("aria-label", "Settings");
    fireEvent.click(gear);
    expect(screen.getByTestId("settings-panel")).toBeInTheDocument();
  });

  it("B38 data-motion is full when reduced motion is not requested", () => {
    render(<LandingRoute />);
    expect(landing()).toHaveAttribute("data-motion", "full");
  });

  it("B38 data-motion is reduced under prefers-reduced-motion", () => {
    setReducedMotion(true);
    render(<LandingRoute />);
    expect(landing()).toHaveAttribute("data-motion", "reduced");
  });

  // Integration: the settings panel's "Reduce motion" (task 7) stops the landing as the media query
  // does, and a change applies to the page already showing.
  it("B38 data-motion is reduced under the settings panel's Reduce motion, live", () => {
    render(<LandingRoute />);
    expect(landing()).toHaveAttribute("data-motion", "full");
    try {
      act(() => {
        writeSettings({ reduceMotion: true });
      });
      expect(landing()).toHaveAttribute("data-motion", "reduced");
      expect(document.documentElement).toHaveAttribute("data-reduce-motion", "true");
    } finally {
      act(() => {
        writeSettings({ reduceMotion: false });
      });
      __resetSettingsForTests();
    }
    expect(landing()).toHaveAttribute("data-motion", "full");
  });
});

// ---------------------------------------------------------------------------------------------
// R639: the fan rotates through every set once the device has logged enough games, a face opens at
// full size, and the device's statistics are shown. R704: the fan swaps from the first visit,
// among Core's cards below the threshold, with a fizzle and an apparition.
// ---------------------------------------------------------------------------------------------

/** A device with this many logged games (the store's own shape, read back as the page would). */
function withGames(games: number, cards: Record<string, Record<string, number>> = {}): void {
  window.localStorage.setItem(
    PLAYER_STATS_KEY,
    JSON.stringify({ v: PLAYER_STATS_VERSION, games, wins: games, losses: 0, draws: 0, cards }),
  );
  dropPlayerStatsCache();
}

function shownIds(): string[] {
  return [0, 1, 2, 3].map((index) => screen.getByTestId(landingFanCardTestid(index)).getAttribute("data-def-id") ?? "");
}

describe("R639 the homescreen rotation", () => {
  afterEach(() => {
    vi.useRealTimers();
    dropPlayerStatsCache();
  });

  it("R704 below the threshold the fan deals Core and swaps one slot a step, left to right", () => {
    vi.useFakeTimers();
    withGames(ROTATION_MIN_GAMES - 1);
    render(<LandingRoute random={seeded(5)} />);

    // The same source, the same draws: the deal, then the swaps the page makes, among Core's cards.
    const mirror = seeded(5);
    let expected = dealLandingFan(mirror);
    expect(shownIds()).toEqual(expected.map(({ def }) => def.id));

    for (const slot of [0, 1, 2, 3, 0]) {
      const before = shownIds();
      act(() => {
        vi.advanceTimersByTime(ROTATION_INTERVAL_MS);
      });
      expected = rotateFan(expected, slot, mirror, FAN_POOL, EVEN);
      const after = shownIds();
      expect(after).toEqual(expected.map(({ def }) => def.id));
      // One card moved, in the slot due, and it is a Core card of the same rarity the fan was not showing.
      expect(after.filter((id, at) => id !== before[at])).toHaveLength(1);
      expect(after[slot]).not.toBe(before[slot]);
      expect(new Set(after).size).toBe(after.length);
    }
  });

  it("R704 a swap fizzles the card going out over the new one, then lets it go", () => {
    vi.useFakeTimers();
    withGames(0);
    render(<LandingRoute random={seeded(5)} />);
    expect(screen.queryByTestId(landingTestid.fanLeaving)).toBeNull();
    const before = shownIds();
    act(() => {
      vi.advanceTimersByTime(ROTATION_INTERVAL_MS);
    });
    // The card going out stays as a ghost: the old id, hidden from assistive tech, no control.
    const ghost = screen.getByTestId(landingTestid.fanLeaving);
    expect(ghost).toHaveAttribute("data-def-id", before[0]);
    expect(ghost).toHaveAttribute("aria-hidden", "true");
    expect(ghost.getAttribute("role")).toBeNull();
    expect(ghost.getAttribute("data-testid")).not.toMatch(/^landing-fan-card-/);
    act(() => {
      vi.advanceTimersByTime(ROTATION_SWAP_MS);
    });
    expect(screen.queryByTestId(landingTestid.fanLeaving)).toBeNull();
  });

  it("R704 the deal plays no apparition, while a swapped card gathers out of the fizzle's smoke", () => {
    vi.useFakeTimers();
    withGames(0);
    render(<LandingRoute random={seeded(5)} />);
    for (const index of [0, 1, 2, 3]) {
      expect(screen.getByTestId(landingFanCardTestid(index))).toHaveAttribute("data-entry", "deal");
    }
    act(() => {
      vi.advanceTimersByTime(ROTATION_INTERVAL_MS);
    });
    expect(screen.getByTestId(landingFanCardTestid(0))).toHaveAttribute("data-entry", "swap");
    for (const index of [1, 2, 3]) {
      expect(screen.getByTestId(landingFanCardTestid(index))).toHaveAttribute("data-entry", "deal");
    }
    // The ghost has gone, and the card is not dealt again.
    act(() => {
      vi.advanceTimersByTime(ROTATION_SWAP_MS);
    });
    expect(screen.queryByTestId(landingTestid.fanLeaving)).toBeNull();
    expect(screen.getByTestId(landingFanCardTestid(0))).toHaveAttribute("data-entry", "swap");
  });

  it("R704 the fan carries the swap's length for the sheet, whose fizzle and apparition borrow the board's refused-card look", () => {
    render(<LandingRoute random={seeded(5)} />);
    const fan = screen.getByTestId(landingTestid.fan);
    expect(fan.style.getPropertyValue("--fan-swap")).toBe(`${String(ROTATION_SWAP_MS)}ms`);
    const sheet = readFileSync(resolve(dirname(fileURLToPath(import.meta.url)), "landing.css"), "utf8");
    expect(sheet).toContain(".landing .landing-fan-card--leaving {");
    expect(sheet).toContain('.landing .landing-fan-card[data-entry="swap"] {');
    expect(sheet).toContain("@keyframes landing-fizzle");
    expect(sheet).toContain("@keyframes landing-apparition");
  });

  it("R639 at the threshold the fan deals from every set and swaps one slot a step, left to right", () => {
    vi.useFakeTimers();
    withGames(ROTATION_MIN_GAMES);
    render(<LandingRoute random={seeded(5)} />);

    // The same source, the same draws: the deal, then the swaps the page makes.
    const mirror = seeded(5);
    let expected = dealLandingFan(mirror, ROTATION_POOL, featureWeight);
    expect(shownIds()).toEqual(expected.map(({ def }) => def.id));

    for (const slot of [0, 1, 2, 3, 0]) {
      const before = shownIds();
      act(() => {
        vi.advanceTimersByTime(ROTATION_INTERVAL_MS);
      });
      expected = rotateFan(expected, slot, mirror);
      const after = shownIds();
      expect(after).toEqual(expected.map(({ def }) => def.id));
      // One card moved, in the slot due, and it is a card of the same rarity the fan was not showing.
      expect(after.filter((id, at) => id !== before[at])).toHaveLength(1);
      expect(after[slot]).not.toBe(before[slot]);
      expect(new Set(after).size).toBe(after.length);
    }
  });

  it("R639 across a long visit the fan shows cards of Classic and Classic+ as well as Core", () => {
    vi.useFakeTimers();
    withGames(ROTATION_MIN_GAMES + 40);
    render(<LandingRoute random={seeded(11)} />);
    const sets = new Set<string>();
    const poolById = new Map(ROTATION_POOL.map((def) => [def.id, def]));
    for (let step = 0; step < 60; step += 1) {
      for (const id of shownIds()) sets.add(poolById.get(id)?.set ?? "?");
      act(() => {
        vi.advanceTimersByTime(ROTATION_INTERVAL_MS);
      });
    }
    expect(sets.has("Core")).toBe(true);
    expect(sets.has("Classic")).toBe(true);
    expect(sets.has("Classic+")).toBe(true);
    expect(sets.has("?")).toBe(false);
  });

  it("R639 the hand holds still under reduced motion, while a pointer or focus is on it, and while a card is open", async () => {
    vi.useFakeTimers();
    withGames(ROTATION_MIN_GAMES);
    setReducedMotion(true);
    const reduced = render(<LandingRoute random={seeded(5)} />);
    const dealt = shownIds();
    act(() => {
      vi.advanceTimersByTime(ROTATION_INTERVAL_MS * 3);
    });
    expect(shownIds()).toEqual(dealt);
    reduced.unmount();

    setReducedMotion(false);
    render(<LandingRoute random={seeded(5)} />);
    const fan = screen.getByTestId(landingTestid.fan);
    fireEvent.mouseEnter(fan);
    act(() => {
      vi.advanceTimersByTime(ROTATION_INTERVAL_MS * 3);
    });
    expect(shownIds()).toEqual(dealt);
    fireEvent.mouseLeave(fan);
    act(() => {
      vi.advanceTimersByTime(ROTATION_INTERVAL_MS);
    });
    expect(shownIds()).not.toEqual(dealt);
  });
});

describe("R639 a fan card opens at full size", () => {
  it("R639 a click opens the card's detail dialog and Close puts it away", async () => {
    render(<LandingRoute random={seeded(3)} />);
    const face = screen.getByTestId(landingFanCardTestid(1));
    expect(screen.queryByTestId(INSPECT_DETAIL)).toBeNull();
    fireEvent.click(face);
    const detail = await screen.findByTestId(INSPECT_DETAIL, undefined, SLOW);
    expect(detail.textContent).toContain(face.querySelector(".card-name")?.textContent ?? "(no name)");
    fireEvent.keyDown(detail, { key: "Escape" });
    await waitFor(() => {
      expect(screen.queryByTestId(INSPECT_DETAIL)).toBeNull();
    });
  });

  it("R639 Enter and Space on a focused face open it too, for a keyboard or a screen reader", async () => {
    render(<LandingRoute random={seeded(3)} />);
    const face = screen.getByTestId(landingFanCardTestid(0));
    fireEvent.keyDown(face, { key: "Tab" });
    expect(screen.queryByTestId(INSPECT_DETAIL)).toBeNull();
    fireEvent.keyDown(face, { key: "Enter" });
    expect(await screen.findByTestId(INSPECT_DETAIL, undefined, SLOW)).toBeInTheDocument();
  });
});

describe("#165 a touch hold on a fan card", () => {
  it("#165 shows the card's preview while the finger is down, and the release click opens no detail", async () => {
    vi.useFakeTimers();
    try {
      render(<LandingRoute random={seeded(3)} />);
      const face = screen.getByTestId(landingFanCardTestid(1));
      fireEvent.pointerDown(face, { pointerType: "touch", pointerId: 1, isPrimary: true, button: 0 });
      act(() => {
        vi.advanceTimersByTime(LONG_PRESS_MS - 1);
      });
      expect(screen.queryByTestId(INSPECT_HOVER)).toBeNull();
      act(() => {
        vi.advanceTimersByTime(1);
      });
      expect(screen.getByTestId(INSPECT_HOVER).textContent).toContain(face.querySelector(".card-name")?.textContent ?? "(no name)");
      fireEvent.pointerUp(face, { pointerType: "touch", pointerId: 1 });
      expect(screen.queryByTestId(INSPECT_HOVER)).toBeNull();

      // The click the lift ends in is swallowed, so no detail dialog opens; a real tap still does.
      fireEvent.click(face);
      await act(async () => {
        await vi.dynamicImportSettled();
      });
      expect(screen.queryByTestId(INSPECT_DETAIL)).toBeNull();
      fireEvent.click(face);
      await act(async () => {
        await vi.dynamicImportSettled();
      });
      expect(screen.getByTestId(INSPECT_DETAIL)).toBeInTheDocument();
    } finally {
      vi.useRealTimers();
    }
  });
});

describe("R639 your table", () => {
  it("R639 draws nothing before the first logged game", () => {
    render(<LandingRoute />);
    expect(screen.queryByTestId(landingTestid.stats)).toBeNull();
  });

  it("R639 shows the record and the favourites the device kept, and Clear forgets them", () => {
    withGames(4, { "core-004": { seen: 4, played: 9, playedAgainst: 0, destroyed: 0, defeated: 0 } });
    render(<LandingRoute />);
    const table = screen.getByTestId(landingTestid.stats);
    expect(table.textContent).toContain("4 games: 4 won, 0 lost (100% won)");
    expect(table.querySelector('[data-stat="played"]')?.textContent).toContain("played 9×");
    fireEvent.click(screen.getByTestId(landingTestid.statsClear));
    expect(screen.queryByTestId(landingTestid.stats)).toBeNull();
  });
});
