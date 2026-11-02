// R1833000: a cosmetic's look, drawn where a surface names one it grants or sells: the card-pack and
// cosmetic-pack reveals, the token redeem choice and the Chase Shard purchase. A foil, finish or
// border is its card's face wearing it, drawn unworn (`SampleFace`, as the Collection's card view
// draws one); a playmat or a backdrop is the Collection's mini board; a card back is the back a game
// draws; a hero portrait is its art. A full art, or a per-card cosmetic with no card to draw it on,
// has no look here (`hasLook`), and the surface draws its card as before. R1953001: a Legendary or
// Mythic one moves wherever it is drawn: a sample as a face in a game does, the mini board's pads as
// a game's (collection.css), a backdrop and a card back by their own sheets.
//
// `SampleFace` gives a face the four attributes `useCosmeticAttributes` would (cards/cosmetics.ts);
// the Collection's card view (CardCosmeticsView.tsx) draws its previews with it too, and the account
// tiles (AccountCosmeticTiles.tsx) their mini boards with `MiniBoard`. Presentation only (CLAUDE.md
// rule 7): nothing here reads what the account owns or writes a pick.

import { useLayoutEffect, useRef, type ReactElement } from "react";

import { CATALOG } from "@jackioh/cards";

import { CardBack, CardFace, faceModel, useCardSettings } from "../cards/index.ts";
import {
  DEFAULT_CARD_COSMETICS,
  MOVING_FINISHES_AND_BORDERS,
  isChaseCosmetic,
  normalizeCosmetics,
  wornCosmetics,
  type CardSlotKind,
  type Cosmetic,
  type CosmeticId,
} from "../cards/cosmetics.ts";
import type { FaceModel } from "../cards/model.ts";
import { portraitOfCosmetic } from "../emotes/ownedPortraits.ts";
import { PortraitArt } from "../emotes/PortraitArt.tsx";
import Backdrop from "../game/Backdrop.tsx";
import { COSMETICS } from "../wire/generated/cosmetics.ts";

import "../game/table.css";
import "./collection.css";
import "./cosmeticLook.css";

/** The attributes of a face's cosmetics (`CosmeticAttributes`, cards/cosmetics.ts). */
export const COSMETIC_ATTRIBUTES = ["data-cosmetic-foil", "data-cosmetic-motion", "data-finish", "data-frame"] as const;

/**
 * What `useCosmeticAttributes` gives a face wearing `wears`: nothing for a default. Its foil, and its
 * moving finish or border, moves where `moving` says so and holds still otherwise. R1953001: a
 * sample takes none of the COSMETIC_FOIL_ANIMATED_MAX slots of a game's faces, since the samples a
 * screen draws are bounded by the registry, and `SampleFace` moves only a chase.
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
 * `face` drawn unworn, wearing `wears` by the attributes its stylesheet (cards/cosmetics.css) keys on.
 * R1953001: it moves while it wears a chase and the Animated foil setting is on, as a face in a game
 * would; anything else it wears holds still.
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

/** Whether `id` has a look to draw: a foil, finish or border on a card the catalog has (`cardId`), and every playmat, backdrop, card back and portrait. */
export function hasLook(id: string, cardId: string | null): boolean {
  const kind = entryOf(id)?.kind;
  if (isCardSlot(kind)) return cardId !== null && CATALOG[cardId] !== undefined;
  if (kind === "portrait") return portraitOfCosmetic(id) !== null;
  return kind === "table" || kind === "field" || kind === "cardBack";
}

/** `id`'s look (`cardId` the card a per-card one is on), or nothing where `hasLook` says it has none. */
export function CosmeticLook({ id, cardId }: { id: string; cardId: string | null }): ReactElement | null {
  const { rarityFoil } = useCardSettings();
  const cosmetic = entryOf(id);
  if (cosmetic === undefined || !hasLook(id, cardId)) return null;
  const { kind } = cosmetic;
  const portrait = portraitOfCosmetic(id);
  // A portrait fills the face in its card's place, as it did before it had a look of its own (R1827000).
  if (kind === "portrait" && portrait !== null) return <PortraitArt portrait={portrait} className="pack-slot__portrait" />;
  let look: ReactElement | null = null;
  if (isCardSlot(kind) && cardId !== null) {
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
  }
  return (
    <span className="cosmetic-look" data-testid={`cosmetic-look-${id}`} data-kind={kind}>
      {look}
    </span>
  );
}
