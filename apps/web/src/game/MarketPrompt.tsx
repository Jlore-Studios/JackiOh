import { useEffect, useRef, type KeyboardEvent } from "react";

import type { ActionBody, PendingOption, PendingView, PlayerView } from "@jackioh/shared";

import { chineseName } from "../cards/chinese.ts";
import { CardFace, faceModel } from "../cards/index.ts";
import { answerAction } from "./actions.ts";
import { useCardInfo } from "./catalog.ts";
import "./market.css";

/** ME-MARKET (Meditative #42 CN Flea Market, R1000–R1002): the night market's stall. The engine
 * offers the deals (CLAUDE.md rule 7): each lot the yuan covers (`cost` its price), on the Radiant
 * face a barter per hand card (`cost` its yuan, written negative), and Leave. A click sends one deal;
 * the market reopens with the engine's next prompt. Nothing here prices, filters or refuses. */
type MarketProps = {
  view: PlayerView;
  pending: Extract<PendingView, { forYou: true }>;
  legal: readonly ActionBody[];
  onAction: (action: ActionBody) => void;
  /** The event type animating the open modal, for `data-animating` (as `PromptModal`'s). */
  animating?: string;
};

const LEAVE = "none";
const LANTERNS = 5;

function isLot(option: PendingOption): boolean {
  return option.instanceId === undefined && option.key !== LEAVE;
}

function yuan(amount: number): string {
  return `¥${String(amount)}`;
}

/** One lot on the stall: the card as it will arrive (its base face), its name and its price. */
function Lot(props: { option: PendingOption; onPick: () => void }) {
  const defId = props.option.defId ?? "";
  const info = useCardInfo(defId, false);
  const price = props.option.cost ?? 0;
  const face = faceModel({ defId, def: info.def, name: info.name, radiant: false, chinese: false, inPlay: {} });
  return (
    <li className="market-lot">
      <button
        type="button"
        className="market-deal"
        data-testid={`prompt-option-${props.option.key}`}
        aria-label={`${props.option.label}, ${String(price)} yuan`}
        onClick={props.onPick}
      >
        <CardFace face={face} layout="full" />
        <span className="market-caption">{props.option.label}</span>
        <span className="market-price">{yuan(price)}</span>
      </button>
    </li>
  );
}

/** One barter: a hand card traded to the merchant for its price. */
function Barter(props: { option: PendingOption; onPick: () => void }) {
  const defId = props.option.defId ?? "";
  const info = useCardInfo(defId, props.option.radiant === true);
  // ME-CN, R1301: a card the view says is Chinese is named in Chinese here too.
  const name = props.option.chinese === true ? chineseName(defId, info.name, info.def) : props.option.label;
  const gain = -(props.option.cost ?? 0);
  return (
    <li className="market-barter">
      <button
        type="button"
        className="market-deal"
        data-testid={`prompt-option-${props.option.key}`}
        aria-label={`Barter ${name} for ${String(gain)} yuan`}
        onClick={props.onPick}
      >
        <span className="market-caption">{name}</span>
        <span className="market-price">+{yuan(gain)}</span>
      </button>
    </li>
  );
}

export default function MarketPrompt(props: MarketProps) {
  const root = useRef<HTMLDivElement>(null);
  const lots = props.pending.options.filter(isLot);
  const barters = props.pending.options.filter((option) => option.instanceId !== undefined);
  const leave = props.pending.options.find((option) => option.key === LEAVE);

  const pick = (key: string): void => {
    props.onAction(answerAction(props.pending, [key], props.view, props.legal));
  };

  // The first deal takes the focus as the market opens, so the keyboard starts at the stall.
  useEffect(() => {
    root.current?.querySelector<HTMLButtonElement>('[data-testid^="prompt-option-"]')?.focus();
  }, []);

  // Arrows walk the deals in order, as the stall lays them out.
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>): void => {
    const step = event.key === "ArrowRight" || event.key === "ArrowDown" ? 1 : event.key === "ArrowLeft" || event.key === "ArrowUp" ? -1 : 0;
    if (step === 0 || root.current === null) return;
    const buttons = [...root.current.querySelectorAll<HTMLButtonElement>('[data-testid^="prompt-option-"]')];
    const at = buttons.findIndex((button) => button === document.activeElement);
    const next = buttons[(at + step + buttons.length) % buttons.length];
    if (next === undefined) return;
    event.preventDefault();
    next.focus();
  };

  return (
    <div className="prompt-scrim" data-testid="prompt-scrim">
      <div
        ref={root}
        className="prompt market"
        data-testid="prompt-modal"
        data-prompt-kind="market"
        data-animating={props.animating}
        role="dialog"
        aria-modal="true"
        aria-label="Night market"
        onKeyDown={onKeyDown}
      >
        <header className="market-header">
          <div className="market-lanterns" aria-hidden="true">
            {Array.from({ length: LANTERNS }, (_, at) => (
              <span key={at} className="market-lantern" />
            ))}
          </div>
          <h2 className="market-title">Night market</h2>
          <p className="market-purse" data-testid="market-purse" role="status">
            {yuan(props.pending.budget ?? 0)}
          </p>
        </header>
        {lots.length === 0 ? (
          <p className="market-empty" data-testid="market-empty">
            Nothing on the stall you can buy.
          </p>
        ) : (
          <ul className="market-stall" aria-label="The stall">
            {lots.map((option) => (
              <Lot key={option.key} option={option} onPick={() => pick(option.key)} />
            ))}
          </ul>
        )}
        {barters.length > 0 ? (
          <section className="market-barters" aria-label="Barter from your hand">
            <h3>Barter from your hand</h3>
            <ul className="market-barter-list">
              {barters.map((option) => (
                <Barter key={option.key} option={option} onPick={() => pick(option.key)} />
              ))}
            </ul>
          </section>
        ) : null}
        {leave === undefined ? null : (
          <button type="button" className="market-leave" data-testid={`prompt-option-${LEAVE}`} onClick={() => pick(LEAVE)}>
            Leave
          </button>
        )}
      </div>
    </div>
  );
}
