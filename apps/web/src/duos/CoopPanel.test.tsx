// #936 S8 B60: `CoopPanel`, the co-op room on /together: create a room, share its code, join by code,
// and both players go on to `/duos/:matchId`. A refusal (B33) is shown in the server's words and moves
// nobody. R2031000: the Random deck offers "More cards from", sent as `leanSet` on the create and join.

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { LEAN_NEWEST } from "@jackioh/shared";

import { PLAY_LEAN_SET_KEY, leanLabel, readPlayLean } from "../game/LeanNewest.tsx";
import { resetLocaleForTests, setLocale } from "../i18n/i18n.ts";
import { COOP_LINK_STORAGE_KEY, findRoomLinkInText } from "../net/roomLink.ts";
import type { Friend } from "../net/social.ts";
import { at, must, refusal, signIn, stubServer, type Call, type Reply, type StubServer } from "../test/stubServer.ts";
import { COOP_DECK_KEY, CoopPanel, defaultCoopDeck } from "./CoopPanel.tsx";
import { COOP_ROOM_POLL_MS } from "./config.ts";
import { setVisibility } from "../test/setup.ts";

const SLOW = { timeout: 5_000 } as const;
/** SPEC S6/S8 give the host's wait for the guest no interval; the host's way on comes well inside this. */
const HOST_WAIT = { timeout: 10_000 } as const;
const TEST_MS = 20_000;

const ME = { profileId: "profile-1", username: "Me#1" };
const ANN: Friend = {
  profileId: "f-ann",
  username: "Ann#2",
  status: "online",
  matchId: null,
  watchable: false,
  since: "2026-10-01T00:00:00.000Z",
};

const CODE = "HJK234";
const OTHER_CODE = "MNP567";

const DECKS = {
  catalogVersion: "v1",
  decks: [{ id: "d-aggro", name: "Aggro", cards: [], catalogVersion: "v1", portrait: null, createdAt: 0, updatedAt: 0 }],
  trios: [],
  limits: { decks: 20, trios: 10, nameLength: 40 },
};

type Room = { guestJoined: boolean; created: boolean };

/** The co-op routes: a room that exists once created and holds a match once `guestJoined`. */
function serve(room: Room, special?: (call: Call) => Reply | undefined): StubServer {
  return stubServer((call) => {
    const answer = special?.(call);
    if (answer !== undefined) return answer;
    const { method, path } = call;
    if (path === "/api/auth/me") {
      return {
        status: 200,
        body: { profile: { id: ME.profileId, status: "active" }, needsInviteCode: false, emailVerified: true, currentMatchId: null, email: null },
      };
    }
    if (method === "GET" && path === "/api/decks") return { status: 200, body: DECKS };
    if (method === "GET" && path === "/api/catalog") return { status: 200, body: { version: "v1", defs: {} } };
    if (method === "GET" && path === "/api/collection") return { status: 200, body: { catalogVersion: "v1", entries: [] } };
    if (method === "POST" && path === "/api/duos/rooms") {
      room.created = true;
      return { status: 201, body: { code: CODE } };
    }
    if (method === "GET" && path === `/api/duos/rooms/${CODE}` && room.created) {
      return {
        status: 200,
        body: {
          code: CODE,
          difficulty: "hard",
          host: ME,
          guest: room.guestJoined ? { profileId: ANN.profileId, username: ANN.username } : null,
          matchId: room.guestJoined ? "dm-5" : null,
        },
      };
    }
    if (method === "GET" && path === "/api/duos/live") {
      return { status: 200, body: { matchId: room.created && room.guestJoined ? "dm-5" : null } };
    }
    if (method === "POST" && path === `/api/duos/rooms/${OTHER_CODE}/join`) return { status: 200, body: { matchId: "dm-9" } };
    if (method === "POST" && path.startsWith("/api/chat/")) {
      const body = (call.body as { body?: unknown } | undefined)?.body;
      return {
        status: 201,
        body: {
          message: {
            id: "msg-1",
            from: ME.profileId,
            to: path.slice("/api/chat/".length),
            body: typeof body === "string" ? body : "",
            createdAt: "2026-10-14T09:00:00.000Z",
            read: false,
          },
        },
      };
    }
    if (method === "DELETE" && path === `/api/duos/rooms/${CODE}`) return { status: 204 };
    return undefined;
  });
}

/** `coop-difficulty` as a select, a radio group or a row of buttons. */
function chooseDifficulty(value: "easy" | "medium" | "hard"): void {
  const control = screen.getByTestId("coop-difficulty");
  if (control instanceof HTMLSelectElement) {
    fireEvent.change(control, { target: { value } });
    return;
  }
  const input = control.querySelector<HTMLInputElement>(`input[value="${value}"]`);
  if (input !== null) {
    fireEvent.click(input);
    return;
  }
  const choice = [...control.querySelectorAll<HTMLElement>("button, [role='radio'], option")].find(
    (element) => element.getAttribute("value") === value || new RegExp(`^\\s*${value}\\s*$`, "i").test(element.textContent ?? ""),
  );
  fireEvent.click(must(choice, `the ${value} difficulty`));
}

