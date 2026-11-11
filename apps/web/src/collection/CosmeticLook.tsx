// R1833000: a cosmetic's look where a surface grants or sells one: a foil, finish or border on its
// card's face, drawn unworn (`SampleFace`); a playmat or backdrop as the mini board; a card back as a
// game's; a portrait as its art. A full art, or a per-card one with no card, has none (`hasLook`).
// R1953001: a Legendary or Mythic one moves wherever it is drawn. Presentation only (CLAUDE.md rule 7).
// R3335000: a Starship cosmetic as a launched Starship wearing it over the standard look (`StarshipPreview`).

import { useLayoutEffect, useRef, type ReactElement } from "react";

import { CATALOG } from "@jackioh/cards";

import { CardBack, CardFace, faceModel, useCardSettings } from "../cards/index.ts";
import {
  DEFAULT_CARD_COSMETICS,
  MOVING_FINISHES_AND_BORDERS,
  STARSHIP_SLOT_KINDS,
  isChaseCosmetic,
  normalizeCosmetics,
  wornCosmetics,
  type CardSlotKind,
  type Cosmetic,
  type CosmeticId,
  type StarshipSlotKind,
} from "../cards/cosmetics.ts";
import { STARSHIP_LOOK_STANDARD } from "../cards/art/starshipLooks.ts";
import type { FaceModel } from "../cards/model.ts";
import { portraitOfCosmetic } from "../emotes/ownedPortraits.ts";
import { PortraitArt } from "../emotes/PortraitArt.tsx";
import Backdrop from "../game/Backdrop.tsx";
import { COSMETICS } from "../wire/generated/cosmetics.ts";
import { useEngineReady } from "../wasm/useEngineReady.ts";
import StarshipPreview from "./StarshipPreview.tsx";

import "../game/table.css";
import "./collection.css";
import "./cosmeticLook.css";

/** The attributes of a face's cosmetics (`CosmeticAttributes`, cards/cosmetics.ts). */
export const COSMETIC_ATTRIBUTES = ["data-cosmetic-foil", "data-cosmetic-motion", "data-finish", "data-frame"] as const;

/**
 * What `useCosmeticAppearance` gives a face wearing `wears`: nothing for a default; a foil, moving
 * finish or border moves where `moving` says. R1953001: a sample takes none of the
 * COSMETIC_FOIL_ANIMATED_MAX slots of a game's faces, since the samples a screen draws are bounded.
 */
export function attributesOf(
  wears: Record<CardSlotKind, CosmeticId>,
  moving: boolean,
): Record<(typeof COSMETIC_ATTRIBUTES)[number], string | undefined> {
  const foiled = wears.foil !== DEFAULT_CARD_COSMETICS.foil;
  const animates = foiled || MOVING_FINISHES_AND_BORDERS.has(wears.finish) || MOVING_FINISHES_AND_BORDERS.has(wears.border);
  return {
    "data-cosmetic-foil": foiled ? wears.foil : undefined,
    "data-cosmetic-motion": animates ? (moving ? "animated" : "static") : undefined,
    "data-finish": wears.finish === DEFAULT_CARD_COSMETICS.finish ? undefined : wears.finish,
    "data-frame": wears.border === DEFAULT_CARD_COSMETICS.border ? undefined : wears.border,
  };
}

/**
 * `face` drawn unworn, wearing `wears` by the attributes cards/cosmetics.css keys on. R1953001: it
 * moves while it wears a chase and the Animated foil setting is on, as a face in a game would.
 */
export function SampleFace({
  face,
  layout,
  wears,
}: {
  face: FaceModel;
  layout: "full" | "compact";
  wears: Record<CardSlotKind, CosmeticId>;
}): ReactElement {
  const host = useRef<HTMLSpanElement>(null);
  const { animatedFoil } = useCardSettings();
  const attributes = attributesOf(wears, animatedFoil && [wears.foil, wears.finish, wears.border].some(isChaseCosmetic));
  useLayoutEffect(() => {
    const root = host.current?.firstElementChild;
    if (root === null || root === undefined) return;
    for (const name of COSMETIC_ATTRIBUTES) {
      const value = attributes[name];
      if (value === undefined) root.removeAttribute(name);
      else root.setAttribute(name, value);
    }
  });
  return (
    <span ref={host} className="collection-sample">
      <CardFace face={face} layout={layout} />
    </span>
  );
}

