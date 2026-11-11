// R3607000: a minion's keyword loops move only while its face holds a motion slot. Every board minion's looping
// keyword treatments (`keywordFxPlan`, `AMBIENT_MAX` of them) run (`data-kw-motion="on"`) only while their face holds
// one of the motion slots (`COSMETIC_FOIL_ANIMATED_MAX`, the hovered face first), and otherwise show their still frame
// (`data-kw-motion="off"`). Off-screen faces, a hidden tab, the shed ladder at `SHED_FACES` and every reduced-motion
// setting hold all loops still; `animatedFoil` off does not; Charge and Rush loop only while the view can act and the
// face holds a slot. Every treatment keeps its still frame and its `data-keyword-fx` attributes throughout.

import { act, cleanup, render } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { Keyword, UnitView } from "@jackioh/shared";

import { resetFxSettingsForTests, setFxSettings } from "../fx/settings.ts";
import { resetShedForTests, setShedLevel } from "../fx/shed.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { resetVisibility, setReducedMotion, setVisibility } from "../test/setup.ts";
import { unit } from "../test/fixtures.ts";
import { COSMETIC_FOIL_ANIMATED_MAX } from "./constants.ts";
import { resetCosmeticsForTests, writeCardPick } from "./cosmetics.ts";
import { FullArtOwned } from "./fullArt.ts";
import { MinionFace } from "./MinionFace.tsx";
import { faceModel } from "./model.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "./settings.ts";
import { keywordFxPlan, planWantsMotion } from "./keywordVisuals.ts";

const MAX = COSMETIC_FOIL_ANIMATED_MAX;
/** A Common Unit with no animated look, so nothing but its keywords wants a slot. */
const COMMON = "core-002";
/** A Mythic, whose art always wants a slot; a Dormant Starship and one of its pieces, whose shipyard idles in one. */
const MYTHIC = "meditative-049";
const DORMANT_STARSHIP = "stinkyscape-t-dormant-starship";
const STARSHIP_PIECE = "stinkyscape-058";

/** Three looping keywords on different layers, all drawn on one unit. */
const LOOPERS: Keyword[] = [{ kind: "Poisonous" }, { kind: "Lifesteal" }, { kind: "Windfury" }];

function defOf(id: string) {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`no ${id}`);
  return def;
}

/** One unit on the board, in a `.card.cf-host` as Card.tsx sets it. */
type Spec = { id: string; over?: Partial<UnitView>; starship?: readonly string[] };

