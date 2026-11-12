// `/collection` (#1307, SPEC §14): each panel reads with the caller's token and never goes blank when a
// read is refused. R3734000: four tabs, Cards, Decks, Cosmetics and Progress, each holding its filters across a
// switch. R1953001: no defaults of every card in it.

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
import { COLLECTED_DEFS, COLLECTION_SECTIONS, collectionTestid as T, type CollectionSection } from "../collection/collection.ts";
import { fullCollection, questsHeld, tracksOf } from "../collection/fixtures.ts";
import {
  ApiRequestError,
  getCollection,
  getCosmeticPacks,
  getDecks,
  getOwnedCosmetics,
  getPacks,
  getQuests,
  getTokens,
  getTracks,
  getWallet,
  keptPaths,
  type CollectionResponse,
} from "../net/api.ts";
import { currentCatalog } from "../game/currentCatalog.ts";
import { DECK_SIZE } from "../game/deckbuilder/deckSize.ts";
import { poolFrom } from "../game/deckbuilder/loadout.ts";
import { DB_EMPTY, DB_SEARCH, DB_SORT, filterOwnershipId, filterRarityId } from "../game/deckbuilder/testids.ts";
import { decksResponse, savedDeck } from "../game/deckbuilder/testkit.ts";
import { documentTitleFor, screenNameFor } from "../net/head.ts";
import { deckPath, newDeckPath, paths } from "../net/navigate.ts";
import { rememberRead } from "../net/readCache.ts";
import { packsInfo } from "../packs/fixtures.ts";
import { SETTINGS_GROUPS } from "../settings/groups.ts";
import { cosmeticPacksInfo, ownedCosmeticsOf, tokenBars, walletInfo } from "../shop/fixtures.ts";
import { pressBack } from "../test/history.ts";
import { expectSkeleton } from "../test/skeleton.ts";
import { expectOnePageTitle } from "../test/pageTitle.ts";
import { navTestid } from "./nav.tsx";
import CollectionRoute from "./collection.tsx";
import { questsTestid } from "./QuestsPanel.tsx";

// The whole catalog is about 480 faces, and jsdom has no layout to window the grid by, so it would draw them all in every
// test of this file: the grid's own windowing is CardGrid.test.tsx's, and what a filter keeps, collection.test.tsx's.
vi.mock("../collection/collection.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../collection/collection.ts")>();
  return { ...actual, COLLECTED_DEFS: actual.COLLECTED_DEFS.slice(0, 48) };
});

