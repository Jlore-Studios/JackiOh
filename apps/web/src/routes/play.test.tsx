// `/play`: the lobby's modes, choices and refusals (R257, R264), and its watch.
//
// The watch: only one side's HTTP response ever carries the match (or series) id, so the other side
// has to read its own `currentMatchId` / `currentSeriesId`. That matters while WAITING in the lobby,
// and again on a RELOAD after being paired — the second loses the waiting flag with the page, and
// was a dead end until the check was moved to run on every mount.
//
// The modes: the lobby sends intent (`ModeChoice`) and relays what the server said. Its verdict is
// the shared validator's, as UX; a refusal is shown in the server's words.

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { DECK_SIZE } from "@jackioh/engine/config";
import { newestShippedSet, type CardDef, type CardDefs } from "@jackioh/shared";
import { validateDeck, validateTrio, type LoadoutResult } from "@jackioh/validator";

import { DECK_NAME_MAX_LENGTH, MAX_SAVED_DECKS, MAX_SAVED_TRIOS } from "@jackioh/server-config";
import { readQueued, rememberQueued } from "../net/liveGame.ts";
import {
  ApiRequestError,
  createRoom,
  dequeue,
  enqueue,
  getCatalog,
  getCollection,
  getDecks,
  getMe,
  getOwnRank,
  getPopulation,
  joinRoom,
  type DecksResponse,
  type EnqueueResponse,
  type OwnRankResponse,
  type QueueMode,
  type SavedDeck,
  type SavedTrio,
} from "../net/api.ts";
import { navigate, paths } from "../net/navigate.ts";
import { ROOM_LINK_SHARE_TITLE, ROOM_LINK_STORAGE_KEY } from "../net/roomLink.ts";
import { PLAY_LEAN_NEWEST_KEY, leanNewestLabel } from "../game/LeanNewest.tsx";
import PlayRoute, {
  MATCH_FOUND_STATUS,
  MODE_LABEL,
  PLAY_CHOICE_KEY,
  pairTargetOf,
  playModeTestid,
  playTestid,
  queuedStatus,
  roomLinkStatus,
} from "./play.tsx";

vi.mock("../net/api.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/api.ts")>();
  return {
    ...actual,
    getMe: vi.fn(),
    enqueue: vi.fn(),
    dequeue: vi.fn(),
    createRoom: vi.fn(),
    joinRoom: vi.fn(),
    getDecks: vi.fn(),
    getCatalog: vi.fn(),
    getCollection: vi.fn(),
    getPopulation: vi.fn(),
    getOwnRank: vi.fn(),
  };
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

// ---------------------------------------------------------------------------------------------
// fixtures: a catalog, a collection that owns one of everything, four decks and two trios
// ---------------------------------------------------------------------------------------------

function cardId(n: number): string {
  return `core-${String(n).padStart(3, "0")}`;
}

/** DECK_SIZE distinct ids from `from`. */
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

/**
 * The validator's own sentences for a verdict. They are computed, never typed out: the lobby shows
 * `@jackioh/validator`'s words verbatim, and `messages.test.ts` fails any client file that carries a
 * copy of them.
 */
function messagesOf(result: LoadoutResult): string[] {
  return result.ok ? [] : result.errors.map((error) => error.message);
}

function openTicket(mode: QueueMode): EnqueueResponse {
  return { ticketId: "tk-1", status: "open", matchId: null, seriesId: null, population: 1, mode };
}

