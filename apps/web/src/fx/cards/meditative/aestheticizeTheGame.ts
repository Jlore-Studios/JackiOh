// R3405001: Meditative #20 Aestheticize the Game (`meditative-020`, bespoke "brushSeal", takeover "brush"), a
// Mythic Spell's signature: its takeover lays brushstrokes on paper (takeoverArt.ts), and as the hold ends
// one stroke sweeps across the board's centre in vermilion, in a splash of ink, while its prompt picks the
// way to win. The pick installs its condition on the caster (R848, a `modifierChanged` added), and the stroke
// dries onto the caster's hero as a vermilion brush seal; should the condition already hold as it resolves, the win
// (`gameOver`, an alternative win) blooms the seal again. Dropped: the other two strokes greying (the pick
// is a prompt, and the takeover is over before it) and the seal filling as the condition nears (no event of
// its play carries that; the modifier's badge shows the progress). Its Radiant draw keeps its own row. It
// keys on its own play, never on a card's definition (R202).

import type { GameEvent } from "@jackioh/shared";

import { sideOf, testid } from "../../../game/contract.ts";
import { tid } from "../../build.ts";
import { FX_CENTER } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { motifBurst, motifScale } from "../../motifs/build.ts";
import type { CardRecipe, RecipeFixture } from "../recipe.ts";

/** The sub-look its stroke splashes: Nerd's ink, a brush on paper. */
const STROKE_PALETTE = "Nerd/ink";

/** The sub-look its stroke and seal are laid in: a vermilion red. */
const SEAL_PALETTE = "Dark/blood";

/** Every particle count it throws, at scale 1 (rule 9): the stroke's ink and the seal's bloom at the win. */
const TUNING = {
  ink: { count: 22, power: 1 },
  bloom: { count: 28, power: 1.1 },
} as const;

/** The stroke sweeps `tailMs` past its entry, the seal dries `sealTailMs` past its own, and blooms `bloomMs` (the game's end holds no entry). */
const TIMING = { tailMs: 400, sealTailMs: 300, bloomMs: 400 } as const;

/** Its win: the chosen condition met (R848). */
const SEAL_WIN: Extract<GameEvent, { type: "gameOver" }>["reason"] = "alt-win";

/** The gallery's condition: the modifier its pick installs. */
const FIXTURE_MODIFIER = "fx-alt-win";

export const brushSeal: CardRecipe = (event, p, play) => {
  const side = sideOf(p.view, play.player);
  const hero = tid(testid.hero(side));
  const scale = motifScale(play.defId, p.view, p.env.intensity);
  const seal = paletteLook(SEAL_PALETTE) ?? FLAVOR_LOOKS.Dark;
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const ink = paletteLook(STROKE_PALETTE) ?? FLAVOR_LOOKS.Nerd;
    const centre = tid(testid.board, FX_CENTER);
    const hold = p.entry.setPiece?.holdMs ?? 0;
    // The stroke is laid in the seal's vermilion, the colour it keeps as it dries onto the hero.
    return {
      cues: [figureCue("inkStroke", centre, figureTint(seal), hold, p.D - hold + TIMING.tailMs, { side }), motifBurst({ look: ink, scale, row: "replace" }, centre, "area", hold, TUNING.ink)],
      row: "replace",
    };
  }
  if (event.type === "modifierChanged" && event.added && event.player === play.player) {
    return { cues: [figureCue("brushSeal", hero, figureTint(seal), 0, p.D + TIMING.sealTailMs)], row: "replace" };
  }
  if (event.type === "gameOver" && event.reason === SEAL_WIN && event.winner === play.player) {
    return {
      cues: [figureCue("brushSeal", hero, figureTint(seal), 0, TIMING.bloomMs), motifBurst({ look: seal, scale, row: "replace" }, hero, "ring", 0, TUNING.bloom, "sparkle")],
      row: "replace",
    };
  }
  return null;
};

/** The gallery's Aestheticize the Game: its pick's condition installed on the caster. */
export const brushSealFixture: RecipeFixture = (_view, _instanceId, player) => [{ type: "modifierChanged", player, modifierId: FIXTURE_MODIFIER, added: true }];
