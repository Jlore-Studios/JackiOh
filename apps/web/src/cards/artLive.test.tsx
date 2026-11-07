// R3340000: art that lives in play. The board's faces (a Unit's `MinionFace`, a face-up backrow card's compact `CardFace`) move
// their art for an animated or signed level and for every Mythic, in the one budget of motion slots the hand shares (ranked
// by the pointer or focus first, then Mythics, Legendaries and the rest newest first); a tab hidden, a face off screen and
// every motion setting hold them still; the art answers the pointer, a press and an attack; and a unit's art wears cracks, a
// glow and frost. Vitest stubs CSS, so artLive.css is read as text; the pixels are e2e's cypress/component/art-answers.cy.tsx.

import { act, cleanup, render } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { UnitView } from "@jackioh/shared";

import { setFxSettings, resetFxSettingsForTests } from "../fx/settings.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { readSheet, resetVisibility, setReducedMotion, setVisibility } from "../test/setup.ts";
import { unit } from "../test/fixtures.ts";
import { THEME_PALETTES, type ArtThemeId } from "./art/themes.ts";
import { PARTICLE_COUNT } from "./art/motion.ts";
import { CardFace } from "./CardFace.tsx";
import { COSMETIC_FOIL_ANIMATED_MAX } from "./constants.ts";
import { resetCosmeticsForTests, writeCardLevel } from "./cosmetics.ts";
import { FullArtOwned } from "./fullArt.ts";
import { MinionFace } from "./MinionFace.tsx";
import { faceModel } from "./model.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "./settings.ts";
import { VERSION_ANIMATED_ID, VERSION_FULL_ART_ANIMATED_ID, VERSION_FULL_ART_SIGNED_ID } from "./versions.ts";

const MAX = COSMETIC_FOIL_ANIMATED_MAX;
const DATA_PREFIX = "data:image/svg+xml,";

/** Mythic Units (none is Ceaseless Void, whose face is its own), Legendary and Common ones. */
const MYTHICS = ["meditative-049", "stinkyscape-074", "stinkyscape-086", "stinkyscape-109"] as const;
const LEGENDARY = "core-092";
const COMMON = "core-002";
const GARY = "core-004";

function defOf(id: string) {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`no ${id}`);
  return def;
}

/** One unit on the board: its card, the view's numbers over its printed ones, its attack when buffed and whether its attack is playing. */
type Spec = { id: string; over?: Partial<UnitView>; attack?: number; attacking?: boolean };

/** A Unit on the board, in a `.card.cf-host` as Card.tsx sets it, with its printed numbers unless `over` or `attack` change them. */
function Minion({ id, over = {}, attack, attacking = false }: Spec): ReactElement {
  const printed = defOf(id).base;
  const health = printed.health ?? 4;
  const u = unit("p1", { defId: id, attack: attack ?? printed.attack ?? 0, health, maxHealth: health, ...over });
  const face = faceModel({
    defId: id,
    def: defOf(id),
    radiant: false,
    live: { attack: u.attack, health: u.health, maxHealth: u.maxHealth, keywords: u.keywords },
  });
  return (
    <div className="card cf-host">
      <MinionFace face={face} unit={u} worn attacking={attacking} />
    </div>
  );
}

function Board({ specs }: { specs: readonly Spec[] }): ReactElement {
  return (
    <FullArtOwned owned="every">
      {specs.map((spec, index) => (
        <Minion key={index} {...spec} />
      ))}
    </FullArtOwned>
  );
}

/** The minion faces drawn, in order. */
const minions = (container: HTMLElement): HTMLElement[] => [...container.querySelectorAll<HTMLElement>(".cf--minion")];
const motionOf = (cf: HTMLElement): string | null => cf.getAttribute("data-art-motion");
const motions = (container: HTMLElement): (string | null)[] => minions(container).map(motionOf);
const host = (cf: HTMLElement): HTMLElement => cf.closest<HTMLElement>(".cf-host") ?? cf;
/** The picture a window draws, decoded: the document its background image holds. */
function documentOf(element: Element | null): Document {
  const uri = /^url\("(.*)"\)$/u.exec((element as HTMLElement | null)?.style.backgroundImage ?? "")?.[1] ?? "";
  expect(uri.startsWith(DATA_PREFIX)).toBe(true);
  return new DOMParser().parseFromString(decodeURIComponent(uri.slice(DATA_PREFIX.length)), "image/svg+xml");
}