/** `GET /api/ranked`'s answer: a Normal Grape III with one pip, unless a test overrides it. */
function rankBody(over: Partial<OwnRankResponse> = {}): OwnRankResponse {
  return {
    season: "v0.2",
    tag: "ABC123",
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
  vi.mocked(getCatalog).mockResolvedValue({ version: "v1", defs: DEFS });
  vi.mocked(getCollection).mockResolvedValue({
    catalogVersion: "v1",
    entries: ALL_IDS.map((id) => ({ cardId: id, quantity: 1 })),
  });
  vi.mocked(getPopulation).mockResolvedValue({ population: 3, byMode: { bo1: 2, bo3: 0, random: 1 } });
  vi.mocked(getOwnRank).mockResolvedValue(rankBody());
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

// ---------------------------------------------------------------------------------------------
// the watch
// ---------------------------------------------------------------------------------------------

describe("the lobby's match watch", () => {
  /**
   * The reload case. `waiting` is component state, so a refresh clears it; before the check ran on
   * mount, this player sat in the lobby while their opponent was already on the board.
   */
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

describe("the lobby's way to practice", () => {
  /** /practice needs no account; the lobby links it so a player can find it without typing the URL. */
  it("links to /practice", async () => {
    const { getByTestId } = render(<PlayRoute token={TOKEN} />);

    expect(getByTestId(playTestid.practice)).toHaveAttribute("href", paths.practice);
    expect(paths.practice).toBe("/practice");
    await waitFor(() => {
      expect(vi.mocked(getMe)).toHaveBeenCalled();
    });
  });
});

describe("R661 the lobby's rank", () => {
  it("R661 shows the player's own rank in R612's words with the season, and links the leaderboard", async () => {
    await renderLobby();

    expect(await screen.findByTestId(playTestid.rank)).toHaveTextContent(
      "Normal Grape III · 1/3 pips · Season v0.2",
    );
    const link = screen.getByTestId(playTestid.leaderboard);
    expect(link).toHaveAttribute("href", paths.leaderboard);
    // A panel of its own beside the queue, never inside the Find a match box (R505).
    const box = screen.getByRole("region", { name: "Find a match" });
    expect(within(box).queryByTestId(playTestid.rank)).toBeNull();
    expect(within(box).queryByTestId(playTestid.leaderboard)).toBeNull();
  });

  it("R661 puts R612's words on placements and a Jlorious position too, and never a rating", async () => {
    vi.mocked(getOwnRank).mockResolvedValue(
      rankBody({ rank: { tier: "raisin", placementsPlayed: 4, placementGames: 10 } }),
    );
    const first = render(<PlayRoute token={TOKEN} />);
    expect(await screen.findByTestId(playTestid.rank)).toHaveTextContent("Raisin · 4/10 placements");
    first.unmount();

    vi.mocked(getOwnRank).mockResolvedValue(rankBody({ rank: { tier: "jlorious", position: 42 } }));
    render(<PlayRoute token={TOKEN} />);
    const rank = await screen.findByTestId(playTestid.rank);
    expect(rank).toHaveTextContent("Jlorious #42 · Season v0.2");
    expect(rank.textContent).not.toMatch(/rating|elo/i);
  });

  it("R661 a rank that cannot be read shows nothing there, and the link stays", async () => {
    vi.mocked(getOwnRank).mockRejectedValue(new Error("network down"));
    await renderLobby();

    expect(screen.queryByTestId(playTestid.rank)).toBeNull();
    expect(screen.getByTestId(playTestid.leaderboard)).toHaveAttribute("href", paths.leaderboard);
  });
});

// ---------------------------------------------------------------------------------------------
// modes and choices (R257)
// ---------------------------------------------------------------------------------------------

describe("the lobby's modes", () => {
  it("R257 the mode picker offers Best of 1, Conquest and All Random, each with its own choice", async () => {
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
    await screen.findByText(/In the All Random queue/);
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

  it("a player with no decks is sent to /decks, and has nothing to queue a Best of 1 with", async () => {
    vi.mocked(getDecks).mockResolvedValue(decksAnswer([], []));
    render(<PlayRoute token={TOKEN} />);

    const link = await screen.findByTestId(playTestid.decksLink);
    expect(link).toHaveAttribute("href", "/decks");
    expect(screen.getByTestId(playTestid.queue)).toBeDisabled();

    // All Random needs no deck at all.
    pickMode("random");
    expect(screen.getByTestId(playTestid.queue)).not.toBeDisabled();
  });

  it("shows the queue's population per mode", async () => {
    await renderLobby();
    const population = await screen.findByTestId(playTestid.population);
    await waitFor(() => {
      expect(population).toHaveAttribute("data-bo1", "2");
    });
    expect(population).toHaveAttribute("data-bo3", "0");
    expect(population).toHaveAttribute("data-random", "1");
  });

  it("R433 All Random names no card of the deck it will deal: that deck is dealt when the game starts", async () => {
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

  it("R505 the queue's counts are on the mode tiles alone: the Find a match box and the queued notice repeat none", async () => {
    vi.mocked(enqueue).mockImplementation((_token, choice) => Promise.resolve({ ...openTicket(choice.mode), population: 7 }));
    await renderLobby();
    const tiles = screen.getByTestId(playTestid.population);
    await waitFor(() => {
      expect(tiles).toHaveAttribute("data-bo1", "2");
    });
    // Each tile says how many are waiting for its mode.
    const tileOf = (mode: QueueMode): HTMLElement | null => screen.getByTestId(playModeTestid(mode)).closest("label");
    expect(tileOf("bo1")).toHaveTextContent("2 waiting");
    expect(tileOf("bo3")).toHaveTextContent("0 waiting");
    expect(tileOf("random")).toHaveTextContent("1 waiting");

    const box = screen.getByRole("region", { name: "Find a match" });
    expect(box).not.toHaveTextContent(/waiting/i);
    expect(within(box).queryByTestId(playTestid.population)).toBeNull();

    // Queued: the box says what it is doing and the notice where you are, and neither counts.
    fireEvent.click(screen.getByTestId(playTestid.queue));
    const status = await screen.findByTestId(playTestid.status);
    expect(status).toHaveTextContent(`In the ${MODE_LABEL.bo1} queue.`);
    expect(status).toHaveTextContent("You will be taken to the game as soon as someone is found.");
    expect(status).not.toHaveTextContent(/waiting|7/);
    expect(within(box).getByTestId(playTestid.searching)).toHaveTextContent(`Looking for a ${MODE_LABEL.bo1} opponent`);
    expect(box).not.toHaveTextContent(/waiting/i);
    // The tiles still count.
    expect(tileOf("bo1")).toHaveTextContent("waiting");
  });
});

// ---------------------------------------------------------------------------------------------
// answers and refusals
// ---------------------------------------------------------------------------------------------

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

  it("a join that makes a series goes to the series screen", async () => {
    vi.mocked(joinRoom).mockResolvedValueOnce({ matchId: null, seriesId: "series-2", code: "ABCD", seat: "p2", mode: "bo3" });
    await renderLobby();
    pickMode("bo3");
    fireEvent.change(screen.getByTestId(playTestid.joinInput), { target: { value: "abcd" } });
    fireEvent.submit(screen.getByTestId(playTestid.joinForm));

    await waitFor(
      () => {
        expect(vi.mocked(navigate)).toHaveBeenCalledWith("/series/series-2");
      },
      { timeout: 5000 },
    );
    expect(vi.mocked(joinRoom)).toHaveBeenCalledWith(TOKEN, "ABCD", { mode: "bo3", trioId: MAIN.id });
  });

  it("a join that makes a match goes to the board", async () => {
    vi.mocked(joinRoom).mockResolvedValueOnce({ matchId: "match-3", seriesId: null, code: "ABCD", seat: "p2", mode: "random" });
    await renderLobby();
    pickMode("random");
    fireEvent.change(screen.getByTestId(playTestid.joinInput), { target: { value: "abcd" } });
    fireEvent.submit(screen.getByTestId(playTestid.joinForm));

    await waitFor(
      () => {
        expect(vi.mocked(navigate)).toHaveBeenCalledWith("/match/match-3");
      },
      { timeout: 5000 },
    );
  });

  it("R264 a join refused for another mode switches to the room's mode and says what to pick", async () => {
    vi.mocked(joinRoom).mockRejectedValue(
      new ApiRequestError(409, {
        code: "conflict",
        message: "This room plays Conquest: pick one of your trios.",
        details: { mode: "bo3" },
      }),
    );
    await renderLobby();
    fireEvent.change(screen.getByTestId(playTestid.joinInput), { target: { value: "wxyz" } });
    fireEvent.submit(screen.getByTestId(playTestid.joinForm));

    const error = await screen.findByTestId(playTestid.error);
    expect(error).toHaveTextContent(`This room plays ${MODE_LABEL.bo3}: pick a trio and join again.`);
    expect(screen.getByTestId(playModeTestid("bo3"))).toBeChecked();
    expect(screen.getByTestId(playTestid.trioSelect)).toBeInTheDocument();
    // The code is kept, so joining again is one press.
    expect(screen.getByTestId(playTestid.joinInput)).toHaveValue("wxyz");
  });

  it("R264 Create a room sends the choice and shows the code with the room's mode", async () => {
    vi.mocked(createRoom).mockResolvedValue({ code: "QRST", expiresAt: 0, mode: "random" });
    await renderLobby();
    pickMode("random");
    fireEvent.click(screen.getByTestId(playTestid.createRoom));

    expect(await screen.findByTestId(playTestid.roomCode)).toHaveTextContent("QRST");
    expect(vi.mocked(createRoom)).toHaveBeenCalledWith(TOKEN, { mode: "random" });
    const roomMode = screen.getByTestId(playTestid.roomMode);
    expect(roomMode).toHaveAttribute("data-mode", "random");
    expect(roomMode).toHaveTextContent("All Random");
  });
});

// ---------------------------------------------------------------------------------------------
// R1372: All Random's "More cards from the newest set"
// ---------------------------------------------------------------------------------------------

describe("R1372 All Random's more cards from the newest set", () => {
  it("R1372 the switch is All Random's alone, names the newest set and starts off", async () => {
    await renderLobby();
    expect(screen.queryByTestId(playTestid.leanNewest)).toBeNull();
    pickMode("random");
    const toggle = screen.getByTestId(playTestid.leanNewest);
    expect(toggle).not.toBeChecked();
    expect(toggle.closest("label")).toHaveTextContent(leanNewestLabel());
    expect(leanNewestLabel()).toBe(`More cards from the newest set (${newestShippedSet()})`);
    pickMode("bo3");
    expect(screen.queryByTestId(playTestid.leanNewest)).toBeNull();
  });

  /** The lobby in All Random, whichever mode this device remembered, once the switch is on screen. */
  async function renderRandomLobby(): Promise<HTMLInputElement> {
    render(<PlayRoute token={TOKEN} />);
    await waitFor(() => {
      expect(vi.mocked(getDecks)).toHaveBeenCalled();
    });
    const random = screen.getByTestId(playModeTestid("random")) as HTMLInputElement;
    if (!random.checked) pickMode("random");
    return (await screen.findByTestId(playTestid.leanNewest)) as HTMLInputElement;
  }

  it("R1372 on, Find a match, Create a room and Join send leanNewest; off, they send none", async () => {
    vi.mocked(enqueue).mockImplementation((_token, choice) => Promise.resolve(openTicket(choice.mode)));
    vi.mocked(createRoom).mockResolvedValue({ code: "QRST", expiresAt: 0, mode: "random" });
    vi.mocked(joinRoom).mockResolvedValue({ matchId: "match-9", seriesId: null, code: "ABCD", seat: "p2", mode: "random" });
    fireEvent.click(await renderRandomLobby());
    expect(screen.getByTestId(playTestid.leanNewest)).toBeChecked();

    fireEvent.click(screen.getByTestId(playTestid.queue));
    await waitFor(() => {
      expect(vi.mocked(enqueue)).toHaveBeenLastCalledWith(TOKEN, { mode: "random", leanNewest: true });
    });
    // Locked while queued, like the rest of the setup.
    expect(screen.getByTestId(playTestid.leanNewest)).toBeDisabled();
    fireEvent.click(screen.getByTestId(playTestid.leaveQueue));
    await waitFor(() => {
      expect(screen.getByTestId(playTestid.queue)).not.toBeDisabled();
    });

    // Off, a room is made with no lean at all.
    fireEvent.click(screen.getByTestId(playTestid.leanNewest));
    fireEvent.click(screen.getByTestId(playTestid.createRoom));
    await screen.findByTestId(playTestid.roomCode);
    expect(vi.mocked(createRoom)).toHaveBeenLastCalledWith(TOKEN, { mode: "random" });
    cleanup();

    // On again, a join carries it.
    fireEvent.click(await renderRandomLobby());
    fireEvent.change(screen.getByTestId(playTestid.joinInput), { target: { value: "abcd" } });
    fireEvent.submit(screen.getByTestId(playTestid.joinForm));
    await waitFor(() => {
      expect(vi.mocked(joinRoom)).toHaveBeenLastCalledWith(TOKEN, "ABCD", { mode: "random", leanNewest: true });
    });
  });

  it("R1372 the pick is kept on this device and a refused storage only starts it off", async () => {
    fireEvent.click(await renderRandomLobby());
    expect(window.localStorage.getItem(PLAY_LEAN_NEWEST_KEY)).toBe("true");
    cleanup();

    expect(await renderRandomLobby()).toBeChecked();
    cleanup();

    const getItem = vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    const setItem = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    try {
      const toggle = await renderRandomLobby();
      expect(toggle).not.toBeChecked();
      fireEvent.click(toggle);
      expect(screen.getByTestId(playTestid.leanNewest)).toBeChecked();
    } finally {
      getItem.mockRestore();
      setItem.mockRestore();
    }
  });
});

// ---------------------------------------------------------------------------------------------
// queue state: locked while queued, announced when paired
// ---------------------------------------------------------------------------------------------

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
    expect(screen.getByTestId(playTestid.createRoom)).toBeDisabled();
    expect(screen.getByTestId(playTestid.joinInput)).toBeDisabled();
    expect(screen.getByTestId(playTestid.joinSubmit)).toBeDisabled();
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

  it("a join that pairs at once announces before navigating", async () => {
    vi.mocked(joinRoom).mockResolvedValue({ matchId: "match-8", seriesId: null, code: "ABCD", seat: "p2", mode: "bo1" });
    await renderLobby();
    fireEvent.change(screen.getByTestId(playTestid.joinInput), { target: { value: "abcd" } });
    fireEvent.submit(screen.getByTestId(playTestid.joinForm));

    expect(await screen.findByTestId(playTestid.status)).toHaveTextContent(MATCH_FOUND_STATUS);
    expect(vi.mocked(navigate)).not.toHaveBeenCalled();
    await waitFor(
      () => {
        expect(vi.mocked(navigate)).toHaveBeenCalledWith("/match/match-8");
      },
      { timeout: 5000 },
    );
  });
});

// ---------------------------------------------------------------------------------------------
// R767: a room shared as a link
// ---------------------------------------------------------------------------------------------

describe("R767 a room shared as a link", () => {
  afterEach(() => {
    window.history.replaceState(null, "", "/");
    window.sessionStorage.clear();
    Reflect.deleteProperty(navigator, "clipboard");
    Reflect.deleteProperty(navigator, "share");
  });

  /** Opens `/play` on `search`, as a visitor who followed the link does, and renders the lobby. */
  function openLink(search: string): void {
    window.history.replaceState(null, "", `${paths.play}${search}`);
    render(<PlayRoute token={TOKEN} />);
  }

  function stubClipboard(writeText: () => Promise<void>): ReturnType<typeof vi.fn> {
    const spy = vi.fn(writeText);
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: spy } });
    return spy;
  }

  async function createRandomRoom(): Promise<void> {
    vi.mocked(createRoom).mockResolvedValue({ code: "QRSTUV", expiresAt: 0, mode: "random" });
    await renderLobby();
    pickMode("random");
    fireEvent.click(screen.getByTestId(playTestid.createRoom));
    await screen.findByTestId(playTestid.roomCode);
  }

  /** Lets a settled clipboard or share promise reach its handler. */
  async function settle(): Promise<void> {
    await act(async () => {
      await new Promise<void>((resolve) => setTimeout(resolve, 0));
    });
  }

  it("R767 /play?room=abc234&mode=bo1 fills in ABC234, chooses Best of 1, says what to pick, focuses the deck and sends no join", async () => {
    // The last choice was Conquest, so the switch to Best of 1 is the link's doing.
    window.localStorage.setItem(PLAY_CHOICE_KEY, JSON.stringify({ mode: "bo3" }));
    openLink("?room=abc234&mode=bo1");

    const select = await screen.findByTestId(playTestid.deckSelect);
    expect(screen.getByTestId(playTestid.joinInput)).toHaveValue("ABC234");
    expect(screen.getByTestId(playModeTestid("bo1"))).toBeChecked();
    expect(screen.getByTestId(playTestid.status)).toHaveTextContent(roomLinkStatus("bo1"));
    expect(roomLinkStatus("bo1")).toBe("Pick a deck for Best of 1, then press Join.");
    await waitFor(() => {
      expect(select).toHaveFocus();
    });
    expect(vi.mocked(joinRoom)).not.toHaveBeenCalled();
    expect(screen.queryByTestId(playTestid.error)).toBeNull();
    // The link is gone from the address bar and from this tab, so a reload or Back refills nothing.
    expect(window.location.pathname).toBe(paths.play);
    expect(window.location.search).toBe("");
    expect(window.sessionStorage.getItem(ROOM_LINK_STORAGE_KEY)).toBeNull();
  });

  it("R767 a Conquest link focuses the trio and an All Random one focuses Join", async () => {
    openLink("?room=abc234&mode=bo3");
    const trioSelect = await screen.findByTestId(playTestid.trioSelect);
    expect(screen.getByTestId(playModeTestid("bo3"))).toBeChecked();
    expect(screen.getByTestId(playTestid.status)).toHaveTextContent("Pick a trio for Conquest, then press Join.");
    await waitFor(() => {
      expect(trioSelect).toHaveFocus();
    });
    cleanup();
    window.sessionStorage.clear();

    openLink("?room=wxyz23&mode=random");
    expect(screen.getByTestId(playTestid.joinInput)).toHaveValue("WXYZ23");
    expect(screen.getByTestId(playModeTestid("random"))).toBeChecked();
    expect(screen.getByTestId(playTestid.status)).toHaveTextContent("No deck needed for All Random: press Join.");
    await waitFor(() => {
      expect(screen.getByTestId(playTestid.joinSubmit)).toHaveFocus();
    });
    expect(vi.mocked(joinRoom)).not.toHaveBeenCalled();
  });

  it("R767 a link with no usable mode fills in the code and leaves the last choice", async () => {
    window.localStorage.setItem(PLAY_CHOICE_KEY, JSON.stringify({ mode: "bo3" }));
    openLink("?room=abc234&mode=bo5");
    await screen.findByTestId(playTestid.trioSelect);
    expect(screen.getByTestId(playTestid.joinInput)).toHaveValue("ABC234");
    expect(screen.getByTestId(playModeTestid("bo3"))).toBeChecked();
    expect(screen.getByTestId(playTestid.status)).toHaveTextContent(roomLinkStatus("bo3"));
  });

  it("R767 a code that is no room code is ignored without a word", async () => {
    // abc123 has a 1 (R104's alphabet has none), abc23 is short and abc2345 is long.
    for (const code of ["abc123", "abc23", "abc2345"]) {
      openLink(`?room=${code}&mode=bo3`);
      await screen.findByTestId(playTestid.deckSelect);
      expect(screen.getByTestId(playTestid.joinInput)).toHaveValue("");
      expect(screen.getByTestId(playModeTestid("bo1"))).toBeChecked();
      expect(screen.queryByTestId(playTestid.status)).toBeNull();
      expect(screen.queryByTestId(playTestid.error)).toBeNull();
      expect(window.location.search).toBe("");
      expect(window.sessionStorage.getItem(ROOM_LINK_STORAGE_KEY)).toBeNull();
      cleanup();
    }
    expect(vi.mocked(joinRoom)).not.toHaveBeenCalled();
  });

  it("R767 the link's mode is a hint: R264's refusal still switches to the room's", async () => {
    vi.mocked(joinRoom).mockRejectedValue(
      new ApiRequestError(409, {
        code: "conflict",
        message: "This room plays Conquest: pick one of your trios.",
        details: { mode: "bo3" },
      }),
    );
    openLink("?room=abc234&mode=bo1");
    await screen.findByTestId(playTestid.deckSelect);
    fireEvent.submit(screen.getByTestId(playTestid.joinForm));

    const error = await screen.findByTestId(playTestid.error);
    expect(vi.mocked(joinRoom)).toHaveBeenCalledWith(TOKEN, "ABC234", { mode: "bo1", deckId: AGGRO.id });
    expect(error).toHaveTextContent(`This room plays ${MODE_LABEL.bo3}: pick a trio and join again.`);
    expect(screen.getByTestId(playModeTestid("bo3"))).toBeChecked();
    // The notice named the link's mode; it must not sit beside an error that says the opposite.
    expect(screen.queryByTestId(playTestid.status)).toBeNull();
    // The code stays, so joining again is one press.
    expect(screen.getByTestId(playTestid.joinInput)).toHaveValue("ABC234");
  });

  it("R767 the link's notice goes when the player picks another mode", async () => {
    openLink("?room=abc234&mode=bo1");
    await screen.findByTestId(playTestid.deckSelect);
    expect(screen.getByTestId(playTestid.status)).toHaveTextContent(roomLinkStatus("bo1"));
    fireEvent.click(screen.getByTestId(playModeTestid("bo3")));
    expect(screen.queryByTestId(playTestid.status)).toBeNull();
  });

  it("R767 a lobby still queued keeps its queue's mode and notice; the link fills in only the code", async () => {
    rememberQueued("bo3");
    openLink("?room=abc234&mode=bo1");
    await screen.findByTestId(playTestid.trioSelect);
    expect(screen.getByTestId(playModeTestid("bo3"))).toBeChecked();
    expect(screen.getByTestId(playTestid.status)).toHaveTextContent(queuedStatus("bo3"));
    expect(screen.getByTestId(playTestid.joinInput)).toHaveValue("ABC234");
    expect(screen.getByTestId(playTestid.trioSelect)).not.toHaveFocus();
    expect(window.location.search).toBe("");
  });

  it("R767 Copy invite link puts the full link on the clipboard where there is no share sheet", async () => {
    const writeText = stubClipboard(() => Promise.resolve());
    await createRandomRoom();
    const button = screen.getByTestId(playTestid.copyRoomLink);
    expect(button).toHaveTextContent("Copy invite link");
    fireEvent.click(button);

    await waitFor(() => {
      expect(button).toHaveTextContent("Link copied");
    });
    expect(writeText).toHaveBeenCalledTimes(1);
    expect(writeText).toHaveBeenCalledWith(`${window.location.origin}/play?room=QRSTUV&mode=random`);
    // The code's own button is a separate one and is untouched.
    expect(screen.getByTestId(playTestid.copyRoomCode)).toHaveTextContent("Copy");
  });

  it("R767 a refused clipboard leaves both copy buttons as they were", async () => {
    const writeText = stubClipboard(() => Promise.reject(new Error("denied")));
    await createRandomRoom();
    fireEvent.click(screen.getByTestId(playTestid.copyRoomLink));
    fireEvent.click(screen.getByTestId(playTestid.copyRoomCode));
    await waitFor(() => {
      expect(writeText).toHaveBeenCalledTimes(2);
    });
    await settle();

    expect(screen.getByTestId(playTestid.copyRoomLink)).toHaveTextContent("Copy invite link");
    expect(screen.getByTestId(playTestid.copyRoomCode)).toHaveTextContent("Copy");
    expect(screen.getByTestId(playTestid.copyRoomCode)).not.toHaveTextContent("Copied");
    expect(screen.queryByTestId(playTestid.error)).toBeNull();
  });

  it("R767 no clipboard at all (an insecure origin) leaves the buttons as they were", async () => {
    await createRandomRoom();
    fireEvent.click(screen.getByTestId(playTestid.copyRoomLink));
    await settle();
    expect(screen.getByTestId(playTestid.copyRoomLink)).toHaveTextContent("Copy invite link");
    expect(screen.queryByTestId(playTestid.error)).toBeNull();
  });

  it("R767 Copy invite link opens the share sheet where there is one", async () => {
    const writeText = stubClipboard(() => Promise.resolve());
    const share = vi.fn(() => Promise.resolve());
    Object.defineProperty(navigator, "share", { configurable: true, value: share });
    await createRandomRoom();
    fireEvent.click(screen.getByTestId(playTestid.copyRoomLink));

    await waitFor(() => {
      expect(screen.getByTestId(playTestid.copyRoomLink)).toHaveTextContent("Link shared");
    });
    expect(share).toHaveBeenCalledWith({
      title: ROOM_LINK_SHARE_TITLE,
      url: `${window.location.origin}/play?room=QRSTUV&mode=random`,
    });
    expect(writeText).not.toHaveBeenCalled();
  });

  it("R767 a share sheet closed without sharing changes nothing", async () => {
    const writeText = stubClipboard(() => Promise.resolve());
    const share = vi.fn(() => Promise.reject(new DOMException("", "AbortError")));
    Object.defineProperty(navigator, "share", { configurable: true, value: share });
    await createRandomRoom();
    fireEvent.click(screen.getByTestId(playTestid.copyRoomLink));
    await waitFor(() => {
      expect(share).toHaveBeenCalledTimes(1);
    });
    await settle();

    expect(screen.getByTestId(playTestid.copyRoomLink)).toHaveTextContent("Copy invite link");
    expect(writeText).not.toHaveBeenCalled();
    expect(screen.queryByTestId(playTestid.error)).toBeNull();
  });
});
