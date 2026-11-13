// `/play`: the lobby's modes, choices and refusals (R257), and its watch. The watch: only one side's HTTP
// response carries the match (or series) id, so the other side reads its own `currentMatchId` /
// `currentSeriesId` on every mount: a reload after pairing loses `waiting`. The modes: the lobby sends intent
// (`ModeChoice`) and relays what the server said; its verdict is the shared validator's, as UX.

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { DECK_SIZE } from "@jackioh/engine/config";
import { LEAN_NEWEST, newestShippedSet, type CardDef, type CardDefs } from "@jackioh/shared";
import { validateDeck, validateTrio, type LoadoutResult } from "@jackioh/validator";

import {
  DECK_NAME_MAX_LENGTH,
  MAX_SAVED_DECKS,
  MAX_SAVED_TRIOS,
  QUEUE_ENDED_AFTER_READS,
  QUEUE_POPULATION_POLL_SECONDS,
  SERIES_POLL_SECONDS,
} from "@jackioh/server-config";
import { readQueued, rememberQueued } from "../net/liveGame.ts";
import { expireRead, keepRead } from "../net/readCache.ts";
import { clearSession, writeSession } from "../net/session.ts";
import {
  ApiRequestError,
  ApiUnreachableError,
  dequeue,
  enqueue,
  getCollection,
  getDecks,
  getMe,
  getOwnRank,
  getPopulation,
  type DecksResponse,
  type EnqueueResponse,
  type OwnRankResponse,
  type QueueMode,
  type SavedDeck,
  type SavedTrio,
} from "../net/api.ts";
import { navigate, paths } from "../net/navigate.ts";
import { createLobby, getMyLobbies } from "../lobby/api.ts";
import { lobbyTestid } from "../lobby/testids.ts";
import { settle } from "../test/settle.ts";
import { expectSkeleton } from "../test/skeleton.ts";
import { currentCatalog } from "../game/currentCatalog.ts";
import { PLAY_LEAN_NEWEST_KEY, PLAY_LEAN_SET_KEY, leanLabel } from "../game/LeanNewest.tsx";
import PlayRoute, {
  LOBBY_UNREACHABLE_AFTER_READS,
  MATCH_FOUND_STATUS,
  MODE_LABEL,
  PLAY_CHOICE_KEY,
  QUEUE_MODES,
  pairTargetOf,
  playModeTestid,
  playTestid,
  playWaitTestid,
} from "./play.tsx";
import { resetVisibility, setVisibility } from "../test/setup.ts";
import { expectOnePageTitle } from "../test/pageTitle.ts";

vi.mock("../net/api.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/api.ts")>();
  return {
    ...actual,
    getMe: vi.fn(),
    enqueue: vi.fn(),
    dequeue: vi.fn(),
    getDecks: vi.fn(),
    getCollection: vi.fn(),
    getPopulation: vi.fn(),
    getOwnRank: vi.fn(),
  };
});
vi.mock("../lobby/api.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lobby/api.ts")>();
  return { ...actual, createLobby: vi.fn(), getMyLobbies: vi.fn() };
});
vi.mock("../game/currentCatalog.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../game/currentCatalog.ts")>();
  return { ...actual, currentCatalog: vi.fn() };
});
vi.mock("../net/navigate.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/navigate.ts")>();
  return { ...actual, navigate: vi.fn() };
});

const TOKEN = "token-1";

function me(currentMatchId: string | null, currentSeriesId: string | null = null) {
  return {
    profile: { id: "p1", status: "active" as const },
    needsInviteCode: false,
    emailVerified: true,
    currentMatchId,
    currentSeriesId,
    email: "player1@example.com",
  };
}

// fixtures: a catalog, a collection that owns one of everything, four decks and two trios

function cardId(n: number): string {
  return `core-${String(n).padStart(3, "0")}`;
}

function run(from: number): string[] {
  return Array.from({ length: DECK_SIZE }, (_, i) => cardId(from + i));
}

const ALL_IDS = Array.from({ length: DECK_SIZE * 3 }, (_, i) => cardId(i + 1));

const DEFS: CardDefs = Object.fromEntries(
  ALL_IDS.map((id) => [id, { id, name: `Card ${id}`, token: false, tags: [] } as unknown as CardDef]),
);

function deck(id: string, name: string, cards: string[]): SavedDeck {
  return { id, name, cards, catalogVersion: "v1", portrait: null, createdAt: 0, updatedAt: 0 };
}

const AGGRO = deck("d-aggro", "Aggro", run(1));
const HALF = deck("d-half", "Half Built", run(1).slice(0, 5));
const CONTROL = deck("d-control", "Control", run(1 + DECK_SIZE));
const RAMP = deck("d-ramp", "Ramp", run(1 + 2 * DECK_SIZE));

function trio(id: string, name: string, deckIds: SavedTrio["deckIds"]): SavedTrio {
  return { id, name, deckIds, createdAt: 0, updatedAt: 0 };
}

const MAIN = trio("t-main", "Main trio", [AGGRO.id, CONTROL.id, RAMP.id]);
const LOOSE = trio("t-loose", "Loose trio", [AGGRO.id, HALF.id, null]);

function decksAnswer(decks: SavedDeck[], trios: SavedTrio[]): DecksResponse {
  return {
    catalogVersion: "v1",
    decks,
    trios,
    limits: { decks: MAX_SAVED_DECKS, trios: MAX_SAVED_TRIOS, nameLength: DECK_NAME_MAX_LENGTH },
  };
}

/** Everyone owns one of every card here, as the collection answer below says. */
const OWNED: Record<string, number> = Object.fromEntries(ALL_IDS.map((id) => [id, 1]));

/** The validator's sentences, computed: `messages.test.ts` fails any client file that copies them. */
function messagesOf(result: LoadoutResult): string[] {
  return result.ok ? [] : result.errors.map((error) => error.message);
}

function openTicket(mode: QueueMode): EnqueueResponse {
  return { ticketId: "tk-1", status: "open", matchId: null, seriesId: null, mode };
}

/** `GET /api/ranked`'s answer: a Normal Grape III with one pip, unless a test overrides it. */
function rankBody(over: Partial<OwnRankResponse> = {}): OwnRankResponse {
  return {
    season: "v0.2",
    ladder: "constructed",
    profileId: "5d0c7a1e-0b9f-4c55-9a3e-6f2d8b1c4e70",
    username: "Max#3",
    rank: { tier: "normal", division: 3, pips: 1, pipsPerDivision: 3, floor: "rotten" },
    streak: 2,
    record: { games: 10, wins: 7, losses: 2, draws: 1 },
    badges: [],
    ...over,
  };
}

beforeEach(() => {
  try {
    window.localStorage.clear();
  } catch {
    // nothing to clear
  }
  vi.mocked(getMe).mockResolvedValue(me(null));
  vi.mocked(getDecks).mockResolvedValue(decksAnswer([AGGRO, HALF, CONTROL, RAMP], [MAIN, LOOSE]));
  vi.mocked(currentCatalog).mockReturnValue({ version: "v1", cards: DEFS });
  vi.mocked(getCollection).mockResolvedValue({
    catalogVersion: "v1",
    entries: ALL_IDS.map((id) => ({ cardId: id, quantity: 1 })),
  });
  vi.mocked(getPopulation).mockResolvedValue({ gamesLastHour: 12, expectedWaitSeconds: { bo1: 30, bo3: null, random: 90 } });
  vi.mocked(getOwnRank).mockResolvedValue(rankBody());
  vi.mocked(getMyLobbies).mockResolvedValue({ lobbies: [] });
});

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
  try {
    window.localStorage.clear();
  } catch {
    // nothing to clear
  }
});

/** Render the lobby and wait for its decks to load. */
async function renderLobby(): Promise<void> {
  render(<PlayRoute token={TOKEN} />);
  await waitFor(() => {
    expect(vi.mocked(getDecks)).toHaveBeenCalled();
  });
  await screen.findByTestId(playTestid.deckSelect);
}

