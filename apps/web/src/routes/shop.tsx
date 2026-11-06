// `/shop`: the one place for spending and opening (#1577, #1580, SPEC §14.1, §14.3–§14.6, R1580000). Each
// panel holds its own reads, and each spend's answer folds into the panels that show it: a buy's into the
// wallet and the held packs, an opening's into the shelf's count, the pity and the token bars (R3211000).
// What no answer carries is read once, when the opening scene closes, not once per spend. The
// client decides nothing (CLAUDE.md rule 7): a refused read says why in its panel. Gated like
// `/collection` (main.tsx). The page never scrolls: the body scrolls inside the board, as `/collection`'s
// does (R1579000); the opening scene plays full screen, over everything.

import { useEffect, useRef, useState, type ReactElement } from "react";

import { forgetOwnedCosmetics } from "../cards/ownedFullArt.ts";
import type { BoughtPackResponse, OpenedCosmeticPackResponse, OpenedPackResponse, OpeningBarsAnswer } from "../net/api.ts";
import PackOpening from "../packs/PackOpening.tsx";
import CosmeticPacks from "../shop/CosmeticPacks.tsx";
import Shop from "../shop/Shop.tsx";
import TokenPicker from "../shop/TokenPicker.tsx";
import { shopTestid } from "../shop/shop.ts";
import { t } from "../i18n/i18n.ts";
import { paths } from "../net/navigate.ts";
import { BackLink, followInApp } from "./nav.tsx";
import { Wordmark } from "./Wordmark.tsx";

import "../auth/tavern.css";
import "./account.css";
import "./purchases.css";
import "../shop/shop.css";

export const shopRouteTestid = {
  purchases: "shop-purchases-link",
} as const;

export default function ShopRoute({ token }: { token: string }): ReactElement {
  // R3211000: the last answer of each spend, held for the panels that show it.
  const [bought, setBought] = useState<BoughtPackResponse | null>(null);
  const [opened, setOpened] = useState<OpenedPackResponse | null>(null);
  const [bars, setBars] = useState<OpeningBarsAnswer | null>(null);
  // How many times the tokens, and the cosmetic packs, were told to read what the answers did not carry.
  const [tokenReads, setTokenReads] = useState(0);
  const [settled, setSettled] = useState(0);
  // Whether a card pack was opened and its scene has not closed yet: leaving then still owes the ledger its read.
  const unsettled = useRef(false);
  useEffect(
    () => () => {
      if (unsettled.current) forgetOwnedCosmetics();
    },
    [],
  );
  const onOpened = (answer: OpenedPackResponse): void => {
    setOpened(answer);
    setBars(answer.bars);
    unsettled.current = true;
  };
  // The one batch of reads, when the scene closes, whatever it opened: what no answer carries (the held
  // tokens, the cosmetic packs and the chase choice's inputs), and the account's owned cosmetics, which
  // an opening may have granted, per-card or account.
  const onSceneClosed = (): void => {
    unsettled.current = false;
    forgetOwnedCosmetics();
    setTokenReads((before) => before + 1);
    setSettled((before) => before + 1);
  };
  // A cosmetic pack has no scene: its answer folds its bars, and the tokens it paid are read once after it.
  const onCosmeticChanged = (answer?: OpenedCosmeticPackResponse): void => {
    forgetOwnedCosmetics();
    if (answer === undefined) return;
    setBars(answer.bars);
    setTokenReads((before) => before + 1);
  };
  return (
    <div className="app-shell tavern shop-screen" data-testid={shopTestid.screen}>
      <BackLink title={t("Shop")} />
      <section className="panel panel--auth">
        <Wordmark />
        <div className="shop-body" data-testid={shopTestid.body}>
          <Shop token={token} opened={opened} onChanged={setBought} />
          <PackOpening token={token} bought={bought} onChanged={onOpened} onClosed={onSceneClosed} />
          <CosmeticPacks token={token} settled={settled} onChanged={onCosmeticChanged} />
          <TokenPicker token={token} revision={tokenReads} opening={bars} />
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
