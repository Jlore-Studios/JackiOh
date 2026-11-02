// R2035000: the landing's quests bookmark (SPEC §14.9, issue #2035): what a signed-in account's
// bookmark on the doors' box shows, when it opens and closes, and who gets none of it.

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { landingTestid } from "../auth/testids.ts";
import type { QuestView, QuestsResponse } from "../net/api.ts";
import { pressBack } from "../test/history.ts";
import { setReducedMotion } from "../test/setup.ts";
import { at, meBody, refusal, signIn, stubServer, type Reply, type StubServer } from "../test/stubServer.ts";
import LandingRoute, { QUESTS_UNROLL_MS } from "./landing.tsx";
import { questsTestid } from "./QuestsPanel.tsx";

const NOW = 1_800_000_000_000;

function quest(over: Partial<QuestView> = {}): QuestView {
  return {
    questId: "win-3",
    title: "Win 3 games",
    progress: 1,
    target: 3,
    jlore: 80,
    xp: 4000,
    arrivedAt: NOW - 1,
    ...over,
  };
}

function book(over: Partial<QuestsResponse> = {}): Reply {
  return { status: 200, body: { quests: [quest()], nextDueAt: null, ...over } };
}

/** `/api/auth/me` answers `status`, and each `/api/quests` read takes the next answer. */
function serve(answers: Reply[], status: "active" | "pending" = "active"): StubServer {
  let reads = 0;
  return stubServer((call) => {
    if (call.path === "/api/auth/me") {
      return { status: 200, body: meBody({ profile: { id: "profile-1", status } }) };
    }
    if (call.path === "/api/quests") {
      const answer = answers[Math.min(reads, answers.length - 1)];
      reads += 1;
      return answer;
    }
    return undefined;
  });
}

function landing(): HTMLElement {
  return screen.getByTestId(landingTestid.root);
}

function questReads(server: StubServer): number {
  return server.to("GET", "/api/quests").length;
}

/** A stubbed round trip can outrun the 1 s default under load. */
const SLOW = { timeout: 5_000 } as const;

