// Polish task 7, B25, B26, B27, B33, B43 and B44 (docs/polish/7-mobile-ux.md, Surface S5, S6, S8, S11).
// Set `glow` directly to test board handling rather than its derivation (glow.test.ts).
// Settings use public `writeSettings`; each test starts from defaults.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import type { ActionBody, PlayerView } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import Board from "../../game/Board.tsx";
import Game from "../../game/Game.tsx";
import { NO_HIGHLIGHT, type Highlight } from "../../game/contract.ts";
import { offerDrawControlText } from "../../game/DrawOffer.tsx";
import { hasMovesLeft } from "../../game/glow.ts";
import { usePathname } from "../../net/navigate.ts";
import { __resetSettingsForTests, writeSettings } from "../../settings/index.ts";
import { baseView, card, emptySide, heroPower, pendingFor, unit } from "../fixtures.ts";
import { pressBack } from "../history.ts";
import { stubPhoneBar } from "../phoneBar.ts";
import { at } from "../stubServer.ts";

afterEach(() => {
  cleanup();
  localStorage.clear();
  __resetSettingsForTests();
});

function boardView(over: Partial<PlayerView> = {}): PlayerView {
  return baseView({
    you: emptySide("p1", {
      hero: { health: 30, armor: 0, powers: [heroPower], power: heroPower },
      hand: [
        card({ instanceId: "h1", defId: "core-008" }),
        card({ instanceId: "h2", defId: "core-005" }),
        card({ instanceId: "h3", defId: "core-019" }),
      ],
      units: [unit("p1", { instanceId: "u1" }), null, null, null, null],
    }),
    opponent: emptySide("p2", { hand: { count: 4 } }),
    ...over,
  });
}

function highlight(legal: string[], glow?: string[], selected: string[] = []): Highlight {
  const base = { legal: new Set(legal), selected: new Set(selected) };
  return glow === undefined ? base : { ...base, glow: new Set(glow) };
}

const MOVES_LEFT = highlight(["end-turn", "offer-draw", "concede", "hand-card-h1"], ["hand-card-h1"]);
const NOTHING_LEFT = highlight(["end-turn", "offer-draw", "concede"], ["end-turn"]);

function endTurn(): HTMLElement {
  return screen.getByTestId("end-turn");
}

function slotOf(testid: string): HTMLElement {
  const slot = screen.getByTestId(testid).closest<HTMLElement>(".hand-slot");
  if (slot === null) throw new Error(`${testid} is not inside a .hand-slot`);
  return slot;
}

function inlineVar(element: HTMLElement, name: string): string {
  const fromStyle = element.style.getPropertyValue(name).trim();
  if (fromStyle !== "") return fromStyle;
  const match = new RegExp(`${name}\\s*:\\s*([^;]+)`).exec(element.getAttribute("style") ?? "");
  return match?.[1]?.trim() ?? "";
}

function setConfirmEndTurn(on: boolean): void {
  act(() => {
    writeSettings({ confirmEndTurn: on });
  });
}

