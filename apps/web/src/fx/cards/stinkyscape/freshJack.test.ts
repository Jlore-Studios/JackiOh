// R3407000: xxFreshJack's signature (freshJack.ts), planned as FxLayer plans it: after his own takeover's
// hold he vaults from the screen's centre onto his zone in a skid of pixels; his Cry's first cast tears the
// gate open on his card and hurls a fire comet to the board's centre, where the cast card's own entry plays.
// His gallery play within a Mythic's ceiling, R200 at any length, nothing with the effects off, and his Radiant
// face's Discover, a prompt, never drawn: from the opponent's seat no cue names a card (R202).

import type { GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import {
  asTheirsAll,
  beatsOf,
  cuesOfRun,
  envOf,
  expectCuesR200,
  expectGalleryWithinCeiling,
  expectNoCard,
  planWithOffsets,
} from "../../../test/signaturePass.ts";
import { frac } from "../../build.ts";
import { FX_CENTER, FX_INTENSITY_SCALE, FX_SLAM_AT } from "../../constants.ts";
import { ENTRANCES } from "../../entrances.ts";
import type { FxAnchor, FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA } from "../recipe.ts";
import { FX_CARD_TABLE } from "../tables.ts";
import { freshJackCometFixture } from "./freshJack.ts";
import { landMsAfter } from "./signature.ts";

const FRESH_JACK = "stinkyscape-109";
const DS = [120, 200, 400, 732, 1400] as const;

const tid = (value: string, at?: { x: number; y: number }): FxAnchor => (at === undefined ? { kind: "testid", testid: value } : { kind: "testid", testid: value, at });
const kinds = (cues: readonly FxCue[] | undefined): string[] => (cues ?? []).map((cue) => cue.kind);
const CENTRE = tid(testid.board, FX_CENTER);
const played = (player: PlayerId): Extract<GameEvent, { type: "cardPlayed" }> => ({ type: "cardPlayed", player, instanceId: "fj", defId: FRESH_JACK, costPaid: 4 });
const landing = (player: PlayerId, lane = 3): Extract<GameEvent, { type: "summoned" }> => ({ type: "summoned", player, instanceId: "fj", defId: FRESH_JACK, row: "units", lane });
const prompt: GameEvent = { type: "promptOpened", player: "p1", choiceId: "fx-discover", kind: "discover" };

describe("R3407000 xxFreshJack's signature", () => {
  it("R3407000 his table entry is his signature, with his own takeover look", () => {
    expect(FX_CARD_TABLE.get(FRESH_JACK)).toEqual({ kind: "bespoke", key: "freshJackComet", entrance: "freshJackVault", aura: { look: "pixelcrawl", palette: "Tech/glitch", scope: "tile" }, takeover: "freshJack", signature: true });
  });

  it("R3407000 he vaults from the screen's centre onto his zone in a skid of pixels, after his takeover's hold", () => {
    const D = 400;
    const tgt = testid.zone("you", "units", 3);
    const slam = frac(FX_SLAM_AT, D);
    const cues = ENTRANCES.freshJackVault({ D, tgt, intensity: 1, view: fullBoardView(), landing: landing("p1") });
    expect(kinds(cues)).toEqual(["burst", "shot", "burst", "burst", "burst", "ring", "sheen", "shake"]);
    expect(cues[0]).toMatchObject({ at: CENTRE, delayMs: 0 });
    expect(cues[1]).toMatchObject({ from: CENTRE, to: tid(tgt) });
    expect(cues.slice(2, 5).every((cue) => cue.kind === "burst" && cue.at.kind === "testid" && cue.at.testid === tgt)).toBe(true);
    expect(cues.at(-1)).toEqual({ kind: "shake", trauma: ENTRANCE_TRAUMA.freshJackVault, delayMs: slam });
    const wait = 1000;
    const held = ENTRANCES.freshJackVault({ D: 1400, tgt, intensity: 1, landMs: landMsAfter(wait, 1400) });
    for (const cue of held) expect(cue.delayMs, cue.kind).toBeGreaterThanOrEqual(wait);
    expect(held.at(-1)?.delayMs).toBe(wait + frac(FX_SLAM_AT, 1400 - wait));
  });

  it("R3407000 his Cry's first cast tears the gate open on his hero (the board not showing him) and hurls a fire comet to the centre; a later one and a prompt draw nothing", () => {
    const view = fullBoardView();
    const D = 300;
    const [comet] = beatsOf(freshJackCometFixture(view, "fj", "p1"), view, D, [played("p1")]);
    expect(comet?.row).toBe("keep");
    expect(kinds(comet?.cues)).toEqual(["ring", "burst", "projectile", "burst", "burst"]);
    expect(comet?.cues[0]).toMatchObject({ at: tid(testid.hero("you")) });
    expect(comet?.cues[2]).toMatchObject({ preset: "fire", from: tid(testid.hero("you")), to: CENTRE, delayMs: frac(0.1, D) });
    expect(comet?.cues[3]).toMatchObject({ at: CENTRE, delayMs: frac(0.6, D) });
    const [, second] = beatsOf([...freshJackCometFixture(view, "fj", "p1"), ...freshJackCometFixture(view, "fj", "p1")], view, D, [played("p1")]);
    expect(second).toBeNull();
    expect(beatsOf([prompt], view, D, [played("p1")])).toEqual([null]);
    // Another Unit's announce is not his.
    expect(beatsOf(freshJackCometFixture(view, "x", "p1"), view, D, [{ ...played("p1"), instanceId: "x", defId: "core-004" }])).toEqual([null]);
  });

  it("R3407000 R200 every cue starts inside its entry and ends within FX_MAX_TAIL_MS, at any length; nothing draws with the effects off", () => {
    const view = fullBoardView();
    const tgt = testid.zone("you", "units", 3);
    for (const D of DS) {
      expectCuesR200(ENTRANCES.freshJackVault({ D, tgt, intensity: 1, view, landing: landing("p1") }), D, "freshJackVault", ENTRANCE_TRAUMA.freshJackVault);
      const comet = beatsOf(freshJackCometFixture(view, "fj", "p1"), view, D, [played("p1")]).flatMap((beat) => beat?.cues ?? []);
      expect(comet.length).toBeGreaterThan(0);
      expectCuesR200(comet, D, "freshJackComet");
    }
    const run = planWithOffsets([played("p1"), landing("p1"), ...freshJackCometFixture(view, "fj", "p1")], view, envOf(FX_INTENSITY_SCALE.off));
    expect(cuesOfRun(run)).toEqual([]);
  });

  it("R3407000 his gallery play ends within a Mythic's ceiling", () => {
    expectGalleryWithinCeiling(FRESH_JACK);
  });

  it("R3407000 R202 from the opponent's seat his Radiant Discover is never sent and his comet names no card", () => {
    const view = fullBoardView();
    const events = asTheirsAll([played("p1"), landing("p1"), { type: "hookResolving", sourceId: "fj", defId: FRESH_JACK, controller: "p1", hook: "cry" }, prompt, ...freshJackCometFixture(view, "fj", "p1")]);
    expect(events.some((event) => event.type === "promptOpened")).toBe(false);
    const cues = cuesOfRun(planWithOffsets(events, view));
    expect(cues.some((cue) => cue.kind === "projectile")).toBe(true);
    expectNoCard(cues, FRESH_JACK);
  });
});
