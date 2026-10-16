// #936 S7 B52: `ShareWithFriend`, a finished online match's replay shared with a friend. It starts
// closed and reads no friend until opened, offers one Share per friend, says so (with the way to Play
// together) when there are none, posts a share as `POST /api/replays/:matchId/shares {friendId}` and
// takes it back with `DELETE /api/replays/:matchId/shares/:friendId`, and shows a refusal in the
// server's words with the friend still unshared.
//
// The server is a stubbed `fetch` (`test/stubServer.ts`); the signed-in session is put where
// `net/session.ts` reads it.

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import type { Friend } from "../net/social.ts";
import { must, refusal, stubServer, type Call, type Reply, type StubServer } from "../test/stubServer.ts";
import { ShareWithFriend } from "./ShareReplay.tsx";

/** A stubbed round trip can outrun the 1 s default under load. */
const SLOW = { timeout: 5_000 } as const;

const SINCE = "2026-10-01T00:00:00.000Z";
const ANN: Friend = { profileId: "f-ann", username: "Ann#2", status: "online", matchId: null, watchable: false, since: SINCE };
const BOB: Friend = { profileId: "f-bob", username: "Bob#3", status: "offline", matchId: null, watchable: false, since: SINCE };

const SHARES = "/api/replays/m-done/shares";

function serve(friends: Friend[], special?: (call: Call) => Reply | undefined): StubServer {
  return stubServer((call) => {
    const answer = special?.(call);
    if (answer !== undefined) return answer;
    if (call.method === "GET" && call.path === "/api/friends") {
      return { status: 200, body: { friends, incoming: [], outgoing: [], blocked: [] } };
    }
    if (call.method === "POST" && call.path === SHARES) return { status: 204 };
    if (call.method === "DELETE" && call.path.startsWith(`${SHARES}/`)) return { status: 204 };
    return undefined;
  });
}

function signedIn(): void {
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "tok-share" }));
}

/** Render the control and open it. */
async function opened(): Promise<void> {
  render(<ShareWithFriend matchId="m-done" />);
  fireEvent.click(screen.getByTestId("share-friend-open"));
  await waitFor(() => {
    expect(screen.queryByRole("status")).toBeNull();
  }, SLOW);
}

beforeEach(() => {
  window.localStorage.clear();
  signedIn();
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.localStorage.clear();
});

describe("B52 sharing a replay with a friend", () => {
  it("B52 starts closed and reads no friend until opened", () => {
    const server = serve([ANN, BOB]);
    render(<ShareWithFriend matchId="m-done" />);
    expect(screen.getByTestId("share-friend-open").getAttribute("aria-expanded")).toBe("false");
    expect(screen.queryAllByTestId("share-friend-option")).toHaveLength(0);
    expect(server.to("GET", "/api/friends")).toHaveLength(0);
  });

  it("B52 lists one Share per friend once opened", async () => {
    serve([ANN, BOB]);
    await opened();
    expect(screen.getByTestId("share-friend-open").getAttribute("aria-expanded")).toBe("true");
    const options = screen.getAllByTestId("share-friend-option");
    expect(options.map((option) => option.getAttribute("data-profile-id"))).toEqual(["f-ann", "f-bob"]);
    expect(options.map((option) => option.getAttribute("data-shared"))).toEqual(["false", "false"]);
  });

  it("B52 with no friends, says so and links to Play together", async () => {
    serve([]);
    await opened();
    const none = screen.getByTestId("share-friend-none");
    expect(must(none.querySelector("a"), "the link").getAttribute("href")).toBe("/together");
  });

  it("B52 Share posts the share; the row then offers Stop sharing, which deletes it", async () => {
    const server = serve([ANN, BOB]);
    await opened();
    fireEvent.click(must(screen.getAllByTestId("share-friend-option")[0], "Ann's Share"));
    const unshare = await screen.findByTestId("share-friend-unshare", undefined, SLOW);
    expect(server.to("POST", SHARES)).toHaveLength(1);
    expect(must(server.to("POST", SHARES)[0], "the POST").body).toEqual({ friendId: "f-ann" });
    expect(unshare.getAttribute("data-shared")).toBe("true");
    expect(unshare.getAttribute("data-profile-id")).toBe("f-ann");

    await waitFor(() => {
      expect((unshare as HTMLButtonElement).disabled).toBe(false);
    }, SLOW);
    fireEvent.click(unshare);
    await waitFor(() => {
      expect(screen.queryByTestId("share-friend-unshare")).toBeNull();
    }, SLOW);
    expect(server.to("DELETE", `${SHARES}/f-ann`)).toHaveLength(1);
    expect(screen.getAllByTestId("share-friend-option").map((option) => option.getAttribute("data-profile-id"))).toEqual(["f-ann", "f-bob"]);
  });

  it("B52 a refused share shows the server's words and the friend stays unshared", async () => {
    const words = "Share a finished match you played.";
    serve([ANN], (call) => (call.method === "POST" && call.path === SHARES ? refusal(403, "forbidden", words) : undefined));
    await opened();
    fireEvent.click(must(screen.getAllByTestId("share-friend-option")[0], "Ann's Share"));
    expect(await screen.findByTestId("share-friend-error", undefined, SLOW)).toHaveTextContent(words);
    expect(screen.queryByTestId("share-friend-unshare")).toBeNull();
    expect(must(screen.getAllByTestId("share-friend-option")[0], "Ann's Share").getAttribute("data-shared")).toBe("false");
  });
});