/** The pointer and focus events a face listens for, on its card's host. */
const fire = (target: Element, type: string, init: MouseEventInit = {}): void => {
  act(() => {
    target.dispatchEvent(type.startsWith("focus") ? new FocusEvent(type, { bubbles: true }) : new MouseEvent(type, { bubbles: true, ...init }));
  });
};

class FakeObserver {
  static all: FakeObserver[] = [];
  readonly targets: Element[] = [];
  constructor(private readonly callback: IntersectionObserverCallback) {
    FakeObserver.all.push(this);
  }
  observe(target: Element): void {
    this.targets.push(target);
  }
  unobserve(): void {}
  disconnect(): void {
    this.targets.length = 0;
  }
  takeRecords(): IntersectionObserverEntry[] {
    return [];
  }
  /** Tells the face this observer watches whether it is on screen. */
  report(isIntersecting: boolean): void {
    act(() => {
      this.callback([{ isIntersecting, target: this.targets[0] } as IntersectionObserverEntry], this as unknown as IntersectionObserver);
    });
  }
}

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
  resetFxSettingsForTests();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  FakeObserver.all = [];
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  resetVisibility();
  setReducedMotion(false);
  __resetSettingsForTests();
  resetCosmeticsForTests();
  resetFxSettingsForTests();
  localStorage.clear();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
});

describe("R3340000 which board faces move", () => {
  it("R3340000 animated and signed board faces and every Mythic move, in any look; a normal Legendary draws still", () => {
    writeCardLevel(COMMON, VERSION_ANIMATED_ID);
    writeCardLevel(GARY, VERSION_FULL_ART_SIGNED_ID);
    const { container } = render(<Board specs={[{ id: COMMON }, { id: GARY }, { id: MYTHICS[0] }, { id: LEGENDARY }, { id: "core-022" }]} />);
    const [animated, signed, mythic, legendary, epic] = minions(container);
    expect(motionOf(animated as HTMLElement), "an animated level").toBe("animated");
    expect(motionOf(signed as HTMLElement), "a signed level").toBe("animated");
    expect(motionOf(mythic as HTMLElement), "a Mythic in its normal look").toBe("animated");
    for (const cf of [animated, signed, mythic]) expect(cf?.querySelector('.cf-art[data-art-moving="true"]'), cf?.getAttribute("data-version") ?? "").not.toBeNull();
    // A Legendary, and an Epic with no look, draw their still picture.
    for (const cf of [legendary, epic]) {
      expect(cf?.hasAttribute("data-art-motion")).toBe(false);
      expect(cf?.querySelector("[data-art-moving], [data-art-live]")).toBeNull();
      expect(cf?.querySelector<HTMLElement>(".cf-art")?.style.backgroundImage).not.toContain("%3Cstyle");
    }
    // A board face draws no signature, sweep or drift of its own (R3046000).
    expect(container.querySelector(".cf-signature, .cf-full-art-sweep, [data-full-art-motion]")).toBeNull();
  });

  it("R3340000 a face-up backrow card that is a Mythic or wears an animated level moves, and a tile that is no card in play does not", () => {
    const mythicSpell = Object.values(CATALOG).find((def) => def.rarity === "Mythic" && def.type === "Spell" && !def.token);
    expect(mythicSpell).toBeDefined();
    if (mythicSpell === undefined) return;
    writeCardLevel("core-018", VERSION_ANIMATED_ID);
    const compact = (id: string, onBoard: boolean): ReactElement => (
      <div className="card cf-host">
        <CardFace face={faceModel({ defId: id, def: defOf(id), radiant: false })} layout="compact" worn onBoard={onBoard} />
      </div>
    );
    const { container } = render(
      <FullArtOwned owned="every">
        {compact(mythicSpell.id, true)}
        {compact("core-018", true)}
        {compact("core-041", true)}
        {compact(mythicSpell.id, false)}
      </FullArtOwned>,
    );
    const [mythic, animated, plain, tile] = [...container.querySelectorAll<HTMLElement>(".cf--compact")];
    expect(mythic?.getAttribute("data-art-motion")).toBe("animated");
    expect(animated?.getAttribute("data-art-motion")).toBe("animated");
    expect(plain?.hasAttribute("data-art-motion")).toBe(false);
    expect(tile?.hasAttribute("data-art-motion")).toBe(false);
    expect(tile?.hasAttribute("data-art-hover")).toBe(false);
    for (const cf of [mythic, animated]) expect(cf?.getAttribute("data-art-hover"), "a card in play tracks the pointer").toBe("false");
  });
});

