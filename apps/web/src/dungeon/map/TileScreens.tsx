// The dungeon run's tiles (DR7, DR14 part F, #2707): the shop, the campfire, the event and the
// shrine. Every control sends the frozen `RunAction` and shows the run's refusal where the screen
// stands; every shop service names its price, which `runShopPrices` answers from the run (the
// stock's after the discount, the services' Coins and Tickets, a relic's next level), and a service
// the purse cannot pay is disabled while the run still decides. The campfire removes the cards
// `runConstants().campfireRemove` says. Players read "relic"; the code keeps the run's word,
// artifact (WRITING.md's jargon). Presentation only (CLAUDE.md rule 7): the run answers every
// rule, and every word goes through `t()` at render.

import { useMemo, useState, type ReactElement } from "react";

import type { RunContent, RunPhase, RunState } from "@jackioh/run";

import { t } from "../../i18n/i18n.ts";
import { runConstants, runShopPrices } from "../../wasm/index.ts";
import { dungeonMapTestid } from "./testids.ts";
import {
  cardName,
  IndexedCardChoices,
  PassiveChoice,
  passiveName,
  RunCardFace,
  TileSection,
  type SendRunAction,
} from "./RunDisplay.tsx";

type ShopPhase = Extract<RunPhase, { phase: "shop" }>;
type EventPhase = Extract<RunPhase, { phase: "event" }>;
type ShrinePhase = Extract<RunPhase, { phase: "shrine" }>;

type TileProps = {
  run: RunState;
  content: RunContent;
  send: SendRunAction;
};

/** The shop: stock for Coins, card services, leveling relics, and leaving, each with its price. */
export function ShopScreen({ run, content, phase, send }: TileProps & { phase: ShopPhase }): ReactElement {
  const prices = useMemo(() => runShopPrices(run, content), [run, content]);
  return (
    <section className="dungeon-run-shop" data-testid={dungeonMapTestid.shop}>
      <h2 className="dungeon-run-title">{t("Shop")}</h2>
      <TileSection title={t("Stock")}>
        <ul className="dungeon-run-choices">
          {phase.stock.map((item, index) => (
            <li className="dungeon-run-stock" data-testid={dungeonMapTestid.shopStock(index)} key={index}>
              {item.good.type === "card" ? (
                <RunCardFace id={item.good.id} radiant={false} content={content} />
              ) : (
                <PassiveChoice id={item.good.id} content={content} />
              )}
              <span className="dungeon-run-stock__price">
                {t("{price} Coins", { price: String(prices.stockCoins[index] ?? item.price) })}
              </span>
              {item.sold ? (
                <span className="dungeon-run-stock__sold">{t("Sold")}</span>
              ) : (
                <button
                  type="button"
                  data-testid={dungeonMapTestid.shopBuy(index)}
                  aria-label={t("Buy {name}", {
                    name:
                      item.good.type === "card"
                        ? cardName(content, item.good.id)
                        : passiveName(content, item.good.id),
                  })}
                  onClick={() => {
                    send({ type: "buy", index });
                  }}
                >
                  {t("Buy")}
                </button>
              )}
            </li>
          ))}
        </ul>
      </TileSection>
      <TileSection title={t("Remove a card for {price} Coins", { price: String(prices.removeCoins) })}>
        <IndexedCardChoices
          run={run}
          content={content}
          testid={(index) => dungeonMapTestid.shopRemove(index)}
          label={(card) => cardName(content, card.id)}
          disabledIndex={() => run.coins < prices.removeCoins}
          onPick={(index) => {
            send({ type: "removeCard", index });
          }}
        />
      </TileSection>
      <TileSection title={t("Make a card Radiant for {price} Tickets", { price: String(prices.radiantTickets) })}>
        <IndexedCardChoices
          run={run}
          content={content}
          testid={(index) => dungeonMapTestid.shopRadiant(index)}
          label={(card) => cardName(content, card.id)}
          disabledIndex={() => run.tickets < prices.radiantTickets}
          onPick={(index) => {
            send({ type: "makeRadiant", index });
          }}
        />
      </TileSection>
      <TileSection title={t("Buff a card for {price} Tickets", { price: String(prices.buffTickets) })}>
        <IndexedCardChoices
          run={run}
          content={content}
          testid={(index) => dungeonMapTestid.shopBuff(index)}
          label={(card) => cardName(content, card.id)}
          disabledIndex={() => run.tickets < prices.buffTickets}
          onPick={(index) => {
            send({ type: "buff", index });
          }}
        />
      </TileSection>
      <TileSection title={t("Level a relic for Tickets")}>
        <ul className="dungeon-run-choices">
          {run.artifacts.map((artifact, index) => {
            const price = prices.levelTickets[index] ?? null;
            return (
              <li key={`${String(index)}-${artifact.id}`}>
                <PassiveChoice id={artifact.id} level={artifact.level} content={content} />
                {price === null ? (
                  <span className="dungeon-run-note" data-testid={dungeonMapTestid.shopLevelMax(index)}>
                    {t("Highest level")}
                  </span>
                ) : (
                  <button
                    type="button"
                    data-testid={dungeonMapTestid.shopLevel(index)}
                    disabled={run.tickets < price}
                    onClick={() => {
                      send({ type: "levelArtifact", index });
                    }}
                  >
                    {t("Level up for {price} Tickets", { price: String(price) })}
                  </button>
                )}
              </li>
            );
          })}
        </ul>
      </TileSection>
      <div className="dungeon-run-actions">
        <button
          type="button"
          data-testid={dungeonMapTestid.shopLeave}
          onClick={() => {
            send({ type: "leave" });
          }}
        >
          {t("Leave")}
        </button>
      </div>
    </section>
  );
}

