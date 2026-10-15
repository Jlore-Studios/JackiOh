// #936 S8 B60: `CoopPanel`, the co-op room on /together. It creates a room (a difficulty, and a deck
// or a random one: exactly one of `deckId` and `random: true`, S6), shows the room's code, invites a
// friend by sending them the code in chat, joins a room by code, and takes both players to
// `/duos/:matchId` once the guest has joined: the guest from the join's answer, the host once the
// server says the room has its match. A refusal (B33: full, own room, no such room) is shown in the
// server's words and moves nobody.

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import type { Friend } from "../net/social.ts";
import { must, refusal, stubServer, type Call, type Reply, type StubServer } from "../test/stubServer.ts";
import { CoopPanel } from "./CoopPanel.tsx";

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

function signedIn(): void {
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "tok-coop" }));
}

function at(path: string): void {
  window.history.replaceState(null, "", path);
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

/** The room the server was asked to create, once. */
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
  signedIn();
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

async function pause(ms: number): Promise<void> {
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, ms));
  });
}

beforeEach(() => {
  window.localStorage.clear();
  at("/together");
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.localStorage.clear();
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
    "B60 the host stays put while nobody has joined",
    async () => {
      const server = serve({ created: false, guestJoined: false });
      await renderPanel();
      await createRoom(server);
      await pause(1_500);
      expect(window.location.pathname).toBe("/together");
      expect(screen.getByTestId("coop-code")).toHaveTextContent(CODE);
    },
    TEST_MS,
  );
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
    await pause(100);
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
