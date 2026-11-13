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
import { forgetQueued, readQueued, rememberQueued } from "../net/liveGame.ts";
import { MenuShellContext } from "../shell/context.ts";
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
import { getDuosLive } from "../net/duos.ts";
import { lobbyTestid } from "../lobby/testids.ts";
import { settle } from "../test/settle.ts";
import { must } from "../test/stubServer.ts";
import { dialogTestid } from "../game/Dialog.tsx";
import { TUTORIAL_LESSONS } from "../tutorial/lessons.ts";
import { PLAY_GROUPS, playCards } from "./playModes.ts";
import { expectSkeleton } from "../test/skeleton.ts";
import { currentCatalog } from "../game/currentCatalog.ts";
import { PLAY_LEAN_NEWEST_KEY, PLAY_LEAN_SET_KEY, leanLabel } from "../game/LeanNewest.tsx";
import PlayRoute, {
  LOBBY_UNREACHABLE_AFTER_READS,
  MATCH_FOUND_STATUS,
  MODE_LABEL,
  PLAY_CHOICE_KEY,
  pairTargetOf,
  playModeTestid,
  playTestid,
  playWaitTestid,
} from "./play.tsx";
import { playCardRankTestid, playGroupTestid } from "./playTestids.ts";
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
vi.mock("../net/duos.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/duos.ts")>();
  return { ...actual, getDuosLive: vi.fn() };
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

/** R3729000: an Online card opens its mode's panel, which is where the picks and Find a match are. */
function pickMode(mode: QueueMode): void {
  fireEvent.click(screen.getByTestId(playModeTestid(mode)));
}

/** Open `mode`'s panel again, after a wait ended: the cards were locked while it lasted. */
async function reopen(mode: QueueMode): Promise<void> {
  await waitFor(() => {
    expect(screen.getByTestId(playModeTestid(mode))).not.toBeDisabled();
  });
  pickMode(mode);
  await screen.findByTestId(playTestid.queuePanel);
}

