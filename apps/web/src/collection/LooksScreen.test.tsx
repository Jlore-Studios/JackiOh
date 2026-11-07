// R3016001 (#3016): a card's equip and buy screen lists its variants as ladders (R3015007): an owned level's Equip wears
// it, the next level's Buy asks how to pay and asks before it spends, Confirm sends one purchase with the chosen token
// or the Chase Shard, Cancel and Escape send nothing, a locked level names what it needs and has no Buy, Buy is
// disabled and says what is missing when nothing can pay, and a refusal shows the server's words and changes nothing.
// Hovering a level shows it in the three views. "Equip or buy" opens it from the grid's previews.

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { MAX_COPIES } from "@jackioh/engine/config";
import type { CardDef } from "@jackioh/shared";

import { readCardPicks, resetCosmeticsForTests } from "../cards/cosmetics.ts";
import { HOVER_DELAY_MS } from "../cards/inspect/constants.ts";
import { FullArtOwned } from "../cards/fullArt.ts";
import { ownedCardCosmetics, resetOwnedFullArtForTests } from "../cards/ownedFullArt.ts";
import { VERSION_SKETCH_FULL_ART_ANIMATED_ID, VERSION_SKETCH_FULL_ART_ID, VERSION_SKETCH_FULL_ART_SIGNED_ID, VERSION_SKETCH_ID } from "../cards/versions.ts";
import { dialogTestid } from "../game/Dialog.tsx";
import type { TokenBarsResponse } from "../net/api.ts";
import { cosmeticPacksFull, cosmeticPacksInfo, tokenBars } from "../shop/fixtures.ts";
import { settle } from "../test/settle.ts";
import { refusal, stubServer, type Reply } from "../test/stubServer.ts";
import { collectionTestid as T } from "./collection.ts";
import { collectionOwning } from "./fixtures.ts";
import LooksScreen from "./LooksScreen.tsx";
import OwnedCards from "./OwnedCards.tsx";
import type { HeldTokens } from "./TokenOffer.tsx";

const TOKEN = "token-1";
/** Big D-fender, offered the Sketch: Sketch (Rare), its full art (Epic), animated (Legendary) and signed (Mythic). */
const BIG_D = "core-001";

function defOf(id: string): CardDef {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`the catalog has no ${id}`);
  return def;
}

function held(over: Partial<HeldTokens> = {}): HeldTokens {
  return { card: 0, cosmeticRare: 0, cosmeticEpic: 0, cosmeticLegendary: 0, cosmeticPack: 0, ...over };
}

/** What the ledger holds on Big D-fender. */
function owning(...ids: string[]): ReturnType<typeof ownedCardCosmetics> {
  return ownedCardCosmetics({ cards: ids.map((cosmeticId) => ({ cardId: BIG_D, cosmeticId })), account: [] });
}

type Server = { tokens?: Partial<HeldTokens>; shardFull?: boolean; redeem?: Reply; chase?: Reply };

function server({ tokens = {}, shardFull = false, redeem, chase }: Server = {}): ReturnType<typeof stubServer> {
  return stubServer((call) => {
    if (call.method === "GET" && call.path === "/api/tokens") return { status: 200, body: { ...tokenBars(), tokens: held(tokens) } satisfies TokenBarsResponse };
    if (call.method === "GET" && call.path === "/api/cosmetic-packs") return { status: 200, body: shardFull ? cosmeticPacksFull() : cosmeticPacksInfo() };
    if (call.method === "POST" && call.path === "/api/tokens/redeem-cosmetic") return redeem ?? { status: 200, body: { ...tokenBars(), tokens: held(tokens) } };
    if (call.method === "POST" && call.path === "/api/cosmetic-packs/chase") return chase ?? { status: 200, body: cosmeticPacksInfo() };
    return undefined;
  });
}

