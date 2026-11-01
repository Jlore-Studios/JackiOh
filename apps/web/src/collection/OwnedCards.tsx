// The owned-cards panel (#1307, SPEC §14): a completion count per shipped set and, behind each set's
// toggle, its cards, owned as `GET /api/collection` says (CLAUDE.md rule 7). An unowned card is a
// locked silhouette that still opens its view (R1655000), so a player sees what they miss. R1953001:
// `token` lets that view offer a held token. R1956000: an owned tile wears the viewer's picks.

import { useId, useState, type ReactElement } from "react";

import { CATALOG } from "@jackioh/cards";
import type { CardDef, SetName } from "@jackioh/shared";

import { readerName } from "../cards/chinese.ts";
import { CardDefsProvider, CardFace, closeInspect, faceModel } from "../cards/index.ts";
import { formatNumber, t } from "../i18n/i18n.ts";
import type { CollectionResponse } from "../net/api.ts";
import { quantitiesOf } from "../shop/shop.ts";
import CardCosmeticsView from "./CardCosmeticsView.tsx";
import { COLLECTED_DEFS, collectionTestid, setCompletions } from "./collection.ts";

type Inspected = { def: CardDef; owned: boolean };

export default function OwnedCards({
  collection,
  defs = COLLECTED_DEFS,
  token = null,
}: {
  collection: CollectionResponse;
  defs?: readonly CardDef[];
  token?: string | null;
}): ReactElement {
  const [openSet, setOpenSet] = useState<SetName | null>(null);
  const [inspected, setInspected] = useState<Inspected | null>(null);
  const idBase = useId();

  return (
    <CardDefsProvider defs={CATALOG}>
      {setCompletions(collection, defs).map(({ set, owned, total, cards }) => {
        const open = openSet === set;
        const gridId = `${idBase}-${set}`;
        return (
          <div key={set} className="collection-set" data-testid={collectionTestid.set(set)}>
            <button
              type="button"
              className="collection-set__toggle"
              data-testid={collectionTestid.setToggle(set)}
              aria-expanded={open}
              aria-controls={gridId}
              onClick={() => {
                setOpenSet(open ? null : set);
              }}
            >
              <span>{t(set)}</span>
              <span data-testid={collectionTestid.completion(set)}>
                {t("{progress} of {target}", { progress: formatNumber(owned), target: formatNumber(total) })}
              </span>
            </button>
            {open ? (
              <ul id={gridId} className="collection-grid">
                {cards.map(({ def, owned: held }) => (
                  <li key={def.id}>
                    <button
                      type="button"
                      className="collection-card"
                      data-testid={collectionTestid.card(def.id)}
                      data-owned={String(held)}
                      aria-label={held ? readerName(def) : t("{name}, not owned yet", { name: readerName(def) })}
                      onClick={() => {
                        closeInspect();
                        setInspected({ def, owned: held });
                      }}
                    >
                      <span className="collection-card__face">
                        <CardFace face={faceModel({ defId: def.id, def, radiant: false })} layout="full" lazyArt worn={held} />
                      </span>
                      {held ? null : (
                        <span className="collection-card__lock" aria-hidden="true">
                          {t("Locked")}
                        </span>
                      )}
                    </button>
                  </li>
                ))}
              </ul>
            ) : null}
          </div>
        );
      })}
      {inspected === null ? null : (
        <CardCosmeticsView
          key={inspected.def.id}
          def={inspected.def}
          owned={inspected.owned}
          copies={quantitiesOf(collection).get(inspected.def.id) ?? 0}
          token={token}
          onClose={() => {
            setInspected(null);
          }}
        />
      )}
    </CardDefsProvider>
  );
}
