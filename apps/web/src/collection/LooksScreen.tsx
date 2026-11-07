// R3016001 (#3016): a card's equip and buy screen, opened by "Equip or buy" on its hover preview or its fullscreen
// preview (LookHover.tsx, OwnedCards.tsx). Fullscreen, modal, focus trapped, like the preview: it is no list or grid,
// so R1953001's rule that no list or grid carries a spend button holds. It lists the card's variants as their ladders
// (collection.ts `cardLadders`, R3015007), lowest level first, each level a rung:
// - an owned level has Equip, which wears it on this card alone (`writeCardLevel`, R3015005), and the worn one reads
//   Equipped;
// - the next level, every level below it owned (R3015004), has Buy. Buy asks how to pay and asks before it spends
//   (R2683000, R2480000): an alert dialog naming the level and what it spends, a choice of the held cosmetic tokens
//   that cover its rarity (R1305003; none covers a Mythic, R2728000) and, for a chase, a full Chase Shard bar (§14.6),
//   then Confirm, which sends one purchase (`redeemCosmeticToken` or `buyChase`), and Cancel, focused, which with
//   Escape sends nothing and gives the focus back to Buy. With nothing that can pay, Buy is disabled and says what is
//   missing, a card short of MAX_COPIES among it (a per-card cosmetic goes only on such a card, the server's
//   `choice_of`);
// - a level past the next has no Buy and names what it needs first.
// Hovering a rung, or its button's keyboard focus, shows that level in the three views (LookViews.tsx). It decides
// nothing (CLAUDE.md rule 7): ownership is the owned-cosmetics read (`useCanEquip`), the held tokens and the bar are
// the server's answers, and a refusal shows the server's words and changes nothing.

import { useEffect, useId, useLayoutEffect, useRef, useState, type KeyboardEvent, type ReactElement } from "react";

import { MAX_COPIES } from "@jackioh/engine/config";
import type { CardDef } from "@jackioh/shared";

import { readerName } from "../cards/chinese.ts";
import { useCanEquip, useCardPicks, writeCardLevel } from "../cards/cosmetics.ts";
import { faceModel } from "../cards/index.ts";
import { rereadOwnedCosmetics } from "../cards/ownedFullArt.ts";
import { formatNumber, t } from "../i18n/i18n.ts";
import { buyChase, getCosmeticPacks, redeemCosmeticToken, type CosmeticTokenTier, type TokenBarState } from "../net/api.ts";
import { useBackLayer } from "../net/backLayers.ts";
import { Segmented } from "../tabs/index.ts";
import { messageOf } from "../net/errors.ts";
import { spendOutNow } from "../net/spending.ts";
import { REGISTRY, type RegistryCosmetic } from "../shop/shop.ts";
import { NO_FULL_ART_ID, cardLadders, collectionTestid as T, equippedRung, type Rung } from "./collection.ts";
import { trapTab } from "./ItemPreview.tsx";
import { LookHoverPanel, useLookHover, type LookHoverTrigger } from "./LookHover.tsx";
import { coveringTiers, heldOf, useHeldTokens, type HeldTokens } from "./TokenOffer.tsx";
import { RungPicture } from "./VariantLadders.tsx";

/** R3016001: one way to pay for a level: a held cosmetic token of a tier, or the full Chase Shard bar. */
export type PayWay = CosmeticTokenTier | "chase";

/**
 * R3016001: the ways the account may pay for `rung`, the cheapest token first and the Chase Shard last: none for a
 * level that is not the next one, nor on a card short of MAX_COPIES. The server still decides (R3015004).
 */
export function payWays(
  rung: Pick<Rung, "next">,
  cosmetic: RegistryCosmetic | undefined,
  copies: number,
  held: HeldTokens | null,
  shard: TokenBarState | null,
): PayWay[] {
  if (!rung.next || cosmetic === undefined || copies < MAX_COPIES) return [];
  const ways: PayWay[] = coveringTiers(cosmetic, held);
  if (cosmetic.chase && shard !== null && shard.points >= shard.size) ways.push("chase");
  return ways;
}

