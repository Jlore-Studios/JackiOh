// The two bars of a card pack's opening (#1658, part of #1651, SPEC §14.5, R1305000): the card token
// bar and the cosmetic token bar, each filling from where the opening found it to where it left it,
// and the reward of every fill revealed as the bar bursts. The pack-opening scene mounts it with the
// opening's answer.
//
// THE SERVER DECIDES (CLAUDE.md rule 7). `bars` is the server's answer: the points before and after,
// the size, how often each bar paid and what the cosmetic bar paid (R1305001). Nothing here counts a
// point or picks a reward; a bar the opening did not move is not drawn, and with neither moved
// nothing is.

import { useState, type ReactElement } from "react";

import { t } from "../i18n/i18n.ts";
import type { OpeningBarsAnswer } from "../net/api.ts";
import { barMoved, fillReward, type RevealItem } from "./bars.ts";
import FillBar from "./FillBar.tsx";
import RewardReveal from "./RewardReveal.tsx";
import "./economy.css";

export type OpeningBarsProps = {
  bars: OpeningBarsAnswer;
  testid?: string;
};

/** The bars of one opening; a new answer plays from its start, its rewards shown afresh. */
export default function OpeningBars({ bars, testid }: OpeningBarsProps): ReactElement | null {
  return <Opening key={JSON.stringify(bars)} bars={bars} testid={testid} />;
}

function Opening({ bars, testid }: OpeningBarsProps): ReactElement | null {
  const [rewards, setRewards] = useState<readonly RevealItem[]>([]);

  const reveal = (bar: "card" | "cosmetic", fill: number): void => {
    const item = fillReward(bars, bar, fill);
    if (item === null) return;
    setRewards((before) => (before.some((shown) => shown.key === item.key) ? before : [...before, item]));
  };

  const cardMoved = barMoved(bars.card);
  const cosmeticMoved = barMoved(bars.cosmetic);
  if (!cardMoved && !cosmeticMoved) return null;
  const part = (name: string): string | undefined => (testid === undefined ? undefined : `${testid}-${name}`);
  return (
    <div className="opening-bars" data-testid={testid}>
      {cardMoved ? (
        <FillBar
          label={t("Card token bar")}
          size={bars.card.size}
          value={bars.card.after}
          from={bars.card.before}
          fills={bars.card.pays}
          onFill={(fill) => {
            reveal("card", fill);
          }}
          testid={part("card")}
        />
      ) : null}
      {cosmeticMoved ? (
        <FillBar
          label={t("Cosmetic token bar")}
          size={bars.cosmetic.size}
          value={bars.cosmetic.after}
          from={bars.cosmetic.before}
          fills={bars.cosmetic.pays}
          onFill={(fill) => {
            reveal("cosmetic", fill);
          }}
          testid={part("cosmetic")}
        />
      ) : null}
      <RewardReveal items={rewards} testid={part("rewards")} />
    </div>
  );
}
