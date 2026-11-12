// R3345000: full-screen art view with depth planes, dolly camera, tilt, and intro audio.
// R3350000: a Mythic with a diorama (art/dioramas/) draws its slices in place of the planes, each shifted with the tilt by its
// depth (`--fs-slice-shift`, FULLSCREEN_SLICE_SHIFT_FAR_PX far to FULLSCREEN_SLICE_SHIFT_NEAR_PX near), bent as it says and
// moving by its loop unless the viewer's motion settings ask for the flat picture, and the view turns up to
// DIORAMA_TURN_MAX_DEG. A tap that pauses the view holds the loops still too. So that no turn shows the black past its
// pictures, the view turns a diorama under a perspective of FULLSCREEN_DIORAMA_PERSPECTIVE_VMAX of the screen's longer side
// and scales it up by as much as its pose needs to keep the screen covered (`fullscreenCoverScale`, `--fs-cover`).
// Presentation only (CLAUDE.md rule 7).

import { useEffect, useId, useRef, useState, type CSSProperties, type ReactElement } from "react";
import { createPortal } from "react-dom";

import { getMusicPlayer } from "../audio/music.ts";
import { MUSIC_CARDS } from "../audio/musicData.ts";
import { DialogClose } from "../game/Dialog.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import { useModalKeys } from "../together/modalKeys.ts";
import { dioramaOf, dioramaSliceBackground } from "./art/dioramas/index.ts";
import { ART_PLANES, artPlaneBackground } from "./art/pictures.ts";
import {
  CARD_TILT_MAX_DEG,
  DIORAMA_TURN_MAX_DEG,
  FULLSCREEN_BLEED_PCT,
  FULLSCREEN_COVER_DIGITS,
  FULLSCREEN_COVER_SCALE_MAX,
  FULLSCREEN_COVER_STEPS,
  FULLSCREEN_DIORAMA_PERSPECTIVE_VMAX,
  FULLSCREEN_SLICE_SHIFT_FAR_PX,
  FULLSCREEN_SLICE_SHIFT_NEAR_PX,
} from "./constants.ts";
import { useDepthMotion } from "./depth.ts";
import { closeFullScreenArt, openFullScreenArt, useCanOpenFullScreen, useFullScreenArt, type FullScreenState } from "./fullscreen.ts";
import { useCardSettings } from "./settings.ts";
import { devicePose, glareOf, pointerPose, tiltDegrees, type TiltPose } from "./tilt.ts";

import "./fullscreen.css";

/** R3350000: the digits a slice's shift is written with, in px. */
const SLICE_SHIFT_DIGITS = 2;

/** R3350000: how far a slice at `depth` (0 the farthest, 1 the nearest) shifts at full tilt, in px. */
export function sliceShiftPx(depth: number): number {
  return Number((FULLSCREEN_SLICE_SHIFT_FAR_PX + depth * (FULLSCREEN_SLICE_SHIFT_NEAR_PX - FULLSCREEN_SLICE_SHIFT_FAR_PX)).toFixed(SLICE_SHIFT_DIGITS));
}

/** Degrees to radians. */
const RAD_PER_DEG = Math.PI / 180;

/**
 * R3350000: whether the planes, the viewport's size plus FULLSCREEN_BLEED_PCT on each side less the farthest a slice shifts,
 * scaled by `scale` and turned `rxDeg` about x and then `ryDeg` about y (fullscreen.css's `transform`, read right to left)
 * under `perspectivePx` about the screen's centre, still hold every corner of the screen, every corner of them in front of
 * the eye. The pictures are symmetric about the centre, so either sign of a turn is the same.
 */
export function fullscreenCovers(width: number, height: number, rxDeg: number, ryDeg: number, perspectivePx: number, scale: number): boolean {
  const shift = Math.max(Math.abs(FULLSCREEN_SLICE_SHIFT_FAR_PX), Math.abs(FULLSCREEN_SLICE_SHIFT_NEAR_PX));
  const bleed = 0.5 + FULLSCREEN_BLEED_PCT / 100;
  const a = scale * (width * bleed - shift);
  const b = scale * (height * bleed - shift);
  const rx = rxDeg * RAD_PER_DEG;
  const ry = ryDeg * RAD_PER_DEG;
  const corners: [number, number][] = [];
  for (const [x, y] of [[-a, -b], [a, -b], [a, b], [-a, b]] as const) {
    const x1 = x * Math.cos(ry);
    const z1 = -x * Math.sin(ry);
    const y2 = y * Math.cos(rx) - z1 * Math.sin(rx);
    const z2 = y * Math.sin(rx) + z1 * Math.cos(rx);
    if (z2 >= perspectivePx) return false;
    const k = perspectivePx / (perspectivePx - z2);
    corners.push([x1 * k, y2 * k]);
  }
  // Each corner of the screen lies on the same side of every edge of the turned planes, which are convex.
  for (const [cx, cy] of [[-width / 2, -height / 2], [width / 2, -height / 2], [width / 2, height / 2], [-width / 2, height / 2]] as const) {
    let side = 0;
    for (let i = 0; i < corners.length; i++) {
      const [x0, y0] = corners[i] ?? [0, 0];
      const [x1, y1] = corners[(i + 1) % corners.length] ?? [0, 0];
      const cross = Math.sign((x1 - x0) * (cy - y0) - (y1 - y0) * (cx - x0));
      if (cross === 0) continue;
      if (side === 0) side = cross;
      else if (cross !== side) return false;
    }
  }
  return true;
}

