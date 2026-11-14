// #2793: Core's three bespoke flourishes (transmogulate.ts, zephrys.ts, craftACard.ts) pinned on their
// gallery fixtures (motifs/gallery.ts RECIPE_FIXTURES), each held inside its entry (R200), keyed only by
// a readable play and naming no hidden id (R202), and each named by its card's table entry (R502).
// R3402001: each grown into its card's signature, its cast opening it.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import { endOf, envOf } from "../../../test/signatures.ts";
import { planCardFx } from "../../cardFx.ts";
import {
  FX_CENTER,
  FX_CRUCIBLE_TRAUMA,
  FX_FORGE_REACH,
  FX_FORGE_TRAUMA,
  FX_INTENSITY_SCALE,
  FX_MAX_TAIL_MS,
  FX_TIDE_TRAUMA,
  FX_WISH_TRAUMA,
} from "../../constants.ts";
import { galleryEntry, RECIPE_FIXTURES } from "../../motifs/gallery.ts";
import type { FxAnchor, FxCue } from "../../types.ts";
import { CARD_RECIPES, type CardRecipeKey } from "../bespoke.ts";
import type { CardFxResult } from "../recipe.ts";
import { FX_CARD_TABLE } from "../tables.ts";

const beats = (cues: readonly FxCue[]): string[] => cues.map((cue) => `${cue.kind}@${String(cue.delayMs)}`);

const FLOURISHES: readonly (readonly [string, CardRecipeKey])[] = [
  ["core-083", "legendaryTide"],
  ["core-097", "perfectWish"],
  ["core-099", "fusionForge"],
];

const played = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 1 });

/**
 * Plays `defId` as `player` and each of `events` under it, an entry each lasting D, as FxLayer does:
 * remember, then plan. Each entry's cues come from the card's recipe (`direct`) or the dispatch, planCardFx.
 */
function run(defId: string, events: readonly GameEvent[], D: number, player: PlayerId = "p1", view: PlayerView = fullBoardView(), direct?: CardRecipeKey): FxCue[][] {
  return results(defId, events, D, player, view, direct).map((result) => result?.cues ?? []);
}

/** As `run`, each entry's whole answer: its cues and whether its event's own row still plays. */
function results(defId: string, events: readonly GameEvent[], D: number, player: PlayerId = "p1", view: PlayerView = fullBoardView(), direct?: CardRecipeKey): (CardFxResult | null)[] {
  const env = envOf();
  env.memory.remember([played(player, "ex", defId)]);
  return events.map((event) => {
    env.memory.remember([event]);
    const entry = { ...galleryEntry([event], view), durationMs: D };
    const p = { entry, view, env, D };
    const play = env.memory.resolving();
    if (direct === undefined) return planCardFx(event, p);
    if (play === undefined) throw new Error("no play resolving");
    return CARD_RECIPES[direct](event, p, play);
  });
}

/** R3402001: the cues `key` plans for its own play's `cardPlayed`, the signature's first beat, over an entry of D with no hold. */
function castCues(defId: string, key: CardRecipeKey, D = 400): FxCue[] {
  return castResult(defId, key, D)?.cues ?? [];
}

/** As `castCues`, the recipe's whole answer. */
function castResult(defId: string, key: CardRecipeKey, D = 400): CardFxResult | null {
  const env = envOf();
  const view = fullBoardView();
  const event = played("p1", "ex", defId);
  env.memory.remember([event]);
  const play = env.memory.resolving();
  if (play === undefined) throw new Error("no play resolving");
  const entry = { ...galleryEntry([event], view), durationMs: D };
  return CARD_RECIPES[key](event, { entry, view, env, D }, play);
}

function fixtureCues(defId: string, key: CardRecipeKey, D = 400): FxCue[][] {
  const view = fullBoardView();
  return run(defId, RECIPE_FIXTURES[key](view, "ex", "p1"), D, "p1", view, key);
}

const tid = (value: string, at?: { x: number; y: number }): FxAnchor => (at === undefined ? { kind: "testid", testid: value } : { kind: "testid", testid: value, at });

