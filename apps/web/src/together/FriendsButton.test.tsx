// #1831: the Friends button on every signed-in screen (R3728000: the menus' shell's navigation; the top bar
// off the shell; a board's control bar), and its panel (R3728001: the shell's drawer). The server is a
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
import { keepRead } from "../net/readCache.ts";
import { forgetRoomLink } from "../net/roomLink.ts";
import { skeletonTestid } from "../skeleton/Skeleton.tsx";
import { expectEmptyState } from "../test/empty.ts";
import { pressBack } from "../test/history.ts";
import { openFriendMenu, pressFriendMenuItem } from "../test/friendMenu.ts";
import { settle } from "../test/settle.ts";
import { expectSkeleton } from "../test/skeleton.ts";
import { menuShellTestid } from "../shell/testids.ts";
import { stopFollowingChallenge } from "./follow.ts";
import { ChallengeToast } from "./ChallengeToast.tsx";
import { UNDO_TOAST_UNDO } from "../undo/UndoToast.tsx";
import { challengeKindTestid, chatTestid, friendsTestid, togetherTestid } from "./testids.ts";
import { FriendsButton } from "./FriendsButton.tsx";
import { tabStops } from "./modalKeys.ts";

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
    if (method === "GET" && path === "/api/social") {
      return { status: 200, body: { incoming: state.incoming, outgoing: state.outgoing, unread: state.unread } };
    }
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

