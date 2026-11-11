// R3405001: Meditative #79 Touched by KY (`meditative-079`, entrance "chalkHand", bespoke "touchedByKy", a
// chalk haze on his tile), a Legendary Unit's signature. A chalk handprint presses down onto his zone from
// above and the student materialises beneath it at the slam in a cloud of chalk, chalk dust raining from his
// hair. His Cry's first draw (`drawn`) has the chalk hand tap his brow and his hand pile stretch to its new
// size in ghostly chalk outlines, the drawn card's back flying in from his deck pile; each draw after it
// flies its own back in to fill it. The hand size his Cry sets is no event, so its outlines play at the
// first draw. A drawn card flies as a card back alone on either seat, its face never shown (R202). It keys
// on his play and what that play has seen (`seen`), never on a card's definition.

import type { GameEvent, PlayerId } from "@jackioh/shared";

import { HIDDEN_ID, animTestid } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import type { FxCue, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, type CardRecipe, type Entrance, type RecipeFixture } from "../recipe.ts";
import { standingAt } from "./knowledgeBreaker.ts";

/** The glow his chalk wears: Holy's, his aura's. */
const KY_LOOK = FLAVOR_LOOKS.Holy;

/** The dust his chalk sheds: Nerd's chalk sub-look. */
const CHALK_PALETTE = "Nerd/chalk";

/** How many of his Cry's draws the gallery plays, and the card its owner draws (the other seat's is hidden). */
const FIXTURE_DRAWS = 3;
const FIXTURE_CARD = "core-002";

/** His brow, where the chalk hand taps, and his hair, where its dust rains from. */
const BROW: FxPoint = { x: 0.5, y: 0.3 };
const HAIR: FxPoint = { x: 0.5, y: 0.12 };

/** Every particle count his signature throws, at intensity "normal" (rule 9). */
const TUNING = {
  press: { count: 18, power: 0.8 },
  rain: { count: 10, power: 0.4 },
  tap: { count: 6, power: 0.4 },
} as const;

/**
 * As fractions of the motion after the lead: the hand presses down from the lead and lasts the entry and
 * `tailMs` more, its dust raining from his hair at `rainAt`. At his first draw, as fractions of the entry:
 * the hand taps his brow at `tapAt` and is gone by `handEnd`; the outlines stretch from `stretchAt` until
 * `tailMs` past the entry; the drawn card's back flies in from `flyAt`.
 */
const TIMING = { rainAt: 0.8, tailMs: 300, tapAt: 0.15, handEnd: 0.4, stretchAt: 0.3, flyAt: 0.45 } as const;

export const chalkHand: Entrance = ({ D, tgt, intensity: i, view, landing, leadMs }) => {
  const chalk = paletteLook(CHALK_PALETTE) ?? FLAVOR_LOOKS.Nerd;
  const at = tid(tgt);
  const lead = leadMs ?? 0;
  const span = D - lead;
  const slam = lead + frac(FX_SLAM_AT, span);
  const side = view === undefined || landing === undefined ? {} : { side: sideOf(view, landing.player) };
  const cues: FxCue[] = [
    figureCue("chalkHand", at, figureTint(KY_LOOK), lead, D - lead + TIMING.tailMs, side),
    { ...tunedBurst(i, chalk.particle, at, "area", slam, TUNING.press), colors: chalk.colors },
    ringCue(D, KY_LOOK.particle, at, slam),
    ...shakeCues(i, ENTRANCE_TRAUMA.chalkHand, slam),
    { ...tunedBurst(i, chalk.particle, tid(tgt, HAIR), "point", lead + frac(TIMING.rainAt, span), TUNING.rain), colors: chalk.colors },
  ];
  return cues;
};

export const touchedByKy: CardRecipe = (event, p, play) => {
  // His Cry's draws, into his own hand.
  if (event.type !== "drawn" || event.player !== play.player || play.hook !== undefined) return null;
  const side = sideOf(p.view, play.player);
  const deck = tid(animTestid.library(side));
  const hand = tid(animTestid.hand(side));
  if ((play.seen.drawn ?? 1) > 1) return { cues: [{ kind: "ghost", from: deck, to: hand, delayMs: 0, durationMs: p.D }], row: "replace" };
  const chalk = paletteLook(CHALK_PALETTE) ?? FLAVOR_LOOKS.Nerd;
  const own = standingAt(p, play);
  const brow = own.kind === "testid" ? tid(own.testid, BROW) : own;
  const stretch = frac(TIMING.stretchAt, p.D);
  const fly = frac(TIMING.flyAt, p.D);
  const cues: FxCue[] = [
    figureCue("chalkHand", own, figureTint(KY_LOOK), 0, frac(TIMING.handEnd, p.D), { side, size: "small" }),
    { ...tunedBurst(p.env.intensity, chalk.particle, brow, "point", frac(TIMING.tapAt, p.D), TUNING.tap), colors: chalk.colors },
    figureCue("handOutline", hand, figureTint(chalk), stretch, p.D - stretch + TIMING.tailMs, { side }),
    { kind: "ghost", from: deck, to: hand, delayMs: fly, durationMs: p.D - fly },
  ];
  return { cues, row: "replace" };
};

/** R3405001: his Cry's first FIXTURE_DRAWS draws as `player` sees them: the other seat's hidden. */
export const touchedByKyFixture: RecipeFixture = (view, _instanceId, player: PlayerId): GameEvent[] =>
  Array.from({ length: FIXTURE_DRAWS }, (_, k): GameEvent =>
    player === view.viewer
      ? { type: "drawn", player, instanceId: `fx-ky-draw-${String(k + 1)}`, defId: FIXTURE_CARD }
      : { type: "drawn", player, instanceId: HIDDEN_ID, defId: HIDDEN_ID },
  );
