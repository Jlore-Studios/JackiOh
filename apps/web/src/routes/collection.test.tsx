// `/collection` (#1307, SPEC §14): each panel reads with the caller's token and never goes blank when a
// read is refused. R1579000: the hub of four doors. R1953001: no defaults of every card in it.

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { newestShippedSet } from "@jackioh/shared";

import {
  COSMETICS_SETTINGS_KEY,
  readCardPicks,
  readCosmetics,
  resetCosmeticsForTests,
  writeCardPick,
  writeCosmetics,
} from "../cards/cosmetics.ts";
import { INSPECT_DETAIL } from "../cards/inspect/testids.ts";
import { collectionTestid as T } from "../collection/collection.ts";
import { fullCollection, questsHeld, tracksOf } from "../collection/fixtures.ts";
import {
  ApiRequestError,
  getCollection,
  getCosmeticPacks,
  getOwnedCosmetics,
  getPacks,
  getQuests,
  getTokens,
  getTracks,
  getWallet,
  keptPaths,
  type CollectionResponse,
} from "../net/api.ts";
import { documentTitleFor, screenNameFor } from "../net/head.ts";
import { paths } from "../net/navigate.ts";
import { rememberRead } from "../net/readCache.ts";
import { packsInfo } from "../packs/fixtures.ts";
import { SETTINGS_GROUPS } from "../settings/groups.ts";
import { cosmeticPacksInfo, ownedCosmeticsOf, tokenBars, walletInfo } from "../shop/fixtures.ts";
import { pressBack } from "../test/history.ts";
import { expectSkeleton } from "../test/skeleton.ts";
import { navTestid } from "./nav.tsx";
import CollectionRoute from "./collection.tsx";
import { questsTestid } from "./QuestsPanel.tsx";

vi.mock("../net/api.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/api.ts")>();
  return {
    ...actual,
    getCollection: vi.fn(),
    getWallet: vi.fn(),
    getPacks: vi.fn(),
    getCosmeticPacks: vi.fn(),
    getOwnedCosmetics: vi.fn(),
    getTracks: vi.fn(),
    getQuests: vi.fn(),
    getTokens: vi.fn(),
  };
});

const TOKEN = "token-1";

function answerAll(): void {
  vi.mocked(getCollection).mockResolvedValue(fullCollection());
  vi.mocked(getWallet).mockResolvedValue(walletInfo());
  vi.mocked(getPacks).mockResolvedValue(packsInfo());
  vi.mocked(getCosmeticPacks).mockResolvedValue(cosmeticPacksInfo());
  vi.mocked(getTracks).mockResolvedValue(tracksOf());
  vi.mocked(getQuests).mockResolvedValue(questsHeld());
  vi.mocked(getOwnedCosmetics).mockResolvedValue(ownedCosmeticsOf());
  vi.mocked(getTokens).mockResolvedValue(tokenBars());
}

beforeEach(() => {
  window.history.replaceState(null, "", paths.collection);
});

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