function pickMode(mode: QueueMode): void {
  fireEvent.click(screen.getByTestId(playModeTestid(mode)));
}

// the watch

describe("the lobby's match watch", () => {
  /** The reload case: `waiting` is component state, so a refresh clears it. */
  it("sends an already-paired player to the board on mount, without queueing again", async () => {
    vi.mocked(getMe).mockResolvedValue(me("match-42"));
    render(<PlayRoute token={TOKEN} />);

    // The pairing announces itself for one beat (`MATCH_FOUND_NAV_DELAY_MS`) before the
    // navigation lands it, so every navigation below waits past that beat.
    await waitFor(
      () => {
        expect(vi.mocked(navigate)).toHaveBeenCalledWith("/match/match-42");
      },
      { timeout: 5000 },
    );
  });

  it("R259 sends a player between the games of a series to the series screen on mount", async () => {
    vi.mocked(getMe).mockResolvedValue(me(null, "series-7"));
    render(<PlayRoute token={TOKEN} />);

    await waitFor(
      () => {
        expect(vi.mocked(navigate)).toHaveBeenCalledWith("/series/series-7");
      },
      { timeout: 5000 },
    );
  });

  it("leaves a player who is in no match exactly where they are", async () => {
    render(<PlayRoute token={TOKEN} />);

    await waitFor(() => {
      expect(vi.mocked(getMe)).toHaveBeenCalled();
    });
    expect(vi.mocked(navigate)).not.toHaveBeenCalled();
  });

  it("a woken page reads the wait again at once, and a hidden tab reads nothing", async () => {
    const visibility = Object.getOwnPropertyDescriptor(document, "visibilityState");
    /** Lets a read that has been answered reach its handler and clear its in-flight mark. */
    const settled = async (): Promise<void> => {
      await act(async () => {
        await new Promise<void>((resolve) => setTimeout(resolve, 0));
      });
    };
    try {
      rememberQueued("bo1");
      render(<PlayRoute token={TOKEN} />);
      await waitFor(() => {
        expect(vi.mocked(getMe)).toHaveBeenCalledTimes(1);
      });
      await settled();

      setVisibility("hidden");
      await settled();
      expect(vi.mocked(getMe)).toHaveBeenCalledTimes(1);

      setVisibility("visible");
      await waitFor(() => {
        expect(vi.mocked(getMe)).toHaveBeenCalledTimes(2);
      });
      await settled();
      expect(vi.mocked(navigate)).not.toHaveBeenCalled();

      // Paired while the tab slept: the wake finds the match and takes the player there.
      vi.mocked(getMe).mockResolvedValue(me("match-42"));
      setVisibility("visible");
      expect(await screen.findByTestId(playTestid.status)).toHaveTextContent(MATCH_FOUND_STATUS);
      await waitFor(
        () => {
          expect(vi.mocked(navigate)).toHaveBeenCalledWith("/match/match-42");
        },
        { timeout: 5000 },
      );
    } finally {
      if (visibility === undefined) Reflect.deleteProperty(document, "visibilityState");
      else Object.defineProperty(document, "visibilityState", visibility);
    }
  });

  it("R3205000 the queue wait on /play keeps reading the account while the tab is hidden", async () => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    const period = SERIES_POLL_SECONDS * 1000;
    try {
      rememberQueued("bo1");
      render(<PlayRoute token={TOKEN} />);
      await settle({ advance: 0 });
      expect(vi.mocked(getMe)).toHaveBeenCalledTimes(1);

      // Its reads keep the queue ticket alive (R897000): a hidden tab sends them as a visible one does.
      setVisibility("hidden");
      await settle({ advance: period });
      expect(vi.mocked(getMe)).toHaveBeenCalledTimes(2);
      await settle({ advance: period });
      expect(vi.mocked(getMe)).toHaveBeenCalledTimes(3);
    } finally {
      resetVisibility();
      vi.useRealTimers();
    }
  });

  /** A lobby that cannot reach the server is still a lobby, not an error screen. */
  it("ignores a failed read rather than surfacing it as a lobby error", async () => {
    vi.mocked(getMe).mockRejectedValue(new Error("network down"));
    render(<PlayRoute token={TOKEN} />);

    await waitFor(() => {
      expect(vi.mocked(getMe)).toHaveBeenCalled();
    });
    expect(vi.mocked(navigate)).not.toHaveBeenCalled();
    expect(screen.queryByTestId(playTestid.error)).toBeNull();
  });
});

