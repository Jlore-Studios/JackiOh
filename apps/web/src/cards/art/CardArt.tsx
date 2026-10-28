// The art in a card's window (docs/polish/6-cards.md, Surface A, B6).
// It is aria-hidden and names nothing, so a face-down card cannot leak (CLAUDE.md rule 7).
// R503: read the motif from the printed name or closest catalog (`useDefResolver`), keeping a
// card's art consistent across zones.

import { useEffect, useRef, useState, type CSSProperties, type ReactElement } from "react";

import type { CardType, Tag } from "@jackioh/shared";

import { useDefResolver } from "../refContext.tsx";
import { ART_MANIFEST, artUrl, type ArtManifest } from "./manifest.ts";
import { canWatchArt, whenNear } from "./near.ts";
import { motifFor } from "./motifs.ts";
import { isStinkyscapeArt, stinkySubject } from "./stinkyscape.ts";
import { proceduralArtUri, stinkyscapeArtUri, trainingGroundsArtUri } from "./svg.ts";
import { isTrainingGroundsArt, snapshotArtOf, yardSubject } from "./traininggrounds.ts";
import { compositionFor, themeFor, THEME_PALETTES } from "./themes.ts";

import "./art.css";

export type ArtShape = "portrait" | "window" | "arch" | "notched" | "oval" | "strip" | "full";

export type CardArtProps = {
  defId: string;
  radiant: boolean;
  tags: readonly Tag[];
  type: CardType;
  shape: ArtShape;
  /** R503's motif name; defaults to the closest catalog. */
  name?: string;
  manifest?: ArtManifest;
  className?: string;
  /** Avoid parsing art for grid cards scrolled past (near.ts); use the theme sky until near. */
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
  // Keep failures per source so cards and faces retry independently.
  const [failedSrc, setFailedSrc] = useState<string | null>(null);
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
  const cardName = name ?? resolve?.(defId)?.name;
  // R2170000 and R2202000: the Stinkyscape and Training Grounds sets draw their own families, and a
  // Training Grounds snapshot its Core card's picture; every other card keeps R503's art.
  const artId = snapshotArtOf(defId) ?? defId;
  const stinky = isStinkyscapeArt(defId);
  const yard = isTrainingGroundsArt(defId);
  const motif = stinky || yard ? null : motifFor(cardName, theme);
  const variant = radiant ? "radiant" : "base";
  const real = artUrl(artId, radiant, manifest);

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

  const uri = !near
    ? null
    : stinky
      ? stinkyscapeArtUri(defId, cardName, tags, type, radiant)
      : yard
        ? trainingGroundsArtUri(defId, cardName, tags, type, radiant)
        : proceduralArtUri(artId, theme, compositionFor(type), radiant, motif);
  const [skyTop, skyBottom] = THEME_PALETTES[theme].sky;
  return (
    <span
      ref={windowRef}
      className={classes(shape, className)}
      data-art="procedural"
      data-art-theme={theme}
      data-art-variant={variant}
      data-art-motif={motif ?? undefined}
      data-art-family={
        stinky ? stinkySubject(defId, cardName, tags, type).family : yard ? yardSubject(defId, cardName, tags, type).family : undefined
      }
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
