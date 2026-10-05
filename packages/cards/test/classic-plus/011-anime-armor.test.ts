// C+ #11 Anime Armor — SPEC §8.7 row 11, BUILD M9 Classic+ row C+ 11: "While it is on the field its
// controller's hero takes at most 1 from each damage instance (§4.4 step 3): combat, Spells, Pierce hits
// and fatigue alike, each Trample excess and each split hit its own capped instance; the lowest cap wins
// beside Anti-oneshot Armor's 5 or 3; lose health (R18) and Set health are not damage and are not
// capped; the opponent's hero is unaffected; the cap lifts when it leaves; the cap reads through
// `param()` and never drops below 1; radiant 8/8 with Reborn, the cap returning with it".
//
// Anime Armor stands in p1's unit lane 1 unless a test says otherwise.

import { stepParam } from "@jackioh/engine";
import type { GameEvent, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/011-anime-armor";

const ANIME = "classicplus-011";
const MENACE = "core-019"; // (3) 9/9 Taunt.
const VANILLA = "core-008"; // (1) 4/4.
const LUNAR = "core-035"; // (1) Deal 3 damage to a target.
const TRUE_STRIKE = "core-044"; // (1) Pierce. Deal 4 damage.
const STOCKPILE = "core-005"; // (1) Draw 2. Heal your hero 2.
const JELLY_BEAN = "core-027"; // Cast on draw: make a random hand card Radiant. Lose 5 health.
const ANTI_ONESHOT = "core-073"; // Field Spell: hits on your hero are capped at 5 (Radiant 3).
const BIG_MAX = "classic-080"; // 13/8 Rush, Trample, Indestructible.
const SNAKE = "classicplus-003"; // Death: a hit of {damage} on a random enemy per Plague Counter.
const VITAL_KILL = "classic-029"; // (1) Set a hero's health to 13.
const HIT_JOB = "core-016"; // (3) Destroy target Unit.
const SILENCE = "classicplus-009"; // (0) Vanilla a Unit.
const FILLER = "core-010";

type Damage = Extract<GameEvent, { type: "damage" }>;

function heroHits(s: Scenario, player: "p1" | "p2"): number[] {
  return s.events.flatMap((event) => (event.type === "damage" && event.targetId === `hero-${player}` ? [(event as Damage).amount] : []));
}

function at(id: string): Selection[] {
  return [{ pick: "instance", instanceId: id }];
}

const HERO_P1: Selection[] = [{ pick: "hero", player: "p1" }];

/** p2 active, attacking or casting into p1, who holds Anime Armor. */
function underFire(p1: SideSetup = {}, p2: SideSetup = {}, radiantFace = false): Scenario {
  return scenario({
    active: "p2",
    p1: { hand: [FILLER], library: [STOCKPILE, STOCKPILE], field: [{ def: ANIME, radiant: radiantFace }], health: 20, ...p1 },
    p2: { hand: [FILLER], library: [STOCKPILE, STOCKPILE], mana: 10, ...p2 },
  });
}

describe("C+ #11 Anime Armor", () => {
  it("is a (2) 4/4 Unit (Radiant 8/8 with Reborn) declaring cap 1, better down, never below 1; one script runs both faces", () => {
    expect([def.cost, def.base.attack, def.base.health, def.radiant.attack, def.radiant.health]).toEqual([2, 4, 4, 8, 8]);
    expect(def.radiant.keywords).toEqual([{ kind: "Reborn" }]);
    expect(def.params).toEqual([{ key: "cap", base: 1, radiant: 1, better: "down", step: 1, min: 1 }]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("§4.4 step 3 a combat hit on your hero takes 1", () => {
      const s = underFire({}, { field: [MENACE] });

      s.attack(MENACE, "hero");

      expect(heroHits(s, "p1")).toEqual([1]);
      s.expectHealth("p1", 19);
    });

    it("a Spell's hit takes 1, and so does a Pierce hit", () => {
      const s = underFire({}, { hand: [LUNAR, TRUE_STRIKE, FILLER] });

      s.play(LUNAR, { targets: HERO_P1 });
      s.play(TRUE_STRIKE, { targets: HERO_P1 });

      expect(heroHits(s, "p1")).toEqual([1, 1]);
      s.expectHealth("p1", 18);
    });

    it("§2.4 R125 each fatigue hit is its own capped instance", () => {
      const s = scenario({
        p1: { hand: [STOCKPILE, FILLER], library: [], field: [ANIME], health: 20 },
        p2: { hand: [FILLER] },
      });

      s.play(STOCKPILE); // draws 2 from an empty deck: fatigue 1, then 2; then heals 2

      expect(heroHits(s, "p1")).toEqual([1, 1]);
      s.expectHealth("p1", 20);
    });

    it("§4.4 step 9 a Trample excess is its own instance, capped at 1", () => {
      const s = underFire({ field: [{ def: ANIME, lane: 1 }, { def: VANILLA, lane: 2 }] }, { field: [BIG_MAX] });

      s.attack(BIG_MAX, s.unit("p1", 2)!);

      expect(heroHits(s, "p1")).toEqual([1]);
      s.expectHealth("p1", 19);
    });

    it("E37 each hit of a split is its own capped instance", () => {
      // p2's Second Amendment Snake, tuned to hits of 2, dies to p1's Hit Job and splits its hits over p1.
      const s = scenario({
        p1: { hand: [HIT_JOB, FILLER], field: [ANIME], health: 20, mana: 8 },
        p2: { hand: [FILLER], field: [{ def: SNAKE, counters: { plague: 6 } }] },
      });
      stepParam(s.card(SNAKE), "damage", 1);

      s.play(HIT_JOB, { targets: at(s.card(SNAKE).id) });

      const onHero = heroHits(s, "p1");
      expect(onHero.length).toBeGreaterThan(0);
      expect(onHero.every((amount) => amount === 1)).toBe(true);
      s.expectHealth("p1", 20 - onHero.length);
    });

    it("E6 the lowest cap wins beside Anti-oneshot Armor's 5, or its Radiant 3", () => {
      const s = underFire({ backrow: [ANTI_ONESHOT] }, { field: [MENACE] });
      s.attack(MENACE, "hero");
      expect(heroHits(s, "p1")).toEqual([1]);

      const loose = underFire({ backrow: [ANTI_ONESHOT] }, { field: [MENACE] });
      stepParam(loose.card(ANIME), "cap", 6); // Degraded six times: cap 7, so Anti-oneshot's 5 is lower
      loose.attack(MENACE, "hero");
      expect(heroHits(loose, "p1")).toEqual([5]);

      const tight = underFire({ backrow: [{ def: ANTI_ONESHOT, radiant: true }] }, { field: [MENACE] });
      stepParam(tight.card(ANIME), "cap", 6);
      tight.attack(MENACE, "hero");
      expect(heroHits(tight, "p1")).toEqual([3]);
    });

    it("R18 lose health is not damage and is not capped", () => {
      const s = scenario({
        p1: { hand: [STOCKPILE, FILLER], library: [JELLY_BEAN, FILLER, FILLER], field: [ANIME], health: 20 },
        p2: { hand: [FILLER] },
      });

      s.play(STOCKPILE); // draws the Jelly Bean, which casts itself: lose 5; then heal 2

      s.expectHealth("p1", 17);
      expect(heroHits(s, "p1")).toEqual([]);
    });

    it("E7 Set health is not damage and is not capped", () => {
      const s = underFire({}, { hand: [VITAL_KILL, FILLER] });

      s.play(VITAL_KILL, { targets: HERO_P1 });

      s.expectHealth("p1", 13);
    });

    it("the opponent's hero is unaffected", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [ANIME, MENACE] },
        p2: { hand: [FILLER] },
      });

      s.attack(MENACE, "hero");

      s.expectHealth("p2", 21);
    });

    it("the cap lifts when it leaves the field", () => {
      const s = underFire({}, { hand: [HIT_JOB, LUNAR, FILLER] });

      s.play(HIT_JOB, { targets: at(s.card(ANIME).id) });
      s.play(LUNAR, { targets: HERO_P1 });

      s.expectInZone(ANIME, "graveyard");
      expect(heroHits(s, "p1")).toEqual([3]);
    });

    it("§6.3 a Vanilla Anime Armor caps nothing: its text is gone", () => {
      const s = underFire({}, { hand: [SILENCE, LUNAR, FILLER] });

      s.play(SILENCE, { targets: at(s.card(ANIME).id) });
      s.play(LUNAR, { targets: HERO_P1 });

      expect(heroHits(s, "p1")).toEqual([3]);
    });

    it("R386 the cap reads through param(): a Degrade lets 2 through, and an Upgrade never takes it below 1", () => {
      const degraded = underFire({}, { hand: [LUNAR, FILLER] });
      stepParam(degraded.card(ANIME), "cap", 1);
      degraded.play(LUNAR, { targets: HERO_P1 });
      expect(heroHits(degraded, "p1")).toEqual([2]);

      const upgraded = underFire({}, { hand: [LUNAR, FILLER] });
      stepParam(upgraded.card(ANIME), "cap", -1);
      upgraded.play(LUNAR, { targets: HERO_P1 });
      expect(heroHits(upgraded, "p1")).toEqual([1]);
    });
  });

  describe("radiant", () => {
    it("is 8/8 with Reborn, and the cap comes back with the Reborn body", () => {
      const s = underFire({}, { hand: [HIT_JOB, LUNAR, FILLER] }, true);
      const anime = s.card(ANIME);
      s.expectStats(anime, { attack: 8, health: 8 });

      s.play(HIT_JOB, { targets: at(anime.id) });
      const back = s.unit("p1", 1);
      expect(back?.defId).toBe(ANIME);
      s.expectStats(back!, { health: 1 });

      s.play(LUNAR, { targets: HERO_P1 });
      expect(heroHits(s, "p1")).toEqual([1]);
    });
  });
});
