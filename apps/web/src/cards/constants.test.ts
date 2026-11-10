// The face numbers' contract (cards/constants.ts, CLAUDE.md rule 9): named, never inline, some
// mirrored in CSS. Each shipped value is pinned; a mutant moving any of them fails here by name.

import { describe, expect, it } from "vitest";

import {
  ART_DWELL_MS,
  ART_NEAR_MARGIN_PX,
  ART_SMALL_FACE_LIFT,
  ART_WEAR_LEVELS,
  CARD_DEPTH_FACE_PX,
  CARD_DEPTH_STEP_CQW,
  CARD_DEPTH_STEP_PX,
  CARD_FRAME_BREAK_PX,
  CARD_FRAME_BREAK_SCALE,
  CARD_GLARE_SIZE_PCT,
  CARD_GLARE_TRAVEL_PCT,
  CARD_PERSPECTIVE_PX,
  CARD_TILT_MAX_DEG,
  COSMETIC_FOIL_ANIMATED_MAX,
  DORMANT_DIM,
  DORMANT_GREY,
  DORMANT_PULSE_MS,
  EXTENDED_ART_BOX_PCT,
  EXTENDED_ART_FRAME_OPACITY,
  EXTENDED_ART_PICTURE_PCT,
  FACE_ASPECT,
  FACE_TEXT_LIMIT,
  FACE_TEXT_MIN_HEIGHT_PX,
  FIT_FLOOR_PX,
  FIT_MIN,
  FIT_STEPS,
  FULL_ART_FINISH_OPACITY,
  FULLSCREEN_HOLD_MS,
  FULLSCREEN_SLOP_PX,
  HAND_ART_GROW_CQH,
  HAND_TEXT_BOX_MIN_CQH,
  IMMERSIVE_BREATH_S,
  IMMERSIVE_BREATH_SCALE,
  IMMERSIVE_SHIFT_PCT,
  IMMERSIVE_TILT_RANGE_DEG,
  IMMERSIVE_TILT_REST_DEG,
  MINION_ART_FOOT_SHARE,
  NAME_TIER_MAX,
  OUT_OF_FRAME_SUBJECT_PCT,
  OUT_OF_FRAME_SUBJECT_SINK_PCT,
  OUT_OF_FRAME_WINDOW_SIDE_PCT,
  OUT_OF_FRAME_WINDOW_TOP_PCT,
  PREVIEW_TEXT_LIMIT,
  RADIANT_GOLD_HUE_MAX_DEG,
  RADIANT_GOLD_HUE_MIN_DEG,
  RADIANT_GOLD_LIGHTNESS_MAX,
  RADIANT_GOLD_LIGHTNESS_MIN,
  RADIANT_GOLD_SATURATION_MIN,
  REF_HOVER_DELAY_MS,
  REF_PANEL_FACE_HEIGHT_PX,
  REF_TOOLTIP_HEIGHT_PX,
  SHIPYARD_IDLE_SPARKS,
  SHIPYARD_IDLE_SWEEP_MS,
  SHIPYARD_IDLE_WELD_MS,
  STARSHIP_HULL_PLATES_MAX,
  STATE_BADGES_SMALL_MAX,
  TERM_HOVER_DELAY_MS,
  TEXT_TIER_MAX,
  TIER_SCALE,
} from "./constants.ts";

