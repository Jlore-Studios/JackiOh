// R987: Luck (Meditative #40 Feng Shui) rides on the hero's portrait while the side has any; the engine
// half is proved in crates/cards/src/scripts/meditative/c040_feng_shui.rs.
// R847: a side whose Temporal Rift turn is granted wears "Rift turn used", on either seat; the engine half
// (the flag, public on the view) is proved in crates/engine/tests/rules/extra_turns.rs.
// R1639000: a Heroic Power used this turn wears "Used", on either seat. R1840005: a seat's Credits are a
// badge by the hero from their first gain, a count of 0 included.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { PlayerView } from "@jackioh/shared";

import { withCatalog } from "../test/board.tsx";
import { baseView, emptySide, heroPower } from "../test/fixtures.ts";
import { readSheet } from "../test/setup.ts";
import Hero, { RIFT_SPENT_LABEL, RIFT_SPENT_NOTE } from "./Hero.tsx";
import {
  HERO_DESKTOP_QUERY,
  HERO_TABLET_QUERY,
  HERO_TRAY_DESKTOP_COUNT,
  HERO_TRAY_PHONE_COUNT,
  HERO_TRAY_TABLET_COUNT,
  PHONE_BAR_QUERY,
} from "./config.ts";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("R987 the hero's Luck badge", () => {
  it("R987 the badge appears with Luck on the side and is gone without it", () => {
    const { unmount } = render(
      withCatalog(<Hero view={baseView({ you: emptySide("p1", { luck: 1 }) })} side="you" />),
    );
    const badge = screen.getByTestId("hero-luck");
    expect(badge.textContent).toBe("1");
    expect(badge.getAttribute("data-luck")).toBe("1");
    expect(badge.getAttribute("title")).toBe(
      "Luck 1: every roll your cards make that keeps a best rolls 1 more times",
    );
    unmount();
    render(withCatalog(<Hero view={baseView()} side="you" />));
    expect(screen.queryByTestId("hero-luck")).toBeNull();
  });
});

describe("R847 the hero's spent Temporal Rift badge", () => {
  it("R847 a side whose Rift turn is granted wears the badge on either seat, and a side without the flag wears none", () => {
    const view = baseView({
      you: emptySide("p1", { riftExtraTurn: true }),
      opponent: emptySide("p2", { hand: { count: 4 }, riftExtraTurn: true }),
    });
    for (const side of ["you", "opponent"] as const) {
      const { unmount } = render(withCatalog(<Hero view={view} side={side} />));
      const badge = screen.getByTestId(`rift-spent-${side}`);
      expect(badge.textContent).toBe(RIFT_SPENT_LABEL);
      expect(badge.querySelector(".rift-badge")?.getAttribute("title")).toBe(RIFT_SPENT_NOTE);
      // Outside the view's modifiers (R169): their badge list is unchanged.
      expect(screen.getByTestId(`modifiers-${side}`).querySelectorAll(".modifier-badge")).toHaveLength(0);
      unmount();
    }
    for (const side of ["you", "opponent"] as const) {
      const { unmount } = render(withCatalog(<Hero view={baseView()} side={side} />));
      expect(screen.queryByTestId(`rift-spent-${side}`)).toBeNull();
      unmount();
    }
  });
});

describe("R1639000 a Heroic Power used this turn", () => {
  const used = { ...heroPower, usedThisTurn: true };
  const ping = { ...heroPower, instanceId: "power-2", title: "Ping", x: 1 };

  it("R1639000 the opponent's power tags wear Used and say so in their title only when used, extra powers included", () => {
    const extra = { ...ping, usedThisTurn: true };
    const opponent = (power: typeof used, powers: (typeof used)[]) =>
      baseView({ opponent: emptySide("p2", { hand: { count: 4 }, hero: { health: 30, armor: 0, power, powers } }) });
    const { container, unmount } = render(withCatalog(<Hero view={opponent(used, [used, extra])} side="opponent" />));
    const tags = container.querySelectorAll(".power-tag");
    expect(tags).toHaveLength(2);
    for (const tag of tags) expect(tag.querySelector(".power-used")).toHaveTextContent(/^Used$/);
    expect(tags[0]).toHaveAttribute("title", "Ranching: used this turn");
    expect(tags[1]).toHaveAttribute("title", "Ping: used this turn");
    unmount();

    const fresh = render(withCatalog(<Hero view={opponent(heroPower, [heroPower])} side="opponent" />));
    const tag = fresh.container.querySelector(".power-tag");
    expect(tag?.querySelector(".power-used")).toBeNull();
    expect(tag).not.toHaveAttribute("title");
    expect(tag).toHaveTextContent(/^Ranching2$/);
  });

  it("R1639000 the viewer's own power button wears Used, and its title and accessible name say it was used", () => {
    const yours = (power: typeof used, powers: (typeof used)[]) =>
      baseView({ you: emptySide("p1", { hero: { health: 30, armor: 0, power, powers } }) });
    const { unmount } = render(withCatalog(<Hero view={yours(used, [used, ping])} side="you" />));
    const button = screen.getByTestId("power");
    expect(button.querySelector(".power-used")).not.toBeNull();
    expect(button).toHaveAttribute("title", "Ranching: used this turn");
    expect(button).toHaveAccessibleName(/^Used Ranching/);
    const other = screen.getByTestId("power-power-2");
    expect(other.querySelector(".power-used")).toBeNull();
    expect(other).toHaveAttribute("title", "Ping: Activate, spend (1)");
    unmount();

    render(withCatalog(<Hero view={yours(heroPower, [heroPower])} side="you" />));
    const fresh = screen.getByTestId("power");
    expect(fresh.querySelector(".power-used")).toBeNull();
    expect(fresh).toHaveAttribute("title", "Ranching: Activate, spend (2)");
  });
});

