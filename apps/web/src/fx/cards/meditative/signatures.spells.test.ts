// R3405001: Meditative's seven Spell signatures, card by card (reachTheSummit.ts, trueCraftACard.ts,
// aestheticizeTheGame.ts, redJade.ts, cnFleaMarket.ts, callToChaos.ts, evilBlueprints.ts): each cast's own
// figures, a Mythic's nothing before its takeover's hold and a Legendary's from the start; then each beat of
// its text planned from the events its script emits under its play, as the board's effects layer plans them
// (remember, then plan through the dispatch): the figures, ghosts and bursts it draws, every card flying to
// its pile as a card back, an owner-only sheen on the caster's seat alone, a summon of a card with an entrance
// of its own playing that entrance, and an event its recipe does not claim left to its own row.

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import { HOLD, added, beats, castOf, envOf, played, run, tid } from "../../../test/signatures.ts";
import { frac } from "../../build.ts";
import { planCardFx } from "../../cardFx.ts";
import { FX_CENTER, FX_CHAOS_DICE_PANEL_MS, FX_CHAOS_DICE_REEL_MS } from "../../constants.ts";
import { figureTint } from "../../figures.ts";
import { paletteLook, type FlavorLook } from "../../flavors.ts";
import { galleryEntry } from "../../motifs/gallery.ts";

const SUMMIT = "meditative-008";
const CRAFT = "meditative-017";
const AESTHETICIZE = "meditative-020";
const RED_JADE = "meditative-039-4";
const JADE_BEAUTY = "meditative-039-5";
const MARKET = "meditative-042";
const CHAOS = "meditative-095";
const CN_GOLEM = "meditative-095-1";
const BLUEPRINTS = "meditative-097";

/** A Legendary's entry; a Mythic's, its takeover's hold and as long again after it. */
const D = 400;
const MYTHIC_D = HOLD + D;

/** The board's centre, where a Spell with no card of its own on the board draws. */
const CENTRE = tid(testid.board, FX_CENTER);

const UNCLAIMED = { cues: [], row: undefined };

const drawn = (player: "p1" | "p2", instanceId = "fx-drawn", defId = "core-002"): GameEvent => ({ type: "drawn", player, instanceId, defId });

const foeOf = (view = fullBoardView()): string => view.opponent.units.find((unit) => unit !== null)?.instanceId ?? "";

/** The look a recipe draws in, by its palette. */
function lookOf(palette: string): FlavorLook {
  const look = paletteLook(palette);
  if (look === undefined) throw new Error(`no look ${palette}`);
  return look;
}

