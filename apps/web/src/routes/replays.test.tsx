// R768000, issue #768: `/replays`, the recent games. It lists the practice games a worker kept and,
// signed in, the account's online games a page at a time; opens one in the player and comes back to
// the list; names a game it cannot show and keeps its Watch button off; and is a route a production
// build keeps, with the GIF code still loaded on demand. R970000: an account that owes the consent
// screen meets it before the online half, which waits on its answer.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { AI_GATE_BUDGET } from "@jackioh/ai";
import { REPLAY_PAGE_STEPS } from "@jackioh/engine/config";
import type { PlayerId } from "@jackioh/shared";
import { TERMS_VERSION } from "@jackioh/server-config";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { consentTestid } from "../auth/testids.ts";
import { ApiRequestError, getMe, getReplaySteps, getReplays, postConsent } from "../net/api.ts";
import type { OnlineReplayListing } from "../net/api.ts";
import { documentTitleFor, screenNameFor } from "../net/head.ts";
import { paths } from "../net/navigate.ts";
import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import type { PracticeCoreEnv } from "../practice/core.ts";
import { createPracticeHost } from "../practice/host.ts";
import type { PracticeHost } from "../practice/host.ts";
import { memorySaveStore } from "../practice/saveStore.ts";
import type { PracticeSaveStore } from "../practice/saveStore.ts";
import { replayTestid } from "../replay/ReplayPlayer.tsx";
import { foldedReplaySteps } from "../test/replay.ts";
import { ReplaysRoute, replaysTestid } from "./replays.tsx";

vi.mock("../net/api.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/api.ts")>();
  return { ...actual, getMe: vi.fn(), getReplays: vi.fn(), getReplaySteps: vi.fn(), postConsent: vi.fn() };
});

const TOKEN = "e2e-token-p1";
const ENV: PracticeCoreEnv = { now: () => 0, dev: true, budget: AI_GATE_BUDGET };
const HUMAN: PlayerId = "p2";
const steps = foldedReplaySteps();

const hosts: PracticeHost[] = [];

/** A host over `saves`, as the route makes its own: in this thread, with the device's kept games. */
function hostOn(saves: PracticeSaveStore): PracticeHost {
  const host = createPracticeHost({ forceInThread: true, env: { ...ENV, saves } });
  hosts.push(host);
  return host;
}

/** A store holding one finished practice game: started and conceded at once. */
async function storeWithGame(): Promise<PracticeSaveStore> {
  const saves = memorySaveStore();
  const host = hostOn(saves);
  await host.request({ type: "start", config: { seed: "replays-list", difficulty: "easy", humanSeat: HUMAN, deck: { kind: "random" } } });
  const done = await host.request({ type: "act", action: { type: "concede" } });
  if (done.type !== "snapshot" || done.snapshot.view.result === null) throw new Error("the conceded game did not end");
  return saves;
}

function renderRoute(saves: PracticeSaveStore): void {
  render(<ReplaysRoute hostFactory={() => hostOn(saves)} />);
}

function listing(over: Partial<OnlineReplayListing> = {}): OnlineReplayListing {
  return {
    matchId: "m-1",
    mode: "bo1",
    endedAt: 0,
    seat: "p1",
    result: "win",
    turns: steps[steps.length - 1]?.turn ?? 0,
    steps: steps.length,
    portraits: { p1: "vanilla", p2: "vanilla" },
    opponent: { profileId: "profile-p2", username: "rival" },
    ...over,
  };
}

function signIn(owed: { termsOwed?: boolean } = {}): void {
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: TOKEN }));
  vi.mocked(getMe).mockResolvedValue({
    profile: { id: "profile-p1", status: "active" },
    needsInviteCode: false,
    emailVerified: true,
    currentMatchId: null,
    email: "player@example.test",
    ...owed,
  } as Awaited<ReturnType<typeof getMe>>);
}

function serveSteps(): void {
  vi.mocked(getReplaySteps).mockImplementation((_token, matchId, from, count) =>
    Promise.resolve({ matchId, from, steps: steps.slice(from, from + Math.min(count, REPLAY_PAGE_STEPS)), total: steps.length }),
  );
}

