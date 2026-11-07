// R2944002 (#2944): the Immersive version's depth. Its near layer (CardFace's `.cf-immersive`) shifts against its hazed
// backdrop with the pointer's place in the viewport or the device's tilt, each read as a shift from -1 to 1 each way
// (`pointerShift`, `tiltShift`) and written to the face as two custom properties that cards.css turns into the near
// layer's `translate`. A face listens only while its depth holds a motion slot (CardFace, `useArtMotion`), so at most
// COSMETIC_FOIL_ANIMATED_MAX faces do, and never under reduced motion, with Animated foil off or on a lazy face; a
// listener reads the event alone and no layout. Presentation only: nothing here reaches the engine, an action,
// `PlayerView` or the log (CLAUDE.md rule 7).

import { useEffect, type RefObject } from "react";

import { IMMERSIVE_TILT_RANGE_DEG, IMMERSIVE_TILT_REST_DEG } from "./constants.ts";

/** R2944002: the custom properties a moving Immersive face holds its near layer's shift in, -1 to 1. */
export const IMMERSIVE_X_PROPERTY = "--cf-immersive-x";
export const IMMERSIVE_Y_PROPERTY = "--cf-immersive-y";

/** The digits a shift is written to its property with. */
const SHIFT_DIGITS = 3;

/** R2944002: how far the near layer shifts, each way, from -1 (left, up) through 0 (its rest) to 1 (right, down). */
export type ImmersiveShift = { x: number; y: number };

/** `value` held to -1 to 1, and 0 for no number. R3344001: tilt.ts holds its poses so too. */
export function unitClamp(value: number): number {
  return Number.isFinite(value) ? Math.min(1, Math.max(-1, value)) : 0;
}

/**
 * R2944002: the shift for a pointer at (`clientX`, `clientY`) in a viewport `width` by `height`: its rest at the
 * middle, and each edge's far shift at that edge. A side of no length gives no shift along it.
 */
export function pointerShift(clientX: number, clientY: number, width: number, height: number): ImmersiveShift {
  return {
    x: width > 0 ? unitClamp((2 * clientX) / width - 1) : 0,
    y: height > 0 ? unitClamp((2 * clientY) / height - 1) : 0,
  };
}

/**
 * R2944002: the shift for a device tilted `gamma` degrees left or right and `beta` forward or back
 * (`DeviceOrientationEvent`): its rest flat across and IMMERSIVE_TILT_REST_DEG forward, and the far shift
 * IMMERSIVE_TILT_RANGE_DEG past that. Null when the device reports no tilt.
 */
export function tiltShift(gamma: number | null, beta: number | null): ImmersiveShift | null {
  if (gamma === null || beta === null) return null;
  return {
    x: unitClamp(gamma / IMMERSIVE_TILT_RANGE_DEG),
    y: unitClamp((beta - IMMERSIVE_TILT_REST_DEG) / IMMERSIVE_TILT_RANGE_DEG),
  };
}

/**
 * R2944002: while `active` (an Immersive face whose depth holds a motion slot), the face at `ref` follows the pointer
 * and the device's tilt through IMMERSIVE_X_PROPERTY and IMMERSIVE_Y_PROPERTY; once not, it listens to neither and
 * holds neither property, so its near layer rests.
 */
export function useImmersiveShift(ref: RefObject<HTMLElement | null>, active: boolean): void {
  useEffect(() => {
    const element = ref.current;
    if (!active || element === null) return undefined;
    const write = (shift: ImmersiveShift): void => {
      element.style.setProperty(IMMERSIVE_X_PROPERTY, shift.x.toFixed(SHIFT_DIGITS));
      element.style.setProperty(IMMERSIVE_Y_PROPERTY, shift.y.toFixed(SHIFT_DIGITS));
    };
    const onPointer = (event: MouseEvent): void => write(pointerShift(event.clientX, event.clientY, window.innerWidth, window.innerHeight));
    const onTilt = (event: DeviceOrientationEvent): void => {
      const shift = tiltShift(event.gamma, event.beta);
      if (shift !== null) write(shift);
    };
    window.addEventListener("pointermove", onPointer, { passive: true });
    window.addEventListener("deviceorientation", onTilt, { passive: true });
    return () => {
      window.removeEventListener("pointermove", onPointer);
      window.removeEventListener("deviceorientation", onTilt);
      element.style.removeProperty(IMMERSIVE_X_PROPERTY);
      element.style.removeProperty(IMMERSIVE_Y_PROPERTY);
    };
  }, [ref, active]);
}