describe("#1307 the collection route", () => {
  beforeEach(() => {
    window.history.replaceState(null, "", paths.collectionSection("cards"));
  });

  it("reads every panel with the caller's token", async () => {
    answerAll();
    render(<CollectionRoute token={TOKEN} />);

    expect(await screen.findByTestId(T.completion("Core"))).toBeInTheDocument();
    expect(await screen.findByTestId(T.jlore)).toHaveTextContent("37 Jlore");
    expect(await screen.findByTestId(T.track(newestShippedSet()))).toBeInTheDocument();
    expect(await screen.findByTestId(questsTestid.panel)).toHaveTextContent("Win 3 games");
    for (const read of [getCollection, getWallet, getPacks, getCosmeticPacks, getTracks, getQuests]) {
      expect(read).toHaveBeenCalledWith(TOKEN);
    }
    expect(screen.getByTestId(T.screen)).toBeInTheDocument();
  });

  it("shows each panel's loading message until it answers", async () => {
    answerAll();
    let answerCards: (collection: CollectionResponse) => void = () => undefined;
    vi.mocked(getCollection).mockReturnValue(
      new Promise<CollectionResponse>((resolve) => {
        answerCards = resolve;
      }),
    );
    render(<CollectionRoute token={TOKEN} />);

    expect(screen.getByTestId(T.loading("cards"))).toHaveTextContent("Loading your cards…");
    expectSkeleton(screen.getByTestId(T.loading("cards")), "rows");
    expect(await screen.findByTestId(T.jlore)).toBeInTheDocument();
    expect(screen.queryByTestId(T.loading("wallet"))).toBeNull();
    expect(screen.queryByTestId(T.loading("tracks"))).toBeNull();
    expect(screen.getByTestId(T.loading("cards"))).toBeInTheDocument();

    await act(async () => {
      answerCards(fullCollection());
      await Promise.resolve();
    });
    expect(screen.queryByTestId(T.loading("cards"))).toBeNull();
    expect(screen.getByTestId(T.completion("Core"))).toBeInTheDocument();
  });

  it("shows the wallet and tracks loading messages before they answer", () => {
    vi.mocked(getWallet).mockReturnValue(new Promise(() => undefined));
    vi.mocked(getPacks).mockReturnValue(new Promise(() => undefined));
    vi.mocked(getCosmeticPacks).mockReturnValue(new Promise(() => undefined));
    vi.mocked(getTracks).mockReturnValue(new Promise(() => undefined));
    vi.mocked(getCollection).mockReturnValue(new Promise(() => undefined));
    vi.mocked(getQuests).mockReturnValue(new Promise(() => undefined));
    render(<CollectionRoute token={TOKEN} />);

    expect(screen.getByTestId(T.loading("wallet"))).toHaveTextContent("Loading your wallet…");
    expectSkeleton(screen.getByTestId(T.loading("wallet")), "rows");
    expect(screen.getByTestId(T.loading("tracks"))).toHaveTextContent("Loading your tracks…");
    expectSkeleton(screen.getByTestId(T.loading("tracks")), "rows");
  });

  it("a refused read says why in its panel; the others still show", async () => {
    answerAll();
    vi.mocked(getPacks).mockRejectedValue(new Error("the packs are unavailable"));
    render(<CollectionRoute token={TOKEN} />);

    expect(await screen.findByTestId(T.error("wallet"))).toHaveTextContent("the packs are unavailable");
    expect(await screen.findByTestId(T.completion("Core"))).toBeInTheDocument();
    expect(await screen.findByTestId(T.track(newestShippedSet()))).toBeInTheDocument();
    expect(screen.queryByTestId(T.error("cards"))).toBeNull();
    expect(screen.queryByTestId(T.error("tracks"))).toBeNull();
    expect(screen.queryByTestId(T.jlore)).toBeNull();
  });

  it("every read refused shows messages, not a blank screen", async () => {
    for (const read of [getCollection, getWallet, getPacks, getCosmeticPacks, getTracks, getQuests]) {
      vi.mocked(read).mockRejectedValue(new Error("the server refused"));
    }
    render(<CollectionRoute token={TOKEN} />);

    for (const part of ["cards", "wallet", "tracks"] as const) {
      expect(await screen.findByTestId(T.error(part))).toHaveTextContent("the server refused");
    }
    expect(await screen.findByTestId(questsTestid.error)).toHaveTextContent("the server refused");
    expect(screen.getByRole("heading", { name: "Cards", level: 2 })).toBeInTheDocument();
  });

  it("#1691 a panel whose read failed offers Try again, which reads it again", async () => {
    answerAll();
    vi.mocked(getPacks).mockRejectedValueOnce(new Error("the packs are unavailable"));
    render(<CollectionRoute token={TOKEN} />);

    expect(await screen.findByTestId(T.error("wallet"))).toHaveTextContent("the packs are unavailable");
    expect(screen.getByTestId(T.retry("wallet"))).toBeEnabled();
    expect(screen.queryByTestId(T.retry("cards"))).toBeNull();
    expect(screen.queryByTestId(T.retry("tracks"))).toBeNull();

    fireEvent.click(screen.getByTestId(T.retry("wallet")));
    expect(await screen.findByTestId(T.jlore)).toHaveTextContent("37 Jlore");
    expect(screen.queryByTestId(T.error("wallet"))).toBeNull();
    expect(screen.queryByTestId(T.retry("wallet"))).toBeNull();
    expect(getPacks).toHaveBeenCalledTimes(2);
    expect(getCollection).toHaveBeenCalledTimes(1);
  });

  it("#1691 a rate-limited panel holds Try again", async () => {
    answerAll();
    vi.mocked(getTracks).mockRejectedValue(
      new ApiRequestError(429, { code: "rate_limited", message: "slow down", details: { retryAfterMs: 60_000 } }),
    );
    render(<CollectionRoute token={TOKEN} />);

    expect(await screen.findByTestId(T.error("tracks"))).toHaveTextContent(/You can try again in \d+ s\./);
    const retry = screen.getByTestId(T.retry("tracks"));
    expect(retry).toBeDisabled();
    expect(retry).toHaveTextContent(/Try again in \d+ s/);
  });

  it("names the screen Collection", () => {
    expect(screenNameFor(paths.collection)).toBe("Collection");
    expect(documentTitleFor(paths.collection)).toMatch(/^Collection · /u);
  });
});