/** R3016001: what a next level nothing pays for lacks. */
export function missingWords(cosmetic: RegistryCosmetic | undefined, copies: number): string {
  if (copies < MAX_COPIES) return t("Collect {count} copies of this card to buy its looks", { count: formatNumber(MAX_COPIES) });
  return cosmetic?.chase === true ? t("No token you hold covers it, and the Chase Shard bar isn't full") : t("No token you hold covers it");
}

function wayWords(way: PayWay, held: HeldTokens | null): string {
  const count = (tier: CosmeticTokenTier): string => formatNumber(held === null ? 0 : heldOf(held, tier));
  switch (way) {
    case "Rare":
      return t("1 Rare token ({count} held)", { count: count(way) });
    case "Epic":
      return t("1 Epic token ({count} held)", { count: count(way) });
    case "Legendary":
      return t("1 Legendary token ({count} held)", { count: count(way) });
    case "chase":
      return t("Your full Chase Shard bar");
  }
}

function questionWords(way: PayWay, name: string): string {
  switch (way) {
    case "Rare":
      return t("Buy {name}? This spends 1 Rare token.", { name });
    case "Epic":
      return t("Buy {name}? This spends 1 Epic token.", { name });
    case "Legendary":
      return t("Buy {name}? This spends 1 Legendary token.", { name });
    case "chase":
      return t("Buy {name}? This spends your full Chase Shard bar.", { name });
  }
}

/** The Chase Shard bar `token`'s account fills, read once; null with no token, while unread, or after a failed read. */
function useShardBar(token: string | null): [TokenBarState | null, (bar: TokenBarState) => void] {
  const [bar, setBar] = useState<TokenBarState | null>(null);
  useEffect(() => {
    if (token === null) return undefined;
    let current = true;
    getCosmeticPacks(token).then(
      (answer) => {
        if (current) setBar(answer.shardBar);
      },
      () => {
        // No bar read, so no chase is offered: the Shop still shows the bar.
      },
    );
    return () => {
      current = false;
    };
  }, [token]);
  return [bar, setBar];
}

type Asking = { id: string; way: PayWay };

/** R3016001: the question Buy asks, in the rung's place: how to pay, then Confirm or Cancel. */
function PayQuestion({
  rung,
  ways,
  asking,
  held,
  busy,
  choose,
  confirm,
  cancel,
}: {
  rung: Rung;
  ways: readonly PayWay[];
  asking: Asking;
  held: HeldTokens | null;
  busy: boolean;
  choose: (way: PayWay) => void;
  confirm: () => void;
  cancel: () => void;
}): ReactElement {
  const questionId = useId();
  // Escape answers Cancel and nothing else: the screen closes on an Escape that reaches it.
  const cancelOnEscape = (event: KeyboardEvent<HTMLDivElement>): void => {
    if (event.key !== "Escape") return;
    event.stopPropagation();
    cancel();
  };
  return (
    <div className="collection-looks__question" role="alertdialog" aria-labelledby={questionId} data-testid={T.looksDialog(rung.id)} onKeyDown={cancelOnEscape}>
      <p id={questionId}>{questionWords(asking.way, rung.name)}</p>
      {ways.length > 1 ? (
        <Segmented
          label={t("Pay with")}
          className="collection-looks__ways"
          value={asking.way}
          onChange={choose}
          options={ways.map((way) => ({
            value: way,
            label: wayWords(way, held),
            testId: T.looksPay(rung.id, way),
          }))}
        />
      ) : null}
      <div className="collection-looks__answers">
        <button type="button" data-testid={T.looksConfirm(rung.id)} disabled={busy} onClick={confirm}>
          {t("Confirm")}
        </button>
        <button type="button" data-testid={T.looksCancel(rung.id)} autoFocus onClick={cancel}>
          {t("Cancel")}
        </button>
      </div>
    </div>
  );
}

