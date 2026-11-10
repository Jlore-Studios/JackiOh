// R3344001 (#3344): the tilt a face with depth (depth.ts) takes. The face turns toward the pointer up to CARD_TILT_MAX_DEG
// each way, and with the device's orientation once the player has allowed it (`deviceTilt`, settings.ts), or by a drag
// in inspect, and springs back when the pointer leaves (depth.css's transition is the spring). It is written to the face
// as custom properties that depth.css turns into the face's rotation, its planes' parallax and its glare:
// `--cf-tilt-x` and `--cf-tilt-y` (the pose, -1 to 1 each way), `--cf-tilt-rx` and `--cf-tilt-ry` (the angles) and
// `--cf-glare-x` and `--cf-glare-y` (where the glare lies, opposite the tilt), with `data-tilt="on"` while it holds.
//
// Where it listens is the face's mode (`TiltMode`): "host" the face's parent, for the card the pointer is on or a touch
// presses (in hand, on the board, in the Collection); "window" the window's pointer, for the hover preview, which takes
// no pointer of its own: its pose is read against the box of the card it is anchored to (`CardFace`'s `tiltAnchor`),
// where the pointer is; "drag" the window's pointer and a touch drag on the face, in inspect, where it holds off
// the sheet's swipe on the face. The pose reads the host's box (the anchor's, in the preview), never the face's own,
// which rotates.
//
// One face tilts at a time (`claim`): a face that starts to tilt takes the slot, and one that asks for it with a lower
// priority than its holder's does not get it. None tilts while the animation queue drains (`useTiltHold`, Game.tsx),
// and none under reduced motion, which draws the face with no depth at all (CardFace). A listener reads the event and
// the host's box and writes six properties; it reads no view (CLAUDE.md rule 7). Presentation only: nothing here
// reaches the engine, an action, `PlayerView` or the log.

import { useEffect, useLayoutEffect, useRef, useSyncExternalStore, type RefObject } from "react";

import { CARD_GLARE_TRAVEL_PCT, CARD_TILT_MAX_DEG } from "./constants.ts";
import { tiltShift, unitClamp, type ImmersiveShift } from "./immersive.ts";
import { useCardSettings } from "./settings.ts";

/** R3344001: where a face listens for what tilts it: not at all, its parent, the window, or the window and a touch drag on it. */
export type TiltMode = "off" | "host" | "window" | "drag";

/** R3344001: where the pointer or the device points, -1 (left, up) through 0 (level) to 1 (right, down), each way. */
export type TiltPose = ImmersiveShift;

/** R3344001: the custom properties a tilting face holds, which depth.css reads. */
export const TILT_X_PROPERTY = "--cf-tilt-x";
export const TILT_Y_PROPERTY = "--cf-tilt-y";
export const TILT_RX_PROPERTY = "--cf-tilt-rx";
export const TILT_RY_PROPERTY = "--cf-tilt-ry";
export const GLARE_X_PROPERTY = "--cf-glare-x";
export const GLARE_Y_PROPERTY = "--cf-glare-y";

const TILT_PROPERTIES: readonly string[] = [
  TILT_X_PROPERTY,
  TILT_Y_PROPERTY,
  TILT_RX_PROPERTY,
  TILT_RY_PROPERTY,
  GLARE_X_PROPERTY,
  GLARE_Y_PROPERTY,
];

/** The digits a pose, an angle and a glare's place are written with. */
const POSE_DIGITS = 3;
const ANGLE_DIGITS = 2;
const GLARE_DIGITS = 1;

/** The glare lies at the middle of its layer when the face is level, in percent. */
const GLARE_REST_PCT = 50;

/** R3344001: the part of a box a pose reads: a face's is its host's, the hover preview's is its anchor's. */
export type TiltBox = { left: number; top: number; width: number; height: number };

/**
 * R3344001: the pose of a pointer at (`clientX`, `clientY`) over `box`: level at the box's middle, and each edge's far
 * pose at that edge, held there past it. A side of no length gives none along it.
 */
export function pointerPose(clientX: number, clientY: number, box: TiltBox): TiltPose {
  return {
    x: box.width > 0 ? unitClamp(((clientX - box.left) / box.width) * 2 - 1) : 0,
    y: box.height > 0 ? unitClamp(((clientY - box.top) / box.height) * 2 - 1) : 0,
  };
}

/**
 * R3344001: the pose of a device tilted `gamma` degrees left or right and `beta` forward or back
 * (`DeviceOrientationEvent`), as Immersive reads it (immersive.ts `tiltShift`); null when the device reports none.
 */
export function devicePose(gamma: number | null, beta: number | null): TiltPose | null {
  return tiltShift(gamma, beta);
}

/**
 * R3344001: the angles a pose turns the face by, in degrees: toward the pointer (the edge it points at turns away, as a
 * card pressed at that edge), never past CARD_TILT_MAX_DEG either way. R3350000: or past `maxDeg`, a diorama's turn
 * (DIORAMA_TURN_MAX_DEG) where the player turns it to look at it.
 */
