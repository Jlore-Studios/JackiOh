// #3727: the Daily screen. Its sections in order, each game's time left (urgent under
// DAILY_URGENT_HOURS), the tab's "Your move (n)", a new game from the queue, a friend's challenge
// accepted, the notification settings, and a read again when the social read's dailyWaiting changes.

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { DailyGameSummary, DailyInvite, DailyList, DailySettings } from "../net/daily.ts";
import { clearReadCache } from "../net/readCache.ts";
import { at, must, signIn, stubServer, type Call, type Reply } from "../test/stubServer.ts";
import { reloadSocial } from "../together/socialPoll.ts";
import { dailyConstants } from "../wasm/index.ts";
import DailyRoute from "./DailyRoute.tsx";
import { dailyKindTestid, dailyModeTestid, dailyTestid } from "./testids.ts";

const SLOW = { timeout: 5_000 } as const;

const ME = { profileId: "me", username: "Me#1" };
const ANN = { profileId: "f-ann", username: "Ann#2" };
const BOB = { profileId: "f-bob", username: "Bob#3" };

function game(matchId: string, over: Partial<DailyGameSummary> = {}): DailyGameSummary {
  return {
    matchId,
    mode: "bo1",
    perMoveDays: 3,
    seat: "p1",
    opponent: ANN,
    portraits: { me: "vanilla", them: "gary" },
    turn: 4,
    life: { me: 30, them: 22 },
    hasMove: false,
    theirMove: true,
    deadline: null,
    over: false,
    result: null,
    endedAt: null,
    createdAt: Date.now() - 1000,
    ...over,
  };
}

function invite(id: string, over: Partial<DailyInvite> = {}): DailyInvite {
  const { MS_PER_DAY } = dailyConstants();
  return {
    id,
    kind: "challenge",
    from: BOB,
    to: ME,
    mode: "random",
    perMoveDays: 1,
    expiresAt: Date.now() + MS_PER_DAY,
    createdAt: Date.now() - 1000,
    ...over,
  };
}

/** A list with one entry in each of four sections, the times left counted from now. */
function list(): DailyList {
  const { MS_PER_DAY, MS_PER_HOUR } = dailyConstants();
  const now = Date.now();
  return {
    games: [
      game("theirs", { deadline: now + 2 * MS_PER_DAY + MS_PER_HOUR / 2, opponent: BOB }),
      game("late", { hasMove: true, theirMove: false, deadline: now + MS_PER_DAY + 4 * MS_PER_HOUR + MS_PER_HOUR / 2 }),
      game("soon", { hasMove: true, theirMove: false, deadline: now + MS_PER_HOUR + MS_PER_HOUR / 2, opponent: BOB }),
      game("done", { over: true, result: { outcome: "loss", reason: "disconnect" }, endedAt: now - 1000 }),
    ],
    challenges: { incoming: [invite("ch-in")], outgoing: [] },
    seeks: [],
    waiting: 2,
  };
}

const SETTINGS: DailySettings = { emailOn: true, hasEmail: true, vapidPublicKey: null, pushDevices: 0 };

const DECKS = {
  catalogVersion: "v1",
  decks: [{ id: "d-aggro", name: "Aggro", cards: [], catalogVersion: "v1", portrait: null, createdAt: 0, updatedAt: 0 }],
  trios: [],
  limits: { decks: 20, trios: 10, nameLength: 40 },
};

type World = { list: DailyList; dailyWaiting: number };

function serve(world: World, special?: (call: Call) => Reply | undefined) {
  return stubServer((call) => {
    const answer = special?.(call);
    if (answer !== undefined) return answer;
    const { method, path } = call;
    if (method === "GET" && path === "/api/daily") return { status: 200, body: world.list };
    if (method === "GET" && path === "/api/daily/settings") return { status: 200, body: SETTINGS };
    if (method === "PUT" && path === "/api/daily/settings") {
      return { status: 200, body: { ...SETTINGS, emailOn: (call.body as { emailOn: boolean }).emailOn } };
    }
    if (method === "GET" && path === "/api/social") {
      return { status: 200, body: { incoming: [], outgoing: [], unread: {}, dailyWaiting: world.dailyWaiting } };
    }
    if (method === "GET" && path === "/api/friends") {
      return { status: 200, body: { friends: [{ ...ANN, status: "offline", matchId: null, watchable: false, since: "" }], incoming: [], outgoing: [], blocked: [] } };
    }
    if (method === "GET" && path === "/api/decks") return { status: 200, body: DECKS };
    return undefined;
  });
}

