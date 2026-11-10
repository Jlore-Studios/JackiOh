// R3615000: once an attacker is picked (a click) or dragged, hovering a legal target with a forecast
// entry previews that one attack alone: the target wears its own damage and the attacker wears what it
// takes back, each as a tentative hit marker. Picking or lifting shows nothing yet, leaving removes it,
// and the client computes none of it (CLAUDE.md rule 7).

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import type { ActionBody, AttackForecast, CombatForecast, PlayerView } from "@jackioh/shared";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { setAudioEngineForTests } from "../audio/engine.ts";
import { resetAudioSettingsForTests } from "../audio/settings.ts";
import { fakeAudioEngine } from "../audio/test/fakeAudio.ts";
import type { AudioEngine } from "../audio/types.ts";
import { resetFxSettingsForTests, setFxSettings } from "../fx/settings.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/index.ts";
import { baseView, emptySide, unit } from "../test/fixtures.ts";
import { ATTACK_OUTCOME_NOTE } from "./AttackOutcome.tsx";
import { DRAG_THRESHOLD_PX } from "./drag/model.ts";
import Game from "./Game.tsx";

const HERE = dirname(fileURLToPath(import.meta.url));

/** attackOutcome.css without comments, as `{ selectors, body }` rules (innermost blocks). */
function cssRules(): { selectors: string[]; body: string }[] {
  const css = readFileSync(join(HERE, "attackOutcome.css"), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
  return [...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)].map((match) => ({
    selectors: (match[1] ?? "").split(",").map((selector) => selector.trim().replace(/\s+/g, " ")),
    body: (match[2] ?? "").replace(/\s+/g, " ").trim(),
  }));
}

/** The tentative modifier's rule body: jsdom lays no CSS out, so stillness and ghosting are proved by the rule. */
function tentativeBody(selector: string): string {
  const rule = cssRules().find((entry) => entry.selectors.includes(selector));
  if (rule === undefined) throw new Error(`missing CSS rule for ${selector}`);
  return rule.body;
}

function tentativeOpacity(body: string): number {
  const opacity = /opacity:\s*([\d.]+)/.exec(body)?.[1];
  expect(opacity).toBeDefined();
  return Number(opacity);
}

/** u1 attacks e1 or the hero; u3 only the hero; e1 is the opponent's. */
function aimView(): PlayerView {
  return baseView({
    you: emptySide("p1", {
      units: [
        unit("p1", { instanceId: "u1", defId: "core-066" }),
        null,
        unit("p1", { instanceId: "u3", defId: "core-011" }),
        null,
        null,
      ],
    }),
    opponent: emptySide("p2", {
      hand: { count: 3 },
      units: [unit("p2", { instanceId: "e1", defId: "core-055" }), null, null, null, null],
    }),
  });
}

const LEGAL: readonly ActionBody[] = [
  { type: "attack", attackerId: "u1", targetId: "e1" },
  { type: "attack", attackerId: "u1", targetId: "hero-p2" },
  { type: "attack", attackerId: "u3", targetId: "hero-p2" },
  { type: "endTurn" },
  { type: "concede" },
];

function entry(attackerId: string, targetId: string, fields: Partial<AttackForecast>): AttackForecast {
  return {
    attackerId,
    targetId,
    dealt: 0,
    taken: 0,
    kills: false,
    survives: true,
    pops: false,
    attackerSize: 0,
    targetSize: 0,
    ...fields,
  };
}

/** Numbers no client could work out from the view's stats: the markers must be reading the forecast. */
const U1_E1 = entry("u1", "e1", { dealt: 2, taken: 3, kills: true, survives: false, pops: true });
const U1_HERO = entry("u1", "hero-p2", { dealt: 2 });
const U3_HERO = entry("u3", "hero-p2", { dealt: 5 });
const FORECAST: CombatForecast = { attacks: [U1_E1, U1_HERO, U3_HERO], lethal: false };

function renderGame(forecast: CombatForecast | undefined, replay = false) {
  const onAction = vi.fn<(body: ActionBody) => void>();
  const utils = render(
    <Game view={aimView()} legal={LEGAL} onAction={onAction} forecast={forecast} replay={replay} />,
  );
  return { ...utils, onAction };
}

