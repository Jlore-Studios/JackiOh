// The Dev tools panel (SPEC §14.10, R1300000): each tool is a request to its dev route, sent as
// typed, and the line under the buttons says what the server answered, a refusal in its own words.
// The quest tools (§14.9, #1304) complete a quest picked from what `GET /api/quests` lists. The bar
// tools (§14.5, R1310003) fill the card or the cosmetic token bar.

import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { QuestView } from "../net/api.ts";
import DevTools, { devToolsTestid } from "./DevTools.tsx";

const TOKEN = "token-1";

const QUESTS: QuestView[] = [
  { questId: "win-3", title: "Win 3 games", progress: 1, target: 3, jlore: 80, xp: 4000, arrivedAt: 1 },
  { questId: "traps-3", title: "Play 3 Traps", progress: 0, target: 3, jlore: 60, xp: 3000, arrivedAt: 2 },
];

/** A response with `status` and `body`. */
function response(status: number, body?: unknown): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    text: () => Promise.resolve(body === undefined ? "" : JSON.stringify(body)),
  } as unknown as Response;
}

/** Whether a request is the panel's own `GET /api/quests`, which lists the held quests. */
function listsQuests(url: string, init?: RequestInit): boolean {
  return new URL(url, "http://localhost").pathname === "/api/quests" && (init?.method ?? "GET") === "GET";
}

/** `fetch` answering `GET /api/quests` with `quests` and every other request with `status` and `body`. */
function answer(status: number, body?: unknown, quests: QuestView[] = []) {
  return vi.fn((url: string, init?: RequestInit) =>
    Promise.resolve(listsQuests(url, init) ? response(200, { quests, nextDueAt: null }) : response(status, body)),
  );
}

/** The requests `fetchMock` saw to a dev route, in order. */
function devCalls(fetchMock: ReturnType<typeof answer>): [url: string, init?: RequestInit][] {
  return fetchMock.mock.calls.filter(([url, init]) => !listsQuests(url, init));
}

