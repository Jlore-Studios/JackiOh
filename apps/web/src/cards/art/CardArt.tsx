// The art in a card's window (docs/polish/6-cards.md, Surface A, B6).
//
// A card with real art listed in the manifest gets an <img>; every other card, and any card
// whose image fails to load, gets its procedural SVG as the span's background. The art is
// decoration: it is aria-hidden, holds no text and names nothing, so a face-down card can never
// leak through it (CLAUDE.md rule 7). The span fills whatever box its parent gives it and clips
// itself to `shape`; sizing it is the parent's job, so nothing here sets a width.
//
// R503: the procedural picture carries the card's motif, read from its printed name (motifs.ts). A
// caller that holds the name passes it; one that does not gets it from the closest catalog
// (`useDefResolver`: the deck builder's, else the board's), so a card draws the same picture in the
// hand, on the board and in the deck list. With no name anywhere, the picture has no motif.

import { useEffect, useRef, useState, type ReactElement } from "react";

import type { CardType, Tag } from "@jackioh/shared";

import { useDefResolver } from "../refContext.tsx";
import { ART_MANIFEST, artUrl, type ArtManifest } from "./manifest.ts";
import { canWatchArt, whenNear } from "./near.ts";
import { motifFor } from "./motifs.ts";
import { proceduralArtUri } from "./svg.ts";
import { compositionFor, themeFor } from "./themes.ts";

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
   * A grid of many faces: draw the procedural picture only once the window is near the screen
   * (near.ts). Until then the window is its dark ground, `data-art-pending="true"`. A real-art
   * `<img>` is already `loading="lazy"`, and a window that cannot be watched draws at once.
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
  // A lazy window is drawn once it has been near the screen, and stays drawn.
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
      style={uri === null ? undefined : { backgroundImage: `url("${uri}")`, backgroundSize: "cover" }}
    />
  );
}
