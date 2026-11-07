// The art in a card's window (docs/polish/6-cards.md, Surface A, B6).
// It is aria-hidden and names nothing, so a face-down card cannot leak (CLAUDE.md rule 7).
// R503: read the motif from the printed name or closest catalog (`useDefResolver`), keeping a
// card's art consistent across zones.
// R2715001: given a full-art card's motion (motion.ts), the procedural picture moves its own layers
// (`data-art-moving`); the face decides when (CardFace, `useArtMotion`). A delivered file, a
// Starship's art and a Legacy card's (R2724000) never move.
// R2940001: given Alt art's picture ("alt", pictures.ts), the procedural, Stinkyscape and Training Grounds painters
// draw the card's subject in a picture seeded apart from its normal one (`data-art-picture`); a Starship's art and a
// Legacy card's keep their one picture. The Sketch ("sketch") pencils whatever picture the window draws, a
// Starship's and a Legacy card's too. A delivered file is the normal picture's, so Alt art and the Sketch draw the
// painter's. R3015002: every level of a variant gives its first level's picture here (versions.ts `artPictureOf`).
// R2942001: Pixel ("pixel") and Comic ("comic") restyle the window's picture as the Sketch does (pictures.ts
// `restyleOf`), a Starship's and a Legacy card's too, and draw the painter's in place of a delivered file.
// R2943001: a Set showcase restyle ("showcase-…") does the same. Given a `part` other than "whole" (Out of frame's),
// the procedural, Stinkyscape and Training Grounds painters draw the picture's backdrop or its subject alone
// (`data-art-part`), and a delivered file, which has no parts, gives way to the painter.
// R3017002: a repaint (Night's, Watercolor's, Stained-glass window's, Tarot card's: pictures.ts `isRepaint`) draws a
// picture of its own, seeded apart as Alt art's is, under its treatment; a Starship's art and a Legacy card's keep their
// one picture under it. Woodcut ("woodcut") and Neon sign ("neon") restyle the window's picture as Pixel does. Each draws
// the painter's in place of a delivered file.
// R3336000: a Drone Battleship layer on the board (`ship`, stinkyscape.ts `shipArtOf`) draws the decks of its ship still
// standing, `data-art-decks`; its printed card draws its own deck large with the other two faint.
// R3045000: a fused definition holding a Miner or a Weapon draws the art of the card it was fused onto
// (fusedArt.ts), its id, name and tags standing for the fusion's wherever the art reads them.
// R3338001: a procedural picture draws the card's own composition where it has one (themes.ts `compositionOf`:
// Ceaseless Void's giant-shaped hole), else its type's.
// R3333000: a Starship's art is drawn from the pieces its view lists, oldest first, and its launch, in the hull design
// of the name it launched under (the face's, `name`), a Dormant one in its shipyard (`data-art-design`).

import { useEffect, useRef, useState, type CSSProperties, type ReactElement } from "react";

import type { CardType, Tag } from "@jackioh/shared";

