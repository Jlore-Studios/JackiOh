// R3728000–R3728002 (#3728): the menus' shell. The route table is driven through `App` with a stubbed server
// (`test/stubServer.ts`), as FriendsButton.test.tsx drives it; jsdom ignores the cascade, so the sheet's
// widths and insets are read from shell/menu-shell.css, and e2e specs 46, 47, 51 and 38 measure the drawn shell.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { QUEUE_POPULATION_POLL_SECONDS } from "@jackioh/server-config";
import { over, upTo } from "../breakpoints.ts";
import { useMarkBoardShown } from "../game/boardShown.ts";
import { forgetQueued, readQueued, rememberQueued } from "../net/liveGame.ts";
import { waitWords } from "../net/queueReadout.ts";
import { currentPath, paths } from "../net/navigate.ts";
import { clearReadCache } from "../net/readCache.ts";
import { forgetRoomLink } from "../net/roomLink.ts";
import { gameBannerTestid } from "../routes/GameBanner.tsx";
import { at, meBody, must, signIn, stubServer, type StubServer } from "../test/stubServer.ts";
import { friendsTestid } from "../together/testids.ts";
import { useMenuShell } from "./context.ts";
import { MenuShell, menuShellOn } from "./MenuShell.tsx";
import { QUEUE_CLOCK_TICK_MS, QueueBanner, queueClock } from "./QueueBanner.tsx";
import { menuShellTestid } from "./testids.ts";

const { App, resetUsernamePromptForTests } = await import("../main.tsx");

const SLOW = { timeout: 8_000 } as const;
const TEST_MS = 15_000;