export function tiltDegrees(pose: TiltPose, maxDeg: number = CARD_TILT_MAX_DEG): { rx: number; ry: number } {
  return { rx: -pose.y * maxDeg, ry: pose.x * maxDeg };
}

/** R3344001: where the glare lies for a pose, in percent of its layer: opposite the tilt, CARD_GLARE_TRAVEL_PCT at the far edge. */
export function glareOf(pose: TiltPose): { x: number; y: number } {
  return { x: GLARE_REST_PCT - pose.x * CARD_GLARE_TRAVEL_PCT, y: GLARE_REST_PCT - pose.y * CARD_GLARE_TRAVEL_PCT };
}

/** A number as a property's text: rounded to `digits`, and no "-0". */
function plain(value: number, digits: number): string {
  return String(Number(value.toFixed(digits)));
}

/** The face holding the one slot a tilt needs, and what yields it. */
type Holder = { readonly priority: number; readonly release: () => void };

/** R3344001: how strongly a mode holds the slot: a drag over the window's pointer over the face's parent's. */
const PRIORITY: Readonly<Record<Exclude<TiltMode, "off">, number>> = { host: 1, window: 2, drag: 3 };

let holder: Holder | null = null;

/**
 * R3344001: whether `next` may tilt: the slot is free or already its, or it asks with at least its holder's priority, in
 * which case the holder lets go (its properties are removed, so it springs back) and `next` holds it.
 */
function claim(next: Holder): boolean {
  if (holder === next) return true;
  if (holder !== null) {
    if (next.priority < holder.priority) return false;
    holder.release();
  }
  holder = next;
  return true;
}

function drop(from: Holder): void {
  if (holder === from) holder = null;
}

/** How many `useTiltHold`s are holding now, and who hears of a change. */
let holds = 0;
const holdListeners = new Set<() => void>();

function subscribeHold(listener: () => void): () => void {
  holdListeners.add(listener);
  return () => {
    holdListeners.delete(listener);
  };
}

export function isHeld(): boolean {
  return holds > 0;
}

function notHeld(): boolean {
  return false;
}

/**
 * R3345000: `isHeld`, subscribed: a component reading it renders again whenever the queue starts or stops draining.
 * #3633: one that says the hold is not `relevant` to it reads `false` and renders again for none, so a queue
 * starting to drain does not draw every card on the board again.
 */
export function useIsHeld(relevant = true): boolean {
  return useSyncExternalStore(subscribeHold, relevant ? isHeld : notHeld, notHeld);
}

function notifyHold(): void {
  for (const listener of [...holdListeners]) listener();
}

/**
 * R3344001: while `held` (the board's animation queue is draining an entry, Game.tsx), no face tilts: a face tilting
 * lets go at once and takes none until it is released.
 */
export function useTiltHold(held: boolean): void {
  useEffect(() => {
    if (!held) return undefined;
    holds += 1;
    notifyHold();
    return () => {
      holds -= 1;
      notifyHold();
    };
  }, [held]);
}

/** Test seam: drops the slot's holder and every hold. */
export function resetTiltForTests(): void {
  holder = null;
  holds = 0;
  holdListeners.clear();
}

/** Whether a pointer event is a finger's: it tilts only while it presses. */
function touches(event: PointerEvent): boolean {
  return event.pointerType === "touch";
}

/**
 * Listens for what tilts `face` in `mode` on `host`, and writes its pose to `face`. Returns what stops it, which also
 * puts the face level. `deviceTilt` adds the device's orientation, from the start in the window's and the drag's modes
 * and while a pointer hovers or presses in the host's. In the window's mode the pose is read against `anchor`'s box
 * when it has one (the hover preview's anchor), else the host's. R3350000: the face turns up to `maxDeg` each way.
 */
