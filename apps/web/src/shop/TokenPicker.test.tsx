// The token picker (#1309, #2251), from fixture responses: the card-token pick as the deck builder's
// browse pane over exactly the unowned cards, with Unlock and a confirm before `redeemCardToken`.
// R1953001: no cosmetic token is picked here (collection/TokenOffer.test.tsx holds its tier rules).

import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";

import type { CardDef } from "@jackioh/shared";

import { CHINESE } from "../cards/chinese.ts";
import { INSPECT_CLOSE, INSPECT_DETAIL } from "../cards/index.ts";
import { DEFAULT_FILTER, DEFAULT_SORT } from "../game/deckbuilder/filters.ts";
import { DB_SEARCH, poolCardId } from "../game/deckbuilder/testids.ts";
import { resetLocaleForTests, setLocale } from "../i18n/i18n.ts";
import type { CollectionResponse, TokenBarsResponse } from "../net/api.ts";
import { paths } from "../net/navigate.ts";
import { skeletonTestid } from "../skeleton/Skeleton.tsx";
import { expectSkeleton } from "../test/skeleton.ts";
import { refusal, stubServer, type Reply } from "../test/stubServer.ts";
import { collectionOf, collectionOwningAllBut, tokenBars } from "./fixtures.ts";
import {
  fittingCards,
  quantitiesOf,
  shopTestid as T,
  tokenPoolIds,
  unownedCards,
  type RegistryCosmetic,
} from "./shop.ts";
import TokenPicker from "./TokenPicker.tsx";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  resetLocaleForTests();
});

const TOKEN = "token-1";

function entry(over: Partial<RegistryCosmetic> & { id: string }): RegistryCosmetic {
  return {
    kind: "foil",
    name: over.id,
    rarity: "Common",
    default: false,
    starter: false,
    chase: false,
    ...over,
  };
}

describe("#1309 the picker model", () => {
  it("the card picker lists only unowned, non-token cards of sets that ship", () => {
    const defs: CardDef[] = [
      { ...CATALOG["core-002"]!, token: false },
      { ...CATALOG["core-002"]!, id: "boss-001", set: "Boss", token: false },
      { ...CATALOG["core-002"]!, id: "tok-001", token: true },
    ];
    // core-001 is owned at MAX_COPIES by the fixture; core-002 is not.
    const ids = unownedCards(collectionOf(), defs).map((def) => def.id);
    expect(ids).toEqual(["core-002"]);
  });

  it("tokenPoolIds sorts as the deck builder sorts", () => {
    const cheap: CardDef = { ...CATALOG["core-002"]!, id: "cheap", cost: 2 };
    const dear: CardDef = { ...CATALOG["core-002"]!, id: "dear", cost: 5 };
    const catalog = { version: "test", cards: { cheap, dear } };
    expect(tokenPoolIds([dear, cheap], catalog, { ...DEFAULT_FILTER, ownedOnly: false }, DEFAULT_SORT)).toEqual([
      "cheap",
      "dear",
    ]);
  });

  it("a full art fits only its named card, owned", () => {
    const fullArt = entry({ id: "fa", kind: "fullArt", rarity: "Epic", card: "core-001" });
    expect(fittingCards(fullArt, quantitiesOf(collectionOf()))).toEqual(["core-001"]);
    expect(fittingCards(fullArt, quantitiesOf(collectionOf({ entries: [{ cardId: "core-002", quantity: 1 }] })))).toEqual(
      [],
    );
  });
});

/** The collection the stubbed server reads: everything owned but `core-002` and `core-003`. */
function smallCollection(): CollectionResponse {
  return collectionOwningAllBut("core-002", "core-003");
}

