// #1831: the Friends button at the top right of every signed-in screen, and its panel. The server is a
// stubbed `fetch` (`test/stubServer.ts`), as `ChallengeToast.test.tsx`'s is, and the shell's toast is
// rendered beside the button, as `main.tsx` mounts it: it is the one poll the badge reads (`socialPoll.ts`).

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import Board from "../game/Board.tsx";
import { NO_HIGHLIGHT } from "../game/contract.ts";
import { navTestid } from "../routes/nav.tsx";
import { BackLink } from "../routes/nav.tsx";
import type { Challenge, Friend, FriendRequest, PlayerRef } from "../net/social.ts";
import { at, meBody, must, refusal, signIn, stubServer, wholeText, type Call, type Reply, type StubServer } from "../test/stubServer.ts";
import { baseView } from "../test/fixtures.ts";
import { stubPhoneBar } from "../test/phoneBar.ts";
import { forgetRoomLink } from "../net/roomLink.ts";
import { skeletonTestid } from "../skeleton/Skeleton.tsx";
import { pressBack } from "../test/history.ts";
import { settle } from "../test/settle.ts";
import { expectSkeleton } from "../test/skeleton.ts";
import { stopFollowingChallenge } from "./follow.ts";
import { ChallengeToast } from "./ChallengeToast.tsx";
import { friendsTestid, togetherTestid } from "./testids.ts";
import { FriendsButton } from "./FriendsButton.tsx";

const { App, resetUsernamePromptForTests } = await import("../main.tsx");

const SLOW = { timeout: 8_000 } as const;
const TEST_MS = 15_000;

const ME: PlayerRef = { profileId: "profile-1", username: "Me#1" };
const SINCE = "2026-10-01T00:00:00.000Z";

function friend(profileId: string, username: string, status: Friend["status"], over: Partial<Friend> = {}): Friend {
  return { profileId, username, status, matchId: null, watchable: false, since: SINCE, ...over };
}

const ANN = friend("f-ann", "Ann#2", "online");
const BOB = friend("f-bob", "Bob#3", "inMatch", { matchId: "m-bob", watchable: true });
const CY = friend("f-cy", "Cy#4", "offline");

function challenge(challengeId: string, from: PlayerRef): Challenge {
  return { challengeId, from, to: ME, roomCode: "HJK234", mode: "bo1", expiresAt: "2099-01-01T00:00:00.000Z" };
}

function request(requestId: string, from: PlayerRef): FriendRequest {
  return { requestId, from, to: ME, createdAt: SINCE };
}

type World = {
  incoming: Challenge[];
  outgoing: Challenge[];
  unread: Record<string, number>;
  friends: Friend[];
  requests: FriendRequest[];
  me: Parameters<typeof meBody>[0];
};

function world(over: Partial<World> = {}): World {
  return {
    incoming: [],
    outgoing: [],
    unread: {},
    friends: [ANN, BOB, CY],
    requests: [],
    me: { profile: { id: ME.profileId, status: "active" }, username: { name: ME.username, nextChangeAt: null, promptOwed: false } },
    ...over,
  };
}

const DECKS = {
  catalogVersion: "v1",
  decks: [{ id: "d-aggro", name: "Aggro", cards: [], catalogVersion: "v1", portrait: null, createdAt: 0, updatedAt: 0 }],
  trios: [],
  limits: { decks: 20, trios: 10, nameLength: 40 },
};

function serve(state: World, special?: (call: Call) => Reply | undefined): StubServer {
  return stubServer((call) => {
    const answer = special?.(call);
    if (answer !== undefined) return answer;
    const { method, path } = call;
    if (path === "/api/auth/me") return { status: 200, body: meBody(state.me) };
    if (method === "GET" && path === "/api/challenges") return { status: 200, body: { incoming: state.incoming, outgoing: state.outgoing } };
    if (method === "GET" && path === "/api/chat/unread") return { status: 200, body: { unread: state.unread } };
    if (method === "GET" && path === "/api/friends") {
      return { status: 200, body: { friends: state.friends, incoming: state.requests, outgoing: [], blocked: [] } };
    }
    if (method === "GET" && path === "/api/decks") return { status: 200, body: DECKS };
    if (method === "GET" && path === "/api/catalog") return { status: 200, body: { version: "v1", defs: {} } };
    if (method === "GET" && path === "/api/wallet") return { status: 200, body: { jlore: 5 } };
    if (method === "GET" && path === "/api/quests") return { status: 200, body: { quests: [], nextDueAt: null } };
    if (method === "GET" && path === "/api/duos/live") return { status: 200, body: { matchId: null } };
    if (method === "POST" && path === "/api/presence") return { status: 204 };
    return undefined;
  });
}

