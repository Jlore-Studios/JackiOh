// #1442: each flavor's look, and a Ranged attacker firing its flavor's shot instead of lunging.
// `#1444` turned `FX_FLAVOR_LIVE` on, so every plan here runs with flavor live. Every plan here is
// `planStage`'s, against a view's public facts alone (R202).

import type { CardDef, Flavor, GameEvent, PlayerView, UnitView } from "@jackioh/shared";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { faceUpBackrow, fullBoardView, unit, withEvents } from "../test/fixtures.ts";
import { FLAVOR_LOOKS, chaosFlavor, flavorLook } from "../fx/flavors.ts";
import {
  FX_INTENSITY_SCALE,
  FX_LUNGE_CONTACT_AT,
  FX_PARTICLE_CAP,
  FX_SHOT_LAUNCH_AT,
  FX_SHOT_SHAKE_TRAUMA,
} from "../fx/constants.ts";
import { planFx } from "../fx/cues.ts";
import { createFxDirector } from "../fx/director.ts";
import { createFxMemory } from "../fx/memory.ts";
import { PARTICLE_PRESETS } from "../fx/presets.ts";
import { resetFxSettingsForTests } from "../fx/settings.ts";
import { planStage } from "../fx/stage.ts";
import type { FxAnchor, FxCue, FxPlanEnv, FxShot, FxShotMotion } from "../fx/types.ts";
import { MIN_ENTRY_MS, planEntries, reducedMotionNow } from "./animations.ts";
import { CATALOG } from "../wire/cards.ts";

beforeEach(() => {
  localStorage.clear();
  resetFxSettingsForTests();
});

afterEach(() => {
  resetFxSettingsForTests();
});

/* fullBoardView's ids: your units u1..u5, the enemy's u6..u10. u1 is core-004 (Hustle), u6 core-008. */
const MINE = "u1";
const ENEMY = "u6";

const tid = (testid: string): FxAnchor => ({ kind: "testid", testid });

/** `fullBoardView` with the unit `id` given Ranged (and whatever else `over` sets). */
function rangedBoard(id: string, over: Partial<UnitView> = {}): PlayerView {
  const view = fullBoardView();
  const patch = (unit: UnitView | null): UnitView | null =>
    unit !== null && unit.instanceId === id ? { ...unit, keywords: [...unit.keywords, { kind: "Ranged" }], ...over } : unit;
  return {
    ...view,
    you: { ...view.you, units: view.you.units.map(patch) },
    opponent: { ...view.opponent, units: view.opponent.units.map(patch) },
  };
}

function attackEvent(attackerId: string, targetId: string, forced = false): GameEvent {
  return { type: "attackDeclared", attackerId, targetId, forced };
}

/** The cues `attackDeclared` plans for `view`'s attacker, as the layer plans them (entry, then stage). */
function attack(
  view: PlayerView,
  attackerId: string,
  targetId: string,
  { D = 350, forced = false, intensity = FX_INTENSITY_SCALE.normal }: { D?: number; forced?: boolean; intensity?: number } = {},
): FxCue[] {
  const events = [attackEvent(attackerId, targetId, forced)];
  const shown = withEvents(view, events);
  const entries = planEntries(events, shown, false);
  expect(entries).toHaveLength(1);
  const env: FxPlanEnv = { intensity, card: () => undefined, memory: createFxMemory() };
  return planStage({ ...entries[0]!, durationMs: D }, shown, env);
}

/** Every flavor, written out so a sixteenth fails here and not silently. */
const FLAVORS = [
  "Beast",
  "Brute",
  "Chaos",
  "Clout",
  "Dark",
  "Fire",
  "Holy",
  "Hustle",
  "Nature",
  "Nerd",
  "Physical",
  "Snack",
  "Spirit",
  "Tech",
  "Toxic",
] as const satisfies readonly Flavor[];

const MOTIONS: readonly FxShotMotion[] = ["arc", "bolt", "beam", "swipe"];

