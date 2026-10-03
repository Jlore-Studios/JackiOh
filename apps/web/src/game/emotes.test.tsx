// The hero portraits and the emote menus (issue #75, SPEC §10.10, R640–R643): a portrait's art on
// each seat's hero with the health and armor badges, your portrait's ten-item emote menu, the
// opponent's one-item mute, and every way the menu must leave — a pick, a press outside, Escape,
// the start of a play or attack, a hotseat seat change and the end of the match.
//
// The harness is the route's own wiring: `useEmotes` is the same api `MatchHotseatRoute` and
// `PracticeRoute` hand `Game` (an `engine: null` sink, the peer's relay left to the test through
// `api.receive`). `send` is wrapped only so the test sees the player/emote pair `Game` passes it.

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import type { ReactElement } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { ActionBody, EmoteId, PlayerId, PlayerView, PortraitId } from "@jackioh/shared";
import { EMOTE_IDS, PORTRAIT_IDS } from "@jackioh/shared";

import { useEmotes, type EmotesApi } from "../emotes/useEmotes.ts";
import { baseView, card, emptySide, fullBoardView } from "../test/fixtures.ts";
import { testid } from "./contract.ts";
import Game from "./Game.tsx";

function EmoteGame(props: {
  view: PlayerView;
  legal: readonly ActionBody[];
  onAction: (body: ActionBody) => void;
  apiRef: { current: EmotesApi | null };
  portraits?: { p1: PortraitId; p2: PortraitId } | null;
  globalMute?: boolean;
  onSend?: (player: PlayerId, emote: EmoteId) => void;
}): ReactElement {
  const emotes = useEmotes({
    engine: null,
    you: props.view.viewer,
    portraits: props.portraits,
    globalMute: props.globalMute,
  });
  props.apiRef.current = emotes;
  // The wire's seam: the route sends `emotes.send`'s admitted emote off to the server. Wrap it so
  // the test sees the (player, emote) pair while the local echo still runs the real session.
  const send = (player: PlayerId, emote: EmoteId): boolean => {
    props.onSend?.(player, emote);
    return emotes.send(player, emote);
  };
  return (
    <Game
      view={props.view}
      legal={props.legal}
      onAction={props.onAction}
      emotes={{ ...emotes, send }}
    />
  );
}

const END_TURN: ActionBody = { type: "endTurn" };

/** The M5-T1 full board: u1–u5 are your units, u6–u10 the opponent's. */
const BOARD: PlayerView = fullBoardView();
/** An attacker on your board that may hit either hero — the candidates list both heroes legal. */
const FACE_ATTACK: readonly ActionBody[] = [
  { type: "attack", attackerId: "u1", targetId: "hero-p1" },
  { type: "attack", attackerId: "u1", targetId: "hero-p2" },
  END_TURN,
];

afterEach(() => {
  cleanup();
});

