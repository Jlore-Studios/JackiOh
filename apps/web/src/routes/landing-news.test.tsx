// R2875002: the landing's news bookmark (SPEC §14.13, issue #2875): beside the quests' bookmark, its red
// count of the entries the server says are unclaimed, a claim's answer moving it, and who gets none of
// it; R2875001 no live entry, no bookmark; R2875003 each reward in its words; R3074003 a dev's post,
// which pays no reward.

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { GLOBAL_PACK_NAME } from "@jackioh/server-config";

import { landingTestid } from "../auth/testids.ts";
import { announceWallet } from "../economy/wallet.ts";
import { resetLocaleForTests, setLocale } from "../i18n/i18n.ts";
import type { NewsEntry } from "../net/api.ts";
import { readSheet, setReducedMotion } from "../test/setup.ts";
import { at, meBody, must, refusal, signIn, stubServer, type Reply, type StubServer } from "../test/stubServer.ts";
import LandingRoute, { QUESTS_UNROLL_MS } from "./landing.tsx";
import { newsTestid, rewardWords } from "./NewsPanel.tsx";

// The wallet's announcement is the real one, watched: `announceWallet` is what the news claims call.
vi.mock("../economy/wallet.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../economy/wallet.ts")>();
  return { ...actual, announceWallet: vi.fn(actual.announceWallet) };
});

const NOW = 1_800_000_000_000;

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;

function entry(over: Partial<NewsEntry> = {}): NewsEntry {
  return {
    id: "patch-0-6-0",
    kind: "patch",
    version: "0.6.0",
    title: "Patch 0.6.0",
    summary: "The AI orders its own attacks by value.",
    postedAt: NOW - 1,
    reward: { kind: "jlore", amount: 100 },
    claimed: false,
    ...over,
  };
}

const ALERT = entry({
  id: "alert-1",
  kind: "alert",
  version: null,
  title: "Servers back",
  summary: "Sorry for the wait.",
  reward: { kind: "cardToken", amount: 1 },
});

/** The server's answer to a claim of `id`: the entries, that one now claimed. */
function claimed(news: readonly NewsEntry[], id: string): Reply {
  return { status: 200, body: { news: news.map((one) => (one.id === id ? { ...one, claimed: true } : one)) } };
}

/**
 * `/api/auth/me` answers `status`, `/api/quests` one quest, `GET /api/news` `news`, and a claim
 * `claim(id)` (by default that entry claimed).
 */
function serve(
  news: NewsEntry[],
  { status = "active", claim = (id) => claimed(news, id) }: { status?: "active" | "pending"; claim?: (id: string) => Reply } = {},
): StubServer {
  return stubServer((call) => {
    if (call.path === "/api/auth/me") {
      return { status: 200, body: meBody({ profile: { id: "profile-1", status } }) };
    }
    if (call.path === "/api/quests") {
      const quest = { questId: "win-3", title: "Win 3 games", progress: 1, target: 3, jlore: 80, xp: 4000, arrivedAt: NOW - 1 };
      return { status: 200, body: { quests: [quest], nextDueAt: null } };
    }
    if (call.method === "GET" && call.path === "/api/news") return { status: 200, body: { news } };
    const claimOf = /^\/api\/news\/([^/]+)\/claim$/u.exec(call.path);
    if (call.method === "POST" && claimOf !== null) return claim(decodeURIComponent(must(claimOf[1], "the claimed id")));
    return undefined;
  });
}

function landing(): HTMLElement {
  return screen.getByTestId(landingTestid.root);
}

function newsReads(server: StubServer): number {
  return server.to("GET", "/api/news").length;
}

/** A rule's `property` in px. */
function px(rule: string, property: string): number {
  return Number(must(new RegExp(`(?:^|[\\s;])${property}: (-?\\d+)px;`, "u").exec(rule)?.[1], property));
}

/** A stubbed round trip can outrun the 1 s default under load. */
const SLOW = { timeout: 5_000 } as const;

