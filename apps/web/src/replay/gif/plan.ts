// The frame plan of a shared GIF (#513, R513000, decision D5): a pure function from a clip's steps
// (each step's view and its redacted events) to a list of frames, each a display list of rectangles,
// text, card faces and backs, and portraits. It reads nothing but those views (CLAUDE.md rule 7), so
// a card the step's view hides is a back, each hand is a count, and no account name can reach a
// frame. Its clock is the fixed frame rate in its options, not the wall clock, and neither the
// device's settings nor its locale change it: the text is plain English, never `t()`'s. The raster
// (`raster.ts`) draws a plan and the encoder (`encoder.ts`) packs the pixels.

import type { ReplayStep } from "@jackioh/engine";
import type { PlayerId, PlayerView, PortraitId, Row } from "@jackioh/shared";

import { HIDDEN_ID, newEventsSince, planEntries } from "../../game/animations.ts";
import { laneIndex, LANES, testid } from "../../game/contract.ts";
import type { Side } from "../../game/contract.ts";
import { REPLAY_GIF_ENTRY_HOLD_MS, REPLAY_GIF_MAX_SECONDS, REPLAY_GIF_MAX_STEPS, REPLAY_GIF_STEP_HOLD_MS } from "../config.ts";

export type GifItem =
  | { kind: "rect"; x: number; y: number; w: number; h: number; fill: string | null; stroke: string | null; line: number }
  | { kind: "text"; x: number; y: number; text: string; size: number; color: string; align: "left" | "center" | "right"; bold: boolean }
  | { kind: "card"; x: number; y: number; w: number; h: number; defId: string; radiant: boolean; cost: number; attack: number | null; health: number | null }
  | { kind: "back"; x: number; y: number; w: number; h: number }
  | { kind: "portrait"; x: number; y: number; w: number; h: number; portrait: PortraitId };

/** One image of the GIF: the step whose board it shows, how long it stays (centiseconds) and what it draws. */
export type GifFrame = { step: number; delayCs: number; items: readonly GifItem[] };

export type GifPlan = { width: number; height: number; fps: number; frames: readonly GifFrame[] };

export type GifPlanOptions = {
  width: number;
  height: number;
  fps: number;
  /** The corner mark's version line. */
  catalogVersion: string;
  /** Each seat's portrait (R641): the seats are drawn as these, never named. */
  portraits: Readonly<Record<PlayerId, PortraitId>>;
};

/** The small JackiOh mark in the top band's corner. */
export const GIF_MARK = "JackiOh";

export { faceKey } from "./face.ts";

// Shares of the picture, so one plan scales to either preset.
const SEAT_SHARE = 0.16;
const CARD_ASPECT = 0.72;
const SLOT_INSET = 0.06;
const SIDE_MARGIN = 0.02;
const SEAT_PAD = 0.1;
const LABEL_SHARE = 0.26;
const SMALL_SHARE = 0.2;
const BADGE_SHARE = 0.36;
const BADGE_TEXT_SHARE = 0.7;
const FIRST_LINE_SHARE = 0.35;
const SECOND_LINE_SHARE = 0.72;
const MANA_OFFSET_SHARE = 0.16;
const BANNER_WIDTH_SHARE = 0.5;
const BANNER_TEXT_SHARE = 0.6;
/** The field's rows from the top, in the order of the board's own `FIELD_ROWS` (Board.tsx). */
const FIELD_ROWS: readonly { side: Side; row: Row }[] = [
  { side: "opponent", row: "backrow" },
  { side: "opponent", row: "units" },
  { side: "you", row: "units" },
  { side: "you", row: "backrow" },
];
const FIELD_ROW_COUNT = FIELD_ROWS.length;

// The frame clock's units: a GIF's delays are centiseconds.
const CS_PER_SECOND = 100;
const MS_PER_CS = 10;
const MS_PER_SECOND = 1000;

const RING_PAD = 3;
const RING_BOLD = 4;
const RING_THIN = 2;
const SLOT_LINE = 1;

const BACKGROUND = "#141726";
const SEAT_FILL = "#1f2440";
const SLOT_FILL = "#232a45";
const SLOT_STROKE = "#3a4266";
const TEXT = "#f4efe1";
const GOLD = "#ffd24a";
const HEALTH = "#c0392b";
const ARMOR = "#7f8c8d";
const BANNER = "#000000";

