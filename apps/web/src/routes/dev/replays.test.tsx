// `/dev/replays` (#511): the replay player's dev list. It lists the practice games a worker kept and,
// signed in, the account's online replays; opens one in the player; and exists only outside a
// production build, which these tests hold three ways: the tab title, the imports of `src/replay/`,
// and the one lazy import in `main.tsx` behind `DEV_ONLY`.

import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { AI_GATE_BUDGET } from "@jackioh/ai";
import { REPLAY_PAGE_STEPS } from "@jackioh/engine/config";
import type { PlayerId } from "@jackioh/shared";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import ts from "typescript";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ApiRequestError, getMe, getReplaySteps, getReplays } from "../../net/api.ts";
import type { OnlineReplayListing } from "../../net/api.ts";
import { screenNameFor } from "../../net/head.ts";
import { paths } from "../../net/navigate.ts";
import { E2E_SESSION_STORAGE_KEY } from "../../net/session.ts";
import type { PracticeCoreEnv } from "../../practice/core.ts";
import { createPracticeHost } from "../../practice/host.ts";
import type { PracticeHost } from "../../practice/host.ts";
import { memorySaveStore } from "../../practice/saveStore.ts";
import type { PracticeSaveStore } from "../../practice/saveStore.ts";
import { replayTestid } from "../../replay/ReplayPlayer.tsx";
import { foldedReplaySteps } from "../../test/replay.ts";
import { navTestid } from "../nav.tsx";
import { DevReplaysRoute, devReplaysTestid } from "./replays.tsx";

vi.mock("../../net/api.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../net/api.ts")>();
  return { ...actual, getMe: vi.fn(), getReplays: vi.fn(), getReplaySteps: vi.fn() };
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
  await host.request({ type: "start", config: { seed: "dev-replays-list", difficulty: "easy", humanSeat: HUMAN, deck: { kind: "random" } } });
  const done = await host.request({ type: "act", action: { type: "concede" } });
  if (done.type !== "snapshot" || done.snapshot.view.result === null) throw new Error("the conceded game did not end");
  return saves;
}

function renderRoute(saves: PracticeSaveStore): void {
  render(<DevReplaysRoute hostFactory={() => hostOn(saves)} />);
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

function signIn(): void {
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: TOKEN }));
  vi.mocked(getMe).mockResolvedValue({
    profile: { id: "profile-p1", status: "active" },
    needsInviteCode: false,
    emailVerified: true,
    currentMatchId: null,
    email: "player@example.test",
  } as Awaited<ReturnType<typeof getMe>>);
}

beforeEach(() => {
  window.localStorage.clear();
  window.history.replaceState(null, "", paths.devReplays);
});

afterEach(() => {
  cleanup();
  for (const host of hosts.splice(0)) host.dispose();
  vi.clearAllMocks();
  vi.unstubAllEnvs();
  window.localStorage.clear();
});

describe("/dev/replays", () => {
  it("lists the device's practice games and opens one; Back returns to the list", async () => {
    renderRoute(await storeWithGame());
    const row = await screen.findByTestId(devReplaysTestid.practice(1));
    expect(row).toHaveTextContent(/^Game 1 · loss · \d+ turns · \d+ steps/u);
    expect(screen.getByText("Sign in to list your online replays.")).toBeInTheDocument();
    expect(getReplays).not.toHaveBeenCalled();

    fireEvent.click(within(row).getByRole("button", { name: "Open" }));
    const player = await screen.findByTestId(replayTestid.player);
    expect(player).toHaveAttribute("data-step", "0");
    expect(screen.queryByTestId(devReplaysTestid.practice(1))).toBeNull();
    expect(screen.getByTestId("game")).toHaveAttribute("data-replay", "true");

    fireEvent.click(screen.getByTestId(replayTestid.back));
    expect(await screen.findByTestId(devReplaysTestid.practice(1))).toBeInTheDocument();
    expect(screen.queryByTestId(replayTestid.player)).toBeNull();
    expect(screen.getByTestId(navTestid.back)).toBeInTheDocument();
  });

  it("says so when the device keeps no game", async () => {
    renderRoute(memorySaveStore());
    expect(await screen.findByText("No finished practice game is kept on this device.")).toBeInTheDocument();
  });

  it("signed in, lists the account's online replays and opens one through getReplaySteps", async () => {
    signIn();
    const gone = listing({ matchId: "m-2", opponent: { profileId: "", username: "" }, mode: null, result: "loss" });
    vi.mocked(getReplays).mockResolvedValue({ replays: [listing(), gone], nextOffset: null });
    vi.mocked(getReplaySteps).mockImplementation((_token, matchId, from, count) =>
      Promise.resolve({
        matchId,
        from,
        steps: steps.slice(from, from + Math.min(count, REPLAY_PAGE_STEPS)),
        total: steps.length,
      }),
    );
    renderRoute(memorySaveStore());

    const first = await screen.findByTestId(devReplaysTestid.online("m-1"));
    expect(getReplays).toHaveBeenCalledWith(TOKEN);
    expect(first).toHaveTextContent(/^vs rival · bo1 · win · \d+ turns/u);
    expect(screen.getByTestId(devReplaysTestid.online("m-2"))).toHaveTextContent(/^vs a deleted account · unknown mode · loss/u);
    expect(screen.queryByText("Sign in to list your online replays.")).toBeNull();

    fireEvent.click(within(first).getByRole("button", { name: "Open" }));
    expect(await screen.findByTestId(replayTestid.player)).toHaveAttribute("data-step", "0");
    expect(getReplaySteps).toHaveBeenCalledWith(TOKEN, "m-1", 0, REPLAY_PAGE_STEPS);
  });

  it("says why a game cannot be shown, in the list and again once opened", async () => {
    signIn();
    vi.mocked(getReplays).mockResolvedValue({ replays: [listing({ unavailable: "earlier_patch" })], nextOffset: null });
    vi.mocked(getReplaySteps).mockRejectedValue(
      new ApiRequestError(422, { code: "replay_unavailable", message: "this replay cannot be shown: earlier_patch", details: { reason: "earlier_patch" } }),
    );
    renderRoute(memorySaveStore());

    const row = await screen.findByTestId(devReplaysTestid.online("m-1"));
    expect(row).toHaveTextContent("This game was played on an earlier patch.");
    fireEvent.click(within(row).getByRole("button", { name: "Open" }));
    expect(await screen.findByTestId(replayTestid.failure)).toHaveTextContent("This game was played on an earlier patch.");
    fireEvent.click(screen.getByTestId(replayTestid.back));
    expect(await screen.findByTestId(devReplaysTestid.online("m-1"))).toBeInTheDocument();
  });

  it("names a list the server refused", async () => {
    signIn();
    vi.mocked(getReplays).mockRejectedValue(new ApiRequestError(429, { code: "rate_limited", message: "Too many requests" }));
    renderRoute(memorySaveStore());
    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent("Too many requests");
    });
  });
});

