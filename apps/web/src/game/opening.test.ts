// #3347: the opening deal's plan, the properties that aim it, and the motion level the settings allow.

import { describe, expect, it } from "vitest";

import { cardMotionLevel } from "./cardMotion.ts";
import { DEAL_FLIGHT_MS, DEAL_STAGGER_MS, RIFFLE_MS } from "./config.ts";
import { aimDeal, clearDeal, planOpening } from "./opening.ts";
import { baseView, card, emptySide } from "../test/fixtures.ts";

/** A mulligan view: `mine` cards in the viewer's hand, `theirs` backs in the opponent's. */
const dealt = (mine: number, theirs: number, phase: "mulligan" | "main" = "mulligan") =>
  baseView({
    phase,
    you: emptySide("p1", { hand: Array.from({ length: mine }, () => card()) }),
    opponent: emptySide("p2", { hand: { count: theirs } }),
  });

describe("#3347 planOpening", () => {
  it("#3347 is null outside the mulligan phase, and for hands with nothing in them", () => {
    expect(planOpening(dealt(3, 3, "main"), 1)).toBeNull();
    expect(planOpening(dealt(0, 0), 1)).toBeNull();
  });

  it("#3347 deals one card at a time, you first, alternating, until each hand's count is spent", () => {
    const plan = planOpening(dealt(3, 4), 1);
    expect(plan?.deals.map((deal) => deal.side)).toEqual(["you", "opponent", "you", "opponent", "you", "opponent", "opponent"]);
  });

  it("#3347 reads only counts: a hand of cards and a hand of backs deal alike", () => {
    expect(planOpening(dealt(2, 2), 1)?.deals).toHaveLength(4);
    expect(planOpening(dealt(0, 2), 1)?.deals.map((deal) => deal.side)).toEqual(["opponent", "opponent"]);
  });

  it("#3347 deals are exactly DEAL_STAGGER_MS apart after the riffle, and the last has DEAL_FLIGHT_MS to land", () => {
    const plan = planOpening(dealt(3, 3), 1);
    expect(plan).not.toBeNull();
    if (plan === null) return;
    expect(plan.riffleMs).toBe(RIFFLE_MS);
    expect(plan.staggerMs).toBe(DEAL_STAGGER_MS);
    expect(plan.flightMs).toBe(DEAL_FLIGHT_MS);
    plan.deals.forEach((deal, k) => expect(deal.atMs).toBe(RIFFLE_MS + k * DEAL_STAGGER_MS));
    const last = plan.deals[plan.deals.length - 1];
    expect(plan.totalMs).toBe((last?.atMs ?? 0) + DEAL_FLIGHT_MS);
  });

  it("#3347 speed 2 halves every figure (R201), and speed 1 changes none", () => {
    const fast = planOpening(dealt(3, 3), 2);
    expect(fast?.riffleMs).toBe(RIFFLE_MS / 2);
    expect(fast?.staggerMs).toBe(DEAL_STAGGER_MS / 2);
    expect(fast?.flightMs).toBe(DEAL_FLIGHT_MS / 2);
    expect(fast?.deals[1]?.atMs).toBe(RIFFLE_MS / 2 + DEAL_STAGGER_MS / 2);
    const normal = planOpening(dealt(3, 3), 1);
    expect(fast?.totalMs).toBe(Math.round((normal?.totalMs ?? 0) / 2));
  });
});

describe("#3347 cardMotionLevel", () => {
  it("#3347 reduced motion is still, whatever the intensity", () => {
    expect(cardMotionLevel(true, "normal")).toBe("still");
    expect(cardMotionLevel(true, "off")).toBe("still");
  });

  it("#3347 effects off is calm: the sounds stay, the flourishes go", () => {
    expect(cardMotionLevel(false, "off")).toBe("calm");
  });

  it("#3347 anything else is full", () => {
    for (const intensity of ["low", "normal", "high"] as const) expect(cardMotionLevel(false, intensity)).toBe("full");
  });
});

describe("#3347 aimDeal and clearDeal", () => {
  /** A board of two decks and two hands, each rect stubbed: jsdom lays nothing out. */
  function board(): HTMLElement {
    const root = document.createElement("div");
    root.innerHTML = `
      <span data-testid="library-you"></span>
      <span data-testid="library-opponent"></span>
      <div data-testid="hand-you"><div class="hand-slot"></div><div class="hand-slot"></div></div>
      <div data-testid="hand-opponent"><div class="hand-slot"></div></div>`;
    const rect = (left: number, top: number): DOMRect =>
      ({ left, top, width: 20, height: 20, right: left + 20, bottom: top + 20, x: left, y: top, toJSON: () => ({}) }) as DOMRect;
    const at = (selector: string, index: number, left: number, top: number): void => {
      const element = root.querySelectorAll(selector)[index];
      if (element === undefined) throw new Error(selector);
      element.getBoundingClientRect = () => rect(left, top);
    };
    at('[data-testid="library-you"]', 0, 500, 400);
    at('[data-testid="library-opponent"]', 0, 500, 0);
    at('[data-testid="hand-you"] .hand-slot', 0, 100, 400);
    at('[data-testid="hand-you"] .hand-slot', 1, 200, 400);
    at('[data-testid="hand-opponent"] .hand-slot', 0, 300, 0);
    document.body.appendChild(root);
    return root;
  }

  it("#3347 aims each slot from its own deck, at the time its card is dealt", () => {
    const root = board();
    const plan = planOpening(dealt(2, 1), 1);
    if (plan === null) throw new Error("no plan");
    aimDeal(root, plan);
    const yours = root.querySelectorAll<HTMLElement>('[data-testid="hand-you"] .hand-slot');
    const theirs = root.querySelectorAll<HTMLElement>('[data-testid="hand-opponent"] .hand-slot');
    // the deck's centre minus the slot's centre
    expect(yours[0]?.style.getPropertyValue("--deal-dx")).toBe("400px");
    expect(yours[0]?.style.getPropertyValue("--deal-dy")).toBe("0px");
    expect(yours[1]?.style.getPropertyValue("--deal-dx")).toBe("300px");
    expect(theirs[0]?.style.getPropertyValue("--deal-dx")).toBe("200px");
    // you, opponent, you
    expect(yours[0]?.style.getPropertyValue("--deal-at")).toBe(`${String(plan.deals[0]?.atMs)}ms`);
    expect(theirs[0]?.style.getPropertyValue("--deal-at")).toBe(`${String(plan.deals[1]?.atMs)}ms`);
    expect(yours[1]?.style.getPropertyValue("--deal-at")).toBe(`${String(plan.deals[2]?.atMs)}ms`);
    root.remove();
  });

  it("#3347 clearDeal takes all three properties off every slot", () => {
    const root = board();
    const plan = planOpening(dealt(2, 1), 1);
    if (plan === null) throw new Error("no plan");
    aimDeal(root, plan);
    clearDeal(root);
    for (const slot of root.querySelectorAll<HTMLElement>(".hand-slot")) {
      for (const property of ["--deal-dx", "--deal-dy", "--deal-at"]) expect(slot.style.getPropertyValue(property), property).toBe("");
    }
    root.remove();
  });
});
