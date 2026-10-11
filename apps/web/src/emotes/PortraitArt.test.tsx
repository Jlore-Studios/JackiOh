// R1332's vivid portraits (issue #544): each of the six has a look of its own, drawn as layers
// over the card's oval — a lit ground, a breathing light, drifting motes and a rim — the same on
// the board, in the hero's inspect view and in the deck builder's picker, deterministic, and
// stilled by Reduce Motion. The colours and the motes are `portraitLook.ts`'s; the CSS is
// `portrait.css`, read here as text the way `emotes.test.tsx` reads emotes.css.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it, vi } from "vitest";

import { PORTRAIT_IDS } from "@jackioh/shared";

import {
  PORTRAIT_BREATH_MS,
  PORTRAIT_MOTE_COUNT,
  PORTRAIT_MOTE_DRIFT_MS,
  PORTRAIT_MOTE_EDGE_PCT,
  PORTRAIT_MOTE_MAX_PCT,
  PORTRAIT_MOTE_MIN_PCT,
} from "./config.ts";
import { PORTRAIT_LOOKS, PORTRAIT_MOTES, portraitMotes } from "./portraitLook.ts";
import { PortraitArt } from "./PortraitArt.tsx";
import { PortraitPicker } from "./PortraitPicker.tsx";

const HEX = /^#[0-9a-f]{6}$/;

afterEach(() => {
  cleanup();
});

describe("R1332 the six looks", () => {
  it("R1332 every portrait has a look of its own: six different grounds, every colour #rrggbb", () => {
    expect(Object.keys(PORTRAIT_LOOKS).sort()).toEqual([...PORTRAIT_IDS].sort());
    const grounds = new Set<string>();
    for (const id of PORTRAIT_IDS) {
      const look = PORTRAIT_LOOKS[id];
      for (const colour of [...look.ground, look.light, look.rim, look.mote]) {
        expect(colour, `${id}'s colour`).toMatch(HEX);
      }
      grounds.add(look.ground.join(">"));
    }
    expect(grounds.size).toBe(PORTRAIT_IDS.length);
  });

  it("R1332 the motes: PORTRAIT_MOTE_COUNT each, inside the edge, sizes in range, the same on every call, different between portraits", () => {
    const seen = new Set<string>();
    for (const id of PORTRAIT_IDS) {
      const motes = portraitMotes(id);
      expect(motes).toHaveLength(PORTRAIT_MOTE_COUNT);
      for (const mote of motes) {
        for (const at of [mote.x, mote.y]) {
          expect(at).toBeGreaterThanOrEqual(PORTRAIT_MOTE_EDGE_PCT);
          expect(at).toBeLessThanOrEqual(100 - PORTRAIT_MOTE_EDGE_PCT);
        }
        expect(mote.size).toBeGreaterThanOrEqual(PORTRAIT_MOTE_MIN_PCT);
        expect(mote.size).toBeLessThanOrEqual(PORTRAIT_MOTE_MAX_PCT);
        expect(Number.isInteger(mote.delayMs)).toBe(true);
        expect(mote.delayMs).toBeGreaterThanOrEqual(0);
        expect(mote.delayMs).toBeLessThanOrEqual(PORTRAIT_MOTE_DRIFT_MS);
      }
      // A pure function of the id: again gives the same, and so does the table built at load.
      expect(portraitMotes(id)).toEqual(motes);
      expect(PORTRAIT_MOTES[id]).toEqual(motes);
      seen.add(JSON.stringify(motes));
    }
    expect(seen.size).toBe(PORTRAIT_IDS.length);
  });
});

