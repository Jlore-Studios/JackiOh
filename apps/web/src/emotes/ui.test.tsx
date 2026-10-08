// The emote UI (ui.tsx): what a shown emote draws (R644) and what the two menus do (issue §2, §5,
// with R643's grey-out). The components are cosmetic — a pick reports the emote upward and the
// session decides whether anything shows — so the tests render them bare, against a hand-read
// `gate`, with no session and no clock but the bubble's `--emote-hold` span.

import { StrictMode } from "react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it, vi } from "vitest";

import {
  EMOJI_EMOTE_IDS,
  EMOTE_HAND_SIZE,
  EMOTE_HAND_VOICE,
  EMOTE_IDS,
  VOICE_EMOTE_IDS,
  isVoiceEmote,
  type EmoteGate,
  type EmoteId,
} from "@jackioh/shared";

import { EmojiArt, EMOJI_LABEL } from "./EmojiArt.tsx";
import { EMOTE_MENU_EDGE_PX } from "./config.ts";
import { DEFAULT_EMOTE_HAND } from "./hand.ts";
import { EmoteMenu, EmoteShow, MuteMenu, VOICE_LABEL, arcDrop, menuShift } from "./ui.tsx";
import type { EmoteShow as EmoteShowState } from "./session.ts";

const noop = (): void => undefined;

/** A dealt hand (the engine's `deal_emote_hand("seed-actor", p1)`, R1341): three lines, five emoji. */
const HAND: readonly EmoteId[] = ["greetings", "thanks", "threaten", "laugh", "wahWah", "wave", "thumbsUp", "party"];

const OPEN: EmoteGate = { ok: true, sentAt: [] };
const LIMITED: EmoteGate = { ok: false, retryAfterMs: 2400, sentAt: [0] };

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("R644 EmoteShow", () => {
  it("R644 a voice emote draws its line's text in a speech bubble", () => {
    const show: EmoteShowState = { key: 1, emote: "greetings", text: "Hey there, friend! Feeling lucky?", until: Date.now() + 3_000 };
    render(<EmoteShow show={show} />);

    const bubble = screen.getByTestId("emote-bubble");
    expect(bubble).toHaveClass("emote-bubble");
    expect(bubble).toHaveTextContent("Hey there, friend! Feeling lucky?");
    expect(bubble).toHaveAttribute("role", "status");
    // A voice show is a bubble, not a sticker: no emoji marker, and its span rides --emote-hold.
    expect(bubble).not.toHaveAttribute("data-emoji");
    expect(bubble.style.getPropertyValue("--emote-hold")).toMatch(/^\d+ms$/);
  });

  it("R644 an emoji emote draws its sticker — the SVG for that id, and no words", () => {
    const show: EmoteShowState = { key: 2, emote: "wahWah", text: null, until: Date.now() + 2_000 };
    render(<EmoteShow show={show} />);

    const sticker = screen.getByTestId("emote-bubble");
    expect(sticker).toHaveClass("emote-sticker");
    expect(sticker).toHaveAttribute("data-emoji", "wahWah");
    expect(sticker.querySelector("svg.emote-emoji-svg")).not.toBeNull();
    expect(sticker).not.toHaveTextContent(/\w/);
  });
});

