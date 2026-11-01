// The Jlore shop and the cosmetic packs (#1309, #1667), from fixture responses: a spent slot's points fly
// into both bars (R1953000), and Buy waits with a wheel while another spend of the tab is out (R2378000).
// The tier filters are pure tests over crafted registries: the shipped registry holds no Legendary or Mythic a
// token could reach.

import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { GLOBAL_PACK_NAME } from "@jackioh/server-config";

import { CHINESE } from "../cards/chinese.ts";
import { DEFAULT_CARD_COSMETICS, readCardPicks, readCosmetics, resetCosmeticsForTests, writeCosmetics } from "../cards/cosmetics.ts";
import { REVEAL_STAGGER_MS, flightsMs, segmentMs } from "../economy/bars.ts";
import { announceWallet } from "../economy/wallet.ts";
import { resetFxSettingsForTests } from "../fx/settings.ts";
import { resetLocaleForTests, setLocale } from "../i18n/i18n.ts";
import type { BoughtPackResponse, OpenedCosmeticPackResponse } from "../net/api.ts";
import { clearReadCache } from "../net/readCache.ts";
import { whileSpending } from "../net/spending.ts";
import { packsInfo } from "../packs/fixtures.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { skeletonTestid } from "../skeleton/Skeleton.tsx";
import { expectEmptyState } from "../test/empty.ts";
import { expectSkeleton } from "../test/skeleton.ts";
import { refusal, stubServer, type Reply } from "../test/stubServer.ts";
import CosmeticPacks from "./CosmeticPacks.tsx";
import Shop from "./Shop.tsx";
import {
  collectionOf,
  cosmeticPacksFull,
  cosmeticPacksInfo,
  openedCosmeticPack,
  ownedCosmeticsOf,
  walletInfo,
} from "./fixtures.ts";
import {
  REGISTRY,
  SHOP_FLY_MS,
  chaseCandidates,
  cosmeticSlotOutcome,
  quantitiesOf,
  shopPacks,
  shopTestid as T,
  shortfallOf,
  type RegistryCosmetic,
} from "./shop.ts";

// The wallet's announcement is the real one, watched: `announceWallet` is what the shop calls.
vi.mock("../economy/wallet.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../economy/wallet.ts")>();
  return { ...actual, announceWallet: vi.fn(actual.announceWallet) };
});

beforeEach(() => {
  __resetSettingsForTests();
  resetFxSettingsForTests();
  vi.mocked(announceWallet).mockClear();
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.useRealTimers();
  __resetSettingsForTests();
  resetFxSettingsForTests();
  resetLocaleForTests();
  window.localStorage.clear();
});

describe("#1309 the shop model", () => {
  it("a short wallet owes the difference, a covered price owes nothing", () => {
    expect(shortfallOf(100, 37)).toBe(63);
    expect(shortfallOf(100, 100)).toBe(0);
    expect(shortfallOf(100, 250)).toBe(0);
  });

  it("R1953000 a cosmetic slot says what it became, a spent one the points the server gave each bar", () => {
    const [foil, , bars] = openedCosmeticPack().slots;
    expect(cosmeticSlotOutcome(foil!, 0)).toBe("New cosmetic: Mr. Vanilla full art");
    expect(cosmeticSlotOutcome(bars!, 4)).toBe("Duplicate: +4 card token bar, +4 cosmetic token bar");
  });

  it("a full shard bar buys Legendary or Mythic, a chase included, and nothing below", () => {
    const registry: RegistryCosmetic[] = [
      { id: "epic-foil", kind: "foil", name: "Epic foil", rarity: "Epic", default: false, starter: false, chase: false },
      {
        id: "legendary-foil",
        kind: "foil",
        name: "Legendary foil",
        rarity: "Legendary",
        default: false,
        starter: false,
        chase: false,
      },
      {
        id: "mythic-table",
        kind: "table",
        name: "Mythic table",
        rarity: "Mythic",
        default: false,
        starter: false,
        chase: false,
      },
      {
        id: "chase-foil",
        kind: "foil",
        name: "Chase foil",
        rarity: "Mythic",
        default: false,
        starter: false,
        chase: true,
      },
      {
        id: "starter-mythic",
        kind: "foil",
        name: "Starter mythic",
        rarity: "Mythic",
        default: false,
        starter: true,
        chase: false,
      },
    ];
    const quantities = quantitiesOf(collectionOf());
    const owned = ownedCosmeticsOf({ cards: [], account: [] });
    const ids = chaseCandidates(owned, quantities, registry).map((entry) => entry.id);
    expect(ids).toEqual(["legendary-foil", "mythic-table", "chase-foil"]);

    const bought = ownedCosmeticsOf({ cards: [], account: ["mythic-table"] });
    expect(chaseCandidates(bought, quantities, registry).map((entry) => entry.id)).toEqual([
      "legendary-foil",
      "chase-foil",
    ]);
  });
});

/** The shop's reads, answered from the fixtures unless a test says otherwise. */
function shopServer(answers: { wallet?: Reply; buy?: Reply } = {}): ReturnType<typeof stubServer> {
  return stubServer((call) => {
    if (call.method === "GET" && call.path === "/api/packs") return { status: 200, body: packsInfo() };
    if (call.method === "GET" && call.path === "/api/wallet") {
      return answers.wallet ?? { status: 200, body: walletInfo() };
    }
    if (call.method === "POST" && call.path === "/api/packs/buy") {
      return (
        answers.buy ?? {
          status: 200,
          body: { pack: "Core", jlore: 150, unopened: 3 } satisfies BoughtPackResponse,
        }
      );
    }
    return undefined;
  });
}

const TOKEN = "token-1";
const text = (id: string): string => screen.getByTestId(id).textContent ?? "";