describe("R1578000 the Play hub", () => {
  function precedes(first: HTMLElement, second: HTMLElement): boolean {
    return (first.compareDocumentPosition(second) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;
  }

  const WAYS: readonly (readonly [string, string])[] = [
    [playTestid.practice, paths.practice],
    // B13: the Play hub links the Dungeon run.
    [playTestid.dungeon, paths.dungeon],
    [playTestid.together, paths.together],
    // #3727: daily games, beside Play together.
    [playTestid.daily, paths.daily],
    [playTestid.replays, paths.replays],
  ];

  it("R3223000 the hub has one visible h1, the PageTitle, signed in and signed out", async () => {
    await renderLobby();
    expectOnePageTitle("Play");
    expect(screen.queryByRole("heading", { name: "JackiOh" })).toBeNull();

    cleanup();
    render(<PlayRoute token={null} />);
    expectOnePageTitle("Play");
  });

  it("R1578000 the hub puts online first with every mode reachable", async () => {
    await renderLobby();

    const online = screen.getByTestId(playTestid.online);
    const ways = screen.getByTestId(playTestid.ways);
    expect(screen.getByTestId(playTestid.screen)).toContainElement(online);
    expect(precedes(online, ways)).toBe(true);
    expect(await within(online).findByTestId(playTestid.rank)).toBeInTheDocument();
    expect(within(online).getByRole("region", { name: "How do you want to play?" })).toBeInTheDocument();
    expect(within(online).getByRole("region", { name: "Find a match" })).toBeInTheDocument();
    expect(within(online).getByTestId(playTestid.deckSelect)).toBeInTheDocument();
    expect(within(online).getByTestId(playTestid.queue)).toBeInTheDocument();
    for (const mode of QUEUE_MODES) expect(within(online).getByTestId(playModeTestid(mode))).toBeInTheDocument();
    expect(within(ways).getAllByRole("link")).toHaveLength(WAYS.length);
    for (const [testid, href] of WAYS) expect(within(ways).getByTestId(testid)).toHaveAttribute("href", href);
    expect(paths.practice).toBe("/practice");
    expect(screen.getByTestId(lobbyTestid.panel)).toBeInTheDocument();
    expect(screen.getAllByTestId(playTestid.replays)).toHaveLength(1);
  });

  it("R1578000 Find a match is the primary action", async () => {
    await renderLobby();

    expect(screen.getByTestId(playTestid.queue)).toHaveClass("button-primary", "play-cta");
    const ways = screen.getByTestId(playTestid.ways);
    expect(ways.querySelector(".button-primary")).toBeNull();
    // A way to play is a plain link, never a second primary button.
    for (const [testid] of WAYS) expect(within(ways).getByTestId(testid).tagName).toBe("A");
  });

  it("R1578000 a way to play opens its screen inside the app", async () => {
    await renderLobby();

    fireEvent.click(screen.getByTestId(playTestid.together));
    expect(vi.mocked(navigate)).toHaveBeenCalledWith(paths.together);
  });

  it("R1578000 signed out reads nothing and offers sign-in", async () => {
    render(<PlayRoute token={null} />);
    await act(async () => {
      await new Promise<void>((resolve) => setTimeout(resolve, 0));
    });

    expect(screen.getByTestId(playTestid.screen)).toBeInTheDocument();
    const online = screen.getByTestId(playTestid.online);
    expect(online).toHaveTextContent("Online play is invite-only for now");
    expect(within(online).getByTestId(playTestid.signIn)).toHaveAttribute("href", paths.login);
    expect(paths.login).toBe("/login");
    expect(screen.queryByTestId(playTestid.queue)).toBeNull();
    expect(screen.queryByTestId(lobbyTestid.panel)).toBeNull();
    const ways = screen.getByTestId(playTestid.ways);
    expect(precedes(online, ways)).toBe(true);
    for (const [testid, href] of WAYS) expect(within(ways).getByTestId(testid)).toHaveAttribute("href", href);
    // Nothing leaves the page: no decks, rank, population, lobbies, collection or me.
    for (const read of [getDecks, getMe, getOwnRank, getPopulation, getMyLobbies, getCollection]) {
      expect(vi.mocked(read)).not.toHaveBeenCalled();
    }
    fireEvent.click(screen.getByTestId(playTestid.signIn));
    expect(vi.mocked(navigate)).toHaveBeenCalledWith(paths.login);
  });
});

describe("R3730000 Play a friend", () => {
  it("R3730000 Play a friend is the private lobby's panel, in the chosen mode", async () => {
    vi.mocked(createLobby).mockResolvedValue({ code: "ABC234" });
    await renderLobby();

    const panel = screen.getByTestId(lobbyTestid.panel);
    expect(panel).toHaveAttribute("data-format", "duel");
    expect(within(panel).getByRole("heading", { name: "Play a friend" })).toBeInTheDocument();
    // The classic room's controls are gone from /play: a friend game starts in a lobby.
    expect(screen.queryByTestId("play-create-room")).toBeNull();
    expect(screen.queryByTestId("play-join-form")).toBeNull();

    pickMode("bo3");
    fireEvent.click(within(panel).getByTestId(lobbyTestid.create));
    await waitFor(() => {
      expect(vi.mocked(navigate)).toHaveBeenCalledWith(paths.lobby("ABC234"));
    });
    expect(vi.mocked(createLobby)).toHaveBeenCalledWith({ format: "duel", mode: "bo3" });
  });
});

describe("R661 the lobby's rank", () => {
  it("R661 shows the player's own rank in R612's words with the season, and links the leaderboard", async () => {
    await renderLobby();

    const rank = await screen.findByTestId(playTestid.rank);
    expect(rank).toHaveTextContent("Normal Grape III · Season v0.2");
    expect(within(rank).getByRole("img", { name: "Normal Grape III · 1/3 pips" })).toHaveAttribute("data-size", "large");
    const link = screen.getByTestId(playTestid.leaderboard);
    expect(link).toHaveAttribute("href", paths.leaderboard);
    // A panel of its own beside the queue, never inside the Find a match box (R505).
    const box = screen.getByRole("region", { name: "Find a match" });
    expect(within(box).queryByTestId(playTestid.rank)).toBeNull();
    expect(within(box).queryByTestId(playTestid.leaderboard)).toBeNull();
  });

  it("R2778000 shows the rank of the picked mode's ladder and links that ladder's board", async () => {
    vi.mocked(getOwnRank).mockImplementation(async (_token, ladder = "constructed") =>
      ladder === "random"
        ? rankBody({ ladder: "random", rank: { tier: "golden", division: 2, pips: 0, pipsPerDivision: 3, floor: "golden" } })
        : rankBody(),
    );
    await renderLobby();

    const constructed = await screen.findByTestId(playTestid.rank);
    expect(constructed).toHaveAttribute("data-ladder", "constructed");
    expect(constructed).toHaveTextContent("Constructed · Normal Grape III");
    expect(screen.getByTestId(playTestid.leaderboard)).toHaveAttribute("href", paths.leaderboard);

    pickMode("random");
    await waitFor(() => expect(screen.getByTestId(playTestid.rank)).toHaveAttribute("data-ladder", "random"));
    expect(screen.getByTestId(playTestid.rank)).toHaveTextContent("All random · Golden Grape II");
    expect(vi.mocked(getOwnRank)).toHaveBeenLastCalledWith(TOKEN, "random");
    expect(screen.getByTestId(playTestid.leaderboard)).toHaveAttribute("href", `${paths.leaderboard}?ladder=random`);

    // Best of 1 and Conquest share the Constructed ladder, so moving between them reads nothing again.
    pickMode("bo3");
    await waitFor(() => expect(screen.getByTestId(playTestid.rank)).toHaveAttribute("data-ladder", "constructed"));
    const reads = vi.mocked(getOwnRank).mock.calls.length;
    pickMode("bo1");
    expect(vi.mocked(getOwnRank).mock.calls.length).toBe(reads);
  });

  it("R661 puts R612's words on placements and a Jlorious position too, and never a rating", async () => {
    vi.mocked(getOwnRank).mockResolvedValue(
      rankBody({ rank: { tier: "raisin", placementsPlayed: 4, placementGames: 10 } }),
    );
    const first = render(<PlayRoute token={TOKEN} />);
    const placing = await screen.findByTestId(playTestid.rank);
    expect(placing).toHaveTextContent("Raisin · Season v0.2");
    expect(within(placing).getByRole("img", { name: "Raisin · 4/10 placements" })).toBeInTheDocument();
    first.unmount();

    vi.mocked(getOwnRank).mockResolvedValue(rankBody({ rank: { tier: "jlorious", position: 42 } }));
    render(<PlayRoute token={TOKEN} />);
    const rank = await screen.findByTestId(playTestid.rank);
    expect(rank).toHaveTextContent("Jlorious · Season v0.2");
    expect(rank).toHaveTextContent("#42");
    expect(within(rank).getByRole("img", { name: "Jlorious #42" })).toBeInTheDocument();
    expect(rank.textContent).not.toMatch(/rating|elo/i);
  });

  it("R661 a rank that cannot be read shows nothing there, and the link stays", async () => {
    vi.mocked(getOwnRank).mockRejectedValue(new Error("network down"));
    await renderLobby();

    expect(screen.queryByTestId(playTestid.rank)).toBeNull();
    expect(screen.getByTestId(playTestid.leaderboard)).toHaveAttribute("href", paths.leaderboard);
  });
});

// modes and choices (R257)

describe("the lobby's modes", () => {
  it("R257 the mode picker offers Best of 1, Conquest and All random, each with its own choice", async () => {
    await renderLobby();

    for (const mode of ["bo1", "bo3", "random"] as const) {
      expect(screen.getByTestId(playModeTestid(mode))).toHaveAttribute("type", "radio");
    }
    // Best of 1 is the default: a deck, listed by name, the first complete one chosen.
    expect(screen.getByTestId(playModeTestid("bo1"))).toBeChecked();
    const deckSelect = screen.getByTestId(playTestid.deckSelect) as HTMLSelectElement;
    expect(Array.from(deckSelect.options).map((option) => option.textContent)).toEqual([
      "Aggro",
      "Half Built",
      "Control",
      "Ramp",
    ]);
    expect(deckSelect.value).toBe(AGGRO.id);
    expect(screen.queryByTestId(playTestid.trioSelect)).toBeNull();

    pickMode("bo3");
    expect(screen.getByTestId(playModeTestid("bo3"))).toBeChecked();
    expect(screen.queryByTestId(playTestid.deckSelect)).toBeNull();
    const trioSelect = screen.getByTestId(playTestid.trioSelect) as HTMLSelectElement;
    expect(Array.from(trioSelect.options).map((option) => option.textContent)).toEqual(["Main trio", "Loose trio"]);

    pickMode("random");
    expect(screen.queryByTestId(playTestid.deckSelect)).toBeNull();
    expect(screen.queryByTestId(playTestid.trioSelect)).toBeNull();
    expect(screen.getByText(/No deck needed/)).toBeInTheDocument();
  });

  it("R257 Find a match sends each mode's choice: a deck, a trio, or nothing at all", async () => {
    vi.mocked(enqueue).mockImplementation((_token, choice) => Promise.resolve(openTicket(choice.mode)));
    await renderLobby();

    fireEvent.change(screen.getByTestId(playTestid.deckSelect), { target: { value: CONTROL.id } });
    fireEvent.click(screen.getByTestId(playTestid.queue));
    await waitFor(() => {
      expect(vi.mocked(enqueue)).toHaveBeenLastCalledWith(TOKEN, { mode: "bo1", deckId: CONTROL.id });
    });
    await screen.findByText(/In the Best of 1 queue/);

    // The setup locks while queued, so the next mode is picked after leaving it.
    fireEvent.click(screen.getByTestId(playTestid.leaveQueue));
    await waitFor(() => {
      expect(screen.getByTestId(playTestid.queue)).not.toBeDisabled();
    });
    pickMode("bo3");
    fireEvent.change(screen.getByTestId(playTestid.trioSelect), { target: { value: LOOSE.id } });
    fireEvent.click(screen.getByTestId(playTestid.queue));
    await waitFor(() => {
      expect(vi.mocked(enqueue)).toHaveBeenLastCalledWith(TOKEN, { mode: "bo3", trioId: LOOSE.id });
    });
    await screen.findByText(new RegExp(`In the ${MODE_LABEL.bo3} queue`));

    fireEvent.click(screen.getByTestId(playTestid.leaveQueue));
    await waitFor(() => {
      expect(screen.getByTestId(playTestid.queue)).not.toBeDisabled();
    });
    pickMode("random");
    fireEvent.click(screen.getByTestId(playTestid.queue));
    await waitFor(() => {
      expect(vi.mocked(enqueue)).toHaveBeenLastCalledWith(TOKEN, { mode: "random" });
    });
    await screen.findByText(/In the All random queue/);
  });

  it("remembers the last mode, deck and trio on this device", async () => {
    window.localStorage.setItem(PLAY_CHOICE_KEY, JSON.stringify({ mode: "bo3", deckId: RAMP.id, trioId: LOOSE.id }));
    render(<PlayRoute token={TOKEN} />);

    const trioSelect = (await screen.findByTestId(playTestid.trioSelect)) as HTMLSelectElement;
    expect(screen.getByTestId(playModeTestid("bo3"))).toBeChecked();
    expect(trioSelect.value).toBe(LOOSE.id);
    pickMode("bo1");
    expect((screen.getByTestId(playTestid.deckSelect) as HTMLSelectElement).value).toBe(RAMP.id);

    await waitFor(() => {
      expect(JSON.parse(window.localStorage.getItem(PLAY_CHOICE_KEY) ?? "{}")).toMatchObject({ mode: "bo1" });
    });
  });

  it("R253 the verdict is the shared validator's, as UX: Ready, or its messages, and the buttons stay live", async () => {
    await renderLobby();
    await waitFor(() => {
      expect(screen.getByTestId(playTestid.verdict)).toHaveAttribute("data-ready", "true");
    });
    expect(screen.getByTestId(playTestid.verdict)).toHaveTextContent("Ready");

    fireEvent.change(screen.getByTestId(playTestid.deckSelect), { target: { value: HALF.id } });
    const verdict = screen.getByTestId(playTestid.verdict);
    expect(verdict).toHaveAttribute("data-ready", "false");
    const halfIssues = messagesOf(
      validateDeck({ deck: { name: HALF.name, cards: HALF.cards }, catalog: { version: "v1", cards: DEFS }, collection: OWNED }),
    );
    expect(halfIssues.length).toBeGreaterThan(0);
    for (const message of halfIssues) expect(verdict).toHaveTextContent(message);
    // UX only: the server decides.
    expect(screen.getByTestId(playTestid.queue)).not.toBeDisabled();

    pickMode("bo3");
    fireEvent.change(screen.getByTestId(playTestid.trioSelect), { target: { value: LOOSE.id } });
    const looseIssues = messagesOf(
      validateTrio({
        decks: [AGGRO, HALF].map((saved) => ({ name: saved.name, cards: saved.cards })),
        catalog: { version: "v1", cards: DEFS },
        collection: OWNED,
      }),
    );
    expect(looseIssues.length).toBeGreaterThan(0);
    expect(screen.getByTestId(playTestid.verdict)).toHaveTextContent(looseIssues[0] ?? "");
  });

  it("#1378 judges the deck by the catalog the client ships while the queue readout and the rank cannot be read", async () => {
    vi.mocked(getPopulation).mockRejectedValue(new TypeError("Failed to fetch"));
    vi.mocked(getOwnRank).mockRejectedValue(new TypeError("Failed to fetch"));
    await renderLobby();
    await waitFor(() => {
      expect(screen.getByTestId(playTestid.verdict)).toHaveAttribute("data-ready", "true");
    });
    expect(currentCatalog).toHaveBeenCalled();
  });

  it("a player with no decks is sent to /decks, and has nothing to queue a Best of 1 with", async () => {
    vi.mocked(getDecks).mockResolvedValue(decksAnswer([], []));
    render(<PlayRoute token={TOKEN} />);

    const link = await screen.findByTestId(playTestid.decksLink);
    expect(link).toHaveAttribute("href", "/decks");
    expect(link).toHaveTextContent("Build one in Decks");
    expect(screen.getByTestId(playTestid.queue)).toBeDisabled();

    // All random needs no deck at all.
    pickMode("random");
    expect(screen.getByTestId(playTestid.queue)).not.toBeDisabled();
  });

  it("R640002 shows each mode's expected wait on its tile and the last hour's games once", async () => {
    await renderLobby();
    const tileOf = (mode: QueueMode): HTMLElement | null => screen.getByTestId(playModeTestid(mode)).closest("label");
    expect(await screen.findByTestId(playWaitTestid("bo1"))).toHaveTextContent("Usually under 30 s");
    expect(tileOf("bo1")).toContainElement(screen.getByTestId(playWaitTestid("bo1")));
    // Too few Conquest games were counted to say, so its tile says nothing.
    expect(screen.queryByTestId(playWaitTestid("bo3"))).toBeNull();
    expect(tileOf("bo3")).not.toHaveTextContent(/usually/i);
    expect(screen.getByTestId(playWaitTestid("random"))).toHaveTextContent("Usually under 2 min");
    expect(screen.getAllByTestId(playTestid.recentGames)).toHaveLength(1);
    expect(screen.getByTestId(playTestid.recentGames)).toHaveTextContent("12 games played in the last hour");
  });

  it("R640002 shows no wait and no games line when the server counted none", async () => {
    vi.mocked(getPopulation).mockResolvedValue({ gamesLastHour: 0, expectedWaitSeconds: { bo1: null, bo3: null, random: null } });
    await renderLobby();
    await waitFor(() => {
      expect(vi.mocked(getPopulation)).toHaveBeenCalled();
    });
    for (const mode of ["bo1", "bo3", "random"] as const) expect(screen.queryByTestId(playWaitTestid(mode))).toBeNull();
    expect(screen.queryByTestId(playTestid.recentGames)).toBeNull();
    expect(screen.getByRole("region", { name: "How do you want to play?" })).not.toHaveTextContent(/usually|played in the last hour/i);
  });

  it("R640002 a server from before the readout sends neither field, and the lobby shows nothing for it", async () => {
    vi.mocked(getPopulation).mockResolvedValue({});
    await renderLobby();
    await waitFor(() => {
      expect(vi.mocked(getPopulation)).toHaveBeenCalled();
    });
    for (const mode of ["bo1", "bo3", "random"] as const) expect(screen.queryByTestId(playWaitTestid(mode))).toBeNull();
    expect(screen.queryByTestId(playTestid.recentGames)).toBeNull();
  });

  it("R433 All random names no card of the deck it will deal: that deck is dealt when the game starts", async () => {
    await renderLobby();
    pickMode("random");
    const setup = screen.getByRole("region", { name: "How do you want to play?" });
    expect(setup).toHaveTextContent("No deck needed: the server deals both of you one when the game starts.");
    expect(within(setup).queryByTestId(playTestid.deckSelect)).toBeNull();
    expect(within(setup).queryByTestId(playTestid.trioSelect)).toBeNull();
    // No card, no card count and no curve: nothing of a dealt deck is known before its game.
    for (const def of Object.values(DEFS)) expect(setup).not.toHaveTextContent(def.name);
    expect(setup.querySelector(".play-pick-summary")).toBeNull();
  });

  it("R505 R640002 the queue's readout is on the mode tiles and the games line alone: the Find a match box and the queued notice repeat none of it", async () => {
    vi.mocked(enqueue).mockImplementation((_token, choice) => Promise.resolve(openTicket(choice.mode)));
    await renderLobby();
    const tileOf = (mode: QueueMode): HTMLElement | null => screen.getByTestId(playModeTestid(mode)).closest("label");
    await waitFor(() => {
      expect(tileOf("bo1")).toHaveTextContent("Usually under 30 s");
    });
    expect(tileOf("random")).toHaveTextContent("Usually under 2 min");
    const setup = screen.getByRole("region", { name: "How do you want to play?" });
    expect(within(setup).getByTestId(playTestid.recentGames)).toHaveTextContent("12 games played in the last hour");

    const READOUT = /usually|played in the last hour|\b(12|30|90)\b/i;
    const box = screen.getByRole("region", { name: "Find a match" });
    expect(box).not.toHaveTextContent(READOUT);
    expect(within(box).queryByTestId(playTestid.recentGames)).toBeNull();

    // Queued: the box says what it is doing and the notice where you are, and neither repeats it.
    fireEvent.click(screen.getByTestId(playTestid.queue));
    const status = await screen.findByTestId(playTestid.status);
    expect(status).toHaveTextContent(`In the ${MODE_LABEL.bo1} queue.`);
    expect(status).toHaveTextContent("You will be taken to the game as soon as someone is found.");
    expect(status).not.toHaveTextContent(READOUT);
    expect(within(box).getByTestId(playTestid.searching)).toHaveTextContent(`Looking for a ${MODE_LABEL.bo1} opponent`);
    expect(box).not.toHaveTextContent(READOUT);
    // The tiles still say it.
    expect(tileOf("bo1")).toHaveTextContent("Usually under 30 s");
  });

  it("#1568 decks that could not be loaded offer Try again, which reads them again and makes the queue live", async () => {
    vi.mocked(getDecks).mockRejectedValueOnce(new Error("decks unavailable"));
    render(<PlayRoute token={TOKEN} />);

    expect(await screen.findByText("Your decks could not be loaded: decks unavailable")).toBeInTheDocument();
    expect(screen.getByTestId(playTestid.queue)).toBeDisabled();
    expect(screen.getByTestId(playTestid.decksRetry)).toBeInTheDocument();

    fireEvent.click(screen.getByTestId(playTestid.decksRetry));
    expect(screen.getByText("Loading your decks…")).toBeInTheDocument();
    expectSkeleton(screen.getByText("Loading your decks…"), "panel");
    expect(screen.queryByTestId(playTestid.decksRetry)).toBeNull();

    await screen.findByTestId(playTestid.deckSelect);
    expect(screen.queryByText("Loading your decks…")).toBeNull();
    expect(screen.getByTestId(playTestid.queue)).not.toBeDisabled();
    expect(screen.queryByRole("alert")).toBeNull();
    expect(getDecks).toHaveBeenCalledTimes(2);
  });
});

// answers and refusals

describe("the lobby's answers", () => {
  it("R253 a 422 loadout_invalid shows the server's messages verbatim", async () => {
    // Whatever the server says is shown as it said it, so these need not be the validator's words.
    const issues = [
      { rule: "L2", message: "Server sentence one about Half Built.", deck: 1 },
      { rule: "L5", message: "Server sentence two about Half Built.", deck: 1 },
    ];
    vi.mocked(enqueue).mockRejectedValue(
      new ApiRequestError(422, { code: "loadout_invalid", message: issues[0]?.message ?? "", details: issues }),
    );
    await renderLobby();
    fireEvent.click(screen.getByTestId(playTestid.queue));

    const error = await screen.findByTestId(playTestid.error);
    for (const issue of issues) expect(error).toHaveTextContent(issue.message);
  });

  it("any other refusal is the server's message, and a series still running is linked", async () => {
    vi.mocked(enqueue).mockRejectedValue(
      new ApiRequestError(409, {
        code: "already_in_match",
        message: "Finish your Conquest series first.",
        details: { seriesId: "series-9" },
      }),
    );
    await renderLobby();
    fireEvent.click(screen.getByTestId(playTestid.queue));

    const error = await screen.findByTestId(playTestid.error);
    expect(error).toHaveTextContent("Finish your Conquest series first.");
    expect(screen.getByTestId(playTestid.seriesLink)).toHaveAttribute("href", "/series/series-9");
  });

  it("R1566000 a player in a live co-op game stays on the hub, and a refusal naming it links the game", async () => {
    vi.mocked(getMe).mockResolvedValue({ ...me(null), currentDuosMatchId: "duos-4" });
    vi.mocked(enqueue).mockRejectedValue(
      new ApiRequestError(409, {
        code: "already_in_match",
        message: "Finish your co-op match first.",
        details: { duosMatchId: "duos-4" },
      }),
    );
    await renderLobby();
    await waitFor(() => {
      expect(vi.mocked(getMe)).toHaveBeenCalled();
    });
    expect(screen.queryByTestId(playTestid.status)).toBeNull();
    expect(vi.mocked(navigate)).not.toHaveBeenCalled();

    fireEvent.click(screen.getByTestId(playTestid.queue));

    const error = await screen.findByTestId(playTestid.error);
    expect(error).toHaveTextContent("Finish your co-op match first.");
    const link = within(error).getByTestId(playTestid.duosLink);
    expect(link).toHaveAttribute("href", "/duos/duos-4");
    expect(link).toHaveTextContent("Go to your co-op game");
    expect(within(error).queryByTestId(playTestid.seriesLink)).toBeNull();
    fireEvent.click(link);
    expect(vi.mocked(navigate)).toHaveBeenCalledWith("/duos/duos-4");
  });

  it("R259 an enqueue that pairs into a series goes to the series screen", async () => {
    vi.mocked(enqueue).mockResolvedValue({
      ...openTicket("bo3"),
      status: "matched",
      seriesId: "series-1",
    });
    await renderLobby();
    pickMode("bo3");
    fireEvent.click(screen.getByTestId(playTestid.queue));

    await waitFor(
      () => {
        expect(vi.mocked(navigate)).toHaveBeenCalledWith("/series/series-1");
      },
      { timeout: 5000 },
    );
  });
});

// R1372, R924000: All random's "More cards from"
describe("R1372 R924000 All random's more cards from", () => {
  it("R1372 R924000 the picker is All random's alone, names the newest set and starts at none", async () => {
    await renderLobby();
    expect(screen.queryByTestId(playTestid.leanNewest)).toBeNull();
    pickMode("random");
    const picker = screen.getByTestId(playTestid.leanNewest) as HTMLSelectElement;
    expect(picker.value).toBe("");
    expect(picker.closest("label")).toHaveTextContent(leanLabel());
    expect(within(picker).getByText(`The newest set (${newestShippedSet()})`)).toBeInTheDocument();
    pickMode("bo3");
    expect(screen.queryByTestId(playTestid.leanNewest)).toBeNull();
  });

  /** The lobby in All random, whichever mode this device remembered, once the picker is on screen. */
  async function renderRandomLobby(): Promise<HTMLSelectElement> {
    render(<PlayRoute token={TOKEN} />);
    await waitFor(() => {
      expect(vi.mocked(getDecks)).toHaveBeenCalled();
    });
    const random = screen.getByTestId(playModeTestid("random")) as HTMLInputElement;
    if (!random.checked) pickMode("random");
    return (await screen.findByTestId(playTestid.leanNewest)) as HTMLSelectElement;
  }

  function pickLean(picker: HTMLElement, value: string): void {
    fireEvent.change(picker, { target: { value } });
  }

  it("R924000 with a pick, Find a match sends leanSet; with none, it sends none", async () => {
    vi.mocked(enqueue).mockImplementation((_token, choice) => Promise.resolve(openTicket(choice.mode)));
    pickLean(await renderRandomLobby(), "Classic");
    expect((screen.getByTestId(playTestid.leanNewest) as HTMLSelectElement).value).toBe("Classic");

    fireEvent.click(screen.getByTestId(playTestid.queue));
    await waitFor(() => {
      expect(vi.mocked(enqueue)).toHaveBeenLastCalledWith(TOKEN, { mode: "random", leanSet: "Classic" });
    });
    // Locked while queued, like the rest of the setup.
    expect(screen.getByTestId(playTestid.leanNewest)).toBeDisabled();
    fireEvent.click(screen.getByTestId(playTestid.leaveQueue));
    await waitFor(() => {
      expect(screen.getByTestId(playTestid.queue)).not.toBeDisabled();
    });

    // None, the queue is joined with no lean at all.
    pickLean(screen.getByTestId(playTestid.leanNewest), "");
    fireEvent.click(screen.getByTestId(playTestid.queue));
    await waitFor(() => {
      expect(vi.mocked(enqueue)).toHaveBeenLastCalledWith(TOKEN, { mode: "random" });
    });
    fireEvent.click(screen.getByTestId(playTestid.leaveQueue));
    await waitFor(() => {
      expect(screen.getByTestId(playTestid.queue)).not.toBeDisabled();
    });

    // "Newest" is sent as the pick: the server resolves the set when it deals.
    pickLean(screen.getByTestId(playTestid.leanNewest), LEAN_NEWEST);
    fireEvent.click(screen.getByTestId(playTestid.queue));
    await waitFor(() => {
      expect(vi.mocked(enqueue)).toHaveBeenLastCalledWith(TOKEN, { mode: "random", leanSet: LEAN_NEWEST });
    });
  });

  it("R924000 the pick is kept on this device, R1372's switch reads as newest, and a refused storage only starts it at none", async () => {
    pickLean(await renderRandomLobby(), "Meditative");
    expect(window.localStorage.getItem(PLAY_LEAN_SET_KEY)).toBe("Meditative");
    cleanup();

    expect((await renderRandomLobby()).value).toBe("Meditative");
    cleanup();

    window.localStorage.removeItem(PLAY_LEAN_SET_KEY);
    window.localStorage.setItem(PLAY_LEAN_NEWEST_KEY, "true");
    expect((await renderRandomLobby()).value).toBe(LEAN_NEWEST);
    cleanup();

    const getItem = vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    const setItem = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    try {
      const picker = await renderRandomLobby();
      expect(picker.value).toBe("");
      pickLean(picker, "Core");
      expect((screen.getByTestId(playTestid.leanNewest) as HTMLSelectElement).value).toBe("Core");
    } finally {
      getItem.mockRestore();
      setItem.mockRestore();
    }
  });
});

// queue state: locked while queued, announced when paired

describe("the lobby's queue state", () => {
  it("pairTargetOf reads the board, the series screen, or nowhere", () => {
    expect(pairTargetOf({ currentMatchId: "match-42", currentSeriesId: null })).toBe("/match/match-42");
    expect(pairTargetOf({ currentMatchId: null, currentSeriesId: "series-7" })).toBe("/series/series-7");
    expect(pairTargetOf({ currentMatchId: null, currentSeriesId: null })).toBeNull();
    // The board wins when both are somehow set.
    expect(pairTargetOf({ currentMatchId: "match-1", currentSeriesId: "series-1" })).toBe("/match/match-1");
  });

  it("locks the setup while queued, with Leave as the way out", async () => {
    vi.mocked(enqueue).mockResolvedValue(openTicket("bo1"));
    await renderLobby();
    fireEvent.click(screen.getByTestId(playTestid.queue));

    await waitFor(() => {
      expect(screen.getByTestId(playTestid.searching)).toBeInTheDocument();
    });
    expect(screen.getByTestId(playTestid.queue)).toBeDisabled();
    for (const mode of ["bo1", "bo3", "random"] as const) {
      expect(screen.getByTestId(playModeTestid(mode))).toBeDisabled();
    }
    expect(screen.getByTestId(playTestid.deckSelect)).toBeDisabled();
    // ...but never traps the player: Leave stays lit.
    expect(screen.getByTestId(playTestid.leaveQueue)).not.toBeDisabled();
  });

  it("announces the pairing before navigating to it", async () => {
    vi.mocked(getMe).mockResolvedValue(me("match-42"));
    render(<PlayRoute token={TOKEN} />);

    // The status paints first and the navigation follows a beat later: a `navigate` in the
    // same breath would unmount this screen before the status commits, which a mocked
    // `navigate` cannot tell — so this asserts the order, not just both happening.
    expect(await screen.findByTestId(playTestid.status)).toHaveTextContent(MATCH_FOUND_STATUS);
    expect(vi.mocked(navigate)).not.toHaveBeenCalled();
    // The setup stays locked while the found beat plays, Leave included.
    expect(screen.getByTestId(playTestid.queue)).toBeDisabled();
    expect(screen.getByTestId(playTestid.leaveQueue)).toBeDisabled();
    for (const mode of ["bo1", "bo3", "random"] as const) {
      expect(screen.getByTestId(playModeTestid(mode))).toBeDisabled();
    }
    await waitFor(
      () => {
        expect(vi.mocked(navigate)).toHaveBeenCalledWith("/match/match-42");
      },
      { timeout: 5000 },
    );
  });

  it("an enqueue that pairs at once announces before navigating", async () => {
    vi.mocked(enqueue).mockResolvedValue({ ...openTicket("bo1"), status: "matched", matchId: "match-7" });
    await renderLobby();
    fireEvent.click(screen.getByTestId(playTestid.queue));

    // A pairing straight out of the answer announces itself like a watched one, not silently.
    expect(await screen.findByTestId(playTestid.status)).toHaveTextContent(MATCH_FOUND_STATUS);
    expect(vi.mocked(navigate)).not.toHaveBeenCalled();
    await waitFor(
      () => {
        expect(vi.mocked(navigate)).toHaveBeenCalledWith("/match/match-7");
      },
      { timeout: 5000 },
    );
  });

  it("R765 Find a match remembers the queue for the rest of the client, and Leave forgets it", async () => {
    vi.mocked(enqueue).mockResolvedValue(openTicket("random"));
    vi.mocked(dequeue).mockResolvedValue({ cancelled: true, ticketId: "tk-1" });
    await renderLobby();
    pickMode("random");
    fireEvent.click(screen.getByTestId(playTestid.queue));
    await screen.findByTestId(playTestid.searching);
    expect(readQueued()).toBe("random");
    expect(screen.getByTestId(playTestid.status)).toHaveTextContent("You can leave this screen while you wait.");

    fireEvent.click(screen.getByTestId(playTestid.leaveQueue));
    await screen.findByText("Left the queue.");
    expect(readQueued()).toBeNull();
  });

  it("R765 back on /play, the lobby picks up the wait it left: the mode, the queued notice, Leave and the watch", async () => {
    rememberQueued("bo3");
    vi.mocked(dequeue).mockResolvedValue({ cancelled: true, ticketId: "tk-1" });
    render(<PlayRoute token={TOKEN} />);
    // Conquest picks a trio, so its select is the one shown.
    await screen.findByTestId(playTestid.trioSelect);

    expect(screen.getByTestId(playTestid.searching)).toHaveAttribute("data-mode", "bo3");
    expect(screen.getByTestId(playModeTestid("bo3"))).toBeChecked();
    expect(screen.getByTestId(playTestid.status)).toHaveTextContent(`In the ${MODE_LABEL.bo3} queue.`);
    expect(screen.getByTestId(playTestid.queue)).toBeDisabled();
    expect(screen.getByTestId(playTestid.leaveQueue)).not.toBeDisabled();
    expect(enqueue).not.toHaveBeenCalled();

    fireEvent.click(screen.getByTestId(playTestid.leaveQueue));
    await screen.findByText("Left the queue.");
    expect(dequeue).toHaveBeenCalledWith(TOKEN);
    expect(readQueued()).toBeNull();
  });

  it("R765 a pairing the lobby finds forgets the queue, so nothing else follows it", async () => {
    rememberQueued("bo1");
    vi.mocked(getMe).mockResolvedValue(me("match-5"));
    render(<PlayRoute token={TOKEN} />);
    expect(await screen.findByTestId(playTestid.status)).toHaveTextContent(MATCH_FOUND_STATUS);
    expect(readQueued()).toBeNull();
  });

  it("R897000 a queue the server no longer holds ends the wait with the rejoin message", async () => {
    vi.mocked(enqueue).mockResolvedValue(openTicket("bo1"));
    vi.mocked(getMe).mockResolvedValue({ ...me(null), queued: false });
    await renderLobby();
    fireEvent.click(screen.getByTestId(playTestid.queue));
    await screen.findByTestId(playTestid.searching);

    // Two reads in a row name no ticket: the wait ends, and Find a match is the way back in.
    await screen.findByText("Your queue expired. Find a match to queue again.", undefined, { timeout: 8000 });
    expect(screen.queryByTestId(playTestid.searching)).toBeNull();
    expect(screen.getByTestId(playTestid.queue)).not.toBeDisabled();
    expect(readQueued()).toBeNull();
  });

  it("R897000 queued true keeps searching", async () => {
    vi.mocked(enqueue).mockResolvedValue(openTicket("bo1"));
    vi.mocked(getMe).mockResolvedValue({ ...me(null), queued: true });
    await renderLobby();
    fireEvent.click(screen.getByTestId(playTestid.queue));
    await screen.findByTestId(playTestid.searching);

    // Reads keep naming the ticket: still waiting, no expiry.
    await waitFor(
      () => {
        expect(vi.mocked(getMe).mock.calls.length).toBeGreaterThanOrEqual(2);
      },
      { timeout: 8000 },
    );
    expect(screen.getByTestId(playTestid.searching)).toBeInTheDocument();
    expect(screen.queryByText("Your queue expired. Find a match to queue again.")).toBeNull();
    expect(screen.getByTestId(playTestid.queue)).toBeDisabled();
    expect(readQueued()).toBe("bo1");
  });
});

describe("R3207000 the queue readout's pace", () => {
  const POPULATION_MS = QUEUE_POPULATION_POLL_SECONDS * 1000;
  const ACCOUNT_MS = SERIES_POLL_SECONDS * 1000;

  afterEach(() => {
    vi.useRealTimers();
  });

  it("R3207000 the queue wait reads the population on mount, then every QUEUE_POPULATION_POLL_SECONDS, while its account read keeps SERIES_POLL_SECONDS", async () => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    rememberQueued("bo1");
    render(<PlayRoute token={TOKEN} />);
    await settle({ advance: 0 });
    expect(vi.mocked(getMe)).toHaveBeenCalledTimes(1);
    expect(vi.mocked(getPopulation)).toHaveBeenCalledTimes(1);

    // The ticket's read has its own pace: one every SERIES_POLL_SECONDS, and none of them reads the readout.
    await settle({ advance: ACCOUNT_MS });
    expect(vi.mocked(getMe)).toHaveBeenCalledTimes(2);
    expect(vi.mocked(getPopulation)).toHaveBeenCalledTimes(1);

    await settle({ advance: POPULATION_MS - ACCOUNT_MS - 1 });
    expect(vi.mocked(getPopulation)).toHaveBeenCalledTimes(1);
    await settle({ advance: 1 });
    expect(vi.mocked(getPopulation)).toHaveBeenCalledTimes(2);
    expect(vi.mocked(getMe).mock.calls.length).toBe(1 + POPULATION_MS / ACCOUNT_MS);

    await settle({ advance: POPULATION_MS });
    expect(vi.mocked(getPopulation)).toHaveBeenCalledTimes(3);
  });

  it("R3207000 an idle lobby reads the population once", async () => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    render(<PlayRoute token={TOKEN} />);
    await settle({ advance: 0 });
    expect(vi.mocked(getPopulation)).toHaveBeenCalledTimes(1);

    await settle({ advance: 3 * POPULATION_MS });
    expect(vi.mocked(getPopulation)).toHaveBeenCalledTimes(1);
  });

  it("R3207000 leaving the wait ends the population poll", async () => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    rememberQueued("bo1");
    vi.mocked(dequeue).mockResolvedValue({ cancelled: true, ticketId: "tk-1" });
    render(<PlayRoute token={TOKEN} />);
    await settle({ advance: 0 });
    expect(vi.mocked(getPopulation)).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByTestId(playTestid.leaveQueue));
    await settle({ advance: 0 });
    await settle({ advance: 3 * POPULATION_MS });
    expect(vi.mocked(getPopulation)).toHaveBeenCalledTimes(1);
  });
});