describe("B25 confirm end turn asks twice only while a move is left", () => {
  it("B25 with the setting on and a card still playable, the first click arms end-turn and sends nothing", () => {
    setConfirmEndTurn(true);
    const onControl = vi.fn();
    render(<Board view={boardView()} highlight={MOVES_LEFT} onControl={onControl} />);

    fireEvent.click(endTurn());

    expect(onControl).not.toHaveBeenCalled();
    expect(endTurn()).toHaveAttribute("data-confirm", "armed");
    expect(endTurn()).toHaveTextContent("Confirm end turn");
  });

  it("B25 the second click on the armed button ends the turn", () => {
    setConfirmEndTurn(true);
    const onControl = vi.fn();
    render(<Board view={boardView()} highlight={MOVES_LEFT} onControl={onControl} />);

    fireEvent.click(endTurn());
    fireEvent.click(endTurn());

    expect(onControl).toHaveBeenCalledTimes(1);
    expect(onControl).toHaveBeenCalledWith("end-turn");
  });

  it("B25 a new view disarms it, so the next click arms again rather than ending the turn", () => {
    setConfirmEndTurn(true);
    const onControl = vi.fn();
    const { rerender } = render(<Board view={boardView()} highlight={MOVES_LEFT} onControl={onControl} />);
    fireEvent.click(endTurn());
    expect(endTurn()).toHaveAttribute("data-confirm", "armed");

    const next = boardView();
    rerender(
      <Board
        view={{ ...next, you: { ...next.you, mana: { current: 3, max: 4 } } }}
        highlight={MOVES_LEFT}
        onControl={onControl}
      />,
    );

    expect(endTurn()).not.toHaveAttribute("data-confirm");
    expect(endTurn()).not.toHaveTextContent("Confirm end turn");
    fireEvent.click(endTurn());
    expect(onControl).not.toHaveBeenCalled();
    expect(endTurn()).toHaveAttribute("data-confirm", "armed");
  });

  it.each([
    ["a unit that can attack", "card-u1"],
    ["the hero power", "power"],
    ["another playable card", "hand-card-h2"],
  ])("B25 %s counts as a move left", (_label, testid) => {
    setConfirmEndTurn(true);
    const onControl = vi.fn();
    render(
      <Board
        view={boardView()}
        highlight={highlight(["end-turn", testid], [testid])}
        onControl={onControl}
      />,
    );

    fireEvent.click(endTurn());

    expect(onControl).not.toHaveBeenCalled();
    expect(endTurn()).toHaveAttribute("data-confirm", "armed");
  });

  it("B25 with the setting off (the default), one click ends the turn even with moves left", () => {
    const onControl = vi.fn();
    render(<Board view={boardView()} highlight={MOVES_LEFT} onControl={onControl} />);

    fireEvent.click(endTurn());

    expect(onControl).toHaveBeenCalledTimes(1);
    expect(onControl).toHaveBeenCalledWith("end-turn");
    expect(endTurn()).not.toHaveAttribute("data-confirm");
  });

  it("B25 with nothing left but ending the turn, one click ends it even with the setting on", () => {
    setConfirmEndTurn(true);
    const onControl = vi.fn();
    render(<Board view={boardView()} highlight={NOTHING_LEFT} onControl={onControl} />);

    fireEvent.click(endTurn());

    expect(onControl).toHaveBeenCalledTimes(1);
    expect(onControl).toHaveBeenCalledWith("end-turn");
    expect(endTurn()).not.toHaveAttribute("data-confirm");
  });

  it("B25 a highlight with no glow at all is no moves left: one click ends the turn", () => {
    setConfirmEndTurn(true);
    const onControl = vi.fn();
    render(
      <Board view={boardView()} highlight={highlight(["end-turn", "hand-card-h1"])} onControl={onControl} />,
    );

    fireEvent.click(endTurn());

    expect(onControl).toHaveBeenCalledTimes(1);
    expect(endTurn()).not.toHaveAttribute("data-confirm");
  });

  it("B25 a glowing zone or hero is not a move left: one click ends the turn", () => {
    setConfirmEndTurn(true);
    const onControl = vi.fn();
    render(
      <Board
        view={boardView()}
        highlight={highlight(["end-turn", "zone-you-units-2", "hero-opponent"], ["zone-you-units-2", "hero-opponent"])}
        onControl={onControl}
      />,
    );

    fireEvent.click(endTurn());

    expect(onControl).toHaveBeenCalledTimes(1);
    expect(endTurn()).not.toHaveAttribute("data-confirm");
  });

  it("B25 an end-turn the engine did not list never arms and never fires", () => {
    setConfirmEndTurn(true);
    const onControl = vi.fn();
    render(
      <Board view={boardView()} highlight={highlight(["hand-card-h1"], ["hand-card-h1"])} onControl={onControl} />,
    );

    fireEvent.click(endTurn());
    fireEvent.click(endTurn());

    expect(onControl).not.toHaveBeenCalled();
    expect(endTurn()).not.toHaveAttribute("data-confirm");
  });

  it.each<[string, Highlight | undefined]>([
    ["no highlight at all", undefined],
    ["NO_HIGHLIGHT", NO_HIGHLIGHT],
    ["a legal hand card that does not glow", highlight(["hand-card-h1"])],
    ["an empty glow", highlight([], [])],
    ["a glowing end-turn", NOTHING_LEFT],
    ["a glowing zone", highlight(["zone-you-units-1"], ["zone-you-units-1"])],
    ["a glowing hero", highlight(["hero-opponent"], ["hero-opponent"])],
    ["a glowing offer-draw", highlight(["offer-draw"], ["offer-draw"])],
  ])("B25 hasMovesLeft is false for %s", (_label, given) => {
    expect(hasMovesLeft(given)).toBe(false);
  });

  it.each<[string, string]>([
    ["a glowing hand card", "hand-card-h1"],
    ["a glowing card on the field", "card-u1"],
    ["the glowing power", "power"],
  ])("B25 hasMovesLeft is true for %s", (_label, testid) => {
    expect(hasMovesLeft(highlight([testid], [testid]))).toBe(true);
  });
});

