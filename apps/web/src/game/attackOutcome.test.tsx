// R3477000: while an attacker is picked (a click) or dragged, each legal target shows the engine's
// forecast entry for that attack, as sent; the client computes none of it (CLAUDE.md rule 7).

import type { ActionBody, AttackForecast, CombatForecast, PlayerView } from "@jackioh/shared";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { setAudioEngineForTests } from "../audio/engine.ts";
import { resetAudioSettingsForTests } from "../audio/settings.ts";
import { fakeAudioEngine } from "../audio/test/fakeAudio.ts";
import type { AudioEngine } from "../audio/types.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/index.ts";
import { baseView, emptySide, unit } from "../test/fixtures.ts";
import { ATTACK_OUTCOME_NOTE } from "./AttackOutcome.tsx";
import { DRAG_THRESHOLD_PX } from "./drag/model.ts";
import Game from "./Game.tsx";

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

/** Numbers no client could work out from the view's stats: the chip must be reading the forecast. */
const U1_E1 = entry("u1", "e1", { dealt: 2, taken: 3, kills: true, survives: false, pops: true });
const U1_HERO = entry("u1", "hero-p2", { dealt: 2 });
const U3_HERO = entry("u3", "hero-p2", { dealt: 5 });
const FORECAST: CombatForecast = { attacks: [U1_E1, U1_HERO, U3_HERO], lethal: false };

function renderGame(forecast: CombatForecast | undefined) {
  const onAction = vi.fn<(body: ActionBody) => void>();
  const utils = render(<Game view={aimView()} legal={LEGAL} onAction={onAction} forecast={forecast} />);
  return { ...utils, onAction };
}

const chips = (): HTMLElement[] => screen.queryAllByTestId(/^outcome-/);
const chip = (target: string): HTMLElement => screen.getByTestId(`outcome-${target}`);
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
});

describe("R3477000 an aimed attack's targets show the engine's forecast", () => {
  it("R3477000 with an attacker selected each legal target shows its forecast's dealt and taken", () => {
    const { onAction } = renderGame(FORECAST);
    expect(chips()).toEqual([]);

    fireEvent.click(screen.getByTestId("card-u1"));

    expect(chips().map((element) => element.getAttribute("data-testid")).sort()).toEqual([
      "outcome-card-e1",
      "outcome-hero-opponent",
    ]);
    const unitChip = chip("card-e1");
    expect(screen.getByTestId("card-e1").contains(unitChip)).toBe(true);
    expect(unitChip).toHaveAttribute("data-dealt", "2");
    expect(unitChip).toHaveAttribute("data-taken", "3");
    expect(unitChip).toHaveTextContent("−2");
    expect(unitChip).toHaveTextContent("↩−3");
    // The hero's is u1's entry, not u3's, and a hero strikes nothing back.
    const heroChip = chip("hero-opponent");
    expect(screen.getByTestId("hero-opponent").contains(heroChip)).toBe(true);
    expect(heroChip).toHaveAttribute("data-dealt", "2");
    expect(heroChip).not.toHaveAttribute("data-taken");
    expect(heroChip).not.toHaveTextContent("↩");
    // It is no promise: its title and its name say a face-down trap can change it.
    for (const element of [unitChip, heroChip]) {
      expect(element).toHaveAttribute("title", ATTACK_OUTCOME_NOTE);
      expect(element.getAttribute("aria-label")).toContain(ATTACK_OUTCOME_NOTE);
    }
    expect(onAction).not.toHaveBeenCalled();
  });

  it("R3477000 a kill mark shows exactly when kills is true", () => {
    renderGame(FORECAST);
    fireEvent.click(screen.getByTestId("card-u1"));

    // e1's entry: kills, the attacker falls, and a shield breaks.
    const unitChip = chip("card-e1");
    expect(marks(unitChip).sort()).toEqual(["falls", "kills", "pops"]);
    expect(unitChip).toHaveAttribute("data-kills", "true");
    expect(unitChip).toHaveAttribute("data-falls", "true");
    expect(unitChip).toHaveAttribute("data-pops", "true");
    expect(unitChip.querySelector('.attack-outcome-dealt [data-mark="kills"]')).not.toBeNull();
    expect(unitChip.querySelector('.attack-outcome-taken [data-mark="falls"]')).not.toBeNull();
    // The hero's entry: no kill, no shield.
    const heroChip = chip("hero-opponent");
    expect(marks(heroChip)).toEqual([]);
    expect(heroChip).not.toHaveAttribute("data-kills");
    expect(heroChip).not.toHaveAttribute("data-pops");
  });

  it("R3477000 a kill on the hero wears the mark, and a target that survives with its attacker none", () => {
    renderGame({
      attacks: [entry("u1", "e1", { dealt: 1, taken: 1 }), entry("u1", "hero-p2", { dealt: 2, kills: true })],
      lethal: true,
    });
    fireEvent.click(screen.getByTestId("card-u1"));

    expect(marks(chip("card-e1"))).toEqual([]);
    expect(chip("card-e1")).not.toHaveAttribute("data-falls");
    expect(marks(chip("hero-opponent"))).toEqual(["kills"]);
  });

  it("R3477000 a dragged attacker shows the same chips", () => {
    renderGame(FORECAST);

    lift(screen.getByTestId("card-u3"));

    expect(document.documentElement).toHaveAttribute("data-dragging", "attack");
    expect(chips().map((element) => element.getAttribute("data-testid"))).toEqual(["outcome-hero-opponent"]);
    expect(chip("hero-opponent")).toHaveAttribute("data-dealt", "5");
  });

  it("R3477000 the setting turned off hides every chip", () => {
    writeSettings({ attackOutcomes: false });
    const { unmount } = renderGame(FORECAST);

    fireEvent.click(screen.getByTestId("card-u1"));
    expect(screen.getByTestId("card-e1")).toHaveAttribute("data-legal", "true");
    expect(chips()).toEqual([]);
    unmount();

    renderGame(FORECAST);
    lift(screen.getByTestId("card-u3"));
    expect(document.documentElement).toHaveAttribute("data-dragging", "attack");
    expect(chips()).toEqual([]);
  });

  it("R3477000 no forecast, a target without an entry, or no attacker picked renders nothing", () => {
    const { unmount } = renderGame(undefined);
    fireEvent.click(screen.getByTestId("card-u1"));
    expect(screen.getByTestId("card-e1")).toHaveAttribute("data-legal", "true");
    expect(chips()).toEqual([]);
    unmount();

    // u1's attack on the hero has no entry: the hero shows nothing while e1 still shows its own.
    renderGame({ attacks: [U1_E1, U3_HERO], lethal: false });
    expect(chips()).toEqual([]);
    fireEvent.click(screen.getByTestId("card-u1"));
    expect(screen.getByTestId("hero-opponent")).toHaveAttribute("data-legal", "true");
    expect(chips().map((element) => element.getAttribute("data-testid"))).toEqual(["outcome-card-e1"]);
  });
});
