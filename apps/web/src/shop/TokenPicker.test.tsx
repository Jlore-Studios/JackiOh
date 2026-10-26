// The token picker (#1309), from fixture responses: the card picker listing only unowned cards,
// both bars and the cycle's next step, a redemption telling the page while a refused one changes
// nothing, and the page's next revision reading the bars again. R1953001: no cosmetic token is
// picked here any more (collection/TokenOffer.test.tsx holds its tier rules); the held counts stay.

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";

import type { CardDef } from "@jackioh/shared";

import type { TokenBarsResponse } from "../net/api.ts";
import { paths } from "../net/navigate.ts";
import { refusal, stubServer, type Reply } from "../test/stubServer.ts";
import { collectionOf, tokenBars } from "./fixtures.ts";
import { fittingCards, quantitiesOf, shopTestid as T, unownedCards, type RegistryCosmetic } from "./shop.ts";
import TokenPicker from "./TokenPicker.tsx";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const TOKEN = "token-1";

/** A crafted registry entry. */
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

  it("a full art fits only its named card, owned", () => {
    const fullArt = entry({ id: "fa", kind: "fullArt", rarity: "Epic", card: "core-001" });
    expect(fittingCards(fullArt, quantitiesOf(collectionOf()))).toEqual(["core-001"]);
    expect(fittingCards(fullArt, quantitiesOf(collectionOf({ entries: [{ cardId: "core-002", quantity: 1 }] })))).toEqual(
      [],
    );
  });
});

/** The picker's reads, answered from the fixtures unless a test says otherwise. */
function tokenServer(answers: { bars?: Reply; redeem?: Reply } = {}): ReturnType<typeof stubServer> {
  return stubServer((call) => {
    if (call.method === "GET" && call.path === "/api/tokens") {
      return answers.bars ?? { status: 200, body: tokenBars() };
    }
    if (call.method === "GET" && call.path === "/api/collection") return { status: 200, body: collectionOf() };
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
  it("the card picker lists only unowned cards", async () => {
    tokenServer();
    render(<TokenPicker token={TOKEN} />);

    expect(await screen.findByTestId(T.cardBar)).toHaveTextContent("149 / 150");
    expect(screen.getByTestId(T.cosmeticBar)).toHaveTextContent("30 / 150");
    expect(screen.getByTestId(T.nextStep)).toHaveTextContent("Epic token");
    expect(screen.getByTestId(T.heldCard)).toHaveTextContent("1");

    // core-002 is unowned; core-001 and core-004 are owned at MAX_COPIES; core-051-1 is a token.
    expect(screen.getByTestId(T.cardPick("core-002"))).toBeInTheDocument();
    expect(screen.queryByTestId(T.cardPick("core-001"))).toBeNull();
    expect(screen.queryByTestId(T.cardPick("core-004"))).toBeNull();
    expect(screen.queryByTestId(T.cardPick("core-051-1"))).toBeNull();
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
    // Drawn at the points with no animation, and the cycle's next step beside the cosmetic bar.
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
    await screen.findByTestId(T.cardPick("core-002"));

    fireEvent.click(screen.getByTestId(T.cardPick("core-002")));

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
    expect(screen.getByTestId(T.cardPick("core-002"))).toBeInTheDocument();
    // The held cosmetic tokens still show, and a link says where they are spent.
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
    await screen.findByTestId(T.cardPick("core-002"));

    fireEvent.click(screen.getByTestId(T.cardPick("core-002")));

    expect(await screen.findByTestId(T.tokenError)).toHaveTextContent("that card is already owned");
    expect(screen.getByTestId(T.cardPick("core-002"))).toBeInTheDocument();
    expect(screen.getByTestId(T.heldCard)).toHaveTextContent("1");
    expect(server.to("POST", "/api/tokens/redeem-card")).toHaveLength(1);
    expect(server.to("POST", "/api/tokens/redeem-card")[0]?.body).toEqual({ cardId: "core-002" });
    expect(onChanged, "a refusal changed nothing to tell the page").not.toHaveBeenCalled();
  });

  it("a redeemed card token moves the held count from the answer and tells the page", async () => {
    const server = tokenServer();
    const onChanged = vi.fn<() => void>();
    render(<TokenPicker token={TOKEN} onChanged={onChanged} />);
    await screen.findByTestId(T.cardPick("core-002"));

    fireEvent.click(screen.getByTestId(T.cardPick("core-002")));

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
      if (call.method === "GET" && call.path === "/api/collection") return { status: 200, body: collectionOf() };
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
    // The read is out: the button waits for it, and a second press sends nothing.
    expect(screen.getByTestId(T.tokenRetry)).toBeDisabled();
    fireEvent.click(screen.getByTestId(T.tokenRetry));

    expect(await screen.findByTestId(T.cardBar)).toHaveTextContent("149 / 150");
    expect(screen.queryByTestId(T.tokenLoadError)).toBeNull();
    expect(server.to("GET", "/api/tokens")).toHaveLength(2);
  });
});
