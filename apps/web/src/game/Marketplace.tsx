// The Marketplace (R1853000–R1853003, SPEC §10.10): Stinkyscape's shop, open for the rest of the game
// once either player gains a Resource or Credits.
//
// A toggle opens a panel that draws `view.marketplace` and `legal` alone: the viewer's Credits, the
// turn the offering next re-rolls on, each lot on the face it is sold on with its price and the copies
// the viewer may still buy, and each priced Resource in the viewer's own tray with its quote. "Buy" is
// live exactly when `legal` lists that lot's `marketBuy`, and a sale exactly when `legal` lists its
// `marketSell`; each sends the listed body. Otherwise the button is greyed, a lot's tooltip the
// engine's reason (`reason`), and sends nothing. Every number shown is the view's: nothing here
// prices, filters or refuses (CLAUDE.md rule 7). The view carries nothing while the Marketplace is
// closed, and then nothing is drawn.

import { useId, useState, type ReactElement } from "react";

import type { ActionBody, CardDef, MarketOfferView, MarketSaleView, PlayerView } from "@jackioh/shared";

import { chineseName, drawnInChinese } from "../cards/chinese.ts";
import { CardFace, faceModel, useInspectTrigger } from "../cards/index.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { useCardInfo } from "./catalog.ts";
import { testid } from "./contract.ts";
import "./market.css";

type MarketplaceProps = {
  view: PlayerView;
  legal: readonly ActionBody[];
  onAction: (action: ActionBody) => void;
};

type MarketBuy = Extract<ActionBody, { type: "marketBuy" }>;
type MarketSell = Extract<ActionBody, { type: "marketSell" }>;

/** A card's name as the reader reads it (R711000): the Chinese table's for a Chinese reader. */
function readerName(defId: string, english: string, def: CardDef | undefined): string {
  return drawnInChinese(false) ? chineseName(defId, english, def) : english;
}

/**
 * One lot: the card as it will arrive (on the face it is sold on), its name, its price and the copies
 * left, and "Buy". Like the night market's lot (MarketPrompt.tsx), the face opens the hover preview
 * and, on a phone, the long-press sheet, so a lot can be read before it is bought; "Buy" is its own
 * button, so a greyed one never stops the face from being read.
 */
function Offer(props: { offer: MarketOfferView; listed: MarketBuy | undefined; onAction: (action: ActionBody) => void }) {
  useLocale();
  const { offer, listed } = props;
  const radiant = offer.radiant === true;
  const info = useCardInfo(offer.defId, radiant);
  const face = faceModel({ defId: offer.defId, def: info.def, name: info.name, radiant, chinese: false, inPlay: {} });
  const name = readerName(offer.defId, info.name, info.def);
  // Lines of code is a hidden stat in matches.
  const inspect = useInspectTrigger({ key: `marketplace-offer-${String(offer.offer)}`, face }, { prefer: "above", showLoc: false });
  return (
    <li className="marketplace-offer" data-offer={offer.offer} data-fixed={offer.fixed === true ? "true" : undefined}>
      <span className="marketplace-face" {...inspect.handlers}>
        <CardFace face={face} layout="full" />
      </span>
      {inspect.overlay}
      <span className="market-caption">{name}</span>
      <span className="market-price">{t("{n} Credits", { n: offer.price })}</span>
      <span className="marketplace-left">{t("{left} left", { left: offer.left })}</span>
      <button
        type="button"
        className="marketplace-deal"
        data-testid={testid.marketBuy(offer.offer)}
        data-legal={listed === undefined ? "false" : "true"}
        disabled={listed === undefined}
        title={listed === undefined && offer.reason !== undefined ? t(offer.reason) : undefined}
        aria-label={t("Buy {name} for {n} Credits", { name, n: offer.price })}
        onClick={() => {
          if (listed !== undefined) props.onAction(listed);
        }}
      >
        {t("Buy")}
      </button>
    </li>
  );
}

/** One sale: a Resource in the viewer's own tray, for this turn's quote. */
function Sale(props: { sale: MarketSaleView; listed: MarketSell | undefined; onAction: (action: ActionBody) => void }) {
  useLocale();
  const { sale, listed } = props;
  const info = useCardInfo(sale.defId, sale.radiant === true);
  const name = readerName(sale.defId, info.name, info.def);
  return (
    <li>
      <button
        type="button"
        className="marketplace-deal"
        data-testid={testid.marketSell(sale.instanceId)}
        data-legal={listed === undefined ? "false" : "true"}
        disabled={listed === undefined}
        onClick={() => {
          if (listed !== undefined) props.onAction(listed);
        }}
      >
        {t("Sell {name} for {n} Credits", { name, n: sale.credits })}
      </button>
    </li>
  );
}

export default function Marketplace({ view, legal, onAction }: MarketplaceProps): ReactElement | null {
  useLocale();
  const [open, setOpen] = useState(false);
  const panelId = useId();
  const market = view.marketplace;
  if (market === undefined) return null;

  const buys = new Map<number, MarketBuy>();
  const sells = new Map<string, MarketSell>();
  for (const body of legal) {
    if (body.type === "marketBuy") buys.set(body.offer, body);
    if (body.type === "marketSell") sells.set(body.instanceId, body);
  }

  return (
    <>
      <button
        type="button"
        className="marketplace-toggle"
        data-testid={testid.marketplace}
        aria-expanded={open}
        aria-controls={panelId}
        onClick={() => {
          setOpen((was) => !was);
        }}
      >
        {t("Marketplace")}
      </button>
      {open ? (
        <section
          className="marketplace-panel"
          id={panelId}
          data-testid={testid.marketplacePanel}
          aria-label={t("Marketplace")}
          onKeyDown={(event) => {
            if (event.key === "Escape") setOpen(false);
          }}
        >
          <header className="marketplace-header">
            <h2 className="market-title">{t("Marketplace")}</h2>
            {/* R1840005: the view's Credits are absent until first gained: none yet. */}
            <p className="market-purse" role="status">
              {t("{n} Credits", { n: view.you.marketCredits ?? 0 })}
            </p>
            <p className="marketplace-restock">{t("New stock on turn {turn}", { turn: market.rerollsOn })}</p>
          </header>
          {market.offers.length === 0 ? (
            <p className="market-empty">{t("Nothing on offer")}</p>
          ) : (
            <ul className="marketplace-offers">
              {market.offers.map((offer) => (
                <Offer key={offer.offer} offer={offer} listed={buys.get(offer.offer)} onAction={onAction} />
              ))}
            </ul>
          )}
          {market.sales.length === 0 ? null : (
            <section className="marketplace-sales" aria-label={t("Resources")}>
              <h3>{t("Resources")}</h3>
              <ul className="market-barter-list">
                {market.sales.map((sale) => (
                  <Sale key={sale.instanceId} sale={sale} listed={sells.get(sale.instanceId)} onAction={onAction} />
                ))}
              </ul>
            </section>
          )}
        </section>
      ) : null}
    </>
  );
}
