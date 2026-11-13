// R3733000 (#3733): `/account` is a profile header over a settings list, six sections each at its own
// `?section=` URL. The list is the old screen's every field and action, regrouped: one test lists them.

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { dailyTestid } from "../daily/testids.ts";
import { holdLinkResult, LINK_RESULT_STORAGE_KEY } from "../auth/redirect.ts";
import { formatDateTime } from "../i18n/i18n.ts";
import {
  getOwnRank,
  getProfile,
  getReplays,
  type MeResponse,
  type OnlineReplayListing,
  type OwnRankResponse,
} from "../net/api.ts";
import { getDailySettings } from "../net/daily.ts";
import { navigate } from "../net/navigate.ts";
import { SESSION_STORAGE_KEY } from "../net/session.ts";
import AccountRoute, { accountTestid } from "./account.tsx";
import {
  ACCOUNT_HISTORY_ROWS,
  ACCOUNT_SECTIONS,
  PENDING_ACCOUNT_SECTIONS,
  accountSectionOf,
  accountSectionPath,
  type AccountSection,
} from "./accountSections.ts";
import { opponentName } from "./replayWords.ts";

vi.mock("../net/api.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/api.ts")>();
  return { ...actual, getProfile: vi.fn(), getOwnRank: vi.fn(), getReplays: vi.fn() };
});

vi.mock("../net/daily.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/daily.ts")>();
  return { ...actual, getDailySettings: vi.fn(), putDailySettings: vi.fn() };
});

const TOKEN = "token-1";
const MADE_AT = Date.UTC(2026, 9, 5, 12);

const ME: MeResponse = {
  profile: { id: "p1", status: "active" },
  needsInviteCode: false,
  emailVerified: true,
  currentMatchId: null,
  email: "player1@example.com",
  username: { name: "Max#3", nextChangeAt: null, promptOwed: false },
};

const PENDING_ME: MeResponse = { ...ME, profile: { id: "p1", status: "pending" }, needsInviteCode: true };

function rankBody(over: Partial<OwnRankResponse> = {}): OwnRankResponse {
  return {
    season: "v0.2",
    ladder: "constructed",
    profileId: "p1",
    username: "Max#3",
    rank: { tier: "normal", division: 3, pips: 1, pipsPerDivision: 3, floor: "rotten" },
    streak: 2,
    record: { games: 10, wins: 7, losses: 2, draws: 1 },
    badges: [{ seasonId: "v0.1", tier: "normal", division: 2 }],
    ...over,
  };
}

function game(matchId: string, over: Partial<OnlineReplayListing> = {}): OnlineReplayListing {
  return {
    matchId,
    mode: "bo1",
    endedAt: Date.UTC(2026, 10, 1, 18),
    seat: "p1",
    result: "win",
    turns: 7,
    steps: 40,
    portraits: { p1: "gary", p2: "timmy" },
    opponent: { profileId: "p2", username: "Grape#12" },
    ...over,
  };
}

/** Shows the screen at a URL, as a link or a reload reaches it. */
function openAt(section: AccountSection | null, me: MeResponse | undefined = ME): void {
  window.history.replaceState(null, "", section === null ? "/account" : accountSectionPath(section));
  render(<AccountRoute token={TOKEN} me={me} />);
}

beforeEach(() => {
  vi.mocked(getProfile).mockResolvedValue({
    id: "p1",
    email: "player1@example.com",
    status: "active",
    record: { wins: 7, losses: 2, draws: 1 },
    winRate: 0.7,
    createdAt: MADE_AT,
  });
  vi.mocked(getOwnRank).mockResolvedValue(rankBody());
  vi.mocked(getReplays).mockResolvedValue({ replays: [game("m1")], nextOffset: null });
  vi.mocked(getDailySettings).mockResolvedValue({ emailOn: true, hasEmail: true, vapidPublicKey: null, pushDevices: 0 });
});

afterEach(() => {
  cleanup();
  window.history.replaceState(null, "", "/");
  window.sessionStorage.removeItem(LINK_RESULT_STORAGE_KEY);
  window.localStorage.removeItem(SESSION_STORAGE_KEY);
  Reflect.deleteProperty(navigator, "clipboard");
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
  vi.resetAllMocks();
});