describe("#1309 the Jlore shop", () => {
  it("a short wallet disables buying and shows the shortfall", async () => {
    shopServer();
    render(<Shop token={TOKEN} />);

    expect(await screen.findByTestId(T.wallet)).toHaveTextContent("37 Jlore");
    for (const pack of [GLOBAL_PACK_NAME, "Core"]) {
      const button = screen.getByTestId(T.buy(pack));
      expect(button).toBeDisabled();
      expect(screen.getByTestId(T.shortfall(pack))).toHaveTextContent("Short by 63 Jlore");
    }
    expect(screen.queryByTestId(T.error)).toBeNull();
  });

  it("#2115 the shop waits as a rows skeleton, and the wallet takes its place when the reads answer", async () => {
    shopServer();
    render(<Shop token={TOKEN} />);

    expectSkeleton(screen.getByTestId(T.loading), "rows");
    expect(screen.getByTestId(T.loading)).toHaveTextContent("Loading your shop…");
    expect(screen.queryByTestId(T.wallet)).toBeNull();

    expect(await screen.findByTestId(T.wallet)).toHaveTextContent("37 Jlore");
    expect(screen.queryByTestId(T.loading)).toBeNull();
    expect(screen.queryByText("Loading your shop…")).toBeNull();
    expect(screen.queryByTestId(skeletonTestid.root)).toBeNull();
  });

  it("a server refusal shows its reason and changes nothing on screen", async () => {
    const server = shopServer({
      wallet: { status: 200, body: { jlore: 250 } },
      buy: refusal(409, "conflict", "not enough Jlore for a card pack"),
    });
    const onChanged = vi.fn<() => void>();
    render(<Shop token={TOKEN} onChanged={onChanged} />);
    await screen.findByTestId(T.wallet);

    fireEvent.click(screen.getByTestId(T.buy("Core")));

    expect(await screen.findByTestId(T.error)).toHaveTextContent("not enough Jlore for a card pack");
    expect(text(T.wallet)).toBe("250 Jlore");
    expect(text(T.held("Core"))).toContain("Core pack: 2 unopened");
    expect(server.to("POST", "/api/packs/buy")).toHaveLength(1);
    expect(server.to("POST", "/api/packs/buy")[0]?.body).toEqual({ pack: "Core" });
    expect(onChanged, "a refusal changed nothing to tell the page").not.toHaveBeenCalled();
    expect(announceWallet, "nor to tell the pills").not.toHaveBeenCalled();
    expect(screen.queryByTestId(T.fly("Core")), "no pack flew").toBeNull();
    expect(screen.getByTestId(T.packObject("Core")), "the press is let go").toHaveAttribute("data-state", "idle");
  });

  it("#2053 a failed read says why in the reader's language", async () => {
    await setLocale("zh-CN");
    shopServer({ wallet: { status: 500 } });
    render(<Shop token={TOKEN} />);
    expect(await screen.findByTestId(T.loadError)).toHaveTextContent("服务器返回了 500");
  });

  it("#2053 a refused buy says why in the reader's language", async () => {
    await setLocale("zh-CN");
    shopServer({ wallet: { status: 200, body: { jlore: 250 } }, buy: { status: 500 } });
    render(<Shop token={TOKEN} />);
    await screen.findByTestId(T.wallet);

    fireEvent.click(screen.getByTestId(T.buy("Core")));

    expect(await screen.findByTestId(T.error)).toHaveTextContent("服务器返回了 500");
  });

  it("#2053 a server sentence the dictionary lacks stays English for a reader of Chinese", async () => {
    await setLocale("zh-CN");
    shopServer({
      wallet: { status: 200, body: { jlore: 250 } },
      buy: refusal(409, "conflict", "not enough Jlore for a card pack"),
    });
    render(<Shop token={TOKEN} />);
    await screen.findByTestId(T.wallet);

    fireEvent.click(screen.getByTestId(T.buy("Core")));

    expect(await screen.findByTestId(T.error)).toHaveTextContent("not enough Jlore for a card pack");
  });

  it("a bought pack moves the wallet and the count from the answer, and tells the page", async () => {
    shopServer({ wallet: { status: 200, body: { jlore: 250 } } });
    const onChanged = vi.fn<() => void>();
    render(<Shop token={TOKEN} onChanged={onChanged} />);
    await screen.findByTestId(T.wallet);

    fireEvent.click(screen.getByTestId(T.buy("Core")));

    expect(await screen.findByText("150 Jlore")).toBeInTheDocument();
    expect(text(T.held("Core"))).toContain("Core pack: 3 unopened");
    expect(screen.queryByTestId(T.shortfall("Core"))).toBeNull();
    expect(screen.queryByTestId(T.error)).toBeNull();
    expect(onChanged).toHaveBeenCalledTimes(1);
  });

  it("each pack kind stands on the shelf as a PackObject beside its count, price and Buy", async () => {
    shopServer({ wallet: { status: 200, body: { jlore: 250 } } });
    render(<Shop token={TOKEN} />);
    await screen.findByTestId(T.wallet);

    expect(shopPacks()[0], "the global pack first").toBe(GLOBAL_PACK_NAME);
    for (const pack of shopPacks()) {
      const row = screen.getByTestId(T.held(pack));
      const object = screen.getByTestId(T.packObject(pack));
      expect(row, pack).toContainElement(object);
      expect(object, pack).toHaveAttribute("data-pack", pack);
      expect(object, pack).toHaveAttribute("data-size", "shelf");
      expect(object, pack).toHaveAttribute("data-state", "idle");
      expect(row, pack).toContainElement(screen.getByTestId(T.buy(pack)));
      expect(row, pack).toHaveTextContent("Jlore");
      expect(screen.queryByTestId(T.fly(pack)), `${pack} flew nothing yet`).toBeNull();
    }
    expect(text(T.held("Core"))).toContain("Core pack: 2 unopened");
    expect(text(T.held(GLOBAL_PACK_NAME))).toContain("Global pack: 1 unopened");
  });

  it("Buy presses the pack; the answer sends a copy of it into the count and announces the wallet", async () => {
    shopServer({ wallet: { status: 200, body: { jlore: 250 } } });
    render(<Shop token={TOKEN} />);
    await screen.findByTestId(T.wallet);
    const pack = screen.getByTestId(T.packObject("Core"));

    fireEvent.click(screen.getByTestId(T.buy("Core")));

    expect(pack, "pressed while the request is out").toHaveAttribute("data-state", "pressed");
    expect(screen.getByTestId(T.packObject(GLOBAL_PACK_NAME)), "no other pack is").toHaveAttribute("data-state", "idle");
    expect(announceWallet, "nothing is announced before the answer").not.toHaveBeenCalled();
    expect(screen.queryByTestId(T.fly("Core"))).toBeNull();

    expect(await screen.findByText("150 Jlore")).toBeInTheDocument();
    expect(pack, "let go on the answer").toHaveAttribute("data-state", "idle");
    expect(announceWallet).toHaveBeenCalledTimes(1);
    expect(announceWallet).toHaveBeenCalledWith(150);
    expect(text(T.held("Core"))).toContain("Core pack: 3 unopened");

    const copy = screen.getByTestId(T.fly("Core"));
    expect(screen.getByTestId(T.held("Core"))).toContainElement(copy);
    expect(copy, "a picture, not another pack to buy").toHaveAttribute("aria-hidden", "true");
    expect(copy.querySelector(".pack-object")).toHaveAttribute("data-pack", "Core");
    expect(copy.style.getPropertyValue("--shop-fly-ms")).toBe(`${String(SHOP_FLY_MS)}ms`);
    expect(screen.queryByTestId(T.fly(GLOBAL_PACK_NAME))).toBeNull();
  });

  it("under reduced motion a bought pack's count changes at once and no copy flies", async () => {
    writeSettings({ reduceMotion: true });
    shopServer({ wallet: { status: 200, body: { jlore: 250 } } });
    render(<Shop token={TOKEN} />);
    await screen.findByTestId(T.wallet);
    expect(screen.getByTestId(T.packObject("Core"))).toHaveAttribute("data-motion", "reduce");

    fireEvent.click(screen.getByTestId(T.buy("Core")));

    expect(await screen.findByText("150 Jlore")).toBeInTheDocument();
    expect(text(T.held("Core"))).toContain("Core pack: 3 unopened");
    expect(screen.queryByTestId(T.fly("Core"))).toBeNull();
    expect(announceWallet).toHaveBeenCalledWith(150);
  });

  it("the page's next revision reads the wallet and the packs again", async () => {
    // The wallet moves at the server after the first read: the second read must show it.
    let jlore = walletInfo().jlore;
    const server = stubServer((call) => {
      if (call.method === "GET" && call.path === "/api/packs") return { status: 200, body: packsInfo() };
      if (call.method === "GET" && call.path === "/api/wallet") return { status: 200, body: { jlore } };
      return undefined;
    });
    const { rerender } = render(<Shop token={TOKEN} />);
    expect(await screen.findByTestId(T.wallet)).toHaveTextContent("37 Jlore");
    expect(server.to("GET", "/api/wallet")).toHaveLength(1);

    jlore = 88;
    rerender(<Shop token={TOKEN} revision={1} />);

    await waitFor(() => {
      expect(text(T.wallet)).toBe("88 Jlore");
    });
    expect(server.to("GET", "/api/packs")).toHaveLength(2);
  });

  it("R2125000 the next visit shows the wallet a buy left at once, and keeps it when the read behind it fails", async () => {
    shopServer({ wallet: { status: 200, body: { jlore: 250 } } });
    render(<Shop token={TOKEN} />);
    await screen.findByTestId(T.wallet);
    fireEvent.click(screen.getByTestId(T.buy("Core")));
    expect(await screen.findByText("150 Jlore")).toBeInTheDocument();
    cleanup();

    shopServer({ wallet: refusal(503, "unavailable", "the wallet is not reachable") });
    render(<Shop token={TOKEN} />);

    expect(text(T.wallet)).toBe("150 Jlore");
    expect(screen.queryByTestId(T.loading)).toBeNull();
    expect(text(T.held("Core"))).toContain("Core pack: 3 unopened");

    await act(async () => {
      await new Promise((resolve) => {
        setTimeout(resolve, 0);
      });
    });
    expect(screen.queryByTestId(T.loadError)).toBeNull();
    expect(screen.queryByTestId(T.retry)).toBeNull();
    expect(text(T.wallet)).toBe("150 Jlore");
  });

  it("R2125000 another account's visit finds nothing kept and says why when its read fails", async () => {
    shopServer({ wallet: { status: 200, body: { jlore: 250 } } });
    render(<Shop token={TOKEN} />);
    await screen.findByTestId(T.wallet);
    cleanup();

    shopServer({ wallet: refusal(503, "unavailable", "the wallet is not reachable") });
    render(<Shop token="token-other" />);

    expect(screen.getByTestId(T.loading)).toBeInTheDocument();
    expect(await screen.findByTestId(T.loadError)).toHaveTextContent("the wallet is not reachable");
  });

  it("a failed read says why", async () => {
    shopServer({ wallet: refusal(503, "unavailable", "the wallet is not reachable") });
    render(<Shop token={TOKEN} />);
    expect(await screen.findByTestId(T.loadError)).toHaveTextContent("the wallet is not reachable");
    expect(screen.queryByTestId(T.wallet)).toBeNull();
  });

  it("#1485 a failed read that works on the page's next revision shows the shop and drops the alert", async () => {
    const answers: { wallet?: Reply } = { wallet: refusal(503, "unavailable", "the wallet is not reachable") };
    shopServer(answers);
    const { rerender } = render(<Shop token={TOKEN} />);
    expect(await screen.findByTestId(T.loadError)).toHaveTextContent("the wallet is not reachable");

    answers.wallet = { status: 200, body: walletInfo() };
    rerender(<Shop token={TOKEN} revision={1} />);

    expect(await screen.findByTestId(T.wallet)).toHaveTextContent("37 Jlore");
    expect(screen.queryByTestId(T.loadError)).toBeNull();
    expect(screen.queryByTestId(T.retry)).toBeNull();
  });

  it("#1485 Try again reads once more, cannot be pressed twice while the read is out, and drops the alert once it works", async () => {
    const answers: { wallet?: Reply } = { wallet: refusal(503, "unavailable", "the wallet is not reachable") };
    const server = shopServer(answers);
    render(<Shop token={TOKEN} />);
    await waitFor(() => {
      expect(screen.getByTestId(T.retry)).toBeEnabled();
    });
    expect(server.to("GET", "/api/wallet")).toHaveLength(1);

    answers.wallet = { status: 200, body: walletInfo() };
    fireEvent.click(screen.getByTestId(T.retry));
    expect(screen.getByTestId(T.retry)).toBeDisabled();
    fireEvent.click(screen.getByTestId(T.retry));

    expect(await screen.findByTestId(T.wallet)).toHaveTextContent("37 Jlore");
    expect(screen.queryByTestId(T.loadError)).toBeNull();
    expect(server.to("GET", "/api/wallet")).toHaveLength(2);
  });
});