const markers = (): HTMLElement[] => screen.queryAllByTestId(/^outcome-/);
const marker = (target: string): HTMLElement => screen.getByTestId(`outcome-${target}`);
const takenMarker = (attacker: string): HTMLElement => screen.getByTestId(`outcome-taken-${attacker}`);
const marks = (element: HTMLElement): string[] =>
  [...element.querySelectorAll("[data-mark]")].map((mark) => mark.getAttribute("data-mark") ?? "");

const POINTER = 1;
const START = { x: 200, y: 400 };
let under: Element[] = [];

function lift(source: Element): void {
  under = [source];
  fireEvent.pointerDown(source, { pointerId: POINTER, button: 0, clientX: START.x, clientY: START.y });
  fireEvent.pointerMove(window, { pointerId: POINTER, button: 0, clientX: START.x, clientY: START.y - DRAG_THRESHOLD_PX });
}

/** Puts `element` under the pointer (null: bare board); the hovered-target reader runs on the move. */
function hover(element: Element | null): void {
  under = element === null ? [] : [element];
  fireEvent.pointerMove(window, { clientX: 10, clientY: 10 });
}

beforeEach(() => {
  under = [];
  document.elementsFromPoint = (() => under) as Document["elementsFromPoint"];
  localStorage.clear();
  resetAudioSettingsForTests();
  setAudioEngineForTests(fakeAudioEngine({ contextsCreated: vi.fn<AudioEngine["contextsCreated"]>(() => 1) }));
});

afterEach(() => {
  cleanup();
  setAudioEngineForTests(null);
  delete (document as Partial<Document>).elementsFromPoint;
  document.documentElement.removeAttribute("data-dragging");
  localStorage.clear();
  __resetSettingsForTests();
  resetAudioSettingsForTests();
  resetFxSettingsForTests();
});

