// R3407000: Megabase's signature (megabase.ts), planned as FxLayer plans it: the sky darkens as the station
// lowers onto its zone ring by ring, its clamps thud and its lobby lights blink on; its Activate opens the
// gift-shop hatch and fountains a credit chip to each hero that gains, and its Death bursts its rings and
// showers gold over each hero. Its gallery play and its Death from its marker within a Legendary's ceiling,
// R200 at any length, nothing with the effects off; Credits are public and no cue names a card (R202).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import {
  beatsOf,
  ceilingOf,
  cuesOfRun,
  envOf,
  expectCuesR200,
  expectGalleryWithinCeiling,
  expectNoCard,
  expectR200,
  latestEnd,
  planWithOffsets,
} from "../../../test/signaturePass.ts";
import { frac } from "../../build.ts";
import { FX_INTENSITY_SCALE, FX_SLAM_AT } from "../../constants.ts";
import { ENTRANCES } from "../../entrances.ts";
import type { FxAnchor, FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA } from "../recipe.ts";
import { FX_CARD_TABLE } from "../tables.ts";
import { giftShopFixture } from "./megabase.ts";

const MEGABASE = "stinkyscape-116";
const DS = [120, 200, 400, 732, 1400] as const;

const tid = (value: string, at?: { x: number; y: number }): FxAnchor => (at === undefined ? { kind: "testid", testid: value } : { kind: "testid", testid: value, at });
const kinds = (cues: readonly FxCue[] | undefined): string[] => (cues ?? []).map((cue) => cue.kind);
const credits = (player: PlayerId): GameEvent => ({ type: "marketCreditsChanged", player, value: 10 });
const death = (player: PlayerId, id: string): GameEvent => ({ type: "hookResolving", sourceId: id, defId: MEGABASE, controller: player, hook: "death" });

/** The viewer's first Unit as Megabase, standing face up in lane 1. */
function standingView(): { view: PlayerView; id: string } {
  const view = fullBoardView();
  const unit = view.you.units[0];
  if (unit === null || unit === undefined) throw new Error("no unit to stand in");
  view.you.units = [{ ...unit, defId: MEGABASE }, ...view.you.units.slice(1)];
  return { view, id: unit.instanceId };
}

describe("R3407000 Megabase's signature", () => {
  it("R3407000 its table entry is its signature", () => {
    expect(FX_CARD_TABLE.get(MEGABASE)).toEqual({ kind: "bespoke", key: "giftShop", entrance: "megabaseDock", aura: { look: "stationrings", palette: "Tech/starfield", scope: "tile" }, signature: true });
  });

  it("R3407000 the sky darkens as the station lowers onto its zone ring by ring, its clamps thudding and its lights blinking on", () => {
    const D = 400;
    const tgt = testid.zone("you", "units", 1);
    const slam = frac(FX_SLAM_AT, D);
    const cues = ENTRANCES.megabaseDock({ D, tgt, intensity: 1, view: fullBoardView() });
    expect(kinds(cues)).toEqual(["figure", "shot", "ring", "ring", "ring", "burst", "burst", "burst", "burst", "shake"]);
    expect(cues[0]).toMatchObject({ at: tid(testid.board) });
    expect(cues[1]).toMatchObject({ to: tid(tgt) });
    expect(cues.slice(2, 5).every((cue) => cue.kind === "ring" && cue.at.kind === "testid" && cue.at.testid === tgt)).toBe(true);
    expect(cues.at(-1)).toEqual({ kind: "shake", trauma: ENTRANCE_TRAUMA.megabaseDock, delayMs: slam });
  });

  it("R3407000 its Activate opens the hatch and fountains a chip to each hero that gains; its Death bursts its rings and showers gold over each hero", () => {
    const { view, id } = standingView();
    const D = 400;
    const hatch = tid(testid.card(id));
    const [marker, mine, theirs] = beatsOf([...giftShopFixture(view, id, "p1"), credits("p2")], view, D);
    expect(marker).toBeNull();
    expect(kinds(mine?.cues)).toEqual(["burst", "projectile"]);
    expect(mine?.cues[1]).toMatchObject({ preset: "gold", from: hatch, to: tid(testid.hero("you")) });
    expect(kinds(theirs?.cues)).toEqual(["projectile"]);
    expect(theirs?.cues[0]).toMatchObject({ to: tid(testid.hero("opponent")) });
    const [, first, second] = beatsOf([death("p1", id), credits("p1"), credits("p2")], view, D);
    expect(kinds(first?.cues)).toEqual(["ring", "burst", "burst"]);
    expect(first?.cues[0]).toMatchObject({ at: hatch });
    expect(kinds(second?.cues)).toEqual(["burst"]);
    expect(second?.cues[0]).toMatchObject({ at: tid(testid.hero("opponent")), preset: "gold" });
    // Credits in its play, or in another card's Activate, are not its gift shop.
    expect(beatsOf([credits("p1")], view, D, [{ type: "cardPlayed", player: "p1", instanceId: id, defId: MEGABASE, costPaid: 6 }])).toEqual([null]);
    expect(beatsOf([credits("p1")], view, D, [{ type: "hookResolving", sourceId: "else", defId: "core-004", controller: "p1", hook: "activate" }])).toEqual([null]);
  });

  it("R3407000 R200 every cue starts inside its entry and ends within FX_MAX_TAIL_MS, at any length; nothing draws with the effects off", () => {
    const { view, id } = standingView();
    const tgt = testid.zone("you", "units", 1);
    for (const D of DS) {
      expectCuesR200(ENTRANCES.megabaseDock({ D, tgt, intensity: 1, view }), D, "megabaseDock", ENTRANCE_TRAUMA.megabaseDock);
      const shop = [...beatsOf([...giftShopFixture(view, id, "p1"), credits("p2")], view, D), ...beatsOf([death("p1", id), credits("p1"), credits("p2")], view, D)].flatMap((beat) => beat?.cues ?? []);
      expect(shop.length).toBeGreaterThan(0);
      expectCuesR200(shop, D, "giftShop");
      expectNoCard(shop, MEGABASE);
    }
    expect(cuesOfRun(planWithOffsets([...giftShopFixture(view, id, "p1"), death("p1", id), credits("p1")], view, envOf(FX_INTENSITY_SCALE.off)))).toEqual([]);
  });

  it("R3407000 its gallery play and its Death from its marker end within a Legendary's ceiling", () => {
    expectGalleryWithinCeiling(MEGABASE);
    const { view, id } = standingView();
    const run = planWithOffsets([death("p1", id), credits("p1"), credits("p2")], view);
    expect(cuesOfRun(run).length).toBeGreaterThan(0);
    expect(latestEnd(run)).toBeLessThanOrEqual(ceilingOf(MEGABASE));
    expectR200(run, MEGABASE);
  });
});