/** The cosmetic packs' reads, answered from the fixtures unless a test says otherwise. */
function cosmeticServer(answers: { packs?: Reply; open?: Reply } = {}): ReturnType<typeof stubServer> {
  return stubServer((call) => {
    if (call.method === "GET" && call.path === "/api/cosmetic-packs") {
      return answers.packs ?? { status: 200, body: cosmeticPacksInfo() };
    }
    if (call.method === "GET" && call.path === "/api/collection") return { status: 200, body: collectionOf() };
    if (call.method === "GET" && call.path === "/api/cosmetics") return { status: 200, body: ownedCosmeticsOf() };
    if (call.method === "POST" && call.path === "/api/cosmetic-packs/open") {
      return answers.open ?? { status: 200, body: openedCosmeticPack() };
    }
    return undefined;
  });
}

describe("R2378000 one spend at a time", () => {
  /** The server's words for a spend sent past the ones waiting their turn. */
  const BUSY = "your last purchase or opening is still going through; try again in a moment";

  let release: () => void = () => undefined;
  let outside: Promise<void> = Promise.resolve();

  /** Another spend of the tab's (a pack opening, say), out until `settleOutside`. */
  function spendOutside(): void {
    act(() => {
      outside = whileSpending(
        () =>
          new Promise<void>((resolve) => {
            release = resolve;
          }),
      );
    });
  }

  async function settleOutside(): Promise<void> {
    await act(async () => {
      release();
      await outside;
    });
  }

  afterEach(async () => {
    await settleOutside();
  });

  it("R2378000 Buy waits while another spend is out, turning a wheel, and a press sends nothing", async () => {
    const server = shopServer({ wallet: { status: 200, body: { jlore: 250 } } });
    render(<Shop token={TOKEN} />);
    await screen.findByTestId(T.wallet);
    expect(screen.getByTestId(T.buy("Core"))).toHaveAttribute("aria-busy", "false");

    spendOutside();
    for (const pack of shopPacks()) {
      expect(screen.getByTestId(T.buy(pack)), pack).toBeDisabled();
      expect(screen.getByTestId(T.buy(pack)), pack).toHaveAttribute("aria-busy", "true");
      expect(screen.getByTestId(T.buy(pack)), pack).toHaveClass("spend-wait");
    }
    fireEvent.click(screen.getByTestId(T.buy("Core")));
    expect(server.to("POST", "/api/packs/buy")).toHaveLength(0);

    await settleOutside();
    expect(screen.getByTestId(T.buy("Core"))).toBeEnabled();
    fireEvent.click(screen.getByTestId(T.buy("Core")));
    expect(screen.getByTestId(T.buy("Core")), "its own press waits with the wheel too").toHaveAttribute("aria-busy", "true");
    expect(await screen.findByText("150 Jlore")).toBeInTheDocument();
    expect(screen.getByTestId(T.buy("Core"))).toHaveAttribute("aria-busy", "false");
    expect(server.to("POST", "/api/packs/buy")).toHaveLength(1);
  });

  it("R2378000 a short wallet's Buy turns no wheel while a spend is out", async () => {
    shopServer();
    render(<Shop token={TOKEN} />);
    await screen.findByTestId(T.wallet);

    spendOutside();
    expect(screen.getByTestId(T.buy("Core"))).toBeDisabled();
    expect(screen.getByTestId(T.buy("Core")), "it waits on the wallet, not the spend").toHaveAttribute("aria-busy", "false");
  });

  it("R2378000 a refusal past the ones waiting their turn shows the server's words, in the reader's language too", async () => {
    shopServer({ wallet: { status: 200, body: { jlore: 250 } }, buy: refusal(429, "rate_limited", BUSY) });
    render(<Shop token={TOKEN} />);
    await screen.findByTestId(T.wallet);

    fireEvent.click(screen.getByTestId(T.buy("Core")));
    expect(await screen.findByTestId(T.error)).toHaveTextContent(BUSY);
    expect(screen.getByTestId(T.buy("Core")), "nothing is out, so Buy can be pressed again").toBeEnabled();

    cleanup();
    await setLocale("zh-CN");
    render(<Shop token={TOKEN} />);
    await screen.findByTestId(T.wallet);
    fireEvent.click(screen.getByTestId(T.buy("Core")));
    expect(await screen.findByTestId(T.error)).toHaveTextContent("你上一次的购买或开包仍在处理中；请稍后再试");
  });
});