beforeEach(() => {
  window.localStorage.clear();
  setReducedMotion(false);
  at("/");
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  setReducedMotion(false);
  resetLocaleForTests();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("R2875002 the landing's news bookmark", () => {
  it("R2875002 the bookmark sits beside the quests' with a red count of the unclaimed entries", async () => {
    signIn();
    serve([entry(), ALERT, entry({ id: "patch-0-5-9", version: "0.5.9", title: "Patch 0.5.9", claimed: true })]);
    render(<LandingRoute />);

    expect(await screen.findByTestId(landingTestid.newsCount, undefined, SLOW)).toHaveTextContent("2");
    const bookmark = screen.getByTestId(landingTestid.newsBookmark);
    expect(bookmark.tagName).toBe("BUTTON");
    // WCAG 2.5.3 (spec 38): the name holds the count the bubble shows.
    expect(bookmark).toHaveAccessibleName("News, 2 to claim");
    expect(bookmark).toHaveAttribute("aria-expanded", "false");
    const panel = screen.getByTestId(landingTestid.newsPanel);
    expect(bookmark.getAttribute("aria-controls")).toBe(panel.getAttribute("id"));
    expect(panel).toHaveAttribute("hidden");
    expect(screen.queryByTestId(newsTestid.entry)).toBeNull();

    // On the doors' box, the ribbon after the quests', hung past its right edge; the bubble is red.
    const ctas = within(landing()).getByRole("navigation", { name: "Play" });
    const quests = await within(ctas).findByTestId(landingTestid.questsBookmark, undefined, SLOW);
    expect(within(ctas).getByTestId(landingTestid.newsBookmark)).toBe(bookmark);
    expect(quests.parentElement?.nextElementSibling).toBe(bookmark.parentElement);
    const sheet = readSheet("src/routes/landing.css");
    const ribbons = must(
      /\.landing button\.landing-quests-bookmark,\s*\.landing button\.landing-news-bookmark \{([^}]*)\}/u.exec(sheet)?.[1],
      "the ribbons' rule",
    );
    // Beside the quests' ribbon when it shows; in its place, the shared rule's, when it does not.
    const news = must(
      /\.landing \.landing-quests ~ \.landing-news button\.landing-news-bookmark \{([^}]*)\}/u.exec(sheet)?.[1],
      "the news' rule",
    );
    expect(px(news, "left")).toBeGreaterThanOrEqual(px(ribbons, "left") + px(ribbons, "width"));
    expect(sheet).toMatch(/\.landing \.landing-news-count \{[^}]*background: var\(--opponent\);/u);
  });

  it("R2875002 all claimed shows no count", async () => {
    signIn();
    serve([entry({ claimed: true }), { ...ALERT, claimed: true }]);
    render(<LandingRoute />);

    const bookmark = await screen.findByTestId(landingTestid.newsBookmark, undefined, SLOW);
    expect(screen.queryByTestId(landingTestid.newsCount)).toBeNull();
    expect(bookmark).toHaveAccessibleName("News");

    // Open, each entry still reads, Claimed and nothing to press.
    fireEvent.click(bookmark);
    const rows = screen.getAllByTestId(newsTestid.entry);
    expect(rows).toHaveLength(2);
    expect(rows[0]).toHaveTextContent("Patch 0.6.0");
    expect(rows[0]).toHaveTextContent("The AI orders its own attacks by value.");
    expect(rows[0]).toHaveTextContent("100 Jlore");
    expect(rows[1]).toHaveTextContent("Servers back");
    expect(rows[1]).toHaveTextContent("Card token");
    expect(screen.getAllByTestId(newsTestid.claimed)).toHaveLength(2);
    expect(screen.queryByTestId(newsTestid.claim)).toBeNull();
  });

  it("R2875002 a claim takes the count down by one once the server answers", async () => {
    signIn();
    const server = serve([entry(), ALERT]);
    // The claim's answer waits for `answer`.
    let answer = (): void => {};
    const answered = new Promise<void>((resolve) => {
      answer = resolve;
    });
    const stubbed = globalThis.fetch;
    vi.stubGlobal(
      "fetch",
      vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
        const reply = stubbed(input, init);
        return init?.method === "POST" ? answered.then(() => reply) : reply;
      }),
    );
    render(<LandingRoute />);

    const count = await screen.findByTestId(landingTestid.newsCount, undefined, SLOW);
    expect(count).toHaveTextContent("2");
    fireEvent.click(screen.getByTestId(landingTestid.newsBookmark));
    fireEvent.click(screen.getByRole("button", { name: "Claim 100 Jlore" }));

    await waitFor(() => {
      expect(server.to("POST", "/api/news/patch-0-6-0/claim")).toHaveLength(1);
    }, SLOW);
    const [post] = server.to("POST", "/api/news/patch-0-6-0/claim");
    expect(post?.headers["idempotency-key"]).toMatch(UUID);
    expect(post?.body).toEqual({});
    // Not answered yet: the count holds and every Claim waits.
    expect(count).toHaveTextContent("2");
    for (const button of screen.getAllByTestId(newsTestid.claim)) expect(button).toBeDisabled();

    await act(async () => {
      answer();
      await answered;
    });
    await waitFor(() => {
      expect(screen.getByTestId(landingTestid.newsCount)).toHaveTextContent("1");
    }, SLOW);
    expect(screen.getByTestId(landingTestid.newsBookmark)).toHaveAccessibleName("News, 1 to claim");
    const [first, second] = screen.getAllByTestId(newsTestid.entry);
    expect(within(must(first, "the patch")).getByTestId(newsTestid.claimed)).toHaveTextContent("Claimed");
    expect(within(must(second, "the alert")).getByRole("button", { name: "Claim Card token" })).toBeEnabled();
    expect(newsReads(server)).toBe(1);
  });

  it("R2875002 a refused claim keeps the count and shows the server's words", async () => {
    signIn();
    const server = serve([entry(), ALERT], { claim: () => refusal(409, "conflict", "This reward was claimed already.") });
    render(<LandingRoute />);

    expect(await screen.findByTestId(landingTestid.newsCount, undefined, SLOW)).toHaveTextContent("2");
    fireEvent.click(screen.getByTestId(landingTestid.newsBookmark));
    fireEvent.click(screen.getByRole("button", { name: "Claim 100 Jlore" }));

    expect(await screen.findByTestId(newsTestid.error, undefined, SLOW)).toHaveTextContent("This reward was claimed already.");
    expect(screen.getByTestId(landingTestid.newsCount)).toHaveTextContent("2");
    expect(screen.queryByTestId(newsTestid.claimed)).toBeNull();
    for (const button of screen.getAllByTestId(newsTestid.claim)) expect(button).toBeEnabled();
    expect(server.to("POST", "/api/news/patch-0-6-0/claim")).toHaveLength(1);
  });

  it("R2875002 a failed read shows no bookmark", async () => {
    signIn();
    const server = stubServer((call) =>
      call.path === "/api/auth/me" ? { status: 200, body: meBody() } : refusal(503, "unavailable", "the server refused"),
    );
    render(<LandingRoute />);

    await waitFor(() => {
      expect(newsReads(server)).toBe(1);
    }, SLOW);
    // The quests' failed read still shows its bookmark (R2035000); the news' shows none.
    await screen.findByTestId(landingTestid.questsBookmark, undefined, SLOW);
    expect(screen.queryByTestId(landingTestid.newsBookmark)).toBeNull();
  });

  it("R2875002 a pending account gets no bookmark and no read", async () => {
    signIn();
    const server = serve([entry()], { status: "pending" });
    render(<LandingRoute />);

    expect(await screen.findByTestId(landingTestid.account)).toBeInTheDocument();
    expect(screen.queryByTestId(landingTestid.newsBookmark)).toBeNull();
    expect(newsReads(server)).toBe(0);
  });

  it("R2875002 with the economy off, none", async () => {
    signIn();
    const server = serve([entry()]);
    render(<LandingRoute economyLive={false} />);

    expect(await screen.findByTestId(landingTestid.account)).toBeInTheDocument();
    expect(screen.queryByTestId(landingTestid.newsBookmark)).toBeNull();
    expect(newsReads(server)).toBe(0);
  });

  it("R2875001 with no live entry, no bookmark", async () => {
    signIn();
    const server = serve([]);
    render(<LandingRoute />);

    await screen.findByTestId(landingTestid.questsBookmark, undefined, SLOW);
    await waitFor(() => {
      expect(newsReads(server)).toBe(1);
    });
    await waitFor(() => {
      expect(screen.queryByTestId(landingTestid.newsBookmark)).toBeNull();
    });
  });

  it("R2875002 Escape rolls it up and gives back the focus", async () => {
    signIn();
    serve([entry()]);
    render(<LandingRoute />);
    await screen.findByTestId(landingTestid.newsCount, undefined, SLOW);
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });

    const bookmark = screen.getByTestId(landingTestid.newsBookmark);
    const panel = screen.getByTestId(landingTestid.newsPanel);
    fireEvent.click(bookmark);
    expect(bookmark).toHaveAttribute("aria-expanded", "true");
    expect(panel).toHaveAttribute("data-state", "open");
    expect(screen.getByTestId(newsTestid.entry)).toBeInTheDocument();

    // With the focus on the Play door.
    screen.getByTestId(landingTestid.play).focus();
    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
    expect(bookmark).toHaveAttribute("aria-expanded", "false");
    expect(panel).toHaveAttribute("data-state", "closing");
    expect(document.activeElement).toBe(bookmark);
    act(() => {
      vi.advanceTimersByTime(QUESTS_UNROLL_MS);
    });
    expect(panel).toHaveAttribute("data-state", "closed");
    expect(panel).toHaveAttribute("hidden");
    expect(screen.queryByTestId(newsTestid.entry)).toBeNull();
  });
});

