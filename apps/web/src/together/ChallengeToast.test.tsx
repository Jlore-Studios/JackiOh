// #936 S8 B57: `ChallengeToast`, mounted once in the shell. While signed in it reads
// `GET /api/challenges`; an incoming challenge shows a toast naming the challenger, whose Accept joins
// the room by its code with the deck picked in the toast and goes to the game (R1831000), with the
// lobby's room link (R767) as the way in when the join is refused, and whose Decline declines the
// challenge.
//
// SPEC S8 names no test id for the toast, so its buttons are found by their role and name, or by
// the ids #1831 gave its Accept, deck picker and lobby link. R2031000: an All Random challenge's
// toast offers "More cards from" on the device's online pick, and Accept joins with it.
// R915005: a deck with a custom card, or a trio holding one, is listed off and never picked.

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { LEAN_NEWEST } from "@jackioh/shared";

import { PLAY_LEAN_SET_KEY, readPlayLean } from "../game/LeanNewest.tsx";
import { getDecks } from "../net/api.ts";
import { guardLeaving } from "../net/navigate.ts";
import { forgetRoomLink, readRoomLink } from "../net/roomLink.ts";
import type { Challenge, PlayerRef } from "../net/social.ts";
import { skeletonTestid } from "../skeleton/Skeleton.tsx";
import { settle } from "../test/settle.ts";
import { expectSkeleton } from "../test/skeleton.ts";
import { at, meBody, must, refusal, signIn, stubServer, type Call, type Reply, type StubServer } from "../test/stubServer.ts";
import { CHALLENGE_CHOICE_KEY, readChallengeChoice } from "./challengeChoice.ts";
import { ChallengeToast } from "./ChallengeToast.tsx";
import { SOCIAL_POLL_MS } from "./config.ts";
import { challengeToastTestid } from "./testids.ts";

// #2115: the decks read passes through to the real one, except while a test holds it (`holdDecks`).
vi.mock("../net/api.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/api.ts")>();
  return { ...actual, getDecks: vi.fn(actual.getDecks) };
});

const { App, resetUsernamePromptForTests } = await import("../main.tsx");

/** SPEC S8 gives the toast's poll no interval of its own; a first read comes well inside this. */
const SOON = { timeout: 8_000 } as const;
const TEST_MS = 15_000;

const ME: PlayerRef = { profileId: "profile-1", username: "Me#1" };
const ANN: PlayerRef = { profileId: "f-ann", username: "Ann#2" };
const BOB: PlayerRef = { profileId: "f-bob", username: "Bob#3" };

const INCOMING: Challenge = {
  challengeId: "ch-1",
  from: ANN,
  to: ME,
  roomCode: "HJK234",
  mode: "bo1",
  expiresAt: "2099-01-01T00:00:00.000Z",
};

const OUTGOING: Challenge = {
  challengeId: "ch-2",
  from: ME,
  to: BOB,
  roomCode: "MNP567",
  mode: "random",
  expiresAt: "2099-01-01T00:00:00.000Z",
};

type Challenges = { incoming: Challenge[]; outgoing: Challenge[] };

const DECK = { catalogVersion: "v1", portrait: null, createdAt: 0, updatedAt: 0 } as const;
const AGGRO = { ...DECK, id: "d-aggro", name: "Aggro", cards: [] };
const CONTROL = { ...DECK, id: "d-control", name: "Control", cards: [] };
const TRIO = { id: "t-one", name: "Trio One", deckIds: ["d-aggro", "d-control", null], createdAt: 0, updatedAt: 0 };

// R915005: Imps holds a card from the creator; Core holds none.
const IMPS = { ...DECK, id: "d-imps", name: "Imps", cards: ["custom-ember-imp"] };
const CORE = { ...DECK, id: "d-core", name: "Core", cards: [] };
const IMP_TRIO = { id: "t-imps", name: "Imp trio", deckIds: ["d-core", "d-imps", "d-core"], createdAt: 0, updatedAt: 0 };
const CORE_TRIO = { id: "t-core", name: "Core trio", deckIds: ["d-core", "d-core", "d-core"], createdAt: 0, updatedAt: 0 };