describe("R3340000 the slots go by rank, in one budget the hand shares", () => {
  it("R3340000 with MAX plus three faces the slots go to the hovered one, the Mythics newest first, the Legendary, then the rest newest first", () => {
    writeCardLevel(GARY, VERSION_ANIMATED_ID);
    writeCardLevel(COMMON, VERSION_ANIMATED_ID);
    writeCardLevel(LEGENDARY, VERSION_ANIMATED_ID);
    expect(MAX).toBe(6);
    // Oldest first: faces 0, 3, 5, 6 and 7 are the rest, face 1 the Legendary, faces 2, 4 and 8 the Mythics.
    const specs: Spec[] = [
      { id: GARY },
      { id: LEGENDARY },
      { id: MYTHICS[0] },
      { id: COMMON },
      { id: MYTHICS[1] },
      { id: GARY },
      { id: GARY },
      { id: COMMON },
      { id: MYTHICS[2] },
    ];
    expect(specs).toHaveLength(MAX + 3);
    const { container } = render(<Board specs={specs} />);
    const state = (): string[] => motions(container).map((motion) => motion ?? "none");
    // Mythics 8, 4 and 2, the Legendary 1, then the newest rest, 7 and 6.
    expect(state()).toEqual(["static", "animated", "animated", "static", "animated", "static", "animated", "animated", "animated"]);
    // The rest that hold still draw no moving art.
    const faces = minions(container);
    for (const index of [0, 3, 5]) expect(faces[index]?.querySelector("[data-art-moving], [data-art-live]"), `face ${index}`).toBeNull();
    for (const index of [1, 2, 4, 6, 7, 8]) expect(faces[index]?.querySelector('[data-art-moving="true"]'), `face ${index}`).not.toBeNull();

    // The oldest face is hovered: it ranks first, and the lowest-ranked holder (face 6, the oldest of the rest that held) gives way.
    fire(host(faces[0] as HTMLElement), "pointerenter");
    expect(state()).toEqual(["animated", "animated", "animated", "static", "animated", "static", "static", "animated", "animated"]);
    fire(host(faces[0] as HTMLElement), "pointerleave");
    expect(state()).toEqual(["static", "animated", "animated", "static", "animated", "static", "animated", "animated", "animated"]);
  });

  it("R3340000 a face of the hand and the board's faces take slots from one budget", () => {
    writeCardLevel(GARY, VERSION_FULL_ART_ANIMATED_ID);
    const gary = faceModel({ defId: GARY, def: defOf(GARY), radiant: false });
    const { container } = render(
      <FullArtOwned owned="every">
        <Board specs={Array.from({ length: MAX }, () => ({ id: GARY }))} />
        <CardFace face={gary} layout="full" worn />
      </FullArtOwned>,
    );
    // The hand's face is the newest of the rest: it takes the oldest board face's slot, and that face draws still.
    const hand = container.querySelector<HTMLElement>(".cf--full");
    expect(hand?.getAttribute("data-art-motion")).toBe("animated");
    const board = minions(container);
    expect(board.map(motionOf)).toEqual(["static", ...Array<string>(MAX - 1).fill("animated")]);
    expect(board[0]?.querySelector("[data-art-moving]")).toBeNull();
  });

  it("R3340000 a slot freed by a face leaving passes to the next face at once", () => {
    writeCardLevel(GARY, VERSION_ANIMATED_ID);
    const { container, rerender } = render(<Board specs={Array.from({ length: MAX + 1 }, () => ({ id: GARY }))} />);
    expect(motions(container)).toEqual(["static", ...Array<string>(MAX).fill("animated")]);
    act(() => {
      rerender(<Board specs={Array.from({ length: MAX }, () => ({ id: GARY }))} />);
    });
    expect(motions(container)).toEqual(Array<string>(MAX).fill("animated"));
  });
});