describe("R3074003 a dev's post on the news", () => {
  const POST = entry({
    id: "post-0a1b2c3d4e5f",
    kind: "alert",
    version: null,
    title: "Maintenance tonight",
    summary: "The server restarts at midnight UTC.",
    reward: null,
  });

  it("R3074003 a post shows no reward, counts as unread, and Mark read claims it, paying nothing", async () => {
    signIn();
    const server = serve([POST, entry()]);
    render(<LandingRoute />);

    expect(await screen.findByTestId(landingTestid.newsCount, undefined, SLOW)).toHaveTextContent("2");
    fireEvent.click(screen.getByTestId(landingTestid.newsBookmark));
    const [post, patch] = screen.getAllByTestId(newsTestid.entry);
    const row = must(post, "the post");
    expect(row).toHaveTextContent("Maintenance tonight");
    expect(row).toHaveTextContent("The server restarts at midnight UTC.");
    expect(row.querySelector(".entry-card__pill")).toBeNull();
    expect(within(must(patch, "the patch")).getByRole("button", { name: "Claim 100 Jlore" })).toBeEnabled();
    vi.mocked(announceWallet).mockClear();

    fireEvent.click(within(row).getByRole("button", { name: "Mark read" }));
    await waitFor(() => {
      expect(screen.getByTestId(landingTestid.newsCount)).toHaveTextContent("1");
    }, SLOW);
    expect(server.to("POST", "/api/news/post-0a1b2c3d4e5f/claim")).toHaveLength(1);
    const read = must(screen.getAllByTestId(newsTestid.entry)[0], "the post, read");
    expect(within(read).getByTestId(newsTestid.claimed)).toHaveTextContent("Read");
    // A post pays nothing, so nothing announces the wallet.
    expect(announceWallet).not.toHaveBeenCalled();
  });

  it("R3074003 a post's text shows as text, never as markup", async () => {
    signIn();
    serve([{ ...POST, title: "<b>Bold</b>", summary: "<img src=x onerror=alert(1)>." }]);
    render(<LandingRoute />);
    fireEvent.click(await screen.findByTestId(landingTestid.newsBookmark, undefined, SLOW));
    const row = must(screen.getAllByTestId(newsTestid.entry)[0], "the post");
    expect(row).toHaveTextContent("<b>Bold</b>");
    expect(row.querySelector("b")).toBeNull();
    expect(row.querySelector("img")).toBeNull();
  });
});

