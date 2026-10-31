// The Try again every screen offers after a failed read (#1691). A wait a rate limit stated (R192)
// is counted down first and holds the button.

import type { ReactElement, ReactNode } from "react";

import { t } from "../i18n/i18n.ts";
import { useCountdown } from "./cooldown.ts";

function RetryButton({
  wait,
  onRetry,
  testId,
  className,
}: {
  wait: number;
  onRetry: () => void;
  testId: string;
  className?: string;
}): ReactElement {
  return (
    <button type="button" className={className} data-testid={testId} disabled={wait > 0} onClick={onRetry}>
      {wait > 0 ? t("Try again in {seconds} s", { seconds: String(wait) }) : t("Try again")}
    </button>
  );
}

/** Try again beside a failure said elsewhere; render it only while that failure shows, so each counts its own wait. */
export function TryAgainButton({
  retryAfterMs,
  onRetry,
  testId,
  className,
}: {
  retryAfterMs?: number;
  onRetry: () => void;
  testId: string;
  className?: string;
}): ReactElement {
  const wait = useCountdown(retryAfterMs);
  return <RetryButton wait={wait} onRetry={onRetry} testId={testId} className={className} />;
}

/** A failed read: its sentence and the wait left, then Try again (off through the wait) and `children` in one row. */
export function AccountError({
  message,
  retryAfterMs,
  onRetry,
  testId,
  retryTestId,
  children,
}: {
  message: string;
  retryAfterMs?: number;
  onRetry: () => void;
  testId: string;
  retryTestId: string;
  children?: ReactNode;
}): ReactElement {
  const wait = useCountdown(retryAfterMs);
  return (
    <>
      <p className="notice" data-testid={testId} role="alert">
        {t(message)}
        {wait > 0 ? ` ${t("You can try again in {seconds} s.", { seconds: String(wait) })}` : null}
      </p>
      <div className="row">
        <RetryButton wait={wait} onRetry={onRetry} testId={retryTestId} className="button-primary" />
        {children}
      </div>
    </>
  );
}
