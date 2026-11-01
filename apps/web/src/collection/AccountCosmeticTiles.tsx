// The account's cosmetics as preview tiles (#1662, R1655000, R1579000, part of #1651): the playmat,
// backdrop and card back your own side wears. An owned tile equips through `writeCosmetics` (R633)
// with no request; ownership is `GET /api/cosmetics` through main.tsx's `AccountCosmeticsOwned`
// (R1833000), else every tile is owned; a locked one offers a held token that covers it (R1953001).
// This decides nothing (CLAUDE.md rule 7). Particles move only on the equipped, hovered or focused tile.

import type { ReactElement } from "react";

import { CardBack } from "../cards/index.ts";
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
import { accountTiles, collectionTestid as T, cosmeticKindWords, type AccountTile } from "./collection.ts";
import { MiniBoard } from "./CosmeticLook.tsx";
import TokenOffer, { offerTier, useHeldTokens, type HeldTokens } from "./TokenOffer.tsx";

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

type Offers = { token: string | null; held: HeldTokens | null; onRedeemed: (held: HeldTokens) => void };

function TileRow({
  kind,
  hint,
  tiles,
  equipped,
  offers,
}: {
  kind: DeckSlotKind;
  hint: string;
  tiles: AccountTile[];
  equipped: Record<DeckSlotKind, CosmeticId>;
  offers: Offers;
}): ReactElement {
  return (
    <section className="collection-tiles" data-testid={T.tiles(kind)} aria-label={cosmeticKindWords(kind)}>
      <h4 className="collection-tiles__title">{cosmeticKindWords(kind)}</h4>
      <p className="settings-hint">{t(hint)}</p>
      <ul className="collection-tiles__list">
        {tiles.map(({ cosmetic, owned }) => {
          const worn = equipped[kind] === cosmetic.id;
          const tier = owned ? null : offerTier(cosmetic, offers.held);
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
              {tier === null || offers.token === null || offers.held === null ? null : (
                <TokenOffer
                  token={offers.token}
                  tier={tier}
                  held={offers.held}
                  cosmeticId={cosmetic.id}
                  name={t(cosmetic.name)}
                  onRedeemed={offers.onRedeemed}
                  testid={T.tokenOffer(cosmetic.id)}
                  errorTestid={T.tokenOfferError(cosmetic.id)}
                />
              )}
            </li>
          );
        })}
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
  const offers: Offers = { token, held, onRedeemed: setHeld };
  const wornOf = (kind: DeckSlotKind): CosmeticId => {
    const id = resolveCosmetic(kind, picks[kind]);
    return owns(id) ? id : DEFAULT_CARD_COSMETICS[kind];
  };
  const equipped: Record<DeckSlotKind, CosmeticId> = { table: wornOf("table"), field: wornOf("field"), cardBack: wornOf("cardBack") };
  return (
    <>
      {rows.map(({ kind, hint, tiles }) => (
        <TileRow key={kind} kind={kind} hint={hint} tiles={tiles} equipped={equipped} offers={offers} />
      ))}
    </>
  );
}