describe("R2875003 a news reward in words", () => {
  it("R2875003 each reward reads as its words", async () => {
    expect(rewardWords({ kind: "jlore", amount: 100 })).toBe("100 Jlore");
    expect(rewardWords({ kind: "jlore", amount: 1500 })).toBe("1,500 Jlore");
    expect(rewardWords({ kind: "cardPack", amount: 1, pack: GLOBAL_PACK_NAME })).toBe("Card pack");
    expect(rewardWords({ kind: "cardPack", amount: 2, pack: GLOBAL_PACK_NAME })).toBe("Card pack × 2");
    expect(rewardWords({ kind: "cardPack", amount: 1, pack: "Stinkyscape" })).toBe("Stinkyscape pack");
    expect(rewardWords({ kind: "cardPack", amount: 3, pack: "Stinkyscape" })).toBe("Stinkyscape pack × 3");
    expect(rewardWords({ kind: "cardToken", amount: 1 })).toBe("Card token");
    expect(rewardWords({ kind: "cardToken", amount: 2 })).toBe("Card token × 2");
    expect(rewardWords({ kind: "cosmeticToken", amount: 1, tier: "Rare" })).toBe("Rare cosmetic token");
    expect(rewardWords({ kind: "cosmeticToken", amount: 1, tier: "Epic" })).toBe("Epic cosmetic token");
    expect(rewardWords({ kind: "cosmeticToken", amount: 2, tier: "Legendary" })).toBe("Legendary cosmetic token × 2");
    expect(rewardWords({ kind: "cosmeticPack", amount: 1 })).toBe("Cosmetic pack");

    // In Chinese a set's pack names the set as the rest of the client does (`packs.ts`'s `packName`).
    await setLocale("zh-CN");
    expect(rewardWords({ kind: "cardPack", amount: 3, pack: "Core" })).toBe("核心卡包 × 3");
    expect(rewardWords({ kind: "cosmeticToken", amount: 1, tier: "Rare" })).toBe("稀有外观代币");
  });
});