describe("R1579000 the Collection hub", () => {
  beforeEach(() => {
    localStorage.clear();
    resetCosmeticsForTests();
  });

  afterEach(() => {
    localStorage.clear();
    resetCosmeticsForTests();
  });

  const READS = [getCollection, getWallet, getPacks, getCosmeticPacks, getTracks, getQuests, getOwnedCosmetics];

  it("R1579000 offers four doors to the decks, the cards, the cosmetics and the card builder, and reads nothing", () => {
    answerAll();
    render(<CollectionRoute token={TOKEN} />);

    expect(screen.getByTestId(T.screen)).toHaveAttribute("data-section", "hub");
    expect(screen.getByRole("heading", { name: "Your collection" })).toBeInTheDocument();
    expect(screen.getByTestId(T.door("decks"))).toHaveAttribute("href", paths.decks);
    expect(screen.getByTestId(T.door("cards"))).toHaveAttribute("href", paths.collectionSection("cards"));
    expect(screen.getByTestId(T.door("cosmetics"))).toHaveAttribute("href", paths.collectionSection("cosmetics"));
    expect(screen.getByTestId(T.door("builder"))).toHaveAttribute("href", paths.creator);
    for (const read of READS) expect(read).not.toHaveBeenCalled();

    fireEvent.click(screen.getByTestId(T.door("builder")));
    expect(window.location.pathname).toBe(paths.creator);

    window.history.replaceState(null, "", paths.collection);
    fireEvent.click(screen.getByTestId(navTestid.back));
    expect(window.location.pathname).toBe(paths.landing);
  });

  it("R1579000 the Cards door opens the cards, wallet, tracks and quests in place, and Back returns to the hub", async () => {
    answerAll();
    render(<CollectionRoute token={TOKEN} />);

    fireEvent.click(screen.getByTestId(T.door("cards")));
    expect(screen.getByTestId(T.screen)).toHaveAttribute("data-section", "cards");
    expect(await screen.findByTestId(T.completion("Core"))).toBeInTheDocument();
    expect(await screen.findByTestId(T.jlore)).toBeInTheDocument();
    expect(await screen.findByTestId(questsTestid.panel)).toHaveTextContent("Win 3 games");
    expect(screen.getByTestId(T.body)).toBeInTheDocument();
    expect(screen.queryByTestId(T.door("cards"))).toBeNull();

    fireEvent.click(screen.getByTestId(navTestid.back));
    expect(screen.getByTestId(T.screen)).toHaveAttribute("data-section", "hub");
    expect(screen.getByTestId(T.door("cards"))).toBeInTheDocument();
    expect(window.location.search).toBe("");
    expect(window.location.pathname).toBe(paths.collection);
  });

  it("R1579000 the Cosmetics door lists what the account owns and equips through the same store, with no request", async () => {
    answerAll();
    render(<CollectionRoute token={TOKEN} />);

    fireEvent.click(screen.getByTestId(T.door("cosmetics")));
    expect(await screen.findByTestId(T.ownedCosmetic("full-art-gary-the-gambler", "core-004"))).toBeInTheDocument();
    expect(screen.getAllByTestId(T.ownedCosmetic("table-felt", null))).toHaveLength(1);
    expect(screen.queryByTestId(T.ownedCosmetic("foil-none", null))).toBeNull();

    const group = SETTINGS_GROUPS.find((candidate) => candidate.id === "cosmetics");
    if (group === undefined) throw new Error("no cosmetics settings group");
    const changed = vi.fn();
    const unsubscribe = group.subscribe(changed);
    // R1655000: the playmat, backdrop and card back are tiles.
    fireEvent.click(within(screen.getByTestId(T.tile("table-felt"))).getByRole("button"));
    fireEvent.click(within(screen.getByTestId(T.tile("card-back-starfield"))).getByRole("button"));
    unsubscribe();

    expect(screen.getByTestId(T.tile("table-felt"))).toHaveAttribute("data-equipped", "true");
    expect(readCosmetics()).toMatchObject({ table: "table-felt", cardBack: "card-back-starfield" });
    expect(JSON.parse(localStorage.getItem(COSMETICS_SETTINGS_KEY) ?? "null")).toMatchObject({
      table: "table-felt",
      cardBack: "card-back-starfield",
    });
    expect(group.read()["table"]).toBe("table-felt");
    expect(changed).toHaveBeenCalled();
    expect(getOwnedCosmetics).toHaveBeenCalledTimes(1);
    expect(getOwnedCosmetics).toHaveBeenCalledWith(TOKEN);
    for (const read of [getCollection, getWallet, getPacks, getCosmeticPacks, getTracks, getQuests]) {
      expect(read).not.toHaveBeenCalled();
    }
  });

  it("R1579000 the browser's Back and Forward move between the hub and a section", async () => {
    answerAll();
    render(<CollectionRoute token={TOKEN} />);
    expect(screen.getByTestId(T.screen)).toHaveAttribute("data-section", "hub");

    act(() => {
      window.history.pushState(null, "", paths.collectionSection("cosmetics"));
      window.dispatchEvent(new PopStateEvent("popstate"));
    });
    expect(screen.getByTestId(T.screen)).toHaveAttribute("data-section", "cosmetics");
    expect(await screen.findByTestId(T.tiles("table"))).toBeInTheDocument();

    act(() => {
      window.history.replaceState(null, "", paths.collection);
      window.dispatchEvent(new PopStateEvent("popstate"));
    });
    expect(screen.getByTestId(T.screen)).toHaveAttribute("data-section", "hub");
  });

  it("R1953001 the Cosmetics tab has no Defaults for every card section", async () => {
    answerAll();
    // The synced settings hold a foil, a finish and a border no select changes.
    writeCosmetics({ foil: "foil-rainbow", finish: "finish-linen", border: "border-wood" });
    render(<CollectionRoute token={TOKEN} />);

    fireEvent.click(screen.getByTestId(T.door("cosmetics")));
    expect(await screen.findByTestId(T.ownedCosmetic("full-art-gary-the-gambler", "core-004"))).toBeInTheDocument();
    expect(screen.queryByText("Defaults for every card")).toBeNull();
    expect(screen.queryByRole("heading", { name: "Defaults for every card" })).toBeNull();
    expect(screen.queryByRole("combobox")).toBeNull();
    expect(screen.getByRole("heading", { name: "Equipped" })).toBeInTheDocument();
    expect(screen.getByTestId(T.tiles("table"))).toBeInTheDocument();
    // R1655000 freezes the group's shape: the picks stay stored.
    expect(readCosmetics()).toMatchObject({ foil: "foil-rainbow", finish: "finish-linen", border: "border-wood" });
  });

  it("R1579000 an unknown section opens the hub", () => {
    answerAll();
    window.history.replaceState(null, "", `${paths.collection}?section=shop`);
    render(<CollectionRoute token={TOKEN} />);

    expect(screen.getByTestId(T.screen)).toHaveAttribute("data-section", "hub");
    expect(screen.getByTestId(T.door("decks"))).toBeInTheDocument();
    for (const read of READS) expect(read).not.toHaveBeenCalled();
  });
});

