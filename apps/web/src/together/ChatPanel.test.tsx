// #936 S8 B58: `ChatPanel`, the conversation with one friend. It shows the page the server answers
// (newest last), polls every CHAT_POLL_MS while it is open and never once it is closed, sends what is
// typed as `POST /api/chat/:friendId {body}`, and shows a refusal (B53: empty, too long, not friends,
// rate-limited) in the server's words without adding a message. `InMatchChat` is the board rail's
// toggle listing online friends, each opening this panel.

import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CHAT_POLL_MS } from "@jackioh/server-config";

import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import type { ChatMessage, Friend, PlayerRef } from "../net/social.ts";
import { must, refusal, stubServer, type Call, type Reply, type StubServer } from "../test/stubServer.ts";
import { ChatPanel } from "./ChatPanel.tsx";
import { InMatchChat } from "./InMatchChat.tsx";

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
    if (call.method === "GET" && call.path === `/api/chat/${ANN.profileId}`) return { status: 200, body: { messages } };
    if (call.method === "POST" && call.path === `/api/chat/${ANN.profileId}`) {
      next += 1;
      const text = (call.body as { body?: unknown } | undefined)?.body;
      const sent = message(`msg-${String(next)}`, ME.profileId, typeof text === "string" ? text.trim() : "", 30);
      messages.push(sent);
      return { status: 201, body: { message: sent } };
    }
    if (call.method === "GET" && call.path === "/api/chat/unread") return { status: 200, body: { unread: {} } };
    return undefined;
  });
}

function signedIn(): void {
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "tok-chat" }));
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

/** Advance fake time by `ms`, then let the stubbed round trips and React's updates settle. */
async function settle(ms = 0): Promise<void> {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms);
    for (let tick = 0; tick < 50; tick += 1) await Promise.resolve();
  });
}

/** After the first render: the account (if the panel reads one), the first page, its render. */
async function opened(): Promise<void> {
  for (let round = 0; round < 3; round += 1) await settle();
}

beforeEach(() => {
  window.localStorage.clear();
  signedIn();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.localStorage.clear();
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
    const server = serve([]);
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await waitFor(() => {
      expect(server.to("GET", `/api/chat/${ANN.profileId}`).length).toBeGreaterThan(0);
    }, SLOW);
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });
    expect(shown()).toHaveLength(0);
    expect(screen.getByTestId("chat-input")).toBeInTheDocument();
  });

  it("B58 a conversation the server refuses to read (no longer friends) shows no message", async () => {
    const server = serve(conversation(), (call) =>
      call.method === "GET" && call.path === `/api/chat/${ANN.profileId}` ? refusal(404, "not_found", "You can only chat with friends.") : undefined,
    );
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await waitFor(() => {
      expect(server.to("GET", `/api/chat/${ANN.profileId}`).length).toBeGreaterThan(0);
    }, SLOW);
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });
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
    const server = serve(conversation(), (call) => (call.method === "POST" ? refusal(422, "too_long", words) : undefined));
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await waitFor(() => {
      expect(shown()).toHaveLength(2);
    }, SLOW);
    type("x".repeat(501));
    send();
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });
    if (server.to("POST", `/api/chat/${ANN.profileId}`).length > 0) {
      await waitFor(() => {
        expect(document.body).toHaveTextContent(words);
      }, SLOW);
    }
    expect(shown()).toHaveLength(2);
  });

  it("B58 a blank message adds no message, whether the panel holds it back or the server refuses it", async () => {
    const words = "Type a message first.";
    serve(conversation(), (call) => (call.method === "POST" ? refusal(422, "empty", words) : undefined));
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await waitFor(() => {
      expect(shown()).toHaveLength(2);
    }, SLOW);
    type("    ");
    send();
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });
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

    await settle(CHAT_POLL_MS);
    const second = reads();
    expect(second).toBeGreaterThan(first);
    await settle(CHAT_POLL_MS);
    expect(reads()).toBeGreaterThan(second);
  });

  it("B58 does not read again before CHAT_POLL_MS has passed", async () => {
    vi.useFakeTimers();
    const server = serve(conversation());
    render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await opened();
    const first = server.to("GET", `/api/chat/${ANN.profileId}`).length;
    await settle(Math.floor(CHAT_POLL_MS / 2));
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
    await settle(CHAT_POLL_MS);
    expect(shown()).toHaveLength(3);
    expect(must(shown()[2], "the new message")).toContain("rematch?");
  });

  it("B58 a closed panel stops polling", async () => {
    vi.useFakeTimers();
    const server = serve(conversation());
    const { unmount } = render(<ChatPanel friend={ANN} onClose={vi.fn()} />);
    await opened();
    await settle(CHAT_POLL_MS);
    unmount();
    const before = server.to("GET", `/api/chat/${ANN.profileId}`).length;
    await settle(CHAT_POLL_MS * 3);
    expect(server.to("GET", `/api/chat/${ANN.profileId}`)).toHaveLength(before);
  });
});

describe("B58 the in-match chat toggle", () => {
  const ONLINE: Friend = { ...ANN, status: "online", matchId: null, watchable: false, since: SINCE };
  const OFFLINE: Friend = { profileId: "f-cy", username: "Cy#4", status: "offline", matchId: null, watchable: false, since: SINCE };

  function serveFriends(): StubServer {
    return serve(conversation(), (call) =>
      call.method === "GET" && call.path === "/api/friends"
        ? { status: 200, body: { friends: [ONLINE, OFFLINE], incoming: [], outgoing: [], blocked: [] } }
        : undefined,
    );
  }

  it("B58 the toggle lists online friends and opens the chat panel with the one picked", async () => {
    const server = serveFriends();
    render(<InMatchChat />);
    expect(screen.queryByTestId("chat-input")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: /chat/i }));
    const ann = await screen.findByText(/Ann#2/, undefined, SLOW);
    fireEvent.click(ann.closest("button") ?? ann);
    expect(await screen.findByTestId("chat-input", undefined, SLOW)).toBeInTheDocument();
    await waitFor(() => {
      expect(server.to("GET", `/api/chat/${ANN.profileId}`).length).toBeGreaterThan(0);
    }, SLOW);
  });

  it("B58 the toggle does not list offline friends", async () => {
    serveFriends();
    render(<InMatchChat />);
    fireEvent.click(screen.getByRole("button", { name: /chat/i }));
    await screen.findByText(/Ann#2/, undefined, SLOW);
    expect(screen.queryByText(/Cy#4/)).toBeNull();
  });

  it("B58 closed, the toggle shows no conversation and reads none", async () => {
    const server = serveFriends();
    render(<InMatchChat />);
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });
    expect(screen.queryByTestId("chat-input")).toBeNull();
    expect(server.calls.some((call) => call.method === "GET" && call.path === `/api/chat/${ANN.profileId}`)).toBe(false);
  });
});