// #2877: the hero's status tray shows a layout's allowance of chips, then a `+N` chip opening
// every entry.

/** Eight modifiers, long labels among them, in the view's order. */
const TRAY_MODIFIERS = [
  { id: "m1", label: "Add a random Call to Chaos to your hand at the start of your every other turn" },
  { id: "m2", label: 'Your cards gain "Combo: draw 1"' },
  { id: "m3", label: "Next Spell gains Echo +1" },
  { id: "m4", label: "Your Heroic Power costs (1) less this turn and your units gain Rush" },
  { id: "m5", label: "+2 Spell Damage while you hold a Secret" },
  { id: "m6", label: "Whenever you draw, give a random unit in your hand +1/+1" },
  { id: "m7", label: "Your traps trigger twice and then keep watching" },
  { id: "m8", label: "At the start of your turn, refresh two spent mana crystals" },
];

/** A seat carrying 8 modifiers, 2 secrets and the Rift flag: 11 tray entries. */
function trayView(): PlayerView {
  return baseView({
    you: emptySide("p1", {
      modifiers: TRAY_MODIFIERS,
      secrets: [
        { id: "s-1", choice: "greed" },
        { id: "s-2", choice: "attack" },
      ],
      riftExtraTurn: true,
    }),
  });
}

type TrayLayout = "phone" | "tablet" | "desktop";

/** A `matchMedia` answering one layout's queries, in the `useOsReducedMotion` test's pattern. */
function mockLayout(layout: TrayLayout): void {
  vi.spyOn(window, "matchMedia").mockImplementation(
    (query: string) =>
      ({
        media: query,
        get matches() {
          if (query === PHONE_BAR_QUERY) return layout === "phone";
          if (query === HERO_DESKTOP_QUERY) return layout === "desktop";
          if (query === HERO_TABLET_QUERY) return layout === "tablet";
          return false;
        },
        onchange: null,
        addListener: () => {},
        removeListener: () => {},
        addEventListener: () => {},
        removeEventListener: () => {},
        dispatchEvent: () => false,
      }) as unknown as MediaQueryList,
  );
}

describe("#2877 the hero's status tray", () => {
  it("#2877 each layout shows its config count of chips, the last a +N with the entries not shown", () => {
    const layouts: { layout: TrayLayout; count: number }[] = [
      { layout: "phone", count: HERO_TRAY_PHONE_COUNT },
      { layout: "tablet", count: HERO_TRAY_TABLET_COUNT },
      { layout: "desktop", count: HERO_TRAY_DESKTOP_COUNT },
    ];
    for (const { layout, count } of layouts) {
      mockLayout(layout);
      const { unmount } = render(withCatalog(<Hero view={trayView()} side="you" />));
      const tray = screen.getByTestId("tray-you");
      expect(tray.getAttribute("data-count")).toBe("11");
      // The `+N` chip takes the allowance's last slot, so one fewer entry renders as a chip.
      expect(tray.querySelectorAll(".modifier-badge")).toHaveLength(count);
      const more = screen.getByTestId("tray-more-you");
      expect(more).toHaveTextContent(`+${11 - (count - 1)}`);
      expect(more).toHaveAttribute("aria-expanded", "false");
      unmount();
      vi.restoreAllMocks();
    }
  });

  it("#2877 a host without matchMedia reads the desktop count", () => {
    const original = window.matchMedia;
    Object.defineProperty(window, "matchMedia", { writable: true, value: undefined });
    try {
      render(withCatalog(<Hero view={trayView()} side="you" />));
      expect(screen.getByTestId("tray-you").querySelectorAll(".modifier-badge")).toHaveLength(HERO_TRAY_DESKTOP_COUNT);
      expect(screen.getByTestId("tray-more-you")).toHaveTextContent(`+${11 - (HERO_TRAY_DESKTOP_COUNT - 1)}`);
    } finally {
      Object.defineProperty(window, "matchMedia", { writable: true, value: original });
    }
  });

  it("#2877 pressing +N opens a list holding every entry, and an outside press closes it", () => {
    mockLayout("desktop");
    render(withCatalog(<Hero view={trayView()} side="you" />));
    expect(screen.queryByTestId("tray-list-you")).toBeNull();

    fireEvent.click(screen.getByTestId("tray-more-you"));
    const list = screen.getByTestId("tray-list-you");
    expect(screen.getByTestId("tray-more-you")).toHaveAttribute("aria-expanded", "true");
    // Every entry is in the list: the modifiers with their full labels, the secrets, the Rift flag.
    expect(list.querySelectorAll(".modifier-badge")).toHaveLength(11);
    for (const modifier of TRAY_MODIFIERS) expect(list).toHaveTextContent(modifier.label);
    expect(list).toHaveTextContent("Secret: Greed");
    expect(list).toHaveTextContent("Secret: Attack");
    expect(list).toHaveTextContent(RIFT_SPENT_LABEL);

    fireEvent.pointerDown(document.body);
    expect(screen.queryByTestId("tray-list-you")).toBeNull();
  });

  it("#2877 modifiers-<side> is present with zero modifiers, and exactly the allowance shows no +N", () => {
    mockLayout("desktop");
    const { unmount } = render(withCatalog(<Hero view={baseView()} side="you" />));
    expect(screen.getByTestId("modifiers-you").querySelectorAll(".modifier-badge")).toHaveLength(0);
    expect(screen.getByTestId("tray-you").getAttribute("data-count")).toBe("0");
    expect(screen.queryByTestId("tray-more-you")).toBeNull();
    unmount();

    const exact = baseView({ you: emptySide("p1", { modifiers: TRAY_MODIFIERS.slice(0, HERO_TRAY_DESKTOP_COUNT) }) });
    render(withCatalog(<Hero view={exact} side="you" />));
    expect(screen.getByTestId("tray-you").querySelectorAll(".modifier-badge")).toHaveLength(HERO_TRAY_DESKTOP_COUNT);
    expect(screen.queryByTestId("tray-more-you")).toBeNull();
  });
});