describe("R1655000 the Collection's card view", () => {
  beforeEach(() => {
    localStorage.clear();
    resetCosmeticsForTests();
    window.history.replaceState(null, "", paths.collectionSection("cards"));
  });

  afterEach(() => {
    localStorage.clear();
    resetCosmeticsForTests();
  });

  it("R1655000 a card of the Cards section opens large wearing its own picks, and equipping on it asks the server nothing", async () => {
    answerAll();
    writeCardPick("core-001", "border", "border-wood");
    render(<CollectionRoute token={TOKEN} />);

    fireEvent.click(await screen.findByTestId(T.setToggle("Core")));
    fireEvent.click(screen.getByTestId(T.card("core-001")));
    const face = screen.getByTestId(T.cardViewFace).querySelector(".cf");
    expect(face?.getAttribute("data-frame")).toBe("border-wood");

    fireEvent.click(screen.getByTestId(T.stripItem("foil-rainbow")));
    expect(readCardPicks("core-001")).toEqual({ border: "border-wood", foil: "foil-rainbow" });
    expect(face?.getAttribute("data-cosmetic-foil")).toBe("foil-rainbow");
    expect(readCosmetics().foil).toBe("foil-none");
    // Ownership is the session's (`useCanEquip`): the section reads nothing more for it.
    expect(getOwnedCosmetics).not.toHaveBeenCalled();
  });

  it("R2127000 Back closes the card's text, then the card's view, one per Back, gives the focus back to the card and keeps ?section=cards and the open set; with none open Back leaves for the hub", async () => {
    answerAll();
    window.history.replaceState(null, "", paths.collection);
    window.history.pushState(null, "", paths.collectionSection("cards"));
    render(<CollectionRoute token={TOKEN} />);

    fireEvent.click(await screen.findByTestId(T.setToggle("Core")));
    const card = screen.getByTestId(T.card("core-001"));
    card.focus();
    fireEvent.click(card);
    expect(screen.getByTestId(T.cardView)).toHaveAttribute("data-card", "core-001");
    fireEvent.click(screen.getByTestId(T.cardViewText));
    expect(screen.getByTestId(INSPECT_DETAIL)).toBeInTheDocument();

    // One Back closes the text over the view, and only it.
    await pressBack();
    expect(screen.queryByTestId(INSPECT_DETAIL)).toBeNull();
    expect(screen.getByTestId(T.cardView)).toBeInTheDocument();
    expect(window.location.search).toBe("?section=cards");

    // The next closes the view: the section, the open set and the focus are as they were.
    await pressBack();
    expect(screen.queryByTestId(T.cardView)).toBeNull();
    expect(card).toHaveFocus();
    expect(window.location.pathname).toBe(paths.collection);
    expect(window.location.search).toBe("?section=cards");
    expect(screen.getByTestId(T.screen)).toHaveAttribute("data-section", "cards");
    expect(screen.getByTestId(T.setToggle("Core"))).toHaveAttribute("aria-expanded", "true");

    // With none open, Back leaves the section as it did.
    await pressBack();
    expect(screen.getByTestId(T.screen)).toHaveAttribute("data-section", "hub");
    expect(window.location.search).toBe("");
  });
});