function Minion({ id, over = {}, starship }: Spec): ReactElement {
  const printed = defOf(id).base;
  const health = printed.health ?? 4;
  const list = starship?.map((defId) => ({ defId, radiant: false }));
  const u = unit("p1", {
    defId: id,
    attack: printed.attack ?? 0,
    health,
    maxHealth: health,
    ...(list === undefined
      ? {}
      : { starship: { pieces: list.map(({ defId }) => ({ defId })) }, slumber: {}, canAct: false }),
    ...over,
  });
  const face = faceModel({
    defId: id,
    def: defOf(id),
    radiant: false,
    live: { attack: u.attack, health: u.health, maxHealth: u.maxHealth, keywords: u.keywords },
    ...(list === undefined ? {} : { inPlay: { starship: { pieces: list.length, name: null, list } } }),
  });
  return (
    <div className="card cf-host">
      <MinionFace face={face} unit={u} worn />
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
/** The minions with at least one running keyword loop. */
const loopingMinions = (container: HTMLElement): HTMLElement[] =>
  minions(container).filter((cf) => cf.querySelector('[data-kw-motion="on"]') !== null);
const loopsOn = (container: HTMLElement): HTMLElement[] => [...container.querySelectorAll<HTMLElement>('[data-kw-motion="on"]')];
const host = (cf: HTMLElement): HTMLElement => cf.closest<HTMLElement>(".cf-host") ?? cf;

/** The pointer events a face listens for, on its card's host. */
const fire = (target: Element, type: string, init: MouseEventInit = {}): void => {
  act(() => {
    target.dispatchEvent(new MouseEvent(type, { bubbles: true, ...init }));
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

/** The observer watching `face`. */
function observerFor(face: HTMLElement): FakeObserver {
  const found = FakeObserver.all.find((observer) => observer.targets.includes(face));
  if (found === undefined) throw new Error("no observer");
  return found;
}

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
  resetFxSettingsForTests();
  resetShedForTests();
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
  resetShedForTests();
  localStorage.clear();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
});

describe("R3607000 a minion's keyword loops move only while its face holds a motion slot", () => {
  it("R3607000 a full board moves keyword loops on at most MAX minions, and the hovered minion is one of them", () => {
    expect(MAX).toBe(6);
    const { container } = render(<Board specs={Array.from({ length: 14 }, () => ({ id: COMMON, over: { keywords: LOOPERS } }))} />);
    const faces = minions(container);
    expect(faces).toHaveLength(14);
    // Every treatment keeps its still frame, its attributes and its chip, moving or not.
    for (const cf of faces) {
      expect([...cf.querySelectorAll("[data-keyword-fx]")].map((el) => el.getAttribute("data-keyword-fx")).sort()).toEqual([
        "Lifesteal",
        "Poisonous",
        "Windfury",
      ]);
      expect(cf.querySelectorAll(".keywords > [data-keyword]")).toHaveLength(3);
    }
    expect(loopingMinions(container).length).toBeLessThanOrEqual(MAX);
    expect(loopingMinions(container).length).toBeGreaterThan(0);
    const oldest = faces[0] as HTMLElement;
    expect(oldest.querySelector('[data-kw-motion="on"]'), "the oldest face holds still").toBeNull();
    for (const fx of oldest.querySelectorAll("[data-keyword-fx]")) expect(fx.getAttribute("data-kw-motion")).toBe("off");

    // The hovered face ranks first: it takes a slot, and the board still moves at most MAX minions.
    fire(host(oldest), "pointerenter");
    expect(oldest.querySelector('[data-kw-motion="on"]'), "the hovered face moves").not.toBeNull();
    expect(loopingMinions(container).length).toBeLessThanOrEqual(MAX);
  });

  it("R3607000 a face off screen holds its keyword loops still", () => {
    vi.stubGlobal("IntersectionObserver", FakeObserver);
    const { container } = render(<Board specs={[{ id: COMMON, over: { keywords: LOOPERS } }]} />);
    const [face] = minions(container) as [HTMLElement];
    expect(face.querySelectorAll('[data-kw-motion="on"]')).toHaveLength(2);
    observerFor(face).report(false);
    expect(loopsOn(container)).toHaveLength(0);
    expect(face.querySelectorAll("[data-keyword-fx]")).toHaveLength(3);
    for (const fx of face.querySelectorAll("[data-keyword-fx]")) expect(fx.getAttribute("data-kw-motion")).toBe("off");
    observerFor(face).report(true);
    expect(face.querySelectorAll('[data-kw-motion="on"]')).toHaveLength(2);
  });

  it("R3607000 a hidden tab holds every keyword loop still", () => {
    const { container } = render(
      <Board specs={[{ id: COMMON, over: { keywords: LOOPERS } }, { id: COMMON, over: { keywords: LOOPERS } }]} />,
    );
    expect(loopsOn(container)).toHaveLength(4);
    act(() => {
      setVisibility("hidden");
    });
    expect(loopsOn(container)).toHaveLength(0);
    expect(container.querySelectorAll("[data-keyword-fx]").length).toBeGreaterThan(0);
    act(() => {
      setVisibility("visible");
    });
    expect(loopsOn(container)).toHaveLength(4);
  });

  it("R3607000 at SHED_FACES every keyword loop holds still (no hover)", () => {
    const { container } = render(
      <Board specs={[{ id: COMMON, over: { keywords: LOOPERS } }, { id: COMMON, over: { keywords: LOOPERS } }]} />,
    );
    expect(loopsOn(container)).toHaveLength(4);
    act(() => {
      setShedLevel(3);
    });
    expect(loopsOn(container)).toHaveLength(0);
    expect(container.querySelectorAll("[data-keyword-fx]").length).toBeGreaterThan(0);
    act(() => {
      setShedLevel(0);
    });
    expect(loopsOn(container)).toHaveLength(4);
  });

  it("R3607000 reduced motion runs no keyword loop, but Animated foil off does not still them", () => {
    writeCardSettings({ animatedFoil: false });
    const { container } = render(<Board specs={[{ id: COMMON, over: { keywords: LOOPERS } }]} />);
    expect(loopsOn(container)).toHaveLength(2);
    cleanup();
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });

    const stills: [string, () => void][] = [
      ["the Reduce motion setting", () => writeSettings({ reduceMotion: true })],
      ["the system's reduced motion", () => setReducedMotion(true)],
      ["Fx intensity off", () => setFxSettings({ intensity: "off" })],
      ["the Effects setting's Reduce motion", () => setFxSettings({ motion: "reduce" })],
    ];
    for (const [why, still] of stills) {
      still();
      const { container: stillContainer } = render(<Board specs={[{ id: COMMON, over: { keywords: LOOPERS } }]} />);
      expect(loopsOn(stillContainer), why).toHaveLength(0);
      expect(stillContainer.querySelectorAll("[data-keyword-fx]").length, `${why} keeps the still marks`).toBeGreaterThan(0);
      cleanup();
      localStorage.clear();
      __resetSettingsForTests();
      setReducedMotion(false);
      resetFxSettingsForTests();
      writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
    }
  });

  it("R3607000 Animated foil off stills a keyword face's art outputs but not its loops", () => {
    writeCardPick(COMMON, "foil", "foil-holo-sweep");
    const specs: readonly Spec[] = [
      { id: MYTHIC, over: { keywords: LOOPERS } },
      { id: COMMON, over: { keywords: LOOPERS } },
      { id: DORMANT_STARSHIP, over: { keywords: LOOPERS }, starship: [STARSHIP_PIECE] },
    ];
    const { container } = render(<Board specs={specs} />);
    const [mythic, common, yard] = minions(container) as [HTMLElement, HTMLElement, HTMLElement];
    expect(mythic.getAttribute("data-art-motion"), "the Mythic's art moves").toBe("animated");
    expect(mythic.hasAttribute("data-art-response"), "the pointer answer is armed").toBe(true);
    expect(common.getAttribute("data-cosmetic-motion"), "the worn foil moves").toBe("animated");
    expect(yard.querySelector(".cf-shipyard-idle"), "the shipyard idles").not.toBeNull();
    const moving = loopsOn(container).length;
    expect(moving).toBeGreaterThan(0);
    cleanup();

    // With Animated foil off every face still holds its slot for its loops, but no art output moves.
    writeCardSettings({ animatedFoil: false });
    const still = render(<Board specs={specs} />);
    const [stillMythic, stillCommon, stillYard] = minions(still.container) as [HTMLElement, HTMLElement, HTMLElement];
    expect(loopsOn(still.container), "the loops keep their slots").toHaveLength(moving);
    expect(stillMythic.getAttribute("data-art-motion"), "the Mythic's art holds still").toBe("static");
    expect(stillMythic.hasAttribute("data-art-response"), "the pointer answer holds still").toBe(false);
    expect(stillCommon.getAttribute("data-cosmetic-motion"), "the foil holds still").toBe("static");
    expect(stillYard.querySelector(".cf-shipyard-idle"), "the shipyard holds still").toBeNull();

    // Hovering ranks the face first for its loops, not for its art.
    fire(host(stillMythic), "pointerenter");
    expect(stillMythic.querySelector('[data-kw-motion="on"]'), "the hovered face loops").not.toBeNull();
    expect(stillMythic.getAttribute("data-art-motion"), "its art still holds").toBe("static");
    expect(stillMythic.hasAttribute("data-art-response"), "its pointer answer still holds").toBe(false);
  });

  it("R3607000 Charge and Rush loop only while the unit can act and its face holds a slot", () => {
    const keywords: Keyword[] = [{ kind: "Charge" }, { kind: "Rush" }, { kind: "First Strike" }];
    const { container } = render(<Board specs={[{ id: COMMON, over: { keywords, canAct: true } }]} />);
    const [face] = minions(container) as [HTMLElement];
    expect(face.querySelector('[data-keyword-fx="Charge"]')?.getAttribute("data-kw-motion")).toBe("on");
    expect(face.querySelector('[data-keyword-fx="Rush"]')?.getAttribute("data-kw-motion")).toBe("on");
    expect(face.querySelector('[data-keyword-fx="First Strike"]')?.getAttribute("data-kw-motion")).toBe("off");
    cleanup();

    const rested = render(<Board specs={[{ id: COMMON, over: { keywords, canAct: false } }]} />);
    const [still] = minions(rested.container) as [HTMLElement];
    expect(still.querySelector('[data-keyword-fx="Charge"]')?.getAttribute("data-kw-motion")).toBe("off");
    expect(still.querySelector('[data-keyword-fx="Rush"]')?.getAttribute("data-kw-motion")).toBe("off");
    expect(still.querySelector('[data-keyword-fx="First Strike"]')?.getAttribute("data-kw-motion")).toBe("on");
    expect(still.querySelector('[data-keyword-fx="Charge"] .kw-streaks'), "Charge keeps its streaks").not.toBeNull();
    expect(still.querySelector('[data-keyword-fx="Rush"] .kw-dashes'), "Rush keeps its dashes").not.toBeNull();
    cleanup();

    // On a full board the slot-less oldest face shows Charge still, its streaks drawn.
    const full = render(
      <Board specs={Array.from({ length: 14 }, () => ({ id: COMMON, over: { keywords, canAct: true } }))} />,
    );
    const oldest = minions(full.container)[0] as HTMLElement;
    expect(oldest.querySelector('[data-kw-motion="on"]')).toBeNull();
    const charge = oldest.querySelector('[data-keyword-fx="Charge"]');
    expect(charge?.getAttribute("data-kw-motion")).toBe("off");
    expect(charge?.querySelector(".kw-streaks"), "the still streaks are drawn").not.toBeNull();
  });

  it("R3607000 planWantsMotion is true exactly when the plan loops", () => {
    expect(planWantsMotion(keywordFxPlan({ keywords: [{ kind: "Poisonous" }], canAct: true, armor: 0 }))).toBe(true);
    expect(planWantsMotion(keywordFxPlan({ keywords: [{ kind: "Charge" }], canAct: true, armor: 0 }))).toBe(true);
    expect(planWantsMotion(keywordFxPlan({ keywords: [{ kind: "Taunt" }], canAct: true, armor: 0 }))).toBe(false);
    expect(planWantsMotion(keywordFxPlan({ keywords: [{ kind: "Charge" }], canAct: false, armor: 0 }))).toBe(false);
    expect(planWantsMotion(keywordFxPlan({ keywords: [], canAct: true, armor: 0 }))).toBe(false);
  });
});
