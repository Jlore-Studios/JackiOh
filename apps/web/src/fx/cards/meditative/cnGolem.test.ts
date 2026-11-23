// R3405001: CN Golem's signature (cnGolem.ts), cue by cue, past what signatures.units-a.test.ts holds: its
// entrance's blocks, clacks, joints and shake in Toxic and Brute's dust, every one after its entry's lead
// (R3405000), and the blocks mirrored for the landing's seat only when the entrance knows both the board and
// the landing; its kill trigger's virus leak, which a `trigger` entry and its play (the gallery's) draw and
// no other hook does; and its gallery fixture, one Super-Virus into the other player's deck.

import type { GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import { run, tid } from "../../../test/signatures.ts";
import { CATALOG } from "../../../wire/cards.ts";
import { frac } from "../../build.ts";
import { FX_INTENSITY_SCALE, FX_SLAM_AT } from "../../constants.ts";
import { figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import { ENTRANCE_TRAUMA, FOOT } from "../recipe.ts";
import { cnGolemFixture, golemLock } from "./cnGolem.ts";

const GOLEM = "meditative-095-1";
const D = 1000;
const LEAD = 200;
const LANE = 3;
const TGT = testid.zone("you", "units", LANE);
const NORMAL = FX_INTENSITY_SCALE.normal;
const TOXIC = FLAVOR_LOOKS.Toxic;
const BRUTE = FLAVOR_LOOKS.Brute;

const landing = (player: PlayerId): Extract<GameEvent, { type: "summoned" }> => ({ type: "summoned", player, instanceId: "ex", defId: GOLEM, row: "units", lane: LANE });
const virus = (player: PlayerId): GameEvent => ({ type: "shuffledIn", player, instanceId: "fx-virus", defId: "core-090-1", position: 0, creator: player === "p1" ? "p2" : "p1" });

describe("R3405001 CN Golem's signature, cue by cue", () => {
  it("R3405000 its blocks tumble in from its entry's lead and outlast the entry by 300 ms; clacks, joint dust, a ring at its foot and its shake land at the slam after the lead", () => {
    const slam = LEAD + frac(FX_SLAM_AT, D - LEAD);
    const cues = golemLock({ D, tgt: TGT, intensity: NORMAL, view: fullBoardView(), landing: landing("p1"), leadMs: LEAD });
    expect(cues).toEqual([
      { kind: "figure", figure: "golemBlocks", at: tid(TGT), tint: figureTint(TOXIC), delayMs: LEAD, durationMs: D - LEAD + 300, side: "you" },
      { kind: "burst", preset: "shard", at: tid(TGT), delayMs: slam, count: 18, spread: "ring", power: 1.1, colors: BRUTE.colors },
      { kind: "burst", preset: "dust", at: tid(TGT, { x: 0.25, y: 0.35 }), delayMs: slam, count: 8, spread: "point", power: 0.7, colors: BRUTE.colors },
      { kind: "burst", preset: "dust", at: tid(TGT, { x: 0.75, y: 0.35 }), delayMs: slam, count: 8, spread: "point", power: 0.7, colors: BRUTE.colors },
      { kind: "ring", preset: "dust", at: tid(TGT, FOOT), delayMs: slam, durationMs: expect.any(Number) as number },
      { kind: "shake", trauma: ENTRANCE_TRAUMA.golemLock, delayMs: slam },
    ]);
    // With no lead its blocks start at once and lock at FX_SLAM_AT of the entry.
    const bare = golemLock({ D, tgt: TGT, intensity: NORMAL });
    expect(bare[0]).toMatchObject({ delayMs: 0, durationMs: D + 300 });
    expect(bare.slice(1).map((cue) => cue.delayMs)).toEqual(Array.from({ length: 5 }, () => frac(FX_SLAM_AT, D)));
  });

  it("R3405001 its blocks face the landing's seat only when the entrance has both the board and the landing", () => {
    const view = fullBoardView();
    expect(golemLock({ D, tgt: TGT, intensity: NORMAL, view, landing: landing("p2") })[0]).toMatchObject({ side: "opponent" });
    for (const plan of [{}, { view }, { landing: landing("p2") }]) {
      expect(golemLock({ D, tgt: TGT, intensity: NORMAL, ...plan })[0]).not.toHaveProperty("side");
    }
  });

  it("R2797000 each Super-Virus into the other deck leaks out of it as a Toxic mote, under its kill trigger and under its play alike, and under no other hook of its", () => {
    const view = fullBoardView({ you: { ...fullBoardView().you, units: fullBoardView().you.units.map((unit, k) => (k === LANE - 1 && unit !== null ? { ...unit, instanceId: "ex" } : unit)) } });
    const golem = tid(testid.card("ex"));
    const pile = tid(animTestid.library("opponent"));
    const lands = frac(0.85, D);
    const [leak] = run(GOLEM, [virus("p2")], D, "p1", view, "whenever");
    expect(leak).toEqual({
      cues: [
        { kind: "burst", preset: TOXIC.particle, at: golem, delayMs: 0, count: 8, spread: "point", power: 0.6, colors: TOXIC.colors },
        expect.objectContaining({ kind: "projectile", from: golem, to: pile, delayMs: 0, flightMs: lands }) as unknown,
        { kind: "ghost", from: golem, to: pile, delayMs: 0, durationMs: lands },
      ],
      row: "replace",
    });
    // Its play (the gallery's) draws the same leak; a Death or an Activate of its is no kill trigger.
    expect(run(GOLEM, [virus("p2")], D, "p1", view)).toEqual([leak]);
    for (const hook of ["death", "activate"] as const) expect(run(GOLEM, [virus("p2")], D, "p1", view, hook)).toEqual([{ cues: [], row: undefined }]);
  });

  it("R3405001 its gallery fixture shuffles one Super-Virus into the other player's deck, which leaks out of it into that pile", () => {
    const view = fullBoardView();
    expect(cnGolemFixture(view, "ex", "p1")).toEqual([{ type: "shuffledIn", player: "p2", instanceId: "fx-virus", defId: "core-090-1", position: 0, creator: "p1" }]);
    expect(cnGolemFixture(view, "ex", "p2")).toEqual([{ type: "shuffledIn", player: "p1", instanceId: "fx-virus", defId: "core-090-1", position: 0, creator: "p2" }]);
    expect(CATALOG["core-090-1"]?.name).toBe("Super-Virus");
    const [mine] = run(GOLEM, cnGolemFixture(view, "ex", "p1"), D, "p1", view);
    expect(mine?.cues[2]).toMatchObject({ kind: "ghost", to: tid(animTestid.library("opponent")) });
  });
});