describe("#1442 the looks", () => {
  it("#1442 every Flavor has a look: two or three colours, a particle shape and a projectile", () => {
    expect([...FLAVORS].sort()).toEqual(Object.keys(FLAVOR_LOOKS).sort());
    for (const flavor of FLAVORS) {
      const look = FLAVOR_LOOKS[flavor];
      expect(look.colors.length, flavor).toBeGreaterThanOrEqual(2);
      expect(look.colors.length, flavor).toBeLessThanOrEqual(3);
      for (const color of look.colors) expect(color, flavor).toMatch(/^#[0-9a-f]{6}$/);
      expect(Object.keys(PARTICLE_PRESETS), flavor).toContain(look.particle);
      expect(MOTIONS, flavor).toContain(look.projectile.motion);
      if (look.projectile.motion === "arc") {
        expect(look.projectile.count, flavor).toBeGreaterThanOrEqual(1);
        expect(look.projectile.sprites.length, flavor).toBeGreaterThanOrEqual(1);
      }
      if (look.projectile.motion === "bolt") expect(Object.keys(PARTICLE_PRESETS), flavor).toContain(look.projectile.trail);
    }
  });

  it("#1442 the note's projectiles: an arrow, a boulder, a claw slash, bolts, a book, a laser and the rest", () => {
    const shots: Record<Flavor, Partial<FxShot>> = {
      Physical: { motion: "arc", sprites: ["arrow"], turn: "point", count: 1 },
      Brute: { motion: "arc", sprites: ["boulder"], turn: "spin", count: 1, shake: true },
      Beast: { motion: "swipe" },
      Fire: { motion: "bolt", trail: "fire" },
      Dark: { motion: "bolt", trail: "void" },
      Holy: { motion: "bolt", trail: "sparkle" },
      Nerd: { motion: "arc", sprites: ["book"], turn: "spin", count: 1 },
      Tech: { motion: "beam" },
      Nature: { motion: "arc", sprites: ["seed"], turn: "point", count: 3 },
      Toxic: { motion: "arc", sprites: ["goo"], turn: "point", count: 1, trail: "poison" },
      Snack: { motion: "arc", sprites: ["snack"], turn: "spin", count: 1 },
      Hustle: { motion: "arc", sprites: ["coin", "bill"], turn: "spin", count: 4, scatter: true },
      Clout: { motion: "arc", sprites: ["heart", "emoji"], turn: "float", count: 3 },
      Spirit: { motion: "bolt", trail: "sparkle", wave: true },
      Chaos: { motion: "bolt", trail: "prismatic" },
    };
    for (const flavor of FLAVORS) expect(FLAVOR_LOOKS[flavor].projectile, flavor).toMatchObject(shots[flavor]);
    // Only Brute's boulder shakes the board.
    for (const flavor of FLAVORS) {
      const shot = FLAVOR_LOOKS[flavor].projectile;
      expect(shot.motion === "arc" && shot.shake === true, flavor).toBe(flavor === "Brute");
    }
  });

  it("#1442 Chaos's pick is the same for the same instance id and never Chaos", () => {
    const seen = new Set<string>();
    for (let i = 0; i < 200; i++) {
      const id = `u${i}`;
      const pick = chaosFlavor(id);
      expect(chaosFlavor(id), id).toBe(pick);
      expect(pick, id).not.toBe("Chaos");
      expect(FLAVORS, id).toContain(pick);
      expect(flavorLook("Chaos", id), id).toBe(FLAVOR_LOOKS[pick]);
      seen.add(pick);
    }
    // It varies with the id, and over enough ids it reaches more than a couple of the fourteen.
    expect(seen.size).toBeGreaterThan(2);
    // Every other flavor draws its own row.
    expect(flavorLook("Fire", "u1")).toBe(FLAVOR_LOOKS.Fire);
  });
});

describe("#1442 a Ranged attacker fires its flavor's shot", () => {
  it("#1442 a Ranged attacker stays in place and fires its flavor's shot at a Unit", () => {
    const D = 350;
    const launch = Math.round(FX_SHOT_LAUNCH_AT * D);
    const hustle = FLAVOR_LOOKS.Hustle;
    expect(CATALOG["core-004"]?.flavor).toBe("Hustle");
    expect(attack(rangedBoard(MINE), MINE, ENEMY, { D })).toEqual([
      { kind: "lunge", attacker: `card-${MINE}`, target: `card-${ENEMY}`, delayMs: 0, durationMs: D, still: true },
      {
        kind: "shot",
        shot: hustle.projectile,
        colors: hustle.colors,
        preset: hustle.particle,
        from: tid(`card-${MINE}`),
        to: tid(`card-${ENEMY}`),
        delayMs: launch,
        flightMs: D - launch,
        density: FX_INTENSITY_SCALE.normal,
      },
    ]);
    expect(launch).toBe(84);
    expect(D - launch).toBe(266);
  });

  it("#1442 … at a hero", () => {
    const cues = attack(rangedBoard(MINE), MINE, "hero-p2");
    expect(cues[0]).toMatchObject({ kind: "lunge", target: "hero-opponent", still: true });
    expect(cues[1]).toMatchObject({ kind: "shot", to: tid("hero-opponent") });
    const back = attack(rangedBoard(ENEMY), ENEMY, "hero-p1");
    expect(back[1]).toMatchObject({ kind: "shot", from: tid(`card-${ENEMY}`), to: tid("hero-you") });
  });

  it("#1442 … a forced attack fires the same way", () => {
    const view = rangedBoard(MINE);
    expect(attack(view, MINE, ENEMY, { forced: true })).toEqual(attack(view, MINE, ENEMY, { forced: false }));
    expect(attack(view, MINE, ENEMY, { forced: true })[1]?.kind).toBe("shot");
  });

  it("#1442 … a Ranged Unit standing in a backrow zone (R1081000) fires the same way", () => {
    /** `fullBoardView` with a Flying Unit (Brute's core-019) standing in your first backrow zone. */
    const flyer = (keywords: UnitView["keywords"]): PlayerView => {
      const view = fullBoardView();
      const bird = unit("p1", { instanceId: "bird-1", defId: "core-019", keywords });
      const slot = faceUpBackrow("p1", { instanceId: "bird-1", defId: "core-019", type: "Unit", unit: bird });
      return { ...view, you: { ...view.you, backrow: [slot, ...view.you.backrow.slice(1)] } };
    };
    const D = 350;
    const cues = attack(flyer([{ kind: "Flying" }, { kind: "Ranged" }]), "bird-1", ENEMY, { D });
    expect(cues[0]).toEqual({ kind: "lunge", attacker: "card-bird-1", target: `card-${ENEMY}`, delayMs: 0, durationMs: D, still: true });
    expect(cues[1]).toMatchObject({ kind: "shot", shot: FLAVOR_LOOKS.Brute.projectile, from: tid("card-bird-1"), to: tid(`card-${ENEMY}`) });
    // Without Ranged it lunges as a unit-zone attacker does.
    expect(attack(flyer([{ kind: "Flying" }]), "bird-1", ENEMY, { D })[0]).toEqual({
      kind: "lunge",
      attacker: "card-bird-1",
      target: `card-${ENEMY}`,
      delayMs: 0,
      durationMs: D,
    });
  });

  it("#1442 the density follows the intensity, and intensity off plans nothing", () => {
    expect(attack(rangedBoard(MINE), MINE, ENEMY, { intensity: FX_INTENSITY_SCALE.low })[1]).toMatchObject({
      kind: "shot",
      density: FX_INTENSITY_SCALE.low,
    });
    expect(attack(rangedBoard(MINE), MINE, ENEMY, { intensity: FX_INTENSITY_SCALE.off })).toEqual([]);
  });

  it("#1442 a non-Ranged attacker still lunges", () => {
    const D = 350;
    expect(attack(fullBoardView(), MINE, ENEMY, { D })).toEqual([
      { kind: "lunge", attacker: `card-${MINE}`, target: `card-${ENEMY}`, delayMs: 0, durationMs: D },
      {
        kind: "burst",
        preset: "spark",
        at: tid(`card-${ENEMY}`),
        delayMs: Math.round(FX_LUNGE_CONTACT_AT * D),
        count: 14,
        spread: "point",
        power: 1,
      },
    ]);
  });

  it("#1442 Brute's boulder shakes on impact, and a Fused attacker throws its first ingredient's", () => {
    expect(CATALOG["core-019"]?.flavor).toBe("Brute");
    const fused: CardDef = { ...CATALOG["core-001"]!, id: "t-1", flavor: undefined, ingredients: [{ defId: "core-019" }] };
    const view = { ...rangedBoard(MINE, { defId: "t-1" }), defs: { "t-1": fused } };
    const D = 350;
    const cues = attack(view, MINE, ENEMY, { D });
    expect(cues).toHaveLength(3);
    expect(cues[1]).toMatchObject({ kind: "shot", shot: FLAVOR_LOOKS.Brute.projectile, preset: "dust" });
    expect(cues[2]).toEqual({ kind: "shake", trauma: FX_SHOT_SHAKE_TRAUMA, delayMs: D });
    expect(FX_SHOT_SHAKE_TRAUMA).toBe(0.3);
    // The shake follows the intensity.
    expect(attack(view, MINE, ENEMY, { D, intensity: FX_INTENSITY_SCALE.high })[2]).toMatchObject({
      kind: "shake",
      trauma: Math.min(1, FX_SHOT_SHAKE_TRAUMA * FX_INTENSITY_SCALE.high),
    });
  });

  it("#1442 a definition no one knows throws Physical's arrow", () => {
    const cues = attack(rangedBoard(MINE, { defId: "t-404" }), MINE, ENEMY);
    expect(cues[1]).toMatchObject({ kind: "shot", shot: FLAVOR_LOOKS.Physical.projectile, preset: "spark" });
  });

  it("#1442 a Chaos attacker throws its instance's pick", () => {
    expect(CATALOG["core-052"]?.flavor).toBe("Chaos");
    const cues = attack(rangedBoard(MINE, { defId: "core-052" }), MINE, ENEMY);
    const pick = FLAVOR_LOOKS[chaosFlavor(MINE)];
    expect(cues[1]).toMatchObject({ kind: "shot", shot: pick.projectile, colors: pick.colors, preset: pick.particle });
    // Another instance may pick another flavor, but one instance never changes its mind.
    expect(attack(rangedBoard(MINE, { defId: "core-052" }), MINE, ENEMY)).toEqual(cues);
  });

  it("#1442 the shot starts inside its entry and lands by its end", () => {
    for (const D of [MIN_ENTRY_MS, 350, 1200]) {
      for (const id of ["core-004", "core-019", "core-052"]) {
        const cues = attack(rangedBoard(MINE, { defId: id }), MINE, ENEMY, { D });
        for (const cue of cues) {
          expect(cue.delayMs, JSON.stringify(cue)).toBeGreaterThanOrEqual(0);
          expect(cue.delayMs, JSON.stringify(cue)).toBeLessThanOrEqual(D);
          if (cue.kind === "lunge") expect(cue.durationMs).toBeLessThanOrEqual(D);
          if (cue.kind === "shot") {
            expect(cue.flightMs).toBeGreaterThan(0);
            expect(cue.delayMs + cue.flightMs).toBe(D);
          }
        }
      }
    }
  });
});

describe("#1442 reduced motion and the effects layer's off switch", () => {
  it("#1442 reduced motion takes the lunge's fallback", () => {
    const events = [attackEvent(MINE, ENEMY)];
    const ranged = planEntries(events, withEvents(rangedBoard(MINE), events), true);
    const plain = planEntries(events, withEvents(fullBoardView(), events), true);
    expect(ranged).toHaveLength(1);
    expect(plain).toHaveLength(1);
    expect(ranged[0]!.type).toBe(plain[0]!.type);
    expect(ranged[0]!.durationMs).toBe(0);
    expect(plain[0]!.durationMs).toBe(0);
    expect([...ranged[0]!.frames]).toEqual([...plain[0]!.frames]);
    // The viewer's own reduce-motion switch counts as the media query does, and FxLayer plans nothing for it (B37).
    expect(reducedMotionNow({ motion: "reduce" })).toBe(true);
  });

  it("#1442 at intensity off a Ranged and a plain attack both plan nothing", () => {
    for (const view of [rangedBoard(MINE), fullBoardView()]) {
      expect(attack(view, MINE, ENEMY, { intensity: FX_INTENSITY_SCALE.off })).toEqual([]);
    }
  });
});

describe("#1444 a Ranged attack into combat damage plays live", () => {
  it("#1444 the shot flies and the combat hit lands at once with today's lunge timing", () => {
    const view = rangedBoard(MINE);
    const events: GameEvent[] = [
      attackEvent(MINE, ENEMY),
      { type: "damage", sourceId: MINE, targetId: ENEMY, amount: 4, combat: true },
    ];
    const shown = withEvents(view, events);
    const entries = planEntries(events, shown, false);
    expect(entries).toHaveLength(2);
    const attackEntry = entries[0]!;
    const damageEntry = entries[1]!;
    const attackD = attackEntry.durationMs;
    const damageD = damageEntry.durationMs;

    const attackEnv: FxPlanEnv = { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
    attackEnv.memory.remember(attackEntry.events);
    const stageCues = planStage({ ...attackEntry, durationMs: attackD }, shown, attackEnv);
    const hustle = FLAVOR_LOOKS.Hustle;
    const launch = Math.round(FX_SHOT_LAUNCH_AT * attackD);
    expect(stageCues[0]).toEqual({
      kind: "lunge",
      attacker: `card-${MINE}`,
      target: `card-${ENEMY}`,
      delayMs: 0,
      durationMs: attackD,
      still: true,
    });
    expect(stageCues[1]).toMatchObject({
      kind: "shot",
      shot: hustle.projectile,
      colors: hustle.colors,
      preset: hustle.particle,
      delayMs: launch,
      flightMs: attackD - launch,
    });

    const fxCues = planFx({ ...attackEntry, durationMs: attackD }, shown, attackEnv);
    expect(fxCues).toHaveLength(1);
    expect(fxCues[0]).toMatchObject({ kind: "burst", preset: "dust", delayMs: 0 });

    const damageEnv: FxPlanEnv = { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
    damageEnv.memory.remember(damageEntry.events);
    const damageCues = planFx({ ...damageEntry, durationMs: damageD }, shown, damageEnv);
    expect(damageCues.some((cue) => cue.kind === "shot" || cue.kind === "projectile")).toBe(false);
    expect(damageCues.find((cue) => cue.kind === "burst" && cue.preset === "spark")).toMatchObject({ delayMs: 0 });
    expect(damageCues.find((cue) => cue.kind === "splat")).toMatchObject({ delayMs: 0 });
  });
});

describe("#1442 the director pins a still lunge", () => {
  it("#1442 the director pins a still lunge at 0 px, and release lets it go", () => {
    for (const id of [`card-${MINE}`, `card-${ENEMY}`]) {
      const el = document.createElement("div");
      el.setAttribute("data-testid", id);
      document.body.appendChild(el);
    }
    const attacker = document.querySelector<HTMLElement>(`[data-testid="card-${MINE}"]`)!;
    const director = createFxDirector({
      surface: null,
      domRoot: document.body,
      now: () => 1000,
      frames: { request: () => 1, cancel: () => undefined },
      visibility: { hidden: () => false, subscribe: () => () => undefined },
      measure: () => null,
      shakeSink: { apply: () => undefined, clear: () => undefined },
      seed: 7,
      capacity: FX_PARTICLE_CAP,
      viewport: { subscribe: () => () => undefined },
    });
    director.play(attack(rangedBoard(MINE), MINE, ENEMY));
    expect(attacker.style.getPropertyValue("--fx-lunge-x")).toBe("0px");
    expect(attacker.style.getPropertyValue("--fx-lunge-y")).toBe("0px");
    director.release();
    expect(attacker.style.getPropertyValue("--fx-lunge-x")).toBe("");
    expect(attacker.style.getPropertyValue("--fx-lunge-y")).toBe("");
    director.dispose();
    document.body.innerHTML = "";
  });
});