type Rect = { x: number; y: number; w: number; h: number };

const px = Math.round;

function text(x: number, y: number, label: string, size: number, color: string, align: "left" | "center" | "right", bold: boolean): GifItem {
  return { kind: "text", x: px(x), y: px(y), text: label, size: px(size), color, align, bold };
}

function rect(r: Rect, fill: string | null, stroke: string | null, line: number): GifItem {
  return { kind: "rect", x: px(r.x), y: px(r.y), w: px(r.w), h: px(r.h), fill, stroke, line };
}

function handCount(hand: PlayerView["you"]["hand"]): number {
  return Array.isArray(hand) ? hand.length : hand.count;
}

function bannerText(view: PlayerView): string | null {
  if (view.result === null) return null;
  if (view.result.winner === "draw") return "Draw";
  return view.result.winner === view.viewer ? "You win" : "You lose";
}

/** One step's board as a display list, and where each element a ring may mark stands (by the board's own test ids). */
function boardItems(view: PlayerView, opts: GifPlanOptions): { items: GifItem[]; spots: Map<string, Rect> } {
  const { width: W, height: H } = opts;
  const items: GifItem[] = [rect({ x: 0, y: 0, w: W, h: H }, BACKGROUND, null, 0)];
  const spots = new Map<string, Rect>();
  const band = H * SEAT_SHARE;
  const margin = W * SIDE_MARGIN;
  const pad = band * SEAT_PAD;
  const s = band - 2 * pad;
  const badge = s * BADGE_SHARE;

  const seats: readonly { side: Side; y0: number }[] = [
    { side: "opponent", y0: 0 },
    { side: "you", y0: H - band },
  ];
  for (const { side, y0 } of seats) {
    const seat = view[side];
    items.push(rect({ x: 0, y: y0, w: W, h: band }, SEAT_FILL, null, 0));
    const portrait: Rect = { x: margin, y: y0 + pad, w: s, h: s };
    spots.set(testid.hero(side), portrait);
    items.push({ kind: "portrait", x: px(portrait.x), y: px(portrait.y), w: px(s), h: px(s), portrait: opts.portraits[seat.player] });
    const health: Rect = { x: portrait.x + s - badge, y: portrait.y + s - badge, w: badge, h: badge };
    items.push(rect(health, HEALTH, null, 0));
    items.push(text(health.x + badge / 2, health.y + badge / 2, String(seat.hero.health), badge * BADGE_TEXT_SHARE, TEXT, "center", true));
    if (seat.hero.armor > 0) {
      const armor: Rect = { x: portrait.x, y: health.y, w: badge, h: badge };
      items.push(rect(armor, ARMOR, null, 0));
      items.push(text(armor.x + badge / 2, armor.y + badge / 2, String(seat.hero.armor), badge * BADGE_TEXT_SHARE, TEXT, "center", true));
    }
    const left = margin + s + pad;
    items.push(text(left, y0 + band * FIRST_LINE_SHARE, side === "you" ? "You" : "Opponent", band * LABEL_SHARE, view.active === seat.player ? GOLD : TEXT, "left", true));
    items.push(text(left, y0 + band * SECOND_LINE_SHARE, `Hand ${String(handCount(seat.hand))}`, band * SMALL_SHARE, TEXT, "left", false));
    items.push(text(left + W * MANA_OFFSET_SHARE, y0 + band * SECOND_LINE_SHARE, `Mana ${String(seat.mana.current)}/${String(seat.mana.max)}`, band * SMALL_SHARE, TEXT, "left", false));
  }
  const right = W - margin;
  items.push(text(right, band * FIRST_LINE_SHARE, GIF_MARK, band * LABEL_SHARE, GOLD, "right", true));
  items.push(text(right, band * SECOND_LINE_SHARE, opts.catalogVersion, band * SMALL_SHARE, TEXT, "right", false));
  items.push(text(right, H - band + band * FIRST_LINE_SHARE, `Turn ${String(view.turn)}`, band * LABEL_SHARE, TEXT, "right", false));

  const rowH = (H - 2 * band) / FIELD_ROW_COUNT;
  const laneW = (W - 2 * margin) / LANES.length;
  const slotH = rowH * (1 - 2 * SLOT_INSET);
  const slotW = Math.min(laneW - 2 * rowH * SLOT_INSET, slotH * CARD_ASPECT);
  FIELD_ROWS.forEach(({ side, row }, rowIndex) => {
    const seat = view[side];
    for (const lane of LANES) {
      const slot: Rect = {
        x: margin + (lane - 1) * laneW + (laneW - slotW) / 2,
        y: band + rowIndex * rowH + rowH * SLOT_INSET,
        w: slotW,
        h: slotH,
      };
      spots.set(testid.zone(side, row, lane), slot);
      const held = row === "units" ? seat.units[laneIndex(lane)] : seat.backrow[laneIndex(lane)];
      const box = { x: px(slot.x), y: px(slot.y), w: px(slot.w), h: px(slot.h) };
      if (held === null || held === undefined) {
        items.push(rect(slot, SLOT_FILL, SLOT_STROKE, SLOT_LINE));
      } else if (("faceDown" in held && held.faceDown) || held.defId === HIDDEN_ID) {
        items.push({ kind: "back", ...box });
      } else {
        spots.set(testid.card(held.instanceId), slot);
        items.push({
          kind: "card",
          ...box,
          defId: held.defId,
          radiant: held.radiant,
          cost: held.cost,
          attack: held.attack ?? null,
          health: held.health ?? null,
        });
      }
    }
  });

  const banner = bannerText(view);
  if (banner !== null) {
    const w = W * BANNER_WIDTH_SHARE;
    const h = band;
    items.push(rect({ x: (W - w) / 2, y: (H - h) / 2, w, h }, BANNER, GOLD, RING_THIN));
    items.push(text(W / 2, H / 2, banner, h * BANNER_TEXT_SHARE, GOLD, "center", true));
  }
  return { items, spots };
}