describe("R3615000 a hovered attack's preview wears each number on the card that takes it", () => {
  it("R3615000 with an attacker selected and nothing hovered no marker renders", () => {
    const { onAction } = renderGame(FORECAST);
    expect(markers()).toEqual([]);

    fireEvent.click(screen.getByTestId("card-u1"));

    expect(markers()).toEqual([]);
    expect(onAction).not.toHaveBeenCalled();
  });

  it("R3615000 hovering a unit target shows dealt on the target and taken on the attacker", () => {
    renderGame(FORECAST);
    fireEvent.click(screen.getByTestId("card-u1"));
    expect(markers()).toEqual([]);

    hover(screen.getByTestId("card-e1"));

    const target = marker("card-e1");
    expect(screen.getByTestId("card-e1").contains(target)).toBe(true);
    expect(target).toHaveAttribute("data-dealt", "2");
    expect(target.querySelector(".fx-splat")).toHaveAttribute("data-amount", "−2");
    const taken = takenMarker("card-u1");
    expect(screen.getByTestId("card-u1").contains(taken)).toBe(true);
    expect(taken).toHaveAttribute("data-taken", "3");
    expect(taken.querySelector(".fx-splat")).toHaveAttribute("data-amount", "−3");
    // Every other legal target renders none.
    expect(screen.queryByTestId("outcome-hero-opponent")).toBeNull();

    // Moving the hover to another target moves both markers.
    hover(screen.getByTestId("hero-opponent"));
    expect(screen.queryByTestId("outcome-card-e1")).toBeNull();
    expect(screen.queryByTestId("outcome-taken-card-u1")).toBeNull();
    const hero = marker("hero-opponent");
    expect(hero).toHaveAttribute("data-dealt", "2");
    expect(hero.querySelector(".fx-splat")).toHaveAttribute("data-amount", "−2");

    // Leaving all targets removes them.
    hover(null);
    expect(markers()).toEqual([]);
  });

  it("R3615000 hovering a hero target shows the target marker alone, and taken 0 shows no attacker marker", () => {
    const { unmount } = renderGame(FORECAST);
    fireEvent.click(screen.getByTestId("card-u1"));

    hover(screen.getByTestId("hero-opponent"));
    expect(marker("hero-opponent")).toHaveAttribute("data-dealt", "2");
    expect(screen.queryByTestId("outcome-taken-card-u1")).toBeNull();
    hover(null);
    unmount();

    // A strike back of 0 shows no marker on the attacker.
    renderGame({ attacks: [entry("u1", "e1", { dealt: 2, taken: 0 })], lethal: false });
    fireEvent.click(screen.getByTestId("card-u1"));
    hover(screen.getByTestId("card-e1"));
    expect(marker("card-e1")).toHaveAttribute("data-dealt", "2");
    expect(screen.queryByTestId("outcome-taken-card-u1")).toBeNull();
  });

  it("R3615000 kills marks the target, a fallen attacker marks its own, pops adds the shield", () => {
    renderGame(FORECAST);
    fireEvent.click(screen.getByTestId("card-u1"));

    hover(screen.getByTestId("card-e1"));
    // e1's entry: kills, the attacker falls, and a shield breaks.
    const target = marker("card-e1");
    expect(marks(target).sort()).toEqual(["kills", "pops"]);
    expect(target).toHaveAttribute("data-kills", "true");
    expect(target).toHaveAttribute("data-pops", "true");
    expect(target.querySelector('[data-mark="kills"]')).not.toBeNull();
    const taken = takenMarker("card-u1");
    expect(marks(taken)).toEqual(["falls"]);
    expect(taken).toHaveAttribute("data-falls", "true");
    // The hero's entry: no kill, no shield.
    hover(screen.getByTestId("hero-opponent"));
    expect(marks(marker("hero-opponent"))).toEqual([]);
  });

  it("R3615000 a kill on the hero wears the mark, and a target that survives with its attacker none", () => {
    renderGame({
      attacks: [entry("u1", "e1", { dealt: 1, taken: 1 }), entry("u1", "hero-p2", { dealt: 2, kills: true })],
      lethal: true,
    });
    fireEvent.click(screen.getByTestId("card-u1"));

    hover(screen.getByTestId("card-e1"));
    expect(marks(marker("card-e1"))).toEqual([]);
    expect(marker("card-e1")).not.toHaveAttribute("data-kills");
    expect(screen.queryByTestId("outcome-taken-card-u1")).not.toBeNull();

    hover(screen.getByTestId("hero-opponent"));
    expect(marks(marker("hero-opponent"))).toEqual(["kills"]);
  });

  it("R3615000 a dragged attacker previews its hovered target", () => {
    renderGame(FORECAST);

    lift(screen.getByTestId("card-u3"));

    expect(document.documentElement).toHaveAttribute("data-dragging", "attack");
    expect(markers()).toEqual([]);

    hover(screen.getByTestId("hero-opponent"));
    // The hero's is u3's entry, and a hero strikes nothing back.
    expect(marker("hero-opponent")).toHaveAttribute("data-dealt", "5");
    expect(screen.queryByTestId("outcome-taken-card-u3")).toBeNull();

    hover(null);
    expect(markers()).toEqual([]);
  });

  it("R3615000 with effects on the marker is the splat with the tentative modifier and holds still", () => {
    renderGame(FORECAST);
    fireEvent.click(screen.getByTestId("card-u1"));
    hover(screen.getByTestId("card-e1"));

    for (const element of [marker("card-e1"), takenMarker("card-u1")]) {
      const splat = element.querySelector(".fx-splat");
      expect(splat).not.toBeNull();
      expect(splat).toHaveAttribute("data-tentative", "true");
      expect(element).toHaveAttribute("data-tentative", "true");
      // Exactly one style of marker is ever visible: no number variant beside the splat.
      expect(element.querySelector(".damage-pop")).toBeNull();
    }
    expect(marker("card-e1").querySelector(".fx-splat")).toHaveAttribute("data-amount", "−2");
    expect(takenMarker("card-u1").querySelector(".fx-splat")).toHaveAttribute("data-amount", "−3");

    // The tentative modifier holds it still at reduced opacity (jsdom lays no CSS out; the rule proves it).
    const body = tentativeBody('.fx-splat[data-tentative="true"]');
    expect(body).toContain("animation: none");
    expect(tentativeOpacity(body)).toBeLessThan(1);
  });

  it("R3615000 with effects off the marker is damage-pop with the modifier", () => {
    setFxSettings({ intensity: "off" });
    renderGame(FORECAST);
    fireEvent.click(screen.getByTestId("card-u1"));
    hover(screen.getByTestId("card-e1"));

    const target = marker("card-e1");
    const pop = target.querySelector(".damage-pop");
    expect(pop).not.toBeNull();
    expect(pop).toHaveAttribute("data-tentative", "true");
    expect(target).toHaveTextContent("−2");
    expect(target.querySelector(".fx-splat")).toBeNull();
    const taken = takenMarker("card-u1");
    expect(taken.querySelector(".damage-pop")).toHaveAttribute("data-tentative", "true");
    expect(taken).toHaveTextContent("−3");
    expect(taken.querySelector(".fx-splat")).toBeNull();

    const body = tentativeBody('.damage-pop[data-tentative="true"]');
    expect(body).toContain("animation: none");
    expect(tentativeOpacity(body)).toBeLessThan(1);
  });

  it("R3615000 each marker's title and accessible name end on ATTACK_OUTCOME_NOTE", () => {
    renderGame(FORECAST);
    fireEvent.click(screen.getByTestId("card-u1"));
    hover(screen.getByTestId("card-e1"));

    // It is no promise: each marker's title and its name say a face-down trap can change it.
    for (const element of [marker("card-e1"), takenMarker("card-u1")]) {
      expect(element).toHaveAttribute("title", ATTACK_OUTCOME_NOTE);
      expect(element.getAttribute("aria-label")).toContain(ATTACK_OUTCOME_NOTE);
    }
  });

  it("R3615000 a replay renders no marker", () => {
    renderGame(FORECAST, true);

    fireEvent.click(screen.getByTestId("card-u1"));
    hover(screen.getByTestId("card-e1"));

    expect(markers()).toEqual([]);
  });
});