function shell(): void {
  render(
    <>
      <ChallengeToast />
      <FriendsButton placement="game" />
    </>,
  );
}

async function button(): Promise<HTMLElement> {
  return screen.findByTestId(friendsTestid.button, undefined, SLOW);
}

async function openPanel(): Promise<HTMLElement> {
  fireEvent.click(await button());
  return screen.findByTestId(friendsTestid.panel, undefined, SLOW);
}

beforeEach(() => {
  window.localStorage.clear();
  forgetRoomLink();
  resetUsernamePromptForTests();
  at("/decks");
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  Reflect.deleteProperty(navigator, "clipboard");
  forgetRoomLink();
  window.localStorage.clear();
});

describe("#1831 where the Friends button sits", () => {
  it(
    "#1831 every signed-in screen has the button just before its language button, sign-in and a password reset none",
    async () => {
      signIn("tok-friends");
      serve(world());

      for (const path of ["/", "/play", "/decks", "/collection", "/account", "/replays", "/together", "/practice"]) {
        at(path);
        render(<App />);
        const found = await button();
        expect(found.nextElementSibling, path).toHaveAttribute("data-testid", "language-nav");
        cleanup();
      }

      // A board's control bar, which every match, Duos, replay and watch screen draws.
      render(<><ChallengeToast /><Board view={baseView()} highlight={NO_HIGHLIGHT} /></>);
      const onBoard = await button();
      expect(onBoard.nextElementSibling).toHaveAttribute("data-testid", "language-game");
      expect(onBoard.closest(".control-bar")).not.toBeNull();
      cleanup();

      // Sign-in and a password reset draw their bar without the wallet, and with no friends to show.
      render(<><ChallengeToast /><BackLink showWallet={false} /></>);
      expect(await screen.findByTestId(navTestid.back)).toBeInTheDocument();
      await waitFor(() => {
        expect(screen.getByTestId("language-nav")).toBeInTheDocument();
      });
      expect(screen.queryByTestId(friendsTestid.button)).toBeNull();
    },
    60_000,
  );
});

describe("#2645 the board's Friends button on a phone", () => {
  it(
    "#2645 on a phone the board's Friends button is a labeled row of the match menu, and its panel opens as the menu closes",
    async () => {
      signIn("tok-friends");
      serve(world());
      const restore = stubPhoneBar(true);
      try {
        render(<><ChallengeToast /><Board view={baseView()} highlight={NO_HIGHLIGHT} /></>);
        const row = await button();
        const menu = screen.getByTestId("match-menu-panel");

        expect(menu).toContainElement(row);
        expect(row).toHaveClass("friends-button--menu");
        expect(row).toHaveTextContent("Friends");
        expect(row.closest(".control-bar")).not.toBeNull();

        fireEvent.click(screen.getByTestId("match-menu"));
        expect(menu).toBeVisible();
        fireEvent.click(row);

        expect(await screen.findByTestId(friendsTestid.panel, undefined, SLOW)).toBeInTheDocument();
        expect(menu).not.toBeVisible();

        // Closing the panel puts the focus on ⋯, the row being out of reach in the closed menu.
        fireEvent.click(screen.getByTestId(friendsTestid.close));
        expect(screen.queryByTestId(friendsTestid.panel)).toBeNull();
        expect(screen.getByTestId("match-menu")).toHaveFocus();
      } finally {
        restore();
      }
    },
    TEST_MS,
  );
});