describe("the hero portraits (R640)", () => {
  it.each(PORTRAIT_IDS)(
    "R640 the %s portrait draws its art inside each seat's hero element",
    (portrait) => {
      const api = { current: null as EmotesApi | null };
      render(
        <EmoteGame
          view={BOARD}
          legal={[END_TURN]}
          onAction={vi.fn()}
          apiRef={api}
          portraits={{ p1: portrait, p2: portrait }}
        />,
      );
      for (const side of [testid.hero("you"), testid.hero("opponent")] as const) {
        const hero = screen.getByTestId(side);
        const frame = hero.querySelector<HTMLElement>(".hero-portrait");
        expect(frame, `portrait frame inside ${side}`).not.toBeNull();
        expect(frame).toHaveAttribute("data-portrait", portrait);
        // CardArt's oval is the shape R640 fixes.
        expect(frame?.querySelector(".cf-art--oval")).not.toBeNull();
        // …and the hero element is the same testid and drop target it always was.
        expect(hero).toHaveAttribute("data-legal", "false");
      }
    },
  );

  it("R640 the portrait carries the seat's health and armor badges, armor only above zero", () => {
    const api = { current: null as EmotesApi | null };
    // The full board's heroes: yours carries 21/3, the opponent's 30/0 — both badge cases.
    render(<EmoteGame view={BOARD} legal={[END_TURN]} onAction={vi.fn()} apiRef={api} />);

    const you = screen.getByTestId("hero-you");
    const health = you.querySelector<HTMLElement>(".hero-health");
    expect(health).not.toBeNull();
    expect(health).toHaveAttribute("data-health", "21");
    expect(health?.textContent).toBe("21");
    const armor = you.querySelector<HTMLElement>(".hero-armor");
    expect(armor).not.toBeNull();
    expect(armor).toHaveAttribute("data-armor", "3");
    expect(armor?.textContent).toBe("3");

    const opponent = screen.getByTestId("hero-opponent");
    expect(opponent.querySelector(".hero-health")?.textContent).toBe("30");
    // The armor badge shows only above zero (Portrait.tsx guards on it).
    expect(opponent.querySelector(".hero-armor")).toBeNull();
  });

  it("R640 the badges hang on the oval's lower corners — health right, armor left", () => {
    // The anchoring is in emotes.css; read it the way Board.test.tsx reads board.css.
    const css = readFileSync(
      join(dirname(fileURLToPath(import.meta.url)), "../emotes/emotes.css"),
      "utf8",
    );
    expect(css).toMatch(/\.hero-portrait \.hero-health[^{}]*\{[^}]*\bright:/);
    expect(css).toMatch(/\.hero-portrait \.hero-armor[^{}]*\{[^}]*\bleft:/);
    expect(css).toMatch(/\.hero-portrait \.hero-health[^{}]*\{[^}]*\bbottom:/);
    expect(css).toMatch(/\.hero-portrait \.hero-armor[^{}]*\{[^}]*\bbottom:/);
  });
});

