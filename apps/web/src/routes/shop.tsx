// `/shop`: the one place for spending and opening (#1577, #1580, SPEC §14.1, §14.3–§14.6, R1580000). Its
// four panels left the account screen, which links here and keeps account things only: the Jlore
// shop and its wallet, the card packs the account holds and the opening scene, the cosmetic packs with
// the Chase Shard bar, and the token picker. Their odds and the pity are `/purchases`', linked at the
// foot. Each panel holds its own reads, so when one changes the account the others read it again
// (`readAgain`): a pack bought in the shop can be opened at once, and an opening's bars, cards and
// cosmetics show in the picker.
//
// The client decides nothing (CLAUDE.md rule 7): every price, odds, pity counter and refusal is the
// server's, and a refused read says why in its panel. Gated like `/collection`: an anonymous visitor
// goes to `/login` (main.tsx).
//
// The page never scrolls: the board shrinks and the body scrolls inside it, as `/collection`'s does
// (R1579000). The opening scene is the exception to the frame: it plays full screen, over everything.

import { useState, type ReactElement } from "react";

import PackOpening from "../packs/PackOpening.tsx";
import CosmeticPacks from "../shop/CosmeticPacks.tsx";
import Shop from "../shop/Shop.tsx";
import TokenPicker from "../shop/TokenPicker.tsx";
import { shopTestid } from "../shop/shop.ts";
import { t } from "../i18n/i18n.ts";
import { paths } from "../net/navigate.ts";
import { BackLink, followInApp } from "./nav.tsx";

import "../auth/tavern.css";
import "./account.css";
import "./purchases.css";
import "../shop/shop.css";

export const shopRouteTestid = {
  purchases: "shop-purchases-link",
} as const;

/** The panels on the screen, each reading its own part of the account. */
type ShopPanel = "opening" | "shop" | "tokens" | "cosmetics";

/** How many times each panel has been told to read again. */
type PanelReads = Record<ShopPanel, number>;

/**
 * The reads after `by` changed the account (a pack bought or opened, a token redeemed, a chase
 * bought): every other panel reads again, since a buy fills the opening's packs and an opening
 * fills the bars, the collection and the cosmetics. `by` already holds its answer.
 */
function readAgain(reads: PanelReads, by: ShopPanel): PanelReads {
  return {
    opening: reads.opening + (by === "opening" ? 0 : 1),
    shop: reads.shop + (by === "shop" ? 0 : 1),
    tokens: reads.tokens + (by === "tokens" ? 0 : 1),
    cosmetics: reads.cosmetics + (by === "cosmetics" ? 0 : 1),
  };
}

export default function ShopRoute({ token }: { token: string }): ReactElement {
  const [reads, setReads] = useState<PanelReads>({ opening: 0, shop: 0, tokens: 0, cosmetics: 0 });
  const changedBy = (by: ShopPanel) => (): void => {
    setReads((before) => readAgain(before, by));
  };
  return (
    <div className="app-shell tavern shop-screen" data-testid={shopTestid.screen}>
      <BackLink />
      <section className="panel panel--auth">
        <div className="brand">
          <h1>JackiOh</h1>
        </div>
        <h2>{t("Shop")}</h2>
        <div className="shop-body" data-testid={shopTestid.body}>
          <Shop token={token} revision={reads.shop} onChanged={changedBy("shop")} />
          <PackOpening token={token} revision={reads.opening} onChanged={changedBy("opening")} />
          <CosmeticPacks token={token} revision={reads.cosmetics} onChanged={changedBy("cosmetics")} />
          <TokenPicker token={token} revision={reads.tokens} onChanged={changedBy("tokens")} />
        </div>
        {/* R1665000: the odds and the pity are on their own page, not in the panels above. */}
        <p className="shop-purchases-link">
          <a href={paths.purchases} data-testid={shopRouteTestid.purchases} onClick={followInApp(paths.purchases)}>
            {t("Odds & pity")}
          </a>
        </p>
      </section>
    </div>
  );
}