beforeEach(() => {
  window.localStorage.clear();
  window.history.replaceState(null, "", paths.replays);
});

afterEach(() => {
  cleanup();
  for (const host of hosts.splice(0)) host.dispose();
  vi.clearAllMocks();
  window.localStorage.clear();
});

describe("R768000 /replays", () => {
  it("R768000 lists the device's practice games and opens one; Back returns to the list", async () => {
    renderRoute(await storeWithGame());
    const row = await screen.findByTestId(replaysTestid.practice(1));
    expect(row).toHaveTextContent(/^Loss · Practice · \d+ turns? · /u);
    expect(screen.getByTestId(replaysTestid.signIn)).toHaveTextContent("Sign in to watch your online games again.");
    expect(getReplays).not.toHaveBeenCalled();

    fireEvent.click(within(row).getByRole("button", { name: "Watch" }));
    const player = await screen.findByTestId(replayTestid.player);
    await waitFor(() => expect(player).toHaveAttribute("data-step", "0"));
    expect(screen.queryByTestId(replaysTestid.practice(1))).toBeNull();
    expect(screen.getByTestId("game")).toHaveAttribute("data-replay", "true");

    fireEvent.click(screen.getByTestId(replayTestid.back));
    expect(await screen.findByTestId(replaysTestid.practice(1))).toBeInTheDocument();
    expect(screen.queryByTestId(replayTestid.player)).toBeNull();
  });

  it("R768000 says so when the device keeps no game", async () => {
    renderRoute(memorySaveStore());
    expect(await screen.findByTestId(replaysTestid.noPractice)).toHaveTextContent("Finish a practice game and it is kept here.");
  });

  it("R768000 signed in, lists the account's online games and opens one through getReplaySteps", async () => {
    signIn();
    serveSteps();
    const gone = listing({ matchId: "m-2", opponent: { profileId: "", username: "" }, mode: null, result: "loss" });
    vi.mocked(getReplays).mockResolvedValue({ replays: [listing(), gone], nextOffset: null });
    renderRoute(memorySaveStore());

    const first = await screen.findByTestId(replaysTestid.online("m-1"));
    expect(getReplays).toHaveBeenCalledWith(TOKEN);
    expect(first).toHaveTextContent(/^Win · vs rival · Best of 1 · \d+ turns? · /u);
    expect(screen.getByTestId(replaysTestid.online("m-2"))).toHaveTextContent(/^Loss · vs a deleted account · \d+ turns?/u);
    expect(screen.queryByTestId(replaysTestid.signIn)).toBeNull();
    expect(screen.queryByTestId(replaysTestid.more)).toBeNull();

    fireEvent.click(within(first).getByRole("button", { name: "Watch" }));
    await waitFor(() => expect(screen.getByTestId(replayTestid.player)).toHaveAttribute("data-step", "0"));
    expect(getReplaySteps).toHaveBeenCalledWith(TOKEN, "m-1", 0, REPLAY_PAGE_STEPS);
  });

  it("R768000 shows older online games a page at a time", async () => {
    signIn();
    vi.mocked(getReplays)
      .mockResolvedValueOnce({ replays: [listing()], nextOffset: 1 })
      .mockResolvedValueOnce({ replays: [listing({ matchId: "m-older" })], nextOffset: null });
    renderRoute(memorySaveStore());

    await screen.findByTestId(replaysTestid.online("m-1"));
    fireEvent.click(screen.getByTestId(replaysTestid.more));
    expect(await screen.findByTestId(replaysTestid.online("m-older"))).toBeInTheDocument();
    expect(getReplays).toHaveBeenLastCalledWith(TOKEN, 1);
    expect(screen.getByTestId(replaysTestid.online("m-1"))).toBeInTheDocument();
    expect(screen.queryByTestId(replaysTestid.more)).toBeNull();
  });

  it("R768000 says why a game cannot be shown and keeps its Watch button off", async () => {
    signIn();
    vi.mocked(getReplays).mockResolvedValue({ replays: [listing({ unavailable: "earlier_patch" })], nextOffset: null });
    renderRoute(memorySaveStore());

    const row = await screen.findByTestId(replaysTestid.online("m-1"));
    expect(row).toHaveTextContent("This game was played on an earlier patch.");
    expect(within(row).getByRole("button", { name: "Watch" })).toBeDisabled();
  });

  it("R768000 names a list the server refused", async () => {
    signIn();
    vi.mocked(getReplays).mockRejectedValue(new ApiRequestError(429, { code: "rate_limited", message: "Too many requests" }));
    renderRoute(memorySaveStore());
    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent("Too many requests");
    });
  });
});

