// The Jlore pill (#1659, part of #1651; SPEC §14.1): Jlore's coin and the balance `GET /api/wallet`
// answers, in the top bar of every signed-in screen that shows one. It links to `/shop`. A visitor
// has no session, so it makes no request and renders nothing; neither does a session whose wallet
// read the server refuses. The balance is the server's (CLAUDE.md rule 7): see `wallet.ts`.
//
// A new balance is counted to, up or down, in the number only (the pill's box does not move); reduce
// motion, the setting or the system's (`reducedMotionNow`), shows it at once.

import { useEffect, useRef, useState, type ReactElement } from "react";

import { reducedMotionNow } from "../game/animations.ts";
import { formatNumber, t, useLocale } from "../i18n/i18n.ts";
import { paths } from "../net/navigate.ts";
import { readSession } from "../net/session.ts";
import { followInApp } from "../routes/nav.tsx";
import { useWallet } from "./wallet.ts";
import "./jlore-pill.css";

export const jlorePillTestid = "jlore-pill";

/** How long the number takes to count from one balance to the next. */
export const JLORE_COUNT_MS = 600;

/** The balance on screen: `target` itself at once, or counted to when it changes and motion is allowed. */
function useCountedTo(target: number | null): number | null {
  const [shown, setShown] = useState<number | null>(target);
  const current = useRef<number | null>(target);

  useEffect(() => {
    const from = current.current;
    const show = (value: number | null): void => {
      current.current = value;
      setShown(value);
    };
    if (target === null || from === null || from === target || reducedMotionNow() || typeof window.requestAnimationFrame !== "function") {
      show(target);
      return;
    }
    let start: number | null = null;
    let frame = 0;
    const step = (now: number): void => {
      start ??= now;
      const done = Math.min(1, (now - start) / JLORE_COUNT_MS);
      show(done >= 1 ? target : Math.round(from + (target - from) * (1 - (1 - done) ** 3)));
      if (done < 1) frame = window.requestAnimationFrame(step);
    };
    frame = window.requestAnimationFrame(step);
    return () => {
      window.cancelAnimationFrame(frame);
    };
  }, [target]);

  return shown;
}

/** Jlore's coin: a brass disc with a stamped J, drawn so every skin's colour applies. */
function Coin(): ReactElement {
  return (
    <svg className="jlore-pill__coin" viewBox="0 0 24 24" width="20" height="20" focusable="false" aria-hidden="true">
      <circle cx="12" cy="12" r="10" fill="currentColor" opacity="0.28" />
      <circle cx="12" cy="12" r="10" fill="none" stroke="currentColor" strokeWidth="1.6" />
      <path d="M13.6 6.5 V13 a2.6 2.6 0 0 1 -5.2 0" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
    </svg>
  );
}

export default function JlorePill(): ReactElement | null {
  useLocale();
  const jlore = useWallet(readSession()?.accessToken ?? null);
  const shown = useCountedTo(jlore);
  if (jlore === null || shown === null) return null;
  return (
    <a
      className="jlore-pill"
      href={paths.shop}
      data-testid={jlorePillTestid}
      aria-label={t("{jlore} Jlore", { jlore: formatNumber(jlore) })}
      title={t("Jlore shop")}
      onClick={followInApp(paths.shop)}
    >
      <Coin />
      <span className="jlore-pill__count" aria-hidden="true">
        {formatNumber(shown)}
      </span>
    </a>
  );
}