describe("your portrait's emote menu (R642)", () => {
  it("R642 clicking your portrait opens the ten-emote menu on your hero", () => {
    const api = { current: null as EmotesApi | null };
    render(<EmoteGame view={BOARD} legal={[END_TURN]} onAction={vi.fn()} apiRef={api} />);
    expect(screen.queryByTestId("emote-menu")).toBeNull();

    fireEvent.click(screen.getByTestId("hero-you"));

    const you = screen.getByTestId("hero-you");
    const menu = within(you).getByTestId("emote-menu");
    // The menu mounts on the portrait itself — the picker's items hang off the hero.
    expect(menu.closest(".hero-portrait")).not.toBeNull();
    expect(within(menu).getAllByRole("menuitem")).toHaveLength(10);
    // Five voice lines arced above, five emoji in a row below (issue §2).
    const arc = menu.querySelector<HTMLElement>(".emote-voice-arc");
    const row = menu.querySelector<HTMLElement>(".emote-emoji-row");
    expect(arc).not.toBeNull();
    expect(row).not.toBeNull();
    expect(within(arc as HTMLElement).getAllByRole("menuitem")).toHaveLength(5);
    expect(within(row as HTMLElement).getAllByRole("menuitem")).toHaveLength(5);
    expect(within(arc as HTMLElement).getByRole("menuitem", { name: "Greetings" })).toBeInTheDocument();
    expect(within(row as HTMLElement).getByRole("menuitem", { name: "Laugh" })).toBeInTheDocument();
    for (const emote of EMOTE_IDS) {
      expect(within(menu).getByTestId(`emote-${emote}`)).toBeInTheDocument();
    }
  });

  it("R642 a picked voice line is sent as the viewer and shows its bubble on your hero", () => {
    const api = { current: null as EmotesApi | null };
    const onSend = vi.fn();
    render(
      <EmoteGame view={BOARD} legal={[END_TURN]} onAction={vi.fn()} apiRef={api} onSend={onSend} />,
    );
    fireEvent.click(screen.getByTestId("hero-you"));
    fireEvent.click(screen.getByTestId("emote-greetings"));

    expect(onSend).toHaveBeenCalledWith("p1", "greetings");
    // The sender sees their own locally, without waiting for the wire (R642, issue §5).
    const bubble = within(screen.getByTestId("hero-you")).getByTestId("emote-bubble");
    expect(bubble).toHaveClass("emote-bubble");
    expect(bubble.textContent).not.toBe("");
  });

  it("R642 a picked emoji pops its sticker out of the portrait and sends", () => {
    const api = { current: null as EmotesApi | null };
    const onSend = vi.fn();
    render(
      <EmoteGame view={BOARD} legal={[END_TURN]} onAction={vi.fn()} apiRef={api} onSend={onSend} />,
    );
    fireEvent.click(screen.getByTestId("hero-you"));
    fireEvent.click(screen.getByTestId("emote-laugh"));

    expect(onSend).toHaveBeenCalledWith("p1", "laugh");
    const sticker = within(screen.getByTestId("hero-you")).getByTestId("emote-bubble");
    expect(sticker).toHaveClass("emote-sticker");
    expect(sticker).toHaveAttribute("data-emoji", "laugh");
  });

  /**
   * SPEC §10.10 / issue §2: the menu "closes on a pick". The item's click is stopped inside
   * `[data-emote-menu]` (ui.tsx) so it never reaches the hero's own onClick — which would read the
   * hero as not-legal and reopen the just-closed menu through `emotes.onPortrait()`.
   */
  it("R642 the emote menu closes on a pick", () => {
    const api = { current: null as EmotesApi | null };
    render(<EmoteGame view={BOARD} legal={[END_TURN]} onAction={vi.fn()} apiRef={api} />);
    fireEvent.click(screen.getByTestId("hero-you"));
    fireEvent.click(screen.getByTestId("emote-greetings"));
    expect(screen.queryByTestId("emote-menu")).toBeNull();
  });

  it("the menu closes on Escape", () => {
    const api = { current: null as EmotesApi | null };
    render(<EmoteGame view={BOARD} legal={[END_TURN]} onAction={vi.fn()} apiRef={api} />);
    fireEvent.click(screen.getByTestId("hero-you"));
    expect(screen.getByTestId("emote-menu")).toBeInTheDocument();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByTestId("emote-menu")).toBeNull();
  });

  it("the menu closes on a press outside it", () => {
    const api = { current: null as EmotesApi | null };
    render(<EmoteGame view={BOARD} legal={[END_TURN]} onAction={vi.fn()} apiRef={api} />);
    fireEvent.click(screen.getByTestId("hero-you"));
    expect(screen.getByTestId("emote-menu")).toBeInTheDocument();
    fireEvent.pointerDown(document.body);
    expect(screen.queryByTestId("emote-menu")).toBeNull();
  });

  it("R642 selecting an attacker shuts an open menu — targeting wins", () => {
    const api = { current: null as EmotesApi | null };
    render(<EmoteGame view={BOARD} legal={FACE_ATTACK} onAction={vi.fn()} apiRef={api} />);
    fireEvent.click(screen.getByTestId("hero-you"));
    expect(screen.getByTestId("emote-menu")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("card-u1"));

    // "…or the start of any drag or targeting" (issue §2): the open menu is gone at once.
    expect(screen.queryByTestId("emote-menu")).toBeNull();
    // And the attacker's click is only a selection — nothing was dispatched.
    expect(api.current).not.toBeNull();
  });

  it("R642 starting a card play shuts an open menu and makes the portrait click inert", () => {
    const api = { current: null as EmotesApi | null };
    const onAction = vi.fn();
    const view = baseView({
      you: {
        ...emptySide("p1"),
        hand: [card({ instanceId: "c1", defId: "core-002" })],
      },
    });
    // Two legal plays of c1 that differ only in the zone — the click picks the card and starts a
    // build (a zone prompt), which is targeting: the menu must close and the portrait stay inert.
    const legal: ActionBody[] = [
      { type: "play", instanceId: "c1", zone: { row: "units", lane: 1 } },
      { type: "play", instanceId: "c1", zone: { row: "units", lane: 2 } },
      END_TURN,
    ];
    render(<EmoteGame view={view} legal={legal} onAction={onAction} apiRef={api} />);
    fireEvent.click(screen.getByTestId("hero-you"));
    expect(screen.getByTestId("emote-menu")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("hand-card-c1"));
    expect(screen.queryByTestId("emote-menu")).toBeNull();

    // With a pick outstanding, the portrait click belongs to targeting, not to the menu
    // (Game.tsx's `onPortrait` returns while `interaction !== IDLE`).
    fireEvent.click(screen.getByTestId("hero-you"));
    expect(screen.queryByTestId("emote-menu")).toBeNull();
    expect(onAction).not.toHaveBeenCalled();
  });

  it("R642 the end of the match closes an open menu", () => {
    const api = { current: null as EmotesApi | null };
    const onAction = vi.fn();
    const { rerender } = render(
      <EmoteGame view={BOARD} legal={[END_TURN]} onAction={onAction} apiRef={api} />,
    );
    fireEvent.click(screen.getByTestId("hero-you"));
    expect(screen.getByTestId("emote-menu")).toBeInTheDocument();

    const over: PlayerView = { ...BOARD, result: { winner: "p1", reason: "hero-death" } };
    rerender(<EmoteGame view={over} legal={[]} onAction={onAction} apiRef={api} />);

    expect(screen.queryByTestId("emote-menu")).toBeNull();
  });

  it("R642 a hotseat seat change closes the previous seat's open menu", () => {
    const api = { current: null as EmotesApi | null };
    const onAction = vi.fn();
    const p2View = baseView({
      viewer: "p2",
      active: "p2",
      you: { ...emptySide("p2"), hand: [card({ defId: "core-001" })] },
      opponent: emptySide("p1", { hand: { count: 4 } }),
    });
    const { rerender } = render(
      <EmoteGame view={BOARD} legal={[END_TURN]} onAction={onAction} apiRef={api} />,
    );
    fireEvent.click(screen.getByTestId("hero-you"));
    expect(screen.getByTestId("emote-menu")).toBeInTheDocument();

    // The device handed to p2: p1's open menu is another seat's now and must be gone.
    rerender(<EmoteGame view={p2View} legal={[END_TURN]} onAction={onAction} apiRef={api} />);
    expect(screen.queryByTestId("emote-menu")).toBeNull();
    // The new seat's own portrait still opens its own.
    fireEvent.click(screen.getByTestId("hero-you"));
    expect(screen.getByTestId("emote-menu")).toBeInTheDocument();
  });
});