describe("#1309 the cosmetic packs", () => {
  it("the shard bar's choice appears only when full", async () => {
    cosmeticServer();
    render(<CosmeticPacks token={TOKEN} />);
    expect(await screen.findByTestId(T.shardBar)).toHaveTextContent("19 / 20");
    expect(screen.queryByTestId(T.shardChoice)).toBeNull();
    expect(screen.queryByTestId(T.shardEmpty)).toBeNull();
    cleanup();
    // R2125000: the second visit is a first one, not a revisit that opens on the kept 19 / 20.
    clearReadCache();

    // Full, every Legendary and Mythic owned (every chase a starter, as before R1833000): the empty
    // sentence, still no choice.
    cosmeticServer({ packs: { status: 200, body: cosmeticPacksFull() } });
    render(<CosmeticPacks token={TOKEN} registry={REGISTRY.filter((cosmetic) => !cosmetic.chase || cosmetic.starter)} />);
    expect(await screen.findByTestId(T.shardBar)).toHaveTextContent("20 / 20");
    expect(screen.queryByTestId(T.shardChoice)).toBeNull();
    expect(screen.getByTestId(T.shardEmpty)).toHaveTextContent("Nothing to buy yet");
  });

  it("the panel shows no odds: they are on the Purchases screen", async () => {
    cosmeticServer();
    render(<CosmeticPacks token={TOKEN} />);
    const panel = await screen.findByTestId(T.packsPanel);
    await screen.findByTestId(T.unopened);

    expect(panel).not.toHaveTextContent("%");
    expect(panel).not.toHaveTextContent(/each slot|guaranteed slot/i);
    expect(panel.querySelector('[data-testid*="odds"]')).toBeNull();
  });

  it("the unopened cosmetic pack is a PackObject above Open, and none is drawn with none unopened", async () => {
    cosmeticServer();
    render(<CosmeticPacks token={TOKEN} />);
    const pack = await screen.findByTestId(T.cosmeticPackObject);
    expect(pack).toHaveAttribute("data-pack", "cosmetic");
    expect(pack).toHaveAttribute("data-size", "shelf");
    expect(pack.compareDocumentPosition(screen.getByTestId(T.open)) & Node.DOCUMENT_POSITION_FOLLOWING).not.toBe(0);
    cleanup();

    cosmeticServer({ packs: { status: 200, body: { ...cosmeticPacksInfo(), unopened: 0 } } });
    render(<CosmeticPacks token={TOKEN} />);
    const noPacks = await screen.findByText("No unopened packs.");
    expect(noPacks).toBeInTheDocument();
    expectEmptyState(noPacks);
    expect(screen.queryByTestId(T.cosmeticPackObject)).toBeNull();
    expect(screen.queryByTestId(T.open)).toBeNull();
  });

  it("#2115 the cosmetic packs wait as a panel skeleton, and the unopened count takes its place when the reads answer", async () => {
    cosmeticServer();
    render(<CosmeticPacks token={TOKEN} />);

    expectSkeleton(screen.getByTestId(T.packsLoading), "panel");
    expect(screen.getByTestId(T.packsLoading)).toHaveTextContent("Loading your cosmetic packs…");
    expect(screen.queryByTestId(T.unopened)).toBeNull();

    expect(await screen.findByTestId(T.unopened)).toBeInTheDocument();
    expect(screen.queryByTestId(T.packsLoading)).toBeNull();
    expect(screen.queryByText("Loading your cosmetic packs…")).toBeNull();
    expect(screen.queryByTestId(skeletonTestid.root)).toBeNull();
  });

  it("R1953000 an opened cosmetic pack pins both token bars at the top, its spent slot's points flying in before they fill", async () => {
    vi.useFakeTimers();
    cosmeticServer();
    render(<CosmeticPacks token={TOKEN} />);
    await flush();
    expect(screen.queryByTestId(T.openingBars)).toBeNull();

    fireEvent.click(screen.getByTestId(T.open));
    await flush();

    const bars = screen.getByTestId(T.openingBars);
    const card = (): HTMLElement => screen.getByTestId(`${T.openingBars}-card`);
    const cosmetic = (): HTMLElement => screen.getByTestId(`${T.openingBars}-cosmetic`);
    expect(bars).toHaveClass("opening-hud");
    expect(bars).toContainElement(card());
    expect(card()).toHaveTextContent("Card token bar");
    expect(cosmetic()).toHaveTextContent("Cosmetic token bar");
    // The spent slot's points, the server's, fly from it; the bars wait where the opening found them.
    const flight = screen.getByTestId(`${T.openingBars}-flight-2`);
    expect(flight).toHaveTextContent("+4");
    expect(flight).toHaveAttribute("data-rarity", "Rare");
    expect(bars.querySelectorAll(".bar-flight")).toHaveLength(1);
    expect(card()).toHaveTextContent("3 / 12");
    expect(cosmetic()).toHaveTextContent("5 / 30");
    expect(text(T.slot(2))).toContain("Duplicate: +4 card token bar, +4 cosmetic token bar");

    advance(flightsMs(1));
    expect(bars.querySelector(".bar-flight")).toBeNull();
    expect(card()).toHaveTextContent("7 / 12");
    expect(cosmetic()).toHaveTextContent("9 / 30");
    cleanup();

    // An opening that spent no slot still shows both bars, at rest, with nothing to fly.
    const still = { card: { before: 3, after: 3, size: 12, pays: 0 }, cosmetic: { before: 5, after: 5, size: 30, pays: 0 } };
    const opened = openedCosmeticPack();
    cosmeticServer({
      open: {
        status: 200,
        body: {
          ...opened,
          slots: opened.slots.slice(0, 2),
          bars: { ...still, cosmeticPaid: [], slotPoints: [0, 0] },
        },
      },
    });
    render(<CosmeticPacks token={TOKEN} />);
    await flush();
    fireEvent.click(screen.getByTestId(T.open));
    await flush();
    expect(screen.getByTestId(T.openingBars).querySelector(".bar-flight")).toBeNull();
    expect(screen.getByTestId(`${T.openingBars}-card`)).toHaveTextContent("3 / 12");
    expect(screen.getByTestId(`${T.openingBars}-cosmetic`)).toHaveTextContent("5 / 30");
  });

  it("an opened cosmetic pack reveals each slot, and tells the page", async () => {
    cosmeticServer();
    const onChanged = vi.fn<() => void>();
    render(<CosmeticPacks token={TOKEN} onChanged={onChanged} />);
    expect(await screen.findByTestId(T.unopened)).toHaveTextContent("Cosmetic pack: 2 unopened");

    fireEvent.click(screen.getByTestId(T.open));

    const reveal = await screen.findByTestId(T.reveal);
    expect(reveal).toBeInTheDocument();
    expect(text(T.slot(0))).toContain("New cosmetic: Mr. Vanilla full art");
    expect(text(T.slot(0))).toContain("Mr. Vanilla");
    expect(text(T.slot(0))).toContain("Its card was granted too");
    expect(text(T.slot(1))).toContain("New cosmetic: Wood");
    expect(text(T.slot(2))).toContain("Duplicate:");
    expect(screen.getByTestId(T.slot(0))).toHaveAttribute("data-rarity", "Epic");
    expect(await screen.findByTestId(T.unopened)).toHaveTextContent("Cosmetic pack: 1 unopened");
    expect(screen.getByTestId(T.shardBar)).toHaveTextContent("20 / 20");
    expect(onChanged).toHaveBeenCalledTimes(1);
  });

  it("R1827000 an opened cosmetic pack's portrait shows its name and its art", async () => {
    const opened = openedCosmeticPack();
    cosmeticServer({
      open: {
        status: 200,
        body: {
          ...opened,
          slots: [
            {
              rarity: "Epic",
              grant: { kind: "cosmetic", cosmeticId: "portrait-timmy", cosmeticRarity: "Epic", cardId: null, cardGranted: false },
            },
            ...opened.slots.slice(1),
          ],
        },
      },
    });
    render(<CosmeticPacks token={TOKEN} />);
    await screen.findByTestId(T.unopened);
    fireEvent.click(screen.getByTestId(T.open));
    await screen.findByTestId(T.reveal);

    const slot = screen.getByTestId(T.slot(0));
    expect(slot).toHaveTextContent("New cosmetic: Tempo Timmy portrait");
    // An account cosmetic: on no card, so no card line.
    expect(slot).not.toHaveTextContent("On ");
    const art = slot.querySelector('[data-portrait-art="timmy"]');
    expect(art).not.toBeNull();
    expect(art).toHaveClass("pack-slot__portrait");
    const outcome = slot.querySelector(".pack-slot__outcome");
    expect(outcome?.nextElementSibling).toBe(art);
    expect(screen.getByTestId(T.slot(1)).querySelector("[data-portrait-art]")).toBeNull();
  });

  it("R1833000 an opened cosmetic pack's new chase shows its name, rarity and look", async () => {
    const opened = openedCosmeticPack();
    cosmeticServer({
      open: {
        status: 200,
        body: {
          ...opened,
          slots: [
            {
              rarity: "Mythic",
              grant: { kind: "cosmetic", cosmeticId: "field-cosmic-drift", cosmeticRarity: "Mythic", cardId: null, cardGranted: false },
            },
            {
              rarity: "Legendary",
              grant: { kind: "cosmetic", cosmeticId: "border-filigree", cosmeticRarity: "Legendary", cardId: "core-004", cardGranted: false },
            },
            ...opened.slots.slice(2),
          ],
        },
      },
    });
    render(<CosmeticPacks token={TOKEN} />);
    await screen.findByTestId(T.unopened);
    fireEvent.click(screen.getByTestId(T.open));
    await screen.findByTestId(T.reveal);

    // A Mythic backdrop, on the account: a mini board on the default playmat, after the outcome's words.
    const drift = screen.getByTestId(T.slot(0));
    expect(drift).toHaveAttribute("data-rarity", "Mythic");
    expect(drift).toHaveTextContent("New cosmetic: Cosmic drift");
    expect(drift).not.toHaveTextContent("On ");
    const look = drift.querySelector(".cosmetic-look");
    expect(drift.querySelector(".pack-slot__outcome")?.nextElementSibling).toBe(look);
    expect(look?.querySelector(".board-backdrop")).toHaveAttribute("data-backdrop", "field-cosmic-drift");
    // A Legendary border, on its card's face.
    const filigree = screen.getByTestId(T.slot(1));
    expect(filigree).toHaveAttribute("data-rarity", "Legendary");
    expect(filigree).toHaveTextContent("New cosmetic: Filigree");
    expect(filigree.querySelector(".cosmetic-look .cf")).toHaveAttribute("data-frame", "border-filigree");
    expect(filigree).toHaveTextContent("On ");
  });

  it("R1833000 a full Chase Shard bar offers the new chases with their names, rarities and looks", async () => {
    cosmeticServer({ packs: { status: 200, body: cosmeticPacksFull() } });
    render(<CosmeticPacks token={TOKEN} />);
    await screen.findByTestId(T.shardChoice);
    // The shipped registry's chases the fixture account does not own, a per-card one for a card it owns.
    const astral = screen.getByTestId(T.chasePick("table-astral")).closest(".token-row");
    expect(astral).toHaveAttribute("data-rarity", "Mythic");
    expect(astral).toHaveTextContent("Astral · Mythic · Chase");
    expect(screen.getByTestId(T.chasePreview("table-astral")).querySelector(".collection-miniboard")).toHaveAttribute("data-table", "table-astral");
    const molten = screen.getByTestId(T.chasePick("foil-molten-gold")).closest(".token-row");
    expect(molten).toHaveTextContent("Molten gold · Legendary · Chase");
    expect(screen.getByTestId(T.chasePreview("foil-molten-gold")).querySelector(".cf")).toHaveAttribute("data-cosmetic-foil", "foil-molten-gold");
    expect(screen.getByTestId(T.chasePreview("card-back-nebula")).querySelector(".cf-back")).toHaveAttribute("data-card-back", "card-back-nebula");
    // A starter chase is owned by every account already: not offered.
    expect(screen.queryByTestId(T.chasePick("foil-cracked-ice"))).toBeNull();
  });

  it("R1579000 an opened cosmetic pack links to the Collection's cosmetics", async () => {
    cosmeticServer();
    render(<CosmeticPacks token={TOKEN} />);
    await screen.findByTestId(T.unopened);
    expect(screen.queryByTestId(T.equipLink)).toBeNull();

    fireEvent.click(screen.getByTestId(T.open));
    await screen.findByTestId(T.reveal);

    expect(screen.getByTestId(T.equipLink)).toHaveAttribute("href", "/collection?section=cosmetics");
  });

  it("#2053 a failed read says why in the reader's language", async () => {
    await setLocale("zh-CN");
    cosmeticServer({ packs: { status: 500 } });
    render(<CosmeticPacks token={TOKEN} />);
    expect(await screen.findByTestId(T.packsLoadError)).toHaveTextContent("服务器返回了 500");
  });

  it("#2053 a refused open says why in the reader's language", async () => {
    await setLocale("zh-CN");
    cosmeticServer({ open: { status: 500 } });
    render(<CosmeticPacks token={TOKEN} />);
    await screen.findByTestId(T.unopened);

    fireEvent.click(screen.getByTestId(T.open));

    expect(await screen.findByTestId(T.packsError)).toHaveTextContent("服务器返回了 500");
  });

  it("#2053 an opened pack names the card its cosmetic went on in the reader's language", async () => {
    await setLocale("zh-CN");
    cosmeticServer();
    render(<CosmeticPacks token={TOKEN} />);
    await screen.findByTestId(T.unopened);

    fireEvent.click(screen.getByTestId(T.open));
    await screen.findByTestId(T.reveal);

    // Only the card line: the slot's outcome line holds the cosmetic's name, which stays English.
    const card = screen.getByTestId(T.slot(0)).querySelector(".pack-slot__card");
    expect(card).toHaveTextContent(`用于${CHINESE["core-008"]!.name}`);
    expect(card).not.toHaveTextContent(CATALOG["core-008"]!.name);
  });

  it("a refused open shows its reason and changes nothing on screen", async () => {
    cosmeticServer({ open: refusal(409, "conflict", "no unopened cosmetic pack") });
    const onChanged = vi.fn<() => void>();
    render(<CosmeticPacks token={TOKEN} onChanged={onChanged} />);
    await screen.findByTestId(T.unopened);

    fireEvent.click(screen.getByTestId(T.open));

    expect(await screen.findByTestId(T.packsError)).toHaveTextContent("no unopened cosmetic pack");
    expect(screen.queryByTestId(T.reveal)).toBeNull();
    expect(text(T.unopened)).toContain("2 unopened");
    expect(onChanged).not.toHaveBeenCalled();
  });

  it("#1485 a failed read that works on the page's next revision shows the shop and drops the alert", async () => {
    const answers: { packs?: Reply } = {
      packs: refusal(503, "unavailable", "the cosmetic packs are not reachable"),
    };
    cosmeticServer(answers);
    const { rerender } = render(<CosmeticPacks token={TOKEN} />);
    expect(await screen.findByTestId(T.packsLoadError)).toHaveTextContent("the cosmetic packs are not reachable");

    answers.packs = { status: 200, body: cosmeticPacksInfo() };
    rerender(<CosmeticPacks token={TOKEN} revision={1} />);

    expect(await screen.findByTestId(T.unopened)).toHaveTextContent("Cosmetic pack: 2 unopened");
    expect(screen.queryByTestId(T.packsLoadError)).toBeNull();
    expect(screen.queryByTestId(T.packsRetry)).toBeNull();
  });

  it("#1485 Try again reads once more, cannot be pressed twice while the read is out, and drops the alert once it works", async () => {
    const answers: { packs?: Reply } = {
      packs: refusal(503, "unavailable", "the cosmetic packs are not reachable"),
    };
    const server = cosmeticServer(answers);
    render(<CosmeticPacks token={TOKEN} />);
    await waitFor(() => {
      expect(screen.getByTestId(T.packsRetry)).toBeEnabled();
    });
    expect(server.to("GET", "/api/cosmetic-packs")).toHaveLength(1);

    answers.packs = { status: 200, body: cosmeticPacksInfo() };
    fireEvent.click(screen.getByTestId(T.packsRetry));
    expect(screen.getByTestId(T.packsRetry)).toBeDisabled();
    fireEvent.click(screen.getByTestId(T.packsRetry));

    expect(await screen.findByTestId(T.unopened)).toHaveTextContent("Cosmetic pack: 2 unopened");
    expect(screen.queryByTestId(T.packsLoadError)).toBeNull();
    expect(server.to("GET", "/api/cosmetic-packs")).toHaveLength(2);
  });
});

