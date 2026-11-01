// R2652001: a card's coin flips and die rolls, one `rolled` batch per verb call or hook.
//
// The engine reports a batch as `rolled { player, instanceId, defId, kind, sides, rolls }`, each roll
// a coin or a die in draw order holding every face it drew (its own, then each extra roll Lucky and
// Luck made of it) and `kept`, the index of the face it kept. A coin is a two-sided die whose face
// `ROLL_HEADS` is heads. R2652000 decides who sees a batch (a face-down trap's reaches its controller
// alone); a batch in the view is the viewer's to read in full, its card named as R97 names it. This
// module is the client's whole reading of it:
// - `rollBatchOf` is the one adapter onto the event, so a rename is one line;
// - `rollWords` states the result in words, for the game log and the showcase's live region;
// - `rollCues` plans the panel the effects layer draws over the board: every coin spinning and every
//   die tumbling at once, each landing on its kept face inside the entry (R200), the faces Lucky drew
//   beside it, and a spark burst as each lands.
//
// Nothing here is a rule: the faces and the one kept are the event's, and the tumble a reel shows
// before it lands is presentation, a fixed walk that says nothing the roll does not (CLAUDE.md rule 7).

import { ROLL_HEADS, ROLL_TAILS } from "@jackioh/engine/config";
import type { GameEvent, PlayerId } from "@jackioh/shared";

import { listSeparator } from "../cards/chinese.ts";
import { t } from "../i18n/i18n.ts";
import { frac, tunedBurst } from "./build.ts";
import {
  FX_BANNER_TAIL_MS,
  FX_CENTER,
  FX_ROLL_LAND_AT,
  FX_ROLL_LAND_LAST,
  FX_ROLL_STAGGER,
  FX_ROLL_TUMBLE_FACES,
  FX_ROLL_TUMBLE_STRIDE,
  FX_TEXT,
} from "./constants.ts";
import type { FxAnchor, FxCue, FxRollCue, FxRollItem } from "./types.ts";

type RolledEvent = Extract<GameEvent, { type: "rolled" }>;

/** One coin or die: every face it drew, in draw order, and the index of the one it kept. */
export type Roll = { faces: readonly number[]; kept: number };

/** A batch as the client reads it. */
export type RollBatch = {
  player: PlayerId;
  instanceId: string;
  defId: string;
  kind: "coin" | "die";
  sides: number;
  rolls: readonly Roll[];
};

/** The one adapter onto `rolled`. */
export function rollBatchOf(event: GameEvent): RollBatch | null {
  if (event.type !== "rolled") return null;
  const rolled: RolledEvent = event;
  return {
    player: rolled.player,
    instanceId: rolled.instanceId,
    defId: rolled.defId,
    kind: rolled.kind,
    sides: rolled.sides,
    rolls: rolled.rolls.map((roll) => ({ faces: [...roll.faces], kept: roll.kept })),
  };
}

/** Whether a coin's face is heads (R2652001: face `ROLL_HEADS`). */
export function isHeads(face: number): boolean {
  return face === ROLL_HEADS;
}

/** The face a roll kept, or 0 for one that drew none. */
export function keptFace(roll: Roll): number {
  return roll.faces[roll.kept] ?? 0;
}

/** A face as its words: a coin's heads or tails, a die's number. */
export function faceWord(kind: RollBatch["kind"], face: number): string {
  if (kind === "coin") return isHeads(face) ? t("heads") : t("tails");
  return String(face);
}

/** The extra faces Lucky and Luck drew across the batch. */
export function extraDraws(batch: RollBatch): number {
  return batch.rolls.reduce((sum, roll) => sum + Math.max(0, roll.faces.length - 1), 0);
}