async function created(server: StubServer): Promise<Record<string, unknown>> {
  await waitFor(() => {
    expect(server.to("POST", "/api/duos/rooms")).toHaveLength(1);
  }, SLOW);
  return must(server.to("POST", "/api/duos/rooms")[0], "the create").body as Record<string, unknown>;
}

/** S6: exactly one of `deckId` and `random: true`. */
function expectOneDeckChoice(body: Record<string, unknown>): void {
  const deck = typeof body.deckId === "string";
  const random = body.random === true;
  expect(deck !== random, `exactly one of deckId and random:true in ${JSON.stringify(body)}`).toBe(true);
}

async function renderPanel(friends: readonly Friend[] = [ANN]): Promise<void> {
  signIn("tok-coop");
  render(<CoopPanel friends={friends} />);
  await screen.findByTestId("coop-create", undefined, SLOW);
}

async function createRoom(server: StubServer): Promise<void> {
  await waitFor(() => {
    expect((screen.getByTestId("coop-create") as HTMLButtonElement).disabled).not.toBe(true);
  }, SLOW);
  fireEvent.click(screen.getByTestId("coop-create"));
  await created(server);
  await waitFor(() => {
    expect(screen.getByTestId("coop-code")).toHaveTextContent(CODE);
  }, SLOW);
}

async function join(code: string): Promise<void> {
  const input = await screen.findByTestId("coop-join-input", undefined, SLOW);
  fireEvent.change(input, { target: { value: code } });
  await waitFor(() => {
    expect((screen.getByTestId("coop-join-submit") as HTMLButtonElement).disabled).not.toBe(true);
  }, SLOW);
  fireEvent.click(screen.getByTestId("coop-join-submit"));
}

/** Let the stubbed round trips and React's updates settle: a 0 ms timer runs after every promise already queued. */
async function flush(): Promise<void> {
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 0));
  });
}

beforeEach(() => {
  window.localStorage.clear();
  window.sessionStorage.clear();
  at("/together");
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.localStorage.clear();
  window.sessionStorage.clear();
  at("/together");
  Reflect.deleteProperty(navigator, "clipboard");
  Reflect.deleteProperty(navigator, "share");
  resetLocaleForTests();
});

