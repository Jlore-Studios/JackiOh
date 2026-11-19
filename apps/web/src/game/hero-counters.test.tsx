// #4251: every counter on a hero's portrait reads at once. Health, Armor, the Jade Counter and
// the Credits keep the oval's corners; Luck, Burn, Toxic and Tipsy ride the hero's tray as
// labelled chips beside the plate, outside the overflow allowance, and the hold-to-inspect view
// lists every counter above 0 by name and number. jsdom has no layout engine, so the pixel
// overlap is the component spec's (`e2e/cypress/component/emotes-layout.cy.tsx`); this file holds
// the rule in its new form on every pull request: each counter drawn once, named, and never
// folded into the `+N` list.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { DEFAULT_PORTRAIT, type EmoteGate, type PlayerView } from "@jackioh/shared";

import {
  HERO_INSPECT_BURN,
  HERO_INSPECT_CREDITS,
  HERO_INSPECT_JADE,
  HERO_INSPECT_LUCK,
  HERO_INSPECT_TIPSY,
  HERO_INSPECT_TOXIC,
  HeroInspect,
} from "../emotes/HeroInspect.tsx";
import { DEFAULT_EMOTE_HAND } from "../emotes/hand.ts";
import { withCatalog } from "../test/board.tsx";
import { baseView, emptySide } from "../test/fixtures.ts";
import { readSheet } from "../test/setup.ts";
import Hero from "./Hero.tsx";
import {
  HERO_DESKTOP_QUERY,
  HERO_TABLET_QUERY,
  HERO_TRAY_DESKTOP_COUNT,
  PHONE_BAR_QUERY,
} from "./config.ts";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const noop = (): void => undefined;
const OPEN: EmoteGate = { ok: true, sentAt: [] };

