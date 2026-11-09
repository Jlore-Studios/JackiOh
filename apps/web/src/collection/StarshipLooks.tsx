// R3335000: the account's Starship looks as preview tiles: the look every Starship it launches wears, then a row per
// kind (hull design, livery, engine trail, piece skin), each tile the equipped look with its own pick in place. An owned
// tile equips through `writeCosmetics` (R633) with no request, as AccountCosmeticTiles' do; ownership is
// `useOwnsAccountCosmetic`'s (R1833000), every tile owned with none above it. R3335002: every Starship cosmetic but the
// standards is earned only, so a locked tile says how it is earned (its Fleet track level, R3335001, or reaching
// Jlorious, R3335003) and offers no token. This decides nothing (CLAUDE.md rule 7).

import type { ReactElement } from "react";

import {
  STARSHIP_SLOT_KINDS,
  useCosmetics,
  useOwnsAccountCosmetic,
  wornStarshipLook,
  writeCosmetics,
  type StarshipLook,
  type StarshipSlotKind,
} from "../cards/cosmetics.ts";
import { t } from "../i18n/i18n.ts";
import { accountTiles, collectionTestid as T, cosmeticKindWords, earnWords, type AccountTile } from "./collection.ts";
import StarshipPreview from "./StarshipPreview.tsx";

import "../settings/settings.css";

/** Each row's hint; each text is its own literal, so the translation guard sees it. */
function rowHint(kind: StarshipSlotKind): string {
  switch (kind) {
    case "starshipHull":
      return t("The shape of every Starship you launch, in place of the one its name gives it.");
    case "starshipLivery":
      return t("The colors of its hull.");
    case "starshipTrail":
      return t("The glow of its engines and the trail it leaves.");
    case "starshipPieceSkin":
      return t("The colors of every module it mounts.");
  }
}

function StarshipTile({ kind, tile, equipped }: { kind: StarshipSlotKind; tile: AccountTile; equipped: StarshipLook }): ReactElement {
  const { cosmetic, owned } = tile;
  const worn = equipped[kind] === cosmetic.id;
  return (
    <li
      className="collection-tile collection-tile--starship"
      data-testid={T.tile(cosmetic.id)}
      data-owned={String(owned)}
      data-equipped={String(worn)}
      data-rarity={cosmetic.rarity}
    >
      <div className="collection-tile__starship" aria-hidden="true">
        <StarshipPreview look={{ ...equipped, [kind]: cosmetic.id }} />
      </div>
      <span className="collection-tile__name">{t(cosmetic.name)}</span>
      <span className="collection-tile__rarity">{t(cosmetic.rarity)}</span>
      {owned ? null : (
        <span className="collection-tile__lock" data-testid={T.tileEarn(cosmetic.id)}>
          {earnWords(cosmetic) ?? t("Locked")}
        </span>
      )}
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
}

/** R3335000: the equipped Starship look and every kind's tiles; an unowned pick is equipped as its kind's default (R1833000). */
export default function StarshipLooks(): ReactElement {
  const picks = useCosmetics();
  const owns = useOwnsAccountCosmetic();
  const equipped = wornStarshipLook(true, picks, null, owns);
  return (
    <div className="collection-starship-looks" data-testid={T.starshipLooks}>
      <div className="collection-starship-equipped" data-testid={T.starshipEquipped}>
        <StarshipPreview look={equipped} />
      </div>
      {STARSHIP_SLOT_KINDS.map((kind) => (
        <section key={kind} className="collection-tiles" data-testid={T.tiles(kind)} aria-label={cosmeticKindWords(kind)}>
          <h3 className="collection-tiles__title">{cosmeticKindWords(kind)}</h3>
          <p className="settings-hint">{rowHint(kind)}</p>
          <ul className="collection-tiles__list">
            {accountTiles(kind, owns).map((tile) => (
              <StarshipTile key={tile.cosmetic.id} kind={kind} tile={tile} equipped={equipped} />
            ))}
          </ul>
        </section>
      ))}
    </div>
  );
}