describe("#2793 Core's bespoke flourishes", () => {
  it("R2784000 legendaryTide washes the caster's Deck and hand piles at its first transform, a sheen on each hand card the viewer reads", () => {
    const view = fullBoardView();
    const [cues = []] = fixtureCues("core-083", "legendaryTide");
    // At D 400: gather 0, travel 80, land 240, linger 300. R3402001: the wave crests over both piles in a ring and a sheen.
    expect(beats(cues)).toEqual(["ring@0", "sheen@0", "ring@0", "sheen@0", "shake@80", "sheen@80"]);
    expect(cues.filter((cue) => cue.kind === "ring").map((cue) => cue.kind === "ring" && cue.at)).toEqual([tid(animTestid.library("you")), tid(animTestid.hand("you"))]);
    expect(cues.find((cue) => cue.kind === "shake")).toEqual({ kind: "shake", trauma: Math.min(1, FX_TIDE_TRAUMA * FX_INTENSITY_SCALE.normal), delayMs: 80 });
    const hand = Array.isArray(view.you.hand) ? view.you.hand[0] : undefined;
    expect(cues.filter((cue) => cue.kind === "sheen").at(-1)).toEqual({ kind: "sheen", at: tid(`hand-card-${hand?.instanceId ?? ""}`), delayMs: 80, durationMs: 160 });

    // A later transform draws its own sheen only; one the viewer may not read draws nothing.
    const second = Array.isArray(view.you.hand) ? view.you.hand[1] : undefined;
    const turn = (instanceId: string, hiddenFrom?: PlayerId[]): GameEvent => ({
      type: "transformed",
      instanceId,
      fromDefId: "core-019",
      toDefId: "core-052",
      newInstanceId: instanceId,
      ...(hiddenFrom === undefined ? {} : { hiddenFrom }),
    });
    const [first = [], later = [], hidden = []] = run("core-083", [turn(hand?.instanceId ?? ""), turn(second?.instanceId ?? ""), turn("hidden", ["p1"])], 400, "p1", view, "legendaryTide");
    expect(first.some((cue) => cue.kind === "ring")).toBe(true);
    expect(beats(later)).toEqual(["sheen@80"]);
    expect(hidden).toEqual([]);
  });

  it("R2784000 perfectWish blows the caster's card in from the far hero into its hand pile; the other player's card draws nothing", () => {
    const view = fullBoardView();
    const [cues = []] = fixtureCues("core-097", "perfectWish");
    // R3402001: one card back on a gust from the far hero into the hand pile, which rings and takes a sheen.
    expect(beats(cues)).toEqual(["ghost@0", "ring@0", "sheen@0", "shake@240"]);
    expect(cues.find((cue) => cue.kind === "ghost")).toEqual({ kind: "ghost", from: tid(testid.hero("opponent")), to: tid(animTestid.hand("you")), delayMs: 0, durationMs: 400 });
    expect(cues.find((cue) => cue.kind === "shake")).toEqual({ kind: "shake", trauma: Math.min(1, FX_WISH_TRAUMA * FX_INTENSITY_SCALE.normal), delayMs: 240 });
    const [theirs = []] = run("core-097", [{ type: "addedToHand", player: "p2", instanceId: "hidden", defId: "hidden" }], 400, "p1", view, "perfectWish");
    expect(theirs).toEqual([]);
  });

  it("R2784000 fusionForge forges two halves from the board's edges at the fuse, and flies the crafted card on into the hand", () => {
    const [added = [], cues = []] = fixtureCues("core-099", "fusionForge");
    // The engine adds the crafted card to the hand before it reports the fuse: R3402001 the hand pile rings
    // as it does, and the forge carries its flight in place of the rows.
    expect(added).toEqual([{ kind: "ring", preset: "frost", at: tid(animTestid.hand("you")), delayMs: 0, durationMs: expect.any(Number) as number }]);
    expect(beats(cues)).toEqual(["ghost@0", "shake@0"]);
    const centre = tid(testid.board, FX_CENTER);
    expect(cues.find((cue) => cue.kind === "ghost")).toEqual({ kind: "ghost", from: centre, to: tid(animTestid.hand("you")), delayMs: 0, durationMs: 240 });
    expect(cues.find((cue) => cue.kind === "shake")).toEqual({ kind: "shake", trauma: Math.min(1, FX_FORGE_TRAUMA * FX_INTENSITY_SCALE.normal), delayMs: 0 });
  });

  it("R3402001 each flourish's cast opens its signature: the crucible at the hero, the silk and the forge's halves at the centre", () => {
    const hero = tid(testid.hero("you"));
    const centre = tid(testid.board, FX_CENTER);
    const tide = castCues("core-083", "legendaryTide");
    expect(beats(tide)).toEqual(["burst@0", "rays@0", "ring@80", "burst@240", "burst@240", "shake@240"]);
    for (const cue of tide) if (cue.kind !== "shake") expect("at" in cue ? cue.at : undefined, cue.kind).toEqual(hero);
    expect(tide.find((cue) => cue.kind === "rays")).toMatchObject({ tone: "legendary" });
    expect(tide.find((cue) => cue.kind === "shake")).toEqual({ kind: "shake", trauma: Math.min(1, FX_CRUCIBLE_TRAUMA * FX_INTENSITY_SCALE.normal), delayMs: 240 });
    // With no takeover hold (Big moments off, or a bare entry) the silk settles and the halves meet as the entry lands.
    const wish = castCues("core-097", "perfectWish");
    expect(beats(wish)).toEqual(["burst@0", "ring@120", "ring@240", "burst@240"]);
    for (const cue of wish) expect("at" in cue ? cue.at : undefined, cue.kind).toEqual(centre);
    const forge = castCues("core-099", "fusionForge");
    expect(beats(forge)).toEqual(["burst@0", "burst@0", "ghost@120", "ghost@120", "burst@240", "ring@240"]);
    expect(forge.filter((cue) => cue.kind === "ghost").map((cue) => cue.kind === "ghost" && [cue.from, cue.to, cue.durationMs])).toEqual([
      [tid(testid.board, { x: FX_FORGE_REACH, y: FX_CENTER.y }), centre, 120],
      [tid(testid.board, { x: 1 - FX_FORGE_REACH, y: FX_CENTER.y }), centre, 120],
    ]);
    // Not cast by the play itself, it draws no first beat: another card's play draws none of it.
    for (const [defId, key] of FLOURISHES) {
      const env = envOf();
      env.memory.remember([played("p1", "ex", defId)]);
      const other = played("p1", "other", "core-003");
      const view = fullBoardView();
      const entry = { ...galleryEntry([other], view), durationMs: 400 };
      const play = env.memory.resolving();
      if (play === undefined) throw new Error("no play resolving");
      expect(CARD_RECIPES[key](other, { entry, view, env, D: 400 }, play), key).toBeNull();
    }
  });

  it("R200 every flourish cue starts inside its entry and ends within FX_MAX_TAIL_MS", () => {
    for (const [defId, key] of FLOURISHES) {
      for (const D of [120, 200, 400, 732, 1400]) {
        const cues = fixtureCues(defId, key, D).flat();
        expect(cues.length, `${key} at D=${String(D)}`).toBeGreaterThan(0);
        for (const cue of cues) {
          const label = `${key} ${cue.kind} at D=${String(D)}`;
          expect(cue.delayMs, label).toBeGreaterThanOrEqual(0);
          expect(cue.delayMs, label).toBeLessThanOrEqual(D);
          if (cue.kind === "shot") expect(cue.delayMs + cue.flightMs, label).toBeLessThanOrEqual(D);
          if (cue.kind === "ghost") expect(cue.delayMs + cue.durationMs, label).toBeLessThanOrEqual(D);
          expect(endOf(cue), label).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
          if (cue.kind === "shake") expect(cue.trauma, label).toBeLessThanOrEqual(1);
        }
      }
    }
  });

  it("R202 a hidden play keys none, and the other seat's cues name no hidden id", () => {
    const view = fullBoardView();
    for (const [defId, key] of FLOURISHES) {
      const events = RECIPE_FIXTURES[key](view, "ex", "p1");
      expect(run("hidden", events, 400).flat(), key).toEqual([]);
      // The same play by the other seat, its cards hidden from the viewer as R97 sends them.
      const theirs: GameEvent[] = events.map((event) => {
        if (event.type === "transformed") return { ...event, instanceId: "hidden", fromDefId: "hidden", toDefId: "hidden", newInstanceId: "hidden", hiddenFrom: ["p1"] };
        if (event.type === "addedToHand") return { ...event, player: "p2", instanceId: "hidden", defId: "hidden" };
        if (event.type === "fused") return { ...event, instanceIds: ["hidden", "hidden"], resultInstanceId: "hidden", defId: "hidden" };
        return event;
      });
      const cues = run(defId, theirs, 400, "p2", view).flat();
      expect(cues.length, key).toBeGreaterThan(0);
      const text = JSON.stringify(cues);
      expect(text, key).not.toMatch(/hidden|fx-|core-\d{3}|hand-card-/);
      expect(text, key).toContain(animTestid.hand("opponent"));
    }
  });

  it("R502 core-083, core-097 and core-099 key their recipes", () => {
    const view = fullBoardView();
    for (const [defId, key] of FLOURISHES) {
      // R3332000: Zephrys's entry also names its takeover look; R3402001 each is its card's signature, Craft a Card's with its own look too.
      const takeover = defId === "core-097" ? { takeover: "genie" } : defId === "core-099" ? { takeover: "crystalForge" } : {};
      expect(FX_CARD_TABLE.get(defId), defId).toEqual({ kind: "bespoke", key, ...takeover, signature: true });
      expect(run(defId, RECIPE_FIXTURES[key](view, "ex", "p1"), 400).flat(), defId).toEqual(fixtureCues(defId, key).flat());
    }
  });
});