describe("#1831 who sees the button", () => {
  it(
    "#1831 a tab with no session shows no button and reads no challenges",
    async () => {
      vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
      const server = serve(world({ incoming: [challenge("ch-1", ANN)] }));
      shell();
      await settle({ advance: 10_000 });
      expect(screen.queryByTestId(friendsTestid.button)).toBeNull();
      expect(server.to("GET", "/api/challenges")).toHaveLength(0);
    },
    TEST_MS,
  );

  it(
    "#1831 a session the server refuses (an account not yet active, 403) shows no button",
    async () => {
      vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
      signIn("tok-pending");
      const server = serve(world(), (call) =>
        call.path === "/api/challenges" ? refusal(403, "forbidden", "Your account is not active yet.") : undefined,
      );
      shell();
      await settle({ advance: 10_000 });
      expect(screen.queryByTestId(friendsTestid.button)).toBeNull();
      expect(server.to("GET", "/api/challenges").length).toBeGreaterThan(0);
    },
    TEST_MS,
  );

  it(
    "#1831 a server whose challenges read fails shows no button until it answers",
    async () => {
      vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
      signIn("tok-down");
      serve(world(), (call) => (call.path === "/api/challenges" ? refusal(500, "internal", "boom") : undefined));
      shell();
      await settle({ advance: 10_000 });
      expect(screen.queryByTestId(friendsTestid.button)).toBeNull();
    },
    TEST_MS,
  );
});

describe("#1831 the badge", () => {
  it(
    "#1831 the badge sums challenges and unread, then adds the requests the friends list last held",
    async () => {
      signIn("tok-badge");
      serve(
        world({
          incoming: [challenge("ch-1", ANN), { ...challenge("ch-2", BOB), roomCode: "MNP567" }],
          unread: { "f-ann": 2, "f-bob": 1 },
          requests: [request("r-1", { profileId: "f-dee", username: "Dee#5" })],
        }),
      );
      shell();
      const badge = await screen.findByTestId(friendsTestid.badge, undefined, SLOW);
      // Two challenges and three unread messages: the one request is not read until a list is.
      await waitFor(() => {
        expect(screen.getByTestId(friendsTestid.badge)).toHaveTextContent("5");
      }, SLOW);
      expect(screen.getByTestId(friendsTestid.button)).toHaveAccessibleName("Friends, 5 waiting for you");
      expect(badge).toHaveAttribute("aria-hidden", "true");

      fireEvent.click(screen.getByTestId(friendsTestid.button));
      await screen.findByTestId(friendsTestid.panel, undefined, SLOW);
      await waitFor(() => {
        expect(screen.getByTestId(friendsTestid.badge)).toHaveTextContent("6");
      }, SLOW);
      expect(screen.getByTestId(friendsTestid.button)).toHaveAccessibleName("Friends, 6 waiting for you");

      fireEvent.click(screen.getByTestId(friendsTestid.close));
      expect(screen.getByTestId(friendsTestid.badge)).toHaveTextContent("6");
    },
    TEST_MS,
  );

  it(
    "#1831 with nothing waiting there is no badge, and the button is named Friends",
    async () => {
      signIn("tok-quiet");
      serve(world());
      shell();
      const found = await button();
      expect(found).toHaveAccessibleName("Friends");
      expect(found).toHaveAttribute("aria-haspopup", "dialog");
      expect(found).toHaveAttribute("aria-expanded", "false");
      expect(screen.queryByTestId(friendsTestid.badge)).toBeNull();
    },
    TEST_MS,
  );
});