function decksBody(decks: readonly object[] = [AGGRO, CONTROL], trios: readonly object[] = [TRIO]): object {
  return { catalogVersion: "v1", decks, trios, limits: { decks: 20, trios: 10, nameLength: 40 } };
}

/** What the stub's room join answers: the match the player lands in. */
const JOINED_MATCH = "m-9";

function serve(challenges: Challenges, special?: (call: Call) => Reply | undefined): StubServer {
  return stubServer((call) => {
    const answer = special?.(call);
    if (answer !== undefined) return answer;
    if (call.path === "/api/auth/me") {
      return { status: 200, body: meBody({ profile: { id: ME.profileId, status: "active" }, email: "me@example.test" }) };
    }
    if (call.method === "GET" && call.path === "/api/challenges") return { status: 200, body: challenges };
    if (call.method === "GET" && call.path === "/api/decks") return { status: 200, body: decksBody() };
    if (call.method === "POST" && /^\/api\/rooms\/[^/]+\/join$/.test(call.path)) {
      const code = call.path.split("/")[3] ?? "";
      const mode = (call.body as { mode?: string } | undefined)?.mode ?? "bo1";
      return { status: 200, body: { matchId: JOINED_MATCH, seriesId: null, code, seat: "p2", mode } };
    }
    if (call.method === "POST" && call.path.endsWith("/decline")) {
      challenges.incoming = challenges.incoming.filter((challenge) => !call.path.includes(challenge.challengeId));
      return { status: 204 };
    }
    if (call.method === "POST" && call.path === "/api/presence") return { status: 204 };
    if (call.method === "GET" && call.path === "/api/friends") {
      return { status: 200, body: { friends: [], incoming: [], outgoing: [], blocked: [] } };
    }
    if (call.method === "GET" && call.path === "/api/chat/unread") return { status: 200, body: { unread: {} } };
    if (call.method === "GET" && call.path === "/api/duos/live") return { status: 200, body: { matchId: null } };
    return undefined;
  });
}

function acceptButtons(): HTMLElement[] {
  return screen.queryAllByRole("button", { name: /^accept/i });
}

const realApi = await vi.importActual<typeof import("../net/api.ts")>("../net/api.ts");

/** The decks reads `holdDecks` has kept from answering, each one's release making the real read. */
let heldDecks: (() => void)[] = [];

/** Every decks read from now on waits for `releaseDecks` (#2115), so the toast can be seen while it is pending. */
function holdDecks(): void {
  vi.mocked(getDecks).mockImplementation(
    (...args) =>
      new Promise((resolve, reject) => {
        heldDecks.push(() => {
          realApi.getDecks(...args).then(resolve, reject);
        });
      }),
  );
}

/** Answers the reads held back, and every read after them, as the real one does. */
function releaseDecks(): void {
  vi.mocked(getDecks).mockImplementation(realApi.getDecks);
  const releases = heldDecks;
  heldDecks = [];
  for (const release of releases) release();
}

beforeEach(() => {
  window.localStorage.clear();
  forgetRoomLink();
  resetUsernamePromptForTests();
  at("/decks");
  heldDecks = [];
  vi.mocked(getDecks).mockImplementation(realApi.getDecks);
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  forgetRoomLink();
  window.localStorage.clear();
});