describe("R3733000 the account screen's sections", () => {
  it("R3733000 every field and action of the old screen is in its section", async () => {
    vi.stubEnv("VITE_SUPABASE_URL", "https://project.supabase.co");
    vi.stubEnv("VITE_SUPABASE_PUBLISHABLE_KEY", "sb_publishable_test");
    vi.stubGlobal("fetch", vi.fn(() => Promise.reject(new Error("no provider"))));
    window.localStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify({ accessToken: TOKEN }));

    /** The old screen's ids, each in the section the issue lists it under. */
    const where: readonly { section: AccountSection; testid: string }[] = [
      { section: "overview", testid: accountTestid.email },
      { section: "overview", testid: accountTestid.status },
      { section: "overview", testid: accountTestid.record },
      { section: "overview", testid: accountTestid.winRate },
      { section: "overview", testid: accountTestid.badges },
      { section: "overview", testid: accountTestid.randomBadges },
      { section: "overview", testid: accountTestid.leaderboard },
      { section: "history", testid: accountTestid.historyRow("m1") },
      { section: "security", testid: accountTestid.username },
      { section: "security", testid: accountTestid.secretIdShow },
      { section: "security", testid: "settings-email" },
      { section: "security", testid: "account-two-step" },
      { section: "notifications", testid: dailyTestid.settings },
      { section: "privacy", testid: accountTestid.privacyPolicy },
      { section: "privacy", testid: accountTestid.watchRules },
      { section: "delete", testid: accountTestid.deleteStart },
    ];
    for (const section of ACCOUNT_SECTIONS) {
      openAt(section);
      const panel = await screen.findByTestId(accountTestid.section);
      for (const row of where.filter((one) => one.section === section)) {
        expect(await within(panel).findByTestId(row.testid), `${section}: ${row.testid}`).toBeInTheDocument();
      }
      // Sign out is at the foot of the list, whichever section is open.
      expect(screen.getByTestId(accountTestid.signOut)).toBeInTheDocument();
      expect(screen.getByTestId(accountTestid.list).lastElementChild).toContainElement(screen.getByTestId(accountTestid.signOut));
      cleanup();
    }
  });

  it("R3733000 each section opens by its URL and an unknown one shows Overview", async () => {
    for (const section of ACCOUNT_SECTIONS) {
      openAt(section);
      const item = await screen.findByTestId(accountTestid.listItem(section));
      expect(item, section).toHaveAttribute("aria-current", "page");
      expect(item).toHaveAttribute("href", `/account?section=${section}`);
      // Notifications holds `/daily`'s own panel, a region of the same name inside the section's.
      expect(screen.getAllByRole("region", { name: item.textContent ?? "" }).length).toBeGreaterThan(0);
      expect(screen.getByTestId(accountTestid.screen).querySelector(".account-layout")).toHaveAttribute("data-open", "section");
      cleanup();
    }
    expect(accountSectionOf("?section=nope")).toBeNull();
    window.history.replaceState(null, "", "/account?section=nope");
    render(<AccountRoute token={TOKEN} me={ME} />);
    expect(await screen.findByTestId(accountTestid.listItem("overview"))).toHaveAttribute("aria-current", "page");
    expect(await screen.findByTestId(accountTestid.email)).toBeInTheDocument();
  });

  it("R3733000 the list shows alone at /account, and a list item pushes a history entry so Back returns to the list", async () => {
    openAt(null);
    const layout = (await screen.findByTestId(accountTestid.screen)).querySelector(".account-layout");
    expect(layout).toHaveAttribute("data-open", "list");

    await userEvent.click(screen.getByTestId(accountTestid.listItem("history")));
    expect(window.location.search).toBe("?section=history");
    expect(layout).toHaveAttribute("data-open", "section");
    expect(screen.getByTestId(accountTestid.listItem("history"))).toHaveAttribute("aria-current", "page");

    act(() => {
      window.history.back();
    });
    await waitFor(() => {
      expect(layout).toHaveAttribute("data-open", "list");
    });
    expect(window.location.search).toBe("");
  });

  it("R3733000 a section's own back link, for a deep link, goes to the list", async () => {
    openAt("security");
    await userEvent.click(await screen.findByTestId(accountTestid.sectionBack));
    expect(window.location.search).toBe("");
    expect(screen.getByTestId(accountTestid.screen).querySelector(".account-layout")).toHaveAttribute("data-open", "list");
  });

  it("R3733000 a waiting provider link opens Sign-in & security, and Back from there is not trapped", async () => {
    holdLinkResult("linked");
    window.history.replaceState(null, "", "/account");
    render(<AccountRoute token={TOKEN} me={ME} />);

    await waitFor(() => {
      expect(window.location.search).toBe("?section=security");
    });
    expect(await screen.findByTestId(accountTestid.listItem("security"))).toHaveAttribute("aria-current", "page");
    expect(screen.getByTestId(accountTestid.username)).toBeInTheDocument();

    await userEvent.click(screen.getByTestId(accountTestid.sectionBack));
    expect(screen.getByTestId(accountTestid.screen).querySelector(".account-layout")).toHaveAttribute("data-open", "list");
  });

  it("R3733000 Delete account is last in the list, in the danger style, and asks first", async () => {
    openAt("delete");
    const items = (await screen.findAllByRole("link")).filter((link) => link.closest(".account-list") !== null);
    expect(items.map((item) => item.getAttribute("data-testid"))).toEqual(
      ACCOUNT_SECTIONS.map((section) => accountTestid.listItem(section)),
    );
    expect(items.at(-1)).toHaveClass("account-list__danger");
    const start = screen.getByTestId(accountTestid.deleteStart);
    expect(start).toHaveClass("button-danger");
    expect(screen.queryByTestId(accountTestid.deleteDialog)).toBeNull();
    await userEvent.click(start);
    expect(screen.getByRole("alertdialog", { name: "Delete your account?" })).toBeInTheDocument();
  });

  it("R3733000 a pending account sees only its four sections", async () => {
    openAt(null, PENDING_ME);
    expect(PENDING_ACCOUNT_SECTIONS).toEqual(["overview", "security", "privacy", "delete"]);
    for (const section of ACCOUNT_SECTIONS) {
      const shown = screen.queryByTestId(accountTestid.listItem(section)) !== null;
      expect(shown, section).toBe(PENDING_ACCOUNT_SECTIONS.includes(section));
    }
    expect(screen.getByTestId(accountTestid.redeem)).toBeInTheDocument();
    expect(getReplays, "a pending account has no history to read").not.toHaveBeenCalled();
    expect(getProfile).not.toHaveBeenCalled();

    // A link to a section it does not have shows Overview.
    act(() => {
      navigate(accountSectionPath("history"));
    });
    expect(screen.getByTestId(accountTestid.redeem)).toBeInTheDocument();
  });
});