/** The screen for Big D-fender, the ledger holding `owned`, the account holding `copies`. */
async function screenOf(owned: ReturnType<typeof owning>, copies: number = MAX_COPIES): Promise<void> {
  render(
    <FullArtOwned owned={owned}>
      <LooksScreen def={defOf(BIG_D)} copies={copies} token={TOKEN} onClose={() => {}} />
    </FullArtOwned>,
  );
  await settle();
}

const row = (id: string): HTMLElement => screen.getByTestId(T.looksRung(id));

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
  resetOwnedFullArtForTests();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  localStorage.clear();
  resetCosmeticsForTests();
  resetOwnedFullArtForTests();
});

describe("R3016001 a card's equip and buy screen", () => {
  it("R3016001 the ladders list each level owned, next or locked: Equip, Buy, or what it needs", async () => {
    server({ tokens: { cosmeticEpic: 1 } });
    await screenOf(owning(VERSION_SKETCH_ID));
    expect(row(VERSION_SKETCH_ID)).toHaveAttribute("data-owned", "true");
    expect(within(row(VERSION_SKETCH_ID)).getByTestId(T.looksEquip(VERSION_SKETCH_ID))).toBeTruthy();
    expect(within(row(VERSION_SKETCH_ID)).queryByTestId(T.looksBuy(VERSION_SKETCH_ID))).toBeNull();

    expect(row(VERSION_SKETCH_FULL_ART_ID)).toHaveAttribute("data-next", "true");
    expect(within(row(VERSION_SKETCH_FULL_ART_ID)).getByTestId(T.looksBuy(VERSION_SKETCH_FULL_ART_ID))).toBeEnabled();
    expect(within(row(VERSION_SKETCH_FULL_ART_ID)).queryByTestId(T.looksEquip(VERSION_SKETCH_FULL_ART_ID))).toBeNull();

    // A level past the next has no Buy and no Equip, and names the level it needs first.
    for (const id of [VERSION_SKETCH_FULL_ART_ANIMATED_ID, VERSION_SKETCH_FULL_ART_SIGNED_ID]) {
      expect(within(row(id)).queryByTestId(T.looksBuy(id)), id).toBeNull();
      expect(within(row(id)).queryByTestId(T.looksEquip(id)), id).toBeNull();
    }
    expect(screen.getByTestId(T.looksNeeds(VERSION_SKETCH_FULL_ART_ANIMATED_ID))).toHaveTextContent("Needs Sketch full art");
    expect(screen.getByTestId(T.looksNeeds(VERSION_SKETCH_FULL_ART_SIGNED_ID))).toHaveTextContent("Needs Sketch full art, animated");
  });

  it("R3016001 an owned level's Equip wears it on this card and reads Equipped", async () => {
    server();
    await screenOf(owning(VERSION_SKETCH_ID, VERSION_SKETCH_FULL_ART_ID));
    const equip = screen.getByTestId(T.looksEquip(VERSION_SKETCH_FULL_ART_ID));
    expect(equip).toHaveAttribute("aria-pressed", "false");
    fireEvent.click(equip);
    expect(readCardPicks(BIG_D)).toEqual({ fullArt: VERSION_SKETCH_FULL_ART_ID });
    expect(screen.getByTestId(T.looksEquip(VERSION_SKETCH_FULL_ART_ID))).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByTestId(T.looksEquip(VERSION_SKETCH_FULL_ART_ID))).toHaveTextContent("Equipped");
    expect(row(VERSION_SKETCH_FULL_ART_ID)).toHaveAttribute("data-equipped", "true");
  });

  it("R3016001 Buy asks how to pay, and Confirm sends one purchase with the chosen token", async () => {
    const stub = server({ tokens: { cosmeticLegendary: 2 }, shardFull: true });
    await screenOf(owning(VERSION_SKETCH_ID, VERSION_SKETCH_FULL_ART_ID));
    const id = VERSION_SKETCH_FULL_ART_ANIMATED_ID;
    fireEvent.click(screen.getByTestId(T.looksBuy(id)));
    const dialog = screen.getByRole("alertdialog");
    expect(dialog).toHaveTextContent("Buy Sketch full art, animated? This spends 1 Legendary token.");
    // R3221000: a dialog over the screen's, Cancel first and the close (no ✕), Confirm last.
    expect(screen.getByTestId(T.looksDialog(id))).toBe(dialog);
    expect([...dialog.querySelectorAll<HTMLElement>(".dialog__actions > button")].map((button) => button.dataset.testid)).toEqual([
      T.looksCancel(id),
      T.looksConfirm(id),
    ]);
    expect(within(dialog).queryByTestId(dialogTestid.close)).toBeNull();
    // A Legendary chase: the Legendary token, or the full Chase Shard bar.
    expect(within(dialog).getByTestId(T.looksPay(id, "Legendary"))).toHaveTextContent("1 Legendary token (2 held)");
    expect(within(dialog).getByTestId(T.looksPay(id, "Legendary"))).toHaveAttribute("aria-pressed", "true");
    expect(within(dialog).getByTestId(T.looksPay(id, "chase"))).toHaveTextContent("Your full Chase Shard bar");
    expect(stub.to("POST", "/api/tokens/redeem-cosmetic")).toHaveLength(0);

    fireEvent.click(screen.getByTestId(T.looksConfirm(id)));
    await settle();
    const sent = stub.to("POST", "/api/tokens/redeem-cosmetic");
    expect(sent).toHaveLength(1);
    expect(sent[0]?.body).toEqual({ token: "Legendary", cosmeticId: id, cardId: BIG_D });
    expect(stub.to("POST", "/api/cosmetic-packs/chase")).toHaveLength(0);
    expect(screen.queryByRole("alertdialog")).toBeNull();
  });

  it("R3016001 the Chase Shard pays for a chase when it is chosen", async () => {
    const stub = server({ tokens: { cosmeticLegendary: 1 }, shardFull: true });
    await screenOf(owning(VERSION_SKETCH_ID, VERSION_SKETCH_FULL_ART_ID));
    const id = VERSION_SKETCH_FULL_ART_ANIMATED_ID;
    fireEvent.click(screen.getByTestId(T.looksBuy(id)));
    fireEvent.click(screen.getByTestId(T.looksPay(id, "chase")));
    expect(screen.getByRole("alertdialog")).toHaveTextContent("Buy Sketch full art, animated? This spends your full Chase Shard bar.");
    fireEvent.click(screen.getByTestId(T.looksConfirm(id)));
    await settle();
    const sent = stub.to("POST", "/api/cosmetic-packs/chase");
    expect(sent).toHaveLength(1);
    expect(sent[0]?.body).toEqual({ cosmeticId: id, cardId: BIG_D });
    expect(stub.to("POST", "/api/tokens/redeem-cosmetic")).toHaveLength(0);
  });

  it("R3016001 Cancel and Escape send nothing and give the focus back to Buy", async () => {
    const stub = server({ tokens: { cosmeticEpic: 1 } });
    await screenOf(owning(VERSION_SKETCH_ID));
    const id = VERSION_SKETCH_FULL_ART_ID;
    fireEvent.click(screen.getByTestId(T.looksBuy(id)));
    expect(screen.getByTestId(T.looksCancel(id))).toHaveFocus();
    fireEvent.click(screen.getByTestId(T.looksCancel(id)));
    expect(screen.queryByRole("alertdialog")).toBeNull();
    expect(screen.getByTestId(T.looksBuy(id))).toHaveFocus();

    fireEvent.click(screen.getByTestId(T.looksBuy(id)));
    fireEvent.keyDown(screen.getByTestId(T.looksCancel(id)), { key: "Escape" });
    expect(screen.queryByRole("alertdialog")).toBeNull();
    // Escape answered the question alone: the screen stays.
    expect(screen.getByTestId(T.looks)).toBeTruthy();
    expect(screen.getByTestId(T.looksBuy(id))).toHaveFocus();
    await settle();
    expect(stub.to("POST", "/api/tokens/redeem-cosmetic")).toHaveLength(0);
    expect(stub.to("POST", "/api/cosmetic-packs/chase")).toHaveLength(0);
  });

  // Each case is its own test: setup clears the kept reads between tests (test/setup.ts), so one case's Chase Shard
  // bar or tokens can't answer the next one's first read.
  it("R3016001 Buy is disabled and says no held token covers a level that is no chase", async () => {
    // No token held covers the Sketch's full art, an Epic, and it is no chase.
    server({ tokens: { cosmeticRare: 3 }, shardFull: true });
    await screenOf(owning(VERSION_SKETCH_ID));
    expect(screen.getByTestId(T.looksBuy(VERSION_SKETCH_FULL_ART_ID))).toBeDisabled();
    expect(screen.getByTestId(T.looksMissing(VERSION_SKETCH_FULL_ART_ID))).toHaveTextContent("No token you hold covers it");
  });

  it("R3016001 Buy is disabled on a chase with no token and the Chase Shard bar not full", async () => {
    server({ shardFull: false });
    await screenOf(owning(VERSION_SKETCH_ID, VERSION_SKETCH_FULL_ART_ID));
    expect(screen.getByTestId(T.looksBuy(VERSION_SKETCH_FULL_ART_ANIMATED_ID))).toBeDisabled();
    expect(screen.getByTestId(T.looksMissing(VERSION_SKETCH_FULL_ART_ANIMATED_ID))).toHaveTextContent(
      "No token you hold covers it, and the Chase Shard bar isn't full",
    );
  });

  it("R3016001 Buy is disabled on a card short of MAX_COPIES, whatever is held", async () => {
    server({ tokens: { cosmeticLegendary: 9, cosmeticEpic: 9 }, shardFull: true });
    await screenOf(owning(VERSION_SKETCH_ID), MAX_COPIES - 1);
    expect(screen.getByTestId(T.looksBuy(VERSION_SKETCH_FULL_ART_ID))).toBeDisabled();
    expect(screen.getByTestId(T.looksMissing(VERSION_SKETCH_FULL_ART_ID))).toHaveTextContent(
      `Collect ${String(MAX_COPIES)} copies of this card to buy its looks`,
    );
  });

  it("R3016001 a refusal shows the server's words and changes nothing", async () => {
    const stub = server({ tokens: { cosmeticEpic: 1 }, redeem: refusal(400, "bad_request", "that level needs every level below it owned first") });
    await screenOf(owning(VERSION_SKETCH_ID));
    const id = VERSION_SKETCH_FULL_ART_ID;
    fireEvent.click(screen.getByTestId(T.looksBuy(id)));
    fireEvent.click(screen.getByTestId(T.looksConfirm(id)));
    await waitFor(() => {
      expect(screen.getByTestId(T.looksError(id))).toHaveTextContent("that level needs every level below it owned first");
    });
    expect(stub.to("POST", "/api/tokens/redeem-cosmetic")).toHaveLength(1);
    // Still the next level, still for sale; nothing worn.
    expect(row(id)).toHaveAttribute("data-owned", "false");
    expect(screen.getByTestId(T.looksBuy(id))).toBeEnabled();
    expect(readCardPicks(BIG_D)).toEqual({});
  });

  it("R3016001 hovering a level shows it in the three views", async () => {
    server();
    await screenOf(owning(VERSION_SKETCH_ID));
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    fireEvent.pointerEnter(row(VERSION_SKETCH_FULL_ART_ID), { pointerType: "mouse" });
    act(() => {
      vi.advanceTimersByTime(HOVER_DELAY_MS);
    });
    const panel = screen.getByTestId(T.lookHover);
    expect(panel).toHaveAttribute("data-level", VERSION_SKETCH_FULL_ART_ID);
    for (const view of ["card", "hand", "board"]) expect(within(panel).getByTestId(T.lookView(view))).toBeTruthy();
    // The level is shown though not owned: the Sketch's full art prints its full-art face.
    expect(within(panel).getByTestId(T.lookView("card")).querySelector(".cf")).toHaveAttribute("data-version", VERSION_SKETCH_FULL_ART_ID);
    // And no pick is written.
    expect(readCardPicks(BIG_D)).toEqual({});
  });

  it("R3016001 hovering a rung of the card view's ladders shows that level in the three views", () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    render(
      <FullArtOwned owned={owning(VERSION_SKETCH_ID)}>
        <OwnedCards collection={collectionOwning([BIG_D])} defs={[defOf(BIG_D)]} />
      </FullArtOwned>,
    );
    fireEvent.click(screen.getByTestId(T.setToggle("Core")));
    fireEvent.click(screen.getByTestId(T.card(BIG_D)));
    fireEvent.pointerEnter(screen.getByTestId(T.rung(VERSION_SKETCH_FULL_ART_ID)), { pointerType: "mouse" });
    act(() => {
      vi.advanceTimersByTime(HOVER_DELAY_MS);
    });
    const panel = screen.getByTestId(T.lookHover);
    expect(panel).toHaveAttribute("data-level", VERSION_SKETCH_FULL_ART_ID);
    expect(within(panel).getByTestId(T.lookView("board")).querySelector(".cf--minion")).not.toBeNull();
    // A press on the rung closes it, as on the grid.
    fireEvent.pointerDown(screen.getByTestId(T.rung(VERSION_SKETCH_FULL_ART_ID)), { pointerType: "mouse", button: 0, pointerId: 1 });
    expect(screen.queryByTestId(T.lookHover)).toBeNull();
  });

  it("R3016001 Equip or buy on the grid's hover preview and fullscreen preview opens the screen", async () => {
    server();
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    render(<OwnedCards collection={collectionOwning([BIG_D])} defs={[defOf(BIG_D)]} token={TOKEN} />);
    fireEvent.click(screen.getByTestId(T.setToggle("Core")));
    const tile = screen.getByTestId(T.card(BIG_D));
    fireEvent.pointerEnter(tile, { pointerType: "mouse" });
    act(() => {
      vi.advanceTimersByTime(HOVER_DELAY_MS);
    });
    fireEvent.click(within(screen.getByTestId(T.lookHover)).getByTestId(T.looksOpen));
    expect(screen.queryByTestId(T.lookHover)).toBeNull();
    expect(screen.getByTestId(T.looks)).toHaveAttribute("data-card", BIG_D);
    fireEvent.click(screen.getByTestId(T.looksClose));
    expect(screen.queryByTestId(T.looks)).toBeNull();
    // The hover panel's button is gone, so the focus goes back to the card.
    expect(tile).toHaveFocus();

    // From the fullscreen preview it opens over it, and Escape closes it alone.
    fireEvent.keyDown(tile, { key: "i" });
    // A pressed button takes the focus, as a browser's click gives it.
    const opener = within(screen.getByTestId(T.preview)).getByTestId(T.looksOpen);
    act(() => {
      opener.focus();
    });
    fireEvent.click(opener);
    expect(screen.getByTestId(T.looks)).toBeTruthy();
    // R3221000: its scrim comes after the preview's, at the same layer, so it draws over it.
    expect(
      screen.getByTestId(T.preview).parentElement!.compareDocumentPosition(screen.getByTestId(T.looks).parentElement!) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    fireEvent.keyDown(screen.getByTestId(T.looksClose), { key: "Escape" });
    expect(screen.queryByTestId(T.looks)).toBeNull();
    expect(screen.getByTestId(T.preview)).toBeTruthy();
    expect(screen.getByTestId(T.looksOpen)).toHaveFocus();
  });
});