describe("#1831 the panel", () => {
  it(
    "#1831 the panel lists friends online first, with Challenge, Watch and Chat as Play together decides them",
    async () => {
      signIn("tok-panel");
      serve(world({ unread: { "f-ann": 2 } }));
      shell();
      const panel = await openPanel();
      expect(panel).toHaveAttribute("role", "dialog");
      expect(panel).toHaveAttribute("aria-modal", "true");
      expect(within(panel).getByRole("heading", { name: "Friends" })).toBeInTheDocument();

      await within(panel).findByText(wholeText("Ann#2"), undefined, SLOW);
      const rows = within(panel).getAllByTestId(togetherTestid.friendRow);
      expect(rows.map((row) => row.getAttribute("data-profile-id"))).toEqual(["f-ann", "f-bob", "f-cy"]);
      expect(rows.map((row) => row.getAttribute("data-status"))).toEqual(["online", "inMatch", "offline"]);

      const [ann, bob, cy] = rows as [HTMLElement, HTMLElement, HTMLElement];
      expect(within(ann).getByTestId(togetherTestid.challenge)).toBeEnabled();
      expect(within(ann).queryByTestId(togetherTestid.watch)).toBeNull();
      expect(within(ann).getByTestId(togetherTestid.chatOpen)).toBeInTheDocument();
      expect(within(ann).getByTestId(togetherTestid.unread)).toHaveTextContent("2");
      expect(within(bob).queryByTestId(togetherTestid.challenge)).toBeNull();
      expect(within(bob).getByTestId(togetherTestid.watch)).toBeInTheDocument();
      expect(within(bob).getByTestId(togetherTestid.chatOpen)).toBeInTheDocument();
      expect(within(cy).queryByTestId(togetherTestid.challenge)).toBeNull();
      expect(within(cy).queryByTestId(togetherTestid.watch)).toBeNull();
      expect(within(cy).getByTestId(togetherTestid.chatOpen)).toBeInTheDocument();
      // Remove and Block are Play together's.
      expect(within(panel).queryByTestId(togetherTestid.remove)).toBeNull();
      expect(within(panel).queryByTestId(togetherTestid.block)).toBeNull();

      expect(within(panel).getByText(wholeText("Me#1"))).toBeInTheDocument();
      const all = within(panel).getByTestId(friendsTestid.all);
      expect(all).toHaveAttribute("href", "/together");
    },
    TEST_MS,
  );

  it(
    "#2115 the panel's friends are a rows skeleton while the list is read, then the friends",
    async () => {
      signIn("tok-panel");
      serve(world());
      shell();
      fireEvent.click(await button());
      // Right after the click the stubbed read has not answered yet.
      const panel = screen.getByTestId(friendsTestid.panel);
      expectSkeleton(within(panel).getByText("Loading your friends…"), "rows");

      await within(panel).findByText(wholeText("Ann#2"), undefined, SLOW);
      expect(within(panel).getAllByTestId(togetherTestid.friendRow)).toHaveLength(3);
      expect(within(panel).queryByText("Loading your friends…")).toBeNull();
      expect(within(panel).queryByTestId(skeletonTestid.root)).toBeNull();
    },
    TEST_MS,
  );

  it(
    "#1831 a panel with no friends says so, and still offers the add form",
    async () => {
      signIn("tok-lonely");
      serve(world({ friends: [] }));
      shell();
      const panel = await openPanel();
      expect(await within(panel).findByTestId(togetherTestid.noFriends, undefined, SLOW)).toBeInTheDocument();
      expect(within(panel).getByTestId(togetherTestid.addInput)).toBeInTheDocument();
    },
    TEST_MS,
  );

  it(
    "#1831 the add form shows the server's refusal in its own words, and a request sent",
    async () => {
      signIn("tok-add");
      let refused = true;
      const server = serve(world(), (call) => {
        if (call.method !== "POST" || call.path !== "/api/friends/requests") return undefined;
        return refused ? refusal(404, "not_found", "No player has that username.") : { status: 201, body: { requestId: "r-9" } };
      });
      shell();
      const panel = await openPanel();
      const input = within(panel).getByTestId(togetherTestid.addInput);

      fireEvent.change(input, { target: { value: "ghost#1" } });
      fireEvent.click(within(panel).getByTestId(togetherTestid.addSubmit));
      expect(await within(panel).findByTestId(togetherTestid.addError, undefined, SLOW)).toHaveTextContent("No player has that username.");
      expect(server.to("POST", "/api/friends/requests")[0]?.body).toEqual({ username: "ghost#1" });

      refused = false;
      fireEvent.change(input, { target: { value: "dee#5" } });
      fireEvent.click(within(panel).getByTestId(togetherTestid.addSubmit));
      expect(await within(panel).findByTestId(togetherTestid.addSent, undefined, SLOW)).toHaveTextContent("dee#5");
      expect(within(panel).queryByTestId(togetherTestid.addError)).toBeNull();
    },
    TEST_MS,
  );

  it(
    "#1831 Copy puts the player's name#tag on the clipboard",
    async () => {
      signIn("tok-copy");
      serve(world());
      shell();
      const panel = await openPanel();
      const writeText = vi.fn(async () => {});
      Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });

      const copy = await within(panel).findByTestId(friendsTestid.copy, undefined, SLOW);
      expect(copy).toHaveTextContent("Copy");
      await act(async () => {
        fireEvent.click(copy);
        await Promise.resolve();
      });
      expect(writeText).toHaveBeenCalledWith("Me#1");
      await waitFor(() => {
        expect(copy).toHaveTextContent("Copied");
      });
    },
    TEST_MS,
  );

  it(
    "#1831 incoming requests offer Accept and Decline, and the list is read again after one",
    async () => {
      signIn("tok-requests");
      const state = world({ friends: [ANN], requests: [request("r-1", { profileId: "f-dee", username: "Dee#5" })] });
      const server = serve(state, (call) => {
        if (call.method === "POST" && call.path === "/api/friends/requests/r-1/accept") {
          state.requests = [];
          state.friends = [...state.friends, friend("f-dee", "Dee#5", "online")];
          return { status: 200, body: { friend: friend("f-dee", "Dee#5", "online") } };
        }
        return undefined;
      });
      shell();
      const panel = await openPanel();
      const row = await within(panel).findByTestId(togetherTestid.requestRow, undefined, SLOW);
      expect(row).toHaveAttribute("data-direction", "incoming");
      expect(within(row).getByTestId(togetherTestid.requestDecline)).toBeEnabled();
      expect(within(row).queryByTestId(togetherTestid.requestBlock)).toBeNull();

      fireEvent.click(within(row).getByTestId(togetherTestid.requestAccept));
      await waitFor(() => {
        expect(server.to("POST", "/api/friends/requests/r-1/accept")).toHaveLength(1);
        expect(within(panel).queryByTestId(togetherTestid.requestRow)).toBeNull();
      }, SLOW);
      await waitFor(() => {
        expect(within(panel).getAllByTestId(togetherTestid.friendRow).map((entry) => entry.getAttribute("data-profile-id"))).toEqual(["f-ann", "f-dee"]);
      }, SLOW);
      expect(screen.queryByTestId(friendsTestid.badge)).toBeNull();
    },
    TEST_MS,
  );

  it(
    "#1831 a request declined from the panel is declined",
    async () => {
      signIn("tok-decline");
      const state = world({ friends: [], requests: [request("r-2", { profileId: "f-dee", username: "Dee#5" })] });
      const server = serve(state, (call) => {
        if (call.method === "POST" && call.path === "/api/friends/requests/r-2/decline") {
          state.requests = [];
          return { status: 204 };
        }
        return undefined;
      });
      shell();
      const panel = await openPanel();
      fireEvent.click(await within(panel).findByTestId(togetherTestid.requestDecline, undefined, SLOW));
      await waitFor(() => {
        expect(server.to("POST", "/api/friends/requests/r-2/decline")).toHaveLength(1);
        expect(within(panel).queryByTestId(togetherTestid.requestRow)).toBeNull();
      }, SLOW);
    },
    TEST_MS,
  );

  it(
    "#1691 a failed friends list says why and offers Try again, which lists the friends",
    async () => {
      signIn("tok-retry");
      let down = true;
      const server = serve(world(), (call) =>
        call.method === "GET" && call.path === "/api/friends" && down ? refusal(500, "internal", "Something went wrong.") : undefined,
      );
      shell();
      const panel = await openPanel();
      expect(await within(panel).findByTestId(friendsTestid.listError, undefined, SLOW)).toHaveTextContent("Something went wrong.");
      expect(within(panel).queryAllByTestId(togetherTestid.friendRow)).toHaveLength(0);

      down = false;
      fireEvent.click(within(panel).getByTestId(friendsTestid.listRetry));
      await within(panel).findByText(wholeText("Ann#2"), undefined, SLOW);
      expect(within(panel).queryByTestId(friendsTestid.listError)).toBeNull();
      expect(server.to("GET", "/api/friends")).toHaveLength(2);
    },
    TEST_MS,
  );

  it(
    "#1831 Watch takes the player to the game and closes the panel",
    async () => {
      signIn("tok-watch");
      serve(world());
      shell();
      const panel = await openPanel();
      await within(panel).findByText(wholeText("Bob#3"), undefined, SLOW);
      fireEvent.click(within(panel).getByTestId(togetherTestid.watch));
      expect(screen.queryByTestId(friendsTestid.panel)).toBeNull();
      expect(window.location.pathname).toBe("/watch/m-bob");
    },
    TEST_MS,
  );

  it(
    "#1831 All of Play together closes the panel and goes to /together",
    async () => {
      signIn("tok-all");
      at("/replays");
      serve(world());
      shell();
      const panel = await openPanel();
      fireEvent.click(within(panel).getByTestId(friendsTestid.all));
      expect(screen.queryByTestId(friendsTestid.panel)).toBeNull();
      expect(window.location.pathname).toBe("/together");
    },
    TEST_MS,
  );
});