describe("B60 creating a room", () => {
  it("B60 sends the chosen difficulty and exactly one deck choice to POST /api/duos/rooms", async () => {
    const server = serve({ created: false, guestJoined: false });
    await renderPanel();
    chooseDifficulty("hard");
    await createRoom(server);
    const body = await created(server);
    expect(body.difficulty).toBe("hard");
    expectOneDeckChoice(body);
  });

  it("B60 easy is sent as easy: the panel passes the difficulty through, it does not pick one", async () => {
    const server = serve({ created: false, guestJoined: false });
    await renderPanel();
    chooseDifficulty("easy");
    await createRoom(server);
    expect((await created(server)).difficulty).toBe("easy");
  });

  it("B60 shows the room's code once the server has made it", async () => {
    const server = serve({ created: false, guestJoined: false });
    await renderPanel();
    await createRoom(server);
    expect(screen.getByTestId("coop-code")).toHaveTextContent(CODE);
  });

  it("B60 before a room is made there is no code to show", async () => {
    serve({ created: false, guestJoined: false });
    await renderPanel();
    const code = screen.queryByTestId("coop-code");
    expect(code === null || !(code.textContent ?? "").includes(CODE)).toBe(true);
  });

  it("B60 a refused creation is shown in the server's words, with no code and nowhere to go", async () => {
    const words = "Redeem an invite code first.";
    const server = serve({ created: false, guestJoined: false }, (call) =>
      call.method === "POST" && call.path === "/api/duos/rooms" ? refusal(403, "account_pending", words) : undefined,
    );
    await renderPanel();
    await waitFor(() => {
      expect((screen.getByTestId("coop-create") as HTMLButtonElement).disabled).not.toBe(true);
    }, SLOW);
    fireEvent.click(screen.getByTestId("coop-create"));
    await waitFor(() => {
      expect(document.body).toHaveTextContent(words);
    }, SLOW);
    const code = screen.queryByTestId("coop-code");
    expect(code === null || !(code.textContent ?? "").includes(CODE)).toBe(true);
    expect(window.location.pathname).toBe("/together");
    expect(server.calls.some((call) => call.method === "POST" && call.path.startsWith("/api/chat/"))).toBe(false);
  });

  it(
    "B60 the host goes to /duos/:matchId once the guest has joined",
    async () => {
      const room: Room = { created: false, guestJoined: true };
      const server = serve(room);
      await renderPanel();
      await createRoom(server);
      await waitFor(() => {
        expect(window.location.pathname).toBe("/duos/dm-5");
      }, HOST_WAIT);
    },
    TEST_MS,
  );

  it(
    "a woken page reads the co-op room again at once, and a joined room still takes the host to the game",
    async () => {
      const visibility = Object.getOwnPropertyDescriptor(document, "visibilityState");
      // The host's own poll ticks on nothing, so only a wake can read the room again.
      const interval = window.setInterval.bind(window);
      vi.spyOn(window, "setInterval").mockImplementation(((handler: TimerHandler, ms?: number, ...args: unknown[]) =>
        ms === COOP_ROOM_POLL_MS ? interval(() => undefined, ms) : interval(handler, ms, ...args)) as typeof window.setInterval);
      try {
        const room: Room = { created: false, guestJoined: false };
        const server = serve(room);
        await renderPanel();
        await createRoom(server);
        await waitFor(() => {
          expect(server.to("GET", `/api/duos/rooms/${CODE}`)).toHaveLength(1);
        }, SLOW);
        await flush();

        setVisibility("hidden");
        await flush();
        expect(server.to("GET", `/api/duos/rooms/${CODE}`)).toHaveLength(1);

        // Wakes that land together are one read.
        setVisibility("visible");
        setVisibility("visible");
        window.dispatchEvent(new Event("online"));
        await waitFor(() => {
          expect(server.to("GET", `/api/duos/rooms/${CODE}`)).toHaveLength(2);
        }, SLOW);
        await flush();
        expect(server.to("GET", `/api/duos/rooms/${CODE}`)).toHaveLength(2);
        expect(window.location.pathname).toBe("/together");

        room.guestJoined = true;
        setVisibility("visible");
        await waitFor(() => {
          expect(window.location.pathname).toBe("/duos/dm-5");
        }, SLOW);
      } finally {
        if (visibility === undefined) Reflect.deleteProperty(document, "visibilityState");
        else Object.defineProperty(document, "visibilityState", visibility);
      }
    },
    TEST_MS,
  );

  it(
    "B60 the host stays put while nobody has joined",
    async () => {
      vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
      const server = serve({ created: false, guestJoined: false });
      await renderPanel();
      await createRoom(server);
      await act(async () => {
        await vi.advanceTimersByTimeAsync(COOP_ROOM_POLL_MS + 1);
      });
      await flush();
      expect(server.to("GET", `/api/duos/rooms/${CODE}`)).toHaveLength(2);
      expect(window.location.pathname).toBe("/together");
      expect(screen.getByTestId("coop-code")).toHaveTextContent(CODE);
    },
    TEST_MS,
  );
});