/** A cue as its kind and the look its strings name: a burst's preset and spread, a ring's preset, rays' tone. */
const shape = (cue: FxCue): string =>
  cue.kind === "burst" ? `burst ${cue.preset} ${cue.spread}` : cue.kind === "ring" ? `ring ${cue.preset}` : cue.kind === "rays" ? `rays ${cue.tone}` : cue.kind;

/** A transform of `instanceId` into `newInstanceId`, hidden from `hiddenFrom` when given. */
const turned = (instanceId: string, newInstanceId = instanceId, hiddenFrom?: PlayerId[]): Extract<GameEvent, { type: "transformed" }> => ({
  type: "transformed",
  instanceId,
  fromDefId: "core-019",
  toDefId: "core-052",
  newInstanceId,
  ...(hiddenFrom === undefined ? {} : { hiddenFrom }),
});

/** Transmogulate's answer for each of `events`, all planned in one entry of D after its cast, as FxLayer plans an entry of many events. */
function tideInOneEntry(events: readonly GameEvent[], view: PlayerView, D = 400): (CardFxResult | null)[] {
  const env = envOf();
  env.memory.remember([played("p1", "ex", "core-083")]);
  env.memory.remember(events);
  const play = env.memory.resolving();
  if (play === undefined) throw new Error("no play resolving");
  const entry = { ...galleryEntry([...events], view), durationMs: D };
  return events.map((event) => CARD_RECIPES.legendaryTide(event, { entry, view, env, D }, play));
}

