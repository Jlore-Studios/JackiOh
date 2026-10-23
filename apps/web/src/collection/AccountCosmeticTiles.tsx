// The account's cosmetics as preview tiles (#1662, R1655000, R1579000, part of #1651): the playmat
// (`table`), the backdrop (`field`) and the card back (`cardBack`) your own side wears in a game. A
// playmat and a backdrop are drawn together as a mini board, each tile its own kind over what is
// equipped of the other; a card back is drawn by `cards/CardBack.tsx`, the back a game draws.
//
// Clicking an owned tile equips it as the select it replaced did: `writeCosmetics`, the same store
// and `cosmetics` group (R633, `cards/cosmetics.ts`, `settings/groups.ts`), and no request. A tile
// the account does not own is locked and greyed with its rarity, and equips nothing. `GET
// /api/cosmetics` says what the account owns (`accountTiles`); this decides nothing (CLAUDE.md rule 7).
//
// Every animation is a transform or opacity (the backdrops' particles, game/table.css) and holds still
// under reduced motion; a tile's particles move only while it is the equipped one, hovered or focused
// (collection.css).

import type { ReactElement } from "react";

import { CardBack } from "../cards/index.ts";
import {
  DEFAULT_CARD_COSMETICS,
  resolveCosmetic,
  useCosmetics,
  writeCosmetics,
  type CosmeticId,
  type DeckSlotKind,
} from "../cards/cosmetics.ts";
import Backdrop from "../game/Backdrop.tsx";
import { t } from "../i18n/i18n.ts";
import type { OwnedCosmeticsResponse } from "../net/api.ts";
import { accountTiles, collectionTestid as T, cosmeticKindWords, type AccountTile } from "./collection.ts";

import "../game/table.css";
import "../settings/settings.css";

/** The playmat and backdrop of a mini board, then the card back, each its own row of tiles. */
const TILE_ROWS: readonly { kind: DeckSlotKind; hint: string }[] = [
  { kind: "table", hint: "The playmat on your side. A deck can pick its own." },
  { kind: "field", hint: "The backdrop behind your side. Reduce motion holds it still." },
  { kind: "cardBack", hint: "The back of your own face-down cards and your deck." },
];

/** A playmat and a backdrop, drawn together: the mat under the viewer's half, the backdrop over it, a lane of pads each side. */
function MiniBoard({ table, field }: { table: CosmeticId; field: CosmeticId }): ReactElement {
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

function TilePicture({ kind, id, equipped }: { kind: DeckSlotKind; id: CosmeticId; equipped: Record<DeckSlotKind, CosmeticId> }): ReactElement {
  if (kind === "cardBack") {
    return (
      <div className="collection-tile__back" aria-hidden="true">
        <CardBack back={id} />
      </div>
    );
  }
  return <MiniBoard table={kind === "table" ? id : equipped.table} field={kind === "field" ? id : equipped.field} />;
}

function TileRow({
  kind,
  hint,
  tiles,
  equipped,
}: {
  kind: DeckSlotKind;
  hint: string;
  tiles: AccountTile[];
  equipped: Record<DeckSlotKind, CosmeticId>;
}): ReactElement {
  return (
    <section className="collection-tiles" data-testid={T.tiles(kind)} aria-label={cosmeticKindWords(kind)}>
      <h4 className="collection-tiles__title">{cosmeticKindWords(kind)}</h4>
      <p className="settings-hint">{t(hint)}</p>
      <ul className="collection-tiles__list">
        {tiles.map(({ cosmetic, owned }) => {
          const worn = equipped[kind] === cosmetic.id;
          return (
            <li
              key={cosmetic.id}
              className="collection-tile"
              data-testid={T.tile(cosmetic.id)}
              data-owned={String(owned)}
              data-equipped={String(worn)}
              data-rarity={cosmetic.rarity}
            >
              <TilePicture kind={kind} id={cosmetic.id} equipped={equipped} />
              <span className="collection-tile__name">{t(cosmetic.name)}</span>
              <span className="collection-tile__rarity">{t(cosmetic.rarity)}</span>
              {owned ? null : <span className="collection-tile__lock">{t("Locked")}</span>}
              <button
                type="button"
                className="collection-tile__button"
                aria-label={t(cosmetic.name)}
                aria-pressed={worn}
                aria-disabled={owned ? undefined : true}
                onClick={() => {
                  if (owned) writeCosmetics({ [kind]: cosmetic.id });
                }}
              />
            </li>
          );
        })}
      </ul>
    </section>
  );
}

/**
 * R1297000, R1655000: the playmat, backdrop and card back your own side wears in a game. A deck's
 * own pick, set in the deck builder, beats these. `owned` is null until `GET /api/cosmetics` answers.
 */
export default function AccountCosmeticTiles({ owned }: { owned: OwnedCosmeticsResponse | null }): ReactElement {
  const picks = useCosmetics();
  const equipped: Record<DeckSlotKind, CosmeticId> = {
    table: resolveCosmetic("table", picks.table),
    field: resolveCosmetic("field", picks.field),
    cardBack: resolveCosmetic("cardBack", picks.cardBack),
  };
  return (
    <>
      {TILE_ROWS.map(({ kind, hint }) => (
        <TileRow key={kind} kind={kind} hint={hint} tiles={accountTiles(kind, owned)} equipped={equipped} />
      ))}
    </>
  );
}