/** R3350000: the full-screen view's perspective while it turns a diorama, in px, for a screen of this size. */
export function fullscreenDioramaPerspectivePx(width: number, height: number): number {
  return FULLSCREEN_DIORAMA_PERSPECTIVE_VMAX * Math.max(width, height);
}

/**
 * R3350000: the least scale, 1 to FULLSCREEN_COVER_SCALE_MAX, at which a diorama turned `rxDeg` and `ryDeg` still covers a
 * screen of this size (`fullscreenCovers`), found by FULLSCREEN_COVER_STEPS halvings and rounded up to
 * FULLSCREEN_COVER_DIGITS, so a level view keeps its pictures' own size and a turned one grows only as far as it must.
 */
export function fullscreenCoverScale(width: number, height: number, rxDeg: number, ryDeg: number): number {
  const perspective = fullscreenDioramaPerspectivePx(width, height);
  const covers = (scale: number): boolean => fullscreenCovers(width, height, rxDeg, ryDeg, perspective, scale);
  if (covers(1)) return 1;
  let low = 1;
  let high = FULLSCREEN_COVER_SCALE_MAX;
  if (!covers(high)) return high;
  for (let step = 0; step < FULLSCREEN_COVER_STEPS; step++) {
    const mid = (low + high) / 2;
    if (covers(mid)) high = mid;
    else low = mid;
  }
  const unit = 10 ** FULLSCREEN_COVER_DIGITS;
  return Math.ceil(high * unit) / unit;
}

export type FullScreenArtProps = {
  defId: string;
  radiant?: boolean;
  name?: string;
  heldPointerId?: number | null;
  onClose: () => void;
};