/** A playmat and a backdrop, drawn together: the mat under the viewer's half, the backdrop over it, a lane of pads each side. */
export function MiniBoard({ table, field }: { table: CosmeticId; field: CosmeticId }): ReactElement {
  return (
    <div className="collection-miniboard" data-table={table} aria-hidden="true">
      {field === DEFAULT_CARD_COSMETICS.field ? null : <Backdrop field={field} />}
      <span className="collection-miniboard__pads collection-miniboard__pads--them">
        <span />
        <span />
        <span />
      </span>
      <span className="collection-miniboard__pads collection-miniboard__pads--you">
        <span />
        <span />
        <span />
      </span>
    </div>
  );
}

function entryOf(id: string): Cosmetic | undefined {
  return COSMETICS.find((cosmetic) => cosmetic.id === id);
}

function isCardSlot(kind: string | undefined): kind is CardSlotKind {
  return kind === "foil" || kind === "finish" || kind === "border";
}

function isStarshipSlot(kind: string | undefined): kind is StarshipSlotKind {
  return STARSHIP_SLOT_KINDS.some((slot) => slot === kind);
}

/**
 * Whether `id` has a look to draw: a foil, finish or border on a card the catalog has (`cardId`), and every playmat,
 * backdrop, card back, portrait and (R3335000) Starship cosmetic.
 */
export function hasLook(id: string, cardId: string | null): boolean {
  const kind = entryOf(id)?.kind;
  if (isCardSlot(kind)) return cardId !== null && CATALOG[cardId] !== undefined;
  if (kind === "portrait") return portraitOfCosmetic(id) !== null;
  return kind === "table" || kind === "field" || kind === "cardBack" || isStarshipSlot(kind);
}

/** `id`'s look (`cardId` the card a per-card one is on), or nothing where `hasLook` says it has none. */
export function CosmeticLook({ id, cardId }: { id: string; cardId: string | null }): ReactElement | null {
  const { rarityFoil } = useCardSettings();
  // A card's look is a face, which the module works out; the shop draws one before the module may be in (#778).
  const engineReady = useEngineReady();
  const cosmetic = entryOf(id);
  if (cosmetic === undefined || !hasLook(id, cardId)) return null;
  const { kind } = cosmetic;
  const portrait = portraitOfCosmetic(id);
  // A portrait fills the face in its card's place (R1827000).
  if (kind === "portrait" && portrait !== null) return <PortraitArt portrait={portrait} className="pack-slot__portrait" />;
  let look: ReactElement | null = null;
  if (isCardSlot(kind) && cardId !== null) {
    if (!engineReady) return null;
    const face = faceModel({ defId: cardId, def: CATALOG[cardId], radiant: false });
    // What the card wears in a game with this pick and the defaults: a Mythic face wearing its
    // rarity foil (R2721000) keeps its own foil.
    look = <SampleFace face={face} layout="full" wears={wornCosmetics(face, normalizeCosmetics({ [kind]: id }), {}, () => true, rarityFoil)} />;
  } else if (kind === "table" || kind === "field") {
    look = (
      <MiniBoard
        table={kind === "table" ? cosmetic.id : DEFAULT_CARD_COSMETICS.table}
        field={kind === "field" ? cosmetic.id : DEFAULT_CARD_COSMETICS.field}
      />
    );
  } else if (kind === "cardBack") {
    look = (
      <span className="cosmetic-look__back">
        <CardBack back={id} />
      </span>
    );
  } else if (isStarshipSlot(kind)) {
    look = <StarshipPreview look={{ ...STARSHIP_LOOK_STANDARD, [kind]: id }} />;
  }
  return (
    <span className="cosmetic-look" data-testid={`cosmetic-look-${id}`} data-kind={kind}>
      {look}
    </span>
  );
}