describe("#1831 the panel is a modal for the keyboard", () => {
  it(
    "#1831 Escape closes the panel and the focus returns to the button",
    async () => {
      signIn("tok-escape");
      serve(world());
      shell();
      const opener = await button();
      const panel = await openPanel();
      expect(opener).toHaveAttribute("aria-expanded", "true");
      expect(opener).toHaveAttribute("aria-controls", panel.id);
      expect(panel).toContainElement(document.activeElement as HTMLElement);

      (document.activeElement as HTMLElement | null)?.blur();
      fireEvent.keyDown(document.body, { key: "Escape" });
      expect(screen.queryByTestId(friendsTestid.panel)).toBeNull();
      expect(opener).toHaveFocus();
      expect(opener).toHaveAttribute("aria-expanded", "false");
    },
    TEST_MS,
  );

  it(
    "R2127000 Back closes the panel, the focus returns to the button and the address stays",
    async () => {
      signIn("tok-back");
      serve(world());
      window.history.pushState(null, "", "/play");
      shell();
      const opener = await button();
      await openPanel();

      await pressBack();
      await waitFor(() => {
        expect(screen.queryByTestId(friendsTestid.panel)).toBeNull();
      }, SLOW);
      expect(opener).toHaveFocus();
      expect(opener).toHaveAttribute("aria-expanded", "false");
      expect(window.location.pathname).toBe("/play");
    },
    TEST_MS,
  );

  it(
    "#1831 Tab and Shift+Tab stay inside the panel, wrapping, and a click on the scrim closes it, one inside it does not",
    async () => {
      signIn("tok-tab");
      serve(world());
      shell();
      const opener = await button();
      const panel = await openPanel();
      const close = within(panel).getByTestId(friendsTestid.close);
      const all = within(panel).getByTestId(friendsTestid.all);
      expect(close).toHaveFocus();

      fireEvent.keyDown(document.activeElement ?? document.body, { key: "Tab", shiftKey: true });
      expect(all).toHaveFocus();
      fireEvent.keyDown(document.activeElement ?? document.body, { key: "Tab" });
      expect(close).toHaveFocus();

      const behind = screen.getByTestId(friendsTestid.button);
      behind.focus();
      fireEvent.keyDown(behind, { key: "Tab" });
      expect(panel).toContainElement(document.activeElement as HTMLElement);

      fireEvent.click(panel);
      expect(screen.getByTestId(friendsTestid.panel)).toBeInTheDocument();
      fireEvent.click(must(document.querySelector<HTMLElement>(".friends-panel-scrim"), "the scrim"));
      expect(screen.queryByTestId(friendsTestid.panel)).toBeNull();
      expect(opener).toHaveFocus();
    },
    TEST_MS,
  );
});