describe("B26 each hand card sits in a slot and a tap lifts it", () => {
  it("B26 each of your hand cards sits in its own .hand-slot, in hand order, with --i its index inside a fan of --n", () => {
    render(<Board view={boardView()} highlight={NO_HIGHLIGHT} />);

    const hand = screen.getByTestId("hand-you");
    const slots = [...hand.querySelectorAll<HTMLElement>(".hand-slot")];
    expect(slots).toHaveLength(3);
    slots.forEach((slot, index) => {
      const cards = slot.querySelectorAll('[data-testid^="hand-card-"]');
      expect(cards).toHaveLength(1);
      expect(cards[0]).toHaveAttribute("data-testid", `hand-card-h${index + 1}`);
      expect(inlineVar(slot, "--i")).toBe(String(index));
      expect(slot).not.toHaveAttribute("data-lifted");
    });

    const fan = hand.querySelector<HTMLElement>(".hand-cards");
    expect(fan).not.toBeNull();
    if (fan !== null) expect(inlineVar(fan, "--n")).toBe("3");
    expect(hand).toHaveAttribute("data-count", "3");
    expect(screen.getByTestId("hand-opponent")).toHaveAttribute("data-count", "4");
  });

  it("B26 a click lifts a hand card even when it is not legal, and fires nothing", () => {
    const onClick = vi.fn();
    render(<Board view={boardView()} highlight={NO_HIGHLIGHT} onClick={onClick} />);

    fireEvent.click(screen.getByTestId("hand-card-h2"));

    expect(slotOf("hand-card-h2")).toHaveAttribute("data-lifted", "true");
    expect(slotOf("hand-card-h1")).not.toHaveAttribute("data-lifted");
    expect(onClick).not.toHaveBeenCalled();
  });

  it("B26 a second click lowers it", () => {
    render(<Board view={boardView()} highlight={NO_HIGHLIGHT} />);
    const target = screen.getByTestId("hand-card-h2");

    fireEvent.click(target);
    fireEvent.click(target);

    expect(slotOf("hand-card-h2")).not.toHaveAttribute("data-lifted");
  });

  it("B26 a real tap on the lifted card (pointerdown, then click) lowers it: its own pointerdown is not outside the hand", () => {
    render(<Board view={boardView()} highlight={NO_HIGHLIGHT} />);
    const target = screen.getByTestId("hand-card-h2");
    fireEvent.pointerDown(target);
    fireEvent.click(target);
    expect(slotOf("hand-card-h2")).toHaveAttribute("data-lifted", "true");

    fireEvent.pointerDown(target);
    expect(slotOf("hand-card-h2")).toHaveAttribute("data-lifted", "true");
    fireEvent.click(target);

    expect(slotOf("hand-card-h2")).not.toHaveAttribute("data-lifted");
  });

  it("B26 a pointerdown outside the hand lowers it", () => {
    render(<Board view={boardView()} highlight={NO_HIGHLIGHT} />);
    fireEvent.click(screen.getByTestId("hand-card-h3"));
    expect(slotOf("hand-card-h3")).toHaveAttribute("data-lifted", "true");

    fireEvent.pointerDown(screen.getByTestId("zone-you-units-2"));

    expect(slotOf("hand-card-h3")).not.toHaveAttribute("data-lifted");
  });

  it("B26 the selected hand card is lifted without a click", () => {
    const selected = highlight(["hand-card-h2"], ["hand-card-h2"], ["hand-card-h2"]);
    render(<Board view={boardView()} highlight={selected} />);

    expect(slotOf("hand-card-h2")).toHaveAttribute("data-lifted", "true");
    expect(slotOf("hand-card-h1")).not.toHaveAttribute("data-lifted");
  });

  it("B26 the selected hand card stays lifted through a pointerdown outside the hand and a click on it", () => {
    const onClick = vi.fn();
    const selected = highlight(["hand-card-h2"], ["hand-card-h2"], ["hand-card-h2"]);
    render(<Board view={boardView()} highlight={selected} onClick={onClick} />);

    fireEvent.pointerDown(screen.getByTestId("zone-you-units-2"));
    expect(slotOf("hand-card-h2")).toHaveAttribute("data-lifted", "true");

    fireEvent.click(screen.getByTestId("hand-card-h2"));
    fireEvent.click(screen.getByTestId("hand-card-h2"));

    expect(onClick).toHaveBeenCalledTimes(2);
    expect(slotOf("hand-card-h2")).toHaveAttribute("data-lifted", "true");
  });

  it("B26 the opponent's hand never lifts", () => {
    render(<Board view={boardView()} highlight={NO_HIGHLIGHT} />);
    const opponent = screen.getByTestId("hand-opponent");
    const backs = [...opponent.querySelectorAll<HTMLElement>('[data-face-down="true"]')];
    expect(backs).toHaveLength(4);

    for (const back of backs) fireEvent.click(back);

    expect(opponent.querySelectorAll("[data-lifted]")).toHaveLength(0);
  });
});

