// The art in a card's window (docs/polish/6-cards.md, Surface A, B6).
//
// Real art listed in the manifest gets an <img>; every other card, and one whose image fails to
// load, gets its procedural SVG as the span's background. The art is aria-hidden and holds no
// text, so a face-down card can never leak through it (CLAUDE.md rule 7). The span fills its
// parent's box and clips itself to `shape`; sizing it is the parent's job.
//
// R503: the picture carries the card's motif, read from its printed name (motifs.ts). A caller
// without the name gets it from the closest catalog (`useDefResolver`), so a card draws the same
// picture in the hand, on the board and in the deck list.

import { useEffect, useRef, useState, type CSSProperties, type ReactElement } from "react";

import type { CardType, Tag } from "@jackioh/shared";

import { useDefResolver } from "../refContext.tsx";
import { ART_MANIFEST, artUrl, type ArtManifest } from "./manifest.ts";
import { canWatchArt, whenNear } from "./near.ts";
import { motifFor } from "./motifs.ts";
import { proceduralArtUri } from "./svg.ts";
import { compositionFor, themeFor, THEME_PALETTES } from "./themes.ts";

import "./art.css";

export type ArtShape = "portrait" | "window" | "arch" | "notched" | "oval" | "strip";

export type CardArtProps = {
  defId: string;
  radiant: boolean;
  tags: readonly Tag[];
  type: CardType;
  shape: ArtShape;
  /** The card's printed name, which picks its motif (R503). Absent: the closest catalog's, if any. */
  name?: string;
  /** For tests. Defaults to ART_MANIFEST. */
  manifest?: ArtManifest;
  className?: string;
  /**
   * A grid of many faces: draw the procedural picture only once the window has stayed near the
   * screen for a short dwell (near.ts); until then it is its theme's sky as a flat gradient,
   * `data-art-pending="true"` (art.css). A window that cannot be watched draws at once.
   */
  lazy?: boolean;
};

function classes(shape: ArtShape, className: string | undefined): string {
  const base = `cf-art cf-art--${shape}`;
  return className === undefined || className === "" ? base : `${base} ${className}`;
}

export function CardArt({
  defId,
  radiant,
  tags,
  type,
  shape,
  name,
  manifest = ART_MANIFEST,
  className,
  lazy = false,
}: CardArtProps): ReactElement {
  // The src that failed to load, so a later src (another card, the other face) gets its own try.
  const [failedSrc, setFailedSrc] = useState<string | null>(null);
  // A lazy window is drawn once it has dwelt near the screen, and stays drawn.
  const windowRef = useRef<HTMLSpanElement>(null);
  const [near, setNear] = useState(() => !lazy || !canWatchArt());
  useEffect(() => {
    const element = windowRef.current;
    if (near || element === null) return undefined;
    return whenNear(element, () => {
      setNear(true);
    });
  }, [near]);
  const resolve = useDefResolver();
  const theme = themeFor(tags, type);
  const motif = motifFor(name ?? resolve?.(defId)?.name, theme);
  const variant = radiant ? "radiant" : "base";
  const real = artUrl(defId, radiant, manifest);

  if (real !== null && real.src !== failedSrc) {
    const src = real.src;
    return (
      <span
        className={classes(shape, className)}
        data-art="real"
        data-art-theme={theme}
        data-art-variant={variant}
        aria-hidden="true"
      >
        <img
          className="cf-art-img"
          src={src}
          alt=""
          loading="lazy"
          decoding="async"
          draggable={false}
          onError={() => setFailedSrc(src)}
        />
        {real.tint ? <span className="cf-art-tint" /> : null}
      </span>
    );
  }

  const uri = near ? proceduralArtUri(defId, theme, compositionFor(type), radiant, motif) : null;
  // The pending window's placeholder sky, which art.css paints as a flat gradient.
  const [skyTop, skyBottom] = THEME_PALETTES[theme].sky;
  return (
    <span
      ref={windowRef}
      className={classes(shape, className)}
      data-art="procedural"
      data-art-theme={theme}
      data-art-variant={variant}
      data-art-motif={motif ?? undefined}
      data-art-pending={uri === null ? "true" : undefined}
      aria-hidden="true"
      style={
        uri === null
          ? ({ "--art-sky-1": skyTop, "--art-sky-2": skyBottom } as CSSProperties)
          : { backgroundImage: `url("${uri}")`, backgroundSize: "cover" }
      }
    />
  );
}