describe("#1831 R3728000 where the Friends button sits", () => {
  it(
    "R3728000 every signed-in menu screen has Friends in the shell's navigation and none in its top bar; practice, a board and sign-in keep theirs",
    async () => {
      signIn("tok-friends");
      serve(world());

      for (const path of ["/", "/play", "/decks", "/collection", "/account", "/replays", "/together"]) {
        at(path);
        render(<App />);
        const found = await button();
        // The screen's own top bar is drawn (the shell's navigation does not wait for the screen's gate).
        const gear = await screen.findByTestId("settings-open-nav", undefined, SLOW);
        // The last of the shell's five destinations, after Shop, wearing the item's look (shell/menu-shell.css).
        expect(screen.getByTestId(menuShellTestid.nav), path).toContainElement(found);
        expect(found.previousElementSibling, path).toHaveAttribute("data-testid", menuShellTestid.shop);
        expect(found, path).toHaveClass("friends-button--rail", "menu-shell__item");
        expect(found, path).not.toHaveClass("top-bar-button");
        expect(found, path).toHaveTextContent("Friends");
        // One Friends on the screen: the top bar (and Home's corner) no longer draws its own.
        expect(screen.getAllByTestId(friendsTestid.button), path).toHaveLength(1);
        for (const bar of document.querySelectorAll(".screen-nav, .landing-corner")) expect(bar, path).not.toContainElement(found);
        // #3189: the top bar's gear still wears its one skin, routes/top-bar.css.
        expect(gear, path).toHaveClass("top-bar-button", "top-bar-button--round");
        cleanup();
      }

      // Practice is a board's path, outside the shell: its top bar keeps the button just before its language button.
      at("/practice");
      render(<App />);
      const inBar = await button();
      expect(screen.queryByTestId(menuShellTestid.root)).toBeNull();
      expect(inBar.nextElementSibling).toHaveAttribute("data-testid", "language-nav");
      expect(inBar).toHaveClass("top-bar-button", "top-bar-button--round");
      expect(inBar.nextElementSibling).toHaveClass("top-bar-button");
      cleanup();

      // A board's control bar, which every match, Duos, replay and watch screen draws.
      render(<><ChallengeToast /><Board view={baseView()} highlight={NO_HIGHLIGHT} /></>);
      const onBoard = await button();
      expect(onBoard.nextElementSibling).toHaveAttribute("data-testid", "language-game");
      expect(onBoard.closest(".control-bar")).not.toBeNull();
      // #3189: the board's control bar draws its own buttons, as it did.
      expect(onBoard).not.toHaveClass("top-bar-button");
      expect(onBoard.nextElementSibling).not.toHaveClass("top-bar-button");
      expect(screen.getByTestId("settings-open-game")).not.toHaveClass("top-bar-button");
      cleanup();

      // Sign-in and a password reset draw their bar without the wallet, and with no friends to show.
      render(<><ChallengeToast /><BackLink showWallet={false} /></>);
      // #3189: Back is the bar's dark pill, and stays the link-button it was.
      expect(await screen.findByTestId(navTestid.back)).toHaveClass("link-button", "top-bar-button", "top-bar-button--text");
      await waitFor(() => {
        expect(screen.getByTestId("language-nav")).toBeInTheDocument();
      });
      expect(screen.queryByTestId(friendsTestid.button)).toBeNull();
    },
    60_000,
  );

  it(
    "R3728000 signed out, the shell's Friends is a link to Play together",
    async () => {
      serve(world());
      at("/almanac");
      render(<App />);
      const link = await screen.findByTestId(menuShellTestid.friends, undefined, SLOW);
      expect(link.tagName).toBe("A");
      expect(link).toHaveAttribute("href", "/together");
      expect(screen.getByTestId(menuShellTestid.nav)).toContainElement(link);
      expect(screen.queryByTestId(friendsTestid.button)).toBeNull();
    },
    TEST_MS,
  );

  it(
    "R3728001 the rail's Friends opens the drawer in place and Escape, Back and the scrim close it",
    async () => {
      signIn("tok-drawer");
      serve(world());
      at("/decks");
      render(<App />);
      const opener = await button();
      const open = async (): Promise<HTMLElement> => {
        fireEvent.click(opener);
        const panel = await screen.findByTestId(friendsTestid.panel, undefined, SLOW);
        const scrim = must(panel.parentElement, "the scrim");
        // The Dialog's drawer, not its sheet, and the screen under it is the one it opened on.
        expect(scrim).toHaveClass("dialog-scrim");
        expect(scrim).toHaveAttribute("data-drawer");
        expect(scrim).not.toHaveAttribute("data-sheet");
        expect(window.location.pathname).toBe("/decks");
        expect(screen.getByTestId(menuShellTestid.root)).toBeInTheDocument();
        return panel;
      };
      const closed = (): void => {
        expect(screen.queryByTestId(friendsTestid.panel)).toBeNull();
        expect(window.location.pathname).toBe("/decks");
        expect(opener).toHaveAttribute("aria-expanded", "false");
      };

      await open();
      fireEvent.keyDown(document.body, { key: "Escape" });
      closed();

      await open();
      await pressBack();
      await waitFor(closed, SLOW);

      const panel = await open();
      fireEvent.click(panel);
      expect(screen.getByTestId(friendsTestid.panel)).toBeInTheDocument();
      fireEvent.click(must(document.querySelector<HTMLElement>(".dialog-scrim[data-drawer]"), "the scrim"));
      closed();
      expect(opener).toHaveFocus();
    },
    TEST_MS,
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
        expect(row).not.toHaveClass("top-bar-button");
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
      expect(server.to("GET", "/api/social")).toHaveLength(0);
    },
    TEST_MS,
  );

  it(
    "#1831 a session the server refuses (an account not yet active, 403) shows no button",
    async () => {
      vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
      signIn("tok-pending");
      const server = serve(world(), (call) =>
        call.path === "/api/social" ? refusal(403, "forbidden", "Your account is not active yet.") : undefined,
      );
      shell();
      await settle({ advance: 10_000 });
      expect(screen.queryByTestId(friendsTestid.button)).toBeNull();
      expect(server.to("GET", "/api/social").length).toBeGreaterThan(0);
    },
    TEST_MS,
  );

  it(
    "#1831 a server whose challenges read fails shows no button until it answers",
    async () => {
      vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
      signIn("tok-down");
      serve(world(), (call) => (call.path === "/api/social" ? refusal(500, "internal", "boom") : undefined));
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
    "R3732002 the panel groups the friends in a game, online and offline with counts, one primary button on each row and the rest in ⋯",
    async () => {
      signIn("tok-panel");
      serve(world({ unread: { "f-ann": 2 } }));
      shell();
      const panel = await openPanel();
      expect(panel).toHaveAttribute("role", "dialog");
      expect(panel).toHaveAttribute("aria-modal", "true");
      expect(within(panel).getByRole("heading", { name: "Friends" })).toBeInTheDocument();

      await within(panel).findByText(wholeText("Ann#2"), undefined, SLOW);
      // The groups, in order, each with its count; Offline is collapsed, so Cy is not drawn yet.
      const groups = within(panel).getAllByTestId(togetherTestid.group);
      expect(groups.map((group) => group.getAttribute("data-group"))).toEqual(["inGame", "online", "offline"]);
      expect(groups.map((group) => within(group).getByRole("heading").textContent?.replace(/[▸▾]\s*/u, ""))).toEqual([
        "In a game (1)",
        "Online (1)",
        "Offline (1)",
      ]);
      const toggle = within(panel).getByTestId(togetherTestid.offlineToggle);
      expect(toggle).toHaveAttribute("aria-expanded", "false");
      expect(within(panel).getAllByTestId(togetherTestid.friendRow).map((row) => row.getAttribute("data-profile-id"))).toEqual(["f-bob", "f-ann"]);

      fireEvent.click(toggle);
      expect(toggle).toHaveAttribute("aria-expanded", "true");
      const rows = within(panel).getAllByTestId(togetherTestid.friendRow);
      expect(rows.map((row) => row.getAttribute("data-profile-id"))).toEqual(["f-bob", "f-ann", "f-cy"]);
      expect(rows.map((row) => row.getAttribute("data-status"))).toEqual(["inMatch", "online", "offline"]);

      // One primary button each: Watch for the game that can be watched, Challenge for the free friend, Daily for the offline one.
      const [bob, ann, cy] = rows as [HTMLElement, HTMLElement, HTMLElement];
      expect(within(bob).getByTestId(togetherTestid.watch)).toBeInTheDocument();
      expect(within(bob).queryByTestId(togetherTestid.challenge)).toBeNull();
      expect(within(ann).getByTestId(togetherTestid.challenge)).toBeEnabled();
      expect(within(ann).getByTestId(togetherTestid.challenge)).toHaveAttribute("data-action", "challenge");
      expect(within(ann).getByTestId(togetherTestid.challenge)).toHaveAttribute("data-live", "true");
      expect(within(ann).queryByTestId(togetherTestid.watch)).toBeNull();
      expect(within(cy).getByTestId(togetherTestid.challenge)).toHaveAttribute("data-action", "daily");
      expect(within(cy).getByTestId(togetherTestid.challenge)).toHaveAttribute("data-live", "false");
      for (const row of rows) expect(within(row).getAllByRole("button").filter((control) => control.className.includes("button-primary"))).toHaveLength(0);

      // What each says they are doing, in words.
      expect(bob).toHaveTextContent("In a game");
      expect(ann).toHaveTextContent("Online");

      // The rest is in ⋯: nothing of it is on the row until the menu opens, and Chat carries the unread count.
      for (const row of rows) {
        expect(within(row).queryByTestId(togetherTestid.chatOpen)).toBeNull();
        expect(within(row).queryByTestId(togetherTestid.remove)).toBeNull();
      }
      expect(within(ann).getByTestId(togetherTestid.menuUnread)).toHaveTextContent("2");
      const menu = openFriendMenu(ann);
      expect(within(ann).getByTestId(togetherTestid.menu)).toHaveAttribute("aria-expanded", "true");
      expect(within(menu).getAllByRole("menuitem").map((item) => item.getAttribute("data-testid"))).toEqual([
        togetherTestid.chatOpen,
        togetherTestid.daily,
        togetherTestid.lobby,
        togetherTestid.remove,
        togetherTestid.block,
      ]);
      expect(within(menu).getByTestId(togetherTestid.unread)).toHaveTextContent("2");
      // A friend in a game: the primary is Watch, so the menu has none; an offline friend has no Invite to lobby.
      expect(within(openFriendMenu(bob)).queryByTestId(togetherTestid.watchItem)).toBeNull();
      expect(within(openFriendMenu(cy)).queryByTestId(togetherTestid.lobby)).toBeNull();

      expect(within(panel).getByText(wholeText("Me#1"))).toBeInTheDocument();
      const all = within(panel).getByTestId(friendsTestid.all);
      expect(all).toHaveAttribute("href", "/together");
      expect(all).toHaveTextContent("See all friends");
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
      // Offline is collapsed: Bob and Ann are drawn, Cy waits behind the toggle.
      expect(within(panel).getAllByTestId(togetherTestid.friendRow)).toHaveLength(2);
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
      expectEmptyState(await within(panel).findByTestId(togetherTestid.noFriends, undefined, SLOW));
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
    "R2881000 a co-op friend's Watch opens the co-op watch",
    async () => {
      signIn("tok-watch");
      const dee = friend("f-dee", "Dee#5", "inMatch", { matchId: "dm-dee", watchable: true, duos: true });
      serve(world({ friends: [dee] }));
      shell();
      const panel = await openPanel();
      await within(panel).findByText(wholeText("Dee#5"), undefined, SLOW);
      const watch = within(panel).getByTestId(togetherTestid.watch);
      expect(watch.getAttribute("data-route")).toContain("coop=1");
      fireEvent.click(watch);
      expect(window.location.pathname).toBe("/watch/dm-dee");
      const search = new URLSearchParams(window.location.search);
      expect(search.get("friend")).toBe("f-dee");
      expect(search.get("coop")).toBe("1");
    },
    TEST_MS,
  );

  it(
    "#1831 See all friends closes the panel and goes to /together",
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

describe("R3732002 the panel's rows, search and ⋯ menus", () => {
  const DI = friend("f-di", "Di#6", "online", { lobbyCode: "ABC234" });
  const FAY = friend("f-fay", "Fay#7", "offline", { lastSeenAt: new Date(Date.now() - 3 * 3_600_000).toISOString() });
  const rowOf = (panel: HTMLElement, profileId: string): HTMLElement =>
    must(
      within(panel)
        .getAllByTestId(togetherTestid.friendRow)
        .find((row) => row.getAttribute("data-profile-id") === profileId),
      `the row of ${profileId}`,
    );

  it(
    "R3732002 Join takes the player to an online friend's open lobby and closes the panel",
    async () => {
      signIn("tok-join");
      serve(world({ friends: [DI, ANN] }));
      shell();
      const panel = await openPanel();
      await within(panel).findByText(wholeText("Di#6"), undefined, SLOW);
      const di = rowOf(panel, "f-di");
      expect(di).toHaveTextContent("In a lobby");
      expect(di).toHaveAttribute("data-primary", "join");
      expect(rowOf(panel, "f-ann")).toHaveTextContent("Online");
      const join = within(di).getByTestId(togetherTestid.join);
      expect(join).toHaveAttribute("data-route", "/lobby/ABC234");
      expect(within(di).queryByTestId(togetherTestid.challenge)).toBeNull();

      fireEvent.click(join);
      expect(screen.queryByTestId(friendsTestid.panel)).toBeNull();
      expect(window.location.pathname).toBe("/lobby/ABC234");
    },
    TEST_MS,
  );

  it(
    "R3732002 an offline friend's row says when they were last seen, and its Daily opens the challenge on Daily",
    async () => {
      signIn("tok-daily");
      serve(world({ friends: [FAY, ANN] }));
      shell();
      const panel = await openPanel();
      await within(panel).findByText(wholeText("Ann#2"), undefined, SLOW);
      // Collapsed: only the online friend is drawn, and the heading counts the offline one.
      expect(within(panel).getAllByTestId(togetherTestid.friendRow)).toHaveLength(1);
      fireEvent.click(within(panel).getByTestId(togetherTestid.offlineToggle));
      const fay = rowOf(panel, "f-fay");
      expect(fay).toHaveTextContent("Last seen 3 h ago");
      expect(within(fay).getByTestId(togetherTestid.challenge)).toHaveAttribute("data-action", "daily");

      fireEvent.click(within(fay).getByTestId(togetherTestid.challenge));
      const form = await screen.findByTestId(togetherTestid.challengeForm, undefined, SLOW);
      expect(form).toHaveAttribute("data-kind", "daily");
      expect(screen.queryByTestId(friendsTestid.panel)).toBeNull();
      cleanup();

      // The menu's Daily challenge of an online friend opens it on Daily too, though Live is open to them.
      serve(world({ friends: [ANN] }));
      shell();
      const again = await openPanel();
      await within(again).findByText(wholeText("Ann#2"), undefined, SLOW);
      pressFriendMenuItem(rowOf(again, "f-ann"), togetherTestid.daily);
      const dailyForm = await screen.findByTestId(togetherTestid.challengeForm, undefined, SLOW);
      expect(dailyForm).toHaveAttribute("data-kind", "daily");
      expect(screen.getByTestId(challengeKindTestid("live"))).toBeEnabled();
    },
    TEST_MS,
  );

  it(
    "R3732002 a search filters the friends by name and opens Offline while it is typed",
    async () => {
      signIn("tok-search");
      serve(world({ friends: [ANN, BOB, CY, FAY] }));
      shell();
      const panel = await openPanel();
      await within(panel).findByText(wholeText("Ann#2"), undefined, SLOW);
      const search = within(panel).getByTestId(togetherTestid.search);

      fireEvent.change(search, { target: { value: " cy " } });
      expect(within(panel).getAllByTestId(togetherTestid.friendRow).map((row) => row.getAttribute("data-profile-id"))).toEqual(["f-cy"]);
      expect(within(panel).getByTestId(togetherTestid.offlineToggle)).toHaveAttribute("aria-expanded", "true");
      expect(within(panel).getByTestId(togetherTestid.group)).toHaveAttribute("data-group", "offline");

      fireEvent.change(search, { target: { value: "nobody" } });
      expect(within(panel).queryAllByTestId(togetherTestid.friendRow)).toHaveLength(0);
      expect(within(panel).getByText("No friend has a name like that.")).toBeInTheDocument();

      fireEvent.change(search, { target: { value: "" } });
      expect(within(panel).getAllByTestId(togetherTestid.friendRow)).toHaveLength(2);
    },
    TEST_MS,
  );

  it(
    "R3732002 Escape and Back close the ⋯ menu alone, a press outside closes it, and an item closes it before it acts",
    async () => {
      signIn("tok-menu");
      serve(world({ friends: [ANN] }));
      window.history.pushState(null, "", "/play");
      shell();
      const panel = await openPanel();
      await within(panel).findByText(wholeText("Ann#2"), undefined, SLOW);
      const ann = rowOf(panel, "f-ann");
      const more = within(ann).getByTestId(togetherTestid.menu);
      expect(more).toHaveAttribute("aria-haspopup", "menu");
      expect(more).toHaveAttribute("aria-label", "More for Ann#2");

      // Escape: the menu goes, the focus is back on ⋯, and the panel stays.
      openFriendMenu(ann);
      expect(within(ann).getByRole("menu")).toBeInTheDocument();
      expect(ann.querySelector('[data-tip-open="true"]')).not.toBeNull();
      expect(within(ann).getAllByRole("menuitem")[0]).toHaveFocus();
      fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
      expect(within(ann).queryByRole("menu")).toBeNull();
      expect(more).toHaveAttribute("aria-expanded", "false");
      expect(more).toHaveFocus();
      expect(screen.getByTestId(friendsTestid.panel)).toBeInTheDocument();
      // The next Escape closes the panel.
      fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
      expect(screen.queryByTestId(friendsTestid.panel)).toBeNull();
      cleanup();

      // Back closes the menu and leaves the panel and the address as they were.
      serve(world({ friends: [ANN] }));
      shell();
      const second = await openPanel();
      await within(second).findByText(wholeText("Ann#2"), undefined, SLOW);
      openFriendMenu(rowOf(second, "f-ann"));
      await pressBack();
      await waitFor(() => {
        expect(within(second).queryByRole("menu")).toBeNull();
      }, SLOW);
      expect(screen.getByTestId(friendsTestid.panel)).toBeInTheDocument();
      expect(window.location.pathname).toBe("/play");

      // A press outside the menu closes it.
      openFriendMenu(rowOf(second, "f-ann"));
      fireEvent.pointerDown(within(second).getByTestId(togetherTestid.search));
      expect(within(second).queryByRole("menu")).toBeNull();
      expect(screen.getByTestId(friendsTestid.panel)).toBeInTheDocument();

      // An item closes the menu, then acts: Chat closes the panel and opens the conversation.
      pressFriendMenuItem(rowOf(second, "f-ann"), togetherTestid.chatOpen);
      expect(screen.queryByTestId(friendsTestid.panel)).toBeNull();
      expect(await screen.findByTestId(chatTestid.panel, undefined, SLOW)).toHaveAttribute("data-friend-id", "f-ann");
    },
    TEST_MS,
  );

  it(
    "R3732002 opened from the shell the chat is the drawer too, in the list's place",
    async () => {
      signIn("tok-chat-drawer");
      serve(world({ friends: [ANN] }));
      at("/decks");
      render(<App />);
      fireEvent.click(await button());
      const panel = await screen.findByTestId(friendsTestid.panel, undefined, SLOW);
      await within(panel).findByText(wholeText("Ann#2"), undefined, SLOW);
      pressFriendMenuItem(rowOf(panel, "f-ann"), togetherTestid.chatOpen);

      const chat = await screen.findByTestId(chatTestid.panel, undefined, SLOW);
      expect(chat.parentElement).toHaveAttribute("data-drawer");
      expect(chat.parentElement).not.toHaveAttribute("data-sheet");
      expect(screen.queryByTestId(friendsTestid.panel)).toBeNull();
      expect(window.location.pathname).toBe("/decks");
    },
    TEST_MS,
  );

  it(
    "R3732002 R2119000 Remove in the menu takes the friend off behind Undo and Undo puts them back; Block still asks first",
    async () => {
      signIn("tok-remove");
      const server = serve(world({ friends: [ANN, BOB] }), (call) =>
        call.method === "DELETE" || (call.method === "POST" && call.path === "/api/blocks") ? { status: 204 } : undefined,
      );
      shell();
      const panel = await openPanel();
      await within(panel).findByText(wholeText("Ann#2"), undefined, SLOW);

      pressFriendMenuItem(rowOf(panel, "f-ann"), togetherTestid.remove);
      expect(within(panel).queryByText(wholeText("Ann#2"))).toBeNull();
      // The toast is inside the panel, so Undo is a stop of its Tab order.
      const undo = within(panel).getByTestId(UNDO_TOAST_UNDO);
      expect(tabStops(panel)).toContain(undo);
      expect(server.to("DELETE", "/api/friends/f-ann")).toHaveLength(0);
      fireEvent.click(undo);
      expect(await within(panel).findByText(wholeText("Ann#2"))).toBeInTheDocument();
      expect(server.to("DELETE", "/api/friends/f-ann")).toHaveLength(0);

      // Block asks first, naming the friend; Keep sends nothing, Block sends one block.
      pressFriendMenuItem(rowOf(panel, "f-ann"), togetherTestid.block);
      const ann = rowOf(panel, "f-ann");
      expect(within(ann).getByRole("alertdialog")).toHaveTextContent("Block Ann#2?");
      expect(server.to("POST", "/api/blocks")).toHaveLength(0);
      fireEvent.click(within(ann).getByTestId(togetherTestid.keep));
      expect(within(rowOf(panel, "f-ann")).getByTestId(togetherTestid.menu)).toBeInTheDocument();
      pressFriendMenuItem(rowOf(panel, "f-ann"), togetherTestid.block);
      fireEvent.click(within(rowOf(panel, "f-ann")).getByTestId(togetherTestid.confirm));
      await waitFor(() => {
        expect(server.to("POST", "/api/blocks")[0]?.body).toEqual({ profileId: "f-ann" });
      }, SLOW);
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
      // R3221000: the panel opens on its ✕, the Dialog's last stop: Tab wraps to the first, Shift+Tab steps back.
      expect(close).toHaveFocus();

      fireEvent.keyDown(document.activeElement ?? document.body, { key: "Tab" });
      expect(tabStops(panel)[0]).toHaveFocus();
      fireEvent.keyDown(document.activeElement ?? document.body, { key: "Tab", shiftKey: true });
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
      fireEvent.click(must(document.querySelector<HTMLElement>(".dialog-scrim"), "the scrim"));
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
      // R3221000: Cancel holds the focus; Send waits for the decks.
      expect(screen.getByTestId(togetherTestid.challengeClose)).toHaveFocus();
      await waitFor(() => {
        expect(screen.getByTestId(togetherTestid.challengeSend)).toBeEnabled();
      }, SLOW);
      expect(form).toHaveAttribute("role", "alertdialog");

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
        expect(screen.getByTestId(togetherTestid.challengeSend)).toBeEnabled();
      }, SLOW);
      expect(screen.getByTestId(togetherTestid.challengeClose)).toHaveFocus();

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
    "#1831 #3727 in a game, Challenge's Live is disabled with the server's sentence, its Daily is not, and Chat stays",
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
          expect(within(panel).getAllByTestId(togetherTestid.challenge)[0]).toHaveAttribute("data-live", "false");
        }, SLOW);
        const challenge = must(within(panel).getAllByTestId(togetherTestid.challenge)[0], "the first Challenge");
        expect(challenge).toBeEnabled();
        const first = must(within(panel).getAllByTestId(togetherTestid.friendRow)[0], "the first row");
        expect(within(openFriendMenu(first)).getByTestId(togetherTestid.chatOpen)).toBeInTheDocument();
        fireEvent.click(challenge);
        const form = await screen.findByTestId(togetherTestid.challengeForm, undefined, SLOW);
        expect(form).toHaveAttribute("data-kind", "daily");
        const live = screen.getByTestId(challengeKindTestid("live"));
        expect(live).toBeDisabled();
        expect(live).toHaveAttribute("title", entry.words);
        expect(screen.getByTestId(togetherTestid.challengeLiveRefusal)).toHaveTextContent(entry.words);
        expect(screen.getByTestId(challengeKindTestid("daily"))).toBeEnabled();
        cleanup();
      }
    },
    TEST_MS,
  );
});

