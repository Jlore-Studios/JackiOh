// R3345000: full-screen art view with depth planes, dolly camera, tilt, and intro audio.
// Presentation only (CLAUDE.md rule 7).

import { useEffect, useId, useRef, useState, type ReactElement } from "react";
import { createPortal } from "react-dom";

import { getMusicPlayer } from "../audio/music.ts";
import { MUSIC_CARDS } from "../audio/musicData.ts";
import { DialogClose } from "../game/Dialog.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import { useModalKeys } from "../together/modalKeys.ts";
import { ART_PLANES, artPlaneBackground } from "./art/pictures.ts";
import { closeFullScreenArt, openFullScreenArt, useCanOpenFullScreen, useFullScreenArt, type FullScreenState } from "./fullscreen.ts";
import { useCardSettings } from "./settings.ts";
import { devicePose, glareOf, pointerPose, tiltDegrees } from "./tilt.ts";

import "./fullscreen.css";

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

    const onPointerMove = (e: PointerEvent) => {
      const box = { left: 0, top: 0, width: window.innerWidth, height: window.innerHeight };
      const pose = pointerPose(e.clientX, e.clientY, box);
      const angles = tiltDegrees(pose);
      const glare = glareOf(pose);
      container.style.setProperty("--cf-tilt-x", String(pose.x));
      container.style.setProperty("--cf-tilt-y", String(pose.y));
      container.style.setProperty("--cf-tilt-rx", `${angles.rx}deg`);
      container.style.setProperty("--cf-tilt-ry", `${angles.ry}deg`);
      container.style.setProperty("--cf-glare-x", `${glare.x}%`);
      container.style.setProperty("--cf-glare-y", `${glare.y}%`);
    };

    const onDevice = (e: DeviceOrientationEvent) => {
      if (!deviceTilt) return;
      const pose = devicePose(e.gamma, e.beta);
      if (!pose) return;
      const angles = tiltDegrees(pose);
      const glare = glareOf(pose);
      container.style.setProperty("--cf-tilt-x", String(pose.x));
      container.style.setProperty("--cf-tilt-y", String(pose.y));
      container.style.setProperty("--cf-tilt-rx", `${angles.rx}deg`);
      container.style.setProperty("--cf-tilt-ry", `${angles.ry}deg`);
      container.style.setProperty("--cf-glare-x", `${glare.x}%`);
      container.style.setProperty("--cf-glare-y", `${glare.y}%`);
    };

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
  }, [deviceTilt]);

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
      data-tilt="on"
      role="dialog"
      aria-modal="true"
      aria-label={name ?? defId}
      onClick={handleClick}
    >
      <div className="fullscreen-art-planes">
        {ART_PLANES.map((plane) => (
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
