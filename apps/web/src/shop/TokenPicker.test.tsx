// The token picker (#1309, #2251), from fixture responses: both bars and the cycle's next step,
// the tokens held and the link to the Collection where each is spent (R2730000), and the page's
// next revision reading the bars again. R1953001: no token is picked here any more
// (collection/TokenOffer.test.tsx holds the offers); the held counts stay.

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { resetLocaleForTests, setLocale } from "../i18n/i18n.ts";
import type { TokenBarsResponse } from "../net/api.ts";
import { paths } from "../net/navigate.ts";
import { skeletonTestid } from "../skeleton/Skeleton.tsx";
import { expectSkeleton } from "../test/skeleton.ts";
import { refusal, stubServer, type Reply } from "../test/stubServer.ts";
import { collectionOf, tokenBars } from "./fixtures.ts";
import { fittingCards, quantitiesOf, shopTestid as T, type RegistryCosmetic } from "./shop.ts";
import TokenPicker from "./TokenPicker.tsx";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  resetLocaleForTests();
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
  it("a full art fits only its named card, owned", () => {
    const fullArt = entry({ id: "fa", kind: "fullArt", rarity: "Epic", card: "core-001" });
    expect(fittingCards(fullArt, quantitiesOf(collectionOf()))).toEqual(["core-001"]);
    expect(fittingCards(fullArt, quantitiesOf(collectionOf({ entries: [{ cardId: "core-002", quantity: 1 }] })))).toEqual(
      [],
    );
  });
});

/** The picker's reads, answered from the fixtures unless a test says otherwise. */
function tokenServer(answers: { bars?: Reply } = {}): ReturnType<typeof stubServer> {
  return stubServer((call) => {
    if (call.method === "GET" && call.path === "/api/tokens") {
      return answers.bars ?? { status: 200, body: tokenBars() };
    }
    return undefined;
  });
}

describe("#1309 the token picker", () => {
  it("R1953001 R2730000 the shop shows the bars, the next step and the held counts, lists no card, offers no Unlock and links to the Collection", async () => {
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
    expect(screen.getByTestId(T.nextStep)).toHaveTextContent("Epic token");
    expect(screen.getByTestId(T.heldCard)).toHaveTextContent("Card tokens held: 1");
    // The held cosmetic tokens still show, and a link says where every token is spent.
    expect(screen.getByTestId(T.heldRare)).toHaveTextContent("Rare tokens held: 2");
    expect(screen.getByTestId(T.heldEpic)).toHaveTextContent("Epic tokens held: 1");
    expect(screen.getByTestId(T.cosmeticSpend)).toHaveAttribute("href", paths.collection);
    expect(screen.getByTestId(T.cosmeticSpend)).toHaveTextContent("Hold a locked card or cosmetic in your Collection to spend a token");
    // No card is listed and no Unlock offered: the Collection's previews spend the tokens.
    expect(document.querySelector('[data-testid^="card-pool-"]')).toBeNull();
    expect(document.querySelector('[data-testid^="token-card-pick-"]')).toBeNull();
    expect(screen.queryByRole("button", { name: "Unlock" })).toBeNull();
    expect(server.to("GET", "/api/collection"), "the shop reads no collection").toHaveLength(0);
  });

  it("with no card token the shop says how one is paid", async () => {
    const bars = tokenBars();
    tokenServer({
      bars: { status: 200, body: { ...bars, tokens: { ...bars.tokens, card: 0 } } satisfies TokenBarsResponse },
    });
    render(<TokenPicker token={TOKEN} />);

    expect(await screen.findByTestId(T.heldCard)).toHaveTextContent("Card tokens held: 0");
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

  it("#2053 a failed read says why in the reader's language", async () => {
    await setLocale("zh-CN");
    tokenServer({ bars: { status: 500 } });
    render(<TokenPicker token={TOKEN} />);
    expect(await screen.findByTestId(T.tokenLoadError)).toHaveTextContent("服务器返回了 500");
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

  it("the page's next revision reads the bars again", async () => {
    // The bars move at the server after the first read (another panel opened a pack).
    let bars = tokenBars();
    const server = stubServer((call) => {
      if (call.method === "GET" && call.path === "/api/tokens") return { status: 200, body: bars };
      return undefined;
    });
    const { rerender } = render(<TokenPicker token={TOKEN} />);
    expect(await screen.findByTestId(T.cardBar)).toHaveTextContent("149 / 150");

    bars = { ...bars, cardBar: { ...bars.cardBar, points: 3 } };
    rerender(<TokenPicker token={TOKEN} revision={1} />);

    await waitFor(() => {
      expect(screen.getByTestId(T.cardBar)).toHaveTextContent("3 / 150");
    });
    expect(server.to("GET", "/api/tokens")).toHaveLength(2);
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