describe("B57 the challenge toast", () => {
  it(
    "B57 an incoming challenge shows a toast naming the challenger, with Accept and Decline",
    async () => {
      signIn("tok-toast");
      serve({ incoming: [INCOMING], outgoing: [] });
      render(<ChallengeToast />);
      expect(await screen.findByRole("button", { name: /^accept/i }, SOON)).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /^decline/i })).toBeInTheDocument();
      expect(document.body).toHaveTextContent(ANN.username);
    },
    TEST_MS,
  );

  it(
    "#2115 the toast's decks are a panel skeleton while they are read, then the deck picker",
    async () => {
      signIn("tok-toast");
      serve({ incoming: [INCOMING], outgoing: [] });
      holdDecks();
      render(<ChallengeToast />);
      const toast = await screen.findByTestId(challengeToastTestid.toast, undefined, SOON);
      expectSkeleton(within(toast).getByText("Loading your decks…"), "panel");
      expect(within(toast).queryByTestId(challengeToastTestid.deck)).toBeNull();

      releaseDecks();
      expect(await within(toast).findByTestId(challengeToastTestid.deck, undefined, SOON)).toBeInTheDocument();
      expect(within(toast).queryByText("Loading your decks…")).toBeNull();
      expect(within(toast).queryByTestId(skeletonTestid.root)).toBeNull();
    },
    TEST_MS,
  );

  it(
    "R1831000 Accept joins with the remembered deck and goes to /match/m-9",
    async () => {
      signIn("tok-toast");
      window.localStorage.setItem(CHALLENGE_CHOICE_KEY, JSON.stringify({ mode: "bo3", deckId: "d-control", trioId: null }));
      const server = serve({ incoming: [INCOMING], outgoing: [] });
      render(<ChallengeToast />);
      const picker = await screen.findByTestId(challengeToastTestid.deck, undefined, SOON);
      expect((picker as HTMLSelectElement).value).toBe("d-control");
      expect([...(picker as HTMLSelectElement).options].map((option) => option.textContent)).toEqual(["Aggro", "Control"]);

      fireEvent.click(screen.getByTestId(challengeToastTestid.accept));
      await waitFor(() => {
        expect(window.location.pathname).toBe(`/match/${JOINED_MATCH}`);
      }, SOON);
      const joins = server.to("POST", `/api/rooms/${INCOMING.roomCode}/join`);
      expect(joins).toHaveLength(1);
      expect(joins[0]?.body).toEqual({ mode: "bo1", deckId: "d-control" });
      expect(screen.queryAllByTestId(challengeToastTestid.toast)).toHaveLength(0);
    },
    TEST_MS,
  );

  it(
    "R1831000 Accept declines nothing and joins once",
    async () => {
      signIn("tok-toast");
      const server = serve({ incoming: [INCOMING], outgoing: [] });
      render(<ChallengeToast />);
      const accept = await screen.findByTestId(challengeToastTestid.accept, undefined, SOON);
      fireEvent.click(accept);
      fireEvent.click(accept);
      await waitFor(() => {
        expect(window.location.pathname).toBe(`/match/${JOINED_MATCH}`);
      }, SOON);
      expect(server.to("POST", "/api/challenges/ch-1/decline")).toHaveLength(0);
      expect(server.to("POST", `/api/rooms/${INCOMING.roomCode}/join`)).toHaveLength(1);
      // The player skipped the lobby: no room link is waiting for them there (R767).
      expect(readRoomLink()).toBeNull();
    },
    TEST_MS,
  );

  it(
    "R1831000 the deck picked in the toast is the one joined with, and is remembered for the next accept",
    async () => {
      signIn("tok-toast");
      const server = serve({ incoming: [INCOMING], outgoing: [] });
      render(<ChallengeToast />);
      const picker = await screen.findByTestId(challengeToastTestid.deck, undefined, SOON);
      // Nothing remembered: the first deck.
      expect((picker as HTMLSelectElement).value).toBe("d-aggro");
      fireEvent.change(picker, { target: { value: "d-control" } });
      fireEvent.click(screen.getByTestId(challengeToastTestid.accept));
      await waitFor(() => {
        expect(window.location.pathname).toBe(`/match/${JOINED_MATCH}`);
      }, SOON);
      expect(server.to("POST", `/api/rooms/${INCOMING.roomCode}/join`)[0]?.body).toEqual({ mode: "bo1", deckId: "d-control" });
      // Accepting remembers the deck, and leaves the mode the player challenges in alone.
      expect(readChallengeChoice()).toEqual({ mode: "bo1", deckId: "d-control", trioId: null });
    },
    TEST_MS,
  );

  it(
    "R1831000 All Random joins with the mode alone",
    async () => {
      signIn("tok-toast");
      const server = serve({ incoming: [{ ...INCOMING, mode: "random" }], outgoing: [] });
      render(<ChallengeToast />);
      fireEvent.click(await screen.findByTestId(challengeToastTestid.accept, undefined, SOON));
      await waitFor(() => {
        expect(window.location.pathname).toBe(`/match/${JOINED_MATCH}`);
      }, SOON);
      expect(server.to("POST", `/api/rooms/${INCOMING.roomCode}/join`)[0]?.body).toEqual({ mode: "random" });
      expect(screen.queryByTestId(challengeToastTestid.deck)).toBeNull();
      expect(server.to("GET", "/api/decks")).toHaveLength(0);
    },
    TEST_MS,
  );

  it(
    "R1831000 a refused join shows the server's words and Open in lobby, which fills /play's code",
    async () => {
      signIn("tok-toast");
      const server = serve({ incoming: [INCOMING], outgoing: [] }, (call) =>
        call.method === "POST" && call.path.endsWith("/join") ? refusal(409, "already_in_match", "finish your current match first") : undefined,
      );
      render(<ChallengeToast />);
      expect(screen.queryByTestId(challengeToastTestid.lobby)).toBeNull();
      fireEvent.click(await screen.findByTestId(challengeToastTestid.accept, undefined, SOON));

      const words = await screen.findByTestId(challengeToastTestid.error, undefined, SOON);
      expect(words).toHaveTextContent("finish your current match first");
      expect(window.location.pathname).toBe("/decks");
      expect(server.to("POST", `/api/rooms/${INCOMING.roomCode}/join`)).toHaveLength(1);
      // The toast stays, to try again or to open the lobby.
      expect(screen.getByTestId(challengeToastTestid.accept)).not.toBeDisabled();

      fireEvent.click(screen.getByTestId(challengeToastTestid.lobby));
      await waitFor(() => {
        expect(window.location.pathname).toBe("/play");
      }, SOON);
      expect(readRoomLink()?.code).toBe(INCOMING.roomCode);
    },
    TEST_MS,
  );

  it(
    "R1831000 with no saved deck the toast offers only Open in lobby",
    async () => {
      signIn("tok-toast");
      serve({ incoming: [INCOMING], outgoing: [] }, (call) =>
        call.method === "GET" && call.path === "/api/decks" ? { status: 200, body: decksBody([]) } : undefined,
      );
      render(<ChallengeToast />);
      const lobby = await screen.findByTestId(challengeToastTestid.lobby, undefined, SOON);
      expect(screen.queryByTestId(challengeToastTestid.accept)).toBeNull();
      expect(screen.queryByTestId(challengeToastTestid.deck)).toBeNull();

      fireEvent.click(lobby);
      await waitFor(() => {
        expect(window.location.pathname).toBe("/play");
      }, SOON);
      expect(readRoomLink()?.code).toBe(INCOMING.roomCode);
    },
    TEST_MS,
  );

  it(
    "R1831000 decks that could not be read leave Open in lobby, not an Accept with nothing to join with",
    async () => {
      signIn("tok-toast");
      serve({ incoming: [INCOMING], outgoing: [] }, (call) =>
        call.method === "GET" && call.path === "/api/decks" ? refusal(500, "internal", "The decks are out of reach.") : undefined,
      );
      render(<ChallengeToast />);
      expect(await screen.findByTestId(challengeToastTestid.lobby, undefined, SOON)).toBeInTheDocument();
      expect(screen.queryByTestId(challengeToastTestid.accept)).toBeNull();
    },
    TEST_MS,
  );

  it(
    "B57 Decline declines the challenge and the toast goes, leaving the player where they are",
    async () => {
      signIn("tok-toast");
      const server = serve({ incoming: [INCOMING], outgoing: [] });
      render(<ChallengeToast />);
      fireEvent.click(await screen.findByRole("button", { name: /^decline/i }, SOON));
      await waitFor(() => {
        expect(server.to("POST", "/api/challenges/ch-1/decline")).toHaveLength(1);
      }, SOON);
      await waitFor(() => {
        expect(acceptButtons()).toHaveLength(0);
      }, SOON);
      expect(window.location.pathname).toBe("/decks");
      expect(readRoomLink()).toBeNull();
    },
    TEST_MS,
  );

  it(
    "B57 no incoming challenge, no toast: the player's own outgoing challenge raises none",
    async () => {
      vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
      signIn("tok-toast");
      const server = serve({ incoming: [], outgoing: [OUTGOING] });
      render(<ChallengeToast />);
      await waitFor(() => {
        expect(server.to("GET", "/api/challenges").length).toBeGreaterThan(0);
      }, SOON);
      await settle({ advance: SOCIAL_POLL_MS + 1 });
      expect(server.to("GET", "/api/challenges")).toHaveLength(2);
      expect(acceptButtons()).toHaveLength(0);
      expect(document.body).not.toHaveTextContent(BOB.username);
    },
    TEST_MS,
  );

  it(
    "B57 signed out: the toast reads no challenges and shows nothing",
    async () => {
      vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
      const server = serve({ incoming: [INCOMING], outgoing: [] });
      render(<ChallengeToast />);
      await settle({ advance: SOCIAL_POLL_MS + 1 });
      expect(server.to("GET", "/api/challenges")).toHaveLength(0);
      expect(acceptButtons()).toHaveLength(0);
      expect(document.body).not.toHaveTextContent(ANN.username);
    },
    TEST_MS,
  );

  it(
    "B57 a refused read of the challenges shows no toast",
    async () => {
      vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
      signIn("tok-toast");
      const server = serve({ incoming: [INCOMING], outgoing: [] }, (call) =>
        call.path === "/api/challenges" ? { status: 500, body: { error: { code: "internal", message: "boom" } } } : undefined,
      );
      render(<ChallengeToast />);
      await waitFor(() => {
        expect(server.to("GET", "/api/challenges").length).toBeGreaterThan(0);
      }, SOON);
      await settle({ advance: SOCIAL_POLL_MS + 1 });
      expect(server.to("GET", "/api/challenges")).toHaveLength(2);
      expect(acceptButtons()).toHaveLength(0);
    },
    TEST_MS,
  );

  it(
    "B57 the shell mounts the toast: an incoming challenge shows on any signed-in screen, once",
    async () => {
      signIn("tok-toast");
      serve({ incoming: [INCOMING], outgoing: [] });
      at("/");
      render(<App />);
      expect(await screen.findByRole("button", { name: /^accept/i }, SOON)).toBeInTheDocument();
      expect(acceptButtons()).toHaveLength(1);
      expect(must(acceptButtons()[0], "accept")).toBeInTheDocument();
    },
    TEST_MS,
  );

  it(
    "R1754000 R1831000 Accept on a guarded page joins only once the guard lets go",
    async () => {
      signIn("tok-toast");
      const server = serve({ incoming: [INCOMING], outgoing: [] });
      const held: (() => void)[] = [];
      const release = guardLeaving((leave) => {
        held.push(leave);
      });
      try {
        render(<ChallengeToast />);
        fireEvent.click(await screen.findByTestId(challengeToastTestid.accept, undefined, SOON));
        expect(held).toHaveLength(1);
        expect(window.location.pathname).toBe("/decks");
        expect(acceptButtons()).toHaveLength(1);
        // Nothing is joined while the page asks: the player may keep their work.
        expect(server.to("POST", `/api/rooms/${INCOMING.roomCode}/join`)).toHaveLength(0);

        act(() => {
          must(held[0], "held move")();
        });
        await waitFor(() => {
          expect(window.location.pathname).toBe(`/match/${JOINED_MATCH}`);
        });
        expect(server.to("POST", `/api/rooms/${INCOMING.roomCode}/join`)).toHaveLength(1);
      } finally {
        release();
      }
    },
    TEST_MS,
  );

  it(
    "R1754000 R1831000 Open in lobby on a guarded page waits for the note too",
    async () => {
      signIn("tok-toast");
      serve({ incoming: [INCOMING], outgoing: [] }, (call) =>
        call.method === "GET" && call.path === "/api/decks" ? { status: 200, body: decksBody([]) } : undefined,
      );
      const held: (() => void)[] = [];
      const release = guardLeaving((leave) => {
        held.push(leave);
      });
      try {
        render(<ChallengeToast />);
        fireEvent.click(await screen.findByTestId(challengeToastTestid.lobby, undefined, SOON));
        expect(held).toHaveLength(1);
        expect(window.location.pathname).toBe("/decks");
        expect(screen.getAllByTestId(challengeToastTestid.toast)).toHaveLength(1);

        act(() => {
          must(held[0], "held move")();
        });
        await waitFor(() => {
          expect(window.location.pathname).toBe("/play");
        });
        expect(readRoomLink()?.code).toBe(INCOMING.roomCode);
      } finally {
        release();
      }
    },
    TEST_MS,
  );
});