describe("R3340000 motion pauses and the settings still rule", () => {
  it("R3340000 a hidden tab holds every face still and a visible one moves them again", () => {
    const { container } = render(<Board specs={[{ id: MYTHICS[0] }, { id: MYTHICS[1] }, { id: MYTHICS[2] }]} />);
    expect(motions(container)).toEqual(["animated", "animated", "animated"]);
    act(() => {
      setVisibility("hidden");
    });
    expect(motions(container)).toEqual(["static", "static", "static"]);
    expect(container.querySelector("[data-art-moving], [data-art-live]")).toBeNull();
    act(() => {
      setVisibility("visible");
    });
    expect(motions(container)).toEqual(["animated", "animated", "animated"]);
  });

  it("R3340000 a face off screen holds still and frees its slot, and moves again on screen", () => {
    vi.stubGlobal("IntersectionObserver", FakeObserver);
    const { container } = render(<Board specs={[{ id: MYTHICS[0] }, { id: MYTHICS[1] }]} />);
    const [first, second] = minions(container) as [HTMLElement, HTMLElement];
    expect(motions(container)).toEqual(["animated", "animated"]);
    const watching = (cf: HTMLElement): FakeObserver => {
      const found = FakeObserver.all.find((observer) => observer.targets.includes(cf));
      if (found === undefined) throw new Error("no observer");
      return found;
    };
    watching(first).report(false);
    expect(motionOf(first)).toBe("static");
    expect(first.querySelector("[data-art-moving]")).toBeNull();
    expect(motionOf(second)).toBe("animated");
    watching(first).report(true);
    expect(motionOf(first)).toBe("animated");
  });

  it("R3340000 Animated foil off, Reduce motion, the system's and Fx intensity off each hold every face still, its marks of state drawn", () => {
    const stills: [string, () => void][] = [
      ["Animated foil off", () => writeCardSettings({ animatedFoil: false })],
      ["the Reduce motion setting", () => writeSettings({ reduceMotion: true })],
      ["the system's reduced motion", () => setReducedMotion(true)],
      ["Fx intensity off", () => setFxSettings({ intensity: "off" })],
      ["the Effects setting's Reduce motion", () => setFxSettings({ motion: "reduce" })],
    ];
    for (const [why, still] of stills) {
      still();
      const { container } = render(
        <Board specs={[{ id: MYTHICS[0], over: { health: 1, maxHealth: 4, frozen: true }, attack: 9 }, { id: MYTHICS[1] }]} />,
      );
      const [worn] = minions(container) as [HTMLElement, HTMLElement];
      expect(motions(container), why).toEqual(["static", "static"]);
      expect(container.querySelector("[data-art-moving], [data-art-live]"), why).toBeNull();
      // Damage, a buff and frost are still marks.
      expect(worn.querySelectorAll(".cf-art-cracks path"), why).toHaveLength(3);
      expect(worn.querySelector(".cf-art-glow"), why).not.toBeNull();
      expect(worn.querySelector(".cf-art-frost"), why).not.toBeNull();
      cleanup();
      __resetSettingsForTests();
      setReducedMotion(false);
      resetFxSettingsForTests();
      writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
    }
  });

  it("R3340000 at Fx intensity low only a Mythic, a Legendary or a hovered face takes a slot", () => {
    writeCardLevel(GARY, VERSION_ANIMATED_ID);
    writeCardLevel(LEGENDARY, VERSION_ANIMATED_ID);
    setFxSettings({ intensity: "low" });
    const { container } = render(<Board specs={[{ id: GARY }, { id: LEGENDARY }, { id: MYTHICS[0] }, { id: GARY }]} />);
    expect(motions(container)).toEqual(["static", "animated", "animated", "static"]);
    fire(host(minions(container)[0] as HTMLElement), "pointerenter");
    expect(motions(container)).toEqual(["animated", "animated", "animated", "static"]);
    act(() => {
      setFxSettings({ intensity: "normal" });
    });
    expect(motions(container)).toEqual(["animated", "animated", "animated", "animated"]);
  });
});

