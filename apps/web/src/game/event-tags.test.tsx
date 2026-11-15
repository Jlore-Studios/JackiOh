// R3910000 / BUILD M5-T4: `attackCancelled` and `countered` say "Canceled" and "Countered" on the board's own elements, with
// the effects Off (R200: the effects layer draws nothing there). A tag holds only the event's type (R97), shows while its
// entry plays, and has Chinese. The Deck pile's "Draw limit" is overflow.test.tsx's, the banner's words turn-start-bleed.test.tsx's.

import type { GameEvent, PlayerView } from "@jackioh/shared";
import { act, cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ANIMATIONS } from "./animations.ts";
import { eventTagTestid } from "./EventTag.tsx";
import Game from "./Game.tsx";
import { resetFxSettingsForTests, setFxSettings } from "../fx/settings.ts";
import { resetLocaleForTests, setLocale, t } from "../i18n/i18n.ts";
import { __resetSettingsForTests } from "../settings/store.ts";
import { withCatalog } from "../test/board.tsx";
import { baseView, card, emptySide, unit, withEvents } from "../test/fixtures.ts";
import { setReducedMotion } from "../test/setup.ts";

const TURN: GameEvent = { type: "turnStarted", player: "p1", turn: 5 };

const cancelled: GameEvent = { type: "attackCancelled", attackerId: "u1", targetId: "u6", byInstanceId: "b5" };
const counteredBy = (player: "p1" | "p2", to: "graveyard" | "exile", instanceId = "c1"): GameEvent => ({
  type: "countered",
  player,
  instanceId,
  defId: "core-001",
  byInstanceId: "b5",
  to,
});

function seatView(events: GameEvent[] = [TURN]): PlayerView {
  return withEvents(
    baseView({
      viewer: "p1",
      you: emptySide("p1", { units: [null, unit("p1", { instanceId: "u1" }), null, null, null], hand: [card({ instanceId: "c1" })] }),
      opponent: emptySide("p2", { hand: { count: 4 } }),
    }),
    events,
  );
}

function play(events: GameEvent[]): void {
  const { rerender } = render(withCatalog(<Game view={seatView()} legal={[]} onAction={vi.fn()} />));
  rerender(withCatalog(<Game view={seatView([TURN, ...events])} legal={[]} onAction={vi.fn()} />));
}

function advance(ms: number): void {
  act(() => {
    vi.advanceTimersByTime(ms);
  });
}

function clearStorage(): void {
  try {
    window.localStorage.clear();
  } catch {
    // Storage is optional.
  }
}

beforeEach(() => {
  vi.useFakeTimers();
  clearStorage();
  resetFxSettingsForTests();
  __resetSettingsForTests();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  setReducedMotion(false);
  clearStorage();
  resetFxSettingsForTests();
  __resetSettingsForTests();
  resetLocaleForTests();
});

describe("R3910000 the board's own tags for a canceled attack and a countered play", () => {
  it("R3910000 attackCancelled puts a Canceled tag on the attacker while it plays, with effects Off", () => {
    setFxSettings({ intensity: "off" });
    play([cancelled]);

    const attacker = screen.getByTestId("card-u1");
    expect(attacker).toHaveAttribute("data-animating", "attackCancelled");
    const tag = within(attacker).getByTestId(eventTagTestid("card-u1"));
    expect(tag).toHaveTextContent(/^Canceled$/);
    expect(tag).toHaveClass("cancel-tag");

    advance(ANIMATIONS.attackCancelled.durationMs);
    expect(screen.queryByTestId(eventTagTestid("card-u1"))).toBeNull();
    expect(attacker).not.toHaveAttribute("data-animating");
  });

  it("R3910000 the tag is there with effects on, and only on the attacker", () => {
    play([cancelled]);

    expect(screen.getByTestId(eventTagTestid("card-u1"))).toHaveTextContent(/^Canceled$/);
    expect(screen.getAllByTestId(/^event-tag-/)).toHaveLength(1);
  });

  it("R3910000 countered puts a Countered tag on the viewer's held hand card, with effects Off", () => {
    setFxSettings({ intensity: "off" });
    play([counteredBy("p1", "graveyard")]);

    const held = screen.getByTestId("hand-card-c1");
    expect(held).toHaveAttribute("data-animating", "countered");
    const tag = within(held).getByTestId(eventTagTestid("hand-card-c1"));
    expect(tag).toHaveTextContent(/^Countered$/);
    expect(tag).toHaveClass("counter-tag");

    advance(ANIMATIONS.countered.durationMs);
    expect(screen.queryByTestId(eventTagTestid("hand-card-c1"))).toBeNull();
  });

  it("R3910000 the opponent's countered play shows Countered on the pile it went to", () => {
    setFxSettings({ intensity: "off" });
    // A card no seat holds in view, as the opponent's face-down play is: the event names its pile.
    play([counteredBy("p2", "graveyard", "c9")]);
    expect(within(screen.getByTestId("graveyard-opponent")).getByTestId(eventTagTestid("graveyard-opponent"))).toHaveTextContent(
      /^Countered$/,
    );
    expect(screen.getAllByTestId(/^event-tag-/)).toHaveLength(1);
    cleanup();

    play([counteredBy("p2", "exile", "c9")]);
    expect(within(screen.getByTestId("exile-opponent")).getByTestId(eventTagTestid("exile-opponent"))).toHaveTextContent(/^Countered$/);
    expect(screen.getAllByTestId(/^event-tag-/)).toHaveLength(1);
  });

  it("R3910000 reduced motion starts no entry, so no tag mounts", () => {
    setReducedMotion(true);
    play([cancelled, counteredBy("p1", "graveyard")]);

    expect(screen.queryAllByTestId(/^event-tag-/)).toHaveLength(0);
  });

  it("R3910000 the new words have Chinese", async () => {
    vi.useRealTimers();
    await setLocale("zh-CN");

    expect(t("Canceled")).toBe("已取消");
    expect(t("Countered")).toBe("被反制");
    expect(t("Draw limit")).toBe("抽牌上限");
  });
});