describe("R970000 /replays and the consent screen", () => {
  it("R970000 an account that owes the terms sees the practice half, and its online half only once it agrees", async () => {
    signIn({ termsOwed: true });
    vi.mocked(getReplays).mockResolvedValue({ replays: [listing()], nextOffset: null });
    vi.mocked(postConsent).mockImplementation(() => {
      signIn();
      return Promise.resolve({ ageCheckOwed: false, termsOwed: false });
    });
    renderRoute(await storeWithGame());

    // The practice half is open; the online half, its "Shared with me" and its shares wait.
    expect(await screen.findByTestId(replaysTestid.practice(1))).toBeInTheDocument();
    const owed = await screen.findByTestId(replaysTestid.consent);
    expect(owed).toHaveTextContent("Agree to the Terms and the Privacy Policy to see your online games.");
    expect(getReplays).not.toHaveBeenCalled();
    expect(screen.queryByTestId(replaysTestid.sharedWithMe)).toBeNull();
    expect(screen.queryByTestId(replaysTestid.online("m-1"))).toBeNull();

    fireEvent.click(within(owed).getByRole("button", { name: "Continue" }));
    expect(await screen.findByTestId(consentTestid.changed)).toBeInTheDocument();
    // Unticked, nothing is sent.
    fireEvent.click(screen.getByTestId(consentTestid.submit));
    expect(postConsent).not.toHaveBeenCalled();
    fireEvent.click(screen.getByTestId(consentTestid.agree));
    fireEvent.click(screen.getByTestId(consentTestid.submit));

    expect(await screen.findByTestId(replaysTestid.online("m-1"))).toBeInTheDocument();
    expect(postConsent).toHaveBeenCalledWith(TOKEN, { termsVersion: TERMS_VERSION });
    expect(getReplays).toHaveBeenCalledWith(TOKEN);
    expect(screen.getByTestId(replaysTestid.sharedWithMe)).toBeInTheDocument();
    expect(screen.queryByTestId(replaysTestid.consent)).toBeNull();
    expect(screen.getByTestId(replaysTestid.practice(1))).toBeInTheDocument();
  });

  it("R970000 an account that owes nothing is not asked", async () => {
    signIn({ termsOwed: false });
    vi.mocked(getReplays).mockResolvedValue({ replays: [listing()], nextOffset: null });
    renderRoute(memorySaveStore());
    expect(await screen.findByTestId(replaysTestid.online("m-1"))).toBeInTheDocument();
    expect(screen.queryByTestId(replaysTestid.consent)).toBeNull();
  });
});

describe("R768000 a route a production build keeps", () => {
  const SRC = join(dirname(fileURLToPath(import.meta.url)), "..");

  it("R768000 is named in every build and lazily loaded by main.tsx, and the GIF code stays on demand", () => {
    vi.stubEnv("MODE", "production");
    expect(screenNameFor(paths.replays)).toBe("Replays");
    expect(documentTitleFor(paths.replays)).toMatch(/^Replays · /u);
    vi.unstubAllEnvs();

    const main = readFileSync(join(SRC, "main.tsx"), "utf8");
    expect(main).toContain('const ReplaysRoute = lazy(() => import("./routes/replays.tsx"));');
    expect(main).not.toContain("DevReplaysRoute");
    // The player imports the export's module only for its type; the code itself is an `import()`.
    const share = readFileSync(join(SRC, "replay/ShareGif.tsx"), "utf8");
    expect(share).toMatch(/import type \* as Gif from "\.\/gif\/export\.ts";/u);
    expect(share).toContain('await import("./gif/export.ts")');
  });
});