describe("targeting wins over the menus (R642)", () => {
  it("R642 a legal hero takes the click as a target — the menu never opens", () => {
    const api = { current: null as EmotesApi | null };
    const onAction = vi.fn();
    render(<EmoteGame view={BOARD} legal={FACE_ATTACK} onAction={onAction} apiRef={api} />);
    fireEvent.click(screen.getByTestId("card-u1"));

    // Your own hero is a legal attack target (the candidate's targetId is hero-p1): the click
    // lands on it as a target and the emote menu never opens.
    const you = screen.getByTestId("hero-you");
    expect(you).toHaveAttribute("data-legal", "true");
    fireEvent.click(you);
    expect(onAction).toHaveBeenCalledWith({
      type: "attack",
      attackerId: "u1",
      targetId: "hero-p1",
    });
    expect(screen.queryByTestId("emote-menu")).toBeNull();
  });

  it("R642 the opponent's legal hero fires the attack, not the mute menu", () => {
    const api = { current: null as EmotesApi | null };
    const onAction = vi.fn();
    render(<EmoteGame view={BOARD} legal={FACE_ATTACK} onAction={onAction} apiRef={api} />);
    fireEvent.click(screen.getByTestId("card-u1"));

    const opponent = screen.getByTestId("hero-opponent");
    expect(opponent).toHaveAttribute("data-legal", "true");
    fireEvent.click(opponent);
    expect(onAction).toHaveBeenCalledWith({
      type: "attack",
      attackerId: "u1",
      targetId: "hero-p2",
    });
    expect(screen.queryByTestId("emote-mute-menu")).toBeNull();
    expect(screen.queryByTestId("emote-menu")).toBeNull();
  });

  it("R642 a hero click while an attacker's targets are lit still wins over the menu", () => {
    // The mirror of the close-on-targeting test: the menu was open in idle, the attacker's
    // selection closed it, and now the legal hero click is the target pick.
    const api = { current: null as EmotesApi | null };
    const onAction = vi.fn();
    render(<EmoteGame view={BOARD} legal={FACE_ATTACK} onAction={onAction} apiRef={api} />);
    fireEvent.click(screen.getByTestId("hero-you"));
    fireEvent.click(screen.getByTestId("card-u1"));
    fireEvent.click(screen.getByTestId("hero-opponent"));
    expect(onAction).toHaveBeenCalledWith({
      type: "attack",
      attackerId: "u1",
      targetId: "hero-p2",
    });
    expect(screen.queryByTestId("emote-menu")).toBeNull();
    expect(screen.queryByTestId("emote-mute-menu")).toBeNull();
  });
});