/** The campfire: more max HP, make a card Radiant, buff a card, or remove cards. */
export function CampfireScreen({ run, content, send }: TileProps): ReactElement {
  const { campfireMaxHp, campfireRemove } = useMemo(() => runConstants(), []);
  const [removing, setRemoving] = useState<readonly number[]>([]);
  // A picked card is put back; another is taken only while fewer than `campfireRemove` are.
  const toggleRemove = (index: number): void => {
    setRemoving((was) => {
      if (was.includes(index)) return was.filter((kept) => kept !== index);
      return was.length < campfireRemove ? [...was, index] : was;
    });
  };
  return (
    <section className="dungeon-run-campfire" data-testid={dungeonMapTestid.campfire}>
      <h2 className="dungeon-run-title">{t("Campfire")}</h2>
      <div className="dungeon-run-actions">
        <button
          type="button"
          className="button-primary"
          data-testid={dungeonMapTestid.campfireMaxHp}
          onClick={() => {
            send({ type: "gainMaxHp" });
          }}
        >
          {t("Gain {n} max HP", { n: String(campfireMaxHp) })}
        </button>
      </div>
      <TileSection title={t("Make a card Radiant")}>
        <IndexedCardChoices
          run={run}
          content={content}
          testid={(index) => dungeonMapTestid.campfireRadiant(index)}
          label={(card) => cardName(content, card.id)}
          onPick={(index) => {
            send({ type: "makeRadiant", index });
          }}
        />
      </TileSection>
      <TileSection title={t("Buff a card")}>
        <IndexedCardChoices
          run={run}
          content={content}
          testid={(index) => dungeonMapTestid.campfireBuff(index)}
          label={(card) => cardName(content, card.id)}
          onPick={(index) => {
            send({ type: "buff", index });
          }}
        />
      </TileSection>
      <TileSection title={t("Remove {count} cards", { count: String(campfireRemove) })}>
        <IndexedCardChoices
          run={run}
          content={content}
          testid={(index) => dungeonMapTestid.campfireRemoveCard(index)}
          label={(card) => cardName(content, card.id)}
          selectedIndex={(_card, index) => removing.includes(index)}
          disabledIndex={(_card, index) => removing.length >= campfireRemove && !removing.includes(index)}
          onPick={toggleRemove}
        />
        <div className="dungeon-run-actions">
          <button
            type="button"
            data-testid={dungeonMapTestid.campfireRemove}
            disabled={removing.length !== campfireRemove}
            onClick={() => {
              send({ type: "removeCards", indices: [...removing] });
            }}
          >
            {t("Remove these cards")}
          </button>
        </div>
      </TileSection>
    </section>
  );
}

/** The event: its story and its choices. */
export function EventScreen({ content, phase, send }: TileProps & { phase: EventPhase }): ReactElement {
  const def = content.events.find((event) => event.id === phase.id);
  return (
    <section className="dungeon-run-event" data-testid={dungeonMapTestid.event}>
      <h2 className="dungeon-run-title">{t("Event")}</h2>
      {def === undefined ? null : (
        <>
          <p className="dungeon-run-note">{t(def.text)}</p>
          <ul className="dungeon-run-choices">
            {def.options.map((option, index) => (
              <li key={index}>
                <button
                  type="button"
                  className="dungeon-run-choice"
                  data-testid={dungeonMapTestid.eventOption(index)}
                  onClick={() => {
                    send({ type: "choose", index });
                  }}
                >
                  {t(option.text)}
                </button>
              </li>
            ))}
          </ul>
        </>
      )}
    </section>
  );
}

/** The shrine: the blessings offered, taken or left. */
export function ShrineScreen({ content, phase, send }: TileProps & { phase: ShrinePhase }): ReactElement {
  return (
    <section className="dungeon-run-shrine" data-testid={dungeonMapTestid.shrine}>
      <h2 className="dungeon-run-title">{t("Shrine")}</h2>
      <ul className="dungeon-run-choices">
        {phase.offers.map((id, index) => (
          <li key={`${String(index)}-${id}`}>
            <button
              type="button"
              className="dungeon-run-choice"
              data-testid={dungeonMapTestid.shrineOffer(index)}
              onClick={() => {
                send({ type: "bless", index });
              }}
            >
              <PassiveChoice id={id} content={content} />
            </button>
          </li>
        ))}
      </ul>
      <div className="dungeon-run-actions">
        <button
          type="button"
          data-testid={dungeonMapTestid.shrineLeave}
          onClick={() => {
            send({ type: "bless", index: null });
          }}
        >
          {t("Leave")}
        </button>
      </div>
    </section>
  );
}