/** The path and the parsed body of the `n`th request to a dev route `fetchMock` saw. */
function sent(fetchMock: ReturnType<typeof answer>, n: number): { path: string; method: string | undefined; body: unknown } {
  const [url, init] = devCalls(fetchMock)[n] ?? [];
  return {
    path: new URL(String(url), "http://localhost").pathname,
    method: init?.method,
    body: typeof init?.body === "string" ? JSON.parse(init.body) : undefined,
  };
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("R1300000 the Dev tools panel", () => {
  it("R1300000 sends each tool to its dev route and shows the answer", async () => {
    const fetchMock = answer(200, { jlore: 50 });
    vi.stubGlobal("fetch", fetchMock);
    render(<DevTools token={TOKEN} />);
    const status = screen.getByTestId(devToolsTestid.status);

    await userEvent.type(screen.getByTestId(devToolsTestid.jlore), "50");
    await userEvent.click(screen.getByTestId(devToolsTestid.giveJlore));
    await waitFor(() => {
      expect(status).toHaveTextContent("Done.");
    });
    expect(sent(fetchMock, 0)).toEqual({ path: "/api/dev/jlore", method: "POST", body: { jlore: 50 } });
    const [, init] = devCalls(fetchMock)[0] ?? [];
    expect((init?.headers as Record<string, string>).authorization).toBe(`Bearer ${TOKEN}`);

    await userEvent.selectOptions(screen.getByTestId(devToolsTestid.set), "Classic");
    await userEvent.type(screen.getByTestId(devToolsTestid.xp), "4500");
    await userEvent.click(screen.getByTestId(devToolsTestid.giveXp));
    await waitFor(() => {
      expect(devCalls(fetchMock)).toHaveLength(2);
    });
    expect(sent(fetchMock, 1)).toEqual({ path: "/api/dev/xp", method: "POST", body: { setId: "Classic", xp: 4500 } });

    await userEvent.click(screen.getByTestId(devToolsTestid.fill));
    await waitFor(() => {
      expect(devCalls(fetchMock)).toHaveLength(3);
    });
    expect(sent(fetchMock, 2).path).toBe("/api/dev/fill");

    await userEvent.click(screen.getByTestId("dev-tools-clear-everything"));
    await waitFor(() => {
      expect(devCalls(fetchMock)).toHaveLength(4);
    });
    expect(sent(fetchMock, 3)).toEqual({ path: "/api/dev/clear/everything", method: "POST", body: {} });
    for (const part of ["cards", "cosmetics", "jlore", "counters", "tracks"]) {
      expect(screen.getByTestId(`dev-tools-clear-${part}`)).toBeInTheDocument();
    }
  });

  it("R1310003 the bar buttons fill the card and cosmetic token bars through the server", async () => {
    const fetchMock = answer(200, { written: 2 });
    vi.stubGlobal("fetch", fetchMock);
    render(<DevTools token={TOKEN} />);
    const status = screen.getByTestId(devToolsTestid.status);
    expect(screen.getByTestId(devToolsTestid.fillCardBar)).toHaveTextContent("Fill the card token bar");
    expect(screen.getByTestId(devToolsTestid.fillCosmeticBar)).toHaveTextContent("Fill the cosmetic token bar");

    await userEvent.click(screen.getByTestId(devToolsTestid.fillCardBar));
    await waitFor(() => {
      expect(status).toHaveTextContent("Done.");
    });
    expect(sent(fetchMock, 0)).toEqual({ path: "/api/dev/bar/card", method: "POST", body: {} });
    const [, init] = devCalls(fetchMock)[0] ?? [];
    expect((init?.headers as Record<string, string>).authorization).toBe(`Bearer ${TOKEN}`);

    await userEvent.click(screen.getByTestId(devToolsTestid.fillCosmeticBar));
    await waitFor(() => {
      expect(devCalls(fetchMock)).toHaveLength(2);
    });
    expect(sent(fetchMock, 1)).toEqual({ path: "/api/dev/bar/cosmetic", method: "POST", body: {} });
    await waitFor(() => {
      expect(status).toHaveTextContent("Done.");
    });
    cleanup();

    // A refusal reads in the server's own words, as every tool's does.
    vi.stubGlobal("fetch", answer(403, { error: { code: "forbidden", message: "Only a dev account may use the dev tools." } }));
    render(<DevTools token={TOKEN} />);
    await userEvent.click(screen.getByTestId(devToolsTestid.fillCosmeticBar));
    expect(await screen.findByText("Only a dev account may use the dev tools.")).toBe(screen.getByTestId(devToolsTestid.status));
  });

  it("R1300000 gives a quest and completes the one picked from those held", async () => {
    const fetchMock = answer(200, { completed: "traps-3", jlore: 60, xp: 3000 }, QUESTS);
    vi.stubGlobal("fetch", fetchMock);
    render(<DevTools token={TOKEN} />);
    const status = screen.getByTestId(devToolsTestid.status);

    await userEvent.click(screen.getByTestId(devToolsTestid.giveQuest));
    await waitFor(() => {
      expect(status).toHaveTextContent("Done.");
    });
    expect(sent(fetchMock, 0)).toEqual({ path: "/api/dev/quests", method: "POST", body: {} });

    const pick = screen.getByTestId(devToolsTestid.quest);
    await waitFor(() => {
      expect(pick).toBeEnabled();
    });
    expect([...pick.querySelectorAll("option")].map((option) => option.textContent)).toEqual(["Win 3 games", "Play 3 Traps"]);
    await userEvent.selectOptions(pick, "traps-3");
    await userEvent.click(screen.getByTestId(devToolsTestid.completeQuest));
    await waitFor(() => {
      expect(devCalls(fetchMock)).toHaveLength(2);
    });
    expect(sent(fetchMock, 1)).toEqual({ path: "/api/dev/quests/traps-3/complete", method: "POST", body: {} });
    await waitFor(() => {
      expect(status).toHaveTextContent("Done.");
    });
  });

  it("R1300000 has no quest to complete while none is held", async () => {
    const fetchMock = answer(200, {});
    vi.stubGlobal("fetch", fetchMock);
    render(<DevTools token={TOKEN} />);
    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalled();
    });
    expect(screen.getByTestId(devToolsTestid.completeQuest)).toBeDisabled();
    expect(screen.getByTestId(devToolsTestid.giveQuest)).toBeEnabled();
  });

  it("R1300000 says the server's refusal in its own words", async () => {
    vi.stubGlobal(
      "fetch",
      answer(403, { error: { code: "forbidden", message: "Only a dev account may use the dev tools." } }),
    );
    render(<DevTools token={TOKEN} />);
    await userEvent.click(screen.getByTestId(devToolsTestid.fill));
    expect(await screen.findByText("Only a dev account may use the dev tools.")).toBe(
      screen.getByTestId(devToolsTestid.status),
    );
  });

  it("R1300000 holds every button while a request is out", async () => {
    let finish: (response: Response) => void = () => undefined;
    vi.stubGlobal(
      "fetch",
      vi.fn((url: string, init?: RequestInit) =>
        listsQuests(url, init)
          ? Promise.resolve(response(200, { quests: [], nextDueAt: null }))
          : new Promise<Response>((resolve) => {
              finish = resolve;
            }),
      ),
    );
    render(<DevTools token={TOKEN} />);
    await userEvent.click(screen.getByTestId(devToolsTestid.fill));

    expect(screen.getByTestId(devToolsTestid.status)).toHaveTextContent("Working…");
    expect(screen.getByTestId(devToolsTestid.giveJlore)).toBeDisabled();
    expect(screen.getByTestId("dev-tools-clear-cards")).toBeDisabled();
    expect(screen.getByTestId(devToolsTestid.giveQuest)).toBeDisabled();
    finish(response(200, { cards: 0, cosmetics: 0 }));
    await waitFor(() => {
      expect(screen.getByTestId(devToolsTestid.status)).toHaveTextContent("Done.");
    });
    expect(screen.getByTestId(devToolsTestid.fill)).toBeEnabled();
  });
});