/** The registry's one chase: a Legendary account cosmetic the fixture's account lacks. */
const AURORA: RegistryCosmetic = {
  id: "table-aurora",
  kind: "table",
  name: "Aurora table",
  rarity: "Legendary",
  default: false,
  starter: false,
  chase: true,
};

/** Lets the stubbed requests answer and React render them, without moving a timer. */
async function flush(): Promise<void> {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(0);
  });
}

function advance(ms: number): void {
  act(() => {
    vi.advanceTimersByTime(ms);
  });
}

const shardBar = (): HTMLElement => screen.getByTestId(T.shardBar);
const shardFill = (): string => shardBar().querySelector<HTMLElement>(".fill-bar__fill")!.style.transform;
const chaseMoment = `${T.shardReveal}-chase`;

describe("#1667 the Chase Shard bar", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  /** The panel read (19 of 20), then the opening pressed: the answer is in and the bar is at its start. */
  async function opened(over: { packs?: Reply; open?: Reply } = {}): Promise<void> {
    cosmeticServer(over);
    render(<CosmeticPacks token={TOKEN} registry={[AURORA]} />);
    await flush();
    fireEvent.click(screen.getByTestId(T.open));
    await flush();
  }

  it("is a FillBar labeled Chase Shard, at rest on what the panel read", async () => {
    cosmeticServer();
    render(<CosmeticPacks token={TOKEN} registry={[AURORA]} />);
    await flush();

    expect(shardBar()).toHaveClass("fill-bar");
    expect(shardBar()).toHaveTextContent("Chase Shard");
    expect(shardBar()).toHaveTextContent("19 / 20");
    expect(shardBar()).toHaveAttribute("data-state", "resting");
    expect(shardFill()).toBe("scaleX(0.95)");
    expect(shardBar().querySelector("progress")).toHaveAttribute("max", "20");
    expect(screen.queryByTestId(T.shardChoice)).toBeNull();
  });

  it("fills from the points read before the opening to the answer's, and at full bursts and reveals the chase choice", async () => {
    await opened();

    // The answer is in: the bar starts where the panel read it and sweeps to where the answer left it.
    expect(shardBar()).toHaveAttribute("data-state", "filling");
    expect(shardFill()).toBe("scaleX(0.95)");
    expect(shardBar().querySelector("progress")).toHaveAttribute("value", "20");
    expect(shardBar()).toHaveTextContent("20 / 20");
    expect(screen.queryByTestId(T.shardBurst), "it has not reached full yet").toBeNull();
    expect(screen.queryByTestId(T.shardChoice), "nor is the choice shown").toBeNull();
    expect(screen.queryByTestId(T.shardEmpty)).toBeNull();

    advance(segmentMs({ from: 19, to: 20, burst: false }, 20));

    // Full: it holds there, bursts, and the choice is on its way as a moment of its own.
    expect(shardBar()).toHaveAttribute("data-state", "resting");
    expect(shardFill()).toBe("scaleX(1)");
    expect(screen.getByTestId(T.shardBurst)).toHaveAttribute("aria-hidden", "true");
    expect(screen.getByTestId(T.shardChoice)).toBeInTheDocument();
    expect(screen.queryByTestId(chaseMoment), "it flies out after its stagger").toBeNull();
    expect(screen.getByTestId(T.shardChoice)).not.toHaveTextContent("pick a chase:");

    advance(REVEAL_STAGGER_MS);

    const moment = screen.getByTestId(chaseMoment);
    expect(moment).toHaveTextContent("The bar is full — pick a chase:");
    expect(screen.getByTestId(T.shardReveal)).toContainElement(moment);
    expect(screen.getByTestId(T.shardChoice)).toContainElement(screen.getByTestId(T.chasePick(AURORA.id)));
    expect(screen.getByTestId(T.shardChoice)).toHaveTextContent("Aurora table · Legendary · Chase");
  });

  it("an opening that leaves the bar short of full fills it and bursts nothing", async () => {
    cosmeticServer({
      packs: { status: 200, body: { ...cosmeticPacksInfo(), shardBar: { points: 5, size: 20 } } },
      open: { status: 200, body: { ...openedCosmeticPack(), shardBefore: 5, shardBar: { points: 6, size: 20 } } },
    });
    render(<CosmeticPacks token={TOKEN} registry={[AURORA]} />);
    await flush();
    fireEvent.click(screen.getByTestId(T.open));
    await flush();
    expect(shardFill()).toBe("scaleX(0.25)");

    advance(segmentMs({ from: 5, to: 6, burst: false }, 20));

    expect(shardBar()).toHaveAttribute("data-state", "resting");
    expect(shardFill()).toBe("scaleX(0.3)");
    expect(shardBar()).toHaveTextContent("6 / 20");
    expect(screen.queryByTestId(T.shardBurst)).toBeNull();
    expect(screen.queryByTestId(T.shardChoice)).toBeNull();
    expect(shardBar()).toHaveAttribute("data-bursts", "0");
  });

  it("fills from the points the server says the opening found, not from the panel's earlier read", async () => {
    // The panel read 5, but another tab opened packs since: the server found 12 and left 13.
    await opened({
      packs: { status: 200, body: { ...cosmeticPacksInfo(), shardBar: { points: 5, size: 20 } } },
      open: { status: 200, body: { ...openedCosmeticPack(), shardBefore: 12, shardBar: { points: 13, size: 20 } } },
    });
    expect(shardFill()).toBe("scaleX(0.6)");

    advance(segmentMs({ from: 12, to: 13, burst: false }, 20));

    expect(shardFill()).toBe("scaleX(0.65)");
    expect(shardBar()).toHaveTextContent("13 / 20");
  });

  it("a full bar holds: another opening leaves it full with no burst, and its choice stays", async () => {
    cosmeticServer({
      packs: { status: 200, body: cosmeticPacksFull() },
      open: { status: 200, body: { ...openedCosmeticPack(), shardBefore: 20 } },
    });
    render(<CosmeticPacks token={TOKEN} registry={[AURORA]} />);
    await flush();
    expect(shardBar()).toHaveTextContent("20 / 20");
    expect(screen.getByTestId(T.shardChoice), "read full, the choice is there at once").toBeInTheDocument();

    fireEvent.click(screen.getByTestId(T.open));
    await flush();
    advance(REVEAL_STAGGER_MS * 4);

    expect(shardBar()).toHaveTextContent("20 / 20");
    expect(shardBar()).toHaveAttribute("data-state", "resting");
    expect(shardFill()).toBe("scaleX(1)");
    expect(shardBar()).toHaveAttribute("data-bursts", "0");
    expect(screen.queryByTestId(T.shardBurst)).toBeNull();
    expect(screen.getByTestId(T.shardChoice)).toBeInTheDocument();
  });

  it("a chase bought empties the bar, which rests at 0, and takes the choice with it", async () => {
    const server = stubServer((call) => {
      if (call.method === "GET" && call.path === "/api/cosmetic-packs") return { status: 200, body: cosmeticPacksFull() };
      if (call.method === "GET" && call.path === "/api/collection") return { status: 200, body: collectionOf() };
      if (call.method === "GET" && call.path === "/api/cosmetics") return { status: 200, body: ownedCosmeticsOf() };
      if (call.method === "POST" && call.path === "/api/cosmetic-packs/chase") {
        return { status: 200, body: { shardBar: { points: 0, size: 20 }, unopened: 1, odds: cosmeticPacksFull().odds } };
      }
      return undefined;
    });
    render(<CosmeticPacks token={TOKEN} registry={[AURORA]} />);
    await flush();
    expect(screen.getByTestId(T.shardChoice)).toBeInTheDocument();

    fireEvent.click(screen.getByTestId(T.chasePick(AURORA.id)));
    await flush();

    expect(server.to("POST", "/api/cosmetic-packs/chase")[0]?.body).toEqual({ cosmeticId: AURORA.id });
    expect(shardBar()).toHaveTextContent("0 / 20");
    expect(shardBar()).toHaveAttribute("data-state", "resting");
    expect(shardFill()).toBe("scaleX(0)");
    expect(screen.queryByTestId(T.shardChoice)).toBeNull();
  });

  it("under reduced motion the end state shows at once: the full bar, the choice and its moment", async () => {
    writeSettings({ reduceMotion: true });
    await opened();

    // No timer has moved.
    expect(shardBar()).toHaveAttribute("data-state", "resting");
    expect(shardFill()).toBe("scaleX(1)");
    expect(shardBar()).toHaveTextContent("20 / 20");
    expect(screen.queryByTestId(T.shardBurst), "nothing bursts").toBeNull();
    expect(screen.getByTestId(T.shardChoice)).toBeInTheDocument();
    expect(screen.getByTestId(chaseMoment)).toHaveTextContent("The bar is full — pick a chase:");
    expect(screen.getByTestId(`${T.openingBars}-card`)).toHaveAttribute("data-state", "resting");
    expect(screen.getByTestId(`${T.openingBars}-cosmetic`)).toHaveAttribute("data-state", "resting");
    expect(screen.getByTestId(T.cosmeticPackObject)).toHaveAttribute("data-motion", "reduce");
  });
});

