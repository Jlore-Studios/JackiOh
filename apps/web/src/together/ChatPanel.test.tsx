// #936 S8 B58: `ChatPanel`, the conversation with one friend. A refusal (B53: empty, too long, not
// friends, rate-limited) shows in the server's words without adding a message. The Friends button's
// panel (#1831) lists friends, each opening this panel. #1898: a failed read says why with Try again.

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CHAT_POLL_MS } from "@jackioh/server-config";

import type { ChatMessage, Friend, PlayerRef } from "../net/social.ts";
import { pressBack } from "../test/history.ts";
import { settle } from "../test/settle.ts";
import { at, must, refusal, signIn, stubServer, wholeText, type Call, type Reply, type StubServer } from "../test/stubServer.ts";
import { ChatPanel } from "./ChatPanel.tsx";
import { SOCIAL_POLL_MS } from "./config.ts";
import { ChallengeToast } from "./ChallengeToast.tsx";
import { FriendsButton } from "./FriendsButton.tsx";
import { friendsTestid, togetherTestid } from "./testids.ts";

const SLOW = { timeout: 5_000 } as const;

const ME: PlayerRef = { profileId: "profile-1", username: "Me#1" };
const ANN: PlayerRef = { profileId: "f-ann", username: "Ann#2" };
const SINCE = "2026-10-01T00:00:00.000Z";

function message(id: string, from: string, body: string, minute: number): ChatMessage {
  const to = from === ME.profileId ? ANN.profileId : ME.profileId;
  return { id, from, to, body, createdAt: `2026-10-14T09:${String(minute).padStart(2, "0")}:00.000Z`, read: true };
}

/** The server's conversation, oldest first: the page answers it newest last. */
function conversation(): ChatMessage[] {
  return [message("msg-1", ANN.profileId, "good luck", 1), message("msg-2", ME.profileId, "you too", 2)];
}

function serve(messages: ChatMessage[], special?: (call: Call) => Reply | undefined): StubServer {
  let next = 100;
  return stubServer((call) => {
    const answer = special?.(call);
    if (answer !== undefined) return answer;
    if (call.path === "/api/auth/me") {
      return {
        status: 200,
        body: { profile: { id: ME.profileId, status: "active" }, needsInviteCode: false, emailVerified: true, currentMatchId: null, email: null },
      };
    }
    if (call.method === "GET" && call.path === `/api/chat/${ANN.profileId}`) return { status: 200, body: { messages, nextCursor: null } };
    if (call.method === "POST" && call.path === `/api/chat/${ANN.profileId}`) {
      next += 1;
      const text = (call.body as { body?: unknown } | undefined)?.body;
      const sent = message(`msg-${String(next)}`, ME.profileId, typeof text === "string" ? text.trim() : "", 30);
      messages.push(sent);
      return { status: 201, body: { message: sent } };
    }
    if (call.method === "GET" && call.path === "/api/chat/unread") return { status: 200, body: { unread: {} } };
    if (call.method === "GET" && call.path === "/api/challenges") return { status: 200, body: { incoming: [], outgoing: [] } };
    return undefined;
  });
}

function shown(): string[] {
  return screen.queryAllByTestId("chat-message").map((element) => element.textContent ?? "");
}

function type(text: string): void {
  fireEvent.change(screen.getByTestId("chat-input"), { target: { value: text } });
}

function send(): void {
  const button = screen.getByTestId("chat-send");
  if ((button as HTMLButtonElement).disabled !== true) fireEvent.click(button);
}

const CHAT = `/api/chat/${ANN.profileId}`;

/** The sends drawn that no answer has replaced yet (R2120000). */
function pending(): HTMLElement[] {
  return screen.queryAllByTestId("chat-pending");
}

/** Every message drawn, the server's and the pending, top to bottom, as its body. */
function bodies(): string[] {
  return Array.from(document.querySelectorAll('[data-testid="chat-message"], [data-testid="chat-pending"]')).map(
    (element) => element.querySelector(".chat-message__body")?.textContent ?? "",
  );
}