function attach(
  face: HTMLElement,
  host: HTMLElement,
  mode: Exclude<TiltMode, "off">,
  deviceTilt: boolean,
  anchor: { readonly current: TiltBox | undefined },
  maxDeg: number,
): () => void {
  let pressed = false;
  let device = false;

  const slot: Holder = { priority: PRIORITY[mode], release: level };

  // Level again: the device keeps its listener in the window's and the drag's modes, which hold it from their start.
  function level(): void {
    pressed = false;
    if (mode === "host") stopDevice();
    for (const property of TILT_PROPERTIES) face.style.removeProperty(property);
    delete face.dataset.tilt;
    drop(slot);
  }

  const write = (pose: TiltPose): void => {
    if (!claim(slot)) return;
    const { rx, ry } = tiltDegrees(pose, maxDeg);
    const glare = glareOf(pose);
    face.style.setProperty(TILT_X_PROPERTY, plain(pose.x, POSE_DIGITS));
    face.style.setProperty(TILT_Y_PROPERTY, plain(pose.y, POSE_DIGITS));
    face.style.setProperty(TILT_RX_PROPERTY, `${plain(rx, ANGLE_DIGITS)}deg`);
    face.style.setProperty(TILT_RY_PROPERTY, `${plain(ry, ANGLE_DIGITS)}deg`);
    face.style.setProperty(GLARE_X_PROPERTY, `${plain(glare.x, GLARE_DIGITS)}%`);
    face.style.setProperty(GLARE_Y_PROPERTY, `${plain(glare.y, GLARE_DIGITS)}%`);
    face.dataset.tilt = "on";
  };

  // The pointer is on the card the preview is anchored to, never over the preview, which is placed clear of it
  // (placePreview): so the window mode reads the anchor's box when it has one, and every other mode its host's.
  const boxOf = (): TiltBox => (mode === "window" ? anchor.current : undefined) ?? host.getBoundingClientRect();
  const poseAt = (event: MouseEvent): TiltPose => pointerPose(event.clientX, event.clientY, boxOf());

  const onDevice = (event: DeviceOrientationEvent): void => {
    const pose = devicePose(event.gamma, event.beta);
    if (pose !== null) write(pose);
  };

  function startDevice(): void {
    if (!deviceTilt || device) return;
    device = true;
    window.addEventListener("deviceorientation", onDevice, { passive: true });
  }

  function stopDevice(): void {
    if (!device) return;
    device = false;
    window.removeEventListener("deviceorientation", onDevice);
  }

  // A press, or a pointer over the host (not a finger's, which only presses).
  const onHostMove = (event: PointerEvent): void => {
    if (touches(event) && !pressed) return;
    startDevice();
    write(poseAt(event));
  };
  const onHostDown = (event: PointerEvent): void => {
    pressed = true;
    startDevice();
    write(poseAt(event));
  };
  const onHostUp = (event: PointerEvent): void => {
    pressed = false;
    if (touches(event)) level();
  };
  const onWindowMove = (event: PointerEvent): void => {
    if (!touches(event)) write(poseAt(event));
  };
  const onDragMove = (event: PointerEvent): void => {
    if (touches(event) && pressed) write(poseAt(event));
  };
  // A drag on the face: the sheet's own swipe never starts on it (useSheetSwipe), nor does the browser scroll it.
  const onDragDown = (event: PointerEvent): void => {
    if (!touches(event)) return;
    event.stopPropagation();
    onHostDown(event);
  };

  const removers: (() => void)[] = [];
  const on = <E extends Event>(target: EventTarget, type: string, handler: (event: E) => void, options?: AddEventListenerOptions): void => {
    target.addEventListener(type, handler as EventListener, options);
    removers.push(() => {
      target.removeEventListener(type, handler as EventListener);
    });
  };

  if (mode === "host") {
    on<PointerEvent>(host, "pointerenter", onHostMove);
    on<PointerEvent>(host, "pointermove", onHostMove);
    on<PointerEvent>(host, "pointerdown", onHostDown);
    on<PointerEvent>(host, "pointerup", onHostUp);
    on<PointerEvent>(host, "pointercancel", onHostUp);
    on(host, "pointerleave", level);
  } else {
    on<PointerEvent>(window, "pointermove", onWindowMove, { passive: true });
    on(document.documentElement, "pointerleave", level);
    on(window, "blur", level);
    startDevice();
    if (mode === "drag") {
      const touchAction = host.style.touchAction;
      host.style.touchAction = "none";
      removers.push(() => {
        host.style.touchAction = touchAction;
      });
      on<PointerEvent>(host, "pointerdown", onDragDown);
      on<PointerEvent>(host, "pointermove", onDragMove);
      on<PointerEvent>(host, "pointerup", onHostUp);
      on<PointerEvent>(host, "pointercancel", onHostUp);
    }
  }

  return () => {
    for (const remove of removers) remove();
    stopDevice();
    level();
  };
}

/**
 * R3344001: while `mode` is not "off", `allowed` and no hold stands, the face at `faceRef` tilts as `mode` says (`attach`),
 * and once not it listens to nothing and holds no property, so it rests level. `anchor` is for the window's mode (the
 * hover preview's anchor): without one a window face reads its host's box. R3350000: it turns up to `maxDeg` each way
 * (CARD_TILT_MAX_DEG; a diorama's DIORAMA_TURN_MAX_DEG where the player turns it to look at it).
 */
export function useCardTilt(
  faceRef: RefObject<HTMLElement | null>,
  mode: TiltMode,
  allowed: boolean,
  anchor?: TiltBox,
  maxDeg: number = CARD_TILT_MAX_DEG,
): void {
  // #3633: a face that does not tilt has no use for the hold.
  const held = useIsHeld(mode !== "off" && allowed);
  const { deviceTilt } = useCardSettings();
  const anchorRef = useRef(anchor);
  useLayoutEffect(() => {
    anchorRef.current = anchor;
  });
  const active = mode !== "off" && allowed && !held;
  useEffect(() => {
    const face = faceRef.current;
    const host = face?.parentElement ?? null;
    if (!active || face === null || host === null) return undefined;
    return attach(face, host, mode, deviceTilt, anchorRef, maxDeg);
  }, [faceRef, active, mode, deviceTilt, maxDeg]);
}
