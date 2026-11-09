// The account's cosmetics as preview tiles (#1662, R1655000, R1579000, part of #1651): the playmat,
// backdrop and card back your own side wears. An owned tile equips through `writeCosmetics` (R633)
// with no request; ownership is `GET /api/cosmetics` through main.tsx's `AccountCosmeticsOwned`
// (R1833000), else every tile is owned. R1953001, R2730000: a locked tile a held token covers offers
// it in its fullscreen preview (a hold or the inspect key opens it), and nowhere else. This decides
// nothing (CLAUDE.md rule 7). Particles move only on the equipped, hovered or focused tile.

import { useState, type ReactElement } from "react";

import { CardBack, INSPECT_KEY_SHORTCUT } from "../cards/index.ts";
import {
  DEFAULT_CARD_COSMETICS,
  resolveCosmetic,
  useCosmetics,
  useOwnsAccountCosmetic,
  writeCosmetics,
  type CosmeticId,
  type DeckSlotKind,
} from "../cards/cosmetics.ts";
import { t } from "../i18n/i18n.ts";
import { accountTiles, collectionTestid as T, cosmeticKindWords, earnWords, type AccountTile } from "./collection.ts";
import { MiniBoard } from "./CosmeticLook.tsx";
import ItemPreview, { usePreviewHold } from "./ItemPreview.tsx";
import TokenOffer, { offerTier, useHeldTokens } from "./TokenOffer.tsx";

import "../game/table.css";
import "../settings/settings.css";

const TILE_ROWS: readonly { kind: DeckSlotKind; hint: string }[] = [
  { kind: "table", hint: "The playmat on your side. A deck can pick its own." },
  { kind: "field", hint: "The backdrop behind your side. Reduce motion holds it still." },
  { kind: "cardBack", hint: "The back of your own face-down cards and your deck." },
];

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

/** R2730000: one tile: a hold or the inspect key previews it fullscreen, a click equips it. */
function Tile({
  kind,
  tile,
  equipped,
  onPreview,
}: {
  kind: DeckSlotKind;
  tile: AccountTile;
  equipped: Record<DeckSlotKind, CosmeticId>;
  onPreview: (id: CosmeticId) => void;
}): ReactElement {
  const { cosmetic, owned } = tile;
  const worn = equipped[kind] === cosmetic.id;
  const hold = usePreviewHold(`tile-${cosmetic.id}`, () => {
    onPreview(cosmetic.id);
  });
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
      {/* R3335002: an earned-only one says how it is earned (its Fleet track level), and no token covers it. */}
      {owned ? null : <span className="collection-tile__lock">{earnWords(cosmetic) ?? t("Locked")}</span>}
      <button
        type="button"
        className="collection-tile__button"
        aria-label={t(cosmetic.name)}
        aria-pressed={worn}
        aria-disabled={owned ? undefined : true}
        aria-keyshortcuts={INSPECT_KEY_SHORTCUT}
        {...hold}
        onClick={() => {
          if (owned) writeCosmetics({ [kind]: cosmetic.id });
        }}
      />
    </li>
  );
}

function TileRow({
  kind,
  hint,
  tiles,
  equipped,
  onPreview,
}: {
  kind: DeckSlotKind;
  hint: string;
  tiles: AccountTile[];
  equipped: Record<DeckSlotKind, CosmeticId>;
  onPreview: (id: CosmeticId) => void;
}): ReactElement {
  return (
    <section className="collection-tiles" data-testid={T.tiles(kind)} aria-label={cosmeticKindWords(kind)}>
      <h3 className="collection-tiles__title">{cosmeticKindWords(kind)}</h3>
      <p className="settings-hint">{t(hint)}</p>
      <ul className="collection-tiles__list">
        {tiles.map((tile) => (
          <Tile key={tile.cosmetic.id} kind={kind} tile={tile} equipped={equipped} onPreview={onPreview} />
        ))}
      </ul>
    </section>
  );
}

/**
 * R1297000, R1655000: the account's picks, which a deck's own pick beats. R1833000: an unowned pick
 * is equipped as its kind's default. R1953001: `token` reads the held tokens a locked tile offers.
 */
export default function AccountCosmeticTiles({ token = null }: { token?: string | null }): ReactElement {
  const picks = useCosmetics();
  const owns = useOwnsAccountCosmetic();
  const rows = TILE_ROWS.map((row) => ({ ...row, tiles: accountTiles(row.kind, owns) }));
  // The held tokens are read only while a tile is locked, the only place they are offered.
  const [held, setHeld] = useHeldTokens(rows.some(({ tiles }) => tiles.some(({ owned }) => !owned)) ? token : null);
  const [previewing, setPreviewing] = useState<CosmeticId | null>(null);
  const wornOf = (kind: DeckSlotKind): CosmeticId => {
    const id = resolveCosmetic(kind, picks[kind]);
    return owns(id) ? id : DEFAULT_CARD_COSMETICS[kind];
  };
  const equipped: Record<DeckSlotKind, CosmeticId> = { table: wornOf("table"), field: wornOf("field"), cardBack: wornOf("cardBack") };
  // Looked up on every render, so the preview's owned state is always current.
  const shown = rows.flatMap(({ kind, tiles }) => tiles.map((tile) => ({ kind, ...tile }))).find((tile) => tile.cosmetic.id === previewing) ?? null;
  const shownTier = shown === null || shown.owned ? null : offerTier(shown.cosmetic, held);
  return (
    <>
      {rows.map(({ kind, hint, tiles }) => (
        <TileRow key={kind} kind={kind} hint={hint} tiles={tiles} equipped={equipped} onPreview={setPreviewing} />
      ))}
      {shown === null ? null : (
        <ItemPreview
          id={shown.cosmetic.id}
          name={t(shown.cosmetic.name)}
          rarity={shown.cosmetic.rarity}
          owned={shown.owned}
          picture={<TilePicture kind={shown.kind} id={shown.cosmetic.id} equipped={equipped} />}
          offer={
            shown.owned || token === null || held === null || shownTier === null ? null : (
              <TokenOffer
                key={shown.cosmetic.id}
                token={token}
                spend={{ kind: "cosmetic", tier: shownTier, cosmeticId: shown.cosmetic.id }}
                held={held}
                name={t(shown.cosmetic.name)}
                onRedeemed={setHeld}
              />
            )
          }
          onClose={() => {
            setPreviewing(null);
          }}
        />
      )}
    </>
  );
}
