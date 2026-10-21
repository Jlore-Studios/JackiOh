// `/collection`'s quests (§14.9, #1304; #1307): what the panel shows of `GET /api/quests`, and the countdown
// to the next quest read from the clock.

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { QUEST_MAX_HELD } from "@jackioh/server-config";
import zhCN from "../i18n/zh-CN.json";
import { getQuests, type QuestView, type QuestsResponse } from "../net/api.ts";
import QuestsPanel, { QuestsList, countdownWords, questsTestid } from "./QuestsPanel.tsx";

vi.mock("../net/api.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/api.ts")>();
  return { ...actual, getQuests: vi.fn() };
});

const REPO = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");
const TOKEN = "token-1";
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

function book(over: Partial<QuestsResponse> = {}): QuestsResponse {
  return { quests: [quest()], nextDueAt: null, ...over };
}

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.resetAllMocks();
});

describe("countdownWords", () => {
  it("reads whole seconds as H:MM:SS, the hours running past a day", () => {
    expect(countdownWords(0)).toBe("0:00:00");
    expect(countdownWords(3725)).toBe("1:02:05");
    expect(countdownWords(90061)).toBe("25:01:01");
  });
});

describe("QuestsList", () => {
  it("shows each held quest's progress and reward", () => {
    render(
      <QuestsList
        quests={book({ quests: [quest(), quest({ questId: "traps-3", title: "Play 3 Traps", progress: 0, jlore: 60, xp: 3000 })] })}
      />,
    );

    const quests = screen.getAllByTestId(questsTestid.quest);
    expect(quests.map((row) => row.getAttribute("data-quest-id"))).toEqual(["win-3", "traps-3"]);
    expect(quests[0]).toHaveTextContent("Win 3 games");
    expect(quests[0]).toHaveTextContent("80 Jlore · 4,000 XP");
    expect(screen.getAllByTestId(questsTestid.progress).map((row) => row.textContent)).toEqual(["1 of 3", "0 of 3"]);
    const bars = screen.getAllByRole("progressbar");
    expect(bars[0]).toHaveAttribute("max", "3");
    expect(bars[0]).toHaveAttribute("value", "1");
  });

  it("counts down to the next quest", () => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval", "Date"] });
    vi.setSystemTime(NOW);
    render(<QuestsList quests={book({ nextDueAt: NOW + 3_725_000 })} />);

    expect(screen.getByTestId(questsTestid.next)).toHaveTextContent("Next quest in 1:02:05");
    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(screen.getByTestId(questsTestid.next)).toHaveTextContent("Next quest in 1:02:04");
    act(() => {
      vi.advanceTimersByTime(3_724_000);
    });
    expect(screen.getByTestId(questsTestid.next)).toHaveTextContent("A new quest has arrived: reopen this page to see it.");
  });

  it("says the next waits for room at the cap", () => {
    const held = Array.from({ length: QUEST_MAX_HELD }, (_, at) => quest({ questId: `quest-${String(at)}` }));
    render(<QuestsList quests={book({ quests: held, nextDueAt: null })} />);

    expect(screen.getByTestId(questsTestid.next)).toHaveTextContent(
      `You hold ${String(QUEST_MAX_HELD)} quests: finish one to make room for the next.`,
    );
  });

  it("says none is held yet while the first is on its way", () => {
    render(<QuestsList quests={book({ quests: [], nextDueAt: Date.now() + 60_000 })} />);

    expect(screen.getByTestId(questsTestid.panel)).toHaveTextContent("No quests yet.");
    expect(screen.getByTestId(questsTestid.next)).toHaveTextContent(/^Next quest in /);
  });

  it("renders nothing with nothing held or due", () => {
    render(<QuestsList quests={book({ quests: [], nextDueAt: null })} />);

    expect(screen.queryByTestId(questsTestid.panel)).toBeNull();
  });
});

describe("QuestsPanel", () => {
  it("loads the caller's quests", async () => {
    vi.mocked(getQuests).mockResolvedValue(book());
    render(<QuestsPanel token={TOKEN} />);

    expect(await screen.findByTestId(questsTestid.panel)).toHaveTextContent("Win 3 games");
    expect(getQuests).toHaveBeenCalledWith(TOKEN);
  });

  it("says why when the read fails", async () => {
    vi.mocked(getQuests).mockRejectedValue(new Error("the server refused"));
    render(<QuestsPanel token={TOKEN} />);

    expect(await screen.findByTestId(questsTestid.error)).toHaveTextContent("the server refused");
    expect(screen.getByTestId(questsTestid.panel)).toHaveTextContent("Quests");
  });

  it("#1568 says it is loading while the quests are on their way, which is not having none", async () => {
    let land: (body: QuestsResponse) => void = () => undefined;
    vi.mocked(getQuests).mockReturnValue(
      new Promise<QuestsResponse>((resolve) => {
        land = resolve;
      }),
    );
    render(<QuestsPanel token={TOKEN} />);

    expect(screen.getByTestId(questsTestid.loading)).toHaveTextContent("Loading your quests…");
    expect(screen.queryByTestId(questsTestid.panel)).toBeNull();

    await act(async () => {
      land(book({ quests: [], nextDueAt: null }));
      await Promise.resolve();
    });
    expect(screen.queryByTestId(questsTestid.loading)).toBeNull();
    expect(screen.queryByTestId(questsTestid.panel)).toBeNull();
  });

  it("#1568 a failed read offers Try again, which reads the quests again", async () => {
    vi.mocked(getQuests).mockRejectedValueOnce(new Error("the server refused")).mockResolvedValueOnce(book());
    render(<QuestsPanel token={TOKEN} />);

    expect(await screen.findByTestId(questsTestid.error)).toHaveTextContent("the server refused");
    expect(screen.getByTestId(questsTestid.retry)).toBeInTheDocument();

    fireEvent.click(screen.getByTestId(questsTestid.retry));
    expect(screen.getByTestId(questsTestid.loading)).toBeInTheDocument();
    expect(screen.queryByTestId(questsTestid.retry)).toBeNull();

    expect(await screen.findByTestId(questsTestid.panel)).toHaveTextContent("Win 3 games");
    expect(getQuests).toHaveBeenCalledTimes(2);
    expect(screen.queryByRole("alert")).toBeNull();
  });
});

describe("the quest pool's titles", () => {
  /** A title is the server's English, looked up where it is shown, so each has its words in zh-CN. */
  it("each have a zh-CN entry", () => {
    const pool = JSON.parse(readFileSync(resolve(REPO, "crates/server/quests.json"), "utf8")) as {
      quests: { title: string }[];
    };
    expect(pool.quests.length).toBeGreaterThan(0);
    expect(pool.quests.map((entry) => entry.title).filter((title) => !Object.hasOwn(zhCN, title))).toEqual([]);
  });
});