describe("B27 the board reflects drag to play and hover previews, live", () => {
  it("B27 by default the board says data-drag=on and your hand says data-hover-preview=on", () => {
    render(<Board view={boardView()} highlight={NO_HIGHLIGHT} />);

    expect(screen.getByTestId("board")).toHaveAttribute("data-drag", "on");
    expect(screen.getByTestId("hand-you")).toHaveAttribute("data-hover-preview", "on");
  });

  it("B27 with drag to play off before render, the board says data-drag=off", () => {
    writeSettings({ dragToPlay: false });
    render(<Board view={boardView()} highlight={NO_HIGHLIGHT} />);

    expect(screen.getByTestId("board")).toHaveAttribute("data-drag", "off");
  });

  it("B27 with hover previews off before render, your hand says data-hover-preview=off", () => {
    writeSettings({ hoverPreviews: false });
    render(<Board view={boardView()} highlight={NO_HIGHLIGHT} />);

    expect(screen.getByTestId("hand-you")).toHaveAttribute("data-hover-preview", "off");
  });

  it("B27 data-drag follows the setting while the board is mounted", () => {
    render(<Board view={boardView()} highlight={NO_HIGHLIGHT} />);

    act(() => {
      writeSettings({ dragToPlay: false });
    });
    expect(screen.getByTestId("board")).toHaveAttribute("data-drag", "off");

    act(() => {
      writeSettings({ dragToPlay: true });
    });
    expect(screen.getByTestId("board")).toHaveAttribute("data-drag", "on");
  });

  it("B27 data-hover-preview follows the setting while the board is mounted", () => {
    render(<Board view={boardView()} highlight={NO_HIGHLIGHT} />);

    act(() => {
      writeSettings({ hoverPreviews: false });
    });
    expect(screen.getByTestId("hand-you")).toHaveAttribute("data-hover-preview", "off");

    act(() => {
      writeSettings({ hoverPreviews: true });
    });
    expect(screen.getByTestId("hand-you")).toHaveAttribute("data-hover-preview", "on");
  });

  it("B27 changing one setting leaves the other attribute alone", () => {
    render(<Board view={boardView()} highlight={NO_HIGHLIGHT} />);

    act(() => {
      writeSettings({ dragToPlay: false });
    });

    expect(screen.getByTestId("hand-you")).toHaveAttribute("data-hover-preview", "on");
  });
});

// B33: safe areas and the 390 px width scan, read off the stylesheets

function css(file: "board.css" | "market.css" | "prompt.css"): string {
  return readFileSync(join(dirname(fileURLToPath(import.meta.url)), "..", "..", "game", file), "utf8");
}

function padsWith(text: string, inset: string): boolean {
  const declaration = /padding(?:-[a-z-]+)?\s*:([^;}]*)/g;
  for (const match of text.matchAll(declaration)) {
    if (new RegExp(`env\\(\\s*${inset}\\b`).test(match[1] ?? "")) return true;
  }
  return false;
}

