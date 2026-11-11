// R3407000: Wildar's signature (wildar.ts), planned as FxLayer plans it (remember, then plan): its cast's
// nebula, orb and crack at the board's centre, its two halves flying as card backs to the hand piles of the
// seats that get them, its gallery play within the Legendary Spell's ceiling, nothing with the effects off,
// R200 at every D, and from the caster's opponent's seat no cue names a card (R202).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid, HIDDEN_ID } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import { envOf, expectGalleryWithinCeiling, expectR200, planWithOffsets, type PlannedEntry } from "../../../test/signaturePass.ts";
import { frac } from "../../build.ts";
import { planCardFx } from "../../cardFx.ts";
import { FX_CENTER } from "../../constants.ts";
import { planFx } from "../../cues.ts";
import { galleryEntry } from "../../motifs/gallery.ts";
import type { FxAnchor, FxCue, FxPlanEnv } from "../../types.ts";
import { FX_CARD_TABLE } from "../tables.ts";
import { chaosOrbFixture } from "./wildar.ts";

const WILDAR = "stinkyscape-149";
const ID = "fx-wildar";
const DS = [120, 200, 400, 732, 1400] as const;
/** R202: a card id of any set. */
const CARD_ID = /(core|classic|meditative|stinkyscape|traininggrounds)[a-z]*-\d/;
/** R202: an anchor reaching for a card in a hand. */
const HAND_CARD = '"kind":"handCard"';

const tid = (value: string, at?: { x: number; y: number }): FxAnchor => (at === undefined ? { kind: "testid", testid: value } : { kind: "testid", testid: value, at });
const CENTRE = tid(testid.board, FX_CENTER);
const played = (player: PlayerId): GameEvent => ({ type: "cardPlayed", player, instanceId: ID, defId: WILDAR, costPaid: 3 });
const resolved = (player: PlayerId): GameEvent => ({ type: "cardResolved", player, instanceId: ID, defId: WILDAR, permanent: false, costPaid: 3 });
const kinds = (cues: readonly FxCue[]): string[] => cues.map((cue) => cue.kind);

type Planned = { cues: FxCue[]; row: string | undefined };

/** Each of `events` as its own entry lasting D, after `before`: remember, then the recipe's answer. */
function recipe(events: readonly GameEvent[], D: number, before: readonly GameEvent[], view: PlayerView = fullBoardView(), env: FxPlanEnv = envOf()): Planned[] {
  env.memory.remember(before);
  return events.map((event) => {
    env.memory.remember([event]);
    const entry = { ...galleryEntry([event], view), durationMs: D };
    const result = planCardFx(event, { entry, view, env, D });
    return { cues: result?.cues ?? [], row: result?.row };
  });
}

/** Each of `events` as its own entry lasting D, planned whole (rows and all), for R200. */
function entries(events: readonly GameEvent[], D: number, view: PlayerView = fullBoardView(), env: FxPlanEnv = envOf()): PlannedEntry[] {
  return events.map((event) => {
    env.memory.remember([event]);
    const entry = { ...galleryEntry([event], view), durationMs: D };
    return { entry, offset: 0, cues: planFx(entry, view, env) };
  });
}

/** Wildar's play as `player` streams it, its Cry's two cards on the viewer's seat. */
function play(player: PlayerId, view: PlayerView): GameEvent[] {
  return [played(player), ...chaosOrbFixture(view, ID, player), resolved(player)];
}

