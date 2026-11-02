// The Trade control (R1838000, SPEC §10.10): the Duos pass control borrowed for a Tradable hand card.
//
// Pointing at one of the viewer's own hand cards puts a small bar above it, as the Duos card bar does
// (`duos/CardActions.tsx`, the same `game/cardBar.ts` underneath). It is offered only on a card
// `legal` lists a `trade` for, or whose view carries a `tradeReason`; it is live exactly when `legal`
// lists that card's `trade`, and otherwise greyed, its tooltip the engine's reason, sending nothing
// (CLAUDE.md rule 7: no keyword, cost or count is read here to decide anything). On a Duos board the
// Trade button rides the card bar beside the pass control instead, and Game's own bar is off (`tradeBar`).

import { useMemo, type ReactElement } from "react";

import type { ActionBody, PlayerView } from "@jackioh/shared";

import { DUOS_HOVER_GRACE_MS } from "../duos/config.ts";
import { spotTestid } from "../duos/spots.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { useBoxes, useCardBarFocus } from "./cardBar.ts";
import { testid } from "./contract.ts";
import "./trade.css";

export type TradeOffer = {
  /** The hand cards `legal` lists a `trade` for, by instance id: only these are live. */
  legal: ReadonlySet<string>;
  /** The engine's refusal for each Tradable hand card it lists no `trade` for (`tradeReason`). */
  reasons: ReadonlyMap<string, string>;
  /** The Trades left this turn, as the view says it. */
  left: number;
};

/** The Trade offer the view and `legal` make, or null while the viewer holds no Tradable card. */
export function tradeOfferFor(view: PlayerView, legal: readonly ActionBody[]): TradeOffer | null {
  const trades = view.trades;
  if (trades === undefined) return null;
  const listed = new Set<string>();
  for (const body of legal) {
    if (body.type === "trade") listed.add(body.instanceId);
  }
  const reasons = new Map<string, string>();
  for (const card of Array.isArray(view.you.hand) ? view.you.hand : []) {
    if (card.tradeReason !== undefined) reasons.set(card.instanceId, card.tradeReason);
  }
  if (listed.size === 0 && reasons.size === 0) return null;
  return { legal: listed, reasons, left: trades.left };
}

/** Whether the offer puts a Trade control on this hand card, live or greyed. */
export function offersTrade(offer: TradeOffer, instanceId: string): boolean {
  return offer.legal.has(instanceId) || offer.reasons.has(instanceId);
}

export type TradeButtonProps = {
  offer: TradeOffer;
  instanceId: string;
  onTrade: (instanceId: string) => void;
};

/** "Trade" on one hand card: the Duos pass control's button (`duos-pass`), and `trade.css`'s copy of it. */
export function TradeButton({ offer, instanceId, onTrade }: TradeButtonProps): ReactElement {
  useLocale();
  const live = offer.legal.has(instanceId);
  const reason = offer.reasons.get(instanceId);
  return (
    <button
      type="button"
      className="duos-pass trade-control"
      data-testid={testid.trade(instanceId)}
      data-instance-id={instanceId}
      data-legal={live ? "true" : "false"}
      disabled={!live}
      title={live || reason === undefined ? undefined : t(reason)}
      onClick={() => {
        if (!live) return;
        onTrade(instanceId);
      }}
    >
      <span className="duos-pass__label">{t("Trade")}</span>
      <span className="duos-pass__meta">{t("{left} Trades left", { left: offer.left })}</span>
    </button>
  );
}

/** The class the 1v1 bar wears, which `cardBar.ts` keeps open while the pointer is on it. */
const TRADE_BAR_CLASS = "trade-bar";

export type TradeBarProps = {
  /** The Game's root element, once it is mounted (an element, not a ref: see CardActions.tsx). */
  container: HTMLDivElement | null;
  view: PlayerView;
  legal: readonly ActionBody[];
  onTrade: (body: ActionBody) => void;
};

/** The 1v1 board's Trade bar: over the pointed-at hand card while the offer puts a control on it. */
export function TradeBar({ container, view, legal, onTrade }: TradeBarProps): ReactElement | null {
  const offer = useMemo(() => tradeOfferFor(view, legal), [view, legal]);
  const { focus, close } = useCardBarFocus(container, TRADE_BAR_CLASS, DUOS_HOVER_GRACE_MS);
  const card = offer !== null && focus?.at === "hand" && offersTrade(offer, focus.instanceId) ? focus.instanceId : null;
  const boxes = useBoxes(container, card === null ? "" : testid.handCard(card), "viewport");
  if (offer === null || card === null || focus === null) return null;
  const box = boxes.get(spotTestid(focus));
  if (box === undefined) return null;
  return (
    <div
      className={TRADE_BAR_CLASS}
      data-testid={testid.tradeBar}
      data-spot={spotTestid(focus)}
      style={{ position: "fixed", left: `${box.left + box.width / 2}px`, top: `${box.top}px` }}
    >
      <TradeButton
        offer={offer}
        instanceId={card}
        onTrade={(instanceId) => {
          const body = legal.find((action) => action.type === "trade" && action.instanceId === instanceId);
          if (body !== undefined) onTrade(body);
          close();
        }}
      />
    </div>
  );
}
