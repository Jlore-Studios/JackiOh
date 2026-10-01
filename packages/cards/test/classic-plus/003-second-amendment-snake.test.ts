// C+ #3 Second Amendment Snake — SPEC §8.7 row 3, BUILD M9 Classic+ row C+ 3: "At its controller's end
// of turn places 2 Plague Tokens on itself as one placement (`counterChanged` with `plague`), not at the
// opponent's end; Death reads its last-known tokens (R78) and deals that many hits of 1, each to a
// random enemy (hero or Unit) still standing, so a unit an earlier hit brought to 0 is not picked again,
// all before the state check (R59); 0 tokens deals nothing and draws nothing (R129); its hits are no
// Spell's, so Spell Damage never raises them; its preview is the hits its Death would deal now (R280);
// tokens per turn and damage per token read through `param()`; radiant 3 tokens a turn".
//
// The R280 proofs are in this file's "R280 preview" block (the pinned list in `../preview.test.ts` is
// the lead's to update). The Death is set off by p1's own Hit Job (Core #16, "Destroy target Unit").

import { stepParam, type GameState } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/003-second-amendment-snake";

const SNAKE = "classicplus-003";
const HIT_JOB = "core-016"; // (3) Spell: Destroy target Unit.
const VANILLA = "core-008"; // (1) 4/4.
const MR_TOKEN = "core-015"; // (1) 1/1.
const SOLARIUS = "classicplus-038"; // Spell Damage +2.
const FILLER = "core-010";
const STOCKPILE = "core-005";
const HITS_LABEL = "for each Plague Token on this";

type Damage = Extract<GameEvent, { type: "damage" }>;

function snakeHits(s: Scenario, snakeId: string): Damage[] {
  return s.events.filter((event): event is Damage => event.type === "damage" && event.sourceId === snakeId);
}

function withSnake(tokens: number, p2: SideSetup = {}, extra: { radiant?: boolean; p1?: SideSetup; seed?: string } = {}): Scenario {
  return scenario({
    ...(extra.seed === undefined ? {} : { seed: extra.seed }),
    p1: {
      hand: [HIT_JOB, FILLER],
      library: [STOCKPILE, STOCKPILE],
      field: [{ def: SNAKE, radiant: extra.radiant === true, counters: { plague: tokens } }],
      ...extra.p1,
    },
    p2: { hand: [FILLER], library: [STOCKPILE, STOCKPILE], ...p2 },
  });
}

function killSnake(s: Scenario): Scenario {
  return s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(SNAKE).id }] });
}

function snakeView(s: Scenario, viewer: "p1" | "p2") {
  const side = viewer === "p1" ? s.view(viewer).you : s.view(viewer).opponent;
  return side.units.find((unit) => unit?.defId === SNAKE) ?? null;
}