beforeEach(() => {
  window.localStorage.clear();
  clearReadCache();
  signIn("tok-daily");
  at("/daily");
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.localStorage.clear();
});

function sectionNames(): (string | null)[] {
  return screen.getAllByTestId(dailyTestid.section).map((section) => section.getAttribute("data-section"));
}

function rowOf(matchId: string): HTMLElement {
  return must(
    screen.getAllByTestId(dailyTestid.gameRow).find((row) => row.getAttribute("data-match-id") === matchId),
    `the row of ${matchId}`,
  );
}

describe("#3727 the Daily screen", () => {
  it("#3727 lists Your move first, least time left first, then Their move, the challenges and the finished, with each move's time left", async () => {
    serve({ list: list(), dailyWaiting: 2 });
    render(<DailyRoute />);
    await screen.findAllByTestId(dailyTestid.gameRow, undefined, SLOW);

    expect(sectionNames()).toEqual(["yourMove", "theirMove", "incoming", "finished"]);
    const yourMove = must(screen.getAllByTestId(dailyTestid.section)[0], "Your move");
    expect(within(yourMove).getByRole("heading")).toHaveTextContent("Your move");
    expect(within(yourMove).getAllByTestId(dailyTestid.gameRow).map((row) => row.getAttribute("data-match-id"))).toEqual(["soon", "late"]);

    const late = within(rowOf("late")).getByTestId(dailyTestid.timeLeft);
    expect(late).toHaveTextContent("Your move · 1d 4h left");
    expect(late).toHaveAttribute("data-urgent", "false");
    const soon = within(rowOf("soon")).getByTestId(dailyTestid.timeLeft);
    expect(soon).toHaveTextContent("Your move · 1h left");
    expect(soon).toHaveAttribute("data-urgent", "true");
    expect(within(rowOf("theirs")).getByTestId(dailyTestid.timeLeft)).toHaveTextContent("Their move · 2d 0h left");
    expect(within(rowOf("done")).getByTestId(dailyTestid.timeLeft)).toHaveTextContent("Lost on time");

    // The opponent, the mode, the turn and both lives; the row opens the board.
    const row = rowOf("late");
    expect(row).toHaveTextContent("Ann#2");
    expect(row).toHaveTextContent("Best of 1 · 3 days per move · Turn 4");
    expect(row).toHaveTextContent("You 30 – 22 Opponent");
    expect(within(row).getByRole("link")).toHaveAttribute("href", "/daily/late");

    const incoming = must(
      screen.getAllByTestId(dailyTestid.inviteRow).find((entry) => entry.getAttribute("data-invite-id") === "ch-in"),
      "the challenge",
    );
    expect(incoming).toHaveAttribute("data-direction", "incoming");
    expect(incoming).toHaveTextContent("Bob#3 challenged you");
    expect(incoming).toHaveTextContent("All random · 1 day per move");
  });

  it("#3727 the tab says how many games wait on the player", async () => {
    serve({ list: list(), dailyWaiting: 2 });
    render(<DailyRoute />);
    await waitFor(() => {
      expect(document.title).toBe("Your move (2) · JackiOh");
    }, SLOW);
  });

  it("#3727 nothing at all shows the empty state, and the tab its name", async () => {
    serve({ list: { games: [], challenges: { incoming: [], outgoing: [] }, seeks: [], waiting: 0 }, dailyWaiting: 0 });
    render(<DailyRoute />);
    expect(await screen.findByTestId(dailyTestid.empty, undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryAllByTestId(dailyTestid.section)).toHaveLength(0);
    expect(document.title).toBe("Daily games · JackiOh");
  });

  it("#3727 the list is read again when the social read's dailyWaiting changes", async () => {
    const world: World = { list: list(), dailyWaiting: 2 };
    const server = serve(world);
    render(<DailyRoute />);
    await screen.findAllByTestId(dailyTestid.gameRow, undefined, SLOW);
    await waitFor(() => {
      expect(server.to("GET", "/api/social").length).toBeGreaterThan(0);
    }, SLOW);
    const reads = server.to("GET", "/api/daily").length;

    world.dailyWaiting = 3;
    act(() => {
      reloadSocial();
    });
    await waitFor(() => {
      expect(server.to("GET", "/api/daily").length).toBe(reads + 1);
    }, SLOW);
  });

  it("#3727 Find an opponent sends a seek with the mode and the time per move, and a seek that waits reads the list again", async () => {
    const server = serve({ list: list(), dailyWaiting: 2 }, (call) =>
      call.method === "POST" && call.path === "/api/daily/seeks" ? { status: 201, body: { seekId: "sk-1" } } : undefined,
    );
    render(<DailyRoute />);
    await screen.findAllByTestId(dailyTestid.gameRow, undefined, SLOW);
    const reads = server.to("GET", "/api/daily").length;

    fireEvent.click(screen.getByTestId(dailyTestid.newGame));
    const dialog = await screen.findByTestId(dailyTestid.dialog, undefined, SLOW);
    expect(dialog).toHaveAttribute("role", "alertdialog");
    fireEvent.click(within(dialog).getByTestId(dailyKindTestid("queue")));
    fireEvent.click(within(dialog).getByTestId(dailyModeTestid("random")));
    const { DAILY_TIME_PER_MOVE_DAYS } = dailyConstants();
    const days = must(DAILY_TIME_PER_MOVE_DAYS.at(-1), "the longest time per move");
    fireEvent.click(within(dialog).getByTestId(`daily-per-move-${String(days)}`));
    fireEvent.click(within(dialog).getByTestId(dailyTestid.send));

    await waitFor(() => {
      expect(screen.queryByTestId(dailyTestid.dialog)).toBeNull();
    }, SLOW);
    expect(server.to("POST", "/api/daily/seeks")[0]?.body).toEqual({ mode: "random", perMoveDays: days });
    await waitFor(() => {
      expect(server.to("GET", "/api/daily").length).toBe(reads + 1);
    }, SLOW);
  });

  it("#3727 a seek paired at once opens its board", async () => {
    serve({ list: list(), dailyWaiting: 2 }, (call) =>
      call.method === "POST" && call.path === "/api/daily/seeks" ? { status: 201, body: { matchId: "m-new" } } : undefined,
    );
    render(<DailyRoute />);
    await screen.findAllByTestId(dailyTestid.gameRow, undefined, SLOW);
    fireEvent.click(screen.getByTestId(dailyTestid.newGame));
    const dialog = await screen.findByTestId(dailyTestid.dialog, undefined, SLOW);
    fireEvent.click(within(dialog).getByTestId(dailyKindTestid("queue")));
    fireEvent.click(within(dialog).getByTestId(dailyModeTestid("random")));
    fireEvent.click(within(dialog).getByTestId(dailyTestid.send));
    await waitFor(() => {
      expect(window.location.pathname).toBe("/daily/m-new");
    }, SLOW);
  });

  it("#3727 a friend's challenge goes to any friend, offline included, with the deck picked", async () => {
    const server = serve({ list: list(), dailyWaiting: 2 }, (call) =>
      call.method === "POST" && call.path === "/api/daily/challenges" ? { status: 201, body: { id: "ch-new" } } : undefined,
    );
    render(<DailyRoute />);
    await screen.findAllByTestId(dailyTestid.gameRow, undefined, SLOW);
    fireEvent.click(screen.getByTestId(dailyTestid.newGame));
    const dialog = await screen.findByTestId(dailyTestid.dialog, undefined, SLOW);
    fireEvent.click(within(dialog).getByTestId(dailyModeTestid("bo1")));
    expect(await within(dialog).findByTestId(dailyTestid.friend, undefined, SLOW)).toHaveValue("f-ann");
    expect(await within(dialog).findByTestId(dailyTestid.deck, undefined, SLOW)).toHaveValue("d-aggro");
    await waitFor(() => {
      expect(within(dialog).getByTestId(dailyTestid.send)).toBeEnabled();
    }, SLOW);
    fireEvent.click(within(dialog).getByTestId(dailyTestid.send));
    await waitFor(() => {
      expect(server.to("POST", "/api/daily/challenges")).toHaveLength(1);
    }, SLOW);
    const { DAILY_TIME_PER_MOVE_DAYS } = dailyConstants();
    expect(server.to("POST", "/api/daily/challenges")[0]?.body).toEqual({
      friendId: "f-ann",
      mode: "bo1",
      deckId: "d-aggro",
      perMoveDays: DAILY_TIME_PER_MOVE_DAYS[0],
    });
  });

  it("#3727 Accept takes a challenge in its own mode and opens the board; Decline sends the decline", async () => {
    const server = serve({ list: list(), dailyWaiting: 2 }, (call) => {
      if (call.method === "POST" && call.path === "/api/daily/challenges/ch-in/accept") return { status: 201, body: { matchId: "m-acc" } };
      if (call.method === "POST" && call.path === "/api/daily/challenges/ch-in/decline") return { status: 204 };
      return undefined;
    });
    render(<DailyRoute />);
    fireEvent.click(await screen.findByTestId(dailyTestid.decline, undefined, SLOW));
    await waitFor(() => {
      expect(server.to("POST", "/api/daily/challenges/ch-in/decline")).toHaveLength(1);
    }, SLOW);

    await waitFor(() => {
      expect(screen.getByTestId(dailyTestid.accept)).toBeEnabled();
    }, SLOW);
    fireEvent.click(screen.getByTestId(dailyTestid.accept));
    const dialog = await screen.findByTestId(dailyTestid.acceptDialog, undefined, SLOW);
    expect(within(dialog).queryByTestId(dailyTestid.mode)).toBeNull();
    fireEvent.click(within(dialog).getByTestId(dailyTestid.acceptSend));
    await waitFor(() => {
      expect(window.location.pathname).toBe("/daily/m-acc");
    }, SLOW);
    expect(server.to("POST", "/api/daily/challenges/ch-in/accept")[0]?.body).toEqual({ mode: "random" });
  });

  it("#3727 email turns off and on; with no push key, push on this device is said to be unavailable", async () => {
    const server = serve({ list: list(), dailyWaiting: 2 });
    render(<DailyRoute />);
    const email = await screen.findByTestId(dailyTestid.email, undefined, SLOW);
    expect(email).toBeChecked();
    fireEvent.click(email);
    await waitFor(() => {
      expect(server.to("PUT", "/api/daily/settings")[0]?.body).toEqual({ emailOn: false });
    }, SLOW);
    await waitFor(() => {
      expect(screen.getByTestId(dailyTestid.email)).not.toBeChecked();
    }, SLOW);
    expect(screen.queryByTestId(dailyTestid.push)).toBeNull();
    expect(screen.getByTestId(dailyTestid.pushNote)).toBeInTheDocument();
  });

  it("#3727 on an iPhone's browser outside the Home Screen, a note says push needs the Home Screen", async () => {
    vi.spyOn(window.navigator, "userAgent", "get").mockReturnValue(
      "Mozilla/5.0 (iPhone; CPU iPhone OS 17_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Mobile/15E148 Safari/604.1",
    );
    serve({ list: list(), dailyWaiting: 2 });
    render(<DailyRoute />);
    expect(await screen.findByTestId(dailyTestid.homeScreenNote, undefined, SLOW)).toHaveTextContent("Add to Home Screen");
    expect(screen.queryByTestId(dailyTestid.push)).toBeNull();
  });
});