beforeEach(() => {
  window.localStorage.clear();
  setReducedMotion(false);
  at("/");
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  setReducedMotion(false);
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("R2035000 the landing's quests bookmark", () => {
  it("R2035000 the bookmark hangs on the doors' box with the held count, closed", async () => {
    signIn();
    serve([book({ quests: [quest(), quest({ questId: "traps-3" })] })]);
    render(<LandingRoute />);

    expect(await screen.findByTestId(landingTestid.questsCount)).toHaveTextContent("2");
    const bookmark = screen.getByTestId(landingTestid.questsBookmark);
    expect(bookmark.tagName).toBe("BUTTON");
    expect(bookmark).toHaveAccessibleName("Quests");
    expect(bookmark).toHaveAttribute("aria-expanded", "false");
    const ctas = within(landing()).getByRole("navigation", { name: "Play" });
    expect(within(ctas).getByTestId(landingTestid.questsBookmark)).toBe(bookmark);
    const panel = screen.getByTestId(landingTestid.questsPanel);
    expect(bookmark.getAttribute("aria-controls")).toBe(panel.getAttribute("id"));
    expect(panel).toHaveAttribute("hidden");
    expect(screen.queryByTestId(questsTestid.quest)).toBeNull();

    // The three doors are unchanged.
    const doors = [...ctas.querySelectorAll("a.landing-cta")];
    expect(doors.map((door) => door.getAttribute("data-testid"))).toEqual([
      landingTestid.play,
      landingTestid.collection,
      landingTestid.shop,
    ]);
  });

  it("R2035000 a click shows each quest's title, progress, reward and the countdown, with no second read", async () => {
    signIn();
    const server = serve([
      book({
        quests: [
          quest(),
          quest({ questId: "traps-3", title: "Play 3 Traps", progress: 0, jlore: 60, xp: 3000 }),
        ],
        nextDueAt: Date.now() + 3_725_000,
      }),
    ]);
    render(<LandingRoute />);

    fireEvent.click(await screen.findByTestId(landingTestid.questsBookmark));
    const rows = await screen.findAllByTestId(questsTestid.quest);
    expect(rows).toHaveLength(2);
    expect(rows[0]).toHaveTextContent("Win 3 games");
    expect(rows[0]).toHaveTextContent("1 of 3");
    expect(rows[0]).toHaveTextContent("80 Jlore · 4,000 XP");
    expect(rows[1]).toHaveTextContent("Play 3 Traps");
    expect(rows[1]).toHaveTextContent("0 of 3");
    expect(rows[1]).toHaveTextContent("60 Jlore · 3,000 XP");
    expect(screen.getAllByRole("progressbar")).toHaveLength(2);
    expect(screen.getByTestId(questsTestid.next)).toHaveTextContent(/^Next quest in 1:02:0\d$/);
    expect(questReads(server)).toBe(1);
  });

  it("R2035000 a second click, Escape or a press outside rolls it up, and Escape gives back the focus", async () => {
    signIn();
    serve([book()]);
    render(<LandingRoute />);
    await screen.findByTestId(landingTestid.questsCount);
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });

    // The unroll's length is the one constant the sheet and the bookmark share, growing from the top.
    const box = document.querySelector(".landing-quests");
    expect(box).not.toBeNull();
    expect((box as HTMLElement).style.getPropertyValue("--quests-unroll")).toBe(`${String(QUESTS_UNROLL_MS)}ms`);
    const sheet = readFileSync(resolve(dirname(fileURLToPath(import.meta.url)), "landing.css"), "utf8");
    expect(sheet).toContain("@keyframes landing-quests-unroll");
    expect(sheet).toContain("@keyframes landing-quests-roll-up");
    expect(sheet).toContain("transform-origin: top");

    const bookmark = screen.getByTestId(landingTestid.questsBookmark);
    const panel = screen.getByTestId(landingTestid.questsPanel);
    const openPanel = (): void => {
      fireEvent.click(bookmark);
      expect(bookmark).toHaveAttribute("aria-expanded", "true");
      expect(panel).toHaveAttribute("data-state", "open");
    };
    const rollsUp = (): void => {
      expect(bookmark).toHaveAttribute("aria-expanded", "false");
      expect(panel).toHaveAttribute("data-state", "closing");
      expect(screen.getByTestId(questsTestid.quest)).toBeInTheDocument();
      act(() => {
        vi.advanceTimersByTime(QUESTS_UNROLL_MS - 1);
      });
      expect(panel).toHaveAttribute("data-state", "closing");
      act(() => {
        vi.advanceTimersByTime(1);
      });
      expect(panel).toHaveAttribute("data-state", "closed");
      expect(panel).toHaveAttribute("hidden");
      expect(screen.queryByTestId(questsTestid.quest)).toBeNull();
    };

    openPanel();
    fireEvent.click(bookmark);
    rollsUp();

    // Escape, with the focus on the Play door: the focus comes back to the bookmark.
    openPanel();
    screen.getByTestId(landingTestid.play).focus();
    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
    rollsUp();
    expect(document.activeElement).toBe(bookmark);

    openPanel();
    fireEvent.pointerDown(document.body);
    rollsUp();

    // A press on a row keeps it open.
    openPanel();
    fireEvent.pointerDown(screen.getByTestId(questsTestid.quest));
    expect(panel).toHaveAttribute("data-state", "open");
  });

  it("R2127000 Back rolls up the open Quests scroll, keeps the home screen and address, restores bookmark focus, and a second Back leaves", async () => {
    at("/play");
    window.history.pushState(null, "", "/");
    signIn();
    serve([book()]);
    render(<LandingRoute />);
    await screen.findByTestId(landingTestid.questsCount);

    const bookmark = screen.getByTestId(landingTestid.questsBookmark);
    const panel = screen.getByTestId(landingTestid.questsPanel);
    fireEvent.click(bookmark);
    expect(panel).toHaveAttribute("data-state", "open");
    screen.getByTestId(landingTestid.play).focus();

    await pressBack();
    // The scroll is rolling up (or, on a loaded machine, rolled up): shut either way, on the same screen and address.
    expect(bookmark).toHaveAttribute("aria-expanded", "false");
    expect(panel).not.toHaveAttribute("data-state", "open");
    expect(bookmark).toHaveFocus();
    expect(landing()).toBeInTheDocument();
    expect(window.location.pathname).toBe("/");

    // The layer is spent: the next Back is the page's and leaves the screen.
    await pressBack();
    expect(window.location.pathname).toBe("/play");
  });

  it("R2035000 a visitor gets no bookmark and no read", async () => {
    const server = serve([book()]);
    render(<LandingRoute />);

    expect(await screen.findByTestId(landingTestid.signIn)).toBeInTheDocument();
    expect(screen.queryByTestId(landingTestid.questsBookmark)).toBeNull();
    expect(questReads(server)).toBe(0);
  });

  it("R2035000 a loading account gets none", () => {
    signIn();
    const questCalls: string[] = [];
    const hanging = (input: unknown): Promise<Response> => {
      if (String(input).includes("/api/quests")) questCalls.push(String(input));
      return new Promise<Response>(() => {
        // The account read never answers.
      });
    };
    vi.stubGlobal("fetch", vi.fn(hanging));
    render(<LandingRoute />);

    expect(screen.queryByTestId(landingTestid.questsBookmark)).toBeNull();
    expect(questCalls).toHaveLength(0);
  });

  it("R2035000 a failed account read gets none, though the corner offers Account", async () => {
    signIn();
    const questCalls: string[] = [];
    const failing = (input: unknown): Promise<Response> => {
      if (String(input).includes("/api/quests")) questCalls.push(String(input));
      return Promise.reject(new TypeError("Failed to fetch"));
    };
    vi.stubGlobal("fetch", vi.fn(failing));
    render(<LandingRoute />);

    expect(await screen.findByTestId(landingTestid.account)).toBeInTheDocument();
    expect(screen.queryByTestId(landingTestid.questsBookmark)).toBeNull();
    expect(questCalls).toHaveLength(0);
  });

  it("R2035000 a pending account gets none", async () => {
    signIn();
    const server = serve([book()], "pending");
    render(<LandingRoute />);

    expect(await screen.findByTestId(landingTestid.account)).toBeInTheDocument();
    expect(screen.queryByTestId(landingTestid.questsBookmark)).toBeNull();
    expect(questReads(server)).toBe(0);
  });

  it("R2035000 with the economy off, none", async () => {
    signIn();
    const server = serve([book()]);
    render(<LandingRoute economyLive={false} />);

    expect(await screen.findByTestId(landingTestid.account)).toBeInTheDocument();
    expect(screen.queryByTestId(landingTestid.questsBookmark)).toBeNull();
    expect(questReads(server)).toBe(0);
  });

  it("R2035000 the next falling due with the panel open reads again and shows the new quest", async () => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval", "Date"] });
    vi.setSystemTime(NOW);
    signIn();
    const server = serve([
      book({ quests: [quest()], nextDueAt: NOW + 2_000 }),
      book({
        quests: [
          quest(),
          quest({ questId: "traps-3", title: "Play 3 Traps", progress: 0, jlore: 60, xp: 3000 }),
        ],
        nextDueAt: NOW + 2_000 + 3_600_000,
      }),
    ]);
    render(<LandingRoute />);
    await act(async () => {
      // The mount's read lands.
    });

    fireEvent.click(screen.getByTestId(landingTestid.questsBookmark));
    expect(screen.getByTestId(questsTestid.next)).toHaveTextContent("Next quest in 0:00:02");
    act(() => {
      vi.advanceTimersByTime(2000);
    });
    await act(async () => {
      // The due time's read lands.
    });

    expect(screen.queryByText(/reopen this page/)).toBeNull();
    expect(screen.getAllByTestId(questsTestid.quest)).toHaveLength(2);
    expect(screen.getByText("Play 3 Traps")).toBeInTheDocument();
    expect(questReads(server)).toBe(2);
    expect(screen.getByTestId(landingTestid.questsCount)).toHaveTextContent("2");
    expect(screen.getByTestId(questsTestid.next)).toHaveTextContent("Next quest in 1:00:00");
  });

  it("R2035000 a failed read shows the alert and Try again in the panel, and Try again reads again", async () => {
    signIn();
    const server = serve([refusal(503, "unavailable", "the server refused"), book()]);
    render(<LandingRoute />);

    fireEvent.click(await screen.findByTestId(landingTestid.questsBookmark, undefined, SLOW));
    expect(await screen.findByTestId(questsTestid.error, undefined, SLOW)).toHaveTextContent("the server refused");
    expect(screen.queryByTestId(landingTestid.questsCount)).toBeNull();

    fireEvent.click(screen.getByTestId(questsTestid.retry));
    expect(await screen.findByTestId(landingTestid.questsCount, undefined, SLOW)).toHaveTextContent("1");
    expect(screen.getAllByTestId(questsTestid.quest)).toHaveLength(1);
    expect(questReads(server)).toBe(2);
  });

  it("R2035000 under reduced motion it opens and closes at once", async () => {
    signIn();
    serve([book()]);
    setReducedMotion(true);
    render(<LandingRoute />);
    await screen.findByTestId(landingTestid.questsCount);

    const bookmark = screen.getByTestId(landingTestid.questsBookmark);
    const panel = screen.getByTestId(landingTestid.questsPanel);
    fireEvent.click(bookmark);
    expect(panel).toHaveAttribute("data-motion", "reduced");
    expect(panel).toHaveAttribute("data-state", "open");
    fireEvent.click(bookmark);
    expect(panel).toHaveAttribute("data-state", "closed");
    expect(panel).toHaveAttribute("hidden");
  });

  it("R2035000 none held but one due says No quests yet., counts down, and shows no count", async () => {
    signIn();
    serve([book({ quests: [], nextDueAt: Date.now() + 60_000 })]);
    render(<LandingRoute />);

    const bookmark = await screen.findByTestId(landingTestid.questsBookmark);
    expect(bookmark).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByTestId(landingTestid.questsCount)).toBeNull();
    fireEvent.click(bookmark);
    expect(await screen.findByText("No quests yet.")).toBeInTheDocument();
    expect(screen.getByTestId(questsTestid.next)).toHaveTextContent(/^Next quest in /);
  });

  it("R2210000 the bookmark's quests offer Replace and draw the server's answer", async () => {
    signIn();
    const traps = quest({ questId: "traps-3", title: "Play 3 Traps", progress: 0, jlore: 60, xp: 3000 });
    const server = stubServer((call) => {
      if (call.path === "/api/auth/me") {
        return { status: 200, body: meBody({ profile: { id: "profile-1", status: "active" } }) };
      }
      if (call.method === "GET" && call.path === "/api/quests") {
        return book({ quests: [quest(), traps], replacesLeft: 1 });
      }
      if (call.method === "POST" && call.path === "/api/quests/win-3/replace") {
        return book({ quests: [traps], replacesLeft: 0 });
      }
      return undefined;
    });
    render(<LandingRoute />);

    fireEvent.click(await screen.findByTestId(landingTestid.questsBookmark));
    fireEvent.click(await screen.findByRole("button", { name: "Replace Win 3 games" }));
    fireEvent.click(screen.getByTestId(questsTestid.replaceConfirm));

    await waitFor(() => {
      expect(screen.queryByText("Win 3 games")).toBeNull();
    }, SLOW);
    expect(screen.getByText("Play 3 Traps")).toBeInTheDocument();
    expect(screen.queryByTestId(questsTestid.replace)).toBeNull();
    expect(server.to("POST", "/api/quests/win-3/replace")).toHaveLength(1);
    expect(questReads(server)).toBe(1);
  });

  it("R2035000 nothing held and nothing due shows no bookmark", async () => {
    signIn();
    const server = serve([book({ quests: [], nextDueAt: null })]);
    render(<LandingRoute />);

    await waitFor(() => {
      expect(questReads(server)).toBe(1);
    });
    await waitFor(() => {
      expect(screen.queryByTestId(landingTestid.questsBookmark)).toBeNull();
    });
  });
});