/** The batch in words, `who` naming the card that rolled: what landed, and how many extra rolls Lucky made. */
export function rollWords(batch: RollBatch, who: string): string {
  const count = batch.rolls.length;
  const kept = batch.rolls.map(keptFace);
  let sentence: string;
  if (batch.kind === "coin") {
    const heads = kept.filter(isHeads).length;
    sentence =
      count === 1
        ? t("{who} flipped a coin: {face}", { who, face: faceWord("coin", kept[0] ?? 0) })
        : t("{who} flipped {count} coins: {heads} heads, {tails} tails", { who, count, heads, tails: count - heads });
  } else {
    sentence =
      count === 1
        ? t("{who} rolled a {sides}-sided die: {face}", { who, sides: batch.sides, face: faceWord("die", kept[0] ?? 0) })
        : t("{who} rolled {count} {sides}-sided dice: {faces}", {
            who,
            count,
            sides: batch.sides,
            faces: kept.map((face) => faceWord("die", face)).join(listSeparator()),
          });
  }
  const extra = extraDraws(batch);
  return extra > 0 ? t("{sentence} (Lucky rolled {count} more)", { sentence, count: extra }) : sentence;
}

/** When item `i` of `n` lands, as a fraction of the entry: the stagger shrinks so the last lands by FX_ROLL_LAND_LAST. */
export function rollLandAt(i: number, n: number): number {
  const stagger = n > 1 ? Math.min(FX_ROLL_STAGGER, (FX_ROLL_LAND_LAST - FX_ROLL_LAND_AT) / (n - 1)) : 0;
  return FX_ROLL_LAND_AT + i * stagger;
}

/**
 * The faces item `i`'s reel shows: FX_ROLL_TUMBLE_FACES faces in a fixed walk, then `face`. A coin
 * alternates heads and tails, so it seems to spin; a die walks its other faces by a stride of
 * FX_ROLL_TUMBLE_STRIDE + i, so a batch of dice does not tumble in step. The walk is the same every time.
 */
export function rollTumble(kind: RollBatch["kind"], sides: number, face: number, i: number): number[] {
  const out: number[] = [];
  const n = Math.max(1, sides);
  for (let k = 0; k < FX_ROLL_TUMBLE_FACES; k += 1) {
    if (kind === "coin") {
      // The faces alternate, the one before the landing face always the other side.
      const otherSide = (FX_ROLL_TUMBLE_FACES - k) % 2 === 1;
      const shownHeads = otherSide ? !isHeads(face) : isHeads(face);
      out.push(shownHeads ? ROLL_HEADS : ROLL_TAILS);
      continue;
    }
    let shown = ((face - 1 + (k + 1) * (FX_ROLL_TUMBLE_STRIDE + i)) % n) + 1;
    if (shown === face && n > 1) shown = (shown % n) + 1;
    out.push(shown);
  }
  return [...out, face];
}

/** The sparks thrown as each coin or die lands (rule 9). */
const ROLL_TUNING = { landing: { count: 10, power: 1 } } as const;

const PANEL: FxAnchor = { kind: "viewport", at: { x: FX_CENTER.x, y: FX_CENTER.y } };

/**
 * R2652001, R200: the panel for one `rolled` entry of duration D. Every coin or die lands inside the
 * entry, and the panel is gone D + FX_BANNER_TAIL_MS after it starts (FX_BANNER_TAIL_MS ≤ T).
 */
export function rollCues(event: GameEvent, D: number, intensity: number): FxCue[] {
  const batch = rollBatchOf(event);
  if (batch === null || batch.rolls.length === 0) return [];
  const n = batch.rolls.length;
  const items: FxRollItem[] = batch.rolls.map((roll, i) => ({
    faces: roll.faces.map((face) => faceWord(batch.kind, face)),
    kept: roll.kept,
    tumble: rollTumble(batch.kind, batch.sides, keptFace(roll), i).map((face) => faceWord(batch.kind, face)),
    landMs: Math.min(D, frac(rollLandAt(i, n), D)),
  }));
  const panel: FxRollCue = {
    kind: "roll",
    title: batch.kind === "coin" ? FX_TEXT.coinsFlipped : FX_TEXT.diceRolled,
    die: batch.kind,
    items,
    delayMs: 0,
    durationMs: D + FX_BANNER_TAIL_MS,
  };
  return [panel, ...items.map((item) => tunedBurst(intensity, "gold", PANEL, "point", item.landMs, ROLL_TUNING.landing))];
}