describe("R2982000 Claim all", () => {
  /**
   * The server keeps every claim a run has landed, so each answer of the run builds on the last
   * (unlike `serve`'s default, which answers every claim from the list as first served).
   */
  function keeping(all: readonly NewsEntry[]): (id: string) => Reply {
    const done = new Set<string>();
    return (id) => {
      done.add(id);
      return { status: 200, body: { news: all.map((one) => (done.has(one.id) ? { ...one, claimed: true } : one)) } };
    };
  }

  it("R2982000 Claim all claims each unclaimed entry newest first, the count dropping per answer", async () => {
    vi.mocked(announceWallet).mockClear();
    signIn();
    const list = [
      entry(),
      ALERT,
      entry({
        id: "patch-0-5-9",
        version: "0.5.9",
        title: "Patch 0.5.9",
        summary: "Older news.",
        reward: { kind: "cardToken", amount: 1 },
        claimed: true,
      }),
    ];
    const server = serve(list, { claim: keeping(list) });
    // Each claim's answer waits for its own `release`, so the count drops one answer at a time.
    let releasePatch = (): void => {};
    const patchReleased = new Promise<void>((resolve) => {
      releasePatch = resolve;
    });
    let releaseAlert = (): void => {};
    const alertReleased = new Promise<void>((resolve) => {
      releaseAlert = resolve;
    });
    const stubbed = globalThis.fetch;
    vi.stubGlobal(
      "fetch",
      vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
        const reply = stubbed(input, init);
        if (init?.method === "POST" && String(input).includes("patch-0-6-0")) return patchReleased.then(() => reply);
        if (init?.method === "POST" && String(input).includes("alert-1")) return alertReleased.then(() => reply);
        return reply;
      }),
    );
    render(<LandingRoute />);

    expect(await screen.findByTestId(landingTestid.newsCount, undefined, SLOW)).toHaveTextContent("2");
    fireEvent.click(screen.getByTestId(landingTestid.newsBookmark));
    fireEvent.click(await screen.findByTestId(newsTestid.claimAll, undefined, SLOW));

    // The newest claim is out but unanswered: the count holds, and the run sends nothing else yet.
    await waitFor(() => {
      expect(server.to("POST", "/api/news/patch-0-6-0/claim")).toHaveLength(1);
    }, SLOW);
    expect(screen.getByTestId(landingTestid.newsCount)).toHaveTextContent("2");
    expect(server.to("POST", "/api/news/alert-1/claim")).toHaveLength(0);
    await act(async () => {
      releasePatch();
      await patchReleased;
    });
    // The patch's answer took the count down by one; the run went on to the alert on its own.
    await waitFor(() => {
      expect(screen.getByTestId(landingTestid.newsCount)).toHaveTextContent("1");
    }, SLOW);
    await waitFor(() => {
      expect(server.to("POST", "/api/news/alert-1/claim")).toHaveLength(1);
    }, SLOW);
    await act(async () => {
      releaseAlert();
      await alertReleased;
    });
    // At zero the bubble is gone, as when all were claimed to begin with.
    await waitFor(() => {
      expect(screen.queryByTestId(landingTestid.newsCount)).toBeNull();
    }, SLOW);
    expect(screen.getByTestId(landingTestid.newsBookmark)).toHaveAccessibleName("News");
    expect(server.to("POST", "/api/news/patch-0-6-0/claim")).toHaveLength(1);
    expect(server.to("POST", "/api/news/alert-1/claim")).toHaveLength(1);
    // Newest first: the patch's POST went out before the alert's.
    const paths = server.calls.filter((call) => call.method === "POST").map((call) => call.path);
    expect(paths).toEqual(["/api/news/patch-0-6-0/claim", "/api/news/alert-1/claim"]);
    // Each claim sent its own Idempotency-Key.
    const [patchPost] = server.to("POST", "/api/news/patch-0-6-0/claim");
    const [alertPost] = server.to("POST", "/api/news/alert-1/claim");
    expect(patchPost?.headers["idempotency-key"]).toMatch(UUID);
    expect(alertPost?.headers["idempotency-key"]).toMatch(UUID);
    expect(alertPost?.headers["idempotency-key"]).not.toBe(patchPost?.headers["idempotency-key"]);
    expect(screen.queryByTestId(newsTestid.claimAll)).toBeNull();
    expect(screen.getAllByTestId(newsTestid.claimed)).toHaveLength(3);
  });

  it("R2982000 a refusal partway stops the run and shows news-error", async () => {
    vi.mocked(announceWallet).mockClear();
    signIn();
    const entries = [
      entry(),
      ALERT,
      entry({
        id: "patch-0-5-9",
        version: "0.5.9",
        title: "Patch 0.5.9",
        summary: "Older news.",
        reward: { kind: "cardToken", amount: 1 },
      }),
    ];
    const server = serve(entries, {
      claim: (id) => (id === ALERT.id ? refusal(409, "conflict", "This reward was claimed already.") : claimed(entries, id)),
    });
    render(<LandingRoute />);

    expect(await screen.findByTestId(landingTestid.newsCount, undefined, SLOW)).toHaveTextContent("3");
    fireEvent.click(screen.getByTestId(landingTestid.newsBookmark));
    fireEvent.click(await screen.findByTestId(newsTestid.claimAll, undefined, SLOW));

    expect(await screen.findByTestId(newsTestid.error, undefined, SLOW)).toHaveTextContent("This reward was claimed already.");
    // The first claim stands, the third was never sent, and the run stopped waiting.
    expect(screen.getByTestId(landingTestid.newsCount)).toHaveTextContent("2");
    const [first] = screen.getAllByTestId(newsTestid.entry);
    expect(within(must(first, "the patch")).getByTestId(newsTestid.claimed)).toHaveTextContent("Claimed");
    expect(server.to("POST", "/api/news/patch-0-5-9/claim")).toHaveLength(0);
    expect(screen.getByTestId(newsTestid.claimAll)).toBeEnabled();
    for (const button of screen.getAllByTestId(newsTestid.claim)) {
      expect(button).toBeEnabled();
      expect(button).toHaveAttribute("aria-busy", "false");
    }
  });

  it("R2982000 while the run is out every Claim and Claim all are disabled with aria-busy", async () => {
    vi.mocked(announceWallet).mockClear();
    signIn();
    const list = [entry(), ALERT];
    const server = serve(list, { claim: keeping(list) });
    // Every claim's answer waits for `answer`.
    let answer = (): void => {};
    const answered = new Promise<void>((resolve) => {
      answer = resolve;
    });
    const stubbed = globalThis.fetch;
    vi.stubGlobal(
      "fetch",
      vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
        const reply = stubbed(input, init);
        return init?.method === "POST" ? answered.then(() => reply) : reply;
      }),
    );
    render(<LandingRoute />);

    expect(await screen.findByTestId(landingTestid.newsCount, undefined, SLOW)).toHaveTextContent("2");
    fireEvent.click(screen.getByTestId(landingTestid.newsBookmark));
    fireEvent.click(await screen.findByTestId(newsTestid.claimAll, undefined, SLOW));

    await waitFor(() => {
      expect(server.to("POST", "/api/news/patch-0-6-0/claim")).toHaveLength(1);
    }, SLOW);
    const claimAll = screen.getByTestId(newsTestid.claimAll);
    expect(claimAll).toBeDisabled();
    expect(claimAll).toHaveAttribute("aria-busy", "true");
    for (const button of screen.getAllByTestId(newsTestid.claim)) {
      expect(button).toBeDisabled();
      expect(button).toHaveAttribute("aria-busy", "true");
    }

    await act(async () => {
      answer();
      await answered;
    });
    await waitFor(() => {
      expect(screen.queryByTestId(landingTestid.newsCount)).toBeNull();
    }, SLOW);
    expect(screen.queryByTestId(newsTestid.claimAll)).toBeNull();
    expect(screen.queryByTestId(newsTestid.claim)).toBeNull();
  });

  it("R2982000 announceWallet fires once for a run that paid Jlore", async () => {
    vi.mocked(announceWallet).mockClear();
    signIn();
    const list = [
      entry(),
      entry({
        id: "patch-0-5-9",
        version: "0.5.9",
        title: "Patch 0.5.9",
        summary: "Older news.",
        reward: { kind: "jlore", amount: 50 },
      }),
    ];
    serve(list, { claim: keeping(list) });
    render(<LandingRoute />);

    expect(await screen.findByTestId(landingTestid.newsCount, undefined, SLOW)).toHaveTextContent("2");
    fireEvent.click(screen.getByTestId(landingTestid.newsBookmark));
    fireEvent.click(await screen.findByTestId(newsTestid.claimAll, undefined, SLOW));

    await waitFor(() => {
      expect(screen.queryByTestId(landingTestid.newsCount)).toBeNull();
    }, SLOW);
    expect(announceWallet).toHaveBeenCalledTimes(1);
  });

  it("R2982000 a single claim still announces once, and a non-Jlore claim not at all", async () => {
    vi.mocked(announceWallet).mockClear();
    signIn();
    const list = [entry(), ALERT];
    serve(list, { claim: keeping(list) });
    render(<LandingRoute />);

    expect(await screen.findByTestId(landingTestid.newsCount, undefined, SLOW)).toHaveTextContent("2");
    fireEvent.click(screen.getByTestId(landingTestid.newsBookmark));
    expect(screen.getByTestId(newsTestid.claimAll)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Claim 100 Jlore" }));

    await waitFor(() => {
      expect(screen.getByTestId(landingTestid.newsCount)).toHaveTextContent("1");
    }, SLOW);
    expect(announceWallet).toHaveBeenCalledTimes(1);
    // Below two unclaimed, Claim all is gone.
    expect(screen.queryByTestId(newsTestid.claimAll)).toBeNull();

    vi.mocked(announceWallet).mockClear();
    fireEvent.click(screen.getByRole("button", { name: "Claim Card token" }));

    await waitFor(() => {
      expect(screen.queryByTestId(landingTestid.newsCount)).toBeNull();
    }, SLOW);
    expect(announceWallet).not.toHaveBeenCalled();
  });
});