vi.mock("../net/api.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/api.ts")>();
  return {
    ...actual,
    getCollection: vi.fn(),
    getDecks: vi.fn(),
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

/** A deck of real cards the validator takes: the first `DECK_SIZE` cards the account owns, a card once (L3). */
function legalCards(): string[] {
  const { cards, version } = currentCatalog();
  const owned = new Set(COLLECTED_DEFS.map((def) => def.id));
  return poolFrom({ version, cards }, null)
    .filter((id) => owned.has(id))
    .slice(0, DECK_SIZE);
}

/** `GET /api/decks`: one deck that is legal, one that is short, and no trio. */
function decksOf() {
  const { version } = currentCatalog();
  return decksResponse(
    [savedDeck("d-legal", "Legal one", legalCards(), 1, version), savedDeck("d-short", "Short one", legalCards().slice(0, 4), 2, version)],
    [],
    version,
  );
}

function answerAll(): void {
  vi.mocked(getCollection).mockResolvedValue(fullCollection());
  vi.mocked(getDecks).mockResolvedValue(decksOf());
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

/** Opens a tab by its button: the route moves `?section=` in place. */
function openTab(section: CollectionSection): void {
  fireEvent.click(screen.getByTestId(T.tab(section)));
}

describe("#1307 the collection route", () => {
  beforeEach(() => {
    window.history.replaceState(null, "", paths.collectionSection("cards"));
  });

  it("reads the cards on Cards and the rest as each tab opens, every read with the caller's token", async () => {
    answerAll();
    render(<CollectionRoute token={TOKEN} />);

    expect(await screen.findByTestId(T.completion("Core"))).toBeInTheDocument();
    expect(getCollection).toHaveBeenCalledWith(TOKEN);
    for (const read of [getWallet, getPacks, getCosmeticPacks, getTracks, getQuests, getDecks]) expect(read).not.toHaveBeenCalled();

    openTab("progress");
    expect(await screen.findByTestId(T.jlore)).toHaveTextContent("37 Jlore");
    expect(await screen.findByTestId(T.track(newestShippedSet()))).toBeInTheDocument();
    expect(await screen.findByTestId(questsTestid.panel)).toHaveTextContent("Win 3 games");
    for (const read of [getWallet, getPacks, getCosmeticPacks, getTracks, getQuests]) expect(read).toHaveBeenCalledWith(TOKEN);
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

    await act(async () => {
      answerCards(fullCollection());
      await Promise.resolve();
    });
    expect(screen.queryByTestId(T.loading("cards"))).toBeNull();
    expect(screen.getByTestId(T.completion("Core"))).toBeInTheDocument();

    openTab("progress");
    expect(await screen.findByTestId(T.jlore)).toBeInTheDocument();
    expect(screen.queryByTestId(T.loading("wallet"))).toBeNull();
    expect(screen.queryByTestId(T.loading("tracks"))).toBeNull();
  });

  it("shows the wallet and tracks loading messages before they answer", () => {
    window.history.replaceState(null, "", paths.collectionSection("progress"));
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
    expect(screen.queryByTestId(T.loading("cards"))).toBeNull();
  });

  it("a refused read says why in its panel; the others still show", async () => {
    answerAll();
    window.history.replaceState(null, "", paths.collectionSection("progress"));
    vi.mocked(getPacks).mockRejectedValue(new Error("the packs are unavailable"));
    render(<CollectionRoute token={TOKEN} />);

    expect(await screen.findByTestId(T.error("wallet"))).toHaveTextContent("the packs are unavailable");
    expect(await screen.findByTestId(T.track(newestShippedSet()))).toBeInTheDocument();
    expect(screen.queryByTestId(T.error("tracks"))).toBeNull();
    expect(screen.queryByTestId(T.jlore)).toBeNull();

    // The cards read for themselves: the refused packs never reach them.
    openTab("cards");
    expect(await screen.findByTestId(T.completion("Core"))).toBeInTheDocument();
    expect(screen.queryByTestId(T.error("cards"))).toBeNull();
  });

  it("every read refused shows messages, not a blank screen", async () => {
    for (const read of [getCollection, getDecks, getWallet, getPacks, getCosmeticPacks, getTracks, getQuests]) {
      vi.mocked(read).mockRejectedValue(new Error("the server refused"));
    }
    render(<CollectionRoute token={TOKEN} />);

    expect(await screen.findByTestId(T.error("cards"))).toHaveTextContent("the server refused");
    openTab("decks");
    expect(await screen.findByTestId(T.error("decks"))).toHaveTextContent("the server refused");
    openTab("progress");
    for (const part of ["wallet", "tracks"] as const) {
      expect(await screen.findByTestId(T.error(part))).toHaveTextContent("the server refused");
    }
    expect(await screen.findByTestId(questsTestid.error)).toHaveTextContent("the server refused");
    expect(screen.getByRole("heading", { name: "Your collection", level: 1 })).toBeInTheDocument();
  });

  it("#1691 a panel whose read failed offers Try again, which reads it again", async () => {
    answerAll();
    window.history.replaceState(null, "", paths.collectionSection("progress"));
    vi.mocked(getPacks).mockRejectedValueOnce(new Error("the packs are unavailable"));
    render(<CollectionRoute token={TOKEN} />);

    expect(await screen.findByTestId(T.error("wallet"))).toHaveTextContent("the packs are unavailable");
    expect(screen.getByTestId(T.retry("wallet"))).toBeEnabled();
    expect(screen.queryByTestId(T.retry("tracks"))).toBeNull();

    fireEvent.click(screen.getByTestId(T.retry("wallet")));
    expect(await screen.findByTestId(T.jlore)).toHaveTextContent("37 Jlore");
    expect(screen.queryByTestId(T.error("wallet"))).toBeNull();
    expect(screen.queryByTestId(T.retry("wallet"))).toBeNull();
    expect(getPacks).toHaveBeenCalledTimes(2);
    expect(getCollection).not.toHaveBeenCalled();
  });

  it("#1691 a rate-limited panel holds Try again", async () => {
    answerAll();
    window.history.replaceState(null, "", paths.collectionSection("progress"));
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

describe("R3734000 the Collection's tabs", () => {
  beforeEach(() => {
    localStorage.clear();
    resetCosmeticsForTests();
  });

  afterEach(() => {
    localStorage.clear();
    resetCosmeticsForTests();
  });

  const READS = [getCollection, getDecks, getWallet, getPacks, getCosmeticPacks, getTracks, getQuests, getOwnedCosmetics];

  it("R3734000 opens on Cards with four tabs, and a tab's own reads only", async () => {
    answerAll();
    render(<CollectionRoute token={TOKEN} />);

    expect(screen.getByTestId(T.screen)).toHaveAttribute("data-section", "cards");
    expect(screen.getByRole("heading", { name: "Your collection" })).toBeInTheDocument();
    const tabs = screen.getAllByRole("tab").map((tab) => [tab.getAttribute("data-testid"), tab.textContent, tab.getAttribute("aria-selected")]);
    expect(tabs).toEqual([
      [T.tab("cards"), "Cards", "true"],
      [T.tab("decks"), "Decks", "false"],
      [T.tab("cosmetics"), "Cosmetics", "false"],
      [T.tab("progress"), "Progress", "false"],
    ]);
    expect(await screen.findByTestId(T.grid)).toBeInTheDocument();
    expect(getCollection).toHaveBeenCalledWith(TOKEN);
    for (const read of READS.filter((read) => read !== getCollection)) expect(read).not.toHaveBeenCalled();
  });

  it("R3734000 a tab moves ?section= in place, so Back leaves the Collection, and each opens its own panel", async () => {
    answerAll();
    window.history.replaceState(null, "", paths.landing);
    window.history.pushState(null, "", paths.collection);
    render(<CollectionRoute token={TOKEN} />);

    for (const section of ["decks", "cosmetics", "progress", "cards"] as const) {
      openTab(section);
      expect(screen.getByTestId(T.screen)).toHaveAttribute("data-section", section);
      expect(screen.getByTestId(T.tab(section))).toHaveAttribute("aria-selected", "true");
      expect(window.location.search).toBe(`?section=${section}`);
    }
    expect(await screen.findByTestId(T.grid)).toBeInTheDocument();

    fireEvent.click(screen.getByTestId(navTestid.back));
    expect(window.location.pathname).toBe(paths.landing);
  });

  it("R3734000 Progress holds the wallet, the set tracks, the Fleet track and the quests, and Cards holds none of them", async () => {
    answerAll();
    const fleet = { level: 3, xp: 1300, levelXp: 600, xpToNext: 200, xpToday: 150, nextRewards: ["starship-piece-skin-gunmetal"] };
    vi.mocked(getTracks).mockResolvedValue({ ...tracksOf(), fleet });
    render(<CollectionRoute token={TOKEN} />);
    expect(await screen.findByTestId(T.grid)).toBeInTheDocument();
    for (const absent of [T.jlore, T.fleet, questsTestid.panel, T.track(newestShippedSet())]) expect(screen.queryByTestId(absent)).toBeNull();

    openTab("progress");
    expect(await screen.findByTestId(T.jlore)).toBeInTheDocument();
    expect(await screen.findByTestId(T.track(newestShippedSet()))).toBeInTheDocument();
    expect(await screen.findByTestId(T.fleet)).toBeInTheDocument();
    expect(await screen.findByTestId(questsTestid.panel)).toHaveTextContent("Win 3 games");
    expect(screen.queryByTestId(T.grid)).toBeNull();
  });

  it("R3734000 the search, the filters and the sort survive a switch of tab, on Cards and on Cosmetics", async () => {
    answerAll();
    render(<CollectionRoute token={TOKEN} />);
    await screen.findByTestId(T.grid);

    fireEvent.change(screen.getByTestId(DB_SEARCH), { target: { value: "zzz-no-such-card" } });
    fireEvent.click(screen.getByTestId(filterOwnershipId("owned")));
    fireEvent.change(screen.getByTestId(DB_SORT), { target: { value: "name" } });
    expect(screen.getByTestId(DB_EMPTY)).toBeInTheDocument();

    openTab("cosmetics");
    fireEvent.change(await screen.findByTestId(T.cosmeticsSearch), { target: { value: "felt" } });
    fireEvent.click(screen.getByTestId(filterRarityId("Rare")));
    fireEvent.change(screen.getByTestId(T.cosmeticsSort), { target: { value: "name" } });

    openTab("progress");
    openTab("decks");
    openTab("cards");
    expect(await screen.findByTestId(DB_EMPTY)).toBeInTheDocument();
    expect(screen.getByTestId(DB_SEARCH)).toHaveValue("zzz-no-such-card");
    expect(screen.getByTestId(filterOwnershipId("owned"))).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByTestId(DB_SORT)).toHaveValue("name");

    openTab("cosmetics");
    expect(screen.getByTestId(T.cosmeticsSearch)).toHaveValue("felt");
    expect(screen.getByTestId(filterRarityId("Rare"))).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByTestId(T.cosmeticsSort)).toHaveValue("name");
  });

  it("R3734000 an unknown section opens Cards, with no read for the others", async () => {
    answerAll();
    window.history.replaceState(null, "", `${paths.collection}?section=shop`);
    render(<CollectionRoute token={TOKEN} />);

    expect(screen.getByTestId(T.screen)).toHaveAttribute("data-section", "cards");
    expect(await screen.findByTestId(T.grid)).toBeInTheDocument();
    for (const read of READS.filter((read) => read !== getCollection)) expect(read).not.toHaveBeenCalled();
  });

  it("R3734000 the browser's Back and Forward move between the tabs", async () => {
    answerAll();
    render(<CollectionRoute token={TOKEN} />);
    expect(screen.getByTestId(T.screen)).toHaveAttribute("data-section", "cards");

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
    expect(screen.getByTestId(T.screen)).toHaveAttribute("data-section", "cards");
  });

  it("R3734000 the Decks tab draws a tile for each deck with its validity and modes, and a New deck tile, each a link", async () => {
    answerAll();
    window.history.replaceState(null, "", paths.collectionSection("decks"));
    render(<CollectionRoute token={TOKEN} />);

    const legal = await screen.findByTestId(T.deckTile("d-legal"));
    expect(legal).toHaveAttribute("href", deckPath("d-legal"));
    expect(legal).toHaveTextContent("Legal one");
    expect(legal).toHaveAttribute("data-status", "ready");
    expect(legal).toHaveTextContent("Ready");
    expect(legal).toHaveTextContent(`${String(DECK_SIZE)}/${String(DECK_SIZE)}`);
    expect(legal).toHaveTextContent("Best of 1");
    expect(legal).toHaveAttribute("data-modes", "bo1");
    expect(legal.querySelector(".portrait-art")).not.toBeNull();

    const short = screen.getByTestId(T.deckTile("d-short"));
    expect(short).toHaveAttribute("data-status", "incomplete");
    expect(short).toHaveTextContent("Not playable yet");
    expect(short).toHaveAttribute("data-modes", "");

    const fresh = screen.getByTestId(T.deckNew);
    expect(fresh).toHaveAttribute("href", newDeckPath());
    expect(within(screen.getByTestId(T.decks)).getAllByRole("listitem")).toHaveLength(3);

    fireEvent.click(legal);
    expect(window.location.pathname).toBe(paths.decks);
    expect(window.location.search).toBe("?deck=d-legal");
  });

  it("R3734000 the New deck tile is no link at the deck limit", async () => {
    answerAll();
    window.history.replaceState(null, "", paths.collectionSection("decks"));
    const full = decksOf();
    vi.mocked(getDecks).mockResolvedValue({ ...full, limits: { ...full.limits, decks: full.decks.length } });
    render(<CollectionRoute token={TOKEN} />);

    const fresh = await screen.findByTestId(T.deckNew);
    expect(fresh).toBeDisabled();
    expect(fresh).not.toHaveAttribute("href");
  });

  it("R1579000 R3734000 the Cosmetics tab equips through the same store, with no request", async () => {
    answerAll();
    render(<CollectionRoute token={TOKEN} />);

    openTab("cosmetics");
    expect(await screen.findByTestId(T.tiles("table"))).toBeInTheDocument();

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
    for (const read of READS.filter((read) => read !== getCollection)) expect(read).not.toHaveBeenCalled();
  });

  it("R3734000 the Cosmetics tab's search, rarity chips and view narrow the tiles, and a row they leave empty is not drawn", async () => {
    answerAll();
    window.history.replaceState(null, "", paths.collectionSection("cosmetics"));
    render(<CollectionRoute token={TOKEN} />);
    expect(await screen.findByTestId(T.tile("table-felt"))).toBeInTheDocument();
    const before = screen.getAllByTestId(/^collection-tile-/u).length;

    fireEvent.change(screen.getByTestId(T.cosmeticsSearch), { target: { value: "felt" } });
    expect(screen.getByTestId(T.tile("table-felt"))).toBeInTheDocument();
    expect(screen.getAllByTestId(/^collection-tile-/u).length).toBeLessThan(before);
    // No card back's name holds "felt": its row is gone, and so are its words.
    expect(screen.queryByTestId(T.tiles("cardBack"))).toBeNull();

    fireEvent.change(screen.getByTestId(T.cosmeticsSearch), { target: { value: "" } });
    expect(screen.getAllByTestId(/^collection-tile-/u).length).toBe(before);
    fireEvent.click(screen.getByTestId(filterOwnershipId("missing")));
    for (const tile of screen.queryAllByTestId(/^collection-tile-/u)) expect(tile).toHaveAttribute("data-owned", "false");
  });

  it("R1953001 the Cosmetics tab has no Defaults for every card section", async () => {
    answerAll();
    // The synced settings hold a foil, a finish and a border no select changes.
    writeCosmetics({ foil: "foil-rainbow", finish: "finish-linen", border: "border-wood" });
    render(<CollectionRoute token={TOKEN} />);

    openTab("cosmetics");
    expect(await screen.findByTestId(T.tiles("table"))).toBeInTheDocument();
    expect(screen.queryByText("Defaults for every card")).toBeNull();
    expect(screen.queryByRole("heading", { name: "Defaults for every card" })).toBeNull();
    expect(screen.getByRole("heading", { name: "Equipped" })).toBeInTheDocument();
    expect(screen.getByTestId(T.tiles("table"))).toBeInTheDocument();
    // R1655000 freezes the group's shape: the picks stay stored.
    expect(readCosmetics()).toMatchObject({ foil: "foil-rainbow", finish: "finish-linen", border: "border-wood" });
  });

  it("R2731000 the Cosmetics tab shows the Equipped heading, its text and the tiles, and no Owned list", async () => {
    answerAll();
    window.history.replaceState(null, "", paths.collectionSection("cosmetics"));
    render(<CollectionRoute token={TOKEN} />);

    expect(screen.getByTestId(T.screen)).toHaveAttribute("data-section", "cosmetics");
    expect(screen.getByRole("heading", { name: "Equipped" })).toBeInTheDocument();
    expect(
      screen.getByText(
        "Your side of the board wears these in every game. A deck can pick its own playmat, backdrop and card back in the deck builder.",
      ),
    ).toBeInTheDocument();
    for (const kind of ["table", "field", "cardBack"] as const) {
      expect(await screen.findByTestId(T.tiles(kind))).toBeInTheDocument();
    }
    expect(screen.queryByRole("heading", { name: "Owned" })).toBeNull();
    expect(screen.queryByText("Loading your cosmetics…")).toBeNull();
    expect(document.querySelector(".collection-list")).toBeNull();
    expect(document.querySelector('[data-testid^="collection-owned-"]')).toBeNull();
    expect(getOwnedCosmetics).not.toHaveBeenCalled();
  });

  it("R3223000 each tab has one visible h1, the PageTitle, and the wordmark is no heading", async () => {
    answerAll();
    for (const section of COLLECTION_SECTIONS) {
      window.history.replaceState(null, "", paths.collectionSection(section));
      render(<CollectionRoute token={TOKEN} />);
      expectOnePageTitle("Your collection");
      expect(screen.queryByRole("heading", { name: "JackiOh" })).toBeNull();
      await screen.findByTestId(T.screen);
      cleanup();
    }
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

  it("R1655000 a card of the Cards tab opens large wearing its own picks, and equipping on it asks the server nothing", async () => {
    answerAll();
    writeCardPick("core-001", "border", "border-wood");
    render(<CollectionRoute token={TOKEN} />);

    fireEvent.click(await screen.findByTestId(T.card("core-001")));
    const face = screen.getByTestId(T.cardViewFace).querySelector(".cf");
    expect(face?.getAttribute("data-frame")).toBe("border-wood");

    fireEvent.click(screen.getByTestId(T.stripItem("foil-rainbow")));
    expect(readCardPicks("core-001")).toEqual({ border: "border-wood", foil: "foil-rainbow" });
    expect(face?.getAttribute("data-cosmetic-foil")).toBe("foil-rainbow");
    expect(readCosmetics().foil).toBe("foil-none");
    // Ownership is the session's (`useCanEquip`): the tab reads nothing more for it.
    expect(getOwnedCosmetics).not.toHaveBeenCalled();
  });

  it("R2127000 Back closes the card's text, then the card's view, one per Back, gives the focus back to the card and keeps ?section=cards; with none open Back leaves the tab", async () => {
    answerAll();
    window.history.replaceState(null, "", paths.collection);
    window.history.pushState(null, "", paths.collectionSection("cards"));
    render(<CollectionRoute token={TOKEN} />);

    const card = await screen.findByTestId(T.card("core-001"));
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

    // The next closes the view: the tab and the focus are as they were.
    await pressBack();
    expect(screen.queryByTestId(T.cardView)).toBeNull();
    expect(card).toHaveFocus();
    expect(window.location.pathname).toBe(paths.collection);
    expect(window.location.search).toBe("?section=cards");
    expect(screen.getByTestId(T.screen)).toHaveAttribute("data-section", "cards");

    // With none open, Back leaves the address it was pushed over, which opens Cards too.
    await pressBack();
    expect(screen.getByTestId(T.screen)).toHaveAttribute("data-section", "cards");
    expect(window.location.search).toBe("");
  });
});

describe("R2125000 the Progress tab opens on what the last visit kept", () => {
  beforeEach(() => {
    window.history.replaceState(null, "", paths.collectionSection("progress"));
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
    expect(screen.getByTestId(T.loading("tracks"))).toBeInTheDocument();
    // Every panel still reads again behind what it shows.
    for (const read of [getWallet, getPacks, getCosmeticPacks, getTracks, getQuests]) {
      expect(read).toHaveBeenCalledWith(TOKEN);
    }
  });

  it("R2125000 the Cards tab draws the collection the last visit kept at once, and reads it again", () => {
    rememberRead(TOKEN, keptPaths.collection, fullCollection());
    pendAll();
    window.history.replaceState(null, "", paths.collectionSection("cards"));
    render(<CollectionRoute token={TOKEN} />);

    expect(screen.getByTestId(T.completion("Core"))).toBeInTheDocument();
    expect(screen.queryByTestId(T.loading("cards"))).toBeNull();
    expect(getCollection).toHaveBeenCalledWith(TOKEN);
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

describe("R3335001 R3335000 the Fleet track and the Starship tab", () => {
  afterEach(() => {
    localStorage.clear();
    resetCosmeticsForTests();
  });

  it("R3335001 the Progress tab shows the Fleet track right after the set tracks, only when the tracks answer has it", async () => {
    answerAll();
    const fleet = { level: 3, xp: 1300, levelXp: 600, xpToNext: 200, xpToday: 150, nextRewards: ["starship-piece-skin-gunmetal"] };
    vi.mocked(getTracks).mockResolvedValue({ ...tracksOf(), fleet });
    render(<CollectionRoute token={TOKEN} />);
    openTab("progress");

    const panel = await screen.findByTestId(T.fleet);
    expect(within(panel).getByTestId(T.fleetLevel)).toHaveTextContent("Level 3 of");
    expect(within(panel).getByTestId(T.fleetReward("starship-piece-skin-gunmetal"))).toHaveTextContent("Gunmetal modules");
    const headings = screen.getAllByRole("heading", { level: 2 }).map((heading) => heading.textContent);
    expect(headings.indexOf("Fleet track")).toBe(headings.indexOf("Set tracks") + 1);
    cleanup();

    // A server from before #3335 answers no Fleet track: no panel, no heading, nothing loading.
    vi.mocked(getTracks).mockResolvedValue(tracksOf());
    window.history.replaceState(null, "", paths.collectionSection("progress"));
    render(<CollectionRoute token={TOKEN} />);
    expect(await screen.findByTestId(T.track(newestShippedSet()))).toBeInTheDocument();
    expect(screen.queryByTestId(T.fleet)).toBeNull();
    expect(screen.queryByRole("heading", { name: "Fleet track" })).toBeNull();
    expect(screen.queryByTestId(T.loading("fleet"))).toBeNull();
  });

  it("R3335000 the Cosmetics tab's Starship tab equips a Starship look through the same store, with no request", async () => {
    answerAll();
    render(<CollectionRoute token={TOKEN} />);
    openTab("cosmetics");
    expect(await screen.findByTestId(T.tiles("table"))).toBeInTheDocument();

    // The Board tab is open first; the Starship tab holds the Starship looks alone.
    expect(screen.getByTestId(T.cosmeticsTab("board"))).toHaveAttribute("aria-selected", "true");
    expect(screen.queryByTestId(T.starshipLooks)).toBeNull();
    fireEvent.click(screen.getByTestId(T.cosmeticsTab("starship")));
    expect(screen.getByTestId(T.cosmeticsTab("starship"))).toHaveAttribute("aria-selected", "true");
    expect(screen.getByTestId(T.starshipLooks)).toBeInTheDocument();
    expect(screen.queryByTestId(T.tiles("table"))).toBeNull();

    fireEvent.click(within(screen.getByTestId(T.tile("starship-trail-ion"))).getByRole("button"));
    expect(readCosmetics().starshipTrail).toBe("starship-trail-ion");
    expect(SETTINGS_GROUPS.find((candidate) => candidate.id === "cosmetics")?.read()["starshipTrail"]).toBe("starship-trail-ion");
    for (const read of [getWallet, getPacks, getCosmeticPacks, getTracks, getQuests, getOwnedCosmetics]) expect(read).not.toHaveBeenCalled();
  });
});