describe("R3340000 cards that answer", () => {
  it("R3340000 hover and focus set the face's hover mark and its aim, and leaving clears them", () => {
    const { container } = render(<Board specs={[{ id: COMMON }]} />);
    const cf = minions(container)[0] as HTMLElement;
    vi.spyOn(cf, "getBoundingClientRect").mockReturnValue({ left: 0, top: 0, width: 100, height: 200, right: 100, bottom: 200, x: 0, y: 0, toJSON: () => ({}) });
    expect(cf.getAttribute("data-art-hover")).toBe("false");
    fire(host(cf), "pointerenter");
    expect(cf.getAttribute("data-art-hover")).toBe("true");
    fire(host(cf), "pointermove", { clientX: 75, clientY: 50 });
    expect(cf.style.getPropertyValue("--art-aim-x")).toBe("0.500");
    expect(cf.style.getPropertyValue("--art-aim-y")).toBe("-0.500");
    fire(host(cf), "pointermove", { clientX: 500, clientY: -40 });
    expect(cf.style.getPropertyValue("--art-aim-x")).toBe("1.000");
    expect(cf.style.getPropertyValue("--art-aim-y")).toBe("-1.000");
    fire(host(cf), "pointerleave");
    expect(cf.getAttribute("data-art-hover")).toBe("false");
    expect(cf.style.getPropertyValue("--art-aim-x")).toBe("");
    // Focus is hover for the keyboard.
    fire(host(cf), "focusin");
    expect(cf.getAttribute("data-art-hover")).toBe("true");
    fire(host(cf), "focusout");
    expect(cf.getAttribute("data-art-hover")).toBe("false");
  });

  it("R3340000 a hovered face of a normal look takes a slot and draws its art in layers to answer in", () => {
    const { container } = render(<Board specs={[{ id: COMMON }]} />);
    const cf = minions(container)[0] as HTMLElement;
    expect(cf.hasAttribute("data-art-motion")).toBe(false);
    expect(cf.querySelector("[data-art-live]")).toBeNull();
    fire(host(cf), "pointerenter");
    expect(motionOf(cf)).toBe("animated");
    expect(cf.querySelector('.cf-art[data-art-live="true"]')).not.toBeNull();
    fire(host(cf), "pointerleave");
    expect(cf.hasAttribute("data-art-motion")).toBe(false);
    expect(cf.querySelector("[data-art-live]")).toBeNull();
  });

  it("R3340000 a press sets the press mark and a lift, a cancel or leaving clears it", () => {
    const { container } = render(<Board specs={[{ id: COMMON }]} />);
    const cf = minions(container)[0] as HTMLElement;
    expect(cf.hasAttribute("data-art-press")).toBe(false);
    for (const end of ["pointerup", "pointercancel", "pointerleave"]) {
      fire(host(cf), "pointerdown");
      expect(cf.getAttribute("data-art-press"), end).toBe("true");
      fire(host(cf), end);
      expect(cf.hasAttribute("data-art-press"), end).toBe(false);
    }
  });

  it("R3340000 an attack in flight sets the attack mark and ranks the attacker first", () => {
    writeCardLevel(GARY, VERSION_ANIMATED_ID);
    const specs = (attacking: boolean): Spec[] => [{ id: GARY, attacking }, ...Array.from({ length: MAX }, () => ({ id: GARY }))];
    const { container, rerender } = render(<Board specs={specs(false)} />);
    expect(motions(container)).toEqual(["static", ...Array<string>(MAX).fill("animated")]);
    expect(minions(container)[0]?.hasAttribute("data-art-attacking")).toBe(false);
    act(() => {
      rerender(<Board specs={specs(true)} />);
    });
    const attacker = minions(container)[0] as HTMLElement;
    expect(attacker.getAttribute("data-art-attacking")).toBe("true");
    expect(motionOf(attacker)).toBe("animated");
    expect(attacker.querySelector('[data-art-live="true"] .cf-art-subject, .cf-art[data-art-live="true"] > .cf-art-subject')).not.toBeNull();
    act(() => {
      rerender(<Board specs={specs(false)} />);
    });
    expect(attacker.hasAttribute("data-art-attacking")).toBe(false);
    expect(motionOf(attacker)).toBe("static");
  });

  it("R3340000 a held procedural, Stinkyscape and Training Grounds face draws its picture as a backdrop, a subject and motes", () => {
    writeCardLevel(GARY, VERSION_ANIMATED_ID);
    writeCardLevel("traininggrounds-016", VERSION_ANIMATED_ID);
    for (const id of [GARY, "stinkyscape-006", "traininggrounds-016"]) {
      const { container } = render(<Board specs={[{ id }]} />);
      const art = container.querySelector<HTMLElement>(".cf-art");
      expect(art?.getAttribute("data-art-live"), id).toBe("true");
      const subject = art?.querySelector<HTMLElement>(":scope > .cf-art-layer.cf-art-subject") ?? null;
      const motes = art?.querySelector<HTMLElement>(":scope > .cf-art-layer.cf-art-motes") ?? null;
      expect(subject, `${id}: subject`).not.toBeNull();
      expect(motes, `${id}: motes`).not.toBeNull();
      // The stage carries no particles; the motes carry them alone, the subject only its own layer.
      const stage = documentOf(art);
      expect(stage.querySelector("parsererror"), id).toBeNull();
      expect(stage.querySelectorAll(".art-m-particle"), `${id}: stage`).toHaveLength(0);
      expect(stage.querySelector(".art-m-subject"), `${id}: stage`).toBeNull();
      expect(documentOf(subject).querySelector(".art-m-subject"), `${id}: subject`).not.toBeNull();
      const moteDocument = documentOf(motes);
      expect(moteDocument.querySelectorAll(".art-m-particle"), `${id}: motes`).toHaveLength(PARTICLE_COUNT);
      expect(moteDocument.querySelector(".art-m-subject"), `${id}: motes`).toBeNull();
      cleanup();
    }
  });
});

