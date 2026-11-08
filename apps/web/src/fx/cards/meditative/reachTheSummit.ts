// R3405001: Meditative #8 Reach the Summit (`meditative-008`, bespoke "summitAscent", takeover "ascent"), a
// Mythic Spell's signature in Nerd's ink: its takeover is the ink-wash mountain, the climber hauling onto
// her ledge (takeoverArt.ts), and as the hold ends her ledge flares under the caster's hero in a wash of
// mist. Each play then gains the one Ascent its level names (R841), drawn in ink: spring mist over what it
// heals and rock falling onto what it damages (both keep their own rows, so the number still shows), a card
// back climbing from the Deck to the hand for its draw, a torn card's ink at the hand the opponent discards
// from, smoke where an enemy permanent fades into exile, and a card back from the hero for each Radiant card
// it adds, its glint the caster's alone; the tenth's win (`gameOver`, won by effect) lights a star over the
// summit. Levels 6 to 9 gain nothing, so nothing plays past the ledge. It keys on its own play and the
// events that play emits, never on a card's definition, and draws card backs alone (R202).

import type { GameEvent } from "@jackioh/shared";

import { animTestid, targetFor } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, tid } from "../../build.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { motifBurst, motifScale, type CueSpec } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import type { CardRecipe, RecipeFixture } from "../recipe.ts";

/** The sub-look it climbs in: Nerd's ink, the wash of the takeover's range. */
const SUMMIT_PALETTE = "Nerd/ink";

/** Every particle count it throws, at scale 1 (rule 9): the mist, the rock, a torn card's ink, a fading permanent's smoke, the summit's stars. */
const TUNING = {
  mist: { count: 26, power: 0.7 },
  rock: { count: 14, power: 1.3 },
  ink: { count: 14, power: 1 },
  smoke: { count: 12, power: 0.6 },
  stars: { count: 30, power: 1.2 },
} as const;

/** The ledge flares `tailMs` past its entry; a drawn card's back leaves at `ghostAt` of its entry; the star burns `starMs` (the game's end holds no entry). */
const TIMING = { tailMs: 400, ghostAt: 0.2, starMs: 400 } as const;

/** Where the falling rock breaks on what it hits: that box's top edge. */
const ROCK_AT = { x: 0.5, y: 0 } as const;

/** The tenth Ascent's win: the game ended by its `winGame` (R850). */
const SUMMIT_WIN: Extract<GameEvent, { type: "gameOver" }>["reason"] = "won-by-effect";

/** The gallery's Ascent: level 2's draw (the heal's and the hit's own rows keep their numbers past a Mythic Spell's ceiling). */
const FIXTURE_DRAWN = { instanceId: "fx-ascent-draw", defId: "core-055" } as const;

export const summitAscent: CardRecipe = (event, p, play) => {
  const look = paletteLook(SUMMIT_PALETTE) ?? FLAVOR_LOOKS.Nerd;
  const spec: CueSpec = { look, scale: motifScale(play.defId, p.view, p.env.intensity), row: "replace" };
  const side = sideOf(p.view, play.player);
  const hero = tid(testid.hero(side));
  const hand = tid(animTestid.hand(side));
  switch (event.type) {
    case "cardPlayed": {
      if (event.instanceId !== play.instanceId) return null;
      const hold = p.entry.setPiece?.holdMs ?? 0;
      return {
        cues: [figureCue("ledge", hero, figureTint(look), hold, p.D - hold + TIMING.tailMs, { side }), motifBurst(spec, hero, "area", hold, TUNING.mist, "smoke")],
        row: "replace",
      };
    }
    case "healed":
    case "damage": {
      // Ascent 0's mist and Ascent 1's rock decorate their rows, which keep the number.
      const tgt = targetFor(event, p.view);
      if (tgt === null) return null;
      const cue = event.type === "healed" ? motifBurst(spec, tid(tgt), "area", 0, TUNING.mist, "smoke") : motifBurst(spec, tid(tgt, ROCK_AT), "area", 0, TUNING.rock, "crumb");
      return { cues: [cue], row: "keep" };
    }
    case "drawn": {
      if (event.player !== play.player) return null;
      const leave = frac(TIMING.ghostAt, p.D);
      return { cues: [{ kind: "ghost", from: tid(animTestid.library(side)), to: hand, delayMs: leave, durationMs: p.D - leave }], row: "replace" };
    }
    case "discarded": {
      // Ascent 3: the opponent's card torn out of its hand; its own row carries it to the graveyard.
      if (event.owner === play.player) return null;
      return { cues: [motifBurst(spec, tid(animTestid.hand(sideOf(p.view, event.owner))), "area", 0, TUNING.ink)], row: "keep" };
    }
    case "exiled": {
      // Ascent 4: an enemy permanent fading; its own row carries it to the exile.
      if (event.owner === play.player) return null;
      const tgt = targetFor(event, p.view);
      return tgt === null ? null : { cues: [motifBurst(spec, tid(tgt), "area", 0, TUNING.smoke, "smoke")], row: "keep" };
    }
    case "addedToHand": {
      // Ascent 5: Radiant cards at (0), each a back from the summit; their glint is the caster's alone.
      if (event.player !== play.player) return null;
      const cues: FxCue[] = [{ kind: "ghost", from: hero, to: hand, delayMs: 0, durationMs: p.D }];
      if (p.view.viewer === play.player) cues.push({ kind: "sheen", at: hand, delayMs: 0, durationMs: p.D });
      return { cues, row: "replace" };
    }
    case "gameOver": {
      if (event.reason !== SUMMIT_WIN || event.winner !== play.player) return null;
      return { cues: [figureCue("peakStar", hero, figureTint(look), 0, TIMING.starMs, { side }), motifBurst(spec, hero, "ring", 0, TUNING.stars, "star")], row: "replace" };
    }
    default:
      return null;
  }
};

/** The gallery's Reach the Summit: its play draws, a back climbing to the hand. */
export const summitAscentFixture: RecipeFixture = (_view, _instanceId, player) => [{ type: "drawn", player, ...FIXTURE_DRAWN }];