/** R3016001: one level of a ladder: its picture, name and rarity, and Equip, Buy or what it needs. */
function RungRow({
  def,
  rung,
  equipped,
  ways,
  cosmetic,
  copies,
  asking,
  held,
  busy,
  error,
  hover,
  equip,
  ask,
  choose,
  confirm,
  cancel,
}: {
  def: CardDef;
  rung: Rung;
  equipped: boolean;
  ways: readonly PayWay[];
  cosmetic: RegistryCosmetic | undefined;
  copies: number;
  asking: Asking | null;
  held: HeldTokens | null;
  busy: boolean;
  error: string | null;
  hover: LookHoverTrigger;
  equip: (rung: Rung) => void;
  ask: (rung: Rung, way: PayWay) => void;
  choose: (way: PayWay) => void;
  confirm: () => void;
  cancel: () => void;
}): ReactElement {
  const missingId = useId();
  const face = faceModel({ defId: def.id, def, radiant: false });
  const first = ways[0];
  let action: ReactElement;
  if (rung.owned) {
    action = (
      <button
        type="button"
        className="collection-looks__equip"
        data-testid={T.looksEquip(rung.id)}
        aria-pressed={equipped}
        onClick={() => {
          equip(rung);
        }}
      >
        {equipped ? t("Equipped") : t("Equip")}
      </button>
    );
  } else if (rung.next && asking?.id === rung.id) {
    action = <PayQuestion rung={rung} ways={ways} asking={asking} held={held} busy={busy} choose={choose} confirm={confirm} cancel={cancel} />;
  } else if (rung.next) {
    action = (
      <span className="collection-looks__buy">
        <button
          type="button"
          className="button-primary"
          data-testid={T.looksBuy(rung.id)}
          disabled={first === undefined || busy}
          aria-describedby={first === undefined ? missingId : undefined}
          onClick={() => {
            if (first !== undefined) ask(rung, first);
          }}
        >
          {t("Buy")}
        </button>
        {first === undefined ? (
          <span id={missingId} className="collection-looks__missing" data-testid={T.looksMissing(rung.id)}>
            {missingWords(cosmetic, copies)}
          </span>
        ) : null}
      </span>
    );
  } else {
    action = (
      <span className="collection-looks__needs" data-testid={T.looksNeeds(rung.id)}>
        {t("Needs {name}", { name: rung.needs ?? "" })}
      </span>
    );
  }
  return (
    <li
      className="collection-looks__rung"
      data-testid={T.looksRung(rung.id)}
      data-owned={String(rung.owned)}
      data-next={String(rung.next)}
      data-equipped={String(equipped)}
      data-rarity={rung.rarity}
      {...hover}
    >
      <span className="collection-looks__picture" aria-hidden="true">
        <RungPicture def={def} face={face} rung={rung} />
      </span>
      <span className="collection-looks__words">
        <span className="collection-looks__name">{rung.name}</span>
        <span className="collection-looks__rarity">{t(rung.rarity)}</span>
      </span>
      {action}
      {error === null ? null : (
        <p className="collection-looks__error" role="alert" data-testid={T.looksError(rung.id)}>
          {error}
        </p>
      )}
    </li>
  );
}

