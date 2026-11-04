// The art in a card's window (docs/polish/6-cards.md, Surface A, B6).
// It is aria-hidden and names nothing, so a face-down card cannot leak (CLAUDE.md rule 7).
// R503: read the motif from the printed name or closest catalog (`useDefResolver`), keeping a
// card's art consistent across zones.
// R2715001: given a full-art card's motion (motion.ts), the procedural picture moves its own layers
// (`data-art-moving`); the face decides when (CardFace, `useArtMotion`). A delivered file, a
// Starship's art and a Legacy card's (R2724000) never move.
// R2949000: given a `picture` of its own (pictures.ts), the procedural, Stinkyscape and Training Grounds
// painters draw the card's subject in a picture seeded apart from its normal one (`data-art-picture`); a
// Starship's art and a Legacy card's keep their one picture.
// R2940001: Alt art's picture ("alt") is seeded apart as Signature's is; the Sketch ("sketch") pencils whatever
// picture the window draws, a Starship's and a Legacy card's too. A delivered file is the normal picture's (and the
// Signature look's, R2949000), so Alt art and the Sketch draw the painter's.

import { useEffect, useRef, useState, type CSSProperties, type ReactElement } from "react";

import type { CardType, Tag } from "@jackioh/shared";

import { useDefResolver } from "../refContext.tsx";
import { ART_MANIFEST, artUrl, type ArtManifest } from "./manifest.ts";
import type { ArtMotion } from "./motion.ts";
import { canWatchArt, whenNear } from "./near.ts";
import { motifFor } from "./motifs.ts";
import { artSeedOf, type ArtPicture } from "./pictures.ts";
import { isStinkyscapeArt, stinkySubject } from "./stinkyscape.ts";
import { legacyArtUri, proceduralArtUri, starshipArtUri, stinkyscapeArtUri, trainingGroundsArtUri } from "./svg.ts";
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
  name?: string;
  manifest?: ArtManifest;
  className?: string;
  /** Avoid parsing art for grid cards scrolled past (near.ts); use the theme sky until near. */
  lazy?: boolean;
  /** R2249000: a Starship in play (`FaceModel.starship`) wears its own art, never its fused definition's. */
  starship?: { pieces: number; launched: boolean } | null;
  /** R2715001: the card's moving art, which its procedural picture draws in place of the still one; null holds it still. */
  motion?: ArtMotion | null;
  /** R2949000, R2940001: which of the card's pictures its painter draws (pictures.ts); "normal" by default. */
  picture?: ArtPicture;
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
  motion = null,
  picture = "normal",
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
  // R2170000, R2202000 and R2724000: the Stinkyscape and Training Grounds sets draw their own
  // families, and a Training Grounds Legacy card its own yard picture with its Core card's subject,
  // read by the Core card's name (its own after "Legacy: ", R2511000); every other card keeps R503's art.
  const coreId = snapshotArtOf(defId);
  const ownName = name ?? resolve?.(defId)?.name;
  const cardName = coreId === null ? ownName : ownName?.replace(LEGACY_PREFIX, "");
  const stinky = isStinkyscapeArt(defId);
  const yard = isTrainingGroundsArt(defId);
  const motif = stinky || yard ? null : motifFor(cardName, theme);
  const variant = radiant ? "radiant" : "base";
  // R2940001: the Sketch pencils the window's picture, whatever draws it.
  const sketch = picture === "sketch";
  const real = starship === null && picture !== "alt" && !sketch ? artUrl(defId, radiant, manifest) : null;
  // R2949000: the seed the painter grows this picture from; a Starship and a Legacy card draw one picture.
  const ownPicture = picture !== "normal" && !sketch && starship === null && coreId === null;
  const seedId = ownPicture ? artSeedOf(defId, picture) : defId;

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
      ? starshipArtUri(starship.pieces, starship.launched, radiant, sketch)
      : stinky
        ? stinkyscapeArtUri(defId, cardName, tags, type, radiant, motion, seedId, sketch)
        : yard
          ? trainingGroundsArtUri(defId, cardName, tags, type, radiant, motion, seedId, sketch)
          : coreId !== null
            ? legacyArtUri(defId, coreId, cardName, tags, type, radiant, sketch)
            : proceduralArtUri(seedId, theme, compositionFor(type), radiant, motif, motion, sketch);
  const family =
    starship !== null
      ? "starship"
      : stinky
        ? stinkySubject(defId, cardName, tags, type).family
        : yard
          ? yardSubject(defId, cardName, tags, type).family
          : coreId !== null
            ? "legacy"
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
      data-art-picture={ownPicture || sketch ? picture : undefined}
      data-art-pending={uri === null ? "true" : undefined}
      data-art-moving={uri !== null && starship === null && coreId === null && motion !== null ? "true" : undefined}
      aria-hidden="true"
      style={
        uri === null
          ? ({ "--art-sky-1": skyTop, "--art-sky-2": skyBottom } as CSSProperties)
          : { backgroundImage: `url("${uri}")`, backgroundSize: "cover" }
      }
    />
  );
}