/** A desktop layout's `matchMedia`, in the `Hero.test.tsx` pattern. */
function mockDesktop(): void {
  vi.spyOn(window, "matchMedia").mockImplementation(
    (query: string) =>
      ({
        media: query,
        get matches() {
          if (query === PHONE_BAR_QUERY) return false;
          if (query === HERO_DESKTOP_QUERY) return true;
          if (query === HERO_TABLET_QUERY) return false;
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

/** A seat carrying every counter above 0 (Credits 0 included) plus `modifierCount` modifiers. */
function counteredSide(player: "p1" | "p2", modifierCount: number): Parameters<typeof emptySide>[1] {
  return {
    hero: { health: 30, armor: 5, powers: [], power: null, burn: 6, toxic: 2 },
    modifiers: Array.from({ length: modifierCount }, (_, index) => ({
      id: `m${player}-${String(index)}`,
      label: `Modifier ${String(index)}`,
    })),
    jade: 3,
    luck: 2,
    tipsy: 4,
    marketCredits: 7,
  };
}

/** Both heroes with every counter above 0. */
function bothCountered(modifierCount: number): PlayerView {
  return baseView({
    you: emptySide("p1", counteredSide("p1", modifierCount)),
    opponent: emptySide("p2", { hand: { count: 4 }, ...counteredSide("p2", modifierCount) }),
  });
}

/** The six counter badges a hero shows, and the name each reads as. */
const COUNTERS: { selector: string; name: string }[] = [
  { selector: ".hero-jade", name: "Jade Counter" },
  { selector: '[data-testid="hero-luck"]', name: "Luck" },
  { selector: ".hero-burn", name: "Burn" },
  { selector: ".hero-toxic", name: "Toxic" },
  { selector: ".hero-tipsy", name: "Tipsy" },
  { selector: '[data-testid^="market-credits-"]', name: "Credits" },
];

describe("#4251 the hero's counters", () => {
  it("#4251 every counter above 0 draws exactly once, named, outside the overflow list", () => {
    mockDesktop();
    // Eight modifiers overflows the desktop allowance of four, so the `+N` chip is up: the
    // counters must still read beside it, never folded in.
    render(withCatalog(<Hero view={bothCountered(8)} side="you" />));
    for (const { selector, name } of COUNTERS) {
      const found = document.querySelectorAll(`[data-testid="hero-you"] ${selector}`);
      expect(found, selector).toHaveLength(1);
      const badge = found[0] as HTMLElement;
      expect(badge.closest('[data-testid^="tray-list-"]'), `${selector} in the overflow list`).toBeNull();
      const text = badge.textContent ?? "";
      const accessible = badge.getAttribute("aria-label") ?? badge.getAttribute("title") ?? "";
      expect(text.includes(name) || accessible.includes(name), `${selector} names ${name}`).toBe(true);
    }
    // The tray's own counters and overflow: eight modifier entries, four counter chips beside them.
    expect(screen.getByTestId("tray-you").getAttribute("data-count")).toBe("8");
    expect(screen.getByTestId("tray-more-you")).toHaveTextContent(`+${8 - (HERO_TRAY_DESKTOP_COUNT - 1)}`);
    expect(screen.getByTestId("hero-counters-you").querySelectorAll(".hero-counter")).toHaveLength(4);
    fireEvent.click(screen.getByTestId("tray-more-you"));
    const list = screen.getByTestId("tray-list-you");
    expect(list.querySelectorAll(".modifier-badge")).toHaveLength(8);
    expect(list.querySelector(".hero-counter"), "a counter in the overflow list").toBeNull();
  });

  it("#4251 both heroes show the same six counters, with health and Armor still on the portrait", () => {
    for (const side of ["you", "opponent"] as const) {
      const { unmount } = render(withCatalog(<Hero view={bothCountered(2)} side={side} />));
      const hero = screen.getByTestId(`hero-${side}`);
      for (const { selector } of COUNTERS) expect(hero.querySelectorAll(selector)).toHaveLength(1);
      expect(hero.querySelector(".hero-portrait .hero-health")).toHaveTextContent("30");
      expect(hero.querySelector(".hero-portrait .hero-armor")).toHaveTextContent("5");
      unmount();
    }
  });

  it("#4251 the inspect view lists every counter above 0 with its name and number", () => {
    render(
      withCatalog(
        <HeroInspect
          side="you"
          portrait={DEFAULT_PORTRAIT}
          health={30}
          armor={5}
          luck={2}
          jade={3}
          burn={6}
          toxic={2}
          tipsy={4}
          marketCredits={7}
          menu="emotes"
          hand={DEFAULT_EMOTE_HAND}
          gate={() => OPEN}
          muted={false}
          onPick={noop}
          onMute={noop}
          onClose={noop}
        />,
      ),
    );
    const stats: [string, string, string][] = [
      [HERO_INSPECT_LUCK, "Luck", "2"],
      [HERO_INSPECT_JADE, "Jade Counter", "3"],
      [HERO_INSPECT_BURN, "Burn", "6"],
      [HERO_INSPECT_TOXIC, "Toxic", "2"],
      [HERO_INSPECT_TIPSY, "Tipsy", "4"],
      [HERO_INSPECT_CREDITS, "Credits", "7"],
    ];
    for (const [testid, name, count] of stats) {
      const stat = screen.getByTestId(testid);
      expect(stat.querySelector("dt")).toHaveTextContent(name);
      expect(stat.querySelector("dd")).toHaveTextContent(count);
    }
  });

  it("#4251 a counter rising from 0 moves no plate", () => {
    const boardCss = readSheet("src/game/board.css");
    // The tray hangs off the plate out of flow (the #2877 reason the plate keeps one size), and
    // the plate keeps its fixed width, so a chip appearing in the tray moves nothing.
    expect(boardCss).toMatch(/\.hero-tray\s*\{[^}]*position:\s*absolute;/);
    expect(boardCss).toMatch(/\.hero\s*\{[^}]*--hero-w:/);
    // The tray's counter chips size like tray chips, not the portrait's 28px pills.
    expect(boardCss).toMatch(/\.hero-tray \.hero-counter\s*\{[^}]*height:\s*auto;/);
    expect(boardCss).not.toMatch(/\.hero-burn,\s*\n\.hero-toxic,/);
    // The portrait's own badges are the same elements with or without a counter: Luck rising
    // adds nothing inside the oval.
    const portraitOf = (view: PlayerView): string | null => {
      const { unmount } = render(withCatalog(<Hero view={view} side="you" />));
      const html = document.querySelector('[data-testid="hero-you"] .hero-portrait')?.innerHTML ?? null;
      unmount();
      return html;
    };
    expect(portraitOf(baseView({ you: emptySide("p1", { luck: 2 }) }))).toBe(portraitOf(baseView()));
  });

  it("#4270 the tray clears the piles, the zones and the portrait's badges on a phone and on its side", () => {
    // The component specs that measure this (board-layout's #2877 and emotes-layout's #4251) run
    // daily only, so this holds the rules that place it on every pull request.
    const boardCss = readSheet("src/game/board.css");
    // The Jade Counter and the Credits stand above the portrait's open button, as health and Armor do.
    expect(boardCss).toMatch(/\.hero-portrait \.hero-jade,\s*\n\.hero-portrait \.hero-credits\s*\{[^}]*z-index:\s*2;/);
    // The desktop's 32px health and Armor badges stand apart on the 44px oval.
    expect(boardCss).toMatch(/\.seat \.hero-portrait \.hero-armor\s*\{[^}]*left:\s*-10px;/);
    expect(boardCss).toMatch(/\.seat \.hero-portrait \.hero-health\s*\{[^}]*right:\s*-10px;/);
    // On a phone the opponent's tray hangs past its pinned pill row, above the pills' tap bands.
    expect(boardCss).toMatch(/\.seat \.pile\s*\{[^}]*height:\s*var\(--seat-pile-h\);/);
    expect(boardCss).toMatch(/:where\(\.board > \.seat-opponent\) > \.hero\s*\{[^}]*z-index:\s*var\(--z-raised\);/);
    expect(boardCss).toMatch(/\.hero-opponent \.hero-tray\s*\{[^}]*top:\s*calc\(100% \+ var\(--seat-pile-h\) \+ 2px\);/);
    expect(boardCss).toMatch(/\.hero-opponent \.hero-tray \.hero-counter-name\s*\{[^}]*display:\s*none;/);
    // On its side both trays show counts alone and hang above their plates.
    expect(boardCss).toMatch(/\.hero-tray \.hero-counter-name\s*\{[^}]*display:\s*none;/);
    expect(boardCss).toMatch(/\.hero-opponent \.hero-tray\s*\{[^}]*top:\s*auto;[^}]*bottom:\s*calc\(100% \+ var\(--space-2\)\);/);
    expect(boardCss).toMatch(/\.hero-you \.hero-tray\s*\{[^}]*bottom:\s*calc\(100% \+ var\(--space-2\)\);/);
  });
});