describe("R2031000 an All Random challenge's more cards from", () => {
  it(
    "R2031000 the toast starts on the device's online pick, and Accept joins with the pick as leanSet",
    async () => {
      window.localStorage.setItem(PLAY_LEAN_SET_KEY, LEAN_NEWEST);
      signIn("tok-toast");
      const server = serve({ incoming: [{ ...INCOMING, mode: "random" }], outgoing: [] });
      render(<ChallengeToast />);
      const picker = await screen.findByTestId<HTMLSelectElement>(challengeToastTestid.lean, undefined, SOON);
      expect(picker.value).toBe(LEAN_NEWEST);
      fireEvent.change(picker, { target: { value: "Classic" } });
      expect(readPlayLean()).toBe("Classic");

      fireEvent.click(screen.getByTestId(challengeToastTestid.accept));
      await waitFor(() => {
        expect(window.location.pathname).toBe(`/match/${JOINED_MATCH}`);
      }, SOON);
      expect(server.to("POST", `/api/rooms/${INCOMING.roomCode}/join`)[0]?.body).toEqual({ mode: "random", leanSet: "Classic" });
    },
    TEST_MS,
  );

  it(
    "R2031000 a challenge that arrives after the pick changed elsewhere starts on the new pick",
    async () => {
      vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
      signIn("tok-toast");
      const challenges: Challenges = { incoming: [], outgoing: [] };
      const server = serve(challenges);
      render(<ChallengeToast />);
      await waitFor(() => {
        expect(server.to("GET", "/api/challenges").length).toBeGreaterThan(0);
      }, SOON);
      expect(screen.queryByTestId(challengeToastTestid.lean)).toBeNull();

      // The lobby (or any other door) writes the device's pick while the shell is up.
      window.localStorage.setItem(PLAY_LEAN_SET_KEY, "Classic");
      challenges.incoming = [{ ...INCOMING, mode: "random" }];
      await settle({ advance: SOCIAL_POLL_MS + 1 });
      await waitFor(() => {
        expect(screen.getByTestId<HTMLSelectElement>(challengeToastTestid.lean).value).toBe("Classic");
      }, SOON);
    },
    TEST_MS,
  );

  it(
    "R2031000 a Best of 1 challenge offers no pick and joins with its deck alone",
    async () => {
      window.localStorage.setItem(PLAY_LEAN_SET_KEY, LEAN_NEWEST);
      signIn("tok-toast");
      const server = serve({ incoming: [INCOMING], outgoing: [] });
      render(<ChallengeToast />);
      await screen.findByTestId(challengeToastTestid.deck, undefined, SOON);
      expect(screen.queryByTestId(challengeToastTestid.lean)).toBeNull();

      fireEvent.click(screen.getByTestId(challengeToastTestid.accept));
      await waitFor(() => {
        expect(window.location.pathname).toBe(`/match/${JOINED_MATCH}`);
      }, SOON);
      expect(server.to("POST", `/api/rooms/${INCOMING.roomCode}/join`)[0]?.body).toEqual({ mode: "bo1", deckId: "d-aggro" });
    },
    TEST_MS,
  );
});