describe("R3402001 Core's flourishes, beat by beat", () => {
  it("R3402001 Transmogulate's crucible: a poison gather in a ring, Legendary rays, its ring, vials bursting prismatic over the area and shards at the point, over the event rows", () => {
    const cast = castResult("core-083", "legendaryTide");
    expect(cast?.row).toBe("keep");
    expect(cast?.cues.map(shape)).toEqual(["burst poison ring", "rays legendary", expect.stringMatching(/^ring /) as string, "burst prismatic area", "burst shard point", "shake"]);
  });

  it("R2784000 Transmogulate's wave crests over both piles in prismatic rings at the first transform only, and each readable card takes its own sheen", () => {
    const view = fullBoardView();
    const [hand, second] = Array.isArray(view.you.hand) ? view.you.hand : [];
    if (hand === undefined || second === undefined) throw new Error("no hand");
    const library = tid(animTestid.library("you"));
    const pile = tid(animTestid.hand("you"));
    const sheen = (id: string): FxCue => ({ kind: "sheen", at: tid(`hand-card-${id}`), delayMs: 80, durationMs: 160 });
    // At D 400: gather 0, travel 80, land 240.
    const piles: FxCue[] = [
      { kind: "ring", preset: "prismatic", at: library, delayMs: 0, durationMs: expect.any(Number) as number },
      { kind: "sheen", at: library, delayMs: 0, durationMs: 240 },
      { kind: "ring", preset: "prismatic", at: pile, delayMs: 0, durationMs: expect.any(Number) as number },
      { kind: "sheen", at: pile, delayMs: 0, durationMs: 240 },
      { kind: "shake", trauma: Math.min(1, FX_TIDE_TRAUMA * FX_INTENSITY_SCALE.normal), delayMs: 80 },
    ];
    // Two transforms in one entry: the wave crests at the first of them alone.
    const [first, later] = tideInOneEntry([turned(hand.instanceId), turned(second.instanceId)], view);
    expect(first).toEqual({ cues: [...piles, sheen(hand.instanceId)], row: "replace" });
    expect(later).toEqual({ cues: [sheen(second.instanceId)], row: "replace" });
    // Another event ahead of it in its entry: the first transform still crests.
    const drawn: GameEvent = { type: "drawn", player: "p1", instanceId: "hidden", defId: "hidden" };
    expect(tideInOneEntry([drawn, turned(hand.instanceId)], view)[1]).toEqual({ cues: [...piles, sheen(hand.instanceId)], row: "replace" });
  });

  it("R202 Transmogulate draws a sheen only on a card the viewer reads and the view shows: none for one hidden from the viewer, one by its new id, none for an id the view cannot find", () => {
    const view = fullBoardView();
    const [hand, second] = Array.isArray(view.you.hand) ? view.you.hand : [];
    if (hand === undefined || second === undefined) throw new Error("no hand");
    const sheen = (id: string): FxCue => ({ kind: "sheen", at: tid(`hand-card-${id}`), delayMs: 80, durationMs: 160 });
    // Each after a first transform, so none crests over the piles.
    const after = (event: GameEvent): CardFxResult | null => results("core-083", [turned("gone"), event], 400, "p1", view, "legendaryTide")[1] ?? null;
    expect(after(turned(hand.instanceId, hand.instanceId, ["p1"]))).toEqual({ cues: [], row: "replace" });
    expect(after(turned(hand.instanceId, hand.instanceId, ["p2"]))).toEqual({ cues: [sheen(hand.instanceId)], row: "replace" });
    expect(after(turned("gone", second.instanceId))).toEqual({ cues: [sheen(second.instanceId)], row: "replace" });
    expect(after(turned("gone"))).toEqual({ cues: [], row: "replace" });
  });

  it("R3402001 Transmogulate Embiggened re-forms a Unit in a prismatic ring and a sheen from its travel to its landing", () => {
    const view = fullBoardView();
    const mine = view.you.units[0];
    if (mine === null || mine === undefined) throw new Error("no unit");
    const card = tid(testid.card(mine.instanceId));
    const [, ripple] = results("core-083", [turned("gone"), turned(mine.instanceId)], 400, "p1", view, "legendaryTide");
    // At D 400: travel 80, land 240.
    expect(ripple).toEqual({
      cues: [
        { kind: "ring", preset: "prismatic", at: card, delayMs: 80, durationMs: expect.any(Number) as number },
        { kind: "sheen", at: card, delayMs: 80, durationMs: 160 },
      ],
      row: "replace",
    });
  });

  it("R3402001 Zephrys's silk gathers in sparkles over the area and settles in frost rings, over the event rows; the wish rings the hand pile in frost in place of its row", () => {
    const cast = castResult("core-097", "perfectWish");
    expect(cast?.row).toBe("keep");
    expect(cast?.cues.map(shape)).toEqual(["burst sparkle area", "ring frost", expect.stringMatching(/^ring /) as string, "burst frost ring"]);
    const [wish] = results("core-097", RECIPE_FIXTURES.perfectWish(fullBoardView(), "ex", "p1"), 400, "p1", fullBoardView(), "perfectWish");
    expect(wish?.row).toBe("replace");
    expect(wish?.cues.map(shape)).toEqual(["ghost", "ring frost", "sheen", "shake"]);
  });

  it("R2784000 Zephrys played by the other seat gusts from the viewer's hero into that seat's hand pile; a hook of its other than its cast draws nothing", () => {
    const view = fullBoardView();
    const theirs: GameEvent = { type: "addedToHand", player: "p2", instanceId: "hidden", defId: "hidden" };
    const [gust = []] = run("core-097", [theirs], 400, "p2", view, "perfectWish");
    expect(gust.find((cue) => cue.kind === "ghost")).toEqual({ kind: "ghost", from: tid(testid.hero("you")), to: tid(animTestid.hand("opponent")), delayMs: 0, durationMs: 400 });
    // R2722002: only its cast plays it; under any other hook (here a turn hook's entry) it draws nothing.
    const env = envOf();
    env.memory.remember([played("p1", "ex", "core-097")]);
    const play = env.memory.resolving();
    if (play === undefined) throw new Error("no play resolving");
    const mine: GameEvent = { type: "addedToHand", player: "p1", instanceId: "w", defId: "core-001" };
    const entry = { ...galleryEntry([mine], view), durationMs: 400 };
    const p = { entry, view, env, D: 400 };
    expect(CARD_RECIPES.perfectWish(mine, p, { ...play, hook: "trigger" })).toBeNull();
    expect(CARD_RECIPES.perfectWish(mine, p, { ...play, hook: "cast" })?.cues.map((cue) => cue.kind)).toEqual(["ghost", "ring", "sheen", "shake"]);
  });
});