describe("#2877 a used Heroic Power's mark is an overlay on a fixed-width chip", () => {
  it("#2877 a used and an unused chip share their structure, and Used stays first in the accessible name", () => {
    mockLayout("desktop");
    const used = { ...heroPower, usedThisTurn: true };
    const yours = (power: typeof used) => baseView({ you: emptySide("p1", { hero: { health: 30, armor: 0, power, powers: [power] } }) });
    const { unmount } = render(withCatalog(<Hero view={yours(used)} side="you" />));
    const worn = screen.getByTestId("power");
    expect(worn.firstElementChild?.classList.contains("power-used")).toBe(true);
    expect(worn).toHaveAccessibleName(/^Used Ranching/);
    unmount();

    render(withCatalog(<Hero view={yours(heroPower)} side="you" />));
    const fresh = screen.getByTestId("power");
    expect(fresh.querySelector(".power-used")).toBeNull();
    expect(fresh.tagName).toBe(worn.tagName);
    expect(fresh.className).toBe(worn.className);
  });

  it("#2877 board.css pins the mark to an overlay and the chip to one width", () => {
    const boardCss = readSheet("src/game/board.css");
    expect(boardCss).toMatch(/\.power-used\s*\{[^}]*position:\s*absolute;/);
    // Inside the chip's corner: the chip's `overflow: hidden` clips anything hung outside it.
    expect(boardCss).toMatch(/\.power-used\s*\{[^}]*top:\s*0;/);
    expect(boardCss).toMatch(/\.power-used\s*\{[^}]*right:\s*0;/);
    expect(boardCss).toMatch(/\.power-button,\s*\.power-tag\s*\{[^}]*width:\s*14ch;/);
    // B45: no seat-wide raise — only the tray itself carries the popover level, so a lifted
    // card's name never slides under a seat.
    expect(boardCss).not.toMatch(/\.seat:has\(\.hero-tray\)/);
  });
});

describe("R1840005 the hero's Credits badge", () => {
  it("R1840005 a count of 0 still shows on either seat, unlike the Jade Counter's, and a seat without Credits draws none", () => {
    const view = baseView({
      you: emptySide("p1", { marketCredits: 0, jade: 0 }),
      opponent: emptySide("p2", { hand: { count: 4 }, marketCredits: 0 }),
    });
    for (const side of ["you", "opponent"] as const) {
      const { unmount } = render(withCatalog(<Hero view={view} side={side} />));
      const badge = screen.getByTestId(`market-credits-${side}`);
      expect(badge.textContent).toBe("0");
      expect(badge.getAttribute("data-credits")).toBe("0");
      expect(badge.classList.contains("hero-credits")).toBe(true);
      expect(badge.getAttribute("title")).toBe("Credits");
      // R961: a Jade Counter of 0 draws nothing, which is the difference.
      expect(document.querySelector(".hero-jade")).toBeNull();
      unmount();
    }
    for (const side of ["you", "opponent"] as const) {
      const { unmount } = render(withCatalog(<Hero view={baseView()} side={side} />));
      expect(screen.queryByTestId(`market-credits-${side}`)).toBeNull();
      unmount();
    }
  });
});