describe("R1332 PortraitArt", () => {
  it("R1332 PortraitArt draws the oval under its light, breath, motes and rim, with its look inline, identically on two renders", () => {
    for (const id of PORTRAIT_IDS) {
      const first = render(<PortraitArt portrait={id} className="extra" />);
      const root = first.container.firstElementChild as HTMLElement;
      expect(root).toHaveClass("portrait-art", "extra");
      expect(root).toHaveAttribute("data-portrait-art", id);
      expect(root).toHaveAttribute("aria-hidden", "true");
      const layers = [...root.children].map((child) => child.className);
      expect(layers).toEqual([
        "cf-art cf-art--oval",
        "portrait-art-light",
        "portrait-art-breath",
        "portrait-art-motes",
        "portrait-art-rim",
      ]);
      const motes = root.querySelectorAll(".portrait-art-mote");
      expect(motes).toHaveLength(PORTRAIT_MOTE_COUNT);

      const look = PORTRAIT_LOOKS[id];
      expect(root.style.getPropertyValue("--portrait-ground-1")).toBe(look.ground[0]);
      expect(root.style.getPropertyValue("--portrait-ground-2")).toBe(look.ground[1]);
      expect(root.style.getPropertyValue("--portrait-light")).toBe(look.light);
      expect(root.style.getPropertyValue("--portrait-rim")).toBe(look.rim);
      expect(root.style.getPropertyValue("--portrait-mote")).toBe(look.mote);
      expect(root.style.getPropertyValue("--portrait-breath")).toBe(`${PORTRAIT_BREATH_MS}ms`);
      expect(root.style.getPropertyValue("--portrait-drift")).toBe(`${PORTRAIT_MOTE_DRIFT_MS}ms`);
      const firstMote = PORTRAIT_MOTES[id][0];
      const mote = motes[0] as HTMLElement;
      expect(mote.style.getPropertyValue("--mote-x")).toBe(`${firstMote?.x}%`);
      expect(mote.style.getPropertyValue("--mote-delay")).toBe(`-${firstMote?.delayMs}ms`);

      const html = first.container.innerHTML;
      first.unmount();
      const again = render(<PortraitArt portrait={id} className="extra" />);
      expect(again.container.innerHTML).toBe(html);
      again.unmount();
    }
  });

  it("R1332 the four Human portraits differ on the page, not only in the table", () => {
    const styles = new Set<string>();
    for (const id of PORTRAIT_IDS) {
      const { container, unmount } = render(<PortraitArt portrait={id} />);
      styles.add((container.firstElementChild as HTMLElement).getAttribute("style") ?? "");
      unmount();
    }
    expect(styles.size).toBe(PORTRAIT_IDS.length);
  });

  it("R1332 portrait.css saturates the picture and animates breath and motes, and Reduce motion stills them", () => {
    const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "portrait.css"), "utf8");
    // The picture is more saturated, over a ground lit from the upper left.
    expect(css).toMatch(/\.portrait-art > \.cf-art\.cf-art--oval\s*\{[^}]*filter:\s*saturate\(1\.\d+\)/);
    expect(css).toMatch(/\.portrait-art\s*\{[^}]*radial-gradient\(circle at 32% 26%/);
    // The idle: the breath and the motes animate on the timings the component sets.
    expect(css).toMatch(/\.portrait-art-breath\s*\{[^}]*animation:\s*portrait-breathe var\(--portrait-breath\)/);
    expect(css).toMatch(/\.portrait-art-mote\s*\{[^}]*animation:\s*portrait-mote var\(--portrait-drift\)/);
    // Reduce motion — the media query and the app's root flag — stops both, and the reaction too.
    const media = /@media \(prefers-reduced-motion: reduce\)\s*\{([\s\S]*?)\n\}/.exec(css)?.[1] ?? "";
    for (const selector of [".portrait-art-breath", ".portrait-art-mote", ".hero-inspect"]) {
      expect(media, `${selector} under the media query`).toContain(selector);
    }
    expect(media).toContain('.hero-portrait[data-reacting="true"] > .portrait-art');
    expect(media).toMatch(/animation:\s*none/);
    expect(media).toMatch(/\.hero-portrait-glint\s*\{\s*display:\s*none/);
    for (const selector of [
      ".portrait-art-breath",
      ".portrait-art-mote",
      '.hero-portrait[data-reacting="true"] > .portrait-art',
      ".hero-inspect",
    ]) {
      expect(css, `${selector} under the root flag`).toContain(`:root[data-reduce-motion="true"] ${selector}`);
    }
    expect(css).toContain(':root[data-reduce-motion="true"] .hero-portrait-glint');
  });
});

describe("R1332 the deck builder's picker", () => {
  it("R1332 the deck builder's picker draws the vivid portrait on its button and every tile", () => {
    const onPick = vi.fn();
    render(<PortraitPicker portrait="timmy" onPick={onPick} />);

    const current = screen.getByTestId("portrait-current");
    expect(current.querySelector('.portrait-art[data-portrait-art="timmy"]')).not.toBeNull();

    fireEvent.click(current);
    for (const id of PORTRAIT_IDS) {
      const tile = screen.getByTestId(`portrait-pick-${id}`);
      const art = tile.querySelector(`.portrait-art[data-portrait-art="${id}"]`);
      expect(art, `${id}'s tile`).not.toBeNull();
      expect(art?.querySelectorAll(".portrait-art-mote")).toHaveLength(PORTRAIT_MOTE_COUNT);
      expect(art?.querySelector(".cf-art--oval")).not.toBeNull();
    }
  });
});