describe("R643 R1343 the emote picker", () => {
  it("R1343 offers the dealt hand and nothing else: its voice lines on the arc, its emoji below", () => {
    render(<EmoteMenu side="you" hand={HAND} gate={() => OPEN} onPick={noop} onClose={noop} />);

    const items = screen.getAllByRole("menuitem");
    expect(items).toHaveLength(EMOTE_HAND_SIZE);
    // In the hand's own (the pool's) order, the lines first, then the emoji.
    expect(items.map((item) => item.dataset.testid)).toEqual(HAND.map((emote) => `emote-${emote}`));
    const arc = screen.getByTestId("emote-menu").querySelector(".emote-voice-arc");
    const row = screen.getByTestId("emote-menu").querySelector(".emote-emoji-row");
    expect(arc?.querySelectorAll(".emote-item")).toHaveLength(EMOTE_HAND_VOICE);
    expect(row?.querySelectorAll(".emote-item")).toHaveLength(EMOTE_HAND_SIZE - EMOTE_HAND_VOICE);
    // The voice items are labelled by name; the emoji carry their title/aria label and a sticker.
    for (const label of ["Greetings", "Thanks", "Threaten"]) {
      expect(screen.getByRole("menuitem", { name: label })).toBeInTheDocument();
    }
    for (const emote of HAND.filter((id) => !isVoiceEmote(id))) {
      const item = screen.getByRole("menuitem", { name: EMOJI_LABEL[emote as keyof typeof EMOJI_LABEL] });
      expect(item.querySelector("svg.emote-emoji-svg")).not.toBeNull();
    }
    // Nothing of the pool outside the hand is offered.
    for (const emote of EMOTE_IDS.filter((id) => !HAND.includes(id))) {
      expect(screen.queryByTestId(`emote-${emote}`), emote).toBeNull();
    }
  });

  it("R1343 the default hand, before a deal arrives, is three lines and patch v0.2.X's five emoji", () => {
    expect(DEFAULT_EMOTE_HAND).toEqual(["greetings", "wellPlayed", "oops", "sob", "yawn", "laugh", "angry", "wahWah"]);
    render(<EmoteMenu side="you" hand={DEFAULT_EMOTE_HAND} gate={() => OPEN} onPick={noop} onClose={noop} />);
    expect(screen.getAllByRole("menuitem")).toHaveLength(EMOTE_HAND_SIZE);
  });

  it("R643 a press reports the emote and closes the menu", () => {
    const onPick = vi.fn();
    const onClose = vi.fn();
    render(<EmoteMenu side="you" hand={HAND} gate={() => OPEN} onPick={onPick} onClose={onClose} />);

    fireEvent.click(screen.getByTestId("emote-thanks"));

    expect(onPick).toHaveBeenCalledTimes(1);
    expect(onPick).toHaveBeenCalledWith("thanks");
    expect(onClose).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByTestId("emote-party"));
    expect(onPick).toHaveBeenLastCalledWith("party");
    expect(onPick).toHaveBeenCalledTimes(2);
  });

  it("R643 while the shared gate says limited, every item greys out with the wait and reports nothing", () => {
    const onPick = vi.fn();
    const onClose = vi.fn();
    render(<EmoteMenu side="you" hand={HAND} gate={() => LIMITED} onPick={onPick} onClose={onClose} />);

    const items = screen.getAllByRole("menuitem");
    expect(items).toHaveLength(EMOTE_HAND_SIZE);
    for (const item of items) {
      expect(item).toBeDisabled();
      expect(item).toHaveAttribute("data-limited", "true");
    }
    // Each item carries the wait in seconds — ceil(2400 / 1000) — the same reading the server drops on.
    const waits = document.querySelectorAll(".emote-wait");
    expect(waits).toHaveLength(EMOTE_HAND_SIZE);
    expect(waits[0]).toHaveTextContent("3");

    // The press is still guarded: a limited menu reports nothing and stays open.
    fireEvent.click(screen.getByTestId("emote-threaten"));
    expect(onPick).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
  });

  it("R643 the menu closes on a press outside it and on Escape, but not on a press inside", () => {
    const onClose = vi.fn();
    render(<EmoteMenu side="you" hand={HAND} gate={() => OPEN} onPick={noop} onClose={onClose} />);

    fireEvent.pointerDown(screen.getByTestId("emote-menu"));
    expect(onClose).not.toHaveBeenCalled();

    fireEvent.pointerDown(document.body);
    expect(onClose).toHaveBeenCalledTimes(1);

    fireEvent.keyDown(window, { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(2);
  });
});

describe("R643 the Mute emotes menu", () => {
  it("R643 offers the one item and reports it, then closes", () => {
    const onMute = vi.fn();
    const onClose = vi.fn();
    render(<MuteMenu muted={false} onMute={onMute} onClose={onClose} />);

    const item = screen.getByRole("menuitem", { name: "Mute emotes" });
    expect(item).not.toBeDisabled();

    fireEvent.click(item);

    expect(onMute).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("R643 an already-muted opponent reads Emotes muted and does nothing", () => {
    const onMute = vi.fn();
    const onClose = vi.fn();
    render(<MuteMenu muted={true} onMute={onMute} onClose={onClose} />);

    const item = screen.getByRole("menuitem", { name: "Emotes muted" });
    expect(item).toBeDisabled();
    fireEvent.click(item);
    expect(onMute).not.toHaveBeenCalled();
  });
});

// #219: jsdom has no layout engine, so what the slide does to a real rect is pinned on `menuShift`
// and the stylesheet's own text; the rendered menu proves the hook writes the variable and keeps
// it across an effect replay. The measurements themselves (emoji and label sizes, 44px targets,
// menus and bubble on screen) are e2e/cypress/component/emotes-layout.cy.tsx's, in Chrome.
describe("#219 menuShift, the slide that keeps a menu on the screen", () => {
  it("#219 shifts right by what pokes out on the left, left by what pokes out on the right, nothing when it fits", () => {
    // The fixture board at 360px puts your portrait at the screen's left edge: a 320px menu
    // centred on it starts at x=-131 and must come right to the margin.
    expect(menuShift({ left: -131, right: 189 }, 360, EMOTE_MENU_EDGE_PX)).toBe(139);
    expect(menuShift({ left: 300, right: 420 }, 360, EMOTE_MENU_EDGE_PX)).toBe(-68);
    expect(menuShift({ left: 20, right: 300 }, 360, EMOTE_MENU_EDGE_PX)).toBe(0);
    // Exactly at the margin is still on the screen.
    expect(menuShift({ left: EMOTE_MENU_EDGE_PX, right: 352 }, 360, EMOTE_MENU_EDGE_PX)).toBe(0);
  });

  it("#219 a menu too wide for both edges keeps its left edge on the screen", () => {
    // Shifting left by the whole overflow (-48) would carry the left edge off, so it stops at
    // the margin (-12 puts left on 8).
    expect(menuShift({ left: 20, right: 400 }, 360, EMOTE_MENU_EDGE_PX)).toBe(-12);
  });
});

/** A DOMRect-shaped answer for the getBoundingClientRect stub — only the fields the hook reads. */
function rect(left: number, right: number, width: number): DOMRect {
  return { left, right, width, top: 0, bottom: 0, height: 0, x: left, y: 0, toJSON: () => ({}) } as DOMRect;
}

describe("#219 an open menu measures itself and slides back on screen", () => {
  it("#219 your picker writes the shift menuShift computes into --emote-menu-shift", () => {
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue(rect(-131, 189, 320));
    vi.spyOn(document.documentElement, "clientWidth", "get").mockReturnValue(360);
    render(<EmoteMenu side="you" hand={HAND} gate={() => OPEN} onPick={noop} onClose={noop} />);

    expect(screen.getByTestId("emote-menu").style.getPropertyValue("--emote-menu-shift")).toBe("139px");
  });

  it("#219 the mute menu slides the same way — left, when it pokes out on the right", () => {
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue(rect(300, 420, 120));
    vi.spyOn(document.documentElement, "clientWidth", "get").mockReturnValue(360);
    render(<MuteMenu muted={false} onMute={noop} onClose={noop} />);

    expect(screen.getByTestId("emote-mute-menu").style.getPropertyValue("--emote-menu-shift")).toBe("-68px");
  });

  it("#219 with nothing to measure (jsdom's zero rects) the menu stays centred — no property", () => {
    render(<EmoteMenu side="you" hand={HAND} gate={() => OPEN} onPick={noop} onClose={noop} />);

    expect(screen.getByTestId("emote-menu").style.getPropertyValue("--emote-menu-shift")).toBe("");
  });

  it("#219 a StrictMode replay — dev's second layout-effect run — keeps the shift, it does not cancel it", () => {
    // jsdom has no layout, so the stub plays the layout engine: the centred rect plus whatever
    // --emote-menu-shift the element already carries, what a real getBoundingClientRect reports
    // once the transform applies. main.tsx mounts the app under StrictMode, so in dev the hook's
    // effect runs twice — a second run that took the slid rect for centred would write 0 back.
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      const applied = Number.parseFloat(this.style.getPropertyValue("--emote-menu-shift")) || 0;
      return rect(-131 + applied, 189 + applied, 320);
    });
    vi.spyOn(document.documentElement, "clientWidth", "get").mockReturnValue(360);
    render(
      <StrictMode>
        <EmoteMenu side="you" hand={HAND} gate={() => OPEN} onPick={noop} onClose={noop} />
      </StrictMode>,
    );

    expect(screen.getByTestId("emote-menu").style.getPropertyValue("--emote-menu-shift")).toBe("139px");
  });
});

// The sheet carries the half of #219 that is pure geometry: the classes that outrank
// `.app-shell button` (index.css pads every shell button 9px 16px, which squeezed each emoji to a
// dot), the shrink-to-fit widths, the two CSS variables, and the phone's flat wrap.
const emotesCss = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "emotes.css"), "utf8");

