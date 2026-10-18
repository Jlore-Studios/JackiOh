// R768000, issue #768: `/replays`, the recent games. It lists the practice games a worker kept and,
// signed in, the account's online games a page at a time; opens one in the player and comes back to
// the list; names a game it cannot show and keeps its Watch button off; and is a route a production
// build keeps, with the GIF code still loaded on demand. R970000: an account that owes the consent
// screen meets it before the online half, which waits on its answer. R1141000: a row that can be
// watched downloads as one JSON file of its steps' views and its own facts, no account named, and
// that file opens again in the player, in the page, or is refused where it stands (#1160).

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { AI_GATE_BUDGET } from "@jackioh/ai";
import { CATALOG_VERSION } from "@jackioh/cards";
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
import { REPLAY_JSON_MAX_BYTES, REPLAY_JSON_MAX_STEPS } from "../replay/config.ts";
import { REPLAY_JSON_FORMAT, REPLAY_JSON_VERSION, practiceGame, replayJsonFile } from "../replay/download.ts";
import type { ReplayJson } from "../replay/download.ts";
import { fakeReplaySource, foldedReplaySteps } from "../test/replay.ts";
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

describe("R1141000 Download JSON", () => {
  /** The files handed to the browser's downloads, by the name each was saved under. */
  let saved: File[];

  beforeEach(() => {
    saved = [];
    Object.defineProperty(URL, "createObjectURL", {
      configurable: true,
      value: vi.fn((file: File) => {
        saved.push(file);
        return "blob:jackioh-replay";
      }),
    });
    Object.defineProperty(URL, "revokeObjectURL", { configurable: true, value: vi.fn() });
    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => undefined);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  async function savedJson(): Promise<{ file: File; text: string; json: ReplayJson }> {
    await waitFor(() => expect(saved).toHaveLength(1));
    const file = saved[0];
    if (file === undefined) throw new Error("nothing was saved");
    const text = await file.text();
    return { file, text, json: JSON.parse(text) as ReplayJson };
  }

  it("R1141000 a practice row saves every step of its game, in order, and nothing of its setup", async () => {
    renderRoute(await storeWithGame());
    const row = await screen.findByTestId(replaysTestid.practice(1));
    // Its button sits just left of Watch.
    expect(within(row).getByRole("button", { name: "Watch" }).previousElementSibling).toBe(
      screen.getByTestId(replaysTestid.download("practice-1")),
    );
    fireEvent.click(screen.getByTestId(replaysTestid.download("practice-1")));

    const { file, text, json } = await savedJson();
    expect(file.name).toBe("jackioh-replay-practice-1.json");
    expect(file.type).toBe("application/json");
    expect(Object.keys(json).sort()).toEqual(["catalogVersion", "format", "game", "savedAt", "steps", "version"]);
    expect(json).toMatchObject({ format: REPLAY_JSON_FORMAT, version: REPLAY_JSON_VERSION, catalogVersion: CATALOG_VERSION });
    expect(json.game).toMatchObject({ kind: "practice", game: 1, result: "loss", steps: json.steps.length });
    expect(Object.keys(json.game).sort()).toEqual(["endedAt", "game", "kind", "portraits", "result", "steps", "turns"]);
    expect(json.steps.map((step) => step.step)).toEqual(json.steps.map((_, index) => index));
    expect(json.steps.at(-1)?.view.result).not.toBeNull();
    // The game's seed and setup stay in the worker (CLAUDE.md rule 7).
    expect(text).not.toContain("replays-list");
    expect(text).not.toMatch(/"(seed|log|decks|handicaps|state)":/u);
    expect(text).not.toMatch(/"(profileId|accountId)":/u);
    await waitFor(() => expect(screen.getByTestId(replaysTestid.download("practice-1"))).toHaveTextContent("Download JSON"));
  });

  it("R1141000 an online row saves the server's steps and the match id, never the opponent's account", async () => {
    signIn();
    serveSteps();
    vi.mocked(getReplays).mockResolvedValue({ replays: [listing()], nextOffset: null });
    renderRoute(memorySaveStore());
    const row = await screen.findByTestId(replaysTestid.online("m-1"));
    expect(within(row).getByRole("button", { name: "Watch" }).previousElementSibling).toBe(
      screen.getByTestId(replaysTestid.download("online-m-1")),
    );
    fireEvent.click(screen.getByTestId(replaysTestid.download("online-m-1")));

    const { file, text, json } = await savedJson();
    expect(file.name).toBe("jackioh-replay-m-1.json");
    expect(json.steps).toEqual(steps);
    expect(json.game).toEqual({
      kind: "online",
      matchId: "m-1",
      mode: "bo1",
      seat: "p1",
      endedAt: 0,
      result: "win",
      turns: listing().turns,
      steps: steps.length,
      portraits: { p1: "vanilla", p2: "vanilla" },
    });
    expect(text).not.toContain("rival");
    expect(text).not.toContain("profile-p2");
    expect(text).not.toMatch(/"(seed|log|decks|handicaps|state)":/u);
    expect(text).not.toMatch(/"(profileId|accountId)":/u);
    expect(getReplaySteps).toHaveBeenCalledWith(TOKEN, "m-1", 0, REPLAY_PAGE_STEPS);
    expect(getReplaySteps).toHaveBeenCalledWith(TOKEN, "m-1", REPLAY_PAGE_STEPS, REPLAY_PAGE_STEPS);
  });

  it("R1141000 a row this build cannot show offers no download", async () => {
    signIn();
    vi.mocked(getReplays).mockResolvedValue({ replays: [listing({ unavailable: "rules_changed" })], nextOffset: null });
    renderRoute(memorySaveStore());
    await screen.findByTestId(replaysTestid.online("m-1"));
    expect(screen.queryByTestId(replaysTestid.download("online-m-1"))).toBeNull();
  });

  it("R1141000 a page that cannot be had saves nothing and says why on the row", async () => {
    signIn();
    vi.mocked(getReplaySteps).mockRejectedValueOnce(new ApiRequestError(410, { code: "gone", message: "purged" }));
    vi.mocked(getReplays).mockResolvedValue({ replays: [listing()], nextOffset: null });
    renderRoute(memorySaveStore());
    const row = await screen.findByTestId(replaysTestid.online("m-1"));
    fireEvent.click(screen.getByTestId(replaysTestid.download("online-m-1")));

    expect(await within(row).findByTestId(replaysTestid.downloadFailed("online-m-1"))).toHaveTextContent("This replay has expired.");
    expect(saved).toEqual([]);
    expect(screen.getByTestId(replaysTestid.download("online-m-1"))).toBeEnabled();
  });
});

describe("R1141000 Open replay JSON", () => {
  /** The file a download of a practice game makes: `replayJsonFile` over the folded steps. */
  function downloaded(): Promise<File> {
    const game = practiceGame({
      game: 1,
      endedAt: 0,
      result: "loss",
      turns: steps[steps.length - 1]?.turn ?? 0,
      steps: steps.length,
      portraits: { p1: "vanilla", p2: "vanilla" },
    });
    return replayJsonFile(game, fakeReplaySource(steps), 0);
  }

  /** The route, unsigned, over a device that keeps no game, once its card data is read and the input is ready. */
  async function openInput(): Promise<HTMLInputElement> {
    renderRoute(memorySaveStore());
    await screen.findByTestId(replaysTestid.noPractice);
    const input = screen.getByTestId<HTMLInputElement>(replaysTestid.openFile);
    await waitFor(() => expect(input).toBeEnabled());
    return input;
  }

  function choose(input: HTMLInputElement, file: File): void {
    fireEvent.change(input, { target: { files: [file] } });
  }

  it("R1141000 a file saved by replayJsonFile opens with its steps in order and turns to move by", async () => {
    const input = await openInput();
    expect(screen.getByLabelText("Open replay JSON")).toBe(input);
    choose(input, await downloaded());

    const player = await screen.findByTestId(replayTestid.player);
    await waitFor(() => expect(player).toHaveAttribute("data-step", "0"));
    expect(screen.queryByTestId(replaysTestid.root)).toBeNull();
    // No match id, so no share with a friend.
    expect(screen.queryByText("Share with a friend")).toBeNull();

    fireEvent.click(screen.getByTestId(replayTestid.forwardStep));
    await waitFor(() => expect(player).toHaveAttribute("data-step", "1"));
    const secondTurn = steps.findIndex((step) => step.turn !== steps[0]?.turn);
    expect(secondTurn).toBeGreaterThan(1);
    fireEvent.click(screen.getByTestId(replayTestid.forwardTurn));
    await waitFor(() => expect(player).toHaveAttribute("data-step", String(secondTurn)));
    fireEvent.click(screen.getByTestId(replayTestid.backTurn));
    await waitFor(() => expect(player).toHaveAttribute("data-step", "0"));
    fireEvent.click(screen.getByTestId(replayTestid.end));
    await waitFor(() => expect(player).toHaveAttribute("data-step", String(steps.length - 1)));

    fireEvent.click(screen.getByTestId(replayTestid.back));
    expect(await screen.findByTestId(replaysTestid.root)).toBeInTheDocument();
    expect(screen.queryByTestId(replayTestid.player)).toBeNull();
    expect(screen.queryByTestId(replaysTestid.openFailed)).toBeNull();
    // The input was emptied, so the same file can be chosen again.
    expect(screen.getByTestId<HTMLInputElement>(replaysTestid.openFile).value).toBe("");
  });

  it("R1141000 invalid, stale and over-limit files refuse before mounting", async () => {
    const input = await openInput();
    const file = (over: Record<string, unknown>): File =>
      new File(
        [JSON.stringify({ format: REPLAY_JSON_FORMAT, version: REPLAY_JSON_VERSION, catalogVersion: CATALOG_VERSION, savedAt: 0, game: {}, steps, ...over })],
        "replay.json",
      );
    const gap = [steps[0], steps[2]];
    const refused: [File, string][] = [
      [new File(["{ not json"], "replay.json"), "This file is not a JackiOh replay file."],
      [file({ format: "jackioh-deck" }), "This replay file uses an unsupported format."],
      [file({ version: 2 }), "This replay file uses an unsupported format."],
      [file({ catalogVersion: "v0.0.0" }), "This replay file is from another patch and cannot be opened here."],
      [file({ steps: gap }), "This file is not a JackiOh replay file."],
      [file({ steps: Array.from({ length: REPLAY_JSON_MAX_STEPS + 1 }, (_, step) => ({ step, turn: 0, view: {} })) }), "This replay file is too big to open."],
      [new File(["x".repeat(REPLAY_JSON_MAX_BYTES + 1)], "replay.json"), "This replay file is too big to open."],
    ];
    for (const [bad, sentence] of refused) {
      choose(input, bad);
      await waitFor(() => expect(screen.getByTestId(replaysTestid.openFailed)).toHaveTextContent(sentence));
      await waitFor(() => expect(input).toBeEnabled());
      expect(screen.queryByTestId(replayTestid.player)).toBeNull();
      expect(screen.getByTestId(replaysTestid.root)).toBeInTheDocument();
    }
    // A good file after the refusals opens, and the sentence is gone from the list it returns to.
    choose(input, await downloaded());
    expect(await screen.findByTestId(replayTestid.player)).toBeInTheDocument();
    fireEvent.click(screen.getByTestId(replayTestid.back));
    await screen.findByTestId(replaysTestid.root);
    expect(screen.queryByTestId(replaysTestid.openFailed)).toBeNull();
  });

  it("R1141000 opening makes no API request and keeps nothing of the file in storage", async () => {
    const input = await openInput();
    const fetching = vi.spyOn(globalThis, "fetch");
    const writing = vi.spyOn(Storage.prototype, "setItem");
    choose(input, await downloaded());
    const player = await screen.findByTestId(replayTestid.player);
    await waitFor(() => expect(player).toHaveAttribute("data-step", "0"));
    fireEvent.click(screen.getByTestId(replayTestid.forwardStep));
    await waitFor(() => expect(player).toHaveAttribute("data-step", "1"));

    expect(getReplays).not.toHaveBeenCalled();
    expect(getReplaySteps).not.toHaveBeenCalled();
    // The board's music is fetched as on any board; nothing is sent to the API.
    expect(fetching.mock.calls.map(([url]) => String(url)).filter((url) => url.includes("/api/"))).toEqual([]);
    // The board saves its own music rotation as on any board; nothing of the file or its steps is kept.
    const written = writing.mock.calls.map(([key, value]) => `${key}=${value}`);
    expect(written.filter((entry) => /jackioh-replay|replay\.json|catalogVersion|"step"/u.test(entry))).toEqual([]);
    vi.restoreAllMocks();
  });
});