export function FullScreenArt({ defId, radiant = false, name, heldPointerId, onClose }: FullScreenArtProps): ReactElement {
  useLocale();
  const [paused, setPaused] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const { deviceTilt } = useCardSettings();
  // R3350000: a Mythic's diorama, its loops held still where the viewer's motion settings ask for the flat picture and while
  // a tap pauses the view (R3345000): a loop runs inside its slice's own picture, where no play state reaches it.
  const diorama = dioramaOf(defId);
  const { flat } = useDepthMotion();
  const maxDeg = diorama !== null ? DIORAMA_TURN_MAX_DEG : CARD_TILT_MAX_DEG;
  const turnsDiorama = diorama !== null;

  // Audio: play intro sting/music on mount, stop on unmount or pause
  useEffect(() => {
    try {
      const intro = MUSIC_CARDS[defId]?.intro;
      if (intro && !paused) {
        getMusicPlayer().playIntro(intro);
      }
    } catch {
      // Audio is never worth an error
    }
    return () => {
      try {
        getMusicPlayer().stopIntro();
      } catch {
        // Audio is never worth an error
      }
    };
  }, [defId, paused]);

  // Release closes: if heldPointerId is released
  useEffect(() => {
    if (heldPointerId == null) return;
    const onPointerUp = (e: PointerEvent) => {
      if (e.pointerId === heldPointerId) {
        onClose();
      }
    };
    window.addEventListener("pointerup", onPointerUp);
    return () => window.removeEventListener("pointerup", onPointerUp);
  }, [heldPointerId, onClose]);

  // Escape and Back close it: the shared modal keys (R2126000 freezes the board's own key handlers).
  useModalKeys(containerRef, onClose);

  // Tilt listener: follows pointer or device orientation
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const apply = (pose: TiltPose) => {
      const angles = tiltDegrees(pose, maxDeg);
      const glare = glareOf(pose);
      container.style.setProperty("--cf-tilt-x", String(pose.x));
      container.style.setProperty("--cf-tilt-y", String(pose.y));
      container.style.setProperty("--cf-tilt-rx", `${angles.rx}deg`);
      container.style.setProperty("--cf-tilt-ry", `${angles.ry}deg`);
      container.style.setProperty("--cf-glare-x", `${glare.x}%`);
      container.style.setProperty("--cf-glare-y", `${glare.y}%`);
      // R3350000: a diorama's turn scales it up as far as the screen needs to stay covered.
      if (turnsDiorama) {
        container.style.setProperty("--fs-perspective", `${fullscreenDioramaPerspectivePx(window.innerWidth, window.innerHeight)}px`);
        container.style.setProperty("--fs-cover", String(fullscreenCoverScale(window.innerWidth, window.innerHeight, angles.rx, angles.ry)));
      }
    };

    const onPointerMove = (e: PointerEvent) => {
      const box = { left: 0, top: 0, width: window.innerWidth, height: window.innerHeight };
      apply(pointerPose(e.clientX, e.clientY, box));
    };

    const onDevice = (e: DeviceOrientationEvent) => {
      if (!deviceTilt) return;
      const pose = devicePose(e.gamma, e.beta);
      if (!pose) return;
      apply(pose);
    };

    if (turnsDiorama) apply({ x: 0, y: 0 });
    window.addEventListener("pointermove", onPointerMove);
    if (deviceTilt) {
      window.addEventListener("deviceorientation", onDevice);
    }
    return () => {
      window.removeEventListener("pointermove", onPointerMove);
      if (deviceTilt) {
        window.removeEventListener("deviceorientation", onDevice);
      }
    };
  }, [deviceTilt, maxDeg, turnsDiorama]);

  const handleClick = (e: React.MouseEvent) => {
    if (e.detail === 2) {
      onClose();
      return;
    }
    setPaused((p) => !p);
  };

  return (
    <div
      ref={containerRef}
      className="fullscreen-art"
      data-testid="fullscreen-art"
      data-paused={paused ? "true" : undefined}
      data-diorama={diorama !== null ? "true" : undefined}
      data-tilt="on"
      role="dialog"
      aria-modal="true"
      aria-label={name ?? defId}
      onClick={handleClick}
    >
      <div className="fullscreen-art-planes" data-diorama={diorama !== null ? "true" : undefined}>
        {diorama !== null
          ? diorama.slices.map((slice, index) => (
              <div
                key={slice.name}
                className="fullscreen-art-plane"
                data-plane={slice.plane}
                data-slice={index}
                data-bend={slice.bend?.axis}
                style={
                  {
                    backgroundImage: dioramaSliceBackground(defId, radiant, index, !flat && !paused),
                    "--fs-slice-shift": `${sliceShiftPx(slice.depth)}px`,
                    ...(slice.bend === undefined ? {} : { "--cf-slice-bend": `${slice.bend.deg}deg` }),
                  } as CSSProperties
                }
              />
            ))
          : ART_PLANES.map((plane) => (
              <div
                key={plane}
                className="fullscreen-art-plane"
                data-plane={plane}
                style={{ backgroundImage: artPlaneBackground(defId, plane, radiant) }}
              />
            ))}
      </div>
      {/* R3221000: the one ✕ every dialog closes by. */}
      <DialogClose
        testid="fullscreen-art-close"
        onClick={(e) => {
          e.stopPropagation();
          onClose();
        }}
      />
    </div>
  );
}

const hosts = new Set<string>();

export function FullScreenArtHost(): ReactElement | null {
  const id = useId();
  const state = useFullScreenArt();
  useEffect(() => {
    hosts.add(id);
    return () => {
      hosts.delete(id);
    };
  }, [id]);

  const isLeader = hosts.size === 0 || Array.from(hosts)[0] === id;
  if (!isLeader || state === null) return null;
  return createPortal(<FullScreenArt {...state} onClose={closeFullScreenArt} />, document.body);
}

/**
 * R3345000: the tap-opened way into the view, for a card that can open it (`useCanOpenFullScreen`): no pointer is held
 * to release, so a tap pauses the view, and a second tap or the ✕ closes it. The hold opens the same view,
 * and its release closes it.
 */
export function FullScreenButton({
  card,
  worn = false,
}: {
  card: NonNullable<FullScreenState> & { rarity?: string | null; printedRarity?: string | null };
  worn?: boolean;
}): ReactElement | null {
  useLocale();
  const can = useCanOpenFullScreen(card, card.defId, worn);
  if (!can) return null;
  return (
    <button
      type="button"
      className="fullscreen-art-open"
      data-testid="fullscreen-art-open"
      onClick={() => openFullScreenArt({ defId: card.defId, radiant: card.radiant, name: card.name, heldPointerId: null })}
    >
      {t("Full screen")}
    </button>
  );
}