describe("R3407000 Wildar's signature", () => {
  it("R3407000 Wildar's table entry is its signature chaos orb", () => {
    expect(FX_CARD_TABLE.get(WILDAR)).toEqual({ kind: "bespoke", key: "chaosOrb", signature: true });
  });

  it("R3407000 Wildar's cast: nebula smoke and a churning orb at the board's centre, cracking in two under a ring, a fracture and a light shake", () => {
    const D = 400;
    const crack = frac(0.5, D);
    const [cast] = recipe([played("p1")], D, []);
    expect(cast?.row).toBe("replace");
    expect(kinds(cast?.cues ?? [])).toEqual(["burst", "burst", "ring", "fracture", "burst", "burst", "shake"]);
    for (const cue of cast?.cues ?? []) if (cue.kind !== "shake") expect(cue, cue.kind).toMatchObject({ at: CENTRE });
    expect(cast?.cues[0]).toMatchObject({ preset: "smoke", delayMs: 0 });
    expect(cast?.cues[3]).toMatchObject({ kind: "fracture", delayMs: crack });
    expect(cast?.cues.slice(2).every((cue) => cue.delayMs === crack)).toBe(true);
  });

  it("R3407000 Wildar's halves: a card back from the centre to the opponent's hand pile, then one to the caster's under a sheen, on either seat", () => {
    const D = 250;
    const view = fullBoardView();
    const [theirs, mine] = recipe(chaosOrbFixture(view, ID, "p1"), D, [played("p1")], view);
    expect(theirs).toEqual({ cues: [{ kind: "ghost", from: CENTRE, to: tid(animTestid.hand("opponent")), delayMs: 0, durationMs: D }], row: "replace" });
    expect(mine?.row).toBe("replace");
    expect(mine?.cues).toEqual([
      { kind: "ghost", from: CENTRE, to: tid(animTestid.hand("you")), delayMs: 0, durationMs: D },
      { kind: "sheen", at: tid(animTestid.hand("you")), delayMs: frac(0.5, D), durationMs: D - frac(0.5, D) },
    ]);
    // Cast by the other seat: the dull half comes to the viewer's hand, the dazzling one goes to theirs.
    const [toYou, toThem] = recipe(chaosOrbFixture(view, ID, "p2"), D, [played("p2")], view);
    expect(toYou?.cues.map((cue) => (cue.kind === "ghost" ? cue.to : null))).toEqual([tid(animTestid.hand("you"))]);
    expect(kinds(toThem?.cues ?? [])).toEqual(["ghost", "sheen"]);
    expect(toThem?.cues[1]).toMatchObject({ at: tid(animTestid.hand("opponent")) });
  });

  it("R3407000 Wildar's gallery play ends within the Legendary Spell's ceiling", () => {
    expectGalleryWithinCeiling(WILDAR);
  });

  it("R3407000 Wildar's every beat is in its gallery play; another card's hand draws keep their own row", () => {
    // Both halves are its fixture's; outside its play, an `addedToHand` is not its.
    expect(recipe([{ type: "addedToHand", player: "p1", instanceId: "x", defId: "core-001" }], 250, [])).toEqual([{ cues: [], row: undefined }]);
  });

  it("R3407000 Wildar draws nothing with the effects off", () => {
    const view = fullBoardView();
    expect(planWithOffsets(play("p1", view), view, envOf(0)).flatMap((entry) => entry.cues)).toEqual([]);
  });

  it("R3407000 Wildar's every cue starts inside its entry and ends within FX_MAX_TAIL_MS of it, at every D (R200)", () => {
    const view = fullBoardView();
    for (const D of DS) {
      const run = entries(play("p1", view), D, view);
      expect(run.flatMap((entry) => entry.cues).length, `D ${String(D)}`).toBeGreaterThan(0);
      expectR200(run, `${WILDAR} D ${String(D)}`);
    }
  });

  it("R3407000 Wildar from the caster's opponent's seat: its halves still fly, no cue names a card and none reaches into a hand (R202)", () => {
    const view = fullBoardView();
    // Cast by p2 and watched by p1: every card added is "hidden" to this test, the viewer's own included.
    const events = play("p2", view).map((event): GameEvent => (event.type === "addedToHand" ? { ...event, instanceId: HIDDEN_ID, defId: HIDDEN_ID } : event));
    const run = planWithOffsets(events, view);
    const halves = run.filter((entry) => entry.entry.events.some((event) => event.type === "addedToHand"));
    expect(halves).toHaveLength(2);
    for (const half of halves) expect(half.cues.some((cue) => cue.kind === "ghost")).toBe(true);
    const cues = run.flatMap((entry) => entry.cues);
    for (const cue of cues) {
      expect(JSON.stringify(cue)).not.toMatch(CARD_ID);
      expect(JSON.stringify(cue)).not.toContain(HAND_CARD);
    }
  });
});