describe("a production build", () => {
  it("a production build names no replay screen", () => {
    vi.stubEnv("MODE", "production");
    expect(import.meta.env.MODE, "vi.stubEnv must reach import.meta.env, or this proves nothing").toBe("production");
    expect(screenNameFor(paths.devReplays)).toBeNull();
    vi.unstubAllEnvs();
    expect(screenNameFor(paths.devReplays)).toBe("Replays");
  });

  const SRC = join(dirname(fileURLToPath(import.meta.url)), "../..");

  /** Every non-test source under src/, as `wording.test.ts` walks them. */
  function sources(dir: string): string[] {
    return readdirSync(dir).flatMap((name) => {
      const path = join(dir, name);
      if (statSync(path).isDirectory()) return name === "test" ? [] : sources(path);
      return /\.(ts|tsx)$/u.test(name) && !/\.test\.tsx?$/u.test(name) ? [path] : [];
    });
  }

  /** The module names a file imports or re-exports, statically or with `import()`. */
  function specifiersOf(path: string): string[] {
    const text = readFileSync(path, "utf8");
    const file = ts.createSourceFile(path, text, ts.ScriptTarget.Latest, true, path.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS);
    const found: string[] = [];
    const visit = (node: ts.Node): void => {
      if ((ts.isImportDeclaration(node) || ts.isExportDeclaration(node)) && node.moduleSpecifier !== undefined && ts.isStringLiteral(node.moduleSpecifier)) {
        found.push(node.moduleSpecifier.text);
      }
      if (ts.isCallExpression(node) && node.expression.kind === ts.SyntaxKind.ImportKeyword) {
        const [argument] = node.arguments;
        if (argument !== undefined && ts.isStringLiteralLike(argument)) found.push(argument.text);
      }
      ts.forEachChild(node, visit);
    };
    visit(file);
    return found;
  }

  it("only the player and the dev routes import src/replay/, and main.tsx loads the dev list behind DEV_ONLY", () => {
    const files = sources(SRC);
    expect(files.length).toBeGreaterThan(100);
    const importers = files
      .map((path) => relative(SRC, path))
      .filter((name) => !name.startsWith("replay/") && !name.startsWith("routes/dev/"))
      .filter((name) => specifiersOf(join(SRC, name)).some((specifier) => specifier.includes("/replay/")));
    expect(importers, "nothing a production build reaches imports the replay player").toEqual([]);
    // The dev list is the player's one way in, and it is imported only by main.tsx's lazy line.
    expect(files.some((path) => specifiersOf(path).some((specifier) => specifier.includes("/replay/")))).toBe(true);
    const listImporters = files
      .map((path) => relative(SRC, path))
      .filter((name) => specifiersOf(join(SRC, name)).some((specifier) => /\/dev\/replays(\.tsx)?$/u.test(specifier)));
    expect(listImporters).toEqual(["main.tsx"]);

    const main = readFileSync(join(SRC, "main.tsx"), "utf8");
    expect(main).toContain('const DevReplaysRoute = DEV_ONLY ? lazy(() => import("./routes/dev/replays.tsx")) : null;');
    expect(main).toMatch(/path === paths\.devReplays\) \{\s+if \(DevReplaysRoute === null\) return <NotFound path=\{path\} \/>;/u);
  });
});