describe("R915005 the challenge toast never offers a deck or trio with a custom card", () => {
  it(
    "R915005 Best of 1: the remembered Imps is listed off, the pick starts on Core, the note says why, and Accept joins with Core",
    async () => {
      signIn("tok-toast");
      window.localStorage.setItem(CHALLENGE_CHOICE_KEY, JSON.stringify({ mode: "bo1", deckId: "d-imps", trioId: null }));
      const server = serve({ incoming: [INCOMING], outgoing: [] }, (call) =>
        call.method === "GET" && call.path === "/api/decks" ? { status: 200, body: decksBody([IMPS, CORE]) } : undefined,
      );
      render(<ChallengeToast />);
      const picker = (await screen.findByTestId(challengeToastTestid.deck, undefined, SOON)) as HTMLSelectElement;
      expect(picker.value).toBe("d-core");
      expect(within(picker).getByRole("option", { name: "Imps (practice only)" })).toBeDisabled();
      expect(within(picker).getByRole("option", { name: "Core" })).toBeEnabled();
      expect(screen.getByTestId(challengeToastTestid.practiceOnly)).toHaveTextContent(
        "A deck with a custom card plays only in practice, against the AI.",
      );

      fireEvent.click(screen.getByTestId(challengeToastTestid.accept));
      await waitFor(() => {
        expect(window.location.pathname).toBe(`/match/${JOINED_MATCH}`);
      }, SOON);
      expect(server.to("POST", `/api/rooms/${INCOMING.roomCode}/join`)[0]?.body).toEqual({ mode: "bo1", deckId: "d-core" });
    },
    TEST_MS,
  );

  it(
    "R915005 Conquest: a trio holding Imps is listed off and never picked, and Accept joins with Core trio",
    async () => {
      signIn("tok-toast");
      window.localStorage.setItem(CHALLENGE_CHOICE_KEY, JSON.stringify({ mode: "bo3", deckId: null, trioId: "t-imps" }));
      const server = serve({ incoming: [{ ...INCOMING, mode: "bo3" }], outgoing: [] }, (call) =>
        call.method === "GET" && call.path === "/api/decks"
          ? { status: 200, body: decksBody([IMPS, CORE], [IMP_TRIO, CORE_TRIO]) }
          : undefined,
      );
      render(<ChallengeToast />);
      const picker = (await screen.findByTestId(challengeToastTestid.deck, undefined, SOON)) as HTMLSelectElement;
      expect(picker.value).toBe("t-core");
      expect(within(picker).getByRole("option", { name: "Imp trio (practice only)" })).toBeDisabled();
      expect(screen.getByTestId(challengeToastTestid.practiceOnly)).toHaveTextContent("cannot play Conquest");

      fireEvent.click(screen.getByTestId(challengeToastTestid.accept));
      await waitFor(() => {
        expect(window.location.pathname).toBe(`/match/${JOINED_MATCH}`);
      }, SOON);
      expect(server.to("POST", `/api/rooms/${INCOMING.roomCode}/join`)[0]?.body).toEqual({ mode: "bo3", trioId: "t-core" });
    },
    TEST_MS,
  );

  it(
    "R915005 with only practice decks the toast offers Open in lobby and says why",
    async () => {
      signIn("tok-toast");
      serve({ incoming: [INCOMING], outgoing: [] }, (call) =>
        call.method === "GET" && call.path === "/api/decks" ? { status: 200, body: decksBody([IMPS]) } : undefined,
      );
      render(<ChallengeToast />);
      expect(await screen.findByTestId(challengeToastTestid.lobby, undefined, SOON)).toBeInTheDocument();
      expect(screen.queryByTestId(challengeToastTestid.accept)).toBeNull();
      expect(screen.queryByTestId(challengeToastTestid.deck)).toBeNull();
      expect(screen.getByTestId(challengeToastTestid.practiceOnly)).toBeInTheDocument();
    },
    TEST_MS,
  );

  it(
    "R915005 a toast with nothing practice-only has no note",
    async () => {
      signIn("tok-toast");
      serve({ incoming: [INCOMING], outgoing: [] });
      render(<ChallengeToast />);
      await screen.findByTestId(challengeToastTestid.deck, undefined, SOON);
      expect(screen.queryByTestId(challengeToastTestid.practiceOnly)).toBeNull();
    },
    TEST_MS,
  );
});