describe("B60 the deck choice", () => {
  it("B60 decks that cannot be read are named under the picker, and a random deck is still offered", async () => {
    serve({ created: false, guestJoined: false }, (call) =>
      call.method === "GET" && call.path === "/api/decks" ? refusal(500, "internal", "The decks are out of reach.") : undefined,
    );
    await renderPanel();
    expect(await screen.findByRole("alert", undefined, SLOW)).toHaveTextContent(
      "Your decks could not be loaded: The decks are out of reach.",
    );
    const picker = screen.getByTestId<HTMLSelectElement>("coop-deck");
    expect([...picker.options].map((option) => option.textContent)).toEqual(["Random deck"]);
  });

  it("B60 saved decks are offered after a random deck, with no alert", async () => {
    serve({ created: false, guestJoined: false });
    await renderPanel();
    await waitFor(
      () => {
        const picker = screen.getByTestId<HTMLSelectElement>("coop-deck");
        expect([...picker.options].map((option) => option.textContent)).toEqual(["Random deck", "Aggro"]);
      },
      SLOW,
    );
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("#1568 decks that could not be read offer Try again, which reads them again and offers them", async () => {
    let reads = 0;
    serve({ created: false, guestJoined: false }, (call) => {
      if (call.method === "GET" && call.path === "/api/decks") {
        reads += 1;
        if (reads === 1) return refusal(500, "internal", "The decks are out of reach.");
      }
      return undefined;
    });
    await renderPanel();
    expect(await screen.findByRole("alert", undefined, SLOW)).toHaveTextContent(
      "Your decks could not be loaded: The decks are out of reach.",
    );

    fireEvent.click(screen.getByTestId("coop-decks-retry"));
    await waitFor(() => {
      const picker = screen.getByTestId<HTMLSelectElement>("coop-deck");
      expect([...picker.options].map((option) => option.textContent)).toEqual(["Random deck", "Aggro"]);
    }, SLOW);
    expect(reads).toBe(2);
    expect(screen.queryByRole("alert")).toBeNull();
  });
});

describe("R2031000 the Random deck's more cards from", () => {
  /** R2094000 starts the deck on the first saved deck: these tests pick the Random deck first. */
  async function chooseRandomDeck(): Promise<void> {
    await waitFor(() => {
      expect(screen.getByTestId<HTMLSelectElement>("coop-deck").options).toHaveLength(2);
    }, SLOW);
    fireEvent.change(screen.getByTestId("coop-deck"), { target: { value: "random" } });
  }

  it("R2031000 the picker is the Random deck's alone and starts on the device's online pick", async () => {
    window.localStorage.setItem(PLAY_LEAN_SET_KEY, "Classic");
    serve({ created: false, guestJoined: false });
    await renderPanel();
    await chooseRandomDeck();
    const picker = screen.getByTestId<HTMLSelectElement>("coop-lean-newest");
    expect(picker.value).toBe("Classic");
    expect(picker.closest("label")).toHaveTextContent(leanLabel());
    fireEvent.change(screen.getByTestId("coop-deck"), { target: { value: "d-aggro" } });
    expect(screen.queryByTestId("coop-lean-newest")).toBeNull();
  });

  it("R2031000 with a pick the create sends leanSet, and the device keeps the pick", async () => {
    const server = serve({ created: false, guestJoined: false });
    await renderPanel();
    await chooseRandomDeck();
    const picker = screen.getByTestId<HTMLSelectElement>("coop-lean-newest");
    expect(picker.value).toBe("");
    fireEvent.change(picker, { target: { value: LEAN_NEWEST } });
    expect(readPlayLean()).toBe(LEAN_NEWEST);
    await createRoom(server);
    expect(await created(server)).toEqual({ difficulty: "easy", random: true, leanSet: LEAN_NEWEST });
  });

  it("R2031000 with none picked the create sends no lean at all", async () => {
    window.localStorage.setItem(PLAY_LEAN_SET_KEY, "Classic");
    const server = serve({ created: false, guestJoined: false });
    await renderPanel();
    await chooseRandomDeck();
    fireEvent.change(screen.getByTestId("coop-lean-newest"), { target: { value: "" } });
    expect(readPlayLean()).toBeNull();
    await createRoom(server);
    expect(await created(server)).toEqual({ difficulty: "easy", random: true });
  });

  it("R2031000 a join on the Random deck sends the pick as leanSet", async () => {
    const server = serve({ created: false, guestJoined: false });
    await renderPanel();
    await chooseRandomDeck();
    fireEvent.change(screen.getByTestId("coop-lean-newest"), { target: { value: "Classic" } });
    await join(OTHER_CODE);
    await waitFor(() => {
      expect(server.to("POST", `/api/duos/rooms/${OTHER_CODE}/join`)).toHaveLength(1);
    }, SLOW);
    expect(must(server.to("POST", `/api/duos/rooms/${OTHER_CODE}/join`)[0], "the join").body).toEqual({
      random: true,
      leanSet: "Classic",
    });
  });
});

describe("#1900 the way back to a live co-op game", () => {
  it("#1900 a live co-op game that could not be read is named in an alert with Try again, and no way back is drawn", async () => {
    serve({ created: false, guestJoined: false }, (call) =>
      call.method === "GET" && call.path === "/api/duos/live" ? refusal(500, "internal", "The co-op games are out of reach.") : undefined,
    );
    await renderPanel();
    expect(await screen.findByRole("alert", undefined, SLOW)).toHaveTextContent(
      "Your live co-op game could not be checked: The co-op games are out of reach.",
    );
    expect(screen.getByTestId("coop-live-retry")).toHaveTextContent("Try again");
    expect(screen.queryByTestId("coop-rejoin")).toBeNull();
  }, TEST_MS);

  it("#1900 Try again reads the live co-op game again and offers the way back to it", async () => {
    let reads = 0;
    serve({ created: false, guestJoined: false }, (call) => {
      if (call.method === "GET" && call.path === "/api/duos/live") {
        reads += 1;
        return reads === 1 ? refusal(500, "internal", "The co-op games are out of reach.") : { status: 200, body: { matchId: "dm-5" } };
      }
      return undefined;
    });
    await renderPanel();
    expect(await screen.findByRole("alert", undefined, SLOW)).toHaveTextContent(
      "Your live co-op game could not be checked: The co-op games are out of reach.",
    );

    fireEvent.click(screen.getByTestId("coop-live-retry"));
    expect(await screen.findByTestId("coop-rejoin", undefined, SLOW)).toHaveAttribute("href", "/duos/dm-5");
    expect(reads).toBe(2);
    expect(screen.queryByRole("alert")).toBeNull();
    expect(screen.queryByTestId("coop-live-retry")).toBeNull();
  }, TEST_MS);

  it("#1900 when both reads fail, each Try again is described by its own alert", async () => {
    serve({ created: false, guestJoined: false }, (call) =>
      call.method === "GET" && call.path === "/api/duos/live"
        ? refusal(500, "internal", "The co-op games are out of reach.")
        : call.method === "GET" && call.path === "/api/decks"
          ? refusal(500, "internal", "The decks are out of reach.")
          : undefined,
    );
    await renderPanel();
    expect(await screen.findByTestId("coop-live-retry", undefined, SLOW)).toHaveAccessibleDescription(
      "Your live co-op game could not be checked: The co-op games are out of reach.",
    );
    expect(await screen.findByTestId("coop-decks-retry", undefined, SLOW)).toHaveAccessibleDescription(
      "Your decks could not be loaded: The decks are out of reach.",
    );
  }, TEST_MS);
});

describe("B60 inviting a friend", () => {
  function inviteControl(): HTMLElement {
    const invites = screen.queryAllByRole("button", { name: /invite/i });
    const beside = invites.find((button) => {
      let node: HTMLElement | null = button;
      for (let depth = 0; node !== null && depth < 4; depth += 1) {
        if ((node.textContent ?? "").includes(ANN.username)) return true;
        node = node.parentElement;
      }
      return false;
    });
    return must(beside ?? (invites.length === 1 ? invites[0] : undefined), "the invite control for Ann#2");
  }

  it("B60 invites a friend by sending them the room's code in chat", async () => {
    const server = serve({ created: false, guestJoined: false });
    await renderPanel();
    await createRoom(server);
    fireEvent.click(inviteControl());
    await waitFor(() => {
      expect(server.to("POST", `/api/chat/${ANN.profileId}`)).toHaveLength(1);
    }, SLOW);
    const body = must(server.to("POST", `/api/chat/${ANN.profileId}`)[0], "the invite").body as { body?: unknown };
    expect(typeof body.body).toBe("string");
    expect(String(body.body)).toContain(CODE);
  });

  it("B60 no invite goes out before there is a room to invite to", async () => {
    const server = serve({ created: false, guestJoined: false });
    await renderPanel();
    for (const button of screen.queryAllByRole("button", { name: /invite/i })) {
      if ((button as HTMLButtonElement).disabled !== true) fireEvent.click(button);
    }
    await flush();
    expect(server.calls.some((call) => call.method === "POST" && call.path.startsWith("/api/chat/"))).toBe(false);
  });

  it("B60 the invite is sent to the friend picked, and to nobody else", async () => {
    const CY: Friend = { ...ANN, profileId: "f-cy", username: "Cy#4" };
    const server = serve({ created: false, guestJoined: false });
    await renderPanel([ANN, CY]);
    await createRoom(server);
    const panel = screen.getByTestId("coop-code").closest("section") ?? document.body;
    const select = within(panel).queryAllByRole("combobox").find((element) =>
      [...(element as HTMLSelectElement).options].some((option) => option.value === ANN.profileId),
    );
    if (select !== undefined) fireEvent.change(select, { target: { value: ANN.profileId } });
    fireEvent.click(inviteControl());
    await waitFor(() => {
      expect(server.to("POST", `/api/chat/${ANN.profileId}`)).toHaveLength(1);
    }, SLOW);
    expect(server.to("POST", "/api/chat/f-cy")).toHaveLength(0);
  });
});

describe("B60 B33 joining by code", () => {
  it("B60 joins with POST /api/duos/rooms/:code/join and exactly one deck choice, then goes to /duos/:matchId", async () => {
    const server = serve({ created: false, guestJoined: false });
    await renderPanel();
    await join(OTHER_CODE);
    await waitFor(() => {
      expect(server.to("POST", `/api/duos/rooms/${OTHER_CODE}/join`)).toHaveLength(1);
    }, SLOW);
    expectOneDeckChoice(must(server.to("POST", `/api/duos/rooms/${OTHER_CODE}/join`)[0], "the join").body as Record<string, unknown>);
    await waitFor(() => {
      expect(window.location.pathname).toBe("/duos/dm-9");
    }, SLOW);
  });

  it("B33 a full room is refused in the server's words and the player stays", async () => {
    const words = "That room is full.";
    serve({ created: false, guestJoined: false }, (call) => (call.path.endsWith("/join") ? refusal(409, "full", words) : undefined));
    await renderPanel();
    await join(OTHER_CODE);
    await waitFor(() => {
      expect(document.body).toHaveTextContent(words);
    }, SLOW);
    expect(window.location.pathname).toBe("/together");
  });

  it("B33 joining one's own room is refused in the server's words and the player stays", async () => {
    const words = "That is your own room.";
    serve({ created: false, guestJoined: false }, (call) => (call.path.endsWith("/join") ? refusal(409, "own_room", words) : undefined));
    await renderPanel();
    await join(OTHER_CODE);
    await waitFor(() => {
      expect(document.body).toHaveTextContent(words);
    }, SLOW);
    expect(window.location.pathname).toBe("/together");
  });

  it("B34 an unknown or expired code is refused in the server's words and the player stays", async () => {
    const words = "No room has that code.";
    serve({ created: false, guestJoined: false }, (call) => (call.path.endsWith("/join") ? refusal(404, "not_found", words) : undefined));
    await renderPanel();
    await join("ZZZ999");
    await waitFor(() => {
      expect(document.body).toHaveTextContent(words);
    }, SLOW);
    expect(window.location.pathname).toBe("/together");
  });

  it("B60 a refused join creates no room and sends no invite", async () => {
    const server = serve({ created: false, guestJoined: false }, (call) =>
      call.path.endsWith("/join") ? refusal(409, "full", "That room is full.") : undefined,
    );
    await renderPanel();
    await join(OTHER_CODE);
    await waitFor(() => {
      expect(document.body).toHaveTextContent("That room is full.");
    }, SLOW);
    expect(server.to("POST", "/api/duos/rooms")).toHaveLength(0);
    expect(server.calls.some((call) => call.method === "POST" && call.path.startsWith("/api/chat/"))).toBe(false);
  });
});

describe("R2094000 a co-op link fills the join form", () => {
  it("R2094000 opening with a stored co-op link fills the join form and focuses Join, and forgets it", async () => {
    serve({ created: false, guestJoined: false });
    at("/together?coop=abc234");
    await renderPanel();
    const input = await screen.findByTestId("coop-join-input", undefined, SLOW);
    expect(input).toHaveValue("ABC234");
    // The link leaves the address bar at once, and the tab forgets it once used.
    expect(window.location.pathname).toBe("/together");
    expect(window.location.search).toBe("");
    await waitFor(() => {
      expect(screen.getByTestId("coop-join-submit")).toHaveFocus();
    }, SLOW);
    await waitFor(() => {
      expect(window.sessionStorage.getItem(COOP_LINK_STORAGE_KEY)).toBeNull();
    }, SLOW);
  });

  it("R2094000 the join deck starts on the last co-op pick, else the first saved deck, else Random", async () => {
    window.localStorage.setItem(COOP_DECK_KEY, "d-control");
    const control = { ...must(DECKS.decks[0], "the saved deck"), id: "d-control", name: "Control" };
    serve({ created: false, guestJoined: false }, (call) =>
      call.method === "GET" && call.path === "/api/decks" ? { status: 200, body: { ...DECKS, decks: [...DECKS.decks, control] } } : undefined,
    );
    await renderPanel();
    await waitFor(() => {
      expect(screen.getByTestId<HTMLSelectElement>("coop-deck").options).toHaveLength(3);
    }, SLOW);
    await waitFor(() => {
      expect(screen.getByTestId<HTMLSelectElement>("coop-deck").value).toBe("d-control");
    }, SLOW);
    cleanup();
    window.sessionStorage.clear();

    window.localStorage.removeItem(COOP_DECK_KEY);
    at("/together");
    serve({ created: false, guestJoined: false });
    await renderPanel();
    await waitFor(() => {
      expect(screen.getByTestId<HTMLSelectElement>("coop-deck").options).toHaveLength(2);
    }, SLOW);
    await waitFor(() => {
      expect(screen.getByTestId<HTMLSelectElement>("coop-deck").value).toBe("d-aggro");
    }, SLOW);
    cleanup();
    window.sessionStorage.clear();

    at("/together");
    serve({ created: false, guestJoined: false }, (call) =>
      call.method === "GET" && call.path === "/api/decks"
        ? { status: 200, body: { catalogVersion: "v1", decks: [], trios: [], limits: DECKS.limits } }
        : undefined,
    );
    await renderPanel();
    await waitFor(() => {
      expect(screen.getByTestId<HTMLSelectElement>("coop-deck").options).toHaveLength(1);
    }, SLOW);
    expect(screen.getByTestId<HTMLSelectElement>("coop-deck").value).toBe("random");
  });

  it("R2094000 a remembered deck the account no longer has is never sent: with no saved deck the create is a random deal", async () => {
    window.localStorage.setItem(COOP_DECK_KEY, "d-gone");
    const server = serve({ created: false, guestJoined: false }, (call) =>
      call.method === "GET" && call.path === "/api/decks"
        ? { status: 200, body: { catalogVersion: "v1", decks: [], trios: [], limits: DECKS.limits } }
        : undefined,
    );
    await renderPanel();
    await waitFor(() => {
      expect(server.to("GET", "/api/decks")).toHaveLength(1);
    }, SLOW);
    await flush();
    expect(screen.getByTestId<HTMLSelectElement>("coop-deck").value).toBe("random");
    expect(screen.getByTestId("coop-lean-newest")).toBeInTheDocument();
    await createRoom(server);
    expect(await created(server)).toEqual({ difficulty: "easy", random: true });
  });

  it("R2094000 a Random pick is a pick: it is remembered, and the next join starts on it", async () => {
    const server = serve({ created: false, guestJoined: false });
    await renderPanel();
    await waitFor(() => {
      expect(screen.getByTestId<HTMLSelectElement>("coop-deck").value).toBe("d-aggro");
    }, SLOW);
    fireEvent.change(screen.getByTestId("coop-deck"), { target: { value: "random" } });
    expect(window.localStorage.getItem(COOP_DECK_KEY)).toBe("random");
    expect(server.to("POST", "/api/duos/rooms")).toHaveLength(0);
    cleanup();
    window.sessionStorage.clear();

    at("/together");
    serve({ created: false, guestJoined: false });
    await renderPanel();
    await waitFor(() => {
      expect(screen.getByTestId<HTMLSelectElement>("coop-deck").options).toHaveLength(2);
    }, SLOW);
    await flush();
    expect(screen.getByTestId<HTMLSelectElement>("coop-deck").value).toBe("random");
  });

  it("R2094000 defaultCoopDeck prefers the remembered deck, then the first saved deck, then none", () => {
    const decks = DECKS.decks;
    expect(defaultCoopDeck(decks, decks[0]?.id ?? null)?.id).toBe("d-aggro");
    expect(defaultCoopDeck(decks, "gone")?.id).toBe("d-aggro");
    expect(defaultCoopDeck(decks, "random")).toBeNull();
    expect(defaultCoopDeck([], null)).toBeNull();
  });

  it("R2094000 Copy invite link writes the co-op link to the clipboard where there is no share sheet", async () => {
    const writeText = vi.fn(() => Promise.resolve());
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    const server = serve({ created: false, guestJoined: false });
    await renderPanel();
    await createRoom(server);
    const button = screen.getByTestId("duos-coop-link");
    expect(button).toHaveTextContent("Copy invite link");
    fireEvent.click(button);
    await waitFor(() => {
      expect(button).toHaveTextContent("Link copied");
    }, SLOW);
    expect(writeText).toHaveBeenCalledTimes(1);
    expect(writeText).toHaveBeenCalledWith(`${window.location.origin}/together?coop=${CODE}`);
  });

  it("R2094000 Copy invite link opens the share sheet where there is one, and does not touch the clipboard", async () => {
    const writeText = vi.fn(() => Promise.resolve());
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    const share = vi.fn(() => Promise.resolve());
    Object.defineProperty(navigator, "share", { configurable: true, value: share });
    const server = serve({ created: false, guestJoined: false });
    await renderPanel();
    await createRoom(server);
    fireEvent.click(screen.getByTestId("duos-coop-link"));
    await waitFor(() => {
      expect(screen.getByTestId("duos-coop-link")).toHaveTextContent("Link shared");
    }, SLOW);
    expect(share).toHaveBeenCalledWith({
      title: "Join my JackiOh room",
      url: `${window.location.origin}/together?coop=${CODE}`,
    });
    expect(writeText).not.toHaveBeenCalled();
  });

  it("R2094000 Send to friend's chat message contains the link and the code", async () => {
    const server = serve({ created: false, guestJoined: false });
    await renderPanel();
    await createRoom(server);
    const panel = screen.getByTestId("coop-code").closest("section") ?? document.body;
    const invite = within(panel).getByTestId("coop-invite");
    fireEvent.click(invite);
    await waitFor(() => {
      expect(server.to("POST", `/api/chat/${ANN.profileId}`)).toHaveLength(1);
    }, SLOW);
    const body = must(server.to("POST", `/api/chat/${ANN.profileId}`)[0], "the invite").body as { body?: unknown };
    expect(typeof body.body).toBe("string");
    expect(String(body.body)).toContain(CODE);
    expect(String(body.body)).toContain(`/together?coop=${CODE}`);
  });

  it("R2094000 the invite sent in Chinese keeps its link apart from the words, so the friend's chat draws Join", async () => {
    await setLocale("zh-CN");
    const server = serve({ created: false, guestJoined: false });
    await renderPanel();
    await createRoom(server);
    fireEvent.click(screen.getByTestId("coop-invite"));
    await waitFor(() => {
      expect(server.to("POST", `/api/chat/${ANN.profileId}`)).toHaveLength(1);
    }, SLOW);
    const body = must(server.to("POST", `/api/chat/${ANN.profileId}`)[0], "the invite").body as { body?: unknown };
    expect(String(body.body)).toContain("房间号");
    expect(findRoomLinkInText(String(body.body), window.location.origin)).toEqual({ path: `/together?coop=${CODE}` });
  });

  it("R2094000 join from the filled form posts /api/duos/rooms/<code>/join with the chosen deck and lands on /duos/<id>", async () => {
    const server = serve({ created: false, guestJoined: false });
    at(`/together?coop=${OTHER_CODE.toLowerCase()}`);
    await renderPanel();
    const input = await screen.findByTestId("coop-join-input", undefined, SLOW);
    expect(input).toHaveValue(OTHER_CODE);
    await waitFor(() => {
      expect(screen.getByTestId<HTMLSelectElement>("coop-deck").options).toHaveLength(2);
    }, SLOW);
    fireEvent.click(screen.getByTestId("coop-join-submit"));
    await waitFor(() => {
      expect(server.to("POST", `/api/duos/rooms/${OTHER_CODE}/join`)).toHaveLength(1);
    }, SLOW);
    expect(must(server.to("POST", `/api/duos/rooms/${OTHER_CODE}/join`)[0], "the join").body).toEqual({
      deckId: "d-aggro",
    });
    await waitFor(() => {
      expect(window.location.pathname).toBe("/duos/dm-9");
    }, SLOW);
    expect(window.localStorage.getItem(COOP_DECK_KEY)).toBe("d-aggro");
  });
});

describe("R915005 the co-op panel never offers a deck with a custom card", () => {
  // Imps holds a card from the creator; the saved Aggro holds none.
  const IMPS = { ...must(DECKS.decks[0], "the saved deck"), id: "d-imps", name: "Imps", cards: ["custom-ember-imp"] };

  function serveDecks(decks: readonly object[]): StubServer {
    return serve({ created: false, guestJoined: false }, (call) =>
      call.method === "GET" && call.path === "/api/decks" ? { status: 200, body: { ...DECKS, decks } } : undefined,
    );
  }

  it("R915005 the remembered Imps is listed off, the panel starts on the first deck that plays online, says why, and creates with it", async () => {
    window.localStorage.setItem(COOP_DECK_KEY, "d-imps");
    const server = serveDecks([IMPS, ...DECKS.decks]);
    await renderPanel();
    await waitFor(() => {
      expect(screen.getByTestId<HTMLSelectElement>("coop-deck").options).toHaveLength(3);
    }, SLOW);
    await waitFor(() => {
      expect(screen.getByTestId<HTMLSelectElement>("coop-deck").value).toBe("d-aggro");
    }, SLOW);
    const picker = screen.getByTestId("coop-deck");
    expect(within(picker).getByRole("option", { name: "Imps (practice only)" })).toBeDisabled();
    expect(within(picker).getByRole("option", { name: "Aggro" })).toBeEnabled();
    expect(screen.getByTestId("coop-practice-only")).toHaveTextContent(
      "A deck with a custom card plays only in practice, against the AI.",
    );

    await createRoom(server);
    expect(await created(server)).toEqual({ difficulty: "easy", deckId: "d-aggro" });
  });

  it("R915005 with only practice decks the panel starts on Random deck", async () => {
    window.localStorage.setItem(COOP_DECK_KEY, "d-imps");
    const server = serveDecks([IMPS]);
    await renderPanel();
    await waitFor(() => {
      expect(screen.getByTestId<HTMLSelectElement>("coop-deck").options).toHaveLength(2);
    }, SLOW);
    await flush();
    expect(screen.getByTestId<HTMLSelectElement>("coop-deck").value).toBe("random");
    expect(screen.getByTestId("coop-practice-only")).toBeInTheDocument();

    await createRoom(server);
    expect(await created(server)).toEqual({ difficulty: "easy", random: true });
  });

  it("R915005 with nothing practice-only there is no note", async () => {
    serve({ created: false, guestJoined: false });
    await renderPanel();
    await waitFor(() => {
      expect(screen.getByTestId<HTMLSelectElement>("coop-deck").options).toHaveLength(2);
    }, SLOW);
    expect(screen.queryByTestId("coop-practice-only")).toBeNull();
  });

  it("R915005 defaultCoopDeck passes over a practice deck, even the remembered one", () => {
    expect(defaultCoopDeck([IMPS, ...DECKS.decks], "d-imps")?.id).toBe("d-aggro");
    expect(defaultCoopDeck([IMPS], "d-imps")).toBeNull();
  });
});
