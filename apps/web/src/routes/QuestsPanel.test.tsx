// `/collection`'s quests (§14.9, #1304; #1307): what the panel shows of `GET /api/quests`, and the countdown
// to the next quest read from the clock.

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { QUEST_MAX_HELD } from "@jackioh/server-config";
import zhCN from "../i18n/zh-CN.json";
import { ApiRequestError, getQuests, replaceQuest, type QuestView, type QuestsResponse } from "../net/api.ts";
import { expectEmptyState } from "../test/empty.ts";
import { expectSkeleton } from "../test/skeleton.ts";
import QuestsPanel, { QuestsList, countdownWords, questsTestid } from "./QuestsPanel.tsx";

vi.mock("../net/api.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/api.ts")>();
  return { ...actual, getQuests: vi.fn(), replaceQuest: vi.fn() };
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
    expectEmptyState(screen.getByText("No quests yet."));
    expect(screen.getByTestId(questsTestid.next)).toHaveTextContent(/^Next quest in /);
  });

  it("renders nothing with nothing held or due", () => {
    render(<QuestsList quests={book({ quests: [], nextDueAt: null })} />);

    expect(screen.queryByTestId(questsTestid.panel)).toBeNull();
  });

  it("R2035000 with onArrived, the next falling due asks for one read instead of the reopen sentence", () => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval", "Date"] });
    vi.setSystemTime(NOW);
    const onArrived = vi.fn();
    const view = render(<QuestsList quests={book({ nextDueAt: NOW + 1_000 })} onArrived={onArrived} />);

    expect(screen.getByTestId(questsTestid.next)).toHaveTextContent("Next quest in 0:00:01");
    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(onArrived).toHaveBeenCalledTimes(1);
    expect(screen.queryByText(/reopen this page/)).toBeNull();

    // The same due time arriving again asks nothing more: one read per due time.
    view.rerender(<QuestsList quests={book({ nextDueAt: NOW + 1_000 })} onArrived={onArrived} />);
    expect(onArrived).toHaveBeenCalledTimes(1);
  });
});

describe("QuestsList's Replace", () => {
  const traps = quest({ questId: "traps-3", title: "Play 3 Traps", progress: 0, jlore: 60, xp: 3000 });

  it("R2210000 offers Replace on each held quest only while a replace is left", () => {
    const onReplace = vi.fn();
    const view = render(<QuestsList quests={book({ quests: [quest(), traps], replacesLeft: 1 })} onReplace={onReplace} />);

    expect(screen.getAllByTestId(questsTestid.replace)).toHaveLength(2);
    expect(screen.getByRole("button", { name: "Replace Win 3 games" })).toBeInTheDocument();

    view.rerender(<QuestsList quests={book({ quests: [quest(), traps], replacesLeft: 0 })} onReplace={onReplace} />);
    expect(screen.queryByTestId(questsTestid.replace)).toBeNull();
    // A server from before #2210 sends no count, which reads as none left.
    view.rerender(<QuestsList quests={book({ quests: [quest(), traps] })} onReplace={onReplace} />);
    expect(screen.queryByTestId(questsTestid.replace)).toBeNull();
    // A caller that cannot send one is offered nothing.
    view.rerender(<QuestsList quests={book({ quests: [quest(), traps], replacesLeft: 1 })} />);
    expect(screen.queryByTestId(questsTestid.replace)).toBeNull();
  });

  it("R2210000 asks first, then shows the quests the server answers", async () => {
    vi.mocked(getQuests).mockResolvedValue(book({ quests: [quest(), traps], replacesLeft: 1 }));
    vi.mocked(replaceQuest).mockResolvedValue(book({ quests: [traps], replacesLeft: 0 }));
    render(<QuestsPanel token={TOKEN} />);
    await screen.findByText("Win 3 games");

    fireEvent.click(screen.getByRole("button", { name: "Replace Win 3 games" }));
    expect(screen.getByRole("alertdialog", { name: "Replace this quest?" })).toHaveTextContent(
      "You lose its progress and get a different quest.",
    );
    fireEvent.click(screen.getByTestId(questsTestid.replaceCancel));
    expect(screen.queryByRole("alertdialog")).toBeNull();
    expect(replaceQuest).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Replace Win 3 games" }));
    fireEvent.click(screen.getByTestId(questsTestid.replaceConfirm));
    await waitFor(() => {
      expect(screen.queryByText("Win 3 games")).toBeNull();
    });
    expect(replaceQuest).toHaveBeenCalledTimes(1);
    expect(replaceQuest).toHaveBeenCalledWith(TOKEN, "win-3");
    expect(screen.getByText("Play 3 Traps")).toBeInTheDocument();
    expect(screen.queryByTestId(questsTestid.replace)).toBeNull();
    expect(screen.queryByTestId(questsTestid.replaceError)).toBeNull();
    expect(getQuests).toHaveBeenCalledTimes(1);
  });

  it("R2210000 a refused replace says why and reads the quests again", async () => {
    vi.mocked(getQuests)
      .mockResolvedValueOnce(book({ replacesLeft: 1 }))
      .mockResolvedValueOnce(book({ replacesLeft: 0 }));
    vi.mocked(replaceQuest).mockRejectedValue(
      new Error("You've already replaced a quest today. You can replace another tomorrow."),
    );
    render(<QuestsPanel token={TOKEN} />);
    await screen.findByText("Win 3 games");

    fireEvent.click(screen.getByTestId(questsTestid.replace));
    fireEvent.click(screen.getByTestId(questsTestid.replaceConfirm));
    expect(await screen.findByTestId(questsTestid.replaceError)).toHaveTextContent(
      "You've already replaced a quest today.",
    );
    await waitFor(() => {
      expect(getQuests).toHaveBeenCalledTimes(2);
    });
    await waitFor(() => {
      expect(screen.queryByTestId(questsTestid.replace)).toBeNull();
    });
    expect(screen.getByText("Win 3 games")).toBeInTheDocument();
  });

  it("R2378000 a replace refused past the account's waiting turns shows the server's words", async () => {
    const busy = "your last purchase or opening is still going through; try again in a moment";
    vi.mocked(getQuests).mockResolvedValue(book({ replacesLeft: 1 }));
    vi.mocked(replaceQuest).mockRejectedValue(
      new ApiRequestError(429, { code: "rate_limited", message: busy, details: { retryAfterMs: 1_000 } }),
    );
    render(<QuestsPanel token={TOKEN} />);
    await screen.findByText("Win 3 games");

    fireEvent.click(screen.getByTestId(questsTestid.replace));
    fireEvent.click(screen.getByTestId(questsTestid.replaceConfirm));
    expect(await screen.findByTestId(questsTestid.replaceError)).toHaveTextContent(busy);
    await waitFor(() => {
      expect(getQuests).toHaveBeenCalledTimes(2);
    });
    expect(screen.getByText("Win 3 games")).toBeInTheDocument();
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
    expectSkeleton(screen.getByTestId(questsTestid.loading), "rows");
    expect(screen.queryByTestId(questsTestid.panel)).toBeNull();

    await act(async () => {
      land(book({ quests: [], nextDueAt: null }));
      await Promise.resolve();
    });
    expect(screen.queryByTestId(questsTestid.loading)).toBeNull();
    expect(screen.queryByText("Loading your quests…")).toBeNull();
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