describe("#219 emotes.css lays the menus and bubble out at readable sizes", () => {
  it("#219 the menu and the bubble shrink to fit their contents, not the 44px portrait", () => {
    expect(emotesCss).toMatch(/\.emote-menu\s*\{[^}]*width:\s*max-content/);
    expect(emotesCss).toMatch(/\.emote-menu\s*\{[^}]*max-width:\s*calc\(100vw - 32px\)/);
    expect(emotesCss).toMatch(/\.emote-menu\s*\{[^}]*--emote-menu-shift/);
    expect(emotesCss).toMatch(/\.emote-bubble\s*\{[^}]*width:\s*max-content/);
    expect(emotesCss).toMatch(/\.emote-bubble\s*\{[^}]*max-width:\s*min\(28ch/);
  });

  it("#219 each item kind's rule is two classes deep, so `.app-shell button` cannot squeeze it", () => {
    expect(emotesCss).toMatch(/\.emote-item\.emote-voice\s*\{/);
    expect(emotesCss).toMatch(/\.emote-item\.emote-emoji-pick\s*\{[^}]*width:\s*52px/);
    expect(emotesCss).toMatch(/\.emote-item\.emote-emoji-pick\s*\{[^}]*height:\s*52px/);
    expect(emotesCss).toMatch(/\.emote-item\.emote-mute\s*\{/);
    // The single-class rules that lost the specificity fight are gone.
    expect(emotesCss).not.toMatch(/^\.emote-voice\s*\{/m);
    expect(emotesCss).not.toMatch(/^\.emote-emoji-pick\s*\{/m);
    expect(emotesCss).not.toMatch(/^\.emote-mute\s*\{/m);
  });

  it("#219 the arc drop rides --emote-arc-drop, four classes deep so :active cannot nudge it", () => {
    expect(emotesCss).toMatch(
      /\.emote-menu \.emote-voice-arc > \.emote-item\.emote-voice\s*\{[^}]*transform:\s*translateY\(var\(--emote-arc-drop/,
    );
  });

  it("#219 touch items keep board.css's 44px floor, and a held-upright phone wraps the lines flat", () => {
    expect(emotesCss).toMatch(/@media \(max-width: 1024px\), \(pointer: coarse\)\s*\{[\s\S]*?min-height:\s*44px/);
    expect(emotesCss).toMatch(/@media \(max-width: 600px\)\s*\{[\s\S]*?\.emote-voice-arc\s*\{[^}]*flex-wrap:\s*wrap/);
    expect(emotesCss).toMatch(/@media \(max-width: 600px\)\s*\{[\s\S]*?transform:\s*none/);
  });
});

describe("#219 each voice item carries its arc drop as the --emote-arc-drop the sheet reads", () => {
  it("#219 R1343 a hand's three lines drop 4, 0, 4, crown in the middle, and no inline transform left", () => {
    render(<EmoteMenu side="you" hand={HAND} gate={() => OPEN} onPick={noop} onClose={noop} />);

    const items = HAND.filter((emote) => isVoiceEmote(emote)).map((emote) => screen.getByTestId(`emote-${emote}`));
    expect(items.map((item) => item.style.getPropertyValue("--emote-arc-drop"))).toEqual(["4px", "0px", "4px"]);
    expect(items[0]?.style.transform).toBe("");
  });

  it("#219 the same curve gives all five lines the arc they always had: 14, 4, 0, 4, 14", () => {
    const fiveLines: readonly EmoteId[] = [...VOICE_EMOTE_IDS, "sob", "yawn", "laugh"];
    render(<EmoteMenu side="you" hand={fiveLines} gate={() => OPEN} onPick={noop} onClose={noop} />);

    const items = VOICE_EMOTE_IDS.map((emote) => screen.getByTestId(`emote-${emote}`));
    expect(items.map((item) => item.style.getPropertyValue("--emote-arc-drop"))).toEqual([
      "14px",
      "4px",
      "0px",
      "4px",
      "14px",
    ]);
    expect([0, 1, 2, 3, 4].map((index) => arcDrop(index, 5))).toEqual([14, 4, 0, 4, 14]);
    expect([0, 1, 2].map((index) => arcDrop(index, 3))).toEqual([4, 0, 4]);
  });
});

// R1345: MN03's fourteen emoji are stickers like the five — inline SVG and no words, each with its
// own accessible name, and still under Reduce Motion but for the fade.
describe("R1345 every emoji of the pool is an inline sticker with its own name", () => {
  it("R1345 each draws one emote-emoji-svg, hidden from the accessibility tree, with no <text> in it", () => {
    for (const emote of EMOJI_EMOTE_IDS) {
      const { container, unmount } = render(<EmojiArt emoji={emote} />);
      const svgs = container.querySelectorAll("svg");
      expect(svgs, emote).toHaveLength(1);
      expect(svgs[0]).toHaveClass("emote-emoji-svg");
      expect(svgs[0]).toHaveAttribute("aria-hidden", "true");
      expect(svgs[0]?.getAttribute("viewBox"), emote).toBe("0 0 48 48");
      expect(container.querySelector("text"), `${emote} draws words`).toBeNull();
      expect(container.querySelector("image, foreignObject, use"), `${emote} borrows art`).toBeNull();
      expect(container.textContent, emote).toBe("");
      unmount();
    }
  });

  it("R1345 no two stickers are the same drawing", () => {
    const drawings = EMOJI_EMOTE_IDS.map((emote) => {
      const { container, unmount } = render(<EmojiArt emoji={emote} />);
      const markup = container.innerHTML;
      unmount();
      return markup;
    });
    expect(new Set(drawings).size).toBe(EMOJI_EMOTE_IDS.length);
  });

  it("R1345 each emoji has its own non-empty label, which a menu holding it names the item by", () => {
    const labels = EMOJI_EMOTE_IDS.map((emote) => EMOJI_LABEL[emote]);
    expect(labels.every((label) => label.trim().length > 0)).toBe(true);
    expect(new Set([...labels, ...Object.values(VOICE_LABEL)]).size).toBe(EMOTE_IDS.length);
    const newEmoji = EMOJI_EMOTE_IDS.slice(5);
    expect(newEmoji.length).toBeGreaterThanOrEqual(14);
    for (let at = 0; at < newEmoji.length; at += EMOTE_HAND_SIZE) {
      const hand = newEmoji.slice(at, at + EMOTE_HAND_SIZE);
      const { unmount } = render(<EmoteMenu side="you" hand={hand} gate={() => OPEN} onPick={noop} onClose={noop} />);
      for (const emote of hand) {
        const item = screen.getByRole("menuitem", { name: EMOJI_LABEL[emote] });
        expect(item).toHaveAttribute("title", EMOJI_LABEL[emote]);
        expect(item.dataset.testid).toBe(`emote-${emote}`);
      }
      unmount();
    }
  });

  it("R1345 every part that moves inside a sticker is stilled under Reduce Motion, by the query and by the flag", () => {
    const moving = new Set<string>();
    for (const emote of EMOJI_EMOTE_IDS) {
      const { container, unmount } = render(<EmojiArt emoji={emote} />);
      for (const element of container.querySelectorAll("[class]")) {
        for (const name of (element.getAttribute("class") ?? "").split(/\s+/)) {
          const rule = new RegExp(`^\\.${name}\\s*\\{[^}]*animation:`, "m");
          if (name !== "emote-emoji-svg" && rule.test(emotesCss)) moving.add(name);
        }
      }
      unmount();
    }
    // The five's tear and trombone, and MN03's six moving parts.
    expect([...moving].sort()).toEqual(
      [
        "emote-clap-hands",
        "emote-confetti",
        "emote-flame",
        "emote-heartbeat",
        "emote-sweat-drop",
        "emote-tear",
        "emote-trombone",
        "emote-wave-hand",
      ].sort(),
    );
    const media = /@media \(prefers-reduced-motion: reduce\)\s*\{([\s\S]*?)\n\}/.exec(emotesCss)?.[1] ?? "";
    for (const name of moving) {
      expect(media, `${name} under prefers-reduced-motion`).toMatch(new RegExp(`\\.${name}\\b[^{]*\\{\\s*animation:\\s*none`));
      expect(emotesCss, `${name} under data-reduce-motion`).toMatch(
        new RegExp(`:root\\[data-reduce-motion="true"\\] \\.${name}[,\\s][^{]*\\{\\s*animation:\\s*none`),
      );
    }
  });
});

// #257: a hero that is not a target is drawn at 0.9 opacity (board.css), which makes it a stacking
// context, so the show's and the menus' z-indexes counted only inside it: the field, which follows
// the opponent's seat in the document, painted over their bubble, sticker and Mute menu. The sheet
// lifts the hero and its seat while a piece is up, and draws the hero at full strength. Whether
// anything still covers a piece takes a layout engine (emotes-layout.cy.tsx); what the sheets pin is
// the lift itself, and that it stays under every overlay the issue keeps above an emote.
function sheet(path: string): string {
  return readFileSync(join(dirname(fileURLToPath(import.meta.url)), path), "utf8");
}

/** The z-index of the first rule whose selector is `selector`, at the start of a line. */
function zIndexOf(css: string, selector: string): number {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = new RegExp(`^${escaped}\\s*\\{[^}]*z-index:\\s*(\\d+)`, "m").exec(css);
  if (match === null) throw new Error(`no z-index for ${selector}`);
  return Number(match[1]);
}

/** The lift emotes.css gives a hero, and its seat, while a bubble, a sticker or a menu is up. */
function emoteLift(): number {
  return zIndexOf(emotesCss, ".seat:has(.emote-show, .emote-menu),\n.hero:has(.emote-show, .emote-menu)");
}

describe("#257 a hero with an emote up is lifted over the board, under every overlay", () => {
  it("#257 the hero and its seat are lifted while a piece is up, and the hero is drawn at full strength", () => {
    expect(emoteLift()).toBeGreaterThan(zIndexOf(sheet("../game/board.css"), '.board .hero[data-glow="ready"]'));
    expect(emotesCss).toMatch(/^\.hero\[data-legal\]:has\(\.emote-show, \.emote-menu\)\s*\{[^}]*opacity:\s*1;/m);
  });

  it("#257 the lift stays under the pile notices, the turn fuse, the effects layer, the result screen, the prompt and the settings", () => {
    const overlays: [string, string][] = [
      ["../game/overflow.css", ".seat:has(.pile-notice)"],
      ["../game/clock.css", ".clock-fuse"],
      ["../fx/fx.css", ".fx-layer"],
      ["../game/animations.css", '[data-testid="result-overlay"]'],
      ["../game/animations.css", ".result-scrim"],
      ["../game/prompt.css", ".prompt-scrim"],
      ["../settings/settings.css", ".settings-scrim"],
    ];
    const lift = emoteLift();
    for (const [file, selector] of overlays) {
      expect(lift, `${selector} in ${file}`).toBeLessThan(zIndexOf(sheet(file), selector));
    }
  });
});
