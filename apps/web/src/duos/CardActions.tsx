// The card bar and the ping markers (#936 B62, B64), drawn over whichever board the Duos board shows.
// The bar's pass and Trade (`TradeControl.tsx`, R1838000) controls are live exactly when `legal` lists
// that card's `pass` (B19) or `trade` (rule 7), else greyed. The bar stays DUOS_HOVER_GRACE_MS after the
// pointer leaves its card, so the pointer can reach it.
//
// Game.tsx stays the one board: nothing here is inside it. The bar and the markers are laid over it, at
// the box of the element it draws for the spot (`spots.ts`), re-measured every frame while any is up,
// since cards move as the board animates (`game/cardBar.ts`, which the Trade control's bar shares).

import { useMemo, type ReactElement } from "react";

import type { PlayerView } from "@jackioh/shared";

import type { DuosSeat } from "../wire/generated/DuosSeat.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { PING_KINDS, type PingKind, type PingTarget } from "../net/duos.ts";
import { useBoxes, useCardBarFocus } from "../game/cardBar.ts";
import { TradeButton, offersTrade, type TradeOffer } from "../game/TradeControl.tsx";
import { DUOS_HOVER_GRACE_MS } from "./config.ts";
import { PING_GLYPH, PING_LABEL } from "./pings.ts";
import { seatLabel } from "./seats.ts";
import { spotTestid, targetOfSpot, type BoardSpot } from "./spots.ts";
import { duosTestid } from "./testids.ts";

/** What the pass control shows on the seat's own hand cards (B62). */
export type PassOffer = {
  /** The hand cards `legal` lists a `pass` for, by instance id: only these are live. */
  legal: ReadonlySet<string>;
  /** The team's `pass_cost`, in mana. */
  cost: number;
  /** `pass_cap` less this round's passes (B22). */
  left: number;
};

/** One ping to mark on this board. */
export type PingMarker = { key: string; spot: BoardSpot; ping: PingKind; from: DuosSeat; mine: boolean };

export type CardActionsProps = {
  /**
   * The element the board is drawn in, once mounted. An element, not a ref: a parent's ref is attached
   * after its children's layout effects run, so a ping already up would wait a frame for its marker.
   */
  container: HTMLDivElement | null;
  /** The view the board is showing, and the duel it is of. */
  view: PlayerView;
  duel: 0 | 1;
  /** Whether the bar is offered at all: not once the game is over. */
  enabled: boolean;
  /** The pass control, on the seat's own board; null on the teammate's. */
  pass: PassOffer | null;
  onPass: (instanceId: string) => void;
  /** R1838000: the Trade control on the seat's own Tradable hand cards; null on the teammate's board. */
  trade?: TradeOffer | null;
  onTrade?: (instanceId: string) => void;
  onPing: (ping: PingKind, target: PingTarget) => void;
  markers: readonly PingMarker[];
};

const BAR_CLASS = "duos-card-actions";

export function CardActions({
  container,
  view,
  duel,
  enabled,
  pass,
  onPass,
  trade = null,
  onTrade,
  onPing,
  markers,
}: CardActionsProps): ReactElement {
  useLocale();
  const { focus, close } = useCardBarFocus(container, BAR_CLASS, DUOS_HOVER_GRACE_MS);

  const shownFocus = enabled ? focus : null;
  const idsKey = useMemo(() => {
    const ids = new Set<string>();
    if (shownFocus !== null) ids.add(spotTestid(shownFocus));
    for (const marker of markers) ids.add(spotTestid(marker.spot));
    return [...ids].sort().join("|");
  }, [shownFocus, markers]);
  const boxes = useBoxes(container, idsKey);

  const box = shownFocus === null ? undefined : boxes.get(spotTestid(shownFocus));
  const handCard = shownFocus?.at === "hand" ? shownFocus.instanceId : null;
  const ownHand = Array.isArray(view.you.hand) ? view.you.hand : [];
  const ownCard = handCard !== null && ownHand.some((card) => card.instanceId === handCard) ? handCard : null;
  const passCard = pass !== null ? ownCard : null;
  const tradeCard = trade !== null && ownCard !== null && offersTrade(trade, ownCard) ? ownCard : null;

  return (
    <>
      {markers.map((marker) => {
        const at = boxes.get(spotTestid(marker.spot));
        if (at === undefined) return null;
        return (
          <span
            key={marker.key}
            className="duos-ping-marker"
            data-testid={duosTestid.pingMarker}
            data-ping={marker.ping}
            data-from={marker.from}
            data-mine={marker.mine ? "true" : undefined}
            style={{ left: `${at.left}px`, top: `${at.top}px`, width: `${at.width}px`, height: `${at.height}px` }}
            role="img"
            aria-label={t("{seat}: {ping}", { seat: seatLabel(marker.from), ping: t(PING_LABEL[marker.ping]) })}
          >
            <span className="duos-ping-marker__glyph" aria-hidden="true">
              {PING_GLYPH[marker.ping]}
            </span>
          </span>
        );
      })}
      {shownFocus === null || box === undefined ? null : (
        <div
          className={BAR_CLASS}
          data-testid={duosTestid.cardActions}
          data-spot={spotTestid(shownFocus)}
          style={{ left: `${box.left + box.width / 2}px`, top: `${box.top}px` }}
        >
          <div className="duos-ping-wheel" data-testid={duosTestid.pingWheel} role="group" aria-label={t("Ping your teammate")}>
            {PING_KINDS.map((kind) => (
              <button
                key={kind}
                type="button"
                className="duos-ping-wheel__ping"
                data-testid={duosTestid.ping(kind)}
                data-ping={kind}
                aria-label={t(PING_LABEL[kind])}
                title={t(PING_LABEL[kind])}
                onClick={() => {
                  const target = targetOfSpot(shownFocus, view, duel);
                  if (target !== null) onPing(kind, target);
                  close();
                }}
              >
                <span aria-hidden="true">{PING_GLYPH[kind]}</span>
              </button>
            ))}
          </div>
          {pass === null || passCard === null ? null : (
            <button
              type="button"
              className="duos-pass"
              data-testid={duosTestid.pass}
              data-instance-id={passCard}
              data-legal={pass.legal.has(passCard) ? "true" : "false"}
              disabled={!pass.legal.has(passCard)}
              onClick={() => {
                if (!pass.legal.has(passCard)) return;
                onPass(passCard);
                close();
              }}
            >
              <span className="duos-pass__label">{t("Pass to teammate")}</span>
              <span className="duos-pass__meta">{t("{cost} mana · {left} left", { cost: pass.cost, left: pass.left })}</span>
            </button>
          )}
          {trade === null || tradeCard === null ? null : (
            <TradeButton
              offer={trade}
              instanceId={tradeCard}
              onTrade={(instanceId) => {
                onTrade?.(instanceId);
                close();
              }}
            />
          )}
        </div>
      )}
    </>
  );
}