describe("R3405001 Meditative's Spell signatures", () => {
  it("R3405001 Reach the Summit: after its hold her ledge flares under the hero in mist; each Ascent plays in ink, a back climbing per draw, the Radiant cards' glint the caster's alone, and the tenth's win lights a star", () => {
    const cast = castOf(SUMMIT, MYTHIC_D);
    const hero = tid(testid.hero("you"));
    expect(cast.row).toBe("replace");
    expect(beats(cast.cues)).toEqual([`figure:ledge@${String(HOLD)}`, `burst@${String(HOLD)}`]);
    expect(cast.cues).toMatchObject([
      { figure: "ledge", at: hero, side: "you", durationMs: D + 400, tint: figureTint(lookOf("Nerd/ink")) },
      { preset: "smoke", spread: "area", at: hero, colors: lookOf("Nerd/ink").colors },
    ]);
    const view = fullBoardView();
    const foe = foeOf(view);
    const hand = tid(animTestid.hand("you"));
    const [mist, rock, climb, torn, fade, radiant, summit, otherWin, ownDiscard] = run(
      SUMMIT,
      [
        { type: "healed", targetId: "hero-p1", amount: 5 },
        { type: "damage", sourceId: "ex", targetId: foe, amount: 3, combat: false },
        drawn("p1"),
        { type: "discarded", instanceId: "fx-torn", defId: "core-002", owner: "p2" },
        { type: "exiled", instanceId: foe, defId: "core-008", owner: "p2" },
        added("p1", "fx-radiant", "core-055"),
        { type: "gameOver", winner: "p1", reason: "won-by-effect" },
        { type: "gameOver", winner: "p1", reason: "hero-death" },
        { type: "discarded", instanceId: "fx-mine", defId: "core-002", owner: "p1" },
      ],
      D,
      "p1",
      view,
    );
    // The mist and the rock decorate their rows, which keep the number.
    expect(mist).toMatchObject({ cues: [{ kind: "burst", preset: "smoke", at: hero }], row: "keep" });
    expect(rock).toMatchObject({ cues: [{ kind: "burst", preset: "crumb", at: tid(testid.card(foe), { x: 0.5, y: 0 }) }], row: "keep" });
    const climbAt = frac(0.2, D);
    expect(climb).toEqual({ cues: [{ kind: "ghost", from: tid(animTestid.library("you")), to: hand, delayMs: climbAt, durationMs: D - climbAt }], row: "replace" });
    expect(torn).toMatchObject({ cues: [{ kind: "burst", at: tid(animTestid.hand("opponent")) }], row: "keep" });
    expect(fade).toMatchObject({ cues: [{ kind: "burst", preset: "smoke", at: tid(testid.card(foe)) }], row: "keep" });
    expect(radiant).toEqual({ cues: [{ kind: "ghost", from: hero, to: hand, delayMs: 0, durationMs: D }, { kind: "sheen", at: hand, delayMs: 0, durationMs: D }], row: "replace" });
    expect(summit).toMatchObject({ cues: [{ kind: "figure", figure: "peakStar", at: hero, side: "you", delayMs: 0 }, { kind: "burst", preset: "star", spread: "ring", at: hero }], row: "replace" });
    // A win by any other way, or the caster's own discard, is no Ascent of its.
    expect([otherWin, ownDiscard]).toEqual([UNCLAIMED, UNCLAIMED]);
    // The other seat's climb: card backs to its piles, its glint unseen here.
    const theirs = run(SUMMIT, [drawn("p2", "hidden", "hidden"), added("p2", "hidden", "hidden")], D, "p2", view);
    expect(theirs[0]?.cues[0]).toMatchObject({ kind: "ghost", from: tid(animTestid.library("opponent")), to: tid(animTestid.hand("opponent")) });
    expect(theirs[1]?.cues).toEqual([{ kind: "ghost", from: tid(testid.hero("opponent")), to: tid(animTestid.hand("opponent")), delayMs: 0, durationMs: D }]);
  });

  it("R3405001 True Craft a Card: after its hold pixel threads stitch a blank card at the centre; the other seat sees it hum at each prompt, the minted card flies to the hand as a card back, its sheen the caster's alone, and the Spell crumbles", () => {
    const cast = castOf(CRAFT, MYTHIC_D);
    expect(cast.row).toBe("replace");
    expect(beats(cast.cues)).toEqual([`figure:pixelThreads@${String(HOLD)}`, `burst@${String(HOLD)}`]);
    expect(cast.cues).toMatchObject([
      { figure: "pixelThreads", at: CENTRE, durationMs: D + 400, tint: figureTint(lookOf("Nerd/pixel")) },
      { spread: "area", at: CENTRE, colors: lookOf("Nerd/pixel").colors },
    ]);
    const view = fullBoardView();
    const hand = tid(animTestid.hand("you"));
    const prompt: GameEvent = { type: "promptOpened", player: "p1" };
    const [editor, minted, crumble, otherExile] = run(
      CRAFT,
      [prompt, added("p1", "fx-crafted", "core-055"), { type: "exiled", instanceId: "ex", defId: CRAFT, owner: "p1" }, { type: "exiled", instanceId: "fx-other", defId: "core-002", owner: "p1" }],
      D,
      "p1",
      view,
    );
    // The caster has the editor: its own prompt draws nothing over it.
    expect(editor).toEqual(UNCLAIMED);
    const leave = frac(0.3, D);
    expect(minted).toEqual({ cues: [{ kind: "ghost", from: CENTRE, to: hand, delayMs: leave, durationMs: D - leave }, { kind: "sheen", at: hand, delayMs: leave, durationMs: D - leave }], row: "replace" });
    expect(crumble).toMatchObject({ cues: [{ kind: "figure", figure: "pixelCrumble", at: CENTRE, delayMs: 0, durationMs: frac(0.8, D) }], row: "replace" });
    expect(crumble?.cues).toHaveLength(1);
    // Another card's exile is no crumble of its.
    expect(otherExile).toEqual(UNCLAIMED);
    // The other seat's craft: the blank card hums at each prompt, and its card back flies to its hand pile, no sheen here.
    const [hum, back] = run(CRAFT, [{ type: "promptOpened", player: "p2" }, added("p2", "hidden", "hidden")], D, "p2", view);
    expect(hum).toMatchObject({ cues: [{ kind: "figure", figure: "pixelThreads", at: CENTRE, delayMs: 0, durationMs: D + 400 }], row: "replace" });
    expect(back?.cues).toEqual([{ kind: "ghost", from: CENTRE, to: tid(animTestid.hand("opponent")), delayMs: leave, durationMs: D - leave }]);
  });

  it("R3405001 Aestheticize the Game: after its hold one vermilion stroke sweeps the centre in ink; the condition picked dries onto the caster's hero as a brush seal, and its win blooms the seal again", () => {
    const cast = castOf(AESTHETICIZE, MYTHIC_D);
    const seal = figureTint(lookOf("Dark/blood"));
    expect(cast.row).toBe("replace");
    expect(beats(cast.cues)).toEqual([`figure:inkStroke@${String(HOLD)}`, `burst@${String(HOLD)}`]);
    // The stroke is laid in the seal's vermilion; its splash is ink.
    expect(cast.cues).toMatchObject([
      { figure: "inkStroke", at: CENTRE, side: "you", durationMs: D + 400, tint: seal },
      { spread: "area", at: CENTRE, colors: lookOf("Nerd/ink").colors },
    ]);
    const view = fullBoardView();
    const hero = tid(testid.hero("you"));
    const installed = (player: "p1" | "p2", on = true): GameEvent => ({ type: "modifierChanged", player, modifierId: "fx-alt-win", added: on });
    const [dried, win, prompt, draw, lifted, theirWin] = run(
      AESTHETICIZE,
      [installed("p1"), { type: "gameOver", winner: "p1", reason: "alt-win" }, { type: "promptOpened", player: "p1" }, drawn("p1"), installed("p1", false), { type: "gameOver", winner: "p2", reason: "alt-win" }],
      D,
      "p1",
      view,
    );
    expect(dried).toEqual({ cues: [{ kind: "figure", figure: "brushSeal", at: hero, tint: seal, delayMs: 0, durationMs: D + 300 }], row: "replace" });
    expect(win).toMatchObject({ cues: [{ kind: "figure", figure: "brushSeal", at: hero, tint: seal, delayMs: 0 }, { kind: "burst", preset: "sparkle", spread: "ring", at: hero, colors: lookOf("Dark/blood").colors }], row: "replace" });
    // Its prompt, its Radiant draw, a condition lifted and the other player's win keep their own rows.
    expect([prompt, draw, lifted, theirWin]).toEqual([UNCLAIMED, UNCLAIMED, UNCLAIMED, UNCLAIMED]);
    // The other seat's seal dries onto its own hero.
    const [theirs] = run(AESTHETICIZE, [installed("p2")], D, "p2", view);
    expect(theirs?.cues[0]).toMatchObject({ figure: "brushSeal", at: tid(testid.hero("opponent")) });
    expect(castOf(AESTHETICIZE, MYTHIC_D, HOLD, "p2", view).cues[0]).toMatchObject({ side: "opponent" });
  });

  it("R3405001 Red Jade: after its hold a red jade shard glints out at the centre in red splinters; as the caster's Jade Counter rises it drops into the counter by the hero in a ring of jade light, and the Jade Beauty it summons lands with her own entrance", () => {
    const cast = castOf(RED_JADE, MYTHIC_D);
    const tint = figureTint(lookOf("Dark/blood"));
    expect(cast.row).toBe("replace");
    expect(beats(cast.cues)).toEqual([`figure:jadeShard@${String(HOLD)}`, `burst@${String(HOLD)}`]);
    expect(cast.cues).toMatchObject([
      { figure: "jadeShard", at: CENTRE, durationMs: D + 400, tint },
      { preset: "shard", spread: "area", at: CENTRE, colors: lookOf("Dark/blood").colors },
    ]);
    const view = fullBoardView();
    const hero = tid(testid.hero("you"));
    const rose = (player: "p1" | "p2", value: number): GameEvent => ({ type: "jadeChanged", player, value });
    const beauty: GameEvent = { type: "summoned", player: "p1", instanceId: "fx-beauty", defId: JADE_BEAUTY, row: "units", lane: 3 };
    const [drop, landing, theirs] = run(RED_JADE, [rose("p1", 5), beauty, rose("p2", 2)], D, "p1", view);
    expect(drop?.row).toBe("replace");
    expect(drop?.cues).toMatchObject([
      { kind: "figure", figure: "jadeShard", at: hero, tint, side: "you", delayMs: 0, durationMs: D + 300 },
      { kind: "ring", preset: "poison", at: hero, delayMs: 0 },
    ]);
    expect(drop?.cues).toHaveLength(2);
    // Jade Beauty's own entrance, never this recipe's.
    expect(landing?.row).toBe("replace");
    expect(landing?.cues[0]).toMatchObject({ kind: "figure", figure: "jadeBoulder", at: tid(testid.zone("you", "units", 3)) });
    // The other player's counter is no drop of its.
    expect(theirs).toEqual(UNCLAIMED);
    // The other seat's shard drops by its own hero.
    const [other] = run(RED_JADE, [rose("p2", 5)], D, "p2", view);
    expect(other?.cues[0]).toMatchObject({ figure: "jadeShard", at: tid(testid.hero("opponent")), side: "opponent" });
  });

  it("R3405001 CN Flea Market: lanterns pop alight along the board and an awning unfurls over the caster's backrow in steam; each lot bought drops a card back from the stall to the hand, its sheen the caster's alone, and a barter carries a hand card to the stall", () => {
    const cast = castOf(MARKET, D, 0);
    const stall = tid(animTestid.backrow("you"));
    const unfurl = frac(0.15, D);
    expect(cast.row).toBe("replace");
    expect(beats(cast.cues)).toEqual(["figure:lanterns@0", `figure:awning@${String(unfurl)}`, `burst@${String(frac(0.45, D))}`]);
    expect(cast.cues).toMatchObject([
      { figure: "lanterns", at: tid(testid.board), durationMs: D + 400, tint: figureTint(lookOf("Hustle/cash")) },
      { figure: "awning", at: stall, side: "you", durationMs: D - unfurl + 400 },
      { preset: "smoke", spread: "area", at: stall, colors: lookOf("Hustle/cash").colors },
    ]);
    const view = fullBoardView();
    const hand = tid(animTestid.hand("you"));
    const traded = Array.isArray(view.you.hand) ? view.you.hand[0] : undefined;
    if (traded === undefined) throw new Error("no hand card");
    const exiled = (instanceId: string, defId: string, owner: "p1" | "p2"): GameEvent => ({ type: "exiled", instanceId, defId, owner });
    const [stallOpen, lot, lot2, barter, itself, theirExile] = run(
      MARKET,
      [{ type: "promptOpened", player: "p1" }, added("p1", "fx-lot-1", "core-055"), added("p1", "fx-lot-2", "meditative-039-1"), exiled(traded.instanceId, traded.defId, "p1"), exiled("ex", MARKET, "p1"), exiled(foeOf(view), "core-008", "p2")],
      D,
      "p1",
      view,
    );
    // The stall and its yuan are the caster's prompt alone.
    expect(stallOpen).toEqual(UNCLAIMED);
    expect(lot).toEqual({ cues: [{ kind: "ghost", from: stall, to: hand, delayMs: 0, durationMs: D }, { kind: "sheen", at: hand, delayMs: 0, durationMs: D }], row: "replace" });
    expect(lot2).toEqual(lot);
    expect(barter).toEqual({ cues: [{ kind: "ghost", from: hand, to: stall, delayMs: 0, durationMs: D }], row: "replace" });
    // Its own card and the other player's are no barter.
    expect([itself, theirExile]).toEqual([UNCLAIMED, UNCLAIMED]);
    // The other seat's market: its lots fly as bare card backs from its stall to its hand pile.
    const [theirs] = run(MARKET, [added("p2", "hidden", "hidden")], D, "p2", view);
    expect(theirs?.cues).toEqual([{ kind: "ghost", from: tid(animTestid.backrow("opponent")), to: tid(animTestid.hand("opponent")), delayMs: 0, durationMs: D }]);
    expect(castOf(MARKET, D, 0, "p2", view).cues[1]).toMatchObject({ at: tid(animTestid.backrow("opponent")), side: "opponent" });
  });

  it("R3405001 Call to Chaos (Meditative Edition): a jade die crashes onto the centre in gravel ripples; its roll rings jade as its panel names the clause within the cap, the outcome's rows play their own beats, and a Call it casts rolls a smaller die", () => {
    const cast = castOf(CHAOS, D, 0);
    const crash = frac(0.4, D);
    expect(cast.row).toBe("replace");
    expect(beats(cast.cues)).toEqual(["figure:jadeDie@0", `ring@${String(crash)}`, `burst@${String(crash)}`]);
    expect(cast.cues).toMatchObject([
      { figure: "jadeDie", at: CENTRE, side: "you", durationMs: D + 400, tint: figureTint(lookOf("Hustle/jade")) },
      { preset: "dust", at: CENTRE },
      { preset: "crumb", spread: "ring", at: CENTRE, colors: lookOf("Hustle/jade").colors },
    ]);
    expect(cast.cues[0]).not.toHaveProperty("size");
    expect(castOf(CHAOS, D, 0, "p2").cues[0]).toMatchObject({ side: "opponent" });
    const view = fullBoardView();
    const rolled: GameEvent = { type: "chaosRolled", player: "p1", instanceId: "ex", defId: CHAOS, effects: ["Your hero gains 5 Armor and you heal it 5"] };
    const golem: GameEvent = { type: "summoned", player: "p1", instanceId: "fx-golem", defId: CN_GOLEM, row: "units", lane: 2 };
    const [roll, landing, bounced] = run(CHAOS, [rolled, golem, { type: "bounced", instanceId: foeOf(view), defId: "core-008", owner: "p2" }], 900, "p1", view);
    expect(roll?.row).toBe("replace");
    expect(roll?.cues.map((cue) => cue.kind)).toEqual(["chaos", "ring"]);
    const panel = roll?.cues[0];
    if (panel?.kind !== "chaos") throw new Error("no panel");
    expect(panel.durationMs).toBe(FX_CHAOS_DICE_PANEL_MS);
    expect(panel.lines.map((line) => line.text)).toEqual(["Hero gains 5 Armor and heals 5"]);
    for (const line of panel.lines) expect(line.landMs).toBeLessThanOrEqual(FX_CHAOS_DICE_REEL_MS);
    expect(roll?.cues[1]).toMatchObject({ preset: "poison", at: CENTRE, delayMs: 0 });
    // The CN Golem it summons lands with its own entrance; a bounce is its own row's.
    expect(landing?.row).toBe("replace");
    expect(landing?.cues[0]).toMatchObject({ kind: "figure", figure: "golemBlocks", at: tid(testid.zone("you", "units", 2)) });
    expect(bounced).toEqual(UNCLAIMED);
    // A Call it casts: the inner play rolls a smaller die.
    const env = envOf();
    const inner = played("p1", "inner", CHAOS);
    env.memory.remember([played("p1", "outer", CHAOS), inner]);
    expect(planCardFx(inner, { entry: { ...galleryEntry([inner], view), durationMs: D }, view, env, D })?.cues[0]).toMatchObject({ figure: "jadeDie", size: "small" });
  });

  it("R3405001 Jlockeed's Evil Blueprints: after its hold the torn blueprint's pieces lock into one at the centre in hologram light; its offer is the caster's alone, and the building picked folds into a card back to the hand, its sheen the caster's alone", () => {
    const cast = castOf(BLUEPRINTS, MYTHIC_D);
    expect(cast.row).toBe("replace");
    expect(beats(cast.cues)).toEqual([`figure:blueprint@${String(HOLD)}`, `burst@${String(HOLD)}`]);
    expect(cast.cues).toMatchObject([
      { figure: "blueprint", at: CENTRE, durationMs: D + 400, tint: figureTint(lookOf("Tech/hologram")) },
      { spread: "area", at: CENTRE, colors: lookOf("Tech/hologram").colors },
    ]);
    const view = fullBoardView();
    const hand = tid(animTestid.hand("you"));
    const [offer, answer, building, theirCard] = run(
      BLUEPRINTS,
      [{ type: "promptOpened", player: "p1" }, { type: "promptAnswered", player: "p1" }, added("p1", "fx-building", "meditative-097-3"), added("p2", "hidden", "hidden")],
      D,
      "p1",
      view,
    );
    // The three buildings offered are the caster's prompt alone; the other player's card is none of its.
    expect([offer, answer, theirCard]).toEqual([UNCLAIMED, UNCLAIMED, UNCLAIMED]);
    const fold = frac(0.2, D);
    expect(building).toEqual({ cues: [{ kind: "ghost", from: CENTRE, to: hand, delayMs: fold, durationMs: D - fold }, { kind: "sheen", at: hand, delayMs: fold, durationMs: D - fold }], row: "replace" });
    // The other seat's pick: a bare card back to its hand pile, no sheen here.
    const theirs = run(BLUEPRINTS, [{ type: "promptOpened", player: "p2" }, added("p2", "hidden", "hidden")], D, "p2", view);
    expect(theirs).toEqual([UNCLAIMED, { cues: [{ kind: "ghost", from: CENTRE, to: tid(animTestid.hand("opponent")), delayMs: fold, durationMs: D - fold }], row: "replace" }]);
  });
});
