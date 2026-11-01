// The Marketplace (R1853000–R1853003, R2250002, SPEC §10.10): Stinkyscape's shop, open for the rest of
// the game once either player gains a Resource or Credits.
//
// A toggle opens a panel that draws `view.marketplace` and `legal` alone: the viewer's Credits, the
// offers rolled for them at the start of their turn (R2250002), each on the face it is sold on with its
// price and the copies the viewer may still buy, and each card in their hand that sells from there
// (R1861000) with its quote. "Buy" is
// live exactly when `legal` lists that lot's `marketBuy`, and a sale exactly when `legal` lists its
// `marketSell`; each sends the listed body. Otherwise the button is greyed, a lot's tooltip the
// engine's reason (`reason`), and sends nothing. Every number shown is the view's: nothing here
// prices, filters or refuses (CLAUDE.md rule 7). The view carries nothing while the Marketplace is
// closed, and then nothing is drawn. Escape and Back (R2127000) close the panel.
//
// The toggle is a button of the control bar (Board.tsx, #2645), which owns whether the panel is open
// (the match menu and the shop are never open together): an icon on a phone, the pill's label where the
// board has its sidebar (market.css). The panel opens upward from the bar's top edge.

import { useCallback, useId, useRef, type ReactElement } from "react";

import type { ActionBody, CardDef, MarketOfferView, MarketSaleView, PlayerView } from "@jackioh/shared";

import { chineseName, drawnInChinese } from "../cards/chinese.ts";
import { CardFace, faceModel, useInspectTrigger } from "../cards/index.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { BackLayer } from "../net/backLayers.ts";
import { useCardInfo } from "./catalog.ts";
import { testid } from "./contract.ts";
import "./market.css";

type MarketplaceProps = {
  view: PlayerView;
  legal: readonly ActionBody[];
  onAction: (action: ActionBody) => void;
  /** Whether the panel is open, which the board holds (#2645). */
  open: boolean;
  onOpenChange: (open: boolean) => void;
};

type MarketBuy = Extract<ActionBody, { type: "marketBuy" }>;
type MarketSell = Extract<ActionBody, { type: "marketSell" }>;

/** A card's name as the reader reads it (R711000): the Chinese table's for a Chinese reader. */
function readerName(defId: string, english: string, def: CardDef | undefined): string {
  return drawnInChinese(false) ? chineseName(defId, english, def) : english;
}

/**
 * Like the night market's lot (MarketPrompt.tsx), the face opens the hover preview and, on a phone,
 * the long-press sheet, so a lot can be read before it is bought; "Buy" is its own button, so a greyed
 * one never stops the face from being read.
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

/** One sale: a card in the viewer's hand that sells from there (R1861000), for this turn's quote. */
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

export default function Marketplace({ view, legal, onAction, open, onOpenChange }: MarketplaceProps): ReactElement | null {
  useLocale();
  const panelId = useId();
  const toggle = useRef<HTMLButtonElement>(null);
  const panel = useRef<HTMLElement>(null);
  // Escape and Back (R2127000) close the shop and give the focus back to its toggle, unless the
  // player put it elsewhere (a prompt drawn over the shop keeps it), as the portrait picker does (#1569).
  const close = useCallback((): void => {
    const focused = document.activeElement;
    const inside = focused === null || focused === document.body || panel.current?.contains(focused) === true;
    onOpenChange(false);
    if (inside) toggle.current?.focus({ preventScroll: true });
  }, [onOpenChange]);
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
        ref={toggle}
        type="button"
        className="marketplace-toggle"
        data-testid={testid.marketplace}
        aria-expanded={open}
        aria-controls={panelId}
        aria-label={t("Marketplace")}
        title={t("Marketplace")}
        onClick={() => {
          onOpenChange(!open);
        }}
      >
        <span className="marketplace-toggle-icon" aria-hidden="true">
          <svg viewBox="0 0 24 24" width="20" height="20" focusable="false">
            <circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" strokeWidth="2" />
            <circle cx="12" cy="12" r="4" fill="currentColor" />
          </svg>
        </span>
        <span className="marketplace-toggle-label">{t("Marketplace")}</span>
      </button>
      {open ? (
        <section
          ref={panel}
          className="marketplace-panel"
          id={panelId}
          data-testid={testid.marketplacePanel}
          aria-label={t("Marketplace")}
          onKeyDown={(event) => {
            if (event.key === "Escape") close();
          }}
        >
          <BackLayer onClose={close} />
          <header className="marketplace-header">
            <h2 className="market-title">{t("Marketplace")}</h2>
            {/* R1840005: the view's Credits are absent until first gained: none yet. */}
            <p className="market-purse" role="status">
              {t("{n} Credits", { n: view.you.marketCredits ?? 0 })}
            </p>
            <p className="marketplace-restock">{t("New offers at the start of each of your turns")}</p>
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
            <section className="marketplace-sales" aria-label={t("Sell")}>
              <h3>{t("Sell")}</h3>
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