/** The ring that marks `spot` on one tick of an entry's pulse. */
function ring(spot: Rect, tick: number): GifItem {
  const line = tick % 2 === 0 ? RING_BOLD : RING_THIN;
  return rect({ x: spot.x - RING_PAD, y: spot.y - RING_PAD, w: spot.w + 2 * RING_PAD, h: spot.h + 2 * RING_PAD }, null, GOLD, line);
}

/**
 * The frames of `clip`: before each step after the first, a few ticks of its animation entries (each
 * ringing what it marks on the board as it stood before the step), then the step's own board held.
 * The clip is cut at `REPLAY_GIF_MAX_STEPS` steps and the plan at `REPLAY_GIF_MAX_SECONDS`.
 */
export function planFrames(clip: readonly ReplayStep[], opts: GifPlanOptions): GifPlan {
  const steps = clip.slice(0, REPLAY_GIF_MAX_STEPS);
  const tickCs = Math.round(CS_PER_SECOND / opts.fps);
  const ticks = Math.max(1, Math.round((REPLAY_GIF_ENTRY_HOLD_MS * opts.fps) / MS_PER_SECOND));
  const holdCs = Math.round(REPLAY_GIF_STEP_HOLD_MS / MS_PER_CS);
  const limitCs = REPLAY_GIF_MAX_SECONDS * CS_PER_SECOND;
  const frames: GifFrame[] = [];
  let total = 0;
  /** Adds a frame unless it would run the plan past its time cap. */
  const push = (frame: GifFrame): boolean => {
    if (total + frame.delayCs > limitCs) return false;
    total += frame.delayCs;
    frames.push(frame);
    return true;
  };

  let prev: ReplayStep | undefined;
  for (const step of steps) {
    if (prev !== undefined) {
      const before = boardItems(prev.view, opts);
      const entries = planEntries(newEventsSince(prev.view.events, step.view.events), prev.view, false);
      for (const entry of entries) {
        const marked: Rect[] = [];
        for (const key of entry.frames.keys()) {
          const spot = before.spots.get(key);
          if (spot !== undefined) marked.push(spot);
        }
        if (marked.length === 0) continue;
        for (let tick = 0; tick < ticks; tick += 1) {
          const items = [...before.items, ...marked.map((spot) => ring(spot, tick))];
          if (!push({ step: prev.step, delayCs: tickCs, items })) return { width: opts.width, height: opts.height, fps: opts.fps, frames };
        }
      }
    }
    if (!push({ step: step.step, delayCs: holdCs, items: boardItems(step.view, opts).items })) break;
    prev = step;
  }
  return { width: opts.width, height: opts.height, fps: opts.fps, frames };
}