describe("#1831 challenging from the panel", () => {
  it(
    "#1831 a challenge sent from the panel keeps the player on the screen they were on",
    async () => {
      signIn("tok-challenge");
      at("/replays");
      const server = serve(world(), (call) =>
        call.method === "POST" && call.path === "/api/challenges" ? { status: 201, body: { challengeId: "ch-9", roomCode: "ROOM01" } } : undefined,
      );
      shell();
      const opener = await button();
      const panel = await openPanel();
      await within(panel).findByText(wholeText("Ann#2"), undefined, SLOW);
      const annRow = within(panel)
        .getAllByTestId(togetherTestid.friendRow)
        .find((row) => row.getAttribute("data-profile-id") === "f-ann");
      fireEvent.click(within(must(annRow, "Ann's row")).getByTestId(togetherTestid.challenge));

      const form = await screen.findByTestId(togetherTestid.challengeForm, undefined, SLOW);
      expect(screen.queryByTestId(friendsTestid.panel)).toBeNull();
      await waitFor(() => {
        expect(screen.getByTestId(togetherTestid.challengeSend)).toHaveFocus();
      }, SLOW);
      expect(form).toHaveAttribute("role", "dialog");

      fireEvent.click(screen.getByTestId(togetherTestid.challengeSend));
      await waitFor(() => {
        expect(screen.queryByTestId(togetherTestid.challengeForm)).toBeNull();
      }, SLOW);
      expect(server.to("POST", "/api/challenges")[0]?.body).toEqual({ mode: "bo1", deckId: "d-aggro", friendId: "f-ann" });
      expect(window.location.pathname).toBe("/replays");
      expect(opener).toHaveFocus();
      stopFollowingChallenge();
    },
    TEST_MS,
  );

  it(
    "R2127000 Back closes the challenge dialog the panel opened, sending nothing",
    async () => {
      signIn("tok-challenge-back");
      window.history.pushState(null, "", "/replays");
      const server = serve(world(), (call) =>
        call.method === "POST" && call.path === "/api/challenges" ? { status: 201, body: { challengeId: "ch-9", roomCode: "ROOM01" } } : undefined,
      );
      shell();
      const opener = await button();
      const panel = await openPanel();
      await within(panel).findByText(wholeText("Ann#2"), undefined, SLOW);
      const annRow = within(panel)
        .getAllByTestId(togetherTestid.friendRow)
        .find((row) => row.getAttribute("data-profile-id") === "f-ann");
      fireEvent.click(within(must(annRow, "Ann's row")).getByTestId(togetherTestid.challenge));
      await screen.findByTestId(togetherTestid.challengeForm, undefined, SLOW);
      await waitFor(() => {
        expect(screen.getByTestId(togetherTestid.challengeSend)).toHaveFocus();
      }, SLOW);

      await pressBack();
      await waitFor(() => {
        expect(screen.queryByTestId(togetherTestid.challengeForm)).toBeNull();
      }, SLOW);
      expect(screen.queryByTestId(friendsTestid.panel)).toBeNull();
      expect(server.to("POST", "/api/challenges")).toHaveLength(0);
      expect(window.location.pathname).toBe("/replays");
      expect(opener).toHaveFocus();
    },
    TEST_MS,
  );

  it(
    "#1831 in a game, Challenge is disabled with the server's sentence, and Watch and Chat are not",
    async () => {
      signIn("tok-ingame");
      const cases = [
        { me: { currentMatchId: "m-1" }, words: "finish your current match first" },
        { me: { currentSeriesId: "s-1" }, words: "Finish your Conquest series first." },
        { me: { currentDuosMatchId: "d-1" }, words: "Finish your co-op match first." },
      ] as const;
      for (const entry of cases) {
        const state = world();
        state.me = { ...state.me, ...entry.me };
        serve(state);
        shell();
        const panel = await openPanel();
        await within(panel).findByText(wholeText("Ann#2"), undefined, SLOW);
        await waitFor(() => {
          expect(within(panel).getAllByTestId(togetherTestid.challenge)[0]).toBeDisabled();
        }, SLOW);
        const challenge = within(panel).getAllByTestId(togetherTestid.challenge)[0];
        expect(challenge).toHaveAttribute("title", entry.words);
        expect(within(panel).getAllByTestId(togetherTestid.chatOpen).length).toBeGreaterThan(0);
        cleanup();
      }
    },
    TEST_MS,
  );
});