/** R3016001: `def`'s equip and buy screen; `copies` the copies the account owns, `token` the session's (none buys nothing). */
export default function LooksScreen({
  def,
  copies,
  token,
  onClose,
  restoreFocus,
}: {
  def: CardDef;
  copies: number;
  token: string | null;
  onClose: () => void;
  /** Where the focus goes back when the element that opened the screen is gone (a hover panel's button). */
  restoreFocus?: (() => void) | undefined;
}): ReactElement {
  const canEquip = useCanEquip();
  const picks = useCardPicks(def.id);
  const ladders = cardLadders(def.id, canEquip);
  const equipped = equippedRung(def.id, picks, canEquip);
  const cosmeticOf = (id: string): RegistryCosmetic | undefined => REGISTRY.find((cosmetic) => cosmetic.id === id);
  const nexts = ladders.flatMap((ladder) => ladder.rungs).filter((rung) => rung.next);
  const buying = token !== null && copies >= MAX_COPIES && nexts.length > 0;
  const [held, setHeld] = useHeldTokens(buying ? token : null);
  const [shard, setShard] = useShardBar(buying && nexts.some((rung) => cosmeticOf(rung.id)?.chase === true) ? token : null);
  const [asking, setAsking] = useState<Asking | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<{ id: string; message: string } | null>(null);
  const hover = useLookHover();
  const root = useRef<HTMLDivElement>(null);
  const closeButton = useRef<HTMLButtonElement>(null);
  const refocus = useRef<string | null>(null);
  const latestRestore = useRef(restoreFocus);
  useLayoutEffect(() => {
    latestRestore.current = restoreFocus;
  });

  // Focus moves in, and goes back to what opened the screen (else `restoreFocus`).
  useLayoutEffect(() => {
    const opener = document.activeElement;
    closeButton.current?.focus({ preventScroll: true });
    return () => {
      if (opener instanceof HTMLElement && opener.isConnected && opener !== document.body) opener.focus();
      else latestRestore.current?.();
    };
  }, []);
  // R2127000: Back closes the screen as Close does.
  useBackLayer(onClose);
  // Cancel, Escape and an answer give the focus back to the rung's Buy, or its Equip once it is owned.
  useEffect(() => {
    const id = refocus.current;
    if (asking !== null || id === null) return;
    refocus.current = null;
    const button = root.current?.querySelector<HTMLElement>(`[data-testid="${T.looksBuy(id)}"], [data-testid="${T.looksEquip(id)}"]`);
    button?.focus();
  }, [asking]);

  const equip = (rung: Rung): void => {
    if (rung.owned) writeCardLevel(def.id, rung.id === NO_FULL_ART_ID ? null : rung.id);
  };
  const ask = (rung: Rung, way: PayWay): void => {
    hover.close();
    setError(null);
    setAsking({ id: rung.id, way });
  };
  const choose = (way: PayWay): void => {
    setAsking((current) => (current === null ? null : { ...current, way }));
  };
  const cancel = (): void => {
    refocus.current = asking?.id ?? null;
    setAsking(null);
  };
  const confirm = (): void => {
    if (asking === null || busy || token === null || spendOutNow()) return;
    const { id, way } = asking;
    setBusy(true);
    const sent =
      way === "chase"
        ? buyChase(token, id, def.id).then((answer) => {
            setShard(answer.shardBar);
          })
        : redeemCosmeticToken(token, way, id, def.id).then((answer) => {
            setHeld(answer.tokens);
          });
    sent
      .then(
        () => {
          rereadOwnedCosmetics();
        },
        (cause: unknown) => {
          setError({ id, message: messageOf(cause) });
        },
      )
      .finally(() => {
        refocus.current = id;
        setAsking(null);
        setBusy(false);
      });
  };
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>): void => {
    // Escape closes this screen alone: a preview under it stays open.
    if (event.key === "Escape") {
      event.stopPropagation();
      onClose();
      return;
    }
    trapTab(event);
  };

  const hovered = hover.hovered;
  return (
    <div
      ref={root}
      className="collection-looks"
      data-testid={T.looks}
      data-card={def.id}
      role="dialog"
      aria-modal="true"
      aria-label={t("Looks for {name}", { name: readerName(def) })}
      onKeyDown={onKeyDown}
    >
      <div className="collection-looks__scrim" aria-hidden="true" onClick={onClose} />
      <div className="collection-looks__panel">
        <h3 className="collection-looks__title">{t("Looks for {name}", { name: readerName(def) })}</h3>
        {ladders.map((ladder) => (
          <section key={ladder.variant} className="collection-looks__ladder" data-variant={ladder.variant} aria-label={ladder.name}>
            <h4 className="collection-looks__variant">{ladder.name}</h4>
            <ol className="collection-looks__rungs">
              {ladder.rungs.map((rung) => (
                <RungRow
                  key={rung.id}
                  def={def}
                  rung={rung}
                  equipped={rung.id === equipped}
                  ways={payWays(rung, cosmeticOf(rung.id), copies, held, shard)}
                  cosmetic={cosmeticOf(rung.id)}
                  copies={copies}
                  asking={asking}
                  held={held}
                  busy={busy}
                  error={error?.id === rung.id ? error.message : null}
                  hover={hover.trigger(rung.id)}
                  equip={equip}
                  ask={ask}
                  choose={choose}
                  confirm={confirm}
                  cancel={cancel}
                />
              ))}
            </ol>
          </section>
        ))}
        <button type="button" ref={closeButton} className="collection-looks__close" data-testid={T.looksClose} onClick={onClose}>
          {t("Close")}
        </button>
      </div>
      {hovered === null || asking !== null ? null : (
        <LookHoverPanel key={hovered.id} def={def} owned={copies > 0} anchor={hovered.anchor} hover={hover} level={hovered.id} />
      )}
    </div>
  );
}