describe("R3477000 an aimed attack's targets show the engine's forecast", () => {
  it("R3477000 the setting turned off hides every marker", () => {
    writeSettings({ attackOutcomes: false });
    const { unmount } = renderGame(FORECAST);

    fireEvent.click(screen.getByTestId("card-u1"));
    expect(screen.getByTestId("card-e1")).toHaveAttribute("data-legal", "true");
    hover(screen.getByTestId("card-e1"));
    expect(markers()).toEqual([]);
    unmount();

    renderGame(FORECAST);
    lift(screen.getByTestId("card-u3"));
    expect(document.documentElement).toHaveAttribute("data-dragging", "attack");
    hover(screen.getByTestId("hero-opponent"));
    expect(markers()).toEqual([]);
  });

  it("R3477000 no forecast, a target without an entry, or no attacker picked renders nothing", () => {
    const { unmount } = renderGame(undefined);
    fireEvent.click(screen.getByTestId("card-u1"));
    expect(screen.getByTestId("card-e1")).toHaveAttribute("data-legal", "true");
    hover(screen.getByTestId("card-e1"));
    expect(markers()).toEqual([]);
    unmount();

    // u1's attack on the hero has no entry: hovering it shows nothing while e1 still shows its own.
    renderGame({ attacks: [U1_E1, U3_HERO], lethal: false });
    hover(screen.getByTestId("card-e1"));
    expect(markers()).toEqual([]);
    fireEvent.click(screen.getByTestId("card-u1"));
    expect(screen.getByTestId("hero-opponent")).toHaveAttribute("data-legal", "true");
    hover(screen.getByTestId("hero-opponent"));
    expect(markers()).toEqual([]);
    hover(screen.getByTestId("card-e1"));
    expect(marker("card-e1")).toHaveAttribute("data-dealt", "2");
    expect(takenMarker("card-u1")).toHaveAttribute("data-taken", "3");
  });
});