describe("C+ #3 Second Amendment Snake", () => {
  it("is a (2) 1/6 Unit declaring tokens 2 → 3 and damage 1; one script runs both faces", () => {
    expect(def.cost).toBe(2);
    expect([def.base.attack, def.base.health, def.radiant.attack, def.radiant.health]).toEqual([1, 6, 2, 12]);
    expect(def.params?.map((p) => [p.key, p.base, p.radiant])).toEqual([
      ["tokens", 2, 3],
      ["damage", 1, 1],
    ]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("E19 at its controller's end of turn places 2 Plague Tokens on itself in one placement, not at the opponent's", () => {
      const s = withSnake(0);
      const snake = s.card(SNAKE);

      s.endTurn();

      expect(s.card(snake).counters.plague).toBe(2);
      const changes = s.events.filter((event) => event.type === "counterChanged" && event.instanceId === snake.id);
      expect(changes).toHaveLength(1);
      expect(changes[0]).toMatchObject({ counter: "plague", value: 2 });

      s.endTurn();
      expect(s.state.active).toBe("p1");
      expect(s.card(snake).counters.plague).toBe(2);
    });

    it("R78 E37 its Death reads its last-known tokens: that many hits of 1, each on a random enemy", () => {
      const s = withSnake(3, { field: [VANILLA, VANILLA] });
      const snake = s.card(SNAKE);

      killSnake(s);

      const hits = snakeHits(s, snake.id);
      expect(hits).toHaveLength(3);
      expect(hits.every((hit) => hit.amount === 1)).toBe(true);
      const enemies = new Set(["hero-p2", ...[1, 2].map((lane) => s.unit("p2", lane)?.id)]);
      expect(hits.every((hit) => enemies.has(hit.targetId))).toBe(true);
      const dealt = 30 - s.state.players.p2.hero.health + [1, 2].reduce((sum, lane) => sum + (4 - s.stats(s.unit("p2", lane)!).health), 0);
      expect(dealt).toBe(3);
      s.expectHealth("p1", 30);
    });

    it("R59 a unit an earlier hit brought to 0 is not picked again, and it dies after the last hit", () => {
      let deaths = 0;
      for (let seed = 1; seed <= 8; seed += 1) {
        const s = withSnake(5, { field: [MR_TOKEN] }, { seed: `snake-${seed}` });
        const token = s.card(MR_TOKEN);

        killSnake(s);

        const hits = snakeHits(s, s.card(SNAKE).id);
        expect(hits).toHaveLength(5);
        expect(hits.filter((hit) => hit.targetId === token.id).length).toBeLessThanOrEqual(1);
        const died = s.events.findIndex((event) => event.type === "destroyed" && event.instanceId === token.id);
        const last = s.events.lastIndexOf(hits[4] as GameEvent);
        if (died >= 0) {
          deaths += 1;
          expect(died).toBeGreaterThan(last);
        }
      }
      expect(deaths).toBeGreaterThan(0);
    });

    it("R129 with no tokens it deals nothing and draws no random number", () => {
      const s = withSnake(0, { field: [VANILLA] });
      const cursor = s.state.rngCursor;

      killSnake(s);

      s.expectInZone(SNAKE, "graveyard");
      expect(snakeHits(s, s.card(SNAKE).id)).toHaveLength(0);
      expect(s.state.rngCursor).toBe(cursor);
      s.expectHealth("p2", 30);
    });

    it("E6 its hits are no Spell's: Spell Damage on its side never raises them", () => {
      const s = withSnake(2, {}, { p1: { field: [{ def: SNAKE, counters: { plague: 2 } }, SOLARIUS] } });

      killSnake(s);

      expect(snakeHits(s, s.card(SNAKE).id).map((hit) => hit.amount)).toEqual([1, 1]);
      s.expectHealth("p2", 28);
    });

    it("dies in combat too: the Death fires off the tokens it had", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [FILLER], field: [{ def: SNAKE, counters: { plague: 2 } }] },
        p2: { hand: [FILLER], field: [{ def: "core-013" }] }, // Jlockeed Shredder-10, 8/10
      });
      const snake = s.card(SNAKE);

      s.attack(s.unit("p2", 1)!, snake);

      s.expectInZone(snake, "graveyard");
      // Its strike back is its own hit of 1; the Death's two come after it dies.
      const died = s.events.findIndex((event) => event.type === "destroyed" && event.instanceId === snake.id);
      expect(snakeHits(s, snake.id).filter((hit) => s.events.indexOf(hit) > died)).toHaveLength(2);
    });

    it("R386 tokens per turn and damage per token read through param(): an Upgrade of each moves what resolves", () => {
      const s = withSnake(0);
      stepParam(s.card(SNAKE), "tokens", 1);
      s.endTurn();
      expect(s.card(SNAKE).counters.plague).toBe(3);

      const t = withSnake(2);
      stepParam(t.card(SNAKE), "damage", 1);
      killSnake(t);
      expect(snakeHits(t, t.card(SNAKE).id).map((hit) => hit.amount)).toEqual([2, 2]);
      t.expectHealth("p2", 26);
    });
  });

  describe("radiant", () => {
    it("places 3 Plague Tokens at its controller's end of turn, and its Death deals 3 hits", () => {
      const s = withSnake(0, {}, { radiant: true });
      const snake = s.card(SNAKE);

      s.endTurn();
      expect(s.card(snake).counters.plague).toBe(3);
      s.endTurn();

      killSnake(s);
      expect(snakeHits(s, snake.id)).toHaveLength(3);
      s.expectHealth("p2", 27);
    });
  });

  describe("R280 preview", () => {
    it("is the hits its Death would deal now, for both viewers, and the label is in each face's text", () => {
      for (const face of [false, true]) {
        const s = withSnake(4, {}, { radiant: face });
        const shown = snakeView(s, "p1")?.preview;
        expect(shown).toEqual([{ label: HITS_LABEL, value: 4 }]);
        expect(snakeView(s, "p2")?.preview).toEqual(shown);
        expect((face ? def.radiant : def.base).text).toContain(HITS_LABEL);

        killSnake(s);
        expect(snakeHits(s, s.card(SNAKE).id)).toHaveLength(4);
      }
    });

    it("follows the tokens: 0 on a fresh Snake, 2 after one end of turn", () => {
      const s = withSnake(0);
      expect(snakeView(s, "p1")?.preview).toEqual([{ label: HITS_LABEL, value: 0 }]);
      s.endTurn();
      expect(snakeView(s, "p2")?.preview).toEqual([{ label: HITS_LABEL, value: 2 }]);
    });

    it("reads only its own counters: the hook answers off a state with no libraries or hands", () => {
      const s = withSnake(3);
      const bare = JSON.parse(JSON.stringify(s.state)) as GameState;
      for (const player of ["p1", "p2"] as const) {
        bare.players[player].library = [];
        bare.players[player].hand = [];
      }
      const self = bare.players.p1.units[0]?.[0];
      if (self === undefined) throw new Error("no Snake in lane 1");
      expect(base.preview?.({ state: bare, self, controller: "p1", radiant: false, zone: "field", yourTurn: true })).toEqual([
        { label: HITS_LABEL, value: 3 },
      ]);
    });
  });
});
