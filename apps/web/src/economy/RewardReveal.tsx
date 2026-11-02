// The reward reveal (#1658, part of #1651, SPEC §14.5): what a bar, a pack or a level paid, flying
// out one by one, each with the pack opening's sting (`packs.ts::playStinger`; no new sound).
// THE SERVER DECIDES (CLAUDE.md rule 7): the items are what the server's answer says was paid.
// `items` may grow while it plays (a bar's fill adds its reward as it bursts): a key already shown
// never flies out again, and `onDone` is called again after later items. REDUCED MOTION shows every
// item at once with the strongest one's sting, as `PackScene` does for a skip-all.

import { useEffect, useRef, useState, type CSSProperties, type ReactElement } from "react";

import { playStinger } from "../packs/packs.ts";
import { REVEAL_FLY_MS, REVEAL_STAGGER_MS, stingOf, strongestSting, type RevealItem } from "./bars.ts";
import { useReducedMotion } from "./FillBar.tsx";
import "./economy.css";

export type RewardRevealProps = {
  items: readonly RevealItem[];
  testid?: string;
  /** Called when every item given so far has flown out. */
  onDone?: () => void;
};

export default function RewardReveal({ items, testid, onDone }: RewardRevealProps): ReactElement {
  const reduced = useReducedMotion();
  const [revealed, setRevealed] = useState<ReadonlySet<string>>(new Set());
  const stung = useRef<Set<string>>(new Set());
  const callbacks = useRef({ onDone });
  useEffect(() => {
    callbacks.current = { onDone };
  });

  const next = reduced ? undefined : items.find((item) => !revealed.has(item.key));
  const settled = !reduced && items.length > 0 && next === undefined;

  useEffect(() => {
    if (!reduced) return;
    const fresh = items.filter((item) => !stung.current.has(item.key));
    if (fresh.length === 0) return;
    for (const item of fresh) stung.current.add(item.key);
    playStinger(strongestSting(fresh));
    setRevealed((before) => new Set([...before, ...fresh.map((item) => item.key)]));
    callbacks.current.onDone?.();
  }, [reduced, items]);

  const nextKey = next?.key;
  useEffect(() => {
    if (next === undefined) return undefined;
    const timer = window.setTimeout(() => {
      stung.current.add(next.key);
      playStinger(stingOf(next));
      setRevealed((before) => new Set(before).add(next.key));
    }, REVEAL_STAGGER_MS);
    return () => {
      window.clearTimeout(timer);
    };
  }, [nextKey]);

  // The last item has flown out: done once its flight ends, unless another item arrives first.
  useEffect(() => {
    if (!settled) return undefined;
    const timer = window.setTimeout(() => {
      callbacks.current.onDone?.();
    }, REVEAL_FLY_MS);
    return () => {
      window.clearTimeout(timer);
    };
  }, [settled, revealed]);

  const shown = reduced ? items : items.filter((item) => revealed.has(item.key));
  return (
    <ul
      className="reward-reveal"
      data-testid={testid}
      aria-live="polite"
      style={{ "--reveal-ms": `${String(REVEAL_FLY_MS)}ms` } as CSSProperties}
    >
      {shown.map((item) => (
        <li
          key={item.key}
          className="reward-reveal__item"
          data-testid={testid === undefined ? undefined : `${testid}-${item.key}`}
          data-kind={item.kind}
          data-rarity={item.rarity}
        >
          {item.label}
        </li>
      ))}
    </ul>
  );
}
