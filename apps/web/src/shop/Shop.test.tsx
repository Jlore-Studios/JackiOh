// The Jlore shop and the cosmetic packs (#1309, #1667), from fixture responses: each pack kind a
// PackObject on the shelf, Buy pressing it and, on the answer, a copy flying into the count and the
// wallet announced; a short wallet disabling every buy with its shortfall, a refused buy changing
// nothing, a bought pack moving the wallet and the count; the cosmetic pack as an object above Open and
// no odds; an opening's reveal naming every slot, with the token bars it moved; the Chase Shard bar
// filling from before to after and, at full, bursting and revealing the chase choice; reduce motion
// showing every end state at once; each panel telling the page what it changed and reading again when
// told. The tier filters live beside them as pure tests over crafted registries, since the shipped
// registry holds no Legendary or Mythic a token could reach. The stylesheet is read as text, as
// `economy/FillBar.test.tsx` reads its own, because Vitest stubs CSS imports.

import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { GLOBAL_PACK_NAME } from "@jackioh/server-config";

import { REVEAL_STAGGER_MS, segmentMs } from "../economy/bars.ts";
import { announceWallet } from "../economy/wallet.ts";
import { resetFxSettingsForTests } from "../fx/settings.ts";
import type { BoughtPackResponse } from "../net/api.ts";
import { packsInfo } from "../packs/fixtures.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
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
  window.localStorage.clear();
});

describe("#1309 the shop model", () => {
  it("a short wallet owes the difference, a covered price owes nothing", () => {
    expect(shortfallOf(100, 37)).toBe(63);
    expect(shortfallOf(100, 100)).toBe(0);
    expect(shortfallOf(100, 250)).toBe(0);
  });

  it("a cosmetic slot says what it became, a spent one the bars it filled", () => {
    const [foil, , bars] = openedCosmeticPack().slots;
    expect(cosmeticSlotOutcome(foil!)).toBe("New cosmetic: Mr. Vanilla full art");
    expect(cosmeticSlotOutcome(bars!)).toContain("Duplicate:");
    expect(cosmeticSlotOutcome(bars!)).toContain("card token bar");
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
    // The read is out: the button waits for it, and a second press sends nothing.
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

describe("#1309 the cosmetic packs", () => {
  it("the shard bar's choice appears only when full", async () => {
    cosmeticServer();
    render(<CosmeticPacks token={TOKEN} />);
    expect(await screen.findByTestId(T.shardBar)).toHaveTextContent("19 / 20");
    expect(screen.queryByTestId(T.shardChoice)).toBeNull();
    expect(screen.queryByTestId(T.shardEmpty)).toBeNull();
    cleanup();

    // Full, with every Legendary and Mythic already owned (the shipped registry's are all
    // starters): the empty sentence, and still no choice.
    cosmeticServer({ packs: { status: 200, body: cosmeticPacksFull() } });
    render(<CosmeticPacks token={TOKEN} />);
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
    expect(await screen.findByText("No unopened packs.")).toBeInTheDocument();
    expect(screen.queryByTestId(T.cosmeticPackObject)).toBeNull();
    expect(screen.queryByTestId(T.open)).toBeNull();
  });

  it("an opened cosmetic pack mounts the token bars it moved in its reveal, and none it left alone", async () => {
    cosmeticServer();
    render(<CosmeticPacks token={TOKEN} />);
    await screen.findByTestId(T.unopened);
    expect(screen.queryByTestId(T.openingBars)).toBeNull();

    fireEvent.click(screen.getByTestId(T.open));
    await screen.findByTestId(T.reveal);

    const bars = screen.getByTestId(T.openingBars);
    expect(bars).toContainElement(screen.getByTestId(`${T.openingBars}-card`));
    expect(screen.getByTestId(`${T.openingBars}-card`)).toHaveTextContent("Card token bar");
    expect(screen.getByTestId(`${T.openingBars}-card`)).toHaveTextContent("7 / 12");
    expect(screen.getByTestId(`${T.openingBars}-cosmetic`)).toHaveTextContent("Cosmetic token bar");
    expect(screen.getByTestId(`${T.openingBars}-cosmetic`)).toHaveTextContent("9 / 30");
    cleanup();

    const still = { card: { before: 3, after: 3, size: 12, pays: 0 }, cosmetic: { before: 5, after: 5, size: 30, pays: 0 } };
    cosmeticServer({
      open: { status: 200, body: { ...openedCosmeticPack(), bars: { ...still, cosmeticPaid: [] } } },
    });
    render(<CosmeticPacks token={TOKEN} />);
    await screen.findByTestId(T.unopened);
    fireEvent.click(screen.getByTestId(T.open));
    await screen.findByTestId(T.reveal);
    expect(screen.queryByTestId(T.openingBars)).toBeNull();
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

  it("R1579000 an opened cosmetic pack links to the Collection's cosmetics", async () => {
    cosmeticServer();
    render(<CosmeticPacks token={TOKEN} />);
    await screen.findByTestId(T.unopened);
    expect(screen.queryByTestId(T.equipLink)).toBeNull();

    fireEvent.click(screen.getByTestId(T.open));
    await screen.findByTestId(T.reveal);

    expect(screen.getByTestId(T.equipLink)).toHaveAttribute("href", "/collection?section=cosmetics");
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
    // The read is out: the button waits for it, and a second press sends nothing.
    expect(screen.getByTestId(T.packsRetry)).toBeDisabled();
    fireEvent.click(screen.getByTestId(T.packsRetry));

    expect(await screen.findByTestId(T.unopened)).toHaveTextContent("Cosmetic pack: 2 unopened");
    expect(screen.queryByTestId(T.packsLoadError)).toBeNull();
    expect(server.to("GET", "/api/cosmetic-packs")).toHaveLength(2);
  });
});

/** The one chase the registry given to the panel offers: a Legendary account cosmetic the fixture's account lacks. */
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
  async function opened(over: { packs?: Reply } = {}): Promise<void> {
    cosmeticServer(over);
    render(<CosmeticPacks token={TOKEN} registry={[AURORA]} />);
    await flush();
    fireEvent.click(screen.getByTestId(T.open));
    await flush();
  }

  it("is a FillBar labelled Chase Shard, at rest on what the panel read", async () => {
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
      open: { status: 200, body: { ...openedCosmeticPack(), shardBar: { points: 6, size: 20 } } },
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

  it("a full bar holds: another opening leaves it full with no burst, and its choice stays", async () => {
    cosmeticServer({ packs: { status: 200, body: cosmeticPacksFull() } });
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

/* ------------------------------------------------------------------------ the stylesheet --- */

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

/** The property names a declaration list holds. */
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