/** Render the hub, wait for its decks to load, and open `open`'s panel (`null`: the hub alone). */
async function renderLobby(open: QueueMode | null = "bo1"): Promise<void> {
  render(<PlayRoute token={TOKEN} />);
  await waitFor(() => {
    expect(vi.mocked(getDecks)).toHaveBeenCalled();
  });
  await screen.findByTestId(playTestid.hub);
  if (open === null) return;
  pickMode(open);
  await screen.findByTestId(playTestid.queuePanel);
  if (open === "bo1") await screen.findByTestId(playTestid.deckSelect);
  if (open === "bo3") await screen.findByTestId(playTestid.trioSelect);
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

function precedes(first: HTMLElement, second: HTMLElement): boolean {
  return (first.compareDocumentPosition(second) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;
}

describe("R1578000 R3729000 the Play hub", () => {
  const LINKS: readonly (readonly [string, string])[] = [
    // #3727: daily games, in their own group.
    [playTestid.daily, paths.daily],
    [playTestid.challenge, paths.together],
    [playTestid.practice, paths.practice],
    // B13: the Play hub links the Dungeon run.
    [playTestid.dungeon, paths.dungeon],
    [playTestid.tutorial, `${paths.practice}?lesson=${must(TUTORIAL_LESSONS[0], "a first lesson").id}`],
  ];

  it("R3223000 the hub has one visible h1, the PageTitle, signed in and signed out", async () => {
    await renderLobby(null);
    expectOnePageTitle("Play");
    expect(screen.queryByRole("heading", { name: "JackiOh" })).toBeNull();

    cleanup();
    render(<PlayRoute token={null} />);
    expectOnePageTitle("Play");
  });

  it("R3729000 the hub shows four groups of cards in order, each card with its name, time and what it needs", async () => {
    await renderLobby(null);

    const hub = screen.getByTestId(playTestid.hub);
    const groups = within(hub).getAllByRole("heading", { level: 2 }).map((heading) => heading.textContent);
    expect(groups).toEqual(["Online", "Daily", "With friends", "Solo"]);
    for (const group of PLAY_GROUPS) expect(within(hub).getByTestId(playGroupTestid(group))).toBeInTheDocument();
    const cards = playCards({ signedIn: true, dailyWaiting: 0 });
    expect(cards.map((card) => card.id)).toEqual([
      "bo1",
      "bo3",
      "random",
      "daily",
      "lobby",
      "challenge",
      "practice",
      "dungeon",
      "tutorial",
    ]);
    for (const card of cards) {
      const element = within(hub).getByTestId(card.testid);
      expect(element).toHaveTextContent(card.name);
      expect(element.querySelector('[data-field="time"]')).toHaveTextContent(card.time);
      expect(element.querySelector('[data-field="needs"]')).toHaveTextContent(card.needs);
    }
    expect(within(hub).getByTestId(playGroupTestid("online"))).toContainElement(screen.getByTestId(playModeTestid("bo1")));
    expect(within(hub).getByTestId(playGroupTestid("daily"))).toContainElement(screen.getByTestId(playTestid.daily));
    expect(within(hub).getByTestId(playGroupTestid("friends"))).toContainElement(screen.getByTestId(playTestid.lobby));
    expect(within(hub).getByTestId(playGroupTestid("solo"))).toContainElement(screen.getByTestId(playTestid.tutorial));
  });

  it("R3729000 an Online card opens one panel, which holds the picker and Find a match as the screen's one primary action", async () => {
    await renderLobby(null);

    expect(screen.queryByTestId(playTestid.queuePanel)).toBeNull();
    expect(screen.queryByTestId(playTestid.queue)).toBeNull();
    pickMode("bo1");
    const panel = await screen.findByTestId(playTestid.queuePanel);
    expect(panel).toHaveAttribute("role", "dialog");
    expect(within(panel).getByRole("heading", { name: "Best of 1" })).toBeInTheDocument();
    expect(await within(panel).findByTestId(playTestid.deckSelect)).toBeInTheDocument();
    expect(await within(panel).findByTestId(playTestid.rank)).toBeInTheDocument();
    const find = within(panel).getByTestId(playTestid.queue);
    expect(find).toHaveClass("button-primary");
    expect(find).toHaveTextContent("Find a match");
    // The hub's cards are never a second primary button, and a link card is a plain link.
    const hub = screen.getByTestId(playTestid.hub);
    expect(hub.querySelector(".button-primary")).toBeNull();
    for (const [testid] of LINKS) expect(within(hub).getByTestId(testid).tagName).toBe("A");
    // The panel closes by its ✕ and leaves the hub as it was.
    fireEvent.click(within(panel).getByTestId(dialogTestid.close));
    expect(screen.queryByTestId(playTestid.queuePanel)).toBeNull();
  });

  it("R1578000 a link card opens its screen inside the app, and Replays and Play together sit under the groups", async () => {
    await renderLobby(null);

    for (const [testid, href] of LINKS) expect(screen.getByTestId(testid)).toHaveAttribute("href", href);
    const more = screen.getByTestId(playTestid.more);
    expect(within(more).getByTestId(playTestid.replays)).toHaveAttribute("href", paths.replays);
    expect(within(more).getByTestId(playTestid.together)).toHaveAttribute("href", paths.together);
    expect(screen.getAllByTestId(playTestid.replays)).toHaveLength(1);
    fireEvent.click(screen.getByTestId(playTestid.together));
    expect(vi.mocked(navigate)).toHaveBeenCalledWith(paths.together);
  });

  it("R3729000 the Daily card names no waiting game while none waits on the player", async () => {
    await renderLobby(null);
    expect(screen.getByTestId(playTestid.daily)).toHaveTextContent("A game played over days");
    expect(screen.queryByTestId(playTestid.dailyBadge)).toBeNull();
  });

  it("R1578000 R3729000 signed out reads nothing, offers sign-in and shows the solo cards alone", async () => {
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
    const hub = screen.getByTestId(playTestid.hub);
    expect(precedes(online, hub)).toBe(true);
    expect(screen.getByTestId(playGroupTestid("solo"))).toBeInTheDocument();
    for (const group of ["online", "daily", "friends"] as const) expect(screen.queryByTestId(playGroupTestid(group))).toBeNull();
    const soloIds: readonly string[] = [playTestid.practice, playTestid.dungeon, playTestid.tutorial];
    for (const [testid, href] of LINKS.filter(([id]) => soloIds.includes(id))) {
      expect(within(hub).getByTestId(testid)).toHaveAttribute("href", href);
    }
    expect(within(hub).getByTestId(playTestid.replays)).toHaveAttribute("href", paths.replays);
    expect(within(hub).getByTestId(playTestid.together)).toHaveAttribute("href", paths.together);
    // Nothing leaves the page: no decks, rank, population, lobbies, collection or me.
    for (const read of [getDecks, getMe, getOwnRank, getPopulation, getMyLobbies, getCollection]) {
      expect(vi.mocked(read)).not.toHaveBeenCalled();
    }
    fireEvent.click(screen.getByTestId(playTestid.signIn));
    expect(vi.mocked(navigate)).toHaveBeenCalledWith(paths.login);
  });
});

describe("R3729000 the private lobby card", () => {
  it("R3729000 R3730000 the Private lobby card opens the lobby's panels, a 1v1 in the chosen mode and a co-op", async () => {
    vi.mocked(createLobby).mockResolvedValue({ code: "ABC234" });
    vi.mocked(getDuosLive).mockResolvedValue({ matchId: null });
    await renderLobby();
    // Best of 1 is the card's panel; pick Conquest, close its panel, and open the lobby with it chosen.
    pickMode("bo3");
    await screen.findByTestId(playTestid.trioSelect);
    fireEvent.click(screen.getByTestId(dialogTestid.close));
    expect(screen.queryByTestId(lobbyTestid.panel)).toBeNull();

    fireEvent.click(screen.getByTestId(playTestid.lobby));
    const dialog = await screen.findByTestId(playTestid.lobbyDialog);
    expect(within(dialog).getByRole("heading", { name: "Private lobby" })).toBeInTheDocument();
    const panels = within(dialog).getAllByTestId(lobbyTestid.panel);
    expect(panels.map((panel) => panel.getAttribute("data-format"))).toEqual(["duel", "coop"]);
    const duel = must(panels[0], "the 1v1 panel");
    expect(within(duel).getByRole("heading", { name: "Play a friend" })).toBeInTheDocument();
    // The classic room's controls are gone from /play: a friend game starts in a lobby.
    expect(screen.queryByTestId("play-create-room")).toBeNull();
    expect(screen.queryByTestId("play-join-form")).toBeNull();

    fireEvent.click(within(duel).getByTestId(lobbyTestid.create));
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
    // R3729000: the rank is the panel's own, above the picks, and the mode's card shows it too.
    expect(screen.getByTestId(playTestid.queuePanel)).toContainElement(rank);
    expect(screen.getByTestId(playCardRankTestid("bo1"))).toHaveTextContent("Normal Grape III");
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
    // R3729000: both ladders are read at mount, and each Online card shows its own ladder's rank.
    await waitFor(() => expect(screen.getByTestId(playCardRankTestid("random"))).toHaveTextContent("Golden Grape II"));
    for (const mode of ["bo1", "bo3"] as const) expect(screen.getByTestId(playCardRankTestid(mode))).toHaveTextContent("Normal Grape III");

    pickMode("random");
    await waitFor(() => expect(screen.getByTestId(playTestid.rank)).toHaveAttribute("data-ladder", "random"));
    expect(screen.getByTestId(playTestid.rank)).toHaveTextContent("All random · Golden Grape II");
    expect(screen.getByTestId(playTestid.leaderboard)).toHaveAttribute("href", `${paths.leaderboard}?ladder=random`);

    // Moving between the modes reads nothing again: one read per ladder, at mount.
    pickMode("bo3");
    await waitFor(() => expect(screen.getByTestId(playTestid.rank)).toHaveAttribute("data-ladder", "constructed"));
    pickMode("bo1");
    expect(vi.mocked(getOwnRank).mock.calls).toEqual([
      [TOKEN, "constructed"],
      [TOKEN, "random"],
    ]);
  });

  it("R661 puts R612's words on placements and a Jlorious position too, and never a rating", async () => {
    vi.mocked(getOwnRank).mockResolvedValue(
      rankBody({ rank: { tier: "raisin", placementsPlayed: 4, placementGames: 10 } }),
    );
    await renderLobby();
    const placing = await screen.findByTestId(playTestid.rank);
    expect(placing).toHaveTextContent("Raisin · Season v0.2");
    expect(within(placing).getByRole("img", { name: "Raisin · 4/10 placements" })).toBeInTheDocument();
    cleanup();

    vi.mocked(getOwnRank).mockResolvedValue(rankBody({ rank: { tier: "jlorious", position: 42 } }));
    await renderLobby();
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
  it("R257 R3729000 the Online cards offer Best of 1, Conquest and All random, each opening its own choice", async () => {
    await renderLobby();

    for (const mode of ["bo1", "bo3", "random"] as const) {
      expect(screen.getByTestId(playModeTestid(mode))).toHaveAttribute("aria-haspopup", "dialog");
    }
    // Best of 1 opens a deck, listed by name, the first complete one chosen.
    expect(screen.getByTestId(playTestid.queuePanel)).toHaveAttribute("data-mode", "bo1");
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
    expect(screen.getByTestId(playTestid.queuePanel)).toHaveAttribute("data-mode", "bo3");
    expect(screen.queryByTestId(playTestid.deckSelect)).toBeNull();
    const trioSelect = screen.getByTestId(playTestid.trioSelect) as HTMLSelectElement;
    expect(Array.from(trioSelect.options).map((option) => option.textContent)).toEqual(["Main trio", "Loose trio"]);

    pickMode("random");
    expect(screen.getByTestId(playTestid.queuePanel)).toHaveAttribute("data-mode", "random");
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
    // R3729000: the panel closes once the server took the ticket.
    expect(screen.queryByTestId(playTestid.queuePanel)).toBeNull();

    // The cards lock while queued, so the next mode is picked after leaving it.
    fireEvent.click(screen.getByTestId(playTestid.leaveQueue));
    await waitFor(() => {
      expect(screen.getByTestId(playModeTestid("bo3"))).not.toBeDisabled();
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
      expect(screen.getByTestId(playModeTestid("random"))).not.toBeDisabled();
    });
    pickMode("random");
    fireEvent.click(screen.getByTestId(playTestid.queue));
    await waitFor(() => {
      expect(vi.mocked(enqueue)).toHaveBeenLastCalledWith(TOKEN, { mode: "random" });
    });
    await screen.findByText(/In the All random queue/);
  });

  it("R3729000 remembers the last mode, deck and trio on this device, and marks the card of the last mode", async () => {
    window.localStorage.setItem(PLAY_CHOICE_KEY, JSON.stringify({ mode: "bo3", deckId: RAMP.id, trioId: LOOSE.id }));
    await renderLobby(null);

    expect(screen.getByTestId(playModeTestid("bo3"))).toHaveAttribute("data-last", "true");
    expect(within(screen.getByTestId(playModeTestid("bo3"))).getByTestId(playTestid.lastUsed)).toHaveTextContent("Last used");
    expect(screen.getAllByTestId(playTestid.lastUsed)).toHaveLength(1);
    pickMode("bo3");
    const trioSelect = (await screen.findByTestId(playTestid.trioSelect)) as HTMLSelectElement;
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
    await renderLobby(null);
    pickMode("bo1");

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
    const tileOf = (mode: QueueMode): HTMLElement | null => screen.getByTestId(playModeTestid(mode)).closest("button");
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
    expect(screen.getByTestId(playTestid.hub)).not.toHaveTextContent(/usually|played in the last hour/i);
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
    const setup = screen.getByTestId(playTestid.queuePanel);
    expect(setup).toHaveTextContent("No deck needed: the server deals both of you one when the game starts.");
    expect(within(setup).queryByTestId(playTestid.deckSelect)).toBeNull();
    expect(within(setup).queryByTestId(playTestid.trioSelect)).toBeNull();
    // No card, no card count and no curve: nothing of a dealt deck is known before its game.
    for (const def of Object.values(DEFS)) expect(setup).not.toHaveTextContent(def.name);
    expect(setup.querySelector(".play-pick-summary")).toBeNull();
  });

  it("R505 R640002 the queue's readout is on the mode cards and the games line alone: the queue panel and the queued notice repeat none of it", async () => {
    vi.mocked(enqueue).mockImplementation((_token, choice) => Promise.resolve(openTicket(choice.mode)));
    await renderLobby();
    const tileOf = (mode: QueueMode): HTMLElement | null => screen.getByTestId(playModeTestid(mode)).closest("button");
    await waitFor(() => {
      expect(tileOf("bo1")).toHaveTextContent("Usually under 30 s");
    });
    expect(tileOf("random")).toHaveTextContent("Usually under 2 min");
    expect(screen.getByTestId(playTestid.hub).parentElement).toContainElement(screen.getByTestId(playTestid.recentGames));
    expect(screen.getByTestId(playTestid.recentGames)).toHaveTextContent("12 games played in the last hour");

    const READOUT = /usually|played in the last hour|\b(12|30|90)\b/i;
    const panel = screen.getByTestId(playTestid.queuePanel);
    expect(panel).not.toHaveTextContent(READOUT);
    expect(within(panel).queryByTestId(playTestid.recentGames)).toBeNull();

    // Queued: the search says what it is doing and the notice where you are, and neither repeats it.
    fireEvent.click(screen.getByTestId(playTestid.queue));
    const status = await screen.findByTestId(playTestid.status);
    const box = screen.getByRole("region", { name: "Find a match" });
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
    await screen.findByTestId(playTestid.hub);
    pickMode("bo1");

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
    await screen.findByTestId(playTestid.hub);
    pickMode("random");
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
    // Locked while queued, like the rest of the setup: its panel is closed and no card opens it.
    expect(screen.queryByTestId(playTestid.leanNewest)).toBeNull();
    expect(screen.getByTestId(playModeTestid("random"))).toBeDisabled();
    fireEvent.click(screen.getByTestId(playTestid.leaveQueue));
    await reopen("random");

    // None, the queue is joined with no lean at all.
    pickLean(screen.getByTestId(playTestid.leanNewest), "");
    fireEvent.click(screen.getByTestId(playTestid.queue));
    await waitFor(() => {
      expect(vi.mocked(enqueue)).toHaveBeenLastCalledWith(TOKEN, { mode: "random" });
    });
    fireEvent.click(screen.getByTestId(playTestid.leaveQueue));
    await reopen("random");

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
    // R3729000: the panel closed with the ticket, and no card opens another while the wait lasts.
    expect(screen.queryByTestId(playTestid.queuePanel)).toBeNull();
    expect(screen.queryByTestId(playTestid.queue)).toBeNull();
    for (const mode of ["bo1", "bo3", "random"] as const) {
      expect(screen.getByTestId(playModeTestid(mode))).toBeDisabled();
    }
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
    // The cards stay locked while the found beat plays, and a found match is no search to leave.
    expect(screen.queryByTestId(playTestid.leaveQueue)).toBeNull();
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
    await screen.findByTestId(playTestid.searching);

    expect(screen.getByTestId(playTestid.searching)).toHaveAttribute("data-mode", "bo3");
    expect(screen.getByTestId(playModeTestid("bo3"))).toHaveAttribute("data-last", "true");
    expect(screen.getByTestId(playTestid.status)).toHaveTextContent(`In the ${MODE_LABEL.bo3} queue.`);
    expect(screen.getByTestId(playModeTestid("bo3"))).toBeDisabled();
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
    expect(screen.getByTestId(playModeTestid("bo1"))).not.toBeDisabled();
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
    expect(screen.getByTestId(playModeTestid("bo1"))).toBeDisabled();
    expect(readQueued()).toBe("bo1");
  });

  it("R3729001 inside the menus' shell the banner shows the search, and Cancel there ends the wait here", async () => {
    vi.mocked(enqueue).mockResolvedValue(openTicket("bo1"));
    render(
      <MenuShellContext.Provider value={{ status: null }}>
        <PlayRoute token={TOKEN} />
      </MenuShellContext.Provider>,
    );
    await waitFor(() => {
      expect(vi.mocked(getDecks)).toHaveBeenCalled();
    });
    await screen.findByTestId(playTestid.hub);
    pickMode("bo1");
    await screen.findByTestId(playTestid.deckSelect);
    fireEvent.click(screen.getByTestId(playTestid.queue));
    await screen.findByText(/In the Best of 1 queue/);
    // The banner is the one search display: this screen draws none of its own.
    expect(screen.queryByTestId(playTestid.searching)).toBeNull();
    expect(screen.queryByTestId(playTestid.leaveQueue)).toBeNull();
    expect(readQueued()).toBe("bo1");

    // Cancel on the banner forgets the queue from outside this screen.
    act(() => {
      forgetQueued();
    });
    await screen.findByText("Left the queue.");
    expect(screen.getByTestId(playModeTestid("bo1"))).not.toBeDisabled();
  });

  it("R3729001 Leave on this screen ends its own wait once, and the banner's Cancel path adds no second line", async () => {
    vi.mocked(enqueue).mockResolvedValue(openTicket("bo1"));
    vi.mocked(dequeue).mockResolvedValue({ cancelled: true, ticketId: "tk-1" });
    await renderLobby();
    fireEvent.click(screen.getByTestId(playTestid.queue));
    await screen.findByTestId(playTestid.searching);
    fireEvent.click(screen.getByTestId(playTestid.leaveQueue));
    // Leave is this screen's own end: one line, "Left the queue.", never a second one for the queue it forgot.
    await screen.findByText("Left the queue.");
    expect(screen.getAllByTestId(playTestid.status)).toHaveLength(1);
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