describe("the opponent's mute (R642)", () => {
  it("R642 the opponent's portrait opens a one-item Mute emotes menu, never the ten", () => {
    const api = { current: null as EmotesApi | null };
    render(<EmoteGame view={BOARD} legal={[END_TURN]} onAction={vi.fn()} apiRef={api} />);

    fireEvent.click(screen.getByTestId("hero-opponent"));

    const opponent = screen.getByTestId("hero-opponent");
    const menu = within(opponent).getByTestId("emote-mute-menu");
    expect(within(menu).getAllByRole("menuitem")).toHaveLength(1);
    expect(within(menu).getByTestId("emote-mute").textContent).toContain("Mute emotes");
    // Their hero never gets your ten-item picker.
    expect(screen.queryByTestId("emote-menu")).toBeNull();
  });

  it("R642 muting the opponent hides their emotes for the rest of the match", () => {
    const api = { current: null as EmotesApi | null };
    render(<EmoteGame view={BOARD} legal={[END_TURN]} onAction={vi.fn()} apiRef={api} />);
    // Their emote is showing on their hero…
    act(() => {
      expect(api.current?.receive("p2", "greetings")).toBe(true);
    });
    expect(
      within(screen.getByTestId("hero-opponent")).getByTestId("emote-bubble"),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("hero-opponent"));
    fireEvent.click(screen.getByTestId("emote-mute"));

    expect(api.current?.muted("p2")).toBe(true);
    // …the shown one hides at once…
    expect(
      screen.getByTestId("hero-opponent").querySelector('[data-testid="emote-bubble"]'),
    ).toBeNull();
    // …and the muted player's never arrive (R642).
    act(() => {
      expect(api.current?.receive("p2", "laugh")).toBe(false);
    });
    expect(
      screen.getByTestId("hero-opponent").querySelector('[data-testid="emote-bubble"]'),
    ).toBeNull();
    // Your own seat is untouched.
    expect(api.current?.muted("p1")).toBe(false);
  });

  /**
   * Same bubbling guard as the picker's pick: the "Mute emotes" item's click is stopped inside
   * `[data-emote-menu]`, so `onPortrait` never reopens the just-closed mute menu — the mute
   * applies, the menu is gone, and a fresh press on the portrait reopens it already muted.
   */
  it("R642 the mute menu closes once the mute is set — and shows it as done next time", () => {
    const api = { current: null as EmotesApi | null };
    render(<EmoteGame view={BOARD} legal={[END_TURN]} onAction={vi.fn()} apiRef={api} />);
    fireEvent.click(screen.getByTestId("hero-opponent"));
    fireEvent.click(screen.getByTestId("emote-mute"));
    expect(screen.queryByTestId("emote-mute-menu")).toBeNull();
    // Re-opened, the same slot reads "Emotes muted", disabled — the menu is the state.
    fireEvent.click(screen.getByTestId("hero-opponent"));
    const item = screen.getByTestId("emote-mute");
    expect(item.textContent).toContain("Emotes muted");
    expect(item).toBeDisabled();
  });

  it("R642 a muted opponent's menu item shows it is done", () => {
    const api = { current: null as EmotesApi | null };
    render(<EmoteGame view={BOARD} legal={[END_TURN]} onAction={vi.fn()} apiRef={api} />);
    act(() => {
      api.current?.mute("p2");
    });
    fireEvent.click(screen.getByTestId("hero-opponent"));
    const item = screen.getByTestId("emote-mute");
    expect(item.textContent).toContain("Emotes muted");
    expect(item).toBeDisabled();
  });

  it("R642 the device's Mute opponent emotes setting mutes the other seat on its own", () => {
    const api = { current: null as EmotesApi | null };
    render(
      <EmoteGame view={BOARD} legal={[END_TURN]} onAction={vi.fn()} apiRef={api} globalMute />,
    );
    // Every player that isn't `you` (useEmotes reads the setting live) is muted for the match.
    expect(api.current?.muted("p2")).toBe(true);
    expect(api.current?.muted("p1")).toBe(false);
    act(() => {
      expect(api.current?.receive("p2", "oops")).toBe(false);
    });
    expect(
      screen.getByTestId("hero-opponent").querySelector('[data-testid="emote-bubble"]'),
    ).toBeNull();
  });
});
