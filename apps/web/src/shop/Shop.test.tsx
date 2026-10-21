// The Jlore shop and the cosmetic packs (#1309), from fixture responses: a short wallet
// disabling every buy with its shortfall, a refused buy changing nothing, a bought pack moving
// the wallet and the count, the shard choice only when full, and an opening's reveal naming
// every slot, and each panel telling the page what it changed and reading again when told. The tier filters live beside them as pure tests over crafted registries, since the
// shipped registry holds no Legendary or Mythic a token could reach.

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { GLOBAL_PACK_NAME } from "@jackioh/server-config";

import type { BoughtPackResponse } from "../net/api.ts";
import { packsInfo } from "../packs/fixtures.ts";
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
  chaseCandidates,
  cosmeticSlotOutcome,
  quantitiesOf,
  shopTestid as T,
  shortfallOf,
  type RegistryCosmetic,
} from "./shop.ts";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
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

  it("R1339000 the cosmetic packs panel shows the server's odds for each slot and the guaranteed one", async () => {
    cosmeticServer();
    render(<CosmeticPacks token={TOKEN} />);
    expect(await screen.findByTestId(T.odds)).toHaveTextContent(
      "Each slot: Common 50%, Rare 30%, Epic 15%, Legendary 4%, Mythic 1%.",
    );
    expect(screen.getByTestId(T.flooredOdds)).toHaveTextContent(
      "Guaranteed slot: Common 0%, Rare 0%, Epic 90%, Legendary 8%, Mythic 2%.",
    );
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