describe("R3207003 the panel opens on the account the page kept", () => {
  /** The path `net/gate.ts` keeps the gate's answer under. */
  const ME_PATH = "/api/auth/me";

  it(
    "R3207003 the panel opens on the account the page kept and sends no /api/auth/me",
    async () => {
      signIn("tok-kept");
      keepRead("tok-kept", ME_PATH, meBody(world().me));
      const server = serve(world());
      shell();
      const panel = await openPanel();

      await within(panel).findByText(wholeText("Me#1"), undefined, SLOW);
      await within(panel).findByText(wholeText("Ann#2"), undefined, SLOW);
      expect(server.to("GET", ME_PATH)).toHaveLength(0);
      expect(within(panel).getAllByTestId(togetherTestid.challenge)[0]).toBeEnabled();
    },
    TEST_MS,
  );

  it(
    "R3207003 a kept account that names a match is read again, for the match may have ended",
    async () => {
      signIn("tok-kept");
      const kept = world();
      kept.me = { ...kept.me, username: { name: "Kept#9", nextChangeAt: null, promptOwed: false }, currentMatchId: "m-1" };
      keepRead("tok-kept", ME_PATH, meBody(kept.me));
      // The server says the match is over.
      const server = serve(world());
      shell();
      const panel = await openPanel();

      await within(panel).findByText(wholeText("Me#1"), undefined, SLOW);
      await within(panel).findByText(wholeText("Ann#2"), undefined, SLOW);
      expect(server.to("GET", ME_PATH)).toHaveLength(1);
      expect(within(panel).queryByText(wholeText("Kept#9"))).toBeNull();
      expect(within(panel).getAllByTestId(togetherTestid.challenge)[0]).toBeEnabled();
      expect(within(panel).getAllByTestId(togetherTestid.challenge)[0]).not.toHaveAttribute("title");
    },
    TEST_MS,
  );

  it(
    "R3207003 a panel opened with nothing kept reads the account itself",
    async () => {
      signIn("tok-fresh");
      const server = serve(world());
      shell();
      const panel = await openPanel();

      await within(panel).findByText(wholeText("Me#1"), undefined, SLOW);
      expect(server.to("GET", ME_PATH)).toHaveLength(1);
    },
    TEST_MS,
  );
});