describe("R3340000 the marks of a unit's state", () => {
  it("R3340000 the cracks grow with the health missing, from none for a whole unit", () => {
    const cracks = (over: Partial<UnitView>): number => {
      const { container } = render(<Board specs={[{ id: COMMON, over }]} />);
      const art = container.querySelector<HTMLElement>(".cf-art");
      const count = art?.querySelectorAll(".cf-art-cracks path").length ?? 0;
      expect(art?.getAttribute("data-art-damage") ?? "0", JSON.stringify(over)).toBe(String(count));
      cleanup();
      return count;
    };
    expect(cracks({ health: 4, maxHealth: 4 })).toBe(0);
    expect(cracks({ health: 3, maxHealth: 4 })).toBe(1);
    expect(cracks({ health: 2, maxHealth: 4 })).toBe(2);
    expect(cracks({ health: 1, maxHealth: 4 })).toBe(3);
    expect(cracks({ health: 1, maxHealth: 30 })).toBe(4);
    const { container } = render(<Board specs={[{ id: COMMON }]} />);
    expect(container.querySelector(".cf-art-wear"), "a whole, unbuffed, unfrozen unit wears nothing").toBeNull();
  });

  it("R3340000 a buffed unit glows in its card's theme colour and a frozen one frosts over", () => {
    const { container } = render(
      <Board specs={[{ id: COMMON, attack: 9, over: { frozen: true } }, { id: MYTHICS[0], attack: 9 }, { id: MYTHICS[1] }]} />,
    );
    const [first, second, plain] = [...container.querySelectorAll<HTMLElement>(".cf-art")] as [HTMLElement, HTMLElement, HTMLElement];
    for (const art of [first, second]) {
      expect(art.getAttribute("data-art-buffed")).toBe("true");
      expect(art.querySelector(".cf-art-glow")).not.toBeNull();
      const theme = art.getAttribute("data-art-theme") as ArtThemeId;
      expect(art.style.getPropertyValue("--art-glow")).toBe(THEME_PALETTES[theme].glow);
    }
    expect(first.getAttribute("data-art-frozen")).toBe("true");
    expect(first.querySelector(".cf-art-frost")).not.toBeNull();
    expect(second.hasAttribute("data-art-frozen")).toBe(false);
    expect(second.querySelector(".cf-art-frost")).toBeNull();
    for (const attribute of ["data-art-buffed", "data-art-frozen", "data-art-damage"]) expect(plain.hasAttribute(attribute), attribute).toBe(false);
    expect(plain.style.getPropertyValue("--art-glow")).toBe("");
    // The marks sit over the picture, and are drawings, hidden from assistive technology like the art.
    expect(first.querySelector(".cf-art-wear")?.getAttribute("aria-hidden")).toBe("true");
  });
});