describe("R2125000 the Cards section opens on what the last visit kept", () => {
  beforeEach(() => {
    window.history.replaceState(null, "", paths.collectionSection("cards"));
  });

  /** Every read waits for ever: only what is kept can be on screen. */
  function pendAll(): void {
    vi.mocked(getCollection).mockReturnValue(new Promise(() => undefined));
    vi.mocked(getWallet).mockReturnValue(new Promise(() => undefined));
    vi.mocked(getPacks).mockReturnValue(new Promise(() => undefined));
    vi.mocked(getCosmeticPacks).mockReturnValue(new Promise(() => undefined));
    vi.mocked(getTracks).mockReturnValue(new Promise(() => undefined));
    vi.mocked(getQuests).mockReturnValue(new Promise(() => undefined));
  }

  /** What a first visit's reads left behind. */
  function keepWallet(): void {
    rememberRead(TOKEN, keptPaths.wallet, walletInfo());
    rememberRead(TOKEN, keptPaths.packs, packsInfo());
    rememberRead(TOKEN, keptPaths.cosmeticPacks, cosmeticPacksInfo());
    rememberRead(TOKEN, keptPaths.quests, questsHeld());
  }

  it("R2125000 a revisit draws the kept wallet and quests at once, the rest loading", () => {
    keepWallet();
    pendAll();
    render(<CollectionRoute token={TOKEN} />);

    expect(screen.getByTestId(T.jlore)).toHaveTextContent("37 Jlore");
    expect(screen.queryByTestId(T.loading("wallet"))).toBeNull();
    expect(screen.getByTestId(questsTestid.panel)).toHaveTextContent("Win 3 games");
    expect(screen.queryByTestId(questsTestid.loading)).toBeNull();
    expect(screen.getByTestId(T.loading("cards"))).toBeInTheDocument();
    expect(screen.getByTestId(T.loading("tracks"))).toBeInTheDocument();
    // Every panel still reads again behind what it shows.
    for (const read of [getCollection, getWallet, getPacks, getCosmeticPacks, getTracks, getQuests]) {
      expect(read).toHaveBeenCalledWith(TOKEN);
    }
  });

  it("R2125000 half of a panel's answers is a first visit: its skeleton shows", () => {
    rememberRead(TOKEN, keptPaths.wallet, walletInfo());
    pendAll();
    render(<CollectionRoute token={TOKEN} />);

    expect(screen.getByTestId(T.loading("wallet"))).toBeInTheDocument();
    expect(screen.queryByTestId(T.jlore)).toBeNull();
  });

  it("R2125000 a read that fails behind a kept panel keeps it and says nothing", async () => {
    keepWallet();
    answerAll();
    vi.mocked(getWallet).mockRejectedValue(new Error("the wallet is unavailable"));
    vi.mocked(getQuests).mockRejectedValue(new Error("the quests are unavailable"));
    render(<CollectionRoute token={TOKEN} />);

    await act(async () => {
      await new Promise((resolve) => {
        setTimeout(resolve, 0);
      });
    });

    expect(screen.getByTestId(T.jlore)).toHaveTextContent("37 Jlore");
    expect(screen.queryByTestId(T.error("wallet"))).toBeNull();
    expect(screen.queryByTestId(T.retry("wallet"))).toBeNull();
    expect(screen.getByTestId(questsTestid.panel)).toHaveTextContent("Win 3 games");
    expect(screen.queryByTestId(questsTestid.error)).toBeNull();
    expect(getWallet).toHaveBeenCalledTimes(1);
  });

  it("R2125000 another account's kept answers never draw", () => {
    keepWallet();
    pendAll();
    render(<CollectionRoute token="token-other" />);

    expect(screen.getByTestId(T.loading("wallet"))).toBeInTheDocument();
    expect(screen.queryByTestId(T.jlore)).toBeNull();
    expect(screen.getByTestId(questsTestid.loading)).toBeInTheDocument();
  });

  it("R2125000 an account switch behind a failing read never keeps the other account's answer", async () => {
    keepWallet();
    pendAll();
    const { rerender } = render(<CollectionRoute token={TOKEN} />);
    expect(screen.getByTestId(T.jlore)).toHaveTextContent("37 Jlore");

    vi.mocked(getWallet).mockRejectedValue(new Error("the wallet is unavailable"));
    rerender(<CollectionRoute token="token-other" />);
    await act(async () => {
      await new Promise((resolve) => {
        setTimeout(resolve, 0);
      });
    });

    expect(screen.queryByTestId(T.jlore)).toBeNull();
    expect(screen.getByTestId(T.error("wallet"))).toBeInTheDocument();
  });
});