describe("B33 the board and the prompt pad for the safe areas, and the board declares no width over 390 px", () => {
  it("B33 board.css pads with env(safe-area-inset-bottom), -left and -right", () => {
    const text = css("board.css");

    expect(padsWith(text, "safe-area-inset-bottom")).toBe(true);
    expect(padsWith(text, "safe-area-inset-left")).toBe(true);
    expect(padsWith(text, "safe-area-inset-right")).toBe(true);
  });

  it("B33 prompt.css pads with env(safe-area-inset-bottom)", () => {
    expect(padsWith(css("prompt.css"), "safe-area-inset-bottom")).toBe(true);
  });

  it("B33 board.css declares no px width or min-width over 390", () => {
    const text = css("board.css");
    // Match declarations, not media features or max-width.
    const widths = [...text.matchAll(/(?:^|[\s;{])((?:min-)?width)\s*:\s*(\d+(?:\.\d+)?)px/g)].map((match) => ({
      property: match[1],
      px: Number(match[2]),
    }));

    expect(text).toContain(".board");
    const tooWide = widths.filter((width) => width.px > 390);
    expect(tooWide).toEqual([]);
  });
});

describe("B43 a phone opens the log from the control bar", () => {
  it("B43 log-toggle opens the log over the board (data-log=open) and closes it again", () => {
    render(<Board view={boardView()} highlight={NOTHING_LEFT} />);
    const toggle = screen.getByTestId("log-toggle");
    const board = screen.getByTestId("board");

    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(toggle).toHaveAccessibleName("Show the game log");
    expect(board).not.toHaveAttribute("data-log");

    fireEvent.click(toggle);
    expect(board).toHaveAttribute("data-log", "open");
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(toggle).toHaveAccessibleName("Hide the game log");

    fireEvent.click(toggle);
    expect(board).not.toHaveAttribute("data-log");
  });

  it("B43 board.css hides the toggle by default, shows it on phones, and shows the log while it is open", () => {
    const text = css("board.css");

    expect(text).toMatch(/\n\.log-toggle\s*\{\s*display:\s*none;/);
    const phones = text.slice(text.indexOf("@media (max-width: 600px), (orientation: landscape) and (max-height: 500px)"));
    expect(phones).toMatch(/\.board \.control-bar \.log-toggle\s*\{[^}]*display:\s*inline-grid/);
    expect(phones).toMatch(/\.board\[data-log="open"\] > \.log\s*\{[^}]*display:\s*block/);
  });
});

// #785: the control bar on a phone held upright
describe("#785 a phone held upright keeps the control bar to one row", () => {
  it("board.css hides the board's mute and language buttons on a phone held upright, where the settings gear holds both", () => {
    const text = css("board.css");
    const start = text.indexOf("@media (max-width: 600px) {");
    const portrait = text.slice(start, text.indexOf("@media", start + 1));

    expect(start).toBeGreaterThan(-1);
    expect(portrait).toMatch(/\.board \.control-bar \.audio-toggle,\s*\.board \.control-bar \.language-button--game\s*\{\s*display:\s*none;/);
  });

  it("#815 board.css wraps End turn's label in the one row, takes an empty notices strip out of it, and gives a notice a row of its own", () => {
    const text = css("board.css");
    const start = text.indexOf("@media (max-width: 600px) {");
    const portrait = text.slice(start, text.indexOf("@media", start + 1));

    expect(start).toBeGreaterThan(-1);
    // "End turn" took 63px on one line in a 44px button at 390px, and ran over Offer draw.
    expect(portrait).toMatch(/\.board \.control-bar \.control-end-turn\s*\{[^}]*white-space:\s*normal;/);
    // An empty strip is hidden the visually-hidden way, so its live region still announces.
    const empty = portrait.match(/\.board \.control-bar \.board-notices:not\(:has\(> :not\(:empty\)\)\)\s*\{([^}]*)\}/);
    expect(empty).not.toBeNull();
    expect(empty?.[1]).toMatch(/position:\s*absolute;/);
    expect(empty?.[1]).not.toMatch(/display:\s*none/);
    expect(portrait).toMatch(/\.board \.control-bar:has\(> \.board-notices > :not\(:empty\)\)\s*\{\s*flex-wrap:\s*wrap;/);
  });
});

// #2645: the control bar on a phone holds the match menu

describe("#2645 a phone's control bar holds the match menu", () => {
  let restore: (() => void) | undefined;
  afterEach(() => {
    restore?.();
    restore = undefined;
  });

  function phone(matches: boolean): void {
    restore = stubPhoneBar(matches);
  }

  function bar(): HTMLElement {
    const found = screen.getByTestId("board").querySelector<HTMLElement>(".control-bar");
    if (found === null) throw new Error("no control bar");
    return found;
  }

  const MARKET_VIEW: Partial<PlayerView> = { marketplace: { offers: [], sales: [] } };
  const LEGAL: ActionBody[] = [{ type: "endTurn" }, { type: "offerDraw" }, { type: "concede" }];

  it("#2645 on a phone the bar holds End turn, the log toggle, ⋯ and the gear, and ⋯'s closed menu holds Offer draw and Concede", () => {
    phone(true);
    render(<Board view={boardView()} highlight={NOTHING_LEFT} />);
    const toggle = screen.getByTestId("match-menu");
    const panel = screen.getByTestId("match-menu-panel");

    expect(toggle.parentElement).toBe(bar());
    expect(screen.getByTestId("log-toggle").parentElement).toBe(bar());
    expect(screen.getByTestId("settings-open-game").parentElement).toBe(bar());
    expect(screen.getByTestId("end-turn").closest(".control-bar")).toBe(bar());
    expect(panel.contains(screen.getByTestId("offer-draw"))).toBe(true);
    expect(panel.contains(screen.getByTestId("concede"))).toBe(true);
    expect(panel).not.toBeVisible();
    expect(bar().querySelector(":scope > [data-testid='offer-draw']")).toBeNull();
    expect(bar().querySelector(".audio-toggle")).toBeNull();
    expect(bar().querySelector(".language-button--game")).toBeNull();
    expect(screen.queryByTestId("marketplace-button")).toBeNull();
  });

  it("#2645 ⋯ opens and closes the menu, and Escape closes it and gives ⋯ the focus", () => {
    phone(true);
    render(<Board view={boardView()} highlight={NOTHING_LEFT} />);
    const toggle = screen.getByTestId("match-menu");
    const panel = screen.getByTestId("match-menu-panel");

    expect(toggle).toHaveAccessibleName("Match menu");
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(toggle).toHaveAttribute("aria-controls", panel.id);

    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(panel).toBeVisible();

    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(panel).not.toBeVisible();

    fireEvent.click(toggle);
    fireEvent.keyDown(screen.getByTestId("concede"), { key: "Escape" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(panel).not.toBeVisible();
    expect(toggle).toHaveFocus();
  });

  it("#2645 the browser's Back closes the open menu and gives ⋯ the focus, keeping the match's address", async () => {
    phone(true);
    at("/decks");
    window.history.pushState(null, "", "/practice");
    // Reads the path: its `usePathname` installs `navigate.ts`'s `popstate` listener, where Back's layers are read.
    function Path(): ReactElement {
      return <p data-testid="path">{usePathname()}</p>;
    }
    render(
      <>
        <Board view={boardView()} highlight={NOTHING_LEFT} />
        <Path />
      </>,
    );
    const toggle = screen.getByTestId("match-menu");

    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    await pressBack();
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(screen.getByTestId("match-menu-panel")).not.toBeVisible();
    expect(toggle).toHaveFocus();
    expect(window.location.pathname).toBe("/practice");
  });

  it("#2645 Concede from the menu opens ConfirmConcede and sends no concede, and Cancel gives ⋯ the focus", () => {
    phone(true);
    const onAction = vi.fn<(body: ActionBody) => void>();
    render(<Game view={boardView()} legal={LEGAL} onAction={onAction} />);
    const toggle = screen.getByTestId("match-menu");

    fireEvent.click(toggle);
    fireEvent.click(screen.getByTestId("concede"));
    expect(screen.getByTestId("concede-dialog")).toBeInTheDocument();
    expect(screen.getByTestId("match-menu-panel")).not.toBeVisible();
    expect(onAction).not.toHaveBeenCalledWith({ type: "concede" });

    fireEvent.click(screen.getByTestId("concede-cancel"));
    expect(screen.queryByTestId("concede-dialog")).toBeNull();
    expect(onAction).not.toHaveBeenCalledWith({ type: "concede" });
    expect(toggle).toHaveFocus();

    fireEvent.click(toggle);
    fireEvent.click(screen.getByTestId("concede"));
    fireEvent.click(screen.getByTestId("concede-confirm"));
    expect(onAction).toHaveBeenCalledWith({ type: "concede" });
  });

  it("#2645 Offer draw in the menu sends its control and closes the menu", () => {
    phone(true);
    const onAction = vi.fn<(body: ActionBody) => void>();
    render(<Game view={boardView()} legal={LEGAL} onAction={onAction} />);

    fireEvent.click(screen.getByTestId("match-menu"));
    fireEvent.click(screen.getByTestId("offer-draw"));
    expect(onAction).toHaveBeenCalledWith({ type: "offerDraw" });
    expect(screen.getByTestId("match-menu-panel")).not.toBeVisible();
    expect(screen.getByTestId("match-menu")).toHaveAttribute("aria-expanded", "false");
  });

  it("#2645 ⋯ wears a dot while a declined offer blocks Offer draw, whose row keeps its label and title", () => {
    phone(true);
    const { rerender } = render(<Board view={boardView()} highlight={NOTHING_LEFT} />);
    expect(screen.getByTestId("match-menu")).not.toHaveAttribute("data-dot");

    const blocked = boardView({ drawOfferBlockedTurns: 2 });
    rerender(<Board view={blocked} highlight={NOTHING_LEFT} />);
    const text = offerDrawControlText(blocked);
    const row = screen.getByTestId("offer-draw");

    expect(screen.getByTestId("match-menu")).toHaveAttribute("data-dot", "true");
    expect(text.title).toBeDefined();
    expect(row).toHaveTextContent(text.label);
    expect(row).toHaveAttribute("title", text.title);
  });

  it("#2645 ⋯ carries the glow of the control a tutorial points at while the menu is closed, and the row carries it once it is open", () => {
    phone(true);
    render(<Board view={boardView()} highlight={highlight(["end-turn", "concede"], ["concede"])} />);
    const toggle = screen.getByTestId("match-menu");

    expect(toggle).toHaveAttribute("data-glow", "ready");
    expect(screen.getByTestId("concede")).toHaveAttribute("data-glow", "ready");

    fireEvent.click(toggle);
    expect(toggle).not.toHaveAttribute("data-glow");
    expect(screen.getByTestId("concede")).toHaveAttribute("data-glow", "ready");
  });

  it("#2645 the Marketplace toggle is a button of the bar only while the view carries a marketplace, and it and the menu are never open together", () => {
    phone(true);
    const market = { legal: [], onAction: vi.fn() };
    const { rerender } = render(<Board view={boardView()} highlight={NOTHING_LEFT} market={market} />);
    expect(screen.queryByTestId("marketplace-button")).toBeNull();

    rerender(<Board view={boardView(MARKET_VIEW)} highlight={NOTHING_LEFT} market={market} />);
    const shop = screen.getByTestId("marketplace-button");
    const toggle = screen.getByTestId("match-menu");
    expect(shop.parentElement).toBe(bar());
    expect(shop).toHaveAccessibleName("Marketplace");

    fireEvent.click(shop);
    expect(screen.getByTestId("marketplace-panel").parentElement).toBe(bar());
    fireEvent.click(toggle);
    expect(screen.queryByTestId("marketplace-panel")).toBeNull();
    expect(toggle).toHaveAttribute("aria-expanded", "true");

    fireEvent.click(shop);
    expect(screen.getByTestId("marketplace-panel")).toBeInTheDocument();
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(screen.getByTestId("match-menu-panel")).not.toBeVisible();

    rerender(<Board view={boardView()} highlight={NOTHING_LEFT} market={market} />);
    expect(screen.queryByTestId("marketplace-button")).toBeNull();
    rerender(<Board view={boardView(MARKET_VIEW)} highlight={NOTHING_LEFT} />);
    expect(screen.queryByTestId("marketplace-button")).toBeNull();
  });

  it("#2645 on a desktop Offer draw and Concede stay buttons of the bar and there is no ⋯", () => {
    phone(false);
    render(<Board view={boardView(MARKET_VIEW)} highlight={NOTHING_LEFT} market={{ legal: [], onAction: vi.fn() }} />);

    expect(screen.queryByTestId("match-menu")).toBeNull();
    expect(screen.queryByTestId("match-menu-panel")).toBeNull();
    expect(screen.getByTestId("offer-draw").parentElement).toBe(bar());
    expect(screen.getByTestId("concede").parentElement).toBe(bar());
    expect(screen.getByTestId("marketplace-button").parentElement).toBe(bar());
    expect(bar().querySelector(".audio-toggle")).not.toBeNull();
  });

  it("#2645 board.css keeps Friends out of the phone bars and lays the landscape column in two rows", () => {
    const text = css("board.css");
    const portraitAt = text.indexOf("@media (max-width: 600px) {");
    const portrait = text.slice(portraitAt, text.indexOf("\n@media", portraitAt + 1));
    const narrowAt = text.indexOf("@media (max-width: 380px) {");
    const narrow = text.slice(narrowAt, text.indexOf("\n@media", narrowAt + 1));
    const landscapeAt = text.indexOf("@media (orientation: landscape) and (max-height: 500px) {");
    const landscape = text.slice(landscapeAt, text.indexOf("\n@media", landscapeAt + 1));

    expect(portraitAt).toBeGreaterThan(-1);
    expect(narrowAt).toBeGreaterThan(-1);
    expect(landscapeAt).toBeGreaterThan(-1);
    // The desktop bar's Friends button stays a button of the bar at its natural size.
    expect(text).toMatch(/\.board \.control-bar \.friends-button--game\s*\{\s*flex:\s*0 0 auto;/);
    // On a phone it is a row of the match menu, so nothing in the phone blocks makes room for it, and
    // Offer draw and Concede are never squeezed into wrapping their labels.
    expect(portrait).not.toContain("friends-button--game");
    expect(narrow).not.toContain("friends-button--game");
    expect(portrait).not.toMatch(/control-offer-draw,\s*[^{]*control-concede\s*\{[^}]*white-space:\s*normal/);
    expect(narrow).not.toMatch(/control-offer-draw|control-concede/);
    // The landscape column is two rows: End turn and the gear, then the toggles of the Marketplace, the log and ⋯.
    expect(landscape).toMatch(
      /\.board \.control-bar \.marketplace-toggle,\s*\.board \.control-bar \.log-toggle,\s*\.board \.control-bar \.match-menu-toggle\s*\{\s*order:\s*2;\s*flex:\s*1 1 0;/,
    );
    // A menu's items are never given a `display`: practice.css and duos.css hide them by selector.
    expect(text).toMatch(/\.match-menu\[hidden\]\s*\{\s*display:\s*none;/);
    const items = text.match(/\.board \.control-bar \.match-menu > \.control,\s*\.board \.control-bar \.match-menu > \.friends-button\s*\{([^}]*)\}/);
    expect(items).not.toBeNull();
    expect(items?.[1]).not.toMatch(/display:/);
    expect(items?.[1]).toMatch(/min-height:\s*44px;/);
    expect(items?.[1]).toMatch(/white-space:\s*nowrap;/);
  });

  it("#2645 market.css opens the panel upward from the bar and keeps the floating pill to the sidebar layout, and duos.css hides an empty menu's ⋯", () => {
    const text = css("market.css");
    const base = text.match(/\n\.marketplace-panel\s*\{([^}]*)\}/);
    const sidebarAt = text.indexOf("@media (min-width: 1025px) and (min-height: 501px)");
    const sidebar = text.slice(sidebarAt, text.indexOf("\n}\n", sidebarAt));
    const duos = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "..", "..", "duos", "duos.css"), "utf8");

    expect(base).not.toBeNull();
    expect(base?.[1]).toMatch(/position:\s*absolute;/);
    expect(base?.[1]).toMatch(/bottom:\s*calc\(100% \+ 6px\);/);
    expect(sidebarAt).toBeGreaterThan(-1);
    expect(sidebar).toMatch(/\.board \.marketplace-toggle,\s*\.marketplace-panel\s*\{[^}]*position:\s*fixed;/);
    expect(duos).toMatch(/\.duos-board \.match-menu-toggle:not\(:has\(~ \.match-menu \.friends-button\)\)\s*\{\s*display:\s*none;/);
  });
});

// #990: an online match's rail (Back, chat, the ranks) on a phone
describe("#990 an online match's rail on a phone", () => {
  it("#990 board.css gives a rail a row of its own above the controls, End turn's housing room in the row under it, and the cards the rail's height", () => {
    const text = css("board.css");
    const start = text.indexOf("@media (max-width: 600px) {");
    const portrait = text.slice(start, text.indexOf("@media", start + 1));

    expect(start).toBeGreaterThan(-1);
    // An online match's rail squeezed End turn to its 44px floor and stood the controls 130px tall.
    expect(portrait).toMatch(/\.board \.control-bar:has\(> \.board-rail\)\s*\{\s*flex-wrap:\s*wrap;/);
    expect(portrait).toMatch(/\.board \.control-bar:has\(> \.board-rail\) > \.board-status\s*\{\s*flex:\s*1 1 100%;/);
    expect(portrait).toMatch(/\.board \.control-bar:has\(> \.board-rail\) \.turn-mechanism\s*\{\s*flex-basis:\s*0;/);
    // `:where`, so duos.css's own budget for its taller rail still wins.
    const budget = portrait.match(/\.board:where\(:has\(> \.control-bar > \.board-rail\)\)\s*\{([^}]*)\}/);
    expect(budget).not.toBeNull();
    expect(budget?.[1]).toMatch(/--card-h:[^;]*var\(--board-rail-phone-h\)/);
  });

  it("#990 board.css shows the compact rank line wrapping beside the rail's buttons, not cut, and steps the connected status aside", () => {
    const text = css("board.css");
    const start = text.indexOf("@media (max-width: 600px), (orientation: landscape) and (max-height: 500px)");
    const phones = text.slice(start, text.indexOf("@media", start + 1));

    expect(start).toBeGreaterThan(-1);
    expect(text).toMatch(/\n\.match-ranks__short\s*\{\s*display:\s*none;/);
    expect(phones).toMatch(/\.match-ranks__full\s*\{\s*display:\s*none;/);
    expect(phones).toMatch(/\.match-ranks__short\s*\{\s*display:\s*inline;/);
    // A one-line ellipsis showed "Ranked · Max…" in the rail and cut the opponent's name and both ranks.
    const line = phones.match(/\n {2}\.match-ranks\s*\{([^}]*)\}/);
    expect(line).not.toBeNull();
    expect(line?.[1]).toMatch(/flex:\s*1 1 0;/);
    expect(line?.[1]).not.toMatch(/white-space:\s*nowrap|text-overflow/);
    expect(phones).toMatch(/\.board \.control-bar \.board-status:has\(> \[data-testid="match-status"\]\[data-connection="open"\]\)\s*\{\s*display:\s*none;/);
  });

  it("#990 board.css makes room for the rail under the opponent's hand on a phone held landscape", () => {
    const text = css("board.css");
    const start = text.indexOf("@media (orientation: landscape) and (max-height: 500px) {");
    const landscape = text.slice(start, text.indexOf("@media", start + 1));

    expect(start).toBeGreaterThan(-1);
    expect(landscape).toMatch(/\.match-ranks\s*\{\s*flex-basis:\s*100%;/);
    const empty = landscape.match(/\.board \.control-bar:has\(> \.board-rail\) \.board-notices:not\(:has\(> :not\(:empty\)\)\)\s*\{([^}]*)\}/);
    expect(empty).not.toBeNull();
    expect(empty?.[1]).toMatch(/position:\s*absolute;/);
    expect(empty?.[1]).not.toMatch(/display:\s*none/);
    expect(landscape).toMatch(/\.board \.control-bar:has\(> \.board-rail\) \.language-button--game\s*\{\s*display:\s*none;/);
    expect(landscape).toMatch(/\.board \.control-bar:has\(> \.board-rail\) \.turn-plate\s*\{[^}]*flex-flow:\s*row wrap;/);
  });
});

describe("B44 the picker says which route opened it", () => {
  it("B44 a play's zone pick is data-prompt-source=play, and an engine prompt is data-prompt-source=engine", () => {
    const legal = [
      { type: "play", instanceId: "h1", zone: { row: "units", lane: 2 } },
      { type: "play", instanceId: "h1", zone: { row: "units", lane: 3 } },
      { type: "endTurn" },
    ] as const;
    const { unmount } = render(<Game view={boardView()} legal={[...legal]} onAction={vi.fn()} />);
    fireEvent.click(screen.getByTestId("hand-card-h1"));

    const play = screen.getByTestId("prompt-modal");
    expect(play).toHaveAttribute("data-prompt-kind", "zone");
    expect(play).toHaveAttribute("data-prompt-source", "play");
    unmount();

    const pending = pendingFor("target", [{ key: "hero:p2", label: "Enemy hero", player: "p2" }]);
    render(<Game view={boardView({ pending })} legal={[]} onAction={vi.fn()} />);
    expect(screen.getByTestId("prompt-modal")).toHaveAttribute("data-prompt-source", "engine");
  });

  it("B44 prompt.css makes a play's board pick a slim bar only on a phone held upright with drag to play on", () => {
    const text = css("prompt.css");
    const portrait = text.slice(text.lastIndexOf("@media (max-width: 600px) {"));

    expect(portrait).toContain('.game:has(> .board[data-drag="on"]) .prompt[data-prompt-source="play"]:is(');
    expect(portrait).toMatch(/> :is\(\.prompt-count, \.prompt-zones, \.prompt-list, \.prompt-board-note\) \{\s*display: none;/);
  });

  it("Confirm sticks to the modal's visible bottom, so no menu pushes it off-screen", () => {
    const text = css("prompt.css");

    expect(text).toMatch(/\.prompt-actions \{\s*[^}]*position: sticky;\s*[^}]*bottom: 0;/);
  });
});