describe("R3733000 match history", () => {
  it("R3733000 match history lists recent games with Watch replay linking to /replays", async () => {
    vi.mocked(getReplays).mockResolvedValue({
      replays: [game("m1"), game("m2", { result: "loss", mode: "random" }), game("m3", { mode: null, result: "draw" })],
      nextOffset: null,
    });
    openAt("history");

    const first = await screen.findByTestId(accountTestid.historyRow("m1"));
    expect(first).toHaveTextContent("Win");
    expect(first).toHaveTextContent("vs Grape#12");
    expect(first).toHaveTextContent("Best of 1");
    expect(first).toHaveTextContent(formatDateTime(Date.UTC(2026, 10, 1, 18), { dateStyle: "medium", timeStyle: "short" }));
    expect(within(first).getByRole("link", { name: "Watch replay" })).toHaveAttribute("href", "/replays");
    expect(screen.getByTestId(accountTestid.historyRow("m2"))).toHaveTextContent("Loss");
    expect(screen.getByTestId(accountTestid.historyRow("m2"))).toHaveTextContent("All random");
    expect(screen.getByTestId(accountTestid.historyRow("m3"))).toHaveTextContent("Draw");
    expect(screen.getByTestId(accountTestid.historyAll)).toHaveAttribute("href", "/replays");
  });

  it("R3733000 a queue opponent shows as /replays shows them: a name, or a deleted account", async () => {
    const named = game("m1");
    const gone = game("m2", { opponent: { profileId: "", username: "" } });
    vi.mocked(getReplays).mockResolvedValue({ replays: [named, gone], nextOffset: null });
    openAt("history");

    const one = await screen.findByTestId(accountTestid.historyRow("m1"));
    expect(one).toHaveTextContent("vs Grape#12");
    expect(one).toHaveTextContent(`vs ${opponentName(named)}`);
    const two = screen.getByTestId(accountTestid.historyRow("m2"));
    expect(two).toHaveTextContent("vs a deleted account");
    expect(two).toHaveTextContent(`vs ${opponentName(gone)}`);
  });

  it("R3733000 lists only the newest ACCOUNT_HISTORY_ROWS games", async () => {
    const replays = Array.from({ length: ACCOUNT_HISTORY_ROWS + 3 }, (_, index) => game(`g${String(index)}`));
    vi.mocked(getReplays).mockResolvedValue({ replays, nextOffset: 99 });
    openAt("history");

    await screen.findByTestId(accountTestid.historyRow("g0"));
    expect(screen.getAllByTestId(/^account-history-g/)).toHaveLength(ACCOUNT_HISTORY_ROWS);
  });

  it("R3733000 a game that cannot be shown names why instead of linking, and an empty history says so", async () => {
    vi.mocked(getReplays).mockResolvedValue({ replays: [game("m1", { unavailable: "earlier_patch" })], nextOffset: null });
    openAt("history");
    const row = await screen.findByTestId(accountTestid.historyRow("m1"));
    expect(within(row).queryByRole("link", { name: "Watch replay" })).toBeNull();
    cleanup();

    vi.mocked(getReplays).mockResolvedValue({ replays: [], nextOffset: null });
    openAt("history");
    expect(await screen.findByTestId(accountTestid.historyEmpty)).toHaveTextContent("No online game of yours is kept yet.");
  });

  it("R3733000 a failed history read says so with Try again, and the rest of the screen stays", async () => {
    vi.mocked(getReplays).mockRejectedValueOnce(new Error("down")).mockResolvedValue({ replays: [game("m1")], nextOffset: null });
    openAt("history");

    expect(await screen.findByTestId(accountTestid.historyError)).toHaveAttribute("role", "alert");
    expect(screen.queryByTestId(accountTestid.error)).toBeNull();
    expect(await screen.findByTestId(accountTestid.rank)).toBeInTheDocument();

    fireEvent.click(screen.getByTestId(accountTestid.historyRetry));
    expect(await screen.findByTestId(accountTestid.historyRow("m1"))).toBeInTheDocument();
  });
});

