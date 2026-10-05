// Issue #124: a burst of casts outlasts one burst budget (Jogg's Box casts ten, each with its own
// CAST_BUDGET_MS), so the showcase's gate must not time out while the runner is still reaching casts:
// every entry the runner starts restarts the gate's wait, and each cast still replaces in as its own
// entry starts. With no runner progress at all, the gate still lets go after SHOWCASE_GATE_MAX_MS.

import { CATALOG } from "@jackioh/cards";
import type { GameEvent, PlayerView } from "@jackioh/shared";
import { act, cleanup, render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { resetFxSettingsForTests } from "../../fx/settings.ts";
import { __resetSettingsForTests } from "../../settings/store.ts";
import { baseView, withEvents } from "../../test/fixtures.ts";
import { createAnimationQueue, type AnimationQueue } from "../animations.ts";
import { CatalogContext, lookupFromDefs } from "../catalog.ts";
import CardShowcase from "./CardShowcase.tsx";
import { MULTICAST_TEXT, SHOWCASE_GATE_MAX_MS, SHOWCASE_HOLD_MS, showcaseTestid as T } from "./constants.ts";

const lookup = lookupFromDefs(CATALOG);
const BOX = "classicplus-047";
const BOX_NAME = CATALOG[BOX]?.name ?? "";
const SPELL = "core-010";
const TURN: GameEvent = { type: "turnStarted", player: "p2", turn: 4 };

const box = (): GameEvent => ({ type: "cardPlayed", player: "p2", instanceId: "box", defId: BOX, costPaid: 4 });
const cast = (id: string): GameEvent => ({ type: "cardPlayed", player: "p2", instanceId: id, defId: SPELL, costPaid: 0 });
const resolved = (id: string): GameEvent => ({ type: "cardResolved", player: "p2", instanceId: id, defId: SPELL, permanent: false, costPaid: 0 });

function withCatalog(node: ReactElement): ReactElement {
  return <CatalogContext.Provider value={lookup}>{node}</CatalogContext.Provider>;
}

/** Mounts on a first view, then gives it `view` (with `events` after TURN). */
function mountThen(events: GameEvent[], options: { queue?: AnimationQueue; view?: PlayerView } = {}) {
  const base = options.view ?? baseView();
  const tree = (view: PlayerView) => withCatalog(<CardShowcase view={view} {...(options.queue === undefined ? {} : { queue: options.queue })} />);
  const utils = render(tree(withEvents(base, [TURN])));
  const next = withEvents(base, [TURN, ...events]);
  utils.rerender(tree(next));
  return { next };
}

function manual(): { queue: AnimationQueue; tick: () => void } {
  const pending: (() => void)[] = [];
  const queue = createAnimationQueue({ schedule: (fn) => pending.push(fn), reducedMotion: false });
  return { queue, tick: () => act(() => pending.shift()?.()) };
}

function advance(ms: number): void {
  act(() => {
    vi.advanceTimersByTime(ms);
  });
}

/** The ordinal badge showing now, or null when nothing is held up. */
function ordinalNow(): string | null {
  return screen.queryByTestId(T.ordinal)?.getAttribute("data-ordinal") ?? null;
}

beforeEach(() => {
  vi.useFakeTimers();
  resetFxSettingsForTests();
  __resetSettingsForTests();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  resetFxSettingsForTests();
  __resetSettingsForTests();
  try {
    window.localStorage.clear();
  } catch {
    // Storage is optional.
  }
});

describe("issue #124 the gate outlasts a burst the runner is still working through", () => {
  it("each cast of a burst longer than the gate still replaces in as its entry starts", () => {
    const { queue, tick } = manual();
    const events = [box(), cast("c1"), resolved("c1"), cast("c2"), resolved("c2"), cast("c3"), resolved("c3"), cast("c4"), resolved("c4")];
    const { next } = mountThen(events, { queue });
    // The box itself is the opponent's ordinary play: up at once, not gated.
    expect(screen.getByTestId(T.root)).toHaveAttribute("data-showcase", "played");
    // Game feeds the runner the same objects the view holds.
    act(() => queue.enqueue(next.events.slice(1), next));
    const seen: (string | null)[] = [];
    // Four casts, each reached more than SHOWCASE_GATE_MAX_MS after the first was gated: every
    // runner start restarts the gate, so nothing times out into the capped hold queue instead.
    // The box's own entry is in flight; each tick below ends one entry and starts the next.
    for (const id of ["1", "2", "3", "4"]) {
      tick(); // This cast's entry starts: it replaces in at once.
      expect(screen.getByTestId(T.root)).toHaveAttribute("data-showcase", "multicast");
      expect(screen.getByTestId(T.caption)).toHaveTextContent(`${MULTICAST_TEXT.by} ${BOX_NAME}`);
      seen.push(ordinalNow());
      expect(ordinalNow()).toBe(id);
      advance(6000); // Past the cast's hold, and 24s past the first gate by the last cast.
      tick(); // Its `cardResolved` starts: matches nothing, but proves the runner alive.
      expect(ordinalNow(), "the next cast is still gated, not timed out into the queue").toBeNull();
    }
    expect(seen).toEqual(["1", "2", "3", "4"]);
  });

  it("with no runner progress at all it still lets go after SHOWCASE_GATE_MAX_MS", () => {
    const { queue } = manual();
    mountThen([box(), cast("c1"), resolved("c1"), cast("c2"), resolved("c2")], { queue });
    advance(SHOWCASE_HOLD_MS); // The box's hold runs out; both casts are still gated.
    expect(screen.queryByTestId(T.root)).toBeNull();
    advance(SHOWCASE_GATE_MAX_MS - SHOWCASE_HOLD_MS); // The stall wait runs out: they hold in order.
    expect(screen.getByTestId(T.root)).toHaveAttribute("data-showcase", "multicast");
    expect(ordinalNow()).toBe("1");
  });
});