import type { StarshipPieceRef } from "../../wire/generated/StarshipPieceRef.ts";
import { useDefResolver } from "../refContext.tsx";
import { artSubjectOf } from "./fusedArt.ts";
import { ART_MANIFEST, artUrl, type ArtManifest } from "./manifest.ts";
import type { ArtMotion } from "./motion.ts";
import { canWatchArt, whenNear } from "./near.ts";
import { motifFor } from "./motifs.ts";
import { artSeedOf, isRepaint, restyleOf, type ArtPart, type ArtPicture } from "./pictures.ts";
import { starshipArtOf } from "./starshipHulls.ts";
import { isStinkyscapeArt, shipDecksOf, stinkySubject, type ShipArt } from "./stinkyscape.ts";
import { legacyArtUri, proceduralArtUri, starshipArtUri, stinkyscapeArtUri, trainingGroundsArtUri } from "./svg.ts";
import { isTrainingGroundsArt, LEGACY_PREFIX, snapshotArtOf, yardSubject } from "./traininggrounds.ts";
import { compositionOf, themeFor, THEME_PALETTES } from "./themes.ts";

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
  /**
   * R2249000: a Starship in play (`FaceModel.starship`) wears its own art, never its fused definition's; R3333000: drawn
   * from the pieces it lists, oldest first.
   */
  starship?: { launched: boolean; list: readonly StarshipPieceRef[] } | null;
  /** R2715001: the card's moving art, which its procedural picture draws in place of the still one; null holds it still. */
  motion?: ArtMotion | null;
  /** R2940001, R2942001: which of the card's pictures its painter draws (pictures.ts); "normal" by default. */
  picture?: ArtPicture;
  /** R2943001: which part of the picture the window draws (pictures.ts `ArtPart`); "whole" by default. */
  part?: ArtPart;
  /** R3336000: a Drone Battleship layer on the board (`shipArtOf`) draws the decks of its ship still standing; null draws its printed picture. */
  ship?: ShipArt | null;
};

/**
 * R2943001: whether a window draws a card's picture in parts (`ArtPart`): its painter's own, which a Starship in play
 * and a Legacy card do not have, their painters drawing one picture.
 */
export function hasArtParts(defId: string, starship: CardArtProps["starship"]): boolean {
  return starship == null && snapshotArtOf(defId) === null;
}

function classes(shape: ArtShape, className: string | undefined): string {
  const base = `cf-art cf-art--${shape}`;
  return className === undefined || className === "" ? base : `${base} ${className}`;
}

export function CardArt({
  defId: shownId,
  radiant,
  tags: shownTags,
  type,
  shape,
  name: shownName,
  manifest = ART_MANIFEST,
  className,
  lazy = false,
  starship = null,
  motion = null,
  picture = "normal",
  part = "whole",
  ship = null,
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
  // R3045000: a Miner or a Weapon fused onto a card leaves that card's art; its id, name and tags stand for the fusion's.
  const { defId, name, tags } = artSubjectOf({ defId: shownId, name: shownName, tags: shownTags }, resolve);
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
  // R2940001: the Sketch pencils the window's picture, whatever draws it; R2942001: Pixel and Comic restyle it so.
  const restyle = restyleOf(picture);
  // R2943001: a part is the painter's, so a delivered file gives way to it; a Starship and a Legacy card draw it whole.
  const split = hasArtParts(defId, starship) ? part : "whole";
  const real = starship === null && picture !== "alt" && restyle === null && split === "whole" ? artUrl(defId, radiant, manifest) : null;
  // R2940001: the seed the painter grows this picture from; a Starship and a Legacy card draw one picture. R3017002: a
  // repaint's is its own, under its treatment.
  const ownPicture = picture !== "normal" && (restyle === null || isRepaint(picture)) && starship === null && coreId === null;
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

  // R3333000: the design a Starship wears is its launched name's, the face's (`artSubjectOf` may stand another for it).
  const starshipArt = starship === null ? null : starshipArtOf(starship.list, starship.launched, shownName, radiant);
  const uri = !near
    ? null
    : starshipArt !== null
      ? starshipArtUri(starshipArt, restyle)
      : stinky
        ? stinkyscapeArtUri(defId, cardName, tags, type, radiant, motion, seedId, restyle, split, ship)
        : yard
          ? trainingGroundsArtUri(defId, cardName, tags, type, radiant, motion, seedId, restyle, split)
          : coreId !== null
            ? legacyArtUri(defId, coreId, cardName, tags, type, radiant, restyle)
            : proceduralArtUri(seedId, theme, compositionOf(defId, type), radiant, motif, motion, restyle, split);
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
      data-art-design={starshipArt?.design}
      data-art-picture={ownPicture || restyle !== null ? picture : undefined}
      data-art-part={split === "whole" ? undefined : split}
      data-art-decks={stinky ? (shipDecksOf(defId, ship) ?? undefined) : undefined}
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