describe("R3733000 the profile header", () => {
  it("R3733000 the header shows the last game's portrait, Member since and Copy", async () => {
    vi.mocked(getReplays).mockResolvedValue({
      replays: [game("m1", { seat: "p2", portraits: { p1: "gary", p2: "timmy" } }), game("m0")],
      nextOffset: null,
    });
    const writeText = vi.fn(async () => {});
    openAt("overview");

    const header = await screen.findByTestId(accountTestid.profile);
    await waitFor(() => {
      expect(within(header).getByTestId(accountTestid.portrait).querySelector("[data-portrait-art]")).toHaveAttribute(
        "data-portrait-art",
        "timmy",
      );
    });
    expect(within(header).getByTestId(accountTestid.memberSince)).toHaveTextContent(
      `Member since ${formatDateTime(MADE_AT, { month: "short", year: "numeric" })}`,
    );
    expect(within(header).getByTestId(accountTestid.memberSince)).toHaveTextContent("2026");
    expect(within(header).getByTestId(accountTestid.rank)).toHaveTextContent("Normal Grape III");
    expect(within(header).getByTestId(accountTestid.seasonRecord)).toHaveTextContent("7–2–1");

    // Stubbed after user-event's clicks, which install a clipboard of their own.
    const copy = within(header).getByTestId(accountTestid.nameCopy);
    expect(header.querySelector(".account-profile__name")).toHaveTextContent("Max#3");
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    await act(async () => {
      fireEvent.click(copy);
      await Promise.resolve();
    });
    await waitFor(() => {
      expect(copy).toHaveTextContent("Copied");
    });
    expect(writeText).toHaveBeenCalledWith("Max#3");
  });

  it("R3733000 the header wears the default portrait before any game, and no Member since from an older server", async () => {
    vi.mocked(getReplays).mockResolvedValue({ replays: [], nextOffset: null });
    vi.mocked(getProfile).mockResolvedValue({
      id: "p1",
      email: "player1@example.com",
      status: "active",
      record: { wins: 7, losses: 2, draws: 1 },
      winRate: 0.7,
    });
    openAt("overview");

    const header = await screen.findByTestId(accountTestid.profile);
    expect(within(header).getByTestId(accountTestid.portrait).querySelector("[data-portrait-art]")).toHaveAttribute(
      "data-portrait-art",
      "vanilla",
    );
    expect(within(header).queryByTestId(accountTestid.memberSince)).toBeNull();
  });
});

describe("R3733000 privacy and notifications", () => {
  it("R3733000 Privacy & data holds the note, the policy link and the watch rules", async () => {
    openAt("privacy");

    expect(await screen.findByTestId(accountTestid.privacyNote)).toHaveTextContent(/for as long as the account exists/);
    expect(screen.getByTestId(accountTestid.privacyPolicy)).toHaveAttribute("href", "/privacy");
    expect(screen.getByTestId(accountTestid.watchRules)).toHaveTextContent(/remove them as a friend or block them/);
  });

  it("R3733000 Notifications is /daily's own settings", async () => {
    openAt("notifications");

    const settings = await screen.findByTestId(dailyTestid.settings);
    expect(await within(settings).findByTestId(dailyTestid.email)).toBeChecked();
    expect(getDailySettings).toHaveBeenCalled();
  });
});