const SRC = join(dirname(fileURLToPath(import.meta.url)), "..");
const sheet = (file: string): string => readFileSync(join(SRC, file), "utf8").replace(/\/\*[\s\S]*?\*\//g, " ");

/** The body of the first rule for `selector` at or after `from` in `css`. */
function block(css: string, selector: string, from = 0): string {
  const start = css.indexOf(`${selector} {`, from);
  expect(start, selector).toBeGreaterThanOrEqual(0);
  return css.slice(css.indexOf("{", start) + 1, css.indexOf("}", start));
}

/** The paths the shell leaves out, each with its reason: a board's, a game's flow, or a dev screen. */
const OUTSIDE_SHELL: Readonly<Record<string, string>> = {
  practice: "practice's setup shares its game's path, a board (isBoardPath)",
  hotseat: "a dev screen, a hotseat board",
  duosDev: "a dev screen, the four-seat co-op board",
  fxGallery: "a dev screen, the motif gallery's fixture board",
  match: "a match's board",
  series: "a Conquest series between its games, part of a game's flow",
  watch: "a friend's game, watched on a board",
  duos: "a co-op match's board",
  dailyGame: "a daily game's board (#3727)",
};

function serve(over: Parameters<typeof meBody>[0] = {}): StubServer {
  return stubServer((call) => {
    if (call.path === "/api/auth/me") return { status: 200, body: meBody(over) };
    if (call.method === "DELETE" && call.path === "/api/queue") return { status: 200, body: { cancelled: true } };
    if (call.method === "POST" && call.path === "/api/presence") return { status: 204 };
    return undefined;
  });
}

beforeEach(() => {
  window.localStorage.clear();
  clearReadCache();
  forgetRoomLink();
  forgetQueued();
  resetUsernamePromptForTests();
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  forgetQueued();
  forgetRoomLink();
  window.localStorage.clear();
});

describe("R3728000 which screens the shell holds", () => {
  it(
    "R3728000 every path in paths renders inside the shell unless it is a board, the series screen or a dev screen",
    () => {
      signIn("tok-walk");
      serve();
      const entries = Object.entries(paths);
      expect(entries.length).toBeGreaterThan(30);
      for (const [name, value] of entries) {
        // One argument every path that takes one accepts: an id, or a Collection section.
        const path = typeof value === "function" ? value("cards") : value;
        at(path);
        render(<App />);
        const inside = screen.queryByTestId(menuShellTestid.root) !== null;
        const reason = OUTSIDE_SHELL[name];
        expect(inside, `${name} (${path})${reason === undefined ? "" : `: ${reason}`}`).toBe(reason === undefined);
        expect(menuShellOn(currentPath()), name).toBe(inside);
        cleanup();
      }
      for (const name of Object.keys(OUTSIDE_SHELL)) expect(Object.keys(paths), name).toContain(name);

      // A path nothing is at is a menu screen too: its way out is the shell's.
      at("/no-such-menu");
      render(<App />);
      expect(screen.getByTestId(menuShellTestid.root)).toBeInTheDocument();
      expect(menuShellOn("/no-such-menu")).toBe(true);
    },
    60_000,
  );

  it(
    "R3728000 the navigation is Home, Play, Collection, Shop, Friends, the screen's own current",
    () => {
      serve();
      at(paths.collection);
      render(<App />);
      const nav = screen.getByTestId(menuShellTestid.nav);
      expect(nav.tagName).toBe("NAV");
      expect(nav).toHaveAttribute("aria-label", "Main menu");
      const items = Array.from(nav.children);
      expect(items.map((item) => item.getAttribute("data-testid"))).toEqual([
        menuShellTestid.home,
        menuShellTestid.play,
        menuShellTestid.collection,
        menuShellTestid.shop,
        // Signed out, no friends panel can open: Friends is a link to Play together.
        menuShellTestid.friends,
      ]);
      expect(items.map((item) => item.textContent)).toEqual(["Home", "Play", "Collection", "Shop", "Friends"]);
      expect(items.map((item) => item.getAttribute("href"))).toEqual(["/", "/play", "/collection", "/shop", "/together"]);
      expect(items.filter((item) => item.getAttribute("aria-current") === "page")).toEqual([
        screen.getByTestId(menuShellTestid.collection),
      ]);
      // The screen's column comes first, so Tab reaches its Back before the navigation.
      const root = screen.getByTestId(menuShellTestid.root);
      expect(root.firstElementChild).toHaveClass("menu-shell__main");
      expect(root.lastElementChild).toBe(nav);

      // A plain click moves in place, and the new screen's item is the current one.
      fireEvent.click(screen.getByTestId(menuShellTestid.shop));
      expect(currentPath()).toBe(paths.shop);
      expect(screen.getByTestId(menuShellTestid.shop)).toHaveAttribute("aria-current", "page");
      expect(screen.getByTestId(menuShellTestid.collection)).not.toHaveAttribute("aria-current");
      expect(screen.getByTestId(menuShellTestid.nav)).toBe(nav);
    },
    TEST_MS,
  );

  it("R3728000 R3728002 a board on screen takes the navigation and the status row away and keeps the screen mounted", () => {
    function Mark(): null {
      useMarkBoardShown();
      return null;
    }
    function Screen({ board }: { board: boolean }): ReactElement {
      // What a bar inside the screen would draw as its status row (routes/nav.tsx).
      const shell = useMenuShell();
      return (
        <div data-testid="screen" data-status={shell?.status === null ? "none" : "row"}>
          {board ? <Mark /> : null}
        </div>
      );
    }
    const view = render(
      <MenuShell path={paths.replays}>
        <Screen board={false} />
      </MenuShell>,
    );
    const shown = screen.getByTestId("screen");
    expect(screen.getByTestId(menuShellTestid.nav)).toBeInTheDocument();
    expect(screen.getByTestId(menuShellTestid.root)).not.toHaveAttribute("data-board");
    expect(shown).toHaveAttribute("data-status", "row");

    view.rerender(
      <MenuShell path={paths.replays}>
        <Screen board />
      </MenuShell>,
    );
    expect(screen.queryByTestId(menuShellTestid.nav)).toBeNull();
    expect(screen.getByTestId(menuShellTestid.root)).toHaveAttribute("data-board");
    expect(screen.getByTestId("screen")).toBe(shown);
    // The board's own rail (the replay player's bar) gets no status row.
    expect(shown).toHaveAttribute("data-status", "none");

    view.rerender(
      <MenuShell path={paths.replays}>
        <Screen board={false} />
      </MenuShell>,
    );
    expect(screen.getByTestId(menuShellTestid.nav)).toBeInTheDocument();
    expect(screen.getByTestId("screen")).toBe(shown);
  });
});

describe("R3728000 the shell's widths and insets", () => {
  const css = sheet("shell/menu-shell.css");

  it("R3728000 the three layouts are the named breakpoints: a rail over 600 px, icons alone to 1024, a tab bar at 600 and under", () => {
    const queries = [...css.matchAll(/@media\s*([^{]+)\{/g)].map((match) => (match[1] ?? "").trim());
    expect(queries).toEqual([over("phone"), `${over("phone")} and ${upTo("tablet")}`, over("tablet"), upTo("phone")]);
    const rail = css.indexOf(`@media ${over("phone")} {`);
    expect(block(css, ".menu-shell:not([data-board])", rail)).toContain("grid-template-columns: var(--shell-rail-w) minmax(0, 1fr);");
    const icons = css.indexOf(`@media ${over("phone")} and ${upTo("tablet")} {`);
    expect(block(css, ".menu-shell:not([data-board])", icons)).toContain("grid-template-columns: var(--shell-rail-icons-w) minmax(0, 1fr);");
    expect(block(css, ".menu-shell .menu-shell__label", icons)).toContain("clip-path: inset(50%);");
    const phone = css.indexOf(`@media ${upTo("phone")} {`);
    expect(block(css, ".menu-shell__nav", phone)).toContain("position: fixed;");
    // Every item is a finger's target.
    expect(block(css, ".menu-shell .menu-shell__item")).toContain("min-height: var(--hit-min);");
    expect(block(css, ".menu-shell .menu-shell__item")).toContain("min-width: var(--hit-min);");
  });

  it("R3728000 the rail, the tab bar and the main column keep inside the safe-area insets", () => {
    expect(block(css, ".menu-shell:not([data-board])")).toContain("--shell-top: env(safe-area-inset-top, 0px);");
    expect(block(css, ".menu-shell:not([data-board]) > .menu-shell__main")).toContain("padding-top: var(--shell-top);");
    // The height a one-screen menu fills is the window less the top inset and the tab bar.
    expect(block(css, ".menu-shell")).toContain("--shell-screen-h: calc(100dvh - var(--shell-top) - var(--shell-bottom));");

    const rail = css.indexOf(`@media ${over("phone")} {`);
    const nav = block(css, ".menu-shell__nav", rail);
    for (const side of ["top", "bottom", "left"]) expect(nav, side).toContain(`env(safe-area-inset-${side})`);
    expect(block(css, ".menu-shell:not([data-board]) > .menu-shell__main", rail)).toContain(
      "padding-right: env(safe-area-inset-right, 0px);",
    );

    const phone = css.indexOf(`@media ${upTo("phone")} {`);
    expect(block(css, ".menu-shell:not([data-board])", phone)).toContain(
      "--shell-bottom: calc(var(--shell-tabbar-h) + env(safe-area-inset-bottom, 0px));",
    );
    const bar = block(css, ".menu-shell__nav", phone);
    for (const side of ["right", "bottom", "left"]) expect(bar, side).toContain(`env(safe-area-inset-${side}, 0px)`);
    expect(bar).toContain("z-index: var(--z-banner);");
    const main = block(css, ".menu-shell:not([data-board]) > .menu-shell__main", phone);
    expect(main).toContain("padding-bottom: var(--shell-bottom);");
    for (const side of ["right", "left"]) expect(main, side).toContain(`padding-${side}: env(safe-area-inset-${side}, 0px);`);

    // R3728001: the drawer, at every width, inside the insets it touches.
    const dialog = sheet("game/dialog.css");
    const drawer = block(dialog, ".dialog-scrim[data-drawer] > .dialog");
    for (const side of ["top", "right", "bottom"]) expect(drawer, side).toContain(`env(safe-area-inset-${side})`);
  });

  it("R3728000 R3734001 a toast fixed at the column's foot sits above a phone's tab bar and centers on the column beside the rail", () => {
    // The room the navigation takes beside the column: none outside the shell or under a board, else the rail's width.
    expect(block(css, ".menu-shell")).toContain("--shell-start: 0px;");
    expect(block(css, ".menu-shell:not([data-board])", css.indexOf(`@media ${over("phone")} {`))).toContain("--shell-start: var(--shell-rail-w);");
    expect(block(css, ".menu-shell:not([data-board])", css.indexOf(`@media ${over("phone")} and ${upTo("tablet")} {`))).toContain(
      "--shell-start: var(--shell-rail-icons-w);",
    );
    // Above the bar (`--shell-bottom`, 0px off a phone), and still above the bottom inset when there is no bar.
    const aboveTheBar = "bottom: max(16px, env(safe-area-inset-bottom), calc(var(--shell-bottom, 0px) + 16px));";

    // The deck builder's status toast is under the bar's layer, so the bar hid it outright.
    const deckbuilder = sheet("game/deckbuilder/deckbuilder.css");
    const status = block(deckbuilder, ".db-deck-status");
    expect(Number(/z-index:\s*(\d+);/.exec(status)?.[1])).toBeLessThan(Number(/--z-banner:\s*(\d+);/.exec(sheet("surfaces.css"))?.[1]));
    expect(status).toContain(aboveTheBar);
    expect(status).toContain("translate: calc(var(--shell-start, 0px) / 2 - 50%) 0;");
    // Each width it takes, in the editor and the workshop, leaves the rail out, so it never lies over the rail or the sidebar.
    const widths = [deckbuilder, sheet("game/deckbuilder/workshop.css")].flatMap((code) =>
      [...code.matchAll(/\.db-deck-status\s*\{([^}]*)\}/g)].flatMap((rule) => [...(rule[1] ?? "").matchAll(/max-width:\s*([^;]+);/g)]),
    );
    // The editor's, the workshop's over 1024 px and its own from 601 to 1024 px (R3734001, the rail 148 px there).
    expect(widths).toHaveLength(5);
    for (const width of widths) expect(width[1]).toMatch(/^calc\(100vw - var\(--shell-start, 0px\) - \d+px\)$/);

    // The undo toast (R2119000, on the workshop and Play together) is over the bar's layer, so it took the middle tabs' taps.
    expect(block(sheet("undo/undo.css"), ".undo-toast-region")).toContain(aboveTheBar);
  });
});

describe("R3728002 the status row", () => {
  it(
    "R3728002 the live game shows on the landing and on /privacy's bar",
    async () => {
      signIn("tok-live");
      serve({ currentMatchId: "m-7" });
      at(paths.landing);
      render(<App />);
      const onHome = await screen.findByTestId(gameBannerTestid.live, undefined, SLOW);
      expect(onHome.closest(".landing-banners")).not.toBeNull();
      expect(within(onHome).getByTestId(gameBannerTestid.rejoin)).toHaveAttribute("href", paths.match("m-7"));
      cleanup();

      at(paths.privacy);
      render(<App />);
      const onBar = await screen.findByTestId(gameBannerTestid.live, undefined, SLOW);
      const row = must(onBar.closest<HTMLElement>(".screen-nav__status"), "the status row");
      // The bar's last slot, under the rest (routes/top-bar.css's slot 5).
      expect(row.parentElement).toHaveClass("screen-nav");
      expect(row.parentElement?.lastElementChild).toBe(row);
      expect(screen.getAllByTestId(gameBannerTestid.live)).toHaveLength(1);
    },
    TEST_MS,
  );

  it(
    "R3728002 R3729001 the queue search shows on every menu screen, /play included, and Cancel dequeues and forgets it",
    async () => {
      signIn("tok-queue");
      const server = serve();
      rememberQueued("bo1");

      at(paths.privacy);
      render(<App />);
      const banner = await screen.findByTestId(menuShellTestid.queue, undefined, SLOW);
      expect(banner).toHaveAttribute("data-mode", "bo1");
      expect(banner).toHaveTextContent("You're in the queue");
      expect(banner).toHaveTextContent("Looking for a Best of 1 opponent…");
      expect(banner.closest(".screen-nav__status")).not.toBeNull();
      expect(within(banner).getByTestId(menuShellTestid.queueLeave)).toHaveTextContent("Cancel");

      // R3729001: /play shows the same search in the same row, one banner and no second one.
      fireEvent.click(screen.getByTestId(menuShellTestid.play));
      expect(currentPath()).toBe(paths.play);
      const onPlay = await screen.findByTestId(menuShellTestid.queue, undefined, SLOW);
      expect(onPlay.closest(".screen-nav__status")).not.toBeNull();
      expect(screen.getAllByTestId(menuShellTestid.queue)).toHaveLength(1);
      fireEvent.click(screen.getByTestId(menuShellTestid.home));
      const onHome = await screen.findByTestId(menuShellTestid.queue, undefined, SLOW);
      expect(onHome.closest(".landing-banners")).not.toBeNull();

      fireEvent.click(within(onHome).getByTestId(menuShellTestid.queueLeave));
      await waitFor(() => {
        expect(screen.queryByTestId(menuShellTestid.queue)).toBeNull();
      }, SLOW);
      expect(server.to("DELETE", "/api/queue")).toHaveLength(1);
      expect(readQueued()).toBeNull();
    },
    TEST_MS,
  );

  it("R3728002 the challenge toasts are on --z-toast inside the insets, and clear the rail and the tab bar", () => {
    const toasts = block(sheet("together/together.css"), ".challenge-toasts");
    expect(toasts).toContain("z-index: var(--z-toast);");
    expect(toasts).toContain("left: max(var(--space-7), env(safe-area-inset-left));");
    expect(toasts).toContain("bottom: max(var(--space-7), env(safe-area-inset-bottom));");

    const css = sheet("shell/menu-shell.css");
    const shelled = "body:has(.menu-shell:not([data-board])) .challenge-toasts";
    expect(block(css, shelled, css.indexOf(`@media ${over("phone")} {`))).toContain("left: calc(var(--shell-rail-w) +");
    expect(block(css, shelled, css.indexOf(`@media ${over("phone")} and ${upTo("tablet")} {`))).toContain(
      "left: calc(var(--shell-rail-icons-w) +",
    );
    // Above the bar and the bottom inset under it, a gap clear of both.
    expect(block(css, shelled, css.indexOf(`@media ${upTo("phone")} {`))).toContain(
      "bottom: calc(var(--shell-tabbar-h) + env(safe-area-inset-bottom, 0px) + var(--space-7));",
    );
  });
});

/** What a screen's bar draws: the shell's status row, as `routes/nav.tsx` does. */
function StatusRow(): ReactElement {
  return <div data-testid="status-row">{useMenuShell()?.status}</div>;
}

describe("R3729001 the queue banner", () => {
  const POPULATION = { gamesLastHour: 12, expectedWaitSeconds: { bo1: 45, bo3: null, random: 130 } } as const;

  function serveQueue(population: unknown = POPULATION): StubServer {
    return stubServer((call) => {
      if (call.path === "/api/auth/me") return { status: 200, body: meBody({}) };
      if (call.method === "DELETE" && call.path === "/api/queue") return { status: 200, body: { cancelled: true } };
      if (call.method === "GET" && call.path === "/api/queue/population") return { status: 200, body: population };
      return undefined;
    });
  }

  afterEach(() => {
    vi.useRealTimers();
  });

  it.each([paths.play, paths.collection, paths.landing, paths.replays])(
    "R3729001 %s shows the search with its mode, the time waited and the expected wait, and Cancel sends DELETE /api/queue",
    async (path) => {
      signIn("tok-banner");
      const server = serveQueue();
      rememberQueued("random");
      render(
        <MenuShell path={path}>
          <StatusRow />
        </MenuShell>,
      );
      const banner = await screen.findByTestId(menuShellTestid.queue, undefined, SLOW);
      expect(banner).toHaveAttribute("data-mode", "random");
      expect(banner).toHaveTextContent("Looking for a All random opponent…");
      expect(within(banner).getByTestId(menuShellTestid.queueClock).textContent).toMatch(/^\d+:\d{2}$/);
      // 130 seconds are "under 3 minutes": the tile's own words (R640002), for this tab's mode alone.
      await waitFor(() => {
        expect(within(banner).getByTestId(menuShellTestid.queueWait)).toHaveTextContent(waitWords(130));
      }, SLOW);
      expect(within(banner).getByTestId(menuShellTestid.queueWait)).toHaveTextContent("Usually under 3 min");

      fireEvent.click(within(banner).getByTestId(menuShellTestid.queueLeave));
      await waitFor(() => {
        expect(screen.queryByTestId(menuShellTestid.queue)).toBeNull();
      }, SLOW);
      expect(server.to("DELETE", "/api/queue")).toHaveLength(1);
      expect(readQueued()).toBeNull();
    },
    TEST_MS,
  );

  it("R3729001 a mode the server has counted no wait for shows no wait, and a refused Cancel keeps the search on screen", async () => {
    signIn("tok-refused");
    const server = stubServer((call) => {
      if (call.path === "/api/auth/me") return { status: 200, body: meBody({}) };
      if (call.method === "GET" && call.path === "/api/queue/population") return { status: 200, body: POPULATION };
      if (call.method === "DELETE" && call.path === "/api/queue") return { status: 409, body: { code: "busy", message: "Not now." } };
      return undefined;
    });
    rememberQueued("bo3");
    render(
      <MenuShell path={paths.shop}>
        <StatusRow />
      </MenuShell>,
    );
    const banner = await screen.findByTestId(menuShellTestid.queue, undefined, SLOW);
    await waitFor(() => {
      expect(server.to("GET", "/api/queue/population")).toHaveLength(1);
    }, SLOW);
    expect(within(banner).queryByTestId(menuShellTestid.queueWait)).toBeNull();
    expect(within(banner).getByTestId(menuShellTestid.queueClock)).toBeInTheDocument();

    fireEvent.click(within(banner).getByTestId(menuShellTestid.queueLeave));
    await waitFor(() => {
      expect(server.to("DELETE", "/api/queue")).toHaveLength(1);
    }, SLOW);
    expect(screen.getByTestId(menuShellTestid.queue)).toBeInTheDocument();
    expect(readQueued()).toBe("bo3");
  });

  it("R3729001 the clock reads the time waited in m:ss, floored, never below zero", () => {
    expect(queueClock(0)).toBe("0:00");
    expect(queueClock(999)).toBe("0:00");
    expect(queueClock(1_000)).toBe("0:01");
    expect(queueClock(59_999)).toBe("0:59");
    expect(queueClock(65_000)).toBe("1:05");
    expect(queueClock(3_725_000)).toBe("62:05");
    expect(queueClock(-4_000)).toBe("0:00");
  });

  /** Let what a timer set off settle, inside `act`. */
  async function advance(ms: number): Promise<void> {
    await act(async () => {
      vi.advanceTimersByTime(ms);
      for (let i = 0; i < 10; i += 1) await Promise.resolve();
    });
  }

  it("R3729001 the clock counts from when the tab queued and advances every tick", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(1_800_000_000_000);
    serveQueue();
    // Queued 65 seconds ago, as a tab that went to another screen and came back would find it.
    rememberQueued("bo1", Date.now() - 65_000);
    render(<QueueBanner mode="bo1" token="tok-clock" />);
    await advance(0);
    const clock = screen.getByTestId(menuShellTestid.queueClock);
    expect(clock).toHaveTextContent("1:05");
    await advance(QUEUE_CLOCK_TICK_MS);
    expect(clock).toHaveTextContent("1:06");
    await advance(QUEUE_CLOCK_TICK_MS * 4);
    expect(clock).toHaveTextContent("1:10");
    await advance(QUEUE_CLOCK_TICK_MS * 55);
    expect(clock).toHaveTextContent("2:05");
    // The 45 seconds the server counted for Best of 1 arrived with the first read.
    expect(screen.getByTestId(menuShellTestid.queueWait)).toHaveTextContent("Usually under 45 s");
  });

  it("R3729001 with no moment kept the clock counts from when the banner first showed", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(1_800_000_000_000);
    serveQueue();
    rememberQueued("bo1");
    window.sessionStorage.removeItem("jackioh.play.queuedSince");
    render(<QueueBanner mode="bo1" token="tok-fallback" />);
    await advance(0);
    const clock = screen.getByTestId(menuShellTestid.queueClock);
    expect(clock).toHaveTextContent("0:00");
    await advance(QUEUE_CLOCK_TICK_MS * 3);
    expect(clock).toHaveTextContent("0:03");
  });

  it("R3729001 the expected wait is read again while the banner is up, at the population's own pace", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(1_800_000_000_000);
    const server = serveQueue();
    rememberQueued("bo1");
    render(<QueueBanner mode="bo1" token="tok-poll" />);
    await advance(0);
    expect(server.to("GET", "/api/queue/population")).toHaveLength(1);
    await advance(QUEUE_POPULATION_POLL_SECONDS * 1000);
    expect(server.to("GET", "/api/queue/population")).toHaveLength(2);
    await advance(QUEUE_POPULATION_POLL_SECONDS * 1000);
    expect(server.to("GET", "/api/queue/population")).toHaveLength(3);
  });
});

describe("R3728001 the friends drawer beside the screen", () => {
  it("R3728001 over 1024 px the screen's column makes room for the drawer, whose scrim is clear there", () => {
    const css = sheet("shell/menu-shell.css");
    const wide = css.indexOf(`@media ${over("tablet")} {`);
    expect(block(css, "body:has(.dialog-scrim[data-drawer]) .menu-shell:not([data-board]) > .menu-shell__main", wide)).toContain(
      "margin-right: var(--shell-drawer-w);",
    );
    const dialog = sheet("game/dialog.css");
    expect(block(dialog, ".dialog-scrim[data-drawer]", dialog.indexOf(`@media ${over("tablet")} {`))).toContain("background: transparent;");
    expect(block(dialog, ".dialog-scrim[data-drawer] > .dialog")).toContain("width: min(var(--shell-drawer-w), 100%);");
    expect(block(dialog, ".dialog-scrim[data-drawer] > .dialog", dialog.indexOf(`@media ${upTo("phone")} {`, dialog.indexOf("[data-drawer]")))).toContain(
      "width: 100%;",
    );
    // The panel is the shell's: no drawer opens from the top bar or a board (FriendsButton.test.tsx).
    expect(friendsTestid.panel).toBe("friends-panel");
  });
});