/** The picker's reads, answered from the fixtures unless a test says otherwise. */
function tokenServer(answers: { bars?: Reply; collection?: Reply; redeem?: Reply } = {}): ReturnType<typeof stubServer> {
  return stubServer((call) => {
    if (call.method === "GET" && call.path === "/api/tokens") {
      return answers.bars ?? { status: 200, body: tokenBars() };
    }
    if (call.method === "GET" && call.path === "/api/collection") {
      return answers.collection ?? { status: 200, body: smallCollection() };
    }
    if (call.method === "POST" && call.path === "/api/tokens/redeem-card") {
      return (
        answers.redeem ?? {
          status: 200,
          body: { ...tokenBars(), tokens: { ...tokenBars().tokens, card: 0 } } satisfies TokenBarsResponse,
        }
      );
    }
    return undefined;
  });
}

describe("#1309 the token picker", () => {
  it("the token pool shows faces for exactly unownedCards and search narrows it", async () => {
    tokenServer();
    render(<TokenPicker token={TOKEN} />);

    expect(await screen.findByTestId(T.cardBar)).toHaveTextContent("149 / 150");
    expect(screen.getByTestId(T.cosmeticBar)).toHaveTextContent("30 / 150");
    expect(screen.getByTestId(T.nextStep)).toHaveTextContent("Epic token");
    expect(screen.getByTestId(T.heldCard)).toHaveTextContent("1");

    // core-002 and core-003 are the stubbed collection's only unowned cards: core-001 and
    // core-004 are owned at MAX_COPIES, core-051-1 is a token, and so is everything else.
    expect(screen.getByTestId(poolCardId("core-002"))).toBeInTheDocument();
    expect(screen.getByTestId(poolCardId("core-003"))).toBeInTheDocument();
    expect(screen.queryByTestId(poolCardId("core-001"))).toBeNull();
    expect(screen.queryByTestId(poolCardId("core-004"))).toBeNull();
    expect(screen.queryByTestId(poolCardId("core-051-1"))).toBeNull();
    expect(document.querySelectorAll('[data-testid^="card-pool-"]')).toHaveLength(unownedCards(smallCollection()).length);

    // "Bigot" is core-002's whole name; core-003's face does not match it.
    fireEvent.change(screen.getByTestId(DB_SEARCH), { target: { value: CATALOG["core-002"]!.name } });
    expect(screen.getByTestId(poolCardId("core-002"))).toBeInTheDocument();
    expect(screen.queryByTestId(poolCardId("core-003"))).toBeNull();
    expect(document.querySelectorAll('[data-testid^="card-pool-"]')).toHaveLength(1);
  });

  it("opening a card shows its detail and Cancel sends nothing", async () => {
    const server = tokenServer();
    render(<TokenPicker token={TOKEN} />);

    fireEvent.click(await screen.findByTestId(poolCardId("core-002")));
    expect(await screen.findByTestId(INSPECT_DETAIL)).toBeInTheDocument();

    // Unlock carries the `cardPick` test id, inside the open detail.
    fireEvent.click(screen.getByTestId(T.cardPick("core-002")));
    const dialog = await screen.findByTestId(T.cardDialog);
    expect(dialog).toHaveTextContent(CATALOG["core-002"]!.name);
    expect(screen.getByRole("alertdialog", { name: `Unlock ${CATALOG["core-002"]!.name}? This spends 1 card token.` })).toBe(
      dialog,
    );
    // The question is in the detail's own actions row, in place of Unlock: the detail is a modal
    // over the page, so a confirm anywhere else would lie under its scrim and out of Tab's reach.
    expect(screen.getByTestId(INSPECT_DETAIL)).toContainElement(dialog);
    expect(screen.queryByTestId(T.cardPick("core-002"))).toBeNull();
    expect(screen.getByTestId(T.cardCancel)).toHaveFocus();

    fireEvent.click(screen.getByTestId(T.cardCancel));
    expect(screen.queryByTestId(T.cardDialog)).toBeNull();
    expect(server.to("POST", "/api/tokens/redeem-card")).toHaveLength(0);
    expect(screen.getByTestId(INSPECT_DETAIL)).toBeInTheDocument();
    expect(screen.getByTestId(T.cardPick("core-002"))).toBeEnabled();
    expect(screen.getByTestId(poolCardId("core-002"))).toBeInTheDocument();
    expect(screen.getByTestId(T.heldCard)).toHaveTextContent("1");
  });

  it("closing the detail while it asks drops the question and sends nothing", async () => {
    const server = tokenServer();
    render(<TokenPicker token={TOKEN} />);

    fireEvent.click(await screen.findByTestId(poolCardId("core-002")));
    fireEvent.click(await screen.findByTestId(T.cardPick("core-002")));
    await screen.findByTestId(T.cardDialog);
    fireEvent.click(screen.getByTestId(INSPECT_CLOSE));
    expect(screen.queryByTestId(INSPECT_DETAIL)).toBeNull();

    fireEvent.click(screen.getByTestId(poolCardId("core-002")));
    expect(await screen.findByTestId(T.cardPick("core-002"))).toBeEnabled();
    expect(screen.queryByTestId(T.cardDialog)).toBeNull();
    expect(server.to("POST", "/api/tokens/redeem-card")).toHaveLength(0);
  });

  it("Confirm sends one redeem-card and the card leaves the pool", async () => {
    let collectionReads = 0;
    const server = stubServer((call) => {
      if (call.method === "GET" && call.path === "/api/tokens") return { status: 200, body: tokenBars() };
      if (call.method === "GET" && call.path === "/api/collection") {
        collectionReads += 1;
        const owned = smallCollection();
        // The redemption's re-read finds the unlocked card owned, so it leaves the pool.
        return collectionReads === 1
          ? { status: 200, body: owned }
          : { status: 200, body: { ...owned, entries: [...owned.entries, { cardId: "core-002", quantity: 1 }] } };
      }
      if (call.method === "POST" && call.path === "/api/tokens/redeem-card") {
        return {
          status: 200,
          body: { ...tokenBars(), tokens: { ...tokenBars().tokens, card: 0 } } satisfies TokenBarsResponse,
        };
      }
      return undefined;
    });
    const onChanged = vi.fn<() => void>();
    render(<TokenPicker token={TOKEN} onChanged={onChanged} />);

    fireEvent.click(await screen.findByTestId(poolCardId("core-002")));
    fireEvent.click(await screen.findByTestId(T.cardPick("core-002")));
    fireEvent.click(await screen.findByTestId(T.cardConfirm));

    await waitFor(() => {
      expect(screen.getByTestId(T.heldCard)).toHaveTextContent("Card tokens held: 0");
    });
    expect(screen.queryByTestId(poolCardId("core-002"))).toBeNull();
    expect(screen.queryByTestId(T.cardDialog)).toBeNull();
    expect(server.to("POST", "/api/tokens/redeem-card")).toHaveLength(1);
    expect(server.to("POST", "/api/tokens/redeem-card")[0]?.body).toEqual({ cardId: "core-002" });
    expect(onChanged).toHaveBeenCalledTimes(1);
  });

  it("with no token the pool still browses and Unlock is disabled", async () => {
    const bars = tokenBars();
    tokenServer({
      bars: { status: 200, body: { ...bars, tokens: { ...bars.tokens, card: 0 } } satisfies TokenBarsResponse },
    });
    render(<TokenPicker token={TOKEN} />);

    // The pool still browses, so a player can plan: the face opens its detail.
    fireEvent.click(await screen.findByTestId(poolCardId("core-002")));
    await screen.findByTestId(INSPECT_DETAIL);
    expect(screen.getByTestId(T.cardPick("core-002"))).toBeDisabled();
    expect(screen.getByTestId(T.cardHint)).toHaveTextContent("A full card token bar pays a card token.");
  });

  it("#2115 the picker waits as a panel skeleton, and the bars take its place when the reads answer", async () => {
    tokenServer();
    render(<TokenPicker token={TOKEN} />);

    expectSkeleton(screen.getByTestId(T.tokenLoading), "panel");
    expect(screen.getByTestId(T.tokenLoading)).toHaveTextContent("Loading your tokens…");
    expect(screen.queryByTestId(T.cardBar)).toBeNull();

    expect(await screen.findByTestId(T.cardBar)).toHaveTextContent("149 / 150");
    expect(screen.queryByTestId(T.tokenLoading)).toBeNull();
    expect(screen.queryByText("Loading your tokens…")).toBeNull();
    expect(screen.queryByTestId(skeletonTestid.root)).toBeNull();
  });

  it("#2053 the card picker names each card in the reader's language", async () => {
    tokenServer();
    const { unmount } = render(<TokenPicker token={TOKEN} />);
    fireEvent.click(await screen.findByTestId(poolCardId("core-002")));
    const english = await screen.findByTestId(INSPECT_DETAIL);
    expect(english).toHaveTextContent(CATALOG["core-002"]!.name);
    unmount();

    await setLocale("zh-CN");
    render(<TokenPicker token={TOKEN} />);
    fireEvent.click(await screen.findByTestId(poolCardId("core-002")));
    const detail = await screen.findByTestId(INSPECT_DETAIL);
    expect(detail).toHaveTextContent(CHINESE["core-002"]!.name);
    expect(detail).not.toHaveTextContent(CATALOG["core-002"]!.name);
  });

  it("R711000 the card search reads the Chinese name live: a switch to Chinese and back re-filters the pool", async () => {
    tokenServer();
    render(<TokenPicker token={TOKEN} />);
    await screen.findByTestId(T.cardBar);
    fireEvent.change(screen.getByTestId(DB_SEARCH), { target: { value: CHINESE["core-002"]!.name } });
    expect(screen.queryByTestId(poolCardId("core-002"))).toBeNull();
    await act(async () => {
      await setLocale("zh-CN");
    });
    expect(screen.getByTestId(poolCardId("core-002"))).toBeInTheDocument();
    expect(screen.queryByTestId(poolCardId("core-003"))).toBeNull();
    await act(async () => {
      await setLocale("en");
    });
    expect(screen.queryByTestId(poolCardId("core-002"))).toBeNull();
  });

  it("#2053 a failed read says why in the reader's language", async () => {
    await setLocale("zh-CN");
    tokenServer({ bars: { status: 500 } });
    render(<TokenPicker token={TOKEN} />);
    expect(await screen.findByTestId(T.tokenLoadError)).toHaveTextContent("服务器返回了 500");
  });

  it("#2053 a refused redemption says why in the reader's language", async () => {
    await setLocale("zh-CN");
    tokenServer({ redeem: { status: 500 } });
    render(<TokenPicker token={TOKEN} />);
    fireEvent.click(await screen.findByTestId(poolCardId("core-002")));
    fireEvent.click(await screen.findByTestId(T.cardPick("core-002")));
    fireEvent.click(await screen.findByTestId(T.cardConfirm));

    expect(await screen.findByTestId(T.tokenError)).toHaveTextContent("服务器返回了 500");
  });

  it("#1658 both bars are FillBars at the server's points and size, the next step beside the cosmetic bar", async () => {
    tokenServer();
    render(<TokenPicker token={TOKEN} />);

    const card = await screen.findByTestId(T.cardBar);
    const cosmetic = screen.getByTestId(T.cosmeticBar);
    expect(card).toHaveClass("fill-bar");
    expect(card).toHaveTextContent("Card token bar");
    expect(card.querySelector("progress")).toHaveProperty("value", 149);
    expect(card.querySelector("progress")).toHaveProperty("max", 150);
    expect(cosmetic).toHaveClass("fill-bar");
    expect(cosmetic).toHaveTextContent("Cosmetic token bar");
    expect(cosmetic.querySelector("progress")).toHaveProperty("value", 30);
    expect(cosmetic.querySelector("progress")).toHaveProperty("max", 150);
    expect(card).toHaveAttribute("data-state", "resting");
    expect(cosmetic).toHaveAttribute("data-state", "resting");
    expect(cosmetic.querySelector<HTMLElement>(".fill-bar__fill")?.style.transform).toBe("scaleX(0.2)");
    expect(cosmetic.parentElement).toHaveClass("fill-bar-row");
    expect(cosmetic.parentElement).toContainElement(screen.getByTestId(T.nextStep));
    expect(cosmetic.parentElement).not.toContainElement(card);
  });

  it("#1658 a redemption's answer moves the bars to its points without playing them", async () => {
    const bars = tokenBars();
    tokenServer({
      redeem: {
        status: 200,
        body: { ...bars, cardBar: { points: 4, size: 150 }, tokens: { ...bars.tokens, card: 0 } } satisfies TokenBarsResponse,
      },
    });
    render(<TokenPicker token={TOKEN} />);
    fireEvent.click(await screen.findByTestId(poolCardId("core-002")));
    fireEvent.click(await screen.findByTestId(T.cardPick("core-002")));
    fireEvent.click(await screen.findByTestId(T.cardConfirm));

    await waitFor(() => {
      expect(screen.getByTestId(T.cardBar)).toHaveTextContent("4 / 150");
    });
    expect(screen.getByTestId(T.cardBar).querySelector("progress")).toHaveProperty("value", 4);
    expect(screen.getByTestId(T.cardBar)).toHaveAttribute("data-state", "resting");
  });

  it("R1953001 the shop keeps the card picker and both bars but offers no cosmetic token", async () => {
    const bars = tokenBars();
    const server = tokenServer({
      bars: {
        status: 200,
        body: { ...bars, tokens: { ...bars.tokens, cosmeticRare: 2, cosmeticEpic: 1 } } satisfies TokenBarsResponse,
      },
    });
    render(<TokenPicker token={TOKEN} />);

    expect(await screen.findByTestId(T.cardBar)).toHaveTextContent("149 / 150");
    expect(screen.getByTestId(T.cosmeticBar)).toHaveTextContent("30 / 150");
    // Unlock carries the `cardPick` test id, inside the open detail.
    fireEvent.click(await screen.findByTestId(poolCardId("core-002")));
    expect(await screen.findByTestId(T.cardPick("core-002"))).toBeInTheDocument();
    expect(screen.getByTestId(T.heldRare)).toHaveTextContent("Rare tokens held: 2");
    expect(screen.getByTestId(T.heldEpic)).toHaveTextContent("Epic tokens held: 1");
    expect(screen.getByTestId(T.cosmeticSpend)).toHaveAttribute("href", paths.collection);
    // No tier to choose and no cosmetic to pick: the Collection offers them in place.
    expect(
      document.querySelector('[data-testid^="token-cosmetic-pick-"], [data-testid^="token-cosmetic-preview-"], [data-testid^="token-tier-"]'),
    ).toBeNull();
    expect(screen.queryByRole("group", { name: "Cosmetic tokens" })).toBeNull();
    expect(server.to("GET", "/api/cosmetics"), "the shop reads no owned cosmetics").toHaveLength(0);
  });

  it("a refused redemption shows its reason and changes nothing on screen", async () => {
    const server = tokenServer({ redeem: refusal(409, "conflict", "that card is already owned") });
    const onChanged = vi.fn<() => void>();
    render(<TokenPicker token={TOKEN} onChanged={onChanged} />);
    fireEvent.click(await screen.findByTestId(poolCardId("core-002")));
    fireEvent.click(await screen.findByTestId(T.cardPick("core-002")));
    fireEvent.click(await screen.findByTestId(T.cardConfirm));

    expect(await screen.findByTestId(T.tokenError)).toHaveTextContent("that card is already owned");
    expect(screen.queryByTestId(T.cardDialog)).toBeNull();
    expect(screen.getByTestId(poolCardId("core-002"))).toBeInTheDocument();
    expect(screen.getByTestId(T.heldCard)).toHaveTextContent("1");
    expect(server.to("POST", "/api/tokens/redeem-card")).toHaveLength(1);
    expect(server.to("POST", "/api/tokens/redeem-card")[0]?.body).toEqual({ cardId: "core-002" });
    expect(onChanged, "a refusal changed nothing to tell the page").not.toHaveBeenCalled();
  });

  it("a redeemed card token moves the held count from the answer and tells the page", async () => {
    const server = tokenServer();
    const onChanged = vi.fn<() => void>();
    render(<TokenPicker token={TOKEN} onChanged={onChanged} />);
    fireEvent.click(await screen.findByTestId(poolCardId("core-002")));
    fireEvent.click(await screen.findByTestId(T.cardPick("core-002")));
    fireEvent.click(await screen.findByTestId(T.cardConfirm));

    await waitFor(() => {
      expect(screen.getByTestId(T.heldCard)).toHaveTextContent("Card tokens held: 0");
    });
    expect(onChanged).toHaveBeenCalledTimes(1);
    expect(server.to("GET", "/api/collection"), "the collection is read back").toHaveLength(2);
  });

  it("the page's next revision reads the bars again", async () => {
    // The bars move at the server after the first read (another panel opened a pack).
    let bars = tokenBars();
    const server = stubServer((call) => {
      if (call.method === "GET" && call.path === "/api/tokens") return { status: 200, body: bars };
      if (call.method === "GET" && call.path === "/api/collection") return { status: 200, body: smallCollection() };
      return undefined;
    });
    const { rerender } = render(<TokenPicker token={TOKEN} />);
    expect(await screen.findByTestId(T.cardBar)).toHaveTextContent("149 / 150");

    bars = { ...bars, cardBar: { ...bars.cardBar, points: 3 } };
    rerender(<TokenPicker token={TOKEN} revision={1} />);

    await waitFor(() => {
      expect(screen.getByTestId(T.cardBar)).toHaveTextContent("3 / 150");
    });
    expect(server.to("GET", "/api/collection")).toHaveLength(2);
  });

  it("#1485 a failed read that works on the page's next revision shows the shop and drops the alert", async () => {
    const answers: { bars?: Reply } = { bars: refusal(503, "unavailable", "the token bars are not reachable") };
    tokenServer(answers);
    const { rerender } = render(<TokenPicker token={TOKEN} />);
    expect(await screen.findByTestId(T.tokenLoadError)).toHaveTextContent("the token bars are not reachable");

    answers.bars = { status: 200, body: tokenBars() };
    rerender(<TokenPicker token={TOKEN} revision={1} />);

    expect(await screen.findByTestId(T.cardBar)).toHaveTextContent("149 / 150");
    expect(screen.queryByTestId(T.tokenLoadError)).toBeNull();
    expect(screen.queryByTestId(T.tokenRetry)).toBeNull();
  });

  it("#1485 Try again reads once more, cannot be pressed twice while the read is out, and drops the alert once it works", async () => {
    const answers: { bars?: Reply } = { bars: refusal(503, "unavailable", "the token bars are not reachable") };
    const server = tokenServer(answers);
    render(<TokenPicker token={TOKEN} />);
    await waitFor(() => {
      expect(screen.getByTestId(T.tokenRetry)).toBeEnabled();
    });
    expect(server.to("GET", "/api/tokens")).toHaveLength(1);

    answers.bars = { status: 200, body: tokenBars() };
    fireEvent.click(screen.getByTestId(T.tokenRetry));
    expect(screen.getByTestId(T.tokenRetry)).toBeDisabled();
    fireEvent.click(screen.getByTestId(T.tokenRetry));

    expect(await screen.findByTestId(T.cardBar)).toHaveTextContent("149 / 150");
    expect(screen.queryByTestId(T.tokenLoadError)).toBeNull();
    expect(server.to("GET", "/api/tokens")).toHaveLength(2);
  });
});