/* the stylesheet */

type Block = { prelude: string; body: string };

/** The top-level `prelude { body }` blocks of a stylesheet's comment-free text. */
function blocks(text: string): Block[] {
  const out: Block[] = [];
  let depth = 0;
  let start = 0;
  let open = 0;
  for (let at = 0; at < text.length; at += 1) {
    const char = text[at];
    if (char === "{") {
      if (depth === 0) open = at;
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        out.push({ prelude: text.slice(start, open).trim(), body: text.slice(open + 1, at) });
        start = at + 1;
      }
    }
  }
  return out;
}

function properties(body: string): string[] {
  return body
    .split(";")
    .map((declaration) => declaration.split(":")[0]!.trim())
    .filter((name) => name !== "");
}

/** `shop.css` as text: vitest stubs CSS imports, so the file is read from the test's working directory. */
const sheet: string = (() => {
  for (const candidate of ["src/shop/shop.css", "apps/web/src/shop/shop.css"]) {
    const path = resolve(process.cwd(), candidate);
    if (existsSync(path)) return readFileSync(path, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
  }
  throw new Error(`shop.css not found from ${process.cwd()}`);
})();
const top = blocks(sheet);
const keyframes = top.filter((block) => block.prelude.startsWith("@keyframes"));
const gates = top.filter((block) => block.prelude.startsWith("@media"));
const rules = top.filter((block) => !block.prelude.startsWith("@"));

describe("#1667 shop.css", () => {
  it("its keyframes animate only transform and opacity", () => {
    expect(keyframes.map((block) => block.prelude)).toEqual(["@keyframes shop-pack-fly"]);
    for (const block of keyframes) {
      const animated = blocks(block.body).flatMap((stop) => properties(stop.body));
      expect(animated.length, block.prelude).toBeGreaterThan(0);
      for (const name of animated) expect(["transform", "opacity"], `${block.prelude}: ${name}`).toContain(name);
    }
  });

  it("reduce motion stills the flight: it plays only inside the gate of both switches, its length from SHOP_FLY_MS", () => {
    for (const rule of rules) {
      for (const name of properties(rule.body)) {
        expect(name, `${rule.prelude}: ${name}`).not.toMatch(/^(animation|transition)/);
      }
    }
    expect(gates.map((gate) => gate.prelude)).toEqual(["@media (prefers-reduced-motion: no-preference)"]);

    const gated = blocks(gates[0]!.body);
    expect(gated.map((rule) => rule.prelude)).toEqual([':root:not([data-reduce-motion="true"]) .shop-fly']);
    expect(gated[0]!.body).toMatch(/animation:\s*shop-pack-fly\s+var\(--shop-fly-ms\)/);
  });

  it("the check would catch a layout property in a keyframe", () => {
    const [bad] = blocks("@keyframes x { from { transform: none } to { left: 4px; opacity: 0 } }");
    const animated = blocks(bad!.body).flatMap((stop) => properties(stop.body));
    expect(animated).toContain("left");
  });
});

describe("R1955000 a cosmetic pack's new cosmetic is equipped from its reveal", () => {
  type Slot = OpenedCosmeticPackResponse["slots"][number];

  beforeEach(() => {
    resetCosmeticsForTests();
  });

  afterEach(() => {
    resetCosmeticsForTests();
  });

  const cosmetic = (cosmeticId: string, cardId: string | null): Slot => ({
    rarity: "Epic",
    grant: { kind: "cosmetic", cosmeticId, cosmeticRarity: "Epic", cardId, cardGranted: false },
  });

  /** The panel opened to a reveal of `open`'s answer, the default fixture's when none. */
  async function opened(slots?: Slot[]): Promise<void> {
    cosmeticServer({ open: { status: 200, body: { ...openedCosmeticPack(), ...(slots === undefined ? {} : { slots }) } } });
    render(<CosmeticPacks token={TOKEN} />);
    await screen.findByTestId(T.unopened);
    fireEvent.click(screen.getByTestId(T.open));
    await screen.findByTestId(T.reveal);
  }

  it("R1955000 a per-card cosmetic's Equip writes the pick of the card it was granted on, and the Collection link stays", async () => {
    await opened([
      cosmetic("border-filigree", "core-004"),
      openedCosmeticPack().slots[0]!,
      openedCosmeticPack().slots[2]!,
    ]);
    expect(screen.getByTestId(T.equip(0))).toHaveTextContent("Equip");

    fireEvent.click(screen.getByTestId(T.equip(0)));
    expect(readCardPicks("core-004").border).toBe("border-filigree");
    expect(readCosmetics().border, "the account's pick is not touched").toBe(DEFAULT_CARD_COSMETICS.border);
    expect(screen.getByTestId(T.equip(0))).toHaveTextContent("Equipped");
    expect(screen.getByTestId(T.equip(0))).toBeDisabled();

    // A full art goes on the card it was granted on, which is not the first slot's.
    fireEvent.click(screen.getByTestId(T.equip(1)));
    expect(readCardPicks("core-008").fullArt).toBe(true);
    expect(readCardPicks("core-004").fullArt).toBeUndefined();

    expect(screen.queryByTestId(T.equip(2)), "bars offer none").toBeNull();
    expect(screen.getByTestId(T.equipLink)).toHaveAttribute("href", "/collection?section=cosmetics");
  });

  it("R1955000 an account cosmetic's Equip writes the account's pick of its kind, and a portrait offers none", async () => {
    await opened([
      cosmetic("table-wood", null),
      cosmetic("field-snow", null),
      cosmetic("card-back-tidal", null),
      cosmetic("portrait-timmy", null),
      openedCosmeticPack().slots[2]!,
    ]);
    for (const index of [0, 1, 2]) fireEvent.click(screen.getByTestId(T.equip(index)));
    expect(readCosmetics()).toMatchObject({ table: "table-wood", field: "field-snow", cardBack: "card-back-tidal" });
    for (const index of [0, 1, 2]) expect(screen.getByTestId(T.equip(index))).toHaveTextContent("Equipped");
    expect(screen.queryByTestId(T.equip(3)), "a portrait is a deck's pick").toBeNull();
    expect(screen.queryByTestId(T.equip(4))).toBeNull();
    expect(screen.getByTestId(T.equipLink)).toBeInTheDocument();
  });

  it("R1955000 a cosmetic the store already holds shows Equipped from the reveal", async () => {
    writeCosmetics({ table: "table-wood" });
    await opened();
    expect(screen.getByTestId(T.equip(1))).toHaveTextContent("Equipped");
    expect(screen.getByTestId(T.equip(1))).toBeDisabled();
    expect(screen.getByTestId(T.equip(0))).toHaveTextContent("Equip");
    expect(screen.getByTestId(T.equip(0))).toBeEnabled();
  });
});