describe("R3340000 the stylesheet", () => {
  const css = readSheet("src/cards/art/artLive.css").replace(/\/\*[\s\S]*?\*\//g, "");

  it("R3340000 every rule that animates or transitions is inside the reduced-motion gate and the Reduce motion switch's", () => {
    const gate = css.indexOf("@media (prefers-reduced-motion: no-preference)");
    expect(gate).toBeGreaterThan(0);
    const outside = css.slice(0, gate);
    expect(outside).not.toMatch(/animation|transition|@keyframes|\brotate\s*:|\bscale\s*:|\btranslate\s*:/u);
    const inside = css.slice(gate);
    for (const rule of inside.split("}")) {
      if (/animation\s*:|transition\s*:|(?<![-\w])(rotate|scale|translate)\s*:/u.test(rule) && !rule.includes("%")) {
        expect(rule, "a gated rule").toContain(':root:not([data-reduce-motion="true"])');
      }
    }
    // The reactions are the subject's and the motes', by transform properties and opacity alone.
    const properties = [...inside.matchAll(/(?<![-\w])([a-z-]+)\s*:/gu)].map((match) => match[1] ?? "");
    const moving = properties.filter((property) => ["rotate", "scale", "translate", "opacity", "transform", "left", "top", "width", "height", "filter", "margin"].includes(property));
    for (const property of moving) expect(["rotate", "scale", "translate", "opacity"], property).toContain(property);
  });

  it("R3340000 the reactions answer hover, press and attack, and the marks stay outside the gate", () => {
    for (const selector of ['[data-art-hover="true"] .cf-art-subject', '[data-art-press="true"] .cf-art-subject', '[data-art-attacking="true"] .cf-art-subject', '[data-art-attacking="true"] .cf-art-motes', '[data-art-hover="true"] .cf-art-motes']) {
      expect(css).toContain(selector);
    }
    expect(css).toContain("--art-aim-x");
    expect(css).toContain("--fx-lunge-x");
    const gate = css.indexOf("@media (prefers-reduced-motion: no-preference)");
    for (const mark of [".cf-art-cracks path", ".cf-art-glow {", ".cf-art-frost {"]) {
      const at = css.indexOf(mark);
      expect(at, mark).toBeGreaterThan(-1);
      expect(at, mark).toBeLessThan(gate);
    }
  });
});