/** From now on the stub takes each POST when it is made, but its answer waits for `release()`, oldest first (R2120000). */
function holdSends(): () => void {
  const answer = globalThis.fetch;
  const held: (() => void)[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
      const reply = answer(input, init);
      if (init?.method !== "POST") return reply;
      return new Promise<Response>((resolve) => {
        held.push(() => {
          resolve(reply);
        });
      });
    }),
  );
  return () => {
    held.shift()?.();
  };
}

/** After the first render: the account (if the panel reads one), the first page, its render. */
async function opened(): Promise<void> {
  for (let round = 0; round < 3; round += 1) await settle({ advance: 0 });
}

beforeEach(() => {
  window.localStorage.clear();
  signIn("tok-chat");
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.localStorage.clear();
});

describe("R1384000 earlier pages", () => {
  const FIRST = "1760432400000_00000000-0000-4000-8000-000000000001";
  const SECOND = "1760432400000_00000000-0000-4000-8000-000000000002";

  it("R1384000 Show earlier asks for each earlier page by the cursor the last page answered, and goes once one answers none", async () => {
    const server = serve(conversation(), (call) => {
      if (call.method !== "GET" || call.path !== `/api/chat/${ANN.profileId}`) return undefined;
      switch (call.query.get("before")) {
        case null:
          return { status: 200, body: { messages: conversation(), nextCursor: FIRST } };
        case FIRST:
          return { status: 200, body: { messages: [message("msg-0c", ANN.profileId, "are you there", 0)], nextCursor: SECOND } };
        case SECOND:
          return {
            status: 200,
            body: { messages: [message("msg-0a", ANN.profileId, "hello", 0), message("msg-0b", ME.profileId, "hi", 0)], nextCursor: null },
          };
        default:
          return refusal(400, "bad_request", "not a cursor");
      }
    });
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await waitFor(() => {
      expect(shown()).toHaveLength(2);
    }, SLOW);

    fireEvent.click(await screen.findByTestId("chat-earlier"));
    await waitFor(() => {
      expect(shown()).toHaveLength(3);
    }, SLOW);
    await waitFor(() => {
      expect(screen.getByTestId("chat-earlier")).toBeEnabled();
    }, SLOW);
    fireEvent.click(screen.getByTestId("chat-earlier"));
    await waitFor(() => {
      expect(shown()).toHaveLength(5);
    }, SLOW);

    expect(screen.queryByTestId("chat-earlier")).toBeNull();
    expect(must(shown()[0], "the oldest")).toContain("hello");
    const asked = server
      .to("GET", `/api/chat/${ANN.profileId}`)
      .map((call) => call.query.get("before"))
      .filter((before) => before !== null);
    expect(asked).toEqual([FIRST, SECOND]);
  });
});

