// The wallet panel (#1307, SPEC §14.1, §14.3): the Jlore balance and the unopened packs, card and
// cosmetic, as `GET /api/wallet`, `/api/packs` and `/api/cosmetic-packs` answer them (CLAUDE.md
// rule 7). Tokens stay in the token picker on `/shop`.

import type { ReactElement } from "react";

import { formatNumber, t } from "../i18n/i18n.ts";
import type { CosmeticPacksResponse, PacksResponse, WalletResponse } from "../net/api.ts";
import { packName } from "../packs/packs.ts";
import { collectionTestid } from "./collection.ts";

export default function WalletPanel({
  wallet,
  packs,
  cosmeticPacks,
}: {
  wallet: WalletResponse;
  packs: PacksResponse;
  cosmeticPacks: CosmeticPacksResponse;
}): ReactElement {
  const none = packs.unopened.length === 0 && cosmeticPacks.unopened <= 0;
  return (
    <>
      <p data-testid={collectionTestid.jlore}>{t("{jlore} Jlore", { jlore: formatNumber(wallet.jlore) })}</p>
      {none ? (
        <p data-testid={collectionTestid.noPacks}>{t("No unopened packs.")}</p>
      ) : (
        <ul className="collection-list">
          {packs.unopened.map(({ pack, count }) => (
            <li key={pack} data-testid={collectionTestid.packs(pack)}>
              {t("{pack}: {count} unopened", { pack: packName(pack), count: formatNumber(count) })}
            </li>
          ))}
          {cosmeticPacks.unopened > 0 ? (
            <li data-testid={collectionTestid.cosmeticPacks}>
              {t("Cosmetic packs: {count} unopened", { count: formatNumber(cosmeticPacks.unopened) })}
            </li>
          ) : null}
        </ul>
      )}
    </>
  );
}
