// #1691: the board rail's chat list offers Try again when its read of the friends failed. The server
// is a stubbed `fetch` (`test/stubServer.ts`) and time is vitest's fake clock, as `polling.test.tsx`
// has them.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import type { Friend } from "../net/social.ts";
import { refusal, stubServer, type StubServer } from "../test/stubServer.ts";
import { InMatchChat } from "./InMatchChat.tsx";
import { chatTestid } from "./testids.ts";

const ANN: Friend = {
  profileId: "f-ann",
  username: "Ann#2",
  status: "online",
  matchId: null,
  watchable: false,
  since: "2026-10-01T00:00:00.000Z",
};

/** Whether the friends read is refused: the test turns it off before Try again. */
function serve(refused: { friends: boolean }): StubServer {
  return stubServer((call) => {
    const { method, path } = call;
    if (method === "GET" && path === "/api/friends") {
      return refused.friends
        ? refusal(500, "internal", "Something went wrong.")
        : { status: 200, body: { friends: [ANN], incoming: [], outgoing: [], blocked: [] } };
    }
    if (method === "GET" && path === "/api/chat/unread") return { status: 200, body: { unread: {} } };
    return undefined;
  });
}

/** Let the stubbed round trips and React's updates settle. */
async function settle(): Promise<void> {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(0);
    for (let tick = 0; tick < 50; tick += 1) await Promise.resolve();
  });
}

beforeEach(() => {
  window.localStorage.clear();
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "tok-chat" }));
  vi.useFakeTimers();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.localStorage.clear();
});

describe("#1691 the board rail's chat list", () => {
  it("#1691 the chat list's friends that could not be read offer Try again, which lists them", async () => {
    const refused = { friends: true };
    const server = serve(refused);
    render(<InMatchChat />);
    await settle();
    fireEvent.click(screen.getByTestId(chatTestid.inMatchToggle));
    await settle();

    expect(screen.getByTestId(chatTestid.inMatchList)).toHaveTextContent("Something went wrong.");
    const retry = screen.getByTestId(chatTestid.inMatchRetry);
    expect(retry).toBeEnabled();
    expect(screen.queryAllByTestId(chatTestid.inMatchFriend)).toHaveLength(0);

    refused.friends = false;
    fireEvent.click(retry);
    await settle();
    expect(screen.getAllByTestId(chatTestid.inMatchFriend).map((friend) => friend.getAttribute("data-profile-id"))).toEqual(["f-ann"]);
    expect(screen.queryByTestId(chatTestid.inMatchRetry)).toBeNull();
    expect(server.to("GET", "/api/friends")).toHaveLength(2);
  });
});