describe("B58 the conversation", () => {
  it("B58 opens on the friend's conversation, one chat-message per message, newest last", async () => {
    const server = serve(conversation());
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await waitFor(() => {
      expect(shown()).toHaveLength(2);
    }, SLOW);
    expect(must(shown()[0], "first")).toContain("good luck");
    expect(must(shown()[1], "second")).toContain("you too");
    expect(server.to("GET", `/api/chat/${ANN.profileId}`).length).toBeGreaterThan(0);
  });

  it("B58 an empty conversation shows no message", async () => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    const server = serve([]);
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await waitFor(() => {
      expect(server.to("GET", `/api/chat/${ANN.profileId}`).length).toBeGreaterThan(0);
    }, SLOW);
    await settle({ advance: CHAT_POLL_MS + 1 });
    expect(server.to("GET", `/api/chat/${ANN.profileId}`)).toHaveLength(2);
    expect(shown()).toHaveLength(0);
    expect(screen.getByTestId("chat-input")).toBeInTheDocument();
  });

  it("B58 a conversation the server refuses to read (no longer friends) shows no message", async () => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    const server = serve(conversation(), (call) =>
      call.method === "GET" && call.path === `/api/chat/${ANN.profileId}` ? refusal(404, "not_found", "You can only chat with friends.") : undefined,
    );
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await waitFor(() => {
      expect(server.to("GET", `/api/chat/${ANN.profileId}`).length).toBeGreaterThan(0);
    }, SLOW);
    await settle({ advance: CHAT_POLL_MS + 1 });
    expect(server.to("GET", `/api/chat/${ANN.profileId}`)).toHaveLength(2);
    expect(shown()).toHaveLength(0);
  });

  it("B58 sends what is typed as POST /api/chat/:friendId {body} and shows it", async () => {
    const server = serve(conversation());
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await waitFor(() => {
      expect(shown()).toHaveLength(2);
    }, SLOW);
    type("well played");
    send();
    await waitFor(() => {
      expect(server.to("POST", `/api/chat/${ANN.profileId}`)).toHaveLength(1);
    }, SLOW);
    expect(server.to("POST", `/api/chat/${ANN.profileId}`)[0]?.body).toEqual({ body: "well played" });
    await waitFor(() => {
      expect(shown()).toHaveLength(3);
    }, SLOW);
    expect(must(shown()[2], "the sent message")).toContain("well played");
  });

  it("B58 a too-long message adds no message; one the server refuses is refused in its words", async () => {
    const words = "That message is too long.";
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    const server = serve(conversation(), (call) => (call.method === "POST" ? refusal(422, "too_long", words) : undefined));
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await waitFor(() => {
      expect(shown()).toHaveLength(2);
    }, SLOW);
    type("x".repeat(501));
    send();
    await settle({ advance: CHAT_POLL_MS + 1 });
    if (server.to("POST", `/api/chat/${ANN.profileId}`).length > 0) {
      await waitFor(() => {
        expect(document.body).toHaveTextContent(words);
      }, SLOW);
    }
    expect(shown()).toHaveLength(2);
  });

  it("B58 a blank message adds no message, whether the panel holds it back or the server refuses it", async () => {
    const words = "Type a message first.";
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    const server = serve(conversation(), (call) => (call.method === "POST" ? refusal(422, "empty", words) : undefined));
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await waitFor(() => {
      expect(shown()).toHaveLength(2);
    }, SLOW);
    type("    ");
    send();
    await settle({ advance: CHAT_POLL_MS + 1 });
    expect(server.to("GET", `/api/chat/${ANN.profileId}`)).toHaveLength(2);
    expect(shown()).toHaveLength(2);
  });

  it("B58 a message past the per-minute limit (429) is refused in the server's words and adds no message", async () => {
    const words = "You are sending messages too fast.";
    serve(conversation(), (call) => (call.method === "POST" ? refusal(429, "rate_limited", words) : undefined));
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await waitFor(() => {
      expect(shown()).toHaveLength(2);
    }, SLOW);
    type("hello again");
    send();
    await waitFor(() => {
      expect(document.body).toHaveTextContent(words);
    }, SLOW);
    expect(shown()).toHaveLength(2);
  });

  it("B58 chat to someone who is no longer a friend (404) is refused in the server's words", async () => {
    const words = "You can only chat with friends.";
    serve(conversation(), (call) => (call.method === "POST" ? refusal(404, "not_found", words) : undefined));
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await waitFor(() => {
      expect(shown()).toHaveLength(2);
    }, SLOW);
    type("still there?");
    send();
    await waitFor(() => {
      expect(document.body).toHaveTextContent(words);
    }, SLOW);
    expect(shown()).toHaveLength(2);
  });

  it("B58 the panel's close control calls onClose", async () => {
    serve(conversation());
    const onClose = vi.fn();
    render(<ChatPanel friend={ANN} onClose={onClose} />);
    await waitFor(() => {
      expect(shown()).toHaveLength(2);
    }, SLOW);
    fireEvent.click(screen.getByRole("button", { name: /close/i }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});

describe("B58 polling while open", () => {
  it("B58 reads the conversation again every CHAT_POLL_MS while open", async () => {
    vi.useFakeTimers();
    const server = serve(conversation());
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await opened();
    const reads = (): number => server.to("GET", `/api/chat/${ANN.profileId}`).length;
    const first = reads();
    expect(first).toBeGreaterThan(0);

    await settle({ advance: CHAT_POLL_MS });
    const second = reads();
    expect(second).toBeGreaterThan(first);
    await settle({ advance: CHAT_POLL_MS });
    expect(reads()).toBeGreaterThan(second);
  });

  it("B58 does not read again before CHAT_POLL_MS has passed", async () => {
    vi.useFakeTimers();
    const server = serve(conversation());
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await opened();
    const first = server.to("GET", `/api/chat/${ANN.profileId}`).length;
    await settle({ advance: Math.floor(CHAT_POLL_MS / 2) });
    expect(server.to("GET", `/api/chat/${ANN.profileId}`)).toHaveLength(first);
  });

  it("B58 a message the friend sends between polls appears on the next poll", async () => {
    vi.useFakeTimers();
    const messages = conversation();
    serve(messages);
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await opened();
    expect(shown()).toHaveLength(2);

    messages.push(message("msg-3", ANN.profileId, "rematch?", 3));
    await settle({ advance: CHAT_POLL_MS });
    expect(shown()).toHaveLength(3);
    expect(must(shown()[2], "the new message")).toContain("rematch?");
  });

  it("B58 a closed panel stops polling", async () => {
    vi.useFakeTimers();
    const server = serve(conversation());
    const { unmount } = render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await opened();
    await settle({ advance: CHAT_POLL_MS });
    unmount();
    const before = server.to("GET", `/api/chat/${ANN.profileId}`).length;
    await settle({ advance: CHAT_POLL_MS * 3 });
    expect(server.to("GET", `/api/chat/${ANN.profileId}`)).toHaveLength(before);
  });
});

describe("#1898 a failed read of the conversation", () => {
  /** The newest page's read is refused while `refused.read` holds; a test turns it off. */
  function serveRefusing(messages: ChatMessage[], refused: { read: boolean }): StubServer {
    return serve(messages, (call) =>
      refused.read && call.method === "GET" && call.path === `/api/chat/${ANN.profileId}`
        ? refusal(500, "internal", "Something went wrong.")
        : undefined,
    );
  }

  it("#1898 a failed first read says why and offers Try again, and never says the conversation is empty", async () => {
    vi.useFakeTimers();
    serveRefusing([], { read: true });
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await opened();

    expect(screen.getByTestId("chat-read-error")).toHaveTextContent("Something went wrong.");
    expect(screen.getByTestId("chat-retry")).toBeEnabled();
    expect(screen.queryByText("No messages yet. Say hello!")).toBeNull();
    expect(shown()).toHaveLength(0);

    await settle({ advance: CHAT_POLL_MS });
    expect(screen.getByTestId("chat-read-error")).toHaveTextContent("Something went wrong.");
    expect(screen.queryByText("No messages yet. Say hello!")).toBeNull();
  });

  it("#1898 a later poll that answers clears the failure and shows the conversation", async () => {
    vi.useFakeTimers();
    const refused = { read: true };
    serveRefusing(conversation(), refused);
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await opened();
    expect(screen.getByTestId("chat-read-error")).toBeInTheDocument();

    refused.read = false;
    await settle({ advance: CHAT_POLL_MS });
    expect(screen.queryByTestId("chat-read-error")).toBeNull();
    expect(screen.queryByTestId("chat-retry")).toBeNull();
    expect(shown()).toHaveLength(2);
  });

  it("#1898 Try again reads the conversation at once and clears the failure", async () => {
    vi.useFakeTimers();
    const refused = { read: true };
    const server = serveRefusing([], refused);
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await opened();
    const before = server.to("GET", `/api/chat/${ANN.profileId}`).length;

    refused.read = false;
    fireEvent.click(screen.getByTestId("chat-retry"));
    await settle({ advance: 0 });
    expect(server.to("GET", `/api/chat/${ANN.profileId}`)).toHaveLength(before + 1);
    expect(screen.queryByTestId("chat-read-error")).toBeNull();
    expect(screen.getByText("No messages yet. Say hello!")).toBeInTheDocument();
  });

  it("#1898 a read that answers leaves a refused send's words on screen", async () => {
    const words = "You are sending messages too fast.";
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    serve(conversation(), (call) => (call.method === "POST" ? refusal(429, "rate_limited", words) : undefined));
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await waitFor(() => {
      expect(shown()).toHaveLength(2);
    }, SLOW);
    type("hello again");
    send();
    await waitFor(() => {
      expect(screen.getByTestId("chat-pending")).toHaveTextContent(words);
    }, SLOW);

    await settle({ advance: CHAT_POLL_MS + 1 });
    expect(screen.getByTestId("chat-pending")).toHaveTextContent(words);
    expect(screen.getByTestId("chat-pending")).toHaveAttribute("data-state", "failed");
    expect(screen.queryByTestId("chat-read-error")).toBeNull();
  });
});

describe("R2120000 a message shows at once while it sends", () => {
  it("R2120000 a sent message shows at once as sending, the draft clears, and the player can write the next", async () => {
    vi.useFakeTimers();
    const server = serve(conversation());
    const release = holdSends();
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await opened();

    type("well played");
    send();
    expect(pending()).toHaveLength(1);
    const row = must(pending()[0], "the sending message");
    expect(row).toHaveAttribute("data-state", "sending");
    expect(row).toHaveTextContent("well played");
    expect(row).toHaveTextContent("Sending…");
    expect(screen.getByTestId("chat-input")).toHaveValue("");
    expect(shown()).toHaveLength(2);

    type("gg");
    expect(screen.getByTestId("chat-send")).toBeEnabled();

    await settle({ advance: 0 });
    expect(server.to("POST", CHAT)).toHaveLength(1);
    release();
    await settle({ advance: 0 });
    expect(pending()).toHaveLength(0);
    expect(shown()).toHaveLength(3);
    expect(must(shown()[2], "the answered message")).toContain("well played");
  });

  it("R2120000 the answer replaces the sending message with no duplicate, even when a poll brings it back first", async () => {
    vi.useFakeTimers();
    const server = serve(conversation());
    const release = holdSends();
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await opened();
    const once = (): number => bodies().filter((body) => body === "well played").length;

    type("well played");
    send();
    await settle({ advance: 0 });
    const reads = server.to("GET", CHAT).length;
    await settle({ advance: CHAT_POLL_MS });
    expect(server.to("GET", CHAT).length).toBeGreaterThan(reads);
    expect(once()).toBe(1);
    expect(pending()).toHaveLength(1);

    release();
    await settle({ advance: 0 });
    expect(pending()).toHaveLength(0);
    expect(once()).toBe(1);
    expect(shown()).toHaveLength(3);
    await settle({ advance: CHAT_POLL_MS });
    expect(once()).toBe(1);
  });

  it("R2120000 a refused send shows failed in the server's words, and Retry sends the same body again", async () => {
    vi.useFakeTimers();
    const words = "You are sending messages too fast.";
    const refused = { send: true };
    const server = serve(conversation(), (call) => (refused.send && call.method === "POST" ? refusal(429, "rate_limited", words) : undefined));
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await opened();

    type("hello again");
    send();
    await settle({ advance: 0 });
    const row = must(pending()[0], "the failed message");
    expect(row).toHaveAttribute("data-state", "failed");
    expect(row).toHaveTextContent(words);
    expect(shown()).toHaveLength(2);
    expect(screen.queryByTestId("chat-error")).toBeNull();

    refused.send = false;
    fireEvent.click(within(row).getByTestId("chat-pending-retry"));
    await settle({ advance: 0 });
    expect(server.to("POST", CHAT).map((call) => call.body)).toEqual([{ body: "hello again" }, { body: "hello again" }]);
    expect(pending()).toHaveLength(0);
    expect(shown()).toHaveLength(3);
    expect(must(shown()[2], "the retried message")).toContain("hello again");
  });

  it("R2120000 Discard drops a failed send and sends nothing more", async () => {
    vi.useFakeTimers();
    const server = serve(conversation(), (call) => (call.method === "POST" ? refusal(404, "not_found", "You can only chat with friends.") : undefined));
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await opened();

    type("still there?");
    send();
    await settle({ advance: 0 });
    expect(must(pending()[0], "the failed message")).toHaveAttribute("data-state", "failed");

    fireEvent.click(screen.getByTestId("chat-pending-discard"));
    await settle({ advance: 0 });
    expect(pending()).toHaveLength(0);
    expect(shown()).toHaveLength(2);
    expect(server.to("POST", CHAT)).toHaveLength(1);
  });

  it("R2120000 two quick sends stay in the order sent, after every message the server has, and reach it in that order", async () => {
    vi.useFakeTimers();
    const messages = conversation();
    const server = serve(messages);
    const release = holdSends();
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await opened();

    type("first");
    send();
    type("second");
    send();
    await settle({ advance: 0 });
    expect(pending().map((row) => row.getAttribute("data-state"))).toEqual(["sending", "sending"]);
    expect(server.to("POST", CHAT)).toHaveLength(1);

    messages.push(message("msg-3", ANN.profileId, "rematch?", 3));
    await settle({ advance: CHAT_POLL_MS });
    expect(bodies()).toEqual(["good luck", "you too", "rematch?", "first", "second"]);

    release();
    await settle({ advance: 0 });
    expect(server.to("POST", CHAT)).toHaveLength(2);
    release();
    await settle({ advance: 0 });
    expect(server.to("POST", CHAT).map((call) => call.body)).toEqual([{ body: "first" }, { body: "second" }]);
    expect(pending()).toHaveLength(0);
    expect(bodies()).toEqual(["good luck", "you too", "rematch?", "first", "second"]);
  });
});

describe("B58 chat from the Friends panel", () => {
  const ONLINE: Friend = { ...ANN, status: "online", matchId: null, watchable: false, since: SINCE };
  const OFFLINE: Friend = { profileId: "f-cy", username: "Cy#4", status: "offline", matchId: null, watchable: false, since: SINCE };

  function serveFriends(): StubServer {
    return serve(conversation(), (call) =>
      call.method === "GET" && call.path === "/api/friends"
        ? { status: 200, body: { friends: [OFFLINE, ONLINE], incoming: [], outgoing: [], blocked: [] } }
        : undefined,
    );
  }

  /** The shell's toast (the one poll) and a board's Friends button, with the panel open. */
  async function openPanel(): Promise<HTMLElement> {
    render(
      <>
        <ChallengeToast />
        <FriendsButton placement="game" />
      </>,
    );
    const button = await screen.findByTestId(friendsTestid.button, undefined, SLOW);
    fireEvent.click(button);
    await screen.findByTestId(friendsTestid.panel, undefined, SLOW);
    return button;
  }

  function annsRow(): HTMLElement {
    const row = screen.getAllByTestId(togetherTestid.friendRow).find((element) => element.getAttribute("data-profile-id") === ANN.profileId);
    return must(row, "Ann's row");
  }

  it("B58 the panel's Chat opens the chat panel with the friend picked, on the screen the player is on", async () => {
    const server = serveFriends();
    await openPanel();
    expect(screen.queryByTestId("chat-input")).toBeNull();
    await screen.findByText(wholeText("Ann#2"), undefined, SLOW);
    fireEvent.click(within(annsRow()).getByTestId(togetherTestid.chatOpen));
    expect(await screen.findByTestId("chat-input", undefined, SLOW)).toBeInTheDocument();
    // The conversation has the panel's place: the panel itself is gone.
    expect(screen.queryByTestId(friendsTestid.panel)).toBeNull();
    await waitFor(() => {
      expect(server.to("GET", `/api/chat/${ANN.profileId}`).length).toBeGreaterThan(0);
    }, SLOW);
  });

  it("B58 closing the conversation, by its close control or by Escape, gives the focus back to the Friends button, which names its panel while open", async () => {
    serveFriends();
    const button = await openPanel();
    const panel = screen.getByTestId(friendsTestid.panel);
    expect(button).toHaveAttribute("aria-controls", panel.id);
    await screen.findByText(wholeText("Ann#2"), undefined, SLOW);
    fireEvent.click(within(annsRow()).getByTestId(togetherTestid.chatOpen));
    expect(await screen.findByTestId("chat-input", undefined, SLOW)).toHaveFocus();
    expect(button).not.toHaveAttribute("aria-controls");

    fireEvent.click(screen.getByTestId("chat-close"));
    expect(screen.queryByTestId("chat-panel")).toBeNull();
    expect(button).toHaveFocus();

    fireEvent.click(button);
    await screen.findByText(wholeText("Ann#2"), undefined, SLOW);
    fireEvent.click(within(annsRow()).getByTestId(togetherTestid.chatOpen));
    fireEvent.keyDown(await screen.findByTestId("chat-input", undefined, SLOW), { key: "Escape" });
    expect(screen.queryByTestId("chat-panel")).toBeNull();
    expect(button).toHaveFocus();
  });

  it("R2127000 Back closes the conversation and gives the focus back to the Friends button", async () => {
    at("/decks");
    window.history.pushState(null, "", "/play");
    serveFriends();
    const button = await openPanel();
    await screen.findByText(wholeText("Ann#2"), undefined, SLOW);
    fireEvent.click(within(annsRow()).getByTestId(togetherTestid.chatOpen));
    expect(await screen.findByTestId("chat-input", undefined, SLOW)).toHaveFocus();

    await pressBack();
    expect(screen.queryByTestId("chat-panel")).toBeNull();
    expect(screen.queryByTestId(friendsTestid.panel)).toBeNull();
    expect(button).toHaveFocus();
    expect(window.location.pathname).toBe("/play");
  });

  it("B58 the panel lists the friends who are on first, the offline ones last", async () => {
    serveFriends();
    await openPanel();
    await screen.findByText(wholeText("Ann#2"), undefined, SLOW);
    const order = screen.getAllByTestId(togetherTestid.friendRow).map((row) => row.getAttribute("data-profile-id"));
    expect(order).toEqual(["f-ann", "f-cy"]);
  });

  it("B58 closed, the Friends button shows no conversation and reads neither a conversation nor the friends", async () => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    const server = serveFriends();
    render(
      <>
        <ChallengeToast />
        <FriendsButton placement="game" />
      </>,
    );
    await settle({ advance: SOCIAL_POLL_MS + 1 });
    expect(server.to("GET", "/api/chat/unread")).toHaveLength(2);
    expect(screen.getByTestId(friendsTestid.button)).toBeInTheDocument();
    expect(screen.queryByTestId("chat-input")).toBeNull();
    expect(server.calls.some((call) => call.method === "GET" && call.path === `/api/chat/${ANN.profileId}`)).toBe(false);
    expect(server.to("GET", "/api/friends")).toHaveLength(0);
  });
});

describe("R2094000 a room link in chat draws Join", () => {
  const origin = window.location.origin;

  async function openWith(body: string): Promise<void> {
    serve([message("msg-1", ANN.profileId, body, 1)]);
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await waitFor(() => {
      expect(shown()).toHaveLength(1);
    }, SLOW);
  }

  it("R2094000 a message with this origin's co-op link shows Join, and Join navigates in-app to that path", async () => {
    window.history.replaceState(null, "", "/together");
    await openWith(`Join my co-op game against the AI here: ${origin}/together?coop=ABC234 (room code ABC234).`);
    const join = await screen.findByTestId("chat-join", undefined, SLOW);
    expect(join).toHaveTextContent("Join");
    fireEvent.click(join);
    expect(window.location.pathname).toBe("/together");
    expect(window.location.search).toBe("?coop=ABC234");
  });

  it("R2094000 a message with this origin's /play?room= link shows Join and navigates to it", async () => {
    window.history.replaceState(null, "", "/together");
    await openWith(`A room for you: ${origin}/play?room=ABC234&mode=bo1`);
    const join = await screen.findByTestId("chat-join", undefined, SLOW);
    fireEvent.click(join);
    expect(window.location.pathname).toBe("/play");
    expect(window.location.search).toBe("?room=ABC234&mode=bo1");
  });

  it("R2094000 a message with another origin's link, a malformed link, or no link shows no Join", async () => {
    for (const body of [
      "join here: https://evil.test/together?coop=ABC234",
      `join here: ${origin}/together?coop=abc123`,
      `join here: ${origin}/play?room=abc123&mode=bo1`,
      "good luck, have fun",
    ]) {
      cleanup();
      window.history.replaceState(null, "", "/together");
      await openWith(body);
      expect(screen.queryByTestId("chat-join")).toBeNull();
    }
  });

  it("R2094000 the message text renders unchanged, with no anchor or other markup made from it", async () => {
    const body = `Join my co-op game against the AI here: ${origin}/together?coop=ABC234 (room code ABC234).`;
    await openWith(body);
    const row = screen.getByTestId("chat-message");
    const text = row.querySelector(".chat-message__body");
    expect(text?.textContent).toBe(body);
    expect(row.querySelector("a")).toBeNull();
    expect(row.innerHTML).not.toContain("<a");
    expect(screen.getByTestId("chat-join")).toBeInTheDocument();
  });
});