describe("R3792000 the wait says when the server cannot be reached", () => {
  const ACCOUNT_MS = SERIES_POLL_SECONDS * 1000;
  const NOTICE = "Can't reach the server. Still trying…";

  beforeEach(() => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  const down = (): void => {
    vi.mocked(getMe).mockRejectedValue(new ApiUnreachableError(new TypeError("Failed to fetch")));
  };

  /** Mount the queue wait and let `reads` account reads pass (the mount's among them). */
  async function waitReads(reads: number): Promise<void> {
    rememberQueued("bo1");
    render(<PlayRoute token={TOKEN} />);
    await settle({ advance: 0 });
    for (let i = 1; i < reads; i += 1) await settle({ advance: ACCOUNT_MS });
  }

  it("R3792000 the constant's count of unreachable reads shows the notice and keeps the queue line", async () => {
    down();
    await waitReads(LOBBY_UNREACHABLE_AFTER_READS);
    expect(vi.mocked(getMe)).toHaveBeenCalledTimes(LOBBY_UNREACHABLE_AFTER_READS);
    const notice = screen.getByTestId(playTestid.unreachable);
    expect(notice).toHaveTextContent(NOTICE);
    expect(notice).toHaveAttribute("role", "status");
    expect(screen.getByTestId(playTestid.status)).toHaveTextContent(`In the ${MODE_LABEL.bo1} queue.`);
    expect(screen.getByTestId(playTestid.searching)).toBeInTheDocument();
  });

  it("R3792000 fewer failed reads than the constant show nothing", async () => {
    down();
    await waitReads(LOBBY_UNREACHABLE_AFTER_READS - 1);
    expect(vi.mocked(getMe)).toHaveBeenCalledTimes(LOBBY_UNREACHABLE_AFTER_READS - 1);
    expect(screen.queryByTestId(playTestid.unreachable)).toBeNull();
  });

  it("R3792000 a good read after the notice clears it and starts the count again", async () => {
    down();
    await waitReads(LOBBY_UNREACHABLE_AFTER_READS);
    expect(screen.getByTestId(playTestid.unreachable)).toBeInTheDocument();

    vi.mocked(getMe).mockResolvedValue({ ...me(null), queued: true });
    await settle({ advance: ACCOUNT_MS });
    expect(screen.queryByTestId(playTestid.unreachable)).toBeNull();

    down();
    for (let i = 1; i < LOBBY_UNREACHABLE_AFTER_READS; i += 1) await settle({ advance: ACCOUNT_MS });
    expect(screen.queryByTestId(playTestid.unreachable)).toBeNull();
    await settle({ advance: ACCOUNT_MS });
    expect(screen.getByTestId(playTestid.unreachable)).toBeInTheDocument();
  });

  it("R3792000 reads saying the ticket is gone still end the wait after the notice", async () => {
    down();
    await waitReads(LOBBY_UNREACHABLE_AFTER_READS);
    expect(screen.getByTestId(playTestid.unreachable)).toBeInTheDocument();

    vi.mocked(getMe).mockResolvedValue({ ...me(null), queued: false });
    for (let i = 0; i < QUEUE_ENDED_AFTER_READS; i += 1) await settle({ advance: ACCOUNT_MS });
    expect(screen.getByText("Your queue expired. Find a match to queue again.")).toBeInTheDocument();
    expect(screen.queryByTestId(playTestid.unreachable)).toBeNull();
    expect(readQueued()).toBeNull();
  });

  it("R3792000 a read the server refused with a 4xx is not an unreachable one", async () => {
    vi.mocked(getMe).mockRejectedValue(new ApiRequestError(403, { code: "forbidden", message: "No." }));
    await waitReads(LOBBY_UNREACHABLE_AFTER_READS + 2);
    expect(vi.mocked(getMe)).toHaveBeenCalledTimes(LOBBY_UNREACHABLE_AFTER_READS + 2);
    expect(screen.queryByTestId(playTestid.unreachable)).toBeNull();
  });

  it("R3846000 a run of 5xx answers shows the notice like unreachable reads", async () => {
    vi.mocked(getMe).mockRejectedValue(new ApiRequestError(503, { code: "unavailable", message: "Try again." }));
    await waitReads(LOBBY_UNREACHABLE_AFTER_READS);
    expect(vi.mocked(getMe)).toHaveBeenCalledTimes(LOBBY_UNREACHABLE_AFTER_READS);
    expect(screen.getByTestId(playTestid.unreachable)).toHaveTextContent(NOTICE);
    expect(screen.getByTestId(playTestid.status)).toHaveTextContent(`In the ${MODE_LABEL.bo1} queue.`);
  });

  it("R3846000 fewer 5xx answers than the constant show nothing", async () => {
    vi.mocked(getMe).mockRejectedValue(new ApiRequestError(500, { code: "internal", message: "Oops." }));
    await waitReads(LOBBY_UNREACHABLE_AFTER_READS - 1);
    expect(vi.mocked(getMe)).toHaveBeenCalledTimes(LOBBY_UNREACHABLE_AFTER_READS - 1);
    expect(screen.queryByTestId(playTestid.unreachable)).toBeNull();
  });

  it("R3846000 a 4xx answer after the notice clears it and starts the count again", async () => {
    const unavailable = new ApiRequestError(503, { code: "unavailable", message: "Try again." });
    vi.mocked(getMe).mockRejectedValue(unavailable);
    await waitReads(LOBBY_UNREACHABLE_AFTER_READS);
    expect(screen.getByTestId(playTestid.unreachable)).toBeInTheDocument();

    vi.mocked(getMe).mockRejectedValue(new ApiRequestError(401, { code: "unauthorized", message: "No." }));
    await settle({ advance: ACCOUNT_MS });
    expect(screen.queryByTestId(playTestid.unreachable)).toBeNull();

    vi.mocked(getMe).mockRejectedValue(unavailable);
    for (let i = 1; i < LOBBY_UNREACHABLE_AFTER_READS; i += 1) await settle({ advance: ACCOUNT_MS });
    expect(screen.queryByTestId(playTestid.unreachable)).toBeNull();
    await settle({ advance: ACCOUNT_MS });
    expect(screen.getByTestId(playTestid.unreachable)).toBeInTheDocument();
  });

  it("R3846000 a mixed run of transport failures and 5xx answers counts together", async () => {
    vi.mocked(getMe)
      .mockRejectedValueOnce(new ApiUnreachableError(new TypeError("Failed to fetch")))
      .mockRejectedValue(new ApiRequestError(502, { code: "bad_gateway", message: "Try again." }));
    await waitReads(LOBBY_UNREACHABLE_AFTER_READS);
    expect(vi.mocked(getMe)).toHaveBeenCalledTimes(LOBBY_UNREACHABLE_AFTER_READS);
    expect(screen.getByTestId(playTestid.unreachable)).toHaveTextContent(NOTICE);
  });

  it("R3792000 an idle lobby never shows it", async () => {
    down();
    render(<PlayRoute token={TOKEN} />);
    await settle({ advance: 0 });
    await settle({ advance: 2 * LOBBY_UNREACHABLE_AFTER_READS * ACCOUNT_MS });
    expect(screen.queryByTestId(playTestid.unreachable)).toBeNull();
  });
});

/** The account a gate that read `body` just now hands the lobby: signed in, and kept fresh as `useAccount` keeps it. */
function handedFresh(body: ReturnType<typeof me>): ReturnType<typeof me> {
  writeSession({ accessToken: TOKEN });
  keepRead(TOKEN, "/api/auth/me", body);
  return body;
}

describe("R3207003 the lobby takes the gate's account", () => {
  afterEach(() => {
    vi.useRealTimers();
    clearSession();
  });

  it("R3207003 a lobby handed an account that names no game reads none at mount", async () => {
    render(<PlayRoute token={TOKEN} me={handedFresh(me(null))} />);
    await waitFor(() => {
      expect(vi.mocked(getDecks)).toHaveBeenCalled();
    });
    await settle({ macrotasks: 3 });

    expect(vi.mocked(getMe)).not.toHaveBeenCalled();
    expect(vi.mocked(navigate)).not.toHaveBeenCalled();
  });

  it("R3207003 a queue wait handed an account that names no game starts its reads at its first tick", async () => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    rememberQueued("bo1");
    render(<PlayRoute token={TOKEN} me={handedFresh(me(null))} />);
    await settle({ advance: 0 });
    expect(vi.mocked(getMe)).not.toHaveBeenCalled();

    await settle({ advance: SERIES_POLL_SECONDS * 1000 });
    expect(vi.mocked(getMe)).toHaveBeenCalledTimes(1);
  });

  it("R3207003 an account the gate opened on from an old kept answer is read again at mount, and a game that started since takes the player there", async () => {
    // The gate opens on a kept answer of any age (R2630000) and reads again behind it: this one is old, so it
    // is no stand-in for the mount read, and the game it does not know of is found.
    const stale = handedFresh(me(null));
    expireRead(TOKEN, "/api/auth/me");
    vi.mocked(getMe).mockResolvedValue(me("match-42"));
    render(<PlayRoute token={TOKEN} me={stale} />);

    await waitFor(
      () => {
        expect(vi.mocked(navigate)).toHaveBeenCalledWith("/match/match-42");
      },
      { timeout: 5000 },
    );
    expect(vi.mocked(getMe)).toHaveBeenCalledTimes(1);
  });

  it("R3207003 an account that names a match is read again, and a match that ended keeps the player in the lobby", async () => {
    // The gate's answer may be 45 s old (R3206000): it says a match, the server now says none.
    vi.mocked(getMe).mockResolvedValue(me(null));
    render(<PlayRoute token={TOKEN} me={handedFresh(me("match-42"))} />);
    await waitFor(() => {
      expect(vi.mocked(getMe)).toHaveBeenCalledTimes(1);
    });
    await settle({ macrotasks: 3 });

    expect(vi.mocked(navigate)).not.toHaveBeenCalled();
    expect(screen.queryByTestId(playTestid.status)).toBeNull();
  });

  it("R3207003 an account that names a match, read again, still takes the player there while the match is on", async () => {
    vi.mocked(getMe).mockResolvedValue(me("match-42"));
    render(<PlayRoute token={TOKEN} me={handedFresh(me("match-42"))} />);

    await waitFor(
      () => {
        expect(vi.mocked(navigate)).toHaveBeenCalledWith("/match/match-42");
      },
      { timeout: 5000 },
    );
    expect(vi.mocked(getMe)).toHaveBeenCalledTimes(1);
  });
});
