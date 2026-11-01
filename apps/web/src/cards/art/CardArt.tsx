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
import { proceduralArtUri, starshipArtUri, stinkyscapeArtUri, trainingGroundsArtUri } from "./svg.ts";
import { isTrainingGroundsArt, LEGACY_PREFIX, snapshotArtOf, yardSubject } from "./traininggrounds.ts";
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
  /**
   * R2249000: the card is a Starship in play (`FaceModel.starship`): it wears the Starship's own art,
   * drawn from its pieces and its launch, whatever definition its fusions gave it.
   */
  starship?: { pieces: number; launched: boolean } | null;
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
  starship = null,
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
  // R2170000 and R2202000: the Stinkyscape and Training Grounds sets draw their own families, and a
  // Training Grounds Legacy card its Core card's picture, read by the Core card's name (its own after
  // "Legacy: ", R2511000); every other card keeps R503's art.
  const coreId = snapshotArtOf(defId);
  const ownName = name ?? resolve?.(defId)?.name;
  const cardName = coreId === null ? ownName : ownName?.replace(LEGACY_PREFIX, "");
  const artId = coreId ?? defId;
  const stinky = isStinkyscapeArt(defId);
  const yard = isTrainingGroundsArt(defId);
  const motif = stinky || yard ? null : motifFor(cardName, theme);
  const variant = radiant ? "radiant" : "base";
  // R2249000: a Starship draws its own art, never a real one its fused definition might name.
  const real = starship === null ? artUrl(artId, radiant, manifest) : null;

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
    : starship !== null
      ? starshipArtUri(starship.pieces, starship.launched, radiant)
      : stinky
        ? stinkyscapeArtUri(defId, cardName, tags, type, radiant)
        : yard
          ? trainingGroundsArtUri(defId, cardName, tags, type, radiant)
          : proceduralArtUri(artId, theme, compositionFor(type), radiant, motif);
  const family =
    starship !== null
      ? "starship"
      : stinky
        ? stinkySubject(defId, cardName, tags, type).family
        : yard
          ? yardSubject(defId, cardName, tags, type).family
          : undefined;
  const [skyTop, skyBottom] = THEME_PALETTES[theme].sky;
  return (
    <span
      ref={windowRef}
      className={classes(shape, className)}
      data-art="procedural"
      data-art-theme={theme}
      data-art-variant={variant}
      data-art-motif={motif ?? undefined}
      data-art-family={family}
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