describe("cards constants", () => {
  it("pins the face shape, the text tiers and the fit schedule", () => {
    expect(FACE_ASPECT).toBe(5 / 7);
    expect(NAME_TIER_MAX).toEqual({ s: 12, m: 18, l: 24, xl: 30 });
    expect(TEXT_TIER_MAX).toEqual({ s: 40, m: 90, l: 160, xl: 260 });
    expect(TIER_SCALE).toEqual({ s: 1, m: 0.92, l: 0.82, xl: 0.72, xxl: 0.62 });
    expect(FIT_MIN).toBe(0.55);
    expect(FIT_STEPS).toBe(6);
    expect(FIT_FLOOR_PX).toBe(9);
  });

  it("pins the text limits #1017 measured and the hover delays", () => {
    expect(FACE_TEXT_LIMIT).toEqual({ charsPerLine: 28, lines: 6 });
    expect(PREVIEW_TEXT_LIMIT).toEqual({ charsPerLine: 45, lines: 10 });
    expect(FACE_TEXT_MIN_HEIGHT_PX).toBe(150);
    expect(REF_HOVER_DELAY_MS).toBe(250);
    expect(TERM_HOVER_DELAY_MS).toBe(250);
    expect(REF_TOOLTIP_HEIGHT_PX).toBe(300);
    expect(REF_PANEL_FACE_HEIGHT_PX).toBe(190);
  });

  it("pins the rail, lazy-art and foil caps", () => {
    expect(STATE_BADGES_SMALL_MAX).toBe(2);
    expect(ART_NEAR_MARGIN_PX).toBe(600);
    expect(ART_DWELL_MS).toBe(150);
    expect(COSMETIC_FOIL_ANIMATED_MAX).toBe(6);
  });

  it("pins the finish, Extended and Out-of-frame numbers", () => {
    expect(FULL_ART_FINISH_OPACITY).toBe(0.6);
    expect(EXTENDED_ART_FRAME_OPACITY).toBe(0.55);
    expect(EXTENDED_ART_BOX_PCT).toBe(90);
    expect(EXTENDED_ART_PICTURE_PCT).toBe(150);
    expect(OUT_OF_FRAME_WINDOW_TOP_PCT).toBe(30);
    expect(OUT_OF_FRAME_WINDOW_SIDE_PCT).toBe(18);
    expect(OUT_OF_FRAME_SUBJECT_PCT).toBe(115);
    expect(OUT_OF_FRAME_SUBJECT_SINK_PCT).toBe(10);
  });

  it("pins the Immersive breath and tilt", () => {
    expect(IMMERSIVE_SHIFT_PCT).toBe(6);
    expect(IMMERSIVE_BREATH_S).toBe(7);
    expect(IMMERSIVE_BREATH_SCALE).toBe(1.04);
    expect(IMMERSIVE_TILT_RANGE_DEG).toBe(30);
    expect(IMMERSIVE_TILT_REST_DEG).toBe(45);
  });

  it("pins the depth planes, tilt, perspective and glare R3344000 measured", () => {
    expect(CARD_TILT_MAX_DEG).toBe(12);
    expect(CARD_DEPTH_STEP_PX).toBe(3);
    expect(CARD_DEPTH_FACE_PX).toBe(170);
    expect(CARD_DEPTH_STEP_CQW).toBe((3 / 170) * 100);
    expect(CARD_FRAME_BREAK_PX).toBe(8);
    expect(CARD_FRAME_BREAK_SCALE).toBe(1.12);
    expect(CARD_PERSPECTIVE_PX).toBe(900);
    expect(CARD_GLARE_TRAVEL_PCT).toBe(60);
    expect(CARD_GLARE_SIZE_PCT).toBe(55);
  });

  it("pins the Radiant gold window cosmetics tests hold borders out of", () => {
    expect(RADIANT_GOLD_HUE_MIN_DEG).toBe(38);
    expect(RADIANT_GOLD_HUE_MAX_DEG).toBe(56);
    expect(RADIANT_GOLD_SATURATION_MIN).toBe(0.5);
    expect(RADIANT_GOLD_LIGHTNESS_MIN).toBe(0.3);
    expect(RADIANT_GOLD_LIGHTNESS_MAX).toBe(0.9);
  });

  it("pins the Dormant veil, the hull plates and the shipyard idle", () => {
    expect(DORMANT_GREY).toBe(0.92);
    expect(DORMANT_DIM).toBe(0.6);
    expect(DORMANT_PULSE_MS).toBe(3200);
    expect(STARSHIP_HULL_PLATES_MAX).toBe(8);
    expect(SHIPYARD_IDLE_SWEEP_MS).toBe(4200);
    expect(SHIPYARD_IDLE_WELD_MS).toBe(900);
    expect(SHIPYARD_IDLE_SPARKS).toBe(5);
  });

  it("pins the small-face lift, the minion art foot, the hand growth and wear/fullscreen", () => {
    expect(ART_SMALL_FACE_LIFT).toEqual({ saturate: 1.15, contrast: 1.06 });
    expect(MINION_ART_FOOT_SHARE).toBe(0.5);
    expect(HAND_ART_GROW_CQH).toBe(4);
    expect(HAND_TEXT_BOX_MIN_CQH).toBe(12);
    expect(ART_WEAR_LEVELS).toBe(4);
    expect(FULLSCREEN_HOLD_MS).toBe(450);
    expect(FULLSCREEN_SLOP_PX).toBe(10);
  });
});
